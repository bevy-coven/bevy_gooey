# Popovers open from their anchor: scale and fade, on the slide's own spring

Researched 2026-11-08 (iteration 68). Issue #71 (G-64, `grimoire-ui` and `feel`, `consumer:bevy_grimoire`): a popover, a menu, a submenu, a context menu, a select list and the colour picker open with a 6 px slide and nothing else. The issue asks for the reference's entrance: a scale from 0.96 and a fade, the scale's origin on the side that faces the anchor, and a fade out. The exit half is already built (iteration 66, `popover::close` puts `Leave` on the panel); this item is the entrance.

## Sources

1. Rune, https://github.com/anomalyco/rune at `a7f276c` (MIT, open reference; the pin in the issue is `f73f72c`), `crates/rune-kit/src/overlay.rs:44-70` (`anchor_origin`, `anchor_at`, `anchor_with`) and `crates/rune-kit/src/kit.rs:222-224` (the overlay spring) (shipping toolkit, read freely, no code taken).
2. Nielsen Norman Group, Page Laubheimer, *Executing UX Animations: Duration and Motion Characteristics*, already quoted in `docs/research/motion.md` (craft: duration and easing); and the W3C's *Understanding SC 2.3.3 Animation from Interactions* (accessibility: the entrance turns off with `MotionSettings`).
3. Bevy 0.20.1 `bevy_ui_widgets/src/popover.rs`, `position_popover` (technical: what owns a popover's transform), and this crate's `docs/research/presence.md` and `motion.md` (M4b).

## Craft principle

NN/g, quoted in `motion.md`: "the duration of most animations should be in the range of 100–500 ms, depending on complexity and on how far the element is traveling. As a rule of thumb, look for the shortest time that an animation can take without being jarring." A menu is opened many times a session, so the entrance is small (a 4% scale, a 6 px slide) and as short as the skin's `snappy` spring (0.25 s), and a player who asks for less motion gets a fade in place (SC 2.3.3: "Motion animation triggered by interaction can be disabled").

## How shipping games do it

Rune's `anchor_at` is documented as giving the content "the overlay entrance: fade plus 0.96 scale from the trigger side": `Transition::scale_anchor(0.96, anchor_origin(side, align))`, where `anchor_origin` puts the origin on the panel's edge that faces the anchor (`Side::Bottom` is `(along, 0.0)`, the top edge, with `along` 0, 0.5 or 1 for start, centre and end alignment). Its overlay spring is documented "Overlays opening and closing (popover, menu, tooltip): ~120–150 ms". A scale is held to [0.9, 1.1] by default (`CHANGELOG.md`: "Transitions keep scale within [0.9, 1.1] by default"): 0.96 is a small breath, not a zoom.

## Technical: what Bevy leaves us

`position_popover` measures where the panel's centre is (`ui_global_transform.translation`) and writes into `UiTransform::translation` the offset that puts that centre at its best candidate, every frame: any translation this crate adds is cancelled (`motion.md` M4b found it the hard way). It never writes `UiTransform::scale`, and a node scales about its own centre, which `position_popover` does not move. So:

- The scale can be written straight into `UiTransform::scale`; only its origin cannot be moved to the anchor's side.
- The origin that does not move is the centre, which makes the near edge step away from the anchor by `(1 - scale) * height / 2` (about 2% of the panel's height: 4 px on a 200 px menu). The 6 px slide already moves the whole panel that much closer to the anchor, so the two overlap: the near edge starts within a few px of its place and the far edge travels, which is what an origin on the near edge looks like.

## Model adopted

- **P1. The entrance is the existing slide plus a scale and a fade, on one spring.** [`PopoverSlide`] (the gap reach, 6 px to 0 on `snappy`) is the one clock. While it runs, `update_motion` writes `UiTransform::scale = 1 - 0.04 * reach / 6` (0.96 at the start, 1 at the end) and the panel carries an `Arriving` fade (`fade_seconds`, 0.15 s) so its fill, frame, words and shadow fade in together. The scale lands on exactly 1 when the slide is spent and is written only when it differs.
- **P2. The slide stays (assumed decision, #71).** The issue asks for a clean cutover of `PopoverSlide` and `POPOVER_SLIDE_PX`; `tests/motion.rs` and `tests/reduced_motion.rs` assert them, and a pass may not re-pin an assertion. The scale and fade are added beside it (the option the earlier pass on this issue recommended); the slide is the near edge's stand-in for the origin Bevy will not let us move (above), so it earns its place.
- **P3. Reduced motion fades in place, as it did.** Under `MotionSettings::reduced` there is no `PopoverSlide` and no scale; the panel gets `Arriving` only. Under `off()` nothing starts.
- **P4. Exit unchanged.** `popover::close` puts `Leave` on the panel: it fades out and despawns (iteration 66), and the pointer and focus are released in the frame it starts. A `Leave` in the middle of an entrance starts the fade from where the arrival had got to (`Arriving` already does this).
- **P5. A panel that carries its own `Opacity` is the host's.** As for a dialog's card, the fade is not added.

## Parameters

|name|value|why|
|---|---|---|
|`POPOVER_SCALE_FROM`|0.96|Rune's `scale_anchor(0.96, ..)`; inside its [0.9, 1.1] floor|
|slide|`POPOVER_SLIDE_PX` 6 px on `snappy`|unchanged|
|fade|`fade_seconds` 0.15 s, ease-in-out (`Arriving`)|the crate's one fade|

## Validation targets

- Opening a menu on a manual clock: the first drawn frame is at scale 0.96 (±0.01) and opacity near 0; by `motion.snappy` (±1 frame) the scale is 1 and no `Arriving`, `Opacity` or `PopoverSlide` is left.
- Under `MotionSettings::reduced` the panel's scale is 1 throughout and it fades; under `off()` it is drawn whole at 1 in the first frame.
- Escape on an open menu fades it out and despawns it (already tested; re-run).
- A settled interface writes nothing.

## What great looks like

A menu that breathes open from its button: a few frames of a slightly smaller, slightly paler card settling into place as its first row lights, never a pop and never a slide long enough to cross the button it came from.

## Deviations

- **The slide is kept** (P2), against the issue's "clean cutover": the protected assertions pin it. Recorded on #71.
- **The scale's origin is the panel's centre, not the anchor's edge** (Bevy's `position_popover` owns the translation); the 6 px slide stands in for the shift. A panel taller than about 300 px has a near edge that moves a pixel or two further than Rune's.
- **The fade is 0.15 s, not the scale's 0.25 s**: the crate's one fade length.

## Pending index

- The corpus has no paper on overlay entrance transitions; NN/g and WCAG are cited from the web, Rune from its source.
