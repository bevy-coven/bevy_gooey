//! What was left of the editor's hand-built nodes after G-86 (G-87, #98): a host-painted colour, flow
//! on container roles and placed nodes, a blocking scrim and the frame, each laid out for real in a
//! headless window.

use std::collections::HashSet;

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
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

// ---- 4. a layer that blocks the pointer: scrim() ----------------------------------------------

/// Moves the mouse to `at` in window px as the window's own pointer input, and returns every entity
/// it is over.
fn hovered_at(app: &mut App, at: Vec2) -> HashSet<Entity> {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window");
    let location = Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    };
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    for _ in 0..3 {
        app.update();
    }
    app.world()
        .resource::<HoverMap>()
        .get(&PointerId::Mouse)
        .map(|hits| hits.keys().copied().collect())
        .unwrap_or_default()
}

/// A full-window node under the layer: stands in for the map the pointer must not reach.
fn world_below(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(-1),
        ))
        .id()
}

/// A `scrim()` blocks the pointer: over it the world under it is not hovered. A `cover()` lets it
/// through. The two are different on purpose.
#[test]
fn a_scrim_blocks_the_pointer_and_a_cover_does_not() {
    let mut app = headless();
    let world = world_below(&mut app);
    let layer = app.world_mut().spawn((scrim(), GlobalZIndex(5))).id();
    settle(&mut app);
    let at = Vec2::new(40.0, 40.0);
    let over = hovered_at(&mut app, at);
    assert!(over.contains(&layer), "the scrim is hit");
    assert!(
        !over.contains(&world),
        "and the world under it is not: it blocks"
    );

    app.world_mut().entity_mut(layer).despawn();
    let overlay = app.world_mut().spawn((cover(), GlobalZIndex(5))).id();
    settle(&mut app);
    let over = hovered_at(&mut app, Vec2::new(41.0, 41.0));
    assert!(!over.contains(&overlay), "a cover is never hit");
    assert!(over.contains(&world), "so the world is");
}

/// It is the skin's scrim colour, full size, centring what it holds, and the host's own colour
/// replaces it with an override.
#[test]
fn a_scrim_is_the_skins_colour_full_size_and_centres_its_child() {
    let mut app = headless();
    let b = probe(&mut app, 100.0, 50.0);
    let layer = app
        .world_mut()
        .spawn((scrim(), GlobalZIndex(5)))
        .with_children(|l| {
            l.spawn(b);
        })
        .id();
    let owned = app
        .world_mut()
        .spawn((
            scrim(),
            InkOverride {
                fill: Some(RED),
                ..default()
            },
        ))
        .id();
    settle(&mut app);
    let r = rect(&app, layer);
    assert!(
        (r.width() - 1280.0).abs() < 0.5 && (r.height() - 720.0).abs() < 0.5,
        "{r:?}"
    );
    assert!(
        (rect(&app, children(&app, layer)[0]).center() - r.center()).length() < 0.5,
        "centred"
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(layer).expect("a fill").0,
        GooeySkin::dark().scrim
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(owned).expect("a fill").0,
        RED
    );
}

/// A hidden scrim (`Shown(false)`) is not hit, takes no room and keeps no tab stop; shown, it is
/// hit again. It is not a dialog: it traps no focus.
#[test]
fn a_hidden_scrim_is_not_hit_and_a_shown_one_traps_no_focus() {
    let mut app = headless();
    let world = world_below(&mut app);
    let behind = app.world_mut().spawn(button("Behind")).id();
    app.world_mut().spawn(screen()).add_child(behind);
    let inside = app.world_mut().spawn(button("Inside")).id();
    let layer = app
        .world_mut()
        .spawn((scrim(), Shown(false), GlobalZIndex(5)))
        .add_child(inside)
        .id();
    settle(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(layer),
        Some(&Visibility::Hidden)
    );
    let over = hovered_at(&mut app, Vec2::new(40.0, 40.0));
    assert!(
        over.contains(&world) && !over.contains(&layer),
        "a hidden scrim is not hit"
    );
    let stops: Vec<_> = (0..4)
        .map(|_| {
            bevy_gooey::testing::press(&mut app, KeyCode::Tab);
            bevy_gooey::testing::focused_label(&app)
        })
        .collect();
    assert!(
        stops.iter().all(|s| s.as_deref() != Some("Inside")),
        "no stop in a hidden layer: {stops:?}"
    );

    *app.world_mut().get_mut::<Shown>(layer).expect("shown") = Shown(true);
    settle(&mut app);
    let over = hovered_at(&mut app, Vec2::new(41.0, 41.0));
    assert!(
        over.contains(&layer) && !over.contains(&world),
        "shown, it blocks again"
    );
}

// ---- 6. per-site leftovers --------------------------------------------------------------------

/// A least height of lines of text follows the text size (the receipt row's `LinesRoom`): `Lines(2.0)`
/// is two body lines at the skin's list-line height, and grows with the player's text scale.
#[test]
fn an_extent_in_lines_follows_the_text_scale() {
    let mut heights = Vec::new();
    for scale in [1.0, 1.5] {
        let mut app = headless();
        let mut skin = GooeySkin::dark();
        skin.text_scale = scale;
        app.insert_resource(skin);
        let e = app
            .world_mut()
            .spawn((
                row(0.0),
                Sizing {
                    min_height: Some(Extent::Lines(2.0)),
                    ..default()
                },
            ))
            .id();
        app.world_mut().spawn(screen()).add_child(e);
        settle(&mut app);
        let Val::Px(h) = app.world().get::<Node>(e).expect("a node").min_height else {
            panic!("not px");
        };
        assert!(
            (rect(&app, e).height() - h).abs() < 0.5,
            "an empty row is as tall as its least height"
        );
        heights.push(h);
    }
    assert!(
        heights[0] > 0.0 && ((heights[1] / heights[0]) - 1.5).abs() < 0.02,
        "{heights:?}"
    );
}

/// A negative or non-finite number of lines is refused like every extent.
#[test]
fn lines_are_finite_checked_and_never_negative() {
    let mut app = headless();
    let e = app
        .world_mut()
        .spawn((
            row(0.0),
            Sizing {
                min_height: Some(Extent::Lines(-3.0)),
                max_height: Some(Extent::Lines(f32::NAN)),
                ..default()
            },
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(e);
    settle(&mut app);
    let node = app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node.min_height, Val::Px(0.0));
    assert_eq!(node.max_height, Val::Auto, "NaN leaves the node");
}

/// A role-less colour system needs a handle: a rail built from `placed_at`, `Sizing` and `InkFollow`
/// is found by `With<InkFollow>`, keeps the colours the host paints and has the skin's size.
#[test]
fn a_rail_is_a_placed_sizing_and_ink_follow_found_by_the_marker() {
    let mut app = headless();
    let rail = app
        .world_mut()
        .spawn((
            placed_at(0.0, 0.0),
            Sizing {
                width: Some(Extent::Px(3.0)),
                height: Some(Extent::Percent(100.0)),
                ..default()
            },
            InkFollow,
        ))
        .id();
    let host = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(200.0),
            height: Val::Px(120.0),
            ..default()
        })
        .add_child(rail)
        .id();
    app.world_mut().spawn(screen()).add_child(host);
    settle(&mut app);
    // The host's colour system queries the marker, not `Without<Role>`.
    let mut found = app.world_mut().query_filtered::<Entity, With<InkFollow>>();
    let ids: Vec<Entity> = found.iter(app.world()).collect();
    assert_eq!(ids, vec![rail]);
    app.world_mut()
        .entity_mut(rail)
        .insert(BackgroundColor(GREEN));
    app.world_mut().resource_mut::<GooeySkin>().surface = Color::srgb(0.3, 0.3, 0.3);
    settle(&mut app);
    let r = rect(&app, rail);
    assert!(
        (r.width() - 3.0).abs() < 0.5 && (r.height() - 120.0).abs() < 0.5,
        "{r:?}"
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(rail).expect("a fill").0,
        GREEN
    );
}

/// A margin the host computes (the toolbar's edge) is a `Space::Px` written from a system.
#[test]
fn a_computed_margin_is_a_px_space_written_by_a_system() {
    let mut app = headless();
    let e = app.world_mut().spawn((row(0.0), Spacing::default())).id();
    let host = app.world_mut().spawn(column(0.0)).add_child(e).id();
    room(&mut app, 400.0, host);
    settle(&mut app);
    for computed in [10.0_f32, 37.5] {
        app.world_mut()
            .get_mut::<Spacing>(e)
            .expect("spacing")
            .margin
            .left = Some(Space::Px(computed));
        settle(&mut app);
        assert_eq!(
            app.world().get::<Node>(e).expect("a node").margin.left,
            Val::Px(computed)
        );
    }
}

/// A placed label whose visibility the host decides each frame (a slot label) uses `Shown` instead of
/// writing `Node::display`: hidden, it is `Display::None`; shown, the display it had.
#[test]
fn a_placed_world_label_is_hidden_and_shown_with_shown() {
    let mut app = headless();
    let target = app
        .world_mut()
        .spawn(Transform::from_translation(Vec3::new(100.0, 50.0, 0.0)))
        .id();
    let label_node = app
        .world_mut()
        .spawn((
            placed_at(0.0, 0.0),
            WorldAnchor::new(target),
            children![label("Slot 2")],
        ))
        .id();
    settle(&mut app);
    let display = |app: &App| app.world().get::<Node>(label_node).expect("a node").display;
    assert_eq!(display(&app), Display::Flex);
    app.world_mut().entity_mut(label_node).insert(Shown(false));
    settle(&mut app);
    assert_eq!(
        display(&app),
        Display::None,
        "hidden by Shown, no Node patched"
    );
    app.world_mut().entity_mut(label_node).insert(Shown(true));
    settle(&mut app);
    assert_eq!(display(&app), Display::Flex);
}
