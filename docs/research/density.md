# Density: a compact skin for desktop tools

Researched 2026-11-01 (iteration 23). Issue #6 (G-02, `grimoire-ui`, `consumer:bevy_grimoire`) is the pass's item: a tool has dozens of rows on screen where a game menu has six, and the crate's comfortable sizes (44 px targets, 14 px padding) are right for a thumb at couch distance and wrong for a mouse at arm's length. The same need is a game's: a strategy game's build list, a modding screen, a dev console, a photo-mode inspector.

## Sources

1. Cockburn, Gutwin, Scarr and Malacria, *Supporting Novice to Expert Transitions in User Interfaces*, ACM Computing Surveys 2014, doi:10.1145/2659796, home-still `10.1145_2659796` (craft: an expert surface is a different design from a novice one, not a shrunk one).
2. Microsoft, *Xbox Accessibility Guideline 101: Text display* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/101, as quoted in `docs/research/scale.md` (shipping games: the PC text minimum a compact skin must still meet).
3. W3C, *Understanding SC 2.5.8: Target Size (Minimum)* — https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html (technical: the 24 px floor and its own advice to offer a density control).
4. Buitinck et al. 2013 (stem `W1728842521`): one way to do a thing, defaults that work (the API: one field, the comfortable default unchanged).

## Craft principle

Cockburn et al.: "interfaces explicitly designed for experts (e.g., keyboard shortcuts or command-line interaction) allow high levels of performance but only after extensive training." The surface that serves an expert at a desk is a separate setting of the same design: the same widgets, the same focus and keys, a smaller measure. Density is a property of the whole skin, not of one widget, so a panel full of rows keeps one rhythm (one spacing scale) instead of each widget being squeezed by hand.

## How shipping games do it

XAG 101 does not ask a PC interface to be large; it asks it to be legible and to scale: "**PC/VR:** Font size should equal or exceed: 18 px at 1080p, 36 px at 4K" (against the console's 26 px). A PC surface used with a mouse is allowed a smaller body text than a couch one, and this crate's reference is 720p, so 14 px there is 21 px at 1080p, 17% above that minimum, and the player's `text_scale` still multiplies it. The compact skin therefore gives up target height and padding, which a mouse does not need, and not legibility.

## Model adopted

- **C1. `GooeySkin::density: Density`**, `Comfortable` (the default) or `Compact`, serde data with `serde(default)`. Density does not change a skin's colours or fonts.
- **C2. Compact is two derived numbers.** The padding every widget reads is half of `padding` ([`GooeySkin::pad`]) and the least target is `min_target` capped at `MIN_TARGET_PX`, 24 px ([`GooeySkin::target`]): WCAG 2.5.8's own floor, which the target-size note itself recommends meeting as a size "regardless of spacing". `apply_skin`, the tooltip's placement and the text field's inset all read `pad()` and `target()`, so there is no widget that forgets the density.
- **C3. The floor is still checked.** `problems()` tests `target()`, so a compact skin whose `min_target` is below 24 px still reports `TargetTooSmall`; `min_target` of 44 or 30 is capped to 24 in Compact and passes, 20 is reported.
- **C4. `SkinPreset::Tool`**, the fourth look: a cool neutral dark palette and a blue accent (the editor palette: unlike Dark's amber it is not a game's), `Density::Compact`, text 14 px, heading 20, gap 6, padding 10 (5 as drawn), radius 4, border 2, meter 10, target 24. It joins `SkinPreset::ALL` after High contrast; a theme step still writes colours only, so a player who steps the Theme row onto Tool gets the palette at their own sizes and density.

## Parameters

| value | Comfortable | Compact / Tool |
| --- | --- | --- |
| least target | `min_target` (44) | `min(min_target, 24)` |
| padding | `padding` (14) | `padding / 2` (Tool: 5) |
| text, heading | 20, 32 | Tool: 14, 20 |
| gap, radius, border, meter | 10, 8, 3, 24 | Tool: 6, 4, 2, 10 |

The 2 px border is the ring's weight: WCAG 2.4.13 asks a focus indicator at least 2 CSS px thick.

## Validation targets

- A compact button is at least 24 px and shorter than the same button in a comfortable skin, at 1280x720 (`tests/skin.rs`, laid out in a headless window).
- `SkinPreset::Tool.skin().problems()` is empty; a compact skin with `min_target: 20` reports `TargetTooSmall { px: 20 }`; with 44 it reports nothing.
- Every preset's skin passes its own checks; the dark, parchment and high-contrast stills are pixel-identical to before (`compare -metric AE` 0): density is invisible until chosen.
- A panel padded by 14 comfortable is padded by 7 compact; a list's rows, a choice, a toggle and a text field are each at least 24 px high in Tool at 1x, 2x and 1.5x text, with nothing leaving its panel.
- The Tool palette's text, muted text, accent, danger, ring and edges pass `problems()`.
- Frame time within 0.5 ms of before.

## What great looks like

A settings panel in Tool beside the same panel in Dark: the same widgets in the same order, a third the height, text still crisp and legible at 1080p, the ring still plainly visible, hit rows evenly spaced. It reads like a tool's inspector, not a game menu shrunk in an image editor: padding and gap shrink together, so rows keep one rhythm.

## Deviations

- The issue asks for "the dark-compact preset". Dark and a compact Dark would have identical colours, and `SkinPreset::of` picks the nearest preset by colour, so a Dark-coloured Tool could never be shown by the Theme row. Tool has its own palette (assumed decision, recorded on the issue). Design default.
- Tool's text is 14 px at the 720p reference: below the console's 26 px at 1080p but above XAG 101's PC minimum (18 px at 1080p; 14 px × 1.5 = 21 px here). A console game keeps `Comfortable`.
- The least compact target is the hard 24 px, not a scaled one: `min_target` is a logical px size, and the player's `UiScale` multiplies both.

## Pending index

- home-still has no document on interface density or on desktop tool design. `distill_search` ("interface density compact expert users information density desktop", 2026-11-01) returned only Cockburn et al. To index: Material Design's *Density* (m2.material.io/design/layout/density.html is a script-rendered page and could not be read here), Apple's *Human Interface Guidelines: Layout* and the Unity UI Toolkit editor theme.
