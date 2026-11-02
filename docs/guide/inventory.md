# Inventory: the slot grid

`slot_grid(columns, items)` is a backpack, a hotbar, a chest, a shop shelf or a crafting bench: a grid of square slots, each holding an item or empty. The contents are plain data, `SlotItems`; change it and the grid follows in the same frame.

```rust
fn backpack(mut commands: Commands) {
    commands
        .spawn(slot_grid(
            5,
            [Some(Slot::new("Sword")), Some(Slot::new("Arrows").count(24)), None, None, None],
        ))
        // The player dropped an item on a slot: swap the two.
        .observe(|moved: On<SlotMoved>, mut grids: Query<&mut SlotItems>| {
            moved.apply(&mut grids);
        })
        // The player chose a slot: Enter, Space, South or a click.
        .observe(|chosen: On<ValueChange<usize>>| {
            if chosen.is_final {
                info!("use slot {}", chosen.value);
            }
        });
}
```

![The inventory example: a Backpack of five columns and a Chest of four in one panel, slots with white-on-transparent icons (a sword, a potion with a count of 3, arrows with 24, a ruby, a key, a torch, gold, a map), empty slots as dark wells with a hairline, two text-only slots (Bread, Rope); the ring walks the backpack and tooltips open (Health potion, Arrows, Torch); Enter reads "Used Torch"; Alt+M lifts the torch, the arrows steer the mark through the backpack into the chest, Alt+M drops it and the panel reads "Moved Torch"; the ring is back in the backpack on the Cellar key](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/inventory.gif)

*`examples/inventory`: Tab enters the backpack, the arrows walk it (tooltips follow the ring), Enter uses the torch, Alt+M lifts it, the arrows steer it across to the chest and Alt+M drops it.*

## The contents

`SlotItems(Vec<Option<Slot>>)` on the grid's root, in reading order: `Some` is an item, `None` an empty slot. Push, remove or write a slot and the grid adds, removes and updates slots to match; a slot whose item did not change is not touched, so an inventory that changes every frame costs only what changed. A game whose inventory lives somewhere else writes `SlotItems` when that changes.

A `Slot` is built with `Slot::new("Health potion")` and the chain of `.count(3)`, `.icon(IconSource)` and `.tooltip("Restores 40 health")`:

- **name**: drawn in the slot while it has no icon (wrapped, centred, a short word reads best), read by a screen reader.
- **icon**: your art, white on transparent, 60% of the slot and tinted by the skin like every `icon()`. See [icons](icons.md).
- **count**: a number in a chip in the bottom-right corner above one (on the opaque surface, so it reads over any art).
- **tooltip**: shown on hover and focus. An icon slot that was given none shows its name, since a picture alone says little; a slot with no icon already shows its name and has none.

A slot is `text_size` times four square (80 px in the dark look at 100% text, 160 px at 200%), so the grid follows the player's text size; size your columns for the largest step, or wrap the grid in a `list` to scroll it.

## Walking it

Every slot is a real widget (Bevy's `Button`, with a focus ring and a tooltip), and **the grid is one tab stop**: `SlotCursor(usize)` on the root is the slot Tab enters on (the one the player last had focus on), and Tab leaves on the next press instead of walking forty slots.

| Input | What it does |
| --- | --- |
| the arrows, the D-pad | one slot that way; from an edge, on to the widget beyond the grid |
| Home, End | the first and last slot of the row |
| Ctrl+Home, Ctrl+End | the first and last slot of the grid |
| Enter, Space, South, a click | choose the slot, **empty or not** (an empty slot may be where the item goes) |
| Tab, Shift+Tab | into and out of the grid |

Focus moving to a slot sends one `ValueChange<usize>` on the grid with `is_final: false` (preview the item in a details panel); choosing sends one with `is_final: true`. Write `SlotCursor` yourself to move the tab stop from code (no event is sent for your own write; a value past the end is pulled back to the last slot).

## Moving items

Moving an item is the crate's own [drag and drop](drag-drop.md): every filled slot carries `DragPayload<SlotRef>` and every slot is a `DropTarget<SlotRef>` (`SlotRef { grid, index }` says which slot of which grid). So:

- the **pointer** drags a slot onto any slot, of this grid or another (backpack to chest);
- the **keyboard and the gamepad** lift, steer and drop with no pointer: Alt+M (a pad's West) lifts the focused item, the arrows steer the mark, Alt+M or South drops it, Escape puts it back ([drag and drop without a pointer](drag-drop.md#without-a-pointer));
- an emptied slot is not a source, so pressing on an empty slot lifts nothing.

A drop is one `SlotMoved { entity: the grid it landed in, from: SlotRef, to: index }`. The grid changes nothing itself. `moved.apply(&mut grids)` swaps the two slots' contents (onto an empty slot that is a move), across two grids when the item came from another, and says whether anything changed. A game with rules does its own and writes the result into `SlotItems`: stacks that merge, a slot that takes only helmets, a full chest.

```rust
.observe(|moved: On<SlotMoved>, mut grids: Query<&mut SlotItems>| {
    let Ok(items) = grids.get(moved.from.grid) else { return };
    let Some(Some(item)) = items.0.get(moved.from.index) else { return };
    if item.name == "Cursed ring" && moved.entity != moved.from.grid {
        return; // it cannot leave the backpack
    }
    moved.apply(&mut grids);
})
```

A move onto the slot it came from sends nothing.

## Looks

A filled slot is a button's fill; an empty one is the panel's own fill with its quiet frame, so the grid reads as wells. The focused slot wears the focus ring; the pressed one the accent. Every colour is the skin's and the count's words are held to 4.5:1 on its chip in every preset.

## Accessibility

Each slot is a button named by its item ("Health potion") with its count as its value ("24"), and an empty slot is "Empty slot". See [accessibility](accessibility.md).

## Not here

Shift-click to transfer, right-click to split a stack, merging, weight and rarity colours are the game's rules; the events above are enough to write them (a right-click is a [context menu](context-menus.md) trigger). Per-slot acceptance (an equipment slot that takes only helmets) is declining to apply the `SlotMoved`; the drop mark does not say no before the player lets go, because every slot is a target.
