# The modal dialog: focus goes in, stays in, and comes back

Researched 2026-10-30 (iteration 1). home-still's distill answered `500: gpu busy` (bge-m3 could not load beside a 14.6 GB vLLM engine on 192.168.1.110), so this note rests on the three web sources below and the vendored Bevy 0.20 source; the corpus searches are under `## Pending index`.

## Sources

1. Microsoft, *Xbox Accessibility Guideline 112: UI navigation* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112 (craft, and its shipped-game examples: Forza Horizon 4, Sea of Thieves, Halo Infinite).
2. W3C WAI, *ARIA Authoring Practices Guide: Dialog (Modal) Pattern* — https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/ (technical: the modal focus contract).
3. Bevy 0.20 source: `bevy_ui-0.20.0/src/auto_directional_navigation.rs` (`AutoDirectionalNavigator::get_navigable_nodes`) and `bevy_input_focus-0.20.0/src/tab_navigation.rs` (`TabNavigation::navigate_in_group`) (technical: what the engine does today).

## Craft principle

XAG 112: players using "keyboard-only or digital, mouth-operated joysticks … can be hampered or blocked from gameplay if focus doesn't move in a clear, consistent way. The movement of focus should always be predictable." And: "If focus can be moved to a UI element of an interface, focus can be moved away from that element by using the same input (keyboard or mouse) method."

A focus that slides behind a modal dialog breaks both: the player sees a dialog and no focused control in it, and the control that has focus is under the scrim, where the D-pad's next press is a guess.

## How shipping games do it

XAG 112, on Forza Horizon 4: "when a player has focus on the "Settings" element button, initiating a control to move the focus down results in the focus landing on the "Drone Mode" button.) If a single down-press had moved the focus from "Settings" to "Change Car" or any other menu element that isn't visually intuitive, this design wouldn't be following the meaning or operation of the UI."

XAG 112, on Halo Infinite: "When the player selects a mission from the missions list, cursor focus is automatically brought to the area of the map in which the selected mission is located." Opening a layer moves focus into it; the player never hunts for it.

## Model adopted

The APG's modal contract, which is also what Bevy's Tab navigation already does for a modal `TabGroup`:

- APG: "Windows under a modal dialog are inert. That is, users cannot interact with content outside an active dialog window." → **D1. Arrows and the D-pad stay in the dialog.** `navigate_by_input` considers only the widgets in the focused widget's *scope*: its nearest `TabGroup` if that group is modal, otherwise every widget outside a modal group. This is exactly the set `TabNavigation` cycles, so Tab and the arrows agree.
- APG: "When a dialog opens, focus moves to an element inside the dialog. … Generally, focus is initially set on the first focusable element." → **D2. A dialog takes focus when it opens.** If focus is not already inside a new `dialog()`, it moves to the dialog's first widget in tab order. A host that wants a different default puts Bevy's `AutoFocus` on it, and that wins (focus is already inside).
- APG: "If a dialog contains the final step in a process that is not easily reversible … it may be advisable to set focus on the least destructive action." → the gallery's Quit dialog autofocuses **Stay**, not Leave.
- APG: "When a dialog closes, focus returns to the element that invoked the dialog unless … The invoking element no longer exists." → **D3. Closing a dialog gives focus back** to the widget that had it when the dialog opened, if focus was still inside the dialog (or gone with it) and that widget still exists. A host that moved focus elsewhere itself keeps its choice.

## Parameters

None: no setting changes. The scope rule is Bevy's own (`TabGroup::modal`), the candidate scoring is Bevy's own (`bevy::input_focus::navigator::find_best_candidate` with the app's `AutoNavigationConfig`), and manual `DirectionalNavigationMap` edges are honoured first, as `AutoDirectionalNavigator` does.

## Validation targets

- With a dialog open and focus on its bottom button, Down leaves focus on that button (before: it moved to the menu button behind the scrim; seen in `/tmp/g1/before/dialog-down-1280x720.png`, where the focus ring sits on Quit, under the dialog).
- Up and Down move between the dialog's own buttons.
- Spawning a `dialog()` from a button's `Activate` moves focus to the dialog's first button, or to its `AutoFocus` child.
- Despawning the dialog returns focus to the button that opened it; a host that set focus elsewhere first keeps it.
- With no dialog open, arrows reach every widget, as before.
- No frame-time cost: the scope check runs only on a frame with a directional press.

## What great looks like

The player never loses the focus ring: it is inside the dialog the frame the dialog appears, it cannot be pushed under the scrim by any arrow, D-pad or Tab press, and it is back on the button they pressed when the dialog goes. A game writes no focus code for any of it: the gallery's Quit dialog drops its hand-written "focus Quit again" system.

## Deviations

- APG: "`Escape`: Closes the dialog." Not adopted in iteration 1: closing is the game's decision (a "you must choose" dialog has no cancel), and a game's back button (Escape, the pad's East) is its own binding. A design default. **Superseded in iteration 15** (`docs/research/cancel.md`): Escape and a pad's East are one `Cancel` event that reaches the dialog; the crate still closes nothing, so the decision stays the game's, and one observer on the dialog serves both devices.
- APG suggests focusing a static heading for long content; `dialog()` has no static focusable element, so D2 focuses the first widget. A design default.
- Added after the first captures: a panel whose parent is the dialog's scrim is drawn opaque, `GooeySkin::solid_surface()` (`surface` composited over `background`), where a HUD panel keeps `surface`'s 0.92 alpha. APG: mark a dialog modal only when "Visual styling obscures the content outside of it"; the dark preset's translucent panel let the menu's "Play" ghost through "Leave the game?" (luminance spread 0.027 → 0.000). `problems()` now checks text against that same opaque colour. Assumed decision: opaque dialog panels over opaque surfaces everywhere (loses HUD translucency) or a darker scrim (still ghosts).

## Pending index

- distill was unavailable this pass (`gpu busy: 4867 MB free < 5000 MB required to load bge-m3`). Search the corpus for: "modal dialog keyboard focus trap", "gamepad menu navigation focus", "spatial navigation directional focus", and cite *Future design of accessibility in games* (2019) for the navigation vocabulary when it answers.
- Index XAG 112 and the APG dialog pattern into home-still.
