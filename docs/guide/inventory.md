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

![The inventory example: a Backpack of five columns, a Chest of four and an Equipment row of three in one panel, slots with white-on-transparent icons (a sword, a potion with a count of 3, arrows with 24, a ruby, a key, a torch, gold, a map), empty slots as dark wells with a hairline, two text-only slots (Bread, Rope); the ring walks the backpack and tooltips open (Health potion, Arrows, Torch); Enter reads "Used Torch"; Alt+M lifts the torch, the slots that take it get a light frame (the Equipment row's weapon and charm slots, which refuse it, stay dark), the arrows steer the mark through the backpack into the chest, Alt+M drops it and the panel reads "Moved Torch"; the ring went with the torch and two Lefts put it on the gold](https://raw.githubusercontent.com/dark-incorporated/bevy_gooey/main/docs/media/inventory.gif)

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

A drop is one `SlotMoved { entity: the grid it landed in, from: SlotRef, to: index }`. The grid changes nothing itself. `moved.apply(&mut grids)` swaps the two slots' contents (onto an empty slot that is a move), across two grids when the item came from another, and says whether anything changed. A game with more rules than "which slot takes which item" (see [Rules](#rules)) does its own and writes the result into `SlotItems`: stacks that merge, a full chest, a weight limit.

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

## Rules

`SlotRules` on the grid says which items its slots take: `accepts(item, index)` is whether slot `index` may hold `item`. It is a plain `fn` (a closure that captures nothing), so it is data you write once. `Slot::kind("helmet")` is a free label for it to read; the crate gives it no meaning.

```rust
commands.spawn((
    slot_grid(3, [None, None, None]),
    // An equipment panel: a helmet, a weapon, and a slot that is locked.
    SlotRules::new(|item, slot| match slot {
        0 => item.kind.as_deref() == Some("helmet"),
        1 => item.kind.as_deref() == Some("weapon"),
        _ => false,
    }),
))
.observe(|refused: On<SlotRefused>| warn!("slot {} will not take that", refused.to));
```

![The inventory example with an Equipment row: the pointer lifts the Iron sword, every slot that takes it is framed in a light outline and the torch slot, which refuses it, is dimmed; over the torch slot the landing mark and the ghost turn red and the line below reads "Iron sword does not fit there"; it is let go there and nothing moves; the second drag puts the sword in the weapon slot, "Moved Iron sword" and the frames are gone.](https://raw.githubusercontent.com/dark-incorporated/bevy_gooey/main/docs/media/slot-rules.gif)

*`examples/inventory` with `GOOEY_KEYS=drag:b0>e1,drag:b0>e0`: the sword is lifted and the slots that take it are framed; over the torch slot it is refused while it is still in the hand, then it goes to the weapon slot.*

- **The slot says no before the player lets go.** Over a slot that will not take the carried item the drop mark and the ghost take their refused look (red), as over any refusing [drop target](drag-drop.md); a release sends one `SlotRefused { entity: the grid, from, to }` (play your "no" sound there) and moves nothing. A keyboard or pad carry never lands on such a slot: the mark skips it.
- **The slots that take the item say so at pick-up.** The moment an item is lifted (pointer, Alt+M or a pad) every slot that would take it, in any grid, is framed in the skin's `outline` (the frame the hover outline uses, held 3:1 against the panel in every preset), and a slot that refuses it is dimmed: a filled one becomes the panel's well with `text_muted` words and icon, an empty one keeps its quiet frame beside the lit ones. With no rule everything takes the item and the backpack only gets a calm frame; in an equipment panel the one slot that fits is the one that stands out. They are [`DropCandidate` and `DropRefusing`](drag-drop.md) on each slot while the drag lasts, and they come off the frame it ends.
- **A move is allowed when both ends are.** A filled target's item goes back to where the carried one came from, so that slot's grid must take it there too (a helmet cannot be swapped for a sword). A drop across two grids obeys both grids' rules.
- **A locked slot is a rule that takes nothing** (`|_, i| i != 3`); its item can still be taken out into a slot that takes it.
- **Change the rule by writing the component again**; the next drag uses it. Do not remove it.

After a drop the focus follows the item: if the slot it came from had focus (a keyboard carry, or the click that began a drag), the ring is on the slot it went to, and the grid's tab stop with it.

## Looks

A filled slot is a button's fill; an empty one is the panel's own fill with its quiet frame, so the grid reads as wells. The focused slot wears the focus ring; the pressed one the accent; while an item is carried a slot that takes it wears the `outline` and one that refuses it is dimmed. Every colour is the skin's and the count's words are held to 4.5:1 on its chip in every preset.

## Accessibility

Each slot is a button named by its item ("Health potion") with its count as its value ("24"), and an empty slot is "Empty slot". See [accessibility](accessibility.md).

## Not here

Shift-click to transfer, right-click to split a stack, merging, weight and rarity colours are the game's rules; the events above are enough to write them (a right-click is a [context menu](context-menus.md) trigger).
