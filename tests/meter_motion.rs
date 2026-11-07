//! A meter's motion as a game sees it, laid out in the same headless window on a clock the test
//! sets (`TimeUpdateStrategy::ManualDuration`, 50 ms a frame): a loss drops the fill at once and
//! leaves a trail of the lost length that holds and drains, a gain eases up with no trail, the
//! first frame is the value, one setting turns it all off, a paused game's bar still settles, and
//! a settled bar writes nothing.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 0.05;

/// How many `Node`s changed.
#[derive(Resource, Default)]
struct NodeWrites(u32);

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
    app.init_resource::<NodeWrites>().add_systems(
        Update,
        |changed: Query<(), Changed<Node>>, mut writes: ResMut<NodeWrites>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    app.world_mut().spawn(Camera2d);
    // The first update only starts the clock.
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// A meter reading `value` of 100 in a panel, laid out; its track.
fn bar(app: &mut App, value: f32) -> Entity {
    let track = app.world_mut().spawn(meter(value, 100.0)).id();
    let panel = app.world_mut().spawn(panel()).add_child(track).id();
    app.world_mut().spawn(screen()).add_child(panel);
    frames(app, 3);
    track
}

fn set(app: &mut App, track: Entity, value: f32) {
    app.world_mut()
        .entity_mut(track)
        .insert(Meter { value, max: 100.0 });
}

fn part(app: &App, track: Entity, role: Role) -> Entity {
    app.world()
        .get::<Children>(track)
        .and_then(|c| {
            c.iter()
                .find(|e| app.world().get::<Role>(*e) == Some(&role))
        })
        .expect("the part")
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

/// The fill's and the trail's drawn lengths as a share of the room inside the track's frame.
fn drawn(app: &App, track: Entity) -> (f32, f32) {
    let node = app.world().get::<ComputedNode>(track).expect("laid out");
    let room = node.size().x
        - node.border().min_inset.x
        - node.border().max_inset.x
        - node.padding().min_inset.x
        - node.padding().max_inset.x;
    let fill = rect(app, part(app, track, Role::MeterFill)).width();
    let trail = app
        .world()
        .get::<ComputedNode>(part(app, track, Role::MeterTrail))
        .expect("laid out");
    let trail = if trail.size().x > 0.0 {
        rect(app, part(app, track, Role::MeterTrail)).width()
    } else {
        0.0
    };
    // A trail starts at the bar's left edge, under the fill, and is as long as its share says.
    (fill / room, trail / room)
}

#[test]
fn the_first_frame_is_the_value_never_a_rise_from_empty() {
    let mut app = app();
    let track = bar(&mut app, 60.0);
    let (fill, trail) = drawn(&app, track);
    assert!((fill - 0.6).abs() < 0.01, "{fill}");
    assert_eq!(trail, 0.0, "no trail at rest");
}

#[test]
fn a_loss_drops_the_fill_at_once_and_the_trail_holds_then_drains_to_it() {
    let mut app = app();
    let track = bar(&mut app, 100.0);
    set(&mut app, track, 40.0);
    app.update();
    let (fill, trail) = drawn(&app, track);
    assert!(
        (fill - 0.4).abs() < 0.01,
        "exact the frame it lands: {fill}"
    );
    assert!(
        (trail - 1.0).abs() < 0.01,
        "the lost length stands: {trail}"
    );
    // The hold: 0.45 s is nine frames of 50 ms, and the trail has not moved by the eighth.
    frames(&mut app, 7);
    let (_, held) = drawn(&app, track);
    assert!((held - 1.0).abs() < 0.01, "{held}");
    // Then it only falls, to the fill, and is zero wide after.
    let mut last = held;
    let mut after = 0;
    loop {
        app.update();
        after += 1;
        let (fill, trail) = drawn(&app, track);
        assert!((fill - 0.4).abs() < 0.01);
        assert!(trail <= last + 0.001, "{trail} after {last}");
        assert!(trail == 0.0 || trail >= fill, "{trail} under the fill");
        last = trail;
        if trail == 0.0 {
            break;
        }
        assert!(after < 40, "never drained: {trail}");
    }
    // 0.6 of the bar at 1.5 a second is 0.4 s: eight frames, and the hold's last one.
    assert!((7..=11).contains(&after), "{after} frames");
}

#[test]
fn a_gain_eases_up_and_draws_no_trail() {
    let mut app = app();
    let track = bar(&mut app, 20.0);
    set(&mut app, track, 90.0);
    app.update();
    let (first, trail) = drawn(&app, track);
    assert!(
        first > 0.25 && first < 0.85,
        "partway in the first frame: {first}"
    );
    assert_eq!(trail, 0.0);
    let mut last = first;
    for _ in 0..12 {
        app.update();
        let (fill, trail) = drawn(&app, track);
        assert!(fill >= last - 0.001 && fill <= 0.91, "{fill} after {last}");
        assert_eq!(trail, 0.0);
        last = fill;
    }
    assert!((last - 0.9).abs() < 0.01, "arrived within 0.6 s: {last}");
}

#[test]
fn the_trail_starts_at_the_bars_left_edge_and_is_the_fills_height_in_danger() {
    let mut app = app();
    let skin = app.world().resource::<GooeySkin>().clone();
    let track = bar(&mut app, 100.0);
    set(&mut app, track, 30.0);
    app.update();
    let trail_entity = part(&app, track, Role::MeterTrail);
    let (fill, trail) = (
        rect(&app, part(&app, track, Role::MeterFill)),
        rect(&app, trail_entity),
    );
    assert!((trail.min.x - fill.min.x).abs() < 0.6, "{fill:?} {trail:?}");
    assert!(trail.width() > fill.width() * 2.0, "{fill:?} {trail:?}");
    assert!((trail.height() - fill.height()).abs() < 0.6 && (trail.min.y - fill.min.y).abs() < 0.6);
    let frame = rect(&app, track);
    assert!(
        trail.max.x <= frame.max.x + 0.01,
        "inside the frame: {frame:?} {trail:?}"
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(trail_entity),
        Some(&BackgroundColor(skin.danger))
    );
    let radius = |e: Entity| app.world().get::<Node>(e).expect("a node").border_radius;
    assert_eq!(
        radius(trail_entity),
        radius(part(&app, track, Role::MeterFill)),
        "rounded like the fill"
    );
}

#[test]
fn a_second_loss_in_the_hold_does_not_restart_it() {
    let mut app = app();
    let track = bar(&mut app, 100.0);
    set(&mut app, track, 80.0);
    frames(&mut app, 5);
    set(&mut app, track, 50.0);
    // 0.25 s in, a second loss: the trail is still the first one's start and drains on the
    // first loss's clock: by 9 + 2 frames it has begun to fall.
    frames(&mut app, 1);
    let (_, trail) = drawn(&app, track);
    assert!((trail - 1.0).abs() < 0.01, "{trail}");
    frames(&mut app, 5);
    let (fill, trail) = drawn(&app, track);
    assert!((fill - 0.5).abs() < 0.01);
    assert!(
        trail < 0.97,
        "a restarted hold would stand until 0.45 s later: {trail}"
    );
}

#[test]
fn a_gain_lifts_a_standing_trail_with_the_fill() {
    let mut app = app();
    let track = bar(&mut app, 100.0);
    set(&mut app, track, 20.0);
    frames(&mut app, 2);
    set(&mut app, track, 70.0);
    for _ in 0..30 {
        app.update();
        let (fill, trail) = drawn(&app, track);
        assert!(trail == 0.0 || trail >= fill, "{fill} {trail}");
    }
    let (fill, trail) = drawn(&app, track);
    assert!((fill - 0.7).abs() < 0.01 && trail == 0.0, "{fill} {trail}");
}

#[test]
fn reduce_motion_makes_both_instant() {
    let mut app = app();
    app.insert_resource(MotionSettings::off());
    let track = bar(&mut app, 100.0);
    set(&mut app, track, 40.0);
    app.update();
    assert_eq!(drawn(&app, track).1, 0.0, "no trail");
    assert!((drawn(&app, track).0 - 0.4).abs() < 0.01);
    set(&mut app, track, 90.0);
    app.update();
    let (fill, trail) = drawn(&app, track);
    assert!(
        (fill - 0.9).abs() < 0.01 && trail == 0.0,
        "a gain at once: {fill}"
    );
}

#[test]
fn turning_it_off_mid_trail_ends_it_in_the_next_frame() {
    let mut app = app();
    let track = bar(&mut app, 100.0);
    set(&mut app, track, 40.0);
    frames(&mut app, 2);
    assert!(drawn(&app, track).1 > 0.9);
    app.insert_resource(MotionSettings::off());
    // The meter is only looked at again when it changes or is not settled: a trail is not settled.
    frames(&mut app, 2);
    assert_eq!(drawn(&app, track).1, 0.0);
}

#[test]
fn a_slower_scale_takes_proportionally_longer() {
    let mut app = app();
    app.insert_resource(MotionSettings {
        duration_scale: 2.0,
        reduced: false,
    });
    let track = bar(&mut app, 100.0);
    set(&mut app, track, 40.0);
    // At 1x the trail is falling by frame 11; at 2x the hold alone lasts 18 frames.
    frames(&mut app, 14);
    assert!((drawn(&app, track).1 - 1.0).abs() < 0.01);
    frames(&mut app, 40);
    assert_eq!(drawn(&app, track).1, 0.0);
}

#[test]
fn a_paused_game_still_settles_the_bar() {
    let mut app = app();
    let track = bar(&mut app, 100.0);
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    set(&mut app, track, 40.0);
    frames(&mut app, 30);
    assert_eq!(drawn(&app, track).1, 0.0);
}

#[test]
fn a_settled_bar_writes_nothing() {
    let mut app = app();
    let track = bar(&mut app, 100.0);
    set(&mut app, track, 40.0);
    frames(&mut app, 40);
    assert_eq!(drawn(&app, track).1, 0.0);
    frames(&mut app, 3);
    app.world_mut().resource_mut::<NodeWrites>().0 = 0;
    frames(&mut app, 30);
    assert_eq!(app.world().resource::<NodeWrites>().0, 0);
}

#[test]
fn a_meter_with_a_bad_maximum_reads_empty_and_draws_no_trail() {
    let mut app = app();
    let track = bar(&mut app, 50.0);
    app.world_mut().entity_mut(track).insert(Meter {
        value: 50.0,
        max: 0.0,
    });
    frames(&mut app, 3);
    assert_eq!(drawn(&app, track).0, 0.0);
    // The trail of the 50% the bar lost drains like any other.
    frames(&mut app, 40);
    assert_eq!(drawn(&app, track), (0.0, 0.0));
}
