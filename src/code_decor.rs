//! What a host draws onto a [`code_editor`](crate::code_editor): syntax colours, diagnostics, a
//! completion list, a hover note and the lines someone else edited. The editor stays
//! language-agnostic: the host knows its language and its schema and says *where* and *what*; the
//! editor knows how to draw it.
//!
//! **Decorations belong to a revision.** [`Highlights`] and [`Diagnostics`] carry the
//! [`CodeEditorValue::revision`](crate::CodeEditorValue::revision) they were computed for, and the
//! editor draws them only while the text is still at that revision: after the player types a
//! character, the old spans would be on the wrong text, so they are dropped (never drawn on new
//! text) until the host sends ones for the new revision. A host that answers a
//! [`ValueChange<String>`](bevy::ui_widgets::ValueChange) in its observer sends them in the same
//! frame, so the player never sees the gap.

use std::ops::Range;

use bevy::prelude::*;

/// What a span of code is, for [`Highlights`]. The skin gives each a colour
/// ([`GooeySkin::syntax`](crate::GooeySkin::syntax)): derived from its seeds like its other
/// semantic colours, and held at the contrast bar on the editor's own ground in every preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, PartialEq, Hash, Clone)]
pub enum SyntaxRole {
    /// A key of a map or a record: a YAML key, a field name.
    Key,
    /// A string literal.
    String,
    /// A number literal.
    Number,
    /// A keyword of the language.
    Keyword,
    /// A comment: the skin's muted text.
    Comment,
    /// A name that points at something else: a variable, a path, an alias.
    Reference,
    /// A function or method name.
    Function,
    /// A type name.
    Type,
    /// An operator.
    Operator,
    /// Something wrong: the skin's danger colour.
    Error,
}

impl SyntaxRole {
    /// Every role, in the order the skin checks them.
    pub const ALL: [SyntaxRole; 10] = [
        Self::Key,
        Self::String,
        Self::Number,
        Self::Keyword,
        Self::Comment,
        Self::Reference,
        Self::Function,
        Self::Type,
        Self::Operator,
        Self::Error,
    ];
}

/// Syntax colours for a [`code_editor`](crate::code_editor): byte ranges of the whole text, each a
/// [`SyntaxRole`], for the text at one revision. Put it on the editor; send a new one when the text
/// changes (the old one is dropped, not drawn).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn colour(
///     change: On<ValueChange<String>>,
///     values: Query<&CodeEditorValue>,
///     mut commands: Commands,
/// ) {
///     let Ok(value) = values.get(change.source) else { return };
///     let spans = value.text().match_indices("let").map(|(at, word)| (at..at + word.len(), SyntaxRole::Keyword));
///     commands.entity(change.source).insert(Highlights::new(value.revision(), spans));
/// }
/// ```
///
/// Spans are sorted on the way in, and one that starts inside the one before it is cut to start
/// where that one ends. A range that is not on character boundaries of the text is ignored when it
/// is drawn.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct Highlights {
    id: u64,
    revision: u64,
    spans: Vec<(Range<usize>, SyntaxRole)>,
}

impl Highlights {
    /// Highlights of the text at `revision`.
    pub fn new(revision: u64, spans: impl IntoIterator<Item = (Range<usize>, SyntaxRole)>) -> Self {
        let mut spans: Vec<_> = spans
            .into_iter()
            .filter(|(range, _)| range.end > range.start)
            .collect();
        spans.sort_by_key(|(range, _)| range.start);
        let mut end = 0;
        spans.retain_mut(|(range, _)| {
            range.start = range.start.max(end);
            end = end.max(range.end);
            range.end > range.start
        });
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            revision,
            spans,
        }
    }

    /// A number no other [`Highlights`] has, so a row can tell these from the ones before them.
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// The revision of the text these were computed for.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The spans, sorted and disjoint.
    pub fn spans(&self) -> &[(Range<usize>, SyntaxRole)] {
        &self.spans
    }
}

/// One run of a line: the text as drawn (tabs as spaces) and the role it has, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    /// The text of the run.
    pub text: String,
    /// The role of the span it is in; `None` for plain text.
    pub role: Option<SyntaxRole>,
}

/// The runs of the display columns `from..to` of `line`, which starts at byte `line_start` of the
/// whole text: each maximal stretch of one role, a tab as the spaces it spans. `spans` are
/// [`Highlights::spans`].
pub fn line_runs(
    line: &str,
    line_start: usize,
    spans: &[(Range<usize>, SyntaxRole)],
    tab: usize,
    from: usize,
    to: usize,
) -> Vec<Run> {
    let tab = tab.max(1);
    let mut runs: Vec<Run> = Vec::new();
    let mut span = spans.partition_point(|(range, _)| range.end <= line_start);
    let mut col = 0;
    for (i, c) in line.char_indices() {
        if col >= to {
            break;
        }
        let at = line_start + i;
        while spans.get(span).is_some_and(|(range, _)| range.end <= at) {
            span += 1;
        }
        let role = spans
            .get(span)
            .filter(|(range, _)| range.start <= at)
            .map(|&(_, role)| role);
        let width = if c == '\t' { tab - col % tab } else { 1 };
        let (low, high) = (col.max(from), (col + width).min(to));
        if high > low {
            let mut piece = String::new();
            if c == '\t' {
                piece.extend(std::iter::repeat_n(' ', high - low));
            } else {
                piece.push(c);
            }
            match runs.last_mut() {
                Some(last) if last.role == role => last.text.push_str(&piece),
                _ => runs.push(Run { text: piece, role }),
            }
        }
        col += width;
    }
    runs
}

/// How bad a [`Diagnostic`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[reflect(Debug, PartialEq, Hash, Clone)]
pub enum CodeSeverity {
    /// A suggestion: the skin's muted text.
    Hint,
    /// Information: the accent.
    Info,
    /// A thing probably not meant: the skin's caution colour.
    Warning,
    /// A thing wrong: the skin's danger colour.
    Error,
}

/// One thing the host found wrong in a range of the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// The bytes it is about, in the whole text.
    pub range: Range<usize>,
    /// How bad it is.
    pub severity: CodeSeverity,
    /// What it says, shown in a note on hover and on Cmd or Ctrl+.
    pub message: String,
}

impl Diagnostic {
    /// A diagnostic of `severity` over `range`.
    pub fn new(range: Range<usize>, severity: CodeSeverity, message: impl Into<String>) -> Self {
        Self {
            range,
            severity,
            message: message.into(),
        }
    }
}

/// The host's findings for a [`code_editor`](crate::code_editor), for the text at one revision: a
/// squiggle under each range, a mark in the gutter on its first line and in the minimap, and the
/// message in a note on hover or on Cmd or Ctrl+. at the caret. Dropped, like [`Highlights`], once
/// the text moves on.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct Diagnostics {
    revision: u64,
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    /// Findings for the text at `revision`.
    pub fn new(revision: u64, items: impl IntoIterator<Item = Diagnostic>) -> Self {
        Self {
            revision,
            items: items.into_iter().collect(),
        }
    }

    /// The revision of the text these were computed for.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The findings.
    pub fn items(&self) -> &[Diagnostic] {
        &self.items
    }
}

/// Why completion was asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, PartialEq, Hash, Clone)]
pub enum CompletionTrigger {
    /// The player pressed Ctrl+Space.
    Manual,
    /// The player typed a letter, a digit or an underscore, and no list was open.
    Typing,
    /// The player typed one of [`CodeEditorSettings::completion_triggers`](crate::CodeEditorSettings).
    Character,
}

/// The editor asks the host for completions at the caret. Observe it on the editor, and answer with
/// [`CodeCompletions`] (an empty list, or none, closes the popup).
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq)]
pub struct CodeCompletionRequested {
    /// The editor.
    #[event_target]
    pub entity: Entity,
    /// Where the caret is, a byte offset into the whole text.
    pub offset: usize,
    /// The word the caret is in or at the end of, up to the caret: what the list narrows by.
    pub prefix: String,
    /// Why it was asked.
    pub trigger: CompletionTrigger,
    /// The revision of the text it was asked on.
    pub revision: u64,
}

/// One thing the host offers to complete with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionItem {
    /// What the list shows and what the player's typing narrows by.
    pub label: String,
    /// A line of what it is, beside the label (a type, a signature, a unit).
    pub detail: String,
    /// What replaces the word: the label unless set.
    pub insert: Option<String>,
}

impl CompletionItem {
    /// An item shown as `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: String::new(),
            insert: None,
        }
    }

    /// This item with its detail line.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    /// This item inserting `text` instead of its label.
    pub fn insert(mut self, text: impl Into<String>) -> Self {
        self.insert = Some(text.into());
        self
    }

    /// What the item puts in the text.
    pub fn text(&self) -> &str {
        self.insert.as_deref().unwrap_or(&self.label)
    }
}

/// The host's answer to a [`CodeCompletionRequested`]: put it on the editor. The list opens under
/// the caret, and narrows by what the player types after `anchor` (the start of the word, a byte
/// offset into the text); Up and Down move the lit row, Enter or Tab chooses it, Escape closes it.
/// It closes when the caret leaves the word, and a choice replaces `anchor..caret` with the item's
/// text, as one undo step.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct CodeCompletions {
    /// Where the word being completed starts.
    pub anchor: usize,
    /// What is offered.
    pub items: Vec<CompletionItem>,
}

/// The editor asks the host what to say about the text under the pointer, which has rested there:
/// answer with [`CodeHover`].
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq)]
pub struct CodeHoverRequested {
    /// The editor.
    #[event_target]
    pub entity: Entity,
    /// The byte offset of the character under the pointer.
    pub offset: usize,
    /// The revision of the text it was asked on.
    pub revision: u64,
}

/// The host's answer to a [`CodeHoverRequested`], for the text at one revision: `text` in a note by
/// `range`. Dropped when the text moves on.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct CodeHover {
    /// The revision of the text it is about.
    pub revision: u64,
    /// The bytes it is about; the note stays while the pointer is over them.
    pub range: Range<usize>,
    /// What it says.
    pub text: String,
}

/// Someone else, for [`CodeEditor::apply_external_edit`](crate::CodeEditor::apply_external_edit):
/// the lines they change are tinted in their colour, and the tint fades.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeAuthor {
    /// Who, for the host's own use.
    pub name: String,
    /// The tint of the lines they changed.
    pub color: Color,
}

impl CodeAuthor {
    /// An author called `name`, drawn in `color`.
    pub fn new(name: impl Into<String>, color: Color) -> Self {
        Self {
            name: name.into(),
            color,
        }
    }
}

/// Where a [`code_editor`](crate::code_editor)'s text can fold, as the host says it: each range is
/// `start..end` over **lines**: the header is `start`, which stays drawn, and a closed fold hides
/// lines `start + 1..end` (a host that wants a block's closing line to stay drawn gives `end` as that
/// line). Ranges nest, and a range that hides nothing is ignored. The player closes and opens them
/// with a chevron in the gutter (a click) or Cmd or Ctrl+Shift+`[` and `]` at the caret.
///
/// Like [`Highlights`] the ranges are for the text at one revision, and the editor uses them while
/// that is the revision it holds; until the host sends ranges for a new revision the last ones
/// stand (a fold the new ranges no longer have opens). A host that answers a
/// [`ValueChange<String>`](bevy::ui_widgets::ValueChange) in its observer sends them in the same
/// frame.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// // Every indented block of a YAML-ish text folds under the line above it.
/// fn ranges(text: &str) -> Vec<std::ops::Range<usize>> {
///     let lines: Vec<&str> = text.lines().collect();
///     let indent = |l: &str| l.len() - l.trim_start().len();
///     (0..lines.len())
///         .filter_map(|at| {
///             let end = (at + 1..lines.len()).find(|&i| indent(lines[i]) <= indent(lines[at]) && !lines[i].trim().is_empty())
///                 .unwrap_or(lines.len());
///             (end > at + 1 && indent(lines[at + 1]) > indent(lines[at])).then_some(at..end)
///         })
///         .collect()
/// }
/// # fn use_it(mut commands: Commands, editor: Entity, value: &CodeEditorValue) {
/// commands.entity(editor).insert(FoldRanges::new(value.revision(), ranges(value.text())));
/// # }
/// ```
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct FoldRanges {
    revision: u64,
    ranges: Vec<Range<usize>>,
}

impl FoldRanges {
    /// Ranges for the text at `revision`.
    pub fn new(revision: u64, ranges: impl IntoIterator<Item = Range<usize>>) -> Self {
        Self {
            revision,
            ranges: ranges.into_iter().filter(|r| r.end > r.start + 1).collect(),
        }
    }

    /// The revision of the text these were computed for.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The ranges.
    pub fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}

/// How long the tint of an external edit takes to fade, seconds.
pub const EXTERNAL_EDIT_FADE_S: f32 = 2.0;

/// How long the lines an external edit changed take to glide in, seconds (on the skin's spring:
/// this is the least a skin's own duration may make it; under reduced motion they land at once).
pub const EXTERNAL_EDIT_GLIDE_S: f32 = 0.25;

/// How far, skin px, the lines an external edit changed start to the left of where they land.
pub const EXTERNAL_EDIT_GLIDE_PX: f32 = 16.0;

/// How opaque the author's tint is at its brightest.
pub const EXTERNAL_TINT_ALPHA: f32 = 0.3;

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(v: &[(Range<usize>, SyntaxRole)]) -> Vec<(Range<usize>, SyntaxRole)> {
        v.to_vec()
    }

    #[test]
    fn highlights_are_sorted_disjoint_and_never_empty() {
        let h = Highlights::new(
            3,
            [
                (10..14, SyntaxRole::String),
                (0..5, SyntaxRole::Key),
                (4..8, SyntaxRole::Number),
                (9..9, SyntaxRole::Error),
            ],
        );
        assert_eq!(h.revision(), 3);
        assert_eq!(
            h.spans(),
            spans(&[
                (0..5, SyntaxRole::Key),
                (5..8, SyntaxRole::Number),
                (10..14, SyntaxRole::String)
            ])
        );
    }

    #[test]
    fn runs_split_by_role_and_expand_tabs_inside_the_columns_in_view() {
        let spans = spans(&[(0..3, SyntaxRole::Keyword), (6..9, SyntaxRole::String)]);
        let runs = line_runs("let x\t\"ab\"", 0, &spans, 4, 0, 40);
        assert_eq!(
            runs,
            vec![
                Run {
                    text: "let".into(),
                    role: Some(SyntaxRole::Keyword)
                },
                // The tab at column 5 runs to the stop at 8: three spaces, in the plain run.
                Run {
                    text: " x   ".into(),
                    role: None
                },
                Run {
                    text: "\"ab".into(),
                    role: Some(SyntaxRole::String)
                },
                Run {
                    text: "\"".into(),
                    role: None
                },
            ]
        );
        // A window narrower than the line cuts the runs, a tab as far as it shows.
        let cut = line_runs("let x\t\"ab\"", 0, &spans, 4, 2, 7);
        assert_eq!(
            cut.iter().map(|r| r.text.as_str()).collect::<String>(),
            "t x  "
        );
        assert_eq!(cut[0].role, Some(SyntaxRole::Keyword));
        // A line that starts later in the text finds its own spans.
        let later = line_runs("abc", 100, &[(101..102, SyntaxRole::Number)], 4, 0, 10);
        assert_eq!(later.len(), 3);
        assert_eq!(
            later[1],
            Run {
                text: "b".into(),
                role: Some(SyntaxRole::Number)
            }
        );
    }
}
