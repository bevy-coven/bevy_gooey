//! Pop: a node swells a little and settles, the second feedback channel a changed HUD number, a
//! chosen slot or a repaired panel gives beside its own words (`docs/research/pop.md`).
//!
//! Like the press scale, only the drawing scales: layout and the hit test keep the resting size.

use bevy::prelude::*;
use bevy::ui::{FixedNode, UiGlobalTransform};

use crate::model::{Spring, SpringState, spring_settled, spring_step};
use crate::motion::{MotionSettings, MotionToken};
use crate::press_scale::{PRESS_RELEASE_BOUNCE, PressScaleRestore, scale_subtree};
use crate::skin::{GooeySkin, SkinScope, skin_of};

/// The swell of a default [`Pop`], as a share of the node's size: Sanctuary's chosen-slot pop
/// (`Pop::with(0.10, 0.12)`), a design default.
pub const POP_GAIN: f32 = 0.10;

/// How near 1 a settling pop must be to count as at rest: a ten-thousandth of the node's size.
const POP_REST: f32 = 1.0e-4;

/// Swell by this share of the node's size about its centre and settle back, on the skin's snappy
/// spring with a slight bounce ([`PRESS_RELEASE_BOUNCE`]). **Inserting it starts a pop, again too**:
/// the component is a trigger and stays, so a game that wants a changed number to swell writes one
/// line when the number changes. A pop on a node still settling stacks on the scale it is drawn at,
/// so a counter ticking up reads as one fuller swell and not a stutter.
///
/// Only the drawing scales: layout and the hit test keep the resting size, and everything under
/// the node scales with it (a popover hung from it does not). It runs on real time, so a paused game
/// still pops its menu, and is off under reduced or no motion ([`MotionSettings::lands_travel`]).
/// A gain that is not a number above zero is no pop.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, coins: Entity) {
/// // The coin readout swells when the count changes.
/// commands.entity(coins).insert(Pop::default());
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Popped)]
pub struct Pop(pub f32);

impl Default for Pop {
    fn default() -> Self {
        Self(POP_GAIN)
    }
}

/// How far a [`Pop`] node is drawn swollen now: its scale and how fast it changes.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(from_reflect = false)]
#[reflect(Component)]
pub struct Popped {
    #[reflect(ignore)]
    scale: SpringState,
}

impl Default for Popped {
    fn default() -> Self {
        Self {
            scale: SpringState {
                value: 1.0,
                velocity: 0.0,
            },
        }
    }
}

impl Popped {
    /// The scale the node is drawn at now: 1 at rest.
    pub fn scale(&self) -> f32 {
        self.scale.value
    }

    fn at_rest(&self) -> bool {
        self.scale.value == 1.0 && self.scale.velocity == 0.0
    }
}

/// Starts a pop on a node whose [`Pop`] was just inserted and steps every pop toward 1, on real
/// time. A node at rest and not triggered is read and not written. Runs in
/// [`crate::GooeySystems::Style`].
pub fn update_pops(
    settings: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    time: Option<Res<Time<Real>>>,
    mut pops: Query<(Entity, Ref<Pop>, &mut Popped)>,
) {
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let lands = settings.lands_travel() || time.is_none();
    for (entity, pop, mut state) in &mut pops {
        let triggered = pop.is_changed();
        if !triggered && state.at_rest() {
            continue;
        }
        if lands {
            if !state.at_rest() {
                state.scale = SpringState {
                    value: 1.0,
                    velocity: 0.0,
                };
            }
            continue;
        }
        let mut now = state.scale;
        if triggered && pop.0.is_finite() && pop.0 > 0.0 {
            now.value = now.value.max(1.0) + pop.0;
        }
        let snappy = skin_of(&skin, entity, &scopes, &parents)
            .motion
            .spring(MotionToken::Snappy);
        let spring = Spring {
            duration: snappy.duration,
            bounce: PRESS_RELEASE_BOUNCE,
        }
        .scaled(settings.duration_scale);
        let next = spring_step(spring, now, 1.0, delta);
        let next = if spring_settled(spring, next, 1.0, POP_REST) {
            SpringState {
                value: 1.0,
                velocity: 0.0,
            }
        } else {
            next
        };
        if state.scale != next {
            state.scale = next;
        }
    }
}

/// Draws every swollen [`Pop`] node and everything under it at its scale about its centre,
/// remembering the resting transforms for [`crate::restore_press_scale`]. Runs in `PostUpdate`,
/// after Bevy's layout and the press scale and before its clipping. A node at rest is not touched.
pub fn draw_pops(
    widgets: Query<(Entity, &Popped), With<Pop>>,
    children: Query<&Children>,
    fixed: Query<(), With<FixedNode>>,
    mut globals: Query<&mut UiGlobalTransform>,
    mut restore: ResMut<PressScaleRestore>,
    mut stack: Local<Vec<Entity>>,
) {
    for (entity, state) in &widgets {
        let scale = state.scale();
        if scale == 1.0 || !scale.is_finite() {
            continue;
        }
        scale_subtree(
            entity,
            scale,
            &children,
            &fixed,
            &mut globals,
            &mut restore,
            &mut stack,
        );
    }
}
