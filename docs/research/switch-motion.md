# Switch motion: a toggle's knob travels

Researched 2026-11-07 for issue #67 (G-60), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune `crates/rune-kit/src/switch.rs` (`:53-54`, `:125-164`): the thumb is a keyed child of the track, 12 pt wide inside a 2 pt inset, "moves 8 points when on"; the track and the thumb both carry `.motion(k.motion.control)`, so the thumb's move and the track's colour change run on the one control spring.
2. Rune `crates/rune-kit/src/kit.rs:200-205`: `Motion::control` is "controls changing state (switch thumb, checkbox, colour changes)", about 90 to 170 ms on the web.
3. Rune `crates/rune-kit/src/checkbox.rs:92`: the checkbox takes the same `token_motion(k.motion.control)`.
4. This crate's own `docs/research/layout-motion.md` (G-45): layout is at the target from the first frame and a spring draws the node's top-left corner travelling to it.
5. This crate's `docs/research/color-motion.md` (G-44): a widget's colours ease to the skin's on springs, text leading its ground.

## Model adopted

- **The knob is a layout-motion node.** `mark_moving_parts` gives a `Role::ToggleKnob` `MovesSmoothly` the frame it gets its role. Its place in the track is already layout's: the track's `justify_content` is `FlexStart` while off and `FlexEnd` while on (`apply_skin`), so a flip moves the knob's corner by the track's room, and `update_layout_motion` draws it travelling there. No second position is kept: the state stays `Checked`, the knob's place says whether it is on, and a value the game sets with no event (`Checked` inserted or removed) travels exactly as a click's does.
- **`TravelSpring(MotionToken)`** (public, a `Reflect` component beside `MovesSmoothly`): which of the skin's springs carries a marked node. No component is `MotionToken::Default`, so the tabs and every game's own marked node travel as before. The knob is given `Snappy`, the skin's token for a small control answering a press (`docs/guide/motion.md`). Rune names one `control` spring for the switch, the checkbox and the colour changes; this crate's four tokens already say the same thing with `snappy`, so no new token was added.
- **The track's colour** follows through colour motion with nothing new: `apply_skin` already writes the track's and the knob's colours from `Checked`, and colour motion eases them (G-44).
- **What lands**, from layout motion unchanged: the knob's first two frames (`LAYOUT_QUIET_FRAMES`), a new window size, scale or skin, a scroll of the parent, and every flip under reduced or no motion (`MotionSettings::lands_travel`). A toggle that starts on has its knob at the end in its first frame and never travels.
- **Mid-flight** a second flip retargets the same spring from the drawn place and its speed (layout motion's own rule), so a double tap has no jump.

## Where this differs from the issue and from Rune

- **No check mark.** The issue's second clause, "a checked mark enters with `Transition::scale(..)` and fade", names a type that does not exist in this crate, and the toggle has no check mark: the one mark of this crate that appears on a state is a menu's `MenuCheck` (a `Display::None` node with a turned `UiTransform`), whose entrance needs the arrival's scale lane and is not part of the issue's acceptance. It is not built here; the issue says so when it closes.
- **Rune stretches the thumb** a point towards the middle on hover and further while pressed (`switch.rs:54, :139`). That is a size animation on the pointer, G-53's hover family, and is not in this item's acceptance.
- **Rune's control spring is 90 to 170 ms;** the issue asks for `motion.snappy` (0.25 s), which a skin can change, and the test reads the skin's own value.

## Proof

`tests/switch_motion.rs`, 6 tests, at 60 Hz:

- A knob flipped by the game (`Checked` inserted) drawn frame by frame: every step in the same direction, none over a third of the travel, at rest on layout's own end place with a zero `LayoutTravel` offset, and no more than 2% of the travel left a frame after the spring's perceptual duration.
- The control: the same spring stepper puts the snappy spring within 2% at that frame and the default spring outside it, and the knob carries `TravelSpring(Snappy)`. Mutation-checked on a copy of the tree: a knob that is not marked (no travel at all) and a knob given `TravelSpring(Default)` each fail `a_knob_glides_to_the_other_end_and_is_nearly_there_in_the_snappy_duration` and `the_knob_runs_on_the_snappy_spring_not_the_default_one`.
- Flipping back four frames in: no frame steps over a third of the travel, the knob never leaves its track, and it ends at the off end.
- A toggle that starts on or off: its knob stands still from its first frame and the on knob is to the right of the off one.
- Reduced motion and `MotionSettings::off()`: the knob is at its end in the frame after the flip.
- A marked node's `TravelSpring` picks the spring: after twelve frames a node left behind by a removed neighbour is closer to its place on snappy than on default, and on default than on calm; a node with no `TravelSpring` is the default spring.
