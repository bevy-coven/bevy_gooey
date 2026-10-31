# The widgets

Every widget is a free function returning a bundle, styled only by `apply_skin` through its `Role`, so
they compose inside `children![]` and take `observe` like any Bevy component. Their sizes are the
skin's, so changing `GooeySkin` restyles all of them that frame.

| widget | what it is |
|---|---|
| `screen()` | the layout root: full window, its empty part `Pickable::IGNORE` so a HUD lets the pointer through |
| `panel()` | a raised box a screen's content stands on |
| `menu()` | the column a set of buttons stack in |
| `heading(text)` | a title in the skin's heading size |
| `label(text)` | body text |
| `button(text)` | a Bevy `Button`, so it takes `observe`, `Tooltip` and `InteractionDisabled` |
| `meter(value, max)` | a HUD bar; `Meter { value, max }` on the entity drives it, and `update_meters` draws it |
| `tooltip` (`Tooltip("…")`) | a component on any widget: its box opens beside the widget under the pointer or on visible focus |
| `dialog()` | a modal: a scrim, focus taken on opening and held, and given back to the opener on closing |
| `toggle`, `choice`, `slider` | the options-screen controls, on Bevy's own `Checkbox`, `Button` and `Slider` — see [controls](controls.md) |
| `tabs`, `tab_page` | a strip of labels over pages of widgets, turned by a click, the arrows and the pad's bumpers — see [tabs](tabs.md) |
| `list` | rows that scroll and follow focus — see [lists](lists.md) |
| `toast`, `toast_alert` | a line of news at the edge of the screen — see [toasts](toasts.md) |

## A meter

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn hud(mut commands: Commands) {
    commands.spawn((
        screen(),
        children![(panel(), children![label("Health"), meter(72.0, 100.0)])],
    ));
}
```

`Meter { value, max }` is the state: write it and the next `update_meters` draws the fill. The frame
(`outline`) and the gap round the fill are drawn whole device pixels wide, so the fill sits centred in
its slot at every scale.

## A tooltip

`Tooltip` is a component whose text is the hint. It opens beside its widget — to the right in a column,
below in a row, flipped at the window's edge — one step outside its panel, so it never covers the next
button. It is a view of its component: `commands.entity(button).remove::<Tooltip>()` takes the hint off
(a "Locked: reach level 5" line once the player unlocks it), and inserting a new `Tooltip` or editing
`Tooltip.0` changes the line the open box shows. A tooltip on a widget inside a clipped container (a
row of a `list`) draws over the clip rather than being cut by it.

## A dialog

`dialog()` is the modal wrapper; spawn the dialog's own `panel()` inside it. Focus moves to its first
button when it opens, Tab, the arrows and the D-pad cannot leave it, and closing it puts focus back on
the button that opened it. Put `AutoFocus` on the dialog's safest choice ("Stay", "Cancel") to start
there instead.

```rust
fn quit(mut commands: Commands) {
    commands.spawn((
        dialog(),
        children![(
            panel(),
            children![
                label("Leave the game?"),
                (menu(), children![(button("Leave"), observe(|_: On<Activate>, mut exit: MessageWriter<AppExit>| { exit.write(AppExit::Success); })), (button("Stay"), AutoFocus)]),
            ]
        )],
    ));
}
```

## Over a game world

A `screen` is layout, not a surface: its empty part is `Pickable::IGNORE`, so a HUD on it lets the
pointer through to the sprites, meshes and UI roots under it (a world sprite opts in to picking with
`Pickable::default()`), while a `panel`, `button`, `meter` or `dialog` on it still takes the pointer, so
a click on one never reaches the world. A click on the empty part still blurs the interface. A menu
screen that must swallow the pointer: `commands.spawn(screen()).insert(Pickable::default())` — a bundle
cannot carry `Pickable` twice, so this is an insert and never part of the spawn's tuple.

See [the pointer](pointer.md) for the rules a HUD and a menu share.
