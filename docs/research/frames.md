# Art frames: a skin's panels and buttons drawn from nine-slice images

Researched 2026-11-08 (iteration 74). No issue is filed. The pass's own pick, from what Sanctuary (the owner's shipped game) builds by hand: its `src/ui/widgets.rs` and `theme.rs` give every panel, name tag, toast and button a nine-slice `ImageNode` of painted art, because a flat colour with a rounded frame ("looks flat next to shipping games", the crate's own bar) is not the look of a parchment-and-ink game. The crate had no way to put art on a widget.

## Sources

1. Sanctuary, `src/ui/widgets.rs:1-8, 55-120` and `src/ui/theme.rs:85-115` (shipping game): "no surface builds a panel, button, label or frame by hand" yet each frame is `frame.node_image()`, an `ImageNode` with `NodeImageMode::Sliced(TextureSlicer { border, .. })` laid over the panel as an absolutely positioned layer (craft/shipping reference).
2. Bevy 0.20, `bevy_ui/src/widget/image.rs:212-233`, `NodeImageMode`: "`Sliced(TextureSlicer)`: The texture will be cut in 9 slices, keeping the texture in proportions on resize" (technical).
3. This crate: `src/color_motion.rs` (an `ImageNode`'s colour is eased by the ICON lane), `GooeyFonts` (assets are the host's, in their own resource, never in the serde skin).
4. *Do Players Prefer Integrated User Interfaces?* (corpus, craft): players weigh an interface that belongs to the game's world; art frames are how a UI belongs to a painted world. Not quoted: the corpus search for nine-slice practice returned nothing (Pending index).

## Craft principle

An interface that "belongs" to its game is drawn in the game's materials: a menu that is a sheet of parchment with a worn border reads as a prop of the world, a flat grey rectangle reads as a tool. The widget's behaviour, focus ring and contrast guarantees must not depend on which look it has.

## How shipping games do it

Sanctuary, `theme.rs:99-113`: each frame has a `TextureSlicer { border: BorderRect::all(n), .. }` and `node_image()` returns `ImageNode::new(handle).with_mode(NodeImageMode::Sliced(slicer))`; the panel stacks a tiled paper layer and a sliced frame layer under its content. The look is data (images and border sizes); the widgets read it.

## Model adopted

- **`GooeyFrames`** (a resource the style plugin inits, **not** serde: it holds image handles, like `GooeyFonts`): `panel: Option<Frame>` and `button: Option<Frame>`; `Frame { image: Handle<Image>, slice: f32 }`, `slice` the corner size in image pixels. Default: none, so nothing changes for a game that sets none.
- **Roles.** `Role::Panel` and `Role::Button`. A button whose variant has no fill at rest (`Ghost`) stays flat; nothing else changes.
- **The art is tinted by the skin.** `update_frames` (Style, right after `apply_skin`) moves the fill `apply_skin` wrote into the node's `ImageNode::color` and clears `BackgroundColor`, so every state colour (rest, hover, press, disabled, selected, a fade's alpha, a `SkinScope`) multiplies the art: a game draws its frames in light neutrals and the skin's colours do the rest. A panel's outline is cleared too (the art has its own edge). A button's border is the focus ring, so it stays: **the ring is drawn over the art**.
- **One path.** Colour motion already eases an `ImageNode`'s colour (the ICON lane), so a framed button eases its tint like an unframed one eases its fill. The widgets are the same entities with the same roles and tests; only the draw changes.
- **The node keeps its own layout.** The image sits on the widget's own node (no extra child), in `NodeImageMode::Sliced`, which Bevy does not use for sizing, so layout is unchanged (tested).
- **Taking the frames away** (setting the field to `None`) removes the `ImageNode` the crate added and gives the fill back.
- **Host art wins**: a `Panel` or `Button` that already has an `ImageNode` of the game's own is not touched.

## What great looks like

A menu panel with a painted border and a paper centre in the skin's colours, buttons with a bevel that darkens on press and lights on hover, the focus ring still crisp over the art; the same widgets with no frames set look exactly as before.

## Parameters

| Name | Value | Where from |
|---|---|---|
| slicer sides and centre | stretch | Bevy default for a frame |
| max corner scale | 1 | corners are never enlarged past the art |

## Validation targets

- With frames set, a panel and a button each carry a sliced `ImageNode` of the frame's image whose colour equals the fill the same widget has unframed, and their `BackgroundColor` is clear.
- A framed widget's laid-out size equals the unframed one's.
- The colour follows state: a disabled framed button's tint equals the unframed disabled fill.
- A ghost button has no image; a host's own `ImageNode` is not replaced.
- Clearing the frames restores the fill and removes the image. A settled framed widget writes nothing.

## Deviations

- Panels, buttons and what floats (`popup`: tooltip, popover, menu panel, palette, toast body). **A text field's art goes on its cell** (`field`, added in iteration 78): Bevy measures an `EditableText` node as a leaf, and any child (the art layer) makes layout ignore that measure, which collapsed the field to 20 px in the first try; so the layer is a child of the field's `Role::TextCell` (a plain container) and the field's own border stays over it. Tabs and toggle groups are not framed either: their selection indicator already lives at `ZIndex(-1)` under the options.
- First slice: panels and buttons only (superseded: see above). Tabs, toggles, sliders, tooltips, menus and the dialog's panel (a `Role::Panel`, so it is framed) follow the same mechanism and can join `GooeyFrames` field by field.
- Corner size is in image pixels and is not scaled with `UiScale` (Bevy slices in node pixels): a game on a large UI scale supplies larger art or sets `slice` to the scaled size.

## Pending index

- No corpus paper on nine-slice UI practice. Sanctuary and Bevy's docs are cited from source.
