//! Press scale: a card or a slot shrinks a little about its centre while a press on it is held,
//! the tactile answer a desktop tile gives, and springs back when it is let go.
//!
//! Only the drawing scales. Layout keeps the widget's resting size, and so does the hit test: the
//! scaled `UiGlobalTransform`s are written after Bevy's layout for the frame to draw, and the
//! resting ones are put back at the start of the next frame, before picking reads them. So a press
//! at the widget's edge pixel still lands on it, and the hover that keeps it pressed does not flicker
//! as the drawing shrinks under the pointer.

use bevy::math::Affine2;
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerPressState;
use bevy::prelude::*;
use bevy::ui::{FixedNode, Pressed, ScrollPosition, UiGlobalTransform};

use crate::model::{Spring, SpringState, spring_settled, spring_step};
use crate::motion::{MotionSettings, MotionToken};
use crate::skin::{GooeySkin, Role, SkinScope, skin_of};

/// The scale a card or a slot shrinks to while pressed: iOS's tiles, which Rune follows (0.96 to
/// 0.97).
pub const PRESS_SCALE: f32 = 0.97;

/// The bounce of the spring a pressed widget comes back on: Rune's `POP`
/// (`crates/rune-motion/src/tokens.rs:30`, `Spring::new(0.25, 0.15)`), a slight overshoot that
/// reads as a let-go.
pub const PRESS_RELEASE_BOUNCE: f32 = 0.15;

/// How near its target a press scale must be to count as there: a ten-thousandth of its size, a
/// hundredth of a pixel across a hundred-pixel card.
const PRESS_SCALE_REST: f32 = 1.0e-4;

/// Shrink to this scale about the centre while a primary press on the widget is held and the
/// pointer is over it (a secondary or middle press does not count), on the skin's snappy spring; back to 1 when the pointer leaves while held,
/// and on release, on a spring with a slight bounce ([`PRESS_RELEASE_BOUNCE`]). Layout and the hit
/// test keep the resting size, and everything under the widget scales with it except a popover hung
/// from it (a [`FixedNode`]). A scroll of any scroll view above the widget during the hold (one that
/// moved, not a wheel tick at its end) cancels the press: it springs back and its release
/// activates nothing. Off under reduced or no motion ([`MotionSettings::lands_travel`]). A factor
/// that is not a number above zero is no press scale.
///
/// The crate puts [`PRESS_SCALE`] on every card of a [`crate::card_grid`] and every slot of a
/// [`crate::slot_grid`]; buttons do not get it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // A tile of the game's own that gives under a press.
/// commands.spawn((button("Inventory"), PressScale(0.96)));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[require(PressScaled, Hovered)]
pub struct PressScale(pub f32);

impl Default for PressScale {
    fn default() -> Self {
        Self(PRESS_SCALE)
    }
}

/// How far a [`PressScale`] widget is drawn shrunk now: its scale and how fast it changes, and
/// whether the press it is answering was cancelled by a scroll.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(from_reflect = false)]
#[reflect(Component)]
pub struct PressScaled {
    #[reflect(ignore)]
    scale: SpringState,
    cancelled: bool,
    /// Where every scroll view above the widget stood when the press began, so a scroll that
    /// really moved cancels it and a wheel tick at a list's end does not.
    scrolled_from: Option<Vec<(Entity, Vec2)>>,
}

impl Default for PressScaled {
    fn default() -> Self {
        Self {
            scale: SpringState {
                value: 1.0,
                velocity: 0.0,
            },
            cancelled: false,
            scrolled_from: None,
        }
    }
}

impl PressScaled {
    /// The scale the widget is drawn at now: 1 at rest.
    pub fn scale(&self) -> f32 {
        self.scale.value
    }

    fn at_rest(&self) -> bool {
        self.scale.value == 1.0 && self.scale.velocity == 0.0
    }
}

/// The resting transforms the drawing pass changed, to be put back before the next frame's picking.
#[derive(Resource, Default)]
pub struct PressScaleRestore(Vec<(Entity, UiGlobalTransform)>);

/// A node that just got its role and has no press scale yet.
type NewTile = (Added<Role>, Without<PressScale>);

/// Gives every card and slot (full or empty) [`PressScale`] the frame it gets its role. A game that takes it off
/// one keeps it off. Runs in [`crate::GooeySystems::Style`].
pub fn mark_press_scale(mut commands: Commands, added: Query<(Entity, &Role), NewTile>) {
    for (entity, role) in &added {
        if matches!(role, Role::CardCell | Role::Slot | Role::SlotEmpty) {
            commands.entity(entity).insert(PressScale::default());
        }
    }
}

/// What the press-scale step reads of a widget.
type Pressing = (
    Entity,
    &'static PressScale,
    &'static mut PressScaled,
    Has<Pressed>,
    Option<&'static Hovered>,
);

/// Steps every [`PressScale`] toward the scale the press asks for, on real time, and cancels a
/// press under a scroll (the widget loses [`Pressed`], so its release activates nothing). A widget
/// at rest and not pressed is read and not written. Runs in [`crate::GooeySystems::Style`].
#[allow(clippy::too_many_arguments)]
pub fn update_press_scale(
    settings: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    scrolls: Query<&ScrollPosition>,
    pointers: Query<&PointerPressState>,
    time: Option<Res<Time<Real>>>,
    mut commands: Commands,
    mut widgets: Query<Pressing>,
) {
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let lands = settings.lands_travel() || time.is_none();
    // A secondary or middle press (a context menu's) is no press to give under.
    let primary = pointers.iter().any(PointerPressState::is_primary_pressed);
    for (entity, factor, mut state, pressed, hovered) in &mut widgets {
        if !pressed && state.at_rest() && !state.cancelled && state.scrolled_from.is_none() {
            continue;
        }
        if !pressed {
            if state.cancelled {
                state.cancelled = false;
            }
            if state.scrolled_from.is_some() {
                state.scrolled_from = None;
            }
        }
        if pressed && !state.cancelled {
            let standing = || {
                parents
                    .iter_ancestors(entity)
                    .filter_map(|a| scrolls.get(a).ok().map(|s| (a, **s)))
            };
            // Collected once, at the press; compared in place after that.
            match &state.scrolled_from {
                None => state.scrolled_from = Some(standing().collect()),
                Some(from) if !from.iter().copied().eq(standing()) => {
                    state.cancelled = true;
                    commands.entity(entity).remove::<Pressed>();
                }
                Some(_) => {}
            }
        }
        let usable = factor.0.is_finite() && factor.0 > 0.0;
        let held = pressed && primary && !state.cancelled && hovered.is_some_and(|h| h.0) && usable;
        let target = if held { factor.0 } else { 1.0 };
        if lands {
            if !state.at_rest() {
                state.scale = SpringState {
                    value: 1.0,
                    velocity: 0.0,
                };
            }
            continue;
        }
        let snappy = skin_of(&skin, entity, &scopes, &parents)
            .motion
            .spring(MotionToken::Snappy);
        let spring = if held {
            snappy
        } else {
            Spring {
                duration: snappy.duration,
                bounce: PRESS_RELEASE_BOUNCE,
            }
        }
        .scaled(settings.duration_scale);
        let next = spring_step(spring, state.scale, target, delta);
        let next = if spring_settled(spring, next, target, PRESS_SCALE_REST) {
            SpringState {
                value: target,
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

/// Draws every shrunk [`PressScale`] widget and everything under it at its scale about its centre,
/// remembering the resting transforms for [`restore_press_scale`]. Runs in `PostUpdate`, after
/// Bevy's layout (and layout motion) and before its clipping. A widget at rest is not touched.
pub fn draw_press_scale(
    widgets: Query<(Entity, &PressScaled), With<PressScale>>,
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

/// Draws `entity` and everything under it at `scale` about its centre, remembering each resting
/// transform in `restore`. A popover or a pointer point hung from the widget is laid out on the
/// window, not inside it: it keeps its own place and size.
pub(crate) fn scale_subtree(
    entity: Entity,
    scale: f32,
    children: &Query<&Children>,
    fixed: &Query<(), With<FixedNode>>,
    globals: &mut Query<&mut UiGlobalTransform>,
    restore: &mut PressScaleRestore,
    stack: &mut Vec<Entity>,
) {
    let Ok(centre) = globals.get(entity).map(|at| at.translation) else {
        return;
    };
    stack.clear();
    stack.push(entity);
    while let Some(node) = stack.pop() {
        if node != entity && fixed.contains(node) {
            continue;
        }
        let Ok(mut at) = globals.get_mut(node) else {
            continue;
        };
        restore.0.push((node, *at));
        let rest = Affine2::from(*at);
        *at = Affine2::from_mat2_translation(
            rest.matrix2 * scale,
            centre + (rest.translation - centre) * scale,
        )
        .into();
        if let Ok(kids) = children.get(node) {
            stack.extend(kids.iter());
        }
    }
}

/// Puts back the resting transforms [`draw_press_scale`] changed, so picking hit-tests the
/// widget's resting size. Runs in `First`, before anything reads them.
pub fn restore_press_scale(
    mut restore: ResMut<PressScaleRestore>,
    mut globals: Query<&mut UiGlobalTransform>,
) {
    // Backwards, so a node a nested press scale stored twice gets its first, resting value last.
    for (entity, rest) in restore.0.drain(..).rev() {
        if let Ok(mut at) = globals.get_mut(entity) {
            *at = rest;
        }
    }
}
