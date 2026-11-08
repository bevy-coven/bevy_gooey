//! Pop (`Pop`): a node whose `Pop` is inserted swells by its gain about its centre and settles on
//! the snappy spring while layout keeps its resting size, a second pop on a node still settling
//! stacks on the scale it is drawn at, nothing is drawn swollen under reduced motion, and a settled
//! node writes nothing. Laid out for real in a headless window on a 60 Hz clock.

use std::time::Duration;

use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

/// Frames in which a pop has long settled (a second).
const SETTLED: usize = 60;

/// How many frames a `Popped` changed in.
#[derive(Resource, Default)]
struct Writes(u32);

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
    .insert_resource(settings)
    .init_resource::<Writes>()
    .add_systems(
        Last,
        |changed: Query<(), Changed<Popped>>, mut n: ResMut<Writes>| {
            n.0 += changed.iter().count() as u32;
        },
    );
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// A HUD readout, a panel with a number in it; returns the panel.
fn readout(app: &mut App) -> Entity {
    let panel = app.world_mut().spawn(panel()).id();
    let text = app.world_mut().spawn(label("Coins 41")).id();
    app.world_mut().entity_mut(panel).add_child(text);
    app.world_mut().spawn(screen()).add_child(panel);
    frames(app, 4);
    panel
}

/// The scale the node is drawn at, read off its transform.
fn drawn_scale(app: &App, entity: Entity) -> f32 {
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("laid out");
    Affine2::from(*at).matrix2.x_axis.x
}

fn size(app: &App, entity: Entity) -> Vec2 {
    app.world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size()
}

#[test]
fn a_pop_swells_about_the_centre_by_its_gain_then_settles() {
    let mut app = app(MotionSettings::default());
    let panel = readout(&mut app);
    let (rest_size, rest_scale) = (size(&app, panel), drawn_scale(&app, panel));
    assert!((rest_scale - 1.0).abs() < 1e-4);
    app.world_mut().entity_mut(panel).insert(Pop::default());
    app.update();
    let first = drawn_scale(&app, panel);
    assert!(
        first > 1.0 + 0.8 * POP_GAIN && first <= 1.0 + POP_GAIN,
        "it starts at the gain, a frame in: {first}"
    );
    assert_eq!(
        size(&app, panel),
        rest_size,
        "layout keeps the resting size"
    );
    assert_eq!(
        app.world().get::<Popped>(panel).map(Popped::scale),
        Some(first)
    );
    let mut last = first;
    for _ in 0..SETTLED {
        app.update();
        last = drawn_scale(&app, panel);
    }
    assert!((last - 1.0).abs() < 1e-4, "settled: {last}");
    assert_eq!(
        app.world().get::<Popped>(panel).map(Popped::scale),
        Some(1.0)
    );
}

#[test]
fn a_second_pop_while_settling_stacks_on_what_is_drawn() {
    let mut app = app(MotionSettings::default());
    let panel = readout(&mut app);
    app.world_mut().entity_mut(panel).insert(Pop::default());
    frames(&mut app, 4);
    let before = app.world().get::<Popped>(panel).expect("popped").scale();
    assert!(before > 1.0, "still swollen: {before}");
    app.world_mut().entity_mut(panel).insert(Pop::default());
    app.update();
    let after = app.world().get::<Popped>(panel).expect("popped").scale();
    assert!(after > before + 0.05, "stacked: {before} then {after}");
}

#[test]
fn a_gain_that_is_no_number_above_zero_pops_nothing() {
    let mut app = app(MotionSettings::default());
    let panel = readout(&mut app);
    for gain in [0.0, -0.2, f32::NAN, f32::INFINITY] {
        app.world_mut().entity_mut(panel).insert(Pop(gain));
        frames(&mut app, 3);
        assert_eq!(drawn_scale(&app, panel), 1.0, "gain {gain}");
    }
}

#[test]
fn nothing_is_drawn_swollen_under_reduced_or_no_motion() {
    for settings in [MotionSettings::off(), MotionSettings::reduced()] {
        let mut app = app(settings);
        let panel = readout(&mut app);
        app.world_mut().entity_mut(panel).insert(Pop::default());
        for _ in 0..10 {
            app.update();
            assert_eq!(drawn_scale(&app, panel), 1.0);
        }
    }
}

#[test]
fn a_settled_node_writes_nothing() {
    let mut app = app(MotionSettings::default());
    let panel = readout(&mut app);
    app.world_mut().entity_mut(panel).insert(Pop::default());
    frames(&mut app, SETTLED);
    let writes = app.world().resource::<Writes>().0;
    frames(&mut app, 30);
    assert_eq!(app.world().resource::<Writes>().0, writes);
}

#[test]
fn a_lift_holds_the_swell_while_it_is_there_and_lets_go_with_a_settle() {
    let mut app = app(MotionSettings::default());
    let panel = readout(&mut app);
    app.world_mut().entity_mut(panel).insert(Lift::default());
    frames(&mut app, SETTLED);
    let held = drawn_scale(&app, panel);
    assert!((held - (1.0 + LIFT_GAIN)).abs() < 1e-3, "held: {held}");
    let writes = app.world().resource::<Writes>().0;
    frames(&mut app, 30);
    assert_eq!(
        app.world().resource::<Writes>().0,
        writes,
        "a held lift writes nothing"
    );
    app.world_mut().entity_mut(panel).remove::<Lift>();
    frames(&mut app, SETTLED);
    assert!((drawn_scale(&app, panel) - 1.0).abs() < 1e-4, "put down");
}

#[test]
fn a_lift_lands_at_once_under_reduced_motion() {
    for settings in [MotionSettings::off(), MotionSettings::reduced()] {
        let mut app = app(settings);
        let panel = readout(&mut app);
        app.world_mut().entity_mut(panel).insert(Lift::default());
        frames(&mut app, 2);
        assert!((drawn_scale(&app, panel) - (1.0 + LIFT_GAIN)).abs() < 1e-3);
    }
}
