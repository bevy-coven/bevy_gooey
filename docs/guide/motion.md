# Motion

A dialog's card rises into place and a menu opens out of its anchor, instead of appearing whole in
one frame. There is nothing to write for it, and one setting turns it all off.

```rust,ignore
// A game's "Reduce motion" option:
commands.insert_resource(MotionSettings::off());
```

## What moves

| What | How | How long |
|---|---|---|
| A dialog's card (a `panel()` or the palette's card inside a `dialog()` scrim) | rises `DIALOG_SLIDE_PX` (12 px) | 0.18 s |
| A popover's panel (`popover`, `popover_modal`, menus, context menus, select lists, the colour picker) | opens `POPOVER_SLIDE_PX` (6 px) closer to its anchor, then settles at its gap | 0.12 s |

Both ease out (`model::ease_out`: it starts quickly and settles, which is how a thing that arrives
looks right) and run on real time, so a menu opened in a paused game still moves. A tooltip, a toast
(which has its own room that opens) and a context menu at the pointer do not slide.

Only the picture moves. Layout and focus are the final ones from the first frame: focus lands in a
dialog in the frame it opens and `ComputedNode` sizes never change; the hit target moves with what is
drawn, so a click in the middle of a slide lands on what the player sees. A popover is drawn a frame
after it is spawned (Bevy lays its text out first); the slide's clock waits for that frame, so all of
it is seen.

## The one setting

`MotionSettings { duration_scale }` is a resource (serde, `Reflect`; `GooeyStylePlugin` adds it with the
default 1):

- `1.0` as designed; `2.0` takes twice as long.
- `0.0`, or `MotionSettings::off()`, is "Reduce motion": nothing starts, and a slide that is running ends
  in the next frame. WCAG 2.2 SC 2.3.3 (Animation from Interactions) asks that motion triggered by
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

Two queries that find nothing once the slides are spent; the gallery's frame time does not move.
Nothing is written after a slide ends (`tests/motion.rs` counts `Changed<UiTransform>` over thirty idle
frames).

Research and parameters: [`docs/research/motion.md`](https://github.com/bevy-coven/bevy_gooey/blob/main/docs/research/motion.md).
