# Colour motion

Researched 2026-11-07 for issue #51 (G-44), asked by bevy_grimoire's feel series (dark-incorporated/bevy_dark_factory#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune `crates/rune/README.md`, "Animation": colours spring in premultiplied Oklab, text colour runs at 0.5x its background's duration, a theme switch uses calm.
2. Björn Ottosson, "A perceptual color space for image processing" (2020), https://bottosson.github.io/posts/oklab/: Oklab is the space the mix is made in. Bevy's `Oklaba` is its implementation.
3. WCAG 2.2 SC 1.4.3 (text contrast 4.5:1) and SC 2.3.3 (animation from interactions).
4. bevy_gooey#76: real-time test apps run with `MotionSettings::off()`.

## Model adopted

- **One track per styled node.** `Role` requires `ColorTrack`. `update_color_motion` (Update, after `GooeySystems::Style`) reads what `apply_skin` and everything else wrote to the node's `BackgroundColor`, `BorderColor` (four sides), `TextColor` and `ImageNode::color`. When one differs from what the track last showed, that value is the new target and the colour moves from where it was drawn on a spring. `apply_skin` is not touched: it stays one path that writes the final colour, and the motion is a second system that leads the drawn colour to it. This is why a settled interface still writes nothing: a lane that is not moving and whose component is unchanged is not read, and a colour is written only when it differs.
- **Premultiplied Oklab.** Each lane springs `[L*a, a*a, b*a, a]` (the L, a and b of Oklab times alpha, and alpha), so a colour that fades in from transparent does not pass through a black or grey of its own.
- **Which spring.**
  - **State change** (hover, focus, selection, checked, disabled): `motion.snappy`.
  - **Skin change** (the `GooeySkin` resource or a `SkinScope` changed this frame): `motion.calm`.
  - **Label and icon tint:** half the length of the fill's spring, so a label leads its ground.
  - **Reduced motion:** a fade over `motion.fade_seconds` (a colour is not travel).
  - **Off:** lands.
- **Lands at once:**
  - **A pressed or dragged widget** (the node or an ancestor up to three levels, so a toggle's or slider's parts follow; a slider or scrollbar thumb drag counts, as `apply_skin` reads it as pressed): a press must read as one.
  - **A rebound table row:** the table's rows are a few entities reused as it scrolls, so a stripe or a selection that moves to another entity must not fade between them (`layout_motion` rules the same). A change of `TableRow` lands its colours, and its cursor moving too.
  - **Motion switched off while a colour is on its way:** it lands in that frame.
  - **The first two frames** a node has colours (its skin is written in the first, and a widget a host spawns late in a frame is styled in the next).
  - **A node that is not drawn.**
  - **The colours that are data:** `Role::Tone`, `ColorSwatchHalf`, `ColorArea` and `ColorRail`.
  - **A change of alpha alone:** a fade, which presence and toasts already run. With nothing moving it lands; with a colour on its way it goes on to the same target at the new alpha, from where it is drawn, so a fade never throws a hover half-eased to its end.
  - **Its release eases:** the colours leave the pressed one on the snappy spring.

## What differs from the issue

- **"Contrast stays at or above the lower of the two presets' values"** cannot hold for a switch between a dark preset and a light one. A label that goes from light to dark on a fill that goes from dark to light must cross it: by the intermediate value theorem, some frame has the label as bright as the fill, whatever the path. What is possible, and tested, is:
  - **A switch of one polarity** (dark to high-contrast dark): contrast stays at or above the lower of the two on every frame.
  - **A switch of opposite polarity:** the label's fast spring keeps the frames under WCAG's 4.5:1 to a third of the calm spring or fewer (the label leads its fill by half the time, and clears it first).
- **Hover "no colour step larger than an uninterrupted change's largest"** is tested by tracing a hover in and out and the same turned round at 3 frames, comparing the largest per-frame step.
- **The gallery and widgets that write colours every frame** (a meter's trail, a toast's alpha) are unchanged: their writes are a fade or data, and land.

## Proof

`tests/color_motion.rs`, with motion on at 60 Hz: hover in and out with the turned-round comparison and 30 idle frames writing nothing; label leads its fill; a skin change arrives within the calm spring's settle time plus a frame; same-polarity switch keeps contrast; a press lands and its release eases; the first frame and a tone land; off and reduced.

Also: switching motion off mid-change lands, and a fade over a colour on its way keeps going to the new colour. The skin-change test compares every colour of every styled node (fill, four border sides, text) with the same screen built in the new skin with no motion.

The colour-sensitive files of the suite, which run on the real clock, run with `MotionSettings::off()` (one line in each `app()`, #76): a hover there is read in the frame it is made.
