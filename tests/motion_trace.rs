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
    trace.step(&mut app, 30);
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
    let allowed = model::spring_settle_seconds(
        GooeySkin::default().motion.snappy,
        model::SpringState {
            value: DIALOG_SLIDE_PX,
            velocity: 0.0,
        },
        0.0,
    );
    assert!(
        settled <= allowed + 2.0 * FRAME,
        "settled after {settled} s of the {allowed} s the spring needs"
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
    let settled = trace.settled_after_seconds().expect("it stopped");
    assert!((settled - 3.0 * FRAME).abs() < 1e-4, "{settled}");
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
        // One node goes at 10 px a frame, then stops dead; the other follows a spring to 100.
        move_to(&mut app, snapped, (frame.min(10) * 10) as f32);
        state = model::spring_step(spring, state, 100.0, FRAME);
        move_to(&mut app, sprung, state.value);
        a.step(&mut app, 1);
        b.step(&mut app, 1);
    }
    // How much of its top speed the biggest change takes: all of it for a stop dead, a fraction
    // for a spring (it needs time to speed up and slow down).
    let share = |t: &MotionTrace| t.max_speed_change_px_per_s() / t.max_speed_px_per_s();
    assert!(
        share(&a) > 0.95,
        "stopping dead from full speed: {}",
        share(&a)
    );
    assert!(share(&b) < 0.5, "a spring: {}", share(&b));
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
        10,
        "a write of the value it had is a write too"
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

/// A node with a track at rest on its place, updated until it is.
fn tracked(app: &mut App, target: MotionTarget) -> Entity {
    let node = app.world_mut().spawn((panel(), target)).id();
    for _ in 0..60 {
        app.update();
    }
    node
}

fn aim(app: &mut App, node: Entity, translation: Vec2) {
    app.world_mut()
        .get_mut::<MotionTarget>(node)
        .expect("a track")
        .translation = translation;
}

/// The biggest frame step of an uninterrupted entrance of the dialog's own rise.
fn open_step(app: &mut App) -> f32 {
    let node = app
        .world_mut()
        .spawn((
            panel(),
            MotionTarget::default().entering_from(Vec2::new(0.0, DIALOG_SLIDE_PX)),
        ))
        .id();
    app.update();
    let mut trace = MotionTrace::watch(app, node);
    trace.step(app, 40);
    trace.max_step_px()
}

#[test]
fn reopening_a_card_partway_through_closing_has_no_step_larger_than_an_open() {
    let mut app = app();
    let uninterrupted = open_step(&mut app);
    assert!(
        uninterrupted > 0.0 && uninterrupted < DIALOG_SLIDE_PX,
        "{uninterrupted}"
    );

    let card = tracked(
        &mut app,
        MotionTarget::default().entering_from(Vec2::new(0.0, DIALOG_SLIDE_PX)),
    );
    let mut trace = MotionTrace::watch(&mut app, card);
    // Closing: the card goes back down; 0.05 s in (two frames of 0.03 s), it is reopened.
    aim(&mut app, card, Vec2::new(0.0, DIALOG_SLIDE_PX));
    trace.step(&mut app, 2);
    let low = trace
        .frames()
        .last()
        .and_then(|f| f.position)
        .expect("laid out")
        .y
        - trace.frames()[0].position.expect("laid out").y;
    assert!(
        low > 0.0 && low < DIALOG_SLIDE_PX,
        "it is partway down: {low}"
    );
    aim(&mut app, card, Vec2::ZERO);
    trace.step(&mut app, 40);
    assert!(
        trace.max_step_px() <= uninterrupted + 1e-3,
        "no step bigger than an open's {uninterrupted}: {}",
        trace.max_step_px()
    );
    assert!(
        trace.settled_after_seconds().is_some(),
        "it comes back to rest"
    );
    assert_eq!(
        app.world().get::<UiTransform>(card).map(|t| t.translation),
        Some(Val2::ZERO)
    );
}

#[test]
fn a_retarget_keeps_the_speed_the_node_had_and_a_snap_would_not() {
    let mut app = app();
    let node = tracked(&mut app, MotionTarget::default());
    let mut trace = MotionTrace::watch(&mut app, node);
    aim(&mut app, node, Vec2::new(200.0, 0.0));
    trace.step(&mut app, 6);
    // Reverse in mid-flight: the speed turns through zero over frames, never in one.
    aim(&mut app, node, Vec2::ZERO);
    trace.step(&mut app, 30);
    let speed_change = trace.max_speed_change_px_per_s();
    // A hard reversal at the speed it had would change speed by twice that in one frame.
    let top = trace
        .frames()
        .windows(2)
        .filter_map(|w| Some(w[1].position?.distance(w[0].position?) / w[1].delta))
        .fold(0.0, f32::max);
    assert!(top > 100.0, "it was moving: {top} px/s");
    assert!(
        speed_change < top,
        "the speed turns smoothly: changed {speed_change} px/s at most, going {top} px/s"
    );
}

#[test]
fn the_first_target_lands_where_it_says_with_no_travel_from_spawn() {
    let mut app = app();
    let node = app
        .world_mut()
        .spawn((
            panel(),
            MotionTarget {
                translation: Vec2::new(50.0, 20.0),
                scale: Vec2::splat(1.5),
                rotation: 0.5,
                ..default()
            },
        ))
        .id();
    let mut trace = MotionTrace::watch(&mut app, node);
    trace.step(&mut app, 6);
    let transform = trace.frames()[1].transform.expect("a node");
    assert_eq!(transform.translation, Val2::px(50.0, 20.0));
    assert_eq!(transform.scale, Vec2::splat(1.5));
    assert!((transform.rotation.as_radians() - 0.5).abs() < 1e-6);
    // The first update lands it; nothing after it travels.
    assert_eq!(trace.frames()[2].transform, trace.frames()[1].transform);
    // Everything after its first update is still.
    assert_eq!(trace.settled_after_seconds(), Some(FRAME));
    assert_eq!(trace.writes_at_rest(4), 0);
}

#[test]
fn scale_and_rotation_travel_on_the_same_spring_and_land_on_their_target() {
    let mut app = app();
    let node = tracked(&mut app, MotionTarget::default());
    app.world_mut().entity_mut(node).insert(MotionTarget {
        scale: Vec2::splat(2.0),
        rotation: 1.0,
        ..default()
    });
    let mut trace = MotionTrace::watch(&mut app, node);
    trace.step(&mut app, 2);
    let partway = trace
        .frames()
        .last()
        .and_then(|f| f.transform)
        .expect("a node");
    assert!(
        partway.scale.x > 1.0 && partway.scale.x < 2.0,
        "{partway:?}"
    );
    assert!(partway.rotation.as_radians() > 0.0 && partway.rotation.as_radians() < 1.0);
    trace.step(&mut app, 40);
    let end = trace
        .frames()
        .last()
        .and_then(|f| f.transform)
        .expect("a node");
    assert_eq!(end.scale, Vec2::splat(2.0));
    assert!((end.rotation.as_radians() - 1.0).abs() < 1e-6);
}

#[test]
fn a_game_that_writes_the_transform_while_a_track_runs_wins() {
    let mut app = app();
    let node = tracked(&mut app, MotionTarget::default());
    aim(&mut app, node, Vec2::new(300.0, 0.0));
    app.update();
    app.update();
    move_to(&mut app, node, 77.0);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world().get::<UiTransform>(node).map(|t| t.translation),
        Some(Val2::px(77.0, 0.0)),
        "the game's value stands"
    );
    assert!(
        app.world().get::<MotionTarget>(node).is_none(),
        "the track is dropped"
    );
}

#[test]
fn an_instant_change_lands_at_once_and_only_that_one() {
    let mut app = app();
    let node = tracked(&mut app, MotionTarget::default());
    app.world_mut().entity_mut(node).insert(InstantMotion);
    aim(&mut app, node, Vec2::new(120.0, 0.0));
    app.update();
    assert_eq!(
        app.world().get::<UiTransform>(node).map(|t| t.translation),
        Some(Val2::px(120.0, 0.0))
    );
    assert!(app.world().get::<InstantMotion>(node).is_none());
    // The next change moves.
    aim(&mut app, node, Vec2::ZERO);
    app.update();
    let x = match app
        .world()
        .get::<UiTransform>(node)
        .map(|t| t.translation.x)
    {
        Some(Val::Px(x)) => x,
        other => panic!("{other:?}"),
    };
    assert!(x > 0.0 && x < 120.0, "{x}");
}

#[test]
fn doubling_the_duration_scale_doubles_the_settle_time_within_a_frame() {
    let settle = |scale: f32| {
        let mut app = app();
        app.insert_resource(MotionSettings {
            duration_scale: scale,
        });
        let node = tracked(&mut app, MotionTarget::default());
        let mut trace = MotionTrace::watch(&mut app, node);
        aim(&mut app, node, Vec2::new(100.0, 0.0));
        trace.step(&mut app, 120);
        trace.settled_after_seconds().expect("it settles")
    };
    let (one, two) = (settle(1.0), settle(2.0));
    assert!(
        (two - 2.0 * one).abs() <= FRAME + 1e-4,
        "{one} s and {two} s"
    );
}

#[test]
fn a_settled_track_writes_nothing_for_thirty_frames() {
    let mut app = app();
    let node = tracked(&mut app, MotionTarget::default());
    let mut trace = MotionTrace::watch(&mut app, node);
    trace.step(&mut app, 30);
    assert_eq!(trace.writes_at_rest(30), 0);
    assert_eq!(trace.settled_after_seconds(), Some(0.0));
}

#[test]
fn a_target_that_is_not_a_number_is_refused_and_the_node_stays_where_it_is() {
    let mut app = app();
    let node = tracked(&mut app, MotionTarget::default());
    aim(&mut app, node, Vec2::new(f32::NAN, 0.0));
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        app.world().get::<UiTransform>(node).map(|t| t.translation),
        Some(Val2::ZERO)
    );
}
