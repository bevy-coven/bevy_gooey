# A code editor: `code_editor`

Written 2026-11-10 for issue #82 (G-71), asked by bevy_dark_factory (tracking #362) for its built-in file editor: "slick as hell and beautifully rendered", for a game defined in YAML and Rhai. `text_area` (G-24) is a form field, not a code editor.

No corpus search was made for this note; the model below follows what a code editor is (a buffer, a caret and a selection, undo, a gutter) and what this crate and Bevy 0.20 already own. The prior art named is the behaviour of widely used editors as their users know it (VS Code, Zed, Sublime Text); none of it was re-read for this note, and nothing here is quoted from it.

## What Bevy has, and why the editor does not use it

Bevy's `EditableText` is one parley layout of the whole text. It is the right tool for a field and the wrong one for a file: it lays out every line it holds, so a 2 MB text costs a 2 MB shaping on each edit, and it has no gutter, no per-line draw and no virtual window. The editor therefore keeps its text in a buffer of its own (`CodeBuffer`, engine-free) and draws a window onto it.

## Model adopted

**The buffer** (`src/code_buffer.rs`, no engine type in it):

- Lines are stored with their own terminator, so a file with CRLF (or a mix) is written back byte for byte, and a line break the player types or pastes takes the buffer's own (the first terminator the loaded text had). A `CodePos`'s column is a byte offset into the line's content, always on a character boundary.
- A column the eye counts is a **display column**: one per character, a tab running to the next multiple of the tab width. `display_col`, `byte_at_display` and `expand_slice` convert. A wide (CJK) glyph is one column: the face is monospace and a second column for a double-width glyph is the host font's business. Assumed decision, reversible: it changes `display_col` alone.
- Moves: character, word (blanks, then a run of one class), line start (smart: the first non-blank, then column 0), line end, document ends, a line or a page up or down keeping the display column the caret last stood at.
- Edits record splices (`at`, `removed`, `inserted`) into undo groups. **A typed character joins the last group** when it is within `GROUP_PAUSE_S` (1 s) of the previous one, continues it at the caret and is of the same class (letters and digits, blanks, anything else), so "foo bar" undoes as `bar`, ` `, `foo`. A caret move ends the run. A paste, a cut, a line break, an indent, a word delete and a replace-all are one step each. History is capped at `UNDO_DEPTH` groups.
- Enter carries the line's indent; Tab inserts the indent unit to the next stop (`CodeIndent::Spaces(n)` or `Tab`), or indents every selected line when several are selected; Shift+Tab takes one level off each.

**The view** (`src/code_editor.rs`):

- A **pool of rows** (a screenful and two more) is re-pointed at other lines as the view scrolls, so the entity count and the per-frame work are the same for 20 lines and 20,000. A row draws only the display columns in view, so a 10,000-character line costs a row of columns. Line numbers are a second pool in the gutter.
- The gutter, the current-line band, the selection (one band per line, a column past the last line so the break shows selected), the caret and the measuring node are parts of the editor; **only `apply_skin` styles them**, through new roles (`CodeEditor`, `CodeGutter`, `CodeNumber`, `CodeNumberLit`, `CodeArea`, `CodeContent`, `CodeText`, `CodeLineLit`, `CodeSelection`, `CodeCaret`, `CodeGutterRows`). Positions and widths that depend on the text are data the update system writes, each only on a difference.
- One column is **measured** (a hidden node of ten digits, laid out like a row), not assumed: any monospace face works, whatever its advance.
- The face is `GooeyFonts::mono` (default: Bevy's default font).
- **Motion** is the skin's. The view glides on the scroll glide's spring (`motion.default` a third as long), the caret travels on `motion.snappy` at half its length (typing must keep up with it), and it blinks on `CodeEditorSettings::blink_period_s` (default 1.06 s, the Windows default of 530 ms each way, which a host sets from the platform's own). Under `MotionSettings::lands_travel()` the view and the caret land at once and the caret is steady (a blink is not information).
- **Keys are an observer on the editor** (`FocusedInput<KeyboardInput>`), not a system: the key arrives only while the editor is focused, and an observer can stop it before the window's tab navigation sees it, which is what lets Tab indent. **Escape gives focus up** (WCAG 2.1.2: Tab is the editor's, so the exit is the key that is not), F6 and the pad's D-pad move on, and the editor is a `FocusRegion::host()` so the arrow keys are its own.
- The clipboard is Bevy's `Clipboard` resource (an in-process buffer unless the host turns on `system_clipboard`; a read that is still pending on the web is polled each frame). IME: `Ime::Commit` types the text, `Ime::Preedit` is drawn inline at the caret, the window's `ime_enabled` follows focus and `ime_position` the caret.

**The host's side**: `CodeEditorValue` (text, revision), written by `set` and read as the player's text; a player's edit is one `ValueChange<String>` with the whole text (`is_final` for Cmd or Ctrl+S, sent even if nothing changed).

## Assumed decisions

- Single caret, single selection: no multi-cursor, no column selection.
- The line is the unit of the view: no soft wrap. A long line scrolls sideways.
- A host's `CodeEditorValue::set` clears the history: the text under it changed without the player, so an undo into it would write over what the host put there.
- `PERF_BUDGET` is 2.0, a ratio of frame times: the work is the same, so this is room for noise on a loaded machine, not a claim that it costs twice.
- The accessibility tree gets the role (a multi-line text input) and the label, not the text: mirroring 2 MB into the tree on every edit is a cost no player asked for. A host that needs the value reads `CodeEditorValue`.

## Parameters

| Name | Value | Where from |
|---|---|---|
| `CODE_LINE_EM` | 1.5 | design default: a line is 1.5 text sizes |
| `CODE_MIN_LINES` | 4 | least height, in lines |
| `CARET_SPRING_FRACTION` | 0.5 | of `motion.snappy` |
| `CARET_BLINK_PERIOD_S` | 1.06 | the Windows default caret blink, 530 ms each way |
| `CODE_SELECTION_ALPHA` | 0.35 | the accent let through; text on it holds `TEXT_CONTRAST` in every preset (tested) |
| `GROUP_PAUSE_S` | 1.0 | an undo group ends at a pause this long |
| `UNDO_DEPTH` | 1000 | groups kept |
| `PERF_BUDGET` | 2.0 | frame-time ratio, 20,000 lines to 20 |

## What this note does not cover

Syntax colours, diagnostics, completion, hover and edits from elsewhere are G-72 (#83); the minimap, folding, guides and find and replace are G-73 (#84). The buffer already carries the two pieces they stand on: byte offsets (`offset_of`, `pos_at`) and one-step multi-range edits (`replace_ranges`, `apply_external`).
