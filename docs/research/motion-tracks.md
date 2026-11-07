# Motion: springs by duration and bounce, a trace that proves them, tracks that retarget

Slices of one series (bevy_grimoire's feel series, tracking bevy-coven/bevy_grimoire#330):
**#47 (G-40)** the spring in `model`, **#46 (G-39)** `MotionTrace` (feature `testing`), **#48 (G-41)**
retargetable tracks on `UiTransform`. Today's motion is `model::ease_out`: a fixed-length curve that
restarts from its first frame when it is interrupted, and cannot carry a velocity in.

## Sources

- Craft (accessibility): W3C, *Understanding SC 2.3.3 Animation from Interactions*, WCAG 2.2.
- Craft (feel): Hicks, Dickinson, Holopainen, Gerling, *Good Game Feel: An Empirically Grounded
  Framework for Juicy Design* (DiGRA 2018, doi 10.26503/dl.v2018i1.936), in the corpus.
- Shipping practice: Apple, *Animate with springs*, WWDC23 session 10158 (springs are SwiftUI's
  default animation and are used "all over iOS").
- Technical: Rune (`anomalyco/rune` at `f73f72c`, MIT): `crates/rune-motion/src/spring.rs`,
  `layer.rs` (`Eps`, `settle_after`), `props.rs` (`PropClass::eps`), `tokens.rs`,
  `crates/rune/src/motion_gate/mod.rs` (the recording gate). Read, not copied: this crate's code is
  its own, on `bevy_ui`.

## Craft principle

WCAG 2.2 SC 2.3.3: "Motion animation triggered by interaction can be disabled, unless the animation
is essential to the functionality or the information being conveyed." Every track in this series
therefore ends in the frame it is seen under `MotionSettings::off()`; none is essential.

*Good Game Feel*: "it is not enough to 'sprinkle' a game with elements of juicy design; it is
something that needs to be approached from a holistic perspective." One spring palette
(`MotionTokens`), not a duration per widget, is how a whole interface gets one feel.

## How shipping games do it

Apple, WWDC23 10158: "if an object's velocity suddenly changes, that also feels unnatural ... So one
goal we have is to make our animations have continuous position and velocity", and "Sometimes, while
an animation hasn't yet finished, a new animation begins that changes to a new target value. When that
happens, a spring animation uses the velocity it had when it was retargeted as the initial velocity
towards its new destination and this same velocity preservation makes these kind of interruptions feel
smooth and natural." And: "When you're not sure, use a spring with bounce 0 ... you should be cautious
about using values higher than around 0.4, since they may feel too exaggerated for a UI element."
The conversion: `stiffness = (2π ÷ duration)²`, damping ratio from bounce.

## Model adopted

- `Spring { duration, bounce }`: `ω = 2π / duration`; `ζ = 1 − bounce` for `bounce ≥ 0`,
  `ζ = 1 / (1 + bounce)` below. Rune's doc comment: "`Spring::new(duration, bounce)` maps exactly like
  SwiftUI's `Spring(duration:bounce:)`". Mass 1. Closed form for the three regimes (under-, critically
  and over-damped, critical within 1e-4 of ζ = 1), exact at any `dt`, so stepping at 60 Hz and at 120 Hz
  agree and a retarget is a restart from `(value, velocity)` with no jump.
- The state is `(value, velocity)` against a target; `spring_step` advances it by `dt`.
- Rest rule (Rune `PropClass::eps`): position within `1/128` px (below the 1/64 px snap quantum, so the
  final snap is invisible), speed within ten times that per second. `spring_settle_seconds` is the
  time the closed-form envelope needs to get there from a given state; it is what the trace and the
  tracks both use.
- `MotionTokens { snappy 0.25 s, default 0.3 s, calm 0.4 s, lead 0.2 s (bounce 0), fade_seconds 0.15 }`
  is `GooeySkin::motion`; `MotionSettings::duration_scale` multiplies every duration.
- `MotionTrace` records the entity's presented pose frame by frame under a manual clock and answers
  the questions Rune's motion gate answers by hand: did it step, did its speed snap, when did it
  settle, did it write at rest.
- `MotionTarget` (a track): a component with the pose it wants (translation, scale, rotation) and a
  `MotionToken`. A change starts a new spring segment from the *presented* pose and velocity.

## Parameters

| name | value | source |
|---|---|---|
| snappy / default / calm / lead | 0.25 / 0.3 / 0.4 / 0.2 s, bounce 0 | Rune `tokens.rs` palette named in the issue |
| fade | 0.15 s | same |
| rest position | 1/128 px | Rune `props.rs` `PropClass::eps` |
| rest speed | 10 × rest position per s | same |
| bounce range | open (−1, 1) | Apple, "values between -1.0 and 1.0"; Rune |
| critical band | 1e-4 around ζ = 1 | Rune `layer.rs` |

## Validation targets

- A 0.3 s bounce 0 spring 0 → 100 never exceeds 100 and is at rest by `spring_settle_seconds`.
- Bounce 0.3 overshoots once; bounce −0.5 never overshoots and is behind bounce 0 at 0.15 s.
- Retarget at 0.1 s keeps value and velocity continuous to 1e-4; 60 Hz and 120 Hz agree to 1e-4.
- `MotionTrace` reports `max_step_px() == 100` for a node moved 100 px in one frame (a check that can
  fail), a settled dialog writes 0 times over 30 frames.
- A dialog card reopened 0.05 s into a close has no frame step larger than the largest of an
  uninterrupted open.

## What great looks like

A dialog that rises and arrives with no "stop" frame, and that, if the player closes and reopens it,
reverses *from where it is* instead of snapping to its start. No visible bounce on utility UI.

## Deviations

- Rune's `Spring::new` panics on a bad value; this crate's refuses with `Result` (no panicking
  constructor in library code, `AGENTS.md`). A deserialised skin can still hold a bad spring, which is
  `SkinProblem::BadMotion`.
- Rune's `still_on_first_frame` has no equivalent word; here it means the entity's pose does not
  change between its first and second recorded frame (nothing animates merely because it appeared).
  A dialog card's entrance starts on its first frame, by design, so its test asserts the start pose
  instead.
- Rune's rest rule has an absolute speed limit (ten times the position tolerance a second), so a spring
  stretched to twice its length settles *sooner* than twice as late (measured: 0.63 s against 1.17 s
  for a 100 px move, 3 frames short of "doubles ±1 frame"). The crate's tracks, the popover slide and
  the meter use `model::spring_settled`, whose speed limit is `tolerance × ω` (it would move less than
  the tolerance in the time it takes to turn): scale-free, so `duration_scale` doubles the settle time.
  `spring_at_rest` keeps Rune's absolute rule as the issue names it.
- The meter's gain was an exponential ease (τ 0.08 s); it is now `motion.snappy`, which takes about
  twice as long to arrive (0.25 s perceptual, ≈0.45 s to rest) and carries its speed into a higher
  target. `MeterShown` gained `speed` and `aim`: with a bounce above 0 the fill passes its target, and
  coming back to the *same* target is not a loss (a lower target is).
- Bounce ≥ 0.4 is allowed by the type (Apple allows it) but the palette never uses it.

## Measured

(Numbers from this pass's tests, snappy spring 0.25 s, bounce 0, a 30 ms frame.)

- A dialog card rise of 12 px: first drawn frame 9.9 px low, settled (`spring_settle_seconds`) in about
  0.44 s; the old ease-out was 0.18 s. The largest frame step of an open is 2.1 px; reopening a card
  0.05 s into a close has no larger step.

## Pending index

home-still's MCP bridge answered 401 (refresh token expired) this pass; the CLI search worked and
returned *Good Game Feel*. WWDC23 10158, WCAG 2.3.3 and the Rune source were read directly and are not
indexed.
