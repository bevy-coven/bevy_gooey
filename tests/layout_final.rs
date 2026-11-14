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

// ---- 3 and 2. RowLayout and LayoutGap on container roles and on placed nodes ------------------

/// A `RowLayout` turns a panel, a surface, a card, a tone box, a menu, a section body, a sized box and
/// a screen into the flow the host asks for.
#[test]
fn a_row_layout_applies_to_the_container_roles() {
    let mut app = headless();
    let roles: Vec<(&str, Entity)> = vec![
        ("panel", app.world_mut().spawn(panel()).id()),
        ("surface", app.world_mut().spawn(surface(1)).id()),
        (
            "card",
            app.world_mut().spawn((Node::default(), Role::Card)).id(),
        ),
        (
            "tone",
            app.world_mut()
                .spawn((Node::default(), Role::Tone("x")))
                .id(),
        ),
        ("menu", app.world_mut().spawn(menu()).id()),
        (
            "section body",
            app.world_mut()
                .spawn((Node::default(), Role::SectionBody))
                .id(),
        ),
        (
            "sized",
            app.world_mut().spawn(sized(Sizing::default())).id(),
        ),
        ("screen", app.world_mut().spawn(screen()).id()),
    ];
    for (_, e) in &roles {
        app.world_mut().entity_mut(*e).insert(RowLayout {
            direction: Some(FlexDirection::Row),
            align: Some(AlignItems::Center),
            justify: Some(JustifyContent::End),
            wrap: Some(FlexWrap::Wrap),
            shrink: Some(0.0),
        });
    }
    settle(&mut app);
    for (name, e) in &roles {
        let node = app.world().get::<Node>(*e).expect("a node");
        assert_eq!(node.flex_direction, FlexDirection::Row, "{name}");
        assert_eq!(node.align_items, AlignItems::Center, "{name}");
        assert_eq!(node.justify_content, JustifyContent::End, "{name}");
        assert_eq!(node.flex_wrap, FlexWrap::Wrap, "{name}");
        assert_eq!(node.flex_shrink, 0.0, "{name}");
    }
}

/// A panel that is a row with children: they stand in a line, a gap of the host's own apart, and
/// the panel keeps its own frame and padding.
#[test]
fn a_panel_can_be_a_row_with_its_own_gap() {
    let mut app = headless();
    let (a, b) = (probe(&mut app, 30.0, 20.0), probe(&mut app, 30.0, 20.0));
    let card = app
        .world_mut()
        .spawn((
            panel(),
            LayoutGap(2.0),
            RowLayout {
                direction: Some(FlexDirection::Row),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn(a);
            p.spawn(b);
        })
        .id();
    room(&mut app, 400.0, card);
    settle(&mut app);
    let kids = children(&app, card);
    let (first, second) = (rect(&app, kids[0]), rect(&app, kids[1]));
    assert!((first.min.y - second.min.y).abs() < 0.5, "a line");
    assert!(
        ((second.min.x - first.max.x) - 2.0 * GooeySkin::dark().gap).abs() < 0.5,
        "two skin gaps: {first:?} {second:?}"
    );
    assert!(
        app.world().get::<Node>(card).expect("a node").padding != UiRect::ZERO,
        "the panel keeps its own padding"
    );
}

/// Without `LayoutGap` a panel keeps the gap it has; with it, the gap is the host's multiple.
#[test]
fn a_layout_gap_on_a_panel_replaces_its_own_row_gap() {
    let mut app = headless();
    let plain = app.world_mut().spawn(panel()).id();
    let gapped = app.world_mut().spawn((panel(), LayoutGap(0.5))).id();
    for e in [plain, gapped] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    let skin = GooeySkin::dark();
    let node = |e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node(plain).row_gap, Val::Px(skin.gap), "the panel's own");
    assert_eq!(node(gapped).row_gap, Val::Px(0.5 * skin.gap));
    assert_eq!(node(gapped).column_gap, Val::Px(0.5 * skin.gap));
}

/// A placed node that is also a flow: `LayoutGap` and `RowLayout` are honoured beside its `PlacedAt`,
/// which keeps owning its place.
#[test]
fn a_placed_node_can_be_a_flow_with_a_gap_beside_its_place() {
    let mut app = headless();
    let (a, b) = (probe(&mut app, 30.0, 20.0), probe(&mut app, 30.0, 20.0));
    let flow = probe(&mut app, 100.0, 50.0);
    let cluster = app
        .world_mut()
        .spawn((
            placed_at(40.0, 30.0),
            LayoutGap(1.5),
            RowLayout {
                direction: Some(FlexDirection::Row),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn(a);
            p.spawn(b);
        })
        .id();
    let host = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(400.0),
            height: Val::Px(300.0),
            ..default()
        })
        .with_children(|h| {
            h.spawn(flow);
        })
        .add_child(cluster)
        .id();
    app.world_mut().spawn(screen()).add_child(host);
    settle(&mut app);
    let (h, c) = (rect(&app, host), rect(&app, cluster));
    assert!(
        (c.min - (h.min + Vec2::new(40.0, 30.0))).length() < 0.5,
        "its place: {c:?} in {h:?}"
    );
    let kids = children(&app, cluster);
    let (first, second) = (rect(&app, kids[0]), rect(&app, kids[1]));
    assert!(
        ((second.min.x - first.max.x) - 1.5 * GooeySkin::dark().gap).abs() < 0.5,
        "{first:?} {second:?}"
    );
    assert!((first.min.y - second.min.y).abs() < 0.5, "a row");
}

/// A changed `LayoutGap` and a changed `RowLayout` on a panel restyle it in the same frame.
#[test]
fn a_changed_layout_gap_and_row_layout_restyle_a_panel() {
    let mut app = headless();
    let e = app
        .world_mut()
        .spawn((
            panel(),
            LayoutGap(1.0),
            RowLayout {
                direction: Some(FlexDirection::Column),
                ..default()
            },
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(e);
    settle(&mut app);
    app.world_mut().get_mut::<LayoutGap>(e).expect("a gap").0 = 3.0;
    app.world_mut()
        .get_mut::<RowLayout>(e)
        .expect("a layout")
        .direction = Some(FlexDirection::Row);
    settle(&mut app);
    let node = app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node.row_gap, Val::Px(3.0 * GooeySkin::dark().gap));
    assert_eq!(node.flex_direction, FlexDirection::Row);
}
