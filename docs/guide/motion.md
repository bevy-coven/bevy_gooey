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

| What | How | On |
|---|---|---|
| A dialog's card (a `panel()` or the palette's card inside a `dialog()` scrim) | rises `DIALOG_SLIDE_PX` (12 px) | `motion.snappy` |
| A popover's panel (`popover`, `popover_modal`, menus, context menus, select lists, the colour picker) | opens `POPOVER_SLIDE_PX` (6 px) closer to its anchor, then settles at its gap | `motion.snappy` |
| A `meter()` that loses value | the fill drops to the new value in that frame; a `Role::MeterTrail` in `danger` keeps the lost length, holds, then drains to the fill | hold `METER_TRAIL_HOLD_SECONDS` (0.45 s), then `METER_TRAIL_RATE` (1.5 bars a second) |
| A `meter()` that gains value | the fill rises to it; no trail | `motion.snappy` |

Every rise is a spring ([`Spring`](https://docs.rs/bevy_gooey/latest/bevy_gooey/model/struct.Spring.html):
a perceptual duration and a bounce, SwiftUI's mapping), not a fixed-length curve: it starts quickly,
settles with no end the eye can see, and if it is interrupted it carries on from where it is drawn
and how fast it is going. They run on real time, so a menu opened in a paused game still moves. A
tooltip, a toast (which has its own room that opens) and a context menu at the pointer do not slide.

Only the picture moves. Layout and focus are the final ones from the first frame: focus lands in a
dialog in the frame it opens and `ComputedNode` sizes never change; the hit target moves with what is
drawn, so a click in the middle of a slide lands on what the player sees. A popover is drawn a frame
after it is spawned (Bevy lays its text out first); the slide's clock waits for that frame, so all of
it is seen.

## The skin's springs

`GooeySkin::motion` is the palette every motion draws from, so a whole interface has one feel:

| Token | Duration | Bounce | For |
|---|---|---|---|
| `snappy` | 0.25 s | 0 | small controls, entrances, pop-ups |
| `default` | 0.3 s | 0 | the ordinary state change |
| `calm` | 0.4 s | 0 | larger things that arrive |
| `lead` | 0.2 s | 0 | a thing that leads another |
| `fade_seconds` | 0.15 s | | an opacity change |

The duration is *perceptual*, the period of the undamped spring; the time to come to rest is longer
(`model::spring_settle_seconds`: about twice it with no bounce). Bounce 0 arrives and stops; above 0
overshoots, below 0 is flatter. A spring that has no motion (a duration of zero, a bounce outside -1
to 1) is `SkinProblem::BadMotion`. A theme step (`with_colours_of`) leaves the palette alone.

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

## Your own panels: `MotionTarget`

```rust,ignore
// A side panel that arrives from the left, 24 logical px.
commands.spawn((
    Node::default(),
    MotionTarget::default().entering_from(Vec2::new(-24.0, 0.0)),
));
// Later, wherever it is drawn and however fast it is going, it turns to the new place:
commands.entity(panel).insert(MotionTarget { translation: Vec2::new(300.0, 0.0), ..default() });
```

`MotionTarget { translation, scale, rotation, spring, enter_from }` on any `Node` is a track on its
`UiTransform`. A change of any field starts a new spring segment from the *drawn* pose and its
velocity, so a second open, a key repeat or an interrupted close never jumps. The first target a node
has lands where it says (no motion on spawn) unless `enter_from` gives it a place to come from;
`InstantMotion` on the node lands its next change at once. A track at rest is skipped before anything
is read, and writes nothing. It follows `MotionSettings`; a game that writes `UiTransform` itself while
a track runs wins (the track is dropped); and the crate never replaces a `MotionTarget` a card already
carries. Do not put one on a `Popover`: Bevy's `Popover` measures where its panel is and corrects
`UiTransform` every frame, which would undo it (the crate's own popovers slide by their gap to the
anchor instead, `PopoverSlide`).

## Testing motion

With the `testing` feature, `testing::MotionTrace` records a node's pose frame by frame on a manual
clock and answers what a hand-sampled test cannot: `max_step_px()` (no frame jumps), 
`max_speed_change_px_per_s()` (no snap in the speed), `settled_after_seconds()` and
`writes_at_rest(frames)` (a settled interface writes nothing).

## Cost

Two queries that skip a settled track before reading it, and a meter that is settled and unchanged is
skipped before any node is touched; the gallery's frame time does not move. Nothing is written after a
spring or a trail ends (`tests/motion.rs` counts `Changed<UiTransform>` and `tests/meter_motion.rs`
`Changed<Node>` over thirty idle frames).

Research and parameters: [`docs/research/motion.md`](https://github.com/bevy-coven/bevy_gooey/blob/main/docs/research/motion.md) and, for the meter, [`docs/research/meter-motion.md`](https://github.com/bevy-coven/bevy_gooey/blob/main/docs/research/meter-motion.md).
