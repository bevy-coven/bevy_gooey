# The selection indicator

Researched 2026-11-07 for issue #68 (G-61), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune `crates/rune-kit/src/segmented.rs` and `shell/tab_strip.rs`: a segmented control and a tab strip move one selection indicator between their options instead of swapping each option's fill.
2. The crate's own `size_motion.rs` (a split's glide) and `motion.rs` (`MotionToken::Snappy`, `spring_step`): the spring the indicator rides.

## Model adopted

- **One indicator per widget**: two absolute nodes, `Pickable::IGNORE` and hidden at rest, the last child of a `toggle_group` and of a `tabs` or `side_tabs` root (so the strip stays child 0, as the protected tests index it), drawn behind its siblings by `ZIndex(-1)`. The node is a **backdrop** in the raised fill over the strip's rect; its child is the **puck** in the accent.
- **It travels only while the lit option changes.** The lit option is found each frame (`Checked` on a group's option, `Selected` on a tab). A different one than last time starts a travel: the puck's left, top, width and height spring from where it is drawn to the new option's rect on `motion.snappy`. A change mid-flight retargets the same four springs, so there is no jump. It lands on the option's rect within the spring's rest tolerance (far inside half a pixel) and the indicator hides.
- **The options' own fills are unchanged at rest** (the protected skin tests pin the lit option's accent and the others' raised fill). While a travel runs, `Indicating(true)` is on the strip's options and `apply_skin` draws them clear, so the backdrop and the puck are what show; the strip itself is never filled. The options' colours land (colour motion holds a node whose `Indicating` changed) in the frame the travel starts and in the frame it ends, so the lit option is the accent again the frame the puck leaves it, with no fade in between. A pressed option keeps its press fill.
- **Reduced motion, no motion and no clock never travel** (`MotionSettings::lands_travel`): the lit option is the accent at once, the indicator never shows.
- **A strip that reflows at rest moves nothing:** the indicator is hidden, and the options draw themselves.
- **A settled interface writes nothing and allocates nothing:** the nodes, the marks and the indicator's own state are written only when they change.

## Where this differs from the issue

- **The indicator is hidden at rest** rather than always behind the options, because the options' fills are opaque and are pinned by existing tests; it is shown, and the fills cleared, only for the travel. The look at rest and at the ends of a travel is the old look, except that the raised backdrop also fills the gaps between options (a pixel or two) for the quarter second a travel lasts.
- **"The lit option's text colour still changes through colour motion"** is unchanged: the text colours are not touched.
- **Dock tab strips and menu bars are not covered** (`DockTab`s are not `tabs`); only `toggle_group`, `tabs` and `side_tabs`.

## Proof

`tests/selection_indicator.rs`: a four-option group stepped from the first to the third travels continuously, only toward the third, with no frame jumping half the way, and lands on the third's rect within 0.5 px with the fills as at rest; a step back mid-flight turns round with the speed changing smoothly; `tabs` and `side_tabs` travel and land on the chosen tab; reduced and no motion never show it; a resting group writes no indicator and no mark.
