# Over a game world

A `screen` is layout, not a surface: its empty part is `Pickable::IGNORE`, so a HUD on it lets the pointer through to the sprites, meshes and UI roots under it (a world sprite opts in to picking with `Pickable::default()`), while a `panel`, `button`, `meter` or `dialog` on it still takes the pointer, so a click on one never reaches the world. A click on the empty part still blurs the interface. A menu screen that must swallow the pointer: `commands.spawn(screen()).insert(Pickable::default())`.
