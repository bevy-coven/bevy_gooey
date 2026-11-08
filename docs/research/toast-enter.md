# Toasts enter from their edge

Researched 2026-11-08 for issue #72 (G-65), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly); this crate's code is its own, on bevy_ui.

## Sources

1. The issue's reading of Rune `crates/rune-kit/src/toast.rs:108`: a toast enters with `Transition::from_edge(..)` on `Spring::new(0.28, 0.0)`. Rune's source was not opened for this item.
2. This crate's `docs/research/layout-motion.md` (G-45), "Which nodes the crate marks": toasts are left out of `MovesSmoothly` on purpose. Their room already opens and closes on `Toast::grow`, and protected tests measure that glide (`toast::a_toast_leaving_closes_its_room_and_the_older_one_settles_without_a_jump`) and the stack's corner.
3. This crate's `docs/research/motion-tracks.md` (`MotionTarget`, `enter_from`): the one way to start a node away from its place, retargetable, landing under `off()`.
4. This crate's `docs/research/reduced-motion.md` (#50): under reduced motion a slide becomes a fade, and a toast already fades.

## Model adopted

`begin_entrances` gives each toast that appears (`Added<Toast>`) a `MotionTarget::default().entering_from((0, ±TOAST_SLIDE_PX))`: below its place (positive y) for a stack anchored to the bottom, above (negative) for one anchored to the top. `update_motion` then carries it to its place on the snappy spring, as it does a dialog's card, while the toast's own fade brings it in; the toast's room still opens on `Toast::grow` and the older toasts still glide up on their own ease when a room opens or closes (both unchanged). The slide is 48 px, a toast's height at one or two lines, so it starts at the edge.

## Where this differs from the issue and from Rune

- **The spring is `motion.snappy`** (0.25 s by default), not a toast-only 0.28 s. A 0.25 s spring settles inside the acceptance's 0.28 s ±1 frame, and a skin that retunes `snappy` retunes every entrance with it.
- **The other half of the proposal already stands**: a toast leaves with its fade and the others travel without a jump (the room glide above). Both are protected, so this change leaves them as they are and adds only the entrance.
- **Reduced motion and no motion add no slide**: a toast fades in and out under reduced motion already, and under no motion it lands.
- **The edge is the stack's vertical edge** (a corner anchor enters from the top or the bottom, the way the stack grows), not the nearest of the two window edges.

## Proof

`tests/toast_enter.rs` (motion on, 60 Hz clock): a bottom stack's first drawn frame is within 10% of 48 px below its place, it only ever rises, it is within 2% of its place at the snappy duration and settles exactly; a top stack enters from above; a toast already standing is not moved by a new one entering or by the oldest leaving; reduced and no motion never move a toast. The existing toast tests run with motion off and still hold.
