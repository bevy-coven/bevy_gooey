# The HUD meter: one shape, inside its track, as large as the HUD

Researched 2026-10-30 (iteration 3). home-still's distill answered `500: gpu busy` (`4869 MB free < 5000 MB required to load bge-m3`, beside a 14.6 GB vLLM engine), so semantic search was unavailable; this note rests on the web sources below and the vendored Bevy 0.20 source.

## Sources

1. Celia Hodent, *The Gamer's Brain: How Neuroscience and UX can impact Design* (GDC 2015), written up at https://celiahodent.com/video-game-ux-psychology/ (craft).
2. Naughty Dog, *The Last of Us Part II: Accessibility Features Detailed* — https://www.naughtydog.com/blog/the_last_of_us_part_ii_accessibility_features_detailed (shipping game).
3. W3C, *CSS Backgrounds and Borders Module Level 3*, §4.2 *Corner Shaping* and §4.3 *Corner Clipping* — https://www.w3.org/TR/css-backgrounds-3/ (technical: how a rounded box and its contents are meant to be drawn).
4. Material Components for Android, *Progress indicators* — https://github.com/material-components/material-components-android/blob/master/docs/components/ProgressIndicator.md (technical: a shipping design system's bar meter).
5. Bevy 0.20 source: `bevy_ui-0.20.1/src/layout/clipping.rs` (`push_rect(computed_node.resolve_clip_rect(..))`) and `ui_node.rs` (`clamp_corner`) (technical: what the engine does).
6. W3C, *WCAG 2.2*, SC 1.4.11 *Non-text Contrast* (3:1 for "graphical objects required to understand the content").

## Craft principle

Hodent (GDC 2015), on reading a HUD with the Gestalt laws: "Objects with similar attributes are perceived as belonging together." And: "Form Follows Function is really important in design. Ideally, you reach affordance = What you see is how it works."

A meter is one object, a quantity inside a container. A fill whose square corners stick out of a rounded track is a second shape laid over the first: the eye reads two objects, and the bar stops matching the rounded panels and buttons around it.

## How shipping games do it

*The Last of Us Part II*, in its vision settings: "HUD Scale · Options: Default, Large · Changes the size of in-game HUD elements." Its Vision Accessibility Preset turns on, among others, "High Contrast Display • HUD Scale > Large". The player's size option enlarges the HUD's elements, meters included, not only its text.

Material's bar meter keeps the fill and the track one shape: "To keep the track fully rounded, the `trackCornerRadius` should be set to `4dp`" for an 8 dp track, with a `50%` corner radius by default, so the rounded end is drawn on the indicator as well as the track.

## Model adopted

- **M1. The fill is drawn inside the track, with the track's corner shape.** CSS §4.3: "other effects that clip painting … must clip to their respective curves. For example … overflow values other than visible to the curved padding edge". Bevy 0.20 does not: `clipping.rs` clips a child to a rectangle (`push_rect`), so a rounded track with `Overflow::clip()` still shows its fill's square corners. The fill therefore carries its own radius.
- **M2. The fill sits one `border` inside the track**, a groove, like the panel's own border width, so the track frames the fill on every side and a full meter still reads as a meter, not a solid bar. Its radius is the track's minus that inset, CSS §4.2: "The padding edge (inner border) radius is the outer border radius minus the corresponding border thickness. In the case where this results in a negative value, the inner radius is zero." The corners are concentric.
- **M3. The meter grows with the player's size option.** Its height is `GooeySkin::meter_height × text_scale`, as *The Last of Us Part II*'s HUD Scale enlarges HUD elements, so at 1.5× the bar keeps its proportion to the label above it.
- **M4. The fill is legible against its track.** `GooeySkin::problems` checks `accent` (the fill) against `surface_raised` (the track) at WCAG 1.4.11's 3:1.

## Parameters

- `GooeySkin::meter_height`: 18 logical px at `text_scale` 1 (was 14, set in the widget). The fill is `18 − 2 × 3 = 12` px high; a 14 px track with a 3 px groove would leave an 8 px sliver. A design default.
- Inset: `GooeySkin::border` (3 px). A design default resting on "one visual language": the panel border width.
- Fill radius: `max(radius − border, 0)` = 5 px; the track keeps `radius` (8 px, which Bevy clamps to half the height when the track is thinner than 16 px).
- Fill against track (1.4.11): dark 6.04:1, parchment 4.68:1.

## Validation targets

- At 4× zoom the fill's four corners are rounded and concentric with the track's; no fill pixel lies outside the track (before: the fill's left corners square over the track's rounded ones, `/tmp/g3/before/z-meter-dark.png`).
- The track shows a `border`-px margin around the fill on all four sides, at any value.
- At `text_scale` 1.5 the meter is 27 px high (before: 14 px, under a 30 px label).
- A meter near empty (fill narrower than twice its radius) draws a small rounded lozenge, not a square sliver and not nothing.
- `problems()` stays empty for both presets; an accent equal to `surface_raised` reports `accent on surface_raised`.
- No frame-time cost: the same nodes, a radius and padding set once by `apply_skin`.

## What great looks like

A health bar a player reads from the couch at a glance: a rounded slot in the panel's colour language, the amber (or oxblood) fill sitting in it like liquid in a tube, its rounded end moving smoothly as the value drains. The same corner radius family as the panel and the buttons around it; nothing pokes out. At large text the bar thickens with the label so the HUD still looks designed, not shrunk.

## Deviations

- Bevy's rectangular clip is the deviation from CSS §4.3; the crate works around it by rounding the fill itself (M1) instead of relying on the track's clip.
- The meter's height moves from the widget (`height: 14px`) into the skin, per the crate's rule that only `apply_skin` sizes a widget. Default 18 px (was 14) to make room for the groove; a design default.

## Pending index

- distill was unavailable (`gpu busy`). Search the corpus for "health bar", "HUD meter readability", "resource bar design" and "HUD scale option".
- Index Hodent's GDC 2015 write-up and Naughty Dog's accessibility post into home-still.
