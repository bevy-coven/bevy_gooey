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
            row(1.0),
            // ... and the row's children, in the same spawn:
        ],
    ));
    commands.spawn((row(1.0), children![
        (cell(7.0), children![label("0 - clip 14")]),
        (overlay(), children![
            image_box(plot, Vec2::new(320.0, 160.0)),
            image_box(hover, Vec2::new(320.0, 160.0)),
        ]),
        label("free x1.00"),
    ]));
}
```

Their spacing is the skin's, so a step of `GooeySkin::text_scale` or a theme change moves all of it.

| primitive | what it is |
|---|---|
| `row(gap)` | a flex row whose children stand `gap` multiples of `GooeySkin::gap` apart; `row(0.0)` is flush |
| `column(gap)` | a stack of rows, the same gap between them; as wide as its widest child |
| `cell(width)` | a box `width` **body text sizes** wide, so a label column lines up down a list of readings; what does not fit is cut. A width of 0 takes no room and draws nothing |
| `image_box(handle, size)` | a picture of `size` logical px, whatever the image is: a plot being written keeps its room, an image that has not loaded draws nothing and still holds the box |
| `overlay()` | a box its first child sizes, with every later child stacked over it filling it |

## Rules the primitives keep

- **A row narrower than its cells shrinks them, in order, and they never overlap** (`flex_shrink: 1`,
  `min_width: 0`). A cell gives way before an `image_box`, which is never squeezed
  (`flex_shrink: 0`).
- **The gap is a multiple of `GooeySkin::gap`,** the same value a panel puts between its own children,
  so `row(1.0)` inside a `panel()` reads as the panel's rhythm. A host that wants a step of
  `GooeySkin::spacing` passes the multiple.
- **An overlay's stacked children are absolutely positioned** by `stack_overlays`
  (`GooeySystems::Style`, right after `apply_skin`), so they add no room and the overlay is exactly
  its first child. To put something in a corner, nest a node of its own in the overlay.
- **An `image_box` is `Pickable::IGNORE`,** like every picture: a plot that reads the cursor keeps a
  wrapper with `RelativeCursorPosition` on it, and the box inside it.

See the [research note](../research/layout-primitives.md) for why the gap is `gap` and not a step of
`spacing`.
