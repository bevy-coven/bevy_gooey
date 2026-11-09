# Art on a multi-line text area

Researched 2026-11-09 (iteration 84). No issue is filed. The pass's own pick: the last text-entry
surface still flat in a painted skin.

The gap. `GooeyFrames::field` draws a single-line text field from art (iteration 78), but a
`text_area` — the same row with a taller field — is not in the map, so a journal or a description box is
a flat rectangle beside bevelled fields. Two surfaces of the same control, drawn two ways.

## Sources

1. *Do Players Prefer Integrated User Interfaces?* — Llanos & Jørgensen, 2011, DiGRA,
   doi:10.26503/dl.v2011i1.514 (craft). An interface drawn in the game's own materials reads as part of
   the world; a form is the place a player notices a mismatch most, because there are many of them at
   once and they are all the same shape.
2. *Good Game Feel* — Hicks et al., 2018, DiGRA, doi:10.26503/dl.v2018i1.936 (craft): consistency is a
   category of its own — "a consistent, believable game world that contributes to a positive player
   experience". One text box framed and its neighbour flat is the inconsistency, at the same size,
   side by side.
3. Rune, `crates/rune/src/el/visuals.rs:345-355` and its field: a field is a box drawn from the same
   surface, radius and hairline tokens as every other control, and a multi-line field is the same box
   made taller — the height is the only difference. Cited as the shipping reference (MIT, read not
   copied).
4. Bevy 0.20: an `EditableText` node is measured as a leaf, so a child layer collapses it (measured in
   iteration 78); the layer must be a child of the row's `Role::TextCell`, which is a plain container.
   That is already how a single-line field is framed.

## Craft principle

The same control drawn twice must be drawn the same. A text area differs from a text field in height
and line breaks, not in what it is; giving it different art is a lie about the widget's identity.

## How shipping games do it

Rune draws both from one field primitive: the box, its hairline and its radius are tokens, and the
multi-line field is the same box with more visible lines. Nothing in its kit gives a text area its own
art.

## Model adopted

- **`Role::TextArea` joins `Role::TextField` in `GooeyFrames::field`**, and takes the same host rule:
  the art layer is a child of the row's cell, not of the field.
- **Nothing else changes**: the field's border (outline and focus ring) stays over the art, the tint is
  the fill `apply_skin` wrote, and the field keeps its size.

## Parameters

| Name | Value | Where from |
|---|---|---|
| art host | the row's `Role::TextCell` | iteration 78's leaf measurement |
| tint | the field's `skin.surface` fill | `apply_skin` |
| layer depth | `ZIndex(-1)` | as every framed widget |

## Validation targets

- With `field` set, a `text_area`'s field carries a `FrameLayer` on its cell, tinted by the flat fill,
  with its own fill cleared and its size unchanged.
- A single-line field's behaviour is unchanged.
- Taking the frame away restores the text area's fill and removes its layer.

## Deviations

- **None.** The design default is that a text area is a text field made taller; the crate's own
  `text_area` already builds it that way (`RowKind::Area`).

## Pending index

- Nothing new: the corpus had two craft stems; Rune is the shipping reference.
