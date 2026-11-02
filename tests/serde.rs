//! A skin and the scale settings are data a project saves: they round-trip, and a partial file
//! fills the rest from the default.
#![cfg(feature = "serde")]

use bevy::prelude::{GamepadButton, KeyCode, MouseButton};
use bevy_gooey::{
    Binding, BindingSettings, Density, DockLayout, DockZone, GooeySkin, MotionSettings, PadStyle,
    SkinPreset, SplitAxis, SplitFraction, SplitMin, ToastAnchor, ToastSettings, UiScaleSettings,
};

#[test]
fn a_skin_round_trips_through_ron() -> Result<(), Box<dyn std::error::Error>> {
    let text = ron::to_string(&GooeySkin::parchment())?;
    assert_eq!(ron::from_str::<GooeySkin>(&text)?, GooeySkin::parchment());
    Ok(())
}

#[test]
fn the_tool_look_keeps_its_density_through_ron_and_a_file_without_one_is_comfortable()
-> Result<(), Box<dyn std::error::Error>> {
    let text = ron::to_string(&GooeySkin::tool())?;
    let back = ron::from_str::<GooeySkin>(&text)?;
    assert_eq!(back, GooeySkin::tool());
    assert_eq!(back.density, Density::Compact);
    // A skin saved before the field existed reads as comfortable.
    assert_eq!(
        ron::from_str::<GooeySkin>("(text_scale: 1.5)")?.density,
        Density::Comfortable
    );
    assert_eq!(
        ron::from_str::<GooeySkin>("(density: Compact)")?.target(),
        24.0
    );
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

#[test]
fn the_ladder_and_the_scales_are_data_and_a_file_without_them_reads_as_the_default()
-> Result<(), Box<dyn std::error::Error>> {
    let mine = GooeySkin {
        spacing: [1.0, 2.0, 3.0, 5.0, 8.0, 13.0],
        radii: [0.0, 3.0, 9.0],
        type_scale: [12.0, 14.0, 18.0, 24.0, 30.0],
        surfaces: GooeySkin::tool().surfaces,
        ..GooeySkin::parchment()
    };
    let back = ron::from_str::<GooeySkin>(&ron::to_string(&mine)?)?;
    assert_eq!(back, mine);
    // A skin saved before the tokens existed fills them from the default skin.
    let old = ron::from_str::<GooeySkin>("(text_scale: 1.5)")?;
    assert_eq!(old.surfaces, GooeySkin::default().surfaces);
    assert_eq!(old.spacing, GooeySkin::default().spacing);
    Ok(())
}

#[test]
fn a_split_keeps_its_fraction_and_minimums_as_data() -> Result<(), Box<dyn std::error::Error>> {
    let fraction = ron::from_str::<SplitFraction>(&ron::to_string(&SplitFraction(0.3))?)?;
    assert_eq!(fraction, SplitFraction(0.3));
    let min = SplitMin::new(180.0, 320.0);
    assert_eq!(ron::from_str::<SplitMin>(&ron::to_string(&min)?)?, min);
    assert_eq!(
        ron::from_str::<SplitAxis>(&ron::to_string(&SplitAxis::Vertical)?)?,
        SplitAxis::Vertical
    );
    Ok(())
}

#[test]
fn a_dock_layout_round_trips_through_ron_and_a_short_file_reads()
-> Result<(), Box<dyn std::error::Error>> {
    let layout = DockLayout::split(
        SplitAxis::Horizontal,
        0.25,
        DockLayout::Group {
            panels: vec!["scene".into(), "assets".into()],
            selected: 1,
        },
        DockLayout::split(
            SplitAxis::Vertical,
            0.7,
            DockLayout::group(["viewport"]),
            DockLayout::group(["log"]),
        ),
    );
    let text = ron::to_string(&layout)?;
    assert_eq!(ron::from_str::<DockLayout>(&text)?, layout);
    // A file that names only the panels: the first tab shows and a split divides in half.
    let short = ron::from_str::<DockLayout>(
        r#"Split(axis: Vertical, first: Group(panels: ["a", "b"]), second: Group(panels: ["c"]))"#,
    )?;
    assert_eq!(
        short,
        DockLayout::split(
            SplitAxis::Vertical,
            0.5,
            DockLayout::group(["a", "b"]),
            DockLayout::group(["c"])
        )
    );
    // The drop's place is data too: a game keeps a layout and a zone to restore a drag.
    assert_eq!(
        ron::from_str::<DockZone>(&ron::to_string(&DockZone::Left)?)?,
        DockZone::Left
    );
    Ok(())
}

#[test]
fn the_reduce_motion_setting_round_trips_and_an_empty_file_is_full_motion()
-> Result<(), Box<dyn std::error::Error>> {
    let off = ron::to_string(&MotionSettings::off())?;
    assert!(ron::from_str::<MotionSettings>(&off)?.is_off());
    assert_eq!(
        ron::from_str::<MotionSettings>("()")?,
        MotionSettings::default()
    );
    Ok(())
}
