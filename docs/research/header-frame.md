# Art on a table's header cells

Researched 2026-11-09 (iteration 84). No issue is filed. The pass's own pick, from measuring the
gallery: the sortable header is the last control-shaped surface with no art in a painted skin.

The gap. `GooeyFrames` draws a panel, a button, a meter, a track, a segment and a field. A table's
header cells are drawn exactly like a button — `apply_skin` handles `Role::Button` and
`Role::TableHeaderCell` in one arm, with the same fill, hover, press and disabled states — and they are
focusable sort controls, but they are not in the frame map. A painted table has a flat grey strip over
rows drawn from the same art as everything else.

## Sources

1. *Good Game Feel* — Hicks et al., 2018, DiGRA, doi:10.26503/dl.v2018i1.936 (craft). Its consistency
   category: "a consistent, believable game world that contributes to a positive player experience".
   Two controls drawn from one rule but only one wearing the art is the inconsistency.
2. *Do Players Prefer Integrated User Interfaces?* — Llanos & Jørgensen, 2011, DiGRA,
   doi:10.26503/dl.v2011i1.514 (craft/shipping). An interface in the game's materials; a sortable strip
   is where a player clicks most often in a ledger or inventory screen.
3. Rune, `crates/rune/src/el/visuals.rs:345-355` and its table: a sortable header is a filled cell from
   the same surface tokens as a button, with the sort mark in the same ink. The header is not a special
   surface (the reference, read not copied).
4. This crate: `src/skin.rs:2655` — `Role::Button | Role::TableHeaderCell` share one fill arm, and
   `:3067` one border arm. The header is already a button to the skin; only the frame map omitted it.

## Craft principle

A control is what it is drawn as. The header already answers to a button's fill, hover and press; being
the one such control that cannot wear a skin's frame is a gap in the skin, not a decision.

## How shipping games do it

Rune's table header is a cell of the list's own surface with the sort chevron in the row ink — no
separate header material. That is this crate's model too: the header is a button, so it takes the
button's art.

## Model adopted

- **`Role::TableHeaderCell` joins `Role::Button` in `GooeyFrames::button`.** No new field: the header is
  already styled as a button, so it takes the button's frame.
- **Its border stays over the art**: a header cell's border is its focus ring (the same arm as a
  button's), so `replaces_outline` does not include it.
- Nothing else changes; a framed header's tint is the fill its state gives it.

## Parameters

| Name | Value | Where from |
|---|---|---|
| frame | `GooeyFrames::button` | `apply_skin` already treats them as one role |
| layer depth | `ZIndex(-1)` on the cell | as every framed widget |
| outline | kept (the focus ring) | `src/skin.rs:3067` |

## Validation targets

- With `button` set, every `Role::TableHeaderCell` carries a `FrameLayer` tinted by the fill the flat
  look gives it, with its own fill cleared and its size unchanged.
- Sorting, hovering and focusing a header still work, and the ring is drawn over the art.
- Taking the frame away restores the header's fill and removes its layer.

## Deviations

- **The header takes the button's art rather than an art of its own.** A skin that wants a different
  header material cannot say so; that is the smaller surface, and a second field can be added later
  without a breaking change (`GooeyFrames` is `#[non_exhaustive]`) (a design default).

## Pending index

- Nothing new: the corpus had two craft stems; Rune is the shipping reference.
