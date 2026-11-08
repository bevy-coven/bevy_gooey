# A collapsed split pane peeks: the pointer at its edge floats it over the content

Researched 2026-11-08 (iteration 68). Issue #69 (G-62, `grimoire-ui` and `feel`, `consumer:bevy_grimoire`): a collapsed first pane (`toggle_split`, #13 and #53's size motion) is gone until restored. Rune's closed sidebar can peek over the content when the pointer rests at its edge and docks where it floats when asked to open from the peek.

## Sources

1. Rune, https://github.com/anomalyco/rune at `a7f276c` (MIT, open reference), `crates/rune-kit/src/shell/pane.rs:20-136` (`Peek`, `PaneState::set_open`, `peek`, `end_peek`) and `:277-345` (`SidebarPeek`: the edge zone, delays, grace and spring) (shipping toolkit; read, no code taken). Its pane range (160 to 480) is the sidebar's, not this crate's: `SplitMin` says it here.
2. W3C WCAG 2.2, *Understanding SC 1.4.13 Content on Hover or Focus* (craft, accessibility): "Dismissible: A mechanism is available to dismiss the additional content without moving pointer hover or keyboard focus ... Hoverable: If pointer hover can trigger the additional content, then the pointer can be moved over the additional content without the additional content disappearing; Persistent: The additional content remains visible until the hover or focus trigger is removed, the user dismisses it, or its information is no longer valid."
3. This crate's `docs/research/split.md` (S1 to S6, the divider and what a collapse is), `size-motion.md` (the collapse glide), `motion-tracks.md` (`MotionTarget`) and Bevy 0.20's flex rules in taffy (an absolute child leaves the flow; a flex container's grows that sum to under 1 share out only that share of the free room) (technical).

## Craft principle

SC 1.4.13 is the contract of anything a hover opens: the peek is content a hover triggers, so it must be **hoverable** (the pointer can cross from the edge onto the pane without it going), **persistent** (it stays while the pointer is on it or its edge, and goes after a short grace, not at the first stray pixel) and **dismissible** (Escape closes it with the pointer where it is). It also obscures the second pane, which is why it floats only on purpose (a rest at the edge, not a pass over it).

## How shipping games do it

Rune's `SidebarPeek`: "A closed sidebar that peeks: resting the pointer on the window's leading edge, or on the sidebar's toggle, slides the sidebar in over the content as a floating surface, without moving the layout. Leaving it closes it again; pressing the toggle docks it where it is." Defaults: "A 16 point edge zone and a 12 point margin; 50 ms on the edge, 100 ms on the toggle, and 150 ms of grace before it closes; the kit's control spring", and "Under Reduce Motion the peek fades in place over `seconds` instead of sliding" (`reduced_fade` 0.15). `PaneState::set_open`: "Opening a peeking sidebar docks it where it floats; closing ends any peek."

## Model adopted

- **P1. The collapsed edge is the divider.** A split peeks when its first pane is collapsed (it holds a `SplitSaved`, the width `toggle_split` will give back, and its fraction is at its smallest, no glide on the way) and the pointer rests on its divider (Bevy's `Hovered`) for `SplitPeekSettings::delay_seconds` (0.05 s, Rune's edge delay) of real time. The divider is the 24 px channel at the collapsed edge, a bigger zone than Rune's 16 pt. The pane itself (when its minimum leaves some of it showing) counts as the edge too.
- **P2. The pane floats; nothing under it moves.** While it peeks the first pane is `position_type: Absolute` at the split's start edge, as wide (or tall) as the restore width (`SplitSaved` of the room, held to the minimums), above the second pane (`ZIndex`), with the `floating` shadow; its slot in the flow is kept by a margin on the divider the width the collapsed pane had, and the second pane takes the free room alone (`flex_grow` 1), so **the second pane's `ComputedNode` is unchanged** (the acceptance).
- **P3. It arrives and leaves from the edge.** On open the pane's `MotionTarget` starts `-width` away along the axis (an `entering_from`, on `snappy`) and a fade (`Arriving`); on close it retargets there (so a pointer returning mid-way retargets back, keeping its speed) and, once it has landed, goes back into the flow. Under `MotionSettings::reduced` it fades without travelling; with no motion it comes and goes in a frame.
- **P4. Hoverable, persistent, dismissible.** While it is open, the pointer on the pane, its divider or within `SplitPeekSettings::margin` (12 px, Rune's) of them holds it; leaving all of them for `grace_seconds` (0.15 s, Rune's) closes it. Escape closes it at once (SC 1.4.13's dismissible; the crate's one reader of Escape for it, never when a `Cancel` was sent).
- **P5. Docking.** `toggle_split` (Enter, South, a double press) while it peeks docks it: the fraction becomes the saved one with **no glide**, because the pane is already that wide, and the peek ends in the same frame. A drag that starts on the divider while it peeks docks it the same way and carries on from the docked fraction. The keys and the D-pad on a focused divider dock it first too (a step is from the docked width).
- **P6. (Superseded, iteration 73: a visible focus inside the pane peeks it too, see Deviations.) Only the pointer peeks.** A keyboard or pad player has the toggle (Enter, South), and focus moving into a collapsed pane's content would reach clipped widgets, which is the open gap noted in the split's note. The issue's "F6 into a collapsed region opens it docked" waits for `FocusRegion` on panes; recorded under Deviations.

## Parameters

|name|value|why|
|---|---|---|
|`SplitPeekSettings::delay_seconds`|0.05|Rune's edge delay|
|`SplitPeekSettings::grace_seconds`|0.15|Rune's grace|
|`SplitPeekSettings::margin`|12 px|Rune's margin|
|travel|the pane's width on `snappy`|Rune's control spring|
|reduced fade|`fade_seconds`|the crate's one fade|

## Validation targets

- Collapsed split, pointer resting on the divider past the delay: the first pane is absolute and drawn over the second pane, with the second pane's `ComputedNode` size and place exactly as before; the pointer gone past the grace: it slides out and is back in the flow; toggling while it peeks: the fraction is the saved one, the pane is the same width, no glide, no second `ValueChange` than the toggle's own.
- Escape closes it with the pointer still on it. A drag started on the divider docks it.
- Reduced motion: no travel, a fade. No motion: opens and closes in a frame.
- A split that is not collapsed never peeks; a settled split writes nothing.

## What great looks like

A sidebar that was put away and is one glance of the pointer from coming back: it slides out over the work with its own shadow as if it had been waiting at the edge, stays for as long as the pointer is near and slips away when the pointer goes, and a click on the divider turns it into the real thing without a twitch of the layout under it.

## Deviations

- **Focus peeks (iteration 73).** `update_split_peeks` counts a *visible* focus on a descendant of the collapsed pane as the pointer being on it (WCAG 2.4.7 Focus Visible: the ring must not be clipped away), so Tab or an arrow into the pane floats it, and focus leaving takes it away after the grace. The divider's own focus does not count. Reduced motion, Escape and the grace behave as for the pointer.
- **No F6 case**: the issue's "F6 into a collapsed region opens it docked" needs a pane that is a `FocusRegion`; a split pane is not one yet. A keyboard player docks it with Enter on the divider.
- **The edge is the divider** (24 px), not a zone over the window's edge.
- **Escape is read directly** while a peek is open (`just_pressed`), because a `Cancel` is sent only from a focused widget and a peek the pointer opened has none. It never fires for a press the focused widget turned into a `Cancel`.

## Pending index

- The corpus has no paper on peeking sidebars; WCAG 1.4.13 is cited from the web, Rune from its source.
