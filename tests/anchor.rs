//! A `WorldAnchor`: an interface node that follows a point in the world. Laid out for real in a
//! headless window, read back from the node's drawn position (`UiGlobalTransform`, physical px).

use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedNode, IsDefaultUiCamera, UiGlobalTransform};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// A headless app whose window is 1280 × 720 logical px at `scale_factor`, with no camera yet.
fn app(scale_factor: f32) -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(
                        (1280.0 * scale_factor) as u32,
                        (720.0 * scale_factor) as u32,
                    )
                    .with_scale_factor_override(scale_factor),
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
    app
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

/// A 2D game's world: a camera at the origin and a target at `at`.
fn world(app: &mut App, at: Vec3) -> Entity {
    app.world_mut().spawn(Camera2d);
    app.world_mut().spawn(Transform::from_translation(at)).id()
}

/// The bottom middle of the node as drawn, in logical window px.
fn foot(app: &App, node: Entity, scale_factor: f32) -> Vec2 {
    let at = app.world().get::<UiGlobalTransform>(node).unwrap();
    let size = app.world().get::<ComputedNode>(node).unwrap().size();
    (at.translation + Vec2::new(0.0, size.y / 2.0)) / scale_factor
}

/// The node's box in logical window px, `(min, max)`.
fn rect(app: &App, node: Entity, scale_factor: f32) -> (Vec2, Vec2) {
    let at = app.world().get::<UiGlobalTransform>(node).unwrap();
    let size = app.world().get::<ComputedNode>(node).unwrap().size();
    (
        (at.translation - size / 2.0) / scale_factor,
        (at.translation + size / 2.0) / scale_factor,
    )
}

fn hidden(app: &App, node: Entity) -> bool {
    *app.world().get::<Visibility>(node).unwrap() == Visibility::Hidden
}

fn move_target(app: &mut App, target: Entity, at: Vec3) {
    app.world_mut()
        .get_mut::<Transform>(target)
        .unwrap()
        .translation = at;
}

fn near(a: Vec2, b: Vec2) {
    assert!((a - b).abs().max_element() < 1.0, "{a} is not {b}");
}

#[test]
fn a_bar_stands_on_its_thing_in_the_frame_it_spawns_and_every_frame_it_moves() {
    let mut app = app(1.0);
    let target = world(&mut app, Vec3::new(100.0, 50.0, 0.0));
    let bar = app
        .world_mut()
        .spawn((meter(0.8, 1.0), WorldAnchor::above(target, 20.0)))
        .id();
    settle(&mut app);
    // World (100, 50 + 20) in a 1280 x 720 window with the camera at the origin: y is up.
    near(foot(&app, bar, 1.0), Vec2::new(740.0, 290.0));
    assert!(!hidden(&app, bar));
    // One update after the thing moves, not two: the bar is never a frame behind.
    for step in 1..=5 {
        let x = 100.0 + step as f32 * 40.0;
        move_target(
            &mut app,
            target,
            Vec3::new(x, 50.0 - step as f32 * 10.0, 0.0),
        );
        app.update();
        near(
            foot(&app, bar, 1.0),
            Vec2::new(640.0 + x, 360.0 - (50.0 - step as f32 * 10.0 + 20.0)),
        );
    }
}

#[test]
fn it_holds_at_a_window_scale_factor_of_two_and_a_player_interface_size() {
    let mut app = app(2.0);
    app.world_mut().resource_mut::<UiScaleSettings>().scale = 1.25;
    let target = world(&mut app, Vec3::new(100.0, 50.0, 0.0));
    let bar = app
        .world_mut()
        .spawn((meter(0.8, 1.0), WorldAnchor::above(target, 20.0)))
        .id();
    settle(&mut app);
    near(foot(&app, bar, 2.0), Vec2::new(740.0, 290.0));
}

#[test]
fn the_pivot_is_the_point_of_the_node_that_sits_on_the_world_point() {
    let mut app = app(1.0);
    let target = world(&mut app, Vec3::new(-200.0, 0.0, 0.0));
    let tag = app
        .world_mut()
        .spawn((
            label("Door"),
            WorldAnchor::new(target).pivot(Vec2::new(0.0, 0.5)),
        ))
        .id();
    settle(&mut app);
    let (min, max) = rect(&app, tag, 1.0);
    // Its left middle is on (440, 360).
    near(
        Vec2::new(min.x, (min.y + max.y) / 2.0),
        Vec2::new(440.0, 360.0),
    );
}

#[test]
fn hiding_follows_the_point_out_of_view_and_back() {
    let mut app = app(1.0);
    let target = world(&mut app, Vec3::ZERO);
    let bar = app
        .world_mut()
        .spawn((meter(0.5, 1.0), WorldAnchor::above(target, 10.0)))
        .id();
    settle(&mut app);
    assert!(!hidden(&app, bar));
    move_target(&mut app, target, Vec3::new(5000.0, 0.0, 0.0));
    app.update();
    assert!(hidden(&app, bar), "off the right edge");
    move_target(&mut app, target, Vec3::new(0.0, -5000.0, 0.0));
    app.update();
    assert!(hidden(&app, bar), "off the bottom");
    move_target(&mut app, target, Vec3::new(300.0, 100.0, 0.0));
    app.update();
    assert!(!hidden(&app, bar), "back in view");
    near(foot(&app, bar, 1.0), Vec2::new(940.0, 250.0));
    // A bar whose foot is just past the edge but whose body is not still draws.
    move_target(&mut app, target, Vec3::new(0.0, -362.0, 0.0));
    app.update();
    assert!(!hidden(&app, bar), "a node half in view is drawn");
}

#[test]
fn pinning_keeps_a_node_whole_inside_the_safe_edge_toward_its_point() {
    let mut app = app(1.0);
    let target = world(&mut app, Vec3::ZERO);
    let beacon = app
        .world_mut()
        .spawn((label("Camp"), WorldAnchor::new(target).pinned()))
        .id();
    settle(&mut app);
    assert!(app.world().get::<AnchorPinned>(beacon).is_none(), "in view");
    near(foot(&app, beacon, 1.0), Vec2::new(640.0, 360.0));

    // Far up and to the right.
    move_target(&mut app, target, Vec3::new(4000.0, 3000.0, 0.0));
    settle(&mut app);
    let (min, max) = rect(&app, beacon, 1.0);
    assert!(min.x >= 64.0 - 1.0 && max.x <= 1216.0 + 1.0, "{min} {max}");
    assert!(min.y >= 36.0 - 1.0 && max.y <= 684.0 + 1.0, "{min} {max}");
    let pinned = *app.world().get::<AnchorPinned>(beacon).unwrap();
    let want = Vec2::new(4000.0, -3000.0).normalize();
    assert!(
        (pinned.direction - want).length() < 1e-3,
        "{:?}",
        pinned.direction
    );
    assert!(!hidden(&app, beacon));
    // Touching the right edge's inset: the box's right side is on it.
    assert!((max.x - 1216.0).abs() < 1.5 || (min.y - 36.0).abs() < 1.5);

    // Straight left.
    move_target(&mut app, target, Vec3::new(-9000.0, 0.0, 0.0));
    settle(&mut app);
    let pinned = *app.world().get::<AnchorPinned>(beacon).unwrap();
    assert!((pinned.direction - Vec2::NEG_X).length() < 1e-3);

    // Back in view: the marker goes.
    move_target(&mut app, target, Vec3::new(10.0, 10.0, 0.0));
    settle(&mut app);
    assert!(app.world().get::<AnchorPinned>(beacon).is_none());
    assert!(!hidden(&app, beacon));
}

#[test]
fn a_point_behind_a_perspective_camera_is_hidden_or_pinned_the_way_it_lies() {
    let mut app = app(1.0);
    app.world_mut()
        .spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 0.0)));
    // The camera looks down -Z: this is behind it and to its right.
    let behind = app
        .world_mut()
        .spawn(Transform::from_xyz(5.0, 0.0, 10.0))
        .id();
    let nameplate = app
        .world_mut()
        .spawn((label("Hidden"), WorldAnchor::new(behind)))
        .id();
    let beacon = app
        .world_mut()
        .spawn((label("Pinned"), WorldAnchor::new(behind).pinned()))
        .id();
    settle(&mut app);
    assert!(hidden(&app, nameplate));
    let pinned = *app.world().get::<AnchorPinned>(beacon).unwrap();
    assert!(
        pinned.direction.x > 0.9,
        "it lies to the right: {:?}",
        pinned.direction
    );
    let (min, _) = rect(&app, beacon, 1.0);
    assert!(min.x > 640.0, "on the right half: {min}");
    // In front of the camera it is an ordinary point.
    move_target(&mut app, behind, Vec3::new(0.0, 0.0, -10.0));
    settle(&mut app);
    assert!(!hidden(&app, nameplate));
    assert!(app.world().get::<AnchorPinned>(beacon).is_none());
    near(foot(&app, nameplate, 1.0), Vec2::new(640.0, 360.0));
}

#[test]
fn despawning_the_target_despawns_its_nodes_at_once() {
    let mut app = app(1.0);
    let target = world(&mut app, Vec3::ZERO);
    let a = app
        .world_mut()
        .spawn((meter(0.5, 1.0), WorldAnchor::above(target, 10.0)))
        .id();
    let b = app
        .world_mut()
        .spawn((label("Name"), WorldAnchor::new(target)))
        .id();
    let other = app.world_mut().spawn(Transform::default()).id();
    let kept = app
        .world_mut()
        .spawn((label("Other"), WorldAnchor::new(other)))
        .id();
    settle(&mut app);
    assert_eq!(app.world().get::<WorldAnchors>(target).unwrap().len(), 2);
    app.world_mut().entity_mut(target).despawn();
    assert!(app.world().get_entity(a).is_err());
    assert!(app.world().get_entity(b).is_err());
    assert!(
        app.world().get_entity(kept).is_ok(),
        "another thing's node stays"
    );
}

#[test]
fn a_nameplate_lets_the_pointer_through_unless_the_game_says_otherwise() {
    let mut app = app(1.0);
    let target = world(&mut app, Vec3::ZERO);
    let plate = app
        .world_mut()
        .spawn((meter(0.5, 1.0), WorldAnchor::above(target, 10.0)))
        .id();
    let clickable = app
        .world_mut()
        .spawn((
            label("Press E"),
            WorldAnchor::new(target),
            Pickable::default(),
        ))
        .id();
    settle(&mut app);
    assert_eq!(
        app.world().get::<Pickable>(plate).copied(),
        Some(Pickable::IGNORE)
    );
    assert_eq!(
        app.world().get::<Pickable>(clickable).copied(),
        Some(Pickable::default())
    );
}

#[test]
fn a_named_world_camera_places_the_node_not_the_ui_camera() {
    let mut app = app(1.0);
    app.world_mut().spawn((Camera2d, IsDefaultUiCamera));
    // The world camera is 200 units to the right of the UI camera.
    let world_camera = app
        .world_mut()
        .spawn((
            Camera2d,
            Camera {
                order: 1,
                ..default()
            },
            Transform::from_xyz(200.0, 0.0, 0.0),
        ))
        .id();
    let target = app
        .world_mut()
        .spawn(Transform::from_xyz(300.0, 0.0, 0.0))
        .id();
    let by_ui = app
        .world_mut()
        .spawn((label("ui"), WorldAnchor::new(target)))
        .id();
    let by_world = app
        .world_mut()
        .spawn((
            label("world"),
            WorldAnchor::new(target).camera(world_camera),
        ))
        .id();
    settle(&mut app);
    near(foot(&app, by_ui, 1.0).with_y(0.0), Vec2::new(940.0, 0.0));
    near(foot(&app, by_world, 1.0).with_y(0.0), Vec2::new(740.0, 0.0));
}

#[test]
fn a_settled_anchor_writes_nothing() {
    let mut app = app(1.0);
    let target = world(&mut app, Vec3::new(10.0, 10.0, 0.0));
    let beacon = app
        .world_mut()
        .spawn((meter(0.5, 1.0), WorldAnchor::above(target, 10.0)))
        .id();
    settle(&mut app);
    let ticks = |app: &App| {
        let e = app.world().entity(beacon);
        (
            e.get_ref::<Node>().unwrap().last_changed(),
            e.get_ref::<UiTransform>().unwrap().last_changed(),
            e.get_ref::<Visibility>().unwrap().last_changed(),
        )
    };
    let before = ticks(&app);
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(ticks(&app), before);
}
