//! The interface keeps its share of the window: a larger window draws every widget larger, in
//! proportion, from the window's shorter side. Laid out for real in a headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// A headless app with a `width` × `height` window, so UI layout runs.
fn app(width: u32, height: u32, plugins: impl PluginGroup) -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(width, height)
                        .with_scale_factor_override(1.0),
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
        plugins,
    ));
    app.world_mut().spawn(Camera2d);
    app
}

/// A panel with a menu of one button; returns the button, laid out.
fn one_button(app: &mut App) -> Entity {
    let world = app.world_mut();
    let play = world.spawn(button("Play")).id();
    let column = world.spawn(menu()).add_child(play).id();
    let panel = world.spawn(panel()).add_child(column).id();
    world.spawn(screen()).add_child(panel);
    for _ in 0..3 {
        app.update();
    }
    play
}

/// The widget's height on screen, in physical pixels.
fn drawn_height(app: &App, widget: Entity) -> f32 {
    app.world()
        .get::<ComputedNode>(widget)
        .expect("a laid-out widget")
        .size()
        .y
}

/// The button's height in a 1280x720 window, where the interface is drawn at the skin's sizes.
fn height_at_720() -> f32 {
    let mut app = app(1280, 720, GooeyPlugins);
    let play = one_button(&mut app);
    assert_eq!(ui_scale(&app), 1.0);
    drawn_height(&app, play)
}

/// `drawn` is `scale` times `at_720`, within the pixel each layout may round away.
fn assert_scaled(drawn: f32, at_720: f32, scale: f32) {
    assert!(
        (drawn - scale * at_720).abs() <= scale,
        "{drawn} px is not {scale} x {at_720} px"
    );
}

fn ui_scale(app: &App) -> f32 {
    app.world().resource::<UiScale>().0
}

fn resize(app: &mut App, width: f32, height: f32) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    windows
        .single_mut(app.world_mut())
        .expect("one primary window")
        .resolution
        .set(width, height);
    app.update();
    app.update();
}

#[test]
fn a_1440p_window_draws_the_interface_twice_as_large_as_720p() {
    let at_720 = height_at_720();
    assert!(at_720 >= GooeySkin::default().min_target);

    let mut app = app(2560, 1440, GooeyPlugins);
    let play = one_button(&mut app);
    assert_eq!(ui_scale(&app), 2.0);
    assert_scaled(drawn_height(&app, play), at_720, 2.0);
}

#[test]
fn a_resized_window_is_drawn_at_its_new_scale() {
    let at_720 = height_at_720();
    let mut app = app(1280, 720, GooeyPlugins);
    let play = one_button(&mut app);

    resize(&mut app, 1920.0, 1080.0);
    assert_eq!(ui_scale(&app), 1.5);
    assert_scaled(drawn_height(&app, play), at_720, 1.5);

    // Portrait: the shorter side, now the width, decides.
    resize(&mut app, 1080.0, 1920.0);
    assert_eq!(ui_scale(&app), 1.5);

    // A minimised window has no size to scale by: the last scale stands.
    resize(&mut app, 0.0, 0.0);
    assert_eq!(ui_scale(&app), 1.5);
}

#[test]
fn the_players_interface_size_multiplies_the_windows() {
    let at_720 = height_at_720();
    let mut app = app(1920, 1080, GooeyPlugins);
    app.insert_resource(UiScaleSettings {
        scale: 1.25,
        ..default()
    });
    let play = one_button(&mut app);
    assert_eq!(ui_scale(&app), 1.875);
    assert_scaled(drawn_height(&app, play), at_720, 1.875);

    // A game designed at 1080p draws its own sizes there.
    app.insert_resource(UiScaleSettings {
        reference_short_side: 1080.0,
        ..default()
    });
    app.update();
    assert_eq!(ui_scale(&app), 1.0);

    // An invalid setting leaves the scale as it was.
    app.insert_resource(UiScaleSettings {
        reference_short_side: 0.0,
        ..default()
    });
    app.update();
    assert_eq!(ui_scale(&app), 1.0);
}

#[test]
fn a_host_that_drives_ui_scale_itself_leaves_the_plugin_out() {
    let at_720 = height_at_720();
    let mut app = app(
        2560,
        1440,
        GooeyPlugins.build().disable::<GooeyScalePlugin>(),
    );
    app.insert_resource(UiScale(1.25));
    let play = one_button(&mut app);
    assert_eq!(ui_scale(&app), 1.25);
    assert_scaled(drawn_height(&app, play), at_720, 1.25);
}
