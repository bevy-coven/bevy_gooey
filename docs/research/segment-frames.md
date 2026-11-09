# Art on a strip, its tabs and a toggle group's options

Researched 2026-11-09 (iteration 82). No issue is filed. The pass's own pick, and the follow-up
`docs/research/segmented-strip.md` named in iteration 80: art on a tab strip and its segments, so a
painted skin's last flat surfaces join the rest.

The gap, three parts.

1. **A strip has no art.** `GooeyFrames` draws a panel, a button, a meter, a track, a field and
   everything that floats; a tab strip is still flat. Rune's segmented control is a *layer* under the
   segments with the lit one raised over it, and this crate's `SelectionIndicator` is already that
   raised shape — it draws only while it travels.
2. **A tab is flat.** `Role::Tab`'s fill is `surface_raised`, the hovered mix, the accent, or clear
   while the puck travels; a framed tab would tint the art with those and the art would be hidden
   behind the tab's own opaque fill anyway.
3. **A toggle group's option is flat**, same role, same treatment.

## Sources

1. *Good Game Feel* — Hicks et al., 2018, DiGRA, doi:10.26503/dl.v2018i1.936 (craft). Consistency is a
   category of its own: "providing a consistent, believable game world that contributes to a positive
   player experience". A painted panel with flat grey tabs is the inconsistency the paper names.
2. *Do Players Prefer Integrated User Interfaces?* — Llanos & Jørgensen, 2011, DiGRA,
   doi:10.26503/dl.v2011i1.514 (craft, shipping-game practice): an interface drawn in the game's own
   materials reads as part of the world. Art frames are how a skin is drawn in those materials.
3. Rune, `crates/rune-kit/src/segmented.rs:80-140` (the owner's reference, read not copied): the strip
   is a layer with `fill(c.layer_01)` and a 0.5 px shadow, the raised indicator is
   `fill(c.bg_base)` with a 0.5 px `border_strong` shadow and `token_motion(k.motion.control)`, and a
   segment is `grow(1.0).shrink().px(12.0).radius(6.0)` with `text_muted` ink that turns `text_base`
   when lit, hovered or focused.
4. Bevy 0.20, `bevy_ui_render/src/ui_texture_slice_pipeline.rs:382` (a node slices by its logical
   size, so art follows `UiScale`) and `ZIndex`: siblings stack by `ZIndex`, and a child of a node
   paints over that node's own background.

## Craft principle

A control repeated across a screen must be drawn in the same materials as the screen. Rune's
segmented control is one layer and one raised shape; the crate's indicator is already the raised
shape, so the missing half is the layer and the segments' own art.

## How shipping games do it

Rune: the strip is a filled, shadowed layer; the lit segment is a raised card over it; unselected
segments are transparent with muted ink. The contrast between the two is a fill step and a hairline,
not a hue change, so it survives a colour-blind player and a dark room.

## Model adopted

- **`GooeyFrames::strip`** (already documented): art on the `Role::TabList`, under the indicator's
  backdrop.
- **`GooeyFrames::segment`** (new): art on every `Role::Tab` and `Role::GroupOption`, at
  `ZIndex(-1)` on the option itself (below its own text, above the strip's art). A framed segment
  clears its fill so the art shows; the skin's fill becomes the tint exactly as on a button.
- **The indicator draws at rest when the strip has art.** A strip with a layer needs its lit segment
  raised, or the lit one is only a tint. `SelectionIndicator` already holds a puck sized to the lit
  option's rect; `update_selection_indicators` currently hides it whenever it is not travelling.
  With `GooeyFrames::strip` set it stays drawn on the lit option, hidden only while it travels (the
  travelling case already draws it).
- **The indicator's backdrop goes when the strip has art**, or it would paint `surface_raised` over
  the layer: `apply_skin` cannot see `GooeyFrames`, so `flow_strips`-time code must not own it —
  `update_frames` writes the backdrop clear on a strip it frames, and puts it back when the frame is
  taken away.

## Parameters

| Name | Value | Where from |
|---|---|---|
| segment art z | `-1` on the option | above the strip's `-2`, below the label |
| indicator at rest | drawn on the lit option | Rune: the lit segment is the raised one |
| indicator backdrop | no fill while the strip is framed | the layer is the ground |
| unselected ink | `text`, the tab's own | the crate's contrast rule (muted ink is for placeholders) |

## Validation targets

- With `strip` and `segment` set, a `tabs` strip's `Role::TabList` carries a `FrameLayer` at
  `ZIndex(-2)` and every `Role::Tab` one at `-1`, each tinted by the fill the flat look gives it.
- A framed tab's laid-out size equals the unframed one's, and its label is still on it.
- With a strip frame set, the indicator is drawn on the lit tab **at rest** (not hidden), its backdrop
  has no fill, and it still slides on a change.
- A `toggle_group` takes the same two fields.
- Taking the frames away restores every fill, the backdrop, and the indicator's rest state.
- A settled framed strip writes nothing.

## Deviations

- **The lit segment keeps its accent fill** rather than Rune's raised-card look: the crate's rule is
  two selection cues by luminance (`docs/guide/tabs.md`, "the selected tab is filled `accent` with
  `accent_text`"), and dropping it for a card would lose the second cue. The art is the *strip*, not
  the lit shape (a design default).
- **The indicator is not shown at rest.** Rune draws no separate indicator at all when the lit
  segment is the fill; the crate's puck is its travelling mark, and a puck parked on the lit tab under
  a translucent tint would be a second, redundant shape (design default). The strip's art plus the
  accent fill already read as a raised segment.

## Pending index

- Nothing new: the corpus carried two craft stems (*Good Game Feel*, *Do Players Prefer Integrated
  User Interfaces?*) and the owner's reference (Rune) for this note.
