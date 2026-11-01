//! The binding row: a row that rebinds one action. It keeps its input, waits for the player's next
//! press, takes the keys, the pointer and the pad, holds every other input system still while it
//! waits, and never changes size. Laid out for real in a headless window, driven by real keyboard,
//! pointer and gamepad input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButtonInput;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{InteractionDisabled, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<Binding>` a game would see: the row and the new input.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, Binding)>);

/// Every `UiFeedback` written.
#[derive(Resource, Default)]
struct Feedback(Vec<UiFeedback>);

/// Every button activated.
#[derive(Resource, Default)]
struct Activations(Vec<Entity>);

/// How many `Cancel`s the screen heard.
#[derive(Resource, Default)]
struct Cancels(u32);

/// How many frames' writes the app has seen to what a capture changes.
#[derive(Resource, Default)]
struct Writes(u32);

/// What a capture writes: a keycap's text and visibility, the state, a fill, a ring or a node.
type Written = Or<(
    Changed<Visibility>,
    Changed<Text>,
    Changed<BindingValue>,
    Changed<BackgroundColor>,
    Changed<BorderColor>,
    Changed<Node>,
)>;

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
    .init_resource::<Changes>()
    .init_resource::<Feedback>()
    .init_resource::<Activations>()
    .init_resource::<Cancels>()
    .init_resource::<Writes>()
    .add_observer(
        |change: On<ValueChange<Binding>>, mut seen: ResMut<Changes>| {
            seen.0.push((change.source, change.value))
        },
    )
    .add_observer(|activate: On<Activate>, mut seen: ResMut<Activations>| {
        seen.0.push(activate.entity)
    })
    .add_systems(
        Update,
        (
            |mut messages: MessageReader<UiFeedback>, mut seen: ResMut<Feedback>| {
                seen.0.extend(messages.read().copied());
            },
            |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
                writes.0 += changed.iter().count() as u32
            },
        ),
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

/// A controls screen as a game builds it: a keyboard row, a pad row and a Back button under them,
/// in a panel on a screen that counts the `Cancel`s it hears.
struct Controls {
    card: Entity,
    jump: Entity,
    pad: Entity,
    back: Entity,
}

fn controls(app: &mut App) -> Controls {
    let world = app.world_mut();
    let jump = world
        .spawn(binding("Jump", Binding::Key(KeyCode::Space)))
        .id();
    let pad = world
        .spawn(binding("Use", Binding::Pad(GamepadButton::South)))
        .id();
    let back = world.spawn(button("Back")).id();
    let card = world.spawn(panel()).add_children(&[jump, pad, back]).id();
    let screen = world.spawn(screen()).add_child(card).id();
    world
        .entity_mut(screen)
        .observe(|_: On<Cancel>, mut seen: ResMut<Cancels>| seen.0 += 1);
    settle(app);
    Controls {
        card,
        jump,
        pad,
        back,
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

/// The part of `root` with `role`.
fn part(app: &App, root: Entity, role: Role) -> Entity {
    descendants(app.world(), root)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&role))
        .unwrap_or_else(|| panic!("a {role:?} under the widget"))
}

/// Where `entity` is drawn, in logical px (the window's scale factor and `UiScale` are 1).
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

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

/// A key pressed and released, one update each.
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

/// A mouse button as the platform reports it: the raw button, which `ButtonInput<MouseButton>`
/// keeps, and the pointer's press or release, which picking hears.
fn mouse(app: &mut App, at: Vec2, button: MouseButton, state: ButtonState) {
    let window = window(app);
    let at = location(app, at);
    app.world_mut().write_message(MouseButtonInput {
        button,
        state,
        window,
    });
    // Back, Forward and the rest have no pointer button: picking never hears them.
    let pointer_button = match button {
        MouseButton::Left => Some(PointerButton::Primary),
        MouseButton::Right => Some(PointerButton::Secondary),
        MouseButton::Middle => Some(PointerButton::Middle),
        _ => None,
    };
    if let Some(pointer_button) = pointer_button {
        let action = match state {
            ButtonState::Pressed => PointerAction::Press(pointer_button),
            ButtonState::Released => PointerAction::Release(pointer_button),
        };
        app.world_mut()
            .write_message(PointerInput::new(PointerId::Mouse, at, action));
    }
    app.update();
}

/// A mouse button pressed and released at `at`, the pointer first moved there.
fn click(app: &mut App, at: Vec2, button: MouseButton) {
    point_at(app, at);
    mouse(app, at, button, ButtonState::Pressed);
    mouse(app, at, button, ButtonState::Released);
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

fn pad_button(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
    app.world_mut()
        .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            pad, button, value,
        )));
    app.update();
}

/// Presses and releases `button` on `pad`, one update each.
fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    pad_button(app, pad, button, 1.0);
    pad_button(app, pad, button, 0.0);
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    app.update();
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn visible(app: &App) -> bool {
    app.world().resource::<InputFocusVisible>().0
}

fn value(app: &App, row: Entity) -> Binding {
    app.world().get::<BindingValue>(row).expect("a value").0
}

fn waiting(app: &App, row: Entity) -> bool {
    app.world().get::<Capturing>(row).is_some()
}

/// Whether the transparent node that takes the pointer while a row waits stands.
fn catcher_up(app: &mut App) -> bool {
    app.world_mut()
        .query::<&Name>()
        .iter(app.world())
        .any(|name| name.as_str() == "Binding catcher")
}

/// The texts a keycap shows: exactly one, the name at rest and the prompt while it waits.
fn shown(app: &App, row: Entity) -> Vec<String> {
    descendants(app.world(), row)
        .into_iter()
        .filter(|&e| {
            matches!(
                app.world().get::<Role>(e),
                Some(Role::BindingText | Role::BindingPrompt)
            )
        })
        .filter(|&e| app.world().get::<Visibility>(e) != Some(&Visibility::Hidden))
        .map(|e| app.world().get::<Text>(e).expect("text").0.clone())
        .collect()
}

fn changes(app: &App) -> Vec<Binding> {
    app.world()
        .resource::<Changes>()
        .0
        .iter()
        .map(|&(_, binding)| binding)
        .collect()
}

fn activations_of(app: &App, entity: Entity) -> usize {
    app.world()
        .resource::<Activations>()
        .0
        .iter()
        .filter(|&&e| e == entity)
        .count()
}

fn feedback_activations(app: &App) -> usize {
    app.world()
        .resource::<Feedback>()
        .0
        .iter()
        .filter(|f| f.kind == UiFeedbackKind::Activate)
        .count()
}

fn cancels(app: &App) -> u32 {
    app.world().resource::<Cancels>().0
}

/// Focuses `row` and starts a capture with Enter.
fn begin(app: &mut App, row: Entity) {
    focus(app, row);
    tap(app, KeyCode::Enter);
    assert!(waiting(app, row), "Enter starts a capture");
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

fn background(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

fn text_colour(app: &App, entity: Entity) -> Color {
    app.world().get::<TextColor>(entity).expect("a colour").0
}

// ---- the state ----

#[test]
fn a_binding_shows_its_input_in_the_vocabulary_of_its_kind() {
    let mut app = app();
    let ui = controls(&mut app);
    assert_eq!(shown(&app, ui.jump), ["Space"]);
    assert_eq!(shown(&app, ui.pad), ["A"]);
    assert_eq!(value(&app, ui.jump), Binding::Key(KeyCode::Space));
    assert!(!waiting(&app, ui.jump));
    assert!(changes(&app).is_empty(), "nothing says a change at spawn");
}

#[test]
fn a_value_the_game_sets_shows_at_once_and_says_nothing() {
    let mut app = app();
    let ui = controls(&mut app);
    app.world_mut().get_mut::<BindingValue>(ui.jump).unwrap().0 =
        Binding::Mouse(MouseButton::Right);
    app.update();
    assert_eq!(shown(&app, ui.jump), ["Right Click"]);
    app.world_mut().get_mut::<BindingValue>(ui.pad).unwrap().0 =
        Binding::Pad(GamepadButton::LeftTrigger2);
    app.update();
    assert_eq!(shown(&app, ui.pad), ["LT"]);
    assert!(
        changes(&app).is_empty(),
        "a change the game made is its own"
    );
}

#[test]
fn enter_space_south_and_a_click_start_a_capture_and_that_press_is_not_the_binding() {
    let mut app = app();
    let ui = controls(&mut app);

    // Enter, the key the player would also like to bind.
    begin(&mut app, ui.jump);
    assert_eq!(shown(&app, ui.jump), ["Press a key, Esc exits"]);
    settle(&mut app);
    assert!(waiting(&app, ui.jump), "it keeps waiting");
    assert_eq!(value(&app, ui.jump), Binding::Key(KeyCode::Space));
    assert!(changes(&app).is_empty());
    tap(&mut app, KeyCode::Escape);
    assert!(!waiting(&app, ui.jump));

    // Space.
    tap(&mut app, KeyCode::Space);
    assert!(waiting(&app, ui.jump), "Space starts a capture too");
    assert_eq!(value(&app, ui.jump), Binding::Key(KeyCode::Space));
    tap(&mut app, KeyCode::Escape);

    // A click on the row, with the mouse's own button: it starts, it is not what is bound.
    let at = rect(&app, ui.jump).center();
    click(&mut app, at, MouseButton::Left);
    assert!(waiting(&app, ui.jump), "a click starts a capture");
    assert_eq!(value(&app, ui.jump), Binding::Key(KeyCode::Space));
    assert!(changes(&app).is_empty());
    tap(&mut app, KeyCode::Escape);

    // South on the pad row.
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.pad);
    press(&mut app, pad, GamepadButton::South);
    assert!(waiting(&app, ui.pad), "South starts a capture");
    assert_eq!(shown(&app, ui.pad), ["Press a button, Menu exits"]);
    settle(&mut app);
    assert_eq!(value(&app, ui.pad), Binding::Pad(GamepadButton::South));
    assert!(
        changes(&app).is_empty(),
        "the South that began it is not the binding"
    );
}

#[test]
fn a_key_binds_once_and_the_row_says_so() {
    let mut app = app();
    let ui = controls(&mut app);
    begin(&mut app, ui.jump);
    tap(&mut app, KeyCode::KeyQ);
    assert_eq!(changes(&app), [Binding::Key(KeyCode::KeyQ)]);
    assert_eq!(
        app.world().resource::<Changes>().0[0].0,
        ui.jump,
        "from the row"
    );
    assert_eq!(value(&app, ui.jump), Binding::Key(KeyCode::KeyQ));
    assert!(!waiting(&app, ui.jump));
    assert_eq!(
        shown(&app, ui.jump),
        ["Q"],
        "the keycap names the new input"
    );
    assert_eq!(focused(&app), Some(ui.jump), "focus stays on the row");
    assert!(visible(&app), "and its ring is shown");
    // The next press is nobody's.
    tap(&mut app, KeyCode::KeyW);
    assert_eq!(changes(&app).len(), 1);
    assert_eq!(value(&app, ui.jump), Binding::Key(KeyCode::KeyQ));
}

#[test]
fn escape_exits_with_no_change_and_no_cancel_and_back_works_again_after() {
    let mut app = app();
    let ui = controls(&mut app);
    begin(&mut app, ui.jump);
    tap(&mut app, KeyCode::Escape);
    assert!(!waiting(&app, ui.jump));
    assert!(changes(&app).is_empty());
    assert_eq!(value(&app, ui.jump), Binding::Key(KeyCode::Space));
    assert_eq!(cancels(&app), 0, "the Escape that exits is not a Back");
    assert_eq!(shown(&app, ui.jump), ["Space"]);
    assert_eq!(focused(&app), Some(ui.jump));
    // With nothing waiting it is the crate's Back again.
    tap(&mut app, KeyCode::Escape);
    assert_eq!(cancels(&app), 1);
}

#[test]
fn the_input_the_row_already_holds_ends_the_capture_with_no_change() {
    let mut app = app();
    let ui = controls(&mut app);
    begin(&mut app, ui.jump);
    tap(&mut app, KeyCode::Space);
    assert!(
        !waiting(&app, ui.jump),
        "Space is bound again, not restarted"
    );
    assert!(changes(&app).is_empty());
    assert_eq!(value(&app, ui.jump), Binding::Key(KeyCode::Space));
}

#[test]
fn tab_the_arrows_enter_and_the_page_keys_are_bound_like_any_key_and_move_nothing() {
    let mut app = app();
    let ui = controls(&mut app);
    for key in [
        KeyCode::Tab,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::Enter,
        KeyCode::PageDown,
        KeyCode::Home,
        KeyCode::ShiftLeft,
    ] {
        app.world_mut().get_mut::<BindingValue>(ui.jump).unwrap().0 = Binding::Key(KeyCode::Space);
        app.world_mut().resource_mut::<Changes>().0.clear();
        begin(&mut app, ui.jump);
        tap(&mut app, key);
        assert_eq!(changes(&app), [Binding::Key(key)], "{key:?} binds");
        assert_eq!(value(&app, ui.jump), Binding::Key(key));
        assert!(!waiting(&app, ui.jump), "{key:?} ends the capture");
        assert_eq!(focused(&app), Some(ui.jump), "{key:?} moves no focus");
        assert_eq!(
            activations_of(&app, ui.back),
            0,
            "{key:?} presses nothing under it"
        );
    }
}

#[test]
fn a_disabled_row_starts_nothing() {
    let mut app = app();
    let ui = controls(&mut app);
    app.world_mut()
        .entity_mut(ui.jump)
        .insert(InteractionDisabled);
    settle(&mut app);
    focus(&mut app, ui.jump);
    tap(&mut app, KeyCode::Enter);
    tap(&mut app, KeyCode::Space);
    let at = rect(&app, ui.jump).center();
    click(&mut app, at, MouseButton::Left);
    assert!(!waiting(&app, ui.jump));
    assert!(changes(&app).is_empty());
    let skin = skin();
    let name = part(&app, ui.jump, Role::BindingText);
    assert_eq!(text_colour(&app, name), skin.text_muted, "it reads dim");
    let cap = part(&app, ui.jump, Role::BindingCap);
    assert_eq!(background(&app, cap), skin.surface, "and flat");
}

fn skin() -> GooeySkin {
    GooeySkin::default()
}

// ---- the pointer ----

#[test]
fn a_mouse_button_binds_and_the_primary_one_presses_nothing_under_it() {
    let mut app = app();
    let ui = controls(&mut app);
    begin(&mut app, ui.jump);
    settle(&mut app);
    assert!(
        catcher_up(&mut app),
        "the pointer is taken while a row waits"
    );
    // The primary button, over the Back button.
    let back = rect(&app, ui.back).center();
    point_at(&mut app, back);
    mouse(&mut app, back, MouseButton::Left, ButtonState::Pressed);
    assert_eq!(changes(&app), [Binding::Mouse(MouseButton::Left)]);
    mouse(&mut app, back, MouseButton::Left, ButtonState::Released);
    settle(&mut app);
    assert_eq!(
        activations_of(&app, ui.back),
        0,
        "the click landed on the catcher, not the button"
    );
    assert_eq!(shown(&app, ui.jump), ["Left Click"]);
    assert_eq!(focused(&app), Some(ui.jump), "focus is the row's again");
    assert!(!visible(&app), "a mouse press leaves the ring unseen");
    assert!(
        !catcher_up(&mut app),
        "and the pointer is the interface's again"
    );
    // With the catcher gone the same click presses the button.
    click(&mut app, back, MouseButton::Left);
    assert_eq!(activations_of(&app, ui.back), 1);
}

#[test]
fn the_other_mouse_buttons_bind_and_the_row_names_them() {
    let mut app = app();
    let ui = controls(&mut app);
    let nowhere = Vec2::new(5.0, 5.0);
    for (button, name) in [
        (MouseButton::Right, "Right Click"),
        (MouseButton::Middle, "Middle Click"),
        (MouseButton::Back, "Mouse 4"),
        (MouseButton::Forward, "Mouse 5"),
    ] {
        begin(&mut app, ui.jump);
        settle(&mut app);
        mouse(&mut app, nowhere, button, ButtonState::Pressed);
        mouse(&mut app, nowhere, button, ButtonState::Released);
        settle(&mut app);
        assert_eq!(value(&app, ui.jump), Binding::Mouse(button));
        assert_eq!(shown(&app, ui.jump), [name]);
    }
    assert_eq!(changes(&app).len(), 4);
}

#[test]
fn the_catcher_stands_exactly_while_a_row_waits() {
    let mut app = app();
    let ui = controls(&mut app);
    assert!(!catcher_up(&mut app), "none at rest");
    begin(&mut app, ui.jump);
    settle(&mut app);
    assert!(catcher_up(&mut app));
    tap(&mut app, KeyCode::Escape);
    assert!(!catcher_up(&mut app), "gone with the capture");
    // And with the row, if the game despawns it mid-capture.
    begin(&mut app, ui.jump);
    settle(&mut app);
    assert!(catcher_up(&mut app));
    app.world_mut().entity_mut(ui.jump).despawn();
    settle(&mut app);
    assert!(!catcher_up(&mut app), "a despawned row leaves no catcher");
}

#[test]
fn focus_the_game_takes_elsewhere_ends_a_capture_without_a_binding() {
    let mut app = app();
    let ui = controls(&mut app);
    begin(&mut app, ui.jump);
    settle(&mut app);
    focus(&mut app, ui.back);
    settle(&mut app);
    assert!(!waiting(&app, ui.jump));
    assert!(changes(&app).is_empty());
    assert_eq!(
        focused(&app),
        Some(ui.back),
        "focus is left where the game put it"
    );
}

// ---- the pad ----

#[test]
fn a_pad_row_binds_any_pad_button_but_start_and_names_it() {
    let mut app = app();
    let ui = controls(&mut app);
    let pad = plug_in_pad(&mut app);
    for (button, name) in [
        (GamepadButton::East, "B"),
        (GamepadButton::LeftTrigger2, "LT"),
        (GamepadButton::DPadUp, "D-pad Up"),
        (GamepadButton::RightThumb, "RS"),
    ] {
        focus(&mut app, ui.pad);
        press(&mut app, pad, GamepadButton::South);
        assert!(waiting(&app, ui.pad));
        settle(&mut app);
        press(&mut app, pad, button);
        assert_eq!(value(&app, ui.pad), Binding::Pad(button), "{button:?}");
        assert_eq!(shown(&app, ui.pad), [name]);
        assert!(!waiting(&app, ui.pad));
        assert_eq!(focused(&app), Some(ui.pad));
    }
    assert_eq!(changes(&app).len(), 4);

    // Start exits, and is nothing to bind.
    press(&mut app, pad, GamepadButton::South);
    settle(&mut app);
    press(&mut app, pad, GamepadButton::Start);
    assert!(!waiting(&app, ui.pad));
    assert_eq!(changes(&app).len(), 4);
    assert_eq!(cancels(&app), 0, "a pad's B binds, it is not a Back");
    assert_eq!(value(&app, ui.pad), Binding::Pad(GamepadButton::RightThumb));
}

#[test]
fn the_wrong_device_exits_a_capture_and_binds_nothing() {
    let mut app = app();
    let ui = controls(&mut app);
    let pad = plug_in_pad(&mut app);
    // A pad button on a keyboard row.
    begin(&mut app, ui.jump);
    settle(&mut app);
    press(&mut app, pad, GamepadButton::East);
    assert!(!waiting(&app, ui.jump));
    // A key on a pad row.
    focus(&mut app, ui.pad);
    press(&mut app, pad, GamepadButton::South);
    settle(&mut app);
    tap(&mut app, KeyCode::KeyF);
    assert!(!waiting(&app, ui.pad));
    // A mouse button on a pad row.
    press(&mut app, pad, GamepadButton::South);
    settle(&mut app);
    let nowhere = Vec2::new(5.0, 5.0);
    mouse(&mut app, nowhere, MouseButton::Right, ButtonState::Pressed);
    mouse(&mut app, nowhere, MouseButton::Right, ButtonState::Released);
    settle(&mut app);
    assert!(!waiting(&app, ui.pad));
    assert!(changes(&app).is_empty(), "nothing was bound");
    assert_eq!(value(&app, ui.jump), Binding::Key(KeyCode::Space));
    assert_eq!(value(&app, ui.pad), Binding::Pad(GamepadButton::South));
    assert_eq!(focused(&app), Some(ui.pad));
}

// ---- nothing else hears the player ----

#[test]
fn while_a_row_waits_the_d_pad_moves_no_focus_and_it_moves_it_again_after() {
    let mut app = app();
    let ui = controls(&mut app);
    let pad = plug_in_pad(&mut app);
    begin(&mut app, ui.jump);
    settle(&mut app);
    // The D-pad is the wrong device for a keyboard row: it exits, and does not also move focus.
    press(&mut app, pad, GamepadButton::DPadDown);
    assert!(!waiting(&app, ui.jump));
    assert_eq!(
        focused(&app),
        Some(ui.jump),
        "the press that exits moves nothing"
    );
    // Now it is navigation again.
    press(&mut app, pad, GamepadButton::DPadDown);
    assert_ne!(focused(&app), Some(ui.jump), "the next one moves focus");

    // On a pad row the D-pad is a binding, and moves nothing either.
    focus(&mut app, ui.pad);
    press(&mut app, pad, GamepadButton::South);
    settle(&mut app);
    press(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(value(&app, ui.pad), Binding::Pad(GamepadButton::DPadDown));
    assert_eq!(
        focused(&app),
        Some(ui.pad),
        "D-pad Down is the binding, not a move"
    );
}

#[test]
fn a_capture_is_the_one_thing_a_player_does_until_it_ends_south_is_not_a_second_start() {
    let mut app = app();
    let ui = controls(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.pad);
    press(&mut app, pad, GamepadButton::South);
    settle(&mut app);
    // South is a button like any other while a pad row waits: it binds, once.
    press(&mut app, pad, GamepadButton::East);
    press(&mut app, pad, GamepadButton::South);
    assert!(waiting(&app, ui.pad), "this South starts a second capture");
    settle(&mut app);
    press(&mut app, pad, GamepadButton::South);
    assert!(!waiting(&app, ui.pad), "and this one binds South again");
    assert_eq!(
        changes(&app),
        [
            Binding::Pad(GamepadButton::East),
            Binding::Pad(GamepadButton::South)
        ]
    );
}

// ---- feedback ----

#[test]
fn a_capture_writes_feedback_when_it_starts_and_when_it_binds_and_nothing_for_an_exit() {
    let mut app = app();
    let ui = controls(&mut app);
    focus(&mut app, ui.jump);
    let before = feedback_activations(&app);
    tap(&mut app, KeyCode::Enter);
    assert_eq!(feedback_activations(&app) - before, 1, "the start");
    tap(&mut app, KeyCode::KeyQ);
    assert_eq!(feedback_activations(&app) - before, 2, "the bind");
    // Enter bound, as the Activate it also makes is not a third.
    tap(&mut app, KeyCode::Enter);
    tap(&mut app, KeyCode::Enter);
    assert_eq!(
        feedback_activations(&app) - before,
        4,
        "a start and the Enter bound"
    );
    // An exit.
    tap(&mut app, KeyCode::Enter);
    assert_eq!(feedback_activations(&app) - before, 5);
    tap(&mut app, KeyCode::Escape);
    assert_eq!(feedback_activations(&app) - before, 5, "an exit is silent");
}

// ---- the settings ----

#[test]
fn the_pad_style_names_the_buttons_and_the_exit_and_the_prompts_are_the_hosts() {
    let mut app = app();
    let ui = controls(&mut app);
    app.world_mut().resource_mut::<BindingSettings>().pad_style = PadStyle::PlayStation;
    app.world_mut().resource_mut::<BindingSettings>().pad_prompt =
        "Appuyez sur un bouton, {exit} pour quitter".into();
    settle(&mut app);
    assert_eq!(shown(&app, ui.pad), ["Cross"]);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.pad);
    press(&mut app, pad, GamepadButton::South);
    assert_eq!(
        shown(&app, ui.pad),
        ["Appuyez sur un bouton, Options pour quitter"]
    );
}

#[test]
fn a_game_that_moves_the_exit_can_bind_escape() {
    let mut app = app();
    let ui = controls(&mut app);
    app.world_mut().resource_mut::<BindingSettings>().cancel_key = KeyCode::Backspace;
    settle(&mut app);
    begin(&mut app, ui.jump);
    assert_eq!(shown(&app, ui.jump), ["Press a key, Backspace exits"]);
    tap(&mut app, KeyCode::Escape);
    assert_eq!(value(&app, ui.jump), Binding::Key(KeyCode::Escape));
    assert_eq!(shown(&app, ui.jump), ["Esc"]);
    begin(&mut app, ui.jump);
    tap(&mut app, KeyCode::Backspace);
    assert!(!waiting(&app, ui.jump));
    assert_eq!(changes(&app).len(), 1, "Backspace was the exit");
}

// ---- the look ----

#[test]
fn waiting_moves_nothing_at_any_scale() {
    for (width, height, text_scale) in [(1280, 720, 1.0), (1280, 720, 1.5), (2560, 1440, 1.0)] {
        let mut app = app_sized(width, height);
        app.insert_resource(GooeySkin {
            text_scale,
            ..GooeySkin::default()
        });
        let ui = controls(&mut app);
        let caps = [
            part(&app, ui.jump, Role::BindingCap),
            part(&app, ui.pad, Role::BindingCap),
        ];
        let measure = |app: &App| {
            [ui.card, ui.jump, ui.pad, ui.back]
                .into_iter()
                .chain(caps)
                .map(|e| rect(app, e))
                .collect::<Vec<Rect>>()
        };
        let at_rest = measure(&app);
        begin(&mut app, ui.jump);
        settle(&mut app);
        let waiting_jump = measure(&app);
        tap(&mut app, KeyCode::KeyQ);
        settle(&mut app);
        let bound = measure(&app);
        for (i, rest) in at_rest.iter().enumerate() {
            for (name, now) in [("waiting", &waiting_jump), ("bound", &bound)] {
                let now = now[i];
                assert!(
                    (rest.min - now.min).abs().max_element() < 0.01
                        && (rest.max - now.max).abs().max_element() < 0.01,
                    "part {i} moved when {name} at {width}x{height}, text x{text_scale}: {rest:?} to {now:?}"
                );
            }
        }
        // And the pad row's capture.
        let pad = plug_in_pad(&mut app);
        focus(&mut app, ui.pad);
        press(&mut app, pad, GamepadButton::South);
        settle(&mut app);
        let waiting_pad = measure(&app);
        for (i, rest) in at_rest.iter().enumerate() {
            let now = waiting_pad[i];
            assert!(
                (rest.min - now.min).abs().max_element() < 0.01
                    && (rest.max - now.max).abs().max_element() < 0.01,
                "part {i} moved when the pad row waited at {width}x{height}, text x{text_scale}"
            );
        }
    }
}

#[test]
fn the_keycaps_line_up_the_row_is_a_target_a_thumb_can_hit_and_a_toggles_height() {
    for (width, height, scale) in [(1280, 720, 1.0), (2560, 1440, 2.0)] {
        let mut app = app_sized(width, height);
        let ui = controls(&mut app);
        let switch = app.world_mut().spawn(toggle("Subtitles", true)).id();
        app.world_mut().entity_mut(ui.card).add_child(switch);
        settle(&mut app);
        let jump = part(&app, ui.jump, Role::BindingCap);
        let pad = part(&app, ui.pad, Role::BindingCap);
        let (j, p) = (rect(&app, jump), rect(&app, pad));
        // The row is the target (the keycap lets the pointer through to it).
        let row = rect(&app, ui.jump);
        assert!(
            row.height() >= 44.0 * scale - 0.5,
            "a row {} px high at {width}x{height}",
            row.height()
        );
        // And the focus ring, the row's own border, never touches the keycap: 4 px of air.
        assert!(
            j.min.y - row.min.y >= (3.0 + 4.0) * scale - 1.0
                && row.max.y - j.max.y >= (3.0 + 4.0) * scale - 1.0,
            "the keycap {j:?} is too near the row {row:?}"
        );
        assert!(
            (j.min.x - p.min.x).abs() < 0.01 && (j.max.x - p.max.x).abs() < 0.01,
            "two rows' keycaps share their edges: {j:?} and {p:?}"
        );
        let row = rect(&app, ui.jump).height();
        let toggle_row = rect(&app, switch).height();
        assert!(
            (row - toggle_row).abs() <= 1.5 * scale,
            "a binding row is {row} px and a toggle's {toggle_row}"
        );
        // The row's edges are the panel's inside, as the toggle's.
        let (r, t) = (rect(&app, ui.jump), rect(&app, switch));
        assert!((r.min.x - t.min.x).abs() < 0.01 && (r.max.x - t.max.x).abs() < 0.01);
    }
}

#[test]
fn the_keycap_reads_in_both_presets_at_rest_and_lit() {
    for (name, skin) in [
        ("dark", GooeySkin::dark()),
        ("parchment", GooeySkin::parchment()),
    ] {
        let mut app = app();
        app.insert_resource(skin.clone());
        let ui = controls(&mut app);
        let cap = part(&app, ui.jump, Role::BindingCap);
        let text = part(&app, ui.jump, Role::BindingText);
        let prompt = part(&app, ui.jump, Role::BindingPrompt);
        let panel_fill = {
            let surface = Srgba::from(skin.surface);
            composite(rgb(skin.surface), surface.alpha, rgb(skin.background))
        };
        // At rest: the name on its keycap, and the keycap's frame on the panel.
        let rest = contrast_ratio(rgb(text_colour(&app, text)), rgb(background(&app, cap)));
        assert!(
            rest >= TEXT_CONTRAST,
            "{name}: the name is {rest:.2}:1 on its keycap"
        );
        let frame = app.world().get::<BorderColor>(cap).expect("a frame").top;
        let framed = contrast_ratio(rgb(frame), panel_fill);
        assert!(
            framed >= NON_TEXT_CONTRAST,
            "{name}: the frame is {framed:.2}:1 on the panel"
        );

        // Waiting: the prompt on the lit keycap, and the lit keycap on the panel.
        begin(&mut app, ui.jump);
        settle(&mut app);
        let lit = background(&app, cap);
        assert_eq!(lit, skin.accent, "{name}: lit by the accent");
        let prompt_ratio = contrast_ratio(rgb(text_colour(&app, prompt)), rgb(lit));
        assert!(
            prompt_ratio >= TEXT_CONTRAST,
            "{name}: the prompt is {prompt_ratio:.2}:1 on the lit keycap"
        );
        let on_panel = contrast_ratio(rgb(lit), panel_fill);
        assert!(
            on_panel >= NON_TEXT_CONTRAST,
            "{name}: the lit keycap is {on_panel:.2}:1 on the panel"
        );
        assert!(
            skin.problems().is_empty(),
            "{name}: the preset has no problem"
        );
    }
}

#[test]
fn the_focused_row_has_the_ring_and_a_hovered_one_the_outline() {
    let mut app = app();
    let ui = controls(&mut app);
    let skin = skin();
    let ring = |app: &App, row: Entity| app.world().get::<BorderColor>(row).expect("a ring").top;
    assert_eq!(ring(&app, ui.jump), Color::NONE, "nothing at rest");
    focus(&mut app, ui.jump);
    settle(&mut app);
    assert_eq!(
        ring(&app, ui.jump),
        skin.focus_ring,
        "the ring when focused"
    );
    let at = rect(&app, ui.pad).center();
    point_at(&mut app, at);
    settle(&mut app);
    assert_eq!(
        ring(&app, ui.pad),
        skin.outline,
        "the outline under the pointer"
    );
    // While the row waits the ring is still there, round the lit keycap.
    begin(&mut app, ui.jump);
    settle(&mut app);
    assert_eq!(ring(&app, ui.jump), skin.focus_ring);
}

// ---- cost ----

#[test]
fn idle_rows_write_nothing() {
    let mut app = app();
    let ui = controls(&mut app);
    focus(&mut app, ui.jump);
    settle(&mut app);
    app.world_mut().insert_resource(Writes::default());
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<Writes>().0,
        0,
        "no keycap was written"
    );
    // And a row that waits writes nothing while it waits.
    begin(&mut app, ui.jump);
    settle(&mut app);
    app.world_mut().insert_resource(Writes::default());
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<Writes>().0,
        0,
        "a waiting row costs no writes"
    );
}

#[test]
fn a_binding_has_a_role_for_every_part() {
    let mut app = app();
    let ui = controls(&mut app);
    let row = ui.jump;
    assert_eq!(app.world().get::<Role>(row), Some(&Role::Binding));
    for role in [
        Role::BindingCap,
        Role::BindingText,
        Role::BindingPrompt,
        Role::ControlText,
    ] {
        part(&app, row, role);
    }
}
