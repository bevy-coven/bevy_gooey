# bevy_gooey: the research behind the design

Searched in home-still (`hs distill search`) on 2026-10-30. Each rule under "Model adopted" is load-bearing in the crate.

## Sources

1. Llanos, S. C. & Jørgensen, K. (2011). *Do Players Prefer Integrated User Interfaces?* DiGRA. doi:10.26503/dl.v2011i1.514
2. Iacovides, I. et al. (2015). *Removing the HUD.* CHI PLAY. doi:10.1145/2793107.2793120
3. *Future design of accessibility in games: A design vocabulary* (2019). International Journal of Human-Computer Studies. doi:10.1016/j.ijhcs.2019.06.010
4. *Text readability in augmented reality* (2024). Virtual Reality. doi:10.1007/s10055-024-00949-6
5. *Expanding Targets in VR* (2023). arXiv:2308.12515
6. *Usability Testing of Video Game Controllers* (2016).
7. Hicks, K. et al. (2018). *Good Game Feel.* DiGRA. doi:10.26503/dl.v2018i1.936
8. *Designing Game Feel: A Survey* (2021). IEEE Transactions on Games. doi:10.1109/tg.2021.3072241
9. *Supporting Novice to Expert Transitions in User Interfaces* (home-still stem; year not indexed).
10. *Promoting Hotkey use through rehearsal with ExposeHK* (2013). CHI. doi:10.1145/2470654.2470735
11. Buitinck, L. et al. (2013). *API design for machine learning software: experiences from the scikit-learn project.*
12. Gregory, J. *Game Engine Architecture*, 3rd edition.

Not from home-still: W3C, *Web Content Accessibility Guidelines (WCAG) 2.2* — the 4.5:1 text and 3:1 non-text contrast minimums and the 24 px target minimum.

## Craft principle

- R1, Llanos & Jørgensen 2011: players "always prefer to have relevant and sufficient information that allows them to interact meaningfully with the game mechanics", but "once the players receive more information than they need, superimposed UI elements become annoying".
- R2, *Future design of accessibility in games* 2019: "all the guidelines could be framed as options"; Presentation Options cover text that is clear, and "resolution of screen could make text too small". *Text readability in augmented reality* 2024: "contrast… and font size are critically important" and "the lower the contrast, the larger the font-size needs to be".
- R4, Hicks et al. 2018: "Every action that the player can take is accompanied by animation, sound, special effects", and multimodal feedback is "an accessibility feature". *Designing Game Feel: A Survey* 2021, citing Kao: "medium and high levels of juiciness outperformed extreme levels and the absence of juiciness".

## How shipping games do it

- Llanos & Jørgensen 2011, from players of shipped games: "once the players receive more information than they need, superimposed UI elements become annoying".
- Iacovides et al. 2015 (*Removing the HUD*): a non-diegetic HUD lowered immersion for expert players only, and made no difference for novices. A HUD that can get out of the way serves both.

## Model adopted

- **R1. Show what is relevant, and let the density change.** HUD widgets take a detail level the player changes (backlog: `HudDetail`).
- **R2. Accessibility is a set of player options.** `GooeySkin::text_scale`; `GooeySkin::problems` checks text contrast against 4.5:1 and the focus ring against 3:1. The interface keeps its share of the screen at every resolution, so text stays above XAG 101's minimum on a 4K television, and `UiScaleSettings::scale` is the player's interface size (`docs/research/scale.md`).
- **R3. Targets a thumb and a pad can reach.** `GooeySkin::min_target` (44 px default, 24 px floor in `problems`); pads move focus between widgets (`navigate_by_input`, `gamepad_select`), never a cursor.
- **R4. Every action answers at once, on more than one channel, in moderation.** `apply_skin` restyles in the frame of the state change; `UiFeedback` lets the host add sound.
- **R5. The novice's action rehearses the expert's.** Shortcuts shown on the button they fire (backlog: `Shortcut`).
- **R6. A consistent, composable, inspectable, few-typed API with sensible defaults.** The widget functions (`button`, `panel`, …) return `impl Bundle` and compose in `children![]`; one `Role` and one `apply_skin`; `GooeySkin::problems` inspects; every default works.
- **R7. Strings come from the host.** Widgets take `impl Into<String>`; fonts come from `GooeyFonts`.

## Parameters

|field|`dark()`|`parchment()`|
|---|---|---|
|background|srgb(0.07,0.08,0.10)|srgb(0.18,0.13,0.09)|
|surface|srgba(0.12,0.13,0.16,0.92)|srgba(0.91,0.85,0.72,0.95)|
|surface_raised|srgb(0.20,0.22,0.27)|srgb(0.83,0.74,0.58)|
|text|srgb(0.93,0.94,0.96)|srgb(0.20,0.14,0.09)|
|text_muted|srgb(0.62,0.65,0.71)|srgb(0.42,0.33,0.24)|
|accent|srgb(0.95,0.68,0.20)|srgb(0.55,0.16,0.12)|
|accent_text|srgb(0.10,0.08,0.05)|srgb(0.97,0.93,0.85)|
|danger|srgb(0.86,0.27,0.24)|srgb(0.70,0.12,0.10)|
|focus_ring|srgb(0.45,0.78,1.0)|srgb(0.12,0.38,0.55)|
|outline|srgb(0.44,0.47,0.54)|srgb(0.48,0.38,0.27)|
|scrim|srgba(0,0,0,0.6)|srgba(0.08,0.05,0.02,0.55)|
|sizes|text 20, heading 32, gap 10, padding 14, radius 8, border 3, meter_height 24, min_target 44, text_scale 1.0|same|

## Validation targets

WCAG contrast of the presets (relative luminance, `surface` composited over `background` at its alpha):

|check|dark|parchment|
|---|---|---|
|text on surface|14.20|9.80|
|text on surface_raised|10.23|8.18|
|text on hovered button|5.16|4.92|
|accent_text on accent|9.42|7.38|
|focus_ring on surface_raised|6.32|3.65|
|accent on surface_raised (a meter's fill on its track, 3:1)|6.04|4.68|
|outline on surface (a meter's frame on the panel, 3:1)|3.67|3.79|

- `min_target` ≥ 24 px (44 px default).
- A focus move is drawn one frame after the input: `GooeySystems::Navigate` runs before `GooeySystems::Style` in the same `Update`.

## Deviations

- No animation in slice 1 (R4 says moderate juice; deferred to the backlog).

## Pending index

Not in home-still yet; `paper_search` for them:

- Celia Hodent, *The Gamer's Brain: How Neuroscience and UX Can Impact Video Game Design*.
- The Game Accessibility Guidelines (gameaccessibilityguidelines.com).
- The Xbox Accessibility Guidelines.

## What great looks like

Legible at couch distance, a focus that is always visible, one spacing scale, a HUD that can get out of the way.
