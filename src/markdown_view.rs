//! The Markdown view (feature `markdown`, #90, `docs/research/markdown-view.md`): a page of
//! CommonMark with tables, strikethrough and task lists, drawn in the skin's type scale in a
//! scrolling viewport. [`markdown_view`] spawns it; [`crate::markdown`] is the engine-free parse it
//! draws, so [`crate::markdown::images`], [`crate::markdown::links`] and
//! [`crate::markdown::outline`] list exactly what the view shows, in the order it shows it.
//!
//! **A page is built once.** [`build_markdown`] parses and spawns the whole page when the
//! [`MarkdownView`]'s source or base is not the one it last built from, and never otherwise: a
//! frame with the page on screen costs what any other node does, and writing the same words again
//! builds nothing. [`MarkdownBuilt`] counts the builds.
//!
//! **Links are widgets.** A link is its own focusable node (Tab, Enter, a click) that sends
//! [`MarkdownLink`]; the view opens nothing itself. A paragraph with no link is one `Text` with a
//! `TextSpan` for each change of emphasis; a paragraph with a link is a row that wraps, a node a
//! word, so the link can be pressed.
//!
//! **Pictures** are [`crate::image_box`]es loaded through the [`MarkdownBase`]'s resolver, from the
//! path relative to its directory. A picture the resolver does not know, one that fails to load
//! and one whose path leaves the directory are boxes ([`Role::MarkdownMissing`]) of the size the
//! picture would have, naming their alt text and path; the last also sends
//! [`MarkdownImageRefused`].

use std::fmt;
use std::sync::Arc;

use bevy::asset::LoadState;
use bevy::ecs::relationship::RelatedSpawner;
use bevy::ecs::spawn::SpawnWith;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{
    FontStyle, FontWeight, Justify, LineBreak, Strikethrough, TextLayout, TextSpan, Underline,
};
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{ComputedNode, ScrollPosition, UiGlobalTransform};
use bevy::ui_widgets::{Activate, Button, ScrollArea, observe};

use crate::layout::{BoxSize, CellWidth, LayoutGap};
use crate::list::span;
use crate::markdown::{
    self, Align, Block, Document, Inline, LinkKind, OutlineEntry, PathRefused, Style,
};
use crate::motion::MotionSettings;
use crate::scroll_glide::{ScrollEdgeFade, ScrollGlide};
use crate::skin::{GooeySkin, Role};
use crate::widgets::{image_box, list_rail, row};

/// The least width of a list's marker column, in body text sizes, and what each digit past one
/// adds: `1.` and `10.` and `100.` line their items up.
const MARKER_TEXT_SIZES: f32 = 1.6;
const MARKER_DIGIT_TEXT_SIZES: f32 = 0.7;

/// The widest a picture is drawn, in body text sizes: a bigger one is scaled down to this.
const IMAGE_MAX_TEXT_SIZES: f32 = 40.0;

/// The box a picture holds before its size is known, and the box a missing one is, in body text
/// sizes: the page keeps its room while pictures load.
const IMAGE_CLASS_TEXT_SIZES: [f32; 2] = [16.0, 9.0];

/// Where a page's pictures come from: the directory its relative paths are read against, and the
/// host's own loader. The view never reads a file: it asks `resolver` for the handle of
/// `dir/path`, so the host decides whether that is the asset server, an embedded table or a
/// test's own images.
///
/// A web picture (`https://...`) is handed to the resolver as written, and a path that leaves the
/// directory with `..` is refused before the resolver sees it ([`markdown::resolve`]).
#[derive(Clone)]
pub struct MarkdownBase {
    dir: String,
    resolver: Arc<Resolver>,
}

/// A host's loader of pictures: the path in, the image's handle out.
type Resolver = dyn Fn(&str) -> Option<Handle<Image>> + Send + Sync;

impl MarkdownBase {
    /// A base of `dir` whose pictures `resolver` loads: it is given the path joined onto `dir`
    /// (`"guide/img/swatch.png"`) and answers with the image's handle, or `None` for one it does
    /// not have.
    ///
    /// ```
    /// # use bevy::prelude::*;
    /// # use bevy_gooey::*;
    /// # fn ui(mut commands: Commands, assets: Res<AssetServer>) {
    /// let assets = AssetServer::clone(&assets);
    /// let base = MarkdownBase::new("guide", move |path: &str| Some(assets.load(path.to_owned())));
    /// commands.spawn(markdown_view("# Guide\n\n![Swatch](img/swatch.png)\n", base));
    /// # }
    /// ```
    pub fn new(
        dir: impl Into<String>,
        resolver: impl Fn(&str) -> Option<Handle<Image>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            dir: dir.into(),
            resolver: Arc::new(resolver),
        }
    }

    /// The directory a page's relative paths are read against.
    pub fn dir(&self) -> &str {
        &self.dir
    }
}

impl fmt::Debug for MarkdownBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarkdownBase")
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

/// Two bases are the same when they read the same directory through the same resolver.
impl PartialEq for MarkdownBase {
    fn eq(&self, other: &Self) -> bool {
        self.dir == other.dir && Arc::ptr_eq(&self.resolver, &other.resolver)
    }
}

/// A page: its source and where its pictures come from, the root [`markdown_view`] spawns.
/// Change the words with [`MarkdownView::set_source`] and the view builds the new page in the
/// same frame; writing the words it already shows builds nothing.
#[derive(Component, Clone, Debug, PartialEq)]
#[require(MarkdownOutline, MarkdownBuilt, MarkdownBuiltFrom)]
pub struct MarkdownView {
    source: String,
    base: MarkdownBase,
}

impl MarkdownView {
    /// The page's Markdown.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Shows `source` instead.
    pub fn set_source(&mut self, source: impl Into<String>) {
        self.source = source.into();
    }

    /// Where the page's pictures come from.
    pub fn base(&self) -> &MarkdownBase {
        &self.base
    }

    /// Reads the page's pictures from `base` instead (and builds the page again).
    pub fn set_base(&mut self, base: MarkdownBase) {
        self.base = base;
    }
}

/// What a [`MarkdownView`] last built (its source and base), so an unchanged page is never built
/// again. Internal: part of what [`markdown_view`] spawns, public because [`build_markdown`]'s
/// signature names it.
#[derive(Component, Default)]
pub struct MarkdownBuiltFrom(Option<(String, MarkdownBase)>);

/// How many times a [`MarkdownView`] has built its page: 1 once it is on screen, and one more for
/// each change of its source or base. A page that is on screen frame after frame does not move it.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarkdownBuilt(pub u32);

/// The headings of a [`MarkdownView`]'s page, in order, written when the page is built. Each is
/// what [`scroll_to_heading`] takes the id of.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct MarkdownOutline(pub Vec<OutlineEntry>);

/// The scrolling part of a [`markdown_view`]: the clipped column the page is a child of. To read
/// where the player is, read its `ScrollPosition`; to jump to a heading use [`scroll_to_heading`].
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(ScrollGlide, ScrollEdgeFade)]
pub struct MarkdownViewport;

/// On each heading node of a page: which view it belongs to and what it is called.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct MarkdownHeading {
    /// The [`MarkdownView`].
    pub view: Entity,
    /// 1 to 6.
    pub level: u8,
    /// The heading's stable id ([`markdown::OutlineEntry::id`]).
    pub id: String,
}

/// On each picture's node (the [`crate::image_box`] of one that loaded, the [`Role::MarkdownMissing`]
/// box of one that did not), in the order [`markdown::images`] lists them.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct MarkdownPicture {
    index: usize,
    dest: String,
    alt: String,
    path: String,
    sized: bool,
}

impl MarkdownPicture {
    /// Where this picture is in [`markdown::images`].
    pub fn index(&self) -> usize {
        self.index
    }

    /// The destination as the page wrote it.
    pub fn dest(&self) -> &str {
        &self.dest
    }

    /// The alternative text.
    pub fn alt(&self) -> &str {
        &self.alt
    }

    /// The path the resolver was asked for (the destination joined onto the base), or the
    /// destination itself for a web picture and a refused path.
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// A link of the page was pressed (a click, or Enter or the pad's South on it). The view opens
/// nothing: the host decides what `dest` means. An [`LinkKind::Anchor`] also scrolls the view to
/// the heading it names ([`scroll_to_heading`]), so a host that has nothing more to do with it
/// writes no observer.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, view: Entity) {
/// commands.entity(view).observe(|link: On<MarkdownLink>| match link.kind {
///     markdown::LinkKind::Web => info!("open {} in a browser", link.dest),
///     markdown::LinkKind::Relative => info!("open the page {}", link.dest),
///     markdown::LinkKind::Anchor => {}
/// });
/// # }
/// ```
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq)]
pub struct MarkdownLink {
    /// The [`MarkdownView`] the link is in.
    #[event_target]
    pub entity: Entity,
    /// The destination as the page wrote it.
    pub dest: String,
    /// What it points at.
    pub kind: LinkKind,
}

/// A picture's path was not loaded: it climbs out of the base directory, or starts at a root. The
/// picture is drawn as a box naming the path. Sent to the [`MarkdownView`] when the page is built,
/// and logged once.
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq)]
pub struct MarkdownImageRefused {
    /// The [`MarkdownView`].
    #[event_target]
    pub entity: Entity,
    /// The destination as the page wrote it.
    pub dest: String,
    /// Why it was refused.
    pub reason: PathRefused,
}

/// Ask a [`MarkdownView`] to scroll to a heading: send [`scroll_to_heading`]'s value with
/// `commands.trigger`. The view glides there (or lands at once under reduced motion), standing the
/// heading at the top of the viewport, or as near it as the page's end allows. An id the page does
/// not have does nothing.
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq)]
pub struct ScrollToHeading {
    /// The [`MarkdownView`].
    #[event_target]
    pub entity: Entity,
    /// The heading's id ([`MarkdownOutline`]).
    pub id: String,
}

/// The request to scroll `view` to the heading called `id`.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn jump(mut commands: Commands, view: Single<Entity, With<MarkdownView>>) {
///     commands.trigger(scroll_to_heading(*view, "palette"));
/// }
/// ```
pub fn scroll_to_heading(view: Entity, id: impl Into<String>) -> ScrollToHeading {
    ScrollToHeading {
        entity: view,
        id: id.into(),
    }
}

/// A page of Markdown in a scrolling viewport that takes the room its host leaves: CommonMark with
/// tables, strikethrough and task lists, in the skin's type scale, with its pictures loaded
/// through `base`. Give the host a height (a [`crate::panel`] in a pane, or any `Node` with one);
/// the rail shows while the page does not fit.
///
/// It is built once, when it is first on screen and when [`MarkdownView::set_source`] changes the
/// words, never per frame. See the module's docs for what is drawn and how.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, assets: Res<AssetServer>) {
/// let assets = AssetServer::clone(&assets);
/// let base = MarkdownBase::new("guide", move |path: &str| Some(assets.load(path.to_owned())));
/// commands.spawn((
///     Node { width: Val::Px(640.0), height: Val::Px(480.0), ..default() },
///     children![markdown_view("# Style guide\n\nSee [the palette](palette.md).\n", base)],
/// ));
/// # }
/// ```
pub fn markdown_view(source: &str, base: MarkdownBase) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            min_height: Val::Px(0.0),
            ..default()
        },
        Role::MarkdownView,
        MarkdownView {
            source: source.to_owned(),
            base,
        },
        Children::spawn(SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
            let viewport = parent
                .spawn((
                    Node {
                        display: Display::Block,
                        overflow: Overflow::scroll_y(),
                        flex_grow: 1.0,
                        flex_basis: Val::Px(0.0),
                        min_height: Val::Px(0.0),
                        ..default()
                    },
                    Role::MarkdownViewport,
                    MarkdownViewport,
                    ScrollArea,
                    children![(
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            flex_shrink: 0.0,
                            min_width: Val::Px(0.0),
                            ..default()
                        },
                        Role::Column,
                        LayoutGap(1.0),
                    )],
                ))
                .id();
            parent.spawn(list_rail(viewport));
        })),
    )
}

/// What a link node knows of where it leads.
#[derive(Component, Clone, Debug)]
struct LinkTarget {
    view: Entity,
    dest: String,
    kind: LinkKind,
}

/// A link was activated: the host hears [`MarkdownLink`], and an anchor also scrolls its view.
fn link_on_activate(event: On<Activate>, targets: Query<&LinkTarget>, mut commands: Commands) {
    let Ok(target) = targets.get(event.entity) else {
        return;
    };
    commands.trigger(MarkdownLink {
        entity: target.view,
        dest: target.dest.clone(),
        kind: target.kind,
    });
    if target.kind == LinkKind::Anchor {
        commands.trigger(scroll_to_heading(
            target.view,
            target.dest.trim_start_matches('#'),
        ));
    }
}

/// What a view is built from and built into.
type Pages = (
    Entity,
    Ref<'static, MarkdownView>,
    &'static mut MarkdownBuiltFrom,
    &'static mut MarkdownBuilt,
    &'static mut MarkdownOutline,
    &'static Children,
);

/// Builds each view's page when its source or base is not the one it last built: parses once and
/// spawns the blocks into the viewport's column. Runs in [`crate::GooeySystems::Style`] before
/// [`crate::apply_skin`].
pub fn build_markdown(
    mut commands: Commands,
    skin: Res<GooeySkin>,
    mut views: Query<Pages>,
    children: Query<&Children>,
) {
    for (view, page, mut from, mut built, mut outline, kids) in &mut views {
        if !page.is_changed() {
            continue;
        }
        if from
            .0
            .as_ref()
            .is_some_and(|(source, base)| *source == page.source && *base == page.base)
        {
            continue;
        }
        let Some(&viewport) = kids.first() else {
            continue;
        };
        let Some(&body) = children.get(viewport).ok().and_then(|c| c.first()) else {
            continue;
        };
        let document = markdown::parse(&page.source);
        commands.entity(body).despawn_related::<Children>();
        let mut build = Build {
            commands: &mut commands,
            view,
            base: &page.base,
            doc: &document,
            text_px: skin.text_size * skin.text_scale,
            refused: Vec::new(),
        };
        for block in &document.blocks {
            build.block(body, block);
        }
        let refused = std::mem::take(&mut build.refused);
        for (dest, reason) in refused {
            warn!("bevy_gooey: markdown picture `{dest}` is not loaded: {reason}");
            commands.trigger(MarkdownImageRefused {
                entity: view,
                dest,
                reason,
            });
        }
        outline.0 = document.outline;
        built.0 += 1;
        from.0 = Some((page.source.clone(), page.base.clone()));
    }
}

/// The page being spawned.
struct Build<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    view: Entity,
    base: &'a MarkdownBase,
    doc: &'a Document,
    /// The body text size in logical px, for the boxes whose size is not known yet.
    text_px: f32,
    refused: Vec<(String, PathRefused)>,
}

/// A run of words with one look.
struct Seg {
    text: String,
    style: Style,
    code: bool,
}

impl Build<'_, '_, '_> {
    fn spawn(&mut self, parent: Entity, bundle: impl Bundle) -> Entity {
        self.commands.spawn((bundle, ChildOf(parent))).id()
    }

    fn blocks(&mut self, parent: Entity, blocks: &[Block]) {
        for block in blocks {
            self.block(parent, block);
        }
    }

    fn block(&mut self, parent: Entity, block: &Block) {
        match block {
            Block::Heading { level, id, inlines } => {
                self.spawn(
                    parent,
                    (
                        Text::new(markdown::plain_text(inlines)),
                        Role::MarkdownHeading(*level),
                        MarkdownHeading {
                            view: self.view,
                            level: *level,
                            id: id.clone(),
                        },
                    ),
                );
            }
            Block::Paragraph(inlines) => self.inlines(parent, inlines, Align::Start),
            Block::Code { text, .. } | Block::Html(text) => {
                let code = self.spawn(
                    parent,
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        Role::MarkdownCode,
                    ),
                );
                self.spawn(
                    code,
                    (
                        Text::new(text.clone()),
                        Role::CodeText,
                        TextLayout::linebreak(LineBreak::WordOrCharacter),
                    ),
                );
            }
            Block::Quote(inner) => {
                let quote = self.spawn(
                    parent,
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        Role::MarkdownQuote,
                    ),
                );
                self.blocks(quote, inner);
            }
            Block::List { start, items } => {
                let list = self.spawn(
                    parent,
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            min_width: Val::Px(0.0),
                            ..default()
                        },
                        Role::Column,
                        LayoutGap(0.5),
                    ),
                );
                let last = start.unwrap_or(1) + items.len().saturating_sub(1) as u64;
                let digits = last.to_string().len();
                let width = MARKER_TEXT_SIZES + MARKER_DIGIT_TEXT_SIZES * (digits - 1) as f32;
                for (i, item) in items.iter().enumerate() {
                    let marker = match (item.task, start) {
                        (Some(true), _) => "[x]".to_owned(),
                        (Some(false), _) => "[ ]".to_owned(),
                        (None, Some(first)) => format!("{}.", first + i as u64),
                        (None, None) => "-".to_owned(),
                    };
                    let line = self.spawn(list, row(0.5));
                    let cell = self.spawn(
                        line,
                        (
                            Node {
                                flex_direction: FlexDirection::Row,
                                ..default()
                            },
                            Role::Cell,
                            CellWidth(if item.task.is_some() {
                                width.max(2.4)
                            } else {
                                width
                            }),
                        ),
                    );
                    self.spawn(cell, (Text::new(marker), Role::Label));
                    let content = self.spawn(
                        line,
                        (
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Stretch,
                                flex_grow: 1.0,
                                flex_shrink: 1.0,
                                flex_basis: Val::Px(0.0),
                                min_width: Val::Px(0.0),
                                ..default()
                            },
                            Role::Column,
                            LayoutGap(0.5),
                        ),
                    );
                    self.blocks(content, &item.blocks);
                }
            }
            Block::Table { aligns, head, rows } => {
                if head.is_empty() {
                    return;
                }
                let columns = head.len() as u16;
                let table = self.spawn(
                    parent,
                    (
                        Node {
                            display: Display::Grid,
                            grid_template_columns: RepeatedGridTrack::minmax(
                                columns,
                                MinTrackSizingFunction::Px(0.0),
                                MaxTrackSizingFunction::Fraction(1.0),
                            ),
                            ..default()
                        },
                        Role::MarkdownTable,
                    ),
                );
                for (column, cell) in head.iter().enumerate() {
                    self.cell(table, Role::MarkdownHeadCell, cell, aligns.get(column));
                }
                for row in rows {
                    for (column, cell) in row.iter().enumerate() {
                        self.cell(table, Role::MarkdownCell, cell, aligns.get(column));
                    }
                }
            }
            Block::Rule => {
                self.spawn(
                    parent,
                    (
                        Node {
                            flex_shrink: 0.0,
                            ..default()
                        },
                        Role::MarkdownRule,
                    ),
                );
            }
        }
    }

    fn cell(&mut self, table: Entity, role: Role, inlines: &[Inline], align: Option<&Align>) {
        let cell = self.spawn(
            table,
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    min_width: Val::Px(0.0),
                    ..default()
                },
                role,
            ),
        );
        self.inlines(cell, inlines, align.copied().unwrap_or_default());
    }

    /// A paragraph's worth of inlines into `parent`, a picture standing as a block of its own
    /// between the words before it and the words after.
    fn inlines(&mut self, parent: Entity, inlines: &[Inline], align: Align) {
        let mut from = 0;
        for (at, inline) in inlines.iter().enumerate() {
            if let Inline::Image { index, link } = inline {
                self.run(parent, &inlines[from..at], align);
                self.picture(parent, *index, *link);
                from = at + 1;
            }
        }
        self.run(parent, &inlines[from..], align);
    }

    /// Words with no picture among them: one `Text`, or a wrapping row of nodes when a link is in
    /// it.
    fn run(&mut self, parent: Entity, run: &[Inline], align: Align) {
        if run.iter().all(|inline| match inline {
            Inline::Text { text, .. } => text.trim().is_empty(),
            _ => false,
        }) {
            return;
        }
        let has_link = run.iter().any(|inline| {
            matches!(
                inline,
                Inline::Text { link: Some(_), .. } | Inline::Code { link: Some(_), .. }
            )
        });
        if has_link {
            self.line(parent, run, align);
        } else {
            self.words(parent, run, align);
        }
    }

    /// One `Text` for the run, with a span for each change of look.
    fn words(&mut self, parent: Entity, run: &[Inline], align: Align) {
        let mut segs: Vec<Seg> = Vec::new();
        for inline in run {
            let (text, style, code) = match inline {
                Inline::Text { text, style, .. } => (text.as_str(), *style, false),
                Inline::Code { text, .. } => (text.as_str(), Style::default(), true),
                Inline::Break => ("\n", Style::default(), false),
                Inline::Image { .. } => continue,
            };
            match segs.last_mut() {
                Some(last) if last.style == style && last.code == code => last.text.push_str(text),
                _ => segs.push(Seg {
                    text: text.to_owned(),
                    style,
                    code,
                }),
            }
        }
        if let Some(first) = segs.first_mut() {
            first.text = first.text.trim_start().to_owned();
        }
        if let Some(last) = segs.last_mut() {
            last.text = last.text.trim_end().to_owned();
        }
        segs.retain(|seg| !seg.text.is_empty());
        let layout = TextLayout::justify(justify_of(align));
        match segs.as_slice() {
            [] => {}
            [only] => {
                let node = self.spawn(
                    parent,
                    (
                        Text::new(only.text.clone()),
                        role_of(only),
                        font_of(only.style),
                        layout,
                    ),
                );
                if only.style.strike {
                    self.commands.entity(node).insert(Strikethrough);
                }
            }
            many => {
                let root = self.spawn(parent, (Text::new(""), Role::Label, layout));
                for seg in many {
                    let span = self.spawn(
                        root,
                        (
                            TextSpan::new(seg.text.clone()),
                            role_of(seg),
                            font_of(seg.style),
                            TextColor::default(),
                        ),
                    );
                    if seg.style.strike {
                        self.commands.entity(span).insert(Strikethrough);
                    }
                }
            }
        }
    }

    /// A wrapping row of the run's words, each a node, so a link among them is a widget. A word
    /// glued to a link (a comma after it, a bracket before it) shares a no-wrap row with it.
    fn line(&mut self, parent: Entity, run: &[Inline], align: Align) {
        enum Unit {
            Glued(Vec<Word>),
            Break,
        }
        struct Word {
            text: String,
            style: Style,
            code: bool,
            link: Option<usize>,
        }
        let mut units: Vec<Unit> = Vec::new();
        let mut current: Vec<Word> = Vec::new();
        let close = |units: &mut Vec<Unit>, current: &mut Vec<Word>| {
            if !current.is_empty() {
                units.push(Unit::Glued(std::mem::take(current)));
            }
        };
        let mut at = 0;
        while at < run.len() {
            match &run[at] {
                Inline::Text {
                    link: Some(link), ..
                }
                | Inline::Code {
                    link: Some(link), ..
                } => {
                    // The whole text of the link, whatever its emphasis, is one word: a link is
                    // pressed as one thing.
                    let link = *link;
                    let mut text = String::new();
                    let mut style = None;
                    while let Some(inline) = run.get(at) {
                        match inline {
                            Inline::Text {
                                text: t,
                                style: s,
                                link: Some(l),
                            } if *l == link => {
                                text.push_str(t);
                                style.get_or_insert(*s);
                            }
                            Inline::Code {
                                text: t,
                                link: Some(l),
                            } if *l == link => text.push_str(t),
                            _ => break,
                        }
                        at += 1;
                    }
                    current.push(Word {
                        text: text.trim().to_owned(),
                        style: style.unwrap_or_default(),
                        code: false,
                        link: Some(link),
                    });
                    continue;
                }
                Inline::Text { text, style, .. } => {
                    if text.starts_with(char::is_whitespace) {
                        close(&mut units, &mut current);
                    }
                    for (i, word) in text.split_whitespace().enumerate() {
                        if i > 0 {
                            close(&mut units, &mut current);
                        }
                        current.push(Word {
                            text: word.to_owned(),
                            style: *style,
                            code: false,
                            link: None,
                        });
                    }
                    if text.ends_with(char::is_whitespace) {
                        close(&mut units, &mut current);
                    }
                }
                Inline::Code { text, .. } => current.push(Word {
                    text: text.clone(),
                    style: Style::default(),
                    code: true,
                    link: None,
                }),
                Inline::Break => {
                    close(&mut units, &mut current);
                    units.push(Unit::Break);
                }
                Inline::Image { .. } => {}
            }
            at += 1;
        }
        close(&mut units, &mut current);

        let line = self.spawn(
            parent,
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    // Centred, not on the baseline: a link's border makes its node a little
                    // taller than a plain word's, and the middles line up.
                    align_items: AlignItems::Center,
                    justify_content: match align {
                        Align::Start | Align::Left => JustifyContent::FlexStart,
                        Align::Center => JustifyContent::Center,
                        Align::Right => JustifyContent::FlexEnd,
                    },
                    min_width: Val::Px(0.0),
                    ..default()
                },
                Role::MarkdownLine,
            ),
        );
        for unit in units {
            match unit {
                Unit::Break => {
                    self.spawn(
                        line,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(0.0),
                            ..default()
                        },
                    );
                }
                Unit::Glued(words) => {
                    let holder = if words.len() > 1 {
                        self.spawn(line, row(0.0))
                    } else {
                        line
                    };
                    for word in words {
                        match word.link {
                            Some(link) => self.link_word(holder, word.text, word.style, link),
                            None => {
                                let seg = Seg {
                                    text: word.text,
                                    style: word.style,
                                    code: word.code,
                                };
                                let node = self.spawn(
                                    holder,
                                    (
                                        Text::new(seg.text.clone()),
                                        role_of(&seg),
                                        font_of(seg.style),
                                    ),
                                );
                                if seg.style.strike {
                                    self.commands.entity(node).insert(Strikethrough);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// A link's words as a focusable node that sends [`MarkdownLink`].
    fn link_word(&mut self, parent: Entity, text: String, style: Style, link: usize) {
        let target = self.target(link);
        self.spawn(
            parent,
            (
                Text::new(text),
                Node {
                    max_width: Val::Percent(100.0),
                    ..default()
                },
                font_of(style),
                Button,
                Role::MarkdownLink,
                Hovered::default(),
                TabIndex(0),
                AutoDirectionalNavigation::default(),
                Underline,
                target,
                observe(link_on_activate),
            ),
        );
    }

    fn target(&self, link: usize) -> LinkTarget {
        let link = &self.doc.links[link];
        LinkTarget {
            view: self.view,
            dest: link.dest.clone(),
            kind: link.kind,
        }
    }

    /// A picture as a block: its box, or a box naming it, inside a link when the page made it one.
    fn picture(&mut self, parent: Entity, index: usize, link: Option<usize>) {
        let image = &self.doc.images[index];
        let class = IMAGE_CLASS_TEXT_SIZES.map(|size| size * self.text_px);
        let kind = LinkKind::of(&image.dest);
        let path = if kind == LinkKind::Web {
            Ok(image.dest.clone())
        } else {
            markdown::resolve(self.base.dir(), &image.dest)
        };
        let mut picture = MarkdownPicture {
            index,
            dest: image.dest.clone(),
            alt: image.alt.clone(),
            path: image.dest.clone(),
            sized: false,
        };
        let size = Vec2::from(class);
        let holder = match link {
            Some(link) => {
                let target = self.target(link);
                self.spawn(
                    parent,
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_self: AlignSelf::FlexStart,
                            ..default()
                        },
                        Button,
                        Role::MarkdownLink,
                        Hovered::default(),
                        TabIndex(0),
                        AutoDirectionalNavigation::default(),
                        target,
                        observe(link_on_activate),
                    ),
                )
            }
            None => parent,
        };
        let handle = match path {
            Ok(path) => {
                picture.path.clone_from(&path);
                (self.base.resolver)(&path)
            }
            Err(reason) => {
                self.refused.push((image.dest.clone(), reason));
                None
            }
        };
        match handle {
            Some(handle) => {
                self.spawn(holder, (image_box(handle, size), picture));
            }
            None => {
                self.missing(holder, picture, size);
            }
        }
    }

    /// The box a picture that is not there is drawn as: its size, its alt text and its path.
    fn missing(&mut self, parent: Entity, picture: MarkdownPicture, size: Vec2) -> Entity {
        let alt = picture.alt.clone();
        let path = picture.path.clone();
        let holder = self.spawn(
            parent,
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    align_self: AlignSelf::FlexStart,
                    ..default()
                },
                Role::MarkdownMissing,
                BoxSize(size),
                Pickable::IGNORE,
                picture,
            ),
        );
        if !alt.is_empty() {
            self.spawn(holder, (Text::new(alt), Role::Label));
        }
        self.spawn(holder, (Text::new(path), Role::MarkdownPath));
        holder
    }
}

fn justify_of(align: Align) -> Justify {
    match align {
        Align::Start | Align::Left => Justify::Left,
        Align::Center => Justify::Center,
        Align::Right => Justify::Right,
    }
}

fn role_of(seg: &Seg) -> Role {
    if seg.code {
        Role::CodeText
    } else {
        Role::Label
    }
}

/// Strong and emphasis are the face's weight and slant, set where the words are spawned: a skin
/// chooses faces, sizes and colours, and the page chooses which words are heavy.
fn font_of(style: Style) -> TextFont {
    TextFont {
        weight: if style.strong {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        },
        style: if style.emphasis {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        ..default()
    }
}

/// Sizes each picture that loaded to its own size (scaled down to 40 body text sizes), and
/// turns one whose image failed to load into a box naming its path. A picture that is still
/// loading keeps the box it was given. Runs in [`crate::GooeySystems::Style`] before
/// [`crate::apply_skin`].
pub fn size_pictures(
    mut commands: Commands,
    skin: Res<GooeySkin>,
    assets: Option<Res<Assets<Image>>>,
    server: Option<Res<AssetServer>>,
    mut pictures: Query<(Entity, &mut MarkdownPicture, &ImageNode, &mut BoxSize)>,
) {
    for (entity, mut picture, image, mut size) in &mut pictures {
        if picture.sized {
            continue;
        }
        if let Some(loaded) = assets.as_ref().and_then(|assets| assets.get(&image.image)) {
            let max = skin.text_size * skin.text_scale * IMAGE_MAX_TEXT_SIZES;
            let next =
                markdown::image_size(Some(loaded.size_f32().to_array()), max, size.0.to_array());
            size.set_if_neq(BoxSize(Vec2::from(next)));
            picture.sized = true;
            continue;
        }
        let gone = server.as_ref().is_some_and(|server| {
            matches!(
                server.load_state(&image.image),
                LoadState::Failed(_) | LoadState::NotLoaded
            )
        });
        if gone {
            picture.sized = true;
            let (alt, path) = (picture.alt.clone(), picture.path.clone());
            warn!("bevy_gooey: markdown picture `{path}` did not load");
            commands
                .entity(entity)
                .remove::<ImageNode>()
                .insert(Role::MarkdownMissing)
                .with_children(|parent| {
                    if !alt.is_empty() {
                        parent.spawn((Text::new(alt), Role::Label));
                    }
                    parent.spawn((Text::new(path), Role::MarkdownPath));
                });
        }
    }
}

/// [`ScrollToHeading`]: stands the heading at the top of the viewport.
pub fn scroll_to_heading_observer(
    event: On<ScrollToHeading>,
    headings: Query<(&MarkdownHeading, &ComputedNode, &UiGlobalTransform)>,
    children: Query<&Children>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform), Without<MarkdownViewport>>,
    mut viewports: Query<
        (&ComputedNode, &mut ScrollPosition, &mut ScrollGlide),
        With<MarkdownViewport>,
    >,
    settings: Res<MotionSettings>,
    time: Option<Res<Time<Real>>>,
) {
    let view = event.entity;
    let Some((_, node, transform)) = headings
        .iter()
        .find(|(heading, ..)| heading.view == view && heading.id == event.id)
    else {
        debug!(
            "`ScrollToHeading` on {view}: no heading is called `{}`",
            event.id
        );
        return;
    };
    let Some(viewport) = children.get(view).ok().and_then(|c| c.first().copied()) else {
        return;
    };
    let Some(body) = children.get(viewport).ok().and_then(|c| c.first().copied()) else {
        return;
    };
    let (Ok((body_node, body_transform)), Ok((port, mut scroll, mut glide))) =
        (nodes.get(body), viewports.get_mut(viewport))
    else {
        return;
    };
    let (top, _) = span(node, transform);
    let (body_top, _) = span(body_node, body_transform);
    let scale = port.inverse_scale_factor();
    let max = (port.content_size().y + port.scrollbar_size.y - port.size().y).max(0.0) * scale;
    // The heading at the top of the viewport, or as near as the end of the page allows. The body
    // moves with the scroll, so the difference of their tops is the same at every scroll.
    let want = (top - body_top).clamp(0.0, max);
    if (want - glide.goal().unwrap_or(scroll.y)).abs() > 0.5 {
        if ScrollGlide::glides(&settings, time.as_deref()) {
            glide.go(&scroll, want);
        } else {
            scroll.y = want;
        }
    }
}
