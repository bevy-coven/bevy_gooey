# State roles: the skin owns the interaction states a host composes its own controls from

Researched 2026-11-02 (iteration 55). Issue #37 (G-33, `grimoire-ui`, `consumer:bevy_grimoire`). A skin has a fill for the crate's own button states (rest `surface_raised`, hovered, pressed `accent`), but a host that builds its own rows, chips and overlays from the skin has no home for "a row under the pointer", "the same row with the pointer down, recessed *below* hover", "the world stepped back while a key is held", "a card the scrim leaves lit" or "the edge of a preview box". Ten such meanings (24 colour references in the editor's protected tests) are hand-kept constants beside the skin, so "the UI's colours are the skin" is false for them.

## Sources

1. W3C, *Understanding Success Criterion 1.4.11: Non-text Contrast* (WCAG 2.2), https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html (craft: which pairs of state colours must differ, and by how much).
2. Google, *Material Design 3: States, State layers*, https://m3.material.io/foundations/interaction/states/state-layers (shipping design system: hover, focus, press and drag as a fixed ladder of one layer each) and Visual Studio Code, *Themes*, https://code.visualstudio.com/docs/configure/themes (shipping tool: a theme names `list.hoverBackground`, `list.activeSelectionBackground` and `editorWidget.background` as separate colours, so a hover and a card are skin data, not code).
3. Bevy 0.20 `bevy_color::Mix` and this crate's own `model::cie_lightness` / `GooeySkin::hovered` (technical: a state colour derived from another is `Mix::mix`, and "distinguishable" is a CIE L\* step, as `ElevationStep` already measures it).

No game-UI paper in home-still speaks to interaction-state colour (`hs distill search` for "interaction states hover pressed visual feedback" returned an unrelated benchmark paper); see Pending index.

## Craft principle

WCAG 1.4.11: "This success criterion does not require that changes in color that differentiate between states of an individual component meet the 3:1 contrast ratio when they do not appear next to each other. For example, there is not a new requirement that ... mouse hover indicators contrast with the default state. However, the component must not lose contrast with the adjacent colors." So the bar for two state fills is not 3:1 against each other (two grounds that both hold 4.5:1 text cannot be 3:1 apart); it is that each keeps the contrast of what is drawn on it and that the two are told apart. The crate's measure of "told apart" is a CIE L\* step.

## How shipping games do it

Material 3: "A state layer is a semi-transparent covering on an element that indicates its state. State layers provide a systematic approach to visualizing states by using opacity as an overlay with a fixed opacity for each state": hover 8%, focus 10%, press 10%, drag 16%, and only the highest-priority state shows at a time (press over focus over hover). A pressed control is therefore always a distinct step from a hovered one, and it is a *named* step of the system, not a per-widget colour. VS Code's themes do the same with names: hover, active selection and widget background are three keys of one table.

## Model adopted

- **S1. Seven state colours on the skin**, each an `Option<Color>` (serde `default`, `None` in every preset) with an accessor that answers from the nearest field when it is `None`, so a skin without them still answers every role: `hover` (`hovered()`, which today is `surface_raised` a third of the way to `accent`; **setting `hover` changes every button's hover too**: one path), `pressed()` (default `solid_surface()`: a control with the pointer down sinks to the panel it stands on, recessed against the lifted hover in every preset), `held()` and `held_ground()`, `unchecked()` (default `text_muted`), `card()` (default `solid_surface()`, the dialog panel's own fill) and `preview_edge()` (default `outline`).
- **S2. One fade for both held colours.** `held_fade: f32` (default 0.6, 0 to 1): `held_ground()` defaults to `solid_surface()` mixed toward `background` by it and `held()` to `text` mixed toward `held_ground()` by it (a first build faded the glyph toward the background too, which in parchment, whose text is already near its dark background, stepped nothing back), so a held ground and a held glyph "come off one fade" and a host that sets one fade keeps them composing. A skin that sets either colour answers for it.
- **S3. `Role::Hovered`, `Pressed`, `Held`, `HeldGround`, `Unchecked`, `Card` fill a node** with that colour and nothing else (the host gives it its own `Node`, as for `Role::Surface` and `Role::Tone`), and `Role::PreviewEdge` paints a node's border and no fill. Restyled in the frame the skin changes or the host swaps the role (`Changed<Role>`). `Hovered` is the issue's `ROW_BG`; it is the seventh because the acceptance names "Pressed and Hover".
- **S4. `problems()` checks the pairs the crate can know**: `text` on `pressed` and on `card` at the text bar; `unchecked` and `preview_edge` on the panel at the 3:1 graphic bar (7:1 for high contrast with every other pair); and **`pressed` against `hovered()` by a CIE L\* step of at least `STATE_STEP` (4)**, reported as the new `SkinProblem::Indistinguishable { pair, delta_l, needed }`. `held_fade` outside 0 to 1 or not finite is `BadSize`. `held` and `held_ground` are not contrast-checked: a held colour is stepped back on purpose, and the crate does not know what is drawn on it.
- **S5. A theme step carries them** (`with_colours_of`, `colour_distance`) as colours of the skin; `held_fade` stays like a size. Not in the issue's list and left to the host: `VOID` (the window's ground is `Role::Surface(0)`), `PANEL_BG` (a row at rest is a step of the ladder or `surface_raised`) and `DIM` ("the live-edit colour, and nothing else": a host's tone).

## Parameters

`STATE_STEP` = CIE L\* 4, the step the presets' ladders keep (`tests/tokens.rs`); a design default (WCAG sets no number for two states not adjacent). `held_fade` 0.6: a design default, a world glyph left at 40% of the way from the background still reads and no longer competes with an overlay.

## Validation targets

A skin with every new field `None` (all four presets) draws every state role with a colour, `problems()` and `high_contrast_problems()` unchanged for the presets; `pressed` set to hover's lightness is `Indistinguishable`; hover set on the skin is the colour a button hovers in; `held()` and `held_ground()` follow `held_fade` together; a theme step to another preset drops the host's state colours with the rest of the palette; one extra match arm per role (no frame cost).

## Deviations

- The issue's roles list has no `Hover`; its acceptance names it and its table has `ROW_BG`, so `hover` and `Role::Hovered` are added (design default, recorded on the issue).
- `pressed` defaults to the panel's fill, not to `accent`: the crate's own pressed button is accent with `accent_text`, which a host's `text` cannot sit on. The crate's own button is unchanged.

## What great looks like

The editor's `ROW_BG`/`ROW_SELECTED` pair is two `Role`s on one node and a hover that is a single skin field; a world gizmo and its ground fade from one number; a dark and a parchment skin both show a pressed row sunk below a lifted hover.

## Pending index

A game-UI source on interaction-state colour is not in home-still (the Material and VS Code pages are web sources, quoted above); a shipped *game* write-up on dimming the world while an overlay is held is not indexed.
