//! Shortcuts: the chord's words per platform, the conflict check, the one rule for a chord pressed,
//! a binding row that captures a chord, and the keycap a tooltip shows for its command. Laid out
//! for real in a headless window, driven by real keyboard and pointer input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButtonInput;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// What a tooltip's box writes when it is redrawn.
type Written = Or<(Changed<Text>, Changed<Node>, Changed<Tooltip>)>;

/// Every `ValueChange<Binding>` a game would see: the row and the new input.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, Binding)>);

/// How many `Cancel`s the screen heard.
#[derive(Resource, Default)]
struct Cancels(u32);

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
    .init_resource::<Changes>()
    .init_resource::<Cancels>()
    .add_observer(
        |change: On<ValueChange<Binding>>, mut seen: ResMut<Changes>| {
            seen.0.push((change.source, change.value))
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

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

/// One key event, then one update.
fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    let window = window(app);
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

fn down(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Pressed);
}

fn up(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Released);
}

fn tap(app: &mut App, key_code: KeyCode) {
    down(app, key_code);
    up(app, key_code);
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    app.update();
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

fn value(app: &App, row: Entity) -> Binding {
    app.world().get::<BindingValue>(row).expect("a value").0
}

fn waiting(app: &App, row: Entity) -> bool {
    app.world().get::<Capturing>(row).is_some()
}

/// Every text under `root` that is not hidden.
fn texts(app: &App, root: Entity) -> Vec<String> {
    descendants(app.world(), root)
        .into_iter()
        .filter(|&e| app.world().get::<Visibility>(e) != Some(&Visibility::Hidden))
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect()
}

fn rect(app: &App, entity: Entity) -> Rect {
    let size = app
        .world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size();
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    Rect::from_center_size(at, size)
}

const CTRL_SHIFT: Modifiers = Modifiers {
    ctrl: true,
    shift: true,
    alt: false,
    meta: false,
};

/// A chord row bound to Ctrl+P and a plain key row under it, in a panel on a screen that counts the
/// `Cancel`s it hears. The row is focused and ready to start.
fn chord_screen(app: &mut App) -> (Entity, Entity) {
    let world = app.world_mut();
    let palette = world
        .spawn(binding(
            "Palette",
            Binding::Chord(Shortcut::new(Modifiers::CTRL, KeyCode::KeyP)),
        ))
        .id();
    let jump = world
        .spawn(binding("Jump", Binding::Key(KeyCode::Space)))
        .id();
    let card = world.spawn(panel()).add_children(&[palette, jump]).id();
    let screen = world.spawn(screen()).add_child(card).id();
    world
        .entity_mut(screen)
        .observe(|_: On<Cancel>, mut seen: ResMut<Cancels>| seen.0 += 1);
    settle(app);
    focus(app, palette);
    (palette, jump)
}

/// Starts the capture the way a player does: Enter on the focused row, then a frame to arm it.
fn start(app: &mut App, row: Entity) {
    tap(app, KeyCode::Enter);
    settle(app);
    assert!(waiting(app, row), "Enter on a binding row starts a capture");
}

// ---- the value -------------------------------------------------------------------------------

#[test]
fn a_chord_is_spelled_in_each_platforms_order() {
    let everything = Modifiers::CTRL | Modifiers::SHIFT | Modifiers::ALT | Modifiers::META;
    let chord = Shortcut::new(everything, KeyCode::KeyX);
    // Microsoft: Windows logo, Ctrl, Alt, Shift. Apple: Control, Option, Shift, Command.
    assert_eq!(chord.label_on(Platform::Other), "Super+Ctrl+Alt+Shift+X");
    assert_eq!(chord.label_on(Platform::Mac), "⌃⌥⇧⌘X");
    let palette = Shortcut::new(CTRL_SHIFT, KeyCode::KeyP);
    assert_eq!(palette.label_on(Platform::Other), "Ctrl+Shift+P");
    assert_eq!(palette.label_on(Platform::Mac), "⌃⇧P");
    // A key with no modifier is its own name; a mac writes its named keys as glyphs.
    let save_as = Shortcut::new(Modifiers::META | Modifiers::SHIFT, KeyCode::KeyS);
    assert_eq!(save_as.label_on(Platform::Mac), "⇧⌘S");
    assert_eq!(
        Shortcut::new(Modifiers::NONE, KeyCode::F5).label_on(Platform::Other),
        "F5"
    );
    assert_eq!(
        Shortcut::new(Modifiers::META, KeyCode::Backspace).label_on(Platform::Mac),
        "⌘⌫"
    );
    assert_eq!(
        Shortcut::new(Modifiers::CTRL, KeyCode::PageDown).label_on(Platform::Other),
        "Ctrl+Page Down"
    );
}

#[test]
fn primary_is_command_on_a_mac_and_ctrl_elsewhere() {
    assert_eq!(Modifiers::primary_on(Platform::Mac), Modifiers::META);
    assert_eq!(Modifiers::primary_on(Platform::Other), Modifiers::CTRL);
    let save = Shortcut::primary(KeyCode::KeyS);
    // This build's own platform decides `label`, the one a player sees.
    assert_eq!(save.label(), save.label_on(Platform::current()));
    assert_eq!(save.modifiers, Modifiers::primary_on(Platform::current()));
}

#[test]
fn a_modifier_is_not_a_shortcut() {
    assert!(Shortcut::new(Modifiers::CTRL, KeyCode::KeyS).is_valid());
    assert!(!Shortcut::new(Modifiers::NONE, KeyCode::ShiftLeft).is_valid());
    assert!(!Shortcut::new(Modifiers::CTRL, KeyCode::ControlRight).is_valid());
    assert!(!Shortcut::new(Modifiers::NONE, KeyCode::SuperLeft).is_valid());
}

#[test]
fn two_commands_on_ctrl_s_are_one_conflict() {
    let save = Shortcut::new(Modifiers::CTRL, KeyCode::KeyS);
    let save_all = Shortcut::new(CTRL_SHIFT, KeyCode::KeyS);
    let keymap = [
        ("Save", save),
        ("Save all", save_all),
        ("Stash", save),
        ("Open", Shortcut::new(Modifiers::CTRL, KeyCode::KeyO)),
    ];
    assert_eq!(
        model::shortcut_conflicts(&keymap),
        vec![("Save".to_owned(), "Stash".to_owned())],
        "Ctrl+S twice clashes; Ctrl+Shift+S and Ctrl+O clash with nothing"
    );
    let three = [("A", save), ("B", save), ("C", save)];
    assert_eq!(
        model::shortcut_conflicts(&three).len(),
        3,
        "three on one chord are three pairs"
    );
    assert!(model::shortcut_conflicts(&[("Save", save)]).is_empty());
    assert!(model::shortcut_conflicts::<Shortcut>(&[]).is_empty());
}

/// A `ButtonInput` as the app's keyboard system keeps it, with `held` pressed in earlier frames
/// and `now` pressed this frame.
fn input(held: &[KeyCode], now: &[KeyCode]) -> ButtonInput<KeyCode> {
    let mut keys = ButtonInput::<KeyCode>::default();
    for &code in held {
        keys.press(code);
    }
    keys.clear();
    for &code in now {
        keys.press(code);
    }
    keys
}

#[test]
fn a_chord_is_pressed_with_exactly_its_modifiers() {
    let save = Shortcut::new(Modifiers::CTRL, KeyCode::KeyS);
    assert!(save.just_pressed(&input(&[KeyCode::ControlLeft], &[KeyCode::KeyS])));
    assert!(
        save.just_pressed(&input(&[KeyCode::ControlRight], &[KeyCode::KeyS])),
        "either Ctrl counts"
    );
    assert!(
        !save.just_pressed(&input(
            &[KeyCode::ControlLeft, KeyCode::ShiftLeft],
            &[KeyCode::KeyS]
        )),
        "Ctrl+Shift+S is not Ctrl+S"
    );
    assert!(
        !save.just_pressed(&input(&[], &[KeyCode::KeyS])),
        "S alone is not Ctrl+S"
    );
    assert!(
        !save.just_pressed(&input(
            &[KeyCode::ControlLeft, KeyCode::KeyS],
            &[KeyCode::ControlLeft, KeyCode::KeyS]
        )),
        "a key held from an earlier frame was not just pressed"
    );
    let save_all = Shortcut::new(CTRL_SHIFT, KeyCode::KeyS);
    assert!(save_all.just_pressed(&input(
        &[KeyCode::ControlLeft, KeyCode::ShiftRight],
        &[KeyCode::KeyS]
    )));
}

// ---- the chord row -----------------------------------------------------------------------------

#[test]
fn a_chord_row_captures_ctrl_shift_k_pressed_a_key_at_a_time() {
    let mut app = app();
    let (palette, _) = chord_screen(&mut app);
    start(&mut app, palette);
    assert!(
        texts(&app, palette).contains(&BindingSettings::default().chord_prompt()),
        "a chord row asks for a shortcut: {:?}",
        texts(&app, palette)
    );
    down(&mut app, KeyCode::ControlLeft);
    down(&mut app, KeyCode::ShiftLeft);
    assert!(
        waiting(&app, palette),
        "Ctrl and Shift alone are not a shortcut: the row keeps waiting"
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
    down(&mut app, KeyCode::KeyK);
    settle(&mut app);
    let chord = Shortcut::new(CTRL_SHIFT, KeyCode::KeyK);
    assert_eq!(value(&app, palette), Binding::Chord(chord));
    assert!(!waiting(&app, palette));
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(palette, Binding::Chord(chord))],
        "one change, with the chord"
    );
    assert!(
        texts(&app, palette).contains(&chord.label()),
        "the keycap says the chord: {:?}",
        texts(&app, palette)
    );
    assert_eq!(
        focus_of(&app),
        Some(palette),
        "focus stays on the row, ring shown"
    );
}

fn focus_of(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

#[test]
fn a_chord_pressed_in_one_frame_binds_whole() {
    let mut app = app();
    let (palette, _) = chord_screen(&mut app);
    start(&mut app, palette);
    let window = window(&mut app);
    for key_code in [KeyCode::ControlRight, KeyCode::ShiftLeft, KeyCode::KeyK] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
    }
    settle(&mut app);
    assert_eq!(
        value(&app, palette),
        Binding::Chord(Shortcut::new(CTRL_SHIFT, KeyCode::KeyK))
    );
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
}

#[test]
fn a_bare_key_binds_a_chord_with_no_modifier() {
    let mut app = app();
    let (palette, _) = chord_screen(&mut app);
    start(&mut app, palette);
    down(&mut app, KeyCode::F5);
    settle(&mut app);
    assert_eq!(
        value(&app, palette),
        Binding::Chord(Shortcut::new(Modifiers::NONE, KeyCode::F5))
    );
    assert_eq!(value(&app, palette).label(PadStyle::default()), "F5");
}

#[test]
fn the_modifiers_held_are_the_ones_bound_not_the_ones_released() {
    let mut app = app();
    let (palette, _) = chord_screen(&mut app);
    start(&mut app, palette);
    down(&mut app, KeyCode::AltLeft);
    up(&mut app, KeyCode::AltLeft);
    down(&mut app, KeyCode::KeyK);
    settle(&mut app);
    assert_eq!(
        value(&app, palette),
        Binding::Chord(Shortcut::new(Modifiers::NONE, KeyCode::KeyK)),
        "Alt was let go before K: a plain K"
    );
}

#[test]
fn the_same_chord_changes_nothing() {
    let mut app = app();
    let (palette, _) = chord_screen(&mut app);
    start(&mut app, palette);
    down(&mut app, KeyCode::ControlLeft);
    down(&mut app, KeyCode::KeyP);
    settle(&mut app);
    assert!(!waiting(&app, palette));
    assert!(
        app.world().resource::<Changes>().0.is_empty(),
        "Ctrl+P was already the binding"
    );
}

#[test]
fn escape_exits_a_chord_row_and_is_never_bound() {
    let mut app = app();
    let (palette, _) = chord_screen(&mut app);
    start(&mut app, palette);
    down(&mut app, KeyCode::ControlLeft);
    tap(&mut app, KeyCode::Escape);
    settle(&mut app);
    assert!(!waiting(&app, palette));
    assert_eq!(
        value(&app, palette),
        Binding::Chord(Shortcut::new(Modifiers::CTRL, KeyCode::KeyP)),
        "Escape exits, even with Ctrl held"
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert_eq!(
        app.world().resource::<Cancels>().0,
        0,
        "no Cancel for the exit"
    );
}

#[test]
fn a_click_is_not_a_shortcut_and_exits_a_chord_row() {
    let mut app = app();
    let (palette, _) = chord_screen(&mut app);
    start(&mut app, palette);
    let window_entity = window(&mut app);
    let at = Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window_entity)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: Vec2::new(900.0, 600.0),
    };
    app.world_mut().write_message(MouseButtonInput {
        button: MouseButton::Left,
        state: ButtonState::Pressed,
        window: window_entity,
    });
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        at.clone(),
        PointerAction::Press(PointerButton::Primary),
    ));
    app.update();
    settle(&mut app);
    assert!(!waiting(&app, palette), "a mouse press leaves the capture");
    assert_eq!(
        value(&app, palette),
        Binding::Chord(Shortcut::new(Modifiers::CTRL, KeyCode::KeyP))
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn a_key_row_still_binds_a_modifier_alone() {
    let mut app = app();
    let (_, jump) = chord_screen(&mut app);
    focus(&mut app, jump);
    start(&mut app, jump);
    down(&mut app, KeyCode::ShiftLeft);
    settle(&mut app);
    assert_eq!(
        value(&app, jump),
        Binding::Key(KeyCode::ShiftLeft),
        "a sprint key is a key row's to bind"
    );
}

#[test]
fn a_chord_row_holds_the_other_input_systems_still_while_it_waits() {
    let mut app = app();
    let (palette, jump) = chord_screen(&mut app);
    start(&mut app, palette);
    // Tab and the arrows are chords in a chord row, and move no focus.
    down(&mut app, KeyCode::ControlLeft);
    tap(&mut app, KeyCode::ArrowDown);
    settle(&mut app);
    assert_eq!(
        value(&app, palette),
        Binding::Chord(Shortcut::new(Modifiers::CTRL, KeyCode::ArrowDown))
    );
    assert_eq!(
        focus_of(&app),
        Some(palette),
        "the arrow was the binding and did not move focus to {jump:?}"
    );
}

#[test]
fn a_chord_names_itself_on_a_prompt() {
    let mut app = app();
    let chord = Shortcut::new(CTRL_SHIFT, KeyCode::KeyP);
    let hint = app
        .world_mut()
        .spawn(prompt(
            "Palette",
            Prompt {
                keyboard: Binding::Chord(chord),
                pad: Binding::Pad(GamepadButton::North),
            },
        ))
        .id();
    settle(&mut app);
    assert!(
        texts(&app, hint).contains(&chord.label()),
        "the hint says the chord: {:?}",
        texts(&app, hint)
    );
    let cap = descendants(app.world(), hint)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&Role::PromptKey))
        .expect("a keyboard keycap");
    assert!(rect(&app, cap).width() >= rect(&app, cap).height());
}

// ---- the tooltip's keycap ----------------------------------------------------------------------

/// A button with a tooltip over a pause-menu column, the tooltip open.
fn tip_for(app: &mut App, tooltip: Tooltip) -> (Entity, Entity) {
    let world = app.world_mut();
    let save = world.spawn((button("Save"), tooltip)).id();
    let other = world.spawn(button("Quit")).id();
    let column = world.spawn(menu()).add_children(&[save, other]).id();
    let card = world.spawn(panel()).add_child(column).id();
    world.spawn(screen()).add_child(card);
    settle(app);
    focus(app, save);
    settle(app);
    let tip = box_of(app, save);
    (save, tip)
}

fn box_of(app: &App, widget: Entity) -> Entity {
    app.world()
        .get::<Children>(widget)
        .expect("children")
        .iter()
        .find(|&c| app.world().get::<TooltipBox>(c).is_some())
        .expect("a tooltip box")
}

fn cap_of(app: &App, tip: Entity) -> Option<Entity> {
    descendants(app.world(), tip)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&Role::PromptKey))
}

#[test]
fn a_tooltip_shows_its_shortcut_on_a_keycap_beside_the_line() {
    let mut app = app();
    let chord = Shortcut::primary(KeyCode::KeyS);
    let (_, tip) = tip_for(
        &mut app,
        Tooltip::new("Write the world to disk").with_shortcut(chord),
    );
    let node = app.world().get::<Node>(tip).expect("a node");
    assert_ne!(node.display, Display::None, "focus opens the tooltip");
    let cap = cap_of(&app, tip).expect("the shortcut's keycap");
    assert_eq!(app.world().get::<Text>(cap).expect("text").0, chord.label());
    let line = descendants(app.world(), tip)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&Role::Label))
        .expect("the line");
    let (line_rect, cap_rect, box_rect) = (rect(&app, line), rect(&app, cap), rect(&app, tip));
    assert!(
        cap_rect.min.x >= line_rect.max.x,
        "the cap is right of the line: line {line_rect:?}, cap {cap_rect:?}"
    );
    assert!(
        cap_rect.max.x <= box_rect.max.x + 0.5 && cap_rect.max.y <= box_rect.max.y + 0.5,
        "the cap is inside the box: cap {cap_rect:?}, box {box_rect:?}"
    );
    assert!(
        (cap_rect.center().y - line_rect.center().y).abs() <= 1.5,
        "cap and line share a centre line: cap {cap_rect:?}, line {line_rect:?}"
    );
    assert!(
        cap_rect.width() >= cap_rect.height(),
        "a cap at least as wide as high"
    );
}

#[test]
fn a_tooltip_without_a_shortcut_has_no_keycap_and_one_edited_in_gains_and_loses_it() {
    let mut app = app();
    let (save, tip) = tip_for(&mut app, Tooltip::new("Write the world to disk"));
    assert!(cap_of(&app, tip).is_none(), "no shortcut, no cap");

    app.world_mut()
        .get_mut::<Tooltip>(save)
        .expect("tooltip")
        .shortcut = Some(Shortcut::new(CTRL_SHIFT, KeyCode::KeyS));
    settle(&mut app);
    let tip = box_of(&app, save);
    let cap = cap_of(&app, tip).expect("the cap appears with the shortcut");
    assert_eq!(
        app.world().get::<Text>(cap).expect("text").0,
        Shortcut::new(CTRL_SHIFT, KeyCode::KeyS).label()
    );

    app.world_mut()
        .get_mut::<Tooltip>(save)
        .expect("tooltip")
        .shortcut = Some(Shortcut::new(Modifiers::NONE, KeyCode::F2));
    settle(&mut app);
    let caps = descendants(app.world(), tip)
        .into_iter()
        .filter(|&e| app.world().get::<Role>(e) == Some(&Role::PromptKey))
        .count();
    assert_eq!(caps, 1, "a changed shortcut changes the one cap");
    assert_eq!(app.world().get::<Text>(cap).expect("text").0, "F2");

    app.world_mut()
        .get_mut::<Tooltip>(save)
        .expect("tooltip")
        .shortcut = None;
    settle(&mut app);
    assert!(
        cap_of(&app, tip).is_none(),
        "the cap goes with the shortcut"
    );
    assert_eq!(
        box_of(&app, save),
        tip,
        "the box is the same one throughout"
    );
}

#[test]
fn a_long_line_and_a_shortcut_stay_inside_the_window_at_double_size() {
    let mut app = app();
    app.world_mut().resource_mut::<GooeySkin>().text_scale = 2.0;
    let chord = Shortcut::new(CTRL_SHIFT, KeyCode::PageDown);
    let (_, tip) = tip_for(
        &mut app,
        Tooltip::new("Write the world to disk").with_shortcut(chord),
    );
    let cap = cap_of(&app, tip).expect("cap");
    let (cap_rect, box_rect) = (rect(&app, cap), rect(&app, tip));
    let window = Rect::new(0.0, 0.0, 1280.0, 720.0);
    assert!(
        window.contains(box_rect.min) && window.contains(box_rect.max),
        "the box is in the window: {box_rect:?}"
    );
    assert!(
        cap_rect.max.x <= box_rect.max.x + 0.5,
        "the cap is in the box: cap {cap_rect:?}, box {box_rect:?}"
    );
    let ink = app.world().get::<Text>(cap).expect("text").0.clone();
    assert_eq!(ink, chord.label());
}

#[test]
fn a_shortcut_tooltip_writes_nothing_while_idle() {
    let mut app = app();
    let (_, _) = tip_for(
        &mut app,
        Tooltip::new("Write the world to disk").with_shortcut(Shortcut::primary(KeyCode::KeyS)),
    );
    settle(&mut app);
    #[derive(Resource, Default)]
    struct Writes(u32);
    app.init_resource::<Writes>().add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    app.update();
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0);
}

// ---- the host's own spelling (#45) -------------------------------------------------------------

/// Every key `KeyCode` has, as the names `model::shortcut_label` reads.
const EVERY_KEY: [KeyCode; 42] = [
    KeyCode::KeyA,
    KeyCode::KeyZ,
    KeyCode::Digit0,
    KeyCode::Digit9,
    KeyCode::F1,
    KeyCode::F12,
    KeyCode::Enter,
    KeyCode::NumpadEnter,
    KeyCode::Backspace,
    KeyCode::Delete,
    KeyCode::Escape,
    KeyCode::Tab,
    KeyCode::Space,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::PageUp,
    KeyCode::PageDown,
    KeyCode::Home,
    KeyCode::End,
    KeyCode::Insert,
    KeyCode::ContextMenu,
    KeyCode::Backquote,
    KeyCode::Backslash,
    KeyCode::BracketLeft,
    KeyCode::BracketRight,
    KeyCode::Comma,
    KeyCode::Equal,
    KeyCode::Minus,
    KeyCode::Period,
    KeyCode::Quote,
    KeyCode::Semicolon,
    KeyCode::Slash,
    KeyCode::NumpadAdd,
    KeyCode::NumpadSubtract,
    KeyCode::NumpadMultiply,
    KeyCode::NumpadDivide,
    KeyCode::NumpadDecimal,
    KeyCode::NumpadComma,
    KeyCode::NumpadEqual,
    KeyCode::Numpad0,
];

/// Every combination of the four modifiers.
fn every_modifier_combination() -> impl Iterator<Item = Modifiers> {
    (0u8..16).map(|bits| Modifiers {
        ctrl: bits & 1 != 0,
        shift: bits & 2 != 0,
        alt: bits & 4 != 0,
        meta: bits & 8 != 0,
    })
}

#[test]
fn a_macs_chord_in_words_is_ascii_for_every_key_and_modifier() {
    for key in EVERY_KEY {
        for modifiers in every_modifier_combination() {
            let chord = Shortcut::new(modifiers, key);
            let words = chord.label_with(Platform::Mac, ChordGlyphs::Words);
            assert!(
                words.is_ascii(),
                "{key:?} with {modifiers:?} is {words:?} in Words, which a symbol-less face cannot draw"
            );
            assert!(!words.is_empty());
        }
    }
}

#[test]
fn the_symbols_are_still_apples_and_the_words_are_ascii() {
    let palette = Shortcut::new(CTRL_SHIFT, KeyCode::KeyP);
    assert_eq!(palette.label_on(Platform::Mac), "⌃⇧P");
    assert_eq!(
        palette.label_with(Platform::Mac, ChordGlyphs::Words),
        "Ctrl+Shift+P"
    );
    // Apple's order, in words, with Command as "Cmd" and Option as "Opt".
    let all = Shortcut::new(
        Modifiers::CTRL | Modifiers::SHIFT | Modifiers::ALT | Modifiers::META,
        KeyCode::KeyX,
    );
    assert_eq!(all.label_with(Platform::Mac, ChordGlyphs::Words), "Ctrl+Opt+Shift+Cmd+X");
    // The named keys are `key_name`'s words, the ones a prompt and a hint already use.
    assert_eq!(
        Shortcut::new(Modifiers::META, KeyCode::Backspace).label_with(Platform::Mac, ChordGlyphs::Words),
        "Cmd+Backspace"
    );
    assert_eq!(
        Shortcut::new(Modifiers::ALT, KeyCode::Enter).label_with(Platform::Mac, ChordGlyphs::Words),
        "Opt+Enter"
    );
    assert_eq!(
        Shortcut::new(Modifiers::CTRL, KeyCode::Escape).label_with(Platform::Mac, ChordGlyphs::Words),
        "Ctrl+Esc"
    );
    assert_eq!(
        Shortcut::new(Modifiers::NONE, KeyCode::ArrowUp).label_with(Platform::Mac, ChordGlyphs::Words),
        "Up Arrow"
    );
}

#[test]
fn the_spelling_only_changes_a_macs_chord() {
    for key in EVERY_KEY {
        for modifiers in every_modifier_combination() {
            let chord = Shortcut::new(modifiers, key);
            assert_eq!(
                chord.label_with(Platform::Other, ChordGlyphs::Glyphs),
                chord.label_with(Platform::Other, ChordGlyphs::Words),
                "{key:?} with {modifiers:?}: Microsoft's notation is already words"
            );
            // A chord with no modifier has no symbols to replace, on either platform.
            if modifiers.is_empty() {
                assert_eq!(
                    chord.label_with(Platform::Mac, ChordGlyphs::Glyphs),
                    chord.label_with(Platform::Mac, ChordGlyphs::Words),
                    "{key:?} alone"
                );
            }
        }
    }
}

#[test]
fn the_settings_spell_a_chord_and_an_input_in_the_hosts_way() {
    let mac_words = BindingSettings {
        platform: Some(Platform::Mac),
        chord_glyphs: ChordGlyphs::Words,
        ..default()
    };
    assert_eq!(
        mac_words.chord(Shortcut::primary(KeyCode::KeyS)),
        "Cmd+S"
    );
    assert_eq!(
        mac_words.input_label(Binding::Chord(Shortcut::primary(KeyCode::KeyZ))),
        "Cmd+Z"
    );
    // A key, a mouse button and a pad button are unaffected by the spelling.
    assert_eq!(
        mac_words.input_label(Binding::Key(KeyCode::Space)),
        Binding::Key(KeyCode::Space).label(PadStyle::Xbox)
    );
    // The default is this build's platform and Apple's symbols, so nothing changes for a host
    // that sets neither.
    let plain = BindingSettings::default();
    assert_eq!(plain.platform(), Platform::current());
    assert_eq!(
        plain.chord(Shortcut::primary(KeyCode::KeyS)),
        Shortcut::primary(KeyCode::KeyS).label()
    );
    // The prompt that names the exit follows the spelling too.
    let mac_exit = BindingSettings {
        platform: Some(Platform::Mac),
        chord_glyphs: ChordGlyphs::Words,
        ..default()
    };
    assert_eq!(mac_exit.key_prompt(), "Press a key, Esc exits");
}

#[test]
fn a_tooltips_keycap_changes_with_the_spelling_without_being_respawned() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<BindingSettings>()
        .platform = Some(Platform::Mac);
    let chord = Shortcut::primary(KeyCode::KeyS);
    let (_, tip) = tip_for(&mut app, Tooltip::new("Save").with_shortcut(chord));
    let cap = cap_of(&app, tip).expect("a keycap");
    settle(&mut app);
    assert_eq!(
        app.world().get::<Text>(cap).expect("text").0,
        "⌘S",
        "the symbols by default"
    );

    app.world_mut()
        .resource_mut::<BindingSettings>()
        .chord_glyphs = ChordGlyphs::Words;
    settle(&mut app);
    assert_eq!(
        app.world().get::<Text>(cap).expect("text").0,
        "Cmd+S",
        "the same cap, in words"
    );
    assert!(
        cap_of(&app, tip) == Some(cap),
        "the keycap is rewritten, not respawned"
    );

    // And nothing is written once the settings settle.
    settle(&mut app);
    assert_eq!(
        app.world().get::<Text>(cap).expect("text").0,
        "Cmd+S"
    );
}

/// A menu row's chord, and the legend's, both read the host's spelling.
#[test]
fn a_menu_row_and_a_badge_read_the_hosts_spelling() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<BindingSettings>()
        .platform = Some(Platform::Mac);
    let chord = Shortcut::primary(KeyCode::KeyZ);
    let item = app
        .world_mut()
        .spawn(menu_item("Undo").with_shortcut(chord))
        .id();
    let title = app.world_mut().spawn(menu_title("Edit", ())).id();
    let bar = app.world_mut().spawn(menu()).id();
    app.world_mut()
        .entity_mut(bar)
        .add_children(&[title])
        .add_child(item);
    settle(&mut app);
    let row_texts = |app: &App| {
        descendants(app.world(), item)
            .into_iter()
            .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
            .collect::<Vec<_>>()
    };
    assert!(
        row_texts(&app).contains(&"⌘Z".to_owned()),
        "the row says the symbols by default: {:?}",
        row_texts(&app)
    );

    app.world_mut()
        .resource_mut::<BindingSettings>()
        .chord_glyphs = ChordGlyphs::Words;
    settle(&mut app);
    assert!(
        row_texts(&app).contains(&"Cmd+Z".to_owned()),
        "the built row follows the setting: {:?}",
        row_texts(&app)
    );
}
