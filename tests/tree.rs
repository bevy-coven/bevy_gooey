//! The tree: nodes that open and close inside the table's pool, ten thousand of them in a handful
//! of entities, with Left and Right, type-ahead, a chevron to click, a selection that keeps its
//! node when others come and go. Laid out for real in a headless window, driven by real pointer,
//! keyboard and gamepad input.

use std::time::Duration;

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
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, ScrollPosition, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

#[derive(Resource, Default)]
struct Log {
    toggled: Vec<(u64, bool)>,
    selections: Vec<Vec<u64>>,
    opened: Vec<u64>,
}

/// How many `Node` writes the app has seen on styled entities.
#[derive(Resource, Default)]
struct Writes(u32);

fn app() -> App {
    let mut app = App::new();
    // Assumed owner exception (#76): colours ease on the real clock, so this app reads them landed.
    app.insert_resource(MotionSettings::off());
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
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )))
    .init_resource::<Log>()
    .init_resource::<Writes>()
    .add_observer(|toggled: On<TreeToggled>, mut log: ResMut<Log>| {
        log.toggled.push((toggled.key, toggled.open));
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

fn click(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
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

/// A character the player types: a key that produces `c`.
fn type_char(app: &mut App, key_code: KeyCode, c: &str) {
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Character(c.into()),
            state,
            text: Some(c.into()),
            repeat: false,
            window,
        });
        app.update();
    }
}

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

/// The keys of the rows the pool shows, top to bottom.
fn keys(app: &mut App) -> Vec<u64> {
    pool(app).iter().map(|(r, _)| r.key).collect()
}

fn row_of(app: &mut App, key: u64) -> Entity {
    pool(app)
        .into_iter()
        .find(|(r, _)| r.key == key)
        .map(|(_, e)| e)
        .unwrap_or_else(|| panic!("node {key} is not shown"))
}

/// The row's label text.
fn label_of(app: &App, row: Entity) -> Entity {
    let cells = app.world().get::<Children>(row).expect("cells");
    let cell = app.world().get::<Children>(cells[0]).expect("cell parts");
    *cell.last().expect("text")
}

/// The row's chevron box.
fn twisty_of(app: &App, row: Entity) -> Entity {
    let cells = app.world().get::<Children>(row).expect("cells");
    app.world().get::<Children>(cells[0]).expect("cell parts")[0]
}

fn the_tree(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<Table>>()
        .single(app.world())
        .expect("one tree")
}

fn cursor(app: &mut App) -> Option<u64> {
    let tree = the_tree(app);
    app.world().get::<TableCursor>(tree).expect("cursor").0
}

fn selected(app: &mut App) -> Vec<u64> {
    let tree = the_tree(app);
    app.world()
        .get::<TableSelection>(tree)
        .expect("selection")
        .iter()
        .collect()
}

/// Changes the tree the way a game does, then lets the frames pass.
fn edit(app: &mut App, change: impl FnOnce(&mut Tree)) {
    let tree = the_tree(app);
    let mut table = app.world_mut().get_mut::<Table>(tree).expect("table");
    change(table.source_mut::<Tree>().expect("a tree"));
    settle(app);
}

fn viewport(app: &mut App) -> Entity {
    app.world_mut()
        .query::<(Entity, &Role)>()
        .iter(app.world())
        .find(|(_, r)| **r == Role::TableView)
        .map(|(e, _)| e)
        .expect("a viewport")
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

/// 1 World (open) { 2 Camera, 3 Lights { 4 Sun, 5 Moon }, 6 Cube }, 7 Terrain { 8 Hill }, 9 Cave.
fn scene() -> Vec<TreeNode> {
    vec![
        TreeNode::new(1, "World").expanded().children([
            TreeNode::new(2, "Camera"),
            TreeNode::new(3, "Lights")
                .child(TreeNode::new(4, "Sun"))
                .child(TreeNode::new(5, "Moon")),
            TreeNode::new(6, "Cube"),
        ]),
        TreeNode::new(7, "Terrain").child(TreeNode::new(8, "Hill")),
        TreeNode::new(9, "Cave"),
    ]
}

fn spawn(app: &mut App, nodes: Vec<TreeNode>, extra: impl Bundle) -> Entity {
    app.world_mut()
        .spawn((panel(), children![(tree(nodes), extra)]));
    settle(app);
    the_tree(app)
}

fn spawn_scene(app: &mut App) -> Entity {
    spawn(app, scene(), ())
}

/// 10 roots, each 10 children, each 10 grandchildren, each 9 leaves: 10,110 nodes, all open.
fn forest() -> Vec<TreeNode> {
    let mut key = 0;
    let mut next = || {
        key += 1;
        key
    };
    (0..10)
        .map(|a| {
            TreeNode::new(next(), format!("Zone {a}"))
                .expanded()
                .children((0..10).map(|b| {
                    TreeNode::new(next(), format!("Area {a}.{b}"))
                        .expanded()
                        .children((0..10).map(|c| {
                            TreeNode::new(next(), format!("Room {a}.{b}.{c}"))
                                .expanded()
                                .children((0..9).map(|d| {
                                    TreeNode::new(next(), format!("Item {a}.{b}.{c}.{d}"))
                                }))
                        }))
                }))
        })
        .collect()
}

fn connect_pad(app: &mut App) -> Entity {
    let pad = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(GamepadConnectionEvent::new(
        pad,
        GamepadConnection::Connected {
            name: "Test pad".into(),
            vendor_id: None,
            product_id: None,
        },
    ));
    settle(app);
    pad
}

fn press_pad(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
}

// The acceptance: ten thousand nodes cost visible + 2 row entities, and Left on an open node
// collapses it.

#[test]
fn ten_thousand_nodes_spawn_at_most_visible_plus_two_entities() {
    let mut app = app();
    spawn(&mut app, forest(), ());
    let visible = 8.5_f32.ceil() as usize;
    assert!(!pool(&mut app).is_empty());
    assert!(
        pool(&mut app).len() <= visible + 2,
        "{} rows for {visible} visible",
        pool(&mut app).len()
    );
    for y in [1234.0, 100_000.0, 439_000.0, 0.0] {
        scroll_to(&mut app, y);
        assert!(pool(&mut app).len() <= visible + 2, "at scroll {y}");
    }
    app.world_mut().resource_mut::<GooeySkin>().text_scale = 2.0;
    settle(&mut app);
    assert!(pool(&mut app).len() <= visible + 2, "at 200% text");
    // Every entity of the tree: ten thousand nodes cost what twenty do.
    let mut small = self::app();
    spawn(
        &mut small,
        (0..20).map(|i| TreeNode::new(i, "n")).collect(),
        (),
    );
    assert_eq!(
        app.world().entities().count_spawned(),
        small.world().entities().count_spawned(),
    );
}

#[test]
fn left_on_an_open_node_collapses_it_and_right_opens_it() {
    let mut app = app();
    let tree = spawn_scene(&mut app);
    assert_eq!(keys(&mut app), vec![1, 2, 3, 6, 7, 9]);
    focus(&mut app, tree);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(cursor(&mut app), Some(1));
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(keys(&mut app), vec![1, 7, 9]);
    assert_eq!(app.world().resource::<Log>().toggled, vec![(1, false)]);
    assert_eq!(cursor(&mut app), Some(1), "the cursor stays on the node");
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(keys(&mut app), vec![1, 2, 3, 6, 7, 9]);
    assert_eq!(
        app.world().resource::<Log>().toggled,
        vec![(1, false), (1, true)]
    );
    assert_eq!(focused(&app), Some(tree));
}

#[test]
fn right_on_an_open_node_goes_to_its_first_child_and_left_goes_back_to_the_parent() {
    let mut app = app();
    let tree = spawn_scene(&mut app);
    focus(&mut app, tree);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(cursor(&mut app), Some(2), "Camera, the first child");
    // Camera has no children: Right does nothing, Left goes to the parent.
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(cursor(&mut app), Some(2));
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(cursor(&mut app), Some(1));
    // Neither sent an event: nothing opened or closed.
    assert!(app.world().resource::<Log>().toggled.is_empty());
    // A closed root has no parent: Left does nothing.
    tap(&mut app, KeyCode::ArrowLeft); // closes World
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(cursor(&mut app), Some(1));
    assert_eq!(app.world().resource::<Log>().toggled, vec![(1, false)]);
}

#[test]
fn up_down_home_and_end_step_only_the_nodes_shown() {
    let mut app = app();
    let tree = spawn_scene(&mut app);
    focus(&mut app, tree);
    tap(&mut app, KeyCode::End);
    assert_eq!(cursor(&mut app), Some(9));
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(
        cursor(&mut app),
        Some(7),
        "Terrain is closed: Hill is not a stop"
    );
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(cursor(&mut app), Some(6));
    tap(&mut app, KeyCode::Home);
    assert_eq!(cursor(&mut app), Some(1));
    // End is the last shown node, whatever is open.
    edit(&mut app, |t| {
        t.set_open(7, true);
    });
    tap(&mut app, KeyCode::End);
    assert_eq!(cursor(&mut app), Some(9));
}

#[test]
fn a_closed_node_keeps_its_descendants_as_they_were() {
    let mut app = app();
    let tree = spawn_scene(&mut app);
    focus(&mut app, tree);
    // Open Lights, then close World and open it again: Lights is still open.
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(cursor(&mut app), Some(3));
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(keys(&mut app), vec![1, 2, 3, 4, 5, 6, 7, 9]);
    edit(&mut app, |t| {
        t.set_open(1, false);
    });
    assert_eq!(keys(&mut app), vec![1, 7, 9]);
    edit(&mut app, |t| {
        t.set_open(1, true);
    });
    assert_eq!(keys(&mut app), vec![1, 2, 3, 4, 5, 6, 7, 9]);
    // The game's own change sent nothing.
    assert_eq!(app.world().resource::<Log>().toggled, vec![(3, true)]);
}

#[test]
fn a_cursor_on_a_node_that_gets_hidden_moves_to_the_node_that_hides_it() {
    let mut app = app();
    let tree = spawn_scene(&mut app);
    edit(&mut app, |t| {
        t.set_open(3, true);
    });
    focus(&mut app, tree);
    for _ in 0..4 {
        tap(&mut app, KeyCode::ArrowDown);
    }
    assert_eq!(cursor(&mut app), Some(4), "Sun");
    edit(&mut app, |t| {
        t.set_open(3, false);
    });
    assert_eq!(cursor(&mut app), Some(3));
    // Two levels at once: the nearest node still shown.
    edit(&mut app, |t| {
        t.set_open(3, true);
    });
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(cursor(&mut app), Some(4));
    edit(&mut app, |t| {
        t.set_open(1, false);
    });
    assert_eq!(cursor(&mut app), Some(1));
}

// Type-ahead.

#[test]
fn typing_goes_to_the_next_node_that_starts_with_it() {
    let mut app = app();
    let tree = spawn_scene(&mut app);
    focus(&mut app, tree);
    type_char(&mut app, KeyCode::KeyT, "t");
    assert_eq!(cursor(&mut app), Some(7), "Terrain");
    assert_eq!(
        selected(&mut app),
        vec![7],
        "a single tree selects where it goes"
    );
    // A longer prefix typed in a rush narrows it: from Terrain, "ca" is Cave.
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            1500,
        )));
    app.update();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            16,
        )));
    type_char(&mut app, KeyCode::KeyC, "c");
    assert_eq!(cursor(&mut app), Some(9), "Cave is the next from Terrain");
    type_char(&mut app, KeyCode::KeyA, "a");
    assert_eq!(cursor(&mut app), Some(9), "'ca' stays on Cave");
    // Case never matters.
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            1500,
        )));
    app.update();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            16,
        )));
    type_char(&mut app, KeyCode::KeyC, "C");
    type_char(&mut app, KeyCode::KeyU, "U");
    assert_eq!(cursor(&mut app), Some(6), "'CU' is Cube");
}

#[test]
fn the_same_letter_again_goes_on_to_the_next_node_and_a_pause_starts_over() {
    let mut app = app();
    let tree = spawn_scene(&mut app);
    focus(&mut app, tree);
    type_char(&mut app, KeyCode::KeyC, "c");
    assert_eq!(cursor(&mut app), Some(2), "Camera");
    type_char(&mut app, KeyCode::KeyC, "c");
    assert_eq!(cursor(&mut app), Some(6), "Cube");
    type_char(&mut app, KeyCode::KeyC, "c");
    assert_eq!(cursor(&mut app), Some(9), "Cave");
    type_char(&mut app, KeyCode::KeyC, "c");
    assert_eq!(cursor(&mut app), Some(2), "and round again");
    // No node starts with it: nothing moves.
    type_char(&mut app, KeyCode::KeyZ, "z");
    assert_eq!(cursor(&mut app), Some(2));
}

#[test]
fn typing_with_ctrl_held_types_nothing_and_hidden_nodes_are_not_found() {
    let mut app = app();
    let tree = spawn_scene(&mut app);
    focus(&mut app, tree);
    holding(&mut app, KeyCode::ControlLeft, |app| {
        type_char(app, KeyCode::KeyT, "t");
    });
    assert_eq!(cursor(&mut app), None);
    // Hill is under a closed Terrain: not a stop.
    type_char(&mut app, KeyCode::KeyH, "h");
    assert_eq!(cursor(&mut app), None);
}

#[test]
fn star_opens_every_sibling_of_the_cursor_node() {
    let mut app = app();
    let tree = spawn_scene(&mut app);
    focus(&mut app, tree);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowLeft); // close World: roots 1, 7, 9 shown
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(cursor(&mut app), Some(7));
    app.world_mut().resource_mut::<Log>().toggled.clear();
    let window = window(&mut app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Digit8,
            logical_key: Key::Character("*".into()),
            state,
            text: Some("*".into()),
            repeat: false,
            window,
        });
        app.update();
    }
    settle(&mut app);
    assert_eq!(keys(&mut app), vec![1, 2, 3, 6, 7, 8, 9]);
    assert_eq!(
        app.world().resource::<Log>().toggled,
        vec![(1, true), (7, true)]
    );
    assert_eq!(cursor(&mut app), Some(7), "the cursor stays");
}

// The pad.

#[test]
fn the_dpad_opens_and_closes_and_keeps_focus() {
    let mut app = app();
    let tree = spawn_scene(&mut app);
    focus(&mut app, tree);
    let pad = connect_pad(&mut app);
    press_pad(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(cursor(&mut app), Some(1));
    press_pad(&mut app, pad, GamepadButton::DPadLeft);
    assert_eq!(keys(&mut app), vec![1, 7, 9]);
    press_pad(&mut app, pad, GamepadButton::DPadRight);
    assert_eq!(keys(&mut app), vec![1, 2, 3, 6, 7, 9]);
    press_pad(&mut app, pad, GamepadButton::DPadRight);
    assert_eq!(cursor(&mut app), Some(2));
    assert_eq!(focused(&app), Some(tree));
}

#[test]
fn left_and_right_never_move_focus_off_a_tree() {
    let mut app = app();
    let beside = app
        .world_mut()
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                ..default()
            },
            children![tree(scene()), button("Beside"), button("Under"),],
        ))
        .id();
    let _ = beside;
    settle(&mut app);
    let tree = the_tree(&mut app);
    focus(&mut app, tree);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowDown); // Camera, an end node
    for _ in 0..3 {
        tap(&mut app, KeyCode::ArrowRight);
    }
    assert_eq!(focused(&app), Some(tree), "Right on an end node");
    tap(&mut app, KeyCode::ArrowLeft);
    tap(&mut app, KeyCode::ArrowLeft); // closes World
    tap(&mut app, KeyCode::ArrowLeft);
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(focused(&app), Some(tree), "Left on a closed root");
}

#[test]
fn a_disabled_tree_takes_no_keys() {
    let mut app = app();
    let tree = spawn(&mut app, scene(), bevy::ui::InteractionDisabled);
    focus(&mut app, tree);
    tap(&mut app, KeyCode::ArrowRight);
    type_char(&mut app, KeyCode::KeyT, "t");
    assert_eq!(cursor(&mut app), None);
    assert!(app.world().resource::<Log>().toggled.is_empty());
}

// The pointer.

#[test]
fn a_click_on_a_chevron_opens_the_node_and_selects_nothing() {
    let mut app = app();
    spawn_scene(&mut app);
    let lights = row_of(&mut app, 3);
    let at = rect(&app, twisty_of(&app, lights)).center();
    click(&mut app, at);
    assert_eq!(keys(&mut app), vec![1, 2, 3, 4, 5, 6, 7, 9]);
    assert_eq!(app.world().resource::<Log>().toggled, vec![(3, true)]);
    assert!(selected(&mut app).is_empty());
    assert!(app.world().resource::<Log>().selections.is_empty());
    // Again: closed. An end node's chevron box does nothing.
    let lights = row_of(&mut app, 3);
    let at = rect(&app, twisty_of(&app, lights)).center();
    click(&mut app, at);
    assert_eq!(keys(&mut app), vec![1, 2, 3, 6, 7, 9]);
    let camera = row_of(&mut app, 2);
    let at = rect(&app, twisty_of(&app, camera)).center();
    click(&mut app, at);
    assert_eq!(app.world().resource::<Log>().toggled.len(), 2);
}

#[test]
fn a_click_on_the_label_selects_and_a_double_click_opens_the_row() {
    let mut app = app();
    spawn_scene(&mut app);
    let row = row_of(&mut app, 6);
    let at = rect(&app, label_of(&app, row)).center();
    click(&mut app, at);
    assert_eq!(selected(&mut app), vec![6]);
    assert_eq!(cursor(&mut app), Some(6));
    assert_eq!(app.world().resource::<Log>().selections, vec![vec![6]]);
    assert_eq!(keys(&mut app).len(), 6, "nothing opened or closed");
    assert!(app.world().resource::<Log>().toggled.is_empty());
}

#[test]
fn a_multi_tree_ranges_over_the_shown_nodes_and_toggles_one_at_a_time() {
    let mut app = app();
    spawn(&mut app, scene(), SelectMode::Multi);
    let click_label = |app: &mut App, key: u64| {
        let row = row_of(app, key);
        let at = rect(app, label_of(app, row)).center();
        click(app, at);
    };
    click_label(&mut app, 1);
    holding(&mut app, KeyCode::ShiftLeft, |app| click_label(app, 7));
    // Lights is closed: Sun and Moon are in no range.
    assert_eq!(selected(&mut app), vec![1, 2, 3, 6, 7]);
    holding(&mut app, KeyCode::ControlLeft, |app| click_label(app, 2));
    assert_eq!(selected(&mut app), vec![1, 3, 6, 7]);
}

// What the selection does as nodes come and go.

#[test]
fn a_closed_node_keeps_its_selected_descendants() {
    let mut app = app();
    let tree = spawn(&mut app, scene(), SelectMode::Multi);
    edit(&mut app, |t| {
        t.set_open(3, true);
    });
    app.world_mut()
        .get_mut::<TableSelection>(tree)
        .expect("selection")
        .0
        .extend([4, 5]);
    settle(&mut app);
    edit(&mut app, |t| {
        t.set_open(1, false);
    });
    assert_eq!(keys(&mut app), vec![1, 7, 9]);
    assert_eq!(selected(&mut app), vec![4, 5], "hidden is not gone");
    edit(&mut app, |t| {
        t.set_open(1, true);
    });
    let sun = row_of(&mut app, 4);
    assert!(app.world().get::<bevy::ui::Selected>(sun).is_some());
}

#[test]
fn a_removed_selected_node_passes_the_selection_to_its_sibling() {
    let mut app = app();
    let tree = spawn(&mut app, scene(), SelectMode::Multi);
    edit(&mut app, |t| {
        t.set_open(3, true);
    });
    let select = |app: &mut App, keys: &[u64]| {
        app.world_mut()
            .get_mut::<TableSelection>(tree)
            .expect("selection")
            .0 = keys.iter().copied().collect();
        app.world_mut().get_mut::<TableCursor>(tree).expect("c").0 = keys.first().copied();
        settle(app);
    };
    // The next sibling first: Sun goes, Moon takes its place.
    select(&mut app, &[4]);
    edit(&mut app, |t| {
        t.remove(4);
    });
    assert_eq!(selected(&mut app), vec![5]);
    assert_eq!(cursor(&mut app), Some(5));
    // No next sibling: the one before. Add Sun back before Moon is removed.
    edit(&mut app, |t| {
        t.insert(Some(3), TreeNode::new(10, "Star"));
    });
    select(&mut app, &[10]);
    edit(&mut app, |t| {
        t.remove(10);
    });
    assert_eq!(selected(&mut app), vec![5], "the one before");
    // An only child: the parent.
    select(&mut app, &[5]);
    edit(&mut app, |t| {
        t.remove(5);
    });
    assert_eq!(selected(&mut app), vec![3], "the parent");
    // A whole subtree with the selection inside: the heir of the node removed.
    edit(&mut app, |t| {
        t.insert(Some(3), TreeNode::new(11, "Lamp"));
        t.set_open(3, true);
    });
    select(&mut app, &[11]);
    edit(&mut app, |t| {
        t.remove(3);
    });
    assert_eq!(selected(&mut app), vec![6], "Lights' next sibling: Cube");
    // The game's own removals sent no ValueChange.
    assert!(app.world().resource::<Log>().selections.is_empty());
}

#[test]
fn opening_a_node_above_the_view_keeps_the_row_at_the_top_where_it_was() {
    let mut app = app();
    let mut nodes =
        vec![TreeNode::new(1, "Folded").children((0..5).map(|i| TreeNode::new(100 + i, "x")))];
    nodes.push(
        TreeNode::new(2, "Long")
            .expanded()
            .children((0..100).map(|i| TreeNode::new(200 + i, format!("Leaf {i}")))),
    );
    spawn(&mut app, nodes, ());
    let pitch = pitch(&app);
    scroll_to(&mut app, 20.0 * pitch);
    let view = viewport(&mut app);
    let view_top = rect(&app, view).min.y;
    let (first, row) = pool(&mut app)[0];
    let before = rect(&app, row).min.y - view_top;
    edit(&mut app, |t| {
        t.set_open(1, true);
    });
    let (now, row) = pool(&mut app)[0];
    assert_eq!(now.key, first.key, "the same node is at the top");
    assert!((rect(&app, row).min.y - view_top - before).abs() < 1.0);
}

// What it looks like.

#[test]
fn chevrons_mark_the_nodes_with_children_and_labels_stand_one_level_in_per_depth() {
    let mut app = app();
    spawn_scene(&mut app);
    edit(&mut app, |t| {
        t.set_open(3, true);
    });
    for scale in [1.0, 2.0] {
        app.world_mut().resource_mut::<GooeySkin>().text_scale = scale;
        settle(&mut app);
        let skin = app.world().resource::<GooeySkin>().clone();
        let indent = tree_indent(skin.text_size * skin.text_scale);
        let left_of = |app: &mut App, key: u64| {
            let row = row_of(app, key);
            rect(app, label_of(app, row)).min.x
        };
        let world = left_of(&mut app, 1);
        for (key, depth) in [(2, 1.0), (3, 1.0), (4, 2.0), (5, 2.0), (7, 0.0), (9, 0.0)] {
            let at = left_of(&mut app, key);
            assert!(
                (at - world - depth * indent).abs() < 1.0,
                "{scale}: node {key} at {at}, World at {world}, depth {depth}, indent {indent}"
            );
        }
        // Which chevron shows: open points down, closed right, an end node none; and the box
        // keeps its room either way, a row high and a level wide.
        let shown = |app: &mut App, key: u64| {
            let row = row_of(app, key);
            let twisty = twisty_of(app, row);
            let both = app.world().get::<Children>(twisty).expect("chevrons");
            let flags: Vec<bool> = both
                .iter()
                .map(|c| app.world().get::<Node>(c).expect("node").display != Display::None)
                .collect();
            let size = rect(app, twisty);
            ((flags[0], flags[1]), size)
        };
        assert_eq!(shown(&mut app, 1).0, (false, true), "World is open");
        assert_eq!(shown(&mut app, 7).0, (true, false), "Terrain is closed");
        let (end, size) = shown(&mut app, 9);
        assert_eq!(end, (false, false), "Cave has no children");
        assert!((size.width() - indent).abs() < 1.0, "{scale}: a level wide");
        // The ring is inside the row's box, so its inside is the pitch less two lines.
        assert!(
            size.height() >= pitch(&app) - 8.0,
            "{scale}: a row high ({} of {})",
            size.height(),
            pitch(&app)
        );
        assert!(size.width() >= 24.0 - 0.5, "{scale}: a target wide");
    }
}

fn rgb(color: Color) -> [f32; 3] {
    let c = color.to_srgba();
    [c.red, c.green, c.blue]
}

#[test]
fn the_chevron_reads_on_every_row_state_in_every_look() {
    for preset in SkinPreset::ALL {
        let mut app = app();
        *app.world_mut().resource_mut::<GooeySkin>() = preset.skin();
        let tree = spawn(&mut app, scene(), SelectMode::Multi);
        app.world_mut()
            .get_mut::<TableSelection>(tree)
            .expect("selection")
            .0
            .insert(1);
        settle(&mut app);
        let skin = app.world().resource::<GooeySkin>().clone();
        for key in [1, 3] {
            let row = row_of(&mut app, key);
            let fill = app.world().get::<BackgroundColor>(row).expect("fill").0;
            let behind = composite(rgb(fill), fill.alpha(), rgb(skin.surface));
            let twisty = twisty_of(&app, row);
            let chevrons = app.world().get::<Children>(twisty).expect("chevrons");
            for chevron in chevrons.iter() {
                if app.world().get::<Node>(chevron).expect("node").display == Display::None {
                    continue;
                }
                let ink = app.world().get::<BorderColor>(chevron).expect("ink").top;
                let ratio = contrast_ratio(rgb(ink), behind);
                assert!(
                    ratio >= NON_TEXT_CONTRAST,
                    "{}: the chevron of node {key} reads at {ratio:.2}:1",
                    preset.name()
                );
            }
        }
    }
}

#[test]
fn a_tree_at_rest_writes_nothing() {
    let mut app = app();
    spawn(&mut app, forest(), ());
    settle(&mut app);
    let before = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, before);
}

// The data, without an app.

#[test]
fn the_tree_data_inserts_removes_and_opens_by_key() {
    let mut tree = Tree::new(scene());
    assert_eq!(tree.node_count(), 9);
    assert_eq!(
        tree.len(),
        6,
        "shown: World's children but not Lights' or Terrain's"
    );
    assert_eq!(tree.open_keys(), vec![1]);
    assert_eq!(tree.label(4), Some("Sun"));
    // A parent that is not there, a key that is taken: nothing changes.
    assert!(!tree.insert(Some(99), TreeNode::new(50, "x")));
    assert!(!tree.insert(None, TreeNode::new(4, "again")));
    assert_eq!(tree.node_count(), 9);
    // A child lands last under its parent, shown only when the parent is open.
    assert!(tree.insert(Some(1), TreeNode::new(50, "Fog")));
    assert_eq!(tree.len(), 7);
    assert!(tree.insert(Some(7), TreeNode::new(51, "Rock")));
    assert_eq!(tree.len(), 7);
    assert!(tree.set_open(7, true));
    assert!(!tree.set_open(7, true));
    assert_eq!(tree.len(), 9);
    // A removed node takes its subtree with it.
    assert!(tree.remove(3));
    assert!(!tree.contains(4) && !tree.contains(5));
    assert!(!tree.remove(3));
    assert_eq!(tree.open_keys(), vec![1, 7]);
}

#[test]
fn a_repeated_key_leaves_the_second_node_and_its_children_out() {
    let tree = Tree::new([
        TreeNode::new(1, "a").child(TreeNode::new(2, "b")),
        TreeNode::new(1, "again").child(TreeNode::new(3, "c")),
    ]);
    assert_eq!(tree.node_count(), 2);
    assert!(!tree.contains(3));
}

#[test]
fn each_row_shows_the_label_of_its_node_and_follows_the_tree() {
    let mut app = app();
    spawn_scene(&mut app);
    let labels = |app: &mut App| {
        pool(app)
            .iter()
            .map(|(_, row)| text_of(app, label_of(app, *row)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        labels(&mut app),
        ["World", "Camera", "Lights", "Cube", "Terrain", "Cave"]
    );
    edit(&mut app, |t| {
        t.set_open(1, false);
        t.set_open(7, true);
    });
    assert_eq!(labels(&mut app), ["World", "Terrain", "Hill", "Cave"]);
}
