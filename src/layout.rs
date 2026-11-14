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

/// How a layout container lays its children out beyond its gap: the cross-axis alignment, whether
/// a line that is full wraps, how the children share the main axis, and whether the container
/// itself gives way when its own host is short of room. The five things the editor patched over a
/// spawned [`row`](crate::row) or [`column`](crate::column).
///
/// Every field is an `Option`, and **`None` leaves the value the role gives the node**: a
/// [`button_row`](crate::button_row) keeps its centring and its wrapping when a `RowLayout` sets only
/// `justify`, and a plain row keeps Bevy's own default for a field nothing sets. `Some` writes it.
///
/// It is a component, so it goes where a patched `Node` cannot: beside `row(1.0)` in a bundle
/// (`(row(1.0), RowLayout { .. })`), in a `Children::spawn(SpawnWith(..))` tuple, on any
/// [`row_with`](crate::row_with) or [`column_with`](crate::column_with). [`crate::apply_skin`] writes
/// it on a [`Role::Row`], [`Role::Column`], [`Role::ButtonRow`], [`Role::FillColumn`],
/// [`Role::FillRegion`] and [`Role::Placed`] **only when the component is there**, so a row with none
/// keeps whatever the host patched on its `Node`. Change it and the node follows in the same frame;
/// setting a field back to `None` leaves the node as it was last written.
///
/// Not serde: it holds Bevy's own layout enums.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct RowLayout {
    /// How the children stand on the cross axis (a row's height, a column's width).
    pub align: Option<AlignItems>,
    /// Whether children that do not fit go on another line. `Some` of anything but
    /// [`FlexWrap::NoWrap`] also puts the skin gap between the lines.
    pub wrap: Option<FlexWrap>,
    /// How the children share the main axis.
    pub justify: Option<JustifyContent>,
    /// Which way the children run: `Some(FlexDirection::Row)` for a [`placed`](crate::placed) cluster
    /// that is a line, `Some(FlexDirection::Column)` for a stack. A [`row`](crate::row) and a
    /// [`column`](crate::column) have theirs by their name; set it on those only to turn one over.
    pub direction: Option<FlexDirection>,
    /// The node's `flex_shrink`: `Some(0.0)` is a rigid container that keeps its size when its host
    /// squeezes, `Some(1.0)` is Bevy's own default.
    pub shrink: Option<f32>,
}

/// The width a [`sized_panel`](crate::sized_panel) is, written by [`crate::apply_skin`]: a number of
/// logical px, or of body text sizes (an `em`, so it grows with the player's text size and the skin's
/// `text_scale`). The card is never wider than the room its host leaves (`max_width: 100%`: for a
/// dialog's card the window less the 5% the scrim keeps clear on each side) and has no least width,
/// so a card narrower than a dialog's usual measure is that narrow.
///
/// Put it on any [`panel`](crate::panel): a dialog's card (a direct child of
/// [`dialog`](crate::dialog)), or a form on a [`screen`](crate::screen), which keeps its cap of the
/// window less the safe edge. A width that is not finite is not written. Removing the component
/// leaves the node as it is; set it to another width instead.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CardWidth {
    /// Logical px, whatever the text size.
    Px(f32),
    /// Body text sizes, times the player's text scale.
    Text(f32),
}

/// Where a [`placed`](crate::placed) node stands in its host, written by [`crate::apply_skin`] as
/// the node's `left`, `top`, `right` and `bottom`. A side left [`Val::Auto`] is not pinned: with
/// `left` and `top` set and the rest `Auto` the node is the size of what it holds; with all four set
/// it fills the rectangle between them, which is what [`cover`](crate::cover) is.
///
/// Note `UiRect::default()` is `Val::ZERO` on every side, not `Auto`: write a corner as
/// `UiRect { right: Val::Px(10.0), bottom: Val::Px(20.0), ..UiRect::all(Val::Auto) }`. Not serde
/// (Bevy's `UiRect`). Removing the component leaves the node as it is.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct PlacedAt(pub UiRect);

/// What an [`aspect_image`](crate::aspect_image) is, written by [`crate::apply_skin`]: its `width`
/// in logical px and its `ratio`, width over height. The box is that wide unless its host is
/// narrower (`max_width: 100%`), and its height follows the width at the ratio, so a splash scales
/// down with its host and never distorts. A ratio that is not a number above zero is no ratio (the
/// height is then the image's own). Removing the component leaves the node as it is.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AspectBox {
    /// The width it is drawn at, logical px, while its host has the room.
    pub width: f32,
    /// Width over height.
    pub ratio: f32,
}

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
