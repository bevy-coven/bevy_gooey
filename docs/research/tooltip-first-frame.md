# Bevy's `position_popover` panics on a widget's first frame

Researched 2026-11-09 (iteration 98). No issue is filed. A **bug found by the loop itself**: the GIF
re-record in iteration 97 caught `examples/inspector` crashing every time it starts.

## The defect, measured

```
$ GOOEY_FRAMES=30 target/release/examples/inspector
thread 'Compute Task Pool' panicked at bevy_math-0.20.1/src/rects/rect.rs:100:
Rect size must be positive
…
free(): invalid size
```

Backtrace, with no `bevy_gooey` frame anywhere in it:

```
<&mut bevy_ui_widgets::popover::position_popover as FnMut<…>>
```

Bevy's `bevy_ui_widgets/src/popover.rs:146-150` computes

```rust
let parent_size = parent_node.size() - parent_node.border.min_inset - parent_node.border.max_inset;
let parent_rect = scale_rect(Rect::from_center_size(parent_transform.translation, parent_size), …);
```

with no guard, and `Rect::from_center_size` asserts `0 <= size.x && 0 <= size.y`. On the frame a widget is
spawned its `ComputedNode` is one frame behind layout, so `size - border` is negative and Bevy panics.
The inspector attaches `Tooltip::new(docs)` to every group and every field row (60 of them), so it hits
this on frame 1 without any input.

## Sources

1. Bevy 0.20, `bevy_ui_widgets/src/popover.rs:130-150` (technical): the unguarded subtraction, and
   `bevy_math/src/rects/rect.rs:99-100` (technical): the assertion that fires.
2. WCAG 2.2 SC 4.1.3 and the crate's own accessibility rule (`src/access.rs`): a tooltip is part of the
   reading of a control, so a widget that cannot survive its own tooltip on its first frame is worse than
   one with none.
3. *Good Game Feel* — Hicks et al., 2018, DiGRA (craft): a game that crashes on a menu screen has no feel
   to judge. The crate's promise is that its widgets are safe to spawn; this breaks that promise through
   no fault of the game.

## Model adopted: **not yet**. Three attempts, all reverted

The right shape is "a box has no `Popover` until its anchor is laid out; `apply_skin` writes it then". Each
attempt failed a different way, and the failures are the value of this note:

1. **Skip the whole `restyle` when the anchor is unsized.** Broke nothing visible but also never styled the
   box: `Role::Popover`'s `Display::Flex` is written inside `restyle`, so a skipped restyle leaves the box
   unstyled and `show_tooltips`' `Display` change has nothing to show.
2. **Take the `Popover` off and `return` from the placement block.** The box latched off permanently:
   `Changed<ComputedNode>` is a filter on *the box*, and the node that changes is its *parent*, so nothing
   re-ran `apply_skin` for the box. `tests/tooltip.rs::focus_opens_a_tooltip_beside_its_widget_clear_of_its_next`
   caught it (`-245 px, wanted 27`).
3. **Take the `Popover` off, and force a restyle from a new system that `set_changed`s the box's `Role`
   when the anchor is sized.** Closer — the box got a `Popover` back — but the placement was the wrong
   one: `tests/tooltip.rs::in_a_row_a_tooltip_opens_below_clear_of_its_neighbours` measured the box on the
   widget's own top edge instead of below it, which is Bevy's *default* `Popover`, not the skin's
   `tooltip_placement`. So the re-added component was the `Popover::default()` the box is spawned with
   (`src/tooltip.rs:175`), not the one `apply_skin` computes — the `set_changed` forced a restyle that ran
   `Role::Popover`'s arm but the `floating` block's condition did not see the box as one of its own.

**The next slice, with the diagnosis in hand:** spawn the tooltip box **without** `Popover::default()`
(`src/tooltip.rs:175`) and give it one only from `apply_skin`'s `floating` block. Then a box has no
`Popover` until the skin writes a correct one, Bevy never sees an unsized anchor, and there is no
remove-and-restore path at all — one writer, no latch, no default to overwrite. That also deletes a
`Popover::default()` whose placement nothing ever wanted.

## Validation targets for that slice

- `examples/inspector` runs to its capture frame with no panic, ten runs in a row.
- `tests/tooltip.rs` (10 tests), `tooltip_delay` and `tooltip_cap` stay green.
- A tooltip opens beside its widget, below it in a row, and flips at the window's edge — the three pinned
  behaviours above.

## Deviations

- The crate does not fork or patch Bevy. The guard has to be "do not give Bevy a `Popover` whose anchor it
  cannot measure", which is this crate's business anyway (it is the crate that hangs the box off the
  widget).

## Pending index

- Nothing new. This is a defect note, not a research claim.
