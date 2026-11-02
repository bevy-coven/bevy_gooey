# Text input

`text_input(label, value)` is a row for a line of text the player types: a name, a seed, a server address, what to call a save. The label is on the left, outside the field and always visible, and a framed field is on the right: Bevy's own headless `TextInput`, so it has a caret, a selection, copy and paste, and the platform's input methods (IME). The root is the row, which keeps the text in `TextInputValue` and says each edit in `ValueChange<String>`.

```rust
fn save_dialog(mut commands: Commands) {
    commands.spawn((
        text_input("Save as", ""),
        TextHint("Name this save".into()),
        TextLimit(16),
        observe(|change: On<ValueChange<String>>, mut commands: Commands| {
            if change.is_final {
                commands.spawn(toast(format!("Saved as {}", change.value)));
            }
        }),
    ));
}
```

- **The state** is `TextInputValue(String)` on the row. Read it to know what the player typed, or write it to change the field from code (a loaded profile, "reset"): the field shows it in the same frame with the caret at its end, and no event comes back for a change you made. A `TextLimit` stops what the player types or pastes and never cuts a value you set.
- **The event** is Bevy's `ValueChange<String>` on the row. `is_final` is false for each edit that changes the text, so a game that wants every keystroke (a live search) has it, and true when the edit ends: **Enter** (a submit, even with nothing changed; the field keeps focus) or **focus leaving the field** after a change (once, however it left). A caret that only moves says nothing.
- **Editing is live.** A focused field takes the keys at once; there is no "press Enter to start typing". A click on the field, its label or the row's padding focuses it (the whole row is the target), and so does Tab, an arrow or an `AutoFocus` on the row. A `dialog()` opens with its first field focused.
- **A field never traps focus.** Left, Right, Home and End are the caret's. **Up, Down and Tab leave the field**, and so does the D-pad: a pad cannot move a caret, so every pad direction moves focus. **Escape ends the edit** (Bevy's own handler blurs the field and lets the press go on) and, inside a `dialog()`, is Back as everywhere: one `Cancel` for the dialog, with the text left as typed for the game to keep or drop. With no dialog open nothing is sent, so Escape stays your pause key. The crate's keys are in [navigation](navigation.md).
- **The frame says who has the keys.** The outline at rest, a lighter frame under the pointer, the focus ring while the field holds focus, **even after a pointer press hid every other ring**: a click puts the caret in the field, and the frame agrees with it. The caret is the text colour, the selection the accent with the accent's text. `GooeySkin::problems()` checks the hint (`text_muted`) against the panel at 4.5:1 and the ring against it at 3:1.
- **The hint is supplementary, never the label.** `TextHint` shows muted words in the field while it is empty ("Name this save"). The label stays outside and always shows: a hint that is the only label disappears when the player starts typing, which is when they need it ([NN/g](https://www.nngroup.com/articles/form-design-placeholders/)).
- **It never changes size.** The field is 17 characters wide at the body size (it follows `text_scale`): 16, and the one character of room Bevy keeps after the caret, so a name that fills a `TextLimit(16)` shows whole with its caret. Past that it scrolls; typing 60 characters moves no pixel of the row, the panel or its neighbours, at any scale. The field is at least `min_target` high.
- **A disabled row** (`InteractionDisabled` on it) disables its field: muted text, no edit, Left and Right move focus again.

## For a game that opens a keyboard, or has hotkeys

- **`TextEditing`** is a marker on the row exactly while its field has focus. A game that opens the platform's on-screen keyboard for a pad player (Steam Deck wants it opened by the game: Valve requires an on-screen keyboard "when requiring the user to input text", and `ShowFloatingGamepadTextInput` "sends OS keyboard keys directly to the game") does it when `TextEditing` is added: the keys it sends are ordinary key events the field already takes. The crate opens nothing and has no platform dependency.
- **`is_text_editing`** is a run condition: `app.add_systems(Update, walk.run_if(not(is_text_editing)))` keeps "W" from walking the character while the player types a name. `TextEditing` is put on and taken off in `GooeySystems::Navigate`, so a system that must not act on the Escape that ends an edit runs before that set, and one that must see the frame's change after it.
- **`TextInputOf`** is on the field and the hint: the row they belong to, for an observer of Bevy's own `FocusGained` or `TextEditChange` on the field.

## What the crate does not do

- **One line only.** No multi-line area: it needs Up and Down for itself and so a different rule for leaving it.
- **No password mask, no filter, no undo.** Bevy's `EditableText` lists masking and undo as not implemented; a character filter (digits only) is a game's rule: observe `ValueChange<String>` and write `TextInputValue` back.
- **No on-screen keyboard.** A pad types through the platform's keyboard or not at all; selection-based entry is slow ([Wobbrock, Myers and Aung](../research/text-input.md)), and every platform ships its own.
- **Validation and error text.** Whether a name is taken is the game's; show it with a `toast_alert` or a `label` under the row.

## A text area

`text_area(label, value, rows)` is the same row with the label over a field `rows` lines high that wraps at its width and scrolls past the last line, for notes and descriptions.

```rust
commands.spawn((
    text_area("Notes", "", 5),
    TextLimit(500),
    observe(|change: On<ValueChange<String>>| {
        if change.is_final {
            info!("saved {} lines", change.value.lines().count());
        }
    }),
));
```

![The workspace example's Inspector: focus is on the Notes field (five lines high, a caret at the end of "Walls hold the roof up."); Enter starts lines "Doors open." and "Lamps hang.", the line under it counts "Editing, 3 lines."; Ctrl+Enter ends the edit and it says "Saved 3 lines." with the field's frame back to the outline](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/text-area.gif)

- **Enter adds a line** (Bevy's `allow_newlines`); **Ctrl+Enter or Cmd+Enter ends the edit**: one final `ValueChange<String>` with the text as it stands, the field kept. Either chord works on every platform.
- **Up and Down are the caret's** (a line up or down): the arrow keys never move focus out of an area. The D-pad still does (a pad has no caret), Tab leaves, and Escape blurs, so the field never traps the keyboard (WCAG 2.1.2).
- **The field is `rows` lines high and stays so**: typing never grows it (it scrolls with the caret), and nothing in the panel moves.
- **`TextLimit` is Bevy's rule**: an edit that would pass the limit is ignored, so a paste that does not fit is not taken at all, and a paste that fits is. The game's own `TextInputValue` is never cut.
- Everything else is the text input's: `TextInputValue` is the state (a write shows with no event), `TextHint`, `FieldProblem`, `TextEditing` and `is_text_editing`.
