# Menus

A menu bar is a row of titles, each opening a card of commands: the first place a new user of a tool
looks for a command and learns its shortcut. `menu_bar(children![menu_title(..)])` is one bundle; the
menu cards are spawned with their titles and shown by opening them, so an `observe` on an item stays where
you wrote it.

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn(menu_bar(children![
        menu_title(
            "File",
            children![
                (
                    menu_item("Open...").with_shortcut(Shortcut::primary(KeyCode::KeyO)),
                    observe(|_: On<Activate>| info!("open")),
                ),
                separator(),
                submenu("Recent", children![menu_item("level-1.ron")]),
                check_item("Autosave", true),
            ]
        ),
        menu_title(
            "View",
            children![radio_item("Small", false), radio_item("Large", true)]
        ),
    ]));
}
```

![The menus example: a bar with File, Edit, View and Help; F10 puts the ring on File, Down opens its menu with the ring on New and the chords Ctrl+N, Ctrl+O and Ctrl+S in a column at the right, Down twice reaches Recent and Right opens its submenu, Left closes it, Right on Save walks to Edit's menu where the disabled Redo shows "Nothing to redo", and Escape closes it](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/menus.gif)

*`examples/menus`: F10, Down, Right into a submenu, Left, Right across the bar to Edit, the reason on a disabled item, Escape.*

## The constructors

| | |
|---|---|
| `menu_bar(titles)` | the row, the panel's colour with one line under it; it takes any widget beside the titles |
| `menu_title(label, items)` | a button of the bar and the menu it opens, `items` a `children![]` of the rows below |
| `menu_item(label)` | a command: sends `Activate` and closes the menu; `.with_shortcut(Shortcut)` shows its chord at the end of the row |
| `check_item(label, checked)` | on or off, kept in Bevy's `Checked`; a choice flips it and sends one `ValueChange<bool>` |
| `radio_item(label, checked)` | one of a run: choosing it sets its own `Checked`, clears the radio items beside it up to the nearest separator, and sends `ValueChange<bool>` with `true` |
| `submenu(label, items)` | a row that opens another menu to its right; nothing in it says "Nothing here" |
| `separator()` | a line between two groups; not focusable, and it ends a radio run |
| `Disabled::new(reason)` | a component on an item: it cannot be chosen (`InteractionDisabled`) and its tooltip says why |

`menu_item`, `check_item` and `radio_item` return a `MenuItem`, a component whose insertion builds the
row (a mark slot, the label, the chord, a chevron on a submenu), so it chains `.with_shortcut(..)` and sits in
`(.., observe(..))` like a bundle. It sets the row's `Node`: put layout on a wrapper. The labels and chords
are the host's strings; this crate draws the chord in the platform's words (`Shortcut::label`) and does
not listen for it: a game's own `Shortcut::just_pressed` runs the command, as the shortcuts guide shows.

## Keys and pad

| | focus on a title | focus in a menu |
|---|---|---|
| F10, or Alt pressed and let go on its own | takes focus into the bar; again, gives it back | |
| Left, Right (D-pad too) | the previous or next title, wrapping | Right opens a submenu, or moves to the next title's menu; Left closes a submenu, or moves to the previous title's menu |
| Down (D-pad too) | opens the menu with focus on its first item | the next item (Up the previous) |
| Home, End | the first, the last title | the first, the last item |
| Enter, Space, South, a click | opens the menu (a click on an open title closes it) | chooses the item and closes the whole cascade; on a submenu, opens it |
| Escape, the pad's B | gives focus back to where F10 took it from | closes one level and puts focus on what opened it |

A disabled item can be focused (the player who walks onto it reads why) and never activates by any of
these. A pointer moving onto another title while a menu is open opens that one instead, and a press
anywhere outside the menus closes the cascade with one `PopoverClosed` per player action.

A menu is a modal `popover` while it is open (a `PopoverPanel`, a `Dialog` and a modal `TabGroup`), so its
placement, its outside-press close, its `Cancel` and its focus in and back are the
[popover's](popovers.md); it opens under its title (right of its item, for a submenu) and slides or flips at
the window's edge. A menu taller than the room the window leaves is capped there and scrolls, the wheel and
the focus following.

## Typing a name

With an open menu's item focused, typing goes to the next item whose label starts with what was typed within a second ("s" is Save, "s" again Save as..., a pause starts over); Space, and anything typed with Ctrl, Alt or Super held, is not a name. The same type-select is in [lists](lists.md), [tables](tables.md), [trees](trees.md) and a [select](select.md) of eight options or fewer.

## What is not here

- No hover-to-open for submenus and no safe triangle: a submenu opens on Right, Enter, Space or a click.
- A game's own chord for a command is the game's: the row shows it, `Shortcut::just_pressed` matches it.
- Bevy's `bevy_ui_widgets::menu` is not used: it closes a menu when focus leaves it, holds one menu open at a
  time and has no submenus.
