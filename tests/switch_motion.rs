//! Switch motion (G-60): a toggle's knob travels from one end of its track to the other on the
//! skin's snappy spring instead of jumping, while the state, the track colours and the first frame
//! are unchanged. Laid out for real in a headless window on a 60 Hz clock the test sets.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::model::{SpringState, spring_settle_seconds, spring_step};
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

/// How many frames the test follows a flip: a second, longer than any spring here settles.
const PATIENCE_FRAMES: usize = 60;

/// The most of its travel the knob may draw in one frame: a third, so a flip is seen as travel.
const MOST_OF_THE_TRAVEL_IN_ONE_FRAME: f32 = 1.0 / 3.0;

/// How much of the travel may be left one frame after the spring's perceptual duration: the
/// critically damped spring is 98.6% of the way at its duration, so 2% leaves a frame of room and
/// is still tighter than the default spring's 3.3% at the same time (the control below).
const LEFT_AT_THE_DURATION: f32 = 0.02;

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

/// A toggle and its knob, laid out and settled.
fn switch(app: &mut App, on: bool) -> (Entity, Entity) {
    let toggle = app.world_mut().spawn(toggle("Subtitles", on)).id();
    frames(app, 10);
    (toggle, knob_of(app, toggle))
}

/// The knob under `toggle`.
fn knob_of(app: &mut App, toggle: Entity) -> Entity {
    let world = app.world_mut();
    let mut knobs = world.query::<(Entity, &Role)>();
    let found: Vec<Entity> = knobs
        .iter(world)
        .filter(|(_, role)| **role == Role::ToggleKnob)
        .map(|(entity, _)| entity)
        .filter(|&knob| {
            world
                .get::<ChildOf>(knob)
                .and_then(|track| world.get::<ChildOf>(track.parent()))
                .is_some_and(|row| row.parent() == toggle)
        })
        .collect();
    assert_eq!(found.len(), 1, "one knob under the toggle");
    found[0]
}

/// Where the knob is drawn: its centre, logical px (the window's scale factor is 1).
fn knob_x(app: &App, knob: Entity) -> f32 {
    app.world()
        .get::<UiGlobalTransform>(knob)
        .expect("placed")
        .translation
        .x
}

/// The knob's drawn x now, then after each of the next [`PATIENCE_FRAMES`] frames.
fn follow(app: &mut App, knob: Entity) -> Vec<f32> {
    let mut xs = vec![knob_x(app, knob)];
    for _ in 0..PATIENCE_FRAMES {
        app.update();
        xs.push(knob_x(app, knob));
    }
    xs
}

/// The flip is a glide: the same direction every frame, none more than a third of the travel.
fn assert_glides(xs: &[f32]) {
    let total = xs[xs.len() - 1] - xs[0];
    assert!(total.abs() > 4.0, "the knob moved only {total} px");
    for pair in xs.windows(2) {
        let step = (pair[1] - pair[0]) * total.signum();
        assert!(step >= -0.01, "the knob went back by {step}");
        assert!(
            step <= MOST_OF_THE_TRAVEL_IN_ONE_FRAME * total.abs() + 1e-3,
            "the knob jumped {step} of {total} in one frame"
        );
    }
}

/// How far from the end a critically damped spring of `duration` s is after `frames` frames of a
/// travel of 1: what the spring itself says, from the same stepper the crate runs.
fn left_after(spring: model::Spring, frames: usize) -> f32 {
    let mut state = SpringState {
        value: 1.0,
        velocity: 0.0,
    };
    for _ in 0..frames {
        state = spring_step(spring, state, 0.0, FRAME);
    }
    state.value
}

#[test]
fn a_knob_glides_to_the_other_end_and_is_nearly_there_in_the_snappy_duration() {
    let mut app = app(MotionSettings::default());
    let (toggle, knob) = switch(&mut app, false);
    let start = knob_x(&app, knob);

    // The game sets the value with no event, as a loaded setting does.
    app.world_mut().entity_mut(toggle).insert(Checked);
    let xs = follow(&mut app, knob);
    assert_glides(&xs);

    let end = xs[xs.len() - 1];
    let travel = end - start;
    assert!(travel > 4.0, "the knob moved right by {travel}");
    // Layout puts it at the end from the first frame; the settled knob is exactly there.
    assert!(
        (xs[xs.len() - 1] - xs[xs.len() - 2]).abs() < 1e-3,
        "the knob is at rest at the end"
    );
    assert_eq!(
        app.world()
            .get::<LayoutTravel>(knob)
            .map(LayoutTravel::offset),
        Some(Vec2::ZERO)
    );

    let snappy = app.world().resource::<GooeySkin>().motion.snappy;
    let at_the_duration = (snappy.duration / FRAME).round() as usize + 1;
    let left = (end - xs[at_the_duration]) / travel;
    assert!(
        left <= LEFT_AT_THE_DURATION,
        "after {at_the_duration} frames {left} of the travel is left"
    );
    // And it is not there much sooner than the spring says: the first frame is a small step.
    let first = (xs[1] - xs[0]) / travel;
    assert!(first < 0.2, "the first frame drew {first} of the travel");
}

#[test]
fn the_knob_runs_on_the_snappy_spring_not_the_default_one() {
    let skin = GooeySkin::dark();
    let snappy = left_after(skin.motion.snappy, 16);
    let default = left_after(skin.motion.default, 16);
    // The control: the same stepper at the same frame separates the two springs, so the test
    // above can tell them apart, and the knob's own track is the snappy one.
    assert!(snappy <= LEFT_AT_THE_DURATION, "snappy leaves {snappy}");
    assert!(default > LEFT_AT_THE_DURATION, "default leaves {default}");

    let mut app = app(MotionSettings::default());
    let (toggle, knob) = switch(&mut app, false);
    assert_eq!(
        app.world().get::<TravelSpring>(knob),
        Some(&TravelSpring(MotionToken::Snappy))
    );
    app.world_mut().entity_mut(toggle).insert(Checked);
    let xs = follow(&mut app, knob);
    let travel = xs[xs.len() - 1] - xs[0];
    // Sixteen frames in, a default-spring knob would still be 3% short; this one is within 2%.
    let left = (xs[xs.len() - 1] - xs[16]) / travel;
    assert!(left <= LEFT_AT_THE_DURATION, "left {left}");
    let settle = spring_settle_seconds(
        app.world().resource::<GooeySkin>().motion.snappy,
        SpringState {
            value: travel,
            velocity: 0.0,
        },
        0.0,
    );
    assert!(settle > 0.0 && settle < 1.0, "settles in {settle} s");
}

#[test]
fn flipping_back_mid_flight_has_no_jump() {
    let mut app = app(MotionSettings::default());
    let (toggle, knob) = switch(&mut app, true);
    let on_x = knob_x(&app, knob);
    app.world_mut().entity_mut(toggle).remove::<Checked>();
    frames(&mut app, PATIENCE_FRAMES);
    let off_x = knob_x(&app, knob);
    let travel = on_x - off_x;
    assert!(travel > 4.0, "the knob has a track to cross: {travel}");

    app.world_mut().entity_mut(toggle).insert(Checked);
    frames(&mut app, 4);
    let mid = knob_x(&app, knob);
    assert!(
        mid > off_x + 0.5 && mid < on_x - 0.5,
        "four frames in the knob is on its way: {mid} of {off_x}..{on_x}"
    );

    // Back again while it is drawn in the middle: each frame steps less than a third of the
    // travel, never leaves the track, and the drawn knob does not jump to either end.
    app.world_mut().entity_mut(toggle).remove::<Checked>();
    let xs = follow(&mut app, knob);
    let mut before = mid;
    for x in &xs {
        assert!(
            (x - before).abs() <= MOST_OF_THE_TRAVEL_IN_ONE_FRAME * travel + 1e-3,
            "the knob jumped from {before} to {x}"
        );
        assert!(
            *x >= off_x - 0.5 && *x <= on_x + 0.5,
            "the knob left the track: {x}"
        );
        before = *x;
    }
    assert!(
        (xs[xs.len() - 1] - off_x).abs() < 1e-3,
        "and ends at the off end"
    );
}

#[test]
fn a_toggle_that_starts_on_has_its_knob_at_the_end_from_its_first_frames() {
    let mut app = app(MotionSettings::default());
    let on = app.world_mut().spawn(toggle("Subtitles", true)).id();
    let off = app.world_mut().spawn(toggle("Captions", false)).id();
    // The knob's x within its own track, every frame from the spawn: it never travels.
    let mut on_knob = None;
    let mut off_knob = None;
    let mut seen = Vec::new();
    for _ in 0..12 {
        app.update();
        if on_knob.is_none() {
            on_knob = Some(knob_of(&mut app, on));
            off_knob = Some(knob_of(&mut app, off));
        }
        let (Some(on_knob), Some(off_knob)) = (on_knob, off_knob) else {
            continue;
        };
        seen.push((knob_x(&app, on_knob), knob_x(&app, off_knob)));
    }
    let last = seen[seen.len() - 1];
    assert!(last.0 > last.1 + 4.0, "on is to the right of off: {last:?}");
    for (frame, pair) in seen.windows(2).enumerate() {
        assert!(
            (pair[1].0 - pair[0].0).abs() < 1e-3 && (pair[1].1 - pair[0].1).abs() < 1e-3,
            "the knobs stand still from their first frame (frame {frame}): {pair:?}"
        );
    }
}

#[test]
fn reduced_and_no_motion_land_the_knob_in_one_frame() {
    for settings in [MotionSettings::off(), MotionSettings::reduced()] {
        let mut app = app(settings);
        let (toggle, knob) = switch(&mut app, false);
        let start = knob_x(&app, knob);
        app.world_mut().entity_mut(toggle).insert(Checked);
        let xs = follow(&mut app, knob);
        let end = xs[xs.len() - 1];
        assert!(end - start > 4.0, "the knob moved");
        assert!(
            (xs[1] - end).abs() < 1e-3,
            "landed in the frame it was seen: {} of {end}",
            xs[1]
        );
    }
}

/// A column of two blocks, the first taken out after both are marked, the second given `token`;
/// how far from its place the second is drawn after `after` frames, logical px.
fn left_behind(token: Option<MotionToken>, after: usize) -> f32 {
    let mut app = app(MotionSettings::default());
    let world = app.world_mut();
    let block = || Node {
        width: Val::Px(100.0),
        height: Val::Px(60.0),
        ..default()
    };
    let first = world.spawn((block(), MovesSmoothly)).id();
    let second = world.spawn((block(), MovesSmoothly)).id();
    if let Some(token) = token {
        world.entity_mut(second).insert(TravelSpring(token));
    }
    world
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .add_children(&[first, second]);
    frames(&mut app, 10);
    app.world_mut().entity_mut(first).despawn();
    frames(&mut app, after);
    app.world()
        .get::<LayoutTravel>(second)
        .map(|travel| travel.offset().y.abs())
        .expect("marked")
}

#[test]
fn a_marked_node_travels_on_the_spring_its_travel_spring_names() {
    let frames_in = 12;
    let snappy = left_behind(Some(MotionToken::Snappy), frames_in);
    let default = left_behind(None, frames_in);
    let default_named = left_behind(Some(MotionToken::Default), frames_in);
    let calm = left_behind(Some(MotionToken::Calm), frames_in);
    assert!(
        snappy < default && default < calm,
        "left behind after {frames_in} frames: snappy {snappy}, default {default}, calm {calm}"
    );
    assert!(
        (default - default_named).abs() < 1e-4,
        "no TravelSpring is the default spring: {default} against {default_named}"
    );
}
