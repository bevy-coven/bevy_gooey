# A Markdown view

Researched 2026-11-13 for issue #90 (G-79), asked by bevy_dark_factory#399 (a game's style guides are
Markdown files with pictures and tables, read rendered, beside a board of the same pictures; Sanctuary's
own is 74,733 bytes and uses tables throughout). At `57d3d49` the crate has `image_box` (G-70), a scrolling
`list` and a code editor, and no way to draw prose.

## Sources

1. *CommonMark Spec 0.31.2* (<https://spec.commonmark.org/0.31.2/>) — technical: what a heading, a list
   item (tight or loose), an image's alt text, a link destination and a raw HTML block are.
2. *GitHub Flavored Markdown Spec* (<https://github.github.com/gfm/>) — technical: tables (with column
   alignment), task list items and strikethrough, the three extensions the issue names.
3. `pulldown-cmark` 0.13 (MIT; <https://docs.rs/pulldown-cmark>) — technical: a pull parser that yields
   `Event`s with their source `Range`, with the extensions as `Options`. It is the dependency the issue
   names; with `default-features = false` it has no `getopts` and no HTML renderer, depends only on
   `bitflags`, `memchr` and `unicase`, and builds for wasm32.
4. W3C, *WCAG 2.2*, SC 1.4.1 Use of Color and SC 2.1.1 Keyboard — craft: a link must not be told from
   body text by colour alone, and must be reachable and pressable without a pointer.
5. W3C, *WAI-ARIA Authoring Practices*, "Link" and `heading` role — craft: a link is named by its words
   and is activated by Enter; a heading has a level.
6. Bevy 0.20 vendored source (`bevy_text` 0.20.1 and `bevy_ui` 0.20.1) — technical, and where the design
   forks: `Text` with `TextSpan` children is one text block (one wrap, any number of looks), but a span is
   not a node (it cannot take focus or a press), and an `InlineBox` that flows in text exists only for
   images (`InlineImage`). A link that is a widget therefore cannot be a span.
7. `docs/research/layout-primitives.md` (#81), `docs/research/text-area-fill.md` (#86) and
   `docs/research/scroll-glide.md` conventions — craft, the crate's own: sizes in the skin's units, the
   list's rail and glide, one path per feature.

## Model adopted

- **One parse, two readers.** `bevy_gooey::markdown` is engine-free (no Bevy type in it): `parse` makes a
  `Document` of `Block`s with the pictures, links and headings in source order, and `images`, `links`,
  `outline`, `resolve` and `image_size` are questions asked of it. The view draws that same `Document`
  depth first, so `images()` lists the pictures in the order the view draws them by construction, and a
  test holds it.
- **A page is built when its source changes.** `MarkdownView { source, base }` is the state;
  `build_markdown` compares them with what it last built (a private copy) and builds only on a
  difference, so a page that is on screen costs what any other node does, and writing the same words
  builds nothing (`MarkdownBuilt` counts the builds, and a test holds it at 1 across 60 frames).
- **A link is a widget, a paragraph without one is not.** Bevy cannot focus or press a text span, and the
  crate's rule is that every pointer thing is reachable by key and pad (SC 2.1.1), so a link is its own
  `Button` node (`Role::MarkdownLink`, Tab, Enter, a click, the pad's South) that sends `MarkdownLink`.
  A paragraph that holds a link is then a wrapping row, a node for each word, with the words glued to a
  link (a comma after it) sharing a no-wrap row; a paragraph with no link is one `Text` with a `TextSpan`
  for each change of look, which keeps a 75 KB page to about 8,000 entities instead of one for every word.
  The link is underlined, so it is never told by colour alone (SC 1.4.1), and drawn in the text colour
  (the skin does not promise accent text its own 4.5:1 on the page).
- **The view opens nothing.** `MarkdownLink { dest, kind }` where `kind` is `Web`, `Relative` or
  `Anchor` by the destination alone. The one thing the view does with a link is scroll to the heading an
  anchor names, because that is internal to the page and a table of contents that does nothing would be a
  trap for every host.
- **Pictures go through the host.** `MarkdownBase::new(dir, resolver)`: the view asks the resolver for
  `dir/path` (the host owns assets) and draws the handle through `image_box` at the image's own size,
  scaled down to 40 body text sizes wide. Until it has loaded, or when it is not there, the box is 16 by 9
  body text sizes (a size class in the skin's own unit, so the page holds its room as pictures load and
  grows with the player's text size at the next build). A picture that is not there, fails to load or has
  a refused path is a box (`Role::MarkdownMissing`) naming its alt text and its path, never nothing; a
  path that climbs out of `dir` with `..`, or starts at a root, is not given to the resolver, warns once
  and sends `MarkdownImageRefused`. An `http(s)` picture is handed to the resolver as written and the view fetches
  nothing; any other scheme (`embedded://`, `file://`: an asset source the directory does not hold) is
  refused like `..` (`PathRefused::Scheme`, `markdown::image_path`).
- **Headings** have ids: the page's `{#id}`, else the lower-cased words with dashes (the way forges name
  them), a repeat counting up. `MarkdownOutline` lists them; `scroll_to_heading(view, id)` stands the
  heading at the top of the viewport on the list's own spring (`ScrollGlide`), or as near as the end of
  the page allows.
- **Tables are a grid whose gaps are the lines.** `Display::Grid`, `minmax(0, 1fr)` columns, the table's
  fill the outline and the gap one border wide, every cell opaque: one line between two cells, not two
  borders, and no per-cell border arithmetic.
- **The look is the skin's.** Fourteen `Role::Markdown…` variants, all styled in `apply_skin`: headings
  step down `type_scale` from its top (1, 2, then 3 to 6), code is `Role::CodeText` (the mono face), a
  code block, quote and rule are the frames every other widget is drawn in. Bold and italic are the
  face's weight and slant, set where the words are spawned.
- **The rail and focus are the list's.** The viewport is a `ScrollArea` with `ScrollGlide` and
  `ScrollEdgeFade` and the `list_rail`; `update_lists`, `scroll_to_focus` and `page_lists` take it as a
  second kind of scroller, so Tab to a link below the fold scrolls it into view and PageUp and PageDown
  page the page.

## Found while building

- **A tight list item has no paragraph.** `Start(Item), Text, End(Item)`: the loose words are gathered
  into a paragraph of their own, and an `End(Strong)` or `End(Link)` met there must still end its
  emphasis (the first version dropped every end tag at block level and left the rest of the item bold and
  linked: caught by looking at the render, held by `a_tight_list_item_ends_its_emphasis_and_its_link`).
- **`AlignItems::Baseline` does not line a row of text nodes up in Bevy 0.20** (a link's node and a plain
  word's sat on different lines); `Center` does, because a link's border only makes its node a little
  taller.
- **A headless test app never finishes an asset load**: a handle from `AssetServer::load` of a missing
  file (or of an unknown extension) stays `Loading` in the crate's test windows (checked over 600 and
  400 frames), so `LoadState::Failed`, the one state `size_pictures` turns into the box, cannot be
  reached by a test here; the branch is one `matches!` and is not covered. A handle nobody has filled
  yet (`NotLoaded`) is deliberately **not** gone: it is a picture still to come, keeps its room and is
  drawn at its own size when the image arrives (`a_picture_whose_image_arrives_late_is_drawn_when_it_is_there`).
- **A heading goes through the paragraph's inline path** in the heading's face (`Face::Heading`): the first
  version drew `plain_text(inlines)`, so a picture in a heading vanished and a link in it was plain
  words while `images()` and `links()` listed them. Review caught it; `a_picture_and_a_link_in_a_heading_are_drawn_and_listed`
  holds it. A heading with a link is then word nodes, each a heading node in the access tree.
- **Focus rings need the focus list.** `apply_skin` restyles on a focus change only the roles in its
  list; `Role::MarkdownLink` and `Role::MarkdownHeadingLink` are in it (`a_focused_link_wears_the_ring_and_loses_it_with_focus`).
- **`find_by_label` reads a `Text`'s own string**, so a paragraph that is one run is findable by its
  words and one with several looks reads as its first run there; the access tree reads the `Text` and
  its `TextSpan` children together (`Look::own`), so a screen reader hears all of it.

## Measured

At 73,590 bytes with 40 tables of 22 rows (the test page; `long_page` in `tests/markdown.rs`, and
`GOOEY_BUILD_TIME=1 cargo run --release --example markdown --features markdown`), on big (Ryzen 9 5900X),
the cost of parsing and of spawning its 8,160 entities, with no window and no layout:

| | parse | parse + spawn |
|---|---|---|
| debug build | 5.7 to 12.6 ms | 124 ms |
| release build | 1.2 to 1.5 ms | 15.3 to 17.2 ms (five runs, at a load average of 13) |

The first frame that lays the page out and shapes its text is a frame's own cost, not the build's, and
grows with the text on the page: 2.6 s in a debug build at this size (`a_page_of_seventy_five_kilobytes…`
prints it). Thirty frames after it rebuild nothing.

## Not done

- No footnotes, math, definition lists or heading classes (the parse options are tables,
  strikethrough, task lists and heading attributes).
- An inline picture is a block of its own between the words before it and after it, not a glyph in the
  line (the picture inside a link is the link's button).
- No syntax colouring of a fence: `Block::Code` keeps the info string (`lang`), and the host that has a
  highlighter can draw it, or put the text in a `code_editor`.
- Selecting and copying the page's words is not offered.
