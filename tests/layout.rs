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

/// A tree in reading order: the entity, then all its descendants depth first.
fn tree(app: &App, root: Entity) -> Vec<Entity> {
    let mut out = vec![root];
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        for child in children(app, entity).into_iter().rev() {
            out.push(child);
            stack.push(child);
        }
    }
    out
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
/// image box and the plot's hover image stacked in an overlay.
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

/// The same row as the editor hand-built it (`anim_tab.rs` ~1409-1441), from `Node` literals: a
/// fixed-width label, the plot, and a relative wrapper holding the plot again with the hover image
/// absolute at 0/0 over it; the three stand a gap apart.
fn hand_built_row(app: &mut App) -> Entity {
    let plot = image(app);
    let hover = image(app);
    let size = Vec2::new(120.0, 60.0);
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
                    width: Val::Px(size.x),
                    height: Val::Px(size.y),
                    flex_shrink: 0.0,
                    ..default()
                },
                ImageNode::new(plot),
            ));
            // The editor's wrapper: relative, holding the plot and the hover image stacked on it.
            row.spawn(Node {
                width: Val::Px(size.x),
                height: Val::Px(size.y),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|wrap| {
                wrap.spawn((
                    Node {
                        width: Val::Px(size.x),
                        height: Val::Px(size.y),
                        ..default()
                    },
                    ImageNode::new(hover.clone()),
                ));
                wrap.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Px(size.x),
                        height: Val::Px(size.y),
                        ..default()
                    },
                    ImageNode::new(hover),
                ));
            });
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

    // Every node of both trees, in step: the row's own children and all their descendants (a
    // cell's label, an overlay's two pictures), so a box one level down is held too.
    let mine = tree(&built, primitives);
    let theirs = tree(&hand, literal);
    assert_eq!(
        mine.len(),
        theirs.len(),
        "the same nodes: {} against {}",
        mine.len(),
        theirs.len()
    );
    for (index, (a, b)) in mine.iter().zip(&theirs).enumerate() {
        let (m, t) = (rect(&built, *a), rect(&hand, *b));
        for (side, (x, y)) in [(m.0, t.0), (m.1, t.1), (m.2, t.2), (m.3, t.3)]
            .into_iter()
            .enumerate()
        {
            assert!(
                (x - y).abs() < 0.5,
                "node {index} of {} ({a:?} against {b:?}), edge {side}: {x} vs {y}",
                mine.len()
            );
        }
    }
    // The overlay's third child is the hover picture, and it covers the plot exactly.
    let overlay = a[2];
    let stacked = children(&built, overlay);
    assert_eq!(stacked.len(), 2);
    assert_eq!(rect(&built, stacked[0]), rect(&built, stacked[1]));

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

/// A row narrows its cells rather than letting them run past it, both shrink by the same share (they
/// are the same width), and no cell ever stands on another.
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
    assert!(
        ((left.2 - left.0) - (right.2 - right.0)).abs() < 0.5,
        "equal cells shrink by the same share: {:?} {:?}",
        left,
        right
    );
    let (_l, _, r, _) = rect(&app, narrow);
    assert!(
        right.2 <= r + 0.5,
        "the cells stay inside the row: {right:?} of {r}"
    );
}

/// A cell given no width takes none, draws none of its words, and leaves no gap beside it.
#[test]
fn a_zero_width_cell_collapses_and_draws_nothing() {
    let mut app = app();
    let world = app.world_mut();
    let holder = world
        .spawn(row(1.0))
        .with_children(|row| {
            row.spawn((cell(4.0), children![label("left")]));
            row.spawn((cell(0.0), children![label("invisible")]));
            row.spawn((cell(4.0), children![label("right")]));
        })
        .id();
    world.spawn(screen()).add_child(holder);
    settle(&mut app);

    let cells = children(&app, holder);
    let empty = cells[1];
    assert_eq!(width(&app, empty), 0.0, "a zero-width cell takes no room");
    // Nothing it holds is drawn: its label's box is empty of room.
    let label = children(&app, empty)[0];
    let (l, t, r, b) = rect(&app, label);
    assert!(
        (r - l) <= 0.5 && (b - t) <= 0.5,
        "the words inside are drawn into nothing: {l}..{r} by {t}..{b}"
    );
    // And no gap stands where it would be: the two cells that are there are one gap apart.
    let gap = rect(&app, cells[2]).0 - rect(&app, cells[0]).2;
    assert!(
        (gap - gap_px(1.0)).abs() < 0.5,
        "one gap between the cells that are shown, not two: {gap}"
    );
    assert_eq!(cells.len(), 3, "it is still the row's child");
}

/// An image that has not loaded keeps its box, draws nothing, and is never squeezed by a narrow
/// row: the cells beside it give way first.
#[test]
fn an_unloaded_image_keeps_its_size_and_draws_nothing() {
    let mut app = app();
    let missing = app
        .world()
        .resource::<AssetServer>()
        .load("no such plot.png");
    let world = app.world_mut();
    let holder = world
        .spawn(row(1.0))
        .with_children(|row| {
            row.spawn((cell(10.0), children![label("a long label in the way")]));
            row.spawn(image_box(missing.clone(), Vec2::new(200.0, 80.0)));
            row.spawn((cell(10.0), children![label("another long label")]));
        })
        .id();
    let room = world
        .spawn(Node {
            width: Val::Px(320.0),
            min_width: Val::Px(320.0),
            max_width: Val::Px(320.0),
            flex_shrink: 0.0,
            ..default()
        })
        .id();
    world.entity_mut(room).add_child(holder);
    world.spawn(screen()).add_child(room);
    settle(&mut app);

    let cells = children(&app, holder);
    let (l, t, r, b) = rect(&app, cells[1]);
    assert!(
        ((r - l) - 200.0).abs() < 0.5,
        "the box keeps its width in a row with 120 px to spare: {} of 200",
        r - l
    );
    assert!(((b - t) - 80.0).abs() < 0.5, "and its height: {}", b - t);
    // Nothing is drawn in it: the handle is one Bevy has no image for, and the node draws no
    // background of its own (a box is the picture and nothing else).
    let drawn = app
        .world()
        .get::<ImageNode>(cells[1])
        .expect("the box's node");
    assert_eq!(drawn.image, missing, "the handle it was given");
    let fill = app.world().get::<BackgroundColor>(cells[1]);
    assert!(
        fill.is_none() || fill.is_some_and(|fill| fill.0 == Color::NONE),
        "no fill stands in for the missing picture: {fill:?}"
    );
    // The cells gave way instead: both are narrower than their ten text sizes.
    for cell in [cells[0], cells[2]] {
        assert!(
            width(&app, cell) < GooeySkin::dark().text_size * 10.0 - 1.0,
            "a cell gave way: {}",
            width(&app, cell)
        );
    }
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

/// An overlay whose first child is despawned is the size of its new first child, in the flow: the
/// stacking of a child that becomes first is given back.
#[test]
fn despawning_an_overlays_first_child_leaves_the_next_one_in_the_flow() {
    let mut app = app();
    let under = image(&mut app);
    let over = image(&mut app);
    let world = app.world_mut();
    let first = world.spawn(image_box(under, Vec2::new(160.0, 90.0))).id();
    let second = world
        .spawn(image_box(over.clone(), Vec2::new(160.0, 90.0)))
        .id();
    let third = world.spawn(image_box(over, Vec2::new(160.0, 90.0))).id();
    let overlay = world
        .spawn(overlay())
        .add_children(&[first, second, third])
        .id();
    world.spawn(screen()).add_child(overlay);
    settle(&mut app);
    assert!(((rect(&app, overlay).2 - rect(&app, overlay).0) - 160.0).abs() < 0.5);

    // The picture a plot is rebuilt from: the first child goes.
    app.world_mut().entity_mut(first).despawn();
    settle(&mut app);
    let now = children(&app, overlay);
    assert_eq!(now.len(), 2);
    assert_eq!(now[0], second, "the picture that followed it is first now");
    assert_eq!(
        app.world()
            .get::<Node>(now[0])
            .expect("a node")
            .position_type,
        PositionType::Relative,
        "and it is back in the flow"
    );
    let (l, t, r, b) = rect(&app, overlay);
    assert!(
        ((r - l) - 160.0).abs() < 0.5 && ((b - t) - 90.0).abs() < 0.5,
        "the overlay is the new first child's size: {l}..{r} by {t}..{b}"
    );
    assert!(
        app.world().get::<OverlayStacked>(now[0]).is_none(),
        "and carries no stacking"
    );
    assert!(
        app.world().get::<OverlayStacked>(now[1]).is_some(),
        "while the one over it still does"
    );
}

/// A stacked child moved out of its overlay is back in the flow wherever it lands, and the overlay
/// it left is still its first child's size.
#[test]
fn a_stacked_child_moved_out_is_in_the_flow_again() {
    let mut app = app();
    let under = image(&mut app);
    let over = image(&mut app);
    let world = app.world_mut();
    let base = world.spawn(image_box(under, Vec2::new(140.0, 70.0))).id();
    let stacked = world
        .spawn(image_box(over.clone(), Vec2::new(140.0, 70.0)))
        .id();
    let overlay = world.spawn(overlay()).add_children(&[base, stacked]).id();
    let elsewhere = world.spawn(row(0.0)).id();
    world.spawn(screen()).add_child(overlay);
    settle(&mut app);
    assert!(app.world().get::<OverlayStacked>(stacked).is_some());

    // Moved into a row, where it is an item of that row and must be laid out as one.
    let second = app
        .world_mut()
        .spawn(image_box(over, Vec2::new(140.0, 70.0)))
        .id();
    app.world_mut()
        .entity_mut(elsewhere)
        .add_children(&[stacked, second]);
    settle(&mut app);

    let node = app.world().get::<Node>(stacked).expect("a node");
    assert_eq!(
        node.position_type,
        PositionType::Relative,
        "back in the flow"
    );
    assert_eq!(node.width, Val::Px(140.0), "its own size again");
    assert!(app.world().get::<OverlayStacked>(stacked).is_none());
    let (l, _, r, _) = rect(&app, stacked);
    assert!(((r - l) - 140.0).abs() < 0.5, "and drawn at it");
    // The row places it after the other, not on top of it.
    let (l2, _, _, _) = rect(&app, second);
    assert!(l2 > r - 0.5, "the two stand side by side: {r} then {l2}");
    // And the overlay it left is unchanged.
    assert!(((rect(&app, overlay).2 - rect(&app, overlay).0) - 140.0).abs() < 0.5);
}

/// In a row short of room a cell gives way before an image box, which is never squeezed.
#[test]
fn a_cell_gives_way_before_an_image_box() {
    let mut app = app();
    let plot = image(&mut app);
    let world = app.world_mut();
    let row = world
        .spawn(row(1.0))
        .with_children(|row| {
            row.spawn((
                cell(12.0),
                children![label("a label far too long for this line")],
            ));
            row.spawn(image_box(plot, Vec2::new(200.0, 80.0)));
        })
        .id();
    let room = world
        .spawn(Node {
            width: Val::Px(260.0),
            min_width: Val::Px(260.0),
            max_width: Val::Px(260.0),
            flex_shrink: 0.0,
            ..default()
        })
        .id();
    world.entity_mut(room).add_child(row);
    world.spawn(screen()).add_child(room);
    settle(&mut app);

    let items = children(&app, row);
    let box_width = width(&app, items[1]);
    let cell_width = width(&app, items[0]);
    assert!(
        (box_width - 200.0).abs() < 0.5,
        "the picture is its own size: {box_width}"
    );
    assert!(
        cell_width < GooeySkin::dark().text_size * 12.0 - 1.0,
        "the cell took the squeeze: {cell_width}"
    );
    assert!(
        cell_width > 0.0,
        "and is still there, a gap apart from the picture: {cell_width}"
    );
    let gap = rect(&app, items[1]).0 - rect(&app, items[0]).2;
    assert!(
        (gap - gap_px(1.0)).abs() < 0.5,
        "one gap between them: {gap}"
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
