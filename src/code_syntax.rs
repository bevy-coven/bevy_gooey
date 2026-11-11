//! The skin's colours for code: one for each [`SyntaxRole`], and one for each [`Severity`]. They
//! are **derived from the skin's own seeds**, like [`GooeySkin::warning`] and [`GooeySkin::live`],
//! so a preset, a theme step and a host's own skin all get a set that belongs to it and nothing is
//! added to the skin's data.
//!
//! A role's colour is its hue at the accent's chroma and a lightness a little toward the ground
//! from the text's, moved back toward the text's until it holds the contrast bar on the editor's
//! own ground ([`GooeySkin::solid_surface`]): 7:1 when the skin's text already holds it (the high
//! contrast preset, and any preset that clears it), else 4.5:1. [`GooeySkin::problems`] checks
//! every one, so a skin that cannot give a set is reported.

use bevy::color::{Oklcha, Srgba};
use bevy::prelude::*;

use crate::code_decor::{Severity, SyntaxRole};
use crate::model::{ENHANCED_CONTRAST, TEXT_CONTRAST, contrast_ratio};
use crate::skin::GooeySkin;

/// How far toward the ground a role's lightness starts from the text's, of the gap.
const SYNTAX_L_SHIFT: f32 = 0.18;
/// How much of the way back to the text's lightness each step takes when a colour is too close to the ground.
const SYNTAX_L_BACK: f32 = 0.25;
/// The least and most chroma a role's colour takes.
const SYNTAX_CHROMA: (f32, f32) = (0.08, 0.16);
/// How much chroma is kept at each step of pulling a colour into sRGB.
const SYNTAX_GAMUT_PULL: f32 = 0.92;

/// A role's hue, degrees of Oklch. `None` is a role that takes a colour the skin already has.
fn hue(role: SyntaxRole) -> Option<f32> {
    match role {
        SyntaxRole::Key => Some(250.0),
        SyntaxRole::String => Some(145.0),
        SyntaxRole::Number => Some(55.0),
        SyntaxRole::Keyword => Some(320.0),
        SyntaxRole::Reference => Some(205.0),
        SyntaxRole::Function => Some(95.0),
        SyntaxRole::Type => Some(175.0),
        SyntaxRole::Comment | SyntaxRole::Operator | SyntaxRole::Error => None,
    }
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

fn in_gamut(color: Color) -> bool {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
        .iter()
        .all(|v| (0.0..=1.0).contains(v))
}

impl GooeySkin {
    /// The colour a span of `role` is drawn in, on the code editor's own ground. Derived from the
    /// skin's seeds (see the module docs); [`GooeySkin::problems`] holds every one at the text
    /// contrast bar.
    ///
    /// ```
    /// use bevy_gooey::{GooeySkin, SyntaxRole};
    /// let skin = GooeySkin::dark();
    /// assert_ne!(skin.syntax(SyntaxRole::Key), skin.syntax(SyntaxRole::String));
    /// assert_eq!(skin.syntax(SyntaxRole::Comment), skin.text_muted);
    /// assert!(skin.problems().is_empty());
    /// ```
    pub fn syntax(&self, role: SyntaxRole) -> Color {
        let Some(hue) = hue(role) else {
            return match role {
                SyntaxRole::Comment => self.text_muted,
                SyntaxRole::Error => self.danger,
                _ => self.text,
            };
        };
        let ground = self.solid_surface();
        let ground_rgb = rgb(ground);
        let needed = if contrast_ratio(rgb(self.text), ground_rgb) >= ENHANCED_CONTRAST {
            ENHANCED_CONTRAST
        } else {
            TEXT_CONTRAST
        };
        let text = Oklcha::from(self.text);
        let ground_l = Oklcha::from(ground).lightness;
        let chroma = Oklcha::from(self.accent)
            .chroma
            .clamp(SYNTAX_CHROMA.0, SYNTAX_CHROMA.1);
        let mut lightness = text.lightness + (ground_l - text.lightness) * SYNTAX_L_SHIFT;
        for _ in 0..24 {
            let mut lch = Oklcha::new(lightness, chroma, hue, 1.0);
            for _ in 0..24 {
                if in_gamut(Color::from(lch)) {
                    break;
                }
                lch.chroma *= SYNTAX_GAMUT_PULL;
            }
            let color = Color::from(lch);
            if contrast_ratio(rgb(color), ground_rgb) >= needed {
                return color;
            }
            lightness += (text.lightness - lightness) * SYNTAX_L_BACK;
        }
        // No hue at this chroma clears the bar before it reaches the text's own lightness: the
        // text's colour does, because the skin's own check says so.
        self.text
    }

    /// The colour of a [`Severity`]: the danger colour, the caution colour, the accent, and the
    /// muted text.
    pub fn severity(&self, severity: Severity) -> Color {
        match severity {
            Severity::Error => self.danger,
            Severity::Warning => self.warning(),
            Severity::Info => self.accent,
            Severity::Hint => self.text_muted,
        }
    }

    /// The pairs [`GooeySkin::problems`] checks for the code editor: every syntax colour on the
    /// editor's ground.
    pub(crate) fn syntax_pairs(&self) -> Vec<(&'static str, [f32; 3], [f32; 3])> {
        const LABELS: [&str; 10] = [
            "syntax key on surface",
            "syntax string on surface",
            "syntax number on surface",
            "syntax keyword on surface",
            "syntax comment on surface",
            "syntax reference on surface",
            "syntax function on surface",
            "syntax type on surface",
            "syntax operator on surface",
            "syntax error on surface",
        ];
        let ground = rgb(self.solid_surface());
        SyntaxRole::ALL
            .into_iter()
            .zip(LABELS)
            .map(|(role, label)| (label, rgb(self.syntax(role)), ground))
            .collect()
    }
}
