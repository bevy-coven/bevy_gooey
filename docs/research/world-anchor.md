# Interface that follows a thing in the world: `WorldAnchor`

Researched 2026-11-03 (iteration 59). home-still's distill answered `500: gpu busy: yielded to bevy_grimoire@big`, so the corpus could not be searched; the sources below are primary documents read directly this pass (the Chalmers thesis downloaded and searched as text, Bevy's vendored source), and the corpus searches are under `## Pending index`.

## Sources

1. Fagerholt and Lorentzon, *Beyond the HUD: User Interfaces for Increased Player Immersion in FPS Games* (Chalmers / EA DICE, 2009) — https://publications.lib.chalmers.se/records/fulltext/111921.pdf (craft: the spatial dimension of a game's interface, as its own axis).
2. The same thesis's game study of shipped games (Mirror's Edge, Left 4 Dead, Splinter Cell: Conviction, Heavy Rain, Dead Space) (shipping games: what each draws in the 3D geometry and why).
3. Bevy 0.20 `bevy_camera::Camera::world_to_viewport` / `world_to_ndc` and `bevy_ecs::relationship` (technical: the projection and its failure cases; a relationship whose source is despawned with its target).

## Craft principle

Fagerholt and Lorentzon, section 7: "Geometric elements are UI elements presented in the 3D geometry without being an entity of the fictional game world. Examples of geometric representations are the character outlines of Left 4 Dead, the runner vision or checkpoint beacons in Mirror's Edge, or the mission briefings projected onto the game environments in Splinter Cell: Conviction." And, on why the axis exists: "a user interface might be a part of the game world spatially but not in a diegetic way … In other words, information might be presented outside of the fictional part of the game but within the 3D geometry of the game world."

So the information that belongs to a thing (an enemy's health, a door's prompt, a quest beacon) is drawn **where the thing is**, in the interface's own look, and not in a corner HUD the eye must cross the screen to match to the thing.

## How shipping games do it

The thesis on Heavy Rain: "The user interface also features elements presented spatially (see Picture 6.15), not entirely dissimilar to the holograms of the Dead Space user interface. As in Dead Space, fiction is used to explain the existence of these elements in Heavy Rain; the character is wearing a pair of advanced forensic glasses, able to pick out details in the environment and augment the characters vision with visual information superimposed on top of the 'real' world." The elements are anchored to what they describe and follow it; Mirror's Edge's checkpoint beacons point at a place that may be off the screen. A game that ships a health bar above each enemy or a "Press E" over a door needs exactly this and nothing else: a node that stays on a world point, goes away when the point is not seen, and for a beacon stays on the edge of the screen toward it.

## Model adopted

`WorldAnchor` is a component on any root UI node (a `meter`, a `label`, a `panel`, a `prompt`, the game's own) that **follows a world point**: the `GlobalTransform` of a target entity plus a world-space offset, seen by a camera.

- **A1. One component, one line.** `commands.spawn((meter(0.8, 1.0), WorldAnchor::above(enemy, 1.2)))`. The node needs no other setup: the component requires `Pickable::IGNORE` (a nameplate never blocks the click on the enemy under it; the game's own `Pickable` in the same tuple wins) and `Visibility::Hidden` (so the node is not drawn at the corner for the frame before its first placement).
- **A2. It is a relationship, so it ends with its target.** `WorldAnchor` is a Bevy `Relationship` on the node and `WorldAnchors` the target's list, `linked_spawn`: despawn the enemy and its bar goes with it; a game never writes the cleanup (Bevy's `Relationship` docs: a relationship to an entity that does not exist is removed with a warning).
- **A3. Placed by `left` and `top` over the same frame's transforms.** The system runs in `PostUpdate` after `TransformSystems::Propagate` and the camera update and before UI layout, so the node is at the thing's position in the frame it is drawn (a system in `Update` reads last frame's `GlobalTransform` and trails a fast thing by a frame). `Camera::world_to_viewport` is in logical window pixels; `UiScale` scales a `Val::Px`, so the position is divided by it.
- **A4. The pivot is part of the data.** `pivot` (0..1 per axis, default bottom-centre, `(0.5, 1.0)`) says which point of the node sits on the world point; it is a constant `UiTransform` translation of `-pivot` of the node's own size (the radial's trick), so the node's size is never read for placement and there is no first-frame jump.
- **A5. Off-screen is a choice.** `OffScreen::Hide` (default) hides the node while its whole box is outside the camera's viewport or the point is behind the camera (`world_to_viewport` fails past the near and far planes: "NDC z-values outside of 0 < z < 1 are outside the (implicit) camera frustum"). `OffScreen::Pin` keeps it inside the safe edge (5% of the viewport, the scrim's and the legend's) on the ray from the viewport's centre toward the point, the box fully inside, and puts `AnchorPinned { direction }` on it while it is pinned so a beacon can turn its arrow; a point behind the camera points the opposite way of where it projects (found from the view-space position, not the mirrored NDC).
- **A6. The camera is the node's own.** `WorldAnchor::camera` is `None` by default: the camera the node is laid out for (`ComputedUiTargetCamera`); a game with a world camera under a separate UI camera names it.
- The maths (`pin_to_edge`, `box_outside`) is engine-free in `model`.

## Parameters

- `WorldAnchor { target, offset: Vec3, pivot: Vec2, off_screen: OffScreen, camera: Option<Entity> }`; builders `new(target)`, `above(target, height)`, `offset`, `pivot`, `pinned`, `camera`.
- `AnchorPinned { direction: Vec2 }` (unit, screen space, y down), present exactly while pinned.
- The safe edge is the crate's `SAFE_EDGE_PERCENT` (5); no skin field (the node's own look is its widget's).

## Validation targets

- A `meter` anchored above a sprite at world (100, 50) with the offset 20 on a 1280×720 window, scale 1, `Camera2d` at the origin: the node's bottom-centre is at window (740, 290) within 1 px, in the frame after the spawn and in the frame a moving target moves (not a frame later).
- The same at a window scale factor of 2 and a `UiScale` of 1.25.
- A target moved past the right edge: `Hide` sets `Visibility::Hidden` and takes it back when it returns; `Pin` leaves the node whole inside the 5% edge with `AnchorPinned.direction.x > 0` and no marker once the point is back.
- A point behind a perspective camera: `Hide` hides it, `Pin` points the opposite way of the mirrored projection.
- Despawning the target despawns the node in the same flush; a game's own `Pickable` is kept; idle frames write nothing (`Changed` on the node stays unset after the first placement).
- The node is a root; one under a parent is placed relative to its parent's padding box (documented, not corrected).

## What great looks like

A health bar riding above a moving enemy with no lag against its sprite at any speed, a "Press E" prompt that fades in at a door, and a quest beacon that slides to the edge of the screen and points to where the thing is, its arrow turning as the player turns: the interface looks the same in the world as in the menu, because it is the same widgets.

## Deviations

- Hiding writes `Visibility`, which the anchor owns while it is present. A design default (the alternative, a `Display` flip, collides with a game that collapses the same node).
- `Pin` keeps the ray toward the point rather than clamping each axis separately: the position says the direction. A design default.
- A pinned node is also pinned while its point is in the viewport but inside the 5% safe edge (the box would be cut by the edge otherwise), so a beacon never half-leaves the screen; shipped in `world`'s capture. A design default.
- No fade by distance, no scale by distance, no occlusion test (a thing behind a wall is still anchored): all are the game's, because each needs its own world data (a design default).

## Pending index

- distill was unavailable (`gpu busy`). Search the corpus for: "off-screen indicator game HUD", "nameplate world space UI", and index the Chalmers thesis (not named in `corpus.md`).
