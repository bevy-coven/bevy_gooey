# Field validation messages

Researched 2026-11-01 (iteration 37, #29 G-25). home-still's index has nothing on form validation (`distill_search` returned agent-benchmark papers), so the sources below were fetched today.

## Sources

1. W3C, *Understanding Success Criterion 3.3.1: Error Identification* (WCAG 2.2, Level A), https://www.w3.org/WAI/WCAG22/Understanding/error-identification.html — craft (accessibility).
2. Xbox Accessibility Guideline 103 / Game Accessibility Guidelines, no information by colour alone (Grounded: "color, as well as shape, iconography, and text labels") — shipping games.
3. W3C, *Understanding SC 1.4.11 Non-text Contrast* and this crate's `text-input.md`, `number-field.md`, `color-field.md` (the rows a problem is drawn on) — technical.

## Craft principle

WCAG 3.3.1: "If an input error is automatically detected, the item that is in error is identified and the error is described to the user in text." and "The error must be indicated in text." Also: "It is perfectly acceptable to indicate the error in other ways such as through the use of an image, color, or other visual indicator, in addition to the text description." And the note on inline placement: "it may be more appropriate to show errors inline, with error messages next to the specific fields that are in error."

## How shipping games do it

Xbox 103: "When color alone is used to signify an element or action ... players who are unable to discern the difference between those colors will be unable to use this key information", so Grounded pairs colour with shape, iconography and text. A game's settings and character-name fields say what is wrong beside the field ("Name already taken"), not in a toast that leaves.

## Model adopted

- **F1. `FieldProblem { text, kind: FieldProblemKind::{Warning, Error} }` is a component on a text row** (`text_input`, `number_field`, `color_field`, a vec field's component row). Inserting it draws `text` under the row; removing it removes the text in the same frame (a hook queues the sync, as `Tooltip`'s does); inserting another over it changes the line in place.
- **F2. The message is words in the text colour, with a mark and the frame in the tone.** `danger` and `accent` are held to 3:1 against the surface (non-text), not the 4.5:1 small text needs, so colouring the words themselves would fail 1.4.3: the words are `text`, the bar at their left and the field's frame are the tone (Error: `danger`; Warning: `accent`). The words alone say it (3.3.1); the colour is the second cue.
- **F3. It takes room.** The message is an absolute child under the row; the row's bottom margin is the message's measured height, so the next row moves down and nothing overlaps; one line costs one line, a long message wraps at the row's width.
- **F4. No input.** The message is `Pickable::IGNORE`, no `TabIndex`; focus, the caret and the ring are the field's.

## Parameters

|part|value|
|---|---|
|component|`FieldProblem { text: String, kind: FieldProblemKind }`, `FieldProblem::error(text)`, `::warning(text)`|
|message|a `Role::FieldMessage` child under the row; a `Role::FieldMark` bar `3 × border` wide at its left; body text size|
|frame|`Role::TextField` border: `danger` for Error, `accent` for Warning, but the focus ring while the field has focus|

## Validation targets

- Inserting `FieldProblem` draws its text under the row; removing it removes the text in the same frame (the issue's acceptance); replacing it changes the line.
- The message is entirely below the row and inside the window; a row below it moves down by the message's height; removing it moves it back.
- The field's frame is `danger` (Error) or `accent` (Warning), and the focus ring while focused.
- The words contrast with the panel at 4.5:1 in all four looks; the bar and frame at 3:1.
- A message wraps inside the row's width at 200% text; thirty idle frames write nothing.

## What great looks like

A form where the wrong field is outlined in red, a one-line reason sits under it with a red bar at its left, and fixing it closes the gap without a flash.

## Deviations

A warning is the accent until named tones (G-29, #33) give the skin a warning colour. Design default.

## Pending index

The WCAG and Xbox pages (not in home-still).
