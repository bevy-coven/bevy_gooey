# The HUD meter: one shape, its whole length legible, as large as the HUD

Researched 2026-10-30 (iteration 3, begun by a pass that was cut off; its draft was adopted and finished). home-still's distill was up this time (`system_status`: healthy, idle), so the craft source is a corpus stem; the shipping-game and technical sources are web pages, each quoted from a fetched copy.

## Sources

1. Stein C. Llanos and Kristine Jørgensen, *Do Players Prefer Integrated User Interfaces? A Qualitative Study of Game UI Design Issues*, DiGRA 2011 — home-still `10.26503_dl.v2011i1.514` (craft).
2. Celia Hodent, *The Gamer's Brain: How Neuroscience and UX can impact Design* (GDC 2015), written up at https://celiahodent.com/video-game-ux-psychology/ (craft).
3. Naughty Dog, *The Last of Us Part II: Accessibility Features Detailed* — https://www.naughtydog.com/blog/the_last_of_us_part_ii_accessibility_features_detailed (shipping game).
4. Material Components for Android, *Progress indicators* — https://github.com/material-components/material-components-android/blob/master/docs/components/ProgressIndicator.md (technical: a shipping design system's bar meter, and how it met the non-text contrast rule).
5. W3C WAI, *Understanding SC 1.4.11: Non-text Contrast* (WCAG 2.2) — https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html (technical).
6. W3C, *CSS Backgrounds and Borders Module Level 3*, §4.2 *Corner Shaping* and §4.3 *Corner Clipping* — https://www.w3.org/TR/css-backgrounds-3/ (technical: how a rounded box and its contents are meant to be drawn).
7. Bevy 0.20 source: `bevy_ui-0.20.1/src/layout/clipping.rs` (a child is clipped with `push_rect`, a rectangle) and `ui_node.rs` (`Node` requires `BorderColor`; `clamp_corner` clamps a radius to half the box) (technical: what the engine does).

## Craft principle

Llanos and Jørgensen, from their player interviews: "regardless of whether one is designing a minimal or a more extensive UI, the designer must always make sure to present gameplay relevant information in a sufficiently clear and precise manner." And: "A minimal UI that seems natural to the game environment is not the best way to present information that is critical, or that needs to be gauged on a continuous basis."

A health bar is exactly the information "gauged on a continuous basis", and a bar is gauged as a ratio: the fill's length against the track's whole length. A track that disappears into its panel leaves a fill whose length has nothing to be measured against; 60 of 100 reads as a short full bar.

Hodent, reading a HUD with the Gestalt laws: "Objects with similar attributes are perceived as belonging together." And: "Form Follows Function is really important in design. Ideally, you reach affordance = What you see is how it works." A meter is one object, a quantity inside a container. A fill whose square corners stick out of a rounded track is a second shape laid over the first, and the bar stops matching the rounded panels and buttons around it.

## How shipping games do it

*The Last of Us Part II* treats the HUD's size and its separation from what is behind it as player options: "HUD Scale · Options: Default, Large · Changes the size of in-game HUD elements." and "HUD Background · Options: Default, Light, Darkened · Changed HUD background darkness." Its vision preset turns on "High Contrast Display • HUD Scale > Large". The player's size option enlarges the HUD's elements, meters included, not only its text; and a HUD element is kept apart from its surroundings.

Material's bar meter met WCAG's non-text contrast rule by separating the parts of the bar that cannot contrast with each other: "In order to comply with the latest accessibility requirements, the `LinearProgressIndicator` and `CircularProgressIndicator` have been updated with additional attributes: `app:indicatorTrackGapSize`: size of the gap between the indicator and the track, 4dp by default. `app:trackStopIndicatorSize`: size of the stop at the end of the track". And it keeps the fill and the track one rounded shape: "To keep the track fully rounded, the `trackCornerRadius` should be set to `4dp`" for an 8 dp track.

## Model adopted

- **M1. The fill is drawn inside the track, with the track's corner shape.** CSS §4.3: "other effects that clip painting … must clip to their respective curves. For example … overflow values other than visible to the curved padding edge". Bevy 0.20 does not: `clipping.rs` clips a child to a rectangle, so a rounded track with `Overflow::clip()` still shows its fill's square corners. The fill therefore carries its own radius.
- **M2. The track's whole length is marked at 3:1 against the panel, by a frame.** WCAG 1.4.11 asks 3:1 for "Parts of graphics required to understand the content", and the track's length is required to read the value. No single track colour can sit 3:1 from both the panel and the fill in either preset (dark: the track would need a relative luminance ≥ 0.144 against the panel and ≤ 0.131 against the amber fill; parchment: ≤ 0.179 against the panel and ≥ 0.318 against the oxblood fill). So, as Material did, the parts that cannot contrast are kept apart: the track gets a frame `border` px wide in `text_muted` (the panel's line weight, and the same thickness as the focus ring), and the fill sits one `border` inside that frame, on the track's own fill (`surface_raised`), the gap Material calls `indicatorTrackGapSize`. The frame against the panel carries the extent; the fill against the track carries the value. Understanding 1.4.11 passes the same anatomy for a radio button: "an inner filled circle with a gap between it and the outer border."
- **M3. The corners are concentric.** CSS §4.2: "The padding edge (inner border) radius is the outer border radius minus the corresponding border thickness. In the case where this results in a negative value, the inner radius is zero." And: "Likewise the content edge radius is the padding edge radius minus the corresponding padding, or if that is negative, zero." The frame's inner radius is `radius − border` (Bevy draws that itself); the fill fills the content box, so its radius is that minus the gap, `max(radius − 2 × border, 0)`.
- **M4. The meter grows with the player's size option.** Its height is `GooeySkin::meter_height × text_scale`, as *The Last of Us Part II*'s HUD Scale enlarges HUD elements, so at 1.5× the bar keeps its proportion to the label above it. The frame and gap stay `border`, like every other line in the skin.
- **M5. A skin is checked for both.** `GooeySkin::problems` checks `accent` (the fill) against `surface_raised` (the track) and `text_muted` (the frame) against the panel, each at 3:1.

## Parameters

- `GooeySkin::meter_height`: 24 logical px at `text_scale` 1 (was 14, set in the widget). The fill is `24 − 4 × 3 = 12` px high, close to the old bar's 14 px. A design default.
- Frame and gap: `GooeySkin::border` (3 px) each. A design default resting on "one visual language": the panel border and the focus ring are 3 px.
- Radii: track 8 (`radius`), track interior 5 (Bevy's inner radius), fill `max(8 − 6, 0)` = 2.
- Least length: 8 text sizes, `8 × text_size × text_scale` (160 px at 1×, the old fixed least width; 240 px at 1.5×). A design default.
- Contrast, computed from the presets' colours: fill on track dark 6.04:1, parchment 4.68:1; frame (`text_muted`) on the panel dark 6.63:1, parchment 4.63:1.

## Validation targets

- At 4× zoom the fill's corners are rounded and concentric with the frame's; no fill pixel lies outside the track interior (before: the fill's left corners square over the track's rounded ones, `before/z-all.png`).
- The track's extent against the panel is ≥ 3:1 in pixels in both presets, at any value, including 0 (before: 1.39:1 dark, 1.25:1 parchment).
- The fill is `border` px from the frame on all four sides, at any value.
- At `text_scale` 1.5 the meter is 36 px high (before: 14 px, under a 30 px label).
- A meter near empty (fill narrower than twice its radius) draws a small rounded sliver inside the frame, not nothing.
- The fill's length stays proportional: `fill / interior width` = value / max within 1 px.
- `problems()` stays empty for both presets; an accent equal to `surface_raised` reports `accent on surface_raised`; a `text_muted` equal to the panel reports `text_muted on surface`.
- No frame-time cost: the same two nodes; a border, a radius and padding set once by `apply_skin`.

## What great looks like

A health bar a player reads from the couch at a glance: a framed slot with the panel's line weight, the amber (or oxblood) fill sitting in it like liquid in a tube, its rounded end moving smoothly as the value drains, and the empty part of the slot plainly empty rather than missing. The same corner family as the panel and the buttons around it; nothing pokes out. At 4% it is a small sliver in a clearly long, clearly empty frame. At large text the bar thickens with the label so the HUD still looks designed, not shrunk.

## Deviations

- Bevy's rectangular clip is the deviation from CSS §4.3; the crate rounds the fill itself (M1) instead of relying on the track's clip.
- The meter's height moves from the widget (`height: 14px`) into the skin, per the crate's rule that only `apply_skin` sizes a widget. Default 24 px (was 14) to make room for the frame and the gap; a design default.
- Material marks the track's end with a 4 dp stop dot; this crate frames the whole track instead. A frame marks both ends, keeps the skin's one line weight, and a 3 px line survives couch distance where a 4 px dot is easy to lose; it is also how framed game health bars are drawn. A design choice, recorded.
- `text_muted` gains a second job (a disabled button's text and a meter's frame) rather than adding a colour to the skin. One fewer field for a project to set; `problems()` checks it for the frame's job.

## Pending index

- Index Hodent's GDC 2015 write-up, Naughty Dog's accessibility post and Material's progress indicator page into home-still.
- Searched and not found in the corpus: a shipped game's write-up on health-bar framing ("health bar HUD meter design readability", "progress bar track and fill contrast"). A GDC talk on HUD bar design would replace Material as the shipping-game source for M2.
