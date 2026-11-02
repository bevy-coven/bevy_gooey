# Slot rules: a slot that says no before the player lets go

Researched 2026-11-02 (iteration 50). `slot_grid` (iteration 46) sends every drop as a `SlotMoved` and leaves the refusal to the game (`slots.md` Deviations: "No per-slot acceptance rule ... the mark does not say no beforehand"). An equipment panel (one helmet slot, one weapon slot), a shop that only buys, a locked slot a level opens: each needs a slot that takes some items and not others, and the player needs to see which before letting go. home-still has nothing on inventory or drop feedback (`distill_search` "drag and drop feedback valid drop target indication", 2026-11-02, returned unrelated papers), so the sources were fetched today.

## Sources

1. Page Laubheimer, Nielsen Norman Group, *Drag–and–Drop: How to Design for Ease of Use*, https://www.nngroup.com/articles/drag-drop/ (2020) — craft.
2. Path of Exile community thread *Inventory Background Colors*, https://www.pathofexile.com/forum/view-thread/15529/page — a shipping game's equipment slots (Grinding Gear Games' inventory, the genre's reference).
3. MDN, *Drag operations* (HTML Drag and Drop API), https://developer.mozilla.org/en-US/docs/Web/API/HTML_Drag_and_Drop_API/Drag_operations — technical (the platform's model of a conditional drop target).

## Craft principle

NN/g: "Once the user has 'grabbed' the object that will be dragged–and–dropped, two types of feedback are needed: (1) feedback that the object was 'grabbed', and (2) feedback previewing what will happen before the user 'drops' the object." The same article praises Trello's drop zone for giving "a clear signifier ... that the drop zone was active before the mouse cursor arrived there". A target that only says no after release is the error NN/g names: "the user drops an item in the wrong spot, and has to start all over again."

## How shipping games do it

Path of Exile's equipment slots colour while an item is held over them: "You'll notice that slots turn red when the item is either over the wrong slot or the requirements are not met." Green is the right slot. The refusal is shown while the item is still in the hand, on the slot the pointer is over, with no extra click.

## Model adopted

- **R1. `SlotRules { accepts: fn(&Slot, usize) -> bool }`** on a `slot_grid` root: whether slot `index` of this grid may hold this item. The default takes everything. A plain `fn` (a non-capturing closure) like `DropTarget::accepting`, so the rule is data a game writes once, not a system.
- **R2. A move is allowed when both ends are.** The target must accept the carried item at its index, and when the target is filled the swap puts its item where the carried one came from, so that slot's grid must accept it there too. A locked slot is a rule that takes nothing (`|_, i| i != 3`); an item can still be taken out of it into an empty slot.
- **R3. The refusal is the crate's existing look.** A slot that will not take the carried item is a refused target: the drop mark turns `Role::DropMarkRefused` and the ghost `DragGhostRefused` while the pointer is over it (`DragState::refused`, already drawn), a release sends one `DropRefused<SlotRef>` and moves nothing, and the keyboard and pad carry skip it when steering (`DropZone::takes`), so the player never lands on a slot that says no. A refused release is one `SlotRefused { entity: grid, from, to }`, the event a game plays its "no" sound on.
- **R4. `Slot::kind: Option<String>`** (builder `kind`): what sort of thing the item is, for a rule to read (`item.kind.as_deref() == Some("helmet")`). The crate attaches no meaning to it.
- **R5. Focus follows the item.** A drop whose source holds focus (a keyboard or pad carry, or a click that began a drag) moves focus to the target slot, so the ring is on the item the player just placed and the next arrow starts there; the grid's cursor (the one tab stop) follows focus as before.

## Parameters

None new: the refused look is `danger`-coloured by `apply_skin` already and held to its contrast pairs by `problems()`.

## Validation targets

A rule that takes nothing refuses every drop onto that slot (one `DropRefused`, no `SlotMoved`, the mark is `DropMarkRefused` while over it); a slot that takes the item gets the move; a swap onto a filled slot is refused when the item there is not accepted at the origin (rule on the other end); a cross-grid drop obeys both grids' rules; the keyboard carry skips a refusing slot and cannot drop on one; focus moves to the target after a keyboard drop and the cursor with it; a grid with no rule behaves as before (the 22 existing slot tests pass untouched); thirty idle frames write nothing; the rule changed at run time applies to the next drag.

## Deviations

No rule-aware `SlotMoved::apply` (a refused move never becomes a `SlotMoved`, so it needs none). No "valid slot" green on every slot at the moment of pick-up (Path of Exile lights the matching slots when an item is picked up): the crate marks the one slot under the pointer or the carry, a design default that keeps the drag a single mark. The rule's removal is unsupported (`SlotRules` is required by `SlotGrid`; overwrite it).

## What great looks like

An equipment panel beside a backpack: lifting a helmet and moving over the weapon slot turns the landing mark red and the ghost red at once, over the helmet slot it stays the normal accent; let go over the red one and nothing moves, the ring stays on the helmet where it was.

## Pending index

The three sources are web pages: `paper_download` cannot take them; home-still has no stem on inventory or drop feedback.
