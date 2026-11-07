//! HUD meters: a value against a maximum, drawn as a fill, with a trail that shows what was lost.

use bevy::prelude::*;

use crate::model::{MeterShown, meter_fraction, meter_motion};
use crate::motion::{MotionSettings, MotionToken};
use crate::skin::{GooeySkin, Role, SkinScope, skin_of};

/// A meter's reading. Change it and [`update_meters`] moves the fill: a loss drops it at once and
/// leaves a [`Role::MeterTrail`] of the lost length that holds a moment and drains; a gain takes
/// it up on the skin's snappy spring (see [`meter_motion`]; [`MotionSettings::off`] makes both instant). Any value is safe: the
/// fill is clamped to empty and full, and a non-finite or non-positive maximum reads as empty.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[require(MeterMotion)]
pub struct Meter {
    /// The current value, e.g. health left.
    pub value: f32,
    /// The value that fills the meter.
    pub max: f32,
}

/// What a meter draws now: its fill and trail, and whether it has been drawn once (the first frame
/// is the value, never a rise from empty). Internal; a game reads [`Meter`].
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct MeterMotion {
    shown: MeterShown,
    primed: bool,
}

/// Moves each [`Meter`]'s fill and trail toward its value: the fill's width, the trail's width and
/// its negative start. A settled meter whose value did not change is skipped before any node is
/// touched. Runs in [`crate::GooeySystems::Style`], on real time (a paused game's HUD still settles).
pub fn update_meters(
    settings: Option<Res<MotionSettings>>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    time: Option<Res<Time<Real>>>,
    mut meters: Query<(Entity, Ref<Meter>, &mut MeterMotion, &Children)>,
    mut parts: Query<(&Role, &mut Node)>,
) {
    // With no clock the old jump: a zero scale is the target at once.
    let scale = match (&settings, &time) {
        (Some(settings), Some(_)) => settings.duration_scale,
        (None, Some(_)) => 1.0,
        _ => 0.0,
    };
    let dt = time.map_or(0.0, |time| time.delta_secs());
    for (entity, meter, mut motion, children) in &mut meters {
        let target = meter_fraction(meter.value, meter.max);
        let shown = motion.shown;
        if motion.primed && !meter.is_changed() && shown.fill == target && shown.trail == target {
            continue;
        }
        let gain = skin_of(&skin, entity, &scopes, &parents)
            .motion
            .spring(MotionToken::Snappy);
        let next = if motion.primed {
            meter_motion(shown, target, dt, gain, scale)
        } else {
            meter_motion(shown, target, dt, gain, 0.0)
        };
        if motion.primed && next == shown {
            continue;
        }
        motion.shown = next;
        motion.primed = true;
        let fill = Val::Percent(next.fill * 100.0);
        // Nothing at all while there is no trail: a node under the fill would still fringe its
        // rounded corners in the trail's colour.
        let (trail, back) = if next.trail > next.fill {
            (
                Val::Percent(next.trail * 100.0),
                Val::Percent(-next.fill * 100.0),
            )
        } else {
            (Val::ZERO, Val::ZERO)
        };
        for child in children.iter() {
            let Ok((role, mut node)) = parts.get_mut(child) else {
                continue;
            };
            match role {
                Role::MeterFill if node.width != fill => node.width = fill,
                // A flex item after the fill, pulled back over it by the fill's own length (a
                // percentage margin is of the container's width, as the width is): it starts at
                // the bar's left edge and the row is as long as the trail.
                Role::MeterTrail if node.width != trail || node.margin.left != back => {
                    node.width = trail;
                    node.margin.left = back;
                }
                _ => {}
            }
        }
    }
}
