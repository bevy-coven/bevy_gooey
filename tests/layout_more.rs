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
