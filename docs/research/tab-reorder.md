# A dock tab carried within its strip: neighbours step aside, the drop settles

Researched 2026-11-08 (iteration 68). Issue #70 (G-63, `grimoire-ui` and `feel`, `consumer:bevy_grimoire`): dragging a dock tab moves nothing but a landing rectangle and a title card beside the pointer; at the drop the whole strip is rebuilt and the tab is simply somewhere else.

## Sources

1. Rune, https://github.com/anomalyco/rune at `a7f276c` (MIT, open reference): `crates/rune-kit/src/shell/reorder.rs` (`Reorder::follow`, `shift`, `end`) and `crates/rune-kit/src/shell/tab_strip/tab.rs:137` (a tab's `Transition::scale(0.95)` on the `SNAP` spring) (shipping toolkit; read, no code taken). Rune's tab strip reorders "with neighbours moving out of the way and the dragged tab landing where it was drawn" (the issue's words for `shell/tab_strip.rs`).
2. W3C WCAG 2.2, *Understanding SC 2.5.7 Dragging Movements* (craft, accessibility): "All functionality that uses a dragging movement for operation can be achieved by a single pointer without dragging" and, for a kanban board, "also accomplish this by selecting the element with a single tap or click, and then activating an arrow button to move the selected element": the dock's Ctrl+Shift+arrow move is that alternative, so it animates the same (below). And SC 2.3.3 for the motion's off switch.
3. This crate's `docs/research/dock.md` (the drag, the landing, the rebuild), `layout-motion.md` (`MovesSmoothly` and `LayoutTravel`), `motion-tracks.md` (`MotionTarget`) (technical).

## Craft principle

SC 2.5.7: a drag has another way to the same end, and this item must not make the other way worse: the keyboard and pad move (Ctrl+Shift and an arrow) is a drop that happens without a pointer, so it gets the same glide, not a jump the drag no longer has. Escape ("taking back" a carried tab) is the drag's undo and springs everything home.

## How shipping games do it

Rune's `Reorder`: "The pointer moved `dy` since the press on row `index`, which is `h` tall": `to = index + round(dy / pitch)` clamped to the list; "How far row `index` sits from its place: the dragged row with the pointer, and the rows between its place and its target a step toward the gap it left" (`shift`: `-pitch` for the rows after the dragged one up to the target, `+pitch` for those from the target to before it). The target comes from the pointer's displacement from where the press was, against **places measured once**, not from where the moving rows are now: nothing feeds back.

## Model adopted

- **R1. Only the picture moves while a tab is carried.** Layout keeps every tab where it was (no child is reordered), so a landing is decided against a strip that holds still. At the press the strip's tabs are read once (`StripDrag`: each tab's centre and size, which is the carried one, where on it the press took hold). The carried tab's `UiTransform` follows the pointer (its `x` along the strip; its `y` too once the pointer leaves the strip's row by more than 1.5 tab heights, so it can be carried away to another group).
- **R2. The tabs it passes step aside on `snappy`.** The target slot `to` is the number of the *other* tabs whose original centre is left of the pointer; tabs after the carried one up to `to` take a `MotionTarget` of `-(its width + the gap)`, tabs from `to` up to the carried one `+(..)`. A pointer off the strip's row, or a strip that wraps onto two rows, steps nothing aside (the layout's own landing rules, `dock_tab_slot`, still decide).
- **R3. The drop is the existing one.** `tab_on_drag_end` makes one `move_panel` and `DockLayoutChanged`; a landing is found from tab centres with each tab's own `UiTransform` taken out (`groups_of`), so the drawing's offsets never choose the slot.
- **R4. A rebuild glides.** `update_docks` keeps where every tab was drawn (`DockShown::departed`, its `UiGlobalTransform` centre) when it rebuilds, and each new tab of the same panel gets a `TravelFrom` of that centre the frame it appears: `update_layout_motion` turns it into the first travel offset of its `MovesSmoothly` (every tab is), so the dropped tab springs from where it was drawn into its slot, the tabs that had stepped aside (now laid out at the places they were drawn at) do not move, and a **keyboard move**, a tab sent to another group, a restored layout and a panel opened from the registry that was already shown all glide the same way. Under reduced or no motion the travel lands.
- **R5. Taking back.** When the carried state ends with no change of layout (Escape, a cancelled pointer, a drop that changes nothing, a drop outside every landing) the carried tab gets a `MotionTarget::entering_from(its offset)` (it springs home from where it is drawn) and the tabs that stepped aside get a target of 0.
- **R6. The title card stays.** The ghost beside the pointer and the landing rectangle are what `tests/dock.rs` pins; they are kept (a carried tab now also follows the pointer; the two are redundant, noted under Deviations).

## Parameters

|name|value|why|
|---|---|---|
|step aside|carried width + gap|Rune's `pitch`|
|spring|`snappy`|Rune's tab spring `SNAP`|
|strip row|1.5 tab heights|a design default|
|scale of the carried tab|1.03 (`LIFT_GAIN`)|Rune's `lift`|

## Validation targets

- Dragging the first of three tabs past the second: the second travels left by the first's width and gap while the pointer moves, the third stays, layout is unchanged (no report) until the release; the release makes one `DockLayoutChanged` and the carried tab's drawn centre moves by less than 3 px in that frame, then settles into its slot.
- Escape mid-drag: no report, everything back to its place on its spring, the carried tab not jumping.
- Ctrl+Shift+Right on a focused tab: one report, the tab drawn near where it was a frame later and in the other group some frames after.
- Pointer leaves the strip: the neighbours go back. No motion: nothing glides.

## What great looks like

A tab that sticks to the pointer, neighbours that make room with a small springy step and a gap the tab is clearly heading for, and a drop that is a settle, not a teleport; the same glide when the keyboard sends a tab to another group.

## Deviations

- **Pickup lift** (iteration 72, `pop.md` addendum): the carried tab is drawn at 1.03 with the floating elevation, as Rune's `lift` does (`tab.rs:148`); the 0.95 of the press is not built.
- **The ghost and the landing rectangle remain** beside the carried tab (`tests/dock.rs` pins them); they could go in a later cutover.
- **Only a single-row strip steps tabs aside**; a wrapped strip drops as before and its tabs glide only at the rebuild.
- **Neighbours of a cross-group drop** glide only if they were laid out elsewhere; they were drawn in place, so they stay.

## Pending index

- The corpus has no paper on drag-to-reorder feedback; WCAG 2.5.7 is cited from the web and Rune from its source.
