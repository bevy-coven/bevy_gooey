# Elevation as data, with CSS shadow semantics

Researched 2026-11-07 for issue #55 (G-48), asked by bevy_grimoire's feel series (dark-incorporated/bevy_dark_factory#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. Its tokens were measured from source and drawn again here with Bevy's `BoxShadow`. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune-kit/src/kit.rs`, `Elevation`: elevation tokens are "box-shadow layers (outer and inset), painted in order", per height. `raised`, `floating` and `overlay` are the three a surface uses; the others are for buttons, switches and knobs.
2. Rune, `crates/rune-kit/src/neutral.rs`: the neutral kit's stacks.
   - `raised` is `drop(1, 2) + ring`, `floating` is `drop(4, 8) + ring` and `overlay` is `drop(8, 16) + ring`.
   - `drop(y, blur)` is a shadow `y` px down with a CSS blur of `blur` px, in `theme.shadow`.
   - `ring` is a zero-blur layer with a 0.5 px spread, in `theme.card_border`.
3. Rune, `crates/rune/src/tok.rs`, the neutral themes.
   - Dark: `shadow = oklcha(0, 0, 0, 0.42)` and `card_border = oklcha(1, 0, 0, 0.05)`.
   - Light: `shadow = oklcha(0.2, 0.02, 265, 0.16)` and `card_border = oklcha(0, 0, 0, 0.1)`.
4. CSS Backgrounds and Borders Module Level 3, `box-shadow`: the blur radius approximates "a Gaussian blur with a standard deviation equal to half the blur radius".
5. Bevy 0.20, `bevy_ui_render/src/box_shadow.wesl`.
   - `roundedBoxShadow` samples `gaussian(y, blur)` and integrates `erf(x * sqrt(0.5) / blur)`. Both are a Gaussian whose standard deviation is the `blur` the vertex carries.
   - `box_shadow.rs` resolves `ShadowStyle::blur_radius` into that `blur` (logical px × the target's scale factor), and sizes the quad to `shadow_size + 6 * blur` (three deviations each side).
   - So Bevy's `blur_radius` is CSS's blur divided by two.

## Model adopted

- `ShadowLayer { x, y, blur, spread, color }` in CSS terms and logical px. The spread is doubled for Bevy, whose `spread_radius` grows the box's whole width (`shadow_size = size + spread`, half on each side) and its height in proportion to the width, so it matches CSS exactly on the left and right only. `Elevation { raised, floating, overlay: Vec<ShadowLayer> }` lives on `GooeySkin::elevation` as serde data.
- `model::shadow_sigma(css_blur) = css_blur / CSS_BLUR_PER_SIGMA` (2.0), read off the shader as above, so `0 8px 24px` is Bevy's 12 px.
- **Where each height is drawn.**
  - `raised`: `Role::Panel` (a host's own too) and `Role::CardCell`.
  - `floating`: `Role::Popover` (which the colour picker and a select's popover are), `Role::MenuPanel` and `Role::Tooltip`.
  - `overlay`: a panel over a dialog's scrim, `Role::Palette` (the command palette, and a filtering select's list, which is the palette's card) and `Role::Toast`.
  - A toast's shadow fades with its opacity.
- **How it is drawn.** `give_elevation` inserts the `BoxShadow` on those roles, chained before `apply_skin`, so the shadow is restyled on the same frame and a skin change restyles it.
- **Presets.**
  - `dark` and `tool` carry Rune's dark stacks, `parchment` the light ones.
  - `high_contrast` carries none: its white edges carry depth.
  - `from_seed` picks dark or light by its side.
- **Checks.** `problems()` reports a blur that is negative or not a number, and an offset or spread that is not finite, as `BadSize { field: "elevation.<level>" }`.

## Where this differs from Rune and from CSS

- **Select lists are overlays, not floating.** A filtering select's list is the command palette's card over a scrim (`Role::Palette`), so it casts `overlay` with the palette. The issue put select lists at floating; the card is one widget, and one role has one height.
- **Bevy does not cut the shadow out under the box.** CSS draws an outer shadow only outside the border box; Bevy's shader draws the whole Gaussian under the node. A translucent fill (dark's panel at 0.92, parchment's at 0.95, a toast mid-fade) lets a little of the drop show through.
- **A role that stops casting gives its shadow back.** `give_elevation` marks the shadows it gave (`Elevated`) and clears one when the role no longer stands at a height; a shadow a host set itself is never touched.

- Rune recolours zero-blur rings from the theme at paint time (`Kit::ring`). Gooey's ring colour is part of the layer and moves with the skin.
- The old tooltip and popover shadow (the background at 0.7 alpha, two borders down, blurred a pad) is gone in favour of `floating`.
