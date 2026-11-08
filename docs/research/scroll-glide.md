# Scrolls that glide and fade at their edges

Researched 2026-11-07 for issue #65 (G-58), asked by bevy_grimoire's feel series (dark-incorporated/bevy_dark_factory#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune `crates/rune/README.md`, "Smooth scrolling" and "Scrolling": `scroll_to` glides on the runtime's clock, a request mid-glide keeps its speed so key repeat scrolls as one motion, and the wheel takes over a glide.
2. Rune `crates/rune/src/runtime/scroll/edge.rs` (`FADE_IN = 15.0`) and `crates/rune/tests/scroll_edge_fade.rs`: each edge fades only while content is hidden under it, at strength `hidden / 15 pt`.
3. bevy_gooey#76: real-clock test apps run with the setting that removes the motion.

## Model adopted

- **`ScrollGlide`** (a component every `ListView` requires). A scroll the crate makes, `scroll_to_focus` (Tab, the arrow keys, the pad and a cursor row, which also covers `page_lists`, Home and End and a tree's cursor) and a table's cursor follow, sets the glide's goal instead of writing `ScrollPosition`. `update_scroll_glides` moves the scroll toward the goal on the skin's `motion.default` spring times `duration_scale` and writes `ScrollPosition` each frame.
- **A request mid-glide** keeps the spring's velocity (the closed form in `model::spring_step` is a retarget from the state it ended in) and reveals the next row from the glide's goal, so asking one row on never pulls the goal back.
- **The wheel, a trackpad, a drag on the rail and a game's own write take over:** the glide remembers what it last wrote, and a `ScrollPosition` that reads otherwise ends it. The wheel moves 1:1 as it always did.
- **Reduced motion, no motion and no clock land a scroll at once** (`MotionSettings::lands_travel`), in the frame it is asked for, as before.
- **`ScrollEdgeFade(px)`** (required by `ListView`, so lists and tables carry it at 24 px): two overlays beside the viewport, `Pickable::IGNORE`, spawned once and set from the viewport's last-frame layout: strength `min(hidden px / 15, 1)`, the colour of the nearest ancestor background (or the skin's surface), `Display::None` when nothing is hidden under the edge. A settled list writes nothing.

## Where this differs from the issue

- **A spring, not Rune's constant clock, a third of `motion.default` long.** The crate's other motion is springs, so the glide is one, on the skin's `motion.default` times `SCROLL_SPRING_FRACTION` (a third: 0.1 s at the default, and `duration_scale` still applies). The whole 0.3 s measured badly: a held key moves the goal 1,300 px a second down a 44 px-row table, a critically damped spring trails a goal at speed `v` by `2v / omega` (125 px at 0.3 s), and the cursor row stood two rows past the viewport's foot on every frame of the hold. At a third the lag is 42 px, inside the row of context a request keeps.
- **Menus and sections are not covered.** `scroll_menus_to_focus` (a menu's panel) still jumps, and a `section`'s height glide is a separate motion (`SectionGlide`). Neither is a `ListView`.
- **The fade is drawn beside the viewport, not over its content,** because a child of a scrolling node scrolls with it. The overlay is placed from the viewport's box one frame late after a layout change.

## Proof

`tests/scroll_glide.rs`: a far focus glides and lands where the jump did; a new request mid-glide keeps its speed and the wheel ends the glide; a wheel notch scrolls the same as without the glide; reduced and no motion land in two frames; holding Down in a 10,000-row table has no step beyond twice the median and the cursor row stays in the viewport; the fade is 0 at the top and 1 after 15 px, half at 7.5 px, none at the end, `Pickable::IGNORE`, and a settled list writes nothing.
