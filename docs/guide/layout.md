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
stand across it, whether a full line wraps, how they share the rest and whether the container itself shrinks, is a
`RowLayout`. Each field is an `Option`: `None` leaves what the role gives the node (a `button_row` keeps
its centring and its wrapping when you set only `justify`), `Some` writes it:

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn settings_row(mut commands: Commands) {
    commands.spawn((
        row_with(
            1.0,
            RowLayout {
                align: Some(AlignItems::Center),
                wrap: Some(FlexWrap::Wrap),
                justify: Some(JustifyContent::SpaceBetween),
                shrink: Some(0.0), // a rigid row: it keeps its size when its host squeezes
            },
        ),
        children![label("Modified; without it the chord is Ctrl+S"), button("Reset")],
    ));
    // The same component on a plain row, so it works in a bundle or a `Children::spawn(SpawnWith(..))`.
    commands.spawn((row(1.0), RowLayout { align: Some(AlignItems::Center), ..default() }));
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

## Room in the skin's steps

`Spacing` is margin and padding in steps of `GooeySkin::spacing`, on any node that has a `Role`:

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn section(mut commands: Commands) {
    commands.spawn((
        label("Section"),
        // A step of the skin's scale under the text; a side left `None` leaves the node alone.
        Spacing {
            margin: Sides { bottom: Some(Space::Step(2)), ..default() },
            padding: Sides::default(),
        },
    ));
    // A padded column; `Space::Zero` is how to ask for none where a role gives some.
    commands.spawn((column(0.0), Spacing { padding: Sides::all(Space::Step(1)), ..default() }));
}
```

A side that is set wins over the role's own (a `panel`'s padding); the unit is the skin's step, so a
theme or a skin change moves all of it.

## A spacer, a gap, a hidden page

`spacer()` takes what its row or column leaves: `children![button("Back"), spacer(), button("Next")]`
puts one button at each end, and two spacers centre what stands between them. `fill_region_with(gap)`
is a `fill_region()` whose children stand `gap` skin gaps apart, and `Shown(false)` hides
it through the skin (`Display::None` and `Visibility::Hidden`: the page takes no room, what follows
closes up, and nothing in it is a tab stop or an open dialog):

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn pages(mut commands: Commands) {
    commands.spawn((row(1.0), children![button("Back"), spacer(), button("Next")]));
    commands.spawn((fill_region_with(1.0), Shown(false), children![heading("Home"), label("Recent")]));
}
```

Set `Shown(true)` to bring the page back: it gets back the display and visibility it had.

## Size and shape

`Sizing` is a width and a height (px, a percentage of the host, or body text sizes), their least and
greatest, and a clip; `Shape` is a border per side in the skin's line weights, its colour, a corner
radius and a fill. Both go on any node, every field is an `Option`, a field left `None` leaves the
node alone, and `sized(sizing)` is a plain box that carries a `Sizing`:

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn chip(mut commands: Commands) {
    commands.spawn((
        placed_at(30.0, 40.0),
        Sizing { width: Some(Extent::Px(120.0)), height: Some(Extent::Px(60.0)), ..default() },
        Shape { border: Sides::all(1.0), ink: Some(Ink::Accent), radius: Some(Radius::Step(1)), fill: Some(1) },
        RowLayout { direction: Some(FlexDirection::Row), ..default() },
        children![label("Look"), label("Free")],
    ));
}
```

A rail is a border on one side (`Sides { right: Some(1.0), ..default() }`); a pill is
`Radius::Full`. A shape with only a radius leaves a button's own fill and edge.

## A text with its own flex

`TextFlow` says how a text takes part in its row: `shrink: Some(0.0)` keeps its words' width,
`grow: Some(1.0)` with `min_width: Some(Extent::Px(0.0))` is a title that takes what is left and may
narrow to nothing, `align` is where it stands across the row. `label_with(text, flow)` and
`fill_text_with(text, flow)` carry it; a field left `None` leaves the role's own.

## A list row, and the editor's chrome from these parts

`list_row()` is a selectable full-width row of a list: Bevy's `Selected` on it lights a rail on its
left in the accent and raises its fill, and nothing moves because the rail's width is always
reserved. `quiet_list_row()` looks the same and is not on the `Activate` bus.

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn files(mut commands: Commands) {
    commands.spawn((column(0.0), children![
        (list_row(), bevy::ui::Selected, children![fill_text("agent_view.rs"), count_text("12")]),
        (list_row(), children![fill_text("chrome.rs"), count_text("3")]),
    ]));
}
```

A tool's frame is built from the parts above: a `sized(100% x 100%)` frame, a bar, a body
(`fill_region()` with `RowLayout { direction: Some(FlexDirection::Row), .. }`), docks as
`sized(width)` (hide one with `Shown(false)`), a viewport slot as `fill_column`, a band as
`status_bar`. **Every box between a tall list and the window needs `min_height` 0**
(`Sizing { min_height: Some(Extent::Px(0.0)), .. }`, the window's root too): a flex item's automatic
minimum is its content, and a tall child would hold the whole chain open. A modal is `dialog()` +
`Shown(false)` + `sized_panel`; a severity rail is a `column` with a `Shape` border on its left and a
`Spacing`. The mapping, builder by builder, is in `docs/research/layout-more.md`.

## Spacing off the scale

`Space::Px(6.0)` is a literal logical px for the measures that are not on the skin's spacing scale (a
chip's `6 x 3`, a tab's `12 x 8`): `Spacing { padding: Sides { left: Some(Space::Px(6.0)), .. }, .. }`. It is
written by the skin and finite-checked (below zero is zero, not a number leaves the side alone) but does
not follow the scale or the text size: prefer a `Step`.

## Viewport lengths

`Extent::Vh(100.0)` is the window's whole height and `Extent::Vw(50.0)` half its width, whatever the
host is (`VMin` and `VMax` are the smaller and larger side): the root of a window-sized frame. Like
every extent it is finite-checked and never negative. Remember `min_height` 0 down the chain.

## Which borders snap

Every border the skin draws is a whole number of device pixels (a 2 px line at 1.25x is 2 device px,
1.6 logical): the frame of every widget, the focus ring, an edge tab's edge, and a `Shape` border weight.
**`BorderPx(Sides::all(3.0))` is not snapped**: it is the logical px it says, for a drag box or a pick
outline whose width is the requirement (Bevy's layout still rounds it to its own device pixels). A
border you write on a `Node` yourself is left alone unless a `Shape` or `BorderPx` names that side.

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
