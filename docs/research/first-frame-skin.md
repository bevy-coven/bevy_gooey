# A node is skinned for the target it is about to be laid out in

Researched 2026-11-14 for issue #94 (G-83), asked by bevy_dark_factory (the Cauldron), issue #336 (UI-R06, the shell moves), tracking #330. Found by measurement in its `a_dock_tab_reordered_glides_to_its_place.rs` (`the_tab_behind_the_glide_lands_at_once_because_the_strip_is_skinned_a_frame_late`): when a dock strip's tabs are rebuilt and reordered, only the leading tab glides to its new place and the tab behind it lands in two frames.

## Sources

1. `src/layout_motion.rs` (this crate): a move lands, with nothing travelling, in a node's first `LAYOUT_QUIET_FRAMES` (2) frames, because a fitting pass (a toolbar's overflow, a status bar hiding what does not fit) reads one frame's layout and moves things in the next. `update_layout_motion` calls `travel.land()` for such a move, which also **zeroes a glide already under way**.
2. Bevy 0.20.1 `bevy_ui` `ui_node.rs` and `update.rs`, read from the registry source: `Node` **requires** `ComputedUiRenderTargetInfo`, whose `Default` is `scale_factor` 1 and `physical_size` 0. The real value (the camera target's scale factor times `UiScale`, and the viewport's physical size) is put on the root by `propagate_ui_target_cameras` and propagated down by `HierarchyPropagatePlugin` in `PostUpdate`, so a node spawned in `Update` has the placeholder until that frame's layout has already run.
3. `src/skin.rs` (this crate): `restyle` took `target.map_or(1.0, |t| t.scale_factor())` and, for a screen's panel, a popover, a menu panel and the radial wheel, `target.logical_size()`.

## The cause

Reproduced with the issue's skin (11 px text, a 1 px line) at `UiScale` 1.2, `tests/strip_glide.rs`:

1. A dock's rebuilt strip spawns new tabs in `Update`. `apply_skin` runs the same frame and reads the placeholder target: text 11.0 px, frame 1.0 px (the scale-1 values) and, for the panel and popover caps, a window width of 0.
2. `PostUpdate` gives the nodes their real target (scale 1.2) and lays the strip out: the first layout.
3. Next frame `Changed<ComputedUiRenderTargetInfo>` restyles every new node at scale 1.2: text 11.67 px, frame 0.83 px. The leading tab grows, its neighbours' corners move, and the move falls inside the first `LAYOUT_QUIET_FRAMES`, so `land()` zeroes the glide the second tab had just been given by `TravelFrom`.

Without the fix the trace of Alpha after the reorder at 1.2 is `[32.1, 41.0, 103.75, 103.75, ...]`: 62.7 of its 71.7 px in the second frame. The skin wrote `font 11.0 -> 11.666666`, `border 1.0 -> 0.8333333` and the panel was `[34, 302]` on frame 1 and `[378, 254]` on frame 2 (its `max_width` was the window's width, 0, for one frame).

## Model adopted

The issue offered three fixes: skin before the first layout, a node hidden from layout until skinned, or `LAYOUT_QUIET_FRAMES` counting from the first skinned layout. The first is built, because it removes the cause and the others would hide it:

- **`restyle` takes a render target it can trust.** A node whose `ComputedUiRenderTargetInfo` has no physical size is on the placeholder: it is skinned at the target it is about to have, which `apply_skin` finds as the nearest ancestor's real one (a rebuilt strip's own container has it), else the primary window's scale factor times `UiScale` and its size over that scale (a root spawned this frame). With no window and no real ancestor it is as before: scale 1, no cap.
- **Everything `restyle` read from the target uses it:** the text and line snapping (`scale`), a screen's panel's width cap, a popover's and a menu panel's width cap and the radial wheel's size. The wheel had been 0 and the panels capped at 0 on the first frame.
- **`LAYOUT_QUIET_FRAMES` is untouched.** It still covers a fitting pass; the skin's first look is no longer a layout change for it to cover.

## Proof

`tests/strip_glide.rs`, 4 tests on a 30 ms clock:

- `every_tab_of_a_reordered_strip_glides_at_interface_size_one` and `..._one_point_two`: a dock group of three tabs is reordered (a, b, c to c, a, b); every tab that has somewhere to go takes at least 6 frames to arrive and never covers more than half its distance in one frame.
- `every_tab_glides_when_the_order_is_reversed`: the same at 1.2 for c, b, a.
- `a_freshly_spawned_skinned_widget_keeps_its_look_on_the_second_frame`: a heading, a label, a button and a strip of three tabs with their pages are spawned at interface sizes 1, 1.2 and 2; every node's `ComputedNode` size, border, padding and text size are the same on frame 1 and frame 2.

Failing first (the placeholder scale kept): the 1.2 reorder test reads `UiScale 1.2: Alpha covered 62.748432 of its 71.66667 px in one frame`, the reversed one `reversed: Alpha covered 113.7502 of its 130 px in one frame`, and the frame test lists every node whose border went from 1 to 0.8333333, whose text went from 11 to 11.666666 and whose height went from 190 to 164 (the panel).

## Not changed

- `LAYOUT_QUIET_FRAMES`, `update_layout_motion` and `TravelFrom`: the glide was right and the layout input was not.
- A host that sets its own `ComputedUiRenderTargetInfo`-dependent values is unaffected: only `restyle`'s reads of the target changed.
