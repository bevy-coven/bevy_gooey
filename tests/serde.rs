//! A skin and the scale settings are data a project saves: they round-trip, and a partial file
//! fills the rest from the default.
#![cfg(feature = "serde")]

use bevy_gooey::{GooeySkin, UiScaleSettings};

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
fn a_scale_file_with_only_the_players_size_keeps_the_reference() -> Result<(), Box<dyn std::error::Error>>
{
    let settings = ron::from_str::<UiScaleSettings>("(scale: 1.25)")?;
    assert_eq!(settings.scale, 1.25);
    assert_eq!(
        settings.reference_short_side,
        UiScaleSettings::default().reference_short_side
    );
    Ok(())
}
