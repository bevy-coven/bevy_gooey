# Size motion: sections and a split's collapse

Researched 2026-11-07 for issue #53 (G-46), asked by bevy_grimoire's feel series (dark-incorporated/bevy_dark_factory#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune/README.md`, "Animation": layout animates per edge on the default spring, and layout is at the target state.
2. Rune, `crates/rune-kit/src/shell/pane.rs:172,400`: side panes open and close on `Spring::new(0.3, 0.0)`, which is this crate's `motion.default`.
3. bevy_gooey#76: the owner decision (assumed under the owner's standing steer) that lets `tests/split.rs` and `tests/section.rs` run with `MotionSettings::off()`. Those files assert layout a frame after a toggle on the real-time clock.

## Model adopted

- **State at once, layout follows.** `SectionOpen` and `SplitFraction` are the new value in the frame they are asked for, and the `ValueChange` is sent then, as before. Only the laid-out size travels, on `motion.default` times `duration_scale`. It lands at once under `lands_travel()` (reduced or off).
- **A section** (`SectionGlide` on its body):
  - **Starting.** A toggle after the section's first frame, by the player or a game's write, starts a glide in `update_sections`. An opening body joins layout and the Tab order at once, at height 0. A closing body stays in layout until its glide ends.
  - **Running.** `update_section_glides` clips the body (`Overflow::clip()`) and lays it out at a height that springs toward the height its widgets need (`ComputedNode::content_size`, read from its second frame, once it has been laid out) or toward 0. So the widgets under it travel through layout.
  - **Pixels.** The height is set on whole device pixels. Layout rounds each edge on its own, and a fractional height made the widget under it wobble by a pixel as it settled. The glide ends within half a device pixel of its target.
  - **Ending.** An opened body gets its own height and overflow back. A shut one leaves layout and sight (`Display::None`, `Visibility::Hidden`), as a closed body always has.
  - **Toggled again on its way:** the glide turns round from where it is.
  - **Focus** inside a body that closes still moves to the header at once. While it shrinks, the body's subtree takes no pointer and no Tab stop (`Pickable::IGNORE`, `TabIndex` taken off and given back), as a leaving widget does.
  - **What counts as a toggle:** only a state that differs from what the body shows. A game that writes the same value every frame starts nothing.
  - **The target:** the clip is on the vertical axis only (`Overflow::clip_y()`). The height is `content_size` plus the body's border and end padding (a clipped box's overflow leaves them out), or the body's own fixed `Px` height when it has one.
- **A split** (`SplitGlide` on its root):
  - `toggle_split` (Enter or South on the divider, a double click) starts a glide from the fraction the panes are laid out at, which is a glide's own when one is on its way. `update_splits` lays the panes out at the glide's shown fraction.
  - `update_split_glides` steps it, and takes it off when it arrives, when `SplitFraction` is moved anywhere else (a drag, a key, the game: those land), or under reduced motion.
  - **A divider drag stays direct manipulation:** 100 px of drag moves the panes in that frame. A drag that starts during a slide takes the divider from where it is drawn: the glide comes off and the fraction is set there first, so nothing jumps.

## Where this differs from the issue and from Rune

- **A popover whose content height changes** does not spring yet. The issue lists it under the proposed API but not under its acceptance, and nothing in this crate changes a popover's content height while it is open today. It is left for G-66 (the combobox), whose filtering list is the first such popover.
- **A row gap at each end.** A shut section's body is out of the column, and so is the column's row gap above it. That gap appears in an open's first frame and goes in a close's last, so the widget under the section steps by it there. That is 3 px under the default skin, under a third of the move.

## Proof

`tests/size_motion.rs`, 10 tests, at 60 Hz with motion on:

- **Opening:** the body's height rises, one way and no frame over a third of the move, to the height an unanimated open body has. The button under it travels the same way to its unanimated place, and the glide ends within the spring's settle time plus three frames.
- **Closing:** the body shrinks and the button under it travels. At the end the body is `Display::None` and `Visibility::Hidden`, and its height is `Auto` again.
- **Turned round:** toggled again six frames in, the body turns round without a jump and ends shut.
- **Reduced motion:** a section opens at once.
- **Split, Enter on the divider:** the fraction is the collapsed one at once. The first pane slides to its 100 px least with no frame over a third of the move, and the glide comes off.
- **A key step during the slide** takes the glide off and the pane is where the fraction says, in that frame.
- **Reduced motion:** a split collapses at once.
- **Double press:** a `DoublePress` on the divider slides the collapse; it is not there in one frame.
- **Same-value writes:** a game writing the same `SectionOpen` every frame starts no glide and lays nothing out again.
- **Tab stops:** a shrinking body's toggle has no `TabIndex`, and it is given back when the glide turns round.

The protected `tests/split.rs` and `tests/section.rs` (43 tests) pass with `MotionSettings::off()` in their apps.
