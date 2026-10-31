//! A skin and the scale settings are data a project saves: they round-trip, and a partial file
//! fills the rest from the default.
#![cfg(feature = "serde")]

use bevy_gooey::{GooeySkin, ToastAnchor, ToastSettings, UiScaleSettings};

#[test]
fn a_skin_round_trips_through_ron() -> Result<(), Box<dyn std::error::Error>> {
    let text = ron::to_string(&GooeySkin::parchment())?;
    assert_eq!(ron::from_str::<GooeySkin>(&text)?, GooeySkin::parchment());
    Ok(())
}

#[test]
fn an_empty_file_is_the_default_skin() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(ron::from_str::<GooeySkin>("()")?, GooeySkin::default());
    Ok(())
}

#[test]
fn a_scale_file_with_only_the_players_size_keeps_the_reference()
-> Result<(), Box<dyn std::error::Error>> {
    let settings = ron::from_str::<UiScaleSettings>("(scale: 1.25)")?;
    assert_eq!(settings.scale, 1.25);
    assert_eq!(
        settings.reference_short_side,
        UiScaleSettings::default().reference_short_side
    );
    Ok(())
}

#[test]
fn toast_settings_are_data_a_player_saves_and_a_partial_file_keeps_the_rest()
-> Result<(), Box<dyn std::error::Error>> {
    let settings = ron::from_str::<ToastSettings>("(duration_scale: 4.0, anchor: TopRight)")?;
    assert_eq!(settings.duration_scale, 4.0);
    assert_eq!(settings.anchor, ToastAnchor::TopRight);
    let default = ToastSettings::default();
    assert_eq!(settings.max_visible, default.max_visible);
    assert_eq!(settings.chars_per_second, default.chars_per_second);
    assert_eq!(
        ron::from_str::<ToastSettings>(&ron::to_string(&settings)?)?,
        settings
    );
    Ok(())
}
