//! Prompts: the hints that say which input does what. They name the input of the device the player
//! used last, stand in the bottom-left corner without moving anything, and are neither a tab stop
//! nor in the pointer's way. Laid out for real in a headless window, driven by real keyboard,
//! pointer and gamepad input.

use std::collections::HashSet;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadAxis, GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
    RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButtonInput;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// How many frames' writes the app has seen to what a legend's look changes.
#[derive(Resource, Default)]
struct Writes(u32);

/// What a prompt writes: a keycap's words and shape, a fill, a frame or a node.
type Written = Or<(
    Changed<Text>,
    Changed<Role>,
    Changed<Prompt>,
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
    .init_resource::<Writes>()
    .add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
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

/// A screen as a game builds one: a panel, and the legend with Select and Back.
struct Ui {
    panel: Entity,
    legend: Entity,
}

fn legend_of(world: &mut World) -> Entity {
    world
        .spawn((
            prompts(),
            children![
                prompt("Select", Prompt::SELECT),
                prompt("Back", Prompt::BACK)
            ],
        ))
        .id()
}

fn ui(app: &mut App) -> Ui {
    let world = app.world_mut();
    let panel = world
        .spawn((panel(), children![heading("Settings"), button("Play")]))
        .id();
    let legend = legend_of(world);
    world.spawn(screen()).add_children(&[panel, legend]);
    settle(app);
    Ui { panel, legend }
}

/// The keycaps of a legend in order, each with its shape and its words.
fn caps(app: &App, legend: Entity) -> Vec<(Role, String)> {
    let world = app.world();
    let mut found = Vec::new();
    for prompt in world.get::<Children>(legend).expect("prompts").iter() {
        for part in world
            .get::<Children>(prompt)
            .expect("a prompt's parts")
            .iter()
        {
            if let Some(&role @ (Role::PromptKey | Role::PromptButton)) = world.get::<Role>(part) {
                found.push((role, world.get::<Text>(part).expect("words").0.clone()));
            }
        }
    }
    found
}

fn names(app: &App, legend: Entity) -> Vec<String> {
    caps(app, legend)
        .into_iter()
        .map(|(_, name)| name)
        .collect()
}

fn shapes(app: &App, legend: Entity) -> Vec<Role> {
    caps(app, legend)
        .into_iter()
        .map(|(role, _)| role)
        .collect()
}

/// The part of `root` with `role`, the first in tree order.
fn part(app: &App, root: Entity, role: Role) -> Entity {
    let world = app.world();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if world.get::<Role>(entity) == Some(&role) {
            return entity;
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().rev());
        }
    }
    panic!("a {role:?} under the widget");
}

/// Where `entity` is drawn, in physical px (the window's scale factor is overridden to 1).
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

/// A pad button pressed and released, one update each.
fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
}

fn push_stick(app: &mut App, pad: Entity, value: f32) {
    app.world_mut()
        .write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
            pad,
            GamepadAxis::LeftStickX,
            value,
        )));
    app.update();
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

fn point_at(app: &mut App, at: Vec2) {
    let at = location(app, at);
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        at,
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    for _ in 0..2 {
        app.update();
    }
}

fn hovered(app: &App) -> HashSet<Entity> {
    app.world()
        .resource::<HoverMap>()
        .get(&PointerId::Mouse)
        .expect("the mouse has a hover set")
        .keys()
        .copied()
        .collect()
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

fn fill(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

fn text_colour(app: &App, entity: Entity) -> Color {
    app.world().get::<TextColor>(entity).expect("a colour").0
}

// ---- what the keycaps say ----

#[test]
fn a_fresh_legend_names_the_keyboards_inputs() {
    let mut app = app();
    let ui = ui(&mut app);
    assert_eq!(names(&app, ui.legend), ["Enter", "Esc"]);
    assert_eq!(shapes(&app, ui.legend), [Role::PromptKey, Role::PromptKey]);
    assert_eq!(
        *app.world().resource::<InputDevice>(),
        InputDevice::Keyboard
    );
}

#[test]
fn a_pad_button_turns_the_legend_to_the_pad_and_a_key_or_a_click_turns_it_back() {
    let mut app = app();
    let ui = ui(&mut app);
    let pad = plug_in_pad(&mut app);

    // A button no widget acts on (West): picking up the pad is all that happens.
    press(&mut app, pad, GamepadButton::West);
    assert_eq!(names(&app, ui.legend), ["A", "B"]);
    assert_eq!(
        shapes(&app, ui.legend),
        [Role::PromptButton, Role::PromptButton]
    );

    // A key that moves nothing (a letter) is the keyboard again.
    tap(&mut app, KeyCode::KeyQ);
    assert_eq!(names(&app, ui.legend), ["Enter", "Esc"]);
    assert_eq!(shapes(&app, ui.legend), [Role::PromptKey, Role::PromptKey]);

    // And a mouse button is the keyboard and mouse as well.
    press(&mut app, pad, GamepadButton::West);
    assert_eq!(names(&app, ui.legend), ["A", "B"]);
    let window = window(&mut app);
    app.world_mut().write_message(MouseButtonInput {
        button: MouseButton::Right,
        state: ButtonState::Pressed,
        window,
    });
    app.update();
    assert_eq!(names(&app, ui.legend), ["Enter", "Esc"]);
}

#[test]
fn a_mouse_that_only_moves_does_not_take_the_pad_players_hints() {
    let mut app = app();
    let ui = ui(&mut app);
    let pad = plug_in_pad(&mut app);
    press(&mut app, pad, GamepadButton::West);
    point_at(&mut app, Vec2::new(300.0, 300.0));
    point_at(&mut app, Vec2::new(500.0, 200.0));
    assert_eq!(names(&app, ui.legend), ["A", "B"]);
}

#[test]
fn a_stick_pushed_hard_is_the_pad_and_a_nudge_is_not() {
    let mut app = app();
    let ui = ui(&mut app);
    let pad = plug_in_pad(&mut app);
    push_stick(&mut app, pad, 0.3);
    assert_eq!(
        names(&app, ui.legend),
        ["Enter", "Esc"],
        "a nudge is no one picking up the pad"
    );
    push_stick(&mut app, pad, 0.9);
    assert_eq!(names(&app, ui.legend), ["A", "B"]);
    // Back to rest alone does not give the keyboard back; a key does.
    push_stick(&mut app, pad, 0.0);
    assert_eq!(names(&app, ui.legend), ["A", "B"]);
}

#[test]
fn the_pad_buttons_are_named_in_the_style_the_game_chose() {
    let mut app = app();
    let ui = ui(&mut app);
    app.world_mut().resource_mut::<BindingSettings>().pad_style = PadStyle::PlayStation;
    let pad = plug_in_pad(&mut app);
    press(&mut app, pad, GamepadButton::West);
    assert_eq!(names(&app, ui.legend), ["Cross", "Circle"]);
    // The keyboard's names do not depend on the pad's make.
    tap(&mut app, KeyCode::KeyQ);
    assert_eq!(names(&app, ui.legend), ["Enter", "Esc"]);
}

#[test]
fn a_game_that_sets_the_device_or_a_prompt_sees_it_in_the_same_frame() {
    let mut app = app();
    let ui = ui(&mut app);

    // A console or a handheld starts on the pad.
    app.world_mut().insert_resource(InputDevice::Pad);
    app.update();
    assert_eq!(names(&app, ui.legend), ["A", "B"]);
    app.world_mut().insert_resource(InputDevice::Keyboard);
    app.update();

    // A player rebinds Back, and the game keeps the hint true: the same keycap, new words.
    let back = app.world().get::<Children>(ui.legend).expect("prompts")[1];
    let cap = part(&app, back, Role::PromptKey);
    app.world_mut().entity_mut(back).insert(Prompt::new(
        Binding::Key(KeyCode::Backspace),
        Binding::Pad(GamepadButton::East),
    ));
    app.update();
    assert_eq!(names(&app, ui.legend), ["Enter", "Backspace"]);
    assert_eq!(part(&app, back, Role::PromptKey), cap, "no new entity");

    // A mouse button is an input too, and its keycap is a key's shape.
    app.world_mut().entity_mut(back).insert(Prompt::new(
        Binding::Mouse(MouseButton::Right),
        Binding::Pad(GamepadButton::East),
    ));
    app.update();
    assert_eq!(names(&app, ui.legend), ["Enter", "Right Click"]);
    assert_eq!(shapes(&app, ui.legend), [Role::PromptKey, Role::PromptKey]);
}

// ---- where it stands ----

#[test]
fn the_legend_stands_in_the_safe_corner_at_every_size() {
    for (width, height) in [(1280, 720), (1920, 1080), (2560, 1440)] {
        let mut app = app_sized(width, height);
        let ui = ui(&mut app);
        let bar = rect(&app, ui.legend);
        let (w, h) = (width as f32, height as f32);
        assert!(
            (bar.min.x - 0.05 * w).abs() <= 1.0,
            "{width}x{height}: the legend starts {} px in, not 5% of {w}",
            bar.min.x
        );
        assert!(
            (h - bar.max.y - 0.05 * h).abs() <= 1.0,
            "{width}x{height}: the legend stands {} px above the bottom, not 5% of {h}",
            h - bar.max.y
        );
        assert!(bar.width() > 0.0 && bar.max.x < w, "inside the window");
    }
}

#[test]
fn adding_the_legend_moves_nothing_on_the_screen() {
    let mut app = app();
    let panel = app
        .world_mut()
        .spawn((panel(), children![heading("Settings"), button("Play")]))
        .id();
    let screen = app.world_mut().spawn(screen()).add_child(panel).id();
    settle(&mut app);
    let before = rect(&app, panel);
    let play = part(&app, panel, Role::Button);
    let button_before = rect(&app, play);

    let legend = legend_of(app.world_mut());
    app.world_mut().entity_mut(screen).add_child(legend);
    settle(&mut app);
    assert_eq!(rect(&app, panel), before, "the panel did not move");
    assert_eq!(rect(&app, play), button_before);
}

#[test]
fn a_legend_in_a_dialog_stands_at_the_same_corner_over_the_scrim() {
    let mut app = app();
    let ui = ui(&mut app);
    let legend = legend_of(app.world_mut());
    app.world_mut()
        .spawn((
            dialog(),
            children![(panel(), children![heading("Leave?"), button("Stay")])],
        ))
        .add_child(legend);
    settle(&mut app);
    assert_eq!(rect(&app, legend), rect(&app, ui.legend));
    // The scrim's own padding (5% of the window) does not move it, nor does the dialog's panel.
    let bar = rect(&app, legend);
    assert!((bar.min.x - 64.0).abs() <= 1.0 && (720.0 - bar.max.y - 36.0).abs() <= 1.0);
}

#[test]
fn a_legend_of_many_long_prompts_squeezes_and_stays_in_the_window() {
    let mut app = app();
    let world = app.world_mut();
    let legend = world
        .spawn((
            prompts(),
            children![
                prompt("Open the inventory", Prompt::SELECT),
                prompt("Close this menu and go back", Prompt::BACK),
                prompt("Compare with equipped gear", Prompt::SELECT),
                prompt("Drop the selected item", Prompt::BACK),
                prompt("Sort by weight, then value", Prompt::SELECT),
            ],
        ))
        .id();
    world.spawn(screen()).add_child(legend);
    settle(&mut app);
    let bar = rect(&app, legend);
    assert!(
        bar.max.x <= 0.95 * 1280.0 + 1.0,
        "the legend ends at {} of 1280",
        bar.max.x
    );
    assert!((bar.min.x - 64.0).abs() <= 1.0);
    assert!(bar.max.y <= 720.0 && bar.min.y >= 0.0);
}

// ---- the keycaps ----

#[test]
fn a_single_character_is_a_square_or_a_circle_and_a_name_is_wider() {
    let mut app = app();
    let ui = ui(&mut app);
    let key = |app: &App, n: usize| {
        let prompt = app.world().get::<Children>(ui.legend).expect("prompts")[n];
        rect(app, part(app, prompt, Role::PromptKey))
    };
    let enter = key(&app, 0);
    assert!(
        enter.width() > enter.height() + 4.0,
        "Enter is a wide keycap: {enter:?}"
    );
    let pad = plug_in_pad(&mut app);
    press(&mut app, pad, GamepadButton::West);
    let prompt = app.world().get::<Children>(ui.legend).expect("prompts")[0];
    let a = rect(&app, part(&app, prompt, Role::PromptButton));
    assert!(
        (a.width() - a.height()).abs() <= 1.0,
        "A is a circle, not {a:?}"
    );
    // A pad's face button is as high as a key: the legend does not change height with the device.
    assert!((a.height() - enter.height()).abs() <= 1.0);
    let radius = app
        .world()
        .get::<Node>(part(&app, prompt, Role::PromptButton))
        .expect("a node")
        .border_radius;
    assert_eq!(radius, BorderRadius::MAX, "a pad button is fully round");
}

#[test]
fn the_hints_read_in_every_preset() {
    for (name, skin) in [
        ("dark", GooeySkin::dark()),
        ("parchment", GooeySkin::parchment()),
        ("high contrast", GooeySkin::high_contrast()),
    ] {
        let mut app = app();
        app.insert_resource(skin.clone());
        let ui = ui(&mut app);
        let surface = Srgba::from(skin.surface);
        let bar = composite(rgb(skin.surface), surface.alpha, rgb(skin.background));
        let raised = Srgba::from(skin.surface_raised);
        let cap_fill = composite(rgb(skin.surface_raised), raised.alpha, bar);

        for device in [InputDevice::Keyboard, InputDevice::Pad] {
            app.world_mut().insert_resource(device);
            settle(&mut app);
            let role = if device == InputDevice::Pad {
                Role::PromptButton
            } else {
                Role::PromptKey
            };
            let cap = part(&app, ui.legend, role);
            let on_cap = contrast_ratio(rgb(text_colour(&app, cap)), cap_fill);
            assert!(
                on_cap >= TEXT_CONTRAST,
                "{name} {device:?}: the name is {on_cap:.2}:1 on its keycap"
            );
            let frame = app.world().get::<BorderColor>(cap).expect("a frame").top;
            let framed = contrast_ratio(rgb(frame), bar);
            assert!(
                framed >= NON_TEXT_CONTRAST,
                "{name} {device:?}: the keycap's frame is {framed:.2}:1 on the bar"
            );
        }
        let words = part(&app, ui.legend, Role::Label);
        let on_bar = contrast_ratio(rgb(text_colour(&app, words)), bar);
        assert!(
            on_bar >= TEXT_CONTRAST,
            "{name}: the words are {on_bar:.2}:1 on the bar"
        );
        assert_eq!(
            fill(&app, ui.legend),
            skin.surface,
            "{name}: the bar is the panel's colour"
        );
        assert!(
            skin.problems().is_empty(),
            "{name}: the preset has no problem"
        );
    }
}

// ---- the pointer and focus ----

#[test]
fn the_pointer_goes_through_the_legend_and_no_prompt_is_a_stop() {
    let mut app = app();
    let below = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(-1),
        ))
        .id();
    let ui = ui(&mut app);
    let at = rect(&app, ui.legend).center();
    point_at(&mut app, at);
    let over = hovered(&app);
    assert!(
        over.contains(&below),
        "the world under the legend is reachable"
    );
    assert!(!over.contains(&ui.legend), "the bar claims the pointer");
    for entity in over {
        assert!(
            app.world().get::<Role>(entity).is_none(),
            "a {:?} under the pointer",
            app.world().get::<Role>(entity)
        );
    }

    // Tab lands on the button and never on a hint, however many times it is pressed.
    let mut stops = HashSet::new();
    for _ in 0..4 {
        tap(&mut app, KeyCode::Tab);
        if let Some(focus) = app
            .world()
            .resource::<bevy::input_focus::InputFocus>()
            .get()
        {
            stops.insert(focus);
        }
    }
    let play = part(&app, ui.panel, Role::Button);
    stops.remove(&window(&mut app));
    assert_eq!(stops, HashSet::from([play]), "Tab visits the button only");
}

// ---- cost ----

#[test]
fn idle_frames_write_nothing() {
    let mut app = app();
    let _ui = ui(&mut app);
    let pad = plug_in_pad(&mut app);
    press(&mut app, pad, GamepadButton::West);
    settle(&mut app);
    app.world_mut().insert_resource(Writes::default());
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<Writes>().0,
        0,
        "nothing about a legend was written"
    );
    // Neither does more of the same input: the device is the same, so nothing is restyled.
    press(&mut app, pad, GamepadButton::West);
    app.world_mut().insert_resource(Writes::default());
    press(&mut app, pad, GamepadButton::West);
    assert_eq!(
        app.world().resource::<Writes>().0,
        0,
        "a second press of the same device writes nothing"
    );
}
