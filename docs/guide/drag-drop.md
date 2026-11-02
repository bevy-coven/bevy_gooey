# Drag and drop

A source carries a typed value, a target takes it, and a drop is one event. Nothing here moves a widget: the game does, in the observer of the drop.

```rust
#[derive(Clone)]
struct Piece(u32);

fn setup(mut commands: Commands) {
    // A source: any pickable node. Pressing it and moving 6 px lifts the payload.
    commands.spawn((button("Wall"), DragPayload(Piece(7))));

    // A target: any node. One `Dropped<Piece>` per drop on it.
    commands
        .spawn((panel(), DropTarget::<Piece>::new()))
        .observe(|dropped: On<Dropped<Piece>>| {
            info!("piece {} dropped at {}", dropped.payload.0, dropped.pointer);
        });

    // The game world under a HUD: the pointer goes to the host's raycast.
    commands.add_observer(|world: On<DroppedOnWorld<Piece>>| {
        info!("place piece {} at {}", world.payload.0, world.pointer);
    });
}
```

![The workspace example: Wall is dragged from the Library onto the viewport, which takes the ring's colour as it can take it, and an orange square stands where it was let go; Door goes to the upper right; Wall over the Lamp slot lights it red with a red-framed ghost and places nothing; Wall over Trash removes the last piece, the green Door, leaving one square](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/drag-drop.gif)

- **The type is the match.** A `DropTarget<B>` refuses a `DragPayload<A>`; `DropTarget::accepting(|piece| piece.0 < 10)` refuses a payload the target's own test says no to. A node may carry several `DropTarget`s, one for each type it takes.
- **What a release does.** Over a target that takes it: one `Dropped<T> { target, source, payload, pointer }` on the target. Over a target that does not: one `DropRefused<T>` on it. Over nothing the interface draws (the empty part of a `screen()`, a sprite or mesh with `Pickable`): one global `DroppedOnWorld<T> { source, payload, pointer }`. Over any other interface node (a panel that is not a target): nothing. `pointer` is the window position in logical pixels, what `Camera::viewport_to_world` takes.
- **The player sees it before letting go.** A ghost with the source's words (`DragLabel`, or its first `Text`) stands beside the pointer, and the target under it gets a mark: the ring's colour when it takes the payload, `skin.danger` (ghost frame too) when it does not. `Role::{DragGhost, DragGhostRefused, DropMark, DropMarkRefused}`.
- **Escape or a pad's East puts it down**: nothing is sent, the ghost goes, and that press is no `Cancel`. A press that moves under 6 px (`DRAG_LIFT_PX`) is a click, not a drag.
- **State.** `Dragging` is on the source exactly while it is carried; `DragState` is the one drag in progress (`source()`, `over()`, `refused()`); `is_dragging` is a run condition for the game's own systems.
- **Dragging is never the only way** (WCAG 2.2, 2.5.7 Dragging Movements). Offer the same move as a command: a `ContextMenu` on the same row with "Move to..." items is the single-pointer alternative the criterion asks for.
- A source that is a `button` is activated by a click as before, and by a drag that ends on itself. A dock's own tab drag is separate (`DockLayout::moved`, `DockDropTarget`).
