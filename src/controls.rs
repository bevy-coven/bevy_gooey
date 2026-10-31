//! Settings controls: what keeps a [`crate::slider`]'s thumb, fill and readout in step with its value.

use bevy::prelude::*;
use bevy::ui_widgets::{SliderRange, SliderValue};

use crate::model::{slider_fraction, slider_text};
use crate::skin::Role;

/// A slider whose value or range changed since the last run.
type Moved = Or<(Changed<SliderValue>, Changed<SliderRange>)>;

/// Moves each changed slider's thumb, fill and readout to its [`SliderValue`] within its
/// [`SliderRange`]: the same frame the value changes, from a drag, a key, the D-pad or the game.
/// Runs in [`crate::GooeySystems::Style`].
///
/// The thumb is placed by flex: a gap before it grows to the value's fraction of the free room, so
/// its travel is the slider's width less the thumb's, which is the travel `bevy_ui_widgets` assumes
/// when it turns a click or a drag into a value. A slider whose value did not change costs nothing.
pub fn update_sliders(
    sliders: Query<(Entity, &SliderValue, &SliderRange), Moved>,
    children: Query<&Children>,
    mut parts: Query<(&Role, &mut Node, Option<&mut Text>)>,
) {
    for (slider, value, range) in &sliders {
        let (min, max) = (range.start(), range.end());
        let fraction = slider_fraction(value.0, min, max);
        for part in children.iter_descendants(slider) {
            let Ok((role, mut node, text)) = parts.get_mut(part) else {
                continue;
            };
            match role {
                Role::SliderGap if node.flex_grow != fraction => node.flex_grow = fraction,
                Role::SliderFill => {
                    let width = Val::Percent(fraction * 100.0);
                    if node.width != width {
                        node.width = width;
                    }
                }
                Role::SliderReadout => {
                    let shown = slider_text(value.0, min, max);
                    if let Some(mut text) = text
                        && text.0 != shown
                    {
                        text.0 = shown;
                    }
                }
                _ => {}
            }
        }
    }
}
