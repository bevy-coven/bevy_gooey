# Docks

`dock(layout)` is a workbench of panels the player arranges: tab groups side by side or stacked, each a strip of tabs over the panel it shows, picked up and put down with the pointer. The layout is a value, [`DockLayout`](../../src/dock_layout.rs), so it can be saved, loaded and reset; what a panel shows is a function you register once under its id.

```rust
fn workbench(mut commands: Commands, mut panels: ResMut<PanelRegistry>) {
    panels
        .register("scene", "Scene", |page| {
            page.insert(children![heading("Scene"), button("Meadow"), button("Harbour")]);
        })
        .register("viewport", "Viewport", |page| {
            page.with_child(slider("Zoom", 0.6, 0.0..=1.0));
        })
        .register("log", "Log", |page| {
            page.with_child(label("12:40  Loaded Meadow"));
        });
    commands.spawn((
        screen(),
        children![(
            dock(DockLayout::split(
                SplitAxis::Horizontal,
                0.25,
                DockLayout::group(["scene"]),
                DockLayout::split(
                    SplitAxis::Vertical,
                    0.7,
                    DockLayout::group(["viewport"]),
                    DockLayout::group(["log"]),
                ),
            )),
            // Every change the player makes arrives here with the layout now shown: save it.
            observe(|changed: On<DockLayoutChanged>| info!("save {:?}", changed.layout)),
        )],
    ));
}
```

![The dock example: a Scene and Assets group, a Viewport over a Log and Console group and an Inspector; Assets is dragged into the Inspector's group with the landing lit and a card carrying its title, Log is dropped on the Viewport's right edge and gets a group of its own, Ctrl+Shift+Left moves it into the Viewport's group and Ctrl+Shift+Right into the Inspector's with the ring on its tab, a divider is dragged, and Console is dropped on the dock's bottom edge](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/dock.gif)

- **The layout is data.** `DockLayout::Split { axis, fraction, first, second }` divides its room in two and `DockLayout::Group { panels, selected }` is a tab group of panel ids (`String`s). With the `serde` feature it reads and writes as RON (or anything serde): a file may name only `panels` (the first tab shows) and leave out `fraction` (half). Build one in code with `DockLayout::group([..])` and `DockLayout::split(axis, fraction, first, second)`. `layout.panels()` lists the ids, `contains(id)` asks for one.
- **The state is `Dock(DockLayout)`** on the entity `dock()` spawns. Write a layout to it to load or reset: the dock shows it in the same frame, keeping the page of every panel it already had (moved, not rebuilt, so a scroll position or a half-typed field survives), spawning the new ones and despawning those no longer named. A game's own write sends no event.
- **What a panel shows** is the function you pass to `PanelRegistry::register(id, title, spawn)`: it gets the panel's page (a column that fills its group, with a `DockPanel(id)` on it) and adds the widgets. It runs once, when a dock first shows the panel. A panel registered after the dock is built appears the frame the registry changes.
- **Unknown and repeated ids** are left out and reported: the dock is built from the layout without them, a group left empty collapses into the node beside it, and a `DockPanelSkipped { entity, id, reason }` (`DockSkip::Unknown` or `Duplicate`) is sent once for each, with a `warn!`. `layout.prune(known)` does the same on plain data.
- **Rearranging.** Press a tab and move: the tab is selected, a card with its title follows the pointer, and the part of the dock the panel will take is lit in the focus ring's colour, framed in it, before the pointer is let go. Over a group's **tab strip** the panel goes into that group at the place under the pointer; over the **middle half** of a group it goes in as its last tab; over an **outer quarter** it gets a group of its own on that side of the group (half each); within 24 px of the **dock's own edge** it takes a quarter of the whole dock. Nothing is lit where the drop would change nothing (a group's own middle, off the dock). Escape puts the tab back. A group left with no panel collapses, so a layout never holds an empty group.
- **Without a drag** (WCAG 2.5.7): with a tab focused, **Ctrl+Shift and an arrow** moves its panel into the nearest other group in that direction, or, with none that way, to that edge of the dock; focus goes with the tab and the ring stays shown. Bevy's tab list also moves to the neighbouring tab on the arrow; the dock puts focus back on the tab that moved.
- **Saving.** Each change the player makes is one `DockLayoutChanged { entity, layout }` on the dock: a tab dropped or moved by key, a tab picked (`selected`), a divider let go (every key step is one). The layout in it is what is shown, and is also in `Dock`. A divider drag reports when it ends, not on every move.
- **Looks and sizes.** A group is a `panel()` with a tab strip that keeps each tab as wide as its label and wraps to a second row when the group is too narrow, so every tab can be seen and reached. Dividers are the [split's](splits.md), each pane at least 96 px (`DOCK_PANE_MIN_PX`). A pane's content that is wider or taller than the pane is clipped, as in any split; a group's page fills the group.
- **Not here:** floating windows, closing a tab, a tab strip that scrolls, and a pad's way to move a tab (the D-pad moves focus and Enter picks a tab).
- `model::dock_zone`, `dock_edge_zone`, `dock_drop_rect`, `dock_tab_slot` and `dock_neighbour` are the engine-free maths of the drop: which zone a point is in, where the landing is drawn and which group is next to another. `DockLayout::moved(panel, &DockDropTarget)` is the edit itself, on plain data: the layout after a drop, or `None` when it changes nothing.
