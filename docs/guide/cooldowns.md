# Cooldowns

An ability that is not ready yet, a reload, a potion with a delay: `Cooldown` on any widget dims it and
sweeps it clear clockwise from twelve o'clock, with the seconds left on a chip in the middle. The crate
counts the time down; the game starts it and hears when it ends.

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn hotbar(mut commands: Commands) {
    commands
        .spawn((
            button("Fireball"),
            Cooldown::default(), // ready: nothing is drawn
        ))
        .observe(|activate: On<Activate>, mut cooldowns: Query<&mut Cooldown>| {
            if let Ok(mut cooldown) = cooldowns.get_mut(activate.entity)
                && cooldown.is_ready()
            {
                cooldown.start(8.0); // cast it
            }
        })
        .observe(|ready: On<CooldownReady>| info!("{} is ready: play the sound", ready.entity));
}
```

![The hotbar example: four 80 px slots with a torch, a gem, an arrow and a flask. Fireball, Frost nova, Dash and Heal are cast one after another; each slot dims, a clear wedge opens clockwise from twelve o'clock with a thin light hand at its edge, the seconds left (whole seconds, then tenths in the last three) stand on a small dark chip in the middle, and the slot is plain again when its time is up, the line under the bar reading "Dash ready", "Frost nova ready"](https://raw.githubusercontent.com/dark-incorporated/bevy_gooey/main/docs/media/cooldown.gif)

*`examples/hotbar`: four abilities in a `slot_grid`, each cell carrying a `Cooldown`.*

## What it does

- **Runs on game time.** `Cooldown { duration, remaining }` is counted down by `Time`, so a paused game
  holds it and a slowed one slows it. A game that keeps its own clock writes `remaining` itself.
- **Says when it ends.** The frame `remaining` reaches zero the widget gets one `CooldownReady`
  (`EntityEvent`): the place for the "ready" sound, so the news is not only visual. Setting `remaining`
  to zero yourself sends nothing.
- **Reads at a glance and in numbers.** The covered part is the skin's `background` at 66%, a thin hand
  marks the sweep's edge, and the seconds are whole numbers from 3 s up, `m:ss` from a minute, tenths under
  3 s (`model::cooldown_text`). The chip is held to 4.5:1 over the brightest and the darkest icon by
  `GooeySkin::problems()` in every preset. Nothing depends on hue.
- **Costs nothing in layout.** The cover is an absolute child on the widget's padding box with the widget's
  corner radius; it is hidden while ready and lets the pointer through. A widget with a ready `Cooldown`
  costs one query check a frame.
- **Is in the accessible name.** A running cooldown makes the widget's name "Fireball, 6 seconds
  remaining" (a host's `AccessibleLabel` included), and it is "Fireball" again when ready.
- **Follows the skin.** A `SkinScope` on or above the widget changes the cover and the chip with it.

It does not stop the widget being pressed: a button on cooldown still sends `Activate`, so the game
decides what a press means (`Cooldown::is_ready`; the hotbar example says "Dash is not ready").

## On a slot grid

A hotbar is a `slot_grid`. Its cells are spawned by the grid, so insert the component when they appear:

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn arm_cells(mut commands: Commands, cells: Query<Entity, Added<SlotRef>>) {
    for cell in &cells {
        commands.entity(cell).insert(Cooldown::default());
    }
}
```

`SlotRef::index` says which slot a cell is, and the grid's final `ValueChange<usize>` says which one was
used. A cooldown belongs to the cell, not the item: a game that lets the player reorder the bar moves the
timer with the item.

Research: [`docs/research/cooldown.md`](https://github.com/dark-incorporated/bevy_gooey/blob/main/docs/research/cooldown.md).
