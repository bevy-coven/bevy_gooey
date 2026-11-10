# Layout primitives: a row, a column, a cell, an image box and an overlay

Researched 2026-11-10 for issue #81 (G-70), asked by the editor's rig bench and Kit workspace
(bevy_dark_factory#244, #245). The bench's panels are built from raw `Node` literals — `anim_tab.rs`
had 14, `tiles.rs` 51, `compose.rs` 5 — so every gap, column width and picture size is a number in
the caller and nothing on the panel follows the skin.

## Sources

1. Brent Lightbown, *Designing the User Experience of Game Development Tools*, CRC Press, 2015,
   §"Hierarchy" — craft (a game tool's interface, and the one book-length source in the corpus on
   tool panels).
2. *A century of Gestalt psychology in visual perception: I. Perceptual grouping and figure–ground
   organization* — Wagemans et al., 2012, Psychological Bulletin, doi:10.1037/a0029333 — craft
   (proximity and similarity, the perceptual grounds of a spacing scale).
3. W3C, *CSS Box Alignment*, https://www.w3.org/TR/css-align-3/, and *CSS Overflow*,
   https://www.w3.org/TR/css-overflow-3/ — technical (how a row squeezes its children, and how an
   absolutely positioned overlay is sized).
4. Valve's *Team Fortress 2* HUD and Bungie's *Destiny* companion screens (write-ups in the
   *Designing the User Experience of Game Development Tools* figure set and Valve's own HUD
   documentation, https://developer.valvesoftware.com/wiki/HUD) — shipping-game practice: a stat
   reading is a fixed-width label column beside a fixed-size picture, so ten readings line up.

## Craft principle

Lightbown, on hierarchy: "Objects that are placed close to each other are considered grouped. This
also means that objects with a lot of white space around them will stand out"; hierarchy is "defined
by four properties: position, thickness, size, and contrast". Wagemans et al. on proximity: grouping
"can be understood as the outcome of a probabilistic competition among potential perceptual
organizations", the odds set by the ratio of distances — which is why a scale of a few gaps (0, 1, 2
of one unit) reads as grouping and a stray number does not.

Two consequences for an API: the distance between two widgets must be one value the whole panel
shares, and a column of readings must share one left edge.

## How shipping games do it

A tool HUD reads down a column: every reading starts at the same x, its label in a fixed-width cell
and its value beside it, and its plot is a picture of a fixed size that does not move when the plot is
being redrawn. Valve's HUD documentation lists the stat panels this way; the TF2 HUD files themselves
(`resource/ui/*.res`) express each as a label with an explicit `wide`/`tall` and an image of a set
size, not as text flowed into whatever room is left.

## Model adopted

- **`row(gap)` and `column(gap)`** are flex containers whose spacing is `gap` multiples of
  `GooeySkin::gap` (`LayoutGap`, read by `apply_skin`), so a panel built from them sits on the same
  rhythm as a panel's own `row_gap`, and a step of `GooeySkin::text_scale` moves all of it. `row`
  leaves its children in the row's own stretch, so a control fills the line's height; `column`
  stretches its children to the widest.
- **`cell(width)`** is a row (its children stretch to its height, a label and a value side by side;
  a stack inside it is a `column`), a fixed width in **body text sizes** (an `em`: `Column::width`'s unit, so it
  grows with the player's text size), `min_width: 0` and `flex_shrink: 1`, with `overflow: clip`.
  A row narrower than its cells shrinks them in order and they never overlap. A cell of no width
  takes none and draws none of its words.
- **`image_box(handle, size)`** is an `ImageNode` in `NodeImageMode::Stretch` of `size` logical px,
  `flex_shrink: 0` and `overflow: clip`. The box is the size whatever the picture is, so a plot
  being written keeps its room; a handle with nothing loaded draws nothing and keeps the box. Its
  picture is `Pickable::IGNORE`, as every picture is: the plot's own wrapper takes the pointer
  (`RelativeCursorPosition` on the box still works, since the box is the node under it).
- **`overlay()`** is a box its first child sizes (`align_self: FlexStart`, so a taller sibling does
  not stretch it); `stack_overlays` puts every later child `Absolute` at 0,0 filling it, in
  `GooeySystems::Style` **right before `apply_skin`**, so one restyle draws the first child in the
  flow, a stacked child in the overlay's box, and a child that has just stopped being either in its
  own. **The stacking is a component (`OverlayStacked`) and is undone**: a child that becomes the
  overlay's first one, because the one before it was despawned or moved out, has it taken off and
  sizes the overlay again, and a child moved to another parent is laid out where it lands (a
  stacked child left `Absolute` in a row would have measured `390` tall in the row that wanted
  `378`). The first child is the only one in the flow, so the overlay is exactly its size.

## Parameters

|item|value|why|
|---|---|---|
|`row`/`column` gap|`LayoutGap × GooeySkin::gap`|one rhythm with a panel's own children|
|`cell` width|body text sizes (`text_px × CellWidth`)|`Column::width`'s unit; grows with the player's text scale|
|`cell` squeeze|`flex_shrink: 1`, `min_width: 0`|flexbox shrinks every item in proportion to its width, so cells give way together and never overlap; `image_box` is `flex_shrink: 0`, so the cells around a picture take the squeeze|
|`image_box`|`NodeImageMode::Stretch`, `flex_shrink: 0`|a picture is never squeezed; its box is its size|
|`overlay` children|`Absolute`, 0, 0, 100% × 100%|CSS absolute positioning in an `Absolute` parent (which taffy sizes to the parent's padding box)|

## Validation targets

A slot row built from the primitives lands on the same pixels as the hand-built row (±0.5 px on all
four edges of all three children); the row's gap is the skin's; the cell is six text sizes wide; the
box is the size it was given. A row narrower than its cells shrinks them in order and none overlaps
or leaves the row. A zero-width cell takes no room and clips. An unloaded image keeps its box and
draws nothing. An overlay's later children cover the first exactly and add no room. A column's gap is
the skin's, two gaps at `column(2.0)` and none at `column(0.0)`. A skin change moves the cell and the
gap with no number left behind.

## Deviations

- **The gap is a multiple of the skin's `gap`, not a step of `GooeySkin::spacing`.** The issue says
  "spacing from the skin"; the panel's own rhythm is `gap`, and `spacing` is a host's own scale (the
  crate's widgets do not draw from it). A designer's `spacing[2]` is 6 px under Tool and 10 under
  dark, so a row built on a step would be a different rhythm from the panel it stands in; on `gap`
  it is the same one. A host that wants a step can pass `gap × step` as its multiple — a design
  default, recorded here so the owner can overrule it.
- **A cell clips; it does not wrap.** A label column is one line per reading; a wrap would make rows
  different heights and the column would stop lining up.
- **An overlay takes no `overflow` of its own** and does not clip what is stacked in it: a picture
  that overflows its own box is cut by the box (`image_box` clips), where an overlay's child that is
  a panel is allowed to stand a shadow out.
- **No `overlay` size of its own.** The first child gives it, as the issue's "children stack
  absolutely over the first one" says; a host that wants a different box puts it in the first child.

## Pending index

None: all three sources are indexed. The Gestalt stem `10.1037/a0029333` carries the proximity
passage; Lightbown is `papers/li/lightbown_designing_ux_of_game_dev_tools_2015.pdf`; CSS Box
Alignment and Overflow are the W3C specifications (not home-still documents).

## What great looks like

A ten-row bench whose left edge is one straight line down the label column, its pictures all the same
size and in one column, its readings starting at the same x in every row, and a hover readout that
appears exactly over the plot it belongs to — with no number in the panel that is not a step of the
skin.
