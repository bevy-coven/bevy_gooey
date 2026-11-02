# Searchable select

Researched 2026-11-02 (iteration 38, #20 G-16). home-still's index has nothing on combobox patterns, so the sources below were fetched today.

## Sources

1. W3C, *WAI-ARIA Authoring Practices, Combobox Pattern* and the *Select-Only Combobox* and *Editable Combobox With List Autocomplete* examples, https://www.w3.org/WAI/ARIA/apg/patterns/combobox/ — craft.
2. Xbox Accessibility Guideline 112 (*UI navigation*), https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/112 — shipping games.
3. This crate's `command-palette.md` (`fuzzy_score`, the lit row and the field that keeps focus), `popover.md`, `choice.md` and Bevy 0.20 `bevy_ui_widgets` — technical.

## Craft principle

APG, Combobox: "DOM focus is maintained on the combobox and the assistive technology focus is moved within the listbox using aria-activedescendant" and, for a select-only combobox, "Enter: Sets the value to the content of the focused option in the listbox. Sets visual focus on the combobox." A filtering list also announces its count; with none, it says so.

## How shipping games do it

A settings screen with a choice of dozens (a language, a resolution, a controller preset) cannot be a row of buttons or a left-right stepper: shipped games open a list over the screen and let the player type to narrow it. XAG 112: "The UI is fully navigable by keyboard and controller digital input alone": the list is walked with Up, Down, the D-pad and South, and a pad player never needs to type.

## Model adopted

- **L1. `select(label, options, selected)`** is a `choice()` row (label, current option, arrows) when there are **8 or fewer** options, and a `Role::Select` row of the label and a button showing the current option when there are more (the issue: "turns into a filtering popover list when there are more than 8 options"). Both keep `ChoiceValue` as the state and send `ValueChange<usize>` on the row (the index in the original list), so a game reads either one way.
- **L2. Opening** (Enter, Space, South, a click) opens a modal popover under the row with a text field that has focus and the matches below it, the command palette's parts: `rank` by `model::fuzzy_score`, the current option lit when the query is empty, the first match lit after typing. Up, Down, PageUp, PageDown and the D-pad move the lit row (wrapping, the caret kept at the end), Enter or a click chooses, Escape or an outside press closes with the value unchanged and focus back on the row.
- **L3. No match** shows "No option matches" and Enter chooses nothing; the value is unchanged (the issue's edge case).
- **L4.** The list shows at most `SELECT_ROWS` (8) rows in a window that follows the lit row, with "n of m" beyond, like the palette.
- **L5.** Choosing sends one `ValueChange<usize>` (`is_final`), closes the list and gives focus back; choosing the current option sends nothing.

## Parameters

|part|value|
|---|---|
|widgets|`select(label, options, selected)`; state `ChoiceValue`, `ChoiceCount`; `SelectOptions`|
|threshold|`SELECT_FILTER_ABOVE` = 8 options|
|rows|`SELECT_ROWS` = 8, a target high, the menu row's look|
|match|`model::fuzzy_score` on the option text, best first, ties by the original order|

## Validation targets

- 40 options: typing "par" leaves only matching rows and Enter selects the first, one `ValueChange` with its original index (the issue's acceptance).
- No match: "No option matches", Enter changes nothing and closes nothing.
- Escape and an outside press close and leave the value and send nothing; focus returns to the row.
- 8 options or fewer is a `choice` (no popover); 9 is a select.
- Up and Down move the lit row and the field keeps focus; every row a target high; the card inside the window at 1x and 2x text.
- Thirty idle frames write nothing.

## What great looks like

A row that reads like a choice, opening a clean card of one field and a short list under it, the first match lit, no jump when typing, a count when the list is long.

## Deviations

- `select` is one function returning `impl Bundle`, which cannot be two different bundle types by count, so it returns a row that is the same `Role::Select` in both cases and hides the arrows below the threshold. Design default.
- Typing is not filtered on the pad: a pad player walks the whole list; there is no on-screen keyboard (the host's hook, `TextEditing`).

## Pending index

The APG combobox pages and the Xbox guidelines are not in home-still.
