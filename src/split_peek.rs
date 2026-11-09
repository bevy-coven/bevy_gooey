//! The split's peek: a collapsed first pane floats over the second one when the pointer rests at
//! its edge, and docks where it floats when the player asks to open it (#69,
//! `docs/research/split-peek.md`).
//!
//! While it peeks the pane is absolute at the split's start edge, as wide as it will be when it is
//! restored, above the second pane; a margin on the divider keeps the room the collapsed pane had
//! and the second pane takes the rest alone, so **the second pane is laid out exactly as before**.
//! [`crate::update_splits`] writes that layout from the [`SplitPeek`] this module keeps.

use bevy::input::ButtonInput;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BoxShadow, ComputedNode, InteractionDisabled, UiTransform};

use crate::elevation::ElevationLevel;
use crate::model::{split_clamp, split_mins};
use crate::motion::{MotionSettings, MotionTarget, MotionToken, MotionTrack};
use crate::presence::{Arriving, Opacity};
use crate::size_motion::SplitGlide;
use crate::skin::{GooeySkin, Role};
use crate::split::{Split, SplitAxis, SplitFraction, SplitMin, SplitSaved};

/// How the peek of a collapsed [`crate::split`] behaves.
///
/// The defaults are Rune's `SidebarPeek` ones: 50 ms of the pointer resting on the collapsed edge
/// before the pane floats, and 150 ms of grace after the pointer leaves it before it goes.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct SplitPeekSettings {
    /// Whether a collapsed pane peeks at all. Default: true.
    pub enabled: bool,
    /// How long the pointer rests on the collapsed edge before the pane floats, seconds of real
    /// time. Default: 0.05.
    pub delay_seconds: f32,
    /// How long the pointer may be off the pane and its edge before the peek closes, seconds.
    /// Default: 0.15.
    pub grace_seconds: f32,
}

impl Default for SplitPeekSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            delay_seconds: 0.05,
            grace_seconds: 0.15,
        }
    }
}

/// Where a split's peek stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Default, Debug, PartialEq, Clone)]
enum Phase {
    /// Nothing: the pane is collapsed (or open) and in the flow.
    #[default]
    Idle,
    /// The pointer is on the collapsed edge and has not rested long enough.
    Waiting,
    /// The pane floats over the second pane.
    Open,
    /// The pane is sliding back to its edge.
    Closing,
}

/// A [`crate::split`]'s peek: whether its collapsed first pane floats over the second one now.
/// Part of what `split()` spawns (a [`Split`] requires it); a game reads [`SplitPeek::is_open`].
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct SplitPeek {
    phase: Phase,
    /// Seconds in the phase that counts: waiting for the delay, or off the pane for the grace.
    seconds: f32,
    /// The floating pane's size along the split's axis, logical px.
    width: f32,
    /// The room the collapsed pane had along the axis, logical px, kept by the second pane's margin.
    collapsed: f32,
    /// Escape closed the peek and the pointer has not left since: it does not open again until it
    /// has.
    dismissed: bool,
    /// The split was collapsed when last looked at, so a collapse is seen once. It is also exactly when the first pane
    /// carries the [`Hovered`] the peek reads: put there on the collapse, taken off when the split is not collapsed, so an
    /// open split's panes are not marked as interface (see `split_pane`).
    armed: bool,
}

impl SplitPeek {
    /// Whether the first pane floats over the second one: drawn there, or sliding back.
    pub fn is_open(&self) -> bool {
        matches!(self.phase, Phase::Open | Phase::Closing)
    }

    /// The floating pane's size along the axis, logical px; 0 while it does not float.
    pub fn width(&self) -> f32 {
        if self.is_open() { self.width } else { 0.0 }
    }

    /// The room the collapsed pane had along the axis, which the second pane's margin keeps.
    pub(crate) fn collapsed(&self) -> f32 {
        if self.is_open() { self.collapsed } else { 0.0 }
    }
}

/// Everything the peek reads of a split.
type Peeked = (
    &'static Split,
    &'static SplitFraction,
    &'static SplitMin,
    &'static ComputedNode,
    &'static Children,
    Option<&'static SplitSaved>,
    Option<&'static SplitGlide>,
    &'static mut SplitPeek,
);

/// What the peek reads of a split's parts.
type PartRead = (
    &'static Role,
    &'static ComputedNode,
    Option<&'static Hovered>,
    Has<InteractionDisabled>,
    &'static UiTransform,
);

/// A split's first pane: its first child that is a [`Role::SplitPane`].
fn first_pane(children: &Children, parts: &Query<PartRead>) -> Option<Entity> {
    children.iter().find(|child| {
        parts
            .get(*child)
            .is_ok_and(|(role, ..)| matches!(role, Role::SplitPane))
    })
}

/// Opens, holds and closes each collapsed split's peek. Runs in [`crate::GooeySystems::Style`],
/// before [`crate::update_motion`] (the slide in is a [`MotionTarget`] it gives the pane) and
/// [`crate::update_splits`] (which lays the floating pane out from the [`SplitPeek`] it keeps).
///
/// A split peeks only while it is collapsed (it holds a `SplitSaved`, sits at its smallest and has
/// no glide on its way) and its divider is not disabled. The pointer resting on the divider or the
/// collapsed pane for [`SplitPeekSettings::delay_seconds`] opens it, and so does a visible focus
/// inside the pane (Tab or an arrow into it); the pointer off the floating
/// pane and the divider for [`SplitPeekSettings::grace_seconds`], or Escape, closes it. A restore
/// (the split is no longer collapsed) ends it in that frame. A split that is not collapsed costs a
/// comparison.
#[allow(clippy::too_many_arguments)]
pub fn update_split_peeks(
    settings: Option<Res<SplitPeekSettings>>,
    motion: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    time: Option<Res<Time<Real>>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    focus: Option<Res<InputFocus>>,
    focus_visible: Option<Res<InputFocusVisible>>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
    mut splits: Query<Peeked>,
    parts: Query<PartRead>,
    mut floating: Query<(&mut BoxShadow, &mut BackgroundColor, Option<&Opacity>)>,
) {
    let settings = settings.map_or_else(SplitPeekSettings::default, |s| *s);
    let delta = time.as_ref().map_or(f32::INFINITY, |t| t.delta_secs());
    let escaped = keys.is_some_and(|k| k.just_pressed(KeyCode::Escape));
    for (split, fraction, min, node, children, saved, glide, mut peek) in &mut splits {
        // Not collapsed, and not peeking: nothing to look at.
        if peek.phase == Phase::Idle && (saved.is_none() || glide.is_some()) {
            if peek.armed {
                peek.armed = false;
                if let Some(pane) = first_pane(children, &parts) {
                    commands.entity(pane).try_remove::<Hovered>();
                }
            }
            continue;
        }
        let mut pane = None;
        let mut divider = None;
        for child in children.iter() {
            let Ok((role, computed, hovered, disabled, transform)) = parts.get(child) else {
                continue;
            };
            match role {
                Role::SplitPane if pane.is_none() => {
                    pane = Some((child, computed, hovered, transform))
                }
                Role::SplitDivider => divider = Some((child, computed, hovered, disabled)),
                _ => {}
            }
        }
        let (
            Some((pane, pane_node, pane_hover, pane_transform)),
            Some((divider, bar, bar_hover, disabled)),
        ) = (pane, divider)
        else {
            continue;
        };
        let along = |size: Vec2| match split.axis() {
            SplitAxis::Horizontal => size.x,
            SplitAxis::Vertical => size.y,
        };
        let logical = |node: &ComputedNode| along(node.size()) * node.inverse_scale_factor();
        let extent = (logical(node) - logical(bar)).max(0.0);
        let smallest = split_clamp(0.0, extent, min.first, min.second);
        let collapsed = settings.enabled
            && !disabled
            && extent > 0.0
            && glide.is_none()
            && saved.is_some()
            && (fraction.0 - smallest) * extent < 0.5;
        // A focus ring inside the collapsed pane is as good as the pointer on it: a keyboard or pad
        // player who tabs into the pane sees it come out (the divider's own focus is not, or a
        // pane collapsed with Enter on it would come straight back).
        let focused_in = focus_visible.as_ref().is_some_and(|v| v.0)
            && focus
                .as_ref()
                .and_then(|f| f.get())
                .is_some_and(|f| parents.iter_ancestors(f).any(|a| a == pane));
        let touching = bar_hover.is_some_and(Hovered::get)
            || pane_hover.is_some_and(Hovered::get)
            || focused_in;
        // The pointer already on the edge when the pane gets collapsed (the double press that did
        // it) has not come to it: the peek waits for it to leave and return.
        if collapsed && !peek.armed {
            peek.armed = true;
            // The pane is marked as hoverable for as long as it is collapsed, and not before: a mark on an open
            // pane would make a host that asks "is any `Hovered` node under the pointer" take it for interface.
            commands.entity(pane).try_insert(Hovered::default());
            if touching {
                peek.dismissed = true;
            }
        } else if !collapsed && peek.armed {
            peek.armed = false;
            commands.entity(pane).try_remove::<Hovered>();
        }
        // A dismissal ends once the pointer is off the edge for good (not while the pane slides
        // away under it).
        if !touching && peek.dismissed && peek.phase == Phase::Idle {
            peek.dismissed = false;
        }
        let over = touching && !peek.dismissed;
        let phase = peek.phase;
        let next = if !collapsed && phase == Phase::Idle {
            Peek::keep()
        } else if !collapsed {
            // Restored, dragged, set by the game, or switched off: the pane is in the flow again.
            Peek::end()
        } else {
            match phase {
                Phase::Idle if over => Peek::waiting(),
                Phase::Idle => Peek::keep(),
                Phase::Waiting if !over => Peek::end(),
                Phase::Waiting if peek.seconds + delta >= settings.delay_seconds => {
                    let restore = saved.map_or(0.5, |s| s.0);
                    let (first, _) = split_mins(extent, min.first, min.second);
                    let width = (split_clamp(restore, extent, min.first, min.second) * extent)
                        .max(first + 1.0);
                    Peek::open(width, logical(pane_node))
                }
                Phase::Waiting => Peek::seconds(peek.seconds + delta),
                Phase::Open if escaped => Peek::closing(),
                Phase::Open if over => Peek::seconds(0.0),
                Phase::Open if peek.seconds + delta >= settings.grace_seconds => Peek::closing(),
                Phase::Open => Peek::seconds(peek.seconds + delta),
                Phase::Closing if over && !escaped => Peek::reopen(),
                Phase::Closing => {
                    let drawn = match (split.axis(), pane_transform.translation) {
                        (SplitAxis::Horizontal, t) => t.x,
                        (SplitAxis::Vertical, t) => t.y,
                    };
                    let gone = matches!(drawn, Val::Px(px) if px <= -peek.width + 0.5)
                        || reduced_or_off(&motion);
                    if gone { Peek::end() } else { Peek::keep() }
                }
            }
        };
        match next {
            Peek::Keep => {}
            Peek::Seconds(seconds) => {
                if peek.seconds != seconds {
                    peek.seconds = seconds;
                }
            }
            Peek::Phase(phase_next, seconds) => {
                let ending = peek.is_open() && !matches!(phase_next, Phase::Open | Phase::Closing);
                peek.phase = phase_next;
                peek.seconds = seconds;
                if ending {
                    release(&mut commands, pane, divider);
                }
                if phase_next == Phase::Closing {
                    // Escape is read once: the peek stays closed until the pointer has left.
                    if escaped && touching {
                        peek.dismissed = true;
                    }
                    close(&mut commands, pane, split.axis(), peek.width, &motion);
                }
                if phase_next == Phase::Open && phase == Phase::Closing {
                    // The pointer came back: the slide turns round from where it is.
                    open(
                        &mut commands,
                        (pane, divider),
                        split.axis(),
                        peek.width,
                        &motion,
                        false,
                    );
                }
            }
            Peek::Open { width, collapsed } => {
                peek.phase = Phase::Open;
                peek.seconds = 0.0;
                peek.width = width;
                peek.collapsed = collapsed;
                open(
                    &mut commands,
                    (pane, divider),
                    split.axis(),
                    width,
                    &motion,
                    true,
                );
            }
        }
        // The floating pane's shadow rises with the slide (or the fade) that brings it in.
        if peek.is_open()
            && let Ok((mut shadow, mut fill, faded)) = floating.get_mut(pane)
        {
            // Opaque, so the second pane's words do not show through a floating one; faded in with
            // the pane under reduced motion.
            let solid = Srgba::from(skin.solid_surface());
            let own = faded.map_or(1.0, Opacity::factor);
            fill.set_if_neq(BackgroundColor(Color::from(
                solid.with_alpha(solid.alpha * own),
            )));
            let progress = match (split.axis(), pane_transform.translation) {
                (SplitAxis::Horizontal, t) => px(t.x),
                (SplitAxis::Vertical, t) => px(t.y),
            };
            let alpha = if peek.width > 0.0 {
                (1.0 + progress / peek.width).clamp(0.0, 1.0)
            } else {
                1.0
            };
            shadow.set_if_neq(skin.elevation.box_shadow(ElevationLevel::Floating, alpha));
        }
    }
}

/// A logical px of a `Val`, 0 for any other.
fn px(val: Val) -> f32 {
    match val {
        Val::Px(px) => px,
        _ => 0.0,
    }
}

/// Whether the pane has nowhere to slide: reduced or no motion lands it where it is going.
fn reduced_or_off(motion: &MotionSettings) -> bool {
    motion.lands_travel()
}

/// What a frame of the peek decides.
enum Peek {
    /// Nothing changes.
    Keep,
    /// Only the clock changes.
    Seconds(f32),
    /// The phase changes, with its clock.
    Phase(Phase, f32),
    /// The pane starts to float, `width` along the axis, in the room of `collapsed`.
    Open { width: f32, collapsed: f32 },
}

impl Peek {
    fn keep() -> Self {
        Peek::Keep
    }
    fn seconds(seconds: f32) -> Self {
        Peek::Seconds(seconds)
    }
    fn waiting() -> Self {
        Peek::Phase(Phase::Waiting, 0.0)
    }
    fn end() -> Self {
        Peek::Phase(Phase::Idle, 0.0)
    }
    fn closing() -> Self {
        Peek::Phase(Phase::Closing, 0.0)
    }
    fn reopen() -> Self {
        Peek::Phase(Phase::Open, 0.0)
    }
    fn open(width: f32, collapsed: f32) -> Self {
        Peek::Open { width, collapsed }
    }
}

/// Starts (or turns round) the pane's slide in: from `width` past its edge on the snappy spring,
/// or a fade in place under reduced motion; nothing under no motion.
fn open(
    commands: &mut Commands,
    (pane, divider): (Entity, Entity),
    axis: SplitAxis,
    width: f32,
    motion: &MotionSettings,
    fresh: bool,
) {
    let away = match axis {
        SplitAxis::Horizontal => Vec2::new(-width, 0.0),
        SplitAxis::Vertical => Vec2::new(0.0, -width),
    };
    // The divider stands on the floating pane's far edge, above it, where a press can reach it.
    if fresh {
        commands.entity(divider).insert(ZIndex(2));
    }
    let mut pane = commands.entity(pane);
    if fresh {
        pane.insert((
            ZIndex(1),
            BoxShadow::default(),
            BackgroundColor(Color::NONE),
        ));
    }
    if motion.reduced && !motion.is_off() {
        if fresh {
            pane.insert(Arriving::begin());
        }
    } else if fresh {
        pane.insert((
            MotionTarget::default().entering_from(away),
            UiTransform::from_translation(Val2::px(away.x, away.y)),
        ));
    } else {
        pane.insert(MotionTarget {
            spring: MotionToken::Snappy,
            ..default()
        });
    }
}

/// Starts the pane's slide back to its edge.
fn close(
    commands: &mut Commands,
    pane: Entity,
    axis: SplitAxis,
    width: f32,
    motion: &MotionSettings,
) {
    if motion.lands_travel() {
        return;
    }
    let away = match axis {
        SplitAxis::Horizontal => Vec2::new(-width, 0.0),
        SplitAxis::Vertical => Vec2::new(0.0, -width),
    };
    commands.entity(pane).insert(MotionTarget {
        translation: away,
        spring: MotionToken::Snappy,
        ..default()
    });
}

/// Puts the pane back in the flow: everything the peek gave it comes off.
fn release(commands: &mut Commands, pane: Entity, divider: Entity) {
    commands.entity(divider).remove::<ZIndex>();
    commands
        .entity(pane)
        .remove::<(
            MotionTarget,
            MotionTrack,
            ZIndex,
            BoxShadow,
            BackgroundColor,
            Arriving,
        )>()
        .insert(UiTransform::default());
}
