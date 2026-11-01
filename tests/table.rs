//! The table: ten thousand rows in a handful of entities, a header that sorts, columns that size,
//! rows that change under the player, and a cursor and a selection that keys, pad and pointer move.
//! Laid out for real in a headless window, driven by real pointer, keyboard and gamepad input.

use std::borrow::Cow;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedNode, ScrollPosition, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

#[derive(Resource, Default)]
struct Log {
    sorts: Vec<(usize, bool)>,
    selections: Vec<Vec<u64>>,
    opened: Vec<u64>,
}

/// How many `Node` writes the app has seen on styled entities.
#[derive(Resource, Default)]
struct Writes(u32);

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1280, 720).with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>(),
        GooeyPlugins,
    ))
    .init_resource::<Log>()
    .init_resource::<Writes>()
    .add_observer(|sort: On<SortBy>, mut log: ResMut<Log>| {
        log.sorts.push((sort.column, sort.descending));
    })
    .add_observer(
        |change: On<ValueChange<TableSelection>>, mut log: ResMut<Log>| {
            log.selections.push(change.value.iter().collect());
        },
    )
    .add_observer(|opened: On<RowActivated>, mut log: ResMut<Log>| {
        log.opened.push(opened.key);
    })
    .add_systems(
        Update,
        |nodes: Query<(), (Changed<Node>, With<Role>)>, mut writes: ResMut<Writes>| {
            writes.0 += nodes.iter().count() as u32;
        },
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn rect(app: &App, entity: Entity) -> Rect {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    let scale = node.inverse_scale_factor();
    Rect::from_center_size(at * scale, node.size() * scale)
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn location(app: &mut App, at: Vec2) -> Location {
    let window = window(app);
    Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    }
}

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let at = location(app, at);
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, at, action));
    app.update();
}

fn point_at(app: &mut App, to: Vec2) {
    pointer(app, to, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
}

fn click(app: &mut App, at: Vec2) {
    point_at(app, at);
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
}

fn key_event(app: &mut App, key_code: KeyCode, state: ButtonState, repeat: bool) {
    let window = window(app);
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state,
        text: None,
        repeat,
        window,
    });
    app.update();
}

fn tap(app: &mut App, key_code: KeyCode) {
    key_event(app, key_code, ButtonState::Pressed, false);
    key_event(app, key_code, ButtonState::Released, false);
}

/// Holds a modifier while `then` runs.
fn holding(app: &mut App, key_code: KeyCode, then: impl FnOnce(&mut App)) {
    key_event(app, key_code, ButtonState::Pressed, false);
    then(app);
    key_event(app, key_code, ButtonState::Released, false);
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    settle(app);
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn text_of(app: &App, entity: Entity) -> String {
    app.world()
        .get::<Text>(entity)
        .map(|t| t.0.clone())
        .unwrap_or_default()
}

/// The text in column `column` of a pool row.
fn cell(app: &App, row: Entity, column: usize) -> String {
    let cells = app.world().get::<Children>(row).expect("cells");
    let boxed = app.world().get::<Children>(cells[column]).expect("text");
    text_of(app, boxed[0])
}

/// The pool: each row entity and what it shows, top to bottom.
fn pool(app: &mut App) -> Vec<(TableRow, Entity)> {
    let mut rows: Vec<(TableRow, Entity)> = app
        .world_mut()
        .query::<(Entity, &TableRow)>()
        .iter(app.world())
        .map(|(e, r)| (*r, e))
        .collect();
    rows.sort_by_key(|(r, _)| r.index);
    rows
}

fn with_role(app: &mut App, role: Role) -> Vec<Entity> {
    let mut found: Vec<(Entity, f32)> = app
        .world_mut()
        .query::<(Entity, &Role)>()
        .iter(app.world())
        .filter(|(_, r)| **r == role)
        .map(|(e, _)| (e, 0.0))
        .collect();
    found.sort_by(|a, b| {
        let (ra, rb) = (rect(app, a.0), rect(app, b.0));
        (ra.min.y, ra.min.x)
            .partial_cmp(&(rb.min.y, rb.min.x))
            .expect("finite")
    });
    found.into_iter().map(|(e, _)| e).collect()
}

fn the_table(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<Table>>()
        .single(app.world())
        .expect("one table")
}

fn viewport(app: &mut App) -> Entity {
    with_role(app, Role::TableView)[0]
}

fn scroll_to(app: &mut App, y: f32) {
    let view = viewport(app);
    app.world_mut()
        .get_mut::<ScrollPosition>(view)
        .expect("scrolls")
        .y = y;
    settle(app);
}

fn pitch(app: &App) -> f32 {
    app.world().resource::<GooeySkin>().table_pitch()
}

fn items(n: usize) -> Vec<Vec<String>> {
    (0..n)
        .map(|i| {
            vec![
                format!("Item {i}"),
                format!("{}", (i * 37) % 1000),
                String::new(),
            ]
        })
        .collect()
}

fn columns() -> [Column; 3] {
    [
        Column::new("Name").width(12.0),
        Column::new("Qty").width(6.0),
        Column::new("Note").width(8.0).unsortable(),
    ]
}

fn spawn(app: &mut App, rows: impl RowSource, extra: impl Bundle) -> Entity {
    app.world_mut()
        .spawn((panel(), children![(table(columns(), rows), extra)]));
    settle(app);
    the_table(app)
}

fn spawn_items(app: &mut App, n: usize) -> Entity {
    spawn(app, items(n), ())
}

/// The centre of a pool row, in pointer units.
fn row_centre(app: &App, row: Entity) -> Vec2 {
    rect(app, row).center() * app.world().resource::<UiScale>().0
}

fn click_row(app: &mut App, row: Entity) {
    let at = row_centre(app, row);
    click(app, at);
}

fn click_entity(app: &mut App, entity: Entity) {
    let at = rect(app, entity).center() * app.world().resource::<UiScale>().0;
    click(app, at);
}

fn viewport_rect(app: &mut App) -> Rect {
    let view = viewport(app);
    rect(app, view)
}

/// A rows source with ids of its own, so rows can come and go and keep their keys.
struct Ids(Vec<u64>);

impl RowSource for Ids {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn cell(&self, row: usize, column: usize) -> Cow<'_, str> {
        match column {
            0 => Cow::Owned(format!("Row {}", self.0[row])),
            _ => Cow::Borrowed(""),
        }
    }
    fn key(&self, row: usize) -> u64 {
        self.0[row]
    }
}

// The acceptance: ten thousand rows cost visible + 2 row entities, and sorting by a column
// sends SortBy and reorders.

#[test]
fn ten_thousand_rows_spawn_at_most_visible_plus_two_entities() {
    let mut app = app();
    spawn_items(&mut app, 10_000);
    let visible = 8.5_f32.ceil() as usize;
    let shown = pool(&mut app);
    assert!(!shown.is_empty());
    assert!(
        shown.len() <= visible + 2,
        "{} rows spawned for {visible} visible",
        shown.len()
    );
    // However far it scrolls, and at a larger text size, it stays as small.
    for y in [1234.0, 100_000.0, 439_000.0, 0.0] {
        scroll_to(&mut app, y);
        assert!(pool(&mut app).len() <= visible + 2, "at scroll {y}");
    }
    app.world_mut().resource_mut::<GooeySkin>().text_scale = 2.0;
    settle(&mut app);
    assert!(pool(&mut app).len() <= visible + 2);
    // Every entity of the table: a header, a viewport and the pool, however long the data is.
    let mut small = self::app();
    spawn_items(&mut small, 20);
    assert_eq!(
        app.world().entities().count_spawned(),
        small.world().entities().count_spawned(),
        "ten thousand rows cost no more entities than twenty"
    );
}

#[test]
fn the_pool_shows_the_rows_the_scroll_has_in_view() {
    let mut app = app();
    spawn_items(&mut app, 10_000);
    let pitch = pitch(&app);
    scroll_to(&mut app, 5_000.0 * pitch);
    let shown = pool(&mut app);
    let (first, row) = shown[0];
    assert_eq!(first.index, 5_000);
    assert_eq!(cell(&app, row, 0), "Item 5000");
    // The first row stands at the top of the viewport, and each next one a pitch below.
    let top = viewport_rect(&mut app).min.y;
    assert!((rect(&app, row).min.y - top).abs() < 1.5);
    let (_, second) = shown[1];
    assert!((rect(&app, second).min.y - rect(&app, row).min.y - pitch).abs() < 1.5);
    // The last row of the data is the last row of the table, not of the pool.
    scroll_to(&mut app, 10_000.0 * pitch);
    let last = *pool(&mut app).last().expect("rows");
    assert_eq!(last.0.index, 9_999);
    assert_eq!(cell(&app, last.1, 0), "Item 9999");
}

#[test]
fn a_click_on_a_header_sorts_and_sends_one_sort_by() {
    let mut app = app();
    spawn_items(&mut app, 10_000);
    let heads = with_role(&mut app, Role::TableHeaderCell);
    assert_eq!(heads.len(), 3);
    // Ascending by quantity: the smallest first, ties in the source's order.
    click_entity(&mut app, heads[1]);
    assert_eq!(app.world().resource::<Log>().sorts, vec![(1, false)]);
    let table = the_table(&mut app);
    assert_eq!(
        *app.world().get::<TableSort>(table).expect("state"),
        TableSort {
            column: Some(1),
            descending: false
        }
    );
    let rows = items(10_000);
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|&a, &b| natural_cmp(&rows[a][1], &rows[b][1]).then(a.cmp(&b)));
    let shown = pool(&mut app);
    for (slot, (_, row)) in shown.iter().enumerate().take(4) {
        assert_eq!(cell(&app, *row, 0), rows[order[slot]][0], "slot {slot}");
    }
    // A second click turns it round.
    click_entity(&mut app, heads[1]);
    assert_eq!(
        app.world().resource::<Log>().sorts,
        vec![(1, false), (1, true)]
    );
    let shown = pool(&mut app);
    let top = (0..rows.len())
        .max_by(|&a, &b| natural_cmp(&rows[a][1], &rows[b][1]).then(b.cmp(&a)))
        .expect("rows");
    assert_eq!(cell(&app, shown[0].1, 0), rows[top][0]);
    // A column that does not sort sends nothing and changes nothing.
    click_entity(&mut app, heads[2]);
    assert_eq!(app.world().resource::<Log>().sorts.len(), 2);
    // The mark says which way.
    assert_eq!(
        with_role(&mut app, Role::TableSortDown)
            .into_iter()
            .filter(|&m| app.world().get::<Node>(m).expect("node").display == Display::Flex)
            .count(),
        1
    );
}

#[test]
fn a_sort_the_game_sets_sorts_and_sends_nothing() {
    let mut app = app();
    let table = spawn_items(&mut app, 500);
    *app.world_mut().get_mut::<TableSort>(table).expect("state") = TableSort {
        column: Some(0),
        descending: true,
    };
    settle(&mut app);
    assert!(app.world().resource::<Log>().sorts.is_empty());
    // "Item 99" is the last in natural order below 500: Item 499 is first when descending.
    let first = pool(&mut app)[0].1;
    assert_eq!(cell(&app, first, 0), "Item 499");
}

#[test]
fn rows_that_change_while_scrolled_keep_the_row_the_player_was_looking_at() {
    let mut app = app();
    let table = spawn(&mut app, Ids((1..=10_000).collect()), ());
    let pitch = pitch(&app);
    // Row 5000 (id 5001) at the top, a third of the way into it.
    scroll_to(&mut app, 5_000.0 * pitch + pitch / 3.0);
    let before = pool(&mut app)[0];
    assert_eq!(cell(&app, before.1, 0), "Row 5001");
    let top = viewport_rect(&mut app).min.y;
    let offset = rect(&app, before.1).min.y - top;
    // A hundred rows come in above it.
    app.world_mut()
        .get_mut::<Table>(table)
        .expect("table")
        .source_mut::<Ids>()
        .expect("ids")
        .0
        .splice(0..0, 20_001..=20_100);
    settle(&mut app);
    let after = pool(&mut app);
    let seen = after
        .iter()
        .find(|(r, _)| r.key == 5_001)
        .copied()
        .expect("the anchor row is still drawn");
    assert_eq!(seen.0.index, 5_100);
    assert!(
        (rect(&app, seen.1).min.y - top - offset).abs() < 1.5,
        "the row moved {} px",
        rect(&app, seen.1).min.y - top - offset
    );
    // Rows gone from under it: the scroll stays where it was.
    app.world_mut()
        .get_mut::<Table>(table)
        .expect("table")
        .source_mut::<Ids>()
        .expect("ids")
        .0
        .retain(|&id| id != 5_001);
    settle(&mut app);
    assert!(pool(&mut app).iter().all(|(r, _)| r.key != 5_001));
}

#[test]
fn a_selected_row_that_is_removed_is_no_longer_selected() {
    let mut app = app();
    let table = spawn(&mut app, Ids((1..=50).collect()), ());
    app.world_mut()
        .get_mut::<TableSelection>(table)
        .expect("state")
        .0
        .extend([3, 4]);
    settle(&mut app);
    app.world_mut()
        .get_mut::<Table>(table)
        .expect("table")
        .source_mut::<Ids>()
        .expect("ids")
        .0
        .retain(|&id| id != 3);
    settle(&mut app);
    let kept: Vec<u64> = app
        .world()
        .get::<TableSelection>(table)
        .expect("state")
        .iter()
        .collect();
    assert_eq!(kept, vec![4]);
    // The game's own change sent no event.
    assert!(app.world().resource::<Log>().selections.is_empty());
}

// The cursor and the keys.

#[test]
fn the_arrows_move_a_cursor_row_and_scroll_it_into_view() {
    let mut app = app();
    let table = spawn_items(&mut app, 10_000);
    focus(&mut app, table);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(app.world().get::<TableCursor>(table).expect("c").0, Some(0));
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(app.world().get::<TableCursor>(table).expect("c").0, Some(2));
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(app.world().get::<TableCursor>(table).expect("c").0, Some(1));
    // Down moved focus nowhere: it stays on the table.
    assert_eq!(focused(&app), Some(table));
    // End is the last row of the data; the pool shows it.
    tap(&mut app, KeyCode::End);
    assert_eq!(
        app.world().get::<TableCursor>(table).expect("c").0,
        Some(9_999)
    );
    let shown = pool(&mut app);
    let last = shown.last().expect("rows");
    assert_eq!(last.0.index, 9_999);
    assert!(last.0.cursor);
    tap(&mut app, KeyCode::Home);
    assert_eq!(app.world().get::<TableCursor>(table).expect("c").0, Some(0));
    assert_eq!(pool(&mut app)[0].0.index, 0);
    // A page is the rows in view less one.
    tap(&mut app, KeyCode::PageDown);
    assert_eq!(app.world().get::<TableCursor>(table).expect("c").0, Some(7));
    tap(&mut app, KeyCode::PageUp);
    assert_eq!(app.world().get::<TableCursor>(table).expect("c").0, Some(0));
}

#[test]
fn the_cursor_row_stays_whole_in_view_as_keys_walk_down() {
    let mut app = app();
    let table = spawn_items(&mut app, 200);
    focus(&mut app, table);
    let view = viewport_rect(&mut app);
    for _ in 0..40 {
        tap(&mut app, KeyCode::ArrowDown);
        let at = pool(&mut app)
            .into_iter()
            .find(|(r, _)| r.cursor)
            .expect("the cursor row is in the pool");
        let row = rect(&app, at.1);
        assert!(
            row.min.y >= view.min.y - 1.5 && row.max.y <= view.max.y + 1.5,
            "row {} at {}..{} outside {}..{}",
            at.0.index,
            row.min.y,
            row.max.y,
            view.min.y,
            view.max.y
        );
    }
}

#[test]
fn up_and_down_leave_the_table_from_its_first_and_last_rows() {
    let mut app = app();
    app.world_mut().spawn((
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (panel(), children![table(columns(), items(3))]),
            button("Below")
        ],
    ));
    settle(&mut app);
    let table = the_table(&mut app);
    let below = with_role(&mut app, Role::Button)[0];
    focus(&mut app, table);
    tap(&mut app, KeyCode::Home);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(
        focused(&app),
        Some(table),
        "Down in the middle moves the cursor"
    );
    assert_eq!(app.world().get::<TableCursor>(table).expect("c").0, Some(1));
    tap(&mut app, KeyCode::End);
    assert_eq!(focused(&app), Some(table));
    // At the last row Down has nowhere to go in the table, so focus goes on to the button.
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(below));
    assert_eq!(
        app.world().get::<TableCursor>(table).expect("c").0,
        Some(2),
        "the cursor is left where it was"
    );
    // Up at the button goes back to the table.
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(focused(&app), Some(table));
}

#[test]
fn enter_and_a_double_click_open_the_cursor_row() {
    let mut app = app();
    let table = spawn_items(&mut app, 50);
    focus(&mut app, table);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::Enter);
    assert_eq!(app.world().resource::<Log>().opened, vec![1]);
    // Held, Enter's repeats open nothing more.
    key_event(&mut app, KeyCode::Enter, ButtonState::Pressed, true);
    assert_eq!(app.world().resource::<Log>().opened.len(), 1);
    let row = pool(&mut app)[3].1;
    let at = row_centre(&app, row);
    point_at(&mut app, at);
    for _ in 0..2 {
        pointer(&mut app, at, PointerAction::Press(PointerButton::Primary));
        pointer(&mut app, at, PointerAction::Release(PointerButton::Primary));
    }
    settle(&mut app);
    assert_eq!(app.world().resource::<Log>().opened.last(), Some(&3));
}

// Selection.

#[test]
fn a_single_table_selects_the_row_clicked_and_the_arrow_moved_to() {
    let mut app = app();
    let table = spawn_items(&mut app, 100);
    let row = pool(&mut app)[2].1;
    click_row(&mut app, row);
    assert_eq!(
        app.world().resource::<Log>().selections,
        vec![vec![2]],
        "one change for one click"
    );
    assert_eq!(
        focused(&app),
        Some(table),
        "a click on a row focuses the table"
    );
    // Clicking the same row again changes nothing, so says nothing.
    click_row(&mut app, row);
    assert_eq!(app.world().resource::<Log>().selections.len(), 1);
    // An arrow moves the cursor and the selection with it.
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(
        app.world().resource::<Log>().selections.last(),
        Some(&vec![3])
    );
    // The selected row is lit.
    let lit: Vec<usize> = pool(&mut app)
        .into_iter()
        .filter(|(_, e)| app.world().get::<bevy::ui::Selected>(*e).is_some())
        .map(|(r, _)| r.index)
        .collect();
    assert_eq!(lit, vec![3]);
}

#[test]
fn a_multi_table_toggles_with_ctrl_and_ranges_with_shift() {
    let mut app = app();
    let table = spawn(&mut app, items(100), SelectMode::Multi);
    let rows: Vec<Entity> = pool(&mut app).iter().map(|(_, e)| *e).collect();
    click_row(&mut app, rows[1]);
    holding(&mut app, KeyCode::ControlLeft, |app| {
        click(app, row_centre(app, rows[3]));
        click(app, row_centre(app, rows[5]));
        click(app, row_centre(app, rows[3]));
    });
    let now: Vec<u64> = app
        .world()
        .get::<TableSelection>(table)
        .expect("s")
        .iter()
        .collect();
    assert_eq!(now, vec![1, 5]);
    holding(&mut app, KeyCode::ShiftLeft, |app| {
        click(app, row_centre(app, rows[4]));
    });
    // The range runs from where the selection began (the Ctrl-clicked 3 was the last anchor).
    let now: Vec<u64> = app
        .world()
        .get::<TableSelection>(table)
        .expect("s")
        .iter()
        .collect();
    assert_eq!(now, vec![3, 4]);
    // Ctrl and A: every row, not the rows of the pool.
    holding(&mut app, KeyCode::ControlLeft, |app| {
        tap(app, KeyCode::KeyA)
    });
    assert_eq!(
        app.world().get::<TableSelection>(table).expect("s").len(),
        100
    );
    // Escape from a click on one row: it alone.
    click_row(&mut app, rows[0]);
    assert_eq!(
        app.world().get::<TableSelection>(table).expect("s").len(),
        1
    );
}

#[test]
fn the_selection_stays_with_its_row_when_the_table_is_sorted() {
    let mut app = app();
    let table = spawn_items(&mut app, 300);
    app.world_mut()
        .get_mut::<TableSelection>(table)
        .expect("s")
        .0
        .insert(7);
    *app.world_mut().get_mut::<TableSort>(table).expect("s") = TableSort {
        column: Some(0),
        descending: false,
    };
    settle(&mut app);
    // "Item 7" is the eighth in natural order: it shows selected there.
    let lit: Vec<String> = pool(&mut app)
        .into_iter()
        .filter(|(_, e)| app.world().get::<bevy::ui::Selected>(*e).is_some())
        .map(|(_, e)| cell(&app, e, 0))
        .collect();
    assert_eq!(lit, vec!["Item 7".to_string()]);
}

#[test]
fn space_selects_the_cursor_row_and_a_none_table_selects_nothing() {
    let mut app = app();
    let table = spawn(&mut app, items(20), SelectMode::None);
    focus(&mut app, table);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::Space);
    assert!(app.world().resource::<Log>().selections.is_empty());
    assert_eq!(app.world().get::<TableCursor>(table).expect("c").0, Some(1));
    app.world_mut().entity_mut(table).insert(SelectMode::Multi);
    tap(&mut app, KeyCode::Space);
    assert_eq!(app.world().resource::<Log>().selections, vec![vec![1]]);
    tap(&mut app, KeyCode::Space);
    assert_eq!(
        app.world().resource::<Log>().selections.last(),
        Some(&Vec::new()),
        "Space again takes it out"
    );
}

// Columns.

#[test]
fn dragging_a_column_edge_widens_the_column_by_the_distance() {
    let mut app = app();
    let table = spawn_items(&mut app, 100);
    let heads = with_role(&mut app, Role::TableHeaderCell);
    let before = rect(&app, heads[0]).width();
    let edge = with_role(&mut app, Role::TableEdge)[0];
    let from = rect(&app, edge).center();
    point_at(&mut app, from);
    pointer(&mut app, from, PointerAction::Press(PointerButton::Primary));
    let to = from + Vec2::new(40.0, 0.0);
    pointer(&mut app, to, PointerAction::Move { delta: to - from });
    settle(&mut app);
    pointer(&mut app, to, PointerAction::Release(PointerButton::Primary));
    settle(&mut app);
    let heads = with_role(&mut app, Role::TableHeaderCell);
    assert!(
        (rect(&app, heads[0]).width() - before - 40.0).abs() < 1.5,
        "{} to {}",
        before,
        rect(&app, heads[0]).width()
    );
    // The player's width is data a game can save.
    let widths = app.world().get::<TableWidths>(table).expect("w");
    assert!(widths.0[0] > 12.0);
    // The drag was not a click on the header: no sort.
    assert!(app.world().resource::<Log>().sorts.is_empty());
    // The rows follow the header: a cell is as wide as its column.
    let row = pool(&mut app)[0].1;
    let cells = app.world().get::<Children>(row).expect("cells");
    assert!((rect(&app, cells[0]).width() - rect(&app, heads[0]).width()).abs() < 1.5);
}

#[test]
fn ctrl_and_an_arrow_on_a_header_size_its_column_and_move_no_focus() {
    let mut app = app();
    let table = spawn_items(&mut app, 100);
    let heads = with_role(&mut app, Role::TableHeaderCell);
    focus(&mut app, heads[0]);
    let before = rect(&app, heads[0]).width();
    holding(&mut app, KeyCode::ControlLeft, |app| {
        tap(app, KeyCode::ArrowRight);
        tap(app, KeyCode::ArrowRight);
    });
    settle(&mut app);
    assert_eq!(focused(&app), Some(heads[0]));
    assert!((rect(&app, heads[0]).width() - before - 32.0).abs() < 1.5);
    // Never under the least: three text sizes.
    holding(&mut app, KeyCode::ControlLeft, |app| {
        for _ in 0..40 {
            tap(app, KeyCode::ArrowLeft);
        }
    });
    settle(&mut app);
    let skin = app.world().resource::<GooeySkin>();
    let least = TABLE_LEAST_COLUMN * skin.text_size * skin.text_scale;
    assert!(rect(&app, heads[0]).width() >= least - 1.0);
    assert_eq!(
        app.world().get::<TableWidths>(table).expect("w").0[0],
        TABLE_LEAST_COLUMN
    );
    // Without Ctrl, Right goes to the next header as any button's does.
    tap(&mut app, KeyCode::ArrowRight);
    assert_ne!(focused(&app), Some(heads[0]));
}

#[test]
fn a_header_is_a_target_high_and_text_follows_the_players_size() {
    let mut app = app();
    spawn_items(&mut app, 100);
    let heads = with_role(&mut app, Role::TableHeaderCell);
    assert!(rect(&app, heads[0]).height() >= 44.0 - 0.5);
    let row = pool(&mut app)[0].1;
    assert!((rect(&app, row).height() - pitch(&app)).abs() < 1.0);
    let before = rect(&app, heads[0]).width();
    app.world_mut().resource_mut::<GooeySkin>().text_scale = 1.5;
    settle(&mut app);
    let heads = with_role(&mut app, Role::TableHeaderCell);
    assert!(
        (rect(&app, heads[0]).width() - before * 1.5).abs() < 2.0,
        "a column is as wide in text sizes at every text size"
    );
}

#[test]
fn every_cell_is_one_line_cut_at_its_column() {
    let mut app = app();
    let long = vec![vec![
        "A name much longer than its column will ever hold in one line".to_string(),
        "1".to_string(),
        String::new(),
    ]];
    spawn(&mut app, long, ());
    let row = pool(&mut app)[0].1;
    let cells = app.world().get::<Children>(row).expect("cells");
    let text = app.world().get::<Children>(cells[0]).expect("text")[0];
    assert!(
        rect(&app, text).height() < pitch(&app),
        "the long name stays on one line"
    );
    let first = rect(&app, cells[0]);
    assert!((rect(&app, row).height() - pitch(&app)).abs() < 1.0);
    assert!(first.width() < 400.0);
}

// Looks.

fn rgb(color: Color) -> [f32; 3] {
    let c = color.to_srgba();
    [c.red, c.green, c.blue]
}

/// The text of `fg` against the fill `bg` seen over `under`.
fn reading(app: &App, fg: Entity, bg: Color, under: Color) -> f32 {
    let text = app.world().get::<TextColor>(fg).expect("coloured").0;
    let behind = composite(rgb(bg), bg.alpha(), rgb(under));
    contrast_ratio(rgb(text), behind)
}

#[test]
fn the_words_of_every_row_state_read_in_every_look() {
    for preset in SkinPreset::ALL {
        let mut app = app();
        *app.world_mut().resource_mut::<GooeySkin>() = preset.skin();
        let table = spawn(&mut app, items(30), SelectMode::Multi);
        app.world_mut()
            .get_mut::<TableSelection>(table)
            .expect("s")
            .0
            .insert(2);
        settle(&mut app);
        let skin = app.world().resource::<GooeySkin>().clone();
        let shown = pool(&mut app);
        for (row, entity) in shown.iter().take(6) {
            let fill = app.world().get::<BackgroundColor>(*entity).expect("fill").0;
            let cells = app.world().get::<Children>(*entity).expect("cells");
            let text = app.world().get::<Children>(cells[0]).expect("text")[0];
            let ratio = reading(&app, text, fill, skin.surface);
            assert!(
                ratio >= TEXT_CONTRAST,
                "{}: row {} reads at {ratio:.2}:1",
                preset.name(),
                row.index
            );
        }
    }
}

#[test]
fn the_ring_is_on_the_cursor_row_only_while_the_table_has_visible_focus() {
    let mut app = app();
    let table = spawn_items(&mut app, 20);
    let ring = app.world().resource::<GooeySkin>().focus_ring;
    let border_of = |app: &mut App, index: usize| {
        let row = pool(app)[index].1;
        app.world().get::<BorderColor>(row).expect("border").top
    };
    focus(&mut app, table);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(border_of(&mut app, 1), ring);
    assert_ne!(border_of(&mut app, 0), ring);
    // A pointer press hides the ring everywhere.
    app.world_mut().insert_resource(InputFocusVisible(false));
    settle(&mut app);
    assert_ne!(border_of(&mut app, 1), ring);
}

#[test]
fn a_disabled_table_takes_no_keys_and_no_clicks() {
    let mut app = app();
    let table = spawn(&mut app, items(20), bevy::ui::InteractionDisabled);
    focus(&mut app, table);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(app.world().get::<TableCursor>(table).expect("c").0, None);
    let row = pool(&mut app)[2].1;
    click_row(&mut app, row);
    assert!(app.world().resource::<Log>().selections.is_empty());
}

#[test]
fn a_table_at_rest_writes_nothing() {
    let mut app = app();
    spawn_items(&mut app, 10_000);
    settle(&mut app);
    let before = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, before);
}

#[test]
fn a_pad_walks_a_table_and_opens_a_row() {
    let mut app = app();
    let table = spawn_items(&mut app, 50);
    focus(&mut app, table);
    let pad = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(GamepadConnectionEvent::new(
        pad,
        GamepadConnection::Connected {
            name: "Test pad".into(),
            vendor_id: None,
            product_id: None,
        },
    ));
    settle(&mut app);
    let press = |app: &mut App, button: GamepadButton, value: f32| {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    };
    press(&mut app, GamepadButton::DPadDown, 1.0);
    press(&mut app, GamepadButton::DPadDown, 0.0);
    press(&mut app, GamepadButton::DPadDown, 1.0);
    press(&mut app, GamepadButton::DPadDown, 0.0);
    assert_eq!(app.world().get::<TableCursor>(table).expect("c").0, Some(1));
    assert_eq!(focused(&app), Some(table));
    press(&mut app, GamepadButton::South, 1.0);
    press(&mut app, GamepadButton::South, 0.0);
    assert_eq!(app.world().resource::<Log>().opened, vec![1]);
}

#[test]
fn at_every_text_size_the_titles_fit_their_headers_and_the_rows_their_table() {
    let mut app = app();
    let table = spawn_items(&mut app, 200);
    for step in TEXT_SIZE_STEPS {
        app.world_mut().resource_mut::<GooeySkin>().text_scale = step;
        settle(&mut app);
        let root = rect(&app, table);
        for head in with_role(&mut app, Role::TableHeaderCell) {
            let head_rect = rect(&app, head);
            let title = app.world().get::<Children>(head).expect("title")[0];
            let text = rect(&app, title);
            assert!(
                text.min.x >= head_rect.min.x - 1.0 && text.max.x <= head_rect.max.x + 1.0,
                "{step}: a title leaves its header ({text:?} in {head_rect:?})"
            );
            assert!(head_rect.height() >= 44.0 - 0.5, "{step}: header height");
        }
        for (_, row) in pool(&mut app) {
            let row_rect = rect(&app, row);
            assert!(
                row_rect.min.x >= root.min.x - 1.0 && row_rect.max.x <= root.max.x + 1.0,
                "{step}: a row leaves the table"
            );
            assert!(
                (row_rect.height() - pitch(&app)).abs() < 1.0,
                "{step}: a row is one pitch"
            );
        }
    }
}
