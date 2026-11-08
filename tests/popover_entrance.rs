//! A popover opens from its anchor: the frame it is first drawn it is at 0.96 scale and nearly
//! transparent, and by the skin's `snappy` spring it is whole (#71). On a clock the test sets (30 ms
//! a frame). Reduced motion fades in place; no motion draws it whole at once.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 0.03;

fn app(motion: MotionSettings) -> App {
    let mut app = App::new();
    app.insert_resource(motion);
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

/// An anchor and a popover opened on it.
fn popped(app: &mut App, placement: Placement) -> Entity {
    let anchor = app.world_mut().spawn(button("Menu")).id();
    let mut node = app.world_mut().get_mut::<Node>(anchor).expect("a node");
    node.position_type = PositionType::Absolute;
    node.left = Val::Px(600.0);
    node.top = Val::Px(300.0);
    app.world_mut()
        .spawn((
            popover(anchor, placement),
            children![button("Copy"), button("Paste")],
        ))
        .id()
}

fn scale(app: &App, panel: Entity) -> Vec2 {
    app.world().get::<UiTransform>(panel).expect("a node").scale
}

fn opacity(app: &App, panel: Entity) -> f32 {
    app.world()
        .get::<Opacity>(panel)
        .map_or(1.0, Opacity::factor)
}

fn drawn(app: &App, panel: Entity) -> bool {
    *app.world().get::<Visibility>(panel).expect("visibility") != Visibility::Hidden
}

/// The snappy spring's length, in frames.
fn snappy_frames(app: &App) -> usize {
    let seconds = app.world().resource::<GooeySkin>().motion.snappy.duration;
    (seconds / FRAME).ceil() as usize
}

#[test]
fn a_popover_is_first_drawn_small_and_faint_and_whole_by_the_snappy_spring() {
    for placement in [
        Placement::Below,
        Placement::Above,
        Placement::Right,
        Placement::Left,
    ] {
        let mut app = app(MotionSettings::default());
        let panel = popped(&mut app, placement);
        let mut first = None;
        for _ in 0..8 {
            app.update();
            if drawn(&app, panel) && first.is_none() {
                first = Some((scale(&app, panel), opacity(&app, panel)));
            }
        }
        let (scale_then, opacity_then) = first.expect("the panel is drawn");
        assert!(
            (scale_then.x - 0.96).abs() <= 0.01 && (scale_then.y - 0.96).abs() <= 0.01,
            "{placement:?}: first drawn at scale {scale_then:?}"
        );
        assert!(
            opacity_then <= 0.5,
            "{placement:?}: first drawn at opacity {opacity_then}"
        );
        // Whole within the snappy spring, one frame of slack.
        let snappy = snappy_frames(&app);
        let mut app = app;
        for _ in 0..snappy + 1 {
            app.update();
        }
        assert_eq!(scale(&app, panel), Vec2::ONE, "{placement:?}");
        assert!(app.world().get::<PopoverSlide>(panel).is_none());
        assert!(app.world().get::<Opacity>(panel).is_none());
    }
}

#[test]
fn the_scale_grows_with_the_slide_and_never_passes_one() {
    let mut app = app(MotionSettings::default());
    let panel = popped(&mut app, Placement::Below);
    let mut seen = Vec::new();
    for _ in 0..30 {
        app.update();
        if drawn(&app, panel) {
            seen.push(scale(&app, panel).x);
        }
    }
    assert!(seen.len() > 5);
    assert!(seen.windows(2).all(|w| w[1] >= w[0] - 1e-6), "{seen:?}");
    assert!(
        seen.iter().all(|&s| (0.96 - 1e-4..=1.0).contains(&s)),
        "{seen:?}"
    );
    assert_eq!(*seen.last().expect("frames"), 1.0);
}

#[test]
fn reduced_motion_fades_in_place_with_no_scale() {
    let mut app = app(MotionSettings::reduced());
    let panel = popped(&mut app, Placement::Below);
    let mut faded = false;
    for _ in 0..12 {
        app.update();
        assert_eq!(scale(&app, panel), Vec2::ONE);
        faded |= drawn(&app, panel) && opacity(&app, panel) < 1.0;
    }
    assert!(faded, "it fades in");
    assert!(app.world().get::<Opacity>(panel).is_none());
}

#[test]
fn no_motion_draws_it_whole_in_its_first_frame() {
    let mut app = app(MotionSettings::off());
    let panel = popped(&mut app, Placement::Below);
    for _ in 0..3 {
        app.update();
        assert_eq!(scale(&app, panel), Vec2::ONE);
        assert_eq!(opacity(&app, panel), 1.0);
    }
}

#[test]
fn a_panel_with_its_own_opacity_keeps_it_and_still_scales() {
    let mut app = app(MotionSettings::default());
    let anchor = app.world_mut().spawn(button("Menu")).id();
    let panel = app
        .world_mut()
        .spawn((
            popover(anchor, Placement::Below),
            Opacity(0.5),
            children![button("Copy")],
        ))
        .id();
    for _ in 0..40 {
        app.update();
        assert_eq!(
            opacity(&app, panel),
            0.5,
            "the host's opacity is left alone"
        );
    }
    assert_eq!(scale(&app, panel), Vec2::ONE);
}

#[test]
fn a_settled_popover_writes_no_transform() {
    let mut app = app(MotionSettings::default());
    let panel = popped(&mut app, Placement::Below);
    for _ in 0..40 {
        app.update();
    }
    app.world_mut().clear_trackers();
    let mut changed = 0;
    for _ in 0..20 {
        app.update();
        let tick = app.world().read_change_tick();
        let _ = tick;
        let transform = app
            .world()
            .entity(panel)
            .get_ref::<UiTransform>()
            .expect("node");
        if transform.is_changed() {
            changed += 1;
        }
    }
    assert_eq!(changed, 0, "a settled panel's transform is not written");
}
