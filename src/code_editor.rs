//! The code editor: [`code_editor`], a buffer with a line-number gutter, a caret on a spring, a
//! selection, undo, a wheel and pointer that glide, and lines that cost nothing off screen.
//!
//! **What is drawn is a window onto the text, never the text.** The rows are a small pool of
//! entities (a screenful and two more) that are re-pointed at other lines as the view scrolls, so
//! a 20,000-line file costs what a 20-line one does ([`PERF_BUDGET`]); a long line is drawn only
//! for the columns in view, so a 10,000-character line is typed in at the speed of a short one.
//! The text itself is a [`CodeBuffer`](crate::CodeBuffer) in the [`CodeEditor`] component, and
//! [`CodeEditorValue`] is its copy for the host: the text and a revision that moves with every
//! change, the player's and the host's.
//!
//! **The keys are an observer, not a system**: [`FocusedInput`] reaches the editor only while it has
//! focus, and an observer on the editor can stop the key there. That is what lets Tab indent
//! (Bevy's tab navigation is an observer on the window, which the key never reaches) without
//! trapping the keyboard: Escape gives focus up, F6 and a pad's D-pad move on, and the editor is a
//! [`FocusRegion::host`], so the arrow keys are its own.
//!
//! Every length here is **skin pixels** (what a `Val::Px` is written in): the layout's
//! `ComputedNode` sizes are physical pixels and a pointer is the window's logical pixels, and each
//! is converted at its edge ([`to_skin`], [`pointer_in`]).

use bevy::clipboard::{Clipboard, ClipboardRead};
use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input::mouse::{MouseScrollPixelsPerLine, MouseScrollUnit, MouseWheel};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusVisible};
use bevy::picking::Pickable;
use bevy::picking::events::{PointerDrag, PointerDragEnd, PointerPress};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::text::{LineBreak, TextLayout, TextSpan};
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{ComputedNode, ComputedUiRenderTargetInfo, InteractionDisabled, UiGlobalTransform};
use bevy::ui_widgets::{ValueChange, observe};
use bevy::window::{Ime, PrimaryWindow};

use crate::clicks::ClickCount;
use crate::code_buffer::{
    CodeBuffer, CodeIndent, CodeMove, CodePos, byte_at_display, display_col, display_width,
    expand_slice,
};
use crate::code_decor::{CodeCompletions, CompletionTrigger};
use crate::code_decor::{Highlights, line_runs};
use crate::code_fold::FoldMap;
use crate::code_overlay::{FoldRequest, Overlay, code_on_move, code_on_out};
use crate::code_search::CodeFind;
use crate::model::{SpringState, snap_line_width, snap_text_size, spring_settled, spring_step};
use crate::motion::{MotionSettings, MotionToken};
use crate::region::FocusRegion;
use crate::scroll_glide::ScrollGlide;
use crate::skin::{GooeySkin, Role, SkinScope, skin_of};
use crate::text::TextEditing;

/// How many times a 20-line file's frame time the frame time of a 20,000-line file may be, while
/// both scroll: the editor draws a screenful whatever the file, so the two are the same work and
/// this is the room for noise. `tests/code_editor.rs` holds it.
pub const PERF_BUDGET: f32 = 2.0;

/// A line is this many text sizes high.
pub const CODE_LINE_EM: f32 = 1.5;

/// The least the editor is high, in lines, so one with no room from its host is still a place to
/// type.
pub const CODE_MIN_LINES: f32 = 4.0;

/// The share of the skin's `motion.snappy` spring the caret travels on: the caret must keep up
/// with typing, so it is half of what a button's press takes.
pub const CARET_SPRING_FRACTION: f32 = 0.5;

/// How wide the caret's blink is, seconds for the caret lit and dark both; it is lit for the first
/// half. The Windows default of 530 ms each; [`CodeEditorSettings::blink_period_s`] sets it.
pub const CARET_BLINK_PERIOD_S: f32 = 1.06;

/// How opaque the selection's accent is, so the text shows through it.
pub const CODE_SELECTION_ALPHA: f32 = 0.35;

/// How many columns the caret keeps between itself and the edge of the view when it is revealed.
const REVEAL_COLUMNS: f32 = 3.0;

/// The settings of every [`code_editor`], a resource [`crate::GooeyStylePlugin`] inits.
///
/// ```
/// # use bevy_gooey::CodeEditorSettings;
/// let settings = CodeEditorSettings::default();
/// assert!(settings.blink_period_s > 0.0);
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
pub struct CodeEditorSettings {
    /// The caret's blink, seconds for lit and dark both ([`CARET_BLINK_PERIOD_S`]). A host that
    /// wants the system's own sets it from the platform's caret blink time; zero or less is a
    /// steady caret.
    pub blink_period_s: f32,
    /// Typing one of these characters asks the host for completions
    /// ([`CodeCompletionRequested`](crate::CodeCompletionRequested) with
    /// [`CompletionTrigger::Character`](crate::CompletionTrigger::Character)). Default `".:"`.
    pub completion_triggers: String,
    /// Whether typing a letter, a digit or an underscore, with no list open, asks the host for
    /// completions ([`CompletionTrigger::Typing`](crate::CompletionTrigger::Typing)). Default
    /// true; a host that never answers loses nothing by it.
    pub complete_while_typing: bool,
}

/// The characters that ask for completions by default.
pub const DEFAULT_COMPLETION_TRIGGERS: &str = ".:";

impl Default for CodeEditorSettings {
    fn default() -> Self {
        Self {
            blink_period_s: CARET_BLINK_PERIOD_S,
            completion_triggers: DEFAULT_COMPLETION_TRIGGERS.to_owned(),
            complete_while_typing: true,
        }
    }
}

/// How a [`code_editor`] indents and draws a tab. Put it on the editor; the default is four spaces
/// and a four-column tab.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     code_editor("Script", "fn main() {\n}\n"),
///     CodeIndentation { indent: CodeIndent::Tab, tab_width: 8 },
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeIndentation {
    /// What the Tab key puts in.
    pub indent: CodeIndent,
    /// How many columns a tab character is drawn.
    pub tab_width: u8,
}

impl Default for CodeIndentation {
    fn default() -> Self {
        Self {
            indent: CodeIndent::default(),
            tab_width: 4,
        }
    }
}

/// The text of a [`code_editor`], for the host: read it for the player's text, and write it with
/// [`CodeEditorValue::set`] to replace the text (the history is cleared, the caret stays where it
/// still is).
///
/// `revision` moves with every change of the text, the player's typing, undo and redo and the
/// host's [`CodeEditorValue::set`] alike, so a host that keeps what it last saw tells a stale copy
/// from a current one by it. A player's edit is also one [`ValueChange<String>`] on the editor
/// (`is_final` for Cmd or Ctrl+S).
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct CodeEditorValue {
    text: String,
    revision: u64,
}

impl CodeEditorValue {
    /// A value of `text`, at revision 0.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            revision: 0,
        }
    }

    /// The text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// How many changes the text has had.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Replaces the text; the editor takes it the same frame, with no [`ValueChange`].
    pub fn set(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.revision += 1;
    }
}

/// Which part of an editor an entity is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PartKind {
    Gutter,
    GutterRows,
    Area,
    AreaRows,
    Probe,
    LineLit,
    Caret,
    /// A line number's row, by slot.
    Number(usize),
    /// A line's row, by slot.
    Text(usize),
    /// A selection band, by slot.
    Band(usize),
}

/// A part of the editor `editor`.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct CodePart {
    editor: Entity,
    kind: PartKind,
}

impl CodePart {
    /// The editor this is a part of.
    pub(crate) fn editor_entity(&self) -> Entity {
        self.editor
    }
}

/// What a pool row shows, so it is written only when that changes.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub(crate) struct RowShown {
    line: usize,
    first_col: usize,
    tab: usize,
    styled: u64,
    cols: usize,
    revision: u64,
    lit: bool,
}

impl RowShown {
    /// The line the row draws, `usize::MAX` for none.
    pub(crate) fn line_shown(&self) -> usize {
        self.line
    }
}

/// The spans a text row draws its coloured runs in: pooled children of the row, re-pointed as the
/// row is.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct RowSpans(pub(crate) Vec<Entity>);

/// The entities of an editor's parts, in slot order.
#[derive(Clone, Debug, Default)]
pub(crate) struct Parts {
    pub(crate) gutter: Option<Entity>,
    pub(crate) gutter_rows: Option<Entity>,
    pub(crate) area: Option<Entity>,
    pub(crate) area_rows: Option<Entity>,
    pub(crate) probe: Option<Entity>,
    pub(crate) line_lit: Option<Entity>,
    pub(crate) caret: Option<Entity>,
    pub(crate) numbers: Vec<Entity>,
    pub(crate) texts: Vec<Entity>,
    pub(crate) bands: Vec<Entity>,
}

/// Where the view stands this frame, written by [`update_code_editors`] for the systems that draw
/// over it ([`crate::code_overlay`]): all in skin px.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct View {
    pub(crate) ready: bool,
    /// The first display row drawn, and the first display column.
    pub(crate) first_row: usize,
    pub(crate) first_col: usize,
    /// How many columns a row draws, how many display rows the text has, and how many are pooled.
    pub(crate) cols: usize,
    pub(crate) rows: usize,
    pub(crate) needed: usize,
    pub(crate) shown: Vec2,
    pub(crate) pad: f32,
    pub(crate) advance: f32,
    pub(crate) line_px: f32,
    pub(crate) gutter_w: f32,
    pub(crate) area: Vec2,
    pub(crate) skin_text: f32,
}

/// Where the pointer last pressed or dragged: written by the observers, read by
/// [`update_code_editors`], which has the geometry.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Gesture {
    /// The press, and how many quick presses it is in a run (1, 2, 3).
    Press(Vec2),
    /// The pointer while the primary button is held.
    Drag(Vec2),
}

/// How a held press selects: by character, word or line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unit {
    Char,
    Word,
    Line,
}

/// A [`code_editor`]: its text, caret, selection and history, and the state of its view. Read it
/// to find where the caret is and what is in view; change the text through [`CodeEditorValue`].
#[derive(Component)]
#[require(
    CodeEditorValue,
    CodeFind,
    CodeIndentation,
    ScrollGlide,
    bevy::picking::hover::Hovered
)]
pub struct CodeEditor {
    pub(crate) label: String,
    pub(crate) buffer: CodeBuffer,
    /// The [`CodeEditorValue`] revision the buffer holds.
    pub(crate) synced: u64,
    /// The buffer revision the value holds.
    pub(crate) published: u64,
    /// The scroll the view draws, and its goal, each in skin px from the text's top-left.
    pub(crate) shown_x: SpringState,
    pub(crate) shown_y: SpringState,
    pub(crate) goal: Vec2,
    /// Whether the next frame brings the caret into view.
    pub(crate) reveal: bool,
    /// The caret as drawn, in the text's own px (before the scroll).
    pub(crate) caret_x: SpringState,
    pub(crate) caret_y: SpringState,
    pub(crate) caret_primed: bool,
    /// Seconds since the player last did something, which restarts the blink.
    pub(crate) blink: f32,
    pub(crate) advance: f32,
    pub(crate) line_px: f32,
    pub(crate) area: Vec2,
    pub(crate) gesture: Option<Gesture>,
    pub(crate) unit: Unit,
    /// The first end of what a held press selects by word or line.
    pub(crate) pivot: Option<(CodePos, CodePos)>,
    pub(crate) pending_paste: Option<ClipboardRead>,
    pub(crate) preedit: String,
    pub(crate) parts: Parts,
    pub(crate) scan: bool,
    /// The widest line, in columns, for the buffer revision and tab width it was measured at.
    pub(crate) longest: ((u64, usize), usize),
    /// The lines the closed folds hide, and the headers the player closed.
    pub(crate) fold: FoldMap,
    pub(crate) folded: std::collections::BTreeSet<usize>,
    pub(crate) fold_dirty: bool,
    pub(crate) view: View,
    /// The byte offset each line starts at, for the buffer revision it was measured at.
    pub(crate) starts: (u64, Vec<usize>),
    /// What is drawn over the text and what the host and the player asked of it.
    pub(crate) overlay: Overlay,
}

impl CodeEditor {
    fn new(label: String, text: &str) -> Self {
        Self {
            label,
            buffer: CodeBuffer::new(text),
            synced: 0,
            published: 1,
            shown_x: SpringState::default(),
            shown_y: SpringState::default(),
            goal: Vec2::ZERO,
            reveal: false,
            caret_x: SpringState::default(),
            caret_y: SpringState::default(),
            caret_primed: false,
            blink: 0.0,
            advance: 0.0,
            line_px: 0.0,
            area: Vec2::ZERO,
            gesture: None,
            unit: Unit::Char,
            pivot: None,
            pending_paste: None,
            preedit: String::new(),
            parts: Parts::default(),
            scan: true,
            longest: ((u64::MAX, 0), 0),
            fold: FoldMap::default(),
            folded: std::collections::BTreeSet::new(),
            fold_dirty: false,
            view: View::default(),
            starts: (u64::MAX, Vec::new()),
            overlay: Overlay::default(),
        }
    }

    /// The name the editor was given, for assistive technology.
    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    /// The text, caret and selection.
    pub fn buffer(&self) -> &CodeBuffer {
        &self.buffer
    }

    /// Selects from `anchor` to `head` and brings the caret into view.
    pub fn select(&mut self, anchor: CodePos, head: CodePos) {
        self.buffer.select(anchor, head);
        self.touched();
    }

    /// Moves the view's goal to put `line` at the top, clamped to the text; the view glides there.
    pub fn scroll_to_line(&mut self, line: usize) {
        self.goal.y = line as f32 * self.line_px;
    }

    /// The lines that show now, first to one past the last: a line cut off at the foot counts.
    pub fn visible_lines(&self) -> std::ops::Range<usize> {
        if self.line_px <= 0.0 {
            return 0..0;
        }
        let first = (self.shown_y.value / self.line_px).floor().max(0.0) as usize;
        let last = ((self.shown_y.value + self.area.y) / self.line_px)
            .ceil()
            .max(0.0) as usize;
        first.min(self.buffer.len_lines())..last.min(self.buffer.len_lines())
    }

    /// How far the view is scrolled, in skin px from the text's top-left.
    pub fn scroll(&self) -> Vec2 {
        Vec2::new(self.shown_x.value, self.shown_y.value)
    }

    /// Where the caret is drawn now, in skin px from the text area's top-left (not the editor's:
    /// the gutter is left of it).
    pub fn caret_at(&self) -> Vec2 {
        Vec2::new(
            self.caret_x.value - self.shown_x.value,
            self.caret_y.value - self.shown_y.value,
        )
    }

    /// The player did something to the text or the caret: the caret shows steady and comes into view.
    pub(crate) fn touched(&mut self) {
        self.reveal = true;
        self.blink = 0.0;
        self.preedit.clear();
        if !self.overlay.note_request {
            self.overlay.note_pinned = false;
        }
    }

    /// Moves the caret `n` rows up or down; a line a closed fold hides is no row.
    pub(crate) fn move_rows(&mut self, up: bool, n: usize, extend: bool) {
        let rows = self.fold.rows(self.buffer.len_lines());
        let row = self.fold.row_of(self.buffer.caret().line);
        let target = if up {
            row.checked_sub(n)
        } else {
            Some(row + n).filter(|&row| row < rows)
        };
        match target {
            Some(row) => self
                .buffer
                .move_to(CodeMove::ToLine(self.fold.line_at(row)), extend),
            None if up => self.buffer.move_to(CodeMove::DocStart, extend),
            None => self.buffer.move_to(CodeMove::DocEnd, extend),
        }
    }

    /// A caret a closed fold hides goes to the end of the fold's header line.
    pub(crate) fn keep_caret_out_of_folds(&mut self) {
        let caret = self.buffer.caret();
        if let Some(header) = self.fold.header_of(caret.line) {
            let end = CodePos::new(header, self.buffer.line(header).len());
            let selection = self.buffer.selection();
            let anchor = if selection.is_empty() {
                end
            } else {
                selection.anchor
            };
            self.buffer.select(anchor, end);
        }
    }

    /// Asks to close (or open) the fold at the caret's line.
    pub(crate) fn fold_at_caret(&mut self, fold: bool) {
        let line = self.buffer.caret().line;
        self.overlay.fold_requests.push(if fold {
            FoldRequest::Fold(line)
        } else {
            FoldRequest::Unfold(line)
        });
    }

    fn page(&self) -> usize {
        if self.line_px <= 0.0 {
            return 1;
        }
        ((self.area.y / self.line_px).floor() as usize)
            .saturating_sub(1)
            .max(1)
    }
}

/// The measures an editor is laid out in, from the skin and the scale: [`crate::apply_skin`] sets
/// the rows' size from them and the update system reads the same, so the two never disagree.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CodeMetrics {
    /// The text size as it is set (whole device pixels), skin px.
    pub font_px: f32,
    /// A line's height.
    pub line_px: f32,
    /// The room left of the text and right of a line number.
    pub pad_px: f32,
    /// The caret's width, on whole device pixels.
    pub caret_px: f32,
}

impl CodeMetrics {
    pub(crate) fn of(skin: &GooeySkin, scale: f32) -> Self {
        let font_px = snap_text_size(skin.text_size * skin.text_scale, scale);
        Self {
            font_px,
            line_px: (font_px * CODE_LINE_EM).round().max(1.0),
            pad_px: skin.gap,
            caret_px: snap_line_width(skin.border, scale),
        }
    }
}

/// A [`code_editor`]'s caret on a line, kept lit while its row is the current line's.
const NUMBER_PROBE: &str = "0000000000";

/// A code editor: a line-number gutter and the text beside it, filling the room its host leaves,
/// with the current line lit, a caret that travels on a spring and blinks, selection by keyboard and
/// pointer (double-click a word, triple-click a line), undo and redo, a clipboard, IME, and a
/// view that glides. `label` names it for assistive technology; `text` is what it holds.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     code_editor("Scene script", "let speed = 3;\n"),
///     observe(|change: On<ValueChange<String>>| {
///         if change.is_final {
///             info!("saved {} bytes", change.value.len());
///         }
///     }),
/// ));
/// # }
/// ```
///
/// **Keys.** Characters, Enter (it carries the line's indent), Backspace and Delete (with Ctrl or
/// Alt, a word), the arrows (with Ctrl or Alt, a word; with Cmd, the line or the text), Home, End,
/// Page Up and Down, Shift to extend a selection, Tab and Shift+Tab (indent and outdent, a block
/// when several lines are selected), and with Ctrl or Cmd: A, C, X, V, Z, Shift+Z or Y, and S (a
/// final [`ValueChange<String>`]). **Escape gives focus up** and F6 and the pad's D-pad move on:
/// Tab is the editor's, so a keyboard player is never trapped.
///
/// **Text** is the host's to give and the editor's to keep: [`CodeEditorValue`] holds a copy, and
/// writes with [`CodeEditorValue::set`] replace the text. The face is [`crate::GooeyFonts::mono`],
/// one column per character and a tab to the next [`CodeIndentation::tab_width`]; a line is never
/// wrapped, it scrolls sideways.
///
/// **Motion.** The view glides ([`crate::ScrollGlide`]'s spring), the caret travels on
/// `motion.snappy` and blinks on [`CodeEditorSettings::blink_period_s`]; under
/// [`MotionSettings::lands_travel`] the view and the caret land at once and the caret is steady.
pub fn code_editor(label: impl Into<String>, text: impl Into<String>) -> impl Bundle {
    let label = label.into();
    let text = text.into();
    (
        Node {
            flex_grow: 1.0,
            align_self: AlignSelf::Stretch,
            ..default()
        },
        Role::CodeEditor,
        CodeEditor::new(label.clone(), &text),
        CodeEditorValue::new(text),
        FocusRegion::host(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        observe(code_on_key),
        observe(code_on_press),
        observe(code_on_drag),
        observe(code_on_drag_end),
        observe(code_on_move),
        observe(code_on_out),
        Children::spawn(bevy::ecs::spawn::SpawnWith(
            move |parts: &mut bevy::ecs::relationship::RelatedSpawner<ChildOf>| {
                let editor = parts.target_entity();
                let part = |kind| CodePart { editor, kind };
                parts.spawn((
                    Node::default(),
                    Role::CodeGutter,
                    part(PartKind::Gutter),
                    Pickable::IGNORE,
                    children![(
                        Node::default(),
                        Role::CodeGutterRows,
                        part(PartKind::GutterRows),
                        Pickable::IGNORE,
                    )],
                ));
                parts.spawn((
                    Node::default(),
                    Role::CodeArea,
                    part(PartKind::Area),
                    Pickable::IGNORE,
                    children![
                        (
                            Node::default(),
                            Role::CodeLineLit,
                            part(PartKind::LineLit),
                            Pickable::IGNORE,
                        ),
                        (
                            Node::default(),
                            Role::CodeContent,
                            part(PartKind::AreaRows),
                            Pickable::IGNORE,
                        ),
                        (
                            Node::default(),
                            Role::CodeCaret,
                            part(PartKind::Caret),
                            Pickable::IGNORE,
                            Visibility::Hidden,
                        ),
                        // Measures one column of the face: hidden, but laid out.
                        (
                            Node {
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            Text::new(NUMBER_PROBE),
                            Role::CodeText,
                            TextLayout::new(Justify::Left, LineBreak::NoWrap),
                            part(PartKind::Probe),
                            Pickable::IGNORE,
                            Visibility::Hidden,
                        ),
                    ],
                ));
            },
        )),
    )
}

/// A skin length of a `physical` one.
fn to_skin(physical: f32, node: &ComputedNode) -> f32 {
    physical * node.inverse_scale_factor()
}

/// A pointer's place (window logical px) in skin px from `node`'s top-left, `None` when the node
/// has no size yet. A pointer is logical px; layout is physical (`target.scale_factor()` of skin px,
/// the window's scale times `UiScale`), so the pointer goes to physical by the window's own scale
/// and the result back by the node's.
fn pointer_in(
    pointer: Vec2,
    node: &ComputedNode,
    at: &UiGlobalTransform,
    target: &ComputedUiRenderTargetInfo,
    ui_scale: f32,
) -> Option<Vec2> {
    let physical = if ui_scale > 0.0 {
        pointer * target.scale_factor() / ui_scale
    } else {
        pointer
    };
    let normal = node.normalize_point(*at, physical)?;
    Some((normal + Vec2::splat(0.5)) * node.size() * node.inverse_scale_factor())
}

/// Whether `c` is one the editor types (a control character is a key, not text).
fn is_text(text: &str) -> bool {
    !text.is_empty() && !text.chars().any(char::is_control)
}

/// Sends the editor's text out after a change: [`CodeEditorValue`] takes it (a new revision) and
/// one [`ValueChange<String>`] says so; `is_final` sends one even if nothing changed.
pub(crate) fn publish(
    root: Entity,
    editor: &mut CodeEditor,
    value: &mut CodeEditorValue,
    is_final: bool,
    commands: &mut Commands,
) {
    let changed = editor.buffer.revision() != editor.published;
    if changed {
        value.text = editor.buffer.text();
        value.revision += 1;
        editor.synced = value.revision;
        editor.published = editor.buffer.revision();
        // The text moved under the tint: lines are not where the marks say.
        editor.overlay.external.clear();
    }
    if changed || is_final {
        commands.trigger(ValueChange::<String> {
            source: root,
            value: value.text.clone(),
            is_final,
        });
    }
}

/// A focused editor's keys. The ones it uses stop here; Escape lets go of focus and bubbles on.
#[allow(clippy::too_many_arguments)]
fn code_on_key(
    mut event: On<FocusedInput<KeyboardInput>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Option<Res<Time<Real>>>,
    mut editors: Query<
        (
            &mut CodeEditor,
            &mut CodeEditorValue,
            Option<&CodeCompletions>,
            &mut CodeFind,
        ),
        Without<InteractionDisabled>,
    >,
    clipboard: Option<ResMut<Clipboard>>,
    settings: Option<Res<CodeEditorSettings>>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    let root = event.event_target();
    let Ok((mut editor, mut value, completions, mut find)) = editors.get_mut(root) else {
        return;
    };
    let key = &event.input;
    if key.state != ButtonState::Pressed {
        return;
    }
    let now = time.map_or(0.0, |time| time.elapsed_secs_f64());
    let held = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    let shift = held(KeyCode::ShiftLeft, KeyCode::ShiftRight);
    let ctrl = held(KeyCode::ControlLeft, KeyCode::ControlRight);
    let alt = held(KeyCode::AltLeft, KeyCode::AltRight);
    let sup = held(KeyCode::SuperLeft, KeyCode::SuperRight);
    let command = ctrl || sup;
    let mut clipboard = clipboard;
    let page = editor.page();

    // An open completion list takes the keys that choose and close.
    if editor.overlay.completion.is_open() && !command {
        let len = editor.overlay.completion.filtered.len();
        let mut taken = true;
        match key.key_code {
            KeyCode::ArrowDown => {
                let next = (editor.overlay.completion.selected + 1) % len;
                editor.overlay.completion.selected = next;
            }
            KeyCode::ArrowUp => {
                let next = (editor.overlay.completion.selected + len - 1) % len;
                editor.overlay.completion.selected = next;
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Tab => {
                let chosen = completions.and_then(|c| {
                    let item = editor
                        .overlay
                        .completion
                        .filtered
                        .get(editor.overlay.completion.selected)
                        .and_then(|&i| c.items.get(i))?;
                    Some((c.anchor, item.text().to_owned()))
                });
                if let Some((anchor, text)) = chosen {
                    let from = editor.buffer.pos_at(anchor);
                    let caret = editor.buffer.caret();
                    editor.buffer.replace_span(from, caret, &text, now);
                    editor.overlay.completion.filtered.clear();
                    commands.entity(root).remove::<CodeCompletions>();
                    event.propagate(false);
                    editor.touched();
                    publish(root, &mut editor, &mut value, false, &mut commands);
                    return;
                }
                taken = false;
            }
            KeyCode::Escape => {
                editor.overlay.completion.filtered.clear();
                commands.entity(root).remove::<CodeCompletions>();
            }
            _ => taken = false,
        }
        if taken {
            event.propagate(false);
            editor.blink = 0.0;
            return;
        }
    }

    let mut final_save = false;
    let mut handled = true;
    let mut asks_completion = None;
    match key.key_code {
        KeyCode::ArrowLeft | KeyCode::ArrowRight => {
            let left = key.key_code == KeyCode::ArrowLeft;
            let movement = match (sup, ctrl || alt, left) {
                (true, _, true) => CodeMove::Home,
                (true, _, false) => CodeMove::LineEnd,
                (false, true, true) => CodeMove::WordLeft,
                (false, true, false) => CodeMove::WordRight,
                (false, false, true) => CodeMove::Left,
                (false, false, false) => CodeMove::Right,
            };
            editor.buffer.move_to(movement, shift);
        }
        KeyCode::ArrowUp | KeyCode::ArrowDown => {
            let up = key.key_code == KeyCode::ArrowUp;
            match (sup, up) {
                (true, true) => editor.buffer.move_to(CodeMove::DocStart, shift),
                (true, false) => editor.buffer.move_to(CodeMove::DocEnd, shift),
                (false, up) => editor.move_rows(up, 1, shift),
            }
        }
        KeyCode::PageUp => editor.move_rows(true, page, shift),
        KeyCode::PageDown => editor.move_rows(false, page, shift),
        KeyCode::Home if ctrl => editor.buffer.move_to(CodeMove::DocStart, shift),
        KeyCode::End if ctrl => editor.buffer.move_to(CodeMove::DocEnd, shift),
        KeyCode::Home => editor.buffer.move_to(CodeMove::Home, shift),
        KeyCode::End => editor.buffer.move_to(CodeMove::LineEnd, shift),
        KeyCode::Backspace => {
            editor.buffer.backspace(ctrl || alt, now);
        }
        KeyCode::Delete => {
            editor.buffer.delete_forward(ctrl || alt, now);
        }
        KeyCode::Enter | KeyCode::NumpadEnter if !command => {
            editor.buffer.newline_here(now);
        }
        KeyCode::Tab if !ctrl && !alt && !sup => {
            if shift {
                editor.buffer.outdent_here(now);
            } else {
                editor.buffer.indent_here(now);
            }
        }
        KeyCode::Escape => {
            if editor.overlay.note.is_some() {
                // The first Escape puts away the note; the next lets go of focus.
                editor.overlay.note_pinned = false;
                editor.overlay.resting = None;
            } else {
                focus.clear();
                handled = false;
            }
        }
        KeyCode::Space if ctrl => asks_completion = Some(CompletionTrigger::Manual),
        KeyCode::Period if command => editor.overlay.note_request = true,
        KeyCode::BracketLeft if command && shift => editor.fold_at_caret(true),
        KeyCode::BracketRight if command && shift => editor.fold_at_caret(false),
        KeyCode::KeyF if command => {
            let seed = editor.buffer.selected_text();
            find.show(alt, &seed);
        }
        KeyCode::KeyA if command => editor.buffer.select_all(),
        KeyCode::KeyC if command => {
            if let (true, Some(clipboard)) = (editor.buffer.has_selection(), clipboard.as_mut()) {
                let _ = clipboard.set_text(editor.buffer.selected_text());
            }
        }
        KeyCode::KeyX if command => {
            if let Some(clipboard) = clipboard.as_mut()
                && let Some(text) = editor.buffer.cut(now)
            {
                let _ = clipboard.set_text(text);
            }
        }
        KeyCode::KeyV if command => {
            if let Some(clipboard) = clipboard.as_mut() {
                editor.pending_paste = Some(clipboard.fetch_text());
            }
        }
        KeyCode::KeyZ if command && shift => {
            editor.buffer.redo();
        }
        KeyCode::KeyZ if command => {
            editor.buffer.undo();
        }
        KeyCode::KeyY if command => {
            editor.buffer.redo();
        }
        KeyCode::KeyS if command => final_save = true,
        _ if command => handled = false,
        _ => match key.text.as_deref() {
            Some(text) if is_text(text) => {
                editor.buffer.insert(text, now);
                let typed = text.chars().next_back();
                let triggers = settings
                    .as_deref()
                    .map_or(DEFAULT_COMPLETION_TRIGGERS, |s| {
                        s.completion_triggers.as_str()
                    });
                let auto = settings.as_deref().is_none_or(|s| s.complete_while_typing);
                if let Some(c) = typed {
                    if auto
                        && (c.is_alphanumeric() || c == '_')
                        && !editor.overlay.completion.is_open()
                    {
                        asks_completion = Some(CompletionTrigger::Typing);
                    } else if triggers.contains(c) {
                        asks_completion = Some(CompletionTrigger::Character);
                    }
                }
            }
            _ => handled = false,
        },
    }
    if handled {
        event.propagate(false);
        editor.keep_caret_out_of_folds();
        editor.touched();
        if let Some(trigger) = asks_completion {
            editor.overlay.complete_request = Some(trigger);
        }
        publish(root, &mut editor, &mut value, final_save, &mut commands);
    }
}

/// A press on the editor: focus, and where to put the caret ([`update_code_editors`] has the
/// geometry).
fn code_on_press(
    mut press: On<PointerPress>,
    mut editors: Query<&mut CodeEditor, Without<InteractionDisabled>>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    let root = press.event_target();
    let Ok(mut editor) = editors.get_mut(root) else {
        return;
    };
    press.propagate(false);
    editor.gesture = Some(Gesture::Press(press.pointer.position));
    if focus.get() != Some(root) {
        focus.set(root, FocusCause::Pressed);
    }
    if visible.0 {
        visible.0 = false;
    }
}

/// A drag from the editor: the selection follows the pointer, scrolling when it leaves.
fn code_on_drag(
    mut drag: On<PointerDrag>,
    mut editors: Query<&mut CodeEditor, Without<InteractionDisabled>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let Ok(mut editor) = editors.get_mut(drag.event_target()) else {
        return;
    };
    drag.propagate(false);
    editor.gesture = Some(Gesture::Drag(drag.pointer.position));
}

fn code_on_drag_end(
    mut end: On<PointerDragEnd>,
    mut editors: Query<&mut CodeEditor, Without<InteractionDisabled>>,
) {
    if let Ok(mut editor) = editors.get_mut(end.event_target()) {
        editor.pivot = None;
        editor.overlay.minimap_drag = false;
        end.propagate(false);
    }
}

/// A wheel turn over an editor moves its view's goal: one line is
/// [`MouseScrollPixelsPerLine`] logical px, and the view glides there.
pub(crate) fn code_on_wheel(
    mut wheels: Option<MessageReader<MouseWheel>>,
    hover: Option<Res<bevy::picking::hover::HoverMap>>,
    mut editors: Query<&mut CodeEditor>,
    parents: Query<&ChildOf>,
    ratio: Option<Res<MouseScrollPixelsPerLine>>,
    ui_scale: Option<Res<UiScale>>,
) {
    let Some(wheels) = wheels.as_mut() else {
        return;
    };
    let lines = ratio.as_deref().copied().unwrap_or_default();
    let mut pixels = Vec2::ZERO;
    for wheel in wheels.read() {
        pixels += match wheel.unit {
            MouseScrollUnit::Line => Vec2::new(wheel.x, wheel.y) * lines,
            MouseScrollUnit::Pixel => Vec2::new(wheel.x, wheel.y),
        };
    }
    if pixels == Vec2::ZERO {
        return;
    }
    let Some(hover) = hover else {
        return;
    };
    let skin = pixels
        / ui_scale
            .as_ref()
            .map_or(1.0, |scale| scale.0)
            .max(f32::EPSILON);
    for (_, hits) in hover.iter() {
        let Some((&hit, _)) = hits.iter().min_by(|a, b| a.1.depth.total_cmp(&b.1.depth)) else {
            continue;
        };
        let editor = std::iter::once(hit)
            .chain(parents.iter_ancestors(hit))
            .find(|&entity| editors.contains(entity));
        if let Some(editor) = editor
            && let Ok(mut editor) = editors.get_mut(editor)
        {
            let base = editor.goal;
            editor.goal = base - Vec2::new(skin.x, skin.y);
        }
    }
}

/// Puts the player's focus and keys on the marker a host reads ([`TextEditing`]) while an editor
/// has them.
pub(crate) fn mark_code_editing(
    focus: Res<InputFocus>,
    editors: Query<(Entity, Has<TextEditing>), With<CodeEditor>>,
    mut commands: Commands,
) {
    for (entity, marked) in &editors {
        let focused = focus.get() == Some(entity);
        if focused && !marked {
            commands.entity(entity).insert(TextEditing);
        } else if !focused && marked {
            commands.entity(entity).remove::<TextEditing>();
        }
    }
}

/// The selection's display columns on `line`: `None` when none of it is there. A line the
/// selection runs through to the next one is one column longer, so the break shows selected.
fn selected_cols(buffer: &CodeBuffer, line: usize) -> Option<(usize, usize)> {
    let sel = buffer.selection();
    if sel.is_empty() || line < sel.start().line || line > sel.end().line {
        return None;
    }
    let tab = buffer.tab_width();
    let text = buffer.line(line);
    let from = if line == sel.start().line {
        display_col(text, sel.start().col, tab)
    } else {
        0
    };
    let to = if line == sel.end().line {
        display_col(text, sel.end().col, tab)
    } else {
        display_width(text, tab) + 1
    };
    (to > from).then_some((from, to))
}

/// What [`update_code_editors`] writes on a part: its node, its words, its visibility and its role.
type PartItem = (
    &'static CodePart,
    &'static mut Node,
    Option<&'static mut Text>,
    Option<&'static mut Visibility>,
    Option<&'static mut RowShown>,
    &'static mut Role,
    Option<&'static mut RowSpans>,
);

/// The whole editor, once a frame: takes what the host and the player asked for (a replaced text,
/// a press or a drag, a paste, IME), brings the caret into view, glides the view and the caret,
/// and re-points the pool of rows at the lines in view. Runs in [`crate::GooeySystems::Style`],
/// before [`crate::apply_skin`].
///
/// Everything it writes it writes only on a difference, so a settled editor costs a look.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub(crate) fn update_code_editors(
    mut editors: Query<(
        Entity,
        &mut CodeEditor,
        &mut CodeEditorValue,
        &CodeIndentation,
        Option<&ComputedUiRenderTargetInfo>,
        Option<&ClickCount>,
        Option<&Highlights>,
    )>,
    mut parts: Query<PartItem>,
    scan: Query<(Entity, &CodePart)>,
    boxes: Query<(&ComputedNode, &UiGlobalTransform)>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    settings: Option<Res<MotionSettings>>,
    blink: Option<Res<CodeEditorSettings>>,
    time: Option<Res<Time<Real>>>,
    focus: Res<InputFocus>,
    ui_scale: Option<Res<UiScale>>,
    mut ime: Option<MessageReader<Ime>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut spans: Query<(&mut TextSpan, &mut Role), Without<CodePart>>,
    mut commands: Commands,
) {
    let settings = settings.as_deref().cloned().unwrap_or_default();
    let blink_period = blink
        .as_deref()
        .map_or(CARET_BLINK_PERIOD_S, |s| s.blink_period_s);
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let glides = ScrollGlide::glides(&settings, time.as_deref());
    let now = time.as_ref().map_or(0.0, |time| time.elapsed_secs_f64());
    let ui_scale = ui_scale.as_ref().map_or(1.0, |scale| scale.0);
    let commits: Vec<Ime> = ime
        .as_mut()
        .map(|ime| ime.read().cloned().collect())
        .unwrap_or_default();
    let mut ime_focus = None;
    for (root, mut editor_mut, mut value, indentation, target, clicks, highlights) in &mut editors {
        let Some(target) = target else {
            continue;
        };
        let editor = &mut *editor_mut;
        let focused = focus.get() == Some(root);
        let look = skin_of(&skin, root, &scopes, &parents);
        let m = CodeMetrics::of(look, target.scale_factor());

        // An edit made from elsewhere reaches the host's copy in this frame, with no event.
        if editor.overlay.external_dirty {
            editor.overlay.external_dirty = false;
            value.text = editor.buffer.text();
            value.revision += 1;
            editor.synced = value.revision;
            editor.published = editor.buffer.revision();
            editor.starts.0 = u64::MAX;
        }
        // What the host wrote, and what the editor is told to do about indenting.
        if value.revision != editor.synced {
            editor.buffer.load(&value.text);
            editor.synced = value.revision;
            editor.published = editor.buffer.revision();
            editor.reveal = true;
        }
        if editor.buffer.indent() != indentation.indent
            || editor.buffer.tab_width() != usize::from(indentation.tab_width.max(1))
        {
            editor.buffer.set_indent(indentation.indent);
            editor
                .buffer
                .set_tab_width(usize::from(indentation.tab_width.max(1)));
        }

        // The parts.
        if editor.scan {
            let mut found = Parts::default();
            let mut numbers = Vec::new();
            let mut texts = Vec::new();
            let mut bands = Vec::new();
            for (entity, part) in &scan {
                if part.editor != root {
                    continue;
                }
                match part.kind {
                    PartKind::Gutter => found.gutter = Some(entity),
                    PartKind::GutterRows => found.gutter_rows = Some(entity),
                    PartKind::Area => found.area = Some(entity),
                    PartKind::AreaRows => found.area_rows = Some(entity),
                    PartKind::Probe => found.probe = Some(entity),
                    PartKind::LineLit => found.line_lit = Some(entity),
                    PartKind::Caret => found.caret = Some(entity),
                    PartKind::Number(slot) => numbers.push((slot, entity)),
                    PartKind::Text(slot) => texts.push((slot, entity)),
                    PartKind::Band(slot) => bands.push((slot, entity)),
                }
            }
            for list in [&mut numbers, &mut texts, &mut bands] {
                list.sort_by_key(|(slot, _)| *slot);
            }
            found.numbers = numbers.into_iter().map(|(_, e)| e).collect();
            found.texts = texts.into_iter().map(|(_, e)| e).collect();
            found.bands = bands.into_iter().map(|(_, e)| e).collect();
            if found.area.is_some() && found.probe.is_some() && found.caret.is_some() {
                editor.parts = found;
                editor.scan = false;
            } else {
                continue;
            }
        }
        let (Some(area), Some(probe), Some(area_rows), Some(gutter), Some(gutter_rows)) = (
            editor.parts.area,
            editor.parts.probe,
            editor.parts.area_rows,
            editor.parts.gutter,
            editor.parts.gutter_rows,
        ) else {
            continue;
        };
        let (Ok((area_node, area_at)), Ok((probe_node, _))) = (boxes.get(area), boxes.get(probe))
        else {
            continue;
        };
        let size = Vec2::new(
            to_skin(area_node.size().x, area_node),
            to_skin(area_node.size().y, area_node),
        );
        let probe_width = to_skin(probe_node.size().x, probe_node);
        let advance = if probe_width > 0.0 {
            probe_width / NUMBER_PROBE.len() as f32
        } else {
            m.font_px * 0.6
        };
        editor.advance = advance;
        editor.line_px = m.line_px;
        editor.area = size;
        let tab = editor.buffer.tab_width();
        let lines = editor.buffer.len_lines();
        let fold = editor.fold.clone();
        let rows = fold.rows(lines);
        let pad = m.pad_px;

        // IME and a paste that was still on its way.
        if focused {
            let window_scale = target.scale_factor() / ui_scale.max(f32::EPSILON);
            let top_left = area_at.translation - area_node.size() * 0.5;
            let below_caret = Vec2::new(
                pad + editor.caret_x.value - editor.shown_x.value,
                editor.caret_y.value - editor.shown_y.value + m.line_px,
            );
            ime_focus = Some(
                (top_left + below_caret * target.scale_factor()) / window_scale.max(f32::EPSILON),
            );
            for event in &commits {
                match event {
                    Ime::Commit { value: text, .. } if is_text(text) => {
                        editor.buffer.insert(text, now);
                        editor.touched();
                        publish(root, editor, &mut value, false, &mut commands);
                    }
                    Ime::Preedit { value: text, .. } => editor.preedit = text.clone(),
                    Ime::Disabled { .. } => editor.preedit.clear(),
                    _ => {}
                }
            }
        }
        if let Some(mut read) = editor.pending_paste.take() {
            match read.poll_result() {
                Some(Ok(text)) => {
                    editor.buffer.paste(&text, now);
                    editor.touched();
                    publish(root, editor, &mut value, false, &mut commands);
                }
                Some(Err(_)) => {}
                None if matches!(read, ClipboardRead::Taken) => {}
                None => editor.pending_paste = Some(read),
            }
        }

        // The pointer.
        if let Some(gesture) = editor.gesture.take()
            && let Ok((root_node, root_at)) = boxes.get(root)
            && let Some(local) = pointer_in(
                match gesture {
                    Gesture::Press(at) | Gesture::Drag(at) => at,
                },
                root_node,
                root_at,
                target,
                ui_scale,
            )
        {
            // The gutter is left of the text: the pointer is in the editor's own box.
            let gutter_w = boxes
                .get(gutter)
                .map_or(0.0, |(node, _)| to_skin(node.size().x, node));
            let x = local.x - gutter_w - pad + editor.shown_x.value;
            let y = local.y + editor.shown_y.value;
            let row = ((y / m.line_px).floor().max(0.0) as usize).min(rows - 1);
            let line = fold.line_at(row);
            let mut special = false;
            if let Some(map) = editor.overlay.minimap {
                let root_w = to_skin(root_node.size().x, root_node);
                let starts = matches!(gesture, Gesture::Press(_));
                if (starts && local.x >= root_w - map.width)
                    || (!starts && editor.overlay.minimap_drag)
                {
                    editor.overlay.minimap_drag = true;
                    let under = map.first as f32 + local.y / map.pitch;
                    let visible = size.y / m.line_px;
                    editor.goal.y = ((under - visible / 2.0) * m.line_px).max(0.0);
                    special = true;
                }
            }
            if !special
                && matches!(gesture, Gesture::Press(_))
                && local.x < gutter_w
                && editor
                    .overlay
                    .available_folds
                    .iter()
                    .any(|r| r.start == line)
            {
                editor.overlay.fold_requests.push(FoldRequest::Toggle(line));
                special = true;
            }
            let col = (x / advance + 0.5).floor().max(0.0) as usize;
            let at = CodePos::new(line, byte_at_display(editor.buffer.line(line), col, tab));
            match gesture {
                _ if special => {}
                Gesture::Press(_) => {
                    let keys = clicks.map_or(1, |clicks| clicks.0);
                    editor.unit = match keys {
                        0 | 1 => Unit::Char,
                        2 => Unit::Word,
                        _ => Unit::Line,
                    };
                    match editor.unit {
                        Unit::Char => {
                            let anchor = editor.buffer.selection().anchor;
                            let extend = false;
                            if extend {
                                editor.buffer.select(anchor, at);
                            } else {
                                editor.buffer.select(at, at);
                            }
                            editor.pivot = Some((at, at));
                        }
                        Unit::Word => {
                            editor.buffer.select_word_at(at);
                            let sel = editor.buffer.selection();
                            editor.pivot = Some((sel.start(), sel.end()));
                        }
                        Unit::Line => {
                            editor.buffer.select_line(line);
                            let sel = editor.buffer.selection();
                            editor.pivot = Some((sel.start(), sel.end()));
                        }
                    }
                }
                Gesture::Drag(_) => {
                    if let Some((first, last)) = editor.pivot {
                        let (anchor, head) = match editor.unit {
                            Unit::Char => (first, at),
                            Unit::Word => {
                                let mut probe = editor.buffer.clone();
                                probe.select_word_at(at);
                                let sel = probe.selection();
                                if at < first {
                                    (last, sel.start())
                                } else {
                                    (first, sel.end())
                                }
                            }
                            Unit::Line => {
                                let end_of = |line: usize| {
                                    if line + 1 < lines {
                                        CodePos::new(line + 1, 0)
                                    } else {
                                        CodePos::new(line, editor.buffer.line(line).len())
                                    }
                                };
                                if at.line < first.line {
                                    (last, CodePos::new(at.line, 0))
                                } else {
                                    (first, end_of(at.line))
                                }
                            }
                        };
                        editor.buffer.select(anchor, head);
                    }
                }
            }
            if !special {
                editor.touched();
            }
        }

        // Where the caret is, in the text's own px.
        let caret = editor.buffer.caret();
        let mut shown_line = editor.buffer.line(caret.line).to_owned();
        if !editor.preedit.is_empty() {
            shown_line.insert_str(caret.col, &editor.preedit.clone());
        }
        let caret_col = display_col(&shown_line, caret.col + editor.preedit.len(), tab);
        let caret_target = Vec2::new(
            caret_col as f32 * advance,
            fold.row_of(caret.line) as f32 * m.line_px,
        );

        // The view: how far it may go, where the caret asks it to be, and the glide there.
        let longest = if editor.longest.0 == (editor.buffer.revision(), tab) {
            editor.longest.1
        } else {
            let cols = editor.buffer.longest_cols();
            editor.longest = ((editor.buffer.revision(), tab), cols);
            cols
        };
        let max = Vec2::new(
            (longest as f32 * advance + 2.0 * pad + advance - size.x).max(0.0),
            (rows as f32 * m.line_px - size.y).max(0.0),
        );
        if editor.reveal {
            editor.reveal = false;
            let margin_y = m.line_px.min(size.y / 4.0);
            let margin_x = REVEAL_COLUMNS * advance;
            let window = Vec2::new((size.x - 2.0 * pad).max(advance), size.y);
            let mut goal = editor.goal;
            if caret_target.y < goal.y + margin_y {
                goal.y = caret_target.y - margin_y;
            } else if caret_target.y + m.line_px > goal.y + window.y - margin_y {
                goal.y = caret_target.y + m.line_px - window.y + margin_y;
            }
            if caret_target.x < goal.x + margin_x {
                goal.x = caret_target.x - margin_x;
            } else if caret_target.x + advance > goal.x + window.x - margin_x {
                goal.x = caret_target.x + advance - window.x + margin_x;
            }
            editor.goal = goal;
        }
        editor.goal = editor.goal.clamp(Vec2::ZERO, max);
        let spring = crate::scroll_glide::glide_spring(look, &settings);
        let goal = editor.goal;
        for (state, to) in [(&mut editor.shown_x, goal.x), (&mut editor.shown_y, goal.y)] {
            if state.value == to && state.velocity == 0.0 {
                continue;
            }
            if glides {
                if spring_settled(spring, *state, to, SCROLL_REST_PX) {
                    *state = SpringState {
                        value: to,
                        velocity: 0.0,
                    };
                } else {
                    *state = spring_step(spring, *state, to, delta);
                }
            } else {
                *state = SpringState {
                    value: to,
                    velocity: 0.0,
                };
            }
        }
        // A held position that no longer fits (a shorter text) is pulled back at once.
        editor.shown_x.value = editor.shown_x.value.clamp(0.0, max.x);
        editor.shown_y.value = editor.shown_y.value.clamp(0.0, max.y);
        let shown = Vec2::new(editor.shown_x.value, editor.shown_y.value);

        // The caret's own spring.
        let caret_spring = {
            let snappy = look
                .motion
                .spring(MotionToken::Snappy)
                .scaled(settings.duration_scale);
            crate::model::Spring {
                duration: snappy.duration * CARET_SPRING_FRACTION,
                ..snappy
            }
        };
        if !editor.caret_primed || !glides {
            editor.caret_x = SpringState {
                value: caret_target.x,
                velocity: 0.0,
            };
            editor.caret_y = SpringState {
                value: caret_target.y,
                velocity: 0.0,
            };
            editor.caret_primed = true;
        } else {
            for (state, to) in [
                (&mut editor.caret_x, caret_target.x),
                (&mut editor.caret_y, caret_target.y),
            ] {
                if state.value == to && state.velocity == 0.0 {
                    continue;
                }
                if spring_settled(caret_spring, *state, to, SCROLL_REST_PX) {
                    *state = SpringState {
                        value: to,
                        velocity: 0.0,
                    };
                } else {
                    *state = spring_step(caret_spring, *state, to, delta);
                }
            }
        }
        editor.blink += delta;
        let lit = focused
            && (!glides || blink_period <= 0.0 || editor.blink % blink_period < blink_period * 0.5);

        // The rows in view.
        let first = (shown.y / m.line_px).floor().max(0.0) as usize;
        let frac = Vec2::new(
            shown.x - (shown.x / advance).floor() * advance,
            shown.y - first as f32 * m.line_px,
        );
        let first_col = (shown.x / advance).floor().max(0.0) as usize;
        let cols = ((size.x - pad) / advance).ceil().max(1.0) as usize + 3;
        let needed = (size.y / m.line_px).ceil().max(1.0) as usize + 2;
        let digits = lines.to_string().len().max(2);
        let gutter_w = (digits as f32 + 2.0) * advance;
        let revision = editor.buffer.revision();
        editor.view = View {
            ready: true,
            first_row: first,
            first_col,
            cols,
            rows,
            needed,
            shown,
            pad,
            advance,
            line_px: m.line_px,
            gutter_w,
            area: size,
            skin_text: look.text_size * look.text_scale,
        };

        // Grow the pools.
        if editor.parts.texts.len() < needed {
            for slot in editor.parts.texts.len()..needed {
                let text = commands
                    .spawn((
                        Node::default(),
                        Text::new(""),
                        Role::CodeText,
                        TextLayout::new(Justify::Left, LineBreak::NoWrap),
                        RowShown::default(),
                        RowSpans::default(),
                        CodePart {
                            editor: root,
                            kind: PartKind::Text(slot),
                        },
                        Pickable::IGNORE,
                        ChildOf(area_rows),
                    ))
                    .id();
                editor.parts.texts.push(text);
            }
        }
        if editor.parts.numbers.len() < needed {
            for slot in editor.parts.numbers.len()..needed {
                let number = commands
                    .spawn((
                        Node::default(),
                        Text::new(""),
                        Role::CodeNumber,
                        TextLayout::new(Justify::Right, LineBreak::NoWrap),
                        RowShown::default(),
                        CodePart {
                            editor: root,
                            kind: PartKind::Number(slot),
                        },
                        Pickable::IGNORE,
                        ChildOf(gutter_rows),
                    ))
                    .id();
                editor.parts.numbers.push(number);
            }
        }
        if editor.parts.bands.len() < needed {
            for slot in editor.parts.bands.len()..needed {
                let band = commands
                    .spawn((
                        Node::default(),
                        Role::CodeSelection,
                        CodePart {
                            editor: root,
                            kind: PartKind::Band(slot),
                        },
                        Pickable::IGNORE,
                        ChildOf(area),
                    ))
                    .id();
                editor.parts.bands.push(band);
            }
        }

        let set_px = |value: &mut Val, px: f32| {
            let next = Val::Px(px);
            if *value != next {
                *value = next;
            }
        };
        let set_display = |node: &mut Node, display: Display| {
            if node.display != display {
                node.display = display;
            }
        };

        // The gutter's width and its rows' place.
        if let Ok((_, mut node, ..)) = parts.get_mut(gutter) {
            set_px(&mut node.width, gutter_w);
        }
        if let Ok((_, mut node, ..)) = parts.get_mut(gutter_rows) {
            set_px(&mut node.top, -frac.y);
        }
        if let Ok((_, mut node, ..)) = parts.get_mut(area_rows) {
            set_px(&mut node.top, -frac.y);
            set_px(&mut node.left, pad - frac.x);
        }

        let highlights = highlights.filter(|highlights| highlights.revision() == value.revision);
        if highlights.is_some() && editor.starts.0 != revision {
            editor.starts = (revision, editor.buffer.line_starts());
        }
        for slot in 0..editor.parts.texts.len() {
            let row = first + slot;
            let line = if slot < needed && row < rows {
                fold.line_at(row)
            } else {
                lines
            };
            let shown_row = line < lines;
            let text_entity = editor.parts.texts[slot];
            if let Ok((_, mut node, text, _, row_state, _, pool)) = parts.get_mut(text_entity) {
                set_display(
                    &mut node,
                    if slot < needed {
                        Display::Flex
                    } else {
                        Display::None
                    },
                );
                if let (Some(mut text), Some(mut row_state), Some(mut pool)) =
                    (text, row_state, pool)
                {
                    let styled = shown_row
                        && highlights.is_some()
                        && (editor.preedit.is_empty() || line != caret.line);
                    let want = RowShown {
                        line: if shown_row { line } else { usize::MAX },
                        first_col,
                        tab,
                        cols,
                        revision: if editor.preedit.is_empty() || line != caret.line {
                            revision
                        } else {
                            u64::MAX
                        },
                        styled: if styled {
                            highlights.map_or(0, Highlights::id)
                        } else {
                            0
                        },
                        lit: false,
                    };
                    if *row_state != want {
                        if let (true, Some(highlights)) = (styled, highlights) {
                            let runs = line_runs(
                                editor.buffer.line(line),
                                editor.starts.1.get(line).copied().unwrap_or(0),
                                highlights.spans(),
                                tab,
                                first_col,
                                first_col + cols,
                            );
                            if !text.0.is_empty() {
                                text.0.clear();
                            }
                            for (i, run) in runs.iter().enumerate() {
                                let role = run.role.map_or(Role::CodeText, Role::CodeSyntax);
                                if let Some(&span) = pool.0.get(i) {
                                    if let Ok((mut span_text, mut span_role)) = spans.get_mut(span)
                                    {
                                        if span_text.0 != run.text {
                                            span_text.0.clone_from(&run.text);
                                        }
                                        if *span_role != role {
                                            *span_role = role;
                                        }
                                    }
                                } else {
                                    let span = commands
                                        .spawn((
                                            TextSpan(run.text.clone()),
                                            TextFont::default(),
                                            TextColor::default(),
                                            role,
                                            ChildOf(text_entity),
                                        ))
                                        .id();
                                    pool.0.push(span);
                                }
                            }
                            for &span in pool.0.iter().skip(runs.len()) {
                                if let Ok((mut span_text, _)) = spans.get_mut(span)
                                    && !span_text.0.is_empty()
                                {
                                    span_text.0.clear();
                                }
                            }
                        } else {
                            let string = if !shown_row {
                                String::new()
                            } else if !editor.preedit.is_empty() && line == caret.line {
                                expand_slice(&shown_line, first_col, first_col + cols, tab)
                            } else {
                                expand_slice(
                                    editor.buffer.line(line),
                                    first_col,
                                    first_col + cols,
                                    tab,
                                )
                            };
                            if text.0 != string {
                                text.0 = string;
                            }
                            for &span in &pool.0 {
                                if let Ok((mut span_text, _)) = spans.get_mut(span)
                                    && !span_text.0.is_empty()
                                {
                                    span_text.0.clear();
                                }
                            }
                        }
                        *row_state = want;
                    }
                }
            }
            let number_entity = editor.parts.numbers[slot];
            if let Ok((_, mut node, text, _, row_state, mut role, _)) = parts.get_mut(number_entity)
            {
                set_display(
                    &mut node,
                    if slot < needed {
                        Display::Flex
                    } else {
                        Display::None
                    },
                );
                let lit = shown_row && line == caret.line;
                let wanted_role = if lit {
                    Role::CodeNumberLit
                } else {
                    Role::CodeNumber
                };
                if *role != wanted_role {
                    *role = wanted_role;
                }
                if let (Some(mut text), Some(mut row_state)) = (text, row_state) {
                    let want = RowShown {
                        line: if shown_row { line } else { usize::MAX },
                        first_col: 0,
                        tab: 0,
                        cols: digits,
                        revision: 0,
                        styled: 0,
                        lit,
                    };
                    if *row_state != want {
                        let string = if shown_row {
                            (line + 1).to_string()
                        } else {
                            String::new()
                        };
                        if text.0 != string {
                            text.0 = string;
                        }
                        *row_state = want;
                    }
                }
            }
            // The selection's band on this line.
            let band_entity = editor.parts.bands[slot];
            if let Ok((_, mut node, ..)) = parts.get_mut(band_entity) {
                let cols_here = if shown_row {
                    selected_cols(&editor.buffer, line)
                } else {
                    None
                };
                match cols_here {
                    Some((from, to)) if slot < needed => {
                        set_display(&mut node, Display::Flex);
                        set_px(&mut node.left, pad + from as f32 * advance - shown.x);
                        set_px(&mut node.width, (to - from) as f32 * advance);
                        set_px(&mut node.top, row as f32 * m.line_px - shown.y);
                    }
                    _ => set_display(&mut node, Display::None),
                }
            }
        }
        if let Some(lit_entity) = editor.parts.line_lit
            && let Ok((_, mut node, ..)) = parts.get_mut(lit_entity)
        {
            set_px(
                &mut node.top,
                fold.row_of(caret.line) as f32 * m.line_px - shown.y,
            );
        }
        if let Some(caret_entity) = editor.parts.caret
            && let Ok((_, mut node, _, visibility, ..)) = parts.get_mut(caret_entity)
        {
            set_px(&mut node.left, pad + editor.caret_x.value - shown.x);
            set_px(&mut node.top, editor.caret_y.value - shown.y);
            if let Some(mut visibility) = visibility {
                let want = if lit {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != want {
                    *visibility = want;
                }
            }
        }
    }
    // The platform's input method follows whichever editor has focus, and its candidate window
    // stands under the caret.
    if let Ok(mut window) = windows.single_mut() {
        let enabled = ime_focus.is_some();
        if window.ime_enabled != enabled {
            window.ime_enabled = enabled;
        }
        if let Some(at) = ime_focus
            && window.ime_position != at
        {
            window.ime_position = at;
        }
    }
}

/// How near a spring is to its goal to count as there: a tenth of a pixel.
const SCROLL_REST_PX: f32 = 0.1;
