# Colours

`color_field(label, colour)` is a row for a colour the player sets: a tint, a light, a material. The label is on the left, then a **swatch** and a framed **hex field**. The field is the same field as [`text_input`](text-input.md) (a caret, a selection, paste, IME, focus it never traps) with a colour behind it, and the swatch opens a **picker**.

```rust
fn material(mut commands: Commands) {
    commands.spawn((
        color_field("Albedo", Srgba::hex("#ff8800").unwrap()),
        observe(|change: On<ValueChange<Srgba>>| {
            if change.is_final {
                info!("albedo is {}", hex_text(change.value.to_f32_array()));
            }
        }),
    ));
}
```

![The colors example: a Material panel with an Albedo row (a swatch and the hex #FF8800), an Emission row whose field is a muted "--" and a Tint row; a press on the Albedo swatch opens a picker with a square of saturation and value, and strips for hue, brightness and alpha, a drag in the square and a press on the hue strip roll the swatch and the hex to a blue, Escape closes it, typing #ff8800 with Enter brings back the orange, Emission takes a green from its own picker, and typing "orange" in Tint is refused with "that is not a hex colour"](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/colors.gif)

- **The state** is `ColorValue(Srgba)` on the row; read it, or write it from code and the swatch, the text and an open picker show it in the same frame with no event coming back. The player's change is one `ValueChange<Srgba>` on the row: `is_final` is false while a drag is still rolling the colour and true when it ends, on Enter, when focus leaves the text after an edit, and for each key or D-pad step. The colour applies as it is picked; there is no OK to press.
- **Typing a colour.** Click the field and type `#ff8800` (or `f80`, or `#ff880080` with an alpha: 3, 4, 6 or 8 hex digits, the `#` optional, either case). **Enter**, or focus leaving the field after an edit, applies it, and the field shows it as it shows colours (`#f80` is `#FF8800`; the alpha only when it is not full). **Escape** puts the old colour back and sends nothing. Anything else is refused: the field shows the colour again and a `ColorRefused { text, reason }` is sent on the row; `reason` is a `model::HexError` that reads as words ("a colour is 3, 4, 6 or 8 hex digits"). Typing is never the only way, and neither is the picker.
- **The swatch** is a button (Tab, then Enter, Space, South or a click): it opens the picker below it, and again closes it. It is split in two halves, the colour opaque on the left and at its own alpha on the right, so a translucent colour reads as one without a checkerboard.
- **The picker** is a modal popover: focus moves in, **Escape** (or the pad's East button) or a press outside closes it and focus goes back to the swatch.
  - The **area** is saturation across and value up. A press puts the ring under the pointer and a drag follows it, held to the edge when it leaves the area (one final change when the pointer lets go). Focused, the **arrows** step it a hundredth (Shift a tenth), a held key repeating; Tab leaves it.
  - Under it are three strips, each Bevy's `Slider`: **Hue** (the six primaries), **Brightness** (black to the colour at full value) and **Alpha** (clear to the colour). Left, Right, Home, End and the D-pad work as on every slider.
  - **A pad** has no 2-D control: the D-pad's Left and Right on the area step the saturation (a twentieth a press, as a slider's) and its Up and Down move on; the strips are the brightness, hue and alpha. Every channel is reachable with no keyboard and no mouse.
  - The picker keeps the **hue, saturation and value** it is on, because a colour cannot (a grey has no hue and a black no saturation): dragging to black and back, or typing a grey, keeps where the player was. A colour a game sets moves them only for what the colour says.
- **A mixed colour.** `MixedValue` on the row is a multi-selection whose members differ: the field is empty and shows a muted `--`, both halves of the swatch are clear, and `ColorValue` is where the picker starts. Any change the player makes takes the marker off and is the one colour for every member; Enter on the untouched field changes nothing.
- **A disabled row** (`InteractionDisabled` on it) opens no picker and takes no text.
- **It does not move.** The field is ten characters wide (`#RRGGBBAA` and the room Bevy keeps after a caret at its end, so the `#` never scrolls out), the swatch a target square, and neither changes size as the colour does. The picker's area is 136 px high at any text size, so 200% text does not push the strips out of a 720p window; the picker is about 450 px tall at 100%.

The maths is in `model`, engine-free: `parse_hex`, `hex_text`, `hsv_to_rgb`, `rgb_to_hsv`, `area_values`, `HexError`. No colour of the skin is added: the swatch frame and the area's frame are the `outline` (3:1 on the panel), the focus ring is the skin's, and the ring on the area is two tones, the text colour inside the panel's colour, so it shows on any fill.

## What the crate does not do

- **No eyedropper, palette swatches, HSL or RGB sliders, or colour-space switch.** A game's palette is a row of buttons it builds itself.
- **No checkerboard behind alpha**: the swatch is split instead, and the alpha strip is drawn over the panel. A checkerboard is an image or a shader, and this crate draws neither.
- **A picker taller than the window is clipped.** At 200% text in a 720p window the strips do not all fit under the swatch (Bevy's `Popover` flips to the side with the least clipped and never shortens a panel).
