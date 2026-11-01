# Anchored popover: a panel beside a widget, kept in the window, closed by the player

Researched 2026-11-01 (iteration 25). Issue #8 (G-04, `grimoire-ui`, `consumer:bevy_grimoire`) is the pass's item: menus (G-05), context menus (G-06), a command palette's results, a select's list and a hover card all need one floating panel placed beside a widget, kept inside the window, and closed the way the player expects. Today a game that wants one hand-places a node with `Val::Px` from a guessed anchor position and finds out at the window's edge that it hangs off the screen.

## Sources

1. Lakier et al., *A Comparison of Virtual Reality Menu Archetypes: Raycasting, Direct Input, and Marking Menus*, IEEE TVCG 2024, home-still `10.1109_tvcg.2024.3420236` (craft: where a pop-up menu appears relative to the invoking hand decides whether it can be used).
2. XAG 112 on *Halo Infinite* (Xbox Accessibility Guidelines), home-still, quoted in `docs/research/dialog.md`; and *Defender's Quest DX* (Doucet), quoted in `docs/research/focus.md` (shipping: a layer that opens takes focus; the player is never left hunting for it).
3. Floating UI, *flip* and *shift* (https://floating-ui.com/docs/flip, https://floating-ui.com/docs/shift) and the W3C WAI-ARIA Authoring Practices, *Menu Button Pattern* and *Menu and Menubar Pattern* (https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/) (technical: the placement algorithm every web UI kit shares, and the focus and dismissal contract of a menu). The scikit-learn API paper (`W1728842521`) for one way to do a thing.

## Craft principle

Lakier et al. on a menu invoked away from where the hand is: "sometimes I would bring up the menu and there would be things a few inches from my face. I could still select them fine, but it was disorienting at first." And on reach: "This can be beyond arms reach if the menu is first invoked too far from the body." A panel the player asked for must open where they are looking, beside what they pressed, and entirely where they can reach it: a menu that is half off the window is a menu whose last items cannot be chosen at all.

## How shipping games do it

XAG 112 on *Halo Infinite*: "When the player selects a mission from the missions list, cursor focus is automatically brought to the area of the map in which the selected mission is located." Opening a layer moves focus into it, and (this crate's dialog rule, `dialog.md`) closing it gives focus back to what opened it. Doucet's *Defender's Quest DX* keeps one fixed cursor for the pad and a free pointer for the mouse and "lets the player fully control what input style is best for them": every popover must be as complete by keys and pad as by mouse, so a modal one takes focus in and its Back (Escape, B) closes it, while a pointer press outside it also closes it.

## Model adopted

- **P1. `popover(anchor, placement)`** is a bundle for a panel that is a child of the anchor (so the anchor despawning despawns it, which is the "anchor gone: closes" edge case for free) and is laid out by Bevy's own `Popover`, `bevy_ui_widgets::popover`, never a second placement engine. The panel is `Role::Popover`: an opaque `solid_surface` card framed in `outline` (it floats over anything, so its edge must hold 3:1 like a dialog's), padded, with a shadow, one `GlobalZIndex` step above a dialog so a popover opened from a dialog shows over it (tooltips stand above both).
- **P2. `Placement { Below, Above, Right, Left, AtPointer }`.** Floating UI, *flip*: "Changes the placement of the floating element to keep it in view ... This prevents the floating element from overflowing along its side axis by flipping it to the opposite side by default"; and "shift prevents the floating element from overflowing a clipping container by shifting it to stay in view" (*useFloating*), with the rule "ensure flip() is placed before shift() in your middleware array". Bevy's `Popover` takes an ordered list of candidate positions and picks the first that fits, else the least occluded, so the list is: the preferred side aligned to the anchor's start, the same side aligned to its end (the shift: a panel wider than the room right of the anchor's start sits flush with its end instead), then the mirrored side both ways. `AtPointer` opens at the pointer, below and to its right, flipping to its left and above at an edge: the anchor is a zero-size point node the crate places at the pointer's position when the popover opens.
- **P3. Closing is the crate's, and it says so once.** `PopoverClosed { entity, reason }` (an `EntityEvent` on the popover, `CloseReason::{Cancel, Outside}`) is triggered and the popover despawned: by a pointer press anywhere outside the panel (a press on the anchor itself is not outside for a side placement, so the anchor's own toggle works; for `AtPointer` the anchor is a region and a press on it is outside), and by `Cancel` (Escape, B). A game's own despawn sends nothing: it did it. Anchor despawned closes it by the hierarchy. Window resize: Bevy's system places it again every frame.
- **P4. `popover_modal(anchor, placement)`** is the same panel as a modal scope (a `Dialog` and a modal `TabGroup`): focus moves into it when it opens, Tab, the arrows and the D-pad stay inside it, `Cancel` is sent to it and closes it, and focus goes back to the widget that opened it (APG, *Menu and Menubar*: "Escape: Close the menu that contains focus and return focus to the element or context, e.g., the menu button or parent menubar item, from which the menu was opened"). `popover()` is non-modal: focus stays where it was (a combobox keeps it in its field), and a `Cancel` from focus on the anchor or inside the panel reaches the panel, which closes. Two functions, not a `bool` parameter.
- **P5. The panel takes no press from under it.** A press, release or click on the panel (its padding, a label) stops at it, so a popover that is a child of a `button` never presses or activates that button. Its own buttons are Bevy's and activate as usual.
- **P6. Nothing draws unstyled.** The panel is born `Display::None` and `apply_skin` shows it styled, like the tooltip's box, so a popover spawned after the skin ran is not drawn bare for a frame.

## Parameters

| value | choice |
| --- | --- |
| gap from the anchor | `gap / 2` plus one line of frame |
| window margin | `gap` |
| padding, row gap | `pad()`, `gap / 2` |
| frame, corner | `border` (whole device pixels), `radius` |
| z order | `GlobalZIndex(150)` (dialog 100, tooltip 200) |
| candidates | side/start, side/end, mirror/start, mirror/end |

## Validation targets

- A popover anchored at the right window edge with `Right` is placed fully inside the window (flipped to the left of the anchor); a press outside closes it once, with one `PopoverClosed { reason: Outside }` (`tests/popover.rs`).
- Each of the four sides opens on that side when there is room, flips when there is not, and shifts along the edge when the anchor is at a corner; never a pixel outside the window at 1x, 2x and 1.5x text.
- `AtPointer` opens at the pointer and flips at the right and bottom edges.
- A press inside the panel closes nothing and does not press the anchor; a press on the anchor does not close a side popover.
- A despawned anchor takes its popover with it; a resized window places it again.
- A modal popover takes focus in, keeps Tab inside, closes on Escape and the pad's East with one `Cancel`-reason event, and returns focus to the anchor; a non-modal one leaves focus where it is and closes on Escape from the anchor.
- The panel's text and frame hold the contrast the dialog's do in every preset, and it writes nothing while idle.

## What great looks like

A card that reads as lifted above the screen: an opaque fill a step above the panel, a 3:1 frame, a soft shadow, the skin's corner, one gap off its anchor and never touching it, never closer than a gap to the window. At 200% text a menu still fits and shifts rather than clips. In the gallery's clip: a button opens a popover below it; the window narrows or the button sits at the right edge and the card flips to the left; Escape closes it and the ring is back on the button.

## Deviations

- The issue's `popover(anchor, placement)` plus `modal: bool` is two functions, `popover` and `popover_modal` (assumed decision): a boolean parameter reads as `popover(a, p, true)` at the call.
- The crate despawns the popover on a close request (the issue asks for it), where `Cancel` for a dialog only reports. A popover has no state a game must save first; `PopoverClosed` is sent before the despawn so the game can follow.
- "Shifts": Bevy's `Popover` flips and picks the least-occluded candidate but never clamps. The start/end candidates are the shift; a panel taller or wider than the window (no candidate fits) takes the least occluded and is clipped, which a host avoids by capping the content (a `list`).
- A press on a side popover's anchor is not "outside", so a game's toggle on `Activate` opens and closes with one rule; it is the game's job to despawn on the second press.

## Pending index

- home-still has no document on pop-up placement. `distill_search` ("pop-up menu placement near screen edge", "cascading menus", "context menu", 2026-11-01) returned VR menu papers, Split menus and Kurtenbach's lineage only. To index: Floating UI's *flip* and *shift* pages, the WAI-ARIA APG *Menu and Menubar* pattern, and Material 3's *Menus* guidelines.
