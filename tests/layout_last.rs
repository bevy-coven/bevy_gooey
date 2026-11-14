//! What was left of the editor's hand-built nodes after G-85 (G-86, #97): off-scale spacing, extents
//! in viewport units, a border in logical px, a button box, a hideable quiet status item, an ink
//! override and the cost of per-frame geometry, each laid out for real in a headless window.

use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

fn headless() -> App {
    let mut app = App::new();
    app.insert_resource(MotionSettings::off());
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

// ---- 2. spacing off the skin's scale: Space::Px ----------------------------------------------

fn pad(px: f32) -> Spacing {
    Spacing {
        padding: Sides::all(Space::Px(px)),
        ..default()
    }
}

/// A literal logical px is that many px, not a step: a chip's 6 by 3 and a tab's 12 by 8.
#[test]
fn a_space_in_px_is_that_many_px_not_a_step() {
    let mut app = headless();
    let b = probe(&mut app, 40.0, 20.0);
    let chip = app
        .world_mut()
        .spawn((
            row(0.0),
            Spacing {
                padding: Sides {
                    left: Some(Space::Px(6.0)),
                    right: Some(Space::Px(6.0)),
                    top: Some(Space::Px(3.0)),
                    bottom: Some(Space::Px(3.0)),
                },
                margin: Sides {
                    left: Some(Space::Px(8.0)),
                    ..default()
                },
            },
        ))
        .with_children(|r| {
            r.spawn(b);
        })
        .id();
    let host = room(&mut app, 400.0, chip);
    settle(&mut app);
    let (c, h, kid) = (
        rect(&app, chip),
        rect(&app, host),
        rect(&app, children(&app, chip)[0]),
    );
    assert!(
        (c.min.x - h.min.x - 8.0).abs() < 0.5,
        "margin 8: {c:?} in {h:?}"
    );
    assert!(
        (kid.min.x - c.min.x - 6.0).abs() < 0.5 && (kid.min.y - c.min.y - 3.0).abs() < 0.5,
        "padding 6 by 3: {kid:?} in {c:?}"
    );
    assert!((c.height() - 26.0).abs() < 0.5, "{c:?}");
}

/// It is off the scale: changing the skin's spacing moves a step and leaves a px alone.
#[test]
fn a_px_space_does_not_follow_the_skin_scale() {
    let mut app = headless();
    let stepped = app
        .world_mut()
        .spawn((
            row(0.0),
            Spacing {
                padding: Sides::all(Space::Step(1)),
                ..default()
            },
        ))
        .id();
    let literal = app.world_mut().spawn((row(0.0), pad(6.0))).id();
    for e in [stepped, literal] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    app.world_mut().resource_mut::<GooeySkin>().spacing = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    settle(&mut app);
    let node = |e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node(stepped).padding, UiRect::all(Val::Px(2.0)));
    assert_eq!(node(literal).padding, UiRect::all(Val::Px(6.0)));
}

/// A px below zero is none, and one that is not a number leaves its side alone.
#[test]
fn a_px_space_is_finite_checked_and_never_negative() {
    let mut app = headless();
    let plain = app.world_mut().spawn(panel()).id();
    let odd = app
        .world_mut()
        .spawn((
            panel(),
            Spacing {
                padding: Sides {
                    left: Some(Space::Px(-4.0)),
                    right: Some(Space::Px(f32::NAN)),
                    top: Some(Space::Px(f32::INFINITY)),
                    ..default()
                },
                ..default()
            },
        ))
        .id();
    for e in [plain, odd] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    let node = |e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node(odd).padding.left, Val::Px(0.0));
    assert_eq!(
        node(odd).padding.right,
        node(plain).padding.right,
        "NaN leaves the side"
    );
    assert_eq!(
        node(odd).padding.top,
        node(plain).padding.top,
        "infinity leaves the side"
    );
}

/// A changed px restyles.
#[test]
fn a_changed_px_space_restyles() {
    let mut app = headless();
    let e = app.world_mut().spawn((row(0.0), pad(6.0))).id();
    app.world_mut().spawn(screen()).add_child(e);
    settle(&mut app);
    app.world_mut()
        .get_mut::<Spacing>(e)
        .expect("spacing")
        .padding = Sides::all(Space::Px(9.0));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(e).expect("a node").padding,
        UiRect::all(Val::Px(9.0))
    );
}
