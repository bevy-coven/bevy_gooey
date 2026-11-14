//! What was left of the editor's hand-built nodes after G-84 (G-85, #96): spacing, a spacer, a
//! region with a gap and a hidden page, a shape and a size on any node, a text's flex measure and
//! the chrome parts, each laid out for real in a headless window and read back from the drawn
//! rectangles.

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

// ---- 1. margin and padding, in skin steps ----------------------------------------------------

fn step(n: usize) -> f32 {
    GooeySkin::dark().space(n)
}

fn margin_left(step: usize) -> Spacing {
    Spacing {
        margin: Sides {
            left: Some(Space::Step(step)),
            ..default()
        },
        ..default()
    }
}

/// A margin is a step of the skin's spacing scale: a row with a left margin of step 2 stands that
/// far in from its host.
#[test]
fn a_margin_is_a_step_of_the_skins_spacing_scale() {
    let mut app = headless();
    let b = probe(&mut app, 40.0, 20.0);
    let row = app
        .world_mut()
        .spawn((row(1.0), margin_left(2)))
        .with_children(|row| {
            row.spawn(b);
        })
        .id();
    let host = room(&mut app, 400.0, row);
    settle(&mut app);
    let (r, h) = (rect(&app, row), rect(&app, host));
    assert!(
        (r.min.x - h.min.x - step(2)).abs() < 0.5,
        "{r:?} in {h:?}, step 2 is {}",
        step(2)
    );
    assert!(step(2) > 0.0);
}

/// Padding on a column puts its children in from its edges on every side.
#[test]
fn padding_puts_children_in_from_the_edges() {
    let mut app = headless();
    let b = probe(&mut app, 40.0, 20.0);
    let column = app
        .world_mut()
        .spawn((
            column(0.0),
            Spacing {
                padding: Sides::all(Space::Step(1)),
                ..default()
            },
        ))
        .with_children(|column| {
            column.spawn(b);
        })
        .id();
    room(&mut app, 400.0, column);
    settle(&mut app);
    let (c, b) = (rect(&app, column), rect(&app, children(&app, column)[0]));
    assert!(
        (b.min.x - c.min.x - step(1)).abs() < 0.5 && (b.min.y - c.min.y - step(1)).abs() < 0.5,
        "{b:?} in {c:?}"
    );
    assert!(
        (c.height() - (20.0 + 2.0 * step(1))).abs() < 0.5,
        "padding is room: {c:?}"
    );
}

/// A text with a bottom margin keeps what follows it that far off.
#[test]
fn a_text_can_carry_a_bottom_margin() {
    let mut app = headless();
    let b = probe(&mut app, 40.0, 20.0);
    let text = app
        .world_mut()
        .spawn((
            label("Section"),
            Spacing {
                margin: Sides {
                    bottom: Some(Space::Step(3)),
                    ..default()
                },
                ..default()
            },
        ))
        .id();
    let b = app.world_mut().spawn(b).id();
    let column = app
        .world_mut()
        .spawn(column(0.0))
        .add_child(text)
        .add_child(b)
        .id();
    room(&mut app, 400.0, column);
    settle(&mut app);
    let gap = rect(&app, b).min.y - rect(&app, text).max.y;
    assert!((gap - step(3)).abs() < 0.5, "{gap} wanted {}", step(3));
}

/// A side left `None` leaves the node alone: a panel keeps its own padding when only its margin is
/// set, and `Space::Zero` is how to ask for none.
#[test]
fn a_side_left_none_leaves_the_node_and_zero_clears_it() {
    let mut app = headless();
    let plain = app.world_mut().spawn(panel()).id();
    let margined = app.world_mut().spawn((panel(), margin_left(1))).id();
    let cleared = app
        .world_mut()
        .spawn((
            panel(),
            Spacing {
                padding: Sides::all(Space::Zero),
                ..default()
            },
        ))
        .id();
    for e in [plain, margined, cleared] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    let node = |e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(
        node(margined).padding,
        node(plain).padding,
        "padding untouched"
    );
    assert_eq!(node(margined).margin.left, Val::Px(step(1)));
    assert_eq!(
        node(margined).margin.right,
        node(plain).margin.right,
        "the other sides untouched"
    );
    assert_eq!(
        node(cleared).padding,
        UiRect::all(Val::Px(0.0)),
        "Zero is none"
    );
    assert_ne!(node(plain).padding, UiRect::all(Val::Px(0.0)));
}

/// Every layout constructor takes it, and it follows the skin and a change.
#[test]
fn spacing_is_on_every_layout_constructor_and_follows_the_skin() {
    let mut app = headless();
    let all = [
        app.world_mut().spawn((row(1.0), margin_left(2))).id(),
        app.world_mut().spawn((column(1.0), margin_left(2))).id(),
        app.world_mut()
            .spawn((button_row(1.0), margin_left(2)))
            .id(),
        app.world_mut()
            .spawn((fill_column(1.0), margin_left(2)))
            .id(),
        app.world_mut().spawn((fill_region(), margin_left(2))).id(),
        app.world_mut()
            .spawn((placed_at(1.0, 1.0), margin_left(2)))
            .id(),
        app.world_mut().spawn((label("x"), margin_left(2))).id(),
    ];
    for e in all {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    for e in all {
        assert_eq!(
            app.world().get::<Node>(e).expect("a node").margin.left,
            Val::Px(step(2))
        );
    }
    app.world_mut().resource_mut::<GooeySkin>().spacing = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    settle(&mut app);
    for e in all {
        assert_eq!(
            app.world().get::<Node>(e).expect("a node").margin.left,
            Val::Px(3.0),
            "the skin's scale"
        );
    }
    app.world_mut()
        .get_mut::<Spacing>(all[0])
        .expect("spacing")
        .margin
        .left = Some(Space::Step(5));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(all[0]).expect("a node").margin.left,
        Val::Px(6.0)
    );
}

// ---- 3. a spacer ------------------------------------------------------------------------------

/// A spacer takes what its row leaves: the box after it is pushed to the far edge.
#[test]
fn a_spacer_takes_the_room_its_row_leaves() {
    let mut app = headless();
    let (a, b) = (probe(&mut app, 50.0, 20.0), probe(&mut app, 50.0, 20.0));
    let row = app
        .world_mut()
        .spawn(row(0.0))
        .with_children(|row| {
            row.spawn(a);
            row.spawn(spacer());
            row.spawn(b);
        })
        .id();
    let host = room(&mut app, 400.0, row);
    settle(&mut app);
    let kids = children(&app, row);
    assert!(
        (rect(&app, kids[2]).max.x - rect(&app, host).max.x).abs() < 0.5,
        "pushed to the far edge"
    );
    assert!(
        (rect(&app, kids[1]).width() - 300.0).abs() < 0.5,
        "{:?}",
        rect(&app, kids[1])
    );
}

/// Two spacers share the room equally, so what stands between them is centred.
#[test]
fn two_spacers_centre_what_is_between_them() {
    let mut app = headless();
    let middle = probe(&mut app, 100.0, 20.0);
    let row = app
        .world_mut()
        .spawn(row(0.0))
        .with_children(|row| {
            row.spawn(spacer());
            row.spawn(middle);
            row.spawn(spacer());
        })
        .id();
    let host = room(&mut app, 400.0, row);
    settle(&mut app);
    let mid = rect(&app, children(&app, row)[1]);
    assert!(
        (mid.center().x - rect(&app, host).center().x).abs() < 0.5,
        "{mid:?}"
    );
}

/// In a column it takes the height; it draws nothing and lets the pointer through.
#[test]
fn a_spacer_grows_in_a_column_and_is_not_pickable() {
    let mut app = headless();
    let (a, b) = (probe(&mut app, 50.0, 20.0), probe(&mut app, 50.0, 20.0));
    let spacer_entity = app.world_mut().spawn(spacer()).id();
    let column = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(100.0),
            height: Val::Px(300.0),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|column| {
            column.spawn(a);
        })
        .add_child(spacer_entity)
        .with_children(|column| {
            column.spawn(b);
        })
        .id();
    app.world_mut().spawn(screen()).add_child(column);
    settle(&mut app);
    assert!(
        (rect(&app, spacer_entity).height() - 260.0).abs() < 0.5,
        "{:?}",
        rect(&app, spacer_entity)
    );
    assert_eq!(
        app.world().get::<Pickable>(spacer_entity),
        Some(&Pickable::IGNORE)
    );
    assert!(
        app.world()
            .get::<BackgroundColor>(spacer_entity)
            .is_none_or(|c| c.0 == Color::NONE)
    );
}

// ---- 4. a fill region with a gap and a page that can be hidden --------------------------------

/// `fill_region_with(gap)` stacks its children a skin gap apart; `fill_region()` has none.
#[test]
fn a_fill_region_can_carry_a_gap() {
    let mut app = headless();
    let mut gap_between = Vec::new();
    for gapped in [true, false] {
        let (a, b) = (probe(&mut app, 50.0, 20.0), probe(&mut app, 50.0, 20.0));
        let region = if gapped {
            app.world_mut().spawn(fill_region_with(1.5)).id()
        } else {
            app.world_mut().spawn(fill_region()).id()
        };
        app.world_mut().entity_mut(region).with_children(|r| {
            r.spawn(a);
            r.spawn(b);
        });
        room(&mut app, 300.0, region);
        settle(&mut app);
        let kids = children(&app, region);
        gap_between.push(rect(&app, kids[1]).min.y - rect(&app, kids[0]).max.y);
    }
    assert!(
        (gap_between[0] - 1.5 * GooeySkin::dark().gap).abs() < 0.5,
        "{gap_between:?}"
    );
    assert!(
        gap_between[1].abs() < 0.5,
        "no gap without it: {gap_between:?}"
    );
}

/// A gap keeps the region's grow and its least height of nothing.
#[test]
fn a_gapped_fill_region_still_shrinks_to_zero() {
    let mut app = headless();
    let region = app.world_mut().spawn(fill_region_with(1.0)).id();
    let node = |app: &App| app.world().get::<Node>(region).expect("a node").clone();
    app.world_mut().spawn(screen()).add_child(region);
    settle(&mut app);
    assert_eq!(node(&app).flex_grow, 1.0);
    assert_eq!(node(&app).min_height, Val::ZERO);
    assert_eq!(node(&app).row_gap, Val::Px(GooeySkin::dark().gap));
}

/// `Shown(false)` is `Display::None` through the skin: the page takes no room and what follows
/// closes up, and `Shown(true)` brings it back, with the host never touching `Node`.
#[test]
fn a_hidden_page_takes_no_room_and_shown_brings_it_back() {
    let mut app = headless();
    let (strip, page, after) = (
        probe(&mut app, 100.0, 50.0),
        probe(&mut app, 100.0, 80.0),
        probe(&mut app, 100.0, 30.0),
    );
    let page = app.world_mut().spawn(page).id();
    let region = app
        .world_mut()
        .spawn((fill_region(), Shown(false)))
        .add_child(page)
        .id();
    let host = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(300.0),
            height: Val::Px(300.0),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|host| {
            host.spawn(strip);
        })
        .add_child(region)
        .with_children(|host| {
            host.spawn(after);
        })
        .id();
    app.world_mut().spawn(screen()).add_child(host);
    settle(&mut app);
    let after_entity = children(&app, host)[2];
    assert_eq!(
        app.world().get::<Node>(region).expect("a node").display,
        Display::None
    );
    assert!(
        (rect(&app, after_entity).min.y - rect(&app, children(&app, host)[0]).max.y).abs() < 0.5,
        "closed up"
    );
    *app.world_mut().get_mut::<Shown>(region).expect("shown") = Shown(true);
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(region).expect("a node").display,
        Display::Flex
    );
    assert!(
        rect(&app, region).height() > 1.0,
        "the page is back: {:?}",
        rect(&app, region)
    );
}

/// A node with no `Shown` is left as it is.
#[test]
fn a_node_with_no_shown_keeps_its_display() {
    let mut app = headless();
    let e = app.world_mut().spawn(row(1.0)).id();
    app.world_mut().spawn(screen()).add_child(e);
    settle(&mut app);
    app.world_mut().get_mut::<Node>(e).expect("a node").display = Display::None;
    app.world_mut()
        .get_mut::<Role>(e)
        .expect("a role")
        .set_changed();
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(e).expect("a node").display,
        Display::None
    );
}

// ---- 5. a size on any node, and a plain sized box ---------------------------------------------

fn px(width: f32, height: f32) -> Sizing {
    Sizing {
        width: Some(Extent::Px(width)),
        height: Some(Extent::Px(height)),
        ..default()
    }
}

/// `sized(sizing)` is a box of that width and height, whatever it holds.
#[test]
fn a_sized_box_is_its_width_and_height() {
    let mut app = headless();
    let big = probe(&mut app, 300.0, 300.0);
    let boxed = app
        .world_mut()
        .spawn(sized(px(120.0, 60.0)))
        .with_children(|b| {
            b.spawn(big);
        })
        .id();
    room(&mut app, 400.0, boxed);
    settle(&mut app);
    let r = rect(&app, boxed);
    assert!(
        (r.width() - 120.0).abs() < 0.5 && (r.height() - 60.0).abs() < 0.5,
        "{r:?}"
    );
}

/// A row that is the host's whole width and a px tall, that never shrinks: a terminal's line.
#[test]
fn a_row_can_be_a_percent_wide_and_a_px_tall_and_rigid() {
    let mut app = headless();
    let line = app
        .world_mut()
        .spawn((
            row(0.0),
            Sizing {
                width: Some(Extent::Percent(100.0)),
                height: Some(Extent::Px(30.0)),
                ..default()
            },
            RowLayout {
                shrink: Some(0.0),
                ..default()
            },
        ))
        .id();
    let host = room(&mut app, 400.0, line);
    settle(&mut app);
    let (l, h) = (rect(&app, line), rect(&app, host));
    assert!(
        (l.width() - h.width()).abs() < 0.5 && (l.height() - 30.0).abs() < 0.5,
        "{l:?} in {h:?}"
    );
}

/// A width in text sizes follows the skin's text scale.
#[test]
fn an_extent_in_text_sizes_follows_the_text_scale() {
    for scale in [1.0, 1.5] {
        let mut app = headless();
        let mut skin = GooeySkin::dark();
        skin.text_scale = scale;
        let want = 10.0 * skin.text_size * scale;
        app.insert_resource(skin);
        let boxed = app
            .world_mut()
            .spawn(sized(Sizing {
                width: Some(Extent::Text(10.0)),
                height: Some(Extent::Px(10.0)),
                ..default()
            }))
            .id();
        room(&mut app, 600.0, boxed);
        settle(&mut app);
        assert!(
            (rect(&app, boxed).width() - want).abs() < 0.5,
            "x{scale}: {:?} wanted {want}",
            rect(&app, boxed)
        );
    }
}

/// Least and greatest sizes clamp a column the room would stretch or shrink.
#[test]
fn min_and_max_sizes_clamp() {
    let mut app = headless();
    let small = probe(&mut app, 20.0, 10.0);
    let column = app
        .world_mut()
        .spawn((
            column(0.0),
            Sizing {
                max_width: Some(Extent::Px(100.0)),
                min_height: Some(Extent::Px(80.0)),
                ..default()
            },
        ))
        .with_children(|c| {
            c.spawn(small);
        })
        .id();
    room(&mut app, 400.0, column);
    settle(&mut app);
    let r = rect(&app, column);
    assert!(
        (r.width() - 100.0).abs() < 0.5 && (r.height() - 80.0).abs() < 0.5,
        "{r:?}"
    );
}

/// A side left `None` leaves the node, and `clip` is the overflow.
#[test]
fn a_size_left_none_leaves_the_node_and_clip_is_overflow() {
    let mut app = headless();
    let plain = app.world_mut().spawn(panel()).id();
    let only_width = app
        .world_mut()
        .spawn((
            panel(),
            Sizing {
                width: Some(Extent::Px(90.0)),
                clip: Some(true),
                ..default()
            },
        ))
        .id();
    for e in [plain, only_width] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    let node = |e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node(only_width).width, Val::Px(90.0));
    assert_eq!(
        node(only_width).height,
        node(plain).height,
        "height untouched"
    );
    assert_eq!(node(only_width).min_width, node(plain).min_width);
    assert_eq!(node(only_width).overflow, Overflow::clip());
    assert_ne!(node(plain).overflow, Overflow::clip());
}

/// A changed sizing moves the node in the same frame.
#[test]
fn a_changed_sizing_restyles() {
    let mut app = headless();
    let boxed = app.world_mut().spawn(sized(px(100.0, 50.0))).id();
    room(&mut app, 400.0, boxed);
    settle(&mut app);
    app.world_mut()
        .get_mut::<Sizing>(boxed)
        .expect("sizing")
        .width = Some(Extent::Px(200.0));
    settle(&mut app);
    assert!((rect(&app, boxed).width() - 200.0).abs() < 0.5);
}

// ---- 2. a placed node with a size, a border, a radius and a fill ------------------------------

fn host_with(app: &mut App, placed: impl Bundle) -> (Entity, Entity) {
    let flow = probe(app, 100.0, 50.0);
    let placed = app.world_mut().spawn(placed).id();
    let host = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(400.0),
            height: Val::Px(300.0),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|h| {
            h.spawn(flow);
        })
        .add_child(placed)
        .id();
    app.world_mut().spawn(screen()).add_child(host);
    (host, placed)
}

/// `placed_at` with a size, a border, a radius and a fill is the chip the editor hand-built:
/// everything from the skin.
#[test]
fn a_placed_node_can_have_a_size_a_border_a_radius_and_a_fill() {
    let mut app = headless();
    let (host, chip) = host_with(
        &mut app,
        (
            placed_at(30.0, 40.0),
            px(120.0, 60.0),
            Shape {
                border: Sides::all(1.0),
                ink: Some(Ink::Accent),
                radius: Some(Radius::Step(1)),
                fill: Some(1),
            },
        ),
    );
    settle(&mut app);
    let skin = GooeySkin::dark();
    let (h, c) = (rect(&app, host), rect(&app, chip));
    assert!(
        (c.min - (h.min + Vec2::new(30.0, 40.0))).length() < 0.5,
        "{c:?} in {h:?}"
    );
    assert!(
        (c.width() - 120.0).abs() < 0.5 && (c.height() - 60.0).abs() < 0.5,
        "{c:?}"
    );
    let node = app.world().get::<Node>(chip).expect("a node");
    assert_eq!(node.border, UiRect::all(Val::Px(skin.border)));
    assert_eq!(
        node.border_radius,
        BorderRadius::all(Val::Px(skin.corner(1)))
    );
    assert_eq!(
        app.world().get::<BorderColor>(chip).expect("a colour").top,
        skin.accent
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(chip).expect("a fill").0,
        skin.surface_at(1)
    );
}

/// Fields left `None` leave the node: a radius alone changes no border, colour or fill, and a
/// border on one side is that side.
#[test]
fn a_shape_changes_only_what_it_sets() {
    let mut app = headless();
    let (_, rounded) = host_with(
        &mut app,
        (
            placed_at(0.0, 0.0),
            Shape {
                radius: Some(Radius::Full),
                ..default()
            },
        ),
    );
    let (_, plain) = host_with(&mut app, placed_at(0.0, 0.0));
    let (_, rail) = host_with(
        &mut app,
        (
            placed_at(0.0, 0.0),
            Shape {
                border: Sides {
                    right: Some(1.0),
                    ..default()
                },
                ..default()
            },
        ),
    );
    settle(&mut app);
    let node = |e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node(rounded).border_radius, BorderRadius::MAX);
    assert_eq!(node(rounded).border, node(plain).border);
    assert_eq!(
        app.world().get::<BorderColor>(rounded),
        app.world().get::<BorderColor>(plain)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(rounded),
        app.world().get::<BackgroundColor>(plain)
    );
    assert_eq!(node(rail).border.right, Val::Px(GooeySkin::dark().border));
    assert_eq!(
        node(rail).border.left,
        node(plain).border.left,
        "the other sides untouched"
    );
    assert_eq!(
        app.world()
            .get::<BorderColor>(rail)
            .expect("a colour")
            .right,
        GooeySkin::dark().outline,
        "a border with no ink is the outline"
    );
}

/// A shape on a widget changes only its radius: a button keeps its own fill and edge.
#[test]
fn a_shape_on_a_button_keeps_its_fill() {
    let mut app = headless();
    let a = app.world_mut().spawn(button("A")).id();
    let b = app
        .world_mut()
        .spawn((
            button("B"),
            Shape {
                radius: Some(Radius::Step(0)),
                ..default()
            },
        ))
        .id();
    for e in [a, b] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    assert_eq!(
        app.world().get::<BackgroundColor>(a),
        app.world().get::<BackgroundColor>(b)
    );
    assert_eq!(
        app.world().get::<BorderColor>(a),
        app.world().get::<BorderColor>(b)
    );
    assert_eq!(
        app.world().get::<Node>(b).expect("a node").border_radius,
        BorderRadius::all(Val::Px(GooeySkin::dark().corner(0)))
    );
}

/// A changed shape restyles and follows the skin.
#[test]
fn a_changed_shape_restyles_and_follows_the_skin() {
    let mut app = headless();
    let (_, chip) = host_with(
        &mut app,
        (
            placed_at(0.0, 0.0),
            Shape {
                radius: Some(Radius::Step(1)),
                ..default()
            },
        ),
    );
    settle(&mut app);
    app.world_mut().resource_mut::<GooeySkin>().radii = [1.0, 7.0, 9.0];
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(chip).expect("a node").border_radius,
        BorderRadius::all(Val::Px(7.0))
    );
    app.world_mut()
        .get_mut::<Shape>(chip)
        .expect("a shape")
        .radius = Some(Radius::Step(2));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(chip).expect("a node").border_radius,
        BorderRadius::all(Val::Px(9.0))
    );
}

/// A placed cluster can run as a line: `RowLayout::direction` turns its children into a row.
#[test]
fn a_placed_node_can_lay_its_children_out_as_a_row() {
    let mut app = headless();
    let (a, b) = (probe(&mut app, 30.0, 20.0), probe(&mut app, 30.0, 20.0));
    let (_, chip) = host_with(
        &mut app,
        (
            placed_at(0.0, 0.0),
            RowLayout {
                direction: Some(FlexDirection::Row),
                ..default()
            },
        ),
    );
    app.world_mut().entity_mut(chip).with_children(|c| {
        c.spawn(a);
        c.spawn(b);
    });
    settle(&mut app);
    let kids = children(&app, chip);
    assert!(
        (rect(&app, kids[0]).min.y - rect(&app, kids[1]).min.y).abs() < 0.5,
        "one line"
    );
    assert!(rect(&app, kids[1]).min.x > rect(&app, kids[0]).max.x - 0.5);
}

// ---- 6. a text with its own flex measure -------------------------------------------------------

/// A text that is rigid (`shrink` 0) keeps the width its words have, beside a text that gives way.
#[test]
fn a_text_can_refuse_to_shrink() {
    let mut app = headless();
    let alone = app.world_mut().spawn(label("Reviewer")).id();
    let natural = app.world_mut().spawn(row(0.0)).add_child(alone).id();
    room(&mut app, 600.0, natural);
    let rigid = app
        .world_mut()
        .spawn(label_with(
            "Reviewer",
            TextFlow {
                shrink: Some(0.0),
                ..default()
            },
        ))
        .id();
    let long = app
        .world_mut()
        .spawn(fill_text(
            "A long line of words that has to wrap inside the room that is left over",
        ))
        .id();
    let row = app
        .world_mut()
        .spawn(row(1.0))
        .add_child(rigid)
        .add_child(long)
        .id();
    room(&mut app, 200.0, row);
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(rigid).expect("a node").flex_shrink,
        0.0
    );
    assert!(
        (rect(&app, rigid).width() - rect(&app, alone).width()).abs() < 0.5,
        "keeps its own width: {:?} vs {:?}",
        rect(&app, rigid),
        rect(&app, alone)
    );
}

/// A title that grows and may narrow to nothing takes what the row leaves.
#[test]
fn a_text_can_grow_and_narrow_to_zero() {
    let mut app = headless();
    let picture = probe(&mut app, 100.0, 20.0);
    let title = app
        .world_mut()
        .spawn(label_with(
            "A long title",
            TextFlow {
                grow: Some(1.0),
                min_width: Some(Extent::Px(0.0)),
                ..default()
            },
        ))
        .id();
    let row = app
        .world_mut()
        .spawn(row(1.0))
        .with_children(|r| {
            r.spawn(picture);
        })
        .add_child(title)
        .id();
    room(&mut app, 400.0, row);
    settle(&mut app);
    assert!(
        (rect(&app, title).width() - (400.0 - 100.0 - GooeySkin::dark().gap)).abs() < 0.5,
        "{:?}",
        rect(&app, title)
    );
    let mut narrow = headless();
    let picture = probe(&mut narrow, 100.0, 20.0);
    let title = narrow
        .world_mut()
        .spawn(label_with(
            "A long title",
            TextFlow {
                grow: Some(1.0),
                shrink: Some(1.0),
                min_width: Some(Extent::Px(0.0)),
                ..default()
            },
        ))
        .id();
    let row = narrow
        .world_mut()
        .spawn(bevy_gooey::row(1.0))
        .with_children(|r| {
            r.spawn(picture);
        })
        .add_child(title)
        .id();
    let host = room(&mut narrow, 60.0, row);
    settle(&mut narrow);
    assert!(
        rect(&narrow, title).width() < 0.5,
        "{:?}",
        rect(&narrow, title)
    );
    assert!(
        rect(&narrow, row).width() < rect(&narrow, host).width() + 0.5,
        "the row is no wider than its host"
    );
}

/// A text aligns itself on the cross axis, by its own field.
#[test]
fn a_text_can_align_itself() {
    let mut app = headless();
    let tall = probe(&mut app, 40.0, 80.0);
    let text = app
        .world_mut()
        .spawn(label_with(
            "Name",
            TextFlow {
                align: Some(AlignSelf::Center),
                ..default()
            },
        ))
        .id();
    let row = app
        .world_mut()
        .spawn((
            row(1.0),
            RowLayout {
                align: Some(AlignItems::FlexStart),
                ..default()
            },
        ))
        .with_children(|r| {
            r.spawn(tall);
        })
        .add_child(text)
        .id();
    room(&mut app, 400.0, row);
    settle(&mut app);
    let kids = children(&app, row);
    assert!((rect(&app, kids[1]).center().y - rect(&app, kids[0]).center().y).abs() < 0.5);
}

/// A field left `None` leaves the role's own: a `fill_text` keeps growing, and it takes the flow
/// too.
#[test]
fn a_text_flow_changes_only_what_it_sets() {
    let mut app = headless();
    let plain = app.world_mut().spawn(fill_text("a")).id();
    let aligned = app
        .world_mut()
        .spawn(fill_text_with(
            "b",
            TextFlow {
                align: Some(AlignSelf::Center),
                ..default()
            },
        ))
        .id();
    let counted = app
        .world_mut()
        .spawn((
            count_text("3"),
            TextFlow {
                shrink: Some(1.0),
                ..default()
            },
        ))
        .id();
    for e in [plain, aligned, counted] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    let node = |e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(
        node(aligned).flex_grow,
        node(plain).flex_grow,
        "still grows"
    );
    assert_eq!(node(aligned).min_width, node(plain).min_width);
    assert_eq!(node(aligned).align_self, AlignSelf::Center);
    assert_eq!(
        node(counted).flex_shrink,
        1.0,
        "the flow beats the count's no-shrink"
    );
    assert_eq!(node(counted).flex_grow, 0.0, "grow untouched");
}

// ---- 7. the chrome builders: what gooey has, and the list row it lacked -----------------------

#[derive(Resource, Default)]
struct Clicks(u32);

/// A list row is full width, with its left rail reserved: selecting it lights the rail and changes
/// the fill, and moves nothing in it.
#[test]
fn a_list_row_lights_a_rail_when_selected_and_moves_nothing() {
    let mut app = headless();
    let text = app.world_mut().spawn(label("a row of a list")).id();
    let row = app.world_mut().spawn(list_row()).add_child(text).id();
    let host = room(&mut app, 400.0, row);
    settle(&mut app);
    let skin = GooeySkin::dark();
    let (before_row, before_text) = (rect(&app, row), rect(&app, text));
    assert!(
        (before_row.width() - rect(&app, host).width()).abs() < 0.5,
        "full width: {before_row:?}"
    );
    let node = app.world().get::<Node>(row).expect("a node");
    let reserved = match node.border.left {
        Val::Px(px) => px,
        _ => 0.0,
    };
    assert!(
        reserved >= skin.border,
        "the rail is reserved: {:?}",
        node.border
    );
    let rest_fill = app.world().get::<BackgroundColor>(row).expect("a fill").0;
    assert_eq!(
        app.world().get::<BorderColor>(row).expect("an edge").left,
        Color::NONE,
        "no rail at rest"
    );

    app.world_mut().entity_mut(row).insert(bevy::ui::Selected);
    settle(&mut app);
    assert_eq!(
        app.world().get::<BorderColor>(row).expect("an edge").left,
        skin.accent,
        "the rail is the accent"
    );
    assert_ne!(
        app.world().get::<BackgroundColor>(row).expect("a fill").0,
        rest_fill,
        "and the fill changes"
    );
    assert_eq!(rect(&app, row), before_row, "the row does not move");
    assert_eq!(rect(&app, text), before_text, "and neither does its text");
}

/// A list row is on the `Activate` bus; the quiet one looks the same and is not.
#[test]
fn a_quiet_list_row_looks_like_a_list_row_and_stays_off_the_activate_bus() {
    let mut app = headless();
    app.init_resource::<Clicks>();
    let count = |_: On<Activate>, mut n: ResMut<Clicks>| n.0 += 1;
    let on = app.world_mut().spawn((list_row(), observe(count))).id();
    let off = app
        .world_mut()
        .spawn((quiet_list_row(), observe(count)))
        .id();
    let holder = app
        .world_mut()
        .spawn(column(0.0))
        .add_child(on)
        .add_child(off)
        .id();
    room(&mut app, 400.0, holder);
    settle(&mut app);
    assert!(app.world().get::<bevy::ui_widgets::Button>(off).is_none());
    assert_eq!(
        app.world().get::<BackgroundColor>(on),
        app.world().get::<BackgroundColor>(off)
    );
    assert_eq!(rect(&app, on).height(), rect(&app, off).height());
    bevy_gooey::testing::click(&mut app, off);
    settle(&mut app);
    assert_eq!(
        app.world().resource::<Clicks>().0,
        0,
        "a quiet row sends no Activate"
    );
    bevy_gooey::testing::click(&mut app, on);
    settle(&mut app);
    assert_eq!(app.world().resource::<Clicks>().0, 1, "a list row does");
}

/// The severity rail is a column with a border on its left in the severity's ink, padding and a
/// margin below, all from the skin: no hand-built node.
#[test]
fn a_severity_rail_is_a_column_with_a_shape_and_spacing() {
    let mut app = headless();
    let skin = GooeySkin::dark();
    let b = probe(&mut app, 40.0, 20.0);
    let rail = app
        .world_mut()
        .spawn((
            column(0.0),
            Shape {
                border: Sides {
                    left: Some(1.0),
                    ..default()
                },
                ink: Some(Ink::Danger),
                ..default()
            },
            Spacing {
                padding: Sides {
                    left: Some(Space::Step(1)),
                    ..default()
                },
                margin: Sides {
                    bottom: Some(Space::Step(1)),
                    ..default()
                },
            },
        ))
        .with_children(|r| {
            r.spawn(b);
        })
        .id();
    room(&mut app, 400.0, rail);
    settle(&mut app);
    assert_eq!(
        app.world().get::<BorderColor>(rail).expect("an edge").left,
        skin.danger
    );
    let (r, c) = (rect(&app, rail), rect(&app, children(&app, rail)[0]));
    assert!(
        (c.min.x - r.min.x - skin.border - skin.space(1)).abs() < 0.5,
        "{c:?} in {r:?}"
    );
}

/// A modal is the dialog gooey has: hidden until shown with `Shown`, its card a width of its own.
#[test]
fn a_modal_is_a_dialog_hidden_until_shown_with_a_card_of_its_own_width() {
    let mut app = headless();
    let card = app
        .world_mut()
        .spawn((
            sized_panel(CardWidth::Text(20.0)),
            children![label("Name it")],
        ))
        .id();
    let modal = app
        .world_mut()
        .spawn((dialog(), Shown(false)))
        .add_child(card)
        .id();
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(modal).expect("a node").display,
        Display::None
    );
    *app.world_mut().get_mut::<Shown>(modal).expect("shown") = Shown(true);
    settle(&mut app);
    let skin = GooeySkin::dark();
    assert_eq!(
        app.world().get::<Node>(modal).expect("a node").display,
        Display::Flex
    );
    assert!(
        (rect(&app, card).width() - 20.0 * skin.text_size).abs() < 1.0,
        "{:?}",
        rect(&app, card)
    );
}

/// The frame of an editor is built from parts gooey has: a bar, a body row (a dock, a viewport that
/// takes the rest, a second dock that can be hidden) and a status band, in a box of the window's size.
#[test]
fn a_frame_is_built_from_gooey_parts() {
    let mut app = headless();
    let (bar, band, left_panel) = (
        probe(&mut app, 100.0, 30.0),
        probe(&mut app, 100.0, 20.0),
        probe(&mut app, 50.0, 900.0),
    );
    let panel_node = app
        .world_mut()
        .spawn((
            panel(),
            Sizing {
                width: Some(Extent::Px(200.0)),
                min_height: Some(Extent::Px(0.0)),
                ..default()
            },
            TextFlow {
                grow: Some(1.0),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn(left_panel);
        })
        .id();
    let left = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::Px(200.0)),
            min_height: Some(Extent::Px(0.0)),
            ..default()
        }))
        .add_child(panel_node)
        .id();
    let centre = app.world_mut().spawn(fill_column(0.0)).id();
    let right = app
        .world_mut()
        .spawn((
            sized(Sizing {
                width: Some(Extent::Px(150.0)),
                ..default()
            }),
            Shown(false),
        ))
        .id();
    let body = app
        .world_mut()
        .spawn((
            fill_region(),
            RowLayout {
                direction: Some(FlexDirection::Row),
                ..default()
            },
        ))
        .add_child(left)
        .add_child(centre)
        .add_child(right)
        .id();
    let bar = app.world_mut().spawn(bar).id();
    let band = app.world_mut().spawn(band).id();
    let frame = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::Percent(100.0)),
            height: Some(Extent::Percent(100.0)),
            min_height: Some(Extent::Px(0.0)),
            ..default()
        }))
        .add_child(bar)
        .add_child(body)
        .add_child(band)
        .id();
    // The window's root takes the window's height and no less: like every box in the chain it may
    // give way to nothing (`min_height` 0), or the tall child below would hold the whole chain open.
    let window_root = app
        .world_mut()
        .spawn((
            screen(),
            Sizing {
                min_height: Some(Extent::Px(0.0)),
                ..default()
            },
        ))
        .add_child(frame)
        .id();
    settle(&mut app);
    let (f, b, l, c) = (
        rect(&app, frame),
        rect(&app, body),
        rect(&app, left),
        rect(&app, centre),
    );
    assert!(
        (f.height() - 720.0).abs() < 0.5 && (f.width() - 1280.0).abs() < 0.5,
        "root {:?} frame {f:?} body {b:?} left {l:?} centre {c:?} panel {:?}",
        rect(&app, window_root),
        rect(&app, panel_node)
    );
    assert!(
        (b.height() - (720.0 - 30.0 - 20.0)).abs() < 0.5,
        "the body takes what the bar and the band leave: {b:?}"
    );
    assert!(
        (l.width() - 200.0).abs() < 0.5 && (c.width() - 1080.0).abs() < 0.5,
        "{l:?} {c:?}"
    );
    assert!(
        (rect(&app, panel_node).height() - b.height()).abs() < 0.5,
        "the dock's panel takes the dock's whole height, a tall child notwithstanding"
    );
    *app.world_mut().get_mut::<Shown>(right).expect("shown") = Shown(true);
    settle(&mut app);
    assert!(
        (rect(&app, centre).width() - 930.0).abs() < 0.5,
        "the second dock takes room when shown: {:?}",
        rect(&app, centre)
    );
}
