//! The Markdown view (#90, G-79): a page of CommonMark with tables, strikethrough and task lists,
//! parsed once with `pulldown-cmark` and drawn in the skin's type scale, laid out for real in a
//! headless window. Every block kind is found by its words, a click on a link sends
//! [`MarkdownLink`], a picture that does not load is a box naming its path, `markdown::images`
//! lists what the view draws in the order it draws it, a path that leaves the base is refused and
//! reported, and a page is built once and rebuilt only when its source changes.
#![cfg(feature = "markdown")]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedNode, ScrollPosition, UiGlobalTransform};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::markdown::{self, Block, Inline, LinkKind, PathRefused};
use bevy_gooey::testing::{click, find_by_label, press};
use bevy_gooey::*;

/// One page with every block kind the view draws, in the order a style guide writes them.
const PAGE: &str = "\
# Style guide

Intro with **bold**, *slanted*, ~~struck~~ and `code` words.

## Palette

See [the palette](palette.md), [the web](https://example.com/x) and [jump](#sizes).

![Swatch](img/swatch.png)

| Name | Hex |
|------|-----|
| Ink | #111111 |
| Paper | #eeeeee |

- first item
- second item
  - nested item

1. step one
2. step two

- [x] done task
- [ ] open task

> a quoted line

---

```rust
fn main() {}
```

<div>raw html</div>

![Missing](img/missing.png)

![Escapes](../outside.png)

### Sizes

| Picture | Note |
|---------|------|
| ![Cell picture](img/cell.png) | beside it |

Closing words.
";

/// The files the page's resolver knows: path to image, and every path it was asked for.
#[derive(Clone, Default)]
struct Shelf {
    known: Arc<Mutex<BTreeMap<String, Handle<Image>>>>,
    asked: Arc<Mutex<Vec<String>>>,
}

impl Shelf {
    fn base(&self) -> MarkdownBase {
        let known = self.known.clone();
        let asked = self.asked.clone();
        MarkdownBase::new("guide", move |path: &str| {
            asked.lock().expect("the shelf").push(path.to_owned());
            known.lock().expect("the shelf").get(path).cloned()
        })
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("the shelf").clone()
    }
}

fn app() -> App {
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

/// A shelf holding the page's three pictures, each a real image in the app's assets.
fn shelf(app: &mut App) -> Shelf {
    let shelf = Shelf::default();
    for path in [
        "guide/img/swatch.png",
        "guide/img/cell.png",
        "guide/img/icon.png",
    ] {
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        shelf
            .known
            .lock()
            .expect("the shelf")
            .insert(path.to_owned(), handle);
    }
    shelf
}

/// A view of `source` in a host box 640 by `height` px, laid out.
fn page(app: &mut App, shelf: &Shelf, source: &str, height: f32) -> Entity {
    let host = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(640.0),
            height: Val::Px(height),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let view = app
        .world_mut()
        .spawn((markdown_view(source, shelf.base()), ChildOf(host)))
        .id();
    settle(app);
    view
}

/// Every `Text` below `root`, depth first, as its words.
fn words_below(app: &App, root: Entity) -> Vec<String> {
    let world = app.world();
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(text) = world.get::<Text>(entity) {
            out.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().rev());
        }
    }
    out
}

/// The entities below `root` carrying component `T`, depth first (the order the view draws).
fn drawn_in_order<T: Component>(app: &App, root: Entity) -> Vec<Entity> {
    let world = app.world();
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if world.get::<T>(entity).is_some() {
            out.push(entity);
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().rev());
        }
    }
    out
}

#[test]
fn a_page_with_every_block_kind_is_found_by_its_words() {
    let mut app = app();
    let shelf = shelf(&mut app);
    let view = page(&mut app, &shelf, PAGE, 6000.0);

    // Headings, cells and links, each by the words a reader sees.
    for words in [
        "Style guide",
        "Palette",
        "Sizes",
        "Name",
        "Hex",
        "Ink",
        "#111111",
        "Paper",
        "#eeeeee",
        "Picture",
        "Note",
        "beside it",
        "the palette",
        "the web",
        "jump",
    ] {
        if let Err(error) = find_by_label(&mut app, words) {
            panic!("the page does not show \"{words}\": {error}");
        }
    }

    // Plain prose, list items, a task, a quote, a rule, a fence and raw HTML as text.
    let words = words_below(&app, view);
    for wanted in [
        "Closing words.",
        "first item",
        "second item",
        "nested item",
        "step one",
        "step two",
        "done task",
        "open task",
        "a quoted line",
        "fn main() {}",
        "<div>raw html</div>",
    ] {
        assert!(
            words.iter().any(|w| w.contains(wanted)),
            "the page does not show \"{wanted}\": {words:?}"
        );
    }
    assert_eq!(
        drawn_in_order::<MarkdownHeading>(&app, view).len(),
        3,
        "three headings are drawn"
    );
    // Four pictures are drawn as pictures: the page's and the cell's, and the missing one, the one
    // that escapes the base: two boxes that name their paths.
    assert_eq!(drawn_in_order::<MarkdownPicture>(&app, view).len(), 4);
    // The page's list and its structure.
    let roles: Vec<Role> = {
        let world = app.world();
        let mut out = Vec::new();
        let mut stack = vec![view];
        while let Some(entity) = stack.pop() {
            if let Some(role) = world.get::<Role>(entity) {
                out.push(*role);
            }
            if let Some(children) = world.get::<Children>(entity) {
                stack.extend(children.iter());
            }
        }
        out
    };
    for role in [
        Role::MarkdownTable,
        Role::MarkdownQuote,
        Role::MarkdownRule,
        Role::MarkdownCode,
        Role::MarkdownLink,
    ] {
        assert!(roles.contains(&role), "no {role:?} is drawn");
    }
}

#[test]
fn clicking_a_link_sends_its_destination_and_the_view_opens_nothing() {
    let mut app = app();
    let shelf = shelf(&mut app);
    let view = page(&mut app, &shelf, PAGE, 600.0);
    let heard: Arc<Mutex<Vec<(String, LinkKind)>>> = Arc::default();
    let sink = heard.clone();
    app.world_mut()
        .entity_mut(view)
        .observe(move |link: On<MarkdownLink>| {
            sink.lock()
                .expect("the sink")
                .push((link.dest.clone(), link.kind));
        });

    for (words, dest, kind) in [
        ("the palette", "palette.md", LinkKind::Relative),
        ("the web", "https://example.com/x", LinkKind::Web),
    ] {
        let link = find_by_label(&mut app, words).expect("the link is drawn");
        click(&mut app, link);
        let last = heard.lock().expect("the sink").last().cloned();
        assert_eq!(
            last,
            Some((dest.to_owned(), kind)),
            "clicking \"{words}\" sends its destination"
        );
    }
    assert_eq!(
        heard.lock().expect("the sink").len(),
        2,
        "one event a click"
    );

    let viewport = drawn_in_order::<MarkdownViewport>(&app, view)[0];
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).map(|s| s.y),
        Some(0.0)
    );
    let anchor = find_by_label(&mut app, "jump").expect("the anchor link is drawn");
    click(&mut app, anchor);
    assert_eq!(
        heard.lock().expect("the sink").last().cloned(),
        Some(("#sizes".to_owned(), LinkKind::Anchor))
    );
    // An anchor is the page's own: the view also scrolls to the heading it names.
    settle(&mut app);
    assert!(
        app.world()
            .get::<ScrollPosition>(viewport)
            .is_some_and(|s| s.y > 0.0),
        "the anchor scrolled the page"
    );
}

#[test]
fn a_link_is_pressed_from_the_keyboard() {
    let mut app = app();
    let shelf = shelf(&mut app);
    let view = page(&mut app, &shelf, PAGE, 600.0);
    let heard: Arc<Mutex<Vec<String>>> = Arc::default();
    let sink = heard.clone();
    app.world_mut()
        .entity_mut(view)
        .observe(move |link: On<MarkdownLink>| {
            sink.lock().expect("the sink").push(link.dest.clone());
        });
    let link = find_by_label(&mut app, "the palette").expect("the link is drawn");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(link, FocusCause::Navigated);
    press(&mut app, KeyCode::Enter);
    assert_eq!(heard.lock().expect("the sink").as_slice(), ["palette.md"]);
}

#[test]
fn a_picture_whose_image_fails_to_load_becomes_a_box_naming_its_path() {
    let mut app = app();
    // A handle to an image that no loader has and the asset server has never heard of: it is
    // neither in the assets nor loading, which is what a failed load reads as too
    // (`LoadState::Failed` and `NotLoaded` are the same box). A real file that is missing takes the
    // same path; an app that is not given its async loaders never gets as far as `Failed`, which is
    // why this test does not wait for a file.
    let handle: Handle<Image> = app.world().resource::<Assets<Image>>().reserve_handle();
    let base = MarkdownBase::new("guide", move |_: &str| Some(handle.clone()));
    let host = app.world_mut().spawn(Node::default()).id();
    let view = app
        .world_mut()
        .spawn((markdown_view("![Gone](nowhere.png)\n", base), ChildOf(host)))
        .id();
    let missing = |app: &App| {
        drawn_in_order::<MarkdownPicture>(app, view)
            .into_iter()
            .find(|e| app.world().get::<Role>(*e) == Some(&Role::MarkdownMissing))
    };
    // The box shows the picture's size first, then the box that names its path.
    let mut found = None;
    for _ in 0..10 {
        app.update();
        found = missing(&app);
        if found.is_some() {
            break;
        }
    }
    let found = found.expect("a picture that failed to load is a box that names its path");
    let words = words_below(&app, found);
    assert!(
        words.iter().any(|w| w == "Gone") && words.iter().any(|w| w == "guide/nowhere.png"),
        "{words:?}"
    );
    assert!(app.world().get::<ImageNode>(found).is_none());
}

#[test]
fn a_picture_that_is_not_there_is_a_box_naming_its_path() {
    let mut app = app();
    let shelf = shelf(&mut app);
    let view = page(&mut app, &shelf, PAGE, 6000.0);

    let boxes = drawn_in_order::<MarkdownPicture>(&app, view);
    let missing: Vec<Entity> = boxes
        .iter()
        .copied()
        .filter(|e| app.world().get::<Role>(*e) == Some(&Role::MarkdownMissing))
        .collect();
    assert_eq!(missing.len(), 2, "the missing one and the one that escapes");
    let first = words_below(&app, missing[0]);
    assert!(
        first.iter().any(|w| w == "Missing") && first.iter().any(|w| w == "guide/img/missing.png"),
        "the box names its alt text and its path: {first:?}"
    );
    // It is a box of a size, not nothing: it holds its room.
    let node = app
        .world()
        .get::<ComputedNode>(missing[0])
        .expect("laid out");
    assert!(node.size().x > 0.0 && node.size().y > 0.0);
    // The found ones are drawn as pictures.
    let drawn = boxes
        .iter()
        .filter(|e| app.world().get::<Role>(**e) == Some(&Role::ImageBox))
        .count();
    assert_eq!(drawn, 2, "the swatch and the cell's picture");
}

#[test]
fn images_lists_the_pictures_in_the_order_the_view_draws_them() {
    let mut app = app();
    let shelf = shelf(&mut app);
    let view = page(&mut app, &shelf, PAGE, 6000.0);

    let listed = markdown::images(PAGE);
    assert_eq!(
        listed
            .iter()
            .map(|(_, dest, alt)| (dest.as_str(), alt.as_str()))
            .collect::<Vec<_>>(),
        [
            ("img/swatch.png", "Swatch"),
            ("img/missing.png", "Missing"),
            ("../outside.png", "Escapes"),
            ("img/cell.png", "Cell picture"),
        ]
    );
    // Each range is the picture's own text in the source.
    for (range, dest, _) in &listed {
        assert!(PAGE[range.clone()].contains(dest.as_str()));
    }
    let drawn: Vec<(String, String)> = drawn_in_order::<MarkdownPicture>(&app, view)
        .into_iter()
        .filter_map(|e| app.world().get::<MarkdownPicture>(e))
        .map(|p| (p.dest().to_owned(), p.alt().to_owned()))
        .collect();
    assert_eq!(
        drawn,
        listed
            .iter()
            .map(|(_, d, a)| (d.clone(), a.clone()))
            .collect::<Vec<_>>(),
        "the view draws what images() lists, in that order"
    );
}

#[test]
fn a_path_that_leaves_the_base_is_not_loaded_and_is_reported() {
    let mut app = app();
    let shelf = shelf(&mut app);
    let refused: Arc<Mutex<Vec<(String, PathRefused)>>> = Arc::default();
    let host = app.world_mut().spawn(Node::default()).id();
    let view = app
        .world_mut()
        .spawn((markdown_view(PAGE, shelf.base()), ChildOf(host)))
        .id();
    let sink = refused.clone();
    app.world_mut()
        .entity_mut(view)
        .observe(move |event: On<MarkdownImageRefused>| {
            sink.lock()
                .expect("the sink")
                .push((event.dest.clone(), event.reason));
        });
    settle(&mut app);

    assert_eq!(
        refused.lock().expect("the sink").as_slice(),
        [("../outside.png".to_owned(), PathRefused::EscapesBase)]
    );
    assert!(
        !shelf.asked().iter().any(|p| p.contains("outside")),
        "the resolver was never asked for it: {:?}",
        shelf.asked()
    );
    // The paths it was asked for are base-joined, and only the page's others.
    assert_eq!(
        shelf.asked(),
        [
            "guide/img/swatch.png",
            "guide/img/missing.png",
            "guide/img/cell.png"
        ]
    );

    // The same rule without an app, for a caller who checks a page's paths before drawing it.
    assert_eq!(
        markdown::resolve("guide", "img/a.png").as_deref(),
        Ok("guide/img/a.png")
    );
    assert_eq!(
        markdown::resolve("guide", "a/../b.png").as_deref(),
        Ok("guide/b.png")
    );
    assert_eq!(
        markdown::resolve("guide/a", "../b.png"),
        Err(PathRefused::EscapesBase)
    );
    assert_eq!(
        markdown::resolve("guide", "img/../../x.png"),
        Err(PathRefused::EscapesBase)
    );
    assert_eq!(
        markdown::resolve("guide", "/etc/x.png"),
        Err(PathRefused::Absolute)
    );
}

#[test]
fn links_and_headings_are_listed_from_the_same_parse() {
    let links = markdown::links(PAGE);
    assert_eq!(
        links
            .iter()
            .map(|l| (l.dest.as_str(), l.kind))
            .collect::<Vec<_>>(),
        [
            ("palette.md", LinkKind::Relative),
            ("https://example.com/x", LinkKind::Web),
            ("#sizes", LinkKind::Anchor),
        ]
    );
    let outline = markdown::outline("# A b\n\n## C\n\n# A b\n\n### D {#own}\n");
    assert_eq!(
        outline
            .iter()
            .map(|h| (h.level, h.id.as_str(), h.title.as_str()))
            .collect::<Vec<_>>(),
        [
            (1, "a-b", "A b"),
            (2, "c", "C"),
            (1, "a-b-1", "A b"),
            (3, "own", "D"),
        ],
        "ids are the words in lower case with dashes, a repeat counts up, an explicit id wins"
    );
}

#[test]
fn a_tight_list_item_ends_its_emphasis_and_its_link() {
    let document = markdown::parse("- **bold** then [a link](x.md) and `code`, *slanted* end\n");
    let [Block::List { items, .. }] = document.blocks.as_slice() else {
        panic!("one list: {:?}", document.blocks);
    };
    let [Block::Paragraph(inlines)] = items[0].blocks.as_slice() else {
        panic!(
            "a tight item's words are one paragraph: {:?}",
            items[0].blocks
        );
    };
    let linked: Vec<bool> = inlines
        .iter()
        .map(|i| match i {
            Inline::Text { link, .. } | Inline::Code { link, .. } => link.is_some(),
            _ => false,
        })
        .collect();
    assert_eq!(
        linked,
        [false, false, true, false, false, false, false, false],
        "only the link's own words are the link's: {inlines:?}"
    );
    let strong: Vec<bool> = inlines
        .iter()
        .filter_map(|i| match i {
            Inline::Text { style, .. } => Some(style.strong),
            _ => None,
        })
        .collect();
    assert_eq!(strong, [true, false, false, false, false, false, false]);
}

#[test]
fn the_view_lists_its_headings_and_jumps_to_one() {
    let mut body = String::from("# Top\n\n");
    for i in 0..60 {
        body.push_str(&format!("Paragraph number {i} of the long page.\n\n"));
    }
    body.push_str("## Far down\n\nThe end.\n");
    let mut app = app();
    let shelf = shelf(&mut app);
    let view = page(&mut app, &shelf, &body, 300.0);

    let outline = app
        .world()
        .get::<MarkdownOutline>(view)
        .expect("the view lists its headings");
    assert_eq!(
        outline
            .0
            .iter()
            .map(|h| (h.level, h.id.as_str()))
            .collect::<Vec<_>>(),
        [(1, "top"), (2, "far-down")]
    );

    let viewport = drawn_in_order::<MarkdownViewport>(&app, view)[0];
    let heading = find_by_label(&mut app, "Far down").expect("the heading is drawn");
    let view_box = rect_of(&app, viewport);
    assert!(
        rect_of(&app, heading).1 > view_box.3,
        "the heading starts below the viewport"
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).map(|s| s.y),
        Some(0.0)
    );

    app.world_mut().trigger(scroll_to_heading(view, "far-down"));
    settle(&mut app);

    let scrolled = app
        .world()
        .get::<ScrollPosition>(viewport)
        .map_or(0.0, |s| s.y);
    assert!(scrolled > 0.0, "the view scrolled");
    let (_, top, _, bottom) = rect_of(&app, heading);
    let (_, view_top, _, view_bottom) = rect_of(&app, viewport);
    assert!(
        top >= view_top - 1.0 && bottom <= view_bottom + 1.0,
        "the heading is in the viewport: {top}..{bottom} in {view_top}..{view_bottom}"
    );
}

fn rect_of(app: &App, entity: Entity) -> (f32, f32, f32, f32) {
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

#[test]
fn a_page_is_built_once_and_again_only_when_its_source_changes() {
    let mut app = app();
    let shelf = shelf(&mut app);
    let view = page(&mut app, &shelf, PAGE, 600.0);
    let built = |app: &App| app.world().get::<MarkdownBuilt>(view).map(|b| b.0);
    assert_eq!(built(&app), Some(1));
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(built(&app), Some(1), "sixty idle frames rebuild nothing");

    // Writing the same source is not a change.
    app.world_mut()
        .entity_mut(view)
        .get_mut::<MarkdownView>()
        .expect("the view")
        .set_source(PAGE);
    settle(&mut app);
    assert_eq!(built(&app), Some(1), "the same words build nothing");

    app.world_mut()
        .entity_mut(view)
        .get_mut::<MarkdownView>()
        .expect("the view")
        .set_source("# Another page\n\nWith other words.\n");
    settle(&mut app);
    assert_eq!(built(&app), Some(2), "other words build once");
    find_by_label(&mut app, "Another page").expect("the new page is drawn");
    assert!(
        find_by_label(&mut app, "Style guide").is_err(),
        "the old page is gone"
    );
}

/// A page like a style guide: 40 tables and enough prose to be about 75 KB.
fn long_page() -> String {
    let mut page = String::new();
    for table in 0..40 {
        page.push_str(&format!("## Section {table}\n\n"));
        page.push_str(
            "A paragraph of prose that explains the section at some length, with a \
             **strong** word, a `code` word and an *emphasised* one, so that a reader has \
             something to read between the tables of the guide.\n\n",
        );
        page.push_str("| Name | Value | Use | Note |\n|------|-------|-----|------|\n");
        for row in 0..22 {
            page.push_str(&format!(
                "| token-{table}-{row} | #{row:02x}{row:02x}{row:02x} | surface {row} | the note for row {row} of table {table} |\n"
            ));
        }
        page.push('\n');
    }
    page
}

#[test]
fn a_page_of_seventy_five_kilobytes_with_forty_tables_is_built_once() {
    let source = long_page();
    assert!(
        (60_000..90_000).contains(&source.len()),
        "the test page is about 75 KB: {} bytes",
        source.len()
    );
    let started = Instant::now();
    let document = markdown::parse(&source);
    let parsed = started.elapsed();
    assert_eq!(document.tables(), 40);
    println!("markdown: parsed {} bytes in {parsed:?}", source.len());

    let mut app = app();
    let shelf = shelf(&mut app);
    let host = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(640.0),
            height: Val::Px(600.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let started = Instant::now();
    let view = app
        .world_mut()
        .spawn((markdown_view(&source, shelf.base()), ChildOf(host)))
        .id();
    app.update();
    let built = started.elapsed();
    println!("markdown: built and laid out 40 tables in {built:?} (debug build)");
    assert_eq!(app.world().get::<MarkdownBuilt>(view).map(|b| b.0), Some(1));
    // Debug builds are far slower than the release build the issue's 50 ms is for; this only
    // keeps the build from being per-row quadratic.
    assert!(built.as_secs_f32() < 20.0, "built in {built:?}");

    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world().get::<MarkdownBuilt>(view).map(|b| b.0),
        Some(1),
        "thirty frames with the page on screen rebuild nothing"
    );
}

/// What the issue's 50 ms is about: parsing the page and spawning its blocks, with no layout and no
/// text shaping in it (those are a frame's own cost and do not grow with a rebuild). A debug build
/// is an order of magnitude slower than a release one, so the bound here only catches a build that
/// is quadratic in the page; the release number is in `docs/research/markdown-view.md`.
#[test]
fn parsing_and_spawning_forty_tables_is_a_plain_cost() {
    let source = long_page();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(GooeySkin::dark())
        .add_systems(Update, build_markdown);
    let view = app
        .world_mut()
        .spawn(markdown_view(&source, Shelf::default().base()))
        .id();
    let before = app.world().entities().len();
    let started = Instant::now();
    app.update();
    let spent = started.elapsed();
    let nodes = app.world().entities().len() - before;
    println!(
        "markdown: parsed and spawned {} bytes, {nodes} entities, in {spent:?} (debug build)",
        source.len()
    );
    assert_eq!(app.world().get::<MarkdownBuilt>(view).map(|b| b.0), Some(1));
    assert!(nodes > 3_000, "the page is spawned: {nodes} entities");
    assert!(spent.as_secs_f32() < 2.0, "built in {spent:?}");

    // A second frame builds nothing and costs nothing like the first.
    let started = Instant::now();
    app.update();
    assert!(started.elapsed() < spent / 10, "a built page is left alone");
    assert_eq!(app.world().get::<MarkdownBuilt>(view).map(|b| b.0), Some(1));
}
