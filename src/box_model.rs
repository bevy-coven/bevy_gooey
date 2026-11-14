//! The box model any laid-out node can carry: [`Spacing`] (margin and padding, in steps of the
//! skin's spacing scale), and what the later items of G-85 add beside it. Every field is an
//! `Option`: a field left `None` leaves the node alone, so a component sets only what it names.
//! Research: `docs/research/layout-more.md`.

use bevy::prelude::*;

use crate::skin::GooeySkin;

/// One amount of room, in the skin's own terms: a step of [`GooeySkin::spacing`] (read with
/// [`GooeySkin::space`]; a step past the end is the last), or none. `Zero` is how to ask for none
/// where a role would otherwise give some (a [`panel`](crate::panel)'s padding); `None` in a
/// [`Sides`] leaves what the node has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, PartialEq, Clone)]
pub enum Space {
    /// No room.
    #[default]
    Zero,
    /// A step of the skin's spacing scale, smallest first.
    Step(usize),
}

impl Space {
    /// The room in logical px under `skin`.
    pub fn px(self, skin: &GooeySkin) -> f32 {
        match self {
            Space::Zero => 0.0,
            Space::Step(step) => skin.space(step),
        }
    }
}

/// A value for each side of a box, each in an `Option`: `None` leaves that side as it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct Sides<T> {
    /// The top side.
    pub top: Option<T>,
    /// The right side.
    pub right: Option<T>,
    /// The bottom side.
    pub bottom: Option<T>,
    /// The left side.
    pub left: Option<T>,
}

impl<T> Default for Sides<T> {
    /// Every side `None`: nothing set.
    fn default() -> Self {
        Self {
            top: None,
            right: None,
            bottom: None,
            left: None,
        }
    }
}

impl<T: Copy> Sides<T> {
    /// All four sides.
    pub fn all(value: T) -> Self {
        Self {
            top: Some(value),
            right: Some(value),
            bottom: Some(value),
            left: Some(value),
        }
    }

    /// Left and right.
    pub fn x(value: T) -> Self {
        Self {
            left: Some(value),
            right: Some(value),
            ..default()
        }
    }

    /// Top and bottom.
    pub fn y(value: T) -> Self {
        Self {
            top: Some(value),
            bottom: Some(value),
            ..default()
        }
    }
}

/// Margin and padding of a node, in steps of the skin's spacing scale ([`GooeySkin::spacing`]), so
/// a theme or a skin change moves all of it. Written by [`crate::apply_skin`] on **any** node that
/// has a [`Role`](crate::Role), which is every constructor in the crate: a [`row`](crate::row), a
/// [`column`](crate::column), a [`label`](crate::label), a [`panel`](crate::panel), a
/// [`placed_at`](crate::placed_at) node. A side left `None` leaves the node alone, so a panel keeps
/// its own padding when only its margin is set; it is applied after the role, so a side that is set
/// wins over the role's. Changing it restyles in the same frame; removing the component leaves the
/// node as it was last written (set the sides to [`Space::Zero`] instead).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     label("Section"),
///     Spacing {
///         margin: Sides { bottom: Some(Space::Step(2)), ..default() },
///         padding: Sides::x(Space::Step(1)),
///     },
/// ));
/// # }
/// ```
///
/// Not serde: it holds `Option`s of a generic.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct Spacing {
    /// The room outside the node.
    pub margin: Sides<Space>,
    /// The room inside the node, between its edge and its children.
    pub padding: Sides<Space>,
}

/// `rect` with every side `sides` names set to its room under `skin`.
pub(crate) fn spaced(mut rect: UiRect, sides: &Sides<Space>, skin: &GooeySkin) -> UiRect {
    let px = |space: Space| Val::Px(space.px(skin));
    if let Some(top) = sides.top {
        rect.top = px(top);
    }
    if let Some(right) = sides.right {
        rect.right = px(right);
    }
    if let Some(bottom) = sides.bottom {
        rect.bottom = px(bottom);
    }
    if let Some(left) = sides.left {
        rect.left = px(left);
    }
    rect
}
