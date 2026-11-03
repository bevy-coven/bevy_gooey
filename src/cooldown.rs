//! `Cooldown`: a timer on any widget, drawn as the radial sweep every action game's hotbar has.
//!
//! The component is the state and its `on_add` and `on_remove` hooks keep one cover child exactly
//! while it exists (the `Tooltip` box's pattern); [`update_cooldowns`] counts it down on game time,
//! draws the sweep and the seconds, and sends [`CooldownReady`] when it ends. Research:
//! `docs/research/cooldown.md`.

use std::f32::consts::TAU;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::ui::{AngularColorStop, BackgroundGradient, ConicGradient, Gradient, UiPosition};

use crate::model::{cooldown_sweep, cooldown_text};
use crate::skin::{GooeySkin, Role, SkinScope, skin_of};

/// A timer on a widget: until it is spent the widget carries a dimmed cover whose clear wedge opens
/// clockwise from twelve o'clock, with the seconds left on a chip in the middle. Put it on any
/// node that is a positioning context (a [`button`](crate::button), an
/// [`icon_button`](crate::icon_button), a slot of a [`slot_grid`](crate::slot_grid)); it is ready,
/// and draws nothing, until [`Cooldown::start`].
///
/// The crate counts it down on game time (`Time`, so a paused game holds it and a slowed one
/// slows it) and sends one [`CooldownReady`] on the widget the frame it reaches zero: that is where
/// a game plays its sound. A game that keeps its own clock writes [`Cooldown::remaining`] itself.
/// The cover is the dimmed [`GooeySkin::surface`] and the chip is read at 4.5:1 in every preset
/// ([`GooeySkin::problems`] checks it), and the widget's accessible name carries the seconds
/// ("Fireball, 6 seconds remaining"). The crate does not stop the widget being pressed: a game that
/// refuses a press on a cooldown asks [`Cooldown::is_ready`].
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn hotbar(mut commands: Commands) {
///     commands.spawn((
///         button("Fireball"),
///         Cooldown::default(),
///         observe(|activate: On<Activate>, mut cooldowns: Query<&mut Cooldown>| {
///             if let Ok(mut cooldown) = cooldowns.get_mut(activate.entity)
///                 && cooldown.is_ready()
///             {
///                 cooldown.start(8.0); // cast it
///             }
///         }),
///         observe(|ready: On<CooldownReady>| info!("{} is ready", ready.entity)),
///     ));
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[component(on_insert = queue_cover_sync, on_remove = queue_cover_sync)]
pub struct Cooldown {
    /// The whole time of the cooldown running, seconds: what the sweep is a share of.
    pub duration: f32,
    /// The time left, seconds. Zero, below it or not finite is ready.
    pub remaining: f32,
}

impl Cooldown {
    /// A cooldown of `seconds`, already running.
    pub fn new(seconds: f32) -> Self {
        Self {
            duration: seconds,
            remaining: seconds,
        }
    }

    /// Starts (or restarts) the cooldown for `seconds`.
    pub fn start(&mut self, seconds: f32) {
        *self = Self::new(seconds);
    }

    /// No time is left: the widget is available.
    pub fn is_ready(&self) -> bool {
        !(self.remaining.is_finite() && self.remaining > 0.0)
    }

    /// The share of the cooldown still to run, `0.0` (ready) to `1.0` (just started).
    pub fn fraction(&self) -> f32 {
        if self.is_ready() || !self.duration.is_finite() || self.duration <= 0.0 {
            return if self.is_ready() { 0.0 } else { 1.0 };
        }
        (self.remaining / self.duration).clamp(0.0, 1.0)
    }
}

/// Sent on a widget the frame its [`Cooldown`] reaches zero (not when a game sets `remaining` to
/// zero itself): the place to play the "ready" sound, XAG's second channel for the same news.
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq, Reflect)]
#[reflect(Event, Clone, Debug)]
pub struct CooldownReady {
    /// The widget whose cooldown ended.
    #[event_target]
    pub entity: Entity,
}

/// Both hooks queue [`sync_cover`] rather than act on what they saw, as `Tooltip` does: a flush
/// that removes a `Cooldown` and inserts another fires two hooks and the command reads the entity
/// when it runs.
fn queue_cover_sync(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world
        .commands()
        .queue(move |world: &mut World| sync_cover(world, entity));
}

/// Makes the cover exist exactly while the entity has a [`Cooldown`]: one when there is none, none
/// when the component is gone, nothing when the entity itself is gone. Idempotent.
fn sync_cover(world: &mut World, entity: Entity) {
    let Ok(owner) = world.get_entity(entity) else {
        return;
    };
    let has = owner.contains::<Cooldown>();
    let covers: Vec<Entity> = owner
        .get::<Children>()
        .into_iter()
        .flat_map(|children| children.iter())
        .filter(|child| world.get::<Role>(*child) == Some(&Role::Cooldown))
        .collect();
    match (has, covers.is_empty()) {
        (true, true) => {
            world.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Role::Cooldown,
                BackgroundGradient::default(),
                // Hidden until the first running frame: a ready widget draws nothing.
                Visibility::Hidden,
                // Over a slot's count, under nothing of the widget's.
                ZIndex(2),
                Pickable::IGNORE,
                ChildOf(entity),
                children![(
                    Text::new(""),
                    TextLayout::new(Justify::Center, LineBreak::NoWrap),
                    Role::CooldownText,
                    Pickable::IGNORE,
                )],
            ));
        }
        (false, false) => {
            for cover in covers {
                world.entity_mut(cover).despawn();
            }
        }
        _ => {}
    }
}

/// The sweep as a gradient: clear up to `angle` clockwise from twelve o'clock, the skin's dim from
/// there round to twelve o'clock again (hard stops).
fn sweep_gradient(dim: Color, angle: f32) -> Gradient {
    let clear = dim.with_alpha(0.0);
    Gradient::Conic(ConicGradient::new(
        UiPosition::CENTER,
        vec![
            AngularColorStop::new(clear, 0.0),
            AngularColorStop::new(clear, angle),
            AngularColorStop::new(dim, angle),
            AngularColorStop::new(dim, TAU),
        ],
    ))
}

/// Counts every running [`Cooldown`] down on game time, sends [`CooldownReady`] for each that
/// reaches zero, and keeps its cover true: hidden while ready, otherwise the sweep and the seconds.
/// Writes a node only on a difference, and walks the children of a cooldown only while it runs or
/// changed, so a hotbar of ready widgets costs a query. Runs in [`crate::GooeySystems::Style`].
#[allow(clippy::too_many_arguments)]
pub fn update_cooldowns(
    time: Option<Res<Time>>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    mut cooldowns: Query<(Entity, Mut<Cooldown>, &Children)>,
    roles: Query<&Role>,
    child_lists: Query<&Children>,
    mut covers: Query<(&mut BackgroundGradient, &mut Visibility)>,
    mut texts: Query<&mut Text>,
    mut commands: Commands,
) {
    let delta = time.map_or(0.0, |time| time.delta_secs());
    for (entity, mut cooldown, children) in &mut cooldowns {
        if cooldown.is_ready() && !cooldown.is_changed() {
            continue;
        }
        if !cooldown.is_ready() && delta > 0.0 {
            cooldown.remaining -= delta;
            if cooldown.remaining <= 0.0 {
                cooldown.remaining = 0.0;
                commands.trigger(CooldownReady { entity });
            }
        }
        let Some(cover) = children
            .iter()
            .find(|child| roles.get(*child) == Ok(&Role::Cooldown))
        else {
            continue;
        };
        let Ok((mut gradient, mut visibility)) = covers.get_mut(cover) else {
            continue;
        };
        let running = !cooldown.is_ready();
        visibility.set_if_neq(if running {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if running {
            let dim = skin_of(&skin, entity, &scopes, &parents).cooldown_dim();
            let wanted = [sweep_gradient(
                dim,
                cooldown_sweep(cooldown.remaining, cooldown.duration),
            )];
            if gradient.0.as_slice() != wanted {
                gradient.0 = wanted.into();
            }
        }
        let words = cooldown_text(cooldown.remaining);
        if let Ok(parts) = child_lists.get(cover) {
            for part in parts {
                if let Ok(mut text) = texts.get_mut(*part)
                    && text.0 != words
                {
                    text.0 = words.clone();
                }
            }
        }
    }
}
