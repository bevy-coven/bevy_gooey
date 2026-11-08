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

![The workspace example: Wall is dragged from the Library onto the viewport, which takes the ring's colour as it can take it, and an orange square stands where it was let go; Door goes to the upper right; Wall over the Lamp slot lights it red with a red-framed ghost and places nothing; Wall over Trash removes the last piece, the green Door, leaving one square](https://raw.githubusercontent.com/dark-incorporated/bevy_gooey/main/docs/media/drag-drop.gif)

- **The type is the match.** A `DropTarget<B>` refuses a `DragPayload<A>`; `DropTarget::accepting(|piece| piece.0 < 10)` refuses a payload the target's own test says no to. A node may carry several `DropTarget`s, one for each type it takes.
- **What a release does.** Over a target that takes it: one `Dropped<T> { target, source, payload, pointer }` on the target. Over a target that does not: one `DropRefused<T>` on it. Over nothing the interface draws (the empty part of a `screen()`, a sprite or mesh with `Pickable`): one global `DroppedOnWorld<T> { source, payload, pointer }`. Over any other interface node (a panel that is not a target): nothing. `pointer` is the window position in logical pixels, what `Camera::viewport_to_world` takes.
- **The player sees it before letting go.** A ghost with the source's words (`DragLabel`, or its first `Text`) stands beside the pointer, and the target under it gets a mark: the ring's colour when it takes the payload, `skin.danger` (ghost frame too) when it does not. `Role::{DragGhost, DragGhostRefused, DropMark, DropMarkRefused}`.
- **The places that take it are marked at pick-up.** While a payload is lifted (pointer, keyboard or pad) every enabled, visible target that takes it carries `DropCandidate` and every one that refuses it `DropRefusing`; the source and a node that is no target carry neither, and both come off when the drag ends. They are the same test the drop and the keyboard steering use, so a candidate is a place the carry can land. The crate draws them for an [inventory slot](inventory.md) (a taker is framed in the skin's outline, a refuser is dimmed) and nothing else: style your own targets with `Has<DropCandidate>` / `Has<DropRefusing>`.
- **Escape or a pad's East puts it down**: nothing is sent, the ghost goes, and that press is no `Cancel`. A press that moves under 6 px (`DRAG_LIFT_PX`) is a click, not a drag.
- **State.** `Dragging` is on the source exactly while it is carried; `DragState` is the one drag in progress (`source()`, `over()`, `refused()`); `is_dragging` is a run condition for the game's own systems.
- **Dragging is never the only way** (WCAG 2.2, 2.5.7 Dragging Movements), so the crate carries a payload without a pointer too; see the next section. A `ContextMenu` on the same row with "Move to..." items is still the single-pointer alternative the criterion names.
- A source that is a `button` is activated by a click as before, and by a drag that ends on itself. A dock's own tab drag is separate (`DockLayout::moved`, `DockDropTarget`).

## Without a pointer

Focus a source (Tab, the arrows, the D-pad) and press **Alt+M** (a pad's **West**, X on an Xbox pad). The same ghost and `Dragging` appear, nothing is sent yet, and focus stays on the source.

![The workspace example with the keyboard only: F6 puts the ring on Wall, Alt+M lifts it (a ghost "Wall" beside the ring), Right marks the viewport, which lights in the ring's colour, and Alt+M again drops it, an orange square standing in the viewport's middle; Door is lifted, Right marks the viewport and Down moves the mark to Trash, and dropping it there removes the orange square; Lamp is lifted, Right marks the Lamp slot, the only target that takes it, and the slot's words turn to "Lamp slot: lit"](https://raw.githubusercontent.com/dark-incorporated/bevy_gooey/main/docs/media/drag-keys.gif)

- **Steer.** The arrows or the D-pad move the mark (the same frame and ghost a pointer's drag draws, the ghost standing beside the marked target) to the nearest target that takes the payload in that direction (`target_toward`: the distance along the direction plus twice the distance across it). PageUp, PageDown, LB and RB step through the targets in reading order and wrap (`target_step`). Only targets that take the payload, are visible and are not disabled are offered: a keyboard player is never shown a refusal, so no `DropRefused` is sent from a key.
- **Drop.** The pick-up key again (a pad's South or West) sends one `Dropped<T>` on the marked target, `pointer` its middle, so a handler needs no second path. With nothing marked it only puts the payload down. Escape and East put it down as for a pointer, with no `Cancel`.
- **The pick-up key is not Enter, Space or South**, because Bevy's `Button` activates on those and a source that is a `button` would be pressed and lifted at once; the drop is the same key for the same reason. Alt+M is the default; `DragSettings { pick_up: Shortcut, pick_up_button: GamepadButton }` is a serde resource, so a game's keymap changes it (`Shortcut::new(Modifiers::NONE, KeyCode::F2)`).
- **Nothing else hears the keys while a payload is carried** (`not_capturing` is false, so `navigate_by_input`, `step_tabs`, `gamepad_select`, `page_lists` and the rest stand still): the arrow that steers never also moves focus. `DragState::carried_by_key()` says so. A world drop has no keyboard form (there is no pointer to put in the world): make the viewport a `DropTarget` and read where the player dropped from `Dropped::pointer`.
- A pointer's drag that is in flight is not disturbed by these keys, and a pointer press cannot start a second drag while a payload is carried.
