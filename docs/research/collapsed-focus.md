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

- **A collapsed first pane disables its subtree.** `update_splits` writes `InteractionDisabled` on
  every focusable descendant of a pane that is collapsed to its minimum (not peeking), and takes the
  marker off when the pane is restored. `InteractionDisabled` is already what `navigate_by_input`,
  `FocusScopes` and `first_stop` treat as "not a stop", so one path serves Tab, the arrows, F6 and
  the pad.
- **`TabIndex` is kept, not removed**: a disabled widget keeps its index so restoring the pane gives
  back the same order, and nothing has to remember it.
- **Focus inside a collapsing pane goes to the divider**, which is the only control that can give the
  pane back — the same rule the crate already gives a section's closing body (`update_sections`).
- **A peeking pane is live.** While it floats it is on screen and fully usable; its content is
  enabled, and Escape closes it.

## Parameters

| Name | Value | Where from |
|---|---|---|
| collapsed | the pane's `flex_grow` at its minimum, no peek | `SplitPeek::collapsed` |
| the new focus | the split's divider | `update_sections`' rule for a closing body |
| reachable while peeking | everything in the pane | it is drawn |

## Validation targets

- A collapsed first pane's widgets carry `InteractionDisabled`; Tab and F6 skip them and land on the
  divider or in the second pane.
- Focus inside the pane when it collapses moves to the divider in the same frame, with the ring
  shown.
- Restoring the pane removes `InteractionDisabled` from every widget it was put on, and nothing else
  changes.
- A peeking pane is not disabled: its widgets take focus and the pointer.
- A settled split writes nothing.

## Deviations

- **The pane keeps its widgets** rather than despawning them (docks do): a split's panes are the
  game's own bundles, and a game's state lives in them. Disabling is the reversible half of the same
  rule (design default).

## Pending index

- Nothing new: the corpus had two craft stems (*Future design of accessibility in games*, *Removing
  the HUD*); no shipped-game write-up on collapsed panels exists in the index, so Rune's behaviour is
  cited as the shipping reference.
