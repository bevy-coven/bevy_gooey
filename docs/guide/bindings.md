# Rebinding controls

`binding(label, value)` is a row that rebinds one action: the action's name on the left and a keycap on the right that shows the key, mouse button or pad button it is bound to. Enter, Space, the pad's South button or a click starts a capture, the keycap lights and says what it waits for, and the player's next press is the new binding.

```rust
fn controls(mut commands: Commands) {
    commands.spawn((
        panel(),
        children![
            (
                binding("Jump", Binding::Key(KeyCode::Space)),
                observe(|c: On<ValueChange<Binding>>| info!("jump is now {:?}", c.value)),
            ),
            binding("Fire", Binding::Mouse(MouseButton::Left)),
            binding("Use", Binding::Pad(GamepadButton::South)),
        ],
    ));
}
```

- **The value** is a `Binding`: `Key(KeyCode)`, `Mouse(MouseButton)`, `Pad(GamepadButton)` or `Chord(Shortcut)`. A key is its *place* on the keyboard, the way Bevy's `KeyCode` and a game's `ButtonInput<KeyCode>` read it, so WASD is the same four keys on a QWERTY and an AZERTY keyboard. `Binding` is `Eq + Hash` (a game finds two actions on one input), and with the `serde` feature a game saves its bindings as data. A row spawned with a `Chord` captures a key chord, "Ctrl+Shift+K": see [shortcuts](shortcuts.md).
- **The state** is `BindingValue(Binding)` on the row's own entity. Read it, or write it to change the row from code (a loaded profile, a "reset to defaults"): the keycap follows in the same frame and no event comes back for a change you made. The player's change is Bevy's `ValueChange<Binding>`, once for each change, `is_final: true`; pressing the input the row already holds ends the capture and says nothing.
- **What a row hears.** A row spawned with a key or a mouse button hears any key and any mouse button, the primary one too (the pointer cannot press anything while a row waits, so a click is a binding and presses nothing under it). A row spawned with a pad button hears any pad button. **Escape** exits a capture without binding and cannot be bound; on a pad row so does **Start** (the Menu button on Xbox). A press on the other device, a pad button on a key row or a key on a pad row, exits too: the player picked up the wrong thing.
- **While a row waits, nothing else hears the player.** The arrows, Tab, the D-pad, the bumpers, PageUp and PageDown and Back all stand still (`not_capturing` is the run condition the crate's own input systems run under), and Tab is bindable. No `Cancel` is sent for the Escape that exits: handle Back through `Cancel` and it never fires for a capture. A game that reads raw keys of its own gates them on `Query<(), With<Capturing>>` being empty.
- **It never changes size.** The name and the prompt stand in one cell of the keycap, one hidden, so the keycap is as wide as the wider (at least 17 body text sizes, so a column of rows shares its edges) and waiting moves nothing, at any text size. The row is a toggle's height, with a little air between its focus ring and the keycap.
- **Names** are `Binding::label(PadStyle)`: "W", "Left Shift", "Up Arrow", "Right Click", "A" or "Cross". Show the same words in your hints and tutorials, so a remapped control is named the same wherever the player sees it (XAG 107). `model::{key_name, mouse_name, pad_name}` is the arithmetic, engine-free.

## What a host sets

`BindingSettings` is a resource (serde data under the `serde` feature):

| field | default | what it does |
|---|---|---|
| `key_prompt` | `"Press a key, {exit} exits"` | what a key row's keycap says while it waits |
| `pad_prompt` | `"Press a button, {exit} exits"` | what a pad row's keycap says while it waits |
| `chord_prompt` | `"Press a shortcut, {exit} exits"` | what a chord row's keycap says while it waits |
| `pad_style` | `PadStyle::Xbox` | whose names a pad button gets: "A", "Cross" (`PlayStation`) or "B" (`Switch`) |
| `cancel_key` | `KeyCode::Escape` | the key that exits a capture, and so cannot be bound |
| `cancel_button` | `GamepadButton::Start` | the pad button that exits, and so cannot be bound |

The crate has no strings of its own, so the prompts are the host's to translate; `{exit}` becomes the name of the exiting input, in the pad's vocabulary. A game that must let the player bind Escape (Xbox Accessibility Guideline 107 asks that the Esc key be remappable) moves `cancel_key` to another key.

## What the crate does not do

- **Conflicts.** The row binds what it hears. The game owns the action map: it sees a clash with `Binding`'s `Eq + Hash` (or `model::shortcut_conflicts`, which lists every pair of commands on one input), and either allows it, swaps (write the other row's `BindingValue`) or refuses (write this one's back). XAG 107 wants an action assignable to any input, "as opposed to simply swapping button assignments".
- **Axes, inversion, unbound, chord sequences.** A binding is one key, button or chord; sensitivity and inversion are a `slider` and a `toggle`; a row cannot be cleared; "Ctrl+K, Ctrl+S" is two chords and not one row.
- **Key names that follow the player's layout.** A key is stored as a place and named by its QWERTY cap, so an AZERTY player who binds the key printed "Z" sees "W". Bevy gives the printed character on a key event, never for a stored `KeyCode`; a game with a layout map writes the names itself, starting from `Binding::label`.
- **Glyphs.** The words are drawn in the host's font. Icons for the pad's buttons are the host's.
