# Double press and click counts

Researched 2026-11-06 for issue #62 (G-55), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference: its rules were measured from its source and written again here on Bevy's picking events. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune/src/runtime/input/pointer.rs`: `CLICK_SLOP = 4.0` (points, per axis); `pointer_down` takes the pending first press (`last_press.take()`: "Any press breaks a pending pair unless it's the second press of it") and arms the pair when the press is on the same element, within `double_click_interval` of the first press and within the slop of where it was; `release` sends the double press only for a click ("Only clicks pair: a press that dragged or was held past the interval is neither half"), primary button, released inside, and otherwise records the press as the next first; `move_pointer` marks a held press as travelled past the slop on either axis; `scroll_cancels_press` and a cancelled capture disarm it.
2. Rune, `crates/rune/src/runtime/input/press_clicks.rs`: the click count of a drag, AppKit's `clickCount`: one more than the last press when that was on the same element, within the interval and within the slop of it, else 1; a press that dragged, another button's press or a reset ends the run.
3. Rune, `crates/rune/src/runtime.rs`: `double_click_interval` is "the system setting on macOS and Windows, 0.5 elsewhere and headless" (default `0.5`).
4. Bevy 0.20 `bevy_picking/src/events.rs` (`pointer_events`): `PointerClick::count` and `PointerPress::count` count presses on an entity within `PickingSettings::multi_click_interval` (500 ms) on `Instant::now()`, with no distance test and no limit, and Bevy sends the click before the release.

## What Bevy's own count got wrong for a desktop

- A third quick click had `count == 3`, so a row clicked three times quickly opened once and a fourth click (count 4) never opened; a desktop opens on the second and again on the fourth.
- A second click 30 px away from the first still counted 2: nothing checks the place.
- A press held for a second and then a quick click counted 2: nothing checks how long a press was held.
- The clock is the wall clock, so a test on `TimeUpdateStrategy::ManualDuration` could not set it, and a paused or slowed app still paired by real time (gooey's other pointer timing reads `Time<Real>`).

## Model adopted

- `PointerSettings { double_click_seconds: 0.5, double_click_slop_px: 4.0 }`, a serde resource `GooeyNavPlugin` inits; the clock is `Time<Real>`.
- Five global observers in `src/clicks.rs` read `PointerPress`, `PointerDrag`, `PointerClick`, `PointerRelease` and `PointerCancel` at the entity each started from (a global observer runs at every hop of a propagating event, so each checks `event_target() == original_event_target()`), in the order picking sends them. Messages would lose that order between event types inside one frame.
- `DoublePress { entity }` is an `EntityEvent` that bubbles up `ChildOf` like the click, triggered in the second click's frame, after that click.
- `ClickCount(u8)` is on the pressed entity from its press until the pointer lets go, so a drag observer reads it (Rune's `Drag::clicks`).
- The split's divider, the table's rows and so the tree's rows open on `DoublePress`; nothing in the crate reads `PointerClick::count` any more.

## Where this differs from Rune

- Rune pairs only on an element that has a double-press handler; here every press is tracked, because a Bevy entity does not declare its observers. The result is the same for an entity that observes `DoublePress`.
- Rune's scroll under a held press cancels the pair; here a press is cancelled by Bevy's `PointerCancel` only. A scroll under a held press is G-54's (press scale) work.
- One pointer presses one entity at a time here: when a backend reports a press on several entities at once (a stack of non-blocking pickables), the last one reported holds the press. bevy_ui's backend reports the topmost blocking node only.
