//! Named tones: a skin carries a host's own colours, a node is drawn with one, and the pairs a host
//! declares are checked with the crate's own arithmetic.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

fn with_tones() -> GooeySkin {
    let mut skin = GooeySkin::dark();
    skin.tones
        .insert("suggest".into(), Color::srgb(0.62, 0.86, 0.55));
    skin.tones
        .insert("veil".into(), Color::srgb(0.02, 0.02, 0.03));
    skin.tones
        .insert("dim".into(), Color::srgb(0.30, 0.32, 0.36));
    skin
}

#[test]
fn a_tone_reads_back_by_name_and_an_absent_one_is_none() {
    let skin = with_tones();
    assert_eq!(skin.tone("suggest"), Some(Color::srgb(0.62, 0.86, 0.55)));
    assert_eq!(skin.tone("absent"), None);
    // Case-sensitive: no default colour, no folding.
    assert_eq!(skin.tone("Suggest"), None);
}

#[test]
fn every_preset_has_no_tones_and_is_valid_with_some() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        assert!(skin.tones.is_empty(), "{}", preset.name());
        assert!(skin.problems().is_empty(), "{}", preset.name());
    }
    // Tones are not checked on their own: a skin with a very dim one is still valid.
    assert!(with_tones().problems().is_empty());
}

#[test]
fn a_declared_pair_under_its_ratio_is_low_contrast_and_one_over_is_not() {
    let skin = with_tones();
    // `suggest` on the dark surface is far above 4.5:1; `dim` on it is far under.
    let problems = skin.tone_problems(&[
        ("suggest on surface", "suggest", "surface", 4.5),
        ("dim on surface", "dim", "surface", 4.5),
    ]);
    assert_eq!(problems.len(), 1, "{problems:?}");
    match &problems[0] {
        SkinProblem::LowContrast {
            pair,
            ratio,
            needed,
        } => {
            assert_eq!(*pair, "dim on surface");
            assert_eq!(*needed, 4.5);
            assert!(*ratio < 4.5 && *ratio > 1.0, "{ratio}");
        }
        other => panic!("expected LowContrast, got {other:?}"),
    }
    // The same pair passes at a lower bar, and fails again at the high-contrast bar.
    assert!(
        skin.tone_problems(&[("dim on surface", "dim", "surface", 1.2)])
            .is_empty()
    );
    assert_eq!(
        skin.tone_problems(&[("suggest on veil", "suggest", "veil", ENHANCED_CONTRAST)])
            .len(),
        0,
        "a bright ink on a near-black ground clears 7:1"
    );
    assert_eq!(
        skin.tone_problems(&[("suggest on text", "suggest", "text", ENHANCED_CONTRAST)])
            .len(),
        1
    );
}

#[test]
fn a_pair_names_the_skins_own_colours_too_and_either_side_may_be_a_tone() {
    let skin = with_tones();
    assert!(
        skin.tone_problems(&[("text on surface", "text", "surface", 4.5)])
            .is_empty()
    );
    // A tone as the ground, a built-in colour as the ink.
    assert!(
        skin.tone_problems(&[("text on veil", "text", "veil", 4.5)])
            .is_empty()
    );
}

#[test]
fn a_name_that_is_nothing_is_reported_and_never_passes() {
    let skin = with_tones();
    let problems = skin.tone_problems(&[
        ("ghost on surface", "ghost", "surface", 1.0),
        ("both", "nope", "nada", 1.0),
    ]);
    let unknown: Vec<&str> = problems
        .iter()
        .filter_map(|p| match p {
            SkinProblem::UnknownTone { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(unknown, ["ghost", "nope", "nada"]);
    assert!(
        problems[0]
            .to_string()
            .contains("\"ghost\" is neither a tone")
    );
}

#[test]
fn a_theme_step_keeps_the_hosts_tones() {
    let stepped = with_tones().with_colours_of(&GooeySkin::parchment());
    assert_eq!(stepped.tone("suggest"), Some(Color::srgb(0.62, 0.86, 0.55)));
    assert_eq!(stepped.surface, GooeySkin::parchment().surface);
}

#[cfg(feature = "serde")]
#[test]
fn a_skin_file_with_tones_reads_back_and_one_without_has_none()
-> Result<(), Box<dyn std::error::Error>> {
    let skin = with_tones();
    let text = ron::to_string(&skin)?;
    let back = ron::from_str::<GooeySkin>(&text)?;
    assert_eq!(back, skin);
    assert_eq!(back.tone("suggest"), skin.tone("suggest"));
    assert_eq!(back.tone("absent"), None);
    // A hand-written file.
    let file = ron::from_str::<GooeySkin>(
        r#"(tones: {"suggest": Srgba((red: 0.6, green: 0.8, blue: 0.5, alpha: 1.0))})"#,
    )?;
    assert_eq!(file.tone("suggest"), Some(Color::srgba(0.6, 0.8, 0.5, 1.0)));
    assert!(
        ron::from_str::<GooeySkin>("(text_scale: 1.5)")?
            .tones
            .is_empty()
    );
    Ok(())
}

fn app_with(skin: GooeySkin) -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1280, 720).with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>(),
        GooeyPlugins,
    ))
    .insert_resource(skin);
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

fn fill(app: &App, entity: Entity) -> Option<Color> {
    app.world().get::<BackgroundColor>(entity).map(|b| b.0)
}

#[test]
fn a_tone_node_is_drawn_with_its_tone_and_follows_the_skin_the_same_frame() {
    let mut app = app_with(with_tones());
    let suggest = app
        .world_mut()
        .spawn((Role::Tone("suggest"), Node::default()))
        .id();
    let absent = app
        .world_mut()
        .spawn((Role::Tone("absent"), Node::default()))
        .id();
    let scoped = app
        .world_mut()
        .spawn((
            Role::Tone("suggest"),
            Node::default(),
            SkinScope(GooeySkin::tool()),
        ))
        .id();
    let card = app.world_mut().spawn(panel()).id();
    for node in [suggest, absent, scoped] {
        app.world_mut().entity_mut(card).add_child(node);
    }
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);

    assert_eq!(
        fill(&app, suggest),
        Some(Color::srgb(0.62, 0.86, 0.55)),
        "the tone is the fill"
    );
    // An unknown tone draws no colour of its own (never a default).
    assert_eq!(fill(&app, absent), Some(Color::NONE));
    // A scope that has no such tone draws none either.
    assert_eq!(fill(&app, scoped), Some(Color::NONE));

    // The skin changes: the node is restyled in the very next frame.
    let mut next = with_tones();
    next.tones
        .insert("suggest".into(), Color::srgb(0.9, 0.2, 0.2));
    app.insert_resource(next);
    app.update();
    assert_eq!(fill(&app, suggest), Some(Color::srgb(0.9, 0.2, 0.2)));
    // And a tone that appears later is picked up.
    let mut later = GooeySkin::dark();
    later
        .tones
        .insert("absent".into(), Color::srgb(0.1, 0.2, 0.9));
    app.insert_resource(later);
    app.update();
    assert_eq!(fill(&app, absent), Some(Color::srgb(0.1, 0.2, 0.9)));
    assert_eq!(
        fill(&app, suggest),
        Some(Color::NONE),
        "a tone the new skin lacks leaves no stale colour of the old skin's"
    );
}
