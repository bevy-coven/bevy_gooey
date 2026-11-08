//! Double press and click counts: the one place the crate decides that two presses are a pair.
//!
//! Bevy's `PointerClick::count` counts every press on an entity within
//! `PickingSettings::multi_click_interval`, wherever the pointer was and however long it was held,
//! so a third quick click is a "second double click" and a click 30 px from the first still pairs.
//! A desktop's double click is narrower, and this module is that rule (measured from Rune's runtime,
//! `crates/rune/src/runtime/input/pointer.rs` and `press_clicks.rs` at `f73f72c`, MIT; research:
//! `docs/research/double-press.md`):
//!
//! - a second primary press on the same entity within [`PointerSettings::double_click_seconds`] of
//!   the first press and within [`PointerSettings::double_click_slop_px`] of where it was, let go
//!   on that entity, is one [`DoublePress`];
//! - the first click is still a click, at once: nothing waits to see whether a second comes;
//! - only clicks pair: a press that travels past the slop or is held past the interval is neither
//!   half of a pair;
//! - any press breaks a pending pair unless it is that pair's second, so a third quick click starts
//!   a new pair;
//! - keys never pair (this module reads only the pointer).
//!
//! [`ClickCount`] is the run a drag reads (1, 2, 3 …: a triple-click drag selects by line): one more
//! than the last press when that press was on the same entity, within the interval and the slop of
//! it, and did not travel; otherwise 1.
//!
//! One pointer presses one entity at a time here: a press that the picking backends report on
//! several entities at once (a stack of non-blocking pickables) pairs on the last of them. The
//! press is let go when the pointer lets go over that entity, or when its drag ends (Bevy sends no
//! release to an entity the pointer left).

use bevy::picking::events::{
    PointerCancel, PointerClick, PointerDrag, PointerPress, PointerRelease,
};
use bevy::picking::pointer::{PointerButton, PointerId};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// How close two presses must be, in time and in place, to pair into a [`DoublePress`]. A resource
/// `GooeyNavPlugin` inits; a host may set it from the system's own double-click time.
///
/// ```
/// use bevy_gooey::PointerSettings;
/// let settings = PointerSettings::default();
/// assert_eq!(settings.double_click_seconds, 0.5);
/// assert_eq!(settings.double_click_slop_px, 4.0);
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct PointerSettings {
    /// Two presses closer than this, seconds from press to press, can pair; a press held longer
    /// than this is not a click. Default 0.5, the double-click time of desktops that have no system
    /// setting. A value that is not a positive finite number pairs nothing.
    pub double_click_seconds: f32,
    /// How far, logical pixels on each axis, the second press may be from the first, and how far a
    /// press may travel while held and still be a click. Default 4.
    pub double_click_slop_px: f32,
}

impl Default for PointerSettings {
    fn default() -> Self {
        Self {
            double_click_seconds: 0.5,
            double_click_slop_px: 4.0,
        }
    }
}

/// The player pressed an entity twice in quick succession with the primary pointer button
/// ([`PointerSettings`]). Sent in the frame of the second click, after that click's
/// `PointerClick`; the first click got its own `PointerClick` at once. Bubbles up `ChildOf` from the
/// entity the pointer pressed, like the click; an observer that acts on it calls
/// `propagate(false)`.
#[derive(EntityEvent, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[entity_event(propagate, auto_propagate)]
#[reflect(Debug, PartialEq, Clone)]
pub struct DoublePress {
    /// The entity the observer sees it on; `On::original_event_target()` is the entity pressed.
    pub entity: Entity,
}

/// Which press of a run the held press is: 1, then 2, 3 … for each quick press on the same entity
/// in the same place ([`PointerSettings`]). On the pressed entity from its press until the pointer
/// lets go (or the next press, or a cancel), so a drag observer reads it: a triple-click drag is a
/// drag with a count of 3.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct ClickCount(pub u8);

/// A press now held.
#[derive(Clone, Copy, Debug)]
struct Held {
    entity: Entity,
    at: Vec2,
    seconds: f64,
    travelled: bool,
    /// It is the second of a pair: a click that ends it is a double press.
    second: bool,
}

/// A click that may be the first of a pair, or the last press of a run.
#[derive(Clone, Copy, Debug)]
struct Mark {
    entity: Entity,
    at: Vec2,
    seconds: f64,
}

#[derive(Default, Debug)]
struct PointerPresses {
    held: Option<Held>,
    first: Option<Mark>,
    run: Option<(Mark, u8)>,
}

/// Every pointer's presses: the one state the double press and the click count are read from.
#[derive(Resource, Default, Debug)]
pub(crate) struct Presses(HashMap<PointerId, PointerPresses>);

fn near(a: Vec2, b: Vec2, slop: f32) -> bool {
    (a.x - b.x).abs() <= slop && (a.y - b.y).abs() <= slop
}

fn within(mark: &Mark, entity: Entity, at: Vec2, seconds: f64, settings: &PointerSettings) -> bool {
    mark.entity == entity
        && seconds - mark.seconds <= f64::from(settings.double_click_seconds)
        && near(mark.at, at, settings.double_click_slop_px)
}

/// A pointer event is seen once, at the entity it started from, before it bubbles: a global
/// observer runs at every hop of a propagating event.
macro_rules! first_hop {
    ($event:expr) => {
        $event.event_target() == $event.original_event_target()
    };
}

/// Lets go of the press `presses` holds, taking its count off its entity.
fn let_go(presses: &mut PointerPresses, commands: &mut Commands) -> Option<Held> {
    let held = presses.held.take()?;
    if held.travelled {
        presses.run = None;
    }
    if let Ok(mut entity) = commands.get_entity(held.entity) {
        entity.try_remove::<ClickCount>();
    }
    Some(held)
}

pub(crate) fn presses_on_press(
    press: On<PointerPress>,
    settings: Res<PointerSettings>,
    time: Res<Time<Real>>,
    mut state: ResMut<Presses>,
    mut commands: Commands,
) {
    if !first_hop!(press) {
        return;
    }
    let seconds = time.elapsed_secs_f64();
    let interval = f64::from(settings.double_click_seconds);
    // A pointer that holds nothing and pressed longer ago than the interval has nothing left to
    // pair (a touch is a new pointer each time): forget it.
    state.0.retain(|id, presses| {
        *id == press.pointer.id
            || presses.held.is_some()
            || presses
                .run
                .is_some_and(|(mark, _)| seconds - mark.seconds <= interval)
    });
    let presses = state.0.entry(press.pointer.id).or_default();
    let_go(presses, &mut commands);
    // Any press breaks a pending pair unless it is that pair's second.
    let first = presses.first.take();
    if press.button != PointerButton::Primary {
        presses.run = None;
        return;
    }
    let (entity, at) = (press.entity, press.pointer.position);
    let count = match presses.run {
        Some((mark, count)) if within(&mark, entity, at, seconds, &settings) => {
            count.saturating_add(1)
        }
        _ => 1,
    };
    presses.run = Some((
        Mark {
            entity,
            at,
            seconds,
        },
        count,
    ));
    presses.held = Some(Held {
        entity,
        at,
        seconds,
        travelled: false,
        second: first.is_some_and(|mark| within(&mark, entity, at, seconds, &settings)),
    });
    commands.entity(entity).try_insert(ClickCount(count));
}

pub(crate) fn presses_on_drag(
    drag: On<PointerDrag>,
    settings: Res<PointerSettings>,
    mut state: ResMut<Presses>,
) {
    if !first_hop!(drag) || drag.button != PointerButton::Primary {
        return;
    }
    let slop = settings.double_click_slop_px;
    if let Some(held) = state
        .0
        .get_mut(&drag.pointer.id)
        .and_then(|presses| presses.held.as_mut())
        .filter(|held| held.entity == drag.entity)
        && !near(drag.distance, Vec2::ZERO, slop)
    {
        held.travelled = true;
    }
}

/// Bevy sends the click before the release, so the press is still held here.
pub(crate) fn presses_on_click(
    click: On<PointerClick>,
    settings: Res<PointerSettings>,
    time: Res<Time<Real>>,
    mut state: ResMut<Presses>,
    mut commands: Commands,
) {
    if !first_hop!(click) || click.button != PointerButton::Primary {
        return;
    }
    let Some(presses) = state.0.get_mut(&click.pointer.id) else {
        return;
    };
    let Some(held) = presses.held.filter(|held| held.entity == click.entity) else {
        return;
    };
    // Only clicks pair: a press that travelled or was held past the interval is neither half.
    let interval = f64::from(settings.double_click_seconds);
    if held.travelled || time.elapsed_secs_f64() - held.seconds > interval {
        return;
    }
    if held.second {
        commands.trigger(DoublePress {
            entity: held.entity,
        });
    } else {
        presses.first = Some(Mark {
            entity: held.entity,
            at: held.at,
            seconds: held.seconds,
        });
    }
}

/// The pointer lets go over the entity it pressed.
pub(crate) fn presses_on_release(
    release: On<PointerRelease>,
    mut state: ResMut<Presses>,
    mut commands: Commands,
) {
    if !first_hop!(release) || release.button != PointerButton::Primary {
        return;
    }
    if let Some(presses) = state.0.get_mut(&release.pointer.id)
        && presses
            .held
            .is_some_and(|held| held.entity == release.entity)
    {
        let_go(presses, &mut commands);
    }
}

/// The pointer lets go after leaving the entity it pressed: Bevy sends that entity no release, but
/// a press that moved is a drag, and every drag ends.
pub(crate) fn presses_on_drag_end(
    end: On<PointerDragEnd>,
    mut state: ResMut<Presses>,
    mut commands: Commands,
) {
    if !first_hop!(end) || end.button != PointerButton::Primary {
        return;
    }
    if let Some(presses) = state.0.get_mut(&end.pointer.id)
        && presses.held.is_some_and(|held| held.entity == end.entity)
    {
        let_go(presses, &mut commands);
    }
}

pub(crate) fn presses_on_cancel(
    cancel: On<PointerCancel>,
    mut state: ResMut<Presses>,
    mut commands: Commands,
) {
    if !first_hop!(cancel) {
        return;
    }
    if let Some(mut presses) = state.0.remove(&cancel.pointer.id) {
        let_go(&mut presses, &mut commands);
    }
}
