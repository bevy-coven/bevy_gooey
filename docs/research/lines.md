# Every line one weight: borders, the focus ring and the meter's gap on whole device pixels

Researched 2026-10-30 (iteration 5). home-still's distill was healthy and idle (`system_status`), so the perception source is a corpus stem; the design-system rule, the shipping game and the layout rule are web pages, each quoted from a fetched copy, and the engine's behaviour is read from the vendored source.

## Sources

1. Wagemans, Elder, Kubovy, Palmer, Peterson, Singh and von der Heydt, *A century of Gestalt psychology in visual perception: I. Perceptual grouping and figure–ground organization*, Psychological Bulletin 2012 — home-still `10.1037_a0029333` (craft: why one weight reads as one family).
2. Microsoft, *Screen sizes and breakpoints* (Windows app design, "Multiples of four") — https://learn.microsoft.com/en-us/windows/apps/design/layout/screen-sizes-and-breakpoints-for-responsive-design (craft: a design system's rule for crisp edges at every scale factor).
3. Wube Software (kovarex), *Friday Facts #277: The scaling problem and solution*, 2019 — https://factorio.com/blog/post/fff-277 (shipping game: Factorio's GUI rewrite for 0.17).
4. W3C, *CSS Values and Units Module Level 4*, §6 "snap a length as a border width" — https://www.w3.org/TR/css-values-4/ — and *CSS Backgrounds and Borders Module Level 3*, `border-width` ("Computed value: absolute length, snapped as a border width") — https://www.w3.org/TR/css-backgrounds-3/ (technical: the rule browsers apply to every border).
5. The engine: Bevy 0.20 `bevy_ui-0.20.0/src/layout/convert.rs` (`Val::Px(value) => style_helpers::length(context.scale_factor * value)`), `update.rs` (the scale factor is `camera.target_scaling_factor() * ui_scale.0`, propagated to every node as `ComputedUiRenderTargetInfo` and re-inserted only when it changes), and taffy 0.14 `compute/mod.rs` `round_layout` (technical: where the uneven pixel comes from).
6. Buitinck et al., *API design for machine learning software: experiences from the scikit-learn project*, 2013 — home-still `W1728842521` (technical: API defaults, for keeping this invisible to the host).

## Craft principle

Similarity is what makes a set of shapes read as one: "all else being equal, the most similar elements (in color, size, and orientation for these examples) tend to be grouped together" (Wagemans et al. 2012, on Wertheimer's principles). A frame that is 4 px on one side and 5 px on the other, or a meter whose gap is a pixel thicker above the fill than below it, is two sizes where the skin asked for one; the eye reads the odd side as a different line, and the fill as sitting off-centre in its slot.

A design system says the same as a rule for scaled interfaces: "The sizes, margins, and positions of UI elements should always be in multiples of 4 epx in your XAML apps. XAML scales across a range of devices with scaling plateaus of 100%, 125%, 150% … The base unit is 4 because it scales to these plateaus as a whole number (for example, 4 × 125% = 5, 4 × 150% = 6). Using multiples of four aligns all UI elements with whole pixels and ensures crisp, sharp edges. (Text does not have this requirement; text can have any size and position.)" (Microsoft, *Screen sizes and breakpoints*).

Measured on this crate's gallery before the change (`/tmp/g5/findings.md`, device px, frame / gap / fill / gap / frame down the meter's middle): 1920x1080 (1.5x) `4 / 5 / 19 / 4 / 4`; 1280x800 (1.11x) `3 / 3 / 14 / 4 / 3`; 1600x900 (1.25x) `3 / 4 / 16 / 4 / 3`, its gap heavier than its frame though the skin draws both `border` wide; 1600x900 at text scale 1.5 `3 / 4 / 32 / 3 / 3`. The focus ring and the panel border measured even in these captures, but only because of where those widgets happened to land: the rounding below depends on a node's sub-pixel position, so any other layout can split them.

## How shipping games do it

Factorio rewrote its GUI so it would scale without deforming, and names the cause: "all the GUI values (sizes, widget positions, paddings etc.) are integer values, as in the end, every element needs to be on some specific pixel as long as we want it to be crispy clear. Imagine that you have a 1 pixel wide gap between two 20 pixel wide buttons and then you want it to show in 120%. The buttons are enlarged to 24 pixels, but the gap either stays 1 pixel or becomes 2, but the proportions of the layout changes." Its answer is a unit that scales to whole pixels: "1 module is 4 pixels in standard scale (100%), and almost everything is a multiplication of modules (sizes, positions, paddings etc). On top of that, we limited the possible scale values to be multiples of 25% (from 75% to 200%)."

The web, which runs at every scale factor a laptop has, snaps line weights at draw time instead of limiting the scale: "`<length>`s in border-width and a few other properties are rounded in a specific fashion to ensure reasonable visual display. … To snap a length as a border width given a `<length>` len: Assert: len is non-negative. If len is an integer number of device pixels, do nothing. If len is greater than zero, but less than 1 device pixel, round len up to 1 device pixel. If len is greater than 1 device pixel, round it down to the nearest integer number of device pixels." (CSS Values 4 §6; `border-width`'s computed value is "absolute length, snapped as a border width", CSS Backgrounds 3).

Why the engine splits a line: Bevy multiplies each `Val::Px` by the node's scale factor (`convert.rs`), and taffy then rounds every edge from the node's position on screen, `border.left = round(x + border.left) - round(x)` and `border.right = round(x + width) - round(x + width - border.right)` (`round_layout`), the same for padding. A 4.5 px border is 4 or 5 depending on the fraction of `x`; a whole-pixel border is the same on every side wherever the node lands, because adding an integer does not change the fraction that is rounded.

## Model adopted

- **L1. Every line weight is a whole number of device pixels, the same on every side.** `apply_skin` draws each border (a panel's, a button's and so its focus ring, a meter's frame, a tooltip's) and the meter's gap at `snap_line_width(border, scale)`: CSS's rule, rounded down above one device pixel, up to one below it, nothing at zero. `scale` is the node's own `ComputedUiRenderTargetInfo::scale_factor()` (camera × `UiScale`), so a window scale factor of 1.5 and a `UiScale` of 1.5 snap alike, and a UI on a second camera snaps to its own target.
- **L2. The fill takes the remainder.** Lines round down, never up, so the meter's frame and gap never grow past the skin's `border` and the fill gets the pixel back: 1920x1080 draws `4 / 4 / 20 / 4 / 4`, the fill centred. The fill's corner radius is `radius − 2 × line`, concentric with the frame it sits in (as before, from the snapped line).
- **L3. Snapped to what the engine multiplies.** Bevy computes `scale_factor * value` in `f32`; a logical length `n / scale` can come back as `n − 1 ulp`, which `round` turns into `n − 1` on a node that sits on a half pixel. `snap_line_width` steps the logical length one ulp at a time until the product is exactly `n`, and where no `f32` gives exactly `n`, takes the least length above it, never one below. At the default border the product is exact at every common window height (720, 768, 800, 900, 1050, 1080, 1200, 1440, 1600, 2160; `/tmp/g5`'s sweep).
- **L4. Restyled when the scale changes.** `apply_skin` also restyles a widget whose `ComputedUiRenderTargetInfo` changed. Bevy computes that in `PostUpdate`, after `apply_skin`, so the first frame at a new scale draws with the old snapping and the next frame is exact; a resize already costs a relayout, and one frame is not visible.
- **L5. Only line weights.** Gaps, padding, radii and sizes stay continuous: they are space, not lines, and a pixel either way in 15 px of space between buttons is not a second weight. Text is laid out at its final size ("Text does not have this requirement", Microsoft).

## Parameters

- `snap_line_width(length, scale)`: `device = floor(length × scale + 0.001)`, at least 1 when `length > 0`; returns `device / scale` (logical px), nudged by ulps so `scale × result == device` where `f32` allows. A non-finite or non-positive length or scale is returned unchanged (`GooeySkin::problems` reports a bad `border`).
- The 0.001 px tolerance: a length already within a thousandth of a device pixel of a whole number is that number (CSS's "do nothing"), not one less.
- Default `border` 3: 3 px at 1x and 1.11x (Steam Deck), 3 at 1.25x (from 3.75), 4 at 1.5x (4.5), 6 at 2x, 9 at 3x.

## Validation targets

- 1920x1080 (1.5x): the meter `4 / 4 / 20 / 4 / 4` down its middle and `4 / 4` frame and gap at both ends; the focus ring 4 px on all four sides; the panel border 4 px on all four sides; the tooltip border 4 px; the dialog's buttons' ring 4 px.
- 1280x800 (1.11x): every line 3 px, the meter `3 / 3 / 14–15 / 3 / 3`.
- 1600x900 (1.25x): every line 3 px; the meter's gap equal to its frame.
- 1280x720 (1x) and 2560x1440 (2x): pixel-identical to the baseline (the default border is already whole there).
- A layout test at 1.5x and 1.11x reads every panel's, button's and meter's `ComputedNode::border` (and the meter's `padding`) equal on all four sides and whole; after a resize from 1280x720 to 1920x1080 it holds two frames later.
- Frame time at 1280x720 within 0.5 ms of the baseline: a restyle per scale change, nothing per frame.

## What great looks like

At any window size and any scale a player picks, every frame in the interface is drawn with one pen: the focus ring is the same thickness on all four sides of the button it marks, the panel's edge is one line all the way round, and the meter's fill sits dead centre in its slot with an even gap all round, the way it does at 720p. Nothing is blurred or doubled at its edge, because every edge sits on a pixel boundary. A screenshot at 1080p, zoomed, shows the same construction as one at 720p, a little heavier.

## Deviations

- **Snapped at draw time, where Factorio limits the scale to 25 % steps and Microsoft to plateaus.** This crate's scale is continuous (window short side / 720, times the player's size), so a 1280x800 Steam Deck is 1.11x; no fixed set of scales makes a 3 px line whole at all of them (3 × 1.25 = 3.75). CSS's draw-time rule fixes every scale, the continuous one included.
- **Lines round down, so a line can be thinner than `border × scale` by up to one device pixel** (3.75 → 3 at 1.25x). The focus ring stays above WCAG 2.4.13's 2 CSS px perimeter at the default 3 px border at every scale of 1 or more: `floor(3s) / s ≥ 3 − 1/s ≥ 2`. A skin with `border` 2 can draw a 1.6 px ring at 1.25x; that is CSS's choice too, and `problems()` does not check ring thickness.
- **One frame at the old snapping after a scale change** (L4): `ComputedUiRenderTargetInfo` is Bevy's own per-node scale, the one its layout uses, and reading it keeps one path; computing the scale again in `apply_skin` from the camera and `UiScale` would be a second copy of Bevy's rule.
- **Not exact for every `f32` scale.** At the default border, 280 of the 3,601 window heights from 720 to 4320 px (7.8 %, none of the common ones in L3) give no `f32` length whose product with the scale is exactly a whole pixel, and the least length above it is used (one ulp over). A node whose right or bottom edge then lands on an exact half pixel draws that one line a pixel heavier. Before this change every fractional scale split lines this way, at any position.

## Pending index

- Index Microsoft's *Screen sizes and breakpoints*, Factorio FFF #277 and CSS Values 4 into home-still.
- Not in the corpus: a GDC talk or console studio write-up on pixel-snapping a scalable HUD (searched "pixel snapping crisp lines user interface scaling blurry borders at fractional display scale": rasterisation chapters only, no game UI source).
