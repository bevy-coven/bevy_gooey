//! The code editor's text, engine-free: a buffer of lines with a caret and a selection, the moves
//! and edits of a code editor, and an undo history that groups what was typed by pause and by word.
//! [`crate::code_editor`] draws it and feeds it keys; nothing here reads the engine, so the whole
//! behaviour is tested without an app.
//!
//! **Lines keep their own terminator.** Each line is stored with the `"\n"` or `"\r\n"` it ended
//! with, so a file with CRLF endings (or a mix) is written back byte for byte after an edit
//! elsewhere. A line break the player types or pastes takes the buffer's own ([`CodeBuffer::newline`],
//! the first terminator the loaded text had, `"\n"` for none). A [`CodePos`]'s column counts bytes of the
//! line's content (never its terminator) and always stands on a character boundary.
//!
//! **Columns the eye counts** are display columns: a character is one, a tab runs to the next
//! multiple of [`CodeBuffer::tab_width`]. [`display_col`], [`byte_at_display`] and [`expand_slice`]
//! turn one into the other. A wide (CJK) character is one column: the face is monospace, and a
//! second column for a double-width glyph is the host font's business.

use std::ops::Range;

/// A pause this long, seconds, between two typed characters ends the undo group they were in.
pub const GROUP_PAUSE_S: f64 = 1.0;

/// How many undo groups a buffer keeps; the oldest goes first.
pub const UNDO_DEPTH: usize = 1000;

/// A place in the text: a line, and a byte offset into that line's content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodePos {
    /// The line, from 0.
    pub line: usize,
    /// The byte offset into the line's content, on a character boundary, never past the content.
    pub col: usize,
}

impl CodePos {
    /// A place in the text.
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

/// How the Tab key indents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeIndent {
    /// To the next multiple of this many columns, with spaces.
    Spaces(u8),
    /// A tab character.
    Tab,
}

impl Default for CodeIndent {
    fn default() -> Self {
        Self::Spaces(4)
    }
}

/// A move of the caret, for [`CodeBuffer::move_to`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeMove {
    /// One character left; across a line break at a line's start.
    Left,
    /// One character right; across a line break at a line's end.
    Right,
    /// To the start of the word on the left.
    WordLeft,
    /// To the end of the word on the right.
    WordRight,
    /// To the line's first non-blank character; again, to its start.
    Home,
    /// To the end of the line.
    LineEnd,
    /// To the start of the text.
    DocStart,
    /// To the end of the text.
    DocEnd,
    /// This many lines up, keeping the display column the caret last stood at.
    Up(usize),
    /// This many lines down, keeping the display column the caret last stood at.
    Down(usize),
    /// To this line, keeping the display column the caret last stood at: a move by rows when
    /// some lines are folded away.
    ToLine(usize),
}

/// A selection or caret: where it began and where it is now. They are equal for a bare caret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeSelection {
    /// Where the selection began, which stays when it is extended.
    pub anchor: CodePos,
    /// Where the caret is, which moves.
    pub head: CodePos,
}

impl CodeSelection {
    /// The selection's first end.
    pub fn start(&self) -> CodePos {
        self.anchor.min(self.head)
    }

    /// The selection's last end.
    pub fn end(&self) -> CodePos {
        self.anchor.max(self.head)
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

/// The class of a character, for words and for what groups under one undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Word,
    Space,
    Other,
}

fn class(c: char) -> Class {
    if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else if c.is_whitespace() {
        Class::Space
    } else {
        Class::Other
    }
}

/// What an undo group is made of, which decides whether the next edit joins it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// One character typed, of this class: a run of one class is one step.
    Type(Class),
    /// Backspace, one character at a time.
    Backspace,
    /// Delete, one character at a time.
    Delete,
    /// Everything else (a paste, a cut, a line break, an indent, a replace-all, a word delete):
    /// a step of its own.
    Block,
}

/// One splice: `removed` was at `at`, and `inserted` stands there now.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Replace {
    at: CodePos,
    removed: String,
    inserted: String,
}

#[derive(Clone, Debug)]
struct Group {
    edits: Vec<Replace>,
    before: CodeSelection,
    after: CodeSelection,
    kind: Kind,
    /// The time of the last edit in it.
    time: f64,
}

/// One edit made from elsewhere: `range` is a byte range of the whole text as it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeEdit {
    /// The bytes replaced, in the whole text, on character boundaries.
    pub range: Range<usize>,
    /// What stands there instead.
    pub text: String,
}

impl CodeEdit {
    /// An edit replacing `range` with `text`.
    pub fn new(range: Range<usize>, text: impl Into<String>) -> Self {
        Self {
            range,
            text: text.into(),
        }
    }
}

/// The text of a code editor: its lines, caret, selection and undo history. See the module docs.
///
/// ```
/// use bevy_gooey::{CodeBuffer, CodeMove};
/// let mut buffer = CodeBuffer::new("let a = 1;\r\nlet b = 2;\r\n");
/// buffer.move_to(CodeMove::DocEnd, false);
/// buffer.insert("// end", 0.0);
/// assert_eq!(buffer.text(), "let a = 1;\r\nlet b = 2;\r\n// end");
/// assert!(buffer.undo());
/// assert_eq!(buffer.text(), "let a = 1;\r\nlet b = 2;\r\n");
/// ```
#[derive(Clone, Debug)]
pub struct CodeBuffer {
    lines: Vec<String>,
    newline: &'static str,
    selection: CodeSelection,
    /// The display column vertical moves keep, until some other move or an edit.
    want: Option<usize>,
    undo: Vec<Group>,
    redo: Vec<Group>,
    /// Whether the next typed edit may join the last undo group.
    joins: bool,
    revision: u64,
    tab_width: usize,
    indent: CodeIndent,
}

impl Default for CodeBuffer {
    fn default() -> Self {
        Self::new("")
    }
}

/// The text of `line` without its terminator.
pub(crate) fn content(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .unwrap_or(line)
}

/// `text`'s line breaks, each as the one `newline`.
fn with_breaks(text: &str, newline: &str) -> String {
    if !text.contains(['\n', '\r']) {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str(newline);
            }
            '\n' => out.push_str(newline),
            other => out.push(other),
        }
    }
    out
}

/// Where `text`, inserted at `at`, ends.
fn end_after(at: CodePos, text: &str) -> CodePos {
    let breaks = text.matches('\n').count();
    if breaks == 0 {
        CodePos::new(at.line, at.col + text.len())
    } else {
        let tail = text.rsplit('\n').next().unwrap_or("");
        CodePos::new(at.line + breaks, tail.len())
    }
}

impl CodeBuffer {
    /// A buffer of `text`, the caret at its start, with four-space indents and four-column tabs.
    pub fn new(text: &str) -> Self {
        let mut buffer = Self {
            lines: Vec::new(),
            newline: "\n",
            selection: CodeSelection {
                anchor: CodePos::default(),
                head: CodePos::default(),
            },
            want: None,
            undo: Vec::new(),
            redo: Vec::new(),
            joins: false,
            revision: 0,
            tab_width: 4,
            indent: CodeIndent::default(),
        };
        buffer.load(text);
        buffer
    }

    /// Replaces the whole text: the history is cleared, the caret and selection are kept where
    /// they still are (held to the new text), and the revision moves.
    pub fn load(&mut self, text: &str) {
        let mut lines: Vec<String> = text.split_inclusive('\n').map(str::to_owned).collect();
        if lines.last().is_none_or(|line| line.ends_with('\n')) {
            lines.push(String::new());
        }
        self.newline = match lines.iter().find(|line| line.ends_with('\n')) {
            Some(line) if line.ends_with("\r\n") => "\r\n",
            _ => "\n",
        };
        self.lines = lines;
        self.undo.clear();
        self.redo.clear();
        self.joins = false;
        self.want = None;
        self.selection = CodeSelection {
            anchor: self.clamp(self.selection.anchor),
            head: self.clamp(self.selection.head),
        };
        self.revision += 1;
    }

    /// The whole text, every line with the terminator it has.
    pub fn text(&self) -> String {
        self.lines.concat()
    }

    /// How many lines there are: one more than the line breaks, so an empty text is one empty line.
    pub fn len_lines(&self) -> usize {
        self.lines.len()
    }

    /// The content of `line`, without its terminator; empty past the end.
    pub fn line(&self, line: usize) -> &str {
        self.lines.get(line).map_or("", |l| content(l))
    }

    /// `line` with the terminator it has (none on the last line); empty past the end.
    pub fn line_with_break(&self, line: usize) -> &str {
        self.lines.get(line).map_or("", String::as_str)
    }

    /// The byte offset each line starts at in [`CodeBuffer::text`].
    pub fn line_starts(&self) -> Vec<usize> {
        let mut at = 0;
        self.lines
            .iter()
            .map(|line| {
                let start = at;
                at += line.len();
                start
            })
            .collect()
    }

    /// Replaces the text between two places with `text` as one undo step (a completion, a
    /// replacement); the caret goes to the end of what went in. Returns whether the text changed.
    pub fn replace_span(&mut self, a: CodePos, b: CodePos, text: &str, now: f64) -> bool {
        let (a, b) = (self.clamp(a.min(b)), self.clamp(a.max(b)));
        if a == b && text.is_empty() {
            return false;
        }
        self.replace(a, b, text, Kind::Block, now);
        true
    }

    /// The line break a new line takes: the first the loaded text had, else `"\n"`.
    pub fn newline(&self) -> &'static str {
        self.newline
    }

    /// How many changes the text has had (every edit, undo, redo and load is one).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// How wide a tab character is drawn, in columns.
    pub fn tab_width(&self) -> usize {
        self.tab_width
    }

    /// Sets how wide a tab character is drawn (at least 1).
    pub fn set_tab_width(&mut self, width: usize) {
        self.tab_width = width.max(1);
    }

    /// How the Tab key indents.
    pub fn indent(&self) -> CodeIndent {
        self.indent
    }

    /// Sets how the Tab key indents.
    pub fn set_indent(&mut self, indent: CodeIndent) {
        self.indent = indent;
    }

    /// The caret and selection.
    pub fn selection(&self) -> CodeSelection {
        self.selection
    }

    /// The caret: the selection's moving end.
    pub fn caret(&self) -> CodePos {
        self.selection.head
    }

    /// Selects from `anchor` to `head`, each held to the text. A caret alone is `anchor == head`.
    pub fn select(&mut self, anchor: CodePos, head: CodePos) {
        self.selection = CodeSelection {
            anchor: self.clamp(anchor),
            head: self.clamp(head),
        };
        self.want = None;
        self.joins = false;
    }

    /// `pos` held to the text: a line past the end is the last, a column past the content is its
    /// end, and one inside a character is the boundary before it.
    pub fn clamp(&self, pos: CodePos) -> CodePos {
        let line = pos.line.min(self.lines.len().saturating_sub(1));
        let text = self.line(line);
        let mut col = pos.col.min(text.len());
        while !text.is_char_boundary(col) {
            col -= 1;
        }
        CodePos::new(line, col)
    }

    /// The end of the text.
    pub fn end(&self) -> CodePos {
        let line = self.lines.len().saturating_sub(1);
        CodePos::new(line, self.line(line).len())
    }

    /// The byte offset of `pos` in [`CodeBuffer::text`].
    pub fn offset_of(&self, pos: CodePos) -> usize {
        let pos = self.clamp(pos);
        self.lines[..pos.line]
            .iter()
            .map(String::len)
            .sum::<usize>()
            + pos.col
    }

    /// The place at byte `offset` of [`CodeBuffer::text`]; an offset inside a line break is the
    /// end of that line's content, one past the text its end.
    pub fn pos_at(&self, offset: usize) -> CodePos {
        let mut rest = offset;
        for (index, line) in self.lines.iter().enumerate() {
            if rest < line.len() || index + 1 == self.lines.len() {
                return self.clamp(CodePos::new(index, rest));
            }
            rest -= line.len();
        }
        self.end()
    }

    /// The text between two places, either order.
    pub fn text_between(&self, a: CodePos, b: CodePos) -> String {
        let (a, b) = (self.clamp(a.min(b)), self.clamp(a.max(b)));
        if a.line == b.line {
            return self.line(a.line)[a.col..b.col].to_owned();
        }
        let mut out = String::new();
        out.push_str(&self.lines[a.line][a.col..]);
        for line in &self.lines[a.line + 1..b.line] {
            out.push_str(line);
        }
        out.push_str(&self.line(b.line)[..b.col]);
        out
    }

    /// The selected text; empty for a caret alone.
    pub fn selected_text(&self) -> String {
        self.text_between(self.selection.anchor, self.selection.head)
    }

    /// Whether anything is selected.
    pub fn has_selection(&self) -> bool {
        !self.selection.is_empty()
    }

    /// The longest line's width in display columns.
    pub fn longest_cols(&self) -> usize {
        // The line with the most bytes is the one that can be widest; a tab-heavy shorter line is
        // not (a tab is a handful of columns, so this errs short only by what a tab adds).
        self.lines
            .iter()
            .max_by_key(|line| content(line).len())
            .map_or(0, |line| display_width(content(line), self.tab_width))
    }

    fn step_left(&self, pos: CodePos) -> CodePos {
        if pos.col > 0 {
            let text = self.line(pos.line);
            let mut col = pos.col - 1;
            while !text.is_char_boundary(col) {
                col -= 1;
            }
            CodePos::new(pos.line, col)
        } else if pos.line > 0 {
            CodePos::new(pos.line - 1, self.line(pos.line - 1).len())
        } else {
            pos
        }
    }

    fn step_right(&self, pos: CodePos) -> CodePos {
        let text = self.line(pos.line);
        if pos.col < text.len() {
            let mut col = pos.col + 1;
            while !text.is_char_boundary(col) {
                col += 1;
            }
            CodePos::new(pos.line, col)
        } else if pos.line + 1 < self.lines.len() {
            CodePos::new(pos.line + 1, 0)
        } else {
            pos
        }
    }

    /// The start of the word on the left of `pos`: blanks, then the run of one class.
    pub fn word_left(&self, pos: CodePos) -> CodePos {
        if pos.col == 0 {
            return self.step_left(pos);
        }
        let before = &self.line(pos.line)[..pos.col];
        let mut chars = before.char_indices().rev().peekable();
        let mut at = pos.col;
        while let Some((i, _)) = chars.next_if(|&(_, c)| class(c) == Class::Space) {
            at = i;
        }
        if let Some(&(_, c)) = chars.peek() {
            let kind = class(c);
            while let Some((i, _)) = chars.next_if(|&(_, c)| class(c) == kind) {
                at = i;
            }
        }
        CodePos::new(pos.line, at)
    }

    /// The end of the word on the right of `pos`: blanks, then the run of one class.
    pub fn word_right(&self, pos: CodePos) -> CodePos {
        let text = self.line(pos.line);
        if pos.col >= text.len() {
            return self.step_right(pos);
        }
        let mut chars = text[pos.col..].char_indices().peekable();
        while chars.next_if(|&(_, c)| class(c) == Class::Space).is_some() {}
        if let Some(&(_, c)) = chars.peek() {
            let kind = class(c);
            while chars.next_if(|&(_, c)| class(c) == kind).is_some() {}
        }
        let at = chars.peek().map_or(text.len(), |&(i, _)| pos.col + i);
        CodePos::new(pos.line, at)
    }

    fn first_nonblank(&self, line: usize) -> usize {
        let text = self.line(line);
        text.len() - text.trim_start_matches([' ', '\t']).len()
    }

    /// Moves the caret; `extend` keeps the anchor, so the selection grows to it. A plain Left or
    /// Right over a selection collapses it to that end.
    pub fn move_to(&mut self, movement: CodeMove, extend: bool) {
        let head = self.selection.head;
        let collapse = !extend && self.has_selection();
        let mut keeps_column = false;
        let target = match movement {
            CodeMove::Left if collapse => self.selection.start(),
            CodeMove::Right if collapse => self.selection.end(),
            CodeMove::Left => self.step_left(head),
            CodeMove::Right => self.step_right(head),
            CodeMove::WordLeft => self.word_left(head),
            CodeMove::WordRight => self.word_right(head),
            CodeMove::Home => {
                let first = self.first_nonblank(head.line);
                CodePos::new(head.line, if head.col == first { 0 } else { first })
            }
            CodeMove::LineEnd => CodePos::new(head.line, self.line(head.line).len()),
            CodeMove::DocStart => CodePos::default(),
            CodeMove::DocEnd => self.end(),
            CodeMove::ToLine(line) => {
                keeps_column = true;
                let want = self
                    .want
                    .unwrap_or_else(|| display_col(self.line(head.line), head.col, self.tab_width));
                self.want = Some(want);
                let line = line.min(self.lines.len() - 1);
                CodePos::new(line, byte_at_display(self.line(line), want, self.tab_width))
            }
            CodeMove::Up(lines) | CodeMove::Down(lines) => {
                keeps_column = true;
                let up = matches!(movement, CodeMove::Up(_));
                let want = self
                    .want
                    .unwrap_or_else(|| display_col(self.line(head.line), head.col, self.tab_width));
                self.want = Some(want);
                let line = if up {
                    head.line.checked_sub(lines)
                } else {
                    Some(head.line + lines).filter(|&l| l < self.lines.len())
                };
                match line {
                    Some(line) => {
                        CodePos::new(line, byte_at_display(self.line(line), want, self.tab_width))
                    }
                    None if up => CodePos::default(),
                    None => self.end(),
                }
            }
        };
        if !keeps_column {
            self.want = None;
        }
        self.joins = false;
        self.selection.head = target;
        if !extend {
            self.selection.anchor = target;
        }
    }

    /// Selects the whole text.
    pub fn select_all(&mut self) {
        self.select(CodePos::default(), self.end());
    }

    /// Selects the word at `pos`: the run of one class under it (the one before it at a line's
    /// end); nothing on an empty line.
    pub fn select_word_at(&mut self, pos: CodePos) {
        let pos = self.clamp(pos);
        let text = self.line(pos.line);
        let at = text[pos.col..]
            .chars()
            .next()
            .or_else(|| text[..pos.col].chars().next_back());
        let Some(kind) = at.map(class) else {
            self.select(pos, pos);
            return;
        };
        let mut start = pos.col;
        for (i, c) in text[..pos.col].char_indices().rev() {
            if class(c) != kind {
                break;
            }
            start = i;
        }
        let mut end = pos.col;
        for (i, c) in text[pos.col..].char_indices() {
            if class(c) != kind {
                break;
            }
            end = pos.col + i + c.len_utf8();
        }
        self.select(CodePos::new(pos.line, start), CodePos::new(pos.line, end));
    }

    /// Selects `line` whole, with its line break (the last line, to its end).
    pub fn select_line(&mut self, line: usize) {
        let line = line.min(self.lines.len() - 1);
        let end = if line + 1 < self.lines.len() {
            CodePos::new(line + 1, 0)
        } else {
            CodePos::new(line, self.line(line).len())
        };
        self.select(CodePos::new(line, 0), end);
    }

    /// Splices `text` over `a..b` as it is (no line-break fixing), returning where it ends.
    fn splice(&mut self, a: CodePos, b: CodePos, text: &str) -> CodePos {
        let prefix = self.line(a.line)[..a.col].to_owned();
        let suffix = self.lines[b.line][b.col..].to_owned();
        let mut pieces: Vec<String> = text.split_inclusive('\n').map(str::to_owned).collect();
        if pieces.last().is_none_or(|piece| piece.ends_with('\n')) {
            pieces.push(String::new());
        }
        let last = pieces.len() - 1;
        pieces[0].insert_str(0, &prefix);
        pieces[last].push_str(&suffix);
        let end = end_after(a, text);
        self.lines.splice(a.line..=b.line, pieces);
        end
    }

    /// One edit: `a..b` becomes `text` (its line breaks made the buffer's own), recorded as one
    /// splice in the history under `kind`. Returns the end of the inserted text.
    fn replace(&mut self, a: CodePos, b: CodePos, text: &str, kind: Kind, now: f64) -> CodePos {
        let before = self.selection;
        let (a, b) = (self.clamp(a.min(b)), self.clamp(a.max(b)));
        let inserted = with_breaks(text, self.newline);
        let removed = self.text_between(a, b);
        let end = self.splice(a, b, &inserted);
        self.revision += 1;
        self.want = None;
        self.selection = CodeSelection {
            anchor: end,
            head: end,
        };
        self.redo.clear();
        let edit = Replace {
            at: a,
            removed,
            inserted,
        };
        let after = self.selection;
        let joined = self.joins
            && kind != Kind::Block
            && self.undo.last_mut().is_some_and(|group| {
                let near = now - group.time <= GROUP_PAUSE_S;
                let contiguous = group.kind == kind
                    && group.after.is_empty()
                    && match kind {
                        Kind::Type(_) => edit.at == group.after.head,
                        Kind::Backspace => end_after(edit.at, &edit.removed) == group.after.head,
                        Kind::Delete => edit.at == group.after.head,
                        Kind::Block => false,
                    };
                if near && contiguous {
                    group.edits.push(edit.clone());
                    group.after = after;
                    group.time = now;
                    true
                } else {
                    false
                }
            });
        if !joined {
            self.undo.push(Group {
                edits: vec![edit],
                before,
                after,
                kind,
                time: now,
            });
            if self.undo.len() > UNDO_DEPTH {
                self.undo.remove(0);
            }
        }
        self.joins = kind != Kind::Block;
        end
    }

    /// Types `text` at the caret, over the selection: a single character joins the undo step of
    /// the run it continues (same class, within [`GROUP_PAUSE_S`]), anything longer is a step of
    /// its own. `now` is the clock, seconds. Returns whether the text changed.
    pub fn insert(&mut self, text: &str, now: f64) -> bool {
        if text.is_empty() && !self.has_selection() {
            return false;
        }
        let kind = {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c != '\n' && c != '\r' && !self.has_selection() => {
                    Kind::Type(class(c))
                }
                _ => Kind::Block,
            }
        };
        let sel = self.selection;
        self.replace(sel.start(), sel.end(), text, kind, now);
        true
    }

    /// Pastes `text` over the selection as one undo step.
    pub fn paste(&mut self, text: &str, now: f64) -> bool {
        if text.is_empty() && !self.has_selection() {
            return false;
        }
        let sel = self.selection;
        self.replace(sel.start(), sel.end(), text, Kind::Block, now);
        true
    }

    /// Takes the selection out (one undo step) and returns it; `None` for a caret alone.
    pub fn cut(&mut self, now: f64) -> Option<String> {
        if !self.has_selection() {
            return None;
        }
        let text = self.selected_text();
        let sel = self.selection;
        self.replace(sel.start(), sel.end(), "", Kind::Block, now);
        Some(text)
    }

    /// A line break at the caret, over the selection, carrying the line's indent.
    pub fn newline_here(&mut self, now: f64) -> bool {
        let sel = self.selection;
        let start = sel.start();
        let lead: String = self
            .line(start.line)
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let lead_len = lead.len().min(start.col);
        let text = format!("\n{}", &lead[..lead_len]);
        self.replace(start, sel.end(), &text, Kind::Block, now);
        true
    }

    /// Backspace: the selection, else the character before the caret (the line break at a line's
    /// start), or with `word` the word before it. Returns whether anything went.
    pub fn backspace(&mut self, word: bool, now: f64) -> bool {
        let sel = self.selection;
        if !sel.is_empty() {
            self.replace(sel.start(), sel.end(), "", Kind::Block, now);
            return true;
        }
        let from = if word {
            self.word_left(sel.head)
        } else {
            self.step_left(sel.head)
        };
        if from == sel.head {
            return false;
        }
        let kind = if word { Kind::Block } else { Kind::Backspace };
        self.replace(from, sel.head, "", kind, now);
        true
    }

    /// Delete: the selection, else the character after the caret, or with `word` the word after it.
    pub fn delete_forward(&mut self, word: bool, now: f64) -> bool {
        let sel = self.selection;
        if !sel.is_empty() {
            self.replace(sel.start(), sel.end(), "", Kind::Block, now);
            return true;
        }
        let to = if word {
            self.word_right(sel.head)
        } else {
            self.step_right(sel.head)
        };
        if to == sel.head {
            return false;
        }
        let kind = if word { Kind::Block } else { Kind::Delete };
        self.replace(sel.head, to, "", kind, now);
        true
    }

    /// The text one indent level is: a tab, or spaces up to `column`'s next stop.
    fn indent_unit(&self, column: usize) -> String {
        match self.indent {
            CodeIndent::Tab => "\t".to_owned(),
            CodeIndent::Spaces(width) => {
                let width = usize::from(width.max(1));
                " ".repeat(width - column % width)
            }
        }
    }

    /// Tab: with a selection over several lines, every one of them is indented a level; otherwise
    /// the indent unit stands at the caret, over the selection.
    pub fn indent_here(&mut self, now: f64) -> bool {
        let sel = self.selection;
        let (start, end) = (sel.start(), sel.end());
        if start.line == end.line {
            let column = display_col(self.line(start.line), start.col, self.tab_width);
            let unit = self.indent_unit(column);
            self.replace(start, end, &unit, Kind::Block, now);
            return true;
        }
        // The last line is left alone when the selection only reaches its start.
        let last = if end.col == 0 { end.line - 1 } else { end.line };
        let before = self.selection;
        let mut edits = Vec::new();
        for line in start.line..=last {
            if self.line(line).is_empty() {
                continue;
            }
            edits.push((line, 0usize, 0usize, self.indent_unit(0)));
        }
        self.edit_lines(edits, before, now);
        true
    }

    /// Shift+Tab: every line the selection touches loses one indent level, if it has one.
    pub fn outdent_here(&mut self, now: f64) -> bool {
        let sel = self.selection;
        let (start, end) = (sel.start(), sel.end());
        let last = if end.col == 0 && end.line > start.line {
            end.line - 1
        } else {
            end.line
        };
        let mut edits = Vec::new();
        for line in start.line..=last {
            let text = self.line(line);
            let cut = if text.starts_with('\t') {
                1
            } else {
                let width = match self.indent {
                    CodeIndent::Spaces(width) => usize::from(width.max(1)),
                    CodeIndent::Tab => self.tab_width,
                };
                text.chars().take(width).take_while(|c| *c == ' ').count()
            };
            if cut > 0 {
                edits.push((line, 0usize, cut, String::new()));
            }
        }
        if edits.is_empty() {
            return false;
        }
        let before = self.selection;
        self.edit_lines(edits, before, now);
        true
    }

    /// Several splices, each `(line, col, bytes removed, inserted)` at the start of a line, as one
    /// undo step; the selection moves with its text.
    fn edit_lines(
        &mut self,
        edits: Vec<(usize, usize, usize, String)>,
        before: CodeSelection,
        now: f64,
    ) {
        let mut sel = before;
        let mut group = Vec::new();
        for (line, col, removed, inserted) in edits {
            let a = CodePos::new(line, col);
            let b = CodePos::new(line, col + removed);
            let text = self.text_between(a, b);
            self.splice(a, b, &inserted);
            let shift = |p: CodePos| {
                if p.line == line && p.col >= col + removed {
                    CodePos::new(line, p.col + inserted.len() - removed)
                } else if p.line == line && p.col > col {
                    CodePos::new(line, col)
                } else {
                    p
                }
            };
            sel = CodeSelection {
                anchor: shift(sel.anchor),
                head: shift(sel.head),
            };
            group.push(Replace {
                at: a,
                removed: text,
                inserted,
            });
        }
        self.revision += 1;
        self.want = None;
        self.selection = sel;
        self.redo.clear();
        self.undo.push(Group {
            edits: group,
            before,
            after: sel,
            kind: Kind::Block,
            time: now,
        });
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.joins = false;
    }

    /// Replaces each byte range of the whole text with its string, all as one undo step: a
    /// replace-all. Ranges must not overlap; they are applied last first, so each is in the
    /// text's own offsets. The selection goes to the end of the last replacement. Returns whether
    /// anything changed.
    pub fn replace_ranges(&mut self, edits: &[CodeEdit], now: f64) -> bool {
        let mut edits: Vec<&CodeEdit> = edits.iter().collect();
        edits.sort_by_key(|edit| edit.range.start);
        if edits.is_empty() {
            return false;
        }
        let before = self.selection;
        let mut group = Vec::new();
        let mut end = before.head;
        for edit in edits.iter().rev() {
            let a = self.pos_at(edit.range.start);
            let b = self.pos_at(edit.range.end);
            let inserted = with_breaks(&edit.text, self.newline);
            let removed = self.text_between(a, b);
            end = self.splice(a, b, &inserted);
            group.push(Replace {
                at: a,
                removed,
                inserted,
            });
        }
        self.revision += 1;
        self.want = None;
        self.selection = CodeSelection {
            anchor: end,
            head: end,
        };
        self.redo.clear();
        let after = self.selection;
        self.undo.push(Group {
            edits: group,
            before,
            after,
            kind: Kind::Block,
            time: now,
        });
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.joins = false;
        true
    }

    /// Undoes the last step, taking the selection back to what it was. Returns whether there was one.
    pub fn undo(&mut self) -> bool {
        let Some(group) = self.undo.pop() else {
            return false;
        };
        for edit in group.edits.iter().rev() {
            let end = end_after(edit.at, &edit.inserted);
            self.splice(edit.at, end, &edit.removed);
        }
        self.selection = group.before;
        self.revision += 1;
        self.want = None;
        self.joins = false;
        self.redo.push(group);
        true
    }

    /// Redoes the step undo took back. Returns whether there was one.
    pub fn redo(&mut self) -> bool {
        let Some(group) = self.redo.pop() else {
            return false;
        };
        for edit in &group.edits {
            let end = end_after(edit.at, &edit.removed);
            self.splice(edit.at, end, &edit.inserted);
        }
        self.selection = group.after;
        self.revision += 1;
        self.want = None;
        self.joins = false;
        self.undo.push(group);
        true
    }

    /// Whether [`CodeBuffer::undo`] has a step to take back.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether [`CodeBuffer::redo`] has a step to put back.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Where byte `offset` of the text moves to when `edit` is made: before it, nowhere; after it,
    /// by how much it grew; inside what it replaced, to its start. A bare insertion exactly at the
    /// offset pushes it along, so a caret stays on the character it was on.
    fn shifted(offset: usize, edit: &CodeEdit) -> usize {
        let (start, end) = (edit.range.start, edit.range.end);
        if offset > end || (offset == end && (start != end || offset > start)) {
            offset + edit.text.len() - (end - start)
        } else if start == end && offset == start {
            offset + edit.text.len()
        } else if offset > start {
            start
        } else {
            offset
        }
    }

    /// Applies edits made by someone else, in the text's own offsets (they are applied together:
    /// none sees another's change). The caret and the selection stay on the text they were on, the
    /// history is dropped (a person's undo never rewrites someone else's text), and the revision
    /// moves. Returns the lines the edits changed, in the text as it is now.
    pub fn apply_external(&mut self, edits: &[CodeEdit]) -> Vec<usize> {
        let mut edits: Vec<CodeEdit> = edits
            .iter()
            .map(|edit| CodeEdit::new(edit.range.clone(), with_breaks(&edit.text, self.newline)))
            .collect();
        edits.sort_by_key(|edit| edit.range.start);
        let (mut anchor, mut head) = (
            self.offset_of(self.selection.anchor),
            self.offset_of(self.selection.head),
        );
        // Last first: each edit is in the original offsets, and one below an offset moves it.
        for edit in edits.iter().rev() {
            anchor = Self::shifted(anchor, edit);
            head = Self::shifted(head, edit);
        }
        let mut placed: Vec<Range<usize>> = Vec::with_capacity(edits.len());
        let mut delta = 0isize;
        for edit in &edits {
            let start = (edit.range.start as isize + delta) as usize;
            placed.push(start..start + edit.text.len());
            delta += edit.text.len() as isize - (edit.range.end - edit.range.start) as isize;
        }
        for edit in edits.iter().rev() {
            let a = self.pos_at(edit.range.start);
            let b = self.pos_at(edit.range.end);
            self.splice(a, b, &edit.text);
        }
        // The lines now holding what the edits put there; a break the edit ended on leaves the
        // line after it as it was.
        let mut touched = Vec::new();
        for range in &placed {
            let first = self.pos_at(range.start);
            let last = self.pos_at(range.end);
            let to = if range.is_empty() || (last.col == 0 && last.line > first.line) {
                last.line.saturating_sub(usize::from(!range.is_empty()))
            } else {
                last.line
            };
            touched.extend(first.line..=to.max(first.line));
        }
        touched.sort_unstable();
        touched.dedup();
        self.undo.clear();
        self.redo.clear();
        self.joins = false;
        self.want = None;
        self.revision += 1;
        self.selection = CodeSelection {
            anchor: self.pos_at(anchor),
            head: self.pos_at(head),
        };
        touched
    }
}

/// How many display columns `line` is wide, a tab running to the next multiple of `tab`.
pub fn display_width(line: &str, tab: usize) -> usize {
    display_col(line, line.len(), tab)
}

/// The display column of byte `col` of `line`.
pub fn display_col(line: &str, col: usize, tab: usize) -> usize {
    let tab = tab.max(1);
    let mut at = 0;
    for (i, c) in line.char_indices() {
        if i >= col {
            break;
        }
        at += if c == '\t' { tab - at % tab } else { 1 };
    }
    at
}

/// The byte of `line` nearest display column `want`: the character boundary whose column is
/// closest, the later on a tie, never past the line's end.
pub fn byte_at_display(line: &str, want: usize, tab: usize) -> usize {
    let tab = tab.max(1);
    let mut at = 0;
    for (i, c) in line.char_indices() {
        let next = at + if c == '\t' { tab - at % tab } else { 1 };
        if want < next {
            // Inside this character: its start if the column is nearer to it.
            return if want - at < next - want {
                i
            } else {
                i + c.len_utf8()
            };
        }
        at = next;
    }
    line.len()
}

/// The part of `line` that shows in display columns `from..to`, a tab as the spaces it spans.
pub fn expand_slice(line: &str, from: usize, to: usize, tab: usize) -> String {
    let tab = tab.max(1);
    let mut out = String::new();
    let mut at = 0;
    for c in line.chars() {
        if at >= to {
            break;
        }
        if c == '\t' {
            let span = tab - at % tab;
            let (lo, hi) = (at.max(from), (at + span).min(to));
            for _ in lo..hi.max(lo) {
                out.push(' ');
            }
            at += span;
        } else {
            if at >= from {
                out.push(c);
            }
            at += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(buffer: &mut CodeBuffer, text: &str, mut now: f64) -> f64 {
        for c in text.chars() {
            buffer.insert(&c.to_string(), now);
            now += 0.05;
        }
        now
    }

    #[test]
    fn crlf_endings_are_kept_and_new_breaks_take_them() {
        let mut buffer = CodeBuffer::new("a\r\nb\r\n");
        assert_eq!(buffer.newline(), "\r\n");
        assert_eq!(buffer.len_lines(), 3);
        buffer.move_to(CodeMove::DocEnd, false);
        buffer.insert("c\nd", 0.0);
        assert_eq!(buffer.text(), "a\r\nb\r\nc\r\nd");
        let mut mixed = CodeBuffer::new("a\nb\r\nc");
        mixed.move_to(CodeMove::DocEnd, false);
        mixed.insert("!", 0.0);
        assert_eq!(mixed.text(), "a\nb\r\nc!");
    }

    #[test]
    fn typing_groups_by_word_and_pause() {
        let mut buffer = CodeBuffer::new("");
        let mut now = typed(&mut buffer, "foo bar", 0.0);
        assert_eq!(buffer.text(), "foo bar");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "foo ");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "foo");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "");
        assert!(!buffer.undo());
        assert!(buffer.redo());
        assert_eq!(buffer.text(), "foo");
        // A pause ends the group even inside a word.
        buffer.load("");
        now += 5.0;
        buffer.insert("a", now);
        buffer.insert("b", now + 0.1);
        buffer.insert("c", now + 3.0);
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "ab");
    }

    #[test]
    fn a_paste_is_one_step_and_undo_restores_the_selection() {
        let mut buffer = CodeBuffer::new("one two");
        buffer.select(CodePos::new(0, 4), CodePos::new(0, 7));
        buffer.paste("a\nb\nc", 0.0);
        assert_eq!(buffer.text(), "one a\nb\nc");
        assert_eq!(buffer.caret(), CodePos::new(2, 1));
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "one two");
        assert_eq!(buffer.selected_text(), "two");
        assert!(buffer.redo());
        assert_eq!(buffer.text(), "one a\nb\nc");
    }

    #[test]
    fn words_lines_and_selecting() {
        let mut buffer = CodeBuffer::new("let  foo_bar = 1;\n  next");
        buffer.move_to(CodeMove::WordRight, false);
        assert_eq!(buffer.caret(), CodePos::new(0, 3));
        buffer.move_to(CodeMove::WordRight, true);
        assert_eq!(buffer.selected_text(), "  foo_bar");
        buffer.select_word_at(CodePos::new(0, 7));
        assert_eq!(buffer.selected_text(), "foo_bar");
        buffer.select_line(0);
        assert_eq!(buffer.selected_text(), "let  foo_bar = 1;\n");
        buffer.select_line(1);
        assert_eq!(buffer.selected_text(), "  next");
        buffer.move_to(CodeMove::DocStart, false);
        buffer.move_to(CodeMove::Down(1), false);
        buffer.move_to(CodeMove::Home, false);
        assert_eq!(buffer.caret(), CodePos::new(1, 2));
        buffer.move_to(CodeMove::Home, false);
        assert_eq!(buffer.caret(), CodePos::new(1, 0));
    }

    #[test]
    fn vertical_moves_keep_the_display_column() {
        let mut buffer = CodeBuffer::new("abcdefgh\nab\nabcdefgh");
        buffer.select(CodePos::new(0, 6), CodePos::new(0, 6));
        buffer.move_to(CodeMove::Down(1), false);
        assert_eq!(buffer.caret(), CodePos::new(1, 2));
        buffer.move_to(CodeMove::Down(1), false);
        assert_eq!(buffer.caret(), CodePos::new(2, 6));
        let tabs = CodeBuffer::new("\tx");
        assert_eq!(display_col(tabs.line(0), 1, 4), 4);
        assert_eq!(byte_at_display("\tx", 4, 4), 1);
        assert_eq!(byte_at_display("\tx", 1, 4), 0);
        assert_eq!(byte_at_display("\tx", 3, 4), 1);
        assert_eq!(expand_slice("a\tb", 0, 9, 4), "a   b");
        assert_eq!(expand_slice("a\tb", 2, 9, 4), "  b");
    }

    #[test]
    fn enter_carries_the_indent_and_backspace_joins_lines() {
        let mut buffer = CodeBuffer::new("    foo");
        buffer.move_to(CodeMove::DocEnd, false);
        buffer.newline_here(0.0);
        assert_eq!(buffer.text(), "    foo\n    ");
        assert_eq!(buffer.caret(), CodePos::new(1, 4));
        buffer.select(CodePos::new(1, 0), CodePos::new(1, 0));
        assert!(buffer.backspace(false, 1.0));
        assert_eq!(buffer.text(), "    foo    ");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "    foo\n    ");
    }

    #[test]
    fn tab_indents_to_the_next_stop_and_shift_tab_takes_a_level_back() {
        let mut buffer = CodeBuffer::new("ab\ncd\nef");
        buffer.move_to(CodeMove::Right, false);
        buffer.indent_here(0.0);
        assert_eq!(buffer.line(0), "a   b");
        buffer.select(CodePos::new(0, 0), CodePos::new(2, 2));
        buffer.indent_here(1.0);
        assert_eq!(buffer.text(), "    a   b\n    cd\n    ef");
        assert!(buffer.outdent_here(2.0));
        assert_eq!(buffer.text(), "a   b\ncd\nef");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "    a   b\n    cd\n    ef");
        buffer.set_indent(CodeIndent::Tab);
        buffer.select(CodePos::new(0, 0), CodePos::new(0, 0));
        buffer.indent_here(3.0);
        assert!(buffer.line(0).starts_with('\t'));
    }

    #[test]
    fn replace_ranges_is_one_step_and_external_edits_keep_the_caret() {
        let mut buffer = CodeBuffer::new("a b a b a");
        let found = [
            CodeEdit::new(0..1, "xx"),
            CodeEdit::new(4..5, "xx"),
            CodeEdit::new(8..9, "xx"),
        ];
        assert!(buffer.replace_ranges(&found, 0.0));
        assert_eq!(buffer.text(), "xx b xx b xx");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "a b a b a");

        let mut buffer = CodeBuffer::new("one\ntwo\nthree");
        buffer.select(CodePos::new(2, 2), CodePos::new(2, 2));
        let touched = buffer.apply_external(&[CodeEdit::new(0..0, "zero\n")]);
        assert_eq!(buffer.text(), "zero\none\ntwo\nthree");
        assert_eq!(buffer.caret(), CodePos::new(3, 2));
        assert_eq!(touched, vec![0]);
        assert!(!buffer.can_undo());
    }

    #[test]
    fn an_external_edit_inside_a_selection_keeps_the_selection_on_its_text() {
        let mut buffer = CodeBuffer::new("let value = 10;");
        buffer.select(CodePos::new(0, 4), CodePos::new(0, 9));
        assert_eq!(buffer.selected_text(), "value");
        buffer.apply_external(&[CodeEdit::new(4..4, "mut ")]);
        assert_eq!(buffer.selected_text(), "value");
        buffer.apply_external(&[CodeEdit::new(14..15, "")]);
        assert_eq!(buffer.selected_text(), "value");
    }
}
