# A collapsed pane holds no focus

Researched 2026-11-09 (iteration 83). No issue is filed. The pass's own pick, and an item
`docs/research/split-peek.md` named as not built.

The gap. A `split` whose first pane is collapsed to its minimum keeps the pane's widgets in the
tree, and they are still tab stops: Tab and F6 walk into a pane the player cannot see, and on a
peek (iteration 73) focus can sit in a floating pane that is only open while the pointer rests at
the edge. A keyboard player then has no way to tell where focus went.

## Sources

1. *Future design of accessibility in games: A design vocabulary* — 2019, IJHCS,
   doi:10.1016/j.ijhcs.2019.06.010 (craft). Its vocabulary separates what a game *shows* from what a
   game *offers*; an interface that offers a control it does not show breaks the pairing. The crate's
   own rule from this paper is in `docs/research/gooey.md`: focus is always visible.
2. *Removing the HUD* — Iacovides et al., 2015, CHI PLAY, doi:10.1145/2793107.2793120 (craft,
   shipped-game study): "those in the diegetic condition were able to leverage their existing
   expertise to progress without this information". Information a player has hidden is not
   information they are attending to; putting focus there is worse than putting it nowhere.
3. WCAG 2.2 SC 2.4.3 and 2.4.7 (technical, the bar this crate already holds focus rings to): focus
   order must be operable and the focus indicator visible. A stop inside a pane drawn 24 px wide
   satisfies neither.
4. Bevy 0.20, `bevy_input_focus::tab_navigation`: Tab walks `TabIndex >= 0` entities in
   `InheritedVisibility`; `InteractionDisabled` is the crate's marker for "not a stop". Bevy's
   `Visibility::Hidden` would also stop the pane drawing during a peek, which is the one thing the
   peek exists for.

## Craft principle

What is not shown must not be offered. Rune says the same of a hidden panel: it is removed from the
focus chain while it is out of view, and the shape that is left (the edge) is what reopens it.

## How shipping games do it

Rune's collapsible panels take their contents out of the focus order when collapsed and put focus
back on the panel's own handle; nothing inside a collapsed panel is reachable by Tab. The crate's
docks do the same thing by despawning what they do not show.

## Model adopted

- **Each pane of a `split` is a [`crate::FocusRegion`]** (`FocusRegion::navigate()` on every
  `Role::SplitPane`). F6 and Shift+F6 then reach a pane as they reach a viewport or a toolbar, and a
  **collapsed** pane's first widget is a region's start — which is the thing that was missing: before
  this, F6 cycled past the split entirely and only Tab found the collapsed pane's stops, so a player
  who navigates by F6 could never get into a collapsed sidebar.
- **Landing there peeks the pane.** The peek already opens for any visible focus inside the collapsed
  pane (iteration 73), so the widget focus lands on is the widget the player sees; the region and the
  peek are the same path, not two.
- **The pane's stops are not disabled.** A first attempt disabled every tab stop in a collapsed pane
  and moved focus to the divider. It was reverted: it contradicts the peek (which exists so a
  keyboard or pad player *can* land in a collapsed pane and see what they landed on, and
  `docs/guide/splits.md` promises exactly that), and the test that pins the peek's focus rule would
  have had to be weakened. The gap was never "the stops exist", it was "F6 could not reach them".

## Parameters

| Name | Value | Where from |
|---|---|---|
| collapsed | the pane's `flex_grow` at its minimum, no peek | `SplitPeek::collapsed` |
| the new focus | the split's divider | `update_sections`' rule for a closing body |
| reachable while peeking | everything in the pane | it is drawn |

## Validation targets

- F6 from the second pane's widget lands on the **first** widget of a collapsed first pane
  (`tests/split.rs::f6_reaches_a_collapsed_pane_and_peeks_it`; mutation-checked: removing the region
  fails it).
- That pane peeks around the landed widget, so the widget is on screen.
- `tests/region.rs`, `tests/split_peek.rs` and the rest of `tests/split.rs` stay green (59 tests across
  the three).
- A settled split writes nothing.

## Deviations

- **A collapsed pane's widgets stay reachable.** The alternative — disable them and move focus to the
  divider — was built and measured this pass and reverted: it breaks the peek's whole purpose (a
  keyboard player landing inside a collapsed pane and seeing it open around them), and the guide
  promises that behaviour. F6 reaching the pane is the fix; hiding the pane would trade one
  accessibility rule (focus is visible) for another (a region is reachable).
- **F6 order is tree order**, so a split's panes come in spawn order, not visual order after a drag
  (a design default, as for every region).

## Pending index

- Nothing new: the corpus had two craft stems (*Future design of accessibility in games*, *Removing
  the HUD*); no shipped-game write-up on collapsed panels exists in the index, so Rune's behaviour is
  cited as the shipping reference.
