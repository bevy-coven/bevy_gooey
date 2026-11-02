//! Progress and spinner: honest feedback for work that takes a while. Laid out for real in a
//! headless window, on a clock the test sets.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::model::{SPINNER_PERIOD_SECS, contrast_ratio, spinner_angle};
use bevy_gooey::*;

/// A headless app with a real window size and a clock that moves 0.1 s a frame.
fn app(skin: GooeySkin) -> App {
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
    .insert_resource(skin)
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

fn in_panel(app: &mut App, bundle: impl Bundle) -> Entity {
    let id = app.world_mut().spawn(bundle).id();
    let panel = app.world_mut().spawn(panel()).add_child(id).id();
    app.world_mut().spawn(screen()).add_child(panel);
    settle(app);
    id
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

fn part(app: &App, root: Entity, role: Role) -> Entity {
    descendants(app.world(), root)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&role))
        .unwrap_or_else(|| panic!("a {role:?} under the widget"))
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

fn fill_share(app: &App, bar: Entity) -> f32 {
    let track = rect(app, part(app, bar, Role::Meter));
    let fill = rect(app, part(app, bar, Role::MeterFill));
    // The fill sits one line inside the frame and the gap: its share of the room inside.
    let inset = 2.0 * GooeySkin::default().border;
    fill.width() / (track.width() - 2.0 * inset)
}

fn line(app: &App, bar: Entity) -> (Display, String) {
    let line = part(app, bar, Role::ProgressLine);
    (
        app.world().get::<Node>(line).expect("a node").display,
        app.world().get::<Text>(line).expect("words").0.clone(),
    )
}

#[test]
fn a_progress_at_half_draws_half_its_track_and_follows_its_value_the_same_frame() {
    let mut app = app(GooeySkin::dark());
    let bar = in_panel(&mut app, progress(0.5));
    assert!(
        (fill_share(&app, bar) - 0.5).abs() < 0.02,
        "{}",
        fill_share(&app, bar)
    );
    for want in [0.25, 1.0, 0.0] {
        app.world_mut().entity_mut(bar).insert(Progress(want));
        app.update();
        app.update();
        assert!(
            (fill_share(&app, bar) - want).abs() < 0.02,
            "{want}: {}",
            fill_share(&app, bar)
        );
    }
}

#[test]
fn a_value_outside_the_range_is_drawn_at_the_nearest_end_and_marked_until_it_is_back() {
    let mut app = app(GooeySkin::dark());
    let bar = in_panel(&mut app, progress(0.5));
    assert!(app.world().get::<ProgressClamped>(bar).is_none());
    app.world_mut().entity_mut(bar).insert(Progress(1.5));
    settle(&mut app);
    assert!((fill_share(&app, bar) - 1.0).abs() < 0.02);
    assert!(app.world().get::<ProgressClamped>(bar).is_some());
    app.world_mut().entity_mut(bar).insert(Progress(2.0));
    settle(&mut app);
    assert!(app.world().get::<ProgressClamped>(bar).is_some());
    app.world_mut().entity_mut(bar).insert(Progress(f32::NAN));
    settle(&mut app);
    assert!(fill_share(&app, bar).abs() < 0.02);
    assert!(app.world().get::<ProgressClamped>(bar).is_some());
    app.world_mut().entity_mut(bar).insert(Progress(-1.0));
    settle(&mut app);
    assert!(fill_share(&app, bar).abs() < 0.02);
    app.world_mut().entity_mut(bar).insert(Progress(0.4));
    settle(&mut app);
    assert!(app.world().get::<ProgressClamped>(bar).is_none());
}

#[test]
fn the_words_are_the_step_and_the_time_left_and_there_is_no_line_without_either() {
    let mut app = app(GooeySkin::dark());
    let bar = in_panel(&mut app, progress(3.0 / 7.0));
    assert_eq!(line(&app, bar).0, Display::None);
    app.world_mut().entity_mut(bar).insert(ProgressText {
        step: Some((3, 7, "Measuring meshes".into())),
        eta_seconds: Some(20.0),
    });
    settle(&mut app);
    assert_eq!(
        line(&app, bar),
        (
            Display::Flex,
            "3 of 7: Measuring meshes, about 20 s left".to_string()
        )
    );
    app.world_mut().entity_mut(bar).insert(ProgressText {
        step: None,
        eta_seconds: Some(4.0),
    });
    settle(&mut app);
    assert_eq!(
        line(&app, bar),
        (Display::Flex, "about 4 s left".to_string())
    );
    app.world_mut()
        .entity_mut(bar)
        .insert(ProgressText::default());
    settle(&mut app);
    assert_eq!(line(&app, bar).0, Display::None);
}

#[test]
fn a_progress_keeps_its_width_as_the_words_change_and_its_words_fit_inside_it() {
    let mut app = app(GooeySkin::dark());
    let bar = in_panel(
        &mut app,
        (
            progress(0.5),
            ProgressText {
                step: Some((3, 7, "Measuring meshes".into())),
                eta_seconds: Some(20.0),
            },
        ),
    );
    let before = rect(&app, bar);
    let words = rect(&app, part(&app, bar, Role::ProgressLine));
    assert!(words.max.x <= before.max.x + 0.5, "{words:?} in {before:?}");
    app.world_mut().entity_mut(bar).insert(ProgressText {
        step: Some((4, 7, "Baking".into())),
        eta_seconds: Some(3.0),
    });
    settle(&mut app);
    assert!(
        (rect(&app, bar).width() - before.width()).abs() < 0.5,
        "{before:?} {:?} {words:?}",
        rect(&app, bar)
    );
}

#[test]
fn the_words_read_at_text_contrast_in_every_look() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app(skin.clone());
        let bar = in_panel(
            &mut app,
            (
                progress(0.5),
                ProgressText {
                    step: None,
                    eta_seconds: Some(9.0),
                },
            ),
        );
        let ink = Srgba::from(
            app.world()
                .get::<TextColor>(part(&app, bar, Role::ProgressLine))
                .expect("an ink")
                .0,
        );
        let panel = Srgba::from(skin.solid_surface());
        let ratio = contrast_ratio(
            [ink.red, ink.green, ink.blue],
            [panel.red, panel.green, panel.blue],
        );
        assert!(ratio >= 4.5, "{}: {ratio}", preset.name());
    }
}

fn turn(app: &App, spinner: Entity) -> f32 {
    app.world()
        .get::<UiTransform>(spinner)
        .expect("a transform")
        .rotation
        .as_radians()
}

#[test]
fn a_spinner_advances_every_frame_on_the_real_clock_and_wraps() {
    let mut app = app(GooeySkin::dark());
    let spinner = in_panel(&mut app, spinner());
    let mut last = turn(&app, spinner);
    let mut moved = 0;
    for _ in 0..30 {
        app.update();
        let now = turn(&app, spinner);
        if (now - last).abs() > 1e-4 {
            moved += 1;
        }
        last = now;
    }
    assert_eq!(moved, 30, "it moves every frame");
    // At a fixed step the angle follows the clock: the real time elapsed, in turns of a period.
    let elapsed = app.world().resource::<Time<Real>>().elapsed_secs();
    let want = spinner_angle(elapsed);
    let got = turn(&app, spinner).rem_euclid(std::f32::consts::TAU);
    assert!(
        (got - want).abs() < 1e-3 || (got - want).abs() > std::f32::consts::TAU - 1e-3,
        "{got} {want}"
    );
    assert!(elapsed > SPINNER_PERIOD_SECS, "the run covered a full turn");
}

#[test]
fn a_spinner_is_a_ring_of_the_outline_with_one_quarter_in_the_accent_one_and_a_half_text_sizes_square()
 {
    let skin = GooeySkin::dark();
    let mut app = app(skin.clone());
    let spinner = in_panel(&mut app, spinner());
    let edge = app.world().get::<BorderColor>(spinner).expect("a ring");
    assert_eq!(edge.top, skin.accent);
    assert_eq!(
        (edge.right, edge.bottom, edge.left),
        (skin.outline, skin.outline, skin.outline)
    );
    let size = rect(&app, spinner).size();
    assert!(
        (size - Vec2::splat(skin.text_size * 1.5))
            .abs()
            .max_element()
            < 1.0,
        "{size:?}"
    );
}
