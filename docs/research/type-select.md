# Type-select in lists, tables, trees, menus and selects

Researched 2026-11-07 for issue #63 (G-56), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. Its rules were measured from its source and written again here on gooey's own widgets. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune/src/runtime/input/focus.rs`:
   - `TYPE_SELECT_ROLES` is menu, listbox, list, tree and tablist.
   - `TYPE_SELECT_PAUSE = 1.0` seconds.
   - `type_select` lower-cases the buffer and clears it after a pause. A buffer of one repeated letter cycles from the item after the focused one; a longer buffer matches from the start.
   - `focus_key` returns early when Cmd, Ctrl, Alt or Super is held, and takes only text that is not whitespace (`!t.trim().is_empty()`), so Space never reaches type-select.
2. Rune README, "Focus and modals" and "Key routing" step 5: type-select happens in focus groups, and typed capitals still type-select.
3. W3C ARIA Authoring Practices, the listbox, tree view and menu patterns, "Type-ahead is recommended for all listboxes" and "characters typed in rapid succession", which gooey's tree already followed (`model::type_ahead`, `TYPE_AHEAD_SECS`, iteration on the tree).

## Model adopted

- **One prefix for the whole keyboard.** `TypedPrefix` is a resource that `GooeyNavPlugin` inits. It clears after `TYPE_AHEAD_SECS` (1.0 s, the same as Rune's pause) on `Time<Real>`, or when the group it was typed in changes. A prefix of one repeated letter is that letter.
- **One matcher.** `model::type_ahead` is the tree's existing rule. A one-character prefix starts after the cursor row; a longer one starts on it. Rune starts a longer prefix from the top instead. Starting on the current row keeps "ca" on "Camera" while the player types "cam", which is the tree behaviour gooey already shipped and tests.
- **Where it runs:**
  - A table's rows, and so a tree's, are `table_by_input`'s. The label is the first column of each display row, and the cursor moves and scrolls into view as an arrow key's move does. A tree keeps `*` for opening siblings.
  - The tree's own type-ahead in `tree_by_input` is gone (clean cutover).
  - `type_select_by_input` covers an open menu's items (a `MenuItem`'s label, under its own panel and not a submenu's), a list's shown, enabled tab stops (the first text under each, in reading order; `scroll_to_focus` follows the focus) and a focused select that does not filter (≤ `SELECT_FILTER_ABOVE` options: one `ValueChange<usize>` through the choice's own move, `Move::To`).
- **Never** while a binding row captures (`not_capturing`) or a text field has focus (`not(is_text_editing)`). Space, whitespace and anything typed with Ctrl, Alt or Super held are not type-select characters. No match leaves focus where it is.

## Where this differs from Rune

- Rune also type-selects in a tab list. gooey's tabs are three to seven labels, which the arrows and the bumpers already reach, so this item leaves them alone.
- A filtering select (more than eight options) types into its own field, which is that list's type-select.
