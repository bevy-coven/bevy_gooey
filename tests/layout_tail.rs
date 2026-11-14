//! What was left of the editor's hand-built nodes after G-87 (G-88, #99): an auto extent, lines that
//! round up, a host-owned dock grid, a scroll list and a host that owns `display`, each laid out for
//! real in a headless window.

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

// ---- 1. an auto extent and a filling split -----------------------------------------------------

/// `Extent::Auto` is the content's size: a panel (which has no width of its own on a screen) and a
/// `sized` box with a fixed width given `Auto` are as wide as what they hold.
#[test]
fn an_auto_extent_is_as_wide_as_the_widest_child() {
    let mut app = headless();
    let b = probe(&mut app, 70.0, 20.0);
    let boxed = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::Px(300.0)),
            ..default()
        }))
        .with_children(|c| {
            c.spawn(b);
        })
        .id();
    let line = app.world_mut().spawn(row(0.0)).add_child(boxed).id();
    room(&mut app, 400.0, line);
    settle(&mut app);
    assert!((rect(&app, boxed).width() - 300.0).abs() < 0.5);
    app.world_mut()
        .get_mut::<Sizing>(boxed)
        .expect("sizing")
        .width = Some(Extent::Auto);
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(boxed).expect("a node").width,
        Val::Auto,
        "a changed extent restyles"
    );
    assert!(
        (rect(&app, boxed).width() - 70.0).abs() < 0.5,
        "as wide as its child: {:?}",
        rect(&app, boxed)
    );
}

/// The shell split: a gooey `split` whose width is `Auto` and that takes what its sibling leaves
/// (`Extent::Auto` plus `TextFlow { grow, min_width }`), so no `Node` is inserted by the host.
#[test]
fn a_split_can_fill_what_its_sibling_leaves() {
    let mut app = headless();
    let sp = app
        .world_mut()
        .spawn((
            split(SplitAxis::Horizontal, (), ()),
            Sizing {
                width: Some(Extent::Auto),
                height: Some(Extent::Percent(100.0)),
                ..default()
            },
            TextFlow {
                grow: Some(1.0),
                min_width: Some(Extent::Px(0.0)),
                ..default()
            },
        ))
        .id();
    let side = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::Px(250.0)),
            height: Some(Extent::Percent(100.0)),
            ..default()
        }))
        .id();
    let host = app
        .world_mut()
        .spawn((
            row(0.0),
            Sizing {
                width: Some(Extent::Px(800.0)),
                height: Some(Extent::Px(200.0)),
                ..default()
            },
        ))
        .add_child(sp)
        .add_child(side)
        .id();
    app.world_mut().spawn(screen()).add_child(host);
    settle(&mut app);
    let (s, d) = (rect(&app, sp), rect(&app, side));
    assert!(
        (s.width() - 550.0).abs() < 0.5 && (d.width() - 250.0).abs() < 0.5,
        "{s:?} {d:?}"
    );
}

// ---- 2. lines that round up --------------------------------------------------------------------

/// `LinesCeil(n)` is n list lines rounded UP to a whole device pixel, so it is never short of the
/// text and follows the text size live; `Lines(n)` is unrounded. Checked at several text scales and
/// window scale factors.
#[test]
fn lines_that_round_up_are_whole_device_pixels_and_never_short() {
    for window_scale in [1.0_f32, 1.25, 1.5] {
        for text_scale in [1.0_f32, 1.07, 1.3, 1.55] {
            let mut app = headless_at(window_scale);
            let mut skin = GooeySkin::dark();
            skin.text_scale = text_scale;
            app.insert_resource(skin.clone());
            let e = app
                .world_mut()
                .spawn((
                    row(0.0),
                    Sizing {
                        min_height: Some(Extent::LinesCeil(2.0)),
                        ..default()
                    },
                ))
                .id();
            let plain = app
                .world_mut()
                .spawn((
                    row(0.0),
                    Sizing {
                        min_height: Some(Extent::Lines(2.0)),
                        ..default()
                    },
                ))
                .id();
            for x in [e, plain] {
                app.world_mut().spawn(screen()).add_child(x);
            }
            settle(&mut app);
            let (Val::Px(up), Val::Px(raw)) = (
                app.world().get::<Node>(e).expect("a node").min_height,
                app.world().get::<Node>(plain).expect("a node").min_height,
            ) else {
                panic!("not px")
            };
            let device = up * window_scale;
            assert!(
                (device - device.round()).abs() < 0.001,
                "whole device px: {up} at {window_scale}"
            );
            assert!(up >= raw - 0.0001, "never short: {up} < {raw}");
            assert!(
                up - raw < 1.0 / window_scale + 0.0001,
                "less than a device pixel over: {up} vs {raw}"
            );
        }
    }
}

/// It follows the text size live: a skin change moves it in the same frame.
#[test]
fn lines_that_round_up_follow_a_skin_change() {
    let mut app = headless();
    let e = app
        .world_mut()
        .spawn((
            row(0.0),
            Sizing {
                min_height: Some(Extent::LinesCeil(3.0)),
                ..default()
            },
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(e);
    settle(&mut app);
    let before = app.world().get::<Node>(e).expect("a node").min_height;
    app.world_mut().resource_mut::<GooeySkin>().text_scale = 1.5;
    settle(&mut app);
    assert_ne!(
        app.world().get::<Node>(e).expect("a node").min_height,
        before
    );
    assert!(
        (rect(&app, e).height()
            - match app.world().get::<Node>(e).expect("a node").min_height {
                Val::Px(p) => p,
                _ => 0.0,
            })
        .abs()
            < 0.5
    );
}

/// Not finite or negative is refused like every extent.
#[test]
fn lines_that_round_up_are_finite_checked() {
    let mut app = headless();
    let e = app
        .world_mut()
        .spawn((
            row(0.0),
            Sizing {
                min_height: Some(Extent::LinesCeil(-2.0)),
                max_height: Some(Extent::LinesCeil(f32::NAN)),
                ..default()
            },
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(e);
    settle(&mut app);
    let node = app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node.min_height, Val::Px(0.0));
    assert_eq!(node.max_height, Val::Auto);
}

// ---- 5. a host that owns `display` --------------------------------------------------------------

/// The labels Tab reaches in `presses` presses.
fn tab_stops(app: &mut App, presses: usize) -> Vec<Option<String>> {
    (0..presses)
        .map(|_| {
            bevy_gooey::testing::press(app, KeyCode::Tab);
            bevy_gooey::testing::focused_label(app)
        })
        .collect()
}

/// A host that writes `Node::display` itself keeps doing so: with `ShownFollow` on the node gooey
/// makes the page hidden from focus too (`Visibility::Hidden`) while the host's display is `None`,
/// and gives the visibility back when the host shows it. No `Shown`, no write of `display`.
#[test]
fn a_host_that_owns_display_gets_focus_right_with_shown_follow() {
    let mut app = headless();
    let page = app
        .world_mut()
        .spawn((column(0.0), ShownFollow, children![button("In the page")]))
        .id();
    let outside = app.world_mut().spawn(button("Outside")).id();
    app.world_mut()
        .spawn(screen())
        .add_child(page)
        .add_child(outside);
    settle(&mut app);
    app.world_mut()
        .get_mut::<Node>(page)
        .expect("a node")
        .display = Display::None;
    settle(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Hidden)
    );
    let stops = tab_stops(&mut app, 4);
    assert!(
        stops.iter().all(|s| s.as_deref() != Some("In the page")),
        "no stop in a page the host closed: {stops:?}"
    );
    app.world_mut()
        .get_mut::<Node>(page)
        .expect("a node")
        .display = Display::Flex;
    settle(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Inherited)
    );
    let stops = tab_stops(&mut app, 4);
    assert!(
        stops.iter().any(|s| s.as_deref() == Some("In the page")),
        "{stops:?}"
    );
}

/// The host's display survives a skin change (nothing of gooey's rewrites it), and adding
/// `ShownFollow` to a node that is already closed hides it; taking it off gives the visibility back.
#[test]
fn shown_follow_keeps_the_hosts_display_through_a_restyle_and_follows_add_and_remove() {
    let mut app = headless();
    let page = app
        .world_mut()
        .spawn((column(0.0), Visibility::Visible))
        .id();
    app.world_mut().spawn(screen()).add_child(page);
    settle(&mut app);
    app.world_mut()
        .get_mut::<Node>(page)
        .expect("a node")
        .display = Display::None;
    settle(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Visible),
        "not followed yet"
    );
    app.world_mut().entity_mut(page).insert(ShownFollow);
    settle(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Hidden),
        "added to a closed node"
    );
    app.world_mut().resource_mut::<GooeySkin>().surface = Color::srgb(0.3, 0.3, 0.3);
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(page).expect("a node").display,
        Display::None,
        "a restyle leaves the host's display"
    );
    app.world_mut().entity_mut(page).remove::<ShownFollow>();
    settle(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Visible),
        "what the node had comes back"
    );
    assert_eq!(
        app.world().get::<Node>(page).expect("a node").display,
        Display::None,
        "and the display stays the host's"
    );
}

// ---- 3. a dock the host places ------------------------------------------------------------------

/// A bare `Dock` on the host's own grid-placed node keeps that node through a skin change and builds
/// its body: the host owns the dock's grid placement and gooey writes nothing of it. It hides with
/// `Shown` and comes back as the grid it was.
#[test]
fn a_dock_the_host_places_in_its_own_grid_keeps_its_placement() {
    let mut app = headless();
    let host_node = Node {
        display: Display::Grid,
        grid_column: GridPlacement::start_span(2, 3),
        grid_row: GridPlacement::start(2),
        height: Val::Px(100.0),
        ..default()
    };
    let dock = app
        .world_mut()
        .spawn((Dock(DockLayout::group(["a"])), host_node.clone()))
        .id();
    let grid = app
        .world_mut()
        .spawn(Node {
            display: Display::Grid,
            width: Val::Px(600.0),
            grid_template_columns: vec![GridTrack::flex(1.0); 4],
            grid_template_rows: vec![GridTrack::px(40.0), GridTrack::auto()],
            ..default()
        })
        .add_child(dock)
        .id();
    app.world_mut().spawn(screen()).add_child(grid);
    settle(&mut app);
    let node = app.world().get::<Node>(dock).expect("a node").clone();
    assert_eq!(
        (node.grid_column, node.grid_row, node.display),
        (host_node.grid_column, host_node.grid_row, Display::Grid)
    );
    assert!(
        children(&app, dock)
            .iter()
            .any(|c| app.world().get::<Role>(*c) == Some(&Role::DockBody)),
        "the body is built"
    );
    app.world_mut().resource_mut::<GooeySkin>().surface = Color::srgb(0.3, 0.3, 0.3);
    settle(&mut app);
    let after = app.world().get::<Node>(dock).expect("a node").clone();
    assert_eq!(
        (after.grid_column, after.grid_row),
        (host_node.grid_column, host_node.grid_row),
        "a restyle leaves the placement"
    );
    let (g, d) = (rect(&app, grid), rect(&app, dock));
    assert!(
        (d.min.y - g.min.y - 40.0).abs() < 0.5,
        "second grid row: {d:?} in {g:?}"
    );
    app.world_mut().entity_mut(dock).insert(Shown(false));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(dock).expect("a node").display,
        Display::None
    );
    app.world_mut().entity_mut(dock).insert(Shown(true));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(dock).expect("a node").display,
        Display::Grid,
        "the grid it was"
    );
}

// ---- 4. a scroll region: a scrolling column that fills its parent -----------------------------

/// A `scroll_region` in a bounded parent takes the rest of it, scrolls what is taller, shows its rail
/// only then, and keeps the content out of the rail's lane.
#[test]
fn a_scroll_region_fills_its_parent_scrolls_and_shows_its_rail_only_when_needed() {
    let mut app = headless();
    let (strip, tall) = (probe(&mut app, 100.0, 40.0), probe(&mut app, 100.0, 1000.0));
    let region = app
        .world_mut()
        .spawn(scroll_region(0.5, None, children![tall]))
        .id();
    let host = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::Px(300.0)),
            height: Some(Extent::Px(300.0)),
            ..default()
        }))
        .with_children(|h| {
            h.spawn(strip);
        })
        .add_child(region)
        .id();
    app.world_mut().spawn(screen()).add_child(host);
    settle(&mut app);
    let r = rect(&app, region);
    assert!(
        (r.height() - 260.0).abs() < 0.5,
        "the rest of its parent: {r:?}"
    );
    let kids = children(&app, region);
    let (view, rail) = (kids[0], *kids.last().expect("a rail"));
    assert_eq!(app.world().get::<Role>(view), Some(&Role::ScrollView));
    assert_eq!(
        app.world().get::<Node>(rail).expect("a node").display,
        Display::Flex,
        "the rail shows for what is taller"
    );
    let skin = GooeySkin::dark();
    let Val::Px(gutter) = app.world().get::<Node>(view).expect("a node").padding.right else {
        panic!("not px")
    };
    assert!(gutter > skin.gap, "a lane for the rail: {gutter}");
    // A short content: no rail.
    let short = app
        .world_mut()
        .spawn(scroll_region(0.5, None, children![label("one line")]))
        .id();
    let host2 = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::Px(300.0)),
            height: Some(Extent::Px(300.0)),
            ..default()
        }))
        .add_child(short)
        .id();
    app.world_mut().spawn(screen()).add_child(host2);
    settle(&mut app);
    let rail = *children(&app, short).last().expect("a rail");
    assert_eq!(
        app.world().get::<Node>(rail).expect("a node").display,
        Display::None
    );
}

/// The gutter is the host's: `Some(Space::Px(20.0))` is a 20 px lane, a change after spawn restyles,
/// and the rows stand the host's gap apart.
#[test]
fn a_scroll_region_takes_the_hosts_gutter_and_gap_and_restyles() {
    let mut app = headless();
    let (a, b) = (probe(&mut app, 50.0, 20.0), probe(&mut app, 50.0, 20.0));
    let region = app
        .world_mut()
        .spawn(scroll_region(0.0, Some(Space::Px(20.0)), children![a, b]))
        .id();
    let host = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::Px(300.0)),
            height: Some(Extent::Px(300.0)),
            ..default()
        }))
        .add_child(region)
        .id();
    app.world_mut().spawn(screen()).add_child(host);
    settle(&mut app);
    let view = children(&app, region)[0];
    assert_eq!(
        app.world().get::<Node>(view).expect("a node").padding.right,
        Val::Px(20.0)
    );
    app.world_mut()
        .get_mut::<ScrollGutter>(view)
        .expect("a gutter")
        .0 = Some(Space::Px(33.0));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(view).expect("a node").padding.right,
        Val::Px(33.0),
        "a changed gutter restyles"
    );
    let col = children(&app, view)[0];
    let rows = children(&app, col);
    assert!(
        (rect(&app, rows[1]).min.y - rect(&app, rows[0]).max.y).abs() < 0.5,
        "gap 0"
    );
}

/// With colour motion ON and a skin change (which restyles every node on the calm spring) the host's
/// `display` and the followed visibility are untouched: nothing of gooey's rewrites what the host owns.
#[test]
fn shown_follow_survives_a_reskin_with_motion_on() {
    let mut app = headless();
    app.insert_resource(MotionSettings::default());
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    let page = app
        .world_mut()
        .spawn((panel(), ShownFollow, children![button("In")]))
        .id();
    app.world_mut().spawn(screen()).add_child(page);
    for _ in 0..10 {
        app.update();
    }
    app.world_mut()
        .get_mut::<Node>(page)
        .expect("a node")
        .display = Display::None;
    for _ in 0..3 {
        app.update();
    }
    app.world_mut().resource_mut::<GooeySkin>().surface = Color::srgb(0.3, 0.3, 0.3);
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(
        app.world().get::<Node>(page).expect("a node").display,
        Display::None
    );
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Hidden)
    );
}
