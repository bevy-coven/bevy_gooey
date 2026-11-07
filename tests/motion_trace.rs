//! `MotionTrace` (feature `testing`), in the same headless window on a clock the test sets: the
//! checks it offers can fail (a step, a speed snap, a write at rest are seen), and it proves the
//! dialog card's entrance continuous, settled in time and silent afterwards.
#![cfg(feature = "testing")]

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::{MotionTrace, TraceEnded};
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 0.03;

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
    ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )));
    app.world_mut().spawn(Camera2d);
    // The first update only starts the clock.
    app.update();
    app
}

/// A dialog as a game writes it, and its card.
fn confirm(app: &mut App) -> Entity {
    let dialog = app
        .world_mut()
        .spawn((
            dialog(),
            children![(
                panel(),
                children![
                    heading("Leave the game?"),
                    (menu(), children![button("Leave")])
                ]
            )],
        ))
        .id();
    app.world()
        .get::<Children>(dialog)
        .and_then(|c| c.first().copied())
        .expect("the dialog has its card")
}

fn move_to(app: &mut App, entity: Entity, x: f32) {
    app.world_mut()
        .get_mut::<UiTransform>(entity)
        .expect("a node")
        .translation = Val2::px(x, 0.0);
}

#[test]
fn a_dialog_card_rises_without_a_step_settles_in_its_time_and_writes_nothing_after() {
    let mut app = app();
    let card = confirm(&mut app);
    // Its first drawn frame is the start of its entrance.
    app.update();
    let mut trace = MotionTrace::watch(&mut app, card);
    trace.step(&mut app, 12);
    let start = trace.frames()[0].position.expect("laid out").y;
    assert!(
        trace.max_step_px() < DIALOG_SLIDE_PX,
        "no frame jumps the whole rise: {}",
        trace.max_step_px()
    );
    let rose = start
        - trace
            .frames()
            .last()
            .and_then(|f| f.position)
            .expect("laid out")
            .y;
    let started_at = match trace.frames()[0].transform.expect("a node").translation.y {
        Val::Px(y) => y,
        other => panic!("not a px translation: {other:?}"),
    };
    assert!(
        rose > 0.0 && (rose - started_at).abs() < 0.5,
        "it rises the {started_at} px it started low, not {rose}"
    );
    assert!(
        trace.frames().iter().all(|f| f.text.is_none()),
        "a card has no text colour"
    );
    let settled = trace.settled_after_seconds().expect("it settles");
    assert!(
        settled <= DIALOG_SLIDE_SECONDS + FRAME,
        "settled after {settled} s of {DIALOG_SLIDE_SECONDS}"
    );
    trace.step(&mut app, 30);
    assert_eq!(trace.writes_at_rest(30), 0);
    assert_eq!(trace.ended(), None);
}

#[test]
fn a_dialogs_first_frame_is_the_start_of_its_entrance_and_a_settled_panel_is_still() {
    let mut app = app();
    let card = confirm(&mut app);
    let mut trace = MotionTrace::watch(&mut app, card);
    trace.step(&mut app, 12);
    // Spawned and watched in the same moment: the card has not been drawn, and its entrance
    // starts on the frame it first is.
    assert!(!trace.still_on_first_frame());
    let first = trace.frames()[1].transform.expect("a node").translation;
    assert!(matches!(first.y, Val::Px(y) if y > DIALOG_SLIDE_PX * 0.5 && y < DIALOG_SLIDE_PX));

    // A panel with no entrance is where it is from the first frame.
    let plain = app.world_mut().spawn(panel()).id();
    app.update();
    app.update();
    let mut trace = MotionTrace::watch(&mut app, plain);
    trace.step(&mut app, 5);
    assert!(trace.still_on_first_frame());
    assert_eq!(trace.settled_after_seconds(), Some(0.0));
    assert_eq!(trace.max_step_px(), 0.0);
}

#[test]
fn a_node_moved_a_hundred_px_in_one_frame_reports_a_step_of_a_hundred() {
    let mut app = app();
    let node = app.world_mut().spawn(panel()).id();
    app.update();
    app.update();
    let mut trace = MotionTrace::watch(&mut app, node);
    trace.step(&mut app, 2);
    move_to(&mut app, node, 100.0);
    trace.step(&mut app, 3);
    assert!(
        (trace.max_step_px() - 100.0).abs() < 0.01,
        "{}",
        trace.max_step_px()
    );
    // It stopped on the frame after, so it settled one frame after the jump.
    assert_eq!(trace.settled_after_seconds(), Some(3.0 * FRAME));
}

#[test]
fn a_speed_snap_is_seen_and_a_spring_has_none() {
    let mut app = app();
    let snapped = app.world_mut().spawn(panel()).id();
    let sprung = app.world_mut().spawn(panel()).id();
    app.update();
    app.update();
    let mut a = MotionTrace::watch(&mut app, snapped);
    let mut b = MotionTrace::watch(&mut app, sprung);
    let spring = Spring::new(0.3, 0.0).expect("a spring");
    let mut state = model::SpringState::default();
    for frame in 0..40 {
        // One node goes at 5 px a frame, then stops dead; the other follows a spring to 100.
        move_to(&mut app, snapped, (frame.min(10) * 5) as f32);
        state = model::spring_step(spring, state, 100.0, FRAME);
        move_to(&mut app, sprung, state.value);
        a.step(&mut app, 1);
        b.step(&mut app, 1);
    }
    let snap = a.max_speed_change_px_per_s();
    let smooth = b.max_speed_change_px_per_s();
    assert!(snap > 100.0, "5 px a frame stopping dead: {snap}");
    assert!(
        smooth < snap * 0.5,
        "a spring {smooth} against a snap {snap}"
    );
    assert!(b.settled_after_seconds().is_some());
    assert!(a.settled_after_seconds().is_some());
}

#[test]
fn a_node_still_moving_is_not_settled_and_one_written_every_frame_counts_its_writes() {
    let mut app = app();
    let node = app
        .world_mut()
        .spawn((panel(), TextColor(Color::WHITE)))
        .id();
    app.update();
    app.update();
    let mut trace = MotionTrace::watch(&mut app, node);
    for frame in 0..10 {
        move_to(&mut app, node, frame as f32);
        trace.step(&mut app, 1);
    }
    assert_eq!(
        trace.settled_after_seconds(),
        None,
        "moving on the last frame"
    );
    assert_eq!(
        trace.writes_at_rest(10),
        9,
        "the first value is the one it had"
    );
    trace.step(&mut app, 10);
    assert_eq!(trace.writes_at_rest(10), 0, "ten quiet frames");
    assert!(trace.settled_after_seconds().is_some());
    // A component the node lacks is None on every frame; one it has is Some on every frame.
    assert!(trace.frames().iter().all(|f| f.text == Some(Color::WHITE)));
}

#[test]
fn a_write_of_the_same_value_is_still_a_write() {
    let mut app = app();
    let node = app.world_mut().spawn(panel()).id();
    app.update();
    app.update();
    let mut trace = MotionTrace::watch(&mut app, node);
    trace.step(&mut app, 1);
    // A system that writes a value the node already has: nothing moves, and it is a write.
    let mut transform = app
        .world_mut()
        .get_mut::<UiTransform>(node)
        .expect("a node");
    transform.set_changed();
    trace.step(&mut app, 1);
    assert_eq!(trace.writes_at_rest(1), 1);
    trace.step(&mut app, 1);
    assert_eq!(trace.writes_at_rest(1), 0);
    assert_eq!(trace.settled_after_seconds(), Some(0.0));
}

#[test]
fn a_node_despawned_mid_trace_ends_the_trace_and_the_app_goes_on() {
    let mut app = app();
    let node = app.world_mut().spawn(panel()).id();
    app.update();
    let mut trace = MotionTrace::watch(&mut app, node);
    trace.step(&mut app, 3);
    assert_eq!(trace.ended(), None);
    app.world_mut().despawn(node);
    trace.step(&mut app, 4);
    assert_eq!(trace.ended(), Some(TraceEnded { frame: 4 }));
    assert_eq!(
        trace.frames().len(),
        4,
        "the frames before it went are kept"
    );
    // Reads on an ended trace still answer.
    assert_eq!(trace.writes_at_rest(10), 0);
    let gone = MotionTrace::watch(&mut app, node);
    assert_eq!(gone.ended(), Some(TraceEnded { frame: 0 }));
    assert!(gone.frames().is_empty());
    assert!(gone.still_on_first_frame());
    assert_eq!(gone.settled_after_seconds(), None);
}
