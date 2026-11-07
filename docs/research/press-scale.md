# Press scale

Researched 2026-11-07 for issue #61 (G-54), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune/src/el/visuals.rs:241-250`, `El::press_scale`: "Shrink to `scale` about the centre while a press on this element is held, as iOS tiles and buttons do (0.96 to 0.97), springing back on release or when the pointer leaves. Layout and hit testing keep the resting size, and Reduce Motion turns it off."
2. Rune, `crates/rune/src/runtime/input/hover.rs:360-361`: the press animates on `motion::INSTANT` and the release on `motion::POP`.
3. Rune, `crates/rune-motion/src/tokens.rs:12,30`: `INSTANT = Spring::new(0.1, 0.0)` and `POP = Spring::new(0.25, 0.15)`.
4. Rune, `crates/rune/src/runtime/tests/picture_scale.rs:191-205`: a row scrolled away under the held press springs back.
5. Bevy 0.20, `bevy_ui_widgets/src/button.rs:60-75`: a `Button` triggers `Activate` on `Pointer<Click>` only while it has `Pressed`.

## Model adopted

- **`PressScale(f32)`**, a public component, defaults to `PRESS_SCALE` (0.97). It requires a public `PressScaled`, whose `scale()` is the scale the widget is drawn at now.
- **The target.** `update_press_scale` (Style) springs the scale toward the factor while the widget has `Pressed`, the pointer is over it (`Hovered`) and the press was not cancelled; otherwise toward 1.
  - Going in uses the skin's `motion.snappy`, as the issue asks. Coming back uses a spring of snappy's length with Rune's `POP` bounce, 0.15 (`PRESS_RELEASE_BOUNCE`). Both are times `duration_scale`.
  - Under `lands_travel()` (reduced or off) the scale is 1.
- **Only the drawing scales.** `draw_press_scale` runs in `PostUpdate`, after layout and layout motion and before clipping. It writes the `UiGlobalTransform` of the widget and its subtree scaled about the widget's centre, and keeps the resting values in `PressScaleRestore`. `restore_press_scale` runs in `First` and puts them back before picking reads them.
  - So layout and the hit test keep the resting size, as the issue asks, and the scale is the whole widget's: its frame, its fill and its words together.
  - The issue proposed an inner content node. The crate's cards and slots have none, and scaling only the children would leave the frame and fill at full size. The restore gives the resting hit test without a node.
- **A scroll cancels.** At the start of a press the widget remembers where every ancestor `ScrollPosition` stands. When one has moved, it takes `Pressed` off the widget, and Bevy's `Button` then sends no `Activate` on the click. A wheel tick at a list's end writes an unchanged value and cancels nothing. The widget springs back.
- **Only a primary press.** `Pressed` comes from any pointer button in Bevy 0.20, so the press also asks `PointerPressState::is_primary_pressed`: a right-click for a context menu does not shrink the card.
- **A popover stays put.** The subtree walk skips a `FixedNode` (a popover, a context menu's pointer point), as layout motion does, so a menu hung from a card is never drawn shrunk.
- **Which widgets.** `mark_press_scale` gives every `Role::CardCell`, `Role::Slot` and `Role::SlotEmpty` the default. Buttons do not get it, as in Rune. A game can put it on any node with a `Pressed` state; `PressScale` requires `Hovered`, so a plain Bevy `Button` works too.
- **Not covered:** a click cancelled by a scroll can still be the second of a `DoublePress` pair. The double-press counter pairs clicks, not activations.
- **At rest** a widget is read and not written, and `PressScaleRestore` stays empty.

## Where this differs from Rune

- **Going in uses `motion.snappy` (0.25 s)**, as the issue specifies, where Rune uses `INSTANT` (0.1 s). The tile reaches 0.97 ± 0.005 in about 0.13 s on snappy.
- **A press is a `Pressed` component.** A key's Enter on a card activates it with no press scale, as before.

## Proof

`tests/press_scale.rs`, 6 tests, at 60 Hz with real pointer input:

- A card held for `motion.snappy` is at 0.97 ± 0.005 and drawn at that scale, with its `ComputedNode` size unchanged. Its release chooses it, and it is back at 1 after a second.
- A press 0.5 px inside the card's edge keeps the card hovered and pressed on each of 30 frames, and it reaches 0.97. Without `restore_press_scale` this test fails at frame 1, because the shrunk drawing slips out from under the pointer.
- A scroll of the view during the hold: the release chooses nothing, and the card springs back.
- Under reduced motion a held card stays at 1.
- A button has no press scale, and 30 idle frames write no `PressScaled`.
- A slot grid's full and empty slots both have press scale; a secondary press leaves a slot at 1, and a primary one shrinks it.
