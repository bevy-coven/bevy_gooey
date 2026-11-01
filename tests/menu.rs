//! A menu bar as a game sees it, laid out in a headless window and driven by real keyboard and
//! pointer input: F10 and Alt take focus into the bar, Left and Right walk it, Down opens a menu
//! with focus on its first item, Enter chooses (a disabled item never), Escape peels one level,
//! checks and radio runs report once, shortcuts end on the card's right edge, a submenu opens on
//! Right and closes on Left, an empty one says so, and a long menu stays inside the window.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::InputFocus;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{Checked, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// What a restyle writes.
type Written = Or<(
    Changed<Node>,
    Changed<BackgroundColor>,
    Changed<BorderColor>,
)>;

#[derive(Resource, Default)]
struct Writes(u32);

/// Every command activated, by its label, in order.
#[derive(Resource, Default)]
struct Chosen(Vec<&'static str>);

/// Every `PopoverClosed` a game's observer heard.
#[derive(Resource, Default)]
struct Closed(Vec<Entity>);

/// Every widget focus has rested on, in order.
#[derive(Resource, Default)]
struct Rested(Vec<Entity>);

/// Every `ValueChange<bool>` a menu item sent: its label and the new value.
#[derive(Resource, Default)]
struct Flipped(Vec<(Entity, bool)>);

fn app_with(skin: GooeySkin) -> App {
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
    .init_resource::<Writes>()
    .init_resource::<Chosen>()
    .init_resource::<Flipped>()
    .init_resource::<Closed>()
    .init_resource::<Rested>()
    .insert_resource(skin)
    .add_observer(|closed: On<PopoverClosed>, mut list: ResMut<Closed>| list.0.push(closed.entity))
    .add_observer(|change: On<ValueChange<bool>>, mut list: ResMut<Flipped>| {
        list.0.push((change.source, change.value));
    })
    .add_systems(
        Update,
        (
            |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
                writes.0 += changed.iter().count() as u32
            },
            |focus: Res<InputFocus>, mut seen: ResMut<Rested>| {
                if let Some(f) = focus.get()
                    && seen.0.last() != Some(&f)
                {
                    seen.0.push(f);
                }
            },
        ),
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn app() -> App {
    app_with(GooeySkin::default())
}

fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

fn chose(label: &'static str) -> impl Bundle {
    observe(move |_: On<Activate>, mut log: ResMut<Chosen>| log.0.push(label))
}

/// The bar of the issue's acceptance: File (New, Open, a separator, Recent, Autosave) and View
/// (Zoom in, a disabled Zoom out, a separator, a radio run).
fn spawn_bar(app: &mut App) -> Entity {
    let bar = app
        .world_mut()
        .spawn(menu_bar(children![
            menu_title(
                "File",
                children![
                    (
                        menu_item("New").with_shortcut(Shortcut::primary(KeyCode::KeyN)),
                        chose("New")
                    ),
                    (
                        menu_item("Open…").with_shortcut(Shortcut::new(
                            Modifiers::CTRL | Modifiers::SHIFT,
                            KeyCode::KeyO
                        )),
                        chose("Open")
                    ),
                    separator(),
                    submenu(
                        "Recent",
                        children![
                            (menu_item("level-1.ron"), chose("level-1")),
                            (menu_item("level-2.ron"), chose("level-2")),
                        ]
                    ),
                    (check_item("Autosave", true), chose("Autosave")),
                ]
            ),
            menu_title(
                "View",
                children![
                    (
                        menu_item("Zoom in")
                            .with_shortcut(Shortcut::new(Modifiers::NONE, KeyCode::Equal)),
                        chose("Zoom in")
                    ),
                    (
                        menu_item("Zoom out"),
                        Disabled::new("Already at the smallest size"),
                        chose("Zoom out")
                    ),
                    separator(),
                    radio_item("Small", false),
                    radio_item("Large", true),
                    radio_item("Huge", false),
                ]
            ),
            menu_title("Help", children![submenu("Nothing", children![])]),
        ]))
        .id();
    settle(app);
    bar
}

fn window_entity(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    let window = window_entity(app);
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

/// A key pressed and let go, then the frames a menu needs to open and take focus.
fn tap(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Pressed);
    key(app, key_code, ButtonState::Released);
    settle(app);
}

fn taps(app: &mut App, keys: &[KeyCode]) {
    for &k in keys {
        tap(app, k);
    }
}

fn focus(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

/// The `Text` node reading `text` whose role is `role`, anywhere.
fn text_with(app: &mut App, role: Role, text: &str) -> Entity {
    let world = app.world_mut();
    world
        .query::<(Entity, &Text, &Role)>()
        .iter(world)
        .find(|(_, t, r)| t.0 == text && **r == role)
        .map(|(e, ..)| e)
        .unwrap_or_else(|| panic!("no {role:?} reading {text:?}"))
}

/// The item (a menu item or a title) whose label reads `text`.
fn item(app: &mut App, text: &str) -> Entity {
    let label = text_with(app, Role::MenuText, text);
    app.world()
        .get::<ChildOf>(label)
        .map(ChildOf::parent)
        .expect("a row")
}

fn title(app: &mut App, text: &str) -> Entity {
    let label = text_with(app, Role::ButtonText, text);
    app.world()
        .get::<ChildOf>(label)
        .map(ChildOf::parent)
        .expect("a title")
}

/// The panel a title or item opens.
fn panel_of(app: &App, anchor: Entity) -> Entity {
    app.world()
        .get::<Children>(anchor)
        .expect("children")
        .iter()
        .find(|&c| app.world().get::<MenuPanel>(c).is_some())
        .expect("a menu panel")
}

fn is_open(app: &App, anchor: Entity) -> bool {
    app.world()
        .get::<PopoverPanel>(panel_of(app, anchor))
        .is_some()
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

fn is_inside(app: &App, entity: Entity, root: Entity) -> bool {
    let mut at = Some(entity);
    while let Some(e) = at {
        if e == root {
            return true;
        }
        at = app.world().get::<ChildOf>(e).map(ChildOf::parent);
    }
    false
}

fn chosen(app: &App) -> Vec<&'static str> {
    app.world().resource::<Chosen>().0.clone()
}

fn mouse(app: &mut App, at: Vec2, action: PointerAction) {
    let window = window_entity(app);
    let location = Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    };
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, location, action));
}

fn point_at(app: &mut App, at: Vec2) {
    mouse(app, at, PointerAction::Move { delta: Vec2::ZERO });
    for _ in 0..3 {
        app.update();
    }
}

fn click_at(app: &mut App, at: Vec2) {
    point_at(app, at);
    for action in [
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        mouse(app, at, action);
        app.update();
    }
    settle(app);
}

fn click_on(app: &mut App, entity: Entity) {
    let at = rect(app, entity).center();
    click_at(app, at);
}

/// The issue's acceptance: F10, Right, Down, Enter chooses the second menu's first item.
#[test]
fn f10_right_down_enter_chooses_the_second_menus_first_item() {
    let mut app = app();
    spawn_bar(&mut app);
    let file = title(&mut app, "File");
    let view = title(&mut app, "View");
    tap(&mut app, KeyCode::F10);
    assert_eq!(focus(&app), Some(file), "F10 lands on the first title");
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(focus(&app), Some(view));
    tap(&mut app, KeyCode::ArrowDown);
    assert!(is_open(&app, view), "Down opens the menu");
    let zoom_in = item(&mut app, "Zoom in");
    assert_eq!(focus(&app), Some(zoom_in), "focus is on the first item");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(chosen(&app), vec!["Zoom in"], "chosen once");
    assert!(!is_open(&app, view), "choosing closes the menu");
}

/// Every `Text` of `role` at or under `root`.
fn texts_under(app: &App, root: Entity, role: Role) -> Vec<Entity> {
    let world = app.world();
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if world.get::<Text>(e).is_some() && world.get::<Role>(e) == Some(&role) {
            found.push(e);
        }
        if let Some(kids) = world.get::<Children>(e) {
            stack.extend(kids.iter());
        }
    }
    found
}

#[test]
fn a_disabled_item_never_activates_and_says_why() {
    let mut app = app();
    spawn_bar(&mut app);
    let view = title(&mut app, "View");
    let zoom_out = item(&mut app, "Zoom out");
    tap(&mut app, KeyCode::F10);
    taps(
        &mut app,
        &[KeyCode::ArrowRight, KeyCode::ArrowDown, KeyCode::ArrowDown],
    );
    assert_eq!(
        focus(&app),
        Some(zoom_out),
        "a disabled item can be focused"
    );
    // The reason is its tooltip, open because focus is visible on it.
    let tip = app
        .world()
        .get::<Children>(zoom_out)
        .expect("children")
        .iter()
        .find(|&c| app.world().get::<TooltipBox>(c).is_some())
        .expect("a tooltip box");
    assert_eq!(
        app.world().get::<Node>(tip).expect("a node").display,
        Display::Flex,
        "the reason is showing"
    );
    assert!(is_open(&app, view), "open before any key");
    for k in [KeyCode::Enter, KeyCode::Space] {
        tap(&mut app, k);
        assert!(chosen(&app).is_empty(), "{k:?} chose a disabled item");
        assert!(is_open(&app, view), "{k:?} closed the menu");
    }
    click_on(&mut app, zoom_out);
    assert!(chosen(&app).is_empty(), "a click chose a disabled item");
    assert!(is_open(&app, view), "a click on it closes nothing");
}

#[test]
fn shortcuts_end_on_one_edge_and_labels_start_on_one() {
    let mut app = app();
    spawn_bar(&mut app);
    let file = title(&mut app, "File");
    tap(&mut app, KeyCode::F10);
    tap(&mut app, KeyCode::ArrowDown);
    let panel = panel_of(&app, file);
    let card = rect(&app, panel);
    let shortcuts = texts_under(&app, panel, Role::MenuShortcut);
    assert_eq!(shortcuts.len(), 2, "New and Open have a chord");
    let gaps: Vec<f32> = shortcuts
        .iter()
        .map(|&s| card.max.x - rect(&app, s).max.x)
        .collect();
    assert!(
        (gaps[0] - gaps[1]).abs() <= 1.0,
        "chords end on one edge, {gaps:?} from the card's"
    );
    assert!(
        gaps[0] > 0.0 && gaps[0] < 40.0,
        "and it is the card's inner edge: {} px",
        gaps[0]
    );
    // Labels share one edge whether or not the item is a check.
    let labels: Vec<f32> = ["New", "Open…", "Recent", "Autosave"]
        .iter()
        .map(|t| {
            let e = text_with(&mut app, Role::MenuText, t);
            rect(&app, e).min.x
        })
        .collect();
    assert!(
        labels.windows(2).all(|w| (w[0] - w[1]).abs() <= 1.0),
        "label edges {labels:?}"
    );
    // The widest chord leaves the label a gap.
    let open = text_with(&mut app, Role::MenuText, "Open…");
    let chord = shortcuts
        .iter()
        .map(|&s| rect(&app, s))
        .min_by(|a, b| a.min.x.total_cmp(&b.min.x))
        .expect("a chord");
    assert!(
        chord.min.x - rect(&app, open).max.x >= 8.0,
        "label and chord touch"
    );
}

#[test]
fn escape_closes_one_level_then_gives_focus_back_to_where_f10_found_it() {
    let mut app = app();
    // Directly under the File title, where a spatial Down would land.
    let canvas = app.world_mut().spawn(button("Canvas")).id();
    {
        let mut node = app.world_mut().get_mut::<Node>(canvas).expect("a node");
        node.position_type = PositionType::Absolute;
        node.left = Val::Px(20.0);
        node.top = Val::Px(400.0);
    }
    spawn_bar(&mut app);
    let file = title(&mut app, "File");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(canvas, bevy::input_focus::FocusCause::Navigated);
    tap(&mut app, KeyCode::F10);
    tap(&mut app, KeyCode::ArrowDown);
    assert!(is_open(&app, file));
    assert!(
        app.world()
            .resource::<Rested>()
            .0
            .iter()
            .filter(|&&e| e == canvas)
            .count()
            == 1,
        "Down on a title opens its menu and does not first move focus to the widget below it: {:?}",
        app.world().resource::<Rested>().0
    );
    tap(&mut app, KeyCode::Escape);
    assert!(!is_open(&app, file), "Escape closed the menu");
    assert_eq!(focus(&app), Some(file), "and focus is on its title");
    tap(&mut app, KeyCode::Escape);
    assert_eq!(
        focus(&app),
        Some(canvas),
        "the second Escape leaves the bar"
    );
    // And F10 inside the bar leaves it the same way.
    tap(&mut app, KeyCode::F10);
    assert_eq!(focus(&app), Some(file));
    tap(&mut app, KeyCode::F10);
    assert_eq!(focus(&app), Some(canvas));
}

#[test]
fn a_submenu_opens_on_right_closes_on_left_and_choosing_closes_the_cascade() {
    let mut app = app();
    spawn_bar(&mut app);
    let file = title(&mut app, "File");
    let recent = item(&mut app, "Recent");
    tap(&mut app, KeyCode::F10);
    taps(
        &mut app,
        &[KeyCode::ArrowDown, KeyCode::ArrowDown, KeyCode::ArrowDown],
    );
    assert_eq!(focus(&app), Some(recent));
    tap(&mut app, KeyCode::ArrowRight);
    assert!(is_open(&app, recent), "Right opens the submenu");
    let first = item(&mut app, "level-1.ron");
    assert_eq!(focus(&app), Some(first));
    // The card stands right of its item, inside the window.
    let sub = rect(&app, panel_of(&app, recent));
    assert!(sub.min.x >= rect(&app, recent).max.x - 12.0, "{sub:?}");
    tap(&mut app, KeyCode::ArrowLeft);
    assert!(!is_open(&app, recent), "Left closes it");
    assert!(is_open(&app, file), "and not its parent");
    assert_eq!(focus(&app), Some(recent), "focus is back on its item");
    taps(
        &mut app,
        &[KeyCode::ArrowRight, KeyCode::ArrowDown, KeyCode::Enter],
    );
    assert_eq!(chosen(&app), vec!["level-2"]);
    assert!(
        !is_open(&app, recent) && !is_open(&app, file),
        "the cascade closed"
    );
    assert!(
        !focus(&app).is_some_and(|f| f == file || is_inside(&app, f, file)),
        "focus left the bar: it was nowhere before F10"
    );
    // Escape closes the submenu first.
    taps(
        &mut app,
        &[
            KeyCode::F10,
            KeyCode::ArrowDown,
            KeyCode::ArrowDown,
            KeyCode::ArrowDown,
            KeyCode::ArrowRight,
        ],
    );
    assert!(is_open(&app, recent));
    tap(&mut app, KeyCode::Escape);
    assert!(!is_open(&app, recent) && is_open(&app, file), "one level");
    assert_eq!(focus(&app), Some(recent));
    assert!(
        app.world().resource::<Closed>().0.len() == 1,
        "one PopoverClosed for the player's Escape"
    );
}

#[test]
fn right_and_left_walk_the_titles_with_their_menus_open() {
    let mut app = app();
    spawn_bar(&mut app);
    let (file, view, help) = (
        title(&mut app, "File"),
        title(&mut app, "View"),
        title(&mut app, "Help"),
    );
    tap(&mut app, KeyCode::F10);
    taps(&mut app, &[KeyCode::ArrowDown, KeyCode::ArrowRight]);
    assert!(
        !is_open(&app, file) && is_open(&app, view),
        "Right moved to View's menu"
    );
    let zoom_in = item(&mut app, "Zoom in");
    assert_eq!(focus(&app), Some(zoom_in));
    taps(&mut app, &[KeyCode::ArrowLeft]);
    assert!(
        is_open(&app, file) && !is_open(&app, view),
        "Left came back"
    );
    taps(&mut app, &[KeyCode::ArrowLeft]);
    assert!(is_open(&app, help), "and wraps to the last title");
    // Escape from a menu reached that way gives focus to the title it opened under.
    tap(&mut app, KeyCode::Escape);
    assert_eq!(focus(&app), Some(help));
    // Left and Right walk a closed bar and wrap.
    taps(&mut app, &[KeyCode::ArrowRight]);
    assert_eq!(focus(&app), Some(file));
    taps(&mut app, &[KeyCode::ArrowLeft]);
    assert_eq!(focus(&app), Some(help));
}

#[test]
fn check_items_flip_and_radio_runs_keep_one_on() {
    let mut app = app();
    spawn_bar(&mut app);
    let autosave = item(&mut app, "Autosave");
    let (small, large, huge) = (
        item(&mut app, "Small"),
        item(&mut app, "Large"),
        item(&mut app, "Huge"),
    );
    let on = |app: &App, e: Entity| app.world().get::<Checked>(e).is_some();
    assert!(on(&app, autosave) && on(&app, large) && !on(&app, small));
    tap(&mut app, KeyCode::F10);
    taps(
        &mut app,
        &[
            KeyCode::ArrowDown,
            KeyCode::ArrowDown,
            KeyCode::ArrowDown,
            KeyCode::ArrowDown,
            KeyCode::Enter,
        ],
    );
    assert_eq!(chosen(&app), vec!["Autosave"]);
    assert!(!on(&app, autosave), "the check flipped off");
    assert_eq!(
        app.world().resource::<Flipped>().0,
        vec![(autosave, false)],
        "one ValueChange"
    );
    // View's radio run: Small is two items down (the disabled one is focusable).
    let flips = app.world().resource::<Flipped>().0.len();
    taps(
        &mut app,
        &[
            KeyCode::F10,
            KeyCode::ArrowRight,
            KeyCode::ArrowDown,
            KeyCode::ArrowDown,
            KeyCode::ArrowDown,
            KeyCode::Enter,
        ],
    );
    assert!(
        on(&app, small) && !on(&app, large) && !on(&app, huge),
        "exactly one is on"
    );
    assert_eq!(
        app.world().resource::<Flipped>().0[flips..],
        [(small, true)],
        "one change, for the one chosen"
    );
    // Choosing the one that is on changes nothing and sends nothing.
    let flips = app.world().resource::<Flipped>().0.len();
    taps(
        &mut app,
        &[
            KeyCode::F10,
            KeyCode::ArrowRight,
            KeyCode::ArrowDown,
            KeyCode::ArrowDown,
            KeyCode::ArrowDown,
            KeyCode::Enter,
        ],
    );
    assert_eq!(app.world().resource::<Flipped>().0.len(), flips);
    assert!(on(&app, small));
}

#[test]
fn an_empty_submenu_says_nothing_here() {
    let mut app = app();
    spawn_bar(&mut app);
    let help = title(&mut app, "Help");
    let nothing = item(&mut app, "Nothing");
    tap(&mut app, KeyCode::F10);
    taps(
        &mut app,
        &[KeyCode::ArrowLeft, KeyCode::ArrowDown, KeyCode::ArrowRight],
    );
    assert!(is_open(&app, nothing));
    let panel = panel_of(&app, nothing);
    let notes = texts_under(&app, panel, Role::MenuNote);
    assert_eq!(notes.len(), 1);
    assert_eq!(
        app.world().get::<Text>(notes[0]).expect("text").0,
        "Nothing here"
    );
    assert!(rect(&app, notes[0]).size().x > 0.0, "and it is drawn");
    tap(&mut app, KeyCode::ArrowLeft);
    assert!(
        !is_open(&app, nothing) && is_open(&app, help),
        "Left closes the empty submenu"
    );
    taps(&mut app, &[KeyCode::ArrowRight, KeyCode::Escape]);
    assert!(
        !is_open(&app, nothing) && is_open(&app, help),
        "so does Escape, one level"
    );
}

#[test]
fn the_pointer_opens_switches_and_closes() {
    let mut app = app();
    spawn_bar(&mut app);
    let (file, view) = (title(&mut app, "File"), title(&mut app, "View"));
    click_on(&mut app, file);
    assert!(is_open(&app, file), "a click opens");
    let at = rect(&app, view).center();
    point_at(&mut app, at);
    settle(&mut app);
    assert!(
        is_open(&app, view) && !is_open(&app, file),
        "the pointer on View switches"
    );
    click_on(&mut app, view);
    assert!(!is_open(&app, view), "a click on the open title closes it");
    // Pointer over a title with nothing open opens nothing.
    let at = rect(&app, file).center();
    point_at(&mut app, at);
    settle(&mut app);
    assert!(!is_open(&app, file));
    // A press outside closes the cascade, once.
    click_on(&mut app, file);
    click_at(&mut app, Vec2::new(900.0, 500.0));
    assert!(!is_open(&app, file), "a press outside closes it");
    // A click on an item chooses it and closes the menu.
    click_on(&mut app, file);
    let new = item(&mut app, "New");
    click_on(&mut app, new);
    assert_eq!(chosen(&app), vec!["New"]);
    assert!(!is_open(&app, file));
}

#[test]
fn alt_tapped_on_its_own_focuses_the_bar_but_a_chord_does_not() {
    let mut app = app();
    spawn_bar(&mut app);
    let file = title(&mut app, "File");
    // Alt with a key between is a chord: no bar.
    key(&mut app, KeyCode::AltLeft, ButtonState::Pressed);
    key(&mut app, KeyCode::KeyS, ButtonState::Pressed);
    key(&mut app, KeyCode::KeyS, ButtonState::Released);
    key(&mut app, KeyCode::AltLeft, ButtonState::Released);
    settle(&mut app);
    assert_ne!(focus(&app), Some(file));
    tap(&mut app, KeyCode::AltLeft);
    assert_eq!(focus(&app), Some(file), "a lone Alt tap does");
}

#[test]
fn a_long_menu_stays_in_the_window_and_follows_focus() {
    let mut app = app();
    let long = app
        .world_mut()
        .spawn(menu_bar(children![menu_title(
            "Many",
            Children::spawn(bevy::ecs::spawn::SpawnIter(
                (0..40).map(|i| menu_item(format!("Command {i}")))
            ))
        )]))
        .id();
    let _ = long;
    settle(&mut app);
    let many = title(&mut app, "Many");
    tap(&mut app, KeyCode::F10);
    tap(&mut app, KeyCode::ArrowDown);
    let panel = panel_of(&app, many);
    let window = Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0));
    let card = rect(&app, panel);
    assert!(
        card.min.y >= window.min.y && card.max.y <= window.max.y + 0.5,
        "the card is inside the window: {card:?}"
    );
    tap(&mut app, KeyCode::End);
    let last = item(&mut app, "Command 39");
    assert_eq!(focus(&app), Some(last));
    settle(&mut app);
    let (card, row) = (rect(&app, panel), rect(&app, last));
    assert!(
        row.min.y >= card.min.y - 0.5 && row.max.y <= card.max.y + 0.5,
        "the last item is scrolled into the card: {row:?} in {card:?}"
    );
    tap(&mut app, KeyCode::Home);
    settle(&mut app);
    let first = item(&mut app, "Command 0");
    let (card, row) = (rect(&app, panel), rect(&app, first));
    assert!(row.min.y >= card.min.y - 0.5 && row.max.y <= card.max.y + 0.5);
}

#[test]
fn nothing_is_written_while_a_menu_idles() {
    let mut app = app();
    spawn_bar(&mut app);
    tap(&mut app, KeyCode::F10);
    taps(&mut app, &[KeyCode::ArrowDown, KeyCode::ArrowDown]);
    let before = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, before);
}

/// Plugs in a gamepad and presses `button` on it, with the frames a menu needs.
fn pad_press(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
    settle(app);
}

#[test]
fn the_pad_walks_a_menu_as_the_keyboard_does() {
    let mut app = app();
    spawn_bar(&mut app);
    let (file, view) = (title(&mut app, "File"), title(&mut app, "View"));
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
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(file, bevy::input_focus::FocusCause::Navigated);
    pad_press(&mut app, pad, GamepadButton::DPadDown);
    assert!(is_open(&app, file), "D-pad Down opens");
    pad_press(&mut app, pad, GamepadButton::DPadRight);
    assert!(
        is_open(&app, view) && !is_open(&app, file),
        "D-pad Right moves to the next menu"
    );
    pad_press(&mut app, pad, GamepadButton::South);
    assert_eq!(chosen(&app), vec!["Zoom in"], "South chooses");
    pad_press(&mut app, pad, GamepadButton::DPadDown);
    assert!(is_open(&app, view));
    pad_press(&mut app, pad, GamepadButton::East);
    assert!(!is_open(&app, view), "B closes it");
    assert_eq!(focus(&app), Some(view));
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

/// The ink of a text node and the fill drawn behind it: the nearest ancestor with a visible fill.
fn ink_and_fill(app: &App, text: Entity) -> (Color, Color) {
    let ink = app.world().get::<TextColor>(text).expect("ink").0;
    let mut at = Some(text);
    while let Some(e) = at {
        if let Some(fill) = app.world().get::<BackgroundColor>(e)
            && Srgba::from(fill.0).alpha > 0.5
        {
            return (ink, fill.0);
        }
        at = app.world().get::<ChildOf>(e).map(ChildOf::parent);
    }
    panic!("nothing is drawn behind the text");
}

fn contrast_of(app: &App, text: Entity) -> f32 {
    let (ink, fill) = ink_and_fill(app, text);
    contrast_ratio(rgb(ink), rgb(fill))
}

#[test]
fn every_word_of_a_menu_reads_in_every_preset_and_state() {
    for (name, skin) in [
        ("dark", GooeySkin::dark()),
        ("parchment", GooeySkin::parchment()),
        ("high contrast", GooeySkin::high_contrast()),
        ("tool", GooeySkin::tool()),
    ] {
        let bar = if name == "high contrast" {
            ENHANCED_CONTRAST
        } else {
            TEXT_CONTRAST
        };
        let mut app = app_with(skin.clone());
        spawn_bar(&mut app);
        // A bar title at rest, on the bar.
        let file_text = text_with(&mut app, Role::ButtonText, "File");
        assert!(
            contrast_of(&app, file_text) >= bar,
            "{name}: a title at rest"
        );
        tap(&mut app, KeyCode::F10);
        // The focused title is lit.
        assert!(contrast_of(&app, file_text) >= bar, "{name}: a title lit");
        tap(&mut app, KeyCode::ArrowDown);
        // The first item is focused (lit), the next is at rest.
        for (label, what) in [
            ("New", "lit"),
            ("Open…", "at rest"),
            ("Autosave", "at rest"),
        ] {
            let text = text_with(&mut app, Role::MenuText, label);
            assert!(contrast_of(&app, text) >= bar, "{name}: {label} {what}");
        }
        let file = title(&mut app, "File");
        let chords = texts_under(&app, panel_of(&app, file), Role::MenuShortcut);
        for chord in chords {
            assert!(contrast_of(&app, chord) >= bar, "{name}: a chord");
        }
        // A disabled item and its chord: muted, and still readable.
        taps(&mut app, &[KeyCode::ArrowRight, KeyCode::ArrowDown]);
        let zoom_out = text_with(&mut app, Role::MenuText, "Zoom out");
        assert!(
            contrast_of(&app, zoom_out) >= bar,
            "{name}: a disabled item, lit"
        );
        taps(&mut app, &[KeyCode::ArrowDown]);
        assert!(
            contrast_of(&app, zoom_out) >= bar,
            "{name}: a disabled item at rest"
        );
        // The row's own frame: the ring is the focus ring, 3:1 on the card.
        let small = item(&mut app, "Small");
        let ring = app.world().get::<BorderColor>(small).expect("a border").top;
        assert_eq!(
            ring, skin.focus_ring,
            "{name}: the focused row carries the ring"
        );
        let view = title(&mut app, "View");
        let card = app
            .world()
            .get::<BackgroundColor>(panel_of(&app, view))
            .expect("fill")
            .0;
        assert!(
            contrast_ratio(rgb(ring), rgb(card)) >= NON_TEXT_CONTRAST,
            "{name}: the ring on the card"
        );
        // The card's own edge holds 3:1 against the screen behind it.
        let edge = app
            .world()
            .get::<BorderColor>(panel_of(&app, view))
            .expect("edge")
            .top;
        assert_eq!(edge, skin.outline, "{name}");
        // Hit targets: a row and a title are at least the skin's target tall.
        for e in [small, view] {
            assert!(
                rect(&app, e).height() >= skin.target() - 0.5,
                "{name}: {} px tall",
                rect(&app, e).height()
            );
        }
    }
}

#[test]
fn nothing_leaves_its_parent_at_every_text_size() {
    for scale in [1.0, 1.5, 2.0] {
        let skin = GooeySkin {
            text_scale: scale,
            ..default()
        };
        let mut app = app_with(skin);
        spawn_bar(&mut app);
        tap(&mut app, KeyCode::F10);
        taps(
            &mut app,
            &[
                KeyCode::ArrowDown,
                KeyCode::ArrowDown,
                KeyCode::ArrowDown,
                KeyCode::ArrowRight,
            ],
        );
        // The File menu and the submenu are both open.
        let file = title(&mut app, "File");
        let recent = item(&mut app, "Recent");
        assert!(is_open(&app, file) && is_open(&app, recent), "{scale}x");
        for panel in [panel_of(&app, file), panel_of(&app, recent)] {
            let card = rect(&app, panel);
            let mut stack = vec![panel];
            while let Some(e) = stack.pop() {
                if let Some(kids) = app.world().get::<Children>(e) {
                    for k in kids.iter() {
                        let Some(node) = app.world().get::<ComputedNode>(k) else {
                            continue;
                        };
                        // A tooltip box or a nested panel is its own layer.
                        if app.world().get::<MenuPanel>(k).is_some()
                            || app.world().get::<TooltipBox>(k).is_some()
                            || node.is_empty()
                        {
                            continue;
                        }
                        let r = rect(&app, k);
                        assert!(
                            r.min.x >= card.min.x - 1.0
                                && r.max.x <= card.max.x + 1.0
                                && r.min.y >= card.min.y - 1.0
                                && r.max.y <= card.max.y + 1.0,
                            "{scale}x: {r:?} leaves {card:?}"
                        );
                        stack.push(k);
                    }
                }
            }
            // Every label stays on one line.
            for label in texts_under(&app, panel, Role::MenuText) {
                let h = rect(&app, label).height();
                let line = app
                    .world()
                    .get::<ComputedNode>(label)
                    .expect("laid out")
                    .size()
                    .y;
                assert!(
                    h <= line + 1.0 && h < 20.0 * scale * 1.8,
                    "{scale}x: a label wrapped ({h} px)"
                );
            }
        }
    }
}

#[test]
fn only_a_menu_with_a_check_or_a_dot_keeps_the_slot_for_it() {
    let mut app = app();
    spawn_bar(&mut app);
    tap(&mut app, KeyCode::F10);
    taps(&mut app, &[KeyCode::ArrowDown, KeyCode::ArrowDown, KeyCode::ArrowDown, KeyCode::ArrowRight]);
    // File holds Autosave, a check: its rows keep the slot. Recent's rows are plain: they do not.
    let indent = |app: &mut App, label: &str| {
        let text = text_with(app, Role::MenuText, label);
        let row = item(app, label);
        rect(app, text).min.x - rect(app, row).min.x
    };
    let with_slot = indent(&mut app, "New");
    let without = indent(&mut app, "level-1.ron");
    assert!(
        with_slot - without >= 15.0,
        "{with_slot} px in a menu with a check, {without} px in one without"
    );
}
