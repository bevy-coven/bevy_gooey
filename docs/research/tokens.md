# Design tokens: spacing, corners, type and a measured surface ladder

Researched 2026-11-01 (iteration 24). Issue #7 (G-03, `grimoire-ui`, `consumer:bevy_grimoire`) is the pass's item. A tool's panels sit on a ladder of surfaces (the window's ground, a panel, a row, a field, a card above the scrim); today every host measures and hard-codes its own, and the editor that asked found its own ladder's first step was below the eye's threshold. The same need is a game's: a photo mode, an inventory over a map, a pause screen over a HUD all stack fills.

## Sources

1. Wagemans et al., *A century of Gestalt psychology in visual perception: I. Perceptual grouping and figure-ground organization*, Psychological Bulletin 2012, doi:10.1037/a0029333, home-still `10.1037_a0029333` (craft: a surface reads as a figure on its ground only when the two differ; elements that look alike group).
2. Google, *Material Design 3*: "Theming in Compose with Material 3", Android Developers, https://developer.android.com/codelabs/jetpack-compose-theming, and *Introducing Tone-based Surfaces in Material 3*, https://m3.material.io/blog/tone-based-surface-color-m3 (shipping: a shipped design system's surface ladder); and bevy_grimoire's `chrome.rs` (a shipped editor, the ladder this request is about).
3. Marschner and Shirley, *Fundamentals of Computer Graphics*, 5th ed., ch. 21 and 22, home-still `book_fundamentals_computer_graphics_marschner` (technical: CIELAB, the formula and why lightness is the quantity to compare); the scikit-learn API paper (`W1728842521`: one way to do a thing, defaults that work).

## Craft principle

Wagemans et al. (the same stem the scope note rests on): "all else being equal, the most similar elements ... tend to be grouped together". Two fills that differ by less than the eye can tell apart are one fill: a panel that "sits on" a ground it cannot be told from has no edge, and a row on that panel has none either. The cure is not a frame on everything (the high-contrast preset's cost) but a ladder of fills whose every adjacent step is visibly a step. Marschner: "The vision system has very little ability, however, to detect spatial discontinuities in color when not accompanied by differences in one of these other properties", so the step is a step in **lightness**, not hue.

## How shipping games do it

Material 3 moved from shadows to a ladder of tones: "Material 3 represents elevation mainly using tonal color overlays. This is a new way to differentiate containers and surfaces from each other" (Android Developers), and its "surface container" roles are five named fills, "Surface Container Lowest" to "Highest" (m3.material.io). Five steps is the shipped number, and this crate's default. bevy_grimoire's own editor measured what goes wrong without a check, in `crates/bevy_grimoire_mapper/src/chrome.rs`: "WCAG's contrast ratio is the wrong instrument down here ... Measured on the shipped ladder, `VOID`→`PANEL_BG` was **ΔL\\* 1.60** — below the JND", and "a just-noticeable difference is about 2 and a comfortable one about 4". Its fix, and test (`the_ladder_is_a_ladder`), is the model here: every adjacent pair at least ΔL* 2.

## Model adopted

- **T1. `GooeySkin::surfaces: Vec<Color>`**, lowest first, opaque (alpha is ignored), serde data. A skin with one surface is valid; an empty one is a `SkinProblem::BadScale`. `GooeySkin::surface_at(level)` reads it, clamped to the top step.
- **T2. `Role::Surface(u8)`** and the widget `surface(level)`: a column filled with step `level` of the ladder and nothing else (no padding, frame or radius: those are the host's, read from the tokens below). A host's own node takes the role the same way as every widget.
- **T3. `SkinProblem::ElevationStep { lower, upper, delta_l }`** when two adjacent surfaces differ by less than `model::ELEVATION_STEP`, ΔL* 2.0, with `model::cie_lightness` (CIE 1976 L* of an sRGB colour: `116 Y^(1/3) − 16`, and the linear segment below `Y = 216/24389`). Direction does not matter: a recessed well (grimoire's pressed row) is a valid step down. A NaN colour is reported, not passed.
- **T4. Three scales as data**, each a fixed-length array so a token has one meaning everywhere: `spacing: [f32; 6]`, `radii: [f32; 3]` (the issue's `radius` is the existing single `radius` field; the scale is `radii`), `type_scale: [f32; 5]`, with `space(step)`, `corner(step)` and `type_size(step)` (which multiplies by `text_scale`, like every text size). A step past the end is the last. `problems()` reports `BadScale { field }` for a non-finite or negative entry, a text size that is not above zero, or a scale that falls.
- **T5. The presets sit on their own scales.** Each preset's `gap`, `padding`, `radius`, `text_size` and `heading_size` are entries of its `spacing`, `radii` and `type_scale`, so a host that builds on the tokens and a widget the crate draws share one rhythm (a test holds it).
- **T6. A theme step carries the ladder** (`with_colours_of`, `SkinPreset::of`): it is a palette, so a player who steps Dark to Parchment does not keep Dark's greys under cream text.

## Parameters

| value | Dark / Parchment / High contrast | Tool |
| --- | --- | --- |
| `spacing` | 4, 8, 10, 14, 20, 28 | 2, 4, 6, 10, 16, 24 |
| `radii` | 4, 8, 16 | 2, 4, 8 |
| `type_scale` | 14, 16, 20, 26, 32 | 11, 12, 14, 16, 20 |
| `surfaces` L* (lowest first) | Dark 6.5, 12.9, 18.3, 23.6, 31.6; Parchment 14.0, 63.0, 77.5, 87.1, 93.5; High contrast 0, 8.0, 16.0, 26.0, 38.0 | 5.9, 10.9, 16.3, 21.3, 29.4 |

Dark's and Tool's first three steps are the ground, the panel and (the middle of) the raised fill the widgets already draw, so `Surface(1)` is a panel's colour and `Surface(3)` a button's; Parchment's are the ground, a mid tan, a button's and a panel's, then a lifted cream. Adjacent steps are at least 5 ΔL*, the comfortable step grimoire measured, in every preset.

## Validation targets

- A ladder of two surfaces ΔL* 1.6 apart reports `ElevationStep { lower: 0, upper: 1 }` with that delta; at 2.0 or more it does not; a single-surface ladder is valid; an empty one reports `BadScale`; a NaN surface is reported (`tests/tokens.rs`).
- `cie_lightness` of white is 100, of black 0, of mid grey (0.5) about 53.4, monotone in luminance (`model`, no app).
- Every preset's `problems()` and `high_contrast_problems()` (for High contrast) are empty and every adjacent step is at least ΔL* 4.
- A `Surface(n)` node's `BackgroundColor` is the ladder's `n`, follows a changed skin and a `SkinScope` in the same frame, and a level past the top is the top.
- Every preset's own `gap`, `padding`, `radius`, `text_size` and `heading_size` is on its scales.
- A skin in RON with none of the new fields reads as the dark ladder (serde default), and one with its own ladder round-trips.
- The gallery stills are pixel-identical to before (no existing widget reads a token), and frame time is within 0.5 ms.

## What great looks like

`examples/ladder.rs`: five nested cards, each `Surface(n)` inside `Surface(n-1)` on the spacing tokens, each labelled with its step and its L*. Every card is plainly a step above the one it sits in, in dark, parchment and high contrast; the labels stay at 4.5:1 on their own fill; the corners are the skin's. A designer reads the ladder at a glance and sees no two cards merge.

## Deviations

- The issue's `radius: [f32; 3]` cannot sit beside the existing `radius: f32` (the corner every widget draws), so the scale is `radii` (assumed decision, recorded on the issue).
- The issue's `surfaces: Vec<Srgba>` is `Vec<Color>` here, like every other colour of the skin; `with_colours_of` and the serde form are the same as for them (assumed decision).
- The ladder's alpha is ignored by the check. A translucent surface (the game panel's 0.92) is composited over the background by `solid_surface` for the text checks; a ladder step is a flat fill and is read as drawn, opaque.
- The tokens are not read by the crate's own widgets (their gap, padding and radius stay the existing fields, T5 keeps the two from drifting). A token the crate never read would be dead weight, but the issue asks for them as the skin's vocabulary for a host's own chrome and `Role::Surface` is the one place the crate draws from them; they are data, checked, and the one source a host should build its nodes from. Design default.
- The ladder step has no `high_contrast_problems` floor: high contrast separates surfaces by frame (`surface_edge`), and a 7:1 fill step would be a black-to-grey jump with no meaning.

## Pending index

- home-still has no document on design tokens or elevation. `distill_search` ("design tokens semantic naming scale consistency design system", "perceptual lightness difference just noticeable difference", 2026-11-01) returned only Marschner's CIELAB chapter and Wagemans. To index: the W3C Design Tokens Format Module (2025.10), Material 3's *Color roles* and Apple's *HIG: Materials*, and a paper on the lightness JND of large flat fields (grimoire's "about 2", "comfortable about 4" is a measured studio figure, not a citation).
