//! The seven layout gaps the editor hit when it retired its hand-built `Node`s (G-84, #95): each
//! constructor laid out for real in a headless window and read back from the drawn rectangles.

use bevy::ecs::relationship::RelatedSpawner;
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
            .spawn(row_with(1.0, RowLayout { align, ..default() }))
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
                wrap: FlexWrap::Wrap,
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
                justify: JustifyContent::End,
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
                align: AlignItems::Center,
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
                align: AlignItems::Center,
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
        .justify = JustifyContent::End;
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
