//! Toasts enter from their edge (G-65): a new toast starts `TOAST_SLIDE_PX` toward the window edge its
//! stack is anchored to and settles on the snappy spring while its own fade brings it in. Reduced motion
//! and no motion only fade, as before; a toast already standing is not moved by a new one. A headless
//! window on a 60 Hz clock the test sets, with motion on (the toast tests of `tests/toast.rs` run with
//! it off).

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{WindowPlugin, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

/// How close to the start's other end counts as arrived: the critically damped spring is 98.6% of the way
/// at its perceptual duration, so 2% leaves a frame of room.
const LEFT: f32 = 0.02;

fn app(motion: MotionSettings, toasts: ToastSettings) -> App {
    let mut app = App::new();
    app.insert_resource(motion).insert_resource(toasts);
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
    )));
    app.world_mut().spawn(Camera2d);
    for _ in 0..5 {
        app.update();
    }
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// How far a toast's transform is from its place vertically, logical px (0 when it has none).
fn offset_y(app: &App, toast: Entity) -> f32 {
    match app
        .world()
        .get::<UiTransform>(toast)
        .map(|t| t.translation.y)
    {
        Some(Val::Px(y)) => y,
        Some(other) => panic!("a px offset, not {other:?}"),
        None => 0.0,
    }
}

/// The offset now, then after each of the next `n` frames.
fn follow(app: &mut App, toast: Entity, n: usize) -> Vec<f32> {
    let mut seen = vec![offset_y(app, toast)];
    for _ in 0..n {
        app.update();
        seen.push(offset_y(app, toast));
    }
    seen
}

fn spawn(app: &mut App, text: &str) -> Entity {
    let id = app.world_mut().spawn(toast(text)).id();
    app.update();
    id
}

fn snappy_frames(app: &App) -> usize {
    let spring = app.world().resource::<GooeySkin>().motion.snappy;
    (spring.duration / FRAME).round() as usize + 1
}

#[test]
fn a_toast_in_a_bottom_stack_starts_below_its_place_and_settles_on_the_snappy_spring() {
    let mut app = app(MotionSettings::default(), ToastSettings::default());
    let first = spawn(&mut app, "Saved");
    let rise = follow(&mut app, first, 40);
    assert!(
        rise[0] > 0.9 * TOAST_SLIDE_PX && rise[0] <= TOAST_SLIDE_PX + 0.5,
        "its first drawn frame stands toward the bottom edge: {}",
        rise[0]
    );
    assert!(
        rise.windows(2).all(|pair| pair[1] <= pair[0] + 1e-3),
        "it only ever rises: {rise:?}"
    );
    let snappy = snappy_frames(&app);
    assert!(
        rise[snappy] <= LEFT * TOAST_SLIDE_PX,
        "within motion.snappy it is in place: {} after {snappy} frames",
        rise[snappy]
    );
    assert_eq!(*rise.last().expect("frames"), 0.0, "and it settles exactly");
}

#[test]
fn a_toast_in_a_top_stack_starts_above_its_place() {
    let settings = ToastSettings {
        anchor: ToastAnchor::TopRight,
        ..default()
    };
    let mut app = app(MotionSettings::default(), settings);
    let first = spawn(&mut app, "Saved");
    let fall = follow(&mut app, first, 40);
    assert!(
        fall[0] < -0.9 * TOAST_SLIDE_PX,
        "its first drawn frame stands toward the top edge: {}",
        fall[0]
    );
    assert_eq!(*fall.last().expect("frames"), 0.0);
}

#[test]
fn a_toast_already_standing_is_not_moved_by_one_that_arrives_or_one_that_leaves() {
    let settings = ToastSettings {
        max_visible: 2,
        ..default()
    };
    let mut app = app(MotionSettings::default(), settings);
    let first = spawn(&mut app, "One");
    frames(&mut app, 40);
    assert_eq!(offset_y(&app, first), 0.0);
    let second = spawn(&mut app, "Two");
    assert!(
        offset_y(&app, second) > 0.9 * TOAST_SLIDE_PX,
        "the new one enters"
    );
    let still = follow(&mut app, first, 30);
    assert!(
        still.iter().all(|&y| y == 0.0),
        "the standing one is not moved: {still:?}"
    );
    // A third sends the oldest out at once; the second, standing now, is not moved either.
    frames(&mut app, 40);
    let third = spawn(&mut app, "Three");
    assert!(
        offset_y(&app, third) > 0.9 * TOAST_SLIDE_PX,
        "the third enters too"
    );
    let held = follow(&mut app, second, 30);
    assert!(
        held.iter().all(|&y| y == 0.0),
        "the one that stays is not moved while the oldest leaves: {held:?}"
    );
}

#[test]
fn reduced_motion_and_no_motion_give_a_toast_no_slide() {
    for (name, motion) in [
        ("reduced", MotionSettings::reduced()),
        ("off", MotionSettings::off()),
    ] {
        let mut app = app(motion, ToastSettings::default());
        let first = spawn(&mut app, "Saved");
        let seen = follow(&mut app, first, 30);
        assert!(
            seen.iter().all(|&y| y == 0.0),
            "{name}: a toast only fades: {seen:?}"
        );
    }
}
