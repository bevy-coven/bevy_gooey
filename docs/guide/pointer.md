# Over a game world

A `screen` is layout, not a surface: its empty part is `Pickable::IGNORE`, so a HUD on it lets the pointer through to the sprites, meshes and UI roots under it (a world sprite opts in to picking with `Pickable::default()`), while a `panel`, `button`, `meter` or `dialog` on it still takes the pointer, so a click on one never reaches the world. A click on the empty part still blurs the interface. A menu screen that must swallow the pointer: `commands.spawn(screen()).insert(Pickable::default())`.

## Double press and click counts

Two clicks are one `DoublePress` when the second press is on the same entity, within `PointerSettings::double_click_seconds` (0.5 s) of the first press and within `PointerSettings::double_click_slop_px` (4 logical px on each axis) of where it was, and it is let go on that entity. The first click is still a `PointerClick` at once: nothing waits to see whether a second follows. Only clicks pair: a press held past the interval, or one that travels past the slop while held, is neither half of a pair. Any press breaks a pending pair unless it is its second, so a third quick click starts a new pair and a fourth closes it. Keys never pair.

```rust,ignore
commands.spawn(button("Open")).observe(|mut double: On<DoublePress>| {
    double.propagate(false);
    info!("opened {}", double.original_event_target());
});
```

`DoublePress` bubbles up `ChildOf` from the entity pressed, as the click does. The split's divider (collapse and restore), the table's rows and the tree's rows (`RowActivated`) open on it. A drag reads `ClickCount` on the pressed entity, from its press until the pointer lets go: 1, 2, 3 for each quick press in the same place, as a desktop's click count is, so a triple-click drag can select by line. Set `PointerSettings` from the system's own double-click time where the host can read it; the clock is `Time<Real>`, so a test sets it with `TimeUpdateStrategy::ManualDuration`.
