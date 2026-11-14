//! What was left of the editor's hand-built nodes after G-86 (G-87, #98): a host-painted colour, flow
//! on container roles and placed nodes, a blocking scrim and the frame, each laid out for real in a
//! headless window.

use std::collections::HashSet;

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::Pickable;
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

// ---- 5. the frame: a gap in logical px, and the frame built from gooey's parts -----------------

/// A gap in logical px: `GapPx(2.0)` between a row's children is 2 px, not a multiple of the skin's
/// gap, on a row, a column, a panel and a placed cluster.
#[test]
fn a_gap_px_is_that_many_px_between_children() {
    let mut app = headless();
    let mut made = Vec::new();
    for kind in 0..4 {
        let (a, b) = (probe(&mut app, 30.0, 20.0), probe(&mut app, 30.0, 20.0));
        let mut e = match kind {
            0 => app.world_mut().spawn(row(1.0)),
            1 => app.world_mut().spawn(column(1.0)),
            2 => app.world_mut().spawn((
                panel(),
                RowLayout {
                    direction: Some(FlexDirection::Row),
                    ..default()
                },
            )),
            _ => app.world_mut().spawn((
                placed_at(0.0, 0.0),
                RowLayout {
                    direction: Some(FlexDirection::Row),
                    ..default()
                },
            )),
        };
        e.insert(GapPx(2.0)).with_children(|p| {
            p.spawn(a);
            p.spawn(b);
        });
        let id = e.id();
        let host = app.world_mut().spawn(column(0.0)).add_child(id).id();
        room(&mut app, 400.0, host);
        made.push(id);
    }
    settle(&mut app);
    for (kind, e) in made.iter().enumerate() {
        let kids = children(&app, *e);
        let (a, b) = (rect(&app, kids[0]), rect(&app, kids[1]));
        let gap = if kind == 1 {
            b.min.y - a.max.y
        } else {
            b.min.x - a.max.x
        };
        assert!((gap - 2.0).abs() < 0.5, "kind {kind}: {gap}");
    }
}

/// It beats a `LayoutGap`, is finite-checked (not a number leaves the role's own gap, below zero is
/// none), reaches both axes of a wrapping flow, and a change restyles.
#[test]
fn a_gap_px_beats_a_layout_gap_is_finite_checked_and_restyles() {
    let mut app = headless();
    let both = app
        .world_mut()
        .spawn((
            row(3.0),
            GapPx(4.0),
            RowLayout {
                wrap: Some(FlexWrap::Wrap),
                ..default()
            },
        ))
        .id();
    let odd = app.world_mut().spawn((row(1.0), GapPx(f32::NAN))).id();
    let negative = app.world_mut().spawn((row(1.0), GapPx(-5.0))).id();
    for e in [both, odd, negative] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    let skin = GooeySkin::dark();
    let node = |app: &App, e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(
        (node(&app, both).column_gap, node(&app, both).row_gap),
        (Val::Px(4.0), Val::Px(4.0)),
        "over LayoutGap, on both axes of a wrap"
    );
    assert_eq!(
        node(&app, odd).column_gap,
        Val::Px(skin.gap),
        "NaN leaves the role's own gap"
    );
    assert_eq!(node(&app, negative).column_gap, Val::Px(0.0));
    app.world_mut().get_mut::<GapPx>(both).expect("a gap").0 = 9.0;
    settle(&mut app);
    assert_eq!(
        node(&app, both).column_gap,
        Val::Px(9.0),
        "a changed GapPx restyles"
    );
}

/// The editor's frame from gooey's parts: a window-sized root, a bar and a door strip with px gaps
/// and the host's own colours, a body of a dock, a viewport that takes the rest and a dock that
/// can be hidden, and a status band. Every container lets the pointer through to the world under it.
#[test]
fn the_frame_is_built_from_gooey_parts_and_lets_the_pointer_through() {
    let mut app = headless();
    let world = world_below(&mut app);
    let ignore = Pickable::IGNORE;
    let (a, b, c) = (
        probe(&mut app, 60.0, 20.0),
        probe(&mut app, 60.0, 20.0),
        probe(&mut app, 80.0, 20.0),
    );
    let bar = app
        .world_mut()
        .spawn((
            row(0.0),
            GapPx(8.0),
            Sizing {
                width: Some(Extent::Percent(100.0)),
                height: Some(Extent::Px(40.0)),
                ..default()
            },
            Spacing {
                padding: Sides {
                    left: Some(Space::Px(12.0)),
                    right: Some(Space::Px(12.0)),
                    ..default()
                },
                ..default()
            },
            RowLayout {
                align: Some(AlignItems::Center),
                shrink: Some(0.0),
                ..default()
            },
            BorderPx(Sides {
                bottom: Some(1.0),
                ..default()
            }),
            InkFollow,
            ignore,
        ))
        .with_children(|r| {
            r.spawn(a);
            r.spawn(b);
        })
        .id();
    let strip = app
        .world_mut()
        .spawn((
            row(0.0),
            GapPx(2.0),
            Sizing {
                width: Some(Extent::Percent(100.0)),
                height: Some(Extent::Px(30.0)),
                ..default()
            },
            RowLayout {
                shrink: Some(0.0),
                ..default()
            },
            ignore,
        ))
        .id();
    let dock = |app: &mut App, width: f32| {
        app.world_mut()
            .spawn((
                column(0.0),
                Sizing {
                    width: Some(Extent::Px(width)),
                    min_height: Some(Extent::Px(0.0)),
                    ..default()
                },
                RowLayout {
                    shrink: Some(0.0),
                    align: Some(AlignItems::FlexStart),
                    ..default()
                },
                ignore,
            ))
            .id()
    };
    let left = dock(&mut app, 200.0);
    let right = dock(&mut app, 150.0);
    app.world_mut().entity_mut(right).insert(Shown(false));
    let viewport = app
        .world_mut()
        .spawn((
            fill_column(0.0),
            Sizing {
                min_height: Some(Extent::Px(0.0)),
                ..default()
            },
            ignore,
        ))
        .id();
    let body = app
        .world_mut()
        .spawn((
            fill_region(),
            RowLayout {
                direction: Some(FlexDirection::Row),
                align: Some(AlignItems::Stretch),
                ..default()
            },
            ignore,
        ))
        .add_child(left)
        .add_child(viewport)
        .add_child(right)
        .id();
    let status = app
        .world_mut()
        .spawn((
            row(0.0),
            Sizing {
                width: Some(Extent::Percent(100.0)),
                height: Some(Extent::Px(24.0)),
                ..default()
            },
            RowLayout {
                shrink: Some(0.0),
                ..default()
            },
            ignore,
        ))
        .with_children(|r| {
            r.spawn(c);
        })
        .id();
    let root = app
        .world_mut()
        .spawn((
            sized(Sizing {
                width: Some(Extent::Percent(100.0)),
                height: Some(Extent::Vh(100.0)),
                min_height: Some(Extent::Px(0.0)),
                ..default()
            }),
            ignore,
        ))
        .add_child(bar)
        .add_child(strip)
        .add_child(body)
        .add_child(status)
        .id();
    app.world_mut()
        .spawn((
            screen(),
            Sizing {
                min_height: Some(Extent::Px(0.0)),
                ..default()
            },
        ))
        .add_child(root);
    settle(&mut app);

    let (r, bd, l, v) = (
        rect(&app, root),
        rect(&app, body),
        rect(&app, left),
        rect(&app, viewport),
    );
    assert!(
        (r.height() - 720.0).abs() < 0.5 && (r.width() - 1280.0).abs() < 0.5,
        "{r:?}"
    );
    assert!(
        (bd.height() - (720.0 - 40.0 - 30.0 - 24.0)).abs() < 0.5,
        "the body takes what the bars leave: {bd:?}"
    );
    assert!(
        (l.width() - 200.0).abs() < 0.5 && (v.width() - 1080.0).abs() < 0.5,
        "{l:?} {v:?}"
    );
    let kids = children(&app, bar);
    assert!(
        ((rect(&app, kids[1]).min.x - rect(&app, kids[0]).max.x) - 8.0).abs() < 0.5,
        "the bar's px gap"
    );
    assert!(
        (rect(&app, kids[0]).min.x - rect(&app, bar).min.x - 12.0).abs() < 0.5,
        "and its padding"
    );
    assert_eq!(
        app.world().get::<Node>(bar).expect("a node").border.bottom,
        Val::Px(1.0)
    );
    // Every container lets the pointer through: over the bar, the body and the status band the
    // world under them is what is hovered.
    for at in [
        Vec2::new(900.0, 20.0),
        Vec2::new(640.0, 300.0),
        Vec2::new(900.0, 710.0),
    ] {
        let over = hovered_at(&mut app, at);
        assert!(over.contains(&world), "the world is reached at {at}");
        assert!(
            ![bar, body, status, root, viewport]
                .iter()
                .any(|e| over.contains(e)),
            "no container is hit at {at}"
        );
    }
    *app.world_mut().get_mut::<Shown>(right).expect("shown") = Shown(true);
    settle(&mut app);
    assert!(
        (rect(&app, viewport).width() - 930.0).abs() < 0.5,
        "the second dock takes room when shown"
    );
}

// ---- review: motion, focus and the gap on a button box ----------------------------------------

/// A headless app with colour motion ON, on a 60 Hz clock the test sets.
fn headless_moving() -> App {
    let mut app = headless();
    app.insert_resource(MotionSettings::default());
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    app
}

fn near(a: Color, b: Color) -> bool {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    (a.red - b.red).abs() < 0.004
        && (a.green - b.green).abs() < 0.004
        && (a.blue - b.blue).abs() < 0.004
        && (a.alpha - b.alpha).abs() < 0.004
}

/// With colour motion ON a host's colours on an `InkFollow` node are what is drawn on the next frame:
/// nothing eases them. A node without the marker eases the same write (the control), so the test
/// would notice.
#[test]
fn with_motion_on_an_ink_follow_node_shows_the_colour_the_host_wrote_at_once() {
    let mut app = headless_moving();
    let followed = app
        .world_mut()
        .spawn((
            button("B"),
            InkFollow,
            BackgroundColor(RED),
            BorderColor::all(RED),
        ))
        .id();
    let eased = app.world_mut().spawn(button("A")).id();
    for e in [followed, eased] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    for _ in 0..20 {
        app.update();
    }
    for target in [GREEN, RED, GREEN, RED] {
        app.world_mut()
            .entity_mut(followed)
            .insert((BackgroundColor(target), BorderColor::all(target)));
        app.update();
        let fill = app
            .world()
            .get::<BackgroundColor>(followed)
            .expect("a fill")
            .0;
        let edge = app.world().get::<BorderColor>(followed).expect("an edge");
        assert!(
            near(fill, target),
            "the fill is what the host wrote: {fill:?} for {target:?}"
        );
        assert!(
            near(edge.top, target) && near(edge.left, target),
            "and the edge: {edge:?}"
        );
    }
    // The control: the same write on a node the skin owns is eased, so it reads an in-between colour.
    app.world_mut()
        .entity_mut(eased)
        .insert(BackgroundColor(GREEN));
    app.update();
    assert!(
        !near(
            app.world().get::<BackgroundColor>(eased).expect("a fill").0,
            GREEN
        ),
        "a node without the marker eases"
    );
}

/// Taking the marker off leaves no easing state behind: the node eases from the colour it shows to
/// the skin's, rather than jumping or springing from a stale one.
#[test]
fn removing_ink_follow_with_motion_on_eases_from_the_shown_colour_to_the_skins() {
    let mut app = headless_moving();
    let b = app
        .world_mut()
        .spawn((button("B"), InkFollow, BackgroundColor(RED)))
        .id();
    let plain = app.world_mut().spawn(button("A")).id();
    for e in [b, plain] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    for _ in 0..20 {
        app.update();
    }
    for target in [GREEN, RED] {
        app.world_mut()
            .entity_mut(b)
            .insert(BackgroundColor(target));
        app.update();
    }
    app.world_mut().entity_mut(b).remove::<InkFollow>();
    app.update();
    let skin_fill = app.world().get::<BackgroundColor>(plain).expect("a fill").0;
    let first = app.world().get::<BackgroundColor>(b).expect("a fill").0;
    assert!(
        !near(first, skin_fill),
        "it does not jump to the skin's colour"
    );
    assert!(
        near(first, RED) || !near(first, GREEN),
        "and does not start from a stale colour: {first:?}"
    );
    for _ in 0..180 {
        app.update();
    }
    assert!(
        near(
            app.world().get::<BackgroundColor>(b).expect("a fill").0,
            skin_fill
        ),
        "it settles on the skin's colour"
    );
}

/// The host paints the focus ring on an `InkFollow` node: the skin writes none, where a plain button
/// gets the ring.
#[test]
fn the_skin_writes_no_focus_ring_on_an_ink_follow_button() {
    use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
    let mut app = headless();
    let followed = app
        .world_mut()
        .spawn((button("B"), InkFollow, BorderColor::all(GREEN)))
        .id();
    let plain = app.world_mut().spawn(button("A")).id();
    for e in [followed, plain] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    for e in [followed, plain] {
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(e, FocusCause::Navigated);
        settle(&mut app);
        let ring = app.world().get::<BorderColor>(e).expect("an edge").top;
        if e == plain {
            assert_eq!(
                ring,
                GooeySkin::dark().focus_ring,
                "the skin draws the plain button's ring"
            );
        } else {
            assert_eq!(
                ring, GREEN,
                "the host's colour stays: the host paints the ring"
            );
        }
    }
}

/// A shown `scrim()` blocks the pointer only: Tab still reaches a control beneath it, which is the
/// host's to manage (use a `dialog()` for a focus trap).
#[test]
fn a_shown_scrim_does_not_trap_tab() {
    let mut app = headless();
    let behind = app.world_mut().spawn(button("Behind")).id();
    app.world_mut().spawn(screen()).add_child(behind);
    app.world_mut().spawn((scrim(), GlobalZIndex(5)));
    settle(&mut app);
    let stops: Vec<_> = (0..3)
        .map(|_| {
            bevy_gooey::testing::press(&mut app, KeyCode::Tab);
            bevy_gooey::testing::focused_label(&app)
        })
        .collect();
    assert!(
        stops.iter().any(|s| s.as_deref() == Some("Behind")),
        "Tab reaches the control beneath: {stops:?}"
    );
}

/// `GapPx` and `LayoutGap` are honoured on a `button_box` (a `Role::Button`) like `RowLayout`.
#[test]
fn a_button_box_takes_a_gap_px_and_a_layout_gap() {
    let mut app = headless();
    let mut boxes = Vec::new();
    for gap in 0..2 {
        let (a, b) = (probe(&mut app, 20.0, 20.0), probe(&mut app, 20.0, 20.0));
        let mut e = app.world_mut().spawn(button_box());
        if gap == 0 {
            e.insert(GapPx(11.0));
        } else {
            e.insert(LayoutGap(2.0));
        }
        e.with_children(|c| {
            c.spawn(a);
            c.spawn(b);
        });
        let id = e.id();
        let host = app.world_mut().spawn(column(0.0)).add_child(id).id();
        room(&mut app, 400.0, host);
        boxes.push(id);
    }
    settle(&mut app);
    let gap_of = |app: &App, e: Entity| {
        let kids = children(app, e);
        rect(app, kids[1]).min.x - rect(app, kids[0]).max.x
    };
    assert!(
        (gap_of(&app, boxes[0]) - 11.0).abs() < 0.5,
        "{}",
        gap_of(&app, boxes[0])
    );
    assert!(
        (gap_of(&app, boxes[1]) - 2.0 * GooeySkin::dark().gap).abs() < 0.5,
        "{}",
        gap_of(&app, boxes[1])
    );
}
