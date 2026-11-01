//! The list: rows that scroll, with a rail that appears only when the rows do not fit, a focus that
//! the list follows with the next row in sight, a wheel and a drag that scroll it, and paging. Laid
//! out for real in a headless window, driven by real keyboard, pointer and gamepad input.

use bevy::camera::NormalizedRenderTarget;
use bevy::ecs::spawn::SpawnIter;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::touch::TouchPhase;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{CalculatedClip, ComputedNode, ScrollPosition, UiGlobalTransform};
use bevy::ui_widgets::Scrollbar;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// How many frames' writes to a scroll position and a rail's node the app has seen.
#[derive(Resource, Default)]
struct Writes {
    scroll: u32,
    rail: u32,
}

/// A headless app with a `width` × `height` window, so UI layout and picking run.
fn app_sized(width: u32, height: u32) -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(width, height)
                        .with_scale_factor_override(1.0),
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
    .init_resource::<Writes>()
    .add_systems(
        Update,
        |scrolls: Query<(), Changed<ScrollPosition>>,
         rails: Query<(), (Changed<Node>, With<Scrollbar>)>,
         mut writes: ResMut<Writes>| {
            writes.scroll += scrolls.iter().count() as u32;
            writes.rail += rails.iter().count() as u32;
        },
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn app() -> App {
    app_sized(1280, 720)
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// A list of `n` buttons, 4.5 rows tall, and a Close button under it, in a panel on a screen.
struct Saves {
    root: Entity,
    view: Entity,
    rail: Entity,
    thumb: Entity,
    rows: Vec<Entity>,
    close: Entity,
}

fn saves(app: &mut App, n: usize) -> Saves {
    let world = app.world_mut();
    let root = world
        .spawn(list(
            4.5,
            Children::spawn(SpawnIter((0..n).map(|i| button(format!("Slot {i:02}"))))),
        ))
        .id();
    let close = world.spawn(button("Close")).id();
    let card = world.spawn(panel()).add_children(&[root, close]).id();
    world.spawn(screen()).add_child(card);
    settle(app);
    found(app, root, close)
}

fn found(app: &App, root: Entity, close: Entity) -> Saves {
    let view = part_with::<ListView>(app, root);
    let rows = app
        .world()
        .get::<Children>(part(app, root, Role::ListRows))
        .expect("rows")
        .iter()
        .collect();
    Saves {
        root,
        view,
        rail: part(app, root, Role::ListRail),
        thumb: part(app, root, Role::ListThumb),
        rows,
        close,
    }
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            for child in children.iter() {
                found.push(child);
                stack.push(child);
            }
        }
    }
    found
}

fn part(app: &App, root: Entity, role: Role) -> Entity {
    descendants(app.world(), root)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&role))
        .unwrap_or_else(|| panic!("a {role:?} under the widget"))
}

fn part_with<C: Component>(app: &App, root: Entity) -> Entity {
    descendants(app.world(), root)
        .into_iter()
        .find(|&e| app.world().get::<C>(e).is_some())
        .expect("the part under the widget")
}

/// Where `entity` is drawn, in logical px.
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

fn scroll_of(app: &App, view: Entity) -> f32 {
    app.world().get::<ScrollPosition>(view).expect("scroll").y
}

/// The furthest the list can scroll, in logical px.
fn max_scroll(app: &App, view: Entity) -> f32 {
    let node = app.world().get::<ComputedNode>(view).expect("laid out");
    (node.content_size().y - node.size().y).max(0.0) * node.inverse_scale_factor()
}

fn skin() -> GooeySkin {
    GooeySkin::default()
}

fn fill_of(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

fn rail_shown(app: &App, rail: Entity) -> bool {
    app.world().get::<Node>(rail).expect("a node").display == Display::Flex
}

/// Whether `row` is whole inside the list's viewport, to half a pixel.
fn whole_inside(app: &App, view: Entity, row: Entity) -> bool {
    let (view, row) = (rect(app, view), rect(app, row));
    row.min.y >= view.min.y - 0.5 && row.max.y <= view.max.y + 0.5
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn tap(app: &mut App, key_code: KeyCode) {
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
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

fn point_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
}

fn click_at(app: &mut App, at: Vec2) {
    point_at(app, at);
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
}

/// One notch of the wheel over `at`, `lines` lines down.
fn wheel(app: &mut App, at: Vec2, lines: f32) {
    point_at(app, at);
    pointer(
        app,
        at,
        PointerAction::Scroll {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -lines,
            phase: TouchPhase::Moved,
        },
    );
    settle(app);
}

fn plug_in_pad(app: &mut App) -> Entity {
    let pad = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(GamepadConnectionEvent::new(
        pad,
        GamepadConnection::Connected {
            name: "Test pad".into(),
            vendor_id: None,
            product_id: None,
        },
    ));
    app.update();
    pad
}

fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
}

/// Sets focus the way a key would have, and shows it.
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

fn index_of(saves: &Saves, entity: Option<Entity>) -> Option<usize> {
    saves.rows.iter().position(|&r| Some(r) == entity)
}

// ---- the shape ----

#[test]
fn a_list_stands_rows_tall_and_cuts_the_fifth_row() {
    for (width, height, text_scale) in [(1280, 720, 1.0), (2560, 1440, 1.0), (1280, 720, 1.5)] {
        let mut app = app_sized(width, height);
        app.world_mut().insert_resource(GooeySkin {
            text_scale,
            ..skin()
        });
        let ui = saves(&mut app, 12);
        let row = rect(&app, ui.rows[0]).height();
        let view = rect(&app, ui.view);
        let want = list_height(4.5, row, skin().gap);
        assert!(
            (view.height() - want).abs() < 2.0,
            "{width}x{height} at {text_scale}x text: the list is {} high, 4.5 rows of {row} and a gap are {want}",
            view.height()
        );
        let fifth = rect(&app, ui.rows[4]);
        assert!(
            fifth.min.y < view.max.y - 4.0 && fifth.max.y > view.max.y + 4.0,
            "the fifth row is cut by the list's edge: row {fifth:?}, list {view:?}"
        );
        assert!(
            whole_inside(&app, ui.view, ui.rows[3]),
            "the fourth row is whole"
        );
    }
}

#[test]
fn rows_keep_a_gap_of_air_from_the_rail_whether_or_not_it_shows() {
    let mut app = app();
    let long = saves(&mut app, 12);
    let rail = rect(&app, long.rail);
    let row = rect(&app, long.rows[0]);
    assert!(rail_shown(&app, long.rail));
    assert!(
        (rail.min.x - row.max.x - skin().gap).abs() < 1.0,
        "a gap between the rows ({row:?}) and the rail ({rail:?})"
    );
    assert!(
        (rail.width() - 2.0 * skin().border).abs() < 0.01,
        "the rail is two lines wide: {}",
        rail.width()
    );
    // A list that fits has the same lane: its rows are as wide.
    let mut short_app = self::app();
    let short = saves(&mut short_app, 3);
    assert!(!rail_shown(&short_app, short.rail));
    assert!((rect(&short_app, short.rows[0]).width() - row.width()).abs() < 0.5);
}

// ---- the rail ----

#[test]
fn the_rail_shows_only_while_the_rows_do_not_fit() {
    let mut app = app();
    let ui = saves(&mut app, 3);
    assert!(!rail_shown(&app, ui.rail), "three rows fit");
    // A fourth and a fifth row still fit 4.5 rows' room only up to the fourth.
    let content = part(&app, ui.root, Role::ListRows);
    for i in 3..12 {
        let row = app.world_mut().spawn(button(format!("Slot {i:02}"))).id();
        app.world_mut().entity_mut(content).add_child(row);
    }
    settle(&mut app);
    assert!(rail_shown(&app, ui.rail), "twelve rows do not fit");
    // Taking rows away hides it again.
    let rows: Vec<Entity> = app
        .world()
        .get::<Children>(content)
        .expect("rows")
        .iter()
        .skip(2)
        .collect();
    for row in rows {
        app.world_mut().entity_mut(row).despawn();
    }
    settle(&mut app);
    assert!(!rail_shown(&app, ui.rail), "two rows fit");
}

#[test]
fn the_thumb_is_the_visible_share_of_the_track_and_sits_where_the_list_is() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    let node = app.world().get::<ComputedNode>(ui.view).expect("laid out");
    let (content, visible) = (
        node.content_size().y * node.inverse_scale_factor(),
        node.size().y * node.inverse_scale_factor(),
    );
    let (rail, thumb) = (rect(&app, ui.rail), rect(&app, ui.thumb));
    assert!(
        (thumb.height() - rail.height() * visible / content).abs() < 1.5,
        "a thumb {} of a track {} for {visible} of {content}",
        thumb.height(),
        rail.height()
    );
    assert!((thumb.min.y - rail.min.y).abs() < 1.0, "at the top at rest");
    assert!(
        (thumb.width() - rail.width()).abs() < 0.5,
        "as wide as the rail"
    );
    // Scrolled to the end, it is at the end.
    let max = max_scroll(&app, ui.view);
    app.world_mut()
        .get_mut::<ScrollPosition>(ui.view)
        .expect("scroll")
        .y = max;
    settle(&mut app);
    let (rail, thumb) = (rect(&app, ui.rail), rect(&app, ui.thumb));
    assert!(
        (thumb.max.y - rail.max.y).abs() < 1.5,
        "at the bottom at the end"
    );
}

#[test]
fn the_thumb_is_the_muted_grey_the_pointer_brightens_and_a_drag_lights() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    let skin = skin();
    assert_eq!(fill_of(&app, ui.rail), skin.surface_raised);
    assert_eq!(fill_of(&app, ui.thumb), skin.text_muted);
    let centre = rect(&app, ui.thumb).center();
    point_at(&mut app, centre);
    settle(&mut app);
    assert!(app.world().get::<Hovered>(ui.thumb).expect("hover").0);
    assert_eq!(fill_of(&app, ui.thumb), skin.text, "under the pointer");
    // Drag it down 40 px: the list scrolls by the share of its length that is.
    pointer(
        &mut app,
        centre,
        PointerAction::Press(PointerButton::Primary),
    );
    let to = centre + Vec2::new(0.0, 40.0);
    pointer(
        &mut app,
        to,
        PointerAction::Move {
            delta: Vec2::new(0.0, 40.0),
        },
    );
    settle(&mut app);
    assert_eq!(fill_of(&app, ui.thumb), skin.accent, "while dragged");
    let (rail, thumb) = (rect(&app, ui.rail), rect(&app, ui.thumb));
    let travel = rail.height() - thumb.height();
    let want = 40.0 * max_scroll(&app, ui.view) / travel;
    assert!(
        (scroll_of(&app, ui.view) - want).abs() < 3.0,
        "dragged 40 px of {travel}: scrolled {} of {want}",
        scroll_of(&app, ui.view)
    );
    pointer(&mut app, to, PointerAction::Release(PointerButton::Primary));
    settle(&mut app);
    assert_eq!(
        fill_of(&app, ui.thumb),
        skin.text,
        "released, still under it"
    );
}

// ---- the wheel ----

#[test]
fn the_wheel_scrolls_the_list_and_stops_at_both_ends() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    let over = rect(&app, ui.rows[1]).center();
    assert_eq!(scroll_of(&app, ui.view), 0.0);
    wheel(&mut app, over, 1.0);
    let one = scroll_of(&app, ui.view);
    assert!(one > 40.0 && one < 200.0, "a notch scrolls {one}");
    for _ in 0..12 {
        wheel(&mut app, over, 1.0);
    }
    assert!((scroll_of(&app, ui.view) - max_scroll(&app, ui.view)).abs() < 1.0);
    for _ in 0..14 {
        wheel(&mut app, over, -1.0);
    }
    assert_eq!(scroll_of(&app, ui.view), 0.0);
    // The last row ends flush with the list at the end.
    for _ in 0..14 {
        wheel(&mut app, over, 1.0);
    }
    let last = rect(&app, *ui.rows.last().expect("rows"));
    assert!(
        (last.max.y - rect(&app, ui.view).max.y).abs() < 1.5,
        "flush at the end"
    );
}

// ---- focus ----

#[test]
fn walking_down_keeps_the_row_and_the_next_one_whole_and_the_ring_complete() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    focus(&mut app, ui.rows[0]);
    let mut last_scroll = scroll_of(&app, ui.view);
    let mut scrolled_by = Vec::new();
    for k in 1..12 {
        tap(&mut app, KeyCode::ArrowDown);
        settle(&mut app);
        assert_eq!(focused(&app), Some(ui.rows[k]), "Down {k}");
        assert!(
            whole_inside(&app, ui.view, ui.rows[k]),
            "focused row {k} whole: {:?} in {:?}",
            rect(&app, ui.rows[k]),
            rect(&app, ui.view)
        );
        if k < 11 {
            assert!(
                whole_inside(&app, ui.view, ui.rows[k + 1])
                    || scroll_of(&app, ui.view) >= max_scroll(&app, ui.view) - 1.0,
                "the next row whole too, at step {k}"
            );
        }
        // Away from the end the list stands on a row's top, as at rest, so the row it cuts at the
        // bottom is cut by the same amount at every step: more than a third of it shows.
        if (3..=8).contains(&k) {
            let view = rect(&app, ui.view);
            assert!(
                ui.rows
                    .iter()
                    .any(|&row| (rect(&app, row).min.y - view.min.y).abs() < 1.5),
                "a row's top is flush with the list's top at step {k}"
            );
            let cut = rect(&app, ui.rows[k + 2]);
            let shown = view.max.y - cut.min.y;
            assert!(
                shown > cut.height() * 0.33 && shown < cut.height() * 0.8,
                "the row after the next is cut, {shown} of {} showing, at step {k}",
                cut.height()
            );
        }
        let now = scroll_of(&app, ui.view);
        scrolled_by.push(now - last_scroll);
        last_scroll = now;
    }
    assert!(
        (scroll_of(&app, ui.view) - max_scroll(&app, ui.view)).abs() < 1.0,
        "at the end the list ends on the last row"
    );
    // The first rows need no scroll; from the fourth on the list moves a pitch (a row and a gap) at
    // a step.
    assert_eq!(scrolled_by[0], 0.0);
    assert_eq!(scrolled_by[1], 0.0);
    let pitch = rect(&app, ui.rows[1]).min.y - rect(&app, ui.rows[0]).min.y;
    for (i, by) in scrolled_by.iter().enumerate().take(8).skip(2) {
        assert!(
            (by - pitch).abs() < 3.0,
            "step {} moves a row's pitch ({pitch}), not {by}",
            i + 1
        );
    }
    // The ring is whole on the focused row: its border is 3 px on every side and the row is inside.
    let node = app
        .world()
        .get::<ComputedNode>(ui.rows[11])
        .expect("laid out");
    assert_eq!(node.border.min_inset, Vec2::splat(3.0));
    assert_eq!(node.border.max_inset, Vec2::splat(3.0));
}

#[test]
fn walking_back_up_does_the_same_in_reverse() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    focus(&mut app, ui.rows[11]);
    assert!((scroll_of(&app, ui.view) - max_scroll(&app, ui.view)).abs() < 1.0);
    for k in (0..11).rev() {
        tap(&mut app, KeyCode::ArrowUp);
        settle(&mut app);
        assert_eq!(focused(&app), Some(ui.rows[k]), "Up to {k}");
        assert!(whole_inside(&app, ui.view, ui.rows[k]), "row {k} whole");
        if k > 0 {
            assert!(
                whole_inside(&app, ui.view, ui.rows[k - 1]) || scroll_of(&app, ui.view) <= 1.0,
                "the row above whole too, at {k}"
            );
        }
    }
    assert_eq!(scroll_of(&app, ui.view), 0.0, "back at the top");
}

#[test]
fn tab_walks_the_list_like_the_arrows_and_leaves_it_for_the_button_below() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    focus(&mut app, ui.rows[0]);
    for k in 1..12 {
        tap(&mut app, KeyCode::Tab);
        settle(&mut app);
        assert_eq!(focused(&app), Some(ui.rows[k]), "Tab {k}");
        assert!(whole_inside(&app, ui.view, ui.rows[k]), "row {k} whole");
    }
    tap(&mut app, KeyCode::Tab);
    settle(&mut app);
    assert_eq!(focused(&app), Some(ui.close));
}

#[test]
fn the_pad_walks_it_too() {
    let mut app = app();
    let pad = plug_in_pad(&mut app);
    let ui = saves(&mut app, 12);
    focus(&mut app, ui.rows[0]);
    for k in 1..8 {
        press(&mut app, pad, GamepadButton::DPadDown);
        settle(&mut app);
        assert_eq!(focused(&app), Some(ui.rows[k]), "D-pad down {k}");
        assert!(whole_inside(&app, ui.view, ui.rows[k]), "row {k} whole");
    }
    assert!(scroll_of(&app, ui.view) > 0.0);
}

#[test]
fn a_click_on_a_half_visible_row_focuses_it_where_it_stands() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    let (row, view) = (rect(&app, ui.rows[4]), rect(&app, ui.view));
    assert!(row.max.y > view.max.y, "the fifth row is cut");
    let seen = Vec2::new(row.center().x, (row.min.y + view.max.y) / 2.0);
    click_at(&mut app, seen);
    assert_eq!(focused(&app), Some(ui.rows[4]));
    assert!(
        !app.world().resource::<InputFocusVisible>().0,
        "a click hides the ring"
    );
    assert_eq!(
        scroll_of(&app, ui.view),
        0.0,
        "nothing scrolled under the pointer"
    );
    // The next key press brings it in.
    tap(&mut app, KeyCode::ArrowDown);
    settle(&mut app);
    assert_eq!(focused(&app), Some(ui.rows[5]));
    assert!(whole_inside(&app, ui.view, ui.rows[5]));
    assert!(whole_inside(&app, ui.view, ui.rows[6]));
}

#[test]
fn focus_set_from_code_to_a_far_row_centres_it() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    focus(&mut app, ui.rows[0]);
    assert_eq!(scroll_of(&app, ui.view), 0.0, "the first row, the top");
    focus(&mut app, ui.rows[9]);
    let (row, view) = (rect(&app, ui.rows[9]), rect(&app, ui.view));
    assert!(
        (row.center().y - view.center().y).abs() < 1.5,
        "a far row is centred: row {row:?} in {view:?}"
    );
    // A near one is not: it is only brought in, with its context.
    focus(&mut app, ui.rows[0]);
    focus(&mut app, ui.rows[4]);
    assert!(whole_inside(&app, ui.view, ui.rows[4]));
    assert!(
        whole_inside(&app, ui.view, ui.rows[5]),
        "the next row whole"
    );
    let (row, view) = (rect(&app, ui.rows[4]), rect(&app, ui.view));
    assert!(row.center().y < view.center().y + 40.0 && row.center().y > view.center().y - 80.0);
}

#[test]
fn a_focus_the_pointer_hid_is_not_followed() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    app.world_mut().insert_resource(InputFocusVisible(false));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(ui.rows[9], FocusCause::Navigated);
    settle(&mut app);
    assert_eq!(
        scroll_of(&app, ui.view),
        0.0,
        "an unseen focus scrolls nothing"
    );
}

// ---- neighbours ----

/// The list beside a button that stands lower than the visible rows: the layout in which a row the
/// list has clipped away lies nearer to the button than any row in view.
fn beside_a_low_button(app: &mut App) -> (Saves, Entity) {
    let world = app.world_mut();
    let root = world
        .spawn(list(
            4.5,
            Children::spawn(SpawnIter((0..12).map(|i| button(format!("Slot {i:02}"))))),
        ))
        .id();
    let side = world.spawn(button("Side")).id();
    let low = world
        .spawn(Node {
            margin: UiRect::top(Val::Px(330.0)),
            ..default()
        })
        .add_child(side)
        .id();
    let row = world
        .spawn(Node {
            column_gap: Val::Px(20.0),
            ..default()
        })
        .add_children(&[root, low])
        .id();
    let card = world.spawn(panel()).add_child(row).id();
    world.spawn(screen()).add_child(card);
    settle(app);
    let ui = found(app, root, side);
    (ui, side)
}

#[test]
fn a_row_the_list_has_clipped_away_is_not_a_neighbour_from_outside() {
    let mut app = app();
    let (ui, side) = beside_a_low_button(&mut app);
    let (button_at, view) = (rect(&app, side), rect(&app, ui.view));
    assert!(
        button_at.center().y > view.max.y,
        "the button stands below the visible rows: {button_at:?} / {view:?}"
    );
    focus(&mut app, side);
    tap(&mut app, KeyCode::ArrowLeft);
    settle(&mut app);
    let to = index_of(&ui, focused(&app)).expect("focus went to a row");
    assert!(
        to <= 4,
        "the nearest row in view, not a clipped one: row {to}"
    );
    assert_eq!(
        to, 4,
        "the nearest to the button's height that has any part in view"
    );
    // From inside the list, Right goes to the button.
    tap(&mut app, KeyCode::ArrowRight);
    settle(&mut app);
    assert_eq!(focused(&app), Some(side));
}

// ---- paging ----

#[test]
fn page_down_and_up_move_a_viewport_and_stop_at_the_ends() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    focus(&mut app, ui.rows[0]);
    tap(&mut app, KeyCode::PageDown);
    settle(&mut app);
    let first = index_of(&ui, focused(&app)).expect("a row");
    assert!(
        (3..=4).contains(&first),
        "a page is about the rows in view: row {first}"
    );
    assert!(whole_inside(&app, ui.view, ui.rows[first]));
    tap(&mut app, KeyCode::PageDown);
    settle(&mut app);
    let second = index_of(&ui, focused(&app)).expect("a row");
    assert!(second > first + 2, "and again: {second}");
    for _ in 0..4 {
        tap(&mut app, KeyCode::PageDown);
        settle(&mut app);
    }
    assert_eq!(
        focused(&app),
        Some(ui.rows[11]),
        "the last row, and no further"
    );
    assert!((scroll_of(&app, ui.view) - max_scroll(&app, ui.view)).abs() < 1.0);
    for _ in 0..5 {
        tap(&mut app, KeyCode::PageUp);
        settle(&mut app);
    }
    assert_eq!(focused(&app), Some(ui.rows[0]), "back to the first row");
    assert_eq!(scroll_of(&app, ui.view), 0.0);
}

#[test]
fn the_pads_triggers_page_like_the_keys() {
    let mut app = app();
    let pad = plug_in_pad(&mut app);
    let ui = saves(&mut app, 12);
    focus(&mut app, ui.rows[0]);
    press(&mut app, pad, GamepadButton::RightTrigger2);
    settle(&mut app);
    let down = index_of(&ui, focused(&app)).expect("a row");
    assert!((3..=4).contains(&down), "row {down}");
    press(&mut app, pad, GamepadButton::LeftTrigger2);
    settle(&mut app);
    assert_eq!(focused(&app), Some(ui.rows[0]));
}

#[test]
fn paging_does_nothing_outside_a_list() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    focus(&mut app, ui.close);
    tap(&mut app, KeyCode::PageUp);
    settle(&mut app);
    assert_eq!(focused(&app), Some(ui.close));
}

// ---- cost and lines ----

#[test]
fn an_idle_list_writes_nothing() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    focus(&mut app, ui.rows[2]);
    for _ in 0..4 {
        app.update();
    }
    app.world_mut().insert_resource(Writes::default());
    for _ in 0..30 {
        app.update();
    }
    let writes = app.world().resource::<Writes>();
    assert_eq!(writes.scroll, 0, "no scroll position was written");
    assert_eq!(writes.rail, 0, "the rail's node was not written");
    let _ = ui;
}

#[test]
fn the_rail_is_whole_device_pixels_at_every_scale() {
    for (width, height) in [(960, 540), (1280, 720), (1920, 1080), (2560, 1440)] {
        let mut app = app_sized(width, height);
        let ui = saves(&mut app, 12);
        let node = app.world().get::<ComputedNode>(ui.rail).expect("laid out");
        let scale = node.inverse_scale_factor().recip();
        let line = snap_line_width(skin().border, scale);
        assert_eq!(
            node.size().x,
            node.size().x.round(),
            "{width}x{height}: whole"
        );
        assert!(
            (node.size().x - 2.0 * line * scale).abs() < 0.01,
            "{width}x{height}: {} device px is two lines of {line}",
            node.size().x
        );
    }
}

#[test]
fn changing_rows_resizes_the_list() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    let before = rect(&app, ui.view).height();
    app.world_mut()
        .get_mut::<ListView>(ui.view)
        .expect("a list view")
        .rows = 3.0;
    settle(&mut app);
    let after = rect(&app, ui.view).height();
    let row = rect(&app, ui.rows[0]).height();
    assert!(after < before);
    assert!((after - list_height(3.0, row, skin().gap)).abs() < 1.0);
}

// ---- a tooltip on a row ----

#[test]
fn a_tooltip_on_a_row_is_not_clipped_by_its_list() {
    let mut app = app();
    let world = app.world_mut();
    let root = world
        .spawn(list(
            4.5,
            Children::spawn(SpawnIter((0..8).map(|i| {
                (
                    button(format!("Slot {i:02}")),
                    Tooltip::new("The harbour at dusk, 9 h 15 min played"),
                )
            }))),
        ))
        .id();
    let close = world.spawn(button("Close")).id();
    let card = world.spawn(panel()).add_children(&[root, close]).id();
    world.spawn(screen()).add_child(card);
    settle(&mut app);
    let ui = found(&app, root, close);
    focus(&mut app, ui.rows[1]);
    settle(&mut app);
    let tip = descendants(app.world(), ui.rows[1])
        .into_iter()
        .find(|&e| app.world().get::<TooltipBox>(e).is_some())
        .expect("the tooltip's box");
    let (tip_at, view) = (rect(&app, tip), rect(&app, ui.view));
    assert!(
        tip_at.width() > 40.0 && tip_at.max.x > view.max.x + 20.0,
        "the box opens beside the row, outside the list: {tip_at:?} / {view:?}"
    );
    assert!(
        app.world().get::<CalculatedClip>(tip).is_none(),
        "the list's clip does not reach the tooltip"
    );
}

#[test]
fn a_click_on_the_rail_pages_by_a_viewport() {
    let mut app = app();
    let ui = saves(&mut app, 12);
    let (rail, view) = (rect(&app, ui.rail), rect(&app, ui.view));
    let below = Vec2::new(rail.center().x, rail.max.y - 10.0);
    click_at(&mut app, below);
    let scrolled = scroll_of(&app, ui.view);
    assert!(
        (scrolled - view.height()).abs() < 2.0,
        "a click below the thumb scrolls a viewport ({}), not {scrolled}",
        view.height()
    );
    let above = Vec2::new(rail.center().x, rail.min.y + 5.0);
    click_at(&mut app, above);
    assert_eq!(
        scroll_of(&app, ui.view),
        0.0,
        "and a click above pages back"
    );
    assert!(
        focused(&app).is_none_or(|f| index_of(&ui, Some(f)).is_none()),
        "a click on the rail focuses no row"
    );
}
