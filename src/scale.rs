//! The interface's size against the window: Bevy's `UiScale`, set from the window's shorter side.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::model::window_ui_scale;

/// How large the whole interface is drawn against the window, for [`scale_with_window`].
///
/// Bevy's `UiScale` is set to the primary window's shorter side divided by
/// [`UiScaleSettings::reference_short_side`], times [`UiScaleSettings::scale`]: the interface keeps
/// its share of the screen at every resolution, as a game's does. The skin's sizes are the sizes a
/// 1280x720 window shows; a 1920x1080 window draws them at 1.5x and a 2560x1440 one at 2x, so the
/// default text stays above the Xbox Accessibility Guidelines' console minimum (26 px at 1080p,
/// 52 px at 4K) on any screen. `UiScale` multiplies every `Val::Px`, so the host's own UI nodes
/// scale with the crate's widgets.
///
/// With the `serde` feature the settings are (de)serialisable, and a partial file fills every
/// missing field from the default.
///
/// ```
/// use bevy_gooey::UiScaleSettings;
/// // A player's "interface size: large" option.
/// let large = UiScaleSettings { scale: 1.25, ..UiScaleSettings::default() };
/// assert_eq!(large.reference_short_side, 720.0);
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct UiScaleSettings {
    /// The window's shorter side, in logical px, at which the interface is drawn at the skin's own
    /// sizes: 720. A game designed at 1080p sets 1080.
    pub reference_short_side: f32,
    /// The player's interface size, on top of the window's: 1.0. It enlarges everything, where
    /// [`crate::GooeySkin::text_scale`] enlarges text alone.
    pub scale: f32,
}

impl Default for UiScaleSettings {
    fn default() -> Self {
        Self {
            reference_short_side: 720.0,
            scale: 1.0,
        }
    }
}

/// Sets `UiScale` from the primary window's shorter side and [`UiScaleSettings`]
/// ([`window_ui_scale`]), writing it only when it changes, so a resized window is drawn at its new
/// scale the next frame. Runs in [`crate::GooeySystems::Style`].
///
/// A window with no size (minimised) keeps the last scale. A reference or a scale at or below zero
/// leaves `UiScale` as it is, with a warning when the settings change.
pub fn scale_with_window(
    settings: Res<UiScaleSettings>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut ui_scale: ResMut<UiScale>,
) {
    let UiScaleSettings {
        reference_short_side,
        scale,
    } = *settings;
    match window_ui_scale(window.width(), window.height(), reference_short_side, scale) {
        Some(new) if ui_scale.0 != new => ui_scale.0 = new,
        Some(_) => {}
        None if settings.is_changed() && !(reference_short_side > 0.0 && scale > 0.0) => {
            warn!(
                "UiScaleSettings: reference_short_side {reference_short_side} and scale {scale} must be above zero; UiScale stays {}",
                ui_scale.0
            );
        }
        None => {}
    }
}
