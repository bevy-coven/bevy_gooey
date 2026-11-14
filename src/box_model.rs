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

/// Whether a node is shown, written by [`crate::apply_skin`] so the host never patches `Node`:
/// `Shown(false)` is `Display::None` (the node takes no room, what follows closes up, nothing in it
/// is drawn or picked), `Shown(true)` puts back `Display::Flex`, which is what every layout
/// constructor in the crate is. A node with no `Shown` is left as it is. It is for a page, a batch
/// column or a section that comes and goes; a list viewport (`Display::Block`) or a zero-width
/// [`cell`](crate::cell) has its display from its role and is not what this is for. Removing the
/// component leaves the node as it was last written: set `Shown(true)` instead.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// let page = commands.spawn((fill_region(), Shown(false), children![label("Hidden until shown")])).id();
/// // Later: commands.entity(page).insert(Shown(true));
/// # let _ = page;
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Shown(pub bool);

/// A length in the skin's own terms: logical px, a share of the host, or body text sizes.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Extent {
    /// Logical px, whatever the text size.
    Px(f32),
    /// A percentage of the host's size on that axis (`100.0` is all of it).
    Percent(f32),
    /// Body text sizes times the skin's `text_scale`, like a [`cell`](crate::cell)'s width.
    Text(f32),
}

impl Extent {
    /// The length as Bevy's `Val`, or `None` for a number that is not finite (refused, not written).
    pub(crate) fn val(self, text_px: f32) -> Option<Val> {
        let val = match self {
            Extent::Px(px) => Val::Px(px),
            Extent::Percent(percent) => Val::Percent(percent),
            Extent::Text(sizes) => Val::Px(sizes * text_px),
        };
        match val {
            Val::Px(v) | Val::Percent(v) if v.is_finite() => Some(val),
            _ => None,
        }
    }
}

/// The size of a node, written by [`crate::apply_skin`] on **any** node with a [`Role`](crate::Role):
/// a width and height, their least and greatest, and whether it clips what overflows. Every field is
/// an `Option` and a field left `None` leaves the node alone, so a [`panel`](crate::panel) given only
/// a width keeps its own height. A length that is not a finite number is refused. Use it on a
/// [`sized`](crate::sized) box, a [`row`](crate::row), a [`column`](crate::column), a
/// [`placed_at`](crate::placed_at) node, a [`label`](crate::label); a card's width stays
/// [`CardWidth`](crate::CardWidth). Removing the component leaves the node as it was last written.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // A terminal's line: all of its host's width, 18 px tall, clipped.
/// commands.spawn((
///     row(0.0),
///     Sizing {
///         width: Some(Extent::Percent(100.0)),
///         height: Some(Extent::Px(18.0)),
///         clip: Some(true),
///         ..default()
///     },
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sizing {
    /// The width.
    pub width: Option<Extent>,
    /// The height.
    pub height: Option<Extent>,
    /// The least width (`Some(Extent::Px(0.0))` lets a text shrink to nothing).
    pub min_width: Option<Extent>,
    /// The least height.
    pub min_height: Option<Extent>,
    /// The greatest width.
    pub max_width: Option<Extent>,
    /// The greatest height.
    pub max_height: Option<Extent>,
    /// `Some(true)` clips what overflows the box, `Some(false)` lets it show.
    pub clip: Option<bool>,
}

/// Which of the skin's colours a [`Shape`]'s border is drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Ink {
    /// [`GooeySkin::outline`](crate::GooeySkin::outline), the edge of a panel.
    #[default]
    Outline,
    /// [`GooeySkin::accent`](crate::GooeySkin::accent).
    Accent,
    /// [`GooeySkin::danger`](crate::GooeySkin::danger).
    Danger,
    /// [`GooeySkin::text`](crate::GooeySkin::text).
    Text,
    /// [`GooeySkin::text_muted`](crate::GooeySkin::text_muted).
    Muted,
}

impl Ink {
    pub(crate) fn color(self, skin: &GooeySkin) -> Color {
        match self {
            Ink::Outline => skin.outline,
            Ink::Accent => skin.accent,
            Ink::Danger => skin.danger,
            Ink::Text => skin.text,
            Ink::Muted => skin.text_muted,
        }
    }
}

/// How round a [`Shape`]'s corners are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Radius {
    /// A step of the skin's radius scale, [`GooeySkin::radii`](crate::GooeySkin::radii), smallest first.
    Step(usize),
    /// As round as the box allows: a pill, a disc when it is square.
    Full,
}

/// The look of a node's box, written by [`crate::apply_skin`] on **any** node with a
/// [`Role`](crate::Role), so a placed chip, a gizmo or a rail is not a hand-built `Node`: a border
/// (per side, in multiples of the skin's line weight [`border`](crate::GooeySkin::border), so `1.0` is
/// the skin's line and `0.0` none), the colour it is drawn in, a corner radius and a fill. Every field
/// is an `Option` (and a side of `border` too): `None` leaves the node and what its role draws, so a
/// shape with only a `radius` changes only the corners, and a border with no `ink` is the
/// [`Ink::Outline`] unless the role has an edge of its own. `fill` is a level of
/// [`GooeySkin::surfaces`](crate::GooeySkin::surfaces), like [`surface`](crate::surface), lowest first.
/// Changing it restyles in the same frame; removing the component leaves the node as it was last
/// written.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // A chip 30 px in from its host's corner: a size, a border, rounded corners, a raised fill.
/// commands.spawn((
///     placed_at(30.0, 40.0),
///     Sizing { width: Some(Extent::Px(120.0)), height: Some(Extent::Px(60.0)), ..default() },
///     Shape {
///         border: Sides::all(1.0),
///         ink: Some(Ink::Accent),
///         radius: Some(Radius::Step(1)),
///         fill: Some(1),
///     },
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct Shape {
    /// The border of each side, in line weights; `Some(0.0)` is none.
    pub border: Sides<f32>,
    /// The colour of the border.
    pub ink: Option<Ink>,
    /// The corner radius.
    pub radius: Option<Radius>,
    /// The fill, a level of the skin's surface ladder.
    pub fill: Option<u8>,
}

/// `rect` with every side `sides` names set to its weight times `line`; a weight that is not a finite
/// number, or is negative, is none.
pub(crate) fn lined(mut rect: UiRect, sides: &Sides<f32>, line: f32) -> UiRect {
    let px = |weight: f32| {
        Val::Px(if weight.is_finite() {
            weight.max(0.0) * line
        } else {
            0.0
        })
    };
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

impl Radius {
    pub(crate) fn border_radius(self, skin: &GooeySkin) -> BorderRadius {
        match self {
            Radius::Step(step) => BorderRadius::all(Val::Px(skin.corner(step))),
            Radius::Full => BorderRadius::MAX,
        }
    }
}

/// How a text (or any node) takes part in its row's or column's flex: whether it grows into what
/// the row leaves, whether it gives way, its least width, and where it stands on the cross axis by
/// its own field. A text that is rigid beside a name that gives way, a title that grows and may
/// narrow to nothing, a value that aligns itself to the centre of a tall neighbour. Written by
/// [`crate::apply_skin`] on any node with a [`Role`](crate::Role), after a [`Sizing`], so its
/// `min_width` wins over that one's; every field is an `Option` and a field left `None` leaves the
/// role's own (a [`fill_text`](crate::fill_text) keeps growing when only `align` is set). A number
/// that is not finite is refused. Use it on [`label_with`](crate::label_with) and
/// [`fill_text_with`](crate::fill_text_with), or beside any text as a component. Removing the
/// component leaves the node as it was last written.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     row(1.0),
///     children![
///         label_with("Reviewer", TextFlow { shrink: Some(0.0), ..default() }),
///         fill_text("a long name that gives way"),
///     ],
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct TextFlow {
    /// The node's `flex_grow`: `Some(1.0)` takes what the row leaves.
    pub grow: Option<f32>,
    /// The node's `flex_shrink`: `Some(0.0)` keeps its words' width, `Some(1.0)` gives way.
    pub shrink: Option<f32>,
    /// The least width: `Some(Extent::Px(0.0))` lets it narrow to nothing.
    pub min_width: Option<Extent>,
    /// Where it stands on the cross axis, on its own (`AlignSelf::Center`).
    pub align: Option<AlignSelf>,
}
