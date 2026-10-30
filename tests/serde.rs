//! A skin is data a project saves: it round-trips, and a partial file fills the rest from the default.
#![cfg(feature = "serde")]

use bevy_gooey::GooeySkin;

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
