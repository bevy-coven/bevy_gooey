//! The hold button: a button that confirms only when held, in the same headless window, on a clock
//! the test sets (0.1 s a frame), driven by real keyboard, pointer and gamepad input.

use std::time::Duration;

use bevy::a11y::{AccessibilityNode, AccessibilityRequested, ActionRequest};
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
use bevy::ui::{ComputedNode, InteractionDisabled, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// What the game heard: confirms, and any `Activate` (there must never be one).
#[derive(Resource, Default)]
struct Heard {
    confirmed: Vec<Entity>,
    activated: usize,
    feedback: usize,
}

/// How many frames the progress or a node of a hold button was written.
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
    .init_resource::<Heard>()
    .init_resource::<Writes>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )))
    .add_observer(|e: On<HoldConfirmed>, mut heard: ResMut<Heard>| heard.confirmed.push(e.entity))
    .add_observer(|_: On<Activate>, mut heard: ResMut<Heard>| heard.activated += 1)
    .add_systems(
        Update,
        (
            |holds: Query<(), Changed<HoldProgress>>,
             nodes: Query<(), (Changed<Node>, With<Role>)>,
             mut writes: ResMut<Writes>| {
                writes.0 += (holds.iter().count() + nodes.iter().count()) as u32;
            },
            |mut feedback: MessageReader<UiFeedback>, mut heard: ResMut<Heard>| {
                heard.feedback += feedback.read().count();
            },
        ),
    );
    app.world_mut().spawn(Camera2d);
    app.world().resource::<AccessibilityRequested>().set(true);
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn settle(app: &mut App) {
    frames(app, 6);
}

/// A screen with a panel holding a "Hold to delete" button of `seconds` and a plain button beside
/// it, settled.
fn rig(app: &mut App, seconds: f32) -> (Entity, Entity) {
    let root = app.world_mut().spawn(screen()).id();
    let panel = app.world_mut().spawn((panel(), ChildOf(root))).id();
    let hold = app
        .world_mut()
        .spawn((hold_button("Hold to delete", seconds), ChildOf(panel)))
        .id();
    let other = app
        .world_mut()
        .spawn((button("Cancel"), ChildOf(panel)))
        .id();
    settle(app);
    (hold, other)
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    settle(app);
}

fn confirmed(app: &App) -> usize {
    app.world().resource::<Heard>().confirmed.len()
}

fn progress(app: &App, hold: Entity) -> f32 {
    app.world().get::<HoldProgress>(hold).expect("state").0
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

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

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let window = window(app);
    let location = Location {
        target: bevy::camera::NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    };
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, location, action));
    app.update();
}

/// Moves the pointer onto `at` (twice: picking needs a frame to see it).
fn point_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::X });
    app.update();
}

fn press_pointer(app: &mut App, at: Vec2) {
    point_at(app, at);
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
}

fn release_pointer(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
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

/// The fill of a hold button's bar: its `width`.
fn fill_width(app: &App, hold: Entity) -> Val {
    let world = app.world();
    let track = world
        .get::<Children>(hold)
        .expect("children")
        .iter()
        .find(|c| world.get::<Role>(*c) == Some(&Role::HoldTrack))
        .expect("a track");
    let fill = world
        .get::<Children>(track)
        .expect("the track's children")
        .iter()
        .find(|c| world.get::<Role>(*c) == Some(&Role::HoldFill))
        .expect("a fill");
    world.get::<Node>(fill).expect("node").width
}

// --- The key ----------------------------------------------------------------------------------

#[test]
fn a_key_held_for_the_whole_time_confirms_once() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 1.0);
    focus(&mut app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 7);
    assert_eq!(confirmed(&app), 0, "0.8 s of a 1 s hold");
    assert!((0.6..0.95).contains(&progress(&app, hold)));
    frames(&mut app, 5);
    assert_eq!(confirmed(&app), 1);
    // Still down: a hold confirms once.
    frames(&mut app, 30);
    assert_eq!(
        confirmed(&app),
        1,
        "a hold still down does not confirm again"
    );
    assert_eq!(progress(&app, hold), 1.0, "the bar stays full while held");
    key(&mut app, KeyCode::Enter, ButtonState::Released);
    settle(&mut app);
    assert_eq!(progress(&app, hold), 0.0, "and empties after");
    assert_eq!(confirmed(&app), 1);
    // A second hold confirms a second time.
    key(&mut app, KeyCode::Space, ButtonState::Pressed);
    frames(&mut app, 14);
    key(&mut app, KeyCode::Space, ButtonState::Released);
    assert_eq!(confirmed(&app), 2, "Space holds as Enter does");
}

#[test]
fn letting_go_early_confirms_nothing_and_unwinds() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 1.0);
    focus(&mut app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 4);
    let held = progress(&app, hold);
    assert!(held > 0.3, "{held}");
    key(&mut app, KeyCode::Enter, ButtonState::Released);
    let after = progress(&app, hold);
    assert!(after < held, "it unwinds the frame it is let go");
    frames(&mut app, 10);
    assert_eq!(progress(&app, hold), 0.0);
    assert_eq!(confirmed(&app), 0);
}

#[test]
fn it_unwinds_faster_than_it_filled_and_resumes_from_where_it_stands() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 2.0);
    focus(&mut app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 7);
    let held = progress(&app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Released);
    let drained = held - progress(&app, hold);
    // Filling took 0.1 / 2.0 a frame; unwinding is four times that.
    assert!(drained > 0.15 && drained < 0.25, "drained {drained}");
    let low = progress(&app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    assert!(
        progress(&app, hold) > low,
        "a new press rises from where it stands"
    );
}

#[test]
fn enter_already_down_when_focus_arrives_does_not_hold() {
    let mut app = app();
    let (hold, other) = rig(&mut app, 1.0);
    focus(&mut app, other);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    // Focus moves onto the hold button with Enter still down, as when a dialog opens on a press.
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(hold, FocusCause::Navigated);
    frames(&mut app, 20);
    assert_eq!(confirmed(&app), 0);
    assert_eq!(progress(&app, hold), 0.0);
}

#[test]
fn focus_leaving_ends_a_key_hold() {
    let mut app = app();
    let (hold, other) = rig(&mut app, 1.0);
    focus(&mut app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 4);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(other, FocusCause::Navigated);
    frames(&mut app, 20);
    assert_eq!(confirmed(&app), 0);
    assert_eq!(progress(&app, hold), 0.0);
}

#[test]
fn another_widgets_enter_does_not_hold_it() {
    let mut app = app();
    let (hold, other) = rig(&mut app, 1.0);
    focus(&mut app, other);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 20);
    assert_eq!(progress(&app, hold), 0.0);
}

// --- The pad ----------------------------------------------------------------------------------

#[test]
fn the_pads_south_button_holds() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 1.0);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, hold);
    pad_button(&mut app, pad, GamepadButton::South, 1.0);
    frames(&mut app, 5);
    assert_eq!(confirmed(&app), 0);
    frames(&mut app, 8);
    assert_eq!(confirmed(&app), 1);
    pad_button(&mut app, pad, GamepadButton::South, 0.0);
    settle(&mut app);
    assert_eq!(progress(&app, hold), 0.0);
    assert_eq!(
        app.world().resource::<Heard>().activated,
        0,
        "South sends no Activate"
    );
}

// --- The pointer ------------------------------------------------------------------------------

#[test]
fn the_pointer_holds_while_it_is_over_the_button() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 1.0);
    let at = rect(&app, hold).center();
    press_pointer(&mut app, at);
    frames(&mut app, 6);
    assert_eq!(confirmed(&app), 0);
    frames(&mut app, 6);
    assert_eq!(confirmed(&app), 1);
    release_pointer(&mut app, at);
    settle(&mut app);
    assert_eq!(progress(&app, hold), 0.0);
    assert_eq!(
        app.world().resource::<Heard>().activated,
        0,
        "a click sends no Activate"
    );
    // A press focuses it, as any button.
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(hold));
}

#[test]
fn a_click_is_not_a_hold() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 1.0);
    let at = rect(&app, hold).center();
    press_pointer(&mut app, at);
    release_pointer(&mut app, at);
    settle(&mut app);
    assert_eq!(confirmed(&app), 0);
    assert_eq!(progress(&app, hold), 0.0);
    assert_eq!(app.world().resource::<Heard>().activated, 0);
}

#[test]
fn the_pointer_leaving_cancels_the_hold() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 1.0);
    let at = rect(&app, hold).center();
    press_pointer(&mut app, at);
    frames(&mut app, 4);
    // Off the button, still pressed.
    pointer(
        &mut app,
        Vec2::new(1000.0, 600.0),
        PointerAction::Move {
            delta: Vec2::new(500.0, 200.0),
        },
    );
    frames(&mut app, 20);
    assert_eq!(
        confirmed(&app),
        0,
        "a hold the pointer left does not run on"
    );
    assert_eq!(progress(&app, hold), 0.0);
    release_pointer(&mut app, Vec2::new(1000.0, 600.0));
    settle(&mut app);
    assert_eq!(confirmed(&app), 0);
}

#[test]
fn a_hold_by_key_and_by_pointer_survives_one_of_them_letting_go() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 2.0);
    let at = rect(&app, hold).center();
    press_pointer(&mut app, at);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 3);
    release_pointer(&mut app, at);
    let before = progress(&app, hold);
    frames(&mut app, 3);
    assert!(progress(&app, hold) > before, "the key still holds");
}

// --- The player's setting ---------------------------------------------------------------------

#[test]
fn a_shorter_scale_shortens_every_hold() {
    let mut app = app();
    app.world_mut().insert_resource(HoldSettings {
        duration_scale: 0.5,
    });
    let (hold, _) = rig(&mut app, 2.0);
    focus(&mut app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 6);
    assert_eq!(confirmed(&app), 0, "0.7 s of 1 s");
    frames(&mut app, 6);
    assert_eq!(confirmed(&app), 1, "a 2 s hold at half is 1 s");
}

#[test]
fn a_tap_setting_confirms_on_the_up_event_of_a_key() {
    let mut app = app();
    app.world_mut().insert_resource(HoldSettings::tap());
    let (hold, _) = rig(&mut app, 1.5);
    focus(&mut app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 3);
    assert_eq!(confirmed(&app), 0, "never on the down event");
    assert_eq!(progress(&app, hold), 1.0, "the bar is full while held");
    key(&mut app, KeyCode::Enter, ButtonState::Released);
    assert_eq!(confirmed(&app), 1);
    settle(&mut app);
    assert_eq!(confirmed(&app), 1);
}

#[test]
fn a_tap_setting_confirms_a_click_and_not_one_the_pointer_left() {
    let mut app = app();
    app.world_mut().insert_resource(HoldSettings::tap());
    let (hold, _) = rig(&mut app, 1.5);
    let at = rect(&app, hold).center();
    press_pointer(&mut app, at);
    release_pointer(&mut app, at);
    settle(&mut app);
    assert_eq!(confirmed(&app), 1, "a click confirms");
    // Pressed, then moved away before the up event: aborted (XAG 107 / WCAG 2.5.2).
    press_pointer(&mut app, at);
    pointer(
        &mut app,
        Vec2::new(1000.0, 600.0),
        PointerAction::Move {
            delta: Vec2::new(500.0, 200.0),
        },
    );
    release_pointer(&mut app, Vec2::new(1000.0, 600.0));
    settle(&mut app);
    assert_eq!(confirmed(&app), 1, "the abort confirmed nothing");
    assert_eq!(app.world().resource::<Heard>().activated, 0);
}

#[test]
fn a_tap_is_cancelled_by_focus_leaving() {
    let mut app = app();
    app.world_mut().insert_resource(HoldSettings::tap());
    let (hold, other) = rig(&mut app, 1.5);
    focus(&mut app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(other, FocusCause::Navigated);
    frames(&mut app, 2);
    key(&mut app, KeyCode::Enter, ButtonState::Released);
    settle(&mut app);
    assert_eq!(confirmed(&app), 0);
}

// --- Disabled and assistive technology --------------------------------------------------------

#[test]
fn a_disabled_hold_button_never_starts_and_one_disabled_mid_hold_ends() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 1.0);
    focus(&mut app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 4);
    app.world_mut().entity_mut(hold).insert(InteractionDisabled);
    frames(&mut app, 20);
    assert_eq!(confirmed(&app), 0);
    assert_eq!(progress(&app, hold), 0.0);
    key(&mut app, KeyCode::Enter, ButtonState::Released);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 20);
    assert_eq!(confirmed(&app), 0, "disabled: no hold starts");
    // The pointer is as dead.
    let at = rect(&app, hold).center();
    press_pointer(&mut app, at);
    frames(&mut app, 20);
    assert_eq!(confirmed(&app), 0);
}

#[test]
fn a_screen_readers_click_confirms_at_once_and_a_hold_button_is_a_named_button() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 1.0);
    let node = app.world().get::<AccessibilityNode>(hold).expect("a node");
    assert_eq!(node.role(), accesskit::Role::Button);
    assert_eq!(node.label(), Some("Hold to delete"));
    assert!(node.supports_action(accesskit::Action::Click));
    app.world_mut()
        .write_message(ActionRequest(accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: accesskit::NodeId(hold.to_bits()),
            data: None,
        }));
    settle(&mut app);
    assert_eq!(confirmed(&app), 1);
    assert_eq!(app.world().resource::<Heard>().activated, 0);
}

#[test]
fn a_confirm_writes_one_activate_feedback_and_a_press_none() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 1.0);
    focus(&mut app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 4);
    assert_eq!(
        app.world().resource::<Heard>().feedback,
        0,
        "none at the press"
    );
    frames(&mut app, 12);
    assert_eq!(app.world().resource::<Heard>().feedback, 1);
}

// --- The look ---------------------------------------------------------------------------------

#[test]
fn the_bar_shows_the_progress_and_idle_writes_nothing() {
    let mut app = app();
    let (hold, _) = rig(&mut app, 2.0);
    assert_eq!(fill_width(&app, hold), Val::Percent(0.0));
    frames(&mut app, 30);
    let before = app.world().resource::<Writes>().0;
    frames(&mut app, 30);
    assert_eq!(
        app.world().resource::<Writes>().0,
        before,
        "an idle hold button writes nothing"
    );
    focus(&mut app, hold);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed);
    frames(&mut app, 5);
    let Val::Percent(width) = fill_width(&app, hold) else {
        panic!("the fill is a percentage");
    };
    assert!(
        (width - progress(&app, hold) * 100.0).abs() < 0.01,
        "{width}"
    );
    assert!(width > 20.0);
}

#[test]
fn it_is_a_buttons_size_with_the_bar_inside_it() {
    let mut app = app();
    let (hold, other) = rig(&mut app, 1.0);
    let hold_box = rect(&app, hold);
    let other_box = rect(&app, other);
    assert!(hold_box.height() >= 44.0, "a target: {}", hold_box.height());
    let track = app
        .world()
        .get::<Children>(hold)
        .expect("children")
        .iter()
        .find(|c| app.world().get::<Role>(*c) == Some(&Role::HoldTrack))
        .expect("a track");
    let track_box = rect(&app, track);
    assert!(
        track_box.min.x >= hold_box.min.x - 0.5 && track_box.max.x <= hold_box.max.x + 0.5,
        "{track_box:?} in {hold_box:?}"
    );
    assert!(
        track_box.min.y >= hold_box.min.y && track_box.max.y <= hold_box.max.y,
        "{track_box:?} in {hold_box:?}"
    );
    assert!(
        (hold_box.width() - other_box.width()).abs() > 0.0
            || hold_box.height() >= other_box.height(),
        "it is no smaller than a button"
    );
}
