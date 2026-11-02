# Meter motion: a fill that eases up and a trail that shows what was lost

Researched 2026-11-02 (iteration 48). `meter()` is the HUD's most-seen widget (health, stamina, a boss bar) and it still jumps: set `Meter::value` and the fill is at its new length in the same frame. A hit that takes a third of the player's health reads as a flicker of a bar, not as a loss of a size. Iteration 47 added motion for panels that arrive and left "a meter that eases to its new value" first in the backlog; Sanctuary's `src/ui/widgets.rs` has only a plain `progress_bar` (read, shallow clone removed), so the owner's projects hand-roll nothing here yet.

## Sources

1. Pichlmair and Johansen, *Designing Game Feel: A Survey* (IEEE Transactions on Games, 2021; home-still stem `10.1109_tg.2021.3072241`) — craft, on how much juice a situation takes.
2. Hicks, Dickinson, Holopainen and Gerling, *Good Game Feel* (home-still stem `10.26503_dl.v2018i1.936`) and its analysis of *Downwell*, a shipped game — shipping game, on feedback that is immediate and legible.
3. Page Laubheimer, *Executing UX Animations: Duration and Motion Characteristics*, Nielsen Norman Group (duration and easing); Bevy 0.20 / taffy flex layout (percentage margins resolve against the container's width) for the mechanism — technical.

## Craft principle

The survey reports the one empirical finding on how much: "Kao [36] conducted a large-scale study on the amount of juice appropriate for a specific gaming situation, concluding that juice has to be applied adequately to the situation. In their study, medium and high levels of juiciness outperformed extreme levels and the absence of juiciness across the measures of player experience, intrinsic motivation, play time, and in-game performance." So a bar that jumps (none) is worse than one that moves a little, and one that bounces or flashes (extreme) is worse too: the motion is one trail and one ease, nothing else.

## How shipping games do it

Hicks et al. quote a designer: "When the player presses input to engage the action, the juice makes the action feel impactful and meaningful", and say of *Downwell*: "All player actions lead to immediate visual and auditory response". The convention in shipping action games (a health bar whose lost part lingers in a second colour for about half a second, then drains) obeys that: the bar is exact the frame the hit lands (the fill drops at once, never lags behind the truth), and the lingering chunk is the extra: it says how much was taken. A heal is the opposite direction and gets the opposite treatment, a short rise, because nothing was lost to show. I could not find a stem in home-still that describes the trail itself; this paragraph is the practice, not a citation (listed under `## Pending index`).

## Model adopted

- **M1. The fill is exact on a loss and eases on a gain.** A new `MeterMotion { fill, trail, hold, primed }` (private state, `Meter` requires it) holds the fractions drawn. A value below the drawn fill sets the fill to the target in that frame. A value above it eases up: `fill += (target − fill) · (1 − e^(−dt / τ))` with τ = `METER_GAIN_TAU_SECONDS` (0.08 s, so 95% in 0.24 s, inside NN/g's 100 to 500 ms), snapping when within `METER_SNAP` (0.002) of the target.
- **M2. The trail is the lost length.** On a loss with no trail standing, `trail` keeps the old fill and `hold` starts at `METER_TRAIL_HOLD_SECONDS` (0.45 s). A second loss during the hold does not restart it (a continuously draining meter, a stamina bar, would hold forever otherwise). When the hold has run out the trail drains linearly at `METER_TRAIL_RATE` (1.5 of the length a second, so the whole bar in 0.67 s) down to the fill. The trail is never below the fill (a gain pulls it up with the fill).
- **M3. One node more.** `meter()` spawns a second child, `Role::MeterTrail`, after the fill, in `danger`. It is a flex item in the same row as the fill with `width = trail` and a negative `margin_left = fill` (a percentage margin is of the container's width, as the width is, so the trail starts at the bar's left edge and the sum is the trail's length) and the fill is `ZIndex(1)`, so the trail shows only to the right of the fill. At rest it is zero wide: nothing is drawn. The fill stays the first child.
- **M4. The reduce-motion switch is the same one.** `MotionSettings::duration_scale` multiplies every duration here (a scale of 2 takes twice as long) and at 0 the fill and the trail are the target in the frame it changes: the old behaviour, frame for frame.
- **M5. Real time**, as the spinner and `SlideIn`: `Time<Real>`, so a paused game's HUD still settles. An `Option<Res<..>>`: a host without the clock gets the old jump.
- **M6. Cost.** The state is compared before it is written; a meter whose fractions equal its target and whose `Meter` did not change is skipped before any child is touched, so a settled HUD writes nothing (the crate's idle-write tests).
- **M7. Pure maths in `model`:** `meter_motion(state, target, dt, duration_scale) -> MeterShown` and the four constants, tested without an app.

## What great looks like

A health bar in the Souls and Hades tradition: the accent fill drops to the new value at once, a band of the danger colour stays where the fill was for a beat, then slides left until it meets the fill. A heal slides the fill up smoothly, with no band. At rest the bar is the same picture as today (the stills are pixel-identical). The band is the same height as the fill with the same rounded right end, flush against the fill, and never pokes outside the frame.

## Parameters

| Name | Value | Why |
|---|---|---|
| `METER_GAIN_TAU_SECONDS` | 0.08 | 95% in 0.24 s: NN/g "shortest … without being jarring", ease-out |
| `METER_TRAIL_HOLD_SECONDS` | 0.45 | long enough to read the size of the hit, short enough not to hide the next one |
| `METER_TRAIL_RATE` | 1.5 /s | the whole bar in 0.67 s; a small chip still takes its hold |
| `METER_SNAP` | 0.002 | under a device pixel on any bar a HUD draws |

## Validation targets

1. A loss: the fill is the target in the first frame, the trail starts at the old length, stays for the hold, then falls monotonically to the fill in at most `(old − new) / rate` seconds, and is then zero wide.
2. A gain eases: the first frame is partway, each later frame is nearer, it is at the target within 0.5 s and no trail ever appears.
3. Motion off: fill and trail are the target in the frame the value changes; `duration_scale` 2 takes twice as long.
4. A second loss in the hold does not restart it; a gain during a trail raises the trail with the fill.
5. A settled meter writes nothing for thirty frames; a paused `Time<Virtual>` still settles.
6. Laid out: the trail starts at the fill's left edge (the track's content edge), ends at its fraction of the room, is the fill's height, and the gallery stills are pixel-identical to the starting build.
7. `access.rs` reads the trail as hidden: a meter's tree node is unchanged.

## Deviations

None from a citation. The trail colour reuses `danger` (no new skin colour, so `problems()` is unchanged); the numbers above are design defaults.

## Pending index

No stem in home-still describes the lost-health trail convention itself; the shipping-game paragraph is practice, not a quotation. A game-UI design source on HUD value changes (the Game Accessibility Guidelines page on non-colour cues is the other open item from the corpus note).
