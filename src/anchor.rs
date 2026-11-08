//! `WorldAnchor`: an interface node that follows a point in the world.
//!
//! A health bar over an enemy, a "Press E" over a door, a quest beacon that slides to the edge of
//! the screen toward a thing out of view. Research: `docs/research/world-anchor.md`.

use bevy::camera::CameraUpdateSystems;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::ui::{ComputedNode, ComputedUiTargetCamera, UiSystems};

use crate::model::{box_outside, pin_to_edge};
use crate::skin::SAFE_EDGE_PERCENT;

/// What an anchored node does when its world point is not in view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, Clone, PartialEq)]
pub enum OffScreen {
    /// Hidden while the whole node is outside the viewport or the point is behind the camera, shown
    /// again when it returns. Right for a nameplate or a health bar.
    #[default]
    Hide,
    /// Pinned inside the safe edge of the viewport (5%), on the line from its middle toward the
    /// point, with [`AnchorPinned`] on it while it is. Right for a beacon.
    Pin,
}

/// An interface node that stays on a point in the world: the [`GlobalTransform`] of
/// [`WorldAnchor::target`] plus a world-space [`offset`](WorldAnchor::offset), seen by a camera.
///
/// Put it on a root node (a [`meter`](crate::meter), a [`label`](crate::label), a
/// [`panel`](crate::panel), anything with a `Node`) and nothing else:
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn health_bar(mut commands: Commands, enemy: Entity) {
///     // A bar 1.2 world units above the enemy, gone with it when it is despawned.
///     commands.spawn((meter(0.8, 1.0), WorldAnchor::above(enemy, 1.2)));
/// }
/// ```
///
/// - It **is a relationship**: [`WorldAnchors`] on the target lists its nodes, and despawning the
///   target despawns them in the same flush.
/// - It is placed in `PostUpdate`, after transforms and before UI layout, so it is on the point in
///   the frame it is drawn, even for a fast thing under a following camera.
/// - The pivot (`(0.5, 1.0)`: the bottom middle) is the point of the node that sits on the world
///   point; [`WorldAnchor::above`] puts the node's foot on it.
/// - It owns the node's `position_type`, `left`, `top`, `UiTransform::translation` and
///   `Visibility`, and requires `Pickable::IGNORE` (a nameplate never blocks a click on the thing
///   under it; put your own `Pickable` in the same tuple to take the pointer).
/// - The node must be a root: its `left` and `top` are relative to the camera's viewport.
/// - The camera is the one the node is laid out for (a `UiTargetCamera`, or the default UI
///   camera); [`WorldAnchor::camera`] names another, for a world camera under a separate UI camera.
///
/// There is no fade or scale by distance and no test for a wall in the way: they need the game's
/// own world data.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Debug, Clone)]
#[relationship(relationship_target = WorldAnchors)]
#[require(
    Node,
    Pickable = Pickable::IGNORE,
    Visibility = Visibility::Hidden,
)]
pub struct WorldAnchor {
    #[relationship]
    target: Entity,
    /// World-space offset from the target's position (the way up for a nameplate).
    pub offset: Vec3,
    /// The point of the node, `0..=1` from its top left, that sits on the world point.
    pub pivot: Vec2,
    /// What the node does when the point is out of view.
    pub off_screen: OffScreen,
    /// The camera that sees the point; `None` is the camera the node is laid out for.
    pub camera: Option<Entity>,
}

/// The nodes anchored to an entity, kept by Bevy; they are despawned with it.
#[derive(Component, Debug, Reflect)]
#[reflect(Component, Debug)]
#[relationship_target(relationship = WorldAnchor, linked_spawn)]
pub struct WorldAnchors(Vec<Entity>);

/// The default pivot: the node's bottom middle, so its foot stands on the point.
const FOOT: Vec2 = Vec2::new(0.5, 1.0);

impl Default for WorldAnchor {
    /// Only for the relationship's own use: it needs a target, so use [`WorldAnchor::new`].
    fn default() -> Self {
        Self {
            target: Entity::PLACEHOLDER,
            offset: Vec3::ZERO,
            pivot: FOOT,
            off_screen: OffScreen::Hide,
            camera: None,
        }
    }
}

impl WorldAnchor {
    /// A node on `target`'s position, its bottom middle on the point.
    pub fn new(target: Entity) -> Self {
        Self {
            target,
            ..default()
        }
    }

    /// A node `height` world units above `target` on the Y axis (a 2D game's up, a 3D game's up).
    pub fn above(target: Entity, height: f32) -> Self {
        Self::new(target).offset(Vec3::Y * height)
    }

    /// The world-space offset from the target.
    #[must_use]
    pub fn offset(mut self, offset: Vec3) -> Self {
        self.offset = offset;
        self
    }

    /// Which point of the node sits on the world point, from `(0, 0)` its top left to `(1, 1)`.
    #[must_use]
    pub fn pivot(mut self, pivot: Vec2) -> Self {
        self.pivot = pivot;
        self
    }

    /// Pin the node to the edge of the screen while the point is out of view
    /// ([`OffScreen::Pin`]), instead of hiding it.
    #[must_use]
    pub fn pinned(mut self) -> Self {
        self.off_screen = OffScreen::Pin;
        self
    }

    /// The camera that sees the point, when it is not the one the node is laid out for.
    #[must_use]
    pub fn camera(mut self, camera: Entity) -> Self {
        self.camera = Some(camera);
        self
    }

    /// The entity the node follows.
    pub fn target(&self) -> Entity {
        self.target
    }
}

/// On an anchored node exactly while it is pinned to the screen's edge: `direction` is the unit
/// vector from the middle of the viewport toward the world point, in screen space (`y` down), for a
/// beacon to turn its arrow by.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Clone, PartialEq)]
pub struct AnchorPinned {
    /// Unit vector toward the point, screen space, `y` down.
    pub direction: Vec2,
}

/// Where an anchored node ends up this frame.
enum Placed {
    /// At `at` (viewport px, the pivot point), `pinned` with a direction when pinned to the edge.
    At { at: Vec2, pinned: Option<Vec2> },
    /// Not drawn.
    Hidden,
}

/// Places every [`WorldAnchor`] on its world point. Runs in `PostUpdate` after transform
/// propagation and the camera update and before UI layout, so a node is where the thing is in the
/// frame it is drawn; see [`WorldAnchor`].
#[allow(clippy::type_complexity)]
pub fn update_world_anchors(
    mut commands: Commands,
    ui_scale: Option<Res<UiScale>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    transforms: Query<&GlobalTransform>,
    mut anchored: Query<(
        Entity,
        &WorldAnchor,
        &mut Node,
        &mut UiTransform,
        &mut Visibility,
        &ComputedNode,
        Option<&ComputedUiTargetCamera>,
        Option<&AnchorPinned>,
    )>,
) {
    let scale = ui_scale.map_or(1.0, |s| s.0).max(f32::EPSILON);
    for (entity, anchor, mut node, mut transform, mut visibility, computed, ui_camera, pinned) in
        &mut anchored
    {
        let placed = place(
            anchor,
            computed,
            ui_camera.and_then(ComputedUiTargetCamera::get),
            scale,
            &cameras,
            &transforms,
        );
        let (visible, pin) = match placed {
            Placed::At { at, pinned } => {
                if node.position_type != PositionType::Absolute {
                    node.position_type = PositionType::Absolute;
                }
                let (left, top) = (Val::Px(at.x / scale), Val::Px(at.y / scale));
                if node.left != left {
                    node.left = left;
                }
                if node.top != top {
                    node.top = top;
                }
                let pivot = Val2::percent(-anchor.pivot.x * 100.0, -anchor.pivot.y * 100.0);
                if transform.translation != pivot {
                    transform.translation = pivot;
                }
                (true, pinned)
            }
            Placed::Hidden => (false, None),
        };
        let want = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
        let want_pin = pin.map(|direction| AnchorPinned { direction });
        if pinned.copied() != want_pin {
            match want_pin {
                Some(pin) => {
                    commands.entity(entity).insert(pin);
                }
                None => {
                    commands.entity(entity).remove::<AnchorPinned>();
                }
            }
        }
    }
}

/// The viewport position of `anchor`, or why it is not drawn.
fn place(
    anchor: &WorldAnchor,
    computed: &ComputedNode,
    ui_camera: Option<Entity>,
    scale: f32,
    cameras: &Query<(&Camera, &GlobalTransform)>,
    transforms: &Query<&GlobalTransform>,
) -> Placed {
    let Some(camera) = anchor.camera.or(ui_camera) else {
        return Placed::Hidden;
    };
    let (Ok((camera, camera_at)), Ok(target_at)) =
        (cameras.get(camera), transforms.get(anchor.target))
    else {
        return Placed::Hidden;
    };
    let Some(view) = camera.logical_viewport_rect() else {
        return Placed::Hidden;
    };
    let world = target_at.translation() + anchor.offset;
    // The node's size in the camera's logical pixels (the UI's logical pixels are `scale` of them).
    let size = computed.size() * computed.inverse_scale_factor() * scale;
    let pivot = anchor.pivot;
    let projected = camera
        .world_to_viewport(camera_at, world)
        .ok()
        .map(|p| p - view.min)
        .filter(|p| p.is_finite());
    let extent = view.size();
    match anchor.off_screen {
        OffScreen::Hide => match projected {
            Some(at) => {
                let min = at - pivot * size;
                if box_outside(
                    (min.x, min.y),
                    (min.x + size.x, min.y + size.y),
                    (0.0, 0.0),
                    (extent.x, extent.y),
                ) {
                    Placed::Hidden
                } else {
                    Placed::At { at, pinned: None }
                }
            }
            None => Placed::Hidden,
        },
        OffScreen::Pin => {
            let margin = extent * SAFE_EDGE_PERCENT / 100.0;
            let lo = margin + pivot * size;
            let hi = extent - margin - (Vec2::ONE - pivot) * size;
            let centre = extent / 2.0;
            // A point past the camera's planes has no projection: its direction is the one it
            // lies in, from the camera's own axes, so a point behind points the way it is.
            let at = projected.unwrap_or_else(|| {
                let local = camera_at.affine().inverse().transform_point3(world);
                let away = Vec2::new(local.x, -local.y);
                centre + away.try_normalize().unwrap_or(Vec2::NEG_Y) * extent.length()
            });
            let behind = projected.is_none();
            let at_edge = pin_to_edge(
                (at.x, at.y),
                (centre.x, centre.y),
                (lo.x, lo.y),
                (hi.x, hi.y),
            );
            let at_edge = Vec2::new(at_edge.0, at_edge.1);
            let inside = !behind && at_edge == at;
            if inside {
                Placed::At { at, pinned: None }
            } else {
                let direction = (at - centre).try_normalize().unwrap_or(Vec2::NEG_Y);
                Placed::At {
                    at: at_edge,
                    pinned: Some(direction),
                }
            }
        }
    }
}

/// Registers [`update_world_anchors`] where it must run: after this frame's transforms and camera
/// update, before UI layout.
pub(crate) fn add_systems(app: &mut App) {
    app.add_systems(
        PostUpdate,
        update_world_anchors
            .in_set(crate::GooeySystems::Layout)
            .after(TransformSystems::Propagate)
            .after(CameraUpdateSystems)
            .before(UiSystems::Prepare),
    );
}
