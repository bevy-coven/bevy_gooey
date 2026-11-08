//! Hover groups (G-53): a container that reveals parts of itself while the pointer is over it.
//!
//! [`HoverGroup`] on a container says "this is one thing for hovering": a row, a card, a list item. A
//! [`GroupReveal`] on anything under it names an [`Opacity`] for the group at rest and another for the
//! group hovered (`rest: 0.0, hovered: 1.0` is a close button that shows only while its row is under the
//! pointer). The opacity does not jump: it enters on the skin's `motion.snappy` spring and leaves on
//! `motion.default`, retargeting from where it is drawn if the pointer comes back mid-way.
//!
//! What counts as hovered, in the order the crate reads it:
//!
//! - **the pointer over the group or anything inside it** (Bevy's [`Hovered`], which counts descendants);
//! - **a press that began inside it and is still held**: a drag that carries the pointer out of the row
//!   keeps the row revealed until the button is let go, so a control being used never vanishes under the
//!   hand that is using it;
//! - **visible focus on the group or anything inside it**: a keyboard or pad player has no hover, and a
//!   control that is revealed only by hover would be an invisible tab stop (WCAG 2.4.7, 2.5.8). Focus
//!   reveals in the frame it arrives, with no motion.
//!
//! Under reduced motion the opacity fades over `motion.fade_seconds` and nothing travels; under no
//! motion it lands. The first frame lands. A settled group writes nothing.
//!
//! The revealed node's [`Opacity`] belongs to this system while it has a [`GroupReveal`]: a fade the
//! game writes on the same node is overwritten, so put the reveal on a node of its own.

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::events::{PointerCancel, PointerDragEnd, PointerPress, PointerRelease};
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::{PointerButton, PointerId};
use bevy::prelude::*;

use crate::model::{SPRING_REST_POSITION, SpringState, spring_settled, spring_step};
use crate::motion::{MotionSettings, MotionToken};
use crate::presence::Opacity;
use crate::skin::{GooeySkin, SkinScope, skin_of};

/// A container that is one thing for hovering: [`GroupReveal`]s under it follow the pointer over it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // A row whose close button shows only while the row is under the pointer or has focus.
/// commands.spawn((
///     Node::default(),
///     HoverGroup,
///     children![label("Mixed.wav"), (button("x"), GroupReveal { rest: 0.0, hovered: 1.0 })],
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Hovered)]
pub struct HoverGroup;

/// The [`Opacity`] a node under a [`HoverGroup`] has while the group is at rest and while it is hovered
/// (held by a press that began in it, or holding visible focus). Both are held to 0 to 1.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[require(RevealTrack)]
pub struct GroupReveal {
    /// The opacity while the group is at rest.
    pub rest: f32,
    /// The opacity while the group is hovered.
    pub hovered: f32,
}

/// Where a [`GroupReveal`] is on its way between rest (0) and hovered (1), and whether it has looked at
/// its group yet. The crate's state; a game reads [`Opacity`].
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(from_reflect = false)]
#[reflect(Component)]
pub struct RevealTrack {
    /// The progress from rest to hovered and how fast it is changing.
    #[reflect(ignore)]
    progress: SpringState,
    /// Whether the first look has landed it where its group says.
    primed: bool,
}

impl RevealTrack {
    /// How far from rest to hovered the node is drawn, 0 to 1.
    pub fn progress(&self) -> f32 {
        self.progress.value
    }
}

/// The pointer whose press began inside a [`HoverGroup`] and has not been let go.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct GroupHold(PointerId);

/// Every [`HoverGroup`] at or above `entity`.
fn groups_at_or_above(
    entity: Entity,
    groups: &Query<(), With<HoverGroup>>,
    parents: &Query<&ChildOf>,
) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut at = entity;
    loop {
        if groups.contains(at) {
            found.push(at);
        }
        match parents.get(at) {
            Ok(parent) => at = parent.parent(),
            Err(_) => return found,
        }
    }
}

/// A primary press on anything inside a group holds the group revealed until that pointer lets go.
pub(crate) fn holds_on_press(
    press: On<PointerPress>,
    groups: Query<(), With<HoverGroup>>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    if press.button != PointerButton::Primary
        || press.event_target() != press.original_event_target()
    {
        return;
    }
    for group in groups_at_or_above(press.entity, &groups, &parents) {
        commands.entity(group).insert(GroupHold(press.pointer.id));
    }
}

/// Lets go every hold the pointer has.
fn let_go(pointer: PointerId, holds: &Query<(Entity, &GroupHold)>, commands: &mut Commands) {
    for (group, hold) in holds {
        if hold.0 == pointer {
            commands.entity(group).remove::<GroupHold>();
        }
    }
}

/// The pointer lets go over something.
pub(crate) fn holds_on_release(
    release: On<PointerRelease>,
    holds: Query<(Entity, &GroupHold)>,
    mut commands: Commands,
) {
    if release.button == PointerButton::Primary {
        let_go(release.pointer.id, &holds, &mut commands);
    }
}

/// The pointer lets go after dragging off what it pressed: Bevy sends that no release, but every drag
/// ends.
pub(crate) fn holds_on_drag_end(
    end: On<PointerDragEnd>,
    holds: Query<(Entity, &GroupHold)>,
    mut commands: Commands,
) {
    if end.button == PointerButton::Primary {
        let_go(end.pointer.id, &holds, &mut commands);
    }
}

/// The pointer is cancelled (a touch lifted off the screen, a pointer that left the window).
pub(crate) fn holds_on_cancel(
    cancel: On<PointerCancel>,
    holds: Query<(Entity, &GroupHold)>,
    mut commands: Commands,
) {
    let_go(cancel.pointer.id, &holds, &mut commands);
}

/// Where a [`GroupReveal`]'s group stands: hovered, held, or holding visible focus.
#[derive(Clone, Copy)]
struct GroupState {
    /// Under the pointer, or held by a press that began in it.
    revealed: bool,
    /// Visible focus is on it or inside it: revealed at once, with no motion.
    focused: bool,
}

/// Moves every [`GroupReveal`] toward its group's state and writes its [`Opacity`]. Runs in
/// [`crate::GooeySystems::Style`] before [`spread_opacity`](crate::spread_opacity), which turns an
/// `Opacity` into what its subtree is drawn at; with nothing changing it writes nothing.
#[allow(clippy::too_many_arguments)]
pub fn update_group_reveals(
    settings: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    time: Option<Res<Time<Real>>>,
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<InputFocusVisible>>,
    groups: Query<(&Hovered, Has<GroupHold>), With<HoverGroup>>,
    parents: Query<&ChildOf>,
    mut reveals: Query<(Entity, &GroupReveal, &mut RevealTrack, Option<&mut Opacity>)>,
    mut chain: Local<Vec<Entity>>,
    mut commands: Commands,
) {
    if reveals.is_empty() {
        return;
    }
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    // The nodes from the focused one up to the root: a group among them holds visible focus.
    chain.clear();
    if visible.is_some_and(|v| v.0)
        && let Some(mut at) = focus.and_then(|f| f.get())
    {
        chain.push(at);
        while let Ok(parent) = parents.get(at) {
            at = parent.parent();
            chain.push(at);
        }
    }
    for (entity, reveal, mut track, own) in &mut reveals {
        let state = nearest_group(entity, &groups, &parents, &chain);
        let target = if state.revealed || state.focused {
            1.0
        } else {
            0.0
        };
        let landing = state.focused || settings.is_off() || time.is_none() || !track.primed;
        let fade = settings.reduced && !landing;
        if landing {
            let landed = SpringState {
                value: target,
                velocity: 0.0,
            };
            if track.progress != landed || !track.primed {
                track.progress = landed;
                track.primed = true;
            }
        } else if fade {
            // Reduced motion: a timed fade, nothing travels.
            let seconds = skin.motion.fade_seconds.max(f32::EPSILON);
            let step = delta / seconds;
            let value = if target > track.progress.value {
                (track.progress.value + step).min(target)
            } else {
                (track.progress.value - step).max(target)
            };
            if value != track.progress.value {
                track.progress = SpringState {
                    value,
                    velocity: 0.0,
                };
            }
        } else if track.progress.value != target || track.progress.velocity != 0.0 {
            // Entering takes the snappy spring and leaving the default one; a reversal mid-way
            // retargets from where it is drawn, with the speed it has.
            let token = if target > 0.5 {
                MotionToken::Snappy
            } else {
                MotionToken::Default
            };
            let spring = skin_of(&skin, entity, &scopes, &parents)
                .motion
                .spring(token)
                .scaled(settings.duration_scale);
            let next = spring_step(spring, track.progress, target, delta);
            track.progress = if spring_settled(spring, next, target, SPRING_REST_POSITION) {
                SpringState {
                    value: target,
                    velocity: 0.0,
                }
            } else {
                next
            };
        }
        let (rest, hovered) = (clamp_unit(reveal.rest), clamp_unit(reveal.hovered));
        let want = rest + (hovered - rest) * track.progress.value.clamp(0.0, 1.0);
        match own {
            Some(mut now) => {
                if (now.0 - want).abs() > f32::EPSILON {
                    now.0 = want;
                }
            }
            None => {
                commands.entity(entity).insert(Opacity(want));
            }
        }
    }
}

/// `value` held to 0 to 1, and 0 for a value that is not a number.
fn clamp_unit(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// The state of the nearest [`HoverGroup`] at or above `entity`; a node under none is at rest.
fn nearest_group(
    entity: Entity,
    groups: &Query<(&Hovered, Has<GroupHold>), With<HoverGroup>>,
    parents: &Query<&ChildOf>,
    focus_chain: &[Entity],
) -> GroupState {
    let mut at = entity;
    loop {
        if let Ok((hovered, held)) = groups.get(at) {
            return GroupState {
                revealed: hovered.0 || held,
                focused: focus_chain.contains(&at),
            };
        }
        match parents.get(at) {
            Ok(parent) => at = parent.parent(),
            Err(_) => {
                return GroupState {
                    revealed: false,
                    focused: false,
                };
            }
        }
    }
}
