# Text on the device-pixel grid

Researched 2026-11-07 for issue #59 (G-52), asked by bevy_grimoire's feel series (dark-incorporated/bevy_dark_factory#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune/README.md`, "Layout": "Text sizes round up to whole device pixels, so layout stays on the pixel grid."
2. Rune, `crates/rune/src/runtime/commit/text_layout.rs`, `CeilTo::ceil_to`: a length is rounded up as `(length × scale − 1e-3).ceil() / scale`, so one already within a thousandth of a device pixel of a whole number is not pushed a pixel up.
3. Rune, `crates/rune/src/text.rs`, `measure`: the measured text box rounds up the same way (`full_width().ceil() / scale`).
4. Rune, `crates/rune-kit/README.md`, "Known differences": Rune keeps Parley's fractional glyph advances and rounds text boxes up to a device pixel.
5. Bevy 0.20.
   - `bevy_ui/src/update.rs`: a UI node's `ComputedUiRenderTargetInfo::scale_factor` is the camera target's scale factor times `UiScale`.
   - `bevy_text/src/text.rs`: `FontSize::Px` is logical px, and the text is laid out at that scale factor.

## Model adopted

- `snap_text_size(size, scale)` is a public pure function beside `snap_line_width`. It returns the smallest logical size whose device size `size × scale` is a whole number, never below `size` and less than one device pixel above it.
  - It uses Rune's tolerance: a size within a thousandth of a device pixel above a whole number counts as that number.
  - It reuses the ulp stepping of `snap_line_width`, so `scale × result` is exactly the whole number as an `f32`.
  - A size or scale that is not finite and above zero comes back unchanged.
- `apply_skin` writes every widget's `TextFont` size through it, at the node's own scale factor. That one place sets every text size in this crate.
- **When it is set again.** `ComputedUiRenderTargetInfo` is in the restyle filter already, so a new window scale factor or a new `UiScale` re-sets every text in the frame after Bevy computes it (Bevy writes it in `PostUpdate`, after `apply_skin` in `Update`). A new node's first frame is styled at the default scale of 1 for the same reason, so at another scale its text is set on the grid one frame later. Line weights lag the same frame. `set_if_neq` keeps a settled interface writing nothing.
- **Heights that hold a line.** A list's row, a text field's inset and a keycap's height are worked out from the body text as it is set, so a list of `rows` rows still holds `rows` rows. Widths keep the unrounded size.
- **A host's own text.** An entity with a `Text` and no `Role` is never styled, so its size is untouched.

## Where this differs from Rune

- **Rune rounds the measured text box, and this crate rounds the font size.** Rune's rule is applied to what text measures (`measure`, `ceil_to`), with fractional advances inside. The issue asks for the font size, which is what decides how sharp a glyph is: a 19.25 px em is rasterised between two pixel sizes, and a 20 px one is not. Bevy's layout already rounds each node's edges to device pixels, so the box is on the grid without a second rule.
- **Rune's CoreText-matched rasteriser is not here.** That needs `bevy_text` changes, and the issue leaves it out.

## Proof

`tests/text_grid.rs`:

- At scale factors 1, 1.25, 1.5 and 2 with `text_scale` 1.1, every widget text of a panel is a whole number of device pixels, at or above its own role's unrounded size and less than a device pixel over it (`every_widget_text_is_a_whole_number_of_device_pixels_at_every_scale`). The panel holds a heading, a label, a button, a button with a cooldown's chip (a type-scale step), a toggle, a slider and a text field.
- 14 px body text at 1.1 and 1.25x (19.25 device px) is set at 16 logical px (20 device px).
- Moving the window to 1.5x, and then changing `UiScaleSettings::scale`, sets every text again.
- A host's own `Text` keeps 13.3 px.
- A Desk list of 8 rows, 8 rows tall at 1.25x, does not scroll: its height counts the rows as they are set (it overflowed by 1.6 px while the height used the unrounded size).
- `snap_text_size` is swept over 5 to 40 px at nine scales.
- On 42a9cf5 (before this change) the three sizing tests fail: 15.4 px stays 15.4 at 1.25x, a 35.2 px heading is off the grid at 1x, and 22 px at 1.25x is 27.5 device px. The host-text test passes on both, as it should.
