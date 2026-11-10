# A text area that fills its host and scrolls

Researched 2026-11-10 for issue #86 (G-75), asked by bevy_dark_factory#350 (AG-05: the agent guide on
the project page, viewable and editable, up to 64 KiB). At `d1ec9ee` `text_area` is 26 character
advances wide by a fixed count of rows; a 64 KiB file can be neither read nor edited in it.

## Sources

1. W3C, *WAI-ARIA Authoring Practices Guide*, "Textbox" — craft (a multi-line textbox is one control
   with one value; its height is the page's to give, not the content's).
2. Bevy 0.20 vendored source — technical, and the reason this module is small: `bevy_ui`'s
   `sync_editable_text_viewports` already makes `EditableText::viewport.size` the field's own box,
   `bevy_text`'s `TextEdit` reveals the caret on every edit, every arrow and every click, and
   `bevy_ui_render` draws the glyphs at `-viewport.offset`. What no Bevy widget does is scroll an
   `EditableText` with the wheel or draw it a rail.
3. `docs/research/layout-primitives.md` (#81) and `docs/research/icons.md` (#25) — craft, the crate's
   own spacing and icon conventions this widget follows.
4. Rune (`https://github.com/anomalyco/rune`, MIT), the runtime's text editing — craft: a document
   view scrolls as one region with a rail the whole height of its lane.

## Model adopted

- **`text_area_fill(label, value, rows)`** is a `text_area`'s row (`TextInputValue`,
  `ValueChange<String>`, `TextLimit`, `TextHint`, `FieldProblem`, `TextEditing`, Ctrl+Enter to end the
  edit) whose field takes the room its parent leaves, with `rows` a *least* height. `TextAreaFill` is
  the marker on the row, so a host finds one and a fixed `text_area` is untouched.
- **A very long line wraps** (`LineBreak::WordOrCharacter`: a word with no break opportunity in it
  still breaks) and the text scrolls vertically only. No horizontal scrolling exists to find, which is
  the issue's stated choice.
- **The wheel, the rail and the rail's drag** are the module's: one writer of the offset outside
  Bevy's own caret reveal. The wheel is read from `MouseWheel` (a `PointerScroll` on the field is
  swallowed by Bevy's text handler) and aimed with the `HoverMap`; the rail is `Role::ListRail` with a
  `Role::ListThumb`, so an area's rail and a list's agree on the eye; a press on the rail scrolls to
  the pointer's share and takes no focus (a focus would put the view back at the caret).
- **The caret is revealed here only when it moves** (`AreaCaret` records the caret last seen). Bevy's
  reveal on the frame focus arrives reads the caret *before* the press that focused the field has
  moved it, so a click in a scrolled area snapped the view back to the top: measured, offset 159331
  before the press and 0 after. With the record, the player's own click, arrow or edit reveals the
  caret, and a wheel turn or a settled frame writes nothing.
- **One owner per fact:** the viewport's size is Bevy's, the offset is this module's, and the thumb's
  size and travel are written only on a difference.

## Units, which cost three bugs while building it

Three lengths meet in this widget and they are not the same number (measured with a probe at 1280x720
and 1920x1080, `UiScale` 1 and 1.5):

|Length|What it is|Where it is read|
|---|---|---|
|skin px|a `Val::Px`|`Node` (`height`, `margin`)|
|physical px|`target.scale_factor()` of skin px|`ComputedNode::size`, `TextLayoutInfo::size`, `TextViewport`, `UiGlobalTransform::translation`|
|logical window px|physical / the window's `scale_factor`|a pointer's `position`|

`ComputedNode::inverse_scale_factor` is the skin-to-physical factor (`1/1.5` at 1080p), *not* the
window's: dividing a pointer by it is wrong, and so is writing a physical height into a `Val::Px`.

## Where this differs from the issue

- **No separate `rows` cap.** The least height is `rows` and the host decides the rest; a host that
  gives the row nothing gets a field `rows` lines high rather than one of none.
- **Paste of 64 KiB is Bevy's own path** (a `TextEdit::Paste` respects `TextLimit`), unchanged by this
  module: the acceptance test types at the end of a 64 KiB document and reads the whole text back.

## Proof

`tests/text_area_fill.rs`, all laid out for real in a headless window and driven by real input: the
acceptance (64 KiB in 40 rows, wheel to the end, the last line inside the field, then Ctrl+End to the
document's end and one character typed there sending one change carrying all of it, and Ctrl+Enter a
final one); the wheel scrolls a filled area
and not a short one; a filled area is as tall as its host; the rail shows while the text overflows,
its thumb is the share shown and a press on it jumps the view; a very long line wraps and scrolls
vertically with `offset.x` 0; a scaled interface (1080p, `UiScale` 1.5) scrolls in its own pixels, a
press a quarter down the rail landing a quarter into the text and the wheel reaching the text's end;
and a settled area writes nothing over thirty frames.
