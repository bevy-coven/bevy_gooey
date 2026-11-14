//! The box model any laid-out node can carry: [`Spacing`] (margin and padding, in steps of the
//! skin's spacing scale), and what the later items of G-85 add beside it. Every field is an
//! `Option`: a field left `None` leaves the node alone, so a component sets only what it names.
//! Research: `docs/research/layout-more.md`.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use crate::skin::GooeySkin;

/// One amount of room: a step of the skin's spacing scale ([`GooeySkin::spacing`], read with
/// [`GooeySkin::space`]; a step past the end is the last), a literal logical px for the few
/// measures that are not on the scale (a chip's 6 by 3, a tab's 12 by 8), or none. `Zero` is how to
/// ask for none where a role would otherwise give some (a [`panel`](crate::panel)'s padding); `None` in
/// a [`Sides`] leaves what the node has. A `Px` is still written by [`crate::apply_skin`] and
/// finite-checked: below zero is zero, a number that is not finite leaves its side alone. It does
/// **not** follow the skin's scale or the text size: prefer a `Step` where a step will do.
///
/// ```
/// # use bevy_gooey::*;
/// let skin = GooeySkin::dark();
/// assert_eq!(Space::Step(1).px(&skin), skin.space(1));
/// assert_eq!(Space::Px(6.0).px(&skin), 6.0);
/// assert_eq!(Space::Zero.px(&skin), 0.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Debug, Default, PartialEq, Clone)]
pub enum Space {
    /// No room.
    #[default]
    Zero,
    /// A step of the skin's spacing scale, smallest first.
    Step(usize),
    /// A literal number of logical px, off the scale.
    Px(f32),
}

impl Space {
    /// The room in logical px under `skin`; a `Px` that is not a finite number is 0 here (it is left
    /// alone, not written, by [`crate::apply_skin`]).
    pub fn px(self, skin: &GooeySkin) -> f32 {
        self.resolve(skin).unwrap_or(0.0)
    }

    /// The room, or `None` for a literal that is not a finite number (refused, not written).
    pub(crate) fn resolve(self, skin: &GooeySkin) -> Option<f32> {
        match self {
            Space::Zero => Some(0.0),
            Space::Step(step) => Some(skin.space(step)),
            Space::Px(px) => px.is_finite().then(|| px.max(0.0)),
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
    for (field, space) in [
        (&mut rect.top, sides.top),
        (&mut rect.right, sides.right),
        (&mut rect.bottom, sides.bottom),
        (&mut rect.left, sides.left),
    ] {
        if let Some(px) = space.and_then(|space| space.resolve(skin)) {
            *field = Val::Px(px);
        }
    }
    rect
}

/// Whether a node is shown, written by [`crate::apply_skin`] so the host never patches `Node`:
/// `Shown(false)` is `Display::None` **and** `Visibility::Hidden` (the node takes no room, what
/// follows closes up, nothing in it is drawn or picked, and none of its widgets is a tab stop, a
/// focus region or an open dialog: Bevy's `Display::None` alone leaves a node's
/// `InheritedVisibility`, which focus navigation reads, untouched). `Shown(true)` gives back what the
/// node had when it was hidden, its display (a grid stays a grid) and its visibility (a `Visible` stays
/// visible). So `(dialog(), Shown(false))` is a modal that is hidden until shown and traps nothing.
/// A node with no `Shown` is left as it is. **To hide or show a node that already exists, insert or
/// change its `Shown`** (`commands.entity(e).insert(Shown(false))`, or `*shown = Shown(true)` through a
/// query): any node with a [`Role`](crate::Role) takes it, never write `Node::display` yourself. Removing
/// the component while hidden leaves the node hidden: set `Shown(true)` instead.
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
#[component(on_insert = shown_inserted)]
pub struct Shown(pub bool);

impl Shown {
    /// `Shown(false)`: a node that **starts closed**. Closing happens when the component is inserted
    /// (a hook), not at the next restyle, so a node spawned after a frame's style pass is already
    /// `Display::None` and `Visibility::Hidden` before any layout could draw it: no frame open.
    pub const fn closed() -> Self {
        Shown(false)
    }
}

/// Closes or opens a node the moment its [`Shown`] is inserted (the hook), so a node that starts closed
/// is never drawn or focusable for a frame; `apply_skin` writes the same and finds it done.
fn shown_inserted(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(shown) = world.get::<Shown>(entity).map(|shown| shown.0) else {
        return;
    };
    let memory = world.get::<ShownMemory>(entity).copied();
    if !shown {
        let display = world
            .get::<Node>(entity)
            .map_or(Display::Flex, |node| node.display);
        let visibility = world.get::<Visibility>(entity).copied().unwrap_or_default();
        if memory.is_none() {
            // A node already `None` (the host closed it) comes back as the flex every constructor is.
            let remembered = ShownMemory {
                display: if display == Display::None {
                    Display::Flex
                } else {
                    display
                },
                visibility,
            };
            // Applied only if no memory has landed by then: two inserts in one flush both see none
            // here, and the first (which read the real display) must win over the second (which
            // reads the `None` the first wrote).
            world.commands().queue(move |world: &mut World| {
                if let Ok(mut entity) = world.get_entity_mut(entity)
                    && !entity.contains::<ShownMemory>()
                {
                    entity.insert(remembered);
                }
            });
        }
        if let Some(mut node) = world.get_mut::<Node>(entity)
            && node.display != Display::None
        {
            node.display = Display::None;
        }
        if visibility != Visibility::Hidden {
            world.commands().entity(entity).insert(Visibility::Hidden);
        }
    } else if let Some(memory) = memory {
        if let Some(mut node) = world.get_mut::<Node>(entity) {
            node.display = memory.display;
        }
        world.commands().entity(entity).insert(memory.visibility);
        world.commands().entity(entity).remove::<ShownMemory>();
    }
}

/// A length in the skin's own terms: logical px, a share of the host, body text sizes, or a share of
/// the viewport.
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
    /// The content's size (`Val::Auto`): as wide or as tall as what the node holds. With a
    /// [`TextFlow`] `grow` it is "what my siblings leave" (a filling split).
    Auto,
    /// Lines of body text rounded **up** to a whole device pixel at the node's scale, so it is never
    /// short of the text it holds (an unrounded [`Extent::Lines`] can be a fraction of a pixel short)
    /// and follows the text size live: a skin change restyles every node.
    LinesCeil(f32),
    /// Lines of body text: `n` times the skin's list-line height (1.2 body text sizes), so a least
    /// height that follows the player's text size (a receipt row two lines tall).
    Lines(f32),
    /// A percentage of the viewport's height (`100.0` is the whole window): a root that is the
    /// window's size whatever its host is. Bevy has no dynamic-viewport unit, so there is no `Dvh`.
    Vh(f32),
    /// A percentage of the viewport's width.
    Vw(f32),
    /// A percentage of the viewport's smaller side.
    VMin(f32),
    /// A percentage of the viewport's larger side.
    VMax(f32),
}

impl Extent {
    /// The length as Bevy's `Val`, or `None` for a number that is not finite (refused, not written).
    pub(crate) fn val(self, text_px: f32, line_px: f32, scale: f32) -> Option<Val> {
        let val = match self {
            Extent::Px(px) => Val::Px(px),
            Extent::Percent(percent) => Val::Percent(percent),
            Extent::Text(sizes) => Val::Px(sizes * text_px),
            Extent::Lines(lines) => Val::Px(lines * line_px),
            Extent::LinesCeil(lines) => {
                let px = lines * line_px;
                if px.is_finite() && scale.is_finite() && scale > 0.0 {
                    Val::Px(((px.max(0.0) * scale) - 0.001).ceil().max(0.0) / scale)
                } else {
                    Val::Px(px)
                }
            }
            Extent::Auto => return Some(Val::Auto),
            Extent::Vh(v) => Val::Vh(v),
            Extent::Vw(v) => Val::Vw(v),
            Extent::VMin(v) => Val::VMin(v),
            Extent::VMax(v) => Val::VMax(v),
        };
        // Not finite is refused; below zero is zero, as a card's width is.
        match val {
            Val::Px(v) if v.is_finite() => Some(Val::Px(v.max(0.0))),
            Val::Percent(v) if v.is_finite() => Some(Val::Percent(v.max(0.0))),
            Val::Vh(v) if v.is_finite() => Some(Val::Vh(v.max(0.0))),
            Val::Vw(v) if v.is_finite() => Some(Val::Vw(v.max(0.0))),
            Val::VMin(v) if v.is_finite() => Some(Val::VMin(v.max(0.0))),
            Val::VMax(v) if v.is_finite() => Some(Val::VMax(v.max(0.0))),
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
/// the skin's line and `0.0` none; **a role whose focus ring is its border, a [`button`](crate::button), has no ring if a `0.0` zeroes that side's width**), the colour it is drawn in, a corner radius and a fill. Every field
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
    // A weight that is not a finite number leaves its side alone; below zero is none.
    let px = |weight: f32| weight.is_finite().then(|| Val::Px(weight.max(0.0) * line));
    for (field, weight) in [
        (&mut rect.top, sides.top),
        (&mut rect.right, sides.right),
        (&mut rect.bottom, sides.bottom),
        (&mut rect.left, sides.left),
    ] {
        if let Some(val) = weight.and_then(px) {
            *field = val;
        }
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

/// What a node had before [`Shown`] hid it, so showing it again gives that back and not a guess: the
/// display (a grid stays a grid) and the visibility (a `Visible` stays visible). Private; put on by
/// [`crate::apply_skin`] the first time a `Shown(false)` is written and taken off when it is shown.
#[derive(Component, Clone, Copy, Debug)]
pub struct ShownMemory {
    pub(crate) display: Display,
    pub(crate) visibility: Visibility,
}

/// A border in logical px, **not snapped to device pixels**, per side, beside a [`Shape`]. Every
/// border the crate writes from the skin is a whole number of device pixels (a role's frame, a focus
/// ring, a [`Shape::border`] weight: [`snap_line_width`](crate::snap_line_width) rounds a 2 px line at
/// 1.25x to 2 device px, 1.6 logical), which keeps a line one weight on every side; a `BorderPx` is the
/// logical px it says (`3.0` is 3 px at any scale; Bevy's layout then rounds it to its own device
/// pixels like any `Node` border). Use it where an exact visible width is the requirement, a drag box
/// or a pick outline; use a `Shape` weight everywhere else. Every side is an `Option` (`None` leaves
/// the side, a number that is not finite leaves it, below zero is none). **A role whose focus ring is its border (a [`button`](crate::button)) loses the ring if this zeroes its width.** A side set here wins over a
/// `Shape`'s on the same side; the colour is the `Shape`'s `ink`, else the outline unless the role has
/// an edge of its own. Removing the component leaves the node as it was last written.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // The drag rectangle: 3 px, exactly, in the accent.
/// commands.spawn((
///     placed_at(10.0, 10.0),
///     BorderPx(Sides::all(3.0)),
///     Shape { ink: Some(Ink::Accent), ..default() },
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct BorderPx(pub Sides<f32>);

/// `rect` with every side `sides` names set to that many logical px, unsnapped; a number that is not
/// finite leaves its side alone, below zero is none.
pub(crate) fn pixeled(mut rect: UiRect, sides: &Sides<f32>) -> UiRect {
    for (field, px) in [
        (&mut rect.top, sides.top),
        (&mut rect.right, sides.right),
        (&mut rect.bottom, sides.bottom),
        (&mut rect.left, sides.left),
    ] {
        if let Some(px) = px.filter(|px| px.is_finite()) {
            *field = Val::Px(px.max(0.0));
        }
    }
    rect
}

/// A colour system the host owns, sharing a node with gooey's layout: the fill, the ink and the edge
/// colour of **this node**, replacing what [`crate::apply_skin`] would draw for its role, a
/// [`Shape`] or a state. Every field is an `Option` and a field left `None` leaves the role's own
/// colour. It is honoured for any [`Role`](crate::Role) and survives a skin change, which restyles
/// every node: the skin does not overwrite it. **The host owns hover and press feedback under an
/// override**: the `fill` replaces the role's whole fill chain, so the fill is the same whatever the
/// state (`under_an_ink_override_fill_press_does_not_change_the_fill` pins it); a host that colours by
/// state changes the component, and the change restyles in the same frame.
///
/// - `fill` is the background. Over a [`Shape::fill`] and a role's fill.
/// - `ink` is the text colour of a text node's own role ([`label`](crate::label),
///   [`fill_text`](crate::fill_text), a button's `Role::ButtonText`); a node that is not text has none
///   to replace. It does not cascade to children: each text carries its own.
/// - `edge` is the border colour, over a [`Shape::ink`]. A focused widget's focus ring still wins over
///   it (only the ring: any other edge a role has is replaced), because it is how a keyboard player
///   sees where they are. **A role whose ring IS its border (a [`button`](crate::button)) has no ring
///   if its border width is zero**, so do not zero such a role's border with a [`Shape`] weight of
///   `0.0` or a [`BorderPx`] of `0.0`.
///
/// It sets colours only: size, border width and spacing stay [`Sizing`], [`Shape`], [`BorderPx`] and
/// [`Spacing`]. Removing the component leaves the node as it was last written.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // A tag chip: gooey's button and layout, the host's own colours for this one node.
/// commands.spawn((
///     button("rust"),
///     InkOverride {
///         fill: Some(Color::srgb(0.2, 0.3, 0.5)),
///         edge: Some(Color::srgb(0.4, 0.5, 0.8)),
///         ..default()
///     },
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct InkOverride {
    /// The background colour.
    pub fill: Option<Color>,
    /// The text colour of a text node.
    pub ink: Option<Color>,
    /// The border colour.
    pub edge: Option<Color>,
}

/// Marks a node whose **geometry the host writes itself, every frame**: [`crate::apply_skin`] then
/// leaves alone exactly two things, the insets a [`PlacedAt`](crate::PlacedAt) would write on a
/// [`placed`](crate::placed) node (`left`, `top`, `right`, `bottom`) and the lengths a [`Sizing`] would
/// write (`width`, `height`, `min_*`, `max_*`), and still writes everything else (borders, radius,
/// padding, colours, overflow, and the widths a role gives itself or a
/// [`CardWidth`](crate::CardWidth), [`AspectBox`](crate::AspectBox) or [`BoxSize`](crate::BoxSize) says:
/// those are not guarded). Adding or removing the marker does not restyle the node by itself (it has no
/// change trigger): the node is read as it is at its next restyle, so add it when you spawn the node. It is the light path for a gizmo, a caret,
/// a terminal's rows or a badge cluster that a system moves each frame: writing `PlacedAt` or `Sizing`
/// every frame makes the skin restyle the node every frame (measured in `docs/research/layout-last.md`:
/// about two thirds more per frame than writing `Node`, for a node that moves every frame, and next to
/// nothing when only the nodes that moved are written), so a node that moves every frame writes its own
/// `Node` and carries this marker, which also keeps a skin change from resetting the geometry it wrote.
/// A node with no marker is positioned and sized by its `PlacedAt` and `Sizing`, as before.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((placed_at(0.0, 0.0), HostGeometry, Shape { radius: Some(Radius::Full), ..default() }));
/// // A system then writes this node's `Node::left`, `top`, `width` and `height` each frame.
/// # }
/// fn follow(mut dots: Query<&mut Node, With<HostGeometry>>) {
///     for mut node in &mut dots {
///         node.left = Val::Px(12.0);
///     }
/// }
/// # let _ = follow;
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct HostGeometry;

/// The colour twin of [`HostGeometry`]: marks a node whose **fill and edge the host paints itself,
/// by state**, from a colour system of its own (a chip held, a row selected or hovered, a field
/// failing). [`crate::apply_skin`] then writes neither its `BackgroundColor` nor its `BorderColor`,
/// whatever the node's role, state, [`Shape`] or [`InkOverride`] say and whatever skin change
/// restyles it, and still writes **every other field**: size, border width, radius, padding, text
/// colour, overflow. Where an [`InkOverride`] pins one colour, `InkFollow` lets the host follow the
/// state with as many colours as it has. The host's colours are the ones on the node: put them there
/// at spawn or from a system (the skin gives the node no colours of its own until the marker is
/// taken off, so a node that is spawned without any is drawn transparent). Removing the marker
/// restyles the node in the same frame and the skin paints it again, easing from the colour shown (no easing
/// state is left behind); adding it leaves the colours as they are. **Colour motion does not touch the
/// fill and edge of such a node either**: with motion on, the colour the host writes is the one drawn on
/// the next frame (only its text and icon colours still ease), so a host that diff-checks its target
/// never fights a spring. **Keyboard focus: the host paints the focus ring** (a button's ring is its
/// border colour): the skin writes none on an `InkFollow` node, so light it from your own system when
/// the node has focus (`the_skin_writes_no_focus_ring_on_an_ink_follow_button`).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     button("held"),
///     InkFollow,
///     BackgroundColor(Color::srgb(0.2, 0.3, 0.5)),
///     BorderColor::all(Color::srgb(0.4, 0.5, 0.8)),
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct InkFollow;

/// A gap between a container's children in **logical px**, where [`LayoutGap`](crate::LayoutGap) is a
/// multiple of the skin's gap: for the measures a host keeps in its own tokens (a bar's 8 px, a tab
/// strip's 2 px). Written by [`crate::apply_skin`] as both `row_gap` and `column_gap` on a
/// [`row`](crate::row), a [`column`](crate::column), a [`button_row`](crate::button_row), a
/// [`fill_column`](crate::fill_column), a [`fill_region`](crate::fill_region) and the containers that
/// take a [`LayoutGap`](crate::LayoutGap) (a panel, a surface, a card, a placed cluster, a
/// [`button_box`](crate::button_box)...); it wins over
/// a `LayoutGap` on the same node, including the gap a wrapping flow puts between its lines. A number
/// that is not finite leaves the role's own gap, below zero is none. It does not follow the skin or the
/// text size (prefer a `LayoutGap` where a multiple will do). Removing it leaves the node as it was last
/// written: set it to another value instead.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((row(0.0), GapPx(2.0), children![edge_tab("Meshes"), edge_tab("Tiles")]));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GapPx(pub f32);

/// The visibility a node had before [`ShownFollow`] hid it, so showing it again gives that back.
#[derive(Component, Clone, Copy, Debug)]
pub struct FollowMemory(Visibility);

/// For a host that **owns `Node::display`** (an open/closed panel written by its own systems):
/// makes gooey hide the node from focus and sight whenever the host's `display` is `Display::None`.
/// [`Shown`] writes `display` itself, which a host that already writes it in several systems cannot
/// adopt; with `ShownFollow` the host keeps writing `display` and `follow_display` follows it:
/// `Display::None` is `Visibility::Hidden` (the page's widgets are no tab stops, focus regions or
/// open dialogs: `Display::None` alone leaves `InheritedVisibility`, which focus reads), anything else
/// gives back the visibility the node had. Nothing of gooey's rewrites the host's `display` (the skin
/// leaves a node's display to the roles that own one; a `Panel`, `Surface`, `Card`, a row and the other
/// containers do not). **Timing:** the visibility follows the display the host wrote when
/// `follow_display` next runs, which is the same frame if the host's systems run before
/// [`GooeySystems::Style`](crate::GooeySystems), and **one frame later** if they run after it (or in
/// `PostUpdate`): that one frame the node is laid out closed but still `Visible` to focus. Adding it to a node that is already closed hides it, removing it gives the
/// visibility back and leaves the display the host's. Use **either** `Shown` or `ShownFollow` on a
/// node, not both (`Shown` wins).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// let page = commands.spawn((panel(), ShownFollow, children![button("In the page")])).id();
/// // Any of the host's systems: `node.display = Display::None;` closes it, `Display::Flex` opens it.
/// # let _ = page;
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[component(on_add = follow_added)]
pub struct ShownFollow;

impl ShownFollow {
    /// A [`ShownFollow`] node that **starts closed** (`Display::None` and `Visibility::Hidden` from the
    /// moment it is spawned, no frame open): for a panel whose `display` is its open/closed state and
    /// is closed until a host system opens it by writing `display`.
    ///
    /// ```
    /// # use bevy::prelude::*;
    /// # use bevy_gooey::*;
    /// # fn ui(mut commands: Commands) {
    /// commands.spawn((panel(), ShownFollow::closed(), children![button("In the page")]));
    /// # }
    /// ```
    pub fn closed() -> impl Bundle {
        (ShownFollow, StartsClosed)
    }
}

/// One-shot: a node that is closed (`Display::None`) at the moment it is spawned. Put on by
/// [`ShownFollow::closed`]; taken off again as soon as it has acted, so the host's later writes of
/// `display` are never second-guessed.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[component(on_add = starts_closed)]
pub struct StartsClosed;

/// Closes a [`ShownFollow`] node: remembers the visibility it had and hides it.
fn close_followed(world: &mut DeferredWorld, entity: Entity) {
    if world.get::<FollowMemory>(entity).is_some() {
        return;
    }
    let current = world.get::<Visibility>(entity).copied().unwrap_or_default();
    world
        .commands()
        .entity(entity)
        .insert(FollowMemory(current));
    if current != Visibility::Hidden {
        world.commands().entity(entity).insert(Visibility::Hidden);
    }
}

fn follow_added(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    if world
        .get::<Node>(entity)
        .is_some_and(|node| node.display == Display::None)
    {
        close_followed(&mut world, entity);
    }
}

fn starts_closed(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    if let Some(mut node) = world.get_mut::<Node>(entity) {
        node.display = Display::None;
    }
    if world.get::<ShownFollow>(entity).is_some() {
        close_followed(&mut world, entity);
    }
    world.commands().entity(entity).remove::<StartsClosed>();
}

/// Makes a [`ShownFollow`] node's visibility follow the display its host wrote.
#[allow(clippy::type_complexity)]
pub fn follow_display(
    mut nodes: Query<
        (Entity, &Node, Option<&Visibility>, Option<&FollowMemory>),
        (
            With<ShownFollow>,
            Without<Shown>,
            Or<(Changed<Node>, Added<ShownFollow>)>,
        ),
    >,
    mut gone: RemovedComponents<ShownFollow>,
    mut remembered: Query<(Option<&Visibility>, &FollowMemory), Without<ShownFollow>>,
    mut commands: Commands,
) {
    for (entity, node, visibility, memory) in &mut nodes {
        let closed = node.display == Display::None;
        match (closed, memory) {
            (true, None) => {
                let current = visibility.copied().unwrap_or_default();
                commands.entity(entity).insert(FollowMemory(current));
                if current != Visibility::Hidden {
                    commands.entity(entity).insert(Visibility::Hidden);
                }
            }
            (false, Some(memory)) => {
                // A visibility the host set while the node was closed is the host's: only the
                // `Hidden` this system wrote is replaced by what the node had.
                if visibility.copied().unwrap_or_default() == Visibility::Hidden {
                    commands.entity(entity).insert(memory.0);
                }
                commands.entity(entity).remove::<FollowMemory>();
            }
            _ => {}
        }
    }
    for entity in gone.read() {
        if let Ok((_, memory)) = remembered.get_mut(entity) {
            commands.entity(entity).insert(memory.0);
            commands.entity(entity).remove::<FollowMemory>();
        }
    }
}

/// The lane a [`scroll_region`](crate::scroll_region)'s rail keeps clear on the right of its content:
/// `None` is the list's own (the rail's width and a skin gap), `Some(Space)` the host's (a step, or a
/// literal px). It is on the region's scrolling viewport, **requires** a [`ScrollGlide`](crate::ScrollGlide)
/// and [`ScrollEdgeFade`](crate::ScrollEdgeFade) (a scroll the crate makes glides, an edge that hides
/// content fades), and marks the viewport the rail scrolls. Written by [`crate::apply_skin`] as the
/// viewport's right padding (inside the clip, so the content stops short of the rail and still counts
/// toward what scrolls); a [`Spacing`] on the same node still wins over it. Change it and the viewport
/// restyles in the same frame.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[require(crate::ScrollGlide, crate::ScrollEdgeFade)]
pub struct ScrollGutter(pub Option<Space>);
