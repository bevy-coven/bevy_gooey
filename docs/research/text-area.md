# Multiline text area

Researched 2026-11-02 (iteration 40, #28 G-24). home-still's index has nothing on multi-line fields, so the sources below were fetched today.

## Sources

1. GOV.UK Design System, *Textarea*, https://design-system.service.gov.uk/components/textarea/ — craft.
2. Bevy 0.20 `bevy_text::EditableText` (`allow_newlines`, `visible_lines`, `max_characters`) and `bevy_ui_widgets::text_input` (`on_focused_keyboard_input`), read from the vendored source — technical.
3. This crate's own `text.md` (the single-line field: Enter is a final change, a text field never traps focus, WCAG 2.1.2) — shipping: the same row widget every other field here is built from.

## Craft principle

GOV.UK: "Use the textarea component when you need to let users enter an amount of text that's longer than a single line." "Make the height of a textarea proportional to the amount of text you expect users to enter. You can set the height of a textarea by specifying the `rows` attribute." "Labels must be aligned above the textarea they refer to." "Users will often need to copy and paste information into a textarea, so do not stop them from doing this."

## How shipping games do it

A text area is the single-line field with Enter changed: every text box in a shipped tool or game chat/notes box inserts a line on Enter and sends or commits on a modified Enter (Ctrl or Cmd). That is the issue's rule, and the one Bevy's own `EditableText` already implements for the first half: its `allow_newlines: bool` ("Allow new lines") makes an unmodified Enter insert `\n`, and `visible_lines: Some(n)` sizes the field `n` lines high.

## Model adopted

- **A1. `text_area(label, value, rows)`** is a `text_row_as` row (so its field, caret, selection, focus, `TextEditing`, `TextInputValue`, `TextHint`, `TextLimit`, `FieldProblem` and `ValueChange<String>` are the text input's) with the label **above** the field and the field `rows` lines high (`EditableText::visible_lines`, `allow_newlines`, wrapping at the field's width, scrolling inside it).
- **A2. Enter inserts a newline** (Bevy's). **Ctrl+Enter or Cmd+Enter** (`Modifiers::PRIMARY`'s platform rule: either is accepted on any platform, as the issue says "Cmd/Ctrl") ends the edit: one final `ValueChange<String>` (`is_final: true`), the text kept, focus kept (like the single-line Enter). An unmodified Enter on a single-line field is unchanged.
- **A3. Up and Down are the caret's** (a line up or down, Bevy's), so `navigate_by_input` leaves a focused enabled text area all four arrow keys (`FocusScopes::types_text` for the area's both axes). The D-pad still moves focus. Tab leaves (Bevy's), Escape blurs (Bevy's): WCAG 2.1.2 holds.
- **A4. `TextLimit` bounds a paste**: `max_characters` is Bevy's and applies to inserted text of any length. The game's own text is never cut.
- **A5. Look**: the same frame, fill, ring and `FieldProblem` as a text field; the field's height is `rows` lines plus its padding. The label's gap above is the panel's gap.

## Parameters

`rows` at least 1. Wrap: word boundary. Field width: the single-line field's (16 characters + 1) but it fills the row when the row's parent gives it room.

## Validation targets

Enter adds a line and sends a non-final change; Ctrl+Enter and Meta+Enter send one final change and keep the text; a 10,000-character insert with `TextLimit(100)` leaves 100; Up and Down move the caret and focus stays; D-pad Down leaves; the field is `rows` lines tall (within a pixel of `rows × line height + padding`) at 1x and 2x text; a long word wraps inside the field; nothing leaves the panel at 200% text; nothing written while idle; a disabled area takes nothing; label above the field.

## Deviations

Design default: both Ctrl+Enter and Cmd+Enter end the edit on every platform (the issue's "Cmd/Ctrl"), not only the platform's primary: a pad of habit crosses platforms.

## What great looks like

A field that looks like a text input grown downward: same frame, same ring, a caret on the right line, no scrollbar chrome (the viewport scrolls by the caret).

## Pending index

GOV.UK textarea page, in home-still.
