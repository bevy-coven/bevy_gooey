# An LED: `led`

Researched 2026-11-08 (iteration 71) for issue #80 (G-69), asked by bevy_grimoire#343: "a glowing indicator virtual LED stating whether something is live. Make it look amazing." home-still's corpus has no source on status indicators (the CLI search for "status indicator redundant coding" returned unrelated papers); the sources below were read directly and are listed under Pending index.

## Sources

1. W3C, *Understanding SC 1.4.1 Use of Color* (WCAG 2.2, Level A) (craft: accessibility).
2. W3C, *Understanding SC 2.3.3 Animation from Interactions* (WCAG 2.2) and, for the pulse, *SC 2.2.2 Pause, Stop, Hide* (technical).
3. Hardware practice, which the widget borrows its look from: a front-panel pilot lamp is a lit lens with a glow around it and an unlit lens is a dark ring (shipping reference; studio consoles and the Rune desktop's own activity dots, `crates/rune-kit/src/` pulse tokens: `rune-motion/src/tokens.rs` `DRIFT`, "Slow reveals ... the eye should follow").
4. This crate: `warning()` in `src/button_variant.rs` (a colour derived from `danger`'s lightness and chroma at another hue), `Cooldown`'s data fill (`src/cooldown.rs`), `ShadowLayer` (`src/elevation.rs`).

## Craft principle

WCAG 2.2 SC 1.4.1: "Color is not used as the only visual means of conveying information, indicating an action, prompting a response, or distinguishing a visual element." So each state has its own silhouette and its word beside the dot, and colour only adds to them.

SC 2.3.3 "Motion animation triggered by interaction can be disabled, unless the animation is essential to the functionality or the information being conveyed": a pulse that says "busy" is information, but the silhouette and the word already say it, so under reduced motion the pulse stops and the glow is steady.

## How shipping games do it

A status lamp is lit, dark or blinking; a failed one is red and does not blink. A pulse says "working", a steady light says "stable" (Rune's drift token for slow reveals the eye should follow: 0.85 s). This widget pulses at 1.2 s, slower than 1 Hz would flash, far below WCAG's three flashes a second.

## Model adopted

- `led(label, state)`, a row: `Role::Led` root with `Led(LedState)` (the state, public, the game writes it), a `Role::LedDot` (a circle-sized node, layout from the skin: 0.9 body text sizes) and two `Role::Label` texts: the label and the state word (`LedWord`, `LedState::word()`: "off", "live", "busy", "failed").
- Silhouettes: **Off** a hollow ring in `outline`, **Live** a filled disc, **Busy** a filled rounded square, **Failed** a filled diamond (a rounded square turned 45 degrees and drawn at 0.78 so its diagonal is the dot's width).
- Colours, all derived, no new skin field and no preset change: Live is `GooeySkin::live()` (the danger colour's lightness and chroma at a green hue, gamut-pulled like `warning()`), Busy is `warning()`, Failed is `danger`. Each holds 3:1 on the opaque panel in every preset (tested).
- Glow: a `BoxShadow` of the LED's colour, blur 1.2 dot sizes, spread 0.15, alpha 0.55 (lit states). Busy's alpha pulses 0.2 to 0.7 on a 1.2 s sine of `Time<Real>`. Off has none.
- The fill, border, radius, rotation, shadow and word are **data written by `update_leds`** (like the cooldown's sweep), each only on a difference, so a settled LED writes nothing. `apply_skin` writes layout only.
- `MotionSettings::lands_travel()` (reduced or off): the glow is steady at its middle alpha.

## What great looks like

A lit lens: a saturated disc with a soft halo bleeding onto the panel, not a flat dot; the four states unmistakable at a glance in greyscale; the pulse slow and calm, never a blink.

## Parameters

| Name | Value | Where from |
|---|---|---|
| `LED_TEXT_SIZES` | 0.9 | design default |
| glow blur / spread / alpha | 1.2 dots / 0.15 dots / 0.55 | design default |
| pulse | 1.2 s sine, alpha 0.2 to 0.7 | design default; under 1 Hz |
| live hue | 145 degrees (Oklch) | design default |

## Validation targets

- Four states: four different (radius, rotation, fill) triples.
- `live()`, `warning()`, `danger` and `outline` hold 3:1 on `solid_surface()` in every preset.
- Busy's glow alpha differs between two frames half a period apart under default motion, and is equal under reduced.
- A settled LED writes nothing in 30 frames; a state change shows in the next frame (dot and word).

## Deviations

- No new skin colour for "live" (design default): a derived accessor keeps every preset and theme step as is.
- What "live" means is the host's (the issue's open question): the widget draws state, it reads nothing.

## Pending index

- WCAG 1.4.1, 2.2.2 and 2.3.3 Understanding pages (W3C), read directly, not in the corpus.
