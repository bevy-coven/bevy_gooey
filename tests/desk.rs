//! The Desk looks (G-49): Rune's neutral desktop tokens as two skins, their colours the ones
//! `GooeySkin::from_seed` derives from Rune's neutral themes, their sizes Rune's, and every check
//! of this crate met.

use bevy::color::Oklcha;
use bevy::prelude::*;
use bevy_gooey::*;

/// `a` and `b` within a rounding of their fourth decimal on every channel, alpha included.
fn close(a: Color, b: Color) -> bool {
    let (a, b) = (Srgba::from(a), Srgba::from(b));
    [
        (a.red, b.red),
        (a.green, b.green),
        (a.blue, b.blue),
        (a.alpha, b.alpha),
    ]
    .iter()
    .all(|(x, y)| (x - y).abs() < 2e-4)
}

#[test]
fn both_desk_looks_pass_every_check() {
    for (name, skin) in [
        ("desk", GooeySkin::desk()),
        ("desk_light", GooeySkin::desk_light()),
    ] {
        assert_eq!(skin.problems(), vec![], "{name}");
    }
}

#[test]
fn the_desk_colours_are_what_from_seed_derives_from_runes_neutral_themes() {
    for (name, skin, seed) in [
        ("desk", GooeySkin::desk(), PaletteSeed::desk()),
        (
            "desk_light",
            GooeySkin::desk_light(),
            PaletteSeed::desk_light(),
        ),
    ] {
        let derived = GooeySkin::from_seed(seed).expect("the Desk seeds are readable");
        let pairs = [
            ("background", skin.background, derived.background),
            ("surface", skin.surface, derived.surface),
            (
                "surface_raised",
                skin.surface_raised,
                derived.surface_raised,
            ),
            ("text", skin.text, derived.text),
            ("text_muted", skin.text_muted, derived.text_muted),
            ("accent", skin.accent, derived.accent),
            ("accent_text", skin.accent_text, derived.accent_text),
            ("danger", skin.danger, derived.danger),
            ("focus_ring", skin.focus_ring, derived.focus_ring),
            ("outline", skin.outline, derived.outline),
            ("hover", skin.hovered(), derived.hovered()),
            ("pressed", skin.pressed(), derived.pressed()),
            ("scrim", skin.scrim, derived.scrim),
            (
                "surface_edge",
                skin.surface_edge.unwrap_or(skin.surface_raised),
                derived.surface_edge.unwrap_or(derived.surface_raised),
            ),
            ("held", skin.held(), derived.held()),
            ("held_ground", skin.held_ground(), derived.held_ground()),
            ("unchecked", skin.unchecked(), derived.unchecked()),
            ("card", skin.card(), derived.card()),
            ("preview_edge", skin.preview_edge(), derived.preview_edge()),
        ];
        for (field, written, from_seed) in pairs {
            assert!(
                close(written, from_seed),
                "{name}.{field}: {written:?} against {from_seed:?}"
            );
        }
        assert_eq!(skin.surfaces.len(), derived.surfaces.len());
        for (step, (written, from_seed)) in skin.surfaces.iter().zip(&derived.surfaces).enumerate()
        {
            assert!(close(*written, *from_seed), "{name}.surfaces[{step}]");
        }
        assert_eq!(
            skin.elevation, derived.elevation,
            "{name}: the elevation of its side"
        );
    }
}

/// Rune's own light accent, `oklch(0.6 0.17 256)`, misses this crate's floors; the light Desk's is
/// 0.05 deeper.
#[test]
fn runes_own_light_accent_is_refused_and_the_deeper_one_is_not() {
    let rune_light = PaletteSeed {
        accent: Color::from(Oklcha::new(0.6, 0.17, 256.0, 1.0)),
        ..PaletteSeed::desk_light()
    };
    let problems = GooeySkin::from_seed(rune_light).expect_err("Rune's light blue misses 4.5:1");
    for pair in ["accent_text on accent", "accent on surface_raised"] {
        assert!(
            problems
                .iter()
                .any(|problem| matches!(problem, SkinProblem::LowContrast { pair: named, .. } if *named == pair)),
            "{pair}: {problems:?}"
        );
    }
    assert!(GooeySkin::from_seed(PaletteSeed::desk_light()).is_ok());
}

#[test]
fn the_desk_sizes_are_runes_compact_desktop_scale() {
    for skin in [GooeySkin::desk(), GooeySkin::desk_light()] {
        assert_eq!(skin.density, Density::Compact);
        assert_eq!((skin.text_size, skin.heading_size), (13.0, 15.0));
        assert_eq!(skin.type_scale, [11.5, 12.0, 13.0, 15.0, 20.0]);
        assert_eq!(skin.spacing, [2.0, 4.0, 8.0, 12.0, 16.0, 24.0]);
        assert_eq!(skin.radii, [6.0, 8.0, 12.0]);
        assert_eq!((skin.radius, skin.border), (6.0, 1.0));
        assert_eq!((skin.gap, skin.padding), (8.0, 12.0));
        assert_eq!(skin.min_target, MIN_TARGET_PX);
        assert_eq!(skin.target(), MIN_TARGET_PX, "24 px, WCAG 2.5.8");
        assert_eq!(
            skin.motion,
            MotionTokens::default(),
            "Rune's motion palette"
        );
    }
    assert_eq!(GooeySkin::desk().elevation, Elevation::on_dark());
    assert_eq!(GooeySkin::desk_light().elevation, Elevation::on_light());
}

#[test]
fn a_theme_step_onto_desk_writes_its_colours_and_none_of_its_sizes() {
    // From parchment, whose light stacks differ from the Desk's dark ones.
    let players = GooeySkin {
        text_scale: 1.5,
        ..GooeySkin::parchment()
    };
    assert_ne!(players.elevation, GooeySkin::desk().elevation);
    let stepped = players.clone().with_colours_of(&GooeySkin::desk());
    assert_eq!(stepped.accent, GooeySkin::desk().accent);
    assert_eq!(stepped.elevation, GooeySkin::desk().elevation);
    assert_eq!(
        (
            stepped.text_size,
            stepped.density,
            stepped.text_scale,
            stepped.radius
        ),
        (players.text_size, players.density, 1.5, players.radius),
        "the player's sizes stay theirs"
    );
}
