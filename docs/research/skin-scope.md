# Scoped skins: a subtree carries its own skin

Researched 2026-11-01 (iteration 22). Issue #5 (G-01, `grimoire-ui`, `consumer:bevy_grimoire`) is the pass's item: an editor shows a game's own skin in a preview panel next to its own tool chrome, and one global `GooeySkin` cannot do both. The same need is a game's: an options screen that previews the interface in the skin the player is about to choose (Dark, Parchment, High contrast) beside the one it wears now, a shop window drawn in a vendor's colours, a photo-mode panel that must stay legible over any world.

## Sources

1. Wagemans, Elder, Kubovy, Palmer, Peterson, Singh and von der Heydt, *A century of Gestalt psychology in visual perception: I. Perceptual grouping and figure–ground organization*, Psychological Bulletin 2012, doi:10.1037/a0029333, home-still `10.1037_a0029333` (craft: what makes a region read as one family, and so as a different one from its surroundings).
2. Microsoft, *Xbox Accessibility Guideline 101: Text display* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/101 (shipping games: Grounded's delete-save dialog, Gears 5's subtitles and chat, Halo Wars 2's speech-to-text window as the examples of text that must follow the player's settings).
3. MDN, *Using CSS custom properties (variables)* — https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Cascading_variables/Using_custom_properties (technical: scoped, inherited values; the model for a scope that the nearest ancestor decides).
4. Buitinck, L. et al. (2013). *API design for machine learning software: experiences from the scikit-learn project* (corpus stem `W1728842521`) (technical: one way to do a thing, defaults that work).

## Craft principle

Wagemans et al., on Wertheimer's principle of similarity: "all else being equal, the most similar elements (in color, size, and orientation for these examples) tend to be grouped together". A subtree drawn in one skin reads as one thing and, set beside a region in another, as a different thing: a preview panel that wore the host's own colours would not read as a preview of anything. The skin has to change for the whole region at once, and by one rule: every widget in it from its one source, so no widget in the region falls back to the surroundings' colours and breaks the group.

## How shipping games do it

XAG 101 asks that a player's text settings reach every surface: "Any adjustable text settings, such as increasing size, weight, and contrast, should carry over to all important text—this includes additional windows that overlay a UI, especially when key information like error messages are being displayed." A game's own overlays (a dialog, a toast) therefore stay in the global skin and wear the player's settings; the scope is for the region a host chose to draw differently on purpose, which is the preview and the tool, and it is explicit: a node gets a scope only where a host put one.

## Model adopted

- **C1. One component, `SkinScope(GooeySkin)`.** On any entity; the node carrying it and everything below it is styled from that skin and not from the `GooeySkin` resource. MDN's model: a custom property "inherit[s] its value from [its] parent", and the declaring selector "defines the scope in which the custom property can be used". The nearest scope at or above a widget wins; a nested scope shadows its parent's; none means the resource.
- **C2. The skin is the whole look.** A scope carries a full `GooeySkin`, sizes and `text_scale` included, so a preview of 200% text or a compact tool panel is a scope too. `GooeyFonts` stays global (a font is an asset the host loads).
- **C3. Changes are exact.** A changed or added scope restyles its subtree and no other widget; a removed scope restyles its subtree from the next one up or the resource in the same frame; a widget moved into or out of a scope is restyled; a changed resource restyles every widget, each from its own skin (a scoped widget's look does not change). Nothing is written in an idle frame, and with no scope anywhere the cost is one empty-query check.
- **C4. A scope is checked like a skin.** A scope that changes logs `problems()` warnings, prefixed, once, and is applied anyway.
- **C5. Nothing else moves.** A toast stack and the legend are spawned by the crate or the host at the root and are the global skin's; a tooltip box is a child of its widget and follows the widget's scope.

## Parameters

None. A scope has no tunable beyond the skin it holds.

## Validation targets

- Two buttons under different scopes carry each scope's `accent` as their pressed fill; the same button with no scope carries the resource's (`tests/scope.rs`).
- Removing a scope restyles its button with the resource's skin in the same frame; a changed scope restyles its subtree and a sibling subtree not at all (the sibling's `BackgroundColor` is not marked changed); a nested scope shadows its parent and, removed, the parent shows through.
- A widget reparented into a scope takes its skin; out of it, the resource's. A changed resource leaves a scoped widget alone and restyles an unscoped one.
- A scope's sizes apply (a `text_scale` of 2 doubles the text size and a different `min_target` the button's height), so the scope is the whole skin.
- Thirty idle frames write no `BackgroundColor`, `Node` or `TextColor` with scopes present.
- Frame time within 0.5 ms of before.

## What great looks like

A settings panel with two columns: the left, the live interface in the player's skin; the right, the same panel under a `SkinScope` of the skin on the focused theme row, fully drawn, nothing bleeding between them (no widget of the preview in the surrounding colours, no frame or ring of the global skin inside it). Beside each other the two read as two families at a glance.

## Deviations

- A scoped widget ignores the global `text_scale`: XAG's "carry over to all important text" is the host's call for a scope, which is a deliberate departure from the player's settings (a preview of a skin is the exception). A host that wants the player's text size inside a scope writes `scope_skin.text_scale = global.text_scale` itself. Design default.

## Pending index

- home-still has no document on theming systems or design tokens; `distill_search` ("design system theming cascade inheritance scoped style override components", 2026-11-01) returned only unrelated game-design papers. To index: MDN's custom properties page, Flutter's `Theme` and `Theme.of(context)` documentation, Material Design's *Theme overlays*.
- `catalog_recent` was not re-scanned this pass beyond the last pass's result (no game-UI document added since iteration 21).
