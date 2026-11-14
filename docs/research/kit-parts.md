# The parts a tool's panel still built from raw nodes

Researched 2026-11-14 for issue #93 (G-82), asked by bevy_dark_factory (the Cauldron), issue #245 (UI-59, the Kit workspace), tracking #208. After four slices its Kit modules still held nine raw `Node { .. }` literals because gooey had no widget that said what each one is. Each is a general need; the editor is the example and its name is in no API.

## Sources

1. WCAG 2.2 SC 2.5.8, Target Size (Minimum): a target is at least 24 by 24 CSS px, which this crate holds as `GooeySkin::target()`. A mark smaller than that is still a target of that size.
2. Fitts, *The information capacity of the human motor system*, 1954 (and MacKenzie 1992): the time to select a target grows with the distance and falls with its width. The width that counts is the one that answers the press, not the one that is drawn. That is why the hit area is a floor of the skin and not a function of the glyph.
3. WAI-ARIA Authoring Practices, Tabs: a tab has the `tab` role and `aria-selected`. A strip that is not a `tablist` with roving focus (the host moves it by its own keys) still names each tab and its selected state.
4. CSS Flexible Box Layout, 4.5 Automatic Minimum Size of Flex Items: an item's `min-width: auto` is its content's width, so a text with no stated minimum is as wide as its longest unbroken line and pushes its container out. A text that is to give way states `min-width: 0` and a `flex-shrink`.
5. Bevy 0.20 `bevy_camera::Camera::world_to_viewport` (read from the vendored 0.20.0 source): it returns window coordinates, `viewport.min` included. `WorldAnchor` takes `view.min` off again, so the node's `left` and `top` are the camera's own viewport pixels.

## What each item of the issue is

1. **A glyph with a minimum hit area: `glyph_button(glyph, name)`**, `Role::GlyphButton`. Bevy's `Button` (focus, `Activate`, a button's fill, frame and states) whose node has **no padding** and `min_width` = `GooeySkin::target()`, `min_height` = the control's height (`ControlSize`, never under the target), `flex_shrink` 0, the glyph centred. So a 6 px caret is a 44 px target under the comfortable skins and 24 px under a compact one, and a 48 px glyph is its own size. `name` is the tooltip's line and the screen-reader name (`AccessibleLabel`): the glyph is what is drawn, never what is said.
2. **A tab with its own inset and an active edge: `edge_tab(label)`**, `Role::EdgeTab`. A tab's inset (`pad * 1.5` across, the button's air up and down), a **bottom border that is always there** at `EDGE_TAB_LINES` = 1.5 line weights (heavier than a frame: it is read across a whole tab), lit in `GooeySkin::accent` only while the tab has Bevy's `Selected`. Lighting it changes a colour on a border that was already laid out, so the selected tab's width and every neighbour's place are the same before and after: the issue's "changes the tab's own padding without moving its neighbours" is met by reserving the edge, not by moving padding. The selected tab's fill is `surface` (the panel's, so the tab continues what it heads), the others `surface_raised`, `hovered()` under the pointer; its words are `text` in every state (`State::edge_tab`: the accent's text is for an accent fill). It takes no focus and answers no Enter, and it is the host's to select (`Selected`); the strip with its own keys and pages is `tabs`. It is said to be a `Tab` to assistive technology (`Ak::Tab`, named from its words).
3. **A full-width band that is a button: `band()`**, `Role::Band`. Bevy's `Button` row, `width: 100%`, a half gap off what is above, a button's fill, frame, focus ring and states, so it shades on hover with exactly the colours of a list's row (`surface_raised` at rest, `hovered()` under the pointer, the accent while pressed). Its children are the host's: a mark in `Role::ButtonText`, a `fill_text` and a `count_text` follow the band's state. It is a button, so the fold is `Activate`.
4. **A text that is its own flex item:** two, because the two uses are opposite halves of one row. `fill_text(text)` (`Role::FillText`): `flex_grow` 1, `flex_shrink` 1, `min_width` 0, wrapping at a word or a letter (`LineBreak::WordOrCharacter`), so a long name wraps inside its own column. `count_text(text)` (`Role::CountText`): `flex_grow` 0, `flex_shrink` 0, `LineBreak::NoWrap`, so a tally keeps its words whole. Neither takes a colour: both are the text's ink, inverted on an accent band while it is pressed, muted when disabled. The gap between them is the row's (`row(1.0)`) or the band's own.
5. **A world-space label over a viewport pane: already covered by `WorldAnchor`**, so nothing was built. `WorldAnchor::new(target).camera(stage_camera)` on a `label(..)` places it on the point the camera projects, hides it while the point is behind the camera or out of view, and lets the pointer through (`Pickable::IGNORE`). **Where the node goes matters:** its `left` and `top` are the camera's viewport pixels, so with a stage camera whose viewport is a rectangle of the window the label is a **child of a node that stands on exactly that rectangle** (the pane), not a root. The test is `a_label_in_a_viewport_pane_stands_on_the_point_its_camera_projects`: a 600 x 400 pane at (300, 100), a slot at world (100, 50), the label's foot at (700, 250), hidden again when the slot leaves the view. What it does not do is the editor's own business: laying the labels of several slots out so they do not overlap.

## Where this differs from what the editor hand-built

- The hit area is the skin's target on both sides (the editor set 20 px by its own `MIN_FIELD_H`): a compact skin gives the editor's 24, a comfortable one the 44 a thumb needs.
- The tab's edge is `1.5 * border` (3 px at the dark skin's 2 px line weight, the editor's 3), not a literal; at `border` 3 it is 4.5 px snapped to the device.
- A band's top margin is half a gap for every band; the editor used a row gap for one and a group gap for the other. A host that wants a bigger step puts a `row(..)` or its own spacing above.

## Proof

`tests/kit_parts.rs`, 11 tests, each laid out for real in a headless 1280 x 720 window:

- `a_glyph_button_keeps_a_hit_area_of_the_floor_whatever_its_glyph`: at skin text sizes 6, 14 and 48 px the button is at least `target()` on both sides and the glyph is centred in it (a 6 px glyph is under half the floor).
- `a_glyph_button_is_pressed_anywhere_in_its_hit_area`: a click sends one `Activate`, and a point 40% of the box off its middle is inside the button and not inside the glyph.
- `the_selected_tab_lights_its_edge_without_moving_a_neighbour`, `a_tab_edge_is_heavier_than_a_frame_and_the_inset_is_the_skins`, `a_tab_takes_its_fill_and_ink_from_the_skin_by_state` (every preset).
- `a_band_is_as_wide_as_its_host_and_shades_like_a_row_on_hover` (every preset: 400 px in a 400 px host, rest and hover equal a `button`'s), `a_band_toggles_its_host_once_a_press`.
- `a_count_never_widens_its_row_however_long_the_name` (three names, one an 80-character path with no spaces: the 300 px row stays 300, the count keeps the width it has alone and neither leaves the row), `a_long_name_wraps_inside_its_own_column`, `a_count_keeps_one_line_and_a_name_wraps_by_the_skin`.
- `a_label_in_a_viewport_pane_stands_on_the_point_its_camera_projects`.

The API is new, so the file does not compile before this change: its first run reads `error[E0425]: cannot find function `glyph_button` in this scope` (and the same for `edge_tab`, `band`, `fill_text` and `count_text`).
