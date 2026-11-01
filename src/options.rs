//! The options screen's rows for the settings this crate itself reads: [`crate::text_size`],
//! [`crate::interface_size`] and [`crate::pad_style`]. Each is a [`crate::choice`] that edits one
//! field of a resource and shows it, so the setting lives once, in the resource.

use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;

use crate::binding::BindingSettings;
use crate::choice::ChoiceValue;
use crate::model::{INTERFACE_SIZE_STEPS, PAD_STYLES, TEXT_SIZE_STEPS, nearest_step};
use crate::scale::UiScaleSettings;
use crate::skin::GooeySkin;

/// Which of the crate's own settings a [`crate::choice`] row edits. Part of what
/// [`crate::text_size`], [`crate::interface_size`] and [`crate::pad_style`] spawn; a game never
/// builds one.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OptionRow {
    /// [`GooeySkin::text_scale`], in [`TEXT_SIZE_STEPS`].
    TextSize,
    /// [`UiScaleSettings::scale`], in [`INTERFACE_SIZE_STEPS`].
    InterfaceSize,
    /// [`BindingSettings::pad_style`], in [`PAD_STYLES`].
    PadStyle,
}

impl OptionRow {
    /// The options the row offers, as the player reads them: a percentage for a size, the maker's
    /// name for a controller. Numerals and proper nouns, so the host localises nothing here.
    pub(crate) fn options(self) -> Vec<String> {
        match self {
            Self::TextSize => TEXT_SIZE_STEPS.iter().map(|&step| percent(step)).collect(),
            Self::InterfaceSize => INTERFACE_SIZE_STEPS
                .iter()
                .map(|&step| percent(step))
                .collect(),
            Self::PadStyle => PAD_STYLES
                .iter()
                .map(|style| style.name().to_owned())
                .collect(),
        }
    }
}

/// A step as a percentage: `1.25` is "125%".
fn percent(step: f32) -> String {
    format!("{:.0}%", step * 100.0)
}

/// A player's step on an option row is one field of the resource it edits, and no other: a game's
/// own skin, the scale's reference and the binding prompts stay as they were. The row's own
/// observer, like [`crate::choice`]'s `choice_self_update`, so it hears only its own row's change;
/// a resource a host left out is not written (the plugin that owns it is not added).
pub(crate) fn option_on_change(
    change: On<ValueChange<usize>>,
    rows: Query<&OptionRow>,
    skin: Option<ResMut<GooeySkin>>,
    scale: Option<ResMut<UiScaleSettings>>,
    binding: Option<ResMut<BindingSettings>>,
) {
    let Ok(row) = rows.get(change.source) else {
        return;
    };
    match row {
        OptionRow::TextSize => {
            if let (Some(mut skin), Some(&step)) = (skin, TEXT_SIZE_STEPS.get(change.value)) {
                skin.text_scale = step;
            }
        }
        OptionRow::InterfaceSize => {
            if let (Some(mut scale), Some(&step)) = (scale, INTERFACE_SIZE_STEPS.get(change.value))
            {
                scale.scale = step;
            }
        }
        OptionRow::PadStyle => {
            if let (Some(mut binding), Some(&style)) = (binding, PAD_STYLES.get(change.value)) {
                binding.pad_style = style;
            }
        }
    }
}

/// Moves each option row to the step nearest its setting: when the setting changes (a loaded save,
/// another row, the host) and on the frame a row is spawned, so a row never draws one frame at the
/// wrong step. The row's state is set directly, and no [`ValueChange`] is sent, so a game's observer
/// hears the player and never this. Runs in [`crate::GooeySystems::Style`], before
/// [`crate::update_choices`], which draws the option; a setting that did not change and a row that
/// was not just spawned are not looked at past a flag.
pub(crate) fn update_option_rows(
    skin: Res<GooeySkin>,
    scale: Option<Res<UiScaleSettings>>,
    binding: Res<BindingSettings>,
    mut rows: Query<(Ref<OptionRow>, &mut ChoiceValue)>,
) {
    for (row, mut value) in &mut rows {
        let (changed, index) = match *row {
            OptionRow::TextSize => (
                skin.is_changed(),
                nearest_step(&TEXT_SIZE_STEPS, skin.text_scale),
            ),
            OptionRow::InterfaceSize => match &scale {
                Some(scale) => (
                    scale.is_changed(),
                    nearest_step(&INTERFACE_SIZE_STEPS, scale.scale),
                ),
                None => continue,
            },
            OptionRow::PadStyle => (
                binding.is_changed(),
                PAD_STYLES
                    .iter()
                    .position(|&style| style == binding.pad_style)
                    .unwrap_or(0),
            ),
        };
        if changed || row.is_added() {
            value.set_if_neq(ChoiceValue(index));
        }
    }
}
