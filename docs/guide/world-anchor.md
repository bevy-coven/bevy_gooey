# World anchors

A health bar over an enemy, a "Press E" over a door, a beacon for a camp that is off the screen: the
interface follows a point in the world. `WorldAnchor` on any root UI node does it, with nothing else to
set up.

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn health_bar(mut commands: Commands, enemy: Entity) {
    // A bar whose foot stands 1.2 world units above the enemy, gone with it when it is despawned.
    commands.spawn((meter(0.8, 1.0), WorldAnchor::above(enemy, 1.2)));
}
```

![The world example: five wandering enemies each with its name and a health bar standing above it while the camera sweeps right and back; a Camp panel stays on the screen's right edge with an arrow pointing right until the yellow camp comes into view, then stands over it with no arrow, and leaves to the edge again when the camera sweeps back](https://raw.githubusercontent.com/dark-incorporated/bevy_gooey/main/docs/media/world-anchor.gif)

*`examples/world`: nameplates over moving enemies and a camp beacon pinned to the screen's edge.*

## What it does

- **Follows the point.** The point is the target's `GlobalTransform` plus `WorldAnchor::offset` (a
  world-space vector; `above(target, height)` is `Vec3::Y * height`), seen by the camera. The node is
  placed in `PostUpdate`, after transforms and the camera update and before UI layout, so it is on the
  point in the frame it is drawn: a fast enemy under a following camera is not trailed by its bar.
- **Ends with its target.** `WorldAnchor` is a Bevy relationship, and `WorldAnchors` on the target lists
  its nodes; despawning the target despawns them in the same flush. A game writes no cleanup.
- **Lets the pointer through.** It requires `Pickable::IGNORE`, so a nameplate never blocks a click on
  the thing under it. A "Press E" that should be clicked takes its own `Pickable::default()` in the same
  tuple.
- **Sits where you say.** `pivot` is the point of the node, from `(0, 0)` its top left to `(1, 1)`, that is
  on the world point: the default `(0.5, 1.0)` stands the node's foot on it, `(0.0, 0.5)` puts its left
  middle there.
- **Uses the node's camera.** By default the camera the node is laid out for (a `UiTargetCamera`, or the
  default UI camera). A world camera under a separate UI camera is named: `WorldAnchor::new(e).camera(cam)`.

## Out of view

`WorldAnchor::off_screen` is `OffScreen::Hide` (the default) or `OffScreen::Pin` (`.pinned()`).

| | when the point is off the screen or behind the camera |
|---|---|
| `Hide` | the node is `Visibility::Hidden` while none of it overlaps the viewport (a bar whose body is half in view still draws), and drawn again when it returns |
| `Pin` | the node stays whole inside the safe edge (5% of the viewport, the scrim's and the legend's), on the line from the viewport's middle toward the point, and carries `AnchorPinned { direction }` while it does |

`AnchorPinned::direction` is a unit vector in screen space (`y` down): turn an arrow by it, or pick
"left" or "right" art from its sign. The component is removed the frame the point is back in view. A
point behind a perspective camera points the way it lies (found from the view-space position, not the
mirrored projection).

## What it owns, and what it does not

The anchor writes the node's `position_type`, `left`, `top`, `UiTransform::translation` and `Visibility`,
only when they differ, so a settled anchor writes nothing. Spawn the node as a **root** (a child's `left`
and `top` would be relative to its parent). Use `Display::None` or despawn to switch a nameplate off for
good: `Visibility` is the anchor's.

There is no fade or scale by distance and no test for a wall in between: each needs the game's own world
data, and a game does it in a system of its own (for instance, set the node's `BackgroundColor` alpha, or
insert `Display::None`, from the distance to the camera).

The maths is `model::{pin_to_edge, box_outside}`, engine-free.

Research: [`docs/research/world-anchor.md`](https://github.com/dark-incorporated/bevy_gooey/blob/main/docs/research/world-anchor.md).
