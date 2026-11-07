//! The options screen's rows for the settings this crate itself reads: [`crate::text_size`],
//! [`crate::interface_size`], [`crate::pad_style`], [`crate::skin_preset`] and
//! [`crate::motion_row`]. Each is a
//! [`crate::choice`] that edits one field of a resource and shows it, so the setting lives once,
//! in the resource.

use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;

use crate::binding::BindingSettings;
use crate::choice::ChoiceValue;
use crate::model::{INTERFACE_SIZE_STEPS, PAD_STYLES, TEXT_SIZE_STEPS, nearest_step};
use crate::motion::MotionSettings;
use crate::scale::UiScaleSettings;
use crate::skin::{GooeySkin, SkinPreset};

/// Which of the crate's own settings a [`crate::choice`] row edits. Part of what
/// [`crate::text_size`], [`crate::interface_size`], [`crate::pad_style`] and
/// [`crate::skin_preset`] spawn, for [`update_option_rows`] to find them by; a game never builds
/// one.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OptionRow {
    /// [`GooeySkin::text_scale`], in [`TEXT_SIZE_STEPS`].
    TextSize,
    /// [`UiScaleSettings::scale`], in [`INTERFACE_SIZE_STEPS`].
    InterfaceSize,
    /// [`BindingSettings::pad_style`], in [`PAD_STYLES`].
    PadStyle,
    /// [`GooeySkin`]'s colours, in [`SkinPreset::ALL`].
    SkinPreset,
    /// [`MotionSettings`], in [`MOTION_STEPS`].
    Motion,
}

/// The steps a [`crate::motion_row`] offers, as the player reads them: full motion, reduced motion
/// ([`MotionSettings::reduced`]) and none ([`MotionSettings::off`]).
pub const MOTION_STEPS: [&str; 3] = ["Full", "Reduced", "Off"];

/// The [`MOTION_STEPS`] index a setting shows as: Off for any scale that is off, Reduced for a
/// reduced one, else Full. A hand-written scale (0.5, 2) shows as the step it is in.
pub fn motion_step(settings: &MotionSettings) -> usize {
    if settings.is_off() {
        2
    } else if settings.reduced {
        1
    } else {
        0
    }
}

/// `settings` moved to [`MOTION_STEPS`]'s step `index`: Full and Reduced keep the player's own
/// `duration_scale` (1 when coming from Off), and Off is [`MotionSettings::off`]. `None` for an
/// index past the steps.
pub fn with_motion_step(settings: &MotionSettings, index: usize) -> Option<MotionSettings> {
    let scale = if settings.is_off() {
        MotionSettings::default().duration_scale
    } else {
        settings.duration_scale
    };
    match index {
        0 | 1 => Some(MotionSettings {
            duration_scale: scale,
            reduced: index == 1,
        }),
        2 => Some(MotionSettings::off()),
        _ => None,
    }
}

impl OptionRow {
    /// The options the row offers, as the player reads them: a percentage for a size, the maker's
    /// name for a controller. Numerals and proper nouns, so the host localises nothing there; the
    /// theme row's names are English words ([`SkinPreset::name`]), and a host that localises writes
    /// its own [`crate::choice`] over [`SkinPreset::ALL`].
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
            Self::SkinPreset => SkinPreset::ALL
                .iter()
                .map(|preset| preset.name().to_owned())
                .collect(),
            Self::Motion => MOTION_STEPS.iter().map(|&step| step.to_owned()).collect(),
        }
    }
}

/// A step as a percentage: `1.25` is "125%".
fn percent(step: f32) -> String {
    format!("{:.0}%", step * 100.0)
}

/// A player's step on an option row is one field of the resource it edits, and no other (a theme
/// row's step is the skin's colours, and no size): a game's own skin sizes, the scale's reference
/// and the binding prompts stay as they were. The row's own
/// observer, like [`crate::choice`]'s `choice_self_update`, so it hears only its own row's change;
/// a resource a host left out is not written (the plugin that owns it is not added).
pub(crate) fn option_on_change(
    change: On<ValueChange<usize>>,
    rows: Query<&OptionRow>,
    skin: Option<ResMut<GooeySkin>>,
    scale: Option<ResMut<UiScaleSettings>>,
    binding: Option<ResMut<BindingSettings>>,
    motion: Option<ResMut<MotionSettings>>,
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
        OptionRow::SkinPreset => {
            if let (Some(mut skin), Some(preset)) = (skin, SkinPreset::ALL.get(change.value)) {
                let next = skin.clone().with_colours_of(&preset.skin());
                skin.set_if_neq(next);
            }
        }
        OptionRow::Motion => {
            if let Some(mut motion) = motion
                && let Some(next) = with_motion_step(&motion, change.value)
            {
                motion.set_if_neq(next);
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
pub fn update_option_rows(
    skin: Res<GooeySkin>,
    scale: Option<Res<UiScaleSettings>>,
    binding: Res<BindingSettings>,
    motion: Option<Res<MotionSettings>>,
    mut rows: Query<(Ref<OptionRow>, &mut ChoiceValue)>,
) {
    for (row, mut value) in &mut rows {
        // The index is looked for only when the setting changed or the row is new.
        let index = match *row {
            OptionRow::TextSize => (skin.is_changed() || row.is_added())
                .then(|| nearest_step(&TEXT_SIZE_STEPS, skin.text_scale)),
            OptionRow::SkinPreset => (skin.is_changed() || row.is_added()).then(|| {
                let preset = SkinPreset::of(&skin);
                SkinPreset::ALL
                    .iter()
                    .position(|&candidate| candidate == preset)
                    .unwrap_or(0)
            }),
            OptionRow::InterfaceSize => scale
                .as_ref()
                .filter(|scale| scale.is_changed() || row.is_added())
                .map(|scale| nearest_step(&INTERFACE_SIZE_STEPS, scale.scale)),
            OptionRow::Motion => motion
                .as_ref()
                .filter(|motion| motion.is_changed() || row.is_added())
                .map(|motion| motion_step(motion)),
            OptionRow::PadStyle => (binding.is_changed() || row.is_added()).then(|| {
                PAD_STYLES
                    .iter()
                    .position(|&style| style == binding.pad_style)
                    .unwrap_or(0)
            }),
        };
        if let Some(index) = index {
            value.set_if_neq(ChoiceValue(index));
        }
    }
}
