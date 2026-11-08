# Hover groups: a row reveals its own actions

Researched 2026-11-08 for issue #60 (G-53), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly); this crate's code is its own, on bevy_ui.

## Sources

1. Rune README, "Hover groups": `.hover_motion(enter, exit)` sets a group's curves, a descendant reads the group with `.group_opacity(rest, hovered)`, and a group stays hovered while a descendant holds the press and counts only its visible part. Read from the README; Rune's source was not opened for this item.
2. Bevy `bevy::picking::hover::Hovered` (0.20): true while the pointer is over the entity or any descendant, which is what a row wants.
3. This crate's `src/clicks.rs`: Bevy sends an entity the pointer has left no release, so a press that moved is let go by `PointerDragEnd` ("every drag ends"). A held press is tracked the same way.
4. WCAG 2.2 SC 2.4.7 Focus Visible and 2.5.8 Target Size: a control that only hover reveals would be an invisible tab stop for a keyboard or pad player. This crate's rule "keyboard and gamepad parity" wins over Rune here.
5. This crate's `docs/research/reduced-motion.md` (#50) and `docs/research/switch-motion.md` (#67): the shape of `MotionSettings` (`reduced`: nothing travels and a fade takes `fade_seconds`; `off`: it lands) and of a skin spring read through `skin_of`.

## Model adopted

`HoverGroup` is a marker that requires Bevy's `Hovered`. `GroupReveal { rest, hovered }` requires a `RevealTrack` (a spring state from 0, rest, to 1, hovered, and whether the node has looked at its group yet). `update_group_reveals` finds the nearest `HoverGroup` at or above each reveal and moves its progress:

- **Target 1** when the group is under the pointer (`Hovered`), held by a press that began inside it (`GroupHold`), or holds visible focus. **Target 0** otherwise, and for a node under no group.
- **Entering** steps a spring on `motion.snappy`, **leaving** one on `motion.default`, both through `skin_of` so a `SkinScope` can retune them and scaled by `duration_scale`. The progress is the spring's state, so a pointer that comes back mid-way carries on from the drawn value and speed.
- **Visible focus lands in that frame** with no motion (a keyboard user's reveal is not an animation, and "Tab onto the button shows it at 1 in that frame" is the acceptance).
- **Reduced motion** fades straight over `fade_seconds`; **no motion** and the **first frame** land.
- `Opacity(rest + (hovered - rest) * progress)` is written only when it differs, so a settled group writes nothing. `spread_opacity` then turns it into what the subtree is drawn at.

`GroupHold(PointerId)` is inserted on every group above a primary `PointerPress` and removed by a primary `PointerRelease`, a primary `PointerDragEnd` or a `PointerCancel` of that pointer.

## Where this differs from the issue and from Rune

- **The first sentence of the proposal is not built**: "hover enters on `motion.snappy` and leaves on `motion.default`" for every widget's hover colour. Colour motion takes one spring for every state change (`docs/research/color-motion.md`), so splitting it by direction means tracking which way each colour's state went; that is its own change to the colour tracks and the issue's acceptance does not test it. The group reveal uses both springs as asked.
- **"Counts only its visible part"** (Rune: a group clipped by a scroll area is hovered only where it shows) is not built: Bevy's picking already hits only what is drawn, so a clipped part under the pointer is not hovered; the case that differs is a descendant that is picked outside the group's clip, which no widget here produces.
- **Focus is part of the group's state**, which Rune's README does not say.
- **A press held by another button** (a second pointer) is its own hold: `GroupHold` names the pointer, and a release from the mouse does not let go a touch's hold.
- **The node's `Opacity` belongs to the reveal.** A fade the game writes on the same node is overwritten; put the reveal on a node of its own. A leave through `Leave` on the node itself still takes the value over, as for any `Opacity`.

## Proof

`tests/hover_group.rs` drives a headless window with real pointer input on a 60 Hz clock: the enter is within 2% of 1 at the snappy duration and rises only; the leave is within 2% of 0 at the default duration, falls only, and is still above 2% at the snappy duration (the control that tells the two springs apart); a press that began on the button keeps the row at exactly 1 through 60 frames while the pointer is far off the row and hides it after the release; visible focus reveals in one frame, unseen focus reveals nothing, and removing focus hides on the default spring; reduced motion is a straight fade over `fade_seconds` and no motion lands in one frame; a settled group, revealed or at rest, writes no `Opacity` over 30 frames; and a reveal in an unhovered group starts at rest on its first frame. Each of six mutations (a press hold never taken, leaving on the snappy spring, focus not landing, no straight reduced fade, hover ignored, focus not a reveal) fails at least one of these.
