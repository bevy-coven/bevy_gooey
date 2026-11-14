//! The seven layout gaps the editor hit when it retired its hand-built `Node`s (G-84, #95): each
//! constructor laid out for real in a headless window and read back from the drawn rectangles.

use bevy::ecs::relationship::RelatedSpawner;
use bevy::input_focus::tab_navigation::TabIndex;
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

// ---- 1. layout-aware row and column ----------------------------------------------------------

/// `row_with` carries the cross-axis alignment the editor patched over the spawned bundle: a short
/// box and a tall one share a centre line, and with `FlexStart` they share a top.
#[test]
fn row_with_aligns_its_children_on_the_cross_axis() {
    for (align, centred) in [(AlignItems::Center, true), (AlignItems::FlexStart, false)] {
        let mut app = headless();
        let (short, tall) = (probe(&mut app, 40.0, 20.0), probe(&mut app, 40.0, 60.0));
        let row = app
            .world_mut()
            .spawn(row_with(
                1.0,
                RowLayout {
                    align: Some(align),
                    ..default()
                },
            ))
            .with_children(|row| {
                row.spawn(short);
                row.spawn(tall);
            })
            .id();
        room(&mut app, 400.0, row);
        settle(&mut app);
        let boxes = children(&app, row);
        let (a, b) = (rect(&app, boxes[0]), rect(&app, boxes[1]));
        assert!((a.height() - 20.0).abs() < 0.5 && (b.height() - 60.0).abs() < 0.5);
        if centred {
            assert!(
                (a.center().y - b.center().y).abs() < 0.5,
                "centred on one line: {a:?} {b:?}"
            );
        } else {
            assert!(
                (a.min.y - b.min.y).abs() < 0.5,
                "tops together: {a:?} {b:?}"
            );
        }
    }
}

/// `wrap` puts what does not fit on a line of its own, a skin gap under the first.
#[test]
fn row_with_wraps_what_does_not_fit() {
    let mut app = headless();
    let boxes: Vec<_> = (0..3).map(|_| probe(&mut app, 100.0, 30.0)).collect();
    let row = app
        .world_mut()
        .spawn(row_with(
            1.0,
            RowLayout {
                wrap: Some(FlexWrap::Wrap),
                ..default()
            },
        ))
        .with_children(|row| {
            for b in boxes {
                row.spawn(b);
            }
        })
        .id();
    room(&mut app, 250.0, row);
    settle(&mut app);
    let boxes: Vec<_> = children(&app, row).iter().map(|b| rect(&app, *b)).collect();
    assert!(
        (boxes[0].min.y - boxes[1].min.y).abs() < 0.5,
        "two on the first line"
    );
    assert!(
        boxes[2].min.y > boxes[0].max.y - 0.5,
        "the third wraps below: {boxes:?}"
    );
    let gap = GooeySkin::dark().gap;
    assert!(
        (boxes[2].min.y - boxes[0].max.y - gap).abs() < 0.5,
        "lines stand a skin gap apart: {boxes:?} gap {gap}"
    );
    assert!(
        boxes[2].min.x < boxes[0].min.x + 0.5,
        "the new line starts at the left"
    );
}

/// The same row without `wrap` runs on in one line: that is what `wrap` changes.
#[test]
fn a_row_without_wrap_stays_on_one_line() {
    let mut app = headless();
    let boxes: Vec<_> = (0..3).map(|_| probe(&mut app, 100.0, 30.0)).collect();
    let row = app
        .world_mut()
        .spawn(row(1.0))
        .with_children(|row| {
            for b in boxes {
                row.spawn(b);
            }
        })
        .id();
    room(&mut app, 250.0, row);
    settle(&mut app);
    let boxes: Vec<_> = children(&app, row).iter().map(|b| rect(&app, *b)).collect();
    assert!(
        (boxes[0].min.y - boxes[2].min.y).abs() < 0.5,
        "one line: {boxes:?}"
    );
}

/// `justify` spreads the row's children along its main axis.
#[test]
fn row_with_justifies_along_the_main_axis() {
    let mut app = headless();
    let b = probe(&mut app, 100.0, 30.0);
    let row = app
        .world_mut()
        .spawn(row_with(
            1.0,
            RowLayout {
                justify: Some(JustifyContent::End),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn(b);
        })
        .id();
    let room = room(&mut app, 400.0, row);
    settle(&mut app);
    let at = rect(&app, children(&app, row)[0]);
    let host = rect(&app, room);
    assert!(
        (at.max.x - host.max.x).abs() < 0.5,
        "flush right: {at:?} in {host:?}"
    );
}

/// The layout is a component, so it joins a plain `row(..)`, a bundle position and a
/// `Children::spawn(SpawnWith(..))` the editor's settings screen uses, none of which can take a
/// patched `Node`.
#[test]
fn row_layout_works_in_a_bundle_and_through_a_related_spawner() {
    let mut app = headless();
    let b = probe(&mut app, 40.0, 20.0);
    let tall = probe(&mut app, 40.0, 60.0);
    let row = app
        .world_mut()
        .spawn((
            row(1.0),
            RowLayout {
                align: Some(AlignItems::Center),
                ..default()
            },
            Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
                parent.spawn(b);
                parent.spawn(tall);
            })),
        ))
        .id();
    room(&mut app, 400.0, row);
    settle(&mut app);
    let kids = children(&app, row);
    assert_eq!(kids.len(), 2);
    let (a, b) = (rect(&app, kids[0]), rect(&app, kids[1]));
    assert!(
        (a.center().y - b.center().y).abs() < 0.5,
        "centred: {a:?} {b:?}"
    );
}

/// A column takes the same layout: it centres its children across its width.
#[test]
fn column_with_aligns_across_its_width() {
    let mut app = headless();
    let b = probe(&mut app, 40.0, 20.0);
    let column = app
        .world_mut()
        .spawn(column_with(
            1.0,
            RowLayout {
                align: Some(AlignItems::Center),
                ..default()
            },
        ))
        .with_children(|column| {
            column.spawn(b);
        })
        .id();
    let room = room(&mut app, 400.0, column);
    settle(&mut app);
    let at = rect(&app, children(&app, column)[0]);
    let host = rect(&app, room);
    assert!(
        (at.center().x - host.center().x).abs() < 0.5,
        "centred across: {at:?} in {host:?}"
    );
}

/// A row with no `RowLayout` keeps a `Node` field the host patched itself: the skin writes the
/// layout only when it is asked to.
#[test]
fn a_plain_row_keeps_a_patched_node() {
    let mut app = headless();
    let (short, tall) = (probe(&mut app, 40.0, 20.0), probe(&mut app, 40.0, 60.0));
    let row = app
        .world_mut()
        .spawn(row(1.0))
        .with_children(|row| {
            row.spawn(short);
            row.spawn(tall);
        })
        .id();
    room(&mut app, 400.0, row);
    settle(&mut app);
    app.world_mut()
        .get_mut::<Node>(row)
        .expect("a node")
        .align_items = AlignItems::Center;
    app.world_mut()
        .get_mut::<Role>(row)
        .expect("a role")
        .set_changed();
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(row).expect("a node").align_items,
        AlignItems::Center,
        "a restyle leaves what the host wrote"
    );
}

/// A `RowLayout` changed after the spawn moves the row in the same frame.
#[test]
fn a_changed_row_layout_restyles() {
    let mut app = headless();
    let b = probe(&mut app, 40.0, 20.0);
    let row = app
        .world_mut()
        .spawn(row_with(1.0, RowLayout::default()))
        .with_children(|row| {
            row.spawn(b);
        })
        .id();
    let room = room(&mut app, 400.0, row);
    settle(&mut app);
    assert!(rect(&app, children(&app, row)[0]).min.x < rect(&app, room).min.x + 0.5);
    app.world_mut()
        .get_mut::<RowLayout>(row)
        .expect("a layout")
        .justify = Some(JustifyContent::End);
    settle(&mut app);
    assert!(rect(&app, children(&app, row)[0]).max.x > rect(&app, room).max.x - 0.5);
}

/// `button_row(gap)`: buttons a tight gap apart on one centred line, wrapping when they do not fit.
#[test]
fn button_row_keeps_buttons_a_gap_apart_and_wraps() {
    let mut app = headless();
    let row = app
        .world_mut()
        .spawn(button_row(0.5))
        .with_children(|row| {
            row.spawn(button("PLAY"));
            row.spawn(button("BUILD"));
            row.spawn(button("A button with a rather long name"));
        })
        .id();
    room(&mut app, 4000.0, row);
    settle(&mut app);
    let wide: Vec<_> = children(&app, row).iter().map(|b| rect(&app, *b)).collect();
    let gap = GooeySkin::dark().gap * 0.5;
    assert!(
        ((wide[1].min.x - wide[0].max.x) - gap).abs() < 0.5,
        "half a skin gap between buttons: {wide:?} {gap}"
    );
    assert!(
        (wide[0].min.y - wide[2].min.y).abs() < 0.5,
        "one line when they fit"
    );

    let mut app = headless();
    let row = app
        .world_mut()
        .spawn(button_row(0.5))
        .with_children(|row| {
            row.spawn(button("PLAY"));
            row.spawn(button("BUILD"));
            row.spawn(button("A button with a rather long name"));
        })
        .id();
    room(&mut app, 220.0, row);
    settle(&mut app);
    let narrow: Vec<_> = children(&app, row).iter().map(|b| rect(&app, *b)).collect();
    assert!(
        narrow[2].min.y > narrow[0].max.y - 0.5,
        "the last button wraps below: {narrow:?}"
    );
}

// ---- 2. a growing column that holds text and can narrow to zero ------------------------------

const WORDS: &str = "A long line of words that has to wrap inside the column it stands in";

/// A `room`-wide row of a fixed 100 px picture and a `fill_column` holding `WORDS`.
fn fill_row(app: &mut App, width: f32) -> (Entity, Entity, Entity) {
    let picture = probe(app, 100.0, 30.0);
    let world = app.world_mut();
    let words = world.spawn(label(WORDS)).id();
    let column = world.spawn(fill_column(0.5)).add_child(words).id();
    let row = world
        .spawn(row(1.0))
        .with_children(|row| {
            row.spawn(picture);
        })
        .add_child(column)
        .id();
    room(app, width, row);
    (row, column, words)
}

/// The column takes the room its row leaves, less the skin gap beside the picture.
#[test]
fn fill_column_takes_what_its_row_leaves() {
    let mut app = headless();
    let (_, column, _) = fill_row(&mut app, 400.0);
    settle(&mut app);
    let gap = GooeySkin::dark().gap;
    assert!(
        (rect(&app, column).width() - (400.0 - 100.0 - gap)).abs() < 0.5,
        "grows into the room: {:?}",
        rect(&app, column)
    );
}

/// With less room than the picture alone needs the column narrows to nothing and the row stays as
/// wide as its host: a text with no least width would push it out.
#[test]
fn fill_column_narrows_to_zero_without_pushing_its_row_out() {
    let mut app = headless();
    let (row, column, _) = fill_row(&mut app, 60.0);
    settle(&mut app);
    assert!(
        rect(&app, column).width() < 0.5,
        "no width left: {:?}",
        rect(&app, column)
    );
    assert!(
        rect(&app, row).width() < 60.5,
        "the row is no wider than its host: {:?}",
        rect(&app, row)
    );
}

/// The text wraps inside the column: narrower room, more lines.
#[test]
fn fill_column_wraps_its_text_inside_itself() {
    let mut wide = headless();
    let (_, _, words) = fill_row(&mut wide, 1000.0);
    settle(&mut wide);
    let mut narrow = headless();
    let (_, _, words_narrow) = fill_row(&mut narrow, 260.0);
    settle(&mut narrow);
    let (one_line, wrapped) = (
        rect(&wide, words).height(),
        rect(&narrow, words_narrow).height(),
    );
    assert!(
        wrapped > one_line * 1.5,
        "wraps: {one_line} high wide, {wrapped} narrow"
    );
    assert!(rect(&narrow, words_narrow).width() <= 260.0 - 100.0 - GooeySkin::dark().gap + 0.5);
}

/// Its children stand a skin gap apart, as a `column`'s do.
#[test]
fn fill_column_keeps_a_gap_between_its_children() {
    let mut app = headless();
    let (a, b) = (
        app.world_mut().spawn(label("one")).id(),
        app.world_mut().spawn(label("two")).id(),
    );
    let column = app
        .world_mut()
        .spawn(fill_column(1.0))
        .add_child(a)
        .add_child(b)
        .id();
    room(&mut app, 300.0, column);
    settle(&mut app);
    let gap = rect(&app, b).min.y - rect(&app, a).max.y;
    assert!(
        (gap - GooeySkin::dark().gap).abs() < 0.5,
        "a skin gap apart: {gap}"
    );
}

// ---- 3. a vertical region that grows and can shrink to zero ----------------------------------

/// A `height`-tall host holding a fixed 50 px strip and a `fill_region` around a 1000 px box, which
/// stands for a tree that is taller than the room.
fn region_host(app: &mut App, height: f32, layout: Option<RowLayout>) -> (Entity, Entity, Entity) {
    let strip = probe(app, 100.0, 50.0);
    let tall = probe(app, 100.0, 1000.0);
    let world = app.world_mut();
    let inside = world.spawn(tall).id();
    let region = world.spawn(fill_region()).add_child(inside).id();
    if let Some(layout) = layout {
        world.entity_mut(region).insert(layout);
    }
    let host = world
        .spawn(Node {
            width: Val::Px(300.0),
            height: Val::Px(height),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|host| {
            host.spawn(strip);
        })
        .add_child(region)
        .id();
    world.spawn(screen()).add_child(host);
    (host, region, inside)
}

/// The region takes the height the strip above it leaves, not its content's.
#[test]
fn fill_region_takes_the_height_its_host_leaves() {
    let mut app = headless();
    let (host, region, _) = region_host(&mut app, 300.0, None);
    settle(&mut app);
    assert!(
        (rect(&app, region).height() - 250.0).abs() < 0.5,
        "{:?}",
        rect(&app, region)
    );
    assert!(
        (rect(&app, region).width() - 300.0).abs() < 0.5,
        "stretches across"
    );
    assert!(
        (rect(&app, host).height() - 300.0).abs() < 0.5,
        "the host keeps its height"
    );
}

/// Under less room than the strip alone the region shrinks to nothing.
#[test]
fn fill_region_shrinks_to_zero() {
    let mut app = headless();
    let (host, region, _) = region_host(&mut app, 40.0, None);
    settle(&mut app);
    assert!(
        rect(&app, region).height() < 0.5,
        "{:?}",
        rect(&app, region)
    );
    assert!(
        (rect(&app, host).height() - 40.0).abs() < 0.5,
        "the host keeps its height"
    );
}

/// A `RowLayout` on the region centres what it holds: loading's centred growing cell.
#[test]
fn fill_region_centres_what_it_holds_with_a_row_layout() {
    let mut app = headless();
    let strip = probe(&mut app, 100.0, 50.0);
    let small = probe(&mut app, 40.0, 20.0);
    let world = app.world_mut();
    let inside = world.spawn(small).id();
    let region = world
        .spawn((
            fill_region(),
            RowLayout {
                align: Some(AlignItems::Center),
                justify: Some(JustifyContent::Center),
                ..default()
            },
        ))
        .add_child(inside)
        .id();
    let host = world
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
        .id();
    world.spawn(screen()).add_child(host);
    settle(&mut app);
    let (region, inside) = (rect(&app, region), rect(&app, inside));
    assert!(
        (inside.center() - region.center()).length() < 0.5,
        "{inside:?} in {region:?}"
    );
}

// ---- 4. a card with a width of its own -------------------------------------------------------

#[derive(Component)]
struct Card;

/// A dialog whose panel is `sized_panel(width)`, under `skin`; the panel's rectangle.
fn dialog_of(skin: GooeySkin, width: CardWidth) -> (App, Entity) {
    let mut app = headless();
    app.insert_resource(skin);
    let card = app
        .world_mut()
        .spawn((
            dialog(),
            children![((sized_panel(width), Card), children![label("Sure?")])],
        ))
        .id();
    settle(&mut app);
    let card = children(&app, card)[0];
    (app, card)
}

/// A width in logical px is that many px, whatever the text size: past the 21 to 36 text size
/// measure a dialog's panel otherwise has.
#[test]
fn a_dialog_card_can_be_a_width_in_px() {
    for width in [300.0, 520.0, 900.0] {
        let (app, card) = dialog_of(GooeySkin::dark(), CardWidth::Px(width));
        assert!(
            (rect(&app, card).width() - width).abs() < 1.0,
            "{width}: {:?}",
            rect(&app, card)
        );
    }
}

/// A width in text sizes follows the player's text size.
#[test]
fn a_dialog_card_can_be_a_width_in_text_sizes() {
    for scale in [1.0, 1.5] {
        let skin = GooeySkin {
            text_scale: scale,
            ..GooeySkin::dark()
        };
        let want = 30.0 * skin.text_size * scale;
        let (app, card) = dialog_of(skin, CardWidth::Text(30.0));
        assert!(
            (rect(&app, card).width() - want).abs() < 1.0,
            "x{scale}: {:?} wanted {want}",
            rect(&app, card)
        );
    }
}

/// Never wider than the room the scrim leaves, the window less its 5% on each side: `max_width:
/// 100%`.
#[test]
fn a_dialog_card_never_outgrows_the_window() {
    let (app, card) = dialog_of(GooeySkin::dark(), CardWidth::Px(5000.0));
    let width = rect(&app, card).width();
    assert!(
        (width - 1280.0 * 0.9).abs() < 1.5,
        "capped at the scrim's inside: {width}"
    );
}

/// A card narrower than the dialog's least measure takes the width it was given.
#[test]
fn a_dialog_card_can_be_narrower_than_the_least_measure() {
    let skin = GooeySkin::dark();
    let (app, card) = dialog_of(skin.clone(), CardWidth::Text(12.0));
    assert!(
        (rect(&app, card).width() - 12.0 * skin.text_size).abs() < 1.0,
        "{:?}",
        rect(&app, card)
    );
}

/// A width on a panel that is not a dialog's sizes it too (a form on a screen).
#[test]
fn a_sized_panel_on_a_screen_takes_its_width() {
    let mut app = headless();
    let card = app
        .world_mut()
        .spawn((sized_panel(CardWidth::Px(420.0)), children![label("Name")]))
        .id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);
    assert!(
        (rect(&app, card).width() - 420.0).abs() < 1.0,
        "{:?}",
        rect(&app, card)
    );
}

/// A changed width moves the card in the same frame.
#[test]
fn a_changed_card_width_restyles() {
    let (mut app, card) = dialog_of(GooeySkin::dark(), CardWidth::Px(400.0));
    *app.world_mut().get_mut::<CardWidth>(card).expect("a width") = CardWidth::Px(600.0);
    settle(&mut app);
    assert!(
        (rect(&app, card).width() - 600.0).abs() < 1.0,
        "{:?}",
        rect(&app, card)
    );
}

// ---- 5. absolutely placed nodes --------------------------------------------------------------

/// A 400 × 300 host holding a 100 × 50 flow box and `placed`.
fn placed_host(app: &mut App, placed: impl Bundle, inside: Entity) -> (Entity, Entity, Entity) {
    let flow = probe(app, 100.0, 50.0);
    let world = app.world_mut();
    let placed = world.spawn(placed).add_child(inside).id();
    let host = world
        .spawn(Node {
            width: Val::Px(400.0),
            height: Val::Px(300.0),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|host| {
            host.spawn(flow);
        })
        .add_child(placed)
        .id();
    let first = children(app, host)[0];
    app.world_mut().spawn(screen()).add_child(host);
    (host, first, placed)
}

/// `placed_at(left, top)` puts a node that many px in from its host's top left corner and takes no
/// room of its own: the host's flow child stays where it was.
#[test]
fn placed_at_puts_a_node_at_left_and_top_and_takes_no_room() {
    let mut app = headless();
    let mark = app.world_mut().spawn(label("area")).id();
    let (host, flow, placed) = placed_host(&mut app, placed_at(30.0, 40.0), mark);
    settle(&mut app);
    let (host, flow, placed) = (rect(&app, host), rect(&app, flow), rect(&app, placed));
    assert!(
        (placed.min - (host.min + Vec2::new(30.0, 40.0))).length() < 0.5,
        "{placed:?} in {host:?}"
    );
    assert!(
        (flow.min - host.min).length() < 0.5,
        "the flow child is not pushed: {flow:?}"
    );
}

/// A node placed by its right and bottom edges stands that far in from the far corner.
#[test]
fn placed_can_hold_a_corner() {
    let mut app = headless();
    let mark = app.world_mut().spawn(label("N")).id();
    let inset = UiRect {
        right: Val::Px(10.0),
        bottom: Val::Px(20.0),
        ..UiRect::all(Val::Auto)
    };
    let (host, _, placed) = placed_host(&mut app, placed(inset), mark);
    settle(&mut app);
    let (host, placed) = (rect(&app, host), rect(&app, placed));
    assert!(
        (placed.max.x - (host.max.x - 10.0)).abs() < 0.5,
        "{placed:?} in {host:?}"
    );
    assert!(
        (placed.max.y - (host.max.y - 20.0)).abs() < 0.5,
        "{placed:?} in {host:?}"
    );
}

/// `cover()` fills its host, whatever size that is: a full-viewport overlay.
#[test]
fn cover_fills_its_host() {
    let mut app = headless();
    let mark = app.world_mut().spawn(label("Nothing here yet")).id();
    let (host, _, cover_node) = placed_host(&mut app, cover(), mark);
    settle(&mut app);
    let (host, cover_rect) = (rect(&app, host), rect(&app, cover_node));
    assert!(
        (cover_rect.min - host.min).length() < 0.5 && (cover_rect.max - host.max).length() < 0.5,
        "{cover_rect:?} over {host:?}"
    );
}

/// A placed node is a mark, not a surface: it carries `Pickable::IGNORE`, like a `screen()`. This
/// reads the component, not a picking run: that the pointer then goes through is Bevy's picking and
/// `screen()`'s own tests' to hold.
#[test]
fn a_placed_node_is_marked_to_let_the_pointer_through() {
    let mut app = headless();
    let a = app.world_mut().spawn(placed_at(1.0, 2.0)).id();
    let b = app.world_mut().spawn(cover()).id();
    for e in [a, b] {
        assert_eq!(app.world().get::<Pickable>(e), Some(&Pickable::IGNORE));
    }
}

/// A changed inset moves the node in the same frame.
#[test]
fn a_changed_inset_restyles() {
    let mut app = headless();
    let mark = app.world_mut().spawn(label("area")).id();
    let (host, _, placed) = placed_host(&mut app, placed_at(30.0, 40.0), mark);
    settle(&mut app);
    app.world_mut()
        .get_mut::<PlacedAt>(placed)
        .expect("an inset")
        .0
        .left = Val::Px(80.0);
    settle(&mut app);
    let (host, placed) = (rect(&app, host), rect(&app, placed));
    assert!(
        (placed.min.x - (host.min.x + 80.0)).abs() < 0.5,
        "{placed:?} in {host:?}"
    );
}

/// A node that follows a point in the world (a balloon over a thing, a label over a map area, a
/// badge on a mesh) is already a `WorldAnchor` on any bundle with a `Node`: its foot stands on the
/// projected point. Documented, not rebuilt.
#[test]
fn a_node_on_a_world_point_is_a_world_anchor() {
    let mut app = headless();
    let target = app
        .world_mut()
        .spawn(Transform::from_translation(Vec3::new(100.0, 50.0, 0.0)))
        .id();
    let balloon = app
        .world_mut()
        .spawn((
            panel(),
            children![label("Hello")],
            WorldAnchor::above(target, 0.0),
        ))
        .id();
    let badge = app
        .world_mut()
        .spawn((
            label("3"),
            WorldAnchor::new(target).pivot(Vec2::new(0.0, 0.0)),
        ))
        .id();
    settle(&mut app);
    let at = rect(&app, balloon);
    // The camera is at the origin looking at a 1280 × 720 window: world (100, 50) is (740, 310).
    assert!(
        (at.center().x - 740.0).abs() < 1.0 && (at.max.y - 310.0).abs() < 1.0,
        "{at:?}"
    );
    let at = rect(&app, badge);
    assert!((at.min - Vec2::new(740.0, 310.0)).length() < 1.0, "{at:?}");
}

// ---- 6. a bare Role node ---------------------------------------------------------------------

#[derive(Resource, Default)]
struct Activated(u32);

/// A `status_mark()` is lit by the `StatusKind` of the node it stands in: a dot for a warning or an
/// error, nothing for plain information; it is not pickable.
#[test]
fn a_status_mark_follows_the_kind_of_its_parent() {
    let mut app = headless();
    let mark = app.world_mut().spawn(status_mark()).id();
    let row = app
        .world_mut()
        .spawn((row(1.0), StatusKind::Error, children![label("Failed")]))
        .add_child(mark)
        .id();
    app.world_mut().spawn(screen()).add_child(row);
    settle(&mut app);
    let shown = |app: &App| app.world().get::<Node>(mark).expect("a node").display;
    let ground = |app: &App| app.world().get::<BackgroundColor>(mark).expect("a fill").0;
    assert_eq!(shown(&app), Display::Flex, "an error shows its mark");
    assert_eq!(ground(&app), GooeySkin::dark().danger);
    assert!(rect(&app, mark).width() > 0.5);
    assert_eq!(app.world().get::<Pickable>(mark), Some(&Pickable::IGNORE));

    *app.world_mut().get_mut::<StatusKind>(row).expect("a kind") = StatusKind::Warning;
    settle(&mut app);
    assert_eq!(shown(&app), Display::Flex);
    assert_eq!(
        ground(&app),
        GooeySkin::dark().accent,
        "a warning is the accent"
    );

    *app.world_mut().get_mut::<StatusKind>(row).expect("a kind") = StatusKind::Info;
    settle(&mut app);
    assert_eq!(
        shown(&app),
        Display::None,
        "plain information shows no mark"
    );
}

/// `plain_button` has a button's look (its fill, its frame, its height) and none of its behaviour:
/// no Bevy `Button`, so no `Activate` and no tab stop; a real button beside it is the control.
#[test]
fn a_plain_button_looks_like_a_button_and_stays_off_the_activate_bus() {
    let mut app = headless();
    app.init_resource::<Activated>();
    let count = |_: On<Activate>, mut n: ResMut<Activated>| n.0 += 1;
    let plain = app
        .world_mut()
        .spawn((plain_button("Cancel"), observe(count)))
        .id();
    let real = app
        .world_mut()
        .spawn((button("Cancel"), observe(count)))
        .id();
    let holder = app
        .world_mut()
        .spawn(row(1.0))
        .add_child(plain)
        .add_child(real)
        .id();
    app.world_mut().spawn(screen()).add_child(holder);
    settle(&mut app);

    assert!(app.world().get::<bevy::ui_widgets::Button>(plain).is_none());
    assert!(
        app.world().get::<TabIndex>(plain).is_none(),
        "nothing for Tab to do on it"
    );
    assert_eq!(app.world().get::<Role>(plain), Some(&Role::Button));
    let (a, b) = (rect(&app, plain), rect(&app, real));
    assert!(
        (a.height() - b.height()).abs() < 0.5,
        "a button's height: {a:?} {b:?}"
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(plain).map(|c| c.0),
        app.world().get::<BackgroundColor>(real).map(|c| c.0),
        "the fill is a button's"
    );
    let words = children(&app, plain);
    assert_eq!(
        app.world().get::<Text>(words[0]).map(|t| t.0.as_str()),
        Some("Cancel")
    );

    bevy_gooey::testing::click(&mut app, plain);
    settle(&mut app);
    assert_eq!(
        app.world().resource::<Activated>().0,
        0,
        "a plain button never sends Activate"
    );
    bevy_gooey::testing::click(&mut app, real);
    settle(&mut app);
    assert_eq!(
        app.world().resource::<Activated>().0,
        1,
        "a real button does: the control"
    );
}

// ---- 7. an image sized by its aspect ratio with a maximum width ------------------------------

/// A splash: 300 px wide at an aspect of 2 (width over height) while its host has the room, and no
/// wider than the host when it does not, its height following.
#[test]
fn an_aspect_image_is_its_width_and_keeps_its_ratio_when_the_host_is_narrower() {
    for (host, want) in [(400.0, (300.0, 150.0)), (150.0, (150.0, 75.0))] {
        let mut app = headless();
        let image = image(&mut app);
        let splash = app.world_mut().spawn(aspect_image(image, 300.0, 2.0)).id();
        room(&mut app, host, splash);
        settle(&mut app);
        let got = rect(&app, splash);
        assert!(
            (got.width() - want.0).abs() < 0.5 && (got.height() - want.1).abs() < 0.5,
            "in {host}: {got:?}, wanted {want:?}"
        );
    }
}

/// A picture is not a surface: it takes no pointer, as every picture in the crate.
#[test]
fn an_aspect_image_takes_no_pointer() {
    let mut app = headless();
    let image = image(&mut app);
    let splash = app.world_mut().spawn(aspect_image(image, 300.0, 2.0)).id();
    assert_eq!(app.world().get::<Pickable>(splash), Some(&Pickable::IGNORE));
}

/// A changed width moves it in the same frame.
#[test]
fn a_changed_aspect_box_restyles() {
    let mut app = headless();
    let image = image(&mut app);
    let splash = app.world_mut().spawn(aspect_image(image, 300.0, 2.0)).id();
    room(&mut app, 600.0, splash);
    settle(&mut app);
    app.world_mut()
        .get_mut::<AspectBox>(splash)
        .expect("a box")
        .width = 400.0;
    settle(&mut app);
    let got = rect(&app, splash);
    assert!(
        (got.width() - 400.0).abs() < 0.5 && (got.height() - 200.0).abs() < 0.5,
        "{got:?}"
    );
}

// ---- review of the first cut: shrink, button_row + RowLayout, non-finite widths ----------------

/// `shrink` is the node's `flex_shrink`: a `row_with` that sets it is rigid in a row that squeezes,
/// and a plain `row(1.0)` beside it is untouched and still gives way.
#[test]
fn row_with_shrink_makes_a_row_rigid_and_a_plain_row_is_untouched() {
    let mut app = headless();
    let (a, b) = (probe(&mut app, 150.0, 20.0), probe(&mut app, 150.0, 20.0));
    let world = app.world_mut();
    let rigid = world
        .spawn(row_with(
            0.0,
            RowLayout {
                shrink: Some(0.0),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn(a);
        })
        .id();
    let plain = world
        .spawn(row(0.0))
        .with_children(|row| {
            row.spawn(b);
        })
        .id();
    let host = world
        .spawn(Node {
            width: Val::Px(200.0),
            min_width: Val::Px(200.0),
            max_width: Val::Px(200.0),
            flex_shrink: 0.0,
            ..default()
        })
        .add_child(rigid)
        .add_child(plain)
        .id();
    world.spawn(screen()).add_child(host);
    settle(&mut app);
    let shrink = |e| app.world().get::<Node>(e).expect("a node").flex_shrink;
    assert_eq!(shrink(rigid), 0.0, "the layout wrote flex_shrink");
    assert_eq!(shrink(plain), 1.0, "a plain row is untouched");
    assert!(
        (rect(&app, rigid).width() - 150.0).abs() < 0.5,
        "rigid keeps its width: {:?}",
        rect(&app, rigid)
    );
    assert!(
        (rect(&app, plain).width() - 50.0).abs() < 0.5,
        "the plain row gives way: {:?}",
        rect(&app, plain)
    );
}

/// The same field on a column.
#[test]
fn column_with_shrink_writes_flex_shrink() {
    let mut app = headless();
    let column = app
        .world_mut()
        .spawn(column_with(
            1.0,
            RowLayout {
                shrink: Some(0.0),
                ..default()
            },
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(column);
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(column).expect("a node").flex_shrink,
        0.0
    );
}

/// A field left `None` leaves the role's own value, so `justify` on a `button_row` keeps its
/// centring and its wrapping.
#[test]
fn a_row_layout_beside_a_button_row_changes_only_what_it_sets() {
    let mut app = headless();
    let row = app
        .world_mut()
        .spawn((
            button_row(0.5),
            RowLayout {
                justify: Some(JustifyContent::End),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn(button("PLAY"));
            row.spawn(button("BUILD"));
            row.spawn(button("A button with a rather long name"));
        })
        .id();
    let host = room(&mut app, 220.0, row);
    settle(&mut app);
    let node = app.world().get::<Node>(row).expect("a node");
    assert_eq!(node.align_items, AlignItems::Center, "still centred");
    assert_eq!(node.flex_wrap, FlexWrap::Wrap, "still wraps");
    assert_eq!(node.justify_content, JustifyContent::End);
    let last = rect(&app, children(&app, row)[2]);
    assert!(
        last.min.y > rect(&app, children(&app, row)[0]).max.y - 0.5,
        "the last button wraps"
    );
    assert!(
        (last.max.x - rect(&app, host).max.x).abs() < 0.5,
        "flush right: {last:?}"
    );
}

/// A width that is not a finite number is refused: the card keeps the dialog's own measure.
#[test]
fn a_card_width_that_is_not_finite_is_refused() {
    let skin = GooeySkin::dark();
    let measure = 21.0 * skin.text_size;
    for width in [
        CardWidth::Px(f32::NAN),
        CardWidth::Px(f32::INFINITY),
        CardWidth::Px(f32::NEG_INFINITY),
        CardWidth::Text(f32::NAN),
        CardWidth::Text(f32::INFINITY),
    ] {
        let (app, card) = dialog_of(skin.clone(), width);
        assert!(
            (rect(&app, card).width() - measure).abs() < 1.0,
            "{width:?}: {:?} wanted the measure {measure}",
            rect(&app, card)
        );
    }
}
