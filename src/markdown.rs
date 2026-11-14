//! Markdown, engine-free (feature `markdown`): a page of CommonMark with tables, strikethrough and
//! task lists, parsed once with [`pulldown_cmark`] into a [`Document`] of blocks, and the questions
//! a host asks of a page without spawning anything: [`images`], [`links`], [`outline`] and
//! [`resolve`]. [`crate::markdown_view`] draws the same [`Document`], so what these list is what the
//! view shows, in the order it shows it.
//!
//! Nothing here names an engine type, so it is tested without an app. Raw HTML is text: a block of
//! it is [`Block::Html`] and an inline tag is part of the words around it, never interpreted.
//!
//! ```
//! use bevy_gooey::markdown;
//!
//! let page = "# Guide\n\nSee [the palette](palette.md).\n\n![Swatch](img/swatch.png)\n";
//! assert_eq!(markdown::images(page)[0].1, "img/swatch.png");
//! assert_eq!(markdown::links(page)[0].dest, "palette.md");
//! assert_eq!(markdown::outline(page)[0].id, "guide");
//! assert_eq!(markdown::resolve("guide", "img/swatch.png").as_deref(), Ok("guide/img/swatch.png"));
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// Where a link points, by its destination alone: the view never opens anything, the host does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkKind {
    /// A destination with a scheme: `https://example.com`, `mailto:a@b.c`.
    Web,
    /// A path to another file, relative to the page: `palette.md`, `../other/page.md#part`.
    Relative,
    /// A heading of the same page: `#sizes`.
    Anchor,
}

impl LinkKind {
    /// The kind of `dest`: an anchor starts with `#`, a web link has a scheme (letters, digits,
    /// `+`, `-` or `.` before a `:`, and more than one letter so a drive such as `C:` is a path),
    /// anything else is a path relative to the page.
    pub fn of(dest: &str) -> Self {
        if dest.starts_with('#') {
            return Self::Anchor;
        }
        match dest.split_once(':') {
            Some((scheme, _))
                if scheme.len() > 1
                    && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                    && scheme
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) =>
            {
                Self::Web
            }
            _ => Self::Relative,
        }
    }
}

/// A link in a page, from [`links`] or [`Document::links`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRef {
    /// The link's own text in the source, `[words](dest)` whole.
    pub range: Range<usize>,
    /// The destination as written.
    pub dest: String,
    /// What the destination is ([`LinkKind::of`]).
    pub kind: LinkKind,
    /// The words the link shows, without their emphasis.
    pub text: String,
}

/// A picture in a page: where it is in the source, what it points at and what it says it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageRef {
    /// The picture's own text in the source, `![alt](dest)` whole.
    pub range: Range<usize>,
    /// The destination as written.
    pub dest: String,
    /// The alternative text, without its emphasis.
    pub alt: String,
}

/// A heading of a page, from [`outline`] or [`Document::outline`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineEntry {
    /// 1 to 6.
    pub level: u8,
    /// The heading's stable id: the explicit `{#id}` when the page gives one, else its words in
    /// lower case with dashes, and a repeat counts up (`intro`, `intro-1`, `intro-2`).
    pub id: String,
    /// The heading's words.
    pub title: String,
}

/// Why a picture's path was not resolved ([`resolve`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathRefused {
    /// The path climbs out of the base with `..`.
    EscapesBase,
    /// The path starts at a root (`/`, or a backslash): it is not relative to the page.
    Absolute,
    /// The destination has a scheme other than `http` and `https` (`embedded://`, `file://`,
    /// `data:`): it would reach an asset source the page's directory does not hold.
    Scheme,
}

impl fmt::Display for PathRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EscapesBase => "the path leaves the page's directory with `..`",
            Self::Absolute => "the path is absolute, not relative to the page",
            Self::Scheme => "the destination has a scheme other than http or https",
        })
    }
}

/// How a column's text is set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// No alignment given: the reader's start edge.
    #[default]
    Start,
    /// `:---`
    Left,
    /// `:---:`
    Center,
    /// `---:`
    Right,
}

/// The emphasis a run of words wears.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    /// `**strong**`
    pub strong: bool,
    /// `*emphasis*`
    pub emphasis: bool,
    /// `~~strikethrough~~`
    pub strike: bool,
}

/// One piece of a paragraph, heading or table cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inline {
    /// Words; a soft line break is a space.
    Text {
        /// The words.
        text: String,
        /// Their emphasis.
        style: Style,
        /// The link they are the text of, as an index into [`Document::links`].
        link: Option<usize>,
    },
    /// `` `code` ``
    Code {
        /// The code.
        text: String,
        /// The link it is the text of, as an index into [`Document::links`].
        link: Option<usize>,
    },
    /// A picture, as an index into [`Document::images`].
    Image {
        /// Which picture.
        index: usize,
        /// The link the picture is inside, as an index into [`Document::links`].
        link: Option<usize>,
    },
    /// A hard line break.
    Break,
}

/// A list item, with its task box when it has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// `Some(done)` for `- [x]` and `- [ ]`.
    pub task: Option<bool>,
    /// What the item holds: paragraphs, nested lists, pictures.
    pub blocks: Vec<Block>,
}

/// A block of a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// `# Heading`, with its stable id.
    Heading {
        /// 1 to 6.
        level: u8,
        /// The heading's id, as in [`OutlineEntry::id`].
        id: String,
        /// The heading's words.
        inlines: Vec<Inline>,
    },
    /// A paragraph, or the loose words of a tight list item.
    Paragraph(Vec<Inline>),
    /// A fenced or indented code block.
    Code {
        /// The info string after the fence (`rust`), if any.
        lang: Option<String>,
        /// The code, without its closing line break.
        text: String,
    },
    /// `> quoted`
    Quote(Vec<Block>),
    /// A bulleted or numbered list.
    List {
        /// `Some(first number)` for a numbered list.
        start: Option<u64>,
        /// Its items.
        items: Vec<Item>,
    },
    /// A table: a header row and the rows under it, every cell a few inlines.
    Table {
        /// How each column's text is set.
        aligns: Vec<Align>,
        /// The header row.
        head: Vec<Vec<Inline>>,
        /// The body rows, each as wide as the header.
        rows: Vec<Vec<Vec<Inline>>>,
    },
    /// `---`
    Rule,
    /// A block of raw HTML, shown as the text it is.
    Html(String),
}

/// A parsed page: its blocks, and the pictures, links and headings in the order they stand in the
/// source (which is the order a view draws them).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Document {
    /// The page's blocks, in order.
    pub blocks: Vec<Block>,
    /// Every picture, in source order.
    pub images: Vec<ImageRef>,
    /// Every link, in source order.
    pub links: Vec<LinkRef>,
    /// Every heading, in source order.
    pub outline: Vec<OutlineEntry>,
}

impl Document {
    /// How many tables the page holds, in quotes and list items too.
    pub fn tables(&self) -> usize {
        fn count(blocks: &[Block]) -> usize {
            blocks
                .iter()
                .map(|block| match block {
                    Block::Table { .. } => 1,
                    Block::Quote(inner) => count(inner),
                    Block::List { items, .. } => items.iter().map(|item| count(&item.blocks)).sum(),
                    _ => 0,
                })
                .sum()
        }
        count(&self.blocks)
    }
}

/// The parser's options: tables, strikethrough, task lists and `{#id}` on headings.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
}

/// Parses a page. Never fails: a page is always some blocks, and what CommonMark cannot make
/// sense of is text.
pub fn parse(source: &str) -> Document {
    let mut events = Parser::new_ext(source, options())
        .into_offset_iter()
        .peekable();
    let mut builder = Builder::default();
    let blocks = builder.blocks(&mut events, None);
    Document {
        blocks,
        images: builder.images,
        links: builder.links,
        outline: builder.outline,
    }
}

/// The pictures of a page, in source order: `(where in the source, destination, alternative
/// text)`. The same parse as [`crate::markdown_view`], so a host can check a page's paths before
/// (or without) drawing it.
pub fn images(source: &str) -> Vec<(Range<usize>, String, String)> {
    parse(source)
        .images
        .into_iter()
        .map(|image| (image.range, image.dest, image.alt))
        .collect()
}

/// The links of a page, in source order.
pub fn links(source: &str) -> Vec<LinkRef> {
    parse(source).links
}

/// The headings of a page, in source order, with their stable ids.
pub fn outline(source: &str) -> Vec<OutlineEntry> {
    parse(source).outline
}

/// The path of a picture on disk or in the host's assets: `dest` read relative to the page's
/// directory `base`, with `.` and `a/..` folded away. A path that climbs out of `base` is
/// [`PathRefused::EscapesBase`] and one that starts at a root is [`PathRefused::Absolute`]; the
/// view loads neither. Separators are `/`; a backslash counts as one.
///
/// ```
/// # use bevy_gooey::markdown::{resolve, PathRefused};
/// assert_eq!(resolve("guide", "a/../img/x.png").as_deref(), Ok("guide/img/x.png"));
/// assert_eq!(resolve("guide", "../x.png"), Err(PathRefused::EscapesBase));
/// assert_eq!(resolve("", "img/x.png").as_deref(), Ok("img/x.png"));
/// ```
pub fn resolve(base: &str, dest: &str) -> Result<String, PathRefused> {
    if dest.starts_with(['/', '\\']) {
        return Err(PathRefused::Absolute);
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in dest.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(PathRefused::EscapesBase);
                }
            }
            part => parts.push(part),
        }
    }
    let base = base.trim_end_matches(['/', '\\']);
    let path = parts.join("/");
    Ok(if base.is_empty() {
        path
    } else if path.is_empty() {
        base.to_owned()
    } else {
        format!("{base}/{path}")
    })
}

/// What the view asks its resolver for when a picture's destination is `dest`: a web address
/// (`http://` or `https://`) as written, any other scheme refused ([`PathRefused::Scheme`]: an
/// `embedded://` or `file://` source is not under the page's directory), and anything else the path
/// [`resolve`]d against `base`.
///
/// ```
/// # use bevy_gooey::markdown::{image_path, PathRefused};
/// assert_eq!(image_path("guide", "img/x.png").as_deref(), Ok("guide/img/x.png"));
/// assert_eq!(image_path("guide", "https://a.b/x.png").as_deref(), Ok("https://a.b/x.png"));
/// assert_eq!(image_path("guide", "embedded://a/x.png"), Err(PathRefused::Scheme));
/// assert_eq!(image_path("guide", "../x.png"), Err(PathRefused::EscapesBase));
/// ```
pub fn image_path(base: &str, dest: &str) -> Result<String, PathRefused> {
    if LinkKind::of(dest) != LinkKind::Web {
        return resolve(base, dest);
    }
    let lower = dest.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Ok(dest.to_owned())
    } else {
        Err(PathRefused::Scheme)
    }
}

/// The size a picture is drawn at, logical px: its own size when the image is known, scaled down
/// to `max_width` keeping its shape and never up; `class` when it is not (an image still loading,
/// or missing), so a page holds its room while it loads. A size that is not finite or not above
/// zero counts as unknown.
///
/// ```
/// # use bevy_gooey::markdown::image_size;
/// assert_eq!(image_size(Some([200.0, 100.0]), 400.0, [160.0, 90.0]), [200.0, 100.0]);
/// assert_eq!(image_size(Some([800.0, 400.0]), 400.0, [160.0, 90.0]), [400.0, 200.0]);
/// assert_eq!(image_size(None, 400.0, [160.0, 90.0]), [160.0, 90.0]);
/// ```
pub fn image_size(natural: Option<[f32; 2]>, max_width: f32, class: [f32; 2]) -> [f32; 2] {
    let usable = |v: f32| v.is_finite() && v > 0.0;
    match natural {
        Some([w, h]) if usable(w) && usable(h) => {
            if usable(max_width) && w > max_width {
                [max_width, h * max_width / w]
            } else {
                [w, h]
            }
        }
        _ => class,
    }
}

/// The id a heading's words make: lower case, letters and digits kept, `-` and `_` kept, spaces
/// turned to `-`, everything else dropped (the way a forge's own pages name theirs).
fn slug(title: &str) -> String {
    title
        .trim()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() {
                Some(c.to_lowercase().collect::<String>())
            } else if matches!(c, '-' | '_') {
                Some(c.to_string())
            } else if c.is_whitespace() {
                Some("-".to_owned())
            } else {
                None
            }
        })
        .collect()
}

type Events<I> = std::iter::Peekable<I>;

/// The state of one parse: the lists the document keeps, and the ids already given out.
#[derive(Default)]
struct Builder {
    images: Vec<ImageRef>,
    links: Vec<LinkRef>,
    outline: Vec<OutlineEntry>,
    ids: BTreeMap<String, usize>,
}

/// Words, code, pictures and breaks gathered until a block starts or ends.
#[derive(Default)]
struct Run {
    out: Vec<Inline>,
    style: Style,
    link: Option<usize>,
    /// The depth of strong, emphasis and strikethrough open, so a nested pair ends in order.
    depth: [u32; 3],
}

/// The words of `inlines` without their emphasis or links: a heading's title, an id's source. A
/// hard break is a space, and a picture is nothing (its alt text is [`ImageRef::alt`]).
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text, .. } | Inline::Code { text, .. } => out.push_str(text),
            Inline::Break => out.push(' '),
            Inline::Image { .. } => {}
        }
    }
    out
}

impl Run {
    fn text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(Inline::Text {
            text: last,
            style,
            link,
        }) = self.out.last_mut()
            && *style == self.style
            && *link == self.link
        {
            last.push_str(text);
            return;
        }
        self.out.push(Inline::Text {
            text: text.to_owned(),
            style: self.style,
            link: self.link,
        });
    }

    fn toggle(&mut self, which: usize, on: bool) {
        let depth = &mut self.depth[which];
        *depth = if on {
            *depth + 1
        } else {
            depth.saturating_sub(1)
        };
        let lit = *depth > 0;
        match which {
            0 => self.style.strong = lit,
            1 => self.style.emphasis = lit,
            _ => self.style.strike = lit,
        }
    }
}

impl Builder {
    /// Blocks up to the end of the container `end` (or the end of the page): a list item holds
    /// inline events of its own when it is tight, which are gathered into a paragraph.
    fn blocks<'a, I>(&mut self, events: &mut Events<I>, end: Option<TagEnd>) -> Vec<Block>
    where
        I: Iterator<Item = (Event<'a>, Range<usize>)>,
    {
        let mut blocks = Vec::new();
        let mut run = Run::default();
        while let Some((event, range)) = events.next() {
            match event {
                Event::End(tag) if Some(tag) == end => break,
                Event::Start(Tag::Paragraph) => {
                    flush(&mut run, &mut blocks);
                    let inlines = self.inlines(events, TagEnd::Paragraph);
                    if !inlines.is_empty() {
                        blocks.push(Block::Paragraph(inlines));
                    }
                }
                Event::Start(Tag::Heading { level, id, .. }) => {
                    flush(&mut run, &mut blocks);
                    let inlines = self.inlines(events, TagEnd::Heading(level));
                    let title = plain_text(&inlines).trim().to_owned();
                    let id = self.id_for(id.as_deref(), &title);
                    let level = level as u8;
                    self.outline.push(OutlineEntry {
                        level,
                        id: id.clone(),
                        title,
                    });
                    blocks.push(Block::Heading { level, id, inlines });
                }
                Event::Start(Tag::BlockQuote(kind)) => {
                    flush(&mut run, &mut blocks);
                    let inner = self.blocks(events, Some(TagEnd::BlockQuote(kind)));
                    blocks.push(Block::Quote(inner));
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    flush(&mut run, &mut blocks);
                    let lang = match kind {
                        CodeBlockKind::Fenced(info) => info
                            .split_whitespace()
                            .next()
                            .map(str::to_owned)
                            .filter(|lang| !lang.is_empty()),
                        CodeBlockKind::Indented => None,
                    };
                    let mut text = String::new();
                    for (event, _) in events.by_ref() {
                        match event {
                            Event::Text(t) | Event::Code(t) => text.push_str(&t),
                            Event::End(TagEnd::CodeBlock) => break,
                            _ => {}
                        }
                    }
                    while text.ends_with('\n') {
                        text.pop();
                    }
                    blocks.push(Block::Code { lang, text });
                }
                Event::Start(Tag::HtmlBlock) => {
                    flush(&mut run, &mut blocks);
                    let mut text = String::new();
                    for (event, _) in events.by_ref() {
                        match event {
                            Event::Html(t) | Event::Text(t) => text.push_str(&t),
                            Event::End(TagEnd::HtmlBlock) => break,
                            _ => {}
                        }
                    }
                    let text = text.trim_end().to_owned();
                    if !text.is_empty() {
                        blocks.push(Block::Html(text));
                    }
                }
                Event::Start(Tag::List(start)) => {
                    flush(&mut run, &mut blocks);
                    let items = self.items(events);
                    blocks.push(Block::List { start, items });
                }
                Event::Start(Tag::Table(aligns)) => {
                    flush(&mut run, &mut blocks);
                    blocks.push(self.table(events, &aligns));
                }
                Event::Rule => {
                    flush(&mut run, &mut blocks);
                    blocks.push(Block::Rule);
                }
                // The loose words of a tight item, and anything else an item holds before its
                // first block.
                other => self.inline_event(other, range, events, &mut run),
            }
        }
        flush(&mut run, &mut blocks);
        blocks
    }

    fn items<'a, I>(&mut self, events: &mut Events<I>) -> Vec<Item>
    where
        I: Iterator<Item = (Event<'a>, Range<usize>)>,
    {
        let mut items = Vec::new();
        while let Some((event, _)) = events.next() {
            match event {
                Event::Start(Tag::Item) => {
                    let task = match events.peek() {
                        Some((Event::TaskListMarker(done), _)) => Some(*done),
                        _ => None,
                    };
                    if task.is_some() {
                        events.next();
                    }
                    let blocks = self.blocks(events, Some(TagEnd::Item));
                    items.push(Item { task, blocks });
                }
                Event::End(TagEnd::List(_)) => break,
                _ => {}
            }
        }
        items
    }

    fn table<'a, I>(&mut self, events: &mut Events<I>, aligns: &[Alignment]) -> Block
    where
        I: Iterator<Item = (Event<'a>, Range<usize>)>,
    {
        let aligns: Vec<Align> = aligns
            .iter()
            .map(|a| match a {
                Alignment::None => Align::Start,
                Alignment::Left => Align::Left,
                Alignment::Center => Align::Center,
                Alignment::Right => Align::Right,
            })
            .collect();
        let mut head: Vec<Vec<Inline>> = Vec::new();
        let mut rows: Vec<Vec<Vec<Inline>>> = Vec::new();
        let mut row: Vec<Vec<Inline>> = Vec::new();
        let mut in_head = false;
        while let Some((event, _)) = events.next() {
            match event {
                Event::Start(Tag::TableHead) => in_head = true,
                Event::End(TagEnd::TableHead) => in_head = false,
                Event::Start(Tag::TableRow) => row = Vec::new(),
                Event::End(TagEnd::TableRow) => {
                    rows.push(std::mem::take(&mut row));
                }
                Event::Start(Tag::TableCell) => {
                    let cell = self.inlines(events, TagEnd::TableCell);
                    if in_head {
                        head.push(cell);
                    } else {
                        row.push(cell);
                    }
                }
                Event::End(TagEnd::Table) => break,
                _ => {}
            }
        }
        // Every body row is as wide as the header: a short row is padded, a long one cut.
        for row in &mut rows {
            row.resize(head.len(), Vec::new());
        }
        let mut aligns = aligns;
        aligns.resize(head.len(), Align::Start);
        Block::Table { aligns, head, rows }
    }

    /// Inlines up to the end tag `end`.
    fn inlines<'a, I>(&mut self, events: &mut Events<I>, end: TagEnd) -> Vec<Inline>
    where
        I: Iterator<Item = (Event<'a>, Range<usize>)>,
    {
        let mut run = Run::default();
        while let Some((event, range)) = events.next() {
            if matches!(&event, Event::End(tag) if *tag == end) {
                break;
            }
            self.inline_event(event, range, events, &mut run);
        }
        run.out
    }

    fn inline_event<'a, I>(
        &mut self,
        event: Event<'a>,
        range: Range<usize>,
        events: &mut Events<I>,
        run: &mut Run,
    ) where
        I: Iterator<Item = (Event<'a>, Range<usize>)>,
    {
        match event {
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                self.note_link_text(run.link, &text);
                run.text(&text);
            }
            Event::Code(text) => {
                self.note_link_text(run.link, &text);
                run.out.push(Inline::Code {
                    text: text.into_string(),
                    link: run.link,
                });
            }
            Event::SoftBreak => {
                self.note_link_text(run.link, " ");
                run.text(" ");
            }
            Event::HardBreak => run.out.push(Inline::Break),
            Event::Start(Tag::Strong) => run.toggle(0, true),
            Event::End(TagEnd::Strong) => run.toggle(0, false),
            Event::Start(Tag::Emphasis) => run.toggle(1, true),
            Event::End(TagEnd::Emphasis) => run.toggle(1, false),
            Event::Start(Tag::Strikethrough) => run.toggle(2, true),
            Event::End(TagEnd::Strikethrough) => run.toggle(2, false),
            Event::Start(Tag::Link { dest_url, .. }) => {
                self.links.push(LinkRef {
                    range,
                    kind: LinkKind::of(&dest_url),
                    dest: dest_url.into_string(),
                    text: String::new(),
                });
                run.link = Some(self.links.len() - 1);
            }
            Event::End(TagEnd::Link) => run.link = None,
            Event::Start(Tag::Image { dest_url, .. }) => {
                let mut alt = String::new();
                let mut depth = 1u32;
                for (event, _) in events.by_ref() {
                    match event {
                        Event::Start(Tag::Image { .. }) => depth += 1,
                        Event::End(TagEnd::Image) => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        Event::Text(t) | Event::Code(t) | Event::InlineHtml(t) => alt.push_str(&t),
                        Event::SoftBreak | Event::HardBreak => alt.push(' '),
                        _ => {}
                    }
                }
                self.images.push(ImageRef {
                    range,
                    dest: dest_url.into_string(),
                    alt,
                });
                run.out.push(Inline::Image {
                    index: self.images.len() - 1,
                    link: run.link,
                });
            }
            // A marker with no item to own it, an end tag nobody opened, footnotes and math the
            // options leave off: nothing to draw.
            _ => {}
        }
    }

    fn note_link_text(&mut self, link: Option<usize>, text: &str) {
        if let Some(link) = link.and_then(|i| self.links.get_mut(i)) {
            link.text.push_str(text);
        }
    }

    /// The id for a heading: the page's own, else its words' slug, counted up on a repeat.
    fn id_for(&mut self, explicit: Option<&str>, title: &str) -> String {
        let base = match explicit {
            Some(id) if !id.is_empty() => id.to_owned(),
            _ => {
                let slug = slug(title);
                if slug.is_empty() {
                    "section".to_owned()
                } else {
                    slug
                }
            }
        };
        let seen = self.ids.entry(base.clone()).or_insert(0);
        let id = if *seen == 0 {
            base
        } else {
            format!("{base}-{seen}")
        };
        *seen += 1;
        id
    }
}

/// Moves a paragraph's worth of loose inlines into a block.
fn flush(run: &mut Run, blocks: &mut Vec<Block>) {
    if !run.out.is_empty() {
        blocks.push(Block::Paragraph(std::mem::take(&mut run.out)));
    }
}
