# Combobox

A desktop picker narrows its options in place, under the field. `combobox(label, options)` is a labelled field with a list under it: type to narrow the list, Down to walk it, Enter to choose. Where a [`select`](select.md) opens a card over the screen, a combobox opens a [popover](popovers.md) under its own field.

```rust
fn country(mut commands: Commands) {
    commands.spawn((
        combobox("Country", [
            ComboOption::new("py", "Paraguay").detail("South America"),
            ComboOption::new("pa", "Panama"),
            ComboOption::new("fr", "France"),
            ComboOption::new("atl", "Atlantis").disabled(),
        ]),
        ComboValue(Some("fr".into())),
        observe(|change: On<ValueChange<String>>| info!("country {}", change.value)),
    ));
}
```

- **An option** is `ComboOption::new(id, label)`, with `.detail("…")` (muted words at the row's end, never matched) and `.disabled()`. The id is what a choice sends; ids are unique in a list.
- **The state** is `ComboValue(Option<String>)`, the chosen id. Set it from code and the field shows the option's label in the same frame, with no event; an id no option has is refused with a warning and shows an empty field.
- **Typing** opens the list and narrows it by the command palette's fuzzy rule ("par" finds "Paraguay"), best match first and the list's own order among equals, and lights the first enabled match. **Down or Up** with the list closed opens it on the chosen option; **a click** on the field opens it too. With nothing matching the list says "No option matches" and Enter chooses nothing.
- **The lit row** moves with Down and Up or Ctrl+N and Ctrl+P, wraps past either end, skips disabled options and is scrolled into sight on the scroll's glide ([lists](lists.md)). The pointer lights an enabled row it moves over. A disabled option is listed and matched but is never lit, and a click or Enter on it chooses nothing.
- **Choosing** (Enter, or a click on a row) puts the label in the field, closes the list and sends one `ValueChange<String>` on the combobox with the id (`is_final`), and none when it was already the choice. Focus stays in the field.
- **Closing without a choice:** the first Escape closes the list and puts the chosen label back; a second Escape is the text field's own, which blurs and sends the dialog's `Cancel` like any field. A press outside, or focus leaving the field, closes it too.
- **Enter in an open list** is the list's: the field does not also submit its dialog's default button.
- **The list** is a `popover` of a `list` showing 6.5 rows (`COMBO_ROWS`), one row per option, so it scrolls with the wheel, fades at its edges and is for tens or hundreds of options, not thousands. Its width is at least the field's.
- **A pad** has no way to type, so it chooses with a `select`. The combobox is a typed field.

`Combobox` holds the options and, while the list is open, `matches()` (indices into `options()`, in the list's order), `lit()` and `list()` (the popover). It is a wrapper around a `text_input` row, so the field's own `TextHint` and the rest of [text input](text-input.md) apply to it.
