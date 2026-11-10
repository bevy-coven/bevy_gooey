//! The layout primitives: a [`row`](crate::row), a [`column`](crate::column), a fixed-width
//! [`cell`](crate::cell), an [`image_box`](crate::image_box) and an [`overlay`](crate::overlay),
//! whose spacing comes from the skin. A panel is built from widgets and never from a `Node` literal.
//!
//! The widgets themselves are free functions in [`crate::widgets`], as every widget here is; this
//! module holds what they carry and the one system that decides which of an overlay's children are
//! stacked.

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
/// `CellWidth` at all, its default) is not shown: it takes no room, no gap stands where it would
/// be, and nothing it holds is drawn.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CellWidth(pub f32);

/// The size an [`image_box`](crate::image_box) draws at, logical px, written by
/// [`crate::apply_skin`]: the box keeps this size whatever its picture is (a plot being written
/// still holds its room) and whatever the row it stands in has left, because a picture never
/// shrinks: a cell gives way before it.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoxSize(pub Vec2);

/// An [`overlay`](crate::overlay)'s child drawn over its first one: absolute, filling the box the
/// first child gives the overlay.
///
/// Put there and taken off by the crate, never by the host: it is the record of what is stacked, so
/// a child that becomes the overlay's first one (because the one before it was despawned or moved
/// out) or that leaves the overlay is put back in the flow, and an overlay is always the size of
/// what it covers. A host that removes it by hand has its child laid out as itself again.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct OverlayStacked;

/// Stacks an [`overlay`](crate::overlay)'s children: every child after the first is drawn over the
/// first one, filling the box the first child gives the overlay.
///
/// Runs in [`crate::GooeySystems::Style`] **right before [`crate::apply_skin`]**, so the restyle
/// that follows draws an overlay's first child (in the flow, its own box), a stacked child (the
/// overlay's box) and a child that has just stopped being either (its own box again) in the same
/// frame. The first child is the only one in the flow, so the overlay is exactly its size.
///
/// A child that becomes the first one, or that leaves the overlay for another parent, is unstaked:
/// [`OverlayStacked`] goes and its box is the one its role gives it. To put something in a corner
/// of an overlay, nest a node of its own in the overlay (that node is the stacked child) rather
/// than styling the stacking by hand.
///
/// Looks at an overlay only when its children changed, and at a stacked child only when it changed
/// parent, and writes only what differs, so a settled overlay costs nothing.
pub(crate) fn stack_overlays(
    overlays: Query<(&Role, &Children), Changed<Children>>,
    mut orphans: Query<Orphan, (With<OverlayStacked>, Changed<ChildOf>)>,
    parents: Query<(&Role, &Children)>,
    stacked: Query<(), With<OverlayStacked>>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    // A stacked child that changed parent is looked at first: it may be another overlay's first
    // child now, or an item of a row that lays it out.
    for (child, parent) in &mut orphans {
        let Ok((role, children)) = parents.get(parent.parent()) else {
            unstack(&mut commands, &mut nodes, child);
            continue;
        };
        if *role != Role::Overlay || children.first() == Some(&child) {
            unstack(&mut commands, &mut nodes, child);
        }
    }
    for (role, children) in &overlays {
        if *role != Role::Overlay {
            continue;
        }
        for (index, child) in children.iter().enumerate() {
            if index == 0 {
                // The first child sizes the overlay; one that was stacked (its predecessor is
                // gone) comes back into the flow.
                if stacked.contains(child) {
                    unstack(&mut commands, &mut nodes, child);
                }
                continue;
            }
            if !stacked.contains(child) {
                commands.entity(child).insert(OverlayStacked);
            }
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

/// A stacked child whose parent changed: the crate's only reason to look at it is that it may have
/// left its overlay or become the overlay's first child.
type Orphan = (Entity, &'static ChildOf);

/// Draws `child` in the flow again: the stacking is taken off it, so the [`crate::apply_skin`] that
/// runs after this writes the box its role gives it (its own width for an [`crate::image_box`], its
/// own size for a [`crate::cell`]). Its width and height are left until then, so nothing collapses
/// in the frame the stacking goes.
fn unstack(commands: &mut Commands, nodes: &mut Query<&mut Node>, child: Entity) {
    if let Ok(mut node) = nodes.get_mut(child) {
        let mut next = node.clone();
        next.position_type = PositionType::Relative;
        next.left = Val::Auto;
        next.top = Val::Auto;
        next.right = Val::Auto;
        next.bottom = Val::Auto;
        node.set_if_neq(next);
    }
    commands.entity(child).remove::<OverlayStacked>();
}
