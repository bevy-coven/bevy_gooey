//! Size motion: a section that opens grows to its height and one that closes shrinks away, and a
//! split whose first pane is collapsed or restored (`toggle_split`) slides there, so what is under
//! them travels instead of teleporting.
//!
//! Both move layout, on the skin's `motion.default` spring times `duration_scale`, and both land at
//! once under reduced or no motion ([`MotionSettings::lands_travel`]). The state they show is the
//! state at once: a section's [`crate::SectionOpen`] and a split's [`crate::SplitFraction`] say the
//! new value in the frame it is asked for, and only what is drawn and laid out follows. A divider
//! drag, a key step and a value the game writes move the panes at once, as before.

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::ComputedNode;

use crate::model::{SPRING_REST_POSITION, SpringState, spring_settled, spring_step};
use crate::motion::{MotionSettings, MotionToken};
use crate::skin::{GooeySkin, SkinScope, skin_of};
use crate::split::SplitFraction;

/// How near its target a split's shown fraction must be to count as there: a ten-thousandth of
/// the split, a tenth of a pixel across a thousand.
const FRACTION_REST: f32 = 1.0e-4;

/// A split sliding to the fraction a collapse or a restore asked for: the fraction it is laid out
/// at now, which [`crate::update_splits`] uses in place of [`SplitFraction`] until it arrives.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(from_reflect = false)]
#[reflect(Component)]
pub struct SplitGlide {
    #[reflect(ignore)]
    shown: SpringState,
    to: f32,
}

impl SplitGlide {
    /// A glide from `from` to `to`.
    pub(crate) fn new(from: f32, to: f32) -> Self {
        Self {
            shown: SpringState {
                value: from,
                velocity: 0.0,
            },
            to,
        }
    }

    /// The fraction the split is laid out at now.
    pub fn shown(&self) -> f32 {
        self.shown.value
    }
}

/// A section's body growing open or shrinking shut: its laid-out height now, logical px, and
/// which way it is going. While it lasts the body is clipped to that height; when it opens, the
/// body's own height comes back, and when it shuts, the body leaves layout as a closed one does.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(from_reflect = false)]
#[reflect(Component)]
pub struct SectionGlide {
    #[reflect(ignore)]
    height: SpringState,
    opening: bool,
    /// The body's own height and overflow, given back when the glide ends.
    own_height: Val,
    own_overflow: Overflow,
    /// Not laid out at its glide's height yet: what its widgets need is read only once it has.
    fresh: bool,
    /// What a shutting glide took off the body's subtree so nothing in it takes the pointer or a
    /// Tab stop while it shrinks: each node's own `Pickable` and `TabIndex`, given back if it turns
    /// round. Empty while opening.
    inert: Vec<(Entity, Option<Pickable>, Option<TabIndex>)>,
}

impl SectionGlide {
    /// A glide of a body laid out `from` logical px high, opening or shutting, that had `node`'s
    /// own height and overflow.
    pub(crate) fn new(from: f32, opening: bool, node: &Node) -> Self {
        Self {
            height: SpringState {
                value: from,
                velocity: 0.0,
            },
            opening,
            own_height: node.height,
            own_overflow: node.overflow,
            fresh: true,
            inert: Vec::new(),
        }
    }

    /// The same glide turned round, from where it is: a section toggled again on its way.
    pub(crate) fn turned(self, opening: bool) -> Self {
        Self { opening, ..self }
    }

    /// Whether the subtree has been made inert for a shutting glide.
    fn is_inert(&self) -> bool {
        !self.inert.is_empty()
    }

    /// The height the body is laid out at now, logical px.
    pub fn height(&self) -> f32 {
        self.height.value
    }

    /// Whether it is opening.
    pub fn opening(&self) -> bool {
        self.opening
    }
}

/// Steps every [`SplitGlide`] toward its fraction on real time, and takes it off when it arrives,
/// when the split's fraction is moved to somewhere else (a drag, a key, the game: those land), or
/// under reduced or no motion. Runs in [`crate::GooeySystems::Style`], before
/// [`crate::update_splits`].
#[allow(clippy::too_many_arguments)]
pub fn update_split_glides(
    settings: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    time: Option<Res<Time<Real>>>,
    mut commands: Commands,
    mut glides: Query<(Entity, &SplitFraction, &mut SplitGlide)>,
) {
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let lands = settings.lands_travel() || time.is_none();
    for (entity, fraction, mut glide) in &mut glides {
        if lands || fraction.0 != glide.to {
            commands.entity(entity).remove::<SplitGlide>();
            continue;
        }
        let spring = skin_of(&skin, entity, &scopes, &parents)
            .motion
            .spring(MotionToken::Default)
            .scaled(settings.duration_scale);
        let next = spring_step(spring, glide.shown, glide.to, delta);
        if spring_settled(spring, next, glide.to, FRACTION_REST) {
            commands.entity(entity).remove::<SplitGlide>();
        } else {
            glide.shown = next;
        }
    }
}

/// The subtree a shutting glide makes inert: the tree and each node's pointer and Tab stop.
type BodyTree<'w, 's> = (
    Query<'w, 's, &'static Children>,
    Query<'w, 's, (Option<&'static Pickable>, Option<&'static TabIndex>)>,
);

/// What a section's glide reads and writes of its body.
type Body = (
    Entity,
    &'static mut SectionGlide,
    &'static mut Node,
    &'static mut Visibility,
    &'static ComputedNode,
);

/// Steps every [`SectionGlide`] toward its body's height (the height its widgets need, when
/// opening; nothing, when shutting) on real time, laying the body out at that height and clipping
/// it. When it arrives an opened body gets its own height back and a shut one leaves layout and
/// sight. Under reduced or no motion it arrives in the frame it is seen. Runs in
/// [`crate::GooeySystems::Style`], after [`crate::update_sections`].
#[allow(clippy::too_many_arguments)]
pub fn update_section_glides(
    settings: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    time: Option<Res<Time<Real>>>,
    mut commands: Commands,
    mut bodies: Query<Body>,
    tree: BodyTree,
) {
    let (children, parts) = tree;
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let lands = settings.lands_travel() || time.is_none();
    for (entity, mut glide, mut node, mut visibility, computed) in &mut bodies {
        // A shutting body takes no pointer and no Tab stop while it shrinks, as a leaving widget
        // does; one turned round gives them back.
        if !glide.opening && !glide.is_inert() {
            let subtree: Vec<Entity> = std::iter::once(entity)
                .chain(children.iter_descendants(entity))
                .collect();
            for part in subtree {
                let (pickable, tab) = parts
                    .get(part)
                    .map(|(p, t)| (p.copied(), t.copied()))
                    .unwrap_or((None, None));
                glide.inert.push((part, pickable, tab));
                commands
                    .entity(part)
                    .insert(Pickable::IGNORE)
                    .remove::<TabIndex>();
            }
        } else if glide.opening && glide.is_inert() {
            give_back(&mut commands, std::mem::take(&mut glide.inert));
        }
        // The frame a glide starts the body has not been laid out at its height yet (an opening one
        // was out of layout): clip it there and read what its widgets need from the next frame.
        if glide.fresh && !lands {
            glide.fresh = false;
            let device = computed.inverse_scale_factor().recip();
            node.height = Val::Px((glide.height.value.max(0.0) * device).round() / device);
            node.overflow = Overflow::clip_y();
            continue;
        }
        // The height the body's widgets take: what they overflow its clipped box by, laid out, and
        // the frame and end padding a clipped box's overflow leaves out; or the body's own fixed
        // height, which it ends at.
        let inverse = computed.inverse_scale_factor();
        let needed = match glide.own_height {
            Val::Px(px) => px,
            _ => {
                (computed.content_size().y
                    + computed.border().min_inset.y
                    + computed.border().max_inset.y
                    + computed.padding().max_inset.y)
                    * inverse
            }
        };
        let target = if glide.opening { needed.max(0.0) } else { 0.0 };
        let spring = skin_of(&skin, entity, &scopes, &parents)
            .motion
            .spring(MotionToken::Default)
            .scaled(settings.duration_scale);
        let next = if lands {
            SpringState {
                value: target,
                velocity: 0.0,
            }
        } else {
            spring_step(spring, glide.height, target, delta)
        };
        // Layout rounds every edge to a device pixel on its own, so a height between two pixels
        // makes the widgets under it wobble by one as it settles. The height is set on whole device
        // pixels (a monotone spring stays monotone rounded), and it has arrived within half of one.
        let device = computed.inverse_scale_factor().recip();
        let half_pixel = 0.5 / device;
        let arrived = lands
            || (next.value - target).abs() < half_pixel
            || spring_settled(spring, next, target, SPRING_REST_POSITION);
        if arrived {
            let (own_height, own_overflow) = (glide.own_height, glide.own_overflow);
            node.height = own_height;
            node.overflow = own_overflow;
            if !glide.opening {
                node.display = Display::None;
                if *visibility != Visibility::Hidden {
                    *visibility = Visibility::Hidden;
                }
            }
            give_back(&mut commands, std::mem::take(&mut glide.inert));
            commands.entity(entity).remove::<SectionGlide>();
            continue;
        }
        glide.height = next;
        let height = Val::Px((next.value.max(0.0) * device).round() / device);
        if node.height != height {
            node.height = height;
        }
        if node.overflow != Overflow::clip_y() {
            node.overflow = Overflow::clip_y();
        }
    }
}

/// Puts back what a shutting glide took off a body's subtree.
fn give_back(commands: &mut Commands, inert: Vec<(Entity, Option<Pickable>, Option<TabIndex>)>) {
    for (part, pickable, tab) in inert {
        let Ok(mut node) = commands.get_entity(part) else {
            continue;
        };
        match pickable {
            Some(pickable) => node.insert(pickable),
            None => node.remove::<Pickable>(),
        };
        if let Some(tab) = tab {
            node.insert(tab);
        }
    }
}
