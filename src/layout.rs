//! The layout primitives: a [`row`](crate::row), a [`column`](crate::column), a fixed-width
//! [`cell`](crate::cell), an [`image_box`](crate::image_box) and an [`overlay`](crate::overlay),
//! whose spacing comes from the skin. A panel is built from widgets and never from a `Node` literal.
//!
//! The widgets themselves are free functions in [`crate::widgets`], as every widget here is; this
//! module holds what they carry and the one system that stacks an overlay's children.

use bevy::prelude::*;

use crate::skin::Role;

/// The gap a [`row`](crate::row) or a [`column`](crate::column) puts between its children: a
/// multiple of [`GooeySkin::gap`](crate::GooeySkin::gap), written by [`crate::apply_skin`], so the
/// gap follows the skin and a player's text size instead of a number in the host.
///
/// `LayoutGap(0.0)` is no gap at all; a negative or non-finite multiple reads as none.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LayoutGap(pub f32);

/// The width a [`cell`](crate::cell) takes, in body text sizes (an `em`, so it grows with the
/// player's text size), written by [`crate::apply_skin`]. A cell of no width (or with no
/// `CellWidth` at all, its default) collapses to nothing and draws nothing of what it holds.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CellWidth(pub f32);

/// The size an [`image_box`](crate::image_box) draws at, logical px, written by
/// [`crate::apply_skin`]: the box keeps this size whatever its picture is (a plot being written
/// still holds its room) and whatever the row it stands in has left (a picture is never squeezed,
/// a cell shrinks before it).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoxSize(pub Vec2);

/// Stacks an [`overlay`](crate::overlay)'s children: every child after the first is drawn over the
/// first one, filling the box the first child gives the overlay.
///
/// Runs in [`crate::GooeySystems::Style`] right after [`crate::apply_skin`], so an overlay's rule
/// is the last word on its children's boxes: a child of an overlay has no size of its own, whichever
/// widget it is. Place a thing in a corner of an overlay by nesting it in a node of its own inside
/// the overlay, or by giving that node a `Node` of its own in the overlay's next child.
///
/// The overlay is sized by its first child, which is the only one left in the flow.
pub fn stack_overlays(
    overlays: Query<(&Role, &Children), Changed<Children>>,
    mut nodes: Query<&mut Node>,
) {
    for (role, children) in &overlays {
        if *role != Role::Overlay {
            continue;
        }
        for child in children.iter().skip(1) {
            let Ok(mut node) = nodes.get_mut(child) else {
                continue;
            };
            let mut next = node.clone();
            next.position_type = PositionType::Absolute;
            next.left = Val::ZERO;
            next.top = Val::ZERO;
            next.right = Val::Auto;
            next.bottom = Val::Auto;
            next.width = Val::Percent(100.0);
            next.height = Val::Percent(100.0);
            node.set_if_neq(next);
        }
    }
}
