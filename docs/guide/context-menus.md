# Context menus

A context menu is the menu of a thing: right-click a crate in the world, a slot in an inventory or a row of
a list and the commands for that thing open at the pointer; press Shift+F10 (or the Menu key) with focus on
it and they open below it. Spawn the menu once, name it on the things, and observe its items where you
wrote them.

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let menu = commands
        .spawn(context_menu(children![
            (
                menu_item("Delete"),
                observe(
                    |activate: On<Activate>,
                     parents: Query<&ChildOf>,
                     targets: Query<&ContextTarget>,
                     mut commands: Commands| {
                        if let Some(thing) = ContextTarget::of(activate.entity, &parents, &targets) {
                            commands.entity(thing).despawn();
                        }
                    }
                ),
            ),
            separator(),
            menu_item("Rename").with_shortcut(Shortcut::new(Modifiers::NONE, KeyCode::F2)),
        ]))
        .id();
    // A thing in the world (a sprite or a mesh is picked only with `Pickable`) ...
    commands.spawn((
        Sprite::from_color(Color::WHITE, Vec2::splat(64.0)),
        Pickable::default(),
        ContextMenu(menu),
    ));
    // ... and a widget.
    commands.spawn((button("Row"), ContextMenu(menu)));
}
```

![The context example: a right-click on a crate opens its menu at the pointer; Locked is ticked and Inspect's chord I stands at the right; choosing Locked says "Toggled lock on: Crate B" at the bottom; a right-click on the crate in the bottom-right corner opens the whole menu above and to the left of the pointer; Down then Shift+F10 opens the inventory row's menu below the row, Right on Move to opens its submenu, and Enter on Chest 2 says which row was moved](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/context-menu.gif)

*`examples/context`: a right-click on a crate, the menu flipping at the window's corner, Shift+F10 on an inventory row, a submenu, and the line at the bottom saying which thing a command was for.*

## What opens it

| | |
|---|---|
| A secondary click (right button) that reaches a thing with a `ContextMenu` | opens its menu with its corner at the pointer, flipped up or to the left at the window's edge |
| Shift+F10, or the Menu key, with focus on a thing or a widget under it | opens the menu of the nearest thing that has one, below the focused widget (above it when there is no room below) |
| A second secondary click, on the same thing or on another | moves the menu there; one context menu is open at a time |

The click that reaches the *nearest* thing above what was picked is the one used and goes no further, so a
sprite's child, or a label in a row, opens its parent's menu. Opening is a menu's, which is a popover's: focus
goes to the first item, Enter chooses (Escape, the pad's B, a press outside close it) and focus goes back to
where it was; the menu is a modal popover while it is open, drawn nowhere and in no focus scope while it
is not.

## What it knows

`ContextMenu(menu)` goes on any entity: a UI node, a sprite, a mesh. One menu can serve many things (every
row of a list); while it is open its panel carries `ContextTarget(thing)`, the entity that had the `ContextMenu`
(not the widget a key opened it beside). Read it in an item's `Activate` observer with `ContextTarget::of(item,
&parents, &targets)`, or in an `On<Add<ContextTarget>>` observer, which runs before the menu is first drawn,
to disable or relabel items for this thing (`Disabled::new(reason)` on an item says why, as in
[menus](menus.md)).

## Notes

- **Bevy's `Button` takes any pointer button.** A right-click on a `button` also sends it `Activate`. Put the
  `ContextMenu` on a row whose own click you do not mind, or on a container of the buttons.
- A sprite or mesh is picked only with `Pickable`; a menu entity the game has despawned opens nothing.
- The menu is a root, not a child of the thing, so it is laid out whatever the thing is and one copy serves
  many; despawn it yourself when the things it serves are gone.
- A long menu scrolls and follows focus, like a menu bar's.
- There is no long-press (touch) trigger; a game that has one sends the secondary `PointerClick` itself.

See [menus](menus.md) for the items, [popovers](popovers.md) for the placement and closing rules, and
[shortcuts](shortcuts.md) for the chord on an item.
