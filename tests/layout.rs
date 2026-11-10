//! The layout primitives: a row, a column, a fixed-width cell, an image box and an overlay, whose
//! spacing comes from the skin so a panel is built from widgets and never from a `Node` literal.
//! Laid out for real in a headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

fn app() -> App {
    let mut app = App::new();
    // A colour reading test is not what this file does, but the rows are measured the frame they
    // change, and no easing can move a laid-out box.
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
    for _ in 0..4 {
        app.update();
    }
}

/// A real image in the app's assets, so the box draws something.
fn image(app: &mut App) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default())
}

/// A node's rectangle in the window, logical px: `(left, top, right, bottom)`.
fn rect(app: &App, entity: Entity) -> (f32, f32, f32, f32) {
    let size = app
        .world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size();
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    (
        at.x - size.x / 2.0,
        at.y - size.y / 2.0,
        at.x + size.x / 2.0,
        at.y + size.y / 2.0,
    )
}

fn width(app: &App, entity: Entity) -> f32 {
    let (l, _, r, _) = rect(app, entity);
    r - l
}

fn children(app: &App, entity: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

/// The width `step` multiples of the skin's gap is: the one spacing unit every primitive reads.
fn gap_px(step: f32) -> f32 {
    GooeySkin::dark().gap * step
}

/// The rig bench's slot row, built from the primitives and from nothing else: a label cell, a plot
/// image box and the plot's hover image in an overlay.
fn slot_row(app: &mut App) -> Entity {
    let plot = image(app);
    let hover = image(app);
    let world = app.world_mut();
    let row = world
        .spawn(row(1.0))
        .with_children(|row| {
            row.spawn((cell(6.0), children![label("0 - clip 14")]));
            row.spawn(image_box(plot, Vec2::new(120.0, 60.0)));
            row.spawn((
                overlay(),
                children![
                    image_box(hover.clone(), Vec2::new(120.0, 60.0)),
                    image_box(hover, Vec2::new(120.0, 60.0)),
                ],
            ));
        })
        .id();
    let panel = world.spawn(panel()).add_child(row).id();
    world.spawn(screen()).add_child(panel);
    settle(app);
    row
}

/// The same row as the editor hand-built it, from `Node` literals.
fn hand_built_row(app: &mut App) -> Entity {
    let plot = image(app);
    let hover = image(app);
    let world = app.world_mut();
    let row = world
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(GooeySkin::dark().gap),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(GooeySkin::dark().text_size * 6.0),
                    min_width: Val::Px(0.0),
                    flex_shrink: 1.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
                children![label("0 - clip 14")],
            ));
            row.spawn((
                Node {
                    width: Val::Px(120.0),
                    height: Val::Px(60.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                ImageNode::new(plot),
            ));
            row.spawn((
                Node {
                    width: Val::Px(120.0),
                    height: Val::Px(60.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                children![(
                    Node {
                        width: Val::Px(120.0),
                        height: Val::Px(60.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    ImageNode::new(hover),
                )],
            ));
        })
        .id();
    let panel = world.spawn(panel()).add_child(row).id();
    world.spawn(screen()).add_child(panel);
    settle(app);
    row
}

/// The rig bench's acceptance: a slot row built from the primitives, with no `Node` literal, lands
/// where the hand-built one does, a skin gap apart.
#[test]
fn a_slot_row_built_from_primitives_lays_out_as_the_hand_built_one() {
    let mut built = app();
    let primitives = slot_row(&mut built);
    let mut hand = app();
    let literal = hand_built_row(&mut hand);

    let a = children(&built, primitives);
    let b = children(&hand, literal);
    assert_eq!(a.len(), 3, "a label cell, an image box and an overlay");
    assert_eq!(b.len(), 3);
    for (index, (mine, theirs)) in a.iter().zip(&b).enumerate() {
        let (m, t) = (rect(&built, *mine), rect(&hand, *theirs));
        for (side, (x, y)) in [(m.0, t.0), (m.1, t.1), (m.2, t.2), (m.3, t.3)]
            .into_iter()
            .enumerate()
        {
            assert!(
                (x - y).abs() < 0.5,
                "child {index}, edge {side}: {x} vs {y}"
            );
        }
    }

    // The gap between the first two is the skin's own gap, not a number of the caller's.
    let gap = rect(&built, a[1]).0 - rect(&built, a[0]).2;
    assert!(
        (gap - gap_px(1.0)).abs() < 0.5,
        "the row's gap is the skin's: {gap}"
    );
    // A cell is a fixed width in body text sizes, so a label column lines up down a list of rows.
    let cell = width(&built, a[0]);
    assert!(
        (cell - GooeySkin::dark().text_size * 6.0).abs() < 0.5,
        "the cell is six body text sizes wide: {cell}"
    );
    // The box is the size it was given, whatever the image inside is.
    let box_size = rect(&built, a[1]);
    assert!(((box_size.2 - box_size.0) - 120.0).abs() < 0.5);
    assert!(((box_size.3 - box_size.1) - 60.0).abs() < 0.5);
}

/// A row narrows its cells rather than letting them run past it, and no cell ever stands on another.
#[test]
fn a_row_narrower_than_its_cells_shrinks_them_and_never_overlaps() {
    let mut app = app();
    let world = app.world_mut();
    let room = world
        .spawn(Node {
            width: Val::Px(100.0),
            min_width: Val::Px(100.0),
            max_width: Val::Px(100.0),
            flex_shrink: 0.0,
            ..default()
        })
        .id();
    let narrow = world
        .spawn(row(1.0))
        .with_children(|row| {
            row.spawn((cell(8.0), children![label("left")]));
            row.spawn((cell(8.0), children![label("right")]));
        })
        .id();
    world.entity_mut(room).add_child(narrow);
    let panel = world.spawn(panel()).add_child(room).id();
    world.spawn(screen()).add_child(panel);
    settle(&mut app);

    let cells = children(&app, narrow);
    let (left, right) = (rect(&app, cells[0]), rect(&app, cells[1]));
    assert!(
        (right.0 - left.2).abs() < 0.5 || right.0 >= left.2,
        "the second cell starts at or after the first ends: {left:?} {right:?}"
    );
    assert!(
        (left.2 - left.0) < GooeySkin::dark().text_size * 8.0,
        "a squeezed cell is narrower than its width: {:?} {:?} row {:?}",
        left,
        right,
        rect(&app, narrow)
    );
    let (_l, _, r, _) = rect(&app, narrow);
    assert!(
        right.2 <= r + 0.5,
        "the cells stay inside the row: {right:?} of {r}"
    );
}

/// A cell given no width takes none and draws none of its words.
#[test]
fn a_zero_width_cell_collapses_and_draws_nothing() {
    let mut app = app();
    let world = app.world_mut();
    let holder = world
        .spawn((
            row(1.0),
            children![(cell(0.0), children![label("invisible")])],
        ))
        .id();
    world.spawn(screen()).add_child(holder);
    settle(&mut app);

    let cells = children(&app, holder);
    let cell = cells[0];
    assert_eq!(width(&app, cell), 0.0, "a zero-width cell takes no room");
    let node = app.world().get::<Node>(cell).expect("a node");
    assert_eq!(
        node.overflow.x,
        OverflowAxis::Clip,
        "what a cell cannot show is cut, not drawn over its neighbour"
    );
}

/// An image that has not loaded keeps its box, so a bench never reflows while a plot is written.
#[test]
fn an_unloaded_image_keeps_its_size_and_draws_nothing() {
    let mut app = app();
    let missing = app
        .world()
        .resource::<AssetServer>()
        .load("no such plot.png");
    let world = app.world_mut();
    let holder = world
        .spawn(image_box(missing.clone(), Vec2::new(200.0, 80.0)))
        .id();
    world.spawn(screen()).add_child(holder);
    settle(&mut app);

    let (l, t, r, b) = rect(&app, holder);
    assert!(((r - l) - 200.0).abs() < 0.5, "the box keeps its width");
    assert!(((b - t) - 80.0).abs() < 0.5, "the box keeps its height");
    let drawn = app.world().get::<ImageNode>(holder).expect("a picture");
    assert_eq!(drawn.image, missing, "the handle it was given");
    let assets = app.world().resource::<Assets<Image>>();
    assert!(
        assets.get(&missing).is_none(),
        "nothing is loaded for it, so nothing can draw"
    );
}

/// An overlay stacks what comes after its first child exactly on top of it, and the first child is
/// what gives the overlay its size.
#[test]
fn an_overlay_stacks_its_later_children_on_the_first() {
    let mut app = app();
    let base = image(&mut app);
    let over = image(&mut app);
    let world = app.world_mut();
    let overlay = world
        .spawn((
            overlay(),
            children![
                image_box(base, Vec2::new(140.0, 70.0)),
                image_box(over, Vec2::new(140.0, 70.0)),
            ],
        ))
        .id();
    world.spawn(screen()).add_child(overlay);
    settle(&mut app);

    let parts = children(&app, overlay);
    let under = rect(&app, parts[0]);
    let on_top = rect(&app, parts[1]);
    assert!(
        (under.0 - on_top.0).abs() < 0.5
            && (under.1 - on_top.1).abs() < 0.5
            && (under.2 - on_top.2).abs() < 0.5
            && (under.3 - on_top.3).abs() < 0.5,
        "the second picture covers the first: {under:?} {on_top:?}"
    );
    let outer = rect(&app, overlay);
    assert!(
        (outer.2 - outer.0 - 140.0).abs() < 0.5 && (outer.3 - outer.1 - 70.0).abs() < 0.5,
        "the overlay is as large as its first child: {outer:?}"
    );
    assert_eq!(
        app.world()
            .get::<Node>(parts[1])
            .expect("a node")
            .position_type,
        PositionType::Absolute,
        "the stacked child is out of the flow, so it adds no room"
    );
}

/// A column stacks its rows a gap apart, and a gap given as nothing gives no gap: the spacing is
/// the skin's however the host asks for it.
#[test]
fn a_column_stacks_its_rows_and_the_gap_is_the_skins() {
    let mut app = app();
    let world = app.world_mut();
    let spaced = world
        .spawn((column(2.0), children![button("One"), button("Two")]))
        .id();
    let flush = world
        .spawn((column(0.0), children![button("One"), button("Two")]))
        .id();
    let panel = world.spawn(panel()).add_children(&[spaced, flush]).id();
    world.spawn(screen()).add_child(panel);
    settle(&mut app);

    let rows = children(&app, spaced);
    let gap = rect(&app, rows[1]).1 - rect(&app, rows[0]).3;
    assert!(
        (gap - gap_px(2.0)).abs() < 0.5,
        "two skin gaps between the rows: {gap}"
    );
    let flush_rows = children(&app, flush);
    let none = rect(&app, flush_rows[1]).1 - rect(&app, flush_rows[0]).3;
    assert!(none.abs() < 0.5, "no gap at all: {none}");
    // A column is as wide as its widest row and never narrower.
    let widest = width(&app, flush_rows[0]).max(width(&app, flush_rows[1]));
    assert!((width(&app, spaced) - widest).abs() < 0.5);
}

/// The primitives are skinned: a preset's gap and text size move them, with no number of the
/// caller's left behind.
#[test]
fn the_primitives_follow_the_skin() {
    let mut app = app();
    let world = app.world_mut();
    let holder = world
        .spawn((
            column(1.0),
            children![(cell(6.0), children![label("a")]), label("b")],
        ))
        .id();
    world.spawn(screen()).add_child(holder);
    settle(&mut app);
    let before = width(&app, children(&app, holder)[0]);

    // The tool look is a smaller text size and a smaller gap than the default's.
    let tool = GooeySkin::tool();
    app.world_mut().insert_resource(tool.clone());
    settle(&mut app);
    let after = width(&app, children(&app, holder)[0]);
    assert!(
        (after - tool.text_size * 6.0).abs() < 0.5,
        "the cell follows the new text size: {after} of {}",
        tool.text_size * 6.0
    );
    assert!(
        after < before,
        "the tool's text is smaller: {after} < {before}"
    );
    let rows = children(&app, holder);
    let gap = rect(&app, rows[1]).1 - rect(&app, rows[0]).3;
    assert!(
        (gap - tool.gap).abs() < 0.5,
        "the gap follows the new skin: {gap} of {}",
        tool.gap
    );
}
