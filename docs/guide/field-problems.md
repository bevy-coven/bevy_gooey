# Field problems

A form says what is wrong next to the field, in words, not in a toast that leaves. Put `FieldProblem` on a text row (`text_input`, `number_field`, `color_field`, a `vec_field`'s component) and its text is drawn under the row. Remove it and the text is gone in the same frame.

```rust
fn rules(
    mut commands: Commands,
    names: Query<(Entity, &TextInputValue, Has<FieldProblem>), Changed<TextInputValue>>,
) {
    for (row, name, has) in &names {
        if name.0.trim().is_empty() {
            commands.entity(row).insert(FieldProblem::error("A name is needed"));
        } else if has {
            commands.entity(row).remove::<FieldProblem>();
        }
    }
}
```

![The forms example: the empty Name field is framed red with "A name is needed" under it behind a red bar; typing "Aurora" removes the message and the rows below close up](https://raw.githubusercontent.com/dark-incorporated/bevy_gooey/main/docs/media/forms.gif)

- **The words are the message** (WCAG 3.3.1 asks for the error to be "described to the user in text"), in the text colour at 4.5:1. The colour is a second cue: a bar at the start of the message in `GooeySkin::danger` for `FieldProblem::error` or `GooeySkin::accent` for `FieldProblem::warning`, and an error also frames the field in `danger`. `danger` and `accent` are held to 3:1 against the panel, which a bar and a frame need; small text needs 4.5:1, which is why the words are not coloured.
- **A warning leaves the frame alone.** It does not block, and the accent is the focus ring's blue in the Tool look and close to the danger's red in parchment, so a warning frame would read as focus or as an error. The bar and the words say it.
- **The focus ring wins** while the field has focus: the player is in it, the caret says so, and the message and its bar stay.
- **It takes room.** The message is laid out under the row and the rows below make way for it (the row keeps the message's measured height as a margin under itself, a frame after it appears); a long message wraps at the row's width. It takes no input and no focus.
- **Replace it in place.** Inserting another `FieldProblem` over the first changes the words and the kind without a blink; the message entity stays.
- **The check is yours**: the crate draws, it never decides what is wrong. Insert on `Changed<TextInputValue>` (or a `ValueChange` observer, or a `NumberRefused` event) and remove when the value is good.
