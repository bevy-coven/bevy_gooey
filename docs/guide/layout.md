# Layout primitives

Five widgets that lay a panel out without a `Node` literal anywhere in the caller: `row(gap)`,
`column(gap)`, `cell(width)`, `image_box(handle, size)` and `overlay()`.

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn bench(mut commands: Commands, plot: Handle<Image>, hover: Handle<Image>) {
    commands.spawn((
        panel(),
        children![
            heading("Rig bench"),
            (row(1.0), children![
                (cell(7.0), children![label("0 - clip 14")]),
                (overlay(), children![
                    image_box(plot.clone(), Vec2::new(320.0, 160.0)),
                    image_box(hover.clone(), Vec2::new(320.0, 160.0)),
                ]),
                label("free x1.00"),
            ]),
            (row(1.0), children![
                (cell(7.0), children![label("1 - clip 2")]),
                image_box(plot, Vec2::new(320.0, 160.0)),
                label("gait x1.00"),
            ]),
        ],
    ));
}
```

Their spacing is the skin's, so a step of `GooeySkin::text_scale` or a theme change moves all of it.

| primitive | what it is |
|---|---|
| `row(gap)` | a flex row whose children stand `gap` multiples of `GooeySkin::gap` apart; `row(0.0)` is flush |
| `column(gap)` | a stack of rows, the same gap between them; its children stretch to its width |
| `cell(width)` | a box `width` **body text sizes** wide, so a label column lines up down a list of readings; what does not fit is cut. A width of 0 is not shown at all: no room, no gap |
| `image_box(handle, size)` | a picture of `size` logical px, whatever the image is: a plot being written keeps its room, an image that has not loaded draws nothing and still holds the box |
| `overlay()` | a box its first child sizes, with every later child stacked over it filling it |

## Rules the primitives keep

- **A row narrower than its children shrinks every cell in it by the same share of its width**
  (`flex_shrink: 1`), so cells give way together and never overlap; an `image_box` never shrinks
  (`flex_shrink: 0`), so a picture keeps its size and the cells around it take the squeeze. `row` and
  `column` are as wide as the room they are given and never push out of their host.
- **The gap is a multiple of `GooeySkin::gap`,** the same value a panel puts between its own children,
  so `row(1.0)` inside a `panel()` reads as the panel's rhythm. A host that wants a step of
  `GooeySkin::spacing` passes the multiple.
- **An overlay's stacked children are absolutely positioned and given the first child's box** by
  `stack_overlays` (`GooeySystems::Style`, right before `apply_skin`), so they add no room and the
  overlay is exactly its first child. A child that becomes the first one (the one before it was
  despawned or moved out) or that leaves the overlay is put back in the flow (`OverlayStacked`
  goes), so an overlay never ends up a box of nothing. To put something in a corner, nest a node of
  its own in the overlay.
- **An `image_box` is `Pickable::IGNORE`,** like every picture: a plot that reads the cursor keeps a
  wrapper with `RelativeCursorPosition` and `Hovered` on it, and the box inside it.

See the [research note](../research/layout-primitives.md) for why the gap is `gap` and not a step of
`spacing`.
