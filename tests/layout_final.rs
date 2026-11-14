//! What was left of the editor's hand-built nodes after G-86 (G-87, #98): a host-painted colour, flow
//! on container roles and placed nodes, a blocking scrim and the frame, each laid out for real in a
//! headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

fn headless() -> App {
    headless_at(1.0)
}

/// The same at a window scale factor, so a device pixel is not a logical one.
fn headless_at(scale: f32) -> App {
    let mut app = App::new();
    app.insert_resource(MotionSettings::off());
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(
                        (1280.0 * scale) as u32,
                        (720.0 * scale) as u32,
                    )
                    .with_scale_factor_override(scale),
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
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// A node's rectangle in the window, logical px.
fn rect(app: &App, entity: Entity) -> Rect {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    let scale = node.inverse_scale_factor();
    Rect::from_center_size(at * scale, node.size() * scale)
}

fn children(app: &App, entity: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

/// A real image in the app's assets, so a box draws something.
fn image(app: &mut App) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default())
}

/// A fixed box `w` × `h` logical px that never shrinks: a probe for where a row puts its children.
fn probe(app: &mut App, w: f32, h: f32) -> impl Bundle + use<> {
    image_box(image(app), Vec2::new(w, h))
}

/// A plain node `width` wide that holds `child` and never gives way: the room a layout is put in.
fn room(app: &mut App, width: f32, child: Entity) -> Entity {
    let room = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(width),
            min_width: Val::Px(width),
            max_width: Val::Px(width),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        })
        .add_child(child)
        .id();
    app.world_mut().spawn(screen()).add_child(room);
    room
}

// ---- 1. a colour the host paints by state: InkFollow -----------------------------------------

const RED: Color = Color::srgb(0.9, 0.1, 0.1);
const GREEN: Color = Color::srgb(0.1, 0.8, 0.2);

/// The colour twin of `HostGeometry`: on a node with `InkFollow` the skin writes neither the fill nor
/// the edge, whatever the state, a skin change or a role; it still writes every other field.
#[test]
fn an_ink_follow_node_keeps_the_hosts_fill_and_edge_and_the_skin_writes_the_rest() {
    let mut app = headless();
    let plain = app.world_mut().spawn(button("A")).id();
    let followed = app
        .world_mut()
        .spawn((
            button("B"),
            InkFollow,
            Shape {
                radius: Some(Radius::Full),
                ..default()
            },
        ))
        .id();
    for e in [plain, followed] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    // The host's own colour system paints it.
    app.world_mut()
        .entity_mut(followed)
        .insert((BackgroundColor(RED), BorderColor::all(GREEN)));
    // A skin change restyles every node; a press changes the role's fill.
    app.world_mut().resource_mut::<GooeySkin>().surface_raised = Color::srgb(0.5, 0.5, 0.5);
    app.world_mut()
        .entity_mut(followed)
        .insert(bevy::ui::Pressed);
    settle(&mut app);
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(followed)
            .expect("a fill")
            .0,
        RED,
        "the host's fill stays"
    );
    assert_eq!(
        app.world()
            .get::<BorderColor>(followed)
            .expect("an edge")
            .top,
        GREEN,
        "and its edge"
    );
    assert_ne!(
        app.world().get::<BackgroundColor>(plain).expect("a fill").0,
        RED
    );
    assert_eq!(
        app.world()
            .get::<Node>(followed)
            .expect("a node")
            .border_radius,
        BorderRadius::MAX,
        "every other field is the skin's"
    );
}

/// It beats an `InkOverride` and a `Shape` on the same node (the host paints), and removing it gives
/// the skin the colours back in the same frame, adding it again leaves them where they are.
#[test]
fn ink_follow_beats_an_override_and_removing_it_restyles() {
    let mut app = headless();
    let b = app
        .world_mut()
        .spawn((
            button("B"),
            InkFollow,
            InkOverride {
                fill: Some(GREEN),
                edge: Some(GREEN),
                ..default()
            },
            Shape {
                fill: Some(2),
                ink: Some(Ink::Danger),
                border: Sides::all(1.0),
                ..default()
            },
        ))
        .id();
    let plain = app.world_mut().spawn(button("A")).id();
    for e in [b, plain] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    app.world_mut()
        .entity_mut(b)
        .insert((BackgroundColor(RED), BorderColor::all(RED)));
    settle(&mut app);
    assert_eq!(
        app.world().get::<BackgroundColor>(b).expect("a fill").0,
        RED
    );
    assert_eq!(app.world().get::<BorderColor>(b).expect("an edge").top, RED);
    // Removing it: the override is the skin's again, in the same frames.
    app.world_mut().entity_mut(b).remove::<InkFollow>();
    settle(&mut app);
    assert_eq!(
        app.world().get::<BackgroundColor>(b).expect("a fill").0,
        GREEN,
        "the skin repaints"
    );
    assert_eq!(
        app.world().get::<BorderColor>(b).expect("an edge").top,
        GREEN
    );
    // Adding it back: the colours stay as the skin left them until the host paints.
    app.world_mut().entity_mut(b).insert(InkFollow);
    app.world_mut()
        .get_mut::<InkOverride>(b)
        .expect("an override")
        .fill = Some(RED);
    settle(&mut app);
    assert_eq!(
        app.world().get::<BackgroundColor>(b).expect("a fill").0,
        GREEN,
        "the skin no longer writes it"
    );
}
