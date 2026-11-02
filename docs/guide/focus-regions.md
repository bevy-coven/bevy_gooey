# Focus regions

A tool's screen is a few parts: a library on the left, a viewport, an inspector. The viewport owns the arrow keys (it pans, it orbits), yet the panels around it still need the keyboard. A `FocusRegion` names a part; F6 walks between them, and a region can keep the arrow keys for the host.

```rust
fn tool(mut commands: Commands) {
    commands.spawn((
        screen(),
        children![
            (FocusRegion::navigate(), children![(panel(), children![button("Wall"), button("Door")])]),
            viewport(), // a framed host region and a tab stop
            (FocusRegion::navigate(), children![(panel(), children![text_area("Notes", "", 5)])]),
        ],
    ));
}

// The host reads the arrows as it always did; gooey moves no focus while the viewport has it.
fn pan(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::ArrowLeft) { /* pan left */ }
}
```

![The workspace example: F6 puts the ring on the Library's Wall button, then on the viewport's frame, where Left, Left and Up change the pan readout and move no focus; F6 goes to the Notes field, F6 back to Wall, and Shift+F6 walks back to the viewport and the Notes](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/focus-regions.gif)

- **F6 and Shift+F6** move focus to the first widget of the next (previous) region, wrapping. Regions are visited in the order they stand in the UI tree, which is the order they were spawned in. A region with nothing focusable in it is skipped, and a region that is itself a tab stop (a `viewport()`) is its own first widget. F6 with Ctrl, Alt or Super held is not ours.
- **`ArrowPolicy::Host`** (`FocusRegion::host()`) leaves the arrow keys to the host: with focus on the region or in it, `navigate_by_input` moves nothing and shows no ring for them, and `ButtonInput<KeyCode>` has them as ever. The nearest region at or above the focused widget decides, so a button inside a host region keeps the arrows for the host too. The widgets that read arrows themselves (a slider, a text field) still do.
- **It never traps the keyboard** (WCAG 2.1.2): Tab, F6 and the D-pad still leave a host region. Tell the player the way out where the viewport is ("F6 leaves").
- **`viewport()`** is that host region ready made: a frame in the skin's `outline` that fills the room its parent leaves, the focus ring while it has focus, a `TabIndex`. Add `DropTarget::<T>::new()` to make it a place pieces are dropped ([drag and drop](drag-drop.md)).
- **A dialog holds focus over every region**: with one open, F6 cycles the dialog's own regions and nothing outside it.
- **A binding row waiting for a key** keeps F6 like every other key (`not_capturing`).
