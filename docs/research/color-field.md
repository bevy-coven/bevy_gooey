# Colour field: a swatch, a hex field and a picker popover

Researched 2026-11-01 (iteration 36, #17 G-13 slice two). home-still's index has nothing on colour pickers (`catalog_recent`, 200 rows: psychology and domestic-violence papers only, no game UI), so the sources below are fetched today. The three kinds are covered; nothing is pending index.

## Sources

1. W3C, *Understanding Success Criterion 1.4.11: Non-text Contrast* (WCAG 2.2, AA), https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html — craft (accessibility): what a control and its state must show at 3:1.
2. Blender Foundation, *Color Picker*, Blender 5.2 manual, https://docs.blender.org/manual/en/latest/interface/controls/templates/color_picker.html (the page itself answered 403 to the reader; the lines below are the manual's, returned by a search of it the same day) — shipping: the picker every 3D tool's colour property opens.
3. Bevy 0.20 `bevy_feathers::controls::color_plane` (`FeathersColorPlane`, a `UiMaterial` with a WGSL shader, `ColorPlaneValue(Vec3)`) and `bevy_ui::gradients` (`BackgroundGradient`, `LinearGradient`, `InterpolationColorSpace`) — technical: how the plane is drawn, and the way this crate can draw it without a shader.
4. This crate's own `number-field.md` (the typed field and its refusals), `popover.md` (a modal popover that holds focus, closes on Escape or an outside press), `controls.md` (the slider's keys and the D-pad) and `text-input.md`.

## Craft principle

WCAG 1.4.11: "Ensure meaningful visual cues achieve 3:1 against the background." and "Even when a control does not need to have a visual boundary indicating its hit area, it will still need to have a sufficiently contrasting focus indication." A colour area has no colour of its own to measure against (its edge is whatever hue and brightness the player has chosen), so its **frame** and its **marker** must carry the 3:1: the frame is the skin's `outline` on the panel, and the marker is a two-tone ring (the text colour inside, the panel's colour outside) that contrasts with any fill under it, as a slider thumb's frame does.

## How shipping games do it

Blender's picker: "The Color Field lets you pick the first and second color component" and "The slider with a gradient in the background lets you define the third color component"; "Hex displays or accepts the color's hexadecimal (hex) representation. Hex shorthand notation can be used (for example, FC0 for FFCC00)"; "For color inputs that include an Alpha Channel, an additional slider and field are shown"; "Holding Ctrl while dragging snaps the hue", "Shift-LMB dragging for precise movement". So the shape is a 2-D field, a gradient slider for the third channel, an alpha slider, and a hex field that takes shorthand.

Bevy's own `bevy_feathers` plane draws its gradient with a `UiMaterial` and a shader (`color_plane.wesl`), which needs `bevy_render` and a GPU: this crate's tests run a headless window with no render backends and `cargo check --lib --target wasm32` compiles only the trimmed features. `BackgroundGradient` is in `bevy_ui`, needs no shader and is drawn by whichever renderer the game has, so the picker is data a headless test can read.

## Technical

HSV is the model a picker edits (a hue can be set at any saturation, and black at any hue). Converting an `Srgba` back to HSV loses the hue of a grey and the saturation of a black, so the picker keeps its own hue, saturation and value, and an outside write of a colour (a game's, a typed hex) changes them only for the channels the colour still defines. A gradient between the hue's six primaries in sRGB is the hue ramp (`InterpolationColorSpace::Srgba`): the area is the hue's pure colour with a white-to-clear horizontal gradient and a clear-to-black vertical one on top.

## Model adopted

- **C1. `color_field(label, Srgba)` is the text row with a swatch.** The row is `text_row_as` (label, hex field): the field is a real text field showing `#RRGGBB` (or `#RRGGBBAA` when the alpha is not 1), so typing `#ff8800` and Enter sets the colour (the issue's acceptance), shorthand (`#f80`, `#f80c`) and a missing `#` are taken, and anything else is refused with its reason (`ColorRefused { reason: HexError }`, as `NumberRefused`). Between the label and the field is a swatch: a button, a target square, split in two halves, the colour opaque on the left and at its alpha on the right, so a translucent colour is visible as such with no checkerboard.
- **C2. State is `ColorValue(Srgba)` on the row; the player's change is one `ValueChange<Srgba>`** (`is_final` false while a drag rolls it, true at its end, on Enter and for a step); a game's write shows at once and sends nothing. `MixedValue` works as in the number field: the field is empty with the muted `--` hint, both swatch halves are clear, and `ColorValue` is where the picker starts.
- **C3. The swatch's `Activate` (a click, Enter, Space, South) opens a modal popover below it, and again closes it.** It is `popover_modal` (focus moves in and comes back, Escape or the pad's East closes it, an outside press closes it) and the colour applies live: there is no OK to press.
- **C4. The picker is an area, three strips and nothing else:** the **area** is saturation across and value up, the pointer sets it (a press and a drag, one final change at the end), the arrows step it by 1% (Shift 10%); under it a **Hue** strip (the six primaries), a **Brightness** strip (black to the colour at full value) and an **Alpha** strip (clear to the colour), each Bevy's `Slider` so a pointer, Left, Right, Home, End and the D-pad work as on every slider. A pad has no 2-D control: the D-pad's Left and Right step the area's saturation (as a slider's), its Up and Down move on, and brightness is the strip's, so every channel is reachable with a pad (WCAG 2.5.7: the typed hex is the other way for a drag).
- **C5. The area's marker is a ring of two tones** (the text colour inside, the panel's outside) at the colour's place, and the area has the outline frame and the focus ring: no colour of the skin is added and `GooeySkin::problems()` is unchanged.
- **C6. One path for the state.** `ColorValue` is the colour; the picker keeps `ColorPicker { hue, saturation, value }` only so that a grey keeps its hue. A sync system (`update_colors`, Style) writes the hex text, the swatch, the strips' values, the marker and the gradients from them, only on a change.

## Parameters

|part|value|
|---|---|
|widget|`color_field(label, Srgba) -> impl Bundle`|
|state|`ColorValue(Srgba)`, marker `MixedValue`|
|events|`ValueChange<Srgba>`, `ColorRefused { entity, text, reason }`|
|hex|`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`, the `#` optional; shown upper-case, with alpha only below 1|
|area|12 body text sizes wide (or the strips' width), 7.5 high; arrows 1%, Shift 10%|
|strips|hue 0 to 360 (step 1% of the range), brightness and alpha 0 to 1 (step 0.01); `Slider` keys and D-pad|
|set|`update_colors` in `GooeySystems::Style` before `update_text_inputs`; the pointer and key edits are observers|

## Validation targets

- Typing `#ff8800` and Enter in the field gives `Srgba::hex("#ff8800")`, one final `ValueChange<Srgba>`; `#f80` gives the same; `ff8800` too; `#ff880080` has alpha 0.502; `orange`, `#ff88` (4 digits is RGBA: valid), `#ff880` and an empty field are refused with a reason and the text is put back; Escape reverts.
- A game's `ColorValue` shows in the swatch and the field in the same frame with no event; thirty idle frames write nothing.
- The swatch opens the picker (Enter, Space, a click, South) and a second activation closes it; Escape closes it and focus returns to the swatch; the picker's area, strips and the swatch are all reachable by Tab.
- A press at the area's right edge, top gives saturation 1, value 1; its left, bottom gives 0, 0; a drag past the edge holds; the pointer press is not final and the release is one final change; the marker is at the pointer.
- Dragging the area to black and back keeps the hue; a grey set by hex keeps the hue the picker had.
- The Hue strip's end is the same red as its start; moving it changes the hue and nothing else; Brightness and Alpha likewise; the Alpha strip changes only `a`.
- Arrows on the area step it and move no focus; the D-pad's Left and Right step saturation and its Up and Down move focus off the area; Tab leaves it.
- The swatch is at least 24 px (a target high at 1x), frame at 3:1 on the panel, the marker's two tones differ at 3:1 from each other and one of them from any fill; the picker is whole inside a 1280x720 window.
- A disabled row opens nothing, takes no text.
- Frame time of the gallery within 0.5 ms of before.

## What great looks like

A picker that reads at once: a saturated square with a crisp white ring, three slim gradient strips beneath it with the same thumb as every other slider, the hex field above in the one place every tool puts it. The swatch beside the hex is the colour, large enough to judge it. Nothing flickers while dragging (gradients are rewritten only when the hue or the colour changes) and a drag to black keeps the hue so coming back is not a surprise.

## Deviations

- Blender picks the third component with one slider and offers HSV, HSL and RGB; this picker is HSV only with three strips, because a pad needs each channel as a slider and RGB/HSL would double the controls. Design default.
- No eyedropper, palette swatches or colour-space switch: a game's palette is a row of buttons it builds itself.
- Alpha is shown by splitting the swatch, not by a checkerboard (a checkerboard is an image or a shader, and this crate draws none). Design default.

## Pending index

None. (home-still has no game-UI or colour-picker document; the sources above are web pages, recorded by URL.)
