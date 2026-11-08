# Pop

Researched 2026-11-08 (iteration 70). No issue is filed: this is the pass's own pick, from reading what Sanctuary hand-builds (`src/ui/juice.rs`, a `Pop` swell its HUD inserts on a changed number, a focus change, a repaired panel and a wheel selection; `Pop::new()`, `Pop::gentle()` and `Pop::with(gain, seconds)` have eight call sites). The reference for the spring is Rune (https://github.com/anomalyco/rune).

## Sources

1. Hicks, Dickinson, Holopainen and Gerling, *Good Game Feel: An Empirically Grounded Framework for Juicy Design*, DiGRA 2018, doi:10.26503/dl.v2018i1.936 (craft; in home-still).
2. Sanctuary (the owner's shipped game), `src/ui/juice.rs:1-9, 62-130` and its eight `Pop` call sites, e.g. `src/siege/hud.rs:914` (a changed number), `src/siege/wheel.rs:1497` (`Pop::with(0.10, 0.12)`, a chosen wheel slot) (shipping game).
3. Rune, `crates/rune-motion/src/tokens.rs:29-31`: `POP = Spring::new(0.25, 0.15)`, "A release with a little life in it. Keep it to presses and confirmations." (technical; open reference).
4. This crate: `src/press_scale.rs` (the drawing-only scale about the centre, its restore pass) and `src/model.rs` (`spring_step`).

## Craft principle

Hicks et al.: "the player pressing the jump button is accompanied by the sound, visual and, in some cases haptic effects at the same time. Respondents expressed how important it was for feedback to be given to the player in multiple ways simultaneously", and the same paper names "consistent and immediate feedback to player actions" delivered "on different feedback channels". A HUD number that changes is information the player must notice while looking at the game: a swell is the cheap second channel that says "this one just changed", beside the digits themselves. It repeats what the interface already shows; it never carries it alone (Sanctuary's own comment on the file says the same).

## How shipping games do it

Sanctuary, `src/ui/juice.rs:1-9`: "A [`Pop`] swells a node and settles it ... All run on real time, so the wheel's slow motion and a pause do not freeze the UI, and each takes itself off when it is spent. `Pop` touches only the scale of a `UiTransform`". It uses the swell for a changed number (the kill count, hearts), a focus change, a new line of dialogue, a chosen wheel slot (`Pop::with(0.10, 0.12)`). The swell is a quadratic ease back, with no overshoot, on a clock of its own.

## Model adopted

- **`Pop(f32)`**, a public component, the swell as a share of the node's size (`POP_GAIN` = 0.10 by default). It is a trigger: **inserting it, again too, starts a pop** (`Ref::is_changed`); the component stays, so there is one tuning to read and nothing to remove. One line from a game: `commands.entity(label).insert(Pop::default())`.
- **The drawing scales, nothing else.** A `Popped` state (required by `Pop`, public like `PressScaled`) holds the scale and its velocity. `draw_pops` in `PostUpdate` scales the node and its subtree about its centre exactly as `draw_press_scale` does and shares its restore (`PressScaleRestore`), so layout and the hit test keep the resting size and a node that is also a `PressScale` widget composes the two. Sanctuary's `Pop` writes `UiTransform::scale`; here that would be read by `MotionTrack` as a host write and end the node's `MotionTarget`, so the drawing pass is the safe form.
- **The spring is a retargetable one, not an ease.** The scale starts at `1 + gain` and the skin's snappy spring (times `duration_scale`) with `PRESS_RELEASE_BOUNCE` (Rune's `POP` bounce, 0.15) takes it to 1, so it settles with the slight undershoot a release has. A second pop on a node still settling restarts from where it is drawn plus the gain: `value = max(value, 1) + gain`, velocity kept, so rapid pops (a counter ticking up) stack into one fuller swell and not a stutter.
- **Real time** (`Time<Real>`), so a paused or slowed game still pops its menu, as Sanctuary does.
- **Reduced or no motion** (`MotionSettings::lands_travel`): no swell, the scale stays 1 (WCAG 2.3.3).
- **A gain that is not a number above zero is no pop.**
- At rest a node is read and not written; `PressScaleRestore` stays empty.

## Addendum, 2026-11-08 (iteration 72): `Lift`, a swell that is held

Sources: Rune `crates/rune-kit/src/shell/tab_strip/tab.rs:148`, "`let lift = |e: El| k.elevate(e.radius(RADIUS).scale(1.03).scale_motion(rune_motion::motion::MORPH).motion_intent(MotionIntent::OVERSHOOT), &k.elevation.floating)`": a picked-up tab is raised to the floating elevation and drawn at 1.03, on a springy morph. Deviation recorded by `tab-reorder.md` ("No pickup scale").

- **`Lift(f32)`** (public, `LIFT_GAIN` = 0.03): the same drawing as `Pop` with a different target. While the component is on a node its scale springs to `1 + gain` and stays; remove it and the spring takes it back to 1 with the release bounce. `Pop` and `Lift` share `Popped` and the drawing pass, so a pop on a lifted node swells from the lift.
- Reduced or no motion: a held scale is not motion, so the node lands on `1 + gain` at once (and a `Pop` still draws nothing).
- **A carried dock tab is lifted**: `update_tab_reorder` puts `Lift` and the skin's floating elevation (`BoxShadow`) on it while it is carried and takes both off when the drag ends (`DockDrag` removed), so it rises on pickup and settles with the bounce on the drop.

## What great looks like

A HUD readout that ticks from 41 to 42 swells 10% for about a tenth of a second and settles with a hair of life, as a coin counter does in a shipping platformer; the digits are never blurred (the whole node scales, so the text is rescaled by the renderer, not re-laid out); a run of five changes in half a second reads as one growing bump, not five.

## Parameters

| Name | Value | Where from |
|---|---|---|
| `POP_GAIN` | 0.10 | Sanctuary's wheel pop (`Pop::with(0.10, 0.12)`); a design default |
| spring | snappy duration, bounce `PRESS_RELEASE_BOUNCE` (0.15) | Rune `POP` (`tokens.rs:30`); the skin's snappy |
| rest | 1e-4 of the size | the press scale's rest |

## Validation targets

- A pop starts at `1 + gain`, is drawn at that scale about the node's centre the same frame, and is at 1 within a second.
- Layout (`ComputedNode::size`) and the resting `UiGlobalTransform` are unchanged; a press at the node's edge still lands on it.
- Inserting a second `Pop` while settling does not drop below the drawn scale.
- Under `MotionSettings::off()` nothing is drawn scaled.
- A settled node writes nothing, and the restore list is empty.

## Deviations

- **A spring, not Sanctuary's quadratic ease** (design choice: the crate's one motion model, retargetable, and the same release as `PressScale`).
- **`Pop` stays on the entity** where Sanctuary's takes itself off: a re-insert is the trigger, so the tuning is data on the node and a game never races a removal. Design default.
- **One swell, not three named shapes** (`gentle`, `number`, `with`): a game picks the gain, `Pop(0.2)`.

## Pending index

- Vlambeer, *The Art of Screenshake* (Nijman, 2013) is the usual shipping-game source for juice; it is a talk, not in the corpus, and not indexed. The Sanctuary call sites stand in for it here.
