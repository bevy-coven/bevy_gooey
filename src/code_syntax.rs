//! The skin's colours for code: one for each [`SyntaxRole`], and one for each [`CodeSeverity`]. They
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

use crate::code_decor::{CodeSeverity, SyntaxRole};
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

/// A role's hue, degrees of Oklch.
fn hue(role: SyntaxRole) -> f32 {
    match role {
        SyntaxRole::Key => 250.0,
        SyntaxRole::String => 145.0,
        SyntaxRole::Number => 55.0,
        SyntaxRole::Keyword => 320.0,
        SyntaxRole::Reference => 205.0,
        SyntaxRole::Function => 95.0,
        SyntaxRole::Type => 175.0,
        // Roles that take a colour the skin already has, or the danger hue, never ask.
        SyntaxRole::Comment | SyntaxRole::Operator | SyntaxRole::Error => 0.0,
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
        let (hue, base_chroma) = match role {
            SyntaxRole::Comment => return self.text_muted,
            SyntaxRole::Operator => return self.text,
            // Wrong is the danger colour's own hue, held to the same bar as the rest.
            SyntaxRole::Error => {
                let danger = Oklcha::from(self.danger);
                (danger.hue, danger.chroma)
            }
            other => (hue(other), Oklcha::from(self.accent).chroma),
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
        let chroma = base_chroma.clamp(SYNTAX_CHROMA.0, SYNTAX_CHROMA.1);
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

    /// The colour of a [`CodeSeverity`]: the danger colour, the caution colour, the accent, and the
    /// muted text.
    pub fn severity(&self, severity: CodeSeverity) -> Color {
        match severity {
            CodeSeverity::Error => self.danger,
            CodeSeverity::Warning => self.warning(),
            CodeSeverity::Info => self.accent,
            CodeSeverity::Hint => self.text_muted,
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
