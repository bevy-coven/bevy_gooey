//! Progress and spinner: honest feedback for anything that takes longer than a second.

use bevy::prelude::*;

use crate::model::{meter_fraction, progress_text, spinner_angle};
use crate::skin::Role;

/// How far along a [`progress`](crate::progress) is, `0.0` to `1.0`. Change it and
/// [`update_progress`] moves the fill the same frame. A value outside the range, or NaN, is drawn
/// as the nearest end (NaN as empty) and reported once: see [`ProgressClamped`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[require(ProgressText)]
pub struct Progress(pub f32);

/// The words under a [`progress`](crate::progress) bar: which step of how many, and about how long
/// is left, drawn "3 of 7: Measuring meshes, about 20 s left". With neither the line is not drawn.
/// Insert it on the progress and edit it in place as the work goes.
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct ProgressText {
    /// The step the work is on, how many there are and what this one is called: `(3, 7, "Measuring
    /// meshes")`.
    pub step: Option<(u32, u32, String)>,
    /// The estimate of the time left, in seconds. It is printed with the precision an estimate
    /// has ([`crate::model::eta_words`]).
    pub eta_seconds: Option<f32>,
}

/// On a [`Progress`] whose value is outside `0.0..=1.0` (or NaN) while it is. A warning is logged
/// once when it is added, not every frame; it is taken off when the value is back in range.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct ProgressClamped;

/// An indeterminate indicator: a ring with one lit quarter turning once every
/// [`SPINNER_PERIOD_SECS`](crate::model::SPINNER_PERIOD_SECS), on real time, so it keeps turning
/// while the game is paused or a frame stalls. Use it for 2 to 10 seconds of work; use a
/// [`progress`](crate::progress) for longer. There is no static variant.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct Spinner;

/// What [`update_progress`] reads of a progress bar.
type Bar = (
    Entity,
    Ref<'static, Progress>,
    Ref<'static, ProgressText>,
    &'static Children,
    Has<ProgressClamped>,
);

/// A progress bar whose value or words changed.
type Moved = Or<(Changed<Progress>, Changed<ProgressText>)>;

/// Sets each changed [`Progress`]'s fill and each changed [`ProgressText`]'s line. Runs in
/// [`crate::GooeySystems::Style`].
pub fn update_progress(
    mut commands: Commands,
    bars: Query<Bar, Moved>,
    mut parts: Query<(&Role, &mut Node)>,
    mut texts: Query<&mut Text>,
    children_of: Query<&Children>,
) {
    for (entity, value, words, children, clamped) in &bars {
        if value.is_changed() {
            let out = !(0.0..=1.0).contains(&value.0);
            match (out, clamped) {
                (true, false) => {
                    warn!(
                        "bevy_gooey: progress {entity}: {} is outside 0 to 1; drawing it clamped",
                        value.0
                    );
                    commands.entity(entity).insert(ProgressClamped);
                }
                (false, true) => {
                    commands.entity(entity).remove::<ProgressClamped>();
                }
                _ => {}
            }
            let width = Val::Percent(meter_fraction(value.0, 1.0) * 100.0);
            for descendant in children_of.iter_descendants(entity) {
                if let Ok((Role::MeterFill, mut node)) = parts.get_mut(descendant)
                    && node.width != width
                {
                    node.width = width;
                }
            }
        }
        if words.is_changed() {
            let line = progress_text(
                words
                    .step
                    .as_ref()
                    .map(|(at, of, name)| (*at, *of, name.as_str())),
                words.eta_seconds,
            );
            let display = if line.is_empty() {
                Display::None
            } else {
                Display::Flex
            };
            for child in children.iter() {
                let Ok((Role::ProgressLine, mut node)) = parts.get_mut(child) else {
                    continue;
                };
                node.display = display;
                if let Ok(mut text) = texts.get_mut(child)
                    && text.0 != line
                {
                    text.0 = line.clone();
                }
            }
        }
    }
}

/// Turns every [`Spinner`] to where real time puts it. Runs in [`crate::GooeySystems::Style`].
pub fn update_spinners(
    time: Option<Res<Time<Real>>>,
    mut spinners: Query<&mut UiTransform, With<Spinner>>,
) {
    let angle = time.map_or(0.0, |time| spinner_angle(time.elapsed_secs()));
    for mut transform in &mut spinners {
        transform.rotation = Rot2::radians(angle);
    }
}
