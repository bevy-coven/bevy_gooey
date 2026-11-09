# A card's frame

A card is a picture, a name, a few lines and a state (chosen, hovered, disabled) — the unit an
inventory, a shop, a character select or a deck builder is built from. It is the largest flat surface
in a game's interface: a `CardCell` is 14 body text sizes wide and 16:9 tall, so an unframed card is a
big rectangle of `solid_surface` beside panels that already take art.

## Sources

1. **Craft** — *Good Game Feel: An Empirically Grounded Framework for Juicy Design*
   (10.26503/dl.v2018i1.936), in the corpus.
2. **Technical** — Bevy 0.20's `bevy_ui` `ImageNode` and `UiSystems::Layout`, and this crate's own
   `GooeyFrames` (`src/frames.rs`), which already draws panels, buttons, meters, tracks, segments,
   fields and popups from one `Frame` each.
3. **Shipping-game feel** — Rune (`https://github.com/anomalyco/rune`), this crate's open look
   reference: its kits draw every surface from one frame image, and a card in its gallery is a panel
   with a picture — the same art as the chrome around it.
4. **Technical, selection** — WCAG 2.2 SC 1.4.11 *Non-text contrast* (3:1 for the visual information
   that identifies a state), which is what a selected card's fill and ring are held to here.

## Craft principle

*Good Game Feel* argues that consistency across an interface is what lets a player read it:

> "This consistency also extends to other game elements, i.e., providing a consistent, believable game
> world that contributes to a positive player experience."

A card that is the one rectangle in the interface with no frame is an inconsistency the player reads
as "unfinished", not as a style. The frame is the same visual language the panel behind it speaks.

## How shipping games do it

Rune's kit draws every surface — a button, a panel, a card — from one nine-slice frame image, tinted by
the state, so a card's edge and a panel's edge are the same art at different tints and sizes. The
general rule: **a widget that is a surface takes the surface's art**; only pure text and pure layout
nodes are flat.

## Model adopted

- One new field, `GooeyFrames::card: Option<Frame>`: the frame of every `card_grid` cell
  (`Role::CardCell`). It joins `GooeyFrames::of` beside `panel` and `button`.
- Nothing else moves: a card's fill is still the state's (`GooeySkin::card()`, hovered, accent when
  chosen, the surface when disabled), and the art is tinted by that fill exactly as a panel's is. A
  card's `outline` border goes when it has art (`replaces_outline`), as a panel's does; its focus ring
  and its chosen ring stay, because those are state, not chrome.
- The art is the existing `FrameLayer` child at `ZIndex(-1)`, the mechanism panels and buttons already
  use, so a card needs no new spawn path.

## Parameters

|What|Value|Why|
|---|---|---|
|Field|`GooeyFrames::card`|Named for the widget a game sees (`card_grid`), not for its `Role`|
|Tint|the state's fill|The same rule as `panel`: art under, the state's colour over|
|Border|the outline goes when framed|A frame has its own edge; the chosen and focus rings stay|
|Art scale|nine-slice, `Frame`'s own rule|A card's corners must not distort (a card is 200-300 px square)|

## Validation targets

1. A `card_grid` cell with `GooeyFrames::card` set carries a `FrameLayer` child, and one without it does
   not.
2. The layer is the cell's child at `ZIndex(-1)`, sized to the cell, and takes no pointer.
3. The cell's fill is the state's and unchanged by being framed; the outline is gone when framed and
   present when not.
4. A chose card still reads as chosen: its accent fill and ring are over the art.
5. `tests/leaf.rs` still refuses game vocabulary; no colour or size is set outside `apply_skin`.
6. An idle framed card grid writes nothing.

## What great looks like

- A card reads as a physical card: a frame with a slight edge, a picture inside it, and the name and
  lines under the picture all inside the frame's padding, in the same rhythm as a panel's padding.
- Beside Rune: a grid of cards whose edges are the same art as the panel they sit on, so the screen is
  one surface language rather than one framed surface and a dozen flat rectangles.
- A chosen card's accent fill under the art still reads at 3:1 against its neighbours (WCAG 2.2
  SC 1.4.11); a disabled card's dimmed picture is still inside a frame, so it reads as *a card that
  cannot be used*, not as a hole.

## Deviations

None. No colour, size, gap or target changes; the field is empty by default, so an app that sets no
frames is byte-identical.

## Pending index

- No corpus stem covers *card grids* as a pattern; the craft claim rests on *Good Game Feel*'s
  consistency passage, and the layout claim on WCAG 1.4.11, which are the two that apply.
- *The Gamer's Brain* (Nacke) remains the natural next craft source for selection salience; still not
  in the corpus.
