//! The settings controls: a toggle and a slider keep their state, draw it, take the pointer, the
//! keys and the pad, and never leave their lines off the pixel grid. Laid out for real in a headless
//! window, driven by real keyboard, pointer and gamepad input.

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
use bevy::ui::{InteractionDisabled, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<bool>` and `ValueChange<f32>` a game would see: the value and `is_final`.
#[derive(Resource, Default)]
struct Changes {
    toggles: Vec<bool>,
    sliders: Vec<(f32, bool)>,
}

/// Every `UiFeedback` written.
#[derive(Resource, Default)]
struct Feedback(Vec<UiFeedback>);

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
    .add_observer(|change: On<ValueChange<bool>>, mut seen: ResMut<Changes>| {
        seen.toggles.push(change.value)
    })
    .add_observer(|change: On<ValueChange<f32>>, mut seen: ResMut<Changes>| {
        seen.sliders.push((change.value, change.is_final))
    })
    .add_systems(
        Update,
        |mut messages: MessageReader<UiFeedback>, mut seen: ResMut<Feedback>| {
            seen.0.extend(messages.read().copied());
        },
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn app() -> App {
    app_sized(1280, 720)
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// An options column as a game builds it, centred on a `screen()`.
struct Options {
    subtitles: Entity,
    music: Entity,
    gamma: Entity,
    back: Entity,
}

fn options(app: &mut App) -> Options {
    let world = app.world_mut();
    let subtitles = world.spawn(toggle("Subtitles", true)).id();
    let music = world.spawn(slider("Music", 0.5, 0.0..=1.0)).id();
    let gamma = world.spawn(slider("Gamma", 2.0, 1.0..=3.0)).id();
    let back = world.spawn(button("Back")).id();
    let card = world
        .spawn(panel())
        .add_children(&[subtitles, music, gamma, back])
        .id();
    world.spawn(screen()).add_child(card);
    settle(app);
    Options {
        subtitles,
        music,
        gamma,
        back,
    }
}

/// A slider with a button to its right, in a row at the window's top-left corner, clear of the
/// options column: the layout in which Right would leave the slider if it were navigation.
fn beside(app: &mut App) -> (Entity, Entity) {
    let world = app.world_mut();
    let side = world.spawn(slider("Side", 0.5, 0.0..=1.0)).id();
    let next = world.spawn(button("Next")).id();
    world
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            column_gap: Val::Px(10.0),
            ..default()
        })
        .add_children(&[side, next]);
    settle(app);
    let (s, n) = (rect(app, side), rect(app, next));
    assert!(n.min.x > s.max.x, "the button is to the slider's right");
    (side, next)
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

/// Presses at `from`, moves to `to` in two steps and releases there.
fn drag(app: &mut App, from: Vec2, to: Vec2) {
    point_at(app, from);
    pointer(app, from, PointerAction::Press(PointerButton::Primary));
    let half = from.lerp(to, 0.5);
    pointer(app, half, PointerAction::Move { delta: half - from });
    pointer(app, to, PointerAction::Move { delta: to - half });
    pointer(app, to, PointerAction::Release(PointerButton::Primary));
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

fn value(app: &App, slider: Entity) -> f32 {
    app.world().get::<SliderValue>(slider).expect("a value").0
}

fn checked(app: &App, toggle: Entity) -> bool {
    app.world().get::<Checked>(toggle).is_some()
}

fn text(app: &App, entity: Entity) -> String {
    app.world().get::<Text>(entity).expect("text").0.clone()
}

fn skin() -> GooeySkin {
    GooeySkin::default()
}

fn border_of(app: &App, entity: Entity) -> BorderColor {
    *app.world().get::<BorderColor>(entity).expect("a border")
}

fn fill_of(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

fn text_color(app: &App, entity: Entity) -> Color {
    app.world().get::<TextColor>(entity).expect("a colour").0
}

/// Where a slider's thumb centre belongs: past the border and half a thumb, a `fraction` of the
/// travel that is left of the width.
fn thumb_centre_x(app: &App, slider: Entity, fraction: f32) -> f32 {
    let row = rect(app, slider);
    let line = app
        .world()
        .get::<ComputedNode>(slider)
        .expect("laid out")
        .border;
    let thumb = rect(app, part(app, slider, Role::SliderThumb)).width();
    row.min.x
        + line.min_inset.x
        + thumb / 2.0
        + fraction * (row.width() - line.min_inset.x - line.max_inset.x - thumb)
}

// ---- the toggle ----

#[test]
fn a_toggle_keeps_its_own_state_and_says_it_to_the_game() {
    let mut app = app();
    let ui = options(&mut app);
    assert!(checked(&app, ui.subtitles), "spawned on");
    let off = app.world_mut().spawn(toggle("Fullscreen", false)).id();
    settle(&mut app);
    assert!(!checked(&app, off), "spawned off");

    // A click anywhere on the row: the label, the track, the knob.
    let row = rect(&app, ui.subtitles);
    click_at(&mut app, Vec2::new(row.min.x + 30.0, row.center().y));
    assert!(!checked(&app, ui.subtitles));
    let knob = rect(&app, part(&app, ui.subtitles, Role::ToggleKnob));
    click_at(&mut app, knob.center());
    assert!(checked(&app, ui.subtitles));
    assert_eq!(app.world().resource::<Changes>().toggles, [false, true]);
    assert_eq!(focused(&app), Some(ui.subtitles), "a click focuses it");
}

#[test]
fn enter_space_and_the_pads_south_button_flip_a_focused_toggle() {
    let mut app = app();
    let ui = options(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.subtitles);

    tap(&mut app, KeyCode::Enter);
    assert!(!checked(&app, ui.subtitles));
    tap(&mut app, KeyCode::Space);
    assert!(checked(&app, ui.subtitles));
    press(&mut app, pad, GamepadButton::South);
    assert!(!checked(&app, ui.subtitles), "South flips it too");
    // Once each: a second handler would flip it twice.
    assert_eq!(
        app.world().resource::<Changes>().toggles,
        [false, true, false]
    );
}

#[test]
fn a_disabled_toggle_and_slider_ignore_every_input_and_read_dim() {
    let mut app = app();
    let ui = options(&mut app);
    let pad = plug_in_pad(&mut app);
    app.world_mut()
        .entity_mut(ui.subtitles)
        .insert(InteractionDisabled);
    app.world_mut()
        .entity_mut(ui.music)
        .insert(InteractionDisabled);
    settle(&mut app);

    focus(&mut app, ui.subtitles);
    tap(&mut app, KeyCode::Enter);
    press(&mut app, pad, GamepadButton::South);
    let row = rect(&app, ui.subtitles);
    click_at(&mut app, row.center());
    assert!(checked(&app, ui.subtitles), "a disabled toggle stays on");

    let rail = rect(&app, ui.music);
    click_at(&mut app, Vec2::new(rail.max.x - 20.0, rail.center().y));
    focus(&mut app, ui.music);
    press(&mut app, pad, GamepadButton::DPadRight);
    assert_eq!(
        value(&app, ui.music),
        0.5,
        "a disabled slider keeps its value"
    );
    assert!(app.world().resource::<Changes>().toggles.is_empty());
    assert!(app.world().resource::<Changes>().sliders.is_empty());

    let label = part(&app, ui.subtitles, Role::ControlText);
    assert_eq!(text_color(&app, label), skin().text_muted);
    let readout = part(&app, ui.music, Role::SliderReadout);
    assert_eq!(text_color(&app, readout), skin().text_muted);
    app.world_mut()
        .entity_mut(ui.subtitles)
        .remove::<InteractionDisabled>();
    settle(&mut app);
    assert_eq!(text_color(&app, label), skin().text, "enabled again");
}

#[test]
fn a_toggles_knob_sits_at_the_end_when_on_and_the_start_when_off() {
    let mut app = app();
    let ui = options(&mut app);
    let skin = skin();
    let track = part(&app, ui.subtitles, Role::ToggleTrack);
    let knob = part(&app, ui.subtitles, Role::ToggleKnob);
    let line = 3.0;

    let (t, k) = (rect(&app, track), rect(&app, knob));
    assert_eq!(k.max.x, t.max.x - 2.0 * line, "on: the knob at the end");
    assert_eq!(fill_of(&app, track), skin.accent);
    assert_eq!(fill_of(&app, knob), skin.accent_text);
    assert_eq!(k.height(), k.width(), "a round knob");
    assert_eq!(k.center().y, t.center().y, "centred in the track");

    tap(&mut app, KeyCode::ArrowDown); // nothing is focused: lands on the first stop
    assert_eq!(focused(&app), Some(ui.subtitles));
    tap(&mut app, KeyCode::Enter);
    let (t, k) = (rect(&app, track), rect(&app, knob));
    assert_eq!(k.min.x, t.min.x + 2.0 * line, "off: the knob at the start");
    assert_eq!(fill_of(&app, track), skin.surface_raised);
    assert_eq!(fill_of(&app, knob), skin.text_muted);
}

// ---- the slider ----

#[test]
fn a_slider_puts_its_thumb_at_its_values_fraction_and_reads_it_out() {
    let mut app = app();
    let ui = options(&mut app);
    assert_eq!(text(&app, part(&app, ui.music, Role::SliderReadout)), "50%");
    assert_eq!(text(&app, part(&app, ui.gamma, Role::SliderReadout)), "2");

    for fraction in [0.0, 0.25, 0.5, 0.7, 1.0] {
        app.world_mut()
            .entity_mut(ui.music)
            .insert(SliderValue(fraction));
        settle(&mut app);
        let thumb = rect(&app, part(&app, ui.music, Role::SliderThumb))
            .center()
            .x;
        let wanted = thumb_centre_x(&app, ui.music, fraction);
        assert!(
            (thumb - wanted).abs() < 1.0,
            "at {fraction} the thumb is at {thumb}, wanted {wanted}"
        );
        let fill = rect(&app, part(&app, ui.music, Role::SliderFill));
        let rail = rect(&app, part(&app, ui.music, Role::SliderRail));
        assert!(
            (fill.width() / (rail.width() - 12.0) - fraction).abs() < 0.01,
            "the fill is {fraction} of the rail"
        );
        let shown = format!("{}%", (fraction * 100.0).round());
        assert_eq!(text(&app, part(&app, ui.music, Role::SliderReadout)), shown);
    }

    // A value past the range is drawn at its end; the game's own value is left alone.
    app.world_mut()
        .entity_mut(ui.music)
        .insert(SliderValue(7.0));
    settle(&mut app);
    let thumb = rect(&app, part(&app, ui.music, Role::SliderThumb))
        .center()
        .x;
    assert!((thumb - thumb_centre_x(&app, ui.music, 1.0)).abs() < 1.0);
    assert_eq!(
        text(&app, part(&app, ui.music, Role::SliderReadout)),
        "100%"
    );

    // A range that is not 0 to 1 reads as the number.
    app.world_mut()
        .entity_mut(ui.gamma)
        .insert(SliderValue(1.25));
    settle(&mut app);
    assert_eq!(
        text(&app, part(&app, ui.gamma, Role::SliderReadout)),
        "1.25"
    );
}

#[test]
fn a_click_anywhere_on_a_slider_puts_the_thumb_under_the_pointer() {
    let mut app = app();
    let ui = options(&mut app);
    let row = rect(&app, ui.music);
    let thumb_width = rect(&app, part(&app, ui.music, Role::SliderThumb)).width();
    let border = 3.0;

    // On the rail, on the label's line (the whole row is the target), near each end.
    for (x_fraction, y) in [
        (0.8, row.center().y + 12.0),
        (0.3, row.min.y + 28.0),
        (0.05, row.center().y),
    ] {
        let x = row.min.x + row.width() * x_fraction;
        click_at(&mut app, Vec2::new(x, y));
        // `bevy_ui_widgets` reads the click across the whole node: that is the value.
        let wanted = ((x - row.min.x) - thumb_width / 2.0) / (row.width() - thumb_width);
        assert!(
            (value(&app, ui.music) - wanted.clamp(0.0, 1.0)).abs() < 1.0e-3,
            "a click at {x_fraction} gave {}, wanted {wanted}",
            value(&app, ui.music)
        );
        // The border takes a few px off the thumb's travel, so the thumb lands within it.
        let thumb = rect(&app, part(&app, ui.music, Role::SliderThumb))
            .center()
            .x;
        assert!(
            (thumb - x).abs() <= border + 0.5,
            "the thumb is at {thumb}, the click at {x}"
        );
    }
    assert_eq!(focused(&app), Some(ui.music), "a click focuses it");
    let changes = &app.world().resource::<Changes>().sliders;
    assert_eq!(changes.len(), 3, "one ValueChange a click: {changes:?}");
}

#[test]
fn dragging_a_slider_follows_the_pointer_and_ends_with_a_final_change() {
    let mut app = app();
    let ui = options(&mut app);
    let row = rect(&app, ui.music);
    let thumb = rect(&app, part(&app, ui.music, Role::SliderThumb));
    let from = thumb.center();
    let started = value(&app, ui.music);

    drag(&mut app, from, from + Vec2::new(40.0, 0.0));
    let travel = row.width() - thumb.width();
    let moved = value(&app, ui.music) - started;
    // The press snaps to the pointer (a few px off the thumb's centre), then 40 px more.
    assert!(
        (moved - 40.0 / travel).abs() < 0.03,
        "40 px moved the value by {moved}, wanted {}",
        40.0 / travel
    );
    let changes = &app.world().resource::<Changes>().sliders;
    assert!(
        changes.len() >= 3,
        "a press and a drag in steps: {changes:?}"
    );
    assert!(
        changes.last().is_some_and(|(_, is_final)| *is_final),
        "the drag ends with is_final: {changes:?}"
    );
    assert!(
        changes.iter().any(|(_, is_final)| !is_final),
        "and goes on with is_final false while it is held: {changes:?}"
    );
}

#[test]
fn a_dragged_thumb_reads_pressed_until_it_is_let_go() {
    let mut app = app();
    let ui = options(&mut app);
    let skin = skin();
    let thumb = part(&app, ui.music, Role::SliderThumb);
    let from = rect(&app, thumb).center();
    assert_eq!(fill_of(&app, thumb), skin.text, "at rest");

    point_at(&mut app, from);
    pointer(&mut app, from, PointerAction::Press(PointerButton::Primary));
    let to = from + Vec2::new(30.0, 0.0);
    pointer(&mut app, to, PointerAction::Move { delta: to - from });
    settle(&mut app);
    assert_eq!(fill_of(&app, thumb), skin.focus_ring, "mid-drag");
    assert_eq!(border_of(&app, ui.music), BorderColor::all(skin.outline));

    pointer(&mut app, to, PointerAction::Release(PointerButton::Primary));
    settle(&mut app);
    assert_eq!(fill_of(&app, thumb), skin.text, "let go");
}

#[test]
fn left_and_right_adjust_a_focused_slider_and_up_and_down_move_on() {
    let mut app = app();
    let ui = options(&mut app);
    focus(&mut app, ui.music);

    tap(&mut app, KeyCode::ArrowRight);
    assert!(
        (value(&app, ui.music) - 0.55).abs() < 1.0e-4,
        "a twentieth of 0 to 1"
    );
    tap(&mut app, KeyCode::ArrowRight);
    tap(&mut app, KeyCode::ArrowLeft);
    assert!((value(&app, ui.music) - 0.55).abs() < 1.0e-4);
    assert_eq!(
        focused(&app),
        Some(ui.music),
        "Left and Right never leave it"
    );
    assert!(app.world().resource::<InputFocusVisible>().0);

    tap(&mut app, KeyCode::End);
    assert_eq!(value(&app, ui.music), 1.0);
    tap(&mut app, KeyCode::Home);
    assert_eq!(value(&app, ui.music), 0.0);
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(
        value(&app, ui.music),
        0.0,
        "stopped at the end of the range"
    );
    assert_eq!(focused(&app), Some(ui.music));

    // Up and Down are focus's, and change no value.
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(ui.gamma));
    assert_eq!(value(&app, ui.music), 0.0);
    // Gamma runs 1 to 3: a step is a tenth.
    tap(&mut app, KeyCode::ArrowRight);
    assert!((value(&app, ui.gamma) - 2.1).abs() < 1.0e-4);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(ui.back));
    tap(&mut app, KeyCode::ArrowUp);
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(focused(&app), Some(ui.music));
    // From a button, Left is still navigation: it lands on the slider beside it, and from the
    // slider Right adjusts, never leaving for the button.
    let (side, next) = beside(&mut app);
    focus(&mut app, next);
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(focused(&app), Some(side), "Left from a button navigates");
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(
        focused(&app),
        Some(side),
        "Right on a slider adjusts, not moves"
    );
    assert!((value(&app, side) - 0.55).abs() < 1.0e-4);
    // A disabled slider takes nothing, so Right is navigation again.
    app.world_mut().entity_mut(side).insert(InteractionDisabled);
    settle(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(
        focused(&app),
        Some(next),
        "a disabled slider keeps no arrows"
    );
    assert!((value(&app, side) - 0.55).abs() < 1.0e-4);
}

#[test]
fn the_d_pad_steps_a_focused_slider_without_leaving_it() {
    let mut app = app();
    let ui = options(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.music);
    app.insert_resource(InputFocusVisible(false));
    app.update();

    press(&mut app, pad, GamepadButton::DPadRight);
    assert!(
        (value(&app, ui.music) - 0.55).abs() < 1.0e-4,
        "one press, one step"
    );
    assert!(
        app.world().resource::<InputFocusVisible>().0,
        "the pad shows focus"
    );
    press(&mut app, pad, GamepadButton::DPadLeft);
    press(&mut app, pad, GamepadButton::DPadLeft);
    assert!((value(&app, ui.music) - 0.45).abs() < 1.0e-4);
    assert_eq!(focused(&app), Some(ui.music), "no select step, no leaving");

    press(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(focused(&app), Some(ui.gamma), "Up and Down move focus on");
    assert!((value(&app, ui.music) - 0.45).abs() < 1.0e-4);

    // With a button to its right, the D-pad's Right still only steps the slider.
    let (side, _) = beside(&mut app);
    focus(&mut app, side);
    press(&mut app, pad, GamepadButton::DPadRight);
    assert_eq!(focused(&app), Some(side));
    assert!((value(&app, side) - 0.55).abs() < 1.0e-4);
}

#[test]
fn a_held_d_pad_direction_repeats_after_a_delay() {
    let mut app = app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    let ui = options(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.music);
    let start = value(&app, ui.music);
    let steps = |app: &App| ((value(app, ui.music) - start) / 0.05).round() as i32;

    pad_button(&mut app, pad, GamepadButton::DPadRight, 1.0);
    assert_eq!(steps(&app), 1, "the press itself");
    // 0.3 s held: still short of the delay.
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        steps(&app),
        1,
        "nothing repeats before {PAD_REPEAT_DELAY} s"
    );
    // Held on to 0.9 s: the repeats at 0.4 s and then 12 a second, as the model counts them.
    let (mut held, mut wanted) = (0.3_f32, 1);
    for _ in 0..6 {
        app.update();
        let before = held;
        held += 0.1;
        wanted += repeat_steps(before, held, PAD_REPEAT_DELAY, PAD_REPEAT_RATE) as i32;
    }
    assert!(wanted >= 6, "the model repeats ({wanted})");
    assert_eq!(steps(&app), wanted.min(20));

    // Let go: it stops. Press again: a fresh press steps at once.
    pad_button(&mut app, pad, GamepadButton::DPadRight, 0.0);
    let stopped = steps(&app);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(steps(&app), stopped);
    pad_button(&mut app, pad, GamepadButton::DPadRight, 1.0);
    assert_eq!(steps(&app), (stopped + 1).min(10));
}

// ---- how they look ----

#[test]
fn a_row_shows_its_focus_ring_and_the_pointer_over_it() {
    let mut app = app();
    let ui = options(&mut app);
    let skin = skin();
    for row in [ui.subtitles, ui.music] {
        assert_eq!(
            border_of(&app, row),
            BorderColor::all(Color::NONE),
            "at rest"
        );
        focus(&mut app, row);
        assert_eq!(
            border_of(&app, row),
            BorderColor::all(skin.focus_ring),
            "focused"
        );
        // Focus moves on: the ring goes with it.
        focus(&mut app, ui.back);
        assert_eq!(border_of(&app, row), BorderColor::all(Color::NONE));
    }
    app.insert_resource(InputFocusVisible(false));
    app.update();
    let at = rect(&app, ui.music).center();
    point_at(&mut app, at);
    assert_eq!(
        border_of(&app, ui.music),
        BorderColor::all(skin.outline),
        "under the pointer"
    );
    point_at(&mut app, Vec2::new(5.0, 5.0));
    assert_eq!(border_of(&app, ui.music), BorderColor::all(Color::NONE));
}

#[test]
fn a_row_is_a_target_a_thumb_can_hit_and_lines_up_with_its_neighbours() {
    let mut app = app();
    let ui = options(&mut app);
    let min = skin().min_target;
    let (toggle, music, gamma, back) = (
        rect(&app, ui.subtitles),
        rect(&app, ui.music),
        rect(&app, ui.gamma),
        rect(&app, ui.back),
    );
    for (name, row) in [("toggle", toggle), ("slider", music)] {
        assert!(
            row.height() >= min,
            "the {name} row is {} px high",
            row.height()
        );
    }
    // One column: every row, button included, spans the same edges, so the rings line up.
    for row in [music, gamma, back] {
        assert_eq!((row.min.x, row.max.x), (toggle.min.x, toggle.max.x));
    }
    // The label, and the readout and switch at the other end, line up between the two kinds.
    let label = |root: Entity| rect(&app, part(&app, root, Role::ControlText)).min.x;
    assert_eq!(label(ui.subtitles), label(ui.music));
    let track = rect(&app, part(&app, ui.subtitles, Role::ToggleTrack))
        .max
        .x;
    let readout = rect(&app, part(&app, ui.music, Role::SliderReadout)).max.x;
    assert_eq!(track, readout);
}

#[test]
fn every_line_of_the_new_controls_is_one_whole_weight_at_every_window_scale() {
    for (width, height, device) in [
        (1280, 720, 3.0),
        (1600, 900, 3.0),
        (1920, 1080, 4.0),
        (2560, 1440, 6.0),
    ] {
        let at = format!("{width}x{height}");
        let mut app = app_sized(width, height);
        let ui = options(&mut app);
        let whole = (Vec2::splat(device), Vec2::splat(device));
        for (root, roles) in [
            (ui.subtitles, vec![Role::Toggle, Role::ToggleTrack]),
            (
                ui.music,
                vec![Role::Slider, Role::SliderRail, Role::SliderThumb],
            ),
        ] {
            for role in roles {
                let entity = if role == Role::Toggle || role == Role::Slider {
                    root
                } else {
                    part(&app, root, role)
                };
                let border = app
                    .world()
                    .get::<ComputedNode>(entity)
                    .expect("laid out")
                    .border;
                assert_eq!(
                    (border.min_inset, border.max_inset),
                    whole,
                    "{at}: a {role:?}'s border"
                );
            }
        }
        // The switch's gap round the knob is one line too, so the knob is two lines in from
        // the track's edge on the end it sits at (on: the right).
        let track = rect(&app, part(&app, ui.subtitles, Role::ToggleTrack));
        let knob = rect(&app, part(&app, ui.subtitles, Role::ToggleKnob));
        assert_eq!(
            track.max.x - knob.max.x,
            2.0 * device,
            "{at}: the knob's gap"
        );
        assert_eq!(
            knob.min.y - track.min.y,
            2.0 * device,
            "{at}: the knob's gap above"
        );
        assert_eq!(
            track.max.y - knob.max.y,
            2.0 * device,
            "{at}: the knob's gap below"
        );
    }
}

#[test]
fn the_controls_grow_with_the_players_text_size() {
    let mut app = app();
    let ui = options(&mut app);
    let track = |app: &App| rect(app, part(app, ui.subtitles, Role::ToggleTrack)).size();
    let thumb = |app: &App| rect(app, part(app, ui.music, Role::SliderThumb)).size();
    let (track_1, thumb_1) = (track(&app), thumb(&app));
    app.insert_resource(GooeySkin {
        text_scale: 1.5,
        ..skin()
    });
    settle(&mut app);
    let (track_2, thumb_2) = (track(&app), thumb(&app));
    assert!(
        (track_2.y / track_1.y - 1.5).abs() < 0.1,
        "{track_1} -> {track_2}"
    );
    assert!(
        (thumb_2.y / thumb_1.y - 1.5).abs() < 0.1,
        "{thumb_1} -> {thumb_2}"
    );
    // And the thumb still lands on the value at the larger size.
    let at = rect(&app, part(&app, ui.music, Role::SliderThumb))
        .center()
        .x;
    assert!((at - thumb_centre_x(&app, ui.music, 0.5)).abs() < 1.0);
}

// ---- feedback and the skin ----

#[test]
fn focus_and_a_flip_write_feedback_like_a_button_does() {
    let mut app = app();
    let ui = options(&mut app);
    app.world_mut().resource_mut::<Feedback>().0.clear();
    focus(&mut app, ui.subtitles);
    tap(&mut app, KeyCode::Enter);
    focus(&mut app, ui.music);
    // The collector may run before the system that writes the message: one more frame.
    app.update();
    let seen = app.world().resource::<Feedback>().0.clone();
    let of = |entity, kind| UiFeedback { entity, kind };
    assert_eq!(
        seen,
        [
            of(ui.subtitles, UiFeedbackKind::Focus),
            of(ui.subtitles, UiFeedbackKind::Activate),
            of(ui.music, UiFeedbackKind::Focus),
        ]
    );
}

#[test]
fn a_skin_whose_off_knob_vanishes_in_its_track_is_reported() {
    for skin in [
        GooeySkin::dark(),
        GooeySkin::parchment(),
        GooeySkin::high_contrast(),
    ] {
        assert_eq!(skin.problems(), vec![]);
    }
    let skin = GooeySkin {
        text_muted: GooeySkin::dark().surface_raised,
        ..GooeySkin::dark()
    };
    assert!(
        skin.problems().iter().any(|p| matches!(
            p,
            SkinProblem::LowContrast {
                pair: "text_muted on surface_raised",
                ..
            }
        )),
        "{:?}",
        skin.problems()
    );
}
