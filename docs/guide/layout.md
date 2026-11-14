# Layout primitives

Five widgets (and `row_with`, `column_with` and `button_row`) that lay a panel out without a `Node` literal anywhere in the caller: `row(gap)`,
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

## How a row lays out

`row(gap)` and `column(gap)` take a gap only. What else a container does, which way its children
stand across it, whether a full line wraps and how they share the rest, is a `RowLayout`:

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn settings_row(mut commands: Commands) {
    commands.spawn((
        row_with(
            1.0,
            RowLayout { align: AlignItems::Center, wrap: FlexWrap::Wrap, justify: JustifyContent::SpaceBetween },
        ),
        children![label("Modified; without it the chord is Ctrl+S"), button("Reset")],
    ));
    // The same component on a plain row, so it works in a bundle or a `Children::spawn(SpawnWith(..))`.
    commands.spawn((row(1.0), RowLayout { align: AlignItems::Center, ..default() }));
    // A line of buttons with a tight gap: centred, wrapping a gap below when the room runs out.
    commands.spawn((button_row(0.5), children![button("PLAY"), button("BUILD")]));
}
```

`fill_column(gap)` is a column that takes the room its row leaves and can narrow to zero
(`flex_grow` 1, `min_width` 0): beside a fixed picture or a `count_text`, the text in it wraps inside
it and never pushes the row out (`fill_text` is the same for one text; this holds several parts).

`fill_region()` is its twin on the height axis (`flex_grow` 1, `min_height` 0) for what holds a tree
or a list: a region of a column that takes the height its siblings leave and can shrink to nothing,
so a tall tree scrolls inside it and never makes its host taller.

`sized_panel(CardWidth::Px(520.0))` (or `CardWidth::Text(30.0)`, in body text sizes) is a `panel()`
with a width of its own: a dialog's card, or a form on a screen, that is not 21 to 36 text sizes. It
is never wider than the room it stands in (a dialog's card: the window less the scrim's 5% on each
side) and has no least width.

`placed_at(30.0, 40.0)` puts a node that many px in from its host's top left corner, `placed(UiRect { right:
.., bottom: .., ..UiRect::all(Val::Auto) })` at any corner or edge, and `cover()` fills its host: all
absolute, so they take no room in the flow and the host's other children stay where they were, and
`Pickable::IGNORE`, so the pointer goes through (`insert(Pickable::default())` for one that must take
it). A node that follows a point **in the world** (a balloon, a label on a map area, a badge on a mesh)
is not this: it is a [`WorldAnchor`](world-anchor.md).

`status_mark()` is the dot at the start of a status item on its own: put a `StatusKind` on the row it
stands in and it is the accent for a warning, the danger colour for an error and gone for information.
`plain_button("Cancel")` is a button's fill, frame, height and hover with none of its behaviour (no
Bevy `Button`): no `Activate`, no tab stop, no pressed state; listen for `PointerClick` yourself.

`aspect_image(handle, 480.0, 16.0 / 9.0)` is a picture sized by its ratio: 480 px wide while its host
has the room, no wider than the host when it does not, its height following (`image_box` is a fixed
size that never gives way).

`RowLayout` is written **only when it is on the node**: a plain `row(1.0)` keeps a `Node` field you
patched yourself. A wrapping container puts the skin's gap between its lines as well as between its
children. `column_with(gap, layout)` is the same for a column (`align` is across its width).

## The parts of a list and a strip

Five more widgets for what a tool's panels are made of between those primitives, each with the look
and the sizes from the skin (`cargo run --example kit` shows them):

```rust
use bevy::prelude::*;
use bevy::ui::Selected;
use bevy_gooey::*;

fn kit(mut commands: Commands) {
    commands.spawn((
        panel(),
        children![
            // A strip the host selects: the edge along the bottom of the selected tab is lit and
            // no tab moves.
            (row(0.0), children![(edge_tab("Meshes"), Selected), edge_tab("Tiles")]),
            // A grouped list's header: a button, full width, shaded on hover like a row.
            (
                band(),
                observe(|_: On<Activate>| info!("fold the group")),
                children![
                    (Text::new("v"), Role::ButtonText),
                    fill_text("kenney_prototype-kit/Models/GLB format"),
                    count_text("145"),
                ],
            ),
            // A mark too small to press, whose hit area is the skin's target.
            (row(1.0), children![
                fill_text("wall.glb"),
                glyph_button("x", "Remove wall.glb"),
            ]),
        ],
    ));
}
```

| part | what it is |
|---|---|
| `glyph_button(glyph, name)` | a `button` for a caret or a cross: no padding, and a hit area of `GooeySkin::target()` on each side (a `ControlSize` taller) **whatever the glyph's size**, the glyph centred. `name` is its tooltip and its screen-reader name |
| `edge_tab(label)` | a tab for a strip the host selects with Bevy's `Selected`: a tab's inset, and a bottom edge that is always there (1.5 line weights) and lit in the accent only on the selected tab, so selecting moves nothing. The selected tab is the surface, the others a button's raised fill. It takes no focus and no Enter: for a strip with keys and pages use `tabs` |
| `band()` | a full-width row that is a button, a half gap off what is above it, with a button's fill and states: a grouped list's header. Put `Role::ButtonText` marks, `fill_text` and `count_text` in it |
| `fill_text(text)` | a text that grows, gives way to its neighbours (`min_width` 0) and wraps at a word or a letter, so a long name wraps in its own column and never pushes its row out |
| `count_text(text)` | a text that never shrinks and never wraps: a tally that keeps a column of its own |

A label that follows a point of a 3D view needs no widget of its own: it is a [`WorldAnchor`](world-anchor.md)
with `.camera(stage)`, as a child of the pane that stands on the camera's viewport.

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
