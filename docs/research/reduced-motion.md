# Reduced motion keeps fades

Researched 2026-11-07 for issue #50 (G-43), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune/README.md`, "Animation": "In reduced motion, spatial travel snaps while opacity, presence and colour take `REDUCED_FADE` (`0.15 s`)." Elsewhere in the same README, a keyframe spec and an explicit settle "land the final pose immediately", and a shared-element match is skipped: the old element exits and the new one enters.
2. Rune, `crates/rune-motion/src/props.rs:44,73`. The code says `REDUCED_FADE = Curve::Tween { duration: 0.18, easing: EASE_IN_OUT }`, and every `PropClass::Opacity | Color | Presence` takes it under reduced motion. That is a fixed-length ease, not a spring. The README says 0.15 s; the code says 0.18 s.
3. Rune, `crates/rune-motion/README.md` (quoted in `docs/research/presence.md`): "Under reduced motion, travel, scale, shape and blur change instantly, and opacity, colour and presence take a 0.18 s ease-in-out fade."
4. WCAG 2.2 SC 2.3.3, Animation from Interactions: motion triggered by interaction can be disabled unless it is essential.

## Model adopted

- **One resource, two answers.** `MotionSettings { duration_scale, reduced: bool }` (serde default `false`), with `MotionSettings::reduced()` beside `off()`.
  - `lands_travel()` is `reduced || is_off()`.
  - `duration_scale` 0 is still off and wins over reduced: nothing starts, fades included.
- **What lands.** Under reduced, the crate's travel lands:
  - every `MotionTarget` track lands in the frame it is seen, and so does one already running (`update_motion`);
  - a popover's slide lands, and the trail behind a meter's fill lands with the fill (`update_meters`);
  - a dialog's card and a popover panel get no slide.
  - A spinner keeps turning and a hold bar keeps filling and draining (`hold_step`'s `instant` is still "off"): they are progress the player is watching, not travel.
- **What still fades.**
  - A `Leave` exit fades on `motion.fade_seconds` exactly as at full motion (`update_presence` is unchanged).
  - The entrance that would have travelled becomes a fade in place. `begin_entrances` gives the node `Arriving` with an `Opacity` of 0. `update_arrivals` raises it to 1 on a fixed-length ease-in-out (smoothstep) over `fade_seconds × duration_scale`, then takes both off, so the node is drawn whole and a settled interface writes nothing.
  - A node that already carries a host's own `Opacity` gets no arrival, and a host that writes one during the fade takes it over: the fade stops and the host's value stays. A fade length that is not a finite number above zero lands at once.
- **Turning it on mid-flight** lands what travels in the next frame and lets a fade finish.
- **Interrupting an arrival.** A `Leave` during it exits from where the fade had got to. The arrival's opacity is not kept as the node's own, so a `Stay` brings it back whole.
- **The option row.** `motion_row(label)` is a `choice` over Full, Reduced and Off (`MOTION_STEPS`), beside `text_size`.
  - A step writes `MotionSettings` and nothing else.
  - Full and Reduced keep the player's own `duration_scale`, 1 when coming from Off. Off is `MotionSettings::off()`.
  - A hand-written value shows the step it is in (`motion_step`). A host that localises the words writes its own `choice` with `with_motion_step`.

## Where this differs from Rune

- **The fade length is the skin's `fade_seconds` (0.15 s, the issue's figure and Rune's README), not Rune's code value of 0.18 s.** It is one token for every opacity change in this crate, and a skin tunes it.
- **The fade-in is a timed ease-in-out, as Rune's `REDUCED_FADE` is.** An exit keeps the presence spring that full motion uses. That spring's length is also `fade_seconds`, and it falls below 1/255 at about 0.177 s (measured in `docs/research/presence.md`).
  - A timed fade lasts exactly what it says, which is what lets the acceptance hold to ±1 frame.
  - The exit was left on its spring so that `Stay` keeps turning a fade round with its velocity.
- **Colour.** Rune also fades colour changes under reduced motion. This crate has no colour motion yet (G-44, #51), so a colour change is instant at every setting; G-44 takes `fade_seconds` for reduced motion when it lands.
- **Toasts.** A toast's own fade and room-opening run on `ToastSettings` (`duration_scale`, `fade_seconds`), a separate setting the toast guide describes, and are unchanged here.

## Proof

`tests/reduced_motion.rs`, 9 tests, at 60 Hz:

- A dialog's card under reduced: no slide and no offset on its first frame, opacity below 0.2 there, never falling, and whole after 0.15 s ±1 frame, with the skin's fill alpha.
- A popover opens with no `PopoverSlide` and fades in standing still, over the fade (it waits its hidden first frame).
- Turning reduced on mid-slide lands the slide in the next frame, while an exit already under way keeps fading and despawns.
- A `Leave` during the fade-in leaves from where it had got to, and a `Leave` then a `Stay` brings the card back whole with no `Opacity` left on it.
- A host's `Opacity(0.5)` written during the fade-in is kept.
- A meter's gain is at its new value in the frame it changes.
- `{ duration_scale: 0, reduced: true }` draws the card whole on its first frame.
- The row steps Full → Reduced → Off with two writes of `MotionSettings` and none of any other setting. Home goes back to Full at the designed lengths, and a setting the game writes moves the row with no event.

On 8c68c5b, which has the field and no behaviour, the five first behaviour tests that were run there (dialog, popover, mid-flight, leave during the fade, meter) fail and the off-wins test passes, as it should; the review's two edge-case tests came after.
