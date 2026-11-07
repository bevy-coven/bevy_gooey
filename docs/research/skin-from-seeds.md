# A skin from three seeds

Researched 2026-11-07 for issue #54 (G-47), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. Its derivation was measured from source and written again on this crate's `GooeySkin`. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune-kit/src/palette.rs`: `Kit::from_palette` and `from_palette_focus`.
   - Layers step Oklab L away from `bg`: `DARK_LAYERS = [0.04, 0.07, 0.10, 0.13]` and `LIGHT_LAYERS = [0.02, 0.045, 0.065, 0.10]`.
   - A dark kit steps from at least `DARK_FLOOR = 0.145`, about `#0a0a0a`.
   - The derived neutrals keep the hue and chroma of the more colourful of `bg` and `fg`.
   - Muted text is `secondary(bg, fg, tint, 0.72 dark / 0.57 light, 4.5)`: a lightness mix toward `fg`, bisected up to a 4.5:1 floor, and `fg` itself when even that cannot reach it.
   - `border_base` is `fg` at 0.11 or 0.10 alpha. Hover is 0.06 or 0.05, pressed 0.10 or 0.08.
   - The focus ring is `oklch(0.70 dark / 0.58 light, 0.15, 264)`, a neutral blue.
2. Rune, `examples/kit-gallery/src/main.rs`, `kit()`: the gallery's seeds.
   - `--kit palette` dark: `#0a0a0a`, `#ededed`, `#e5484d`.
   - `--kit palette` light: `#ffffff`, `#171717`, `#e5484d`.
   - The `black` variant: `#000000`, `#ffffff`, `#ff2a2a`.
   - The `warm` variant: `#e8e4dc`, `#000000`, `#d81414`.

## Model adopted

`GooeySkin::from_seed(PaletteSeed { background, text, accent }) -> Result<GooeySkin, Vec<SkinProblem>>`:

- **Ladder.** `surfaces` is five steps: the base (the background's Oklab L, at least 0.145 on a dark seed, at the neutrals' hue), then Rune's four offsets. A step never starts nearer the background than the one below, and one still under `ELEVATION_STEP` (2.0 CIE L\*) past it, signed, is pushed out 0.002 Oklab L at a time until it is not. A saturated hue clamped at the edge of sRGB can still fall short, which is refused as `SkinProblem::ElevationStep`. `problems()` compares only the size of a step, so `from_seed` checks its direction itself. Rune's 0.02 first light step is about 2 L\* and sits on the threshold. `surface` is step 1 and `surface_raised` step 3, as in `GooeySkin::tool()`.
- **Muted text.** Rune's mix, bisected to 4.5:1 on the panel (`surface`), which is the fill `problems()` checks it on.
- **Outline.** The text at alpha over the panel, starting at Rune's border alpha and raised until it reaches 3:1. Rune's 0.11 is a hairline; gooey's outline frames a meter and is held to 3:1.
- **Hover and press.** The text at Rune's alphas over the resting button (`surface_raised`) rather than over the background. Gooey's hovered and pressed fills replace a button's resting fill, so that is what they are composited over. The press alpha is raised until it is `STATE_STEP` (4 L\*) from the hover, which `problems()` checks.
- **Accent words.** Black or white, whichever has the better worst contrast over the accent and the two fills a primary button lifts it to (`primary_hovered`, `primary_pressed`).
- **Focus ring.** Rune's neutral blue, moved away from the ladder (lighter on a dark seed, darker on a light one) until it is 3:1 on the background and on every step. Rune's fixed 0.58 lightness is 2.8:1 on the warm paper's resting button. When no lightness reaches every ground, each one it misses is reported.
- **Danger.** The tool preset's on a dark seed. On a light one it is parchment's, because the tool preset's danger is 2.5:1 on the warm paper (3.2:1 on white) and `problems()` asks 3:1.
- **Everything else** comes from `GooeySkin::tool()`: sizes, density, type, spacing and motion. The scrim is black at 0.55.
- **Refusal.** A seed that is not opaque is `SkinProblem::TranslucentSeed`. A derived skin with any problem is refused with every problem it has; it is never adjusted to pass.

## Where this differs from Rune

- **Rune's light gallery seed is refused.** `#ffffff`, `#171717`, `#e5484d`: white words on the accent are 3.91:1, and black words, which read 5.37:1 on the accent itself, fall to 3.48:1 on the pressed fill this crate lifts toward the dark text. This crate holds an accent's words to 4.5:1 on all three fills, so the seed is refused. The gallery's dark, black and warm seeds pass.

- Rune's derived borders, hover and press are alpha colours solved over the background at draw time (`blend_over`). Gooey's skin holds opaque fills, so they are composited once, over the fill they are drawn on.
- Rune also derives status tints (success, warning, info), submit sheens and elevation rings. Gooey's skin has `danger` and no tint roles, and elevation is G-48's.
