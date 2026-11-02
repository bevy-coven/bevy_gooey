# Motion

A dialog's card rises into place and a menu opens out of its anchor, instead of appearing whole in
one frame, and a HUD meter that loses health leaves the lost length behind its fill before it drains.
There is nothing to write for any of it, and one setting turns it all off.

```rust,ignore
// A game's "Reduce motion" option:
commands.insert_resource(MotionSettings::off());
```

![The gallery's Quit dialog opening twice over the settings screen: the card rises a few pixels into place, settles, vanishes on Escape and rises again on Enter](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/motion.gif)

![The gallery's Health meter: a hit drops the fill at once and leaves the lost length in red, which holds and drains; a heal eases the fill up with no red](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/meter.gif)

## What moves

| What | How | How long |
|---|---|---|
| A dialog's card (a `panel()` or the palette's card inside a `dialog()` scrim) | rises `DIALOG_SLIDE_PX` (12 px) | 0.18 s |
| A popover's panel (`popover`, `popover_modal`, menus, context menus, select lists, the colour picker) | opens `POPOVER_SLIDE_PX` (6 px) closer to its anchor, then settles at its gap | 0.12 s |
| A `meter()` that loses value | the fill drops to the new value in that frame; a `Role::MeterTrail` in `danger` keeps the lost length, holds, then drains to the fill | hold `METER_TRAIL_HOLD_SECONDS` (0.45 s), then `METER_TRAIL_RATE` (1.5 bars a second) |
| A `meter()` that gains value | the fill eases up (an ease-out with a 0.08 s time constant); no trail | about 0.24 s |

Both ease out (`model::ease_out`: it starts quickly and settles, which is how a thing that arrives
looks right) and run on real time, so a menu opened in a paused game still moves. A tooltip, a toast
(which has its own room that opens) and a context menu at the pointer do not slide.

Only the picture moves. Layout and focus are the final ones from the first frame: focus lands in a
dialog in the frame it opens and `ComputedNode` sizes never change; the hit target moves with what is
drawn, so a click in the middle of a slide lands on what the player sees. A popover is drawn a frame
after it is spawned (Bevy lays its text out first); the slide's clock waits for that frame, so all of
it is seen.

## A meter's trail

```rust,ignore
commands.spawn(meter(100.0, 100.0)).insert(Health);
// later: a hit for 40
meter.value -= 40.0;
```

Change `Meter::value` and the bar does the rest (`model::meter_motion` is the maths, with no engine type).
The fill is always exact: on a loss it is at the new value in the frame you wrote it, never lagging
behind the truth, and the trail is the extra, saying how much was taken. A second hit during the hold
does not restart it (a stamina bar that drains every frame would hold forever otherwise). The first
frame of a meter is its value, not a rise from empty. A meter with no `Time<Real>` (a host without the
clock) jumps as before. The trail is a second child of the track, zero wide at rest, so a settled
HUD draws and writes exactly what it did before. The gallery stages it: `GOOEY_KEYS=hit,hit,heal`.

## The one setting

`MotionSettings { duration_scale }` is a resource (serde, `Reflect`; `GooeyStylePlugin` adds it with the
default 1):

- `1.0` as designed; `2.0` takes twice as long.
- `0.0`, or `MotionSettings::off()`, is "Reduce motion": nothing starts, and a slide or a trail that is
  running ends in the next frame. WCAG 2.2 SC 2.3.3 (Animation from Interactions) asks that motion triggered by
  interaction can be turned off unless it is essential, and an entrance never is.

The crate cannot read the operating system's own reduced-motion preference (Bevy has no way to), so a
game wires its own option, or reads the platform's and sets this.

## Your own panels: `SlideIn`

```rust,ignore
// A side panel that arrives from the left, 24 logical px over a fifth of a second.
commands.spawn((
    Node::default(),
    SlideIn::new(Vec2::new(-24.0, 0.0), 0.2),
    // ..
));
```

`SlideIn { from, seconds }` on any `Node` eases its `UiTransform::translation` from `from` (logical
px, positive `y` down) to nothing, then takes itself off. It writes the translation only, so a scale of
your own on the same node stays; it follows `MotionSettings`; and the crate never replaces one a card
already carries. Do not put it on a `Popover`: Bevy's `Popover` measures where its panel is and
corrects `UiTransform` every frame, which would undo it (the crate's own popovers slide by their gap
to the anchor instead, `PopoverSlide`).

## Cost

Two queries that find nothing once the slides are spent, and a meter that is settled and unchanged is
skipped before any node is touched; the gallery's frame time does not move. Nothing is written after a
slide or a trail ends (`tests/motion.rs` counts `Changed<UiTransform>` and `tests/meter_motion.rs`
`Changed<Node>` over thirty idle frames).

Research and parameters: [`docs/research/motion.md`](https://github.com/bevy-coven/bevy_gooey/blob/main/docs/research/motion.md) and, for the meter, [`docs/research/meter-motion.md`](https://github.com/bevy-coven/bevy_gooey/blob/main/docs/research/meter-motion.md).
