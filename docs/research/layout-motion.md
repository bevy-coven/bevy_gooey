# Layout motion: moved widgets travel instead of teleporting

Researched 2026-11-07 for issue #52 (G-45), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune/README.md`, "Animation":
   - "Layout animates per edge, so stationary edges never move."
   - "Children ride their parent's left edge."
   - "Retargets keep exact position and velocity."
   - Layout itself is computed at the target state ("Taffy 0.15 at the target state", "Layout").
2. Bevy 0.20, `bevy_ui/src/layout/mod.rs`, `ui_layout_system` and `update_uinode_geometry_recursive`.
   - Every frame, every root is laid out.
   - Each node's `UiGlobalTransform` is recomputed from its parent's, its `UiTransform` and its layout location, and written whenever it differs from what is stored.
   - So a value written into `UiGlobalTransform` after `UiSystems::Layout` is drawn that frame, and is replaced by layout's own value the next.
   - Picking and the renderer read `UiGlobalTransform`.

## Model adopted

- **`MovesSmoothly`**, a public marker. It requires a public `LayoutTravel`, whose `offset()` is how far from its layout place the node is drawn.
- **What is tracked.** `update_layout_motion` runs in `PostUpdate`, after `UiSystems::Layout` and before `UiSystems::PostLayout`, so it is ahead of the clipping pass. Each frame it reads the node's top-left corner inside its parent, in logical px. The corner is read in the parent's own space (the inverse of its `UiGlobalTransform`) and without the node's own `UiTransform` translation, so a `MotionTarget` on the node, or a scale or turn on an ancestor, is not read as a move. The offset is applied turned and scaled by the parent's matrix.
  - **When the corner moves,** the old minus the new place is added to the offset, and the velocity is kept: a retarget continues from where the node is drawn.
  - **Each frame,** the offset springs to 0 on `motion.default` (scoped skin, times `duration_scale`). The offset is then added to the `UiGlobalTransform` translation of the node and everything under it, so children ride it.
  - **Order.** Every corner is read before any drawing is moved, so a moving parent's offset is never read into its child's place.
  - **Layout itself** is untouched and at the target from the first frame; only the drawing travels. Picking travels with the drawing because it reads the same transform.
- **Per edge.** Tracking the top-left corner is how a stationary edge never moves: a node that grows or shrinks with its corner standing still does not travel. Its size change is G-46's (size motion), not this item's.
- **What lands** (moves at once, with nothing travelling):
  - a move in the node's first `LAYOUT_QUIET_FRAMES` (2) frames;
  - a move in the frames after a new window size or scale (`ComputedUiRenderTargetInfo` changed), a new skin, or a new parent;
  - a move caused by a scroll of the parent;
  - every move under `MotionSettings::lands_travel()` (reduced or off).

  The window of two frames exists because the crate's fitting passes read one frame's layout and move things in the next: a toolbar's overflow, and a status bar hiding what does not fit. A resize should not make them glide.
- **Hidden.** `Display::None` lays a node out empty at its parent's corner. A node that is empty, or whose parent is, is not tracked, and it starts over (a quiet window) when shown, so it never flies in from the corner.
- **The subtree.** The offset goes to every node layout places from the moving one. A `FixedNode` (laid out on the window, as a popover's pointer point is) keeps its own place, and so does everything under it.
- **At rest,** a node is read and not written: its offset is zero, and `LayoutTravel` changes only when a move starts or a settling window counts down.

## Where this differs from the issue and from Rune

- **Which nodes the crate marks.** The issue proposed toast rows, tabs, dock tabs, toolbar items and status items. This crate marks **tabs** (`tabs` and `side_tabs`, `Role::Tab`) and nothing else, for these reasons:
  - **Toasts** already glide. Their room opens and closes on its own ease (`Toast::grow`), and protected tests measure exactly that glide (`toast::a_toast_leaving_closes_its_room_and_the_older_one_settles_without_a_jump`) and the stack's corner three frames after an anchor change. A second spring on top made both fail. G-65 moves toasts onto presence and this motion together.
  - **Toolbar and status items** are moved by a fitting pass that hides what does not fit, a frame after the layout it read. At 200% text, a status item gliding into place stood outside the bar for several frames (`status_bar::at_200_percent_text_...` failed).
  - **A dock's tabs** are marked too, but `update_docks` spawns every tab again on each change to the dock's layout, so a dock tab is new on the only frames it moves and lands. G-63 (tab reorder with motion) carries the drawn place across that respawn.
  - A game can still mark any of these itself.
- **Rune animates each of the four edges** and can make the leading edge lead (`.lead`, `.width_leads`). Here the top-left corner is the one tracked, so a node's size never animates through this.
- **Rune never draws a frame at the new place.** Neither does this crate: the offset is applied in the same frame layout moved the node.

## Proof

`tests/layout_motion.rs`, 11 tests, at 60 Hz:

- In a column of three 60 px blocks, the first is taken out. The second travels up 70 px with no frame over a third of the move and every step the same way. It settles within the spring's settle time, and its child rides it.
- Without `MovesSmoothly` the same block is at its new place in one frame. That is the control, and it shows the check can fail.
- Reduced motion lands the move.
- A block that grows does not move, and the block under it travels.
- Resizing the window from 1280×720 to 1600×900 moves a centred column with no offset on any block.
- Tabs are marked. Toasts, status items and toolbar items are not.
- A move in the frame after a node first appears lands.
- A settled column writes no `LayoutTravel` in 30 frames.
- The child rides the block on every frame, and a growing block's corner stands still on every frame.
- Taking `MovesSmoothly` off stops the motion; a `MotionTarget` slide on a marked node leaves its `LayoutTravel` at zero; a block hidden and shown again is at its place in its first frame back.
