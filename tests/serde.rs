//! A skin and the scale settings are data a project saves: they round-trip, and a partial file
//! fills the rest from the default.
#![cfg(feature = "serde")]

use bevy::prelude::{GamepadButton, KeyCode, MouseButton};
use bevy_gooey::{
    Binding, BindingSettings, GooeySkin, PadStyle, SkinPreset, ToastAnchor, ToastSettings,
    UiScaleSettings,
};

#[test]
fn a_skin_round_trips_through_ron() -> Result<(), Box<dyn std::error::Error>> {
    let text = ron::to_string(&GooeySkin::parchment())?;
    assert_eq!(ron::from_str::<GooeySkin>(&text)?, GooeySkin::parchment());
    Ok(())
}

#[test]
fn a_high_contrast_skin_keeps_its_edge_through_ron_and_a_file_without_one_has_none()
-> Result<(), Box<dyn std::error::Error>> {
    let text = ron::to_string(&GooeySkin::high_contrast())?;
    let back = ron::from_str::<GooeySkin>(&text)?;
    assert_eq!(back, GooeySkin::high_contrast());
    assert!(back.surface_edge.is_some());
    // A skin saved before the field existed reads as the quiet frame.
    assert_eq!(
        ron::from_str::<GooeySkin>("(text_scale: 1.5)")?.surface_edge,
        None
    );
    Ok(())
}

#[test]
fn the_players_look_is_data_a_game_saves() -> Result<(), Box<dyn std::error::Error>> {
    for preset in SkinPreset::ALL {
        let text = ron::to_string(&preset)?;
        assert_eq!(ron::from_str::<SkinPreset>(&text)?, preset);
    }
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

#[test]
fn a_games_bindings_are_data_it_saves_and_loads() -> Result<(), Box<dyn std::error::Error>> {
    let saved = vec![
        Binding::Key(KeyCode::Space),
        Binding::Mouse(MouseButton::Right),
        Binding::Pad(GamepadButton::South),
    ];
    let text = ron::to_string(&saved)?;
    assert_eq!(ron::from_str::<Vec<Binding>>(&text)?, saved);
    Ok(())
}

#[test]
fn binding_settings_are_data_and_a_partial_file_keeps_the_rest()
-> Result<(), Box<dyn std::error::Error>> {
    let settings = ron::from_str::<BindingSettings>(
        r#"(pad_style: PlayStation, key_prompt: "Appuyez sur une touche, {exit} pour quitter")"#,
    )?;
    assert_eq!(settings.pad_style, PadStyle::PlayStation);
    assert_eq!(settings.cancel_key, KeyCode::Escape);
    assert_eq!(settings.cancel_button, GamepadButton::Start);
    assert_eq!(
        settings.key_prompt(),
        "Appuyez sur une touche, Esc pour quitter"
    );
    assert_eq!(settings.pad_prompt, BindingSettings::default().pad_prompt);
    assert_eq!(
        ron::from_str::<BindingSettings>(&ron::to_string(&settings)?)?,
        settings
    );
    Ok(())
}
