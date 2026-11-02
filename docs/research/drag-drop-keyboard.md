# Carrying a payload without a pointer

Researched 2026-11-02 (iteration 44). Extends `drag-drop.md`, whose deviation "the keyboard alternative is the host's" is replaced: a keyboard or gamepad player lifts, steers and drops a payload with the crate's own widgets. home-still's index has nothing on keyboard drag and drop for interfaces (a `distill_search` returned only a pointing-device study), so the sources below were fetched today.

## Sources

1. W3C, *Understanding SC 2.5.7 Dragging Movements*, https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html, and Game Accessibility Guidelines, *Ensure that all areas of the user interface can be accessed using the same input method as the gameplay*, https://gameaccessibilityguidelines.com/ — craft (accessibility).
2. Capcom, *Resident Evil 4 Official Web Manual*, "Items", https://game.capcom.com/manual/re4/en/ps5/page/15/7 — shipping game (a grid inventory with a controller).
3. dnd kit, *Keyboard sensor*, https://dndkit.com/legacy/api-documentation/sensors/keyboard/ — technical (the keyboard model every web drag-and-drop library copies).

## Craft principle

Game Accessibility Guidelines: "If I can perform 90% of in-game actions using ONLY the mouse, don't make me pick up a keyboard for the remaining 10%." Its rule for the interface is "ensure that all menus also work for both keyboard and joypad". The previous note answered WCAG 2.5.7 ("All functionality that uses a dragging movement for operation can be achieved by a single pointer without dragging") with a command the host writes; that is a second implementation of the same move for every game. A game that has an inventory with drag and drop has to write its pad path, a keyboard path and a menu path. The crate can make the move itself once, since it already knows the source, the typed targets and where each target is.

## How shipping games do it

Resident Evil 4's attaché case is the reference for a grid inventory on a pad: "Moving items: Aim the laser pointer at the item and select it with [the select button], then move it to the desired location and press [the same button] again." The prompt table beside it gives the same three steps on every device: select (X, A or F), rotate (L1/R1, LB/RB or Q/E), and the second press of select places. The shape: **a press lifts, the directional input moves the carried item, a press of the same button puts it down.** Nothing in it needs a pointer.

## Model adopted

dnd kit's keyboard sensor has the same three phases as its default: "By default, the keys that activate the Keyboard sensor are `Space` and `Enter`", "cancel: ['Escape']", "end: ['Space', 'Enter']", the arrows moving the item, and the activator "**must** be able to receive focus".

- **K1. A focused `DragPayload<T>` source is lifted by a pick-up key** (`DragSettings::pick_up`, default Alt+M; a pad's West, `DragSettings::pick_up_button`). It is not Enter, Space or South: Bevy's `Button` activates on those, and a source that is a `button` would be both activated and lifted. The state is the pointer drag's (`DragState`, `Dragging`, the ghost and the mark), with `keyboard` set: one path for what the player sees and for what the game hears.
- **K2. While carried, the arrows and the D-pad move the mark to the nearest target in that direction** (`model::target_toward`), PageUp, PageDown and LB, RB step through the targets in reading order (`model::target_step`, wrapping). Only targets that take the payload are candidates; a keyboard player is never offered a refusal. The source's own node is not a candidate.
- **K3. Enter, Space, South and the pick-up key again drop** (RE4's "press the same button again"): one `Dropped<T>` on the mark's target, `pointer` its centre. With no target marked nothing happens. Escape and East put it down as for a pointer. A drop in the game world has no keyboard form (a viewport that takes drops is a `DropTarget`).
- **K4. The rest of the crate's input stands still while a payload is carried**: `not_capturing` is false, so the arrow that moves the mark never also moves focus, Back is not sent, a tab does not turn.

## Parameters

`DragSettings { pick_up: Shortcut::new(Modifiers::ALT, KeyCode::KeyM), pick_up_button: GamepadButton::West }`, serde. A direction is a target whose centre lies strictly on that side; the score is the distance along the direction plus twice the distance across it (the usual spatial-navigation weight), the lowest wins, ties by reading order.

## Validation targets

Pick-up on a focused source: `Dragging` on it, one ghost and one mark, nothing sent. Arrow: the mark on the nearest accepting target that way, none past the last. Enter: one `Dropped` with the payload and no other event, `Dragging` gone, the ghost gone. Escape: nothing sent. A target of another type, or an `accepting` that says no, is never marked. The arrows move no focus and send no `Cancel`. The pad does the same with West, the D-pad and South. A button source is not also activated by the pick-up. Nothing written while idle.

## Deviations

The pick-up key is not Space or Enter, against dnd kit's default (and its note that the third rule of ARIA wants both): a gooey source is often a `button`, whose Enter and Space are Bevy's. Assumed decision, recorded in the log. A refused target is never reachable by key (a pointer sees it as a danger mark): design default.

## What great looks like

The same ghost and ring a pointer drag draws, now stepping from target to target with the arrows without a jump, the lifted source dimmed by the host through `Dragging`, one more key to drop.

## Pending index

All three sources, in home-still.
