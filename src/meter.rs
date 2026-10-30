//! HUD meters: a value against a maximum, drawn as a fill.

use bevy::prelude::*;

use crate::model::meter_fraction;
use crate::skin::Role;

/// A meter's reading. Change it and [`update_meters`] moves the fill the same frame. Any value is
/// safe: the fill is clamped to empty and full, and a non-finite or non-positive maximum reads as
/// empty.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Meter {
    /// The current value, e.g. health left.
    pub value: f32,
    /// The value that fills the meter.
    pub max: f32,
}

/// Sets each changed [`Meter`]'s fill width to its fraction. Runs in [`crate::GooeySystems::Style`].
pub fn update_meters(
    meters: Query<(&Meter, &Children), Changed<Meter>>,
    mut fills: Query<(&Role, &mut Node)>,
) {
    for (meter, children) in &meters {
        let width = Val::Percent(meter_fraction(meter.value, meter.max) * 100.0);
        for child in children.iter() {
            if let Ok((Role::MeterFill, mut node)) = fills.get_mut(child)
                && node.width != width
            {
                node.width = width;
            }
        }
    }
}
