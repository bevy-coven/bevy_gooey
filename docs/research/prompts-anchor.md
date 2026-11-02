# Prompts anchor: the host chooses the corner of the legend

Researched 2026-11-02 (iteration 54). Extends `docs/research/prompts.md` (P5: the legend stands bottom-left). home-still has nothing on legend placement (`hs distill search "button prompt legend placement screen corner console UI safe area"`, 2026-11-02: only a gaming-disorder case series came back). The craft and shipping-game sources are web copies, the technical one is taffy's source.

## Sources

1. Microsoft, *Xbox Accessibility Guideline 112: UI navigation* — https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/112 (craft: what must stay constant is the *place* and the *order*, not which corner).
2. Epic Games, *Using the Common Bound Action Bar in Unreal Engine* — https://dev.epicgames.com/documentation/en-us/unreal-engine/using-the-common-bound-action-bar-in-unreal-engine (shipping practice: the engine plugin behind Lyra's menus leaves the place to the host).
3. taffy 0.14.0 `src/compute/flexbox.rs`, `perform_absolute_layout_on_absolute_children`, and Bevy 0.20's `UiTransform` (technical: how an absolute node with only one inset is sized and how a percentage translation is read).

## Craft principle

XAG 112 (already quoted in `prompts.md`): "If interaction methods were to change from screen to screen, this would be very disorienting and confusing for a player." What it asks for is constancy: "these elements still appear in the same location and relative order throughout the navigation experience". Sea of Thieves chose the bottom-left; the guideline does not say every game must. A game with a minimap or a quest tracker in the bottom-left, or whose HUD hints stand bottom-right, needs the legend on another edge and still the *same* edge everywhere, which is one setting kept in one place, not a position each screen computes.

## How shipping games do it

Epic's Common UI documentation: "Common Bound Action Bar ... shows all Input Actions in your currently active UI in a single, easily referenced location. Typically, this widget is placed across the bottom of the screen." and "Put the Common Bound Action Bar wherever it makes sense in a widget blueprint". So the engine's own nav bar is bottom *and* the host's to place; the Content Examples project "anchors it to the bottom of the screen". The crate's fixed corner is XAG's example turned into a rule; the owner of a HUD needs the other corners.

## Technical

taffy gives an absolute node with a `left` and a `right` and an auto width the whole room between them (`new_width_raw = container - left - right`), so a centred legend cannot be "left 5%, right 5%" without being stretched across the screen. A node with only one horizontal inset is its content's width, which is what a legend wants. Centring is therefore `left: 50%` plus a constant `UiTransform` translation of -50% (a percentage in `UiTransform` is of the node's own size), the way `ShortcutLegend`'s card is centred. `apply_skin` writes `Node` only, and starts from the node's current value, so every inset of every anchor is written each time (the unused ones `Val::Auto`); the translation is written by one small Style system, only on a difference.

## Model adopted

- **A1. `PromptsAnchor`**, a public `Component` (`Copy`, `Reflect`, serde-free) with `BottomLeft` (the default and XAG's example), `BottomCenter`, `BottomRight`, `TopLeft`, `TopCenter`, `TopRight`. `prompts_at(anchor)` is `prompts()` with that anchor; `prompts()` is `prompts_at(PromptsAnchor::default())`. A bundle cannot carry a component twice, so there are two constructors, not a required component the game would collide with.
- **A2. The 5% safe margin is the same on every edge** (`SAFE_EDGE_PERCENT`), on the horizontal edge for a corner and on the vertical edge for the top or bottom. A centred legend has only the vertical margin.
- **A3. `max_width` stays 90%.** A centred or a right-anchored legend never leaves the window; a longer one squeezes its words.
- **A4. Changing the component restyles the legend in the frame** (`Changed<PromptsAnchor>`), so a game that moves it when its HUD changes sees it at once. Nothing is written while it stays.
- **A5. A dialog's legend takes its own anchor**, spawned by the game: a dialog over a bottom-right legend shows its hints bottom-right, the same place.

## Parameters

|part|value|
|---|---|
|insets|bottom anchors: `bottom` 5%, `top` auto; top anchors: `top` 5%, `bottom` auto; left anchors: `left` 5%, `right` auto; right anchors: `right` 5%, `left` auto; centred: `left` 50%, `right` auto|
|centring|`UiTransform` translation x -50% on a centred anchor, none otherwise|
|rest|as `prompts.md`|

## Validation targets

- At 720p, 1080p and 1440p each anchor's legend stands 5% (of the window's width, which is how taffy resolves a percentage inset on both axes... checked: a percentage of the containing block's height for top and bottom) from the edge(s) it names and is wholly inside the window; a centred one's middle is the window's middle to 1 px.
- A long legend squeezes inside 90% under every anchor.
- Changing the anchor on a live legend moves it the frame after and no insets of the old anchor remain.
- Contrast and the pointer going through are the legend's (`tests/prompts.rs`), unchanged; adding it moves no other widget.
- Thirty idle frames write nothing, for every anchor.

## What great looks like

A bottom-right legend that is the mirror of the bottom-left one: the same bar, the same 5% from the right and the bottom, the hints reading left to right ending at the corner. A centred one sits on the middle of the bottom edge like a console's nav bar. A game that moves it is not asked to compute a pixel.

## Deviations

- **Percent insets on the vertical edges are of the container's height in taffy**; the default's `bottom: 5%` is therefore already a share of the height, not the width as the old docs said ("the 5% the scrim pads is a percentage of the window's width"; that sentence is about a scrim's padding, which really is a width percentage). The validation measures it instead of assuming.
- **No free offset.** A game that wants another place spawns its own absolute node or puts the legend in a node of its own: one setting with six values, not a second coordinate system. A design default.
- Assumed decisions (owner may override): the names `PromptsAnchor` and `prompts_at`; six values; the same 5% margin.

## Pending index

`hs distill search` found nothing on HUD hint placement; the Game Accessibility Guidelines and *The Gamer's Brain* remain pending index as in `corpus.md`.
