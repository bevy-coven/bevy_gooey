//! What is drawn over a [`code_editor`](crate::code_editor)'s text, and what the host and the player
//! ask of it there: squiggles and gutter marks for [`Diagnostics`], a note for a diagnostic or a
//! [`CodeHover`], the completion list, and the lines someone else edited
//! ([`CodeEditor::apply_external_edit`]).
//!
//! Everything here is built on demand and drawn in the editor's own area, so a settled editor with
//! nothing to say owns none of it. The pools of squiggles, marks and bands grow to what is in view
//! and are written only on a difference.

use std::ops::Range;

use bevy::picking::Pickable;
use bevy::picking::events::{PointerMove, PointerOut};
use bevy::prelude::*;
use bevy::text::{LineBreak, TextLayout};
use bevy::ui::{ComputedNode, UiGlobalTransform};

use crate::code_buffer::{
    CodeBuffer, CodeEdit, CodePos, byte_at_display, display_col, display_width,
};
use crate::code_decor::{
    CodeAuthor, CodeCompletionRequested, CodeCompletions, CodeHover, CodeHoverRequested,
    CodeSeverity, CompletionTrigger, Diagnostics, EXTERNAL_EDIT_FADE_S, EXTERNAL_EDIT_GLIDE_PX,
    EXTERNAL_EDIT_GLIDE_S, EXTERNAL_TINT_ALPHA, FoldRanges,
};
use crate::code_editor::{CodeEditor, CodeEditorValue, CodePart, RowShown};
use crate::code_fold::FoldMap;
use crate::model::fuzzy_score;
use crate::motion::MotionSettings;
use crate::scroll_glide::ScrollGlide;
use crate::skin::Role;

/// How long the pointer rests on a character before the editor asks what it is (or shows the
/// diagnostic under it), seconds.
pub const CODE_HOVER_DELAY_S: f64 = 0.5;

/// How many rows of the completion list show at once.
pub const COMPLETION_ROWS: usize = 8;

/// How wide a note may be, in body text sizes; longer words wrap.
pub(crate) const CODE_NOTE_EM: f32 = 32.0;
/// How wide the completion list is at least, in body text sizes.
pub(crate) const CODE_COMPLETION_EM: f32 = 16.0;
/// The minimap's width, in body text sizes.
pub(crate) const MINIMAP_EM: f32 = 9.0;
/// A minimap line's bar is this high, and the pitch between lines is twice it, skin px.
pub(crate) const MINIMAP_BAR_PX: f32 = 1.5;
/// A minimap diagnostic mark's width, skin px.
pub(crate) const MINIMAP_MARK_PX: f32 = 3.0;
/// The minimap's ground, a line's bar, and the band over the part in view: opacities.
pub(crate) const MINIMAP_GROUND_ALPHA: f32 = 0.35;
pub(crate) const MINIMAP_BAR_ALPHA: f32 = 0.55;
pub(crate) const MINIMAP_BAND_ALPHA: f32 = 0.2;
/// The lit brackets' fill, opacity.
pub(crate) const BRACKET_ALPHA: f32 = 0.25;
/// A find match's fill, and the current match's, opacities.
pub(crate) const MATCH_ALPHA: f32 = 0.3;
pub(crate) const MATCH_LIT_ALPHA: f32 = 0.55;

/// A part the overlay system owns, so its pools do not meet the rows' queries.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct OverlayPart;

/// Lines someone else changed, and the tint they carry while it fades.
#[derive(Clone, Debug)]
pub(crate) struct ExternalMark {
    /// The first and last line (inclusive) in the text as it stood after the edit.
    pub(crate) first: usize,
    pub(crate) last: usize,
    pub(crate) color: Color,
    /// Seconds since the edit.
    pub(crate) age: f32,
}

/// The note under way: its entity and what it says, and the bytes it is about.
#[derive(Clone, Debug)]
pub(crate) struct Note {
    pub(crate) entity: Entity,
    pub(crate) text: String,
    pub(crate) at: CodePos,
}

/// The pointer's last place over the editor (window logical px), and when it came to rest.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Resting {
    pub(crate) at: Vec2,
    pub(crate) since: f64,
}

/// The completion list's state.
#[derive(Clone, Debug, Default)]
pub(crate) struct Completion {
    /// Whether a list is showing, with this many rows narrowed to, indexes into the items.
    pub(crate) filtered: Vec<usize>,
    pub(crate) selected: usize,
    pub(crate) first: usize,
    pub(crate) prefix: String,
    pub(crate) panel: Option<Entity>,
    pub(crate) rows: Vec<(Entity, Entity, Entity)>,
}

impl Completion {
    pub(crate) fn is_open(&self) -> bool {
        !self.filtered.is_empty()
    }
}

/// Everything the overlay system keeps for one editor.
#[derive(Default)]
pub(crate) struct Overlay {
    pub(crate) external: Vec<ExternalMark>,
    pub(crate) external_dirty: bool,
    pub(crate) external_bands: Vec<Entity>,
    pub(crate) squiggles: Vec<Entity>,
    pub(crate) marks: Vec<Entity>,
    pub(crate) note: Option<Note>,
    /// Cmd or Ctrl+. was pressed: show the note for the caret.
    pub(crate) note_request: bool,
    /// The note for the caret stays until the player does something else.
    pub(crate) note_pinned: bool,
    pub(crate) resting: Option<Resting>,
    /// The offset and revision a hover was last asked for, so a pointer at rest asks once.
    pub(crate) hover_asked: Option<(usize, u64)>,
    pub(crate) completion: Completion,
    /// What the player's last key asks of completion, for the system to send.
    pub(crate) complete_request: Option<CompletionTrigger>,
    /// Folds the player asked to close or open, applied by [`update_code_folds`].
    pub(crate) fold_requests: Vec<FoldRequest>,
    /// The ranges the host last gave for the revision the editor holds.
    pub(crate) available_folds: Vec<Range<usize>>,
    pub(crate) brackets: Vec<Entity>,
    pub(crate) guides: Vec<Entity>,
    pub(crate) chevrons: Vec<Entity>,
    pub(crate) minimap: Option<MinimapView>,
    pub(crate) minimap_drag: bool,
    pub(crate) minimap_root: Option<Entity>,
    pub(crate) minimap_band: Option<Entity>,
    pub(crate) minimap_bars: Vec<Entity>,
    pub(crate) minimap_marks: Vec<Entity>,
    pub(crate) minimap_key: Option<MinimapKey>,
}

/// What the minimap was last drawn for: the first row, the row count, the text's revision, the
/// lines folded away and the diagnostic marks.
pub(crate) type MinimapKey = (usize, usize, u64, usize, Vec<(usize, CodeSeverity)>);

/// What the player asked of a fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FoldRequest {
    /// Close the innermost fold at or around this line.
    Fold(usize),
    /// Open the fold that hides or heads this line.
    Unfold(usize),
    /// Flip the fold headed by this line.
    Toggle(usize),
}

/// Applies the host's [`FoldRanges`] and the player's fold requests: which headers are closed, the
/// map from lines to rows, and a caret a fold hides moved to its header. Runs before
/// [`crate::code_editor::update_code_editors`] so the rows are laid out on this frame's folds.
pub(crate) fn update_code_folds(
    mut editors: Query<(&mut CodeEditor, &CodeEditorValue, Option<&FoldRanges>)>,
) {
    for (mut editor_mut, value, ranges) in &mut editors {
        let editor = &mut *editor_mut;
        let mut dirty = false;
        if let Some(ranges) = ranges
            && ranges.revision() == value.revision()
            && editor.overlay.available_folds != ranges.ranges()
        {
            editor.overlay.available_folds = ranges.ranges().to_vec();
            let available = &editor.overlay.available_folds;
            let before = editor.folded.len();
            editor
                .folded
                .retain(|header| available.iter().any(|range| range.start == *header));
            dirty |= before != editor.folded.len() || !editor.folded.is_empty();
        }
        let requests: Vec<FoldRequest> = std::mem::take(&mut editor.overlay.fold_requests);
        for request in requests {
            let available = &editor.overlay.available_folds;
            match request {
                FoldRequest::Fold(line) => {
                    let header = available
                        .iter()
                        .filter(|range| range.start <= line && line < range.end)
                        .map(|range| range.start)
                        .max();
                    if let Some(header) = header {
                        dirty |= editor.folded.insert(header);
                    }
                }
                FoldRequest::Unfold(line) => {
                    let header = if editor.folded.contains(&line) {
                        Some(line)
                    } else {
                        editor.fold.header_of(line)
                    };
                    if let Some(header) = header {
                        // The closed fold that hides the line, whichever of the nested ones.
                        let heads: Vec<usize> = editor
                            .folded
                            .iter()
                            .copied()
                            .filter(|&h| {
                                available.iter().any(|r| {
                                    r.start == h
                                        && (h == header || (r.start < line && line < r.end))
                                })
                            })
                            .collect();
                        for head in heads {
                            dirty |= editor.folded.remove(&head);
                        }
                    }
                }
                FoldRequest::Toggle(line) => {
                    if editor.folded.remove(&line) {
                        dirty = true;
                    } else if available.iter().any(|range| range.start == line) {
                        editor.folded.insert(line);
                        dirty = true;
                    }
                }
            }
        }
        if dirty || editor.fold_dirty {
            editor.fold_dirty = false;
            let available = editor.overlay.available_folds.clone();
            editor.fold = FoldMap::new(
                editor
                    .folded
                    .iter()
                    .filter_map(|header| available.iter().find(|r| r.start == *header).cloned()),
            );
            editor.keep_caret_out_of_folds();
            editor.reveal = true;
        }
    }
}

impl CodeEditor {
    /// Applies edits made by someone else (`range` is in byte offsets of the text as it stands) with
    /// `author`'s colour. Applied together, they land without moving the player's caret or
    /// selection: both stay on the text they were on (a selection an edit falls inside keeps its
    /// text), the view stays on the text it showed (lines added above it move the scroll), the
    /// lines they changed glide in tinted in the author's colour, and the tint fades over
    /// [`EXTERNAL_EDIT_FADE_S`](crate::EXTERNAL_EDIT_FADE_S). [`CodeEditorValue`] takes the new text
    /// in the same frame, with no [`ValueChange`](bevy::ui_widgets::ValueChange): the host made
    /// the edit.
    ///
    /// The player's undo history is dropped: it records positions in a text that changed under it
    /// (a person's undo never rewrites someone else's text). A later edit by the player clears the
    /// tint.
    ///
    /// ```
    /// # use bevy::prelude::*;
    /// # use bevy_gooey::*;
    /// fn remote(mut editors: Query<&mut CodeEditor>) {
    ///     for mut editor in &mut editors {
    ///         editor.apply_external_edit(
    ///             &[CodeEdit::new(0..0, "// from Ada\n")],
    ///             CodeAuthor::new("Ada", Color::srgb(0.9, 0.5, 0.2)),
    ///         );
    ///     }
    /// }
    /// ```
    pub fn apply_external_edit(&mut self, edits: &[CodeEdit], author: CodeAuthor) {
        if edits.is_empty() {
            return;
        }
        // Lines added above the first line in view move the scroll, so the text in view stays put.
        let first_visible = self.fold.line_at(self.view.first_row);
        let mut above: isize = 0;
        for edit in edits {
            let start = self.buffer.pos_at(edit.range.start);
            if start.line < first_visible {
                let end = self.buffer.pos_at(edit.range.end);
                let removed = self.buffer.text_between(start, end).matches('\n').count() as isize;
                above += edit.text.matches('\n').count() as isize - removed;
            }
        }
        let touched = self.buffer.apply_external(edits);
        let mut at = 0;
        while at < touched.len() {
            let first = touched[at];
            let mut last = first;
            while touched.get(at + 1) == Some(&(last + 1)) {
                last += 1;
                at += 1;
            }
            self.overlay.external.push(ExternalMark {
                first,
                last,
                color: author.color,
                age: 0.0,
            });
            at += 1;
        }
        self.overlay.external_dirty = true;
        if above != 0 {
            let dy = above as f32 * self.line_px;
            self.goal.y = (self.goal.y + dy).max(0.0);
            self.shown_y.value = (self.shown_y.value + dy).max(0.0);
        }
    }
}

/// The pointer rests: remember where, and since when, for [`update_code_overlays`].
pub(crate) fn code_on_move(
    mut moved: On<PointerMove>,
    mut editors: Query<&mut CodeEditor>,
    time: Option<Res<Time<Real>>>,
) {
    let Ok(mut editor) = editors.get_mut(moved.event_target()) else {
        return;
    };
    moved.propagate(false);
    let now = time.map_or(0.0, |time| time.elapsed_secs_f64());
    let at = moved.pointer.position;
    match editor.overlay.resting {
        Some(resting) if resting.at.distance(at) < 2.0 => {}
        _ => editor.overlay.resting = Some(Resting { at, since: now }),
    }
}

/// The pointer left: nothing rests.
pub(crate) fn code_on_out(mut out: On<PointerOut>, mut editors: Query<&mut CodeEditor>) {
    if let Ok(mut editor) = editors.get_mut(out.event_target()) {
        editor.overlay.resting = None;
        out.propagate(false);
    }
}

/// The place of byte `offset` of the text, from the line starts.
pub(crate) fn pos_of(buffer: &CodeBuffer, starts: &[usize], offset: usize) -> CodePos {
    let line = starts
        .partition_point(|&start| start <= offset)
        .saturating_sub(1);
    buffer.clamp(CodePos::new(
        line,
        offset - starts.get(line).copied().unwrap_or(0),
    ))
}

fn set_px(value: &mut Val, px: f32) {
    let next = Val::Px(px);
    if *value != next {
        *value = next;
    }
}

fn set_display(node: &mut Node, display: Display) {
    if node.display != display {
        node.display = display;
    }
}

/// What the overlay writes on a part.
type OverlayItem = (
    &'static mut Node,
    Option<&'static mut Text>,
    &'static mut Role,
    Option<&'static mut BackgroundColor>,
);

/// A diagnostic's segment on one display row, in display columns.
struct Segment {
    row: usize,
    from: usize,
    to: usize,
    severity: CodeSeverity,
}

/// The overlay of every editor, once a frame, after [`crate::code_editor::update_code_editors`] has
/// laid the view out and before [`crate::apply_skin`] styles what it spawns.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub(crate) fn update_code_overlays(
    mut editors: Query<(
        Entity,
        &mut CodeEditor,
        &CodeEditorValue,
        Option<&Diagnostics>,
        Option<&CodeCompletions>,
        Option<&CodeHover>,
        Option<&bevy::ui::ComputedUiRenderTargetInfo>,
        Has<CodeMinimap>,
    )>,
    mut items: Query<OverlayItem, With<OverlayPart>>,
    mut rows: Query<(&CodePart, &RowShown, &mut UiTransform)>,
    boxes: Query<(&ComputedNode, &UiGlobalTransform)>,
    settings: Option<Res<MotionSettings>>,
    time: Option<Res<Time<Real>>>,
    ui_scale: Option<Res<UiScale>>,
    mut commands: Commands,
) {
    let settings = settings.as_deref().cloned().unwrap_or_default();
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let now = time.as_ref().map_or(0.0, |time| time.elapsed_secs_f64());
    let glides = ScrollGlide::glides(&settings, time.as_deref());
    let ui_scale = ui_scale.as_ref().map_or(1.0, |scale| scale.0);
    for (root, mut editor_mut, value, diagnostics, completions, hover, target, minimap_on) in
        &mut editors
    {
        let editor = &mut *editor_mut;
        if !editor.view.ready {
            continue;
        }
        let Some(area) = editor.parts.area else {
            continue;
        };
        let view = editor.view;
        let fold = editor.fold.clone();
        let lines = editor.buffer.len_lines();
        let tab = editor.buffer.tab_width();
        let revision = editor.buffer.revision();
        if editor.starts.0 != revision {
            editor.starts = (revision, editor.buffer.line_starts());
        }
        let visible_rows = view.first_row..(view.first_row + view.needed).min(view.rows);
        let line_of = |row: usize| fold.line_at(row);

        // -- lines someone else edited: the tint, and the glide -------------------------------
        for mark in &mut editor.overlay.external {
            mark.age += delta;
        }
        editor
            .overlay
            .external
            .retain(|mark| mark.age < EXTERNAL_EDIT_FADE_S);
        let marks: Vec<ExternalMark> = editor.overlay.external.clone();
        let mut bands: Vec<(usize, Color)> = Vec::new();
        for mark in &marks {
            let alpha =
                EXTERNAL_TINT_ALPHA * (1.0 - mark.age / EXTERNAL_EDIT_FADE_S).clamp(0.0, 1.0);
            for line in mark.first..=mark.last.min(lines.saturating_sub(1)) {
                let row = fold.row_of(line);
                if visible_rows.contains(&row) {
                    bands.push((row, mark.color.with_alpha(alpha)));
                }
            }
        }
        grow(
            &mut commands,
            &mut editor.overlay.external_bands,
            bands.len(),
            |commands| {
                commands
                    .spawn((
                        Node::default(),
                        Role::CodeExternal,
                        BackgroundColor(Color::NONE),
                        Pickable::IGNORE,
                        OverlayPart,
                        ChildOf(area),
                    ))
                    .id()
            },
        );
        for (slot, &entity) in editor.overlay.external_bands.iter().enumerate() {
            let Ok((mut node, _, _, background)) = items.get_mut(entity) else {
                continue;
            };
            match bands.get(slot) {
                Some(&(row, color)) => {
                    set_display(&mut node, Display::Flex);
                    set_px(&mut node.top, row as f32 * view.line_px - view.shown.y);
                    if let Some(mut background) = background
                        && background.0 != color
                    {
                        background.0 = color;
                    }
                }
                None => set_display(&mut node, Display::None),
            }
        }
        // The rows glide in from the left while they are new.
        for (part, shown, mut transform) in &mut rows {
            if part.editor_entity() != root {
                continue;
            }
            let mut x = 0.0;
            if shown.line_shown() < lines {
                for mark in &marks {
                    if (mark.first..=mark.last).contains(&shown.line_shown()) {
                        let glide = if glides {
                            (mark.age / EXTERNAL_EDIT_GLIDE_S).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        // Ease out: quick at first, settling.
                        let eased = 1.0 - (1.0 - glide) * (1.0 - glide);
                        x = -(1.0 - eased) * EXTERNAL_EDIT_GLIDE_PX;
                    }
                }
            }
            let want = UiTransform::from_translation(Val2::px(x, 0.0));
            if transform.translation != want.translation {
                transform.translation = want.translation;
            }
        }

        // -- diagnostics: squiggles under the range, a mark in the gutter ---------------------
        let diagnostics = diagnostics.filter(|d| d.revision() == value.revision());
        let mut segments: Vec<Segment> = Vec::new();
        let mut marks_by_row: Vec<(usize, CodeSeverity)> = Vec::new();
        if let Some(diagnostics) = diagnostics {
            for item in diagnostics.items() {
                let start = pos_of(&editor.buffer, &editor.starts.1, item.range.start);
                let end = pos_of(
                    &editor.buffer,
                    &editor.starts.1,
                    item.range.end.max(item.range.start),
                );
                if end.line < line_of(view.first_row)
                    && lines > 0
                    && start.line < line_of(view.first_row)
                {
                    // wholly above the view: only rows in view are drawn
                }
                for row in visible_rows.clone() {
                    let line = line_of(row);
                    if line < start.line || line > end.line {
                        continue;
                    }
                    let text = editor.buffer.line(line);
                    let from = if line == start.line {
                        display_col(text, start.col, tab)
                    } else {
                        0
                    };
                    let mut to = if line == end.line {
                        display_col(text, end.col, tab)
                    } else {
                        display_col(text, text.len(), tab) + 1
                    };
                    if to <= from {
                        to = from + 1;
                    }
                    segments.push(Segment {
                        row,
                        from,
                        to,
                        severity: item.severity,
                    });
                    if line == start.line {
                        match marks_by_row.iter_mut().find(|(r, _)| *r == row) {
                            Some((_, severity)) => *severity = (*severity).max(item.severity),
                            None => marks_by_row.push((row, item.severity)),
                        }
                    }
                }
            }
        }
        grow(
            &mut commands,
            &mut editor.overlay.squiggles,
            segments.len(),
            |commands| {
                commands
                    .spawn((
                        Node::default(),
                        Text::new(""),
                        Role::CodeSquiggle(CodeSeverity::Error),
                        TextLayout::new(Justify::Left, LineBreak::NoWrap),
                        Pickable::IGNORE,
                        OverlayPart,
                        ChildOf(area),
                    ))
                    .id()
            },
        );
        for (slot, &entity) in editor.overlay.squiggles.iter().enumerate() {
            let Ok((mut node, text, mut role, _)) = items.get_mut(entity) else {
                continue;
            };
            let Some(segment) = segments.get(slot) else {
                set_display(&mut node, Display::None);
                continue;
            };
            let from = segment.from.max(view.first_col);
            let to = segment.to.min(view.first_col + view.cols);
            if to <= from {
                set_display(&mut node, Display::None);
                continue;
            }
            set_display(&mut node, Display::Flex);
            set_px(
                &mut node.left,
                view.pad + from as f32 * view.advance - view.shown.x,
            );
            set_px(
                &mut node.top,
                segment.row as f32 * view.line_px - view.shown.y + view.line_px * SQUIGGLE_DROP,
            );
            set_px(&mut node.width, (to - from) as f32 * view.advance);
            let wanted = Role::CodeSquiggle(segment.severity);
            if *role != wanted {
                *role = wanted;
            }
            if let Some(mut text) = text {
                let squiggle = "~".repeat(to - from);
                if text.0 != squiggle {
                    text.0 = squiggle;
                }
            }
        }
        if let Some(gutter) = editor.parts.gutter {
            grow(
                &mut commands,
                &mut editor.overlay.marks,
                marks_by_row.len(),
                |commands| {
                    commands
                        .spawn((
                            Node::default(),
                            Role::CodeMark(CodeSeverity::Error),
                            Pickable::IGNORE,
                            OverlayPart,
                            ChildOf(gutter),
                        ))
                        .id()
                },
            );
            for (slot, &entity) in editor.overlay.marks.iter().enumerate() {
                let Ok((mut node, _, mut role, _)) = items.get_mut(entity) else {
                    continue;
                };
                match marks_by_row.get(slot) {
                    Some(&(row, severity)) => {
                        set_display(&mut node, Display::Flex);
                        set_px(
                            &mut node.top,
                            row as f32 * view.line_px - view.shown.y + view.line_px * MARK_DROP,
                        );
                        let wanted = Role::CodeMark(severity);
                        if *role != wanted {
                            *role = wanted;
                        }
                    }
                    None => set_display(&mut node, Display::None),
                }
            }
        }

        let mark_lines: Vec<(usize, CodeSeverity)> = diagnostics
            .map(|d| {
                d.items()
                    .iter()
                    .map(|item| {
                        (
                            pos_of(&editor.buffer, &editor.starts.1, item.range.start).line,
                            item.severity,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        draw_navigation(
            &mut commands,
            editor,
            area,
            &mut items,
            &boxes,
            minimap_on,
            &mark_lines,
        );
        // -- the note: a diagnostic's message, or the host's answer to a hover -----------------
        let caret = editor.buffer.caret();
        let mut wanted_note: Option<(String, CodePos)> = None;
        let mut asked: Option<usize> = None;
        let note_at = |offset: usize, pos: CodePos| -> Option<(String, CodePos)> {
            let mut said: Vec<&str> = Vec::new();
            if let Some(diagnostics) = diagnostics {
                said.extend(
                    diagnostics
                        .items()
                        .iter()
                        .filter(|d| in_range(&d.range, offset))
                        .map(|d| d.message.as_str()),
                );
            }
            if said.is_empty()
                && let Some(hover) = hover
                && hover.revision == value.revision()
                && in_range(&hover.range, offset)
            {
                said.push(hover.text.as_str());
            }
            (!said.is_empty()).then(|| (said.join("\n"), pos))
        };
        if editor.overlay.note_request {
            editor.overlay.note_request = false;
            editor.overlay.note_pinned = true;
        }
        if editor.overlay.note_pinned {
            let offset = editor.buffer.offset_of(caret);
            wanted_note = note_at(offset, caret);
            if wanted_note.is_none() {
                asked = Some(offset);
            }
        } else if let Some(resting) = editor.overlay.resting
            && now - resting.since >= CODE_HOVER_DELAY_S
            && editor.gesture.is_none()
            && let Some(target) = target
            && let Ok((root_node, root_at)) = boxes.get(root)
            && let Some(local) = pointer_local(resting.at, root_node, root_at, target, ui_scale)
        {
            let x = local.x - view.gutter_w - view.pad + view.shown.x;
            let y = local.y + view.shown.y;
            let row = (y / view.line_px).floor();
            if row >= 0.0 && (row as usize) < view.rows && x >= 0.0 {
                let line = line_of(row as usize);
                let text = editor.buffer.line(line);
                let col = (x / view.advance).floor() as usize;
                if col < display_col(text, text.len(), tab).max(1) {
                    let pos = CodePos::new(line, byte_at_display(text, col, tab).min(text.len()));
                    let offset = editor.buffer.offset_of(pos);
                    wanted_note = note_at(offset, pos);
                    if wanted_note.is_none() {
                        asked = Some(offset);
                    }
                }
            }
        }
        if let Some(offset) = asked
            && editor.overlay.hover_asked != Some((offset, value.revision()))
        {
            editor.overlay.hover_asked = Some((offset, value.revision()));
            commands.trigger(CodeHoverRequested {
                entity: root,
                offset,
                revision: value.revision(),
            });
        }
        let same = match (&editor.overlay.note, &wanted_note) {
            (Some(note), Some((text, at))) => note.text == *text && note.at == *at,
            (None, None) => true,
            _ => false,
        };
        if !same {
            if let Some(note) = editor.overlay.note.take() {
                commands.entity(note.entity).despawn();
            }
            if let Some((text, at)) = wanted_note {
                let entity = commands
                    .spawn((
                        Node::default(),
                        Role::CodeNote,
                        Pickable::IGNORE,
                        OverlayPart,
                        ChildOf(area),
                        children![(
                            Text::new(text.clone()),
                            Role::CodeNoteText,
                            Pickable::IGNORE,
                        )],
                    ))
                    .id();
                editor.overlay.note = Some(Note { entity, text, at });
            }
        }
        if let Some(note) = &editor.overlay.note
            && let Ok((mut node, _, _, _)) = items.get_mut(note.entity)
        {
            let row = fold.row_of(note.at.line);
            let text = editor.buffer.line(note.at.line);
            let col = display_col(text, note.at.col, tab);
            let height = boxes.get(note.entity).map_or(view.line_px * 2.0, |(n, _)| {
                n.size().y * n.inverse_scale_factor()
            });
            let below = (row + 1) as f32 * view.line_px - view.shown.y;
            let top = if below + height > view.area.y {
                (row as f32 * view.line_px - view.shown.y - height).max(0.0)
            } else {
                below
            };
            set_px(&mut node.top, top);
            set_px(
                &mut node.left,
                (view.pad + col as f32 * view.advance - view.shown.x).max(0.0),
            );
        }

        // -- completion -------------------------------------------------------------------------
        if let Some(trigger) = editor.overlay.complete_request.take() {
            let start = word_start(editor.buffer.line(caret.line), caret.col);
            let prefix = editor.buffer.line(caret.line)[start..caret.col].to_owned();
            commands.trigger(CodeCompletionRequested {
                entity: root,
                offset: editor.buffer.offset_of(caret),
                prefix,
                trigger,
                revision: value.revision(),
            });
        }
        update_completion(
            root,
            editor,
            completions,
            &view,
            &fold,
            area,
            &mut items,
            &boxes,
            &mut commands,
        );
        let _ = lines;
    }
}

/// The part of the minimap in view: the first display row it draws, the height of a row and its width.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MinimapView {
    pub(crate) first: usize,
    pub(crate) pitch: f32,
    pub(crate) width: f32,
}

/// How many columns of a line the minimap's bar can show.
const MINIMAP_COLS: usize = 100;

/// The brackets that match, the indent guides, the fold chevrons and the minimap.
#[allow(clippy::too_many_arguments)]
fn draw_navigation(
    commands: &mut Commands,
    editor: &mut CodeEditor,
    area: Entity,
    items: &mut Query<OverlayItem, With<OverlayPart>>,
    boxes: &Query<(&ComputedNode, &UiGlobalTransform)>,
    minimap_on: bool,
    marks: &[(usize, CodeSeverity)],
) {
    let view = editor.view;
    let fold = editor.fold.clone();
    let tab = editor.buffer.tab_width();
    let visible = view.first_row..(view.first_row + view.needed).min(view.rows);

    // The brackets the caret is at.
    let mut spots: Vec<(usize, usize)> = Vec::new();
    if let Some((a, b)) = editor.buffer.matching_bracket() {
        for place in [a, b] {
            if fold.is_hidden(place.line) {
                continue;
            }
            let row = fold.row_of(place.line);
            if visible.contains(&row) {
                let col = display_col(editor.buffer.line(place.line), place.col, tab);
                spots.push((row, col));
            }
        }
    }
    grow(commands, &mut editor.overlay.brackets, 2, |commands| {
        commands
            .spawn((
                Node::default(),
                Role::CodeBracket,
                Pickable::IGNORE,
                OverlayPart,
                ChildOf(area),
            ))
            .id()
    });
    for (slot, &entity) in editor.overlay.brackets.iter().enumerate() {
        let Ok((mut node, ..)) = items.get_mut(entity) else {
            continue;
        };
        match spots.get(slot) {
            Some(&(row, col)) => {
                set_display(&mut node, Display::Flex);
                set_px(
                    &mut node.left,
                    view.pad + col as f32 * view.advance - view.shown.x,
                );
                set_px(&mut node.width, view.advance);
                set_px(&mut node.top, row as f32 * view.line_px - view.shown.y);
            }
            None => set_display(&mut node, Display::None),
        }
    }

    // Indent guides: one tall line per level over each run of rows that are at least that deep.
    let unit = match editor.buffer.indent() {
        crate::code_buffer::CodeIndent::Spaces(n) => usize::from(n).max(1),
        crate::code_buffer::CodeIndent::Tab => tab,
    };
    let mut levels: Vec<usize> = Vec::new();
    let mut last = 0;
    for row in visible.clone() {
        let text = editor.buffer.line(fold.line_at(row));
        let level = if text.trim().is_empty() {
            last
        } else {
            let lead = text.len() - text.trim_start_matches([' ', '\t']).len();
            display_col(text, lead, tab) / unit
        };
        last = level;
        levels.push(level);
    }
    let deepest = levels.iter().copied().max().unwrap_or(0);
    let mut runs: Vec<(usize, usize, usize)> = Vec::new();
    for level in 1..=deepest {
        let mut start: Option<usize> = None;
        for (i, &have) in levels.iter().enumerate() {
            match (have >= level, start) {
                (true, None) => start = Some(i),
                (false, Some(s)) => {
                    runs.push((level, s, i - s));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(s) = start {
            runs.push((level, s, levels.len() - s));
        }
    }
    grow(
        commands,
        &mut editor.overlay.guides,
        runs.len(),
        |commands| {
            commands
                .spawn((
                    Node::default(),
                    Role::CodeGuide,
                    Pickable::IGNORE,
                    OverlayPart,
                    ChildOf(area),
                ))
                .id()
        },
    );
    for (slot, &entity) in editor.overlay.guides.iter().enumerate() {
        let Ok((mut node, ..)) = items.get_mut(entity) else {
            continue;
        };
        match runs.get(slot) {
            Some(&(level, start, len)) => {
                set_display(&mut node, Display::Flex);
                set_px(
                    &mut node.left,
                    view.pad + ((level - 1) * unit) as f32 * view.advance - view.shown.x,
                );
                set_px(
                    &mut node.top,
                    (view.first_row + start) as f32 * view.line_px - view.shown.y,
                );
                set_px(&mut node.height, len as f32 * view.line_px);
            }
            None => set_display(&mut node, Display::None),
        }
    }

    // Chevrons in the gutter, on the lines that head a fold.
    let mut chevrons: Vec<(usize, bool)> = Vec::new();
    for row in visible.clone() {
        let line = fold.line_at(row);
        if editor
            .overlay
            .available_folds
            .iter()
            .any(|r| r.start == line)
        {
            chevrons.push((row, editor.folded.contains(&line)));
        }
    }
    if let Some(gutter) = editor.parts.gutter {
        grow(
            commands,
            &mut editor.overlay.chevrons,
            chevrons.len(),
            |commands| {
                commands
                    .spawn((
                        Node::default(),
                        Text::new(">"),
                        Role::CodeChevron,
                        Pickable::IGNORE,
                        OverlayPart,
                        ChildOf(gutter),
                    ))
                    .id()
            },
        );
        for (slot, &entity) in editor.overlay.chevrons.iter().enumerate() {
            let Ok((mut node, text, ..)) = items.get_mut(entity) else {
                continue;
            };
            match chevrons.get(slot) {
                Some(&(row, closed)) => {
                    set_display(&mut node, Display::Flex);
                    set_px(
                        &mut node.left,
                        (view.gutter_w - view.advance * 1.6).max(0.0),
                    );
                    set_px(&mut node.top, row as f32 * view.line_px - view.shown.y);
                    if let Some(mut text) = text {
                        let glyph = if closed { ">" } else { "v" };
                        if text.0 != glyph {
                            text.0 = glyph.to_owned();
                        }
                    }
                }
                None => set_display(&mut node, Display::None),
            }
        }
    }

    // The minimap down the right edge.
    if !minimap_on {
        editor.overlay.minimap = None;
        if let Some(root) = editor.overlay.minimap_root.take() {
            commands.entity(root).despawn();
            editor.overlay.minimap_bars.clear();
            editor.overlay.minimap_marks.clear();
            editor.overlay.minimap_band = None;
            editor.overlay.minimap_key = None;
        }
        return;
    }
    let root = match editor.overlay.minimap_root {
        Some(root) => root,
        None => {
            let root = commands
                .spawn((
                    Node::default(),
                    Role::CodeMinimap,
                    Pickable::IGNORE,
                    OverlayPart,
                    ChildOf(area),
                ))
                .id();
            let band = commands
                .spawn((
                    Node::default(),
                    Role::CodeMinimapBand,
                    Pickable::IGNORE,
                    OverlayPart,
                    ChildOf(root),
                ))
                .id();
            editor.overlay.minimap_root = Some(root);
            editor.overlay.minimap_band = Some(band);
            return;
        }
    };
    let width = boxes
        .get(root)
        .map_or(view.skin_text * MINIMAP_EM, |(n, _)| {
            n.size().x * n.inverse_scale_factor()
        });
    let pitch = MINIMAP_BAR_PX * 2.0;
    let capacity = ((view.area.y / pitch).floor() as usize).max(1);
    let rows = view.rows;
    let travel = (rows as f32 * view.line_px - view.area.y).max(1.0);
    let first = if rows <= capacity {
        0
    } else {
        (((rows - capacity) as f32) * (view.shown.y / travel).clamp(0.0, 1.0)).round() as usize
    };
    let count = capacity.min(rows - first);
    editor.overlay.minimap = Some(MinimapView {
        first,
        pitch,
        width,
    });
    let key = (
        first,
        count,
        editor.buffer.revision(),
        fold.hidden_lines(),
        marks.to_vec(),
    );
    let redraw = editor.overlay.minimap_key.as_ref() != Some(&key);
    grow(
        commands,
        &mut editor.overlay.minimap_bars,
        count,
        |commands| {
            commands
                .spawn((
                    Node::default(),
                    Role::CodeMinimapBar,
                    Pickable::IGNORE,
                    OverlayPart,
                    ChildOf(root),
                ))
                .id()
        },
    );
    let mark_rows: Vec<(usize, CodeSeverity)> = marks
        .iter()
        .filter_map(|&(line, severity)| {
            let row = fold.row_of(line);
            (row >= first && row < first + count).then_some((row, severity))
        })
        .collect();
    grow(
        commands,
        &mut editor.overlay.minimap_marks,
        mark_rows.len(),
        |commands| {
            commands
                .spawn((
                    Node::default(),
                    Role::CodeMinimapMark(CodeSeverity::Error),
                    Pickable::IGNORE,
                    OverlayPart,
                    ChildOf(root),
                ))
                .id()
        },
    );
    if redraw {
        let column = ((width - 4.0) / MINIMAP_COLS as f32).max(0.5);
        for (slot, &entity) in editor.overlay.minimap_bars.iter().enumerate() {
            let Ok((mut node, ..)) = items.get_mut(entity) else {
                continue;
            };
            let row = first + slot;
            let cols = if slot < count && row < rows {
                display_width(editor.buffer.line(fold.line_at(row)), tab).min(MINIMAP_COLS)
            } else {
                0
            };
            if cols == 0 {
                set_display(&mut node, Display::None);
            } else {
                set_display(&mut node, Display::Flex);
                set_px(&mut node.left, 2.0);
                set_px(&mut node.width, cols as f32 * column);
                set_px(&mut node.top, slot as f32 * pitch);
            }
        }
        for (slot, &entity) in editor.overlay.minimap_marks.iter().enumerate() {
            let Ok((mut node, _, mut role, _)) = items.get_mut(entity) else {
                continue;
            };
            match mark_rows.get(slot) {
                Some(&(row, severity)) => {
                    set_display(&mut node, Display::Flex);
                    set_px(&mut node.top, (row - first) as f32 * pitch);
                    let wanted = Role::CodeMinimapMark(severity);
                    if *role != wanted {
                        *role = wanted;
                    }
                }
                None => set_display(&mut node, Display::None),
            }
        }
        editor.overlay.minimap_key = Some(key);
    }
    // The band over the part in view follows the glided scroll.
    if let Some(band) = editor.overlay.minimap_band
        && let Ok((mut node, ..)) = items.get_mut(band)
    {
        let top = (view.shown.y / view.line_px - first as f32) * pitch;
        let height = (view.area.y / view.line_px * pitch).min(count as f32 * pitch);
        set_px(&mut node.top, top);
        set_px(&mut node.height, height.max(pitch));
    }
}

/// Put on a [`code_editor`](crate::code_editor) to draw its minimap: the text small down the
/// right edge, one bar a line as long as the line, the part in view as a band you press or drag to
/// scroll, and the diagnostics marked.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CodeMinimap;

/// The squiggle sits this far down its line, in lines.
const SQUIGGLE_DROP: f32 = 0.5;
/// The gutter mark sits this far down its line, in lines.
const MARK_DROP: f32 = 0.3;

fn in_range(range: &Range<usize>, offset: usize) -> bool {
    range.contains(&offset) || (range.is_empty() && range.start == offset)
}

/// Where the word the caret is in starts on `line`: the byte of its first letter, digit or
/// underscore.
pub(crate) fn word_start(line: &str, col: usize) -> usize {
    let mut start = col;
    for (i, c) in line[..col].char_indices().rev() {
        if c.is_alphanumeric() || c == '_' {
            start = i;
        } else {
            break;
        }
    }
    start
}

/// A pointer (window logical px) in skin px from `node`'s top-left.
fn pointer_local(
    pointer: Vec2,
    node: &ComputedNode,
    at: &UiGlobalTransform,
    target: &bevy::ui::ComputedUiRenderTargetInfo,
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

/// Spawns parts until a pool holds `n`.
fn grow(
    commands: &mut Commands,
    pool: &mut Vec<Entity>,
    n: usize,
    mut spawn: impl FnMut(&mut Commands) -> Entity,
) {
    while pool.len() < n {
        pool.push(spawn(commands));
    }
}

/// Narrows the host's items by what the player has typed since `anchor`, opens, moves and closes
/// the list, and keeps its rows.
#[allow(clippy::too_many_arguments)]
fn update_completion(
    root: Entity,
    editor: &mut CodeEditor,
    completions: Option<&CodeCompletions>,
    view: &crate::code_editor::View,
    fold: &crate::code_fold::FoldMap,
    area: Entity,
    items: &mut Query<OverlayItem, With<OverlayPart>>,
    boxes: &Query<(&ComputedNode, &UiGlobalTransform)>,
    commands: &mut Commands,
) {
    let caret = editor.buffer.caret();
    let mut open = false;
    if let Some(completions) = completions {
        let anchor = pos_of(&editor.buffer, &editor.starts.1, completions.anchor);
        let line = editor.buffer.line(caret.line);
        let valid = anchor.line == caret.line
            && caret.col >= anchor.col
            && line[anchor.col..caret.col]
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_');
        if valid {
            let prefix = line[anchor.col..caret.col].to_owned();
            if prefix != editor.overlay.completion.prefix || !editor.overlay.completion.is_open() {
                if prefix != editor.overlay.completion.prefix {
                    editor.overlay.completion.selected = 0;
                    editor.overlay.completion.first = 0;
                }
                let mut scored: Vec<(usize, u32)> = completions
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(|(i, item)| {
                        if prefix.is_empty() {
                            Some((i, 0))
                        } else {
                            fuzzy_score(&prefix, &item.label).map(|score| (i, score))
                        }
                    })
                    .collect();
                scored.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
                editor.overlay.completion.filtered = scored.into_iter().map(|(i, _)| i).collect();
                editor.overlay.completion.prefix = prefix;
            }
            open = editor.overlay.completion.is_open();
        } else {
            commands.entity(root).remove::<CodeCompletions>();
        }
    }
    if !open {
        let completion = &mut editor.overlay.completion;
        completion.filtered.clear();
        completion.prefix.clear();
        if let Some(panel) = completion.panel.take() {
            commands.entity(panel).despawn();
            completion.rows.clear();
        }
        return;
    }
    let Some(completions) = completions else {
        return;
    };
    let completion = &mut editor.overlay.completion;
    let len = completion.filtered.len();
    completion.selected = completion.selected.min(len - 1);
    if completion.selected < completion.first {
        completion.first = completion.selected;
    } else if completion.selected >= completion.first + COMPLETION_ROWS {
        completion.first = completion.selected + 1 - COMPLETION_ROWS;
    }
    if completion.panel.is_none() {
        let mut rows = Vec::new();
        let panel = commands
            .spawn((
                Node::default(),
                Role::CodeCompletion,
                Pickable::IGNORE,
                OverlayPart,
                ChildOf(area),
            ))
            .id();
        for _ in 0..COMPLETION_ROWS {
            let row = commands
                .spawn((
                    Node::default(),
                    Role::CodeCompletionRow,
                    Pickable::IGNORE,
                    OverlayPart,
                    ChildOf(panel),
                ))
                .id();
            let label = commands
                .spawn((
                    Node::default(),
                    Text::new(""),
                    Role::CodeCompletionLabel,
                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                    Pickable::IGNORE,
                    OverlayPart,
                    ChildOf(row),
                ))
                .id();
            let detail = commands
                .spawn((
                    Node::default(),
                    Text::new(""),
                    Role::CodeCompletionDetail,
                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                    Pickable::IGNORE,
                    OverlayPart,
                    ChildOf(row),
                ))
                .id();
            rows.push((row, label, detail));
        }
        completion.panel = Some(panel);
        completion.rows = rows;
        return;
    }
    // Under the caret, or over it when there is no room.
    if let Some(panel) = completion.panel
        && let Ok((mut node, _, _, _)) = items.get_mut(panel)
    {
        let row = fold.row_of(caret.line);
        let text = editor.buffer.line(caret.line);
        let col = display_col(text, caret.col, editor.buffer.tab_width());
        let height = boxes
            .get(panel)
            .map_or(view.line_px * COMPLETION_ROWS as f32, |(n, _)| {
                n.size().y * n.inverse_scale_factor()
            });
        let below = (row + 1) as f32 * view.line_px - view.shown.y;
        let top =
            if below + height > view.area.y && row as f32 * view.line_px - view.shown.y > height {
                row as f32 * view.line_px - view.shown.y - height
            } else {
                below
            };
        set_px(&mut node.top, top);
        set_px(
            &mut node.left,
            (view.pad + col as f32 * view.advance - view.shown.x).max(0.0),
        );
    }
    for (slot, &(row, label, detail)) in completion.rows.iter().enumerate() {
        let at = completion.first + slot;
        let item = completion
            .filtered
            .get(at)
            .and_then(|&i| completions.items.get(i));
        if let Ok((mut node, _, mut role, _)) = items.get_mut(row) {
            set_display(
                &mut node,
                if item.is_some() {
                    Display::Flex
                } else {
                    Display::None
                },
            );
            let wanted = if at == completion.selected {
                Role::CodeCompletionRowLit
            } else {
                Role::CodeCompletionRow
            };
            if *role != wanted {
                *role = wanted;
            }
        }
        for (entity, words) in [
            (label, item.map(|i| i.label.as_str()).unwrap_or("")),
            (detail, item.map(|i| i.detail.as_str()).unwrap_or("")),
        ] {
            if let Ok((_, Some(mut text), _, _)) = items.get_mut(entity)
                && text.0 != words
            {
                text.0 = words.to_owned();
            }
        }
    }
}
