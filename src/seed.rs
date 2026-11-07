//! A skin from three seeds: a background, a text colour and an accent ([`GooeySkin::from_seed`]).
//!
//! The derivation is Rune's `Kit::from_palette` (`crates/rune-kit/src/palette.rs` at `f73f72c`,
//! MIT), measured from its source and written again here against this crate's own checks:
//!
//! - The ladder of surfaces steps Oklab lightness away from the background: lighter on a dark
//!   seed, darker on a light one. A dark seed steps from at least `#0a0a0a`'s lightness, so a
//!   pure black background still gets steps the eye can see.
//! - The derived neutrals keep the hue and chroma of the more colourful of the background and the
//!   text, so a warm palette stays warm.
//! - Secondary text mixes toward the text in Oklab, with a contrast floor.
//! - Hover and press are the text at a low alpha over the resting fill. The focus ring is a
//!   neutral blue, not the accent.
//!
//! Where this crate's checks ask for more than Rune's numbers give, the checks win:
//!
//! - Each step of the ladder starts no nearer the background than the one below and is pushed out
//!   until it is [`ELEVATION_STEP`] of CIE L\* past it; a ladder that still falls short (a
//!   saturated hue clamped at the edge of sRGB) is refused with `SkinProblem::ElevationStep`.
//! - The outline is solved for [`NON_TEXT_CONTRAST`] over the panel, where Rune uses a hairline
//!   alpha.
//! - The press is solved for [`STATE_STEP`] away from the hover.
//!
//! The result is checked by [`GooeySkin::problems`], and a seed whose skin fails is refused, never
//! adjusted. Research: `docs/research/skin-from-seeds.md`.

use bevy::color::{Oklaba, Oklcha, Srgba};
use bevy::prelude::*;

use crate::elevation::Elevation;
use crate::model::{
    ELEVATION_STEP, MIN_TARGET_PX, NON_TEXT_CONTRAST, PRIMARY_LIFT_L, STATE_STEP, TEXT_CONTRAST,
    cie_lightness, composite, contrast_ratio, lift_toward, relative_luminance,
};
use crate::motion::MotionTokens;
use crate::skin::Density;
use crate::skin::{GooeySkin, SkinProblem};

/// Three opaque colours a whole skin is derived from ([`GooeySkin::from_seed`]).
///
/// ```
/// use bevy::prelude::*;
/// use bevy_gooey::{GooeySkin, PaletteSeed};
/// let seed = PaletteSeed {
///     background: Srgba::hex("#0a0a0a").unwrap().into(),
///     text: Srgba::hex("#ededed").unwrap().into(),
///     accent: Srgba::hex("#e5484d").unwrap().into(),
/// };
/// let skin = GooeySkin::from_seed(seed).expect("a readable seed");
/// assert!(skin.problems().is_empty());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaletteSeed {
    /// Behind everything; the ladder of surfaces steps away from it.
    pub background: Color,
    /// Text, and the ink that hover, press, borders and secondary text are made of. Lighter than
    /// the background makes a dark skin.
    pub text: Color,
    /// The primary button's fill, a meter's fill and a toast's mark.
    pub accent: Color,
}

/// Oklab lightness steps of the four surfaces above the base, on a dark seed (Rune's
/// `DARK_LAYERS`).
const DARK_LAYERS_L: [f32; 4] = [0.04, 0.07, 0.10, 0.13];
/// The same on a light seed, stepping darker (Rune's `LIGHT_LAYERS`).
const LIGHT_LAYERS_L: [f32; 4] = [0.02, 0.045, 0.065, 0.10];
/// A dark seed's ladder starts at least at this Oklab lightness, about `#0a0a0a` (Rune's
/// `DARK_FLOOR`): steps from pure black would be invisible.
const DARK_FLOOR_L: f32 = 0.145;
/// How far a ladder step is pushed out each time it is still under [`ELEVATION_STEP`], Oklab L.
const LADDER_NUDGE_L: f32 = 0.002;
/// How far toward the text the muted text starts, before its contrast floor: dark, light (Rune's
/// `secondary` amounts for muted text).
const MUTED_TOWARD_TEXT: [f32; 2] = [0.72, 0.57];
/// The text's alpha over a resting button that is its hover fill: dark, light (Rune's `hover`).
const HOVER_INK: [f32; 2] = [0.06, 0.05];
/// The least alpha of the text over a resting button that is its pressed fill: dark, light (Rune's
/// `pressed`); raised until the press is [`STATE_STEP`] from the hover.
const PRESSED_INK: [f32; 2] = [0.10, 0.08];
/// The least alpha of the text over the panel that is the outline: dark, light (Rune's
/// `border_base`); raised until the outline reaches [`NON_TEXT_CONTRAST`].
const OUTLINE_INK: [f32; 2] = [0.11, 0.10];
/// The focus ring's Oklch lightness on a dark and a light seed, its chroma and its hue in degrees:
/// Rune's neutral blue.
const FOCUS_L: [f32; 2] = [0.70, 0.58];
const FOCUS_CHROMA: f32 = 0.15;
const FOCUS_HUE_DEG: f32 = 264.0;
/// The veil behind a dialog: black at this alpha.
const SCRIM_ALPHA: f32 = 0.55;
/// Bisection rounds when a colour is solved for a floor.
const SOLVE_ROUNDS: usize = 24;

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

fn from_rgb([red, green, blue]: [f32; 3]) -> Color {
    Color::srgb(red, green, blue)
}

/// The least `t` in `low..=1` for which `meets(t)` holds, by bisection, or `None` when even `1`
/// does not.
fn solve(low: f32, meets: impl Fn(f32) -> bool) -> Option<f32> {
    if meets(low) {
        return Some(low);
    }
    if !meets(1.0) {
        return None;
    }
    let (mut low, mut high) = (low, 1.0);
    for _ in 0..SOLVE_ROUNDS {
        let mid = (low + high) * 0.5;
        if meets(mid) {
            high = mid;
        } else {
            low = mid;
        }
    }
    Some(high)
}

impl GooeySkin {
    /// A whole skin derived from three colours: the surfaces, the borders, the secondary text, the
    /// hover and press fills and the focus ring. The derivation follows Rune's `Kit::from_palette`;
    /// the sizes, density and type are [`GooeySkin::tool`]'s, a desktop tool's.
    ///
    /// # Errors
    ///
    /// Every [`SkinProblem`] the derived skin has, when there is one: a seed whose text cannot be
    /// read on its own surfaces is refused, never adjusted. A translucent seed is
    /// [`SkinProblem::TranslucentSeed`].
    pub fn from_seed(seed: PaletteSeed) -> Result<GooeySkin, Vec<SkinProblem>> {
        let translucent: Vec<SkinProblem> = [
            ("background", seed.background),
            ("text", seed.text),
            ("accent", seed.accent),
        ]
        .into_iter()
        .filter(|(_, colour)| colour.alpha() != 1.0)
        .map(|(seed, _)| SkinProblem::TranslucentSeed { seed })
        .collect();
        if !translucent.is_empty() {
            return Err(translucent);
        }
        let (background, text, accent) = (rgb(seed.background), rgb(seed.text), rgb(seed.accent));
        let dark = relative_luminance(text) > relative_luminance(background);
        let side = usize::from(!dark);

        // The neutrals keep the hue of whichever seed is more colourful.
        let lab_bg = Oklaba::from(Srgba::from(seed.background));
        let lab_text = Oklaba::from(Srgba::from(seed.text));
        let chroma = |lab: Oklaba| lab.a.hypot(lab.b);
        let hue = if chroma(lab_bg) >= chroma(lab_text) {
            lab_bg
        } else {
            lab_text
        };
        let tone = |lightness: f32| {
            let lab = Oklaba::new(lightness.clamp(0.0, 1.0), hue.a, hue.b, 1.0);
            let s = Srgba::from(lab);
            from_rgb([
                s.red.clamp(0.0, 1.0),
                s.green.clamp(0.0, 1.0),
                s.blue.clamp(0.0, 1.0),
            ])
        };
        let base = if dark {
            lab_bg.lightness.max(DARK_FLOOR_L)
        } else {
            lab_bg.lightness
        };
        let away = if dark { 1.0 } else { -1.0 };
        let layers = if dark { DARK_LAYERS_L } else { LIGHT_LAYERS_L };

        // The ladder: the base, then each step at Rune's offset or pushed out until the eye sees it.
        // A step never starts behind the one below (an earlier step may have been pushed past its
        // offset), and it is pushed on until it is the step away from it, signed: a step that
        // went back toward the background would pass an unsigned check.
        let mut surfaces = vec![tone(base)];
        let mut last_l = base;
        for offset in layers {
            let below = cie_lightness(rgb(surfaces[surfaces.len() - 1]));
            let start = base + away * offset;
            let mut l = if away * (start - last_l) > 0.0 {
                start
            } else {
                last_l + away * LADDER_NUDGE_L
            };
            let mut step = tone(l);
            while away * (cie_lightness(rgb(step)) - below) < ELEVATION_STEP
                && (0.0..=1.0).contains(&l)
            {
                l += away * LADDER_NUDGE_L;
                step = tone(l);
            }
            last_l = l;
            surfaces.push(step);
        }
        let mut problems: Vec<SkinProblem> = surfaces
            .windows(2)
            .enumerate()
            .filter_map(|(lower, pair)| {
                let delta_l = away * (cie_lightness(rgb(pair[1])) - cie_lightness(rgb(pair[0])));
                (delta_l.is_nan() || delta_l < ELEVATION_STEP).then_some(
                    SkinProblem::ElevationStep {
                        lower,
                        upper: lower + 1,
                        delta_l,
                    },
                )
            })
            .collect();
        let surface = surfaces[1];
        let surface_raised = surfaces[3];
        let (on_surface, on_raised) = (rgb(surface), rgb(surface_raised));

        // Muted text: toward the text in lightness at the neutrals' hue, held to 4.5:1 on the panel.
        let lab_surface = Oklaba::from(Srgba::from(surface));
        let muted_at = |t: f32| {
            let lab = Oklaba::new(
                lab_surface.lightness + (lab_text.lightness - lab_surface.lightness) * t,
                hue.a,
                hue.b,
                1.0,
            );
            let s = Srgba::from(lab);
            [
                s.red.clamp(0.0, 1.0),
                s.green.clamp(0.0, 1.0),
                s.blue.clamp(0.0, 1.0),
            ]
        };
        let text_muted = solve(MUTED_TOWARD_TEXT[side], |t| {
            contrast_ratio(muted_at(t), on_surface) >= TEXT_CONTRAST
        })
        .map_or(text, muted_at);

        // Ink: the text at an alpha over a fill.
        let ink = |alpha: f32, over: [f32; 3]| composite(text, alpha, over);
        let outline = solve(OUTLINE_INK[side], |a| {
            contrast_ratio(ink(a, on_surface), on_surface) >= NON_TEXT_CONTRAST
        })
        .map_or(text, |a| ink(a, on_surface));
        let hover = ink(HOVER_INK[side], on_raised);
        let hover_l = cie_lightness(hover);
        let pressed = solve(PRESSED_INK[side], |a| {
            (cie_lightness(ink(a, on_raised)) - hover_l).abs() >= STATE_STEP
        })
        .map_or(text, |a| ink(a, on_raised));

        // The accent's words: black or white, whichever stays more readable on the accent and on
        // the two fills a primary button lifts it to under the pointer and held down.
        let lifted = [
            accent,
            lift_toward(accent, text, PRIMARY_LIFT_L),
            lift_toward(accent, text, PRIMARY_LIFT_L * 2.0),
        ];
        let worst = |ink: [f32; 3]| {
            lifted
                .iter()
                .map(|fill| contrast_ratio(ink, *fill))
                .fold(f32::INFINITY, f32::min)
        };
        let accent_text = if worst([0.0; 3]) >= worst([1.0; 3]) {
            Color::BLACK
        } else {
            Color::WHITE
        };
        // The focus ring: Rune's blue, moved away from the ladder (lighter on a dark seed, darker on
        // a light one) until it is 3:1 on the background and on every step.
        let ring_at = |t: f32| {
            let start = FOCUS_L[side];
            let end = if dark { 1.0 } else { 0.0 };
            let c = Srgba::from(Oklcha::new(
                start + (end - start) * t,
                FOCUS_CHROMA,
                FOCUS_HUE_DEG,
                1.0,
            ));
            [
                c.red.clamp(0.0, 1.0),
                c.green.clamp(0.0, 1.0),
                c.blue.clamp(0.0, 1.0),
            ]
        };
        let grounds: Vec<[f32; 3]> = std::iter::once(background)
            .chain(surfaces.iter().map(|s| rgb(*s)))
            .collect();
        let ring_meets =
            |ring: [f32; 3], ground: &[f32; 3]| contrast_ratio(ring, *ground) >= NON_TEXT_CONTRAST;
        let focus_ring = match solve(0.0, |t| grounds.iter().all(|g| ring_meets(ring_at(t), g))) {
            Some(t) => ring_at(t),
            None => {
                // No blue reaches every ground: say which, as `problems()` checks only two.
                let ring = ring_at(0.0);
                for (i, ground) in grounds.iter().enumerate() {
                    if !ring_meets(ring, ground) {
                        problems.push(SkinProblem::LowContrast {
                            pair: if i == 0 {
                                "focus_ring on background"
                            } else {
                                "focus_ring on a surface step"
                            },
                            ratio: contrast_ratio(ring, *ground),
                            needed: NON_TEXT_CONTRAST,
                        });
                    }
                }
                ring
            }
        };
        let focus_ring = from_rgb(focus_ring);
        // The tool preset's danger on a dark seed; parchment's on a light one, where the tool's is
        // too light for a warm paper (2.5:1).
        let danger = if dark {
            GooeySkin::tool().danger
        } else {
            GooeySkin::parchment().danger
        };

        let skin = GooeySkin {
            background: seed.background,
            surface,
            surface_raised,
            text: seed.text,
            text_muted: from_rgb(text_muted),
            accent: seed.accent,
            accent_text,
            danger,
            focus_ring,
            outline: from_rgb(outline),
            surface_edge: None,
            scrim: Color::srgba(0.0, 0.0, 0.0, SCRIM_ALPHA),
            surfaces,
            elevation: if dark {
                Elevation::on_dark()
            } else {
                Elevation::on_light()
            },
            hover: Some(from_rgb(hover)),
            pressed: Some(from_rgb(pressed)),
            ..GooeySkin::tool()
        };
        problems.extend(skin.problems());
        if problems.is_empty() {
            Ok(skin)
        } else {
            Err(problems)
        }
    }
}

/// The Oklch lightness, chroma and hue (degrees) of Rune's neutral themes and its blue accent
/// (`crates/rune/src/tok.rs` `theme()` and `accent(0, ..)` at `f73f72c`): background, text and
/// accent, dark then light.
const DESK_DARK_LCH: [[f32; 3]; 3] = [
    [0.185, 0.004, 265.0],
    [0.955, 0.003, 265.0],
    [0.66, 0.17, 256.0],
];
/// Rune's light accent is `oklch(0.6 0.17 256)`; words on it reach 4.0:1 and it stands 2.9:1 on a
/// resting button, under this crate's 4.5:1 and 3:1. 0.55 is the lightest step of 0.05 that meets
/// both, so the light Desk's accent is that much deeper.
const DESK_LIGHT_LCH: [[f32; 3]; 3] = [
    [0.958, 0.004, 265.0],
    [0.225, 0.006, 265.0],
    [0.55, 0.17, 256.0],
];

/// An Oklch colour held as sRGB, the space every preset is written in (`Color::mix` mixes in a
/// colour's own space, so the space a seed is held in reaches the fills derived from it).
fn lch([l, c, h]: [f32; 3]) -> Color {
    Color::Srgba(Srgba::from(Oklcha::new(l, c, h, 1.0)))
}

impl PaletteSeed {
    /// Rune's neutral dark desktop theme: a blue-grey ground, near-white text, its blue accent.
    /// [`GooeySkin::desk`] is this seed's skin.
    pub fn desk() -> Self {
        let [background, text, accent] = DESK_DARK_LCH.map(lch);
        Self {
            background,
            text,
            accent,
        }
    }

    /// Rune's neutral light desktop theme, with a deeper blue accent (its own fails 4.5:1).
    /// [`GooeySkin::desk_light`] is this seed's skin.
    pub fn desk_light() -> Self {
        let [background, text, accent] = DESK_LIGHT_LCH.map(lch);
        Self {
            background,
            text,
            accent,
        }
    }
}

impl GooeySkin {
    /// A desktop tool's neutral dark look (Rune's): the colours [`GooeySkin::from_seed`] derives from
    /// [`PaletteSeed::desk`], written out here so the preset is plain data; Rune's sizes, compact
    /// density, a 24 px least target (WCAG 2.5.8), Rune's dark elevation and motion palette.
    /// Not a step of the theme row ([`SkinPreset::ALL`](crate::SkinPreset::ALL) is the row's four
    /// looks): a host that wants it sets it.
    ///
    /// ```
    /// use bevy_gooey::GooeySkin;
    /// assert!(GooeySkin::desk().problems().is_empty());
    /// assert_eq!(GooeySkin::desk().text_size, 13.0);
    /// ```
    pub fn desk() -> Self {
        Self {
            background: Color::srgb(0.0697, 0.0732, 0.0804),
            surface: Color::srgb(0.1050, 0.1087, 0.1163),
            surface_raised: Color::srgb(0.1610, 0.1649, 0.1730),
            text: Color::srgb(0.9372, 0.9412, 0.9494),
            text_muted: Color::srgb(0.6774, 0.6824, 0.6927),
            accent: Color::srgb(0.2617, 0.5721, 0.9702),
            accent_text: Color::BLACK,
            danger: Color::srgb(0.93, 0.36, 0.34),
            focus_ring: Color::srgb(0.4313, 0.6076, 0.9850),
            outline: Color::srgb(0.3983, 0.4021, 0.4099),
            hover: Some(Color::srgb(0.2076, 0.2115, 0.2195)),
            pressed: Some(Color::srgb(0.2425, 0.2465, 0.2545)),
            surfaces: vec![
                Color::srgb(0.0697, 0.0732, 0.0804),
                Color::srgb(0.1050, 0.1087, 0.1163),
                Color::srgb(0.1326, 0.1364, 0.1442),
                Color::srgb(0.1610, 0.1649, 0.1730),
                Color::srgb(0.1902, 0.1942, 0.2024),
            ],
            elevation: Elevation::on_dark(),
            ..Self::desk_sizes()
        }
    }

    /// The light Desk: [`PaletteSeed::desk_light`]'s colours, the same sizes as
    /// [`GooeySkin::desk`] and Rune's light elevation.
    ///
    /// ```
    /// use bevy_gooey::GooeySkin;
    /// assert!(GooeySkin::desk_light().problems().is_empty());
    /// ```
    pub fn desk_light() -> Self {
        Self {
            background: Color::srgb(0.9399, 0.9452, 0.9562),
            surface: Color::srgb(0.9114, 0.9194, 0.9356),
            surface_raised: Color::srgb(0.8534, 0.8613, 0.8774),
            text: Color::srgb(0.1032, 0.1087, 0.1201),
            text_muted: Color::srgb(0.4097, 0.4166, 0.4307),
            accent: Color::srgb(0.1105, 0.4364, 0.8223),
            accent_text: Color::WHITE,
            danger: Color::srgb(0.70, 0.12, 0.10),
            focus_ring: Color::srgb(0.2817, 0.4448, 0.8070),
            outline: Color::srgb(0.5225, 0.5293, 0.5432),
            hover: Some(Color::srgb(0.8159, 0.8237, 0.8395)),
            pressed: Some(Color::srgb(0.7724, 0.7800, 0.7956)),
            surfaces: vec![
                Color::srgb(0.9374, 0.9454, 0.9618),
                Color::srgb(0.9114, 0.9194, 0.9356),
                Color::srgb(0.8791, 0.8870, 0.9032),
                Color::srgb(0.8534, 0.8613, 0.8774),
                Color::srgb(0.8089, 0.8166, 0.8326),
            ],
            elevation: Elevation::on_light(),
            ..Self::desk_sizes()
        }
    }

    /// Rune's desktop sizes (`tok.rs` `type_`, `space`, `radius`): 13 px body text, a 4/8/12/24
    /// spacing scale, 6/8/12 corners, a 1 px frame, compact density at a 24 px least target.
    fn desk_sizes() -> Self {
        Self {
            scrim: Color::srgba(0.0, 0.0, 0.0, SCRIM_ALPHA),
            surface_edge: None,
            density: Density::Compact,
            text_size: 13.0,
            heading_size: 15.0,
            type_scale: [11.5, 12.0, 13.0, 15.0, 20.0],
            spacing: [2.0, 4.0, 8.0, 12.0, 16.0, 24.0],
            radii: [6.0, 8.0, 12.0],
            radius: 6.0,
            border: 1.0,
            gap: 8.0,
            padding: 12.0,
            min_target: MIN_TARGET_PX,
            motion: MotionTokens::default(),
            ..GooeySkin::tool()
        }
    }
}
