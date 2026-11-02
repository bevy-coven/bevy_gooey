# Drop candidates: every place that takes the carried item says so at pick-up

Researched 2026-11-02 (iteration 52). `slot_grid` rules (iteration 50) mark the one slot under the pointer or the carry as accepted or refused. The slot-rules note's Deviations left "valid slot on every slot at the moment of pick-up" open. An equipment panel with eight slots and a backpack of forty means the player hunts for the one slot that takes a helmet by sweeping the pointer over them. home-still has no stem on inventory or drop feedback (`distill_search`, iteration 50, unrelated papers), so the sources were fetched today.

## Sources

1. Page Laubheimer, Nielsen Norman Group, *Drag–and–Drop: How to Design for Ease of Use*, https://www.nngroup.com/articles/drag-drop/ (2020) — craft.
2. Path of Exile community thread *Inventory Background Colors*, https://www.pathofexile.com/forum/view-thread/15529 (2011) — a shipping game's paperdoll and a player asking for more than it does.
3. W3C, *WAI-ARIA 1.1*, `aria-dropeffect`, https://www.w3.org/TR/wai-aria-1.1/ — technical (the platform's model of "what can be dropped where", and the rule for showing it).

## Craft principle

NN/g: "Once the user has 'grabbed' the object that will be dragged–and–dropped, two types of feedback are needed: (1) feedback that the object was 'grabbed', and (2) feedback previewing what will happen before the user 'drops' the object." Trello's drop zones give "a clear signifier ... that the drop zone was active before the mouse cursor arrived there". A mark that moves only with the pointer is feedback after the arrival; a candidate frame is feedback before it.

## How shipping games do it

Path of Exile colours the slot under the item in the hand: "Pick up an equippable focus item and mouseover the equipment slots. You'll notice that slots turn red when the item is either over the wrong slot or the requirements are not met. The slot will turn green when the proper item is held over it." The same thread shows what that leaves out: a player asks "for the equippable slot in the paperdoll inventory outline or highlight to indicate which item slot the currently hovered/selected piece of equipment is going to go", and another, "that's only if you actually pick the item up and mouseover slots. I think it would make things much more clear if the slots highlighted". Torchlight is named there as the game that highlights ahead. (The slot-rules note said Path of Exile lights matching slots at pick-up; the thread says it does not. Corrected in that note.)

## Technical

WAI-ARIA 1.1, `aria-dropeffect`: "Typically, drop effect functions can only be provided once an object has been grabbed for a drag operation as the drop effect functions available are dependent on the object being dragged ... In addition to setting the aria-dropeffect attribute, authors SHOULD show a visual indication of potential drop targets." The candidates are a function of the grabbed object, known at the moment of grab.

## Model adopted

- **C1. `DropCandidate`** (public marker component, `Component + Reflect`): on every enabled, visible [`DropTarget`] node that takes the payload being carried, exactly while a drag is lifted. Not on the source itself (a drop there is a no-op). It is the same test the drop and the keyboard steering use (`DropZone::takes`), so a candidate is exactly a place the carry can land.
- **C2. `DropRefusing`**: the opposite, on every enabled, visible target that refuses (a `DropTarget::accepting` test, a slot's rule on either end). One system, `update_drop_candidates` (Style, first chain), puts and takes both on a difference only, for pointer, keyboard and pad carries; it runs its test only while a drag is lifted, and while none is it only takes the marks off whatever still has them.
- **C3. The crate's look, slots only**: a `Role::Slot` or `SlotEmpty` that is a candidate takes the skin's `outline` as its frame (the hover outline's colour, held 3:1 against `surface` by `problems()` in every preset); a refusing filled slot becomes the panel's well (`surface` fill, quiet frame, `text_muted` name and icon); the focus ring still wins. Other targets get the markers and no look: a host styles its own with `Has<DropCandidate>`.
- **C4. The look says "here" calmly.** A first build framed takers in `accent` (amber). In the inventory example, with 40 slots mostly empty and every one taking the sword, the whole screen lit amber and the one slot that mattered, the weapon slot, was lost in it (capture `/tmp/g52/sheet.png`, iteration 52). The frame is the neutral `outline` instead, so a backpack where everything takes the item gets a calm frame and an equipment panel, where the takers are few, reads by contrast with the dimmed refusers; the red landing mark under the pointer is unchanged (R3).

## Parameters

None new: `outline`, `text_muted`, `surface` and the slot's existing border width.

## Validation targets

Lifting the sword: the helm's slot (it may go back), the free slots and the slots of a second grid carry `DropCandidate`, the helmet-only slot carries `DropRefusing`, the source neither; let go, Escape or a cancelled drag removes them in the next frame; a keyboard carry marks the same set; a rule changed during the carry moves the set; a filled slot is a candidate only when it would also take the swap back (the back-rule of R2); a plain `DropTarget` with `accepting` follows its test and a disabled one is neither; the taker's frame is `outline` and a refusing filled slot's fill, frame and words are `surface`, the quiet frame and `text_muted` in all four looks, the focus ring still wins on a focused taker; thirty carried frames write no component.

## Deviations

A design default: takers are framed in `outline`, not filled, so a filled slot's icon is never tinted; non-takers are dimmed only when filled (an empty slot refusing keeps the panel's well and its quiet frame, and is told apart by its neighbours' frames). No "locked slot" look beyond that. Not in scope: a hint for a stack merge.

## What great looks like

An equipment panel beside a backpack: lift the sword and, at once, the weapon slot and the free slots take a clear light frame while the torch slot's icon turns grey; the pointer over the torch slot has the red landing mark as before; the frames vanish the frame the item lands.

## Pending index

The three sources are web pages: `paper_download` cannot take them; home-still has no stem on inventory or drop feedback. Not queried this pass (the PoE page needed the `read` tool; the NN/g and ARIA text is quoted from the iteration-50 note and the W3C page).
