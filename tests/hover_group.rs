//! Hover groups (G-53): a `GroupReveal` under a `HoverGroup` follows the pointer over the group on the
//! skin's snappy spring in and its default spring out, stays revealed while a press that began inside
//! is held, shows at once under visible focus, and fades (reduced motion) or lands (no motion). Laid
//! out for real in a headless window, driven by real pointer input, on a 60 Hz clock the test sets.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

/// How close to the end counts as arrived: the critically damped spring is 98.6% of the way at its
/// perceptual duration, so 2% leaves a frame of room and is tighter than the other spring's 3.3% there.
const LEFT: f32 = 0.02;

/// The row is this tall and wide, logical px; the empty block under it the same.
const ROW_W: f32 = 320.0;
const ROW_H: f32 = 48.0;

fn app(settings: MotionSettings) -> App {
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
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )))
    .insert_resource(settings);
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

struct Ui {
    /// The row, a hover group.
    row: Entity,
    /// The button in the row that reveals with it.
    close: Entity,
    /// A block that is not in the group, under the row.
    elsewhere: Entity,
}

fn ui(app: &mut App, rest: f32) -> Ui {
    let world = app.world_mut();
    let close = world
        .spawn((button("x"), GroupReveal { rest, hovered: 1.0 }))
        .id();
    let row = world
        .spawn((
            Node {
                width: Val::Px(ROW_W),
                height: Val::Px(ROW_H),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            HoverGroup,
        ))
        .add_child(close)
        .id();
    let elsewhere = world
        .spawn(Node {
            width: Val::Px(ROW_W),
            height: Val::Px(ROW_H),
            ..default()
        })
        .id();
    world
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .add_children(&[row, elsewhere]);
    frames(app, 10);
    Ui {
        row,
        close,
        elsewhere,
    }
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

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window");
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
    app.update();
}

/// The pointer moves to `at` (two frames: the move, then the hover it causes).
fn move_to(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
}

/// Where in the row the pointer is hovering over the group but not over the button: its left end.
fn on_the_row(app: &App, ui: &Ui) -> Vec2 {
    let row = rect(app, ui.row);
    Vec2::new(row.min.x + 10.0, row.center().y)
}

fn opacity(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<Opacity>(entity)
        .map_or(1.0, |opacity| opacity.0)
}

/// The opacity now, then after each of the next `n` frames.
fn follow(app: &mut App, entity: Entity, n: usize) -> Vec<f32> {
    let mut seen = vec![opacity(app, entity)];
    for _ in 0..n {
        app.update();
        seen.push(opacity(app, entity));
    }
    seen
}

fn settle_frames(app: &App, token: fn(&MotionTokens) -> model::Spring) -> usize {
    let spring = token(&app.world().resource::<GooeySkin>().motion);
    (spring.duration / FRAME).round() as usize + 1
}

#[test]
fn a_group_that_is_not_hovered_is_at_rest_from_its_first_frame() {
    let mut app = app(MotionSettings::default());
    let ui = ui(&mut app, 0.25);
    assert_eq!(opacity(&app, ui.close), 0.25, "at rest, not animating in");
    assert_eq!(
        app.world()
            .get::<RevealTrack>(ui.close)
            .map(RevealTrack::progress),
        Some(0.0)
    );
}

#[test]
fn hovering_the_row_reveals_on_the_snappy_spring_and_leaving_hides_on_the_default_one() {
    let mut app = app(MotionSettings::default());
    let ui = ui(&mut app, 0.0);
    assert_eq!(opacity(&app, ui.close), 0.0);

    let at = on_the_row(&app, &ui);
    pointer(&mut app, at, PointerAction::Move { delta: Vec2::ZERO });
    let enter = follow(&mut app, ui.close, 40);
    assert!(
        enter.windows(2).all(|pair| pair[1] >= pair[0] - 1e-4),
        "it only ever rises: {enter:?}"
    );
    let snappy = settle_frames(&app, |m| m.snappy);
    assert!(
        1.0 - enter[snappy] <= LEFT,
        "within motion.snappy it is nearly there: {} after {snappy} frames",
        enter[snappy]
    );
    assert_eq!(*enter.last().expect("frames"), 1.0, "and it settles at 1");

    let away = rect(&app, ui.elsewhere).center();
    move_to(&mut app, away);
    let leave = follow(&mut app, ui.close, 40);
    assert!(
        leave.windows(2).all(|pair| pair[1] <= pair[0] + 1e-4),
        "it only ever falls: {leave:?}"
    );
    let default = settle_frames(&app, |m| m.default);
    assert!(
        leave[default] <= LEFT,
        "within motion.default it is nearly gone: {} after {default} frames",
        leave[default]
    );
    // The control: the leave is not on the snappy spring, which would be nearly gone a frame sooner.
    let snappy_leave = leave[snappy.saturating_sub(2)];
    assert!(
        snappy_leave > LEFT,
        "at the snappy duration the default spring still has {snappy_leave} left"
    );
    assert_eq!(
        *leave.last().expect("frames"),
        0.0,
        "and it settles at rest"
    );
}

#[test]
fn a_press_that_began_in_the_row_keeps_it_revealed_when_the_pointer_leaves_until_it_is_let_go() {
    let mut app = app(MotionSettings::default());
    let ui = ui(&mut app, 0.0);
    let on_button = rect(&app, ui.close).center();
    move_to(&mut app, on_button);
    frames(&mut app, 30);
    assert_eq!(opacity(&app, ui.close), 1.0);

    pointer(
        &mut app,
        on_button,
        PointerAction::Press(PointerButton::Primary),
    );
    // Drag the pointer a long way off the row, in steps, holding the button.
    let away = rect(&app, ui.elsewhere).center() + Vec2::new(0.0, 200.0);
    for step in 1..=10 {
        let at = on_button.lerp(away, step as f32 / 10.0);
        pointer(&mut app, at, PointerAction::Move { delta: Vec2::ZERO });
    }
    let held = follow(&mut app, ui.close, 60);
    assert!(
        held.iter().all(|&o| o == 1.0),
        "revealed while the press is held, though the pointer is off the row: {held:?}"
    );

    pointer(
        &mut app,
        away,
        PointerAction::Release(PointerButton::Primary),
    );
    let after = follow(&mut app, ui.close, 40);
    assert_eq!(
        *after.last().expect("frames"),
        0.0,
        "let go off the row: it hides"
    );
    assert!(after.windows(2).all(|pair| pair[1] <= pair[0] + 1e-4));
}

#[test]
fn visible_focus_on_the_group_reveals_in_that_frame_and_unseen_focus_does_not() {
    let mut app = app(MotionSettings::default());
    let ui = ui(&mut app, 0.0);

    // Focus with no ring: a pointer user's focus does not reveal anything.
    app.world_mut()
        .insert_resource(InputFocus::from_entity(ui.close));
    app.world_mut().insert_resource(InputFocusVisible(false));
    frames(&mut app, 20);
    assert_eq!(
        opacity(&app, ui.close),
        0.0,
        "focus that is not shown reveals nothing"
    );

    // Tab onto the button: the ring shows, and the control is shown in that frame.
    app.world_mut().insert_resource(InputFocusVisible(true));
    app.update();
    assert_eq!(
        opacity(&app, ui.close),
        1.0,
        "visible focus reveals in the frame it arrives"
    );

    // Focus leaves: it hides on the default spring, never with a jump.
    app.world_mut().insert_resource(InputFocus::default());
    let leave = follow(&mut app, ui.close, 40);
    assert!(leave[1] > 0.5, "it does not snap away: {leave:?}");
    assert_eq!(*leave.last().expect("frames"), 0.0);
}

#[test]
fn reduced_motion_fades_over_the_fade_length_and_no_motion_lands() {
    let mut reduced = app(MotionSettings::reduced());
    let ui_reduced = ui(&mut reduced, 0.0);
    let at = on_the_row(&reduced, &ui_reduced);
    pointer(&mut reduced, at, PointerAction::Move { delta: Vec2::ZERO });
    let fade = follow(&mut reduced, ui_reduced.close, 30);
    let fade_frames =
        (reduced.world().resource::<GooeySkin>().motion.fade_seconds / FRAME).ceil() as usize;
    let first_full = fade.iter().position(|&o| o >= 1.0).expect("it reaches 1");
    assert!(
        first_full.abs_diff(fade_frames) <= 3,
        "fully shown after about {fade_frames} frames, not {first_full}: {fade:?}"
    );
    let steps: Vec<f32> = fade
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .filter(|s| *s > 1e-6)
        .collect();
    let first = steps[0];
    assert!(
        steps[..steps.len() - 1]
            .iter()
            .all(|s| (s - first).abs() < 1e-3),
        "a straight fade, no spring: {steps:?}"
    );

    let mut off = app(MotionSettings::off());
    let ui_off = ui(&mut off, 0.0);
    let at = on_the_row(&off, &ui_off);
    pointer(&mut off, at, PointerAction::Move { delta: Vec2::ZERO });
    off.update();
    assert_eq!(opacity(&off, ui_off.close), 1.0, "no motion: it lands");
}

#[test]
fn a_settled_group_writes_no_opacity() {
    #[derive(Resource, Default)]
    struct Writes(u32);
    let mut app = app(MotionSettings::default());
    app.init_resource::<Writes>().add_systems(
        Update,
        |changed: Query<(), Changed<Opacity>>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    let ui = ui(&mut app, 0.0);
    let at = on_the_row(&app, &ui);
    move_to(&mut app, at);
    frames(&mut app, 60);
    app.world_mut().resource_mut::<Writes>().0 = 0;
    frames(&mut app, 30);
    assert_eq!(app.world().resource::<Writes>().0, 0, "settled revealed");
    let away = rect(&app, ui.elsewhere).center();
    move_to(&mut app, away);
    frames(&mut app, 60);
    app.world_mut().resource_mut::<Writes>().0 = 0;
    frames(&mut app, 30);
    assert_eq!(app.world().resource::<Writes>().0, 0, "settled at rest");
}
