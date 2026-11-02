# The slot grid (inventory)

Researched 2026-11-02 (iteration 46). The log's backlog has carried an "inventory grid" for twenty iterations and the loop brief names it among the widgets every game needs: a backpack, a hotbar, a chest, a shop shelf, an equipment panel, a crafting bench. Every part exists here already (a button with a ring, `icon`, `Tooltip`, typed drag and drop with a keyboard and pad path, spatial navigation) and none of it is a grid of slots a game fills with items. home-still's index has nothing on inventory interfaces (three `distill_search` queries on 2026-11-02 returned unrelated papers), so the sources were fetched today.

## Sources

1. W3C, *WAI-ARIA Authoring Practices*, "Developing a Keyboard Interface" (roving tabindex), https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/, and the Grid pattern already cited in `table.md`; W3C *Understanding SC 2.5.7 Dragging Movements* (`drag-drop-keyboard.md`) — craft (accessibility and keyboard use).
2. Capcom, *Resident Evil 4 Official Web Manual*, "Items" (a grid inventory on a pad, `drag-drop-keyboard.md`), and the Minecraft Wiki, *Inventory*, https://minecraft.wiki/w/Inventory (the mouse model every inventory copies) — shipping games.
3. W3C APG as above for the one-tab-stop rule; Bevy 0.20 `bevy_input_focus` `TabIndex` (a negative index is skipped by Tab) — technical.

## Craft principle

The APG says how a component of many focusable parts keeps the tab sequence short: "When using roving tabindex to manage focus in a composite UI component, the element that is to be included in the tab sequence has tabindex="0" and all other focusable elements contained in the composite have tabindex="-1"." A backpack of 40 slots that Tab walks one at a time takes 40 presses to leave; a player on a keyboard or a pad is stuck in it.

The same note is about dragging: WCAG 2.5.7 asks that "all functionality that uses a dragging movement for operation can be achieved by a single pointer without dragging", and `drag-drop-keyboard.md` already gives every `DragPayload` source a lift, steer and drop on keys and a pad. Moving an item is the one thing an inventory does, so the grid must be a set of such sources and targets and not a second implementation.

## How shipping games do it

Resident Evil 4's attaché case: "Moving items: Aim the laser pointer at the item and select it with [the select button], then move it to the desired location and press [the same button] again." A press lifts, direction moves, a press puts down.

Minecraft's inventory is the model for the mouse and its vocabulary: "holding a stack of items in the player's cursor and left-clicking in an inventory slot places the full stack into the slot and right-clicking places items one at a time. If there is already a stack in that slot, it gets swapped with the grabbed stack." Three rules follow that a grid can own: a slot is a square well that shows its item and a count; moving onto a filled slot swaps; what happens to two stacks of the same item (merge, split, refuse) is the game's.

## Model adopted

- **S1. `slot_grid(columns, items)`** is a grid of square slots, `columns` wide, the items in reading order. The contents are data, `SlotItems(Vec<Option<Slot>>)` on the root: the game writes it when its inventory changes and the grid follows in the same frame (slots are added and removed to match the length; nothing is rebuilt that did not change). A `Slot` is a name, an optional icon, a count and an optional tooltip; `None` is an empty slot.
- **S2. Every slot is a real widget**: Bevy's `Button` (Enter, Space, South and a click arrive as `Activate`), a tab stop with `AutoDirectionalNavigation`, hovered, with the focus ring. The arrows and the D-pad walk the grid with the crate's spatial navigation and leave it from its edges, as from any list of buttons.
- **S3. One tab stop (roving).** Exactly one slot, the cursor, has `TabIndex(0)` and the others `TabIndex(-1)`, so Tab enters the grid on the slot the player last had and leaves it on the next press. `SlotCursor(usize)` is that slot, the grid's state: focus moving to a slot writes it and sends one non-final `ValueChange<usize>` on the grid; a game that writes it sends nothing.
- **S4. Activate is one final `ValueChange<usize>`** on the grid for the slot activated, empty or not (an empty slot may be where the item goes). The game decides what use, equip or place means.
- **S5. Moving is the crate's drag and drop.** A filled slot carries `DragPayload<SlotRef>`, every slot is a `DropTarget<SlotRef>` (`SlotRef { grid, index }`, so a drag may cross grids: backpack to chest). A drop is one `SlotMoved { entity: the target grid, from: SlotRef, to: index }`; `SlotMoved::apply` swaps the two slots' contents in `SlotItems` (a move onto an empty slot is the same swap), across grids too. The pointer, the pick-up key (Alt+M), the pad's West and the steering keys are `carry_by_input`'s, untouched. An emptied slot stops being a source (`DragPayload` removed takes `DragSource` with it, so an empty slot is never lifted).
- **S6. Looks.** A filled slot is a button's fill with its icon (60% of the slot) or, with no icon, its name wrapped and centred; a count above one is a small chip in the bottom-right corner on the opaque surface (`solid_surface`, held 4.5:1 by `problems()`); an empty slot is the panel's fill with its quiet frame. The focused slot wears the focus ring, the pressed one the accent. The slot is `SLOT_TEXT_SIZES` (4) body text sizes square, so it follows the player's text scale (64 px at 100% in the dark skin, over the 44 px target).
- **S7. An icon slot is named by its tooltip**: the tooltip text, else the name, opens on hover and focus (`Tooltip`), because an icon says nothing to a reader and little to a player who has not met the item. A slot with no icon already shows its name and has no tooltip unless the game gives one.

## Parameters

`SLOT_TEXT_SIZES` = 4 (a design default: 64 px at 16 px text, four text sizes square holds a 7 character word wrapped to two lines and an icon of 38 px), icon 60% of the side, the gap between slots half the skin's gap, the count in the third step of the type scale.

## Validation targets

The grid is `columns` slots wide and `ceil(len / columns)` high with every slot square and at least 44 px at 720p, at every `TEXT_SIZE_STEPS` step and no slot leaving the grid; the slots follow `SlotItems` as it grows, shrinks and changes (same entities for the unchanged ones, no write while idle); Tab enters on the cursor and leaves in one press from any slot, and an arrow moves one slot and moves the cursor; Enter, Space, South and a click send one final `ValueChange` with the index, on an empty slot too; a drag from a filled slot to another sends one `SlotMoved` and `apply` swaps, an empty slot cannot be lifted, a drag crosses two grids; the keyboard carry moves an item with no pointer; contrast of the name and count in all four looks; the accessibility tree names each slot with its count.

## Deviations

No Shift-click or right-click split: they are the game's rules and a `Secondary` click is already a `context_menu` trigger (the log's drag work); the widget sends `Activate`-level events only. No per-slot acceptance rule (an equipment slot that takes only helmets): every slot is a `DropTarget<SlotRef>` that takes any `SlotRef`, so the game refuses by not applying the `SlotMoved` (the mark does not say no beforehand: `DropTarget::accepting` takes a plain `fn` with no state and a second `DropTarget<SlotRef>` on the slot would add a second handler, not replace the first). No stack merging, no weight, no rarity colour: tones are the host's (`IconTone`, `Role::Tone`). Design defaults, recorded in the log.

## What great looks like

A tidy block of equal square wells in the panel's own colours: filled ones lifted a step above the panel with their icon centred and a small count chip tucked in a corner, empty ones recessed with a hairline; the focused slot ringed in the focus colour with its tooltip beside it; a drag shows the ghost and the landing mark on the slot the item would go to, and on release the two slots swap in one frame.

## Pending index

The APG and the Minecraft Wiki are web pages: `paper_download` cannot take them; `distill_search` has nothing on inventory interfaces (index *The Gamer's Brain* and the Game Accessibility Guidelines, still pending from `corpus.md`).
