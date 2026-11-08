# The combobox

Researched 2026-11-07 for issue #73 (G-66), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune `crates/rune-kit/README.md`, "Components", `Combobox`: a text field, a popover and virtual rows; options with stable ids, labels, details and a disabled state; case-insensitive filtering; ArrowUp and ArrowDown and Ctrl-N and Ctrl-P loop across enabled rows; the keyboard choice is scrolled into view; selection keys are ignored during IME composition; Escape closes.
2. WAI-ARIA Authoring Practices, "Combobox Pattern": focus stays in the field while the lit option moves (`aria-activedescendant`); Escape closes the popup.
3. This crate's `palette.rs`: the matcher (`fuzzy_score`), the reused rows with one lit, the keys while focus stays in the field, and the IME rule.

## Model adopted

- **A wrapper over a `text_input` row,** not the row itself: a `text_input` sends a `ValueChange<String>` for every edit, and the combobox's one `ValueChange<String>` is the choice. The field row's own events go to the field row; the combobox's go to the combobox.
- **The list is a `popover` of a `list`** under the field row, one row per option, spawned when the list opens and left when it closes. Rows are reused: typing rewrites their words and hides the rows past the last match, so nothing spawns or despawns while typing. The list gets the wheel, the rail, the glide and the edge fade from `list` (G-58).
- **The lit row is always enabled** (or none is lit). Down and Up (Ctrl+N and Ctrl+P) step to the next enabled match and wrap; the pointer lights only enabled rows.
- **Escape** keeps focus in the field for the list's close: Bevy's text input blurs on Escape and lets the key bubble, so `keep_focus_on_escape` (`PreUpdate`, after dispatch) puts focus back when the list was open, and the `Cancel` the press sends is taken by the popover. A second Escape blurs and bubbles as for any field.
- **Enter** is the list's while it is open (`ComboListOpen` on the field keeps the text input's own Enter from submitting its dialog).

## Where this differs from the issue

- **Opening from the anchor and the height spring (#71 and the size motion) are not built.** The issue depends on bevy-coven/bevy_gooey#71 (G-64), still open; the list is a plain `popover` and takes that motion when #71 lands. The list's height lands with the rows.
- **No virtual rows.** Rune's rows are virtual; this list builds one row per option (`list` is "for tens of rows, not thousands"). A combobox of ten thousand options would need the table's row pool; recorded here, not built.
- **No pad path.** A pad cannot type; the D-pad moves focus off a text field. A pad player uses a `select`.
- **Case-insensitive** comes from `fuzzy_score`, which lower-cases both sides.

## Proof

`tests/combobox.rs`: forty options, typing "par" narrows to Paraguay, Down, Enter sends one `ValueChange` with the id; several matches in rank order; Ctrl+N and Ctrl+P wrap over the enabled options and skip the disabled; a disabled option is never lit and refuses a click and Enter; a click chooses; no match says so and Enter chooses nothing; Escape closes before the second Escape cancels; focus leaving closes and restores the label.
