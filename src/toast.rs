//! Toasts: a line of news a game spawns in one call. A stack takes it in, ages it by the message's
//! reading time and the player's setting, opens room for it, fades it in and out, and removes it.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::ui::ComputedNode;

use crate::model::{toast_alpha, toast_grow, toast_hold, toast_life};
use crate::skin::Role;

/// Where a toast's stack stands in the window. The newest toast stands nearest the screen's edge,
/// and the older ones are pushed inward as room opens for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ToastAnchor {
    /// Top left corner.
    TopLeft,
    /// Top edge, centred.
    TopCenter,
    /// Top right corner.
    TopRight,
    /// Bottom left corner.
    BottomLeft,
    /// Bottom edge, centred.
    #[default]
    BottomCenter,
    /// Bottom right corner.
    BottomRight,
}

impl ToastAnchor {
    /// The stack's flex layout: down or up from the edge, hugging it, and across.
    pub(crate) fn layout(self) -> (FlexDirection, JustifyContent, AlignItems) {
        use ToastAnchor::*;
        let direction = match self {
            TopLeft | TopCenter | TopRight => FlexDirection::ColumnReverse,
            BottomLeft | BottomCenter | BottomRight => FlexDirection::Column,
        };
        let across = match self {
            TopLeft | BottomLeft => AlignItems::FlexStart,
            TopCenter | BottomCenter => AlignItems::Center,
            TopRight | BottomRight => AlignItems::FlexEnd,
        };
        (direction, JustifyContent::FlexEnd, across)
    }
}

/// How long toasts stand, how many, how they fade and where: the timing a player may change.
///
/// A toast stands `(base_seconds + characters ÷ chars_per_second) × duration_scale` fully shown
/// ([`crate::toast_hold`]), after fading in and before fading out over `fade_seconds` each. Offer
/// `duration_scale` in your settings screen: Xbox Accessibility Guideline 116 asks that a player can
/// stretch the time notifications stay up to at least ten times the default, and any value above
/// zero works. `fade_seconds` of zero removes the fade and the opening of room, for a player who
/// wants no motion: a toast then appears and goes.
///
/// With the `serde` feature the settings are (de)serialisable, and a partial file fills every
/// missing field from the default.
///
/// ```
/// use bevy_gooey::ToastSettings;
/// // A player's "notification time: long" option.
/// let long = ToastSettings { duration_scale: 3.0, ..ToastSettings::default() };
/// assert_eq!(long.max_visible, 3);
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct ToastSettings {
    /// Where the stack stands. Default: bottom centre.
    pub anchor: ToastAnchor,
    /// How many toasts stand at once, at least one: a spawn beyond it sends the oldest into its fade
    /// out at once. Default: 3.
    pub max_visible: usize,
    /// Seconds to notice a toast and look at it, before any reading. Default: 2.
    pub base_seconds: f32,
    /// The reading speed a message's length is timed at, characters a second. Default: 20, the most
    /// Netflix lets timed text ask of an adult.
    pub chars_per_second: f32,
    /// The player's multiplier on how long a toast stands. Default: 1.
    pub duration_scale: f32,
    /// Seconds a toast takes to fade in, rising into place, and to fade out. Default: 0.3.
    pub fade_seconds: f32,
}

impl Default for ToastSettings {
    fn default() -> Self {
        Self {
            anchor: ToastAnchor::default(),
            max_visible: 3,
            base_seconds: 2.0,
            chars_per_second: 20.0,
            duration_scale: 1.0,
            fade_seconds: 0.3,
        }
    }
}

/// What a toast tells the player; the mark down its left edge shows it (the words carry it too).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, PartialEq, Clone)]
pub enum ToastKind {
    /// Information, news: the mark is [`crate::GooeySkin::accent`].
    #[default]
    Info,
    /// A refusal or a failure: the mark is [`crate::GooeySkin::danger`].
    Alert,
}

/// A line of news at the edge of the screen, from [`crate::toast`] or [`crate::toast_alert`]. It
/// stands for as long as its message takes to read ([`ToastSettings`]), fades out, and is
/// despawned; despawn it yourself to take it back sooner.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn save(mut commands: Commands) {
///     commands.spawn(toast("Game saved"));
/// }
/// ```
///
/// Spawning one makes it a child of the one [`ToastStack`], which the first toast creates, in the
/// frame it is spawned: a game never lays one out, and spawns from any system. A toast takes no
/// input: every node of it is [`Pickable::IGNORE`], it is no tab stop and focus never goes to it,
/// so a press, a key or a D-pad direction reaches what is under it. It is drawn above a [`crate::dialog`]'s
/// scrim and below a tooltip. Its time runs on `Time<Real>`, so a paused game's menu still ages its
/// toasts.
///
/// This component is its clock and its look: read it, do not write it. It is not announced to
/// screen readers.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Debug, Clone)]
#[component(on_add = queue_join)]
pub struct Toast {
    kind: ToastKind,
    /// The message's length in characters, which its reading time is made of.
    characters: usize,
    /// Seconds since it was spawned, on real time.
    age: f32,
    /// The longest it may stand fully shown: infinite until the stack is over `max_visible`.
    hold_cap: f32,
    alpha: f32,
    /// How much of its room is open, from 0 to 1: [`toast_grow`] of its age.
    grow: f32,
    /// How tall its body stands in full, in logical px, measured while its room opens or closes.
    body_px: f32,
}

impl Toast {
    /// A toast of `kind` for `text`, hidden until [`update_toasts`] first runs.
    pub(crate) fn new(kind: ToastKind, text: &str) -> Self {
        Self {
            kind,
            characters: text.chars().count(),
            age: 0.0,
            hold_cap: f32::INFINITY,
            alpha: 0.0,
            grow: 0.0,
            body_px: 0.0,
        }
    }

    /// What it tells the player.
    pub fn kind(&self) -> ToastKind {
        self.kind
    }

    /// Seconds since it was spawned, on real time.
    pub fn age(&self) -> f32 {
        self.age
    }

    /// How opaque it is now, from 0 to 1: [`crate::toast_alpha`] of its age.
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// How much of its room is open, from 0 to 1: [`crate::toast_grow`] of its age. The toasts
    /// around it make way as it opens and close up as it goes, so none of them jumps.
    pub fn grow(&self) -> f32 {
        self.grow
    }

    /// How tall its body stands in full, in logical px, as last measured while its room was
    /// opening or closing; zero until it has been laid out once.
    pub fn body_px(&self) -> f32 {
        self.body_px
    }
}

/// The one column every [`Toast`] stands in: a full-window, pointer-transparent node the first
/// toast spawns, aligned by [`ToastSettings::anchor`] and kept above a dialog (`GlobalZIndex` 150).
/// Query it to hide the toasts (a cutscene) by setting its `Visibility`.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ToastStack;

/// Above a [`crate::dialog`]'s scrim (100), so a message about what its button did is readable over
/// it, and below a tooltip (200).
const STACK_Z: i32 = 150;

/// A toast was just added: make it a child of the stack in this flush, creating the stack if none
/// stands, before any frame can draw it unparented.
fn queue_join(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world.commands().queue(move |world: &mut World| {
        if world.get_entity(entity).is_err() {
            return;
        }
        let existing = world
            .query_filtered::<Entity, With<ToastStack>>()
            .iter(world)
            .next();
        let stack = existing.unwrap_or_else(|| {
            world
                .spawn((
                    Name::new("ToastStack"),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    Role::ToastStack,
                    ToastStack,
                    Pickable::IGNORE,
                    GlobalZIndex(STACK_Z),
                ))
                .id()
        });
        world.entity_mut(entity).insert(ChildOf(stack));
    });
}

/// Ages every [`Toast`], fades it in and out, sends the oldest into its fade when more than
/// [`ToastSettings::max_visible`] stand, despawns the spent, and lays the stack out for
/// [`ToastSettings::anchor`]. Runs in [`crate::GooeySystems::Style`], before [`crate::apply_skin`],
/// which draws the result.
///
/// A toast is written only when its opacity or room changes, so one that is holding costs a clock
/// tick and nothing else, and nothing restyles it.
pub fn update_toasts(
    time: Res<Time<Real>>,
    settings: Res<ToastSettings>,
    mut commands: Commands,
    mut stacks: Query<(&mut Node, &Children), With<ToastStack>>,
    mut toasts: Query<(Entity, &mut Toast)>,
    kids: Query<&Children, With<Toast>>,
    bodies: Query<&ComputedNode, Without<Toast>>,
) {
    let delta = time.delta_secs();
    let usable = |x: f32| x.is_finite() && x > 0.0;
    let fade = if usable(settings.fade_seconds) {
        settings.fade_seconds
    } else {
        0.0
    };
    let hold_of = |toast: &Toast| {
        toast_hold(
            toast.characters,
            settings.base_seconds,
            settings.chars_per_second,
            settings.duration_scale,
        )
        .min(toast.hold_cap)
    };
    let (direction, justify, across) = settings.anchor.layout();
    let max = settings.max_visible.max(1);

    for (mut node, children) in &mut stacks {
        if node.flex_direction != direction {
            node.flex_direction = direction;
        }
        if node.justify_content != justify {
            node.justify_content = justify;
        }
        if node.align_items != across {
            node.align_items = across;
        }

        // The clock, and who is still fully shown (not yet fading out). A node under the stack
        // that is not a toast is skipped.
        let mut standing = 0usize;
        let mut live = toasts.iter_many_mut(children.iter());
        while let Some(item) = live.fetch_next() {
            let Ok((entity, mut toast)) = item else {
                continue;
            };
            // The clock is not a change worth a restyle: only the look below is.
            toast.bypass_change_detection().age += delta;
            let hold = hold_of(&toast);
            if toast.age >= toast_life(hold, fade) {
                commands.entity(entity).despawn();
            } else if toast.age < fade + hold {
                standing += 1;
            }
        }

        // Too many: the oldest begin to fade now, whole as they are.
        let mut excess = standing.saturating_sub(max);
        let mut live = toasts.iter_many_mut(children.iter());
        while let Some(item) = live.fetch_next() {
            if excess == 0 {
                break;
            }
            let Ok((_, mut toast)) = item else {
                continue;
            };
            if toast.age < fade + hold_of(&toast) {
                let cut = (toast.age - fade).max(0.0);
                toast.bypass_change_detection().hold_cap = cut;
                excess -= 1;
            }
        }

        // How open its room is. While it opens or closes, its body is measured: laid out whole in
        // a toast whose height is held down, so the room it opens is the body's height.
        let mut live = toasts.iter_many_mut(children.iter());
        while let Some(item) = live.fetch_next() {
            let Ok((entity, mut toast)) = item else {
                continue;
            };
            let hold = hold_of(&toast);
            let alpha = toast_alpha(toast.age, hold, fade);
            let grow = toast_grow(toast.age, hold, fade);
            let body_px = if grow < 1.0 {
                kids.get(entity)
                    .ok()
                    .and_then(|kids| kids.first())
                    .and_then(|&body| bodies.get(body).ok())
                    .map_or(toast.body_px, |body| {
                        body.size().y * body.inverse_scale_factor()
                    })
            } else {
                toast.body_px
            };
            if toast.alpha != alpha || toast.grow != grow || toast.body_px != body_px {
                toast.alpha = alpha;
                toast.grow = grow;
                toast.body_px = body_px;
            }
        }
    }
}
