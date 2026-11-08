# The Desk looks: Rune's neutral desktop tokens

Researched 2026-11-07 for issue #56 (G-49), asked by bevy_grimoire's feel series (dark-incorporated/bevy_dark_factory#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. Its tokens were measured and written again on `GooeySkin`. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune/src/tok.rs`.
   - `type_`: BODY 13/400, LABEL 12/500, SECTION 11.5/600, TITLE 13/600, MONO_CAPTION 12/500.
   - `space`: 4, 8, 12, 24. `radius`: 6, 8, 12.
   - `theme(dark)`: background `oklch(0.185 0.004 265)`, text `oklch(0.955 0.003 265)`.
   - `theme(light)`: background `oklch(0.958 0.004 265)`, text `oklch(0.225 0.006 265)`.
   - `accent(0, dark)`: blue `oklch(0.66 0.17 256)` dark and `oklch(0.6 0.17 256)` light.
   - `motion`: snappy 0.25 s, default 0.3 s, calm 0.4 s, lead 0.2 s, fade 0.15 s.
2. Rune, `crates/rune-kit/src/neutral.rs`: the neutral kit's elevation stacks (see `docs/research/elevation.md`).
3. Rune's `examples/kit-gallery`, built on mac_air with `cargo build --release -p kit-gallery`, rendered with `kit-gallery --kit neutral --scene ui-button--variants --snap <out>.png --theme dark|light`.

## Model adopted

- `PaletteSeed::desk()` and `desk_light()` are Rune's neutral themes and blue accent.
- `GooeySkin::desk()` and `desk_light()` are those seeds' `from_seed` colours, written out as literals so a preset is plain data. `tests/desk.rs` holds every colour to what `from_seed` derives, within 1e-3 per channel.
- On top of the colours:
  - Rune's sizes: 13 px text, 15 px heading, type scale `[11.5, 12, 13, 15, 20]`, spacing `[2, 4, 8, 12, 16, 24]`, radii `[6, 8, 12]`, radius 6 and a 1 px frame.
  - Gap 8, padding 12, `Density::Compact` and a least target of 24 px (WCAG 2.5.8).
  - `Elevation::on_dark()` or `on_light()`, and the default `MotionTokens`, which are Rune's palette.

## Where this differs from Rune

- **The light accent is deeper.**
  - Rune's light blue, `oklch(0.6 0.17 256)`, carries white words at 4.0:1 and stands 2.9:1 on a resting button, under this crate's 4.5:1 and 3:1.
  - 0.55 is the lightest step of 0.05 that meets both, so `desk_light()`'s accent is `oklch(0.55 0.17 256)`.
  - `tests/desk.rs` pins Rune's own as refused.
- **They are skins, not steps of the theme row.** Protected tests pin `SkinPreset::ALL` to Dark, Parchment, High contrast and Tool (`tests/density.rs`, `tests/options.rs`). The owner chose (2026-11-07) to ship the Desk looks as `GooeySkin::desk()` and `desk_light()` and leave the row as it is. A host sets one.

## Stills

`docs/media/desk.png` puts the gooey gallery in `desk` and `desk_light` (1280×720 each, scaled to 640×360) above Rune's `ui-button--variants` scene in its neutral kit, dark and light, as rendered at the pin above.
