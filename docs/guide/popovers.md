# Popovers

A popover is a panel that opens beside a widget (a menu under its button, a context menu at the pointer,
a combo box's list) and is kept inside the window. `popover(anchor, placement)` is one bundle: spawn it
with its contents and it is placed, styled, closed and cleaned up.

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn open_menu(mut commands: Commands, file_button: Entity) {
    commands.spawn((
        popover_modal(file_button, Placement::Below),
        children![button("New"), button("Open"), button("Quit")],
    ));
}
```

![The popover example: a File menu under its button with the ring on its second item, Escape closing it, Edit pinned to the window's right edge with its menu sliding to end at the button's edge, and a context menu at the pointer that flips up and to the left in the corner](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/popover.gif)

*`examples/popover`: File's menu, Escape, Edit at the window's edge, a right-click near the corner, a press elsewhere.*

## Placement

| `Placement` | opens | flips to | slides |
|---|---|---|---|
| `Below` | under the anchor, left edges together | above | to the anchor's right edge, then centred |
| `Above` | over the anchor | below | likewise |
| `Right` | right of the anchor, top edges together | left | to the anchor's bottom edge, then centred |
| `Left` | left of the anchor | right | likewise |
| `AtPointer` | below and right of the pointer | above and left | to end at the pointer |

The crate hands Bevy's own `Popover` (`bevy_ui_widgets::popover`) a list of positions, the preferred
side first, and Bevy picks the first that fits the window, else the one with least outside it, every
frame; so a resized window places it again and a panel is never drawn off the screen while any side has
room. It sits half a gap off the anchor and a gap off the window's edge. A panel is never wider than the
window less two gaps, so a long line wraps. When no side has room (a panel wider than the window less the
anchor) it takes the least clipped one and the host shortens the content, for instance with a `list`.

The panel is a child of the anchor, so despawning the anchor despawns it. It is laid out against the
window and not inside the anchor (Bevy's `FixedNode`): the anchor's width never wraps its lines and no
scroll clip reaches it. It stands over a `dialog` it was opened from and under a `Tooltip`.

`AtPointer` needs a pointer: the anchor is the widget, panel or world entity (a sprite or mesh) the pointer
was over, and the crate puts a zero-size `PopoverPoint` node at the pointer's position for the panel to
hang from; it goes with the panel. (A world entity cannot be a UI node's parent, so its point is a root of
its own.) For a menu of commands that belongs to a thing, [context menus](context-menus.md) do this opening
for you, with Shift+F10 as well.

```rust
# use bevy::prelude::*;
# use bevy::picking::events::PointerPress;
# use bevy::picking::pointer::PointerButton;
# use bevy_gooey::*;
fn setup(mut commands: Commands) {
    commands.spawn((
        Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        observe(|press: On<PointerPress>, mut commands: Commands| {
            if press.button == PointerButton::Secondary {
                commands.spawn((
                    popover_modal(press.entity, Placement::AtPointer),
                    children![button("Inspect"), button("Delete")],
                ));
            }
        }),
    ));
}
```

## Closing

The player closes a popover and the crate despawns it, after sending one `PopoverClosed { entity, anchor,
reason }` on the popover (observe it there, or globally):

- `CloseReason::Outside`: a pointer press anywhere that is not the panel. A press on the **anchor** is
  not outside for a side placement, so the anchor's own `Activate` can toggle the popover: despawn the
  one whose `PopoverPanel::anchor` is the button, or spawn one (the example's `toggle`). For
  `AtPointer` the anchor is a region, and a press on it closes the popover like a press anywhere else.
  The press that opened a popover, in the frame it opened, is never outside. A press goes on to what it
  landed on: closing a menu by clicking a button clicks the button.
- `CloseReason::Cancel`: Escape or the pad's B, as one `Cancel`. It reaches a modal popover from focus
  inside it, and a non-modal one from focus on its anchor (or inside the anchor). It stops there, so the
  screen under does not also back out. With focus on an unrelated widget Escape is the screen's.

A popover the game despawns, or whose anchor was despawned, sends no event: the game did it. Nested
popovers (a submenu anchored to a button of a menu) close one at a time by a press inside the parent
and all together by a press outside both, each reported once; the ones that go with a parent send
nothing.

A press on the panel itself, its padding or a label, stops at the panel: it never presses or activates
the anchor, which the panel is a child of, and it asks for no focus change. Buttons inside are Bevy's
own and activate as usual.

## Modal or not

`popover_modal` holds focus, as a menu does (WAI-ARIA APG, *Menu Button*): focus moves into it when it
opens (to the first widget or the one with `AutoFocus`), Tab, the arrows and the D-pad stay inside, and
closing it gives focus back to the widget that had it. It is a `Dialog` and a modal `TabGroup`
without the scrim.

`popover` leaves focus where it is. Use it for what the player reads or what a field drives (a combo
box's list under its text field). Its contents are in the normal tab order, and Escape closes it from
the anchor.

## What it does not do

It is the primitive under menus, not a menu: no roving highlight, type-ahead or submenu opening on a
key (a menu bar and `menu` items are the next widgets built on it). It does not close on a window blur
or on scroll, and it does not keep a panel inside the window by overlapping its anchor when no side has
room.

## What a host touches

`popover`, `popover_modal`, `Placement`, `PopoverPanel` (query it to find the popover open for an
anchor), `PopoverClosed` and `CloseReason`, `PopoverPoint`, and the systems `close_popovers_on_press`,
`anchor_pointer_popovers` and `drop_pointer_anchors` in `GooeySystems::Navigate`. The look is
`Role::Popover`: an opaque `solid_surface` card, a frame in `outline` (3:1 on the card in every preset,
like a dialog's), a shadow, padded like a panel; skin it with the rest of the skin.
