# Prompts

A prompt is one hint of what an input does, "Esc Back", and the keycap names the input of the device the player used last: the keyboard's while they use the keyboard or the mouse, the pad's the frame they press a pad button. `prompts()` is the legend that holds them, in the bottom-left corner, where Xbox Accessibility Guideline 112 puts "select" and "back" on every screen.

```rust
fn hints(mut commands: Commands) {
    commands.spawn((
        screen(),
        children![
            (panel(), children![heading("Settings"), button("Back")]),
            (
                prompts(),
                children![prompt("Select", Prompt::SELECT), prompt("Back", Prompt::BACK)],
            ),
        ],
    ));
}
```

![The Quit dialog over the settings panel with the legend "A Select, B Back" in the corner: the keycaps start as the keyboard's, "Enter" and "Esc", turn round ("A", "B") when a pad button is pressed and back to keys on the next key; the dialog has its own legend, undimmed above the scrim](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/prompts.gif)

- **The words are yours, the input is the crate's.** `prompt(label, keys)` takes the label, so the crate has no string to translate; the keycap is `Binding::label`, the same words a `binding` row shows ("Enter", "Esc", "Right Click", "A" or "Cross"), so a hint and a rebinding screen never disagree. A pad button is named by `BindingSettings::pad_style`, the one setting that says "A", "Cross" or "B".
- **A prompt is an action with two inputs.** `Prompt { keyboard, pad }` is the prompt's state. `Prompt::SELECT` is Enter and the pad's South button and `Prompt::BACK` is Escape and the pad's East button: the inputs the crate itself acts on (Space selects too; the prompt names one). For an action of your own, `Prompt::new(Binding::Key(KeyCode::KeyI), Binding::Pad(GamepadButton::North))`. A mouse button is a keyboard-side input: `Binding::Mouse(MouseButton::Right)` reads "Right Click".
- **The device follows the player.** `InputDevice` is a resource, `Keyboard` or `Pad`, kept by `track_input_device`: a key or a mouse button pressed is `Keyboard`; a pad button pressed, or a stick pushed past half its travel (`STICK_PUSH`), is `Pad`. A mouse that only moves changes nothing, so a bump of the mouse does not swap a pad player's hints; a click does. It starts as `Keyboard`: a console or a handheld build writes `InputDevice::Pad` at start-up. Read it too, to swap glyphs of your own (the crate draws words, not icons).
- **A keycap's shape says the device as well.** A key is a rounded rectangle, the keycap of a `binding` row at rest; a pad button is fully round (a circle for "A", a pill for "LB"), in the same colours. The bar is the panel's colour, so the words read over any world.
- **The legend moves nothing.** `prompts()` is absolute, 5% in from its parent's left and bottom edges (the television's safe area), so adding it, or taking it away, changes no other widget by a pixel; it is no wider than 90% of its parent, and a longer legend squeezes its words. Put it in a `screen()`, and one in a `dialog()` too: it stands at the same corner over the scrim, which is where XAG 112 wants the prompts when a dialog opens.
- **A hint is not a control.** Every part is `Pickable::IGNORE` and none is a tab stop: the pointer goes through to the game, and Tab never lands on a hint. A mouse player's way back is still a Back button (`docs/guide/navigation.md#back`).
- **A game that rebinds keeps the hint true.** `Prompt` is a component: set it from the `ValueChange<Binding>` of a `binding` row and the keycap follows in the same frame, in the same entity.

```rust
fn keep_the_hint_true(
    change: On<ValueChange<Binding>>,
    mut inventory: Single<&mut Prompt, With<InventoryHint>>,
) {
    inventory.keyboard = change.value;
}
```

## What the crate does not do

- **One input per device.** "LB / RB Tabs" is two prompts and Ctrl+PageDown has none; a prompt that names a chord or two keys is the game's to spell.
- **Icons.** The words are drawn in the host's font, the pad's buttons as words in a round cap. A game that ships glyph art reads `InputDevice` and draws its own.
- **Click-to-act.** A prompt is a hint. The crate sends `Cancel` for Escape and East only.
- **A Switch pad's habit.** The crate selects with the south button on every pad, so `Prompt::SELECT` on `PadStyle::Switch` reads "B". A game that selects with East there uses `Prompt::new(Binding::Key(KeyCode::Enter), Binding::Pad(GamepadButton::East))` and moves its own handler.
