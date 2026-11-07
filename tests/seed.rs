//! A skin from three seeds (G-47): dark and light seeds give skins that pass every check with a
//! ladder that steps away from the background, and a seed whose text cannot be read is refused,
//! naming the pair.

use bevy::prelude::*;
use bevy_gooey::*;

fn hex(code: &str) -> Color {
    Srgba::hex(code).expect("a hex colour").into()
}

fn seed(background: &str, text: &str, accent: &str) -> PaletteSeed {
    PaletteSeed {
        background: hex(background),
        text: hex(text),
        accent: hex(accent),
    }
}

fn rgb(colour: Color) -> [f32; 3] {
    let c = Srgba::from(colour);
    [c.red, c.green, c.blue]
}

fn lightness(colour: Color) -> f32 {
    cie_lightness(rgb(colour))
}

/// Three of Rune's `examples/kit-gallery` seeds at `f73f72c` (`--kit palette` dark, its `black`
/// and its `warm` variants), and a white paper with a blue accent for a light seed of the
/// default kind.
fn passing() -> [(&'static str, PaletteSeed, bool); 4] {
    [
        ("dark", seed("#0a0a0a", "#ededed", "#e5484d"), true),
        ("black", seed("#000000", "#ffffff", "#ff2a2a"), true),
        ("light", seed("#ffffff", "#171717", "#1d4ed8"), false),
        ("warm", seed("#e8e4dc", "#000000", "#d81414"), false),
    ]
}

#[test]
fn seeds_give_skins_that_pass_every_check_with_a_ladder_away_from_the_background() {
    for (name, seed, dark) in passing() {
        let skin = GooeySkin::from_seed(seed)
            .unwrap_or_else(|problems| panic!("{name}: refused with {problems:?}"));
        assert_eq!(skin.surfaces.len(), 5, "{name}: five steps");
        let away = if dark { 1.0 } else { -1.0 };
        let background = lightness(skin.background);
        let steps: Vec<f32> = skin.surfaces.iter().map(|s| lightness(*s)).collect();
        assert!(
            away * (steps[0] - background) >= 0.0,
            "{name}: the ladder starts at the background or past it: {steps:?} from {background}"
        );
        for pair in steps.windows(2) {
            assert!(
                away * (pair[1] - pair[0]) >= ELEVATION_STEP,
                "{name}: every step goes on away from the background by at least {ELEVATION_STEP} L*: {steps:?}"
            );
        }
        assert_eq!(
            skin.surface, skin.surfaces[1],
            "{name}: the panel is step 1"
        );
        assert_eq!(
            skin.surface_raised, skin.surfaces[3],
            "{name}: a resting button is step 3"
        );
        // `problems()` checks the ring on the panel and the resting button; the seed promises it
        // on the background and every step.
        let ring = rgb(skin.focus_ring);
        for ground in std::iter::once(skin.background).chain(skin.surfaces.iter().copied()) {
            assert!(
                contrast_ratio(ring, rgb(ground)) >= NON_TEXT_CONTRAST,
                "{name}: the focus ring is 3:1 on {ground:?}"
            );
        }
    }
}

/// Rune's light gallery seed puts words on a mid red (`#e5484d`) that neither black nor white reads
/// on at 4.5:1 across the button's three fills: this crate's rule wins over the reference, and the
/// seed is refused, never adjusted.
#[test]
fn runes_light_gallery_seed_is_refused_for_the_words_on_its_accent() {
    let problems = GooeySkin::from_seed(seed("#ffffff", "#171717", "#e5484d"))
        .expect_err("words on #e5484d cannot reach 4.5:1");
    assert!(!problems.is_empty());
    assert!(
        problems.iter().all(|problem| matches!(
            problem,
            SkinProblem::LowContrast { pair, .. } if pair.starts_with("accent_text on")
        )),
        "only the accent's words fail: {problems:?}"
    );
}

#[test]
fn the_derived_fills_are_the_text_over_the_resting_button_and_the_press_is_a_step_from_the_hover() {
    let skin = GooeySkin::from_seed(passing()[0].1).expect("the dark seed");
    let (hover, pressed, rest) = (
        lightness(skin.hovered()),
        lightness(skin.pressed()),
        lightness(skin.surface_raised),
    );
    assert!(
        rest < hover && hover < pressed,
        "dark: ink lightens the button, more for a press: {rest} {hover} {pressed}"
    );
    assert!(pressed - hover >= STATE_STEP);
    let light = GooeySkin::from_seed(passing()[2].1).expect("the light seed");
    assert!(
        lightness(light.pressed()) < lightness(light.hovered())
            && lightness(light.hovered()) < lightness(light.surface_raised),
        "light: ink darkens it"
    );
}

#[test]
fn a_warm_seed_keeps_its_hue_in_the_ladder() {
    let skin = GooeySkin::from_seed(passing()[3].1).expect("the warm seed");
    for step in &skin.surfaces {
        let c = Srgba::from(*step);
        assert!(
            c.red > c.blue,
            "a warm paper's steps stay warm, red over blue: {c:?}"
        );
    }
}

#[test]
fn a_grey_on_grey_seed_is_refused_naming_the_pair() {
    let problems = GooeySkin::from_seed(seed("#808080", "#9a9a9a", "#e5484d"))
        .expect_err("grey text on grey cannot be read");
    assert!(
        problems.iter().any(|problem| matches!(
            problem,
            SkinProblem::LowContrast {
                pair: "text on surface",
                ..
            }
        )),
        "the refusal names text on surface: {problems:?}"
    );
}

#[test]
fn a_translucent_seed_is_refused() {
    let mut translucent = passing()[0].1;
    translucent.text = translucent.text.with_alpha(0.5);
    assert_eq!(
        GooeySkin::from_seed(translucent),
        Err(vec![SkinProblem::TranslucentSeed { seed: "text" }])
    );
    let mut unknown = passing()[0].1;
    unknown.background = unknown.background.with_alpha(f32::NAN);
    assert_eq!(
        GooeySkin::from_seed(unknown),
        Err(vec![SkinProblem::TranslucentSeed { seed: "background" }]),
        "an alpha that is not a number is not opaque"
    );
}

/// A saturated green text on white tints the ladder out of sRGB, where the clamp flattens its
/// lightness: no step may come back toward the background, whatever the ladder does to get there.
#[test]
fn a_saturated_ladder_never_steps_back_toward_the_background() {
    match GooeySkin::from_seed(seed("#ffffff", "#004d00", "#1d4ed8")) {
        Ok(skin) => {
            let steps: Vec<f32> = skin.surfaces.iter().map(|s| lightness(*s)).collect();
            for pair in steps.windows(2) {
                assert!(
                    pair[0] - pair[1] >= ELEVATION_STEP,
                    "each step darker than the last by {ELEVATION_STEP} L*: {steps:?}"
                );
            }
        }
        Err(problems) => assert!(
            problems
                .iter()
                .any(|p| matches!(p, SkinProblem::ElevationStep { .. })),
            "a refusal for the ladder names its step: {problems:?}"
        ),
    }
}
