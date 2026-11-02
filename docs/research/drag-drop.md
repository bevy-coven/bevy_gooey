# Drag and drop with payloads

Researched 2026-11-02 (iteration 40, #26 G-22). home-still's index has nothing on drag and drop for interfaces, so the sources below were fetched today.

## Sources

1. W3C, *Understanding SC 2.5.7 Dragging Movements*, https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html — craft (accessibility).
2. Dear ImGui, `imgui.h`, "Drag and Drop" (`BeginDragDropSource`, `SetDragDropPayload`, `BeginDragDropTarget`, `AcceptDragDropPayload`), https://raw.githubusercontent.com/ocornut/imgui/master/imgui.h — shipping tool library (it ships in most game tools).
3. Bevy 0.20 picking (`PointerDragStart`, `PointerDrag`, `PointerDragEnd`, `HoverMap`, `PointerLocation`) and this crate's own dock tab drag (`src/dock.rs`: ghost, landing overlay, Escape) — technical.

## Craft principle

WCAG 2.5.7: "All functionality that uses a dragging movement for operation can be achieved by a single pointer without dragging, unless dragging is essential". Its examples are the ones a tool has: "A sortable list of elements may, after tapping or clicking on a list element, provide adjacent controls for moving the element up or down" and "A task board … also provides an additional pop-up menu after tapping or clicking on items for moving the selected element to another column".

Dragging is therefore a shortcut and never the only way. The crate offers the drag; the single-pointer alternative is a command the host already has (a context menu on the same row, `ContextMenu`).

## How shipping games do it

Dear ImGui's drag and drop is the model every Bevy and Rust tool also copies: "On source items, call BeginDragDropSource(), if it returns true also call SetDragDropPayload() + EndDragDropSource(). On target candidates, call BeginDragDropTarget(), if it returns true also call AcceptDragDropPayload() + EndDragDropTarget()." The payload is typed: "`type` is a user defined string of maximum 32 characters", and a target accepts "contents of a given type", so a target never sees a payload it did not ask for. "If ImGuiDragDropFlags_AcceptBeforeDelivery is set you can peek into the payload before the mouse button is released", which is how a target shows that it will accept before the drop.

## Model adopted

- **D1. A source is any `Pickable` node with `DragPayload<T>(pub T)`** (`T: Clone + Send + Sync + 'static`). A press and drag past Bevy's drag threshold starts it. Its hook adds the source's observers, so there is no per-type plugin to add. The ghost's words are the source's `DragLabel` or else its first descendant `Text`.
- **D2. A target is any node with `DropTarget<T>`** (which requires the private marker `DropZone`). `DropTarget::new()` takes every `T`; `DropTarget::accepting(fn(&T) -> bool)` asks the payload. The type is the match, as ImGui's `type` string is: a `DropTarget<B>` is a `DropZone` that refuses a `DragPayload<A>`.
- **D3. A release over a target that accepts sends one `Dropped<T> { target, payload, pointer }`** (an `EntityEvent` on the target, `pointer` the window position in logical pixels). A release over no UI node at all (the HUD's empty space, the world) sends one global `DroppedOnWorld<T> { payload, pointer }`, the position a host passes to `Camera::viewport_to_world`. A release over a node that is not a target (a panel) sends nothing: the ghost goes, nothing moves.
- **D4. Refusal is visible while it happens.** The node under the pointer that is a `DropZone` gets a landing mark (`Role::DropMark`, a frame on the target's rect): `accent` when it accepts, `danger` when the payload is the wrong type or `accepting` says no. The ghost's frame follows the same colour. Releasing over a refusing target sends `DropRefused<T> { target, payload }` (once) and nothing else.
- **D5. Escape cancels** (a `Cancel` that reaches the source, or the key directly while a drag exists): the ghost goes and nothing is sent. `Dragging` is on the source exactly while it is dragged.
- **D6. The ghost and the mark** are root nodes above everything (`GlobalZIndex`), `Pickable::IGNORE`, spawned for the drag and despawned after, like the dock's.

## Parameters

Ghost: one body text size card, `surface_raised` fill and the `accent` or `danger` frame, 80% opaque. Mark: frame one `focus_ring` weight wide inside the target's rect. Z order 900 (under tooltips' 1000).

## Validation targets

Drag a source onto an accepting target: one `Dropped`, none other. Release over empty space: one `DroppedOnWorld` with the pointer position. Over a target of another type: `DropRefused`, danger mark while hovering. `accepting` returning false refuses the same. Escape mid-drag sends nothing and removes the ghost. A panel under the pointer sends nothing. A drag never also clicks the source. Nothing written while idle. The ghost's words 4.5:1 on its fill in the four looks.

## Deviations

`Dropped` carries `pointer` besides `target` and `payload` (the issue's fields): a drop onto a viewport needs the position. Design default. `DropRefused` and `Dragging` are additions. The keyboard alternative was left to the host (WCAG 2.5.7 asks for a single-pointer one; a context menu is one): assumed decision, recorded on the issue; **superseded in iteration 44**: the crate carries a payload from the keyboard and a pad itself, see `drag-drop-keyboard.md`.

## What great looks like

A row lifted off its list by a ghost that sits a few pixels off the pointer, the one frame on a target going accent as it can take it and danger as it cannot, no flicker between neighbouring rows.

## Pending index

All three sources, in home-still.
