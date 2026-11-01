//! The skin: the look of every widget, as data, and the one system that applies it.

use std::fmt;

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{FontSize, FontSource, TextCursorStyle};
use bevy::ui::{Checked, ComputedUiRenderTargetInfo, InteractionDisabled, Pressed, Selected};
use bevy::ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide};
use bevy::ui_widgets::{ScrollbarDragState, SliderDragState};

use crate::binding::Capturing;
use crate::choice::{ChoiceCount, ChoiceValue};
use crate::list::ListView;
use crate::menu::{MenuMarked, MenuOpen};
use crate::model::{
    COMFORT_TARGET_PX, ELEVATION_STEP, ENHANCED_CONTRAST, MIN_TARGET_PX, NON_TEXT_CONTRAST,
    TEXT_CONTRAST, cie_lightness, composite, contrast_ratio, list_height, snap_line_width,
};
use crate::popover::{PopoverPanel, placement_of};
use crate::toast::{Toast, ToastKind};
use crate::tooltip::TooltipBox;

/// A meter's least length, in body text sizes: 8 × 20 px = 160 px at the default size, growing
/// with [`GooeySkin::text_scale`].
const METER_MIN_TEXT_SIZES: f32 = 8.0;

/// A slider's least width, in body text sizes: 10 × 20 px = 200 px at the default size.
const SLIDER_MIN_TEXT_SIZES: f32 = 10.0;

/// The least width of a choice's value cell (its option, the air round it and nothing of the
/// arrows), in body text sizes: 8 × 20 px = 160 px at the default size, which holds "PlayStation"
/// and "Fullscreen" in the host's font with room, so choices stacked in a panel have their arrows
/// and values aligned instead of each at its widest option's width.
const CHOICE_MIN_TEXT_SIZES: f32 = 8.0;

/// A toggle's track is this many body text sizes high and twice as wide.
const SWITCH_TEXT_SIZES: f32 = 1.5;

/// A slider's thumb is this many body text sizes wide, and this many rails high.
const THUMB_TEXT_SIZES: f32 = 1.0;

/// A list's row is a button: at least `min_target` high, or its text's line (this many text sizes),
/// its vertical padding (this many paddings, `0.6` each side) and its frame, whichever is more.
const LIST_LINE_TEXT_SIZES: f32 = 1.2;
const LIST_PADDING_FACTOR: f32 = 1.2;

/// A list's rail is this many line weights wide, and its thumb is as wide.
const RAIL_LINES: f32 = 2.0;

/// A slider's thumb stands this many rails tall, a little over the rail on each side.
const THUMB_RAIL_RATIO: f32 = 1.25;

/// A binding's keycap is at least this many body text sizes wide (17 × 20 px = 340 px at the
/// default size), so every row of a controls screen has its keycap's edges in one column, and the
/// prompt that replaces the name ("Press a button, Menu exits", about 26 characters) fits without
/// the row ever changing size. It is also as wide as a toggle's row leaves a four-letter label.
const BINDING_CAP_TEXT_SIZES: f32 = 17.0;

/// A command palette's card is this many body text sizes wide (800 px at the default size), or the
/// scrim's inside when the window is narrower.
const PALETTE_TEXT_SIZES: f32 = 40.0;

/// How far below the scrim's safe edge a palette's card starts, as a share of the window's
/// height: high enough to be where the player's eyes are on a command line, low enough to clear a
/// menu bar (with the 5% the scrim pads, the card's top is about a seventh of the way down).
const PALETTE_TOP_VH: f32 = 5.0;

/// A menu's card is at least this many body text sizes wide (10 × 20 px = 200 px at the default
/// size): a short menu is still a card, and a title's menu is never narrower than its words.
const MENU_MIN_TEXT_SIZES: f32 = 10.0;

/// The air between a menu item's label and its chord, beside the row's gap, in body text sizes:
/// at 200% text it grows with the words (a gap alone left "Open..." nearly touching "Ctrl+O").
const MENU_CHORD_TEXT_SIZES: f32 = 0.5;

/// The slot a menu item keeps for its check or dot, in body text sizes, in every item so labels
/// share one edge whether or not a menu has a check in it.
const MENU_MARK_TEXT_SIZES: f32 = 1.1;

/// A toast's longest line, in body text sizes: the tooltip's measure, about 40 characters.
const TOAST_MEASURE_TEXT_SIZES: f32 = 24.0;

/// An information toast's mark is this many lines wide, and an alert's twice that: the kind is the
/// mark's width first and its colour second, as a switch's state is its knob's place.
const TOAST_MARK_LINES: f32 = 4.0;

/// A choice's chevron is a square this many body text sizes on a side, turned an eighth of a turn.
const CHEVRON_TEXT_SIZES: f32 = 0.6;

/// A choice's arrow is at least this many chevrons wide: the turned square's half plus the shift
/// that centres its ink (`1 / √2` of its size), each side of the arrow's middle. A compact skin's
/// 24 px arrow holds a 200% chevron only because of this.
const CHEVRON_ARROW_FIT: f32 = 1.0 + std::f32::consts::FRAC_1_SQRT_2;

/// A dialog's panel is at least this many body text sizes wide (21 × 20 px = 420 px: a platform
/// dialog's 320 epx at this crate's 4/3) and at most this many (36 × 20 px = 720 px: its 548 epx).
/// A short title fills the least; a long body wraps at the most.
const DIALOG_MIN_TEXT_SIZES: f32 = 21.0;
const DIALOG_MAX_TEXT_SIZES: f32 = 36.0;

/// A dialog's panel has this many times a screen panel's padding and gap, so its title, body and
/// choices are read in order and no word stands nearer an edge than the next line.
const DIALOG_ROOM_FACTOR: f32 = 1.5;
/// How much of the accent the landing of a carried tab lets over the panels it covers.
const DOCK_DROP_ALPHA: f32 = 0.3;

/// The share of the window, on each side, that a dialog's scrim and a prompts legend keep clear: a
/// TV may not show the outer 5%, so nothing a player must read or press stands there.
pub(crate) const SAFE_EDGE_PERCENT: f32 = 5.0;

/// A prompts legend's prompts stand this many paddings from the next, and a prompt's own cap and
/// words a gap apart, so each pair reads as one thing.
const PROMPT_SPACING_PADDINGS: f32 = 2.0;

/// A prompts legend is no wider than this share of its parent (its 5% margin and the same on the
/// right): a longer legend squeezes its words, and never leaves the window.
const PROMPTS_MAX_WIDTH_PERCENT: f32 = 100.0 - 2.0 * SAFE_EDGE_PERCENT;

/// How much room a skin gives each widget: a game menu's comfortable one, or a desktop tool's
/// compact one ([`GooeySkin::density`]).
///
/// Density changes two numbers every widget reads, the padding ([`GooeySkin::pad`]) and the least
/// target ([`GooeySkin::target`]), and nothing else: colours, the focus ring and the keys are the
/// same.
///
/// ```
/// use bevy_gooey::{Density, GooeySkin};
/// let tool = GooeySkin { density: Density::Compact, ..GooeySkin::dark() };
/// assert_eq!(tool.target(), 24.0);
/// assert_eq!(tool.pad(), GooeySkin::dark().padding / 2.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default, Debug, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Density {
    /// The skin's own padding and least target: sized for a thumb or a pad at couch distance.
    #[default]
    Comfortable,
    /// Half the padding and a least target of [`MIN_TARGET_PX`] (24 px, WCAG 2.2 SC 2.5.8): rows
    /// close enough for a mouse at arm's length, a tool's inspector or a long list.
    Compact,
}

/// The share of [`GooeySkin::padding`] a [`Density::Compact`] skin draws.
const COMPACT_PADDING: f32 = 0.5;

/// The look of every widget: colours and sizes, as plain data a project saves and an editor edits
/// live.
///
/// Insert a new value (or mutate the resource) and [`apply_skin`] restyles everything already on
/// screen in the same frame. With the `serde` feature a skin is (de)serialisable, and a partial file
/// fills every missing field from [`GooeySkin::default`]. Fonts are not here: they are asset
/// handles the host loads, in [`GooeyFonts`].
///
/// [`GooeySkin::problems`] checks a skin against the WCAG 2.2 contrast and target-size rules; a
/// skin that fails them is still applied, with a warning per problem.
///
/// ```
/// use bevy_gooey::GooeySkin;
/// let large = GooeySkin { text_scale: 1.5, ..GooeySkin::parchment() };
/// assert!(large.problems().is_empty());
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct GooeySkin {
    /// Behind everything: a host's clear colour, and what a translucent [`GooeySkin::surface`] is
    /// checked over.
    pub background: Color,
    /// A panel's fill. It may be translucent; its contrast is checked over [`GooeySkin::background`].
    pub surface: Color,
    /// A button at rest, a meter's track inside its frame, a tooltip and a panel's border.
    pub surface_raised: Color,
    /// Headings, labels and button text.
    pub text: Color,
    /// The text of a disabled control, and the knob of a switch that is off; the knob is checked
    /// against [`GooeySkin::surface_raised`], the switch's track, at the 3:1 non-text minimum.
    pub text_muted: Color,
    /// A pressed button, a meter's fill and a toast's mark; checked against
    /// [`GooeySkin::surface_raised`], the meter's track, and against the solid surface a toast
    /// stands on, at the 3:1 non-text minimum.
    pub accent: Color,
    /// Text drawn on [`GooeySkin::accent`].
    pub accent_text: Color,
    /// A colour for destructive actions, and the mark of an alert toast ([`crate::toast_alert`]);
    /// checked against the solid surface a toast stands on at the 3:1 non-text minimum.
    pub danger: Color,
    /// The border of the focused button while focus is visible.
    pub focus_ring: Color,
    /// The edge of a widget whose fill does not show its size, so its whole extent stays visible:
    /// a meter's frame. Checked against the panel at the 3:1 non-text minimum.
    pub outline: Color,
    /// The border of a panel, a tooltip and a prompts legend. `None` draws it in
    /// [`GooeySkin::surface_raised`], the quiet frame the dark and parchment presets have; the
    /// high-contrast preset sets white, so a tooltip over a panel is a card of its own. Checked
    /// against [`GooeySkin::surface`] at the 3:1 non-text minimum when set.
    pub surface_edge: Option<Color>,
    /// The veil behind a modal dialog.
    pub scrim: Color,
    /// Body text size, logical px, before [`GooeySkin::text_scale`].
    pub text_size: f32,
    /// Heading text size, logical px, before [`GooeySkin::text_scale`].
    pub heading_size: f32,
    /// The space between a panel's or a menu's children, logical px.
    pub gap: f32,
    /// A panel's inner padding, logical px; buttons and tooltips derive theirs from it.
    pub padding: f32,
    /// The corner radius of panels, buttons, meters and tooltips, logical px.
    pub radius: f32,
    /// The border width of panels and buttons, logical px. A button's focus ring is drawn in it,
    /// and a meter's track is framed this wide, with its fill this far inside the frame. Drawn as a
    /// whole number of device pixels at any scale, rounded down (at least one), so a line is the
    /// same weight on every side: [`snap_line_width`].
    pub border: f32,
    /// A meter's height, frame included, logical px, before [`GooeySkin::text_scale`]: the HUD
    /// grows with the player's size option, not only its text. It must leave room for a fill
    /// inside the frame and the gap (more than 4 × [`GooeySkin::border`]).
    pub meter_height: f32,
    /// The least width and height of a button, logical px.
    pub min_target: f32,
    /// How much room each widget gets: [`Density::Comfortable`] reads [`GooeySkin::padding`] and
    /// [`GooeySkin::min_target`] as they are, [`Density::Compact`] draws half the padding and
    /// caps the least target at [`MIN_TARGET_PX`]. See [`GooeySkin::pad`] and [`GooeySkin::target`].
    pub density: Density,
    /// A player option: every text size is multiplied by it.
    pub text_scale: f32,
    /// The spacing scale, logical px, smallest first: the gaps and paddings a host's own chrome is
    /// built from. Read it with [`GooeySkin::space`]. Each preset's [`GooeySkin::gap`] and
    /// [`GooeySkin::padding`] are on its own. Finite, at least zero, never falling
    /// ([`SkinProblem::BadScale`]).
    pub spacing: [f32; 6],
    /// The corner-radius scale, logical px, smallest first. Read it with [`GooeySkin::corner`]; each
    /// preset's [`GooeySkin::radius`] is on its own. (Named `radii` because `radius` is the corner
    /// every widget draws.)
    pub radii: [f32; 3],
    /// The type scale, logical px before [`GooeySkin::text_scale`], smallest first. Read it with
    /// [`GooeySkin::type_size`]; each preset's [`GooeySkin::text_size`] and
    /// [`GooeySkin::heading_size`] are on its own. Finite, above zero, never falling.
    pub type_scale: [f32; 5],
    /// The ladder of flat fills a tool's grounds stand on, lowest first (the window's ground, a
    /// panel, a row, a field, a card): [`Role::Surface`] draws step `n`, read with
    /// [`GooeySkin::surface_at`]. Opaque: alpha is ignored. Every adjacent pair must differ by at
    /// least [`ELEVATION_STEP`] in CIE L\* ([`SkinProblem::ElevationStep`]); a ladder of one is
    /// valid, an empty one is not.
    pub surfaces: Vec<Color>,
}

impl GooeySkin {
    /// A dark skin: near-black surfaces, light text, an amber accent.
    pub fn dark() -> Self {
        Self {
            background: Color::srgb(0.07, 0.08, 0.10),
            surface: Color::srgba(0.12, 0.13, 0.16, 0.92),
            surface_raised: Color::srgb(0.20, 0.22, 0.27),
            text: Color::srgb(0.93, 0.94, 0.96),
            text_muted: Color::srgb(0.62, 0.65, 0.71),
            accent: Color::srgb(0.95, 0.68, 0.20),
            accent_text: Color::srgb(0.10, 0.08, 0.05),
            danger: Color::srgb(0.86, 0.27, 0.24),
            focus_ring: Color::srgb(0.45, 0.78, 1.0),
            outline: Color::srgb(0.44, 0.47, 0.54),
            scrim: Color::srgba(0.0, 0.0, 0.0, 0.6),
            surfaces: vec![
                Color::srgb(0.07, 0.08, 0.10),
                Color::srgb(0.12, 0.13, 0.16),
                Color::srgb(0.16, 0.175, 0.215),
                Color::srgb(0.20, 0.22, 0.27),
                Color::srgb(0.273, 0.292, 0.339),
            ],
            ..Self::sizes()
        }
    }

    /// A light skin: cream panels, dark brown text, a dark red accent.
    pub fn parchment() -> Self {
        Self {
            background: Color::srgb(0.18, 0.13, 0.09),
            surface: Color::srgba(0.91, 0.85, 0.72, 0.95),
            surface_raised: Color::srgb(0.83, 0.74, 0.58),
            text: Color::srgb(0.20, 0.14, 0.09),
            text_muted: Color::srgb(0.42, 0.33, 0.24),
            accent: Color::srgb(0.55, 0.16, 0.12),
            accent_text: Color::srgb(0.97, 0.93, 0.85),
            danger: Color::srgb(0.70, 0.12, 0.10),
            focus_ring: Color::srgb(0.12, 0.38, 0.55),
            outline: Color::srgb(0.48, 0.38, 0.27),
            scrim: Color::srgba(0.08, 0.05, 0.02, 0.7),
            surfaces: vec![
                Color::srgb(0.18, 0.13, 0.09),
                Color::srgb(0.667, 0.588, 0.458),
                Color::srgb(0.83, 0.74, 0.58),
                Color::srgb(0.91, 0.85, 0.72),
                Color::srgb(0.955, 0.925, 0.86),
            ],
            ..Self::sizes()
        }
    }

    /// A high-contrast skin, for the player who needs more than WCAG's minimum: opaque black
    /// surfaces, white text and white outlines, an amber accent, a cyan focus ring and a light red
    /// for danger (red on black is the one pairing WCAG advises against, for protanopia).
    ///
    /// Every pair [`GooeySkin::problems`] checks is at least [`ENHANCED_CONTRAST`], 7:1, which is what
    /// Xbox Accessibility Guideline 102 asks of a game's high-contrast mode; [`GooeySkin::high_contrast_problems`]
    /// checks it. The sizes are the other presets', so a layout never moves when the look changes.
    pub fn high_contrast() -> Self {
        Self {
            background: Color::BLACK,
            surface: Color::srgb(0.02, 0.02, 0.02),
            surface_raised: Color::srgb(0.133, 0.133, 0.133),
            text: Color::WHITE,
            text_muted: Color::srgb(0.72, 0.72, 0.72),
            accent: Color::srgb(1.0, 0.66, 0.0),
            accent_text: Color::BLACK,
            danger: Color::srgb(1.0, 0.5, 0.5),
            focus_ring: Color::srgb(0.2, 0.9, 1.0),
            outline: Color::WHITE,
            surface_edge: Some(Color::WHITE),
            scrim: Color::srgba(0.0, 0.0, 0.0, 0.8),
            surfaces: vec![
                Color::srgb(0.0, 0.0, 0.0),
                Color::srgb(0.092, 0.092, 0.092),
                Color::srgb(0.156, 0.156, 0.156),
                Color::srgb(0.241, 0.241, 0.241),
                Color::srgb(0.351, 0.351, 0.351),
            ],
            ..Self::sizes()
        }
    }

    /// A skin for a desktop tool, [`SkinPreset::Tool`]: a cool neutral dark with a blue accent,
    /// [`Density::Compact`] and sizes a mouse at arm's length reads: 14 px text, a 24 px least
    /// target, half the padding. Passes [`GooeySkin::problems`] (the target floor included).
    pub fn tool() -> Self {
        Self {
            background: Color::srgb(0.07, 0.075, 0.085),
            surface: Color::srgb(0.11, 0.115, 0.13),
            surface_raised: Color::srgb(0.19, 0.20, 0.23),
            text: Color::srgb(0.90, 0.91, 0.93),
            text_muted: Color::srgb(0.62, 0.64, 0.68),
            accent: Color::srgb(0.30, 0.60, 0.98),
            accent_text: Color::srgb(0.04, 0.05, 0.08),
            danger: Color::srgb(0.93, 0.36, 0.34),
            focus_ring: Color::srgb(0.55, 0.80, 1.0),
            outline: Color::srgb(0.45, 0.48, 0.54),
            scrim: Color::srgba(0.0, 0.0, 0.0, 0.55),
            text_size: 14.0,
            heading_size: 20.0,
            gap: 6.0,
            padding: 10.0,
            radius: 4.0,
            border: 2.0,
            meter_height: 10.0,
            min_target: MIN_TARGET_PX,
            density: Density::Compact,
            surfaces: vec![
                Color::srgb(0.07, 0.075, 0.085),
                Color::srgb(0.11, 0.115, 0.13),
                Color::srgb(0.15, 0.158, 0.18),
                Color::srgb(0.19, 0.20, 0.23),
                Color::srgb(0.261, 0.271, 0.30),
            ],
            spacing: [2.0, 4.0, 6.0, 10.0, 16.0, 24.0],
            radii: [2.0, 4.0, 8.0],
            type_scale: [11.0, 12.0, 14.0, 16.0, 20.0],
            ..Self::sizes()
        }
    }

    /// The sizes both presets share; the colours are overwritten by each preset.
    fn sizes() -> Self {
        Self {
            background: Color::BLACK,
            surface: Color::BLACK,
            surface_raised: Color::BLACK,
            text: Color::WHITE,
            text_muted: Color::WHITE,
            accent: Color::WHITE,
            accent_text: Color::BLACK,
            danger: Color::WHITE,
            focus_ring: Color::WHITE,
            outline: Color::WHITE,
            surface_edge: None,
            scrim: Color::BLACK,
            text_size: 20.0,
            heading_size: 32.0,
            gap: 10.0,
            padding: 14.0,
            radius: 8.0,
            border: 3.0,
            meter_height: 24.0,
            min_target: COMFORT_TARGET_PX,
            density: Density::Comfortable,
            text_scale: 1.0,
            spacing: [4.0, 8.0, 10.0, 14.0, 20.0, 28.0],
            radii: [4.0, 8.0, 16.0],
            type_scale: [14.0, 16.0, 20.0, 26.0, 32.0],
            surfaces: vec![Color::BLACK],
        }
    }

    /// Step `step` of the [`GooeySkin::spacing`] scale, logical px; a step past the end is the
    /// last. Not scaled by [`GooeySkin::density`] or [`GooeySkin::text_scale`]: a skin that wants
    /// tighter chrome has tighter tokens, as [`GooeySkin::tool`] does.
    ///
    /// ```
    /// use bevy_gooey::GooeySkin;
    /// let skin = GooeySkin::dark();
    /// assert_eq!(skin.space(0), 4.0);
    /// assert_eq!(skin.space(99), skin.spacing[5]);
    /// ```
    pub fn space(&self, step: usize) -> f32 {
        self.spacing[step.min(self.spacing.len() - 1)]
    }

    /// Step `step` of the [`GooeySkin::radii`] scale, logical px; a step past the end is the last.
    pub fn corner(&self, step: usize) -> f32 {
        self.radii[step.min(self.radii.len() - 1)]
    }

    /// Step `step` of the [`GooeySkin::type_scale`], logical px as drawn: multiplied by
    /// [`GooeySkin::text_scale`], like every text size; a step past the end is the last.
    pub fn type_size(&self, step: usize) -> f32 {
        self.type_scale[step.min(self.type_scale.len() - 1)] * self.text_scale
    }

    /// Step `level` of the [`GooeySkin::surfaces`] ladder, lowest first; a level past the top is the
    /// top, and a skin with no surfaces (which [`GooeySkin::problems`] reports) draws
    /// [`GooeySkin::surface`].
    ///
    /// ```
    /// use bevy_gooey::GooeySkin;
    /// let skin = GooeySkin::dark();
    /// assert_eq!(skin.surface_at(0), skin.background);
    /// assert_eq!(skin.surface_at(200), skin.surfaces[skin.surfaces.len() - 1]);
    /// ```
    pub fn surface_at(&self, level: u8) -> Color {
        self.surfaces
            .get(usize::from(level))
            .or(self.surfaces.last())
            .copied()
            .unwrap_or(self.surface)
    }

    /// A hovered button's fill: [`GooeySkin::surface_raised`] a third of the way to
    /// [`GooeySkin::accent`]. [`apply_skin`] and [`GooeySkin::problems`] both use it, so the colour
    /// checked is the colour drawn.
    pub fn hovered(&self) -> Color {
        self.surface_raised.mix(&self.accent, 0.35)
    }

    /// [`GooeySkin::surface`] as it looks over [`GooeySkin::background`], made opaque: the fill of
    /// a panel in a [`crate::dialog`], so nothing behind the scrim shows through its text.
    /// [`GooeySkin::problems`] checks text against it, so the colour checked is the colour drawn.
    pub fn solid_surface(&self) -> Color {
        let surface = Srgba::from(self.surface);
        let [r, g, b] = composite(
            [surface.red, surface.green, surface.blue],
            surface.alpha,
            rgb(self.background),
        );
        Color::srgb(r, g, b)
    }

    /// Everything about this skin that breaks the contrast, target-size or size rules, in a fixed
    /// order: low contrast pairs, then a too-small target, then a meter too thin for its fill,
    /// then unusable sizes, then the surface ladder's steps and the token scales. Empty for a good
    /// skin; all four presets are good.
    pub fn problems(&self) -> Vec<SkinProblem> {
        self.check(0.0)
    }

    /// [`GooeySkin::problems`] with every contrast pair held to [`ENHANCED_CONTRAST`], 7:1 (WCAG 2.2
    /// success criterion 1.4.6, Xbox Accessibility Guideline 102's high-contrast mode): a skin a
    /// game writes for its own high-contrast mode is tested with this call. Empty for
    /// [`GooeySkin::high_contrast`], and not for [`GooeySkin::dark`] or [`GooeySkin::parchment`],
    /// which meet the 4.5:1 and 3:1 minimums and no more.
    pub fn high_contrast_problems(&self) -> Vec<SkinProblem> {
        self.check(ENHANCED_CONTRAST)
    }

    /// The checks of [`GooeySkin::problems`], with each pair needing at least `floor`.
    fn check(&self, floor: f32) -> Vec<SkinProblem> {
        let mut out = Vec::new();
        let surface = rgb(self.solid_surface());
        let text = rgb(self.text);
        let pairs = [
            ("text on surface", text, surface, TEXT_CONTRAST),
            (
                "text on surface_raised",
                text,
                rgb(self.surface_raised),
                TEXT_CONTRAST,
            ),
            (
                "text on hovered button",
                text,
                rgb(self.hovered()),
                TEXT_CONTRAST,
            ),
            (
                "accent_text on accent",
                rgb(self.accent_text),
                rgb(self.accent),
                TEXT_CONTRAST,
            ),
            (
                "focus_ring on surface_raised",
                rgb(self.focus_ring),
                rgb(self.surface_raised),
                NON_TEXT_CONTRAST,
            ),
            (
                "accent on surface_raised",
                rgb(self.accent),
                rgb(self.surface_raised),
                NON_TEXT_CONTRAST,
            ),
            (
                "outline on surface",
                rgb(self.outline),
                surface,
                NON_TEXT_CONTRAST,
            ),
            (
                "text_muted on surface_raised",
                rgb(self.text_muted),
                rgb(self.surface_raised),
                NON_TEXT_CONTRAST,
            ),
            (
                "accent on surface",
                rgb(self.accent),
                surface,
                NON_TEXT_CONTRAST,
            ),
            (
                "danger on surface",
                rgb(self.danger),
                surface,
                NON_TEXT_CONTRAST,
            ),
            (
                "text_muted on surface",
                rgb(self.text_muted),
                surface,
                TEXT_CONTRAST,
            ),
            (
                "focus_ring on surface",
                rgb(self.focus_ring),
                surface,
                NON_TEXT_CONTRAST,
            ),
        ];
        // A skin that draws its own edge answers for it: the frame is what a panel, a tooltip and
        // the legend are told apart by. The quiet default is not checked, as `surface_raised` on
        // `surface` is the low-key frame both presets have always drawn.
        let edge = self.surface_edge.map(|edge| {
            (
                "surface_edge on surface",
                rgb(edge),
                surface,
                NON_TEXT_CONTRAST,
            )
        });
        for (pair, a, b, needed) in pairs.into_iter().chain(edge) {
            let needed = needed.max(floor);
            let ratio = contrast_ratio(a, b);
            // A NaN colour gives a NaN ratio, which is reported, not passed.
            if ratio.is_nan() || ratio < needed {
                out.push(SkinProblem::LowContrast {
                    pair,
                    ratio,
                    needed,
                });
            }
        }
        let target = self.target();
        if target < MIN_TARGET_PX {
            out.push(SkinProblem::TargetTooSmall { px: target });
        }
        let meter = self.meter_height * self.text_scale;
        let frames = 4.0 * self.border;
        if meter.is_finite() && meter > 0.0 && frames.is_finite() && meter <= frames {
            out.push(SkinProblem::MeterTooThin {
                px: meter,
                needed: frames,
            });
        }
        let positive = [
            ("text_size", self.text_size),
            ("heading_size", self.heading_size),
            ("text_scale", self.text_scale),
            ("min_target", self.min_target),
            ("meter_height", self.meter_height),
        ];
        let non_negative = [
            ("gap", self.gap),
            ("padding", self.padding),
            ("radius", self.radius),
            ("border", self.border),
        ];
        for (field, value) in positive {
            if !value.is_finite() || value <= 0.0 {
                out.push(SkinProblem::BadSize { field });
            }
        }
        for (field, value) in non_negative {
            if !value.is_finite() || value < 0.0 {
                out.push(SkinProblem::BadSize { field });
            }
        }
        for (i, pair) in self.surfaces.windows(2).enumerate() {
            let delta = (cie_lightness(rgb(pair[1])) - cie_lightness(rgb(pair[0]))).abs();
            // A NaN colour gives a NaN step, which is reported, not passed.
            if delta.is_nan() || delta < ELEVATION_STEP {
                out.push(SkinProblem::ElevationStep {
                    lower: i,
                    upper: i + 1,
                    delta_l: delta,
                });
            }
        }
        let usable = |values: &[f32], above_zero: bool| {
            values
                .iter()
                .all(|v| v.is_finite() && if above_zero { *v > 0.0 } else { *v >= 0.0 })
                && values.windows(2).all(|pair| pair[0] <= pair[1])
        };
        let scales = [
            ("surfaces", !self.surfaces.is_empty()),
            ("spacing", usable(&self.spacing, false)),
            ("radii", usable(&self.radii, false)),
            ("type_scale", usable(&self.type_scale, true)),
        ];
        for (field, fine) in scales {
            if !fine {
                out.push(SkinProblem::BadScale { field });
            }
        }
        out
    }

    /// The padding the widgets draw, logical px: [`GooeySkin::padding`] for
    /// [`Density::Comfortable`], half of it for [`Density::Compact`]. Buttons, tooltips, rows and
    /// fields derive theirs from it, so a compact skin keeps one rhythm.
    pub fn pad(&self) -> f32 {
        match self.density {
            Density::Comfortable => self.padding,
            Density::Compact => self.padding * COMPACT_PADDING,
        }
    }

    /// The least width and height of a button the widgets draw, logical px:
    /// [`GooeySkin::min_target`] for [`Density::Comfortable`], that capped at [`MIN_TARGET_PX`] for
    /// [`Density::Compact`]. [`GooeySkin::problems`] checks this value, so a compact skin whose
    /// `min_target` is under 24 px is still refused.
    pub fn target(&self) -> f32 {
        match self.density {
            Density::Comfortable => self.min_target,
            Density::Compact => self.min_target.min(MIN_TARGET_PX),
        }
    }

    /// This skin with every colour taken from `look` and every size kept, the text scale and a
    /// game's own gap and padding included: the step of a [`crate::skin_preset`] row, so a player
    /// who set 200% text and then asks for high contrast has both.
    ///
    /// ```
    /// use bevy_gooey::GooeySkin;
    /// let large = GooeySkin { text_scale: 2.0, ..GooeySkin::dark() };
    /// let high = large.with_colours_of(&GooeySkin::high_contrast());
    /// assert_eq!(high.text_scale, 2.0);
    /// assert_eq!(high.text, GooeySkin::high_contrast().text);
    /// ```
    pub fn with_colours_of(self, look: &GooeySkin) -> Self {
        Self {
            background: look.background,
            surface: look.surface,
            surface_raised: look.surface_raised,
            text: look.text,
            text_muted: look.text_muted,
            accent: look.accent,
            accent_text: look.accent_text,
            danger: look.danger,
            focus_ring: look.focus_ring,
            outline: look.outline,
            surface_edge: look.surface_edge,
            scrim: look.scrim,
            surfaces: look.surfaces.clone(),
            ..self
        }
    }

    /// How far this skin's colours are from `other`'s: the sum of the squares of every channel's
    /// difference over every colour. Zero for equal colours, whatever the sizes.
    fn colour_distance(&self, other: &GooeySkin) -> f32 {
        let colours = |skin: &GooeySkin| {
            [
                skin.background,
                skin.surface,
                skin.surface_raised,
                skin.text,
                skin.text_muted,
                skin.accent,
                skin.accent_text,
                skin.danger,
                skin.focus_ring,
                skin.outline,
                skin.surface_edge.unwrap_or(skin.surface_raised),
                skin.scrim,
            ]
        };
        colours(self)
            .into_iter()
            .zip(colours(other))
            .chain(
                self.surfaces
                    .iter()
                    .copied()
                    .zip(other.surfaces.iter().copied()),
            )
            .map(|(a, b)| {
                let (a, b) = (Srgba::from(a), Srgba::from(b));
                (a.red - b.red).powi(2)
                    + (a.green - b.green).powi(2)
                    + (a.blue - b.blue).powi(2)
                    + (a.alpha - b.alpha).powi(2)
            })
            .sum()
    }
}

impl Default for GooeySkin {
    fn default() -> Self {
        Self::dark()
    }
}

/// The looks the crate ships, in the order a [`crate::skin_preset`] row offers them.
///
/// ```
/// use bevy_gooey::{GooeySkin, SkinPreset};
/// assert_eq!(SkinPreset::of(&GooeySkin::parchment()), SkinPreset::Parchment);
/// assert_eq!(SkinPreset::HighContrast.name(), "High contrast");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum SkinPreset {
    /// [`GooeySkin::dark`].
    Dark,
    /// [`GooeySkin::parchment`].
    Parchment,
    /// [`GooeySkin::high_contrast`].
    HighContrast,
    /// [`GooeySkin::tool`]: dark, blue and [`Density::Compact`].
    Tool,
}

impl SkinPreset {
    /// Every preset, in the order a [`crate::skin_preset`] row offers them.
    pub const ALL: [SkinPreset; 4] = [Self::Dark, Self::Parchment, Self::HighContrast, Self::Tool];

    /// The preset's name in English, as the row shows it; a host that localises writes its own
    /// [`crate::choice`] over [`SkinPreset::ALL`].
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Parchment => "Parchment",
            Self::HighContrast => "High contrast",
            Self::Tool => "Tool",
        }
    }

    /// The whole preset: its colours and its sizes (every preset but [`SkinPreset::Tool`] shares
    /// the same ones).
    pub fn skin(self) -> GooeySkin {
        match self {
            Self::Dark => GooeySkin::dark(),
            Self::Parchment => GooeySkin::parchment(),
            Self::HighContrast => GooeySkin::high_contrast(),
            Self::Tool => GooeySkin::tool(),
        }
    }

    /// The preset whose colours are nearest `skin`'s, sizes ignored: what a
    /// [`crate::skin_preset`] row shows for the skin the game set. A skin a game tuned shows the
    /// nearest preset; equal distances take the first.
    pub fn of(skin: &GooeySkin) -> Self {
        let mut best = (Self::Dark, f32::INFINITY);
        for preset in Self::ALL {
            let distance = skin.colour_distance(&preset.skin());
            if distance < best.1 {
                best = (preset, distance);
            }
        }
        best.0
    }
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

/// One way a [`GooeySkin`] breaks a rule, from [`GooeySkin::problems`].
#[derive(Clone, Debug, PartialEq)]
pub enum SkinProblem {
    /// Two colours drawn on each other are too close: `ratio` is below `needed` (WCAG 2.2: 4.5:1
    /// for text, 3:1 for a focus ring).
    LowContrast {
        /// Which colours, as "`<front>` on `<back>`".
        pair: &'static str,
        /// The contrast they have.
        ratio: f32,
        /// The contrast they need.
        needed: f32,
    },
    /// [`GooeySkin::target`] (the [`GooeySkin::min_target`] a compact skin caps at
    /// [`MIN_TARGET_PX`]) is below [`MIN_TARGET_PX`].
    TargetTooSmall {
        /// The size the skin asks for.
        px: f32,
    },
    /// A size field is not finite, or is at or below zero where it must be positive, or below zero.
    BadSize {
        /// The field's name.
        field: &'static str,
    },
    /// A meter, [`GooeySkin::meter_height`] × [`GooeySkin::text_scale`] high, has no room left for
    /// its fill inside its frame and gap, 4 × [`GooeySkin::border`]: it would draw empty.
    MeterTooThin {
        /// The meter's height.
        px: f32,
        /// The frame and gap it must exceed.
        needed: f32,
    },
    /// Two adjacent steps of the surface ladder, [`GooeySkin::surfaces`], differ by less than
    /// [`ELEVATION_STEP`] in CIE L\*: the eye cannot tell the upper from the lower, so a surface
    /// on it has no edge.
    ElevationStep {
        /// The lower step's index.
        lower: usize,
        /// The upper step's index, `lower + 1`.
        upper: usize,
        /// The difference they have, L\*.
        delta_l: f32,
    },
    /// A token scale is unusable: no [`GooeySkin::surfaces`] at all, or a [`GooeySkin::spacing`],
    /// [`GooeySkin::radii`] or [`GooeySkin::type_scale`] with a non-finite, negative (zero for the
    /// type scale) or falling step.
    BadScale {
        /// The field's name.
        field: &'static str,
    },
}

impl fmt::Display for SkinProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LowContrast {
                pair,
                ratio,
                needed,
            } => {
                write!(
                    f,
                    "{pair} has contrast {ratio:.2}:1; {needed:.2}:1 is the minimum"
                )
            }
            Self::TargetTooSmall { px } => write!(
                f,
                "min_target is {px} px; a target must be at least {MIN_TARGET_PX} px"
            ),
            Self::BadSize { field } => write!(
                f,
                "{field} is not a usable size: it must be finite, and above zero for a text size, text_scale, min_target or meter_height, at least zero otherwise"
            ),
            Self::MeterTooThin { px, needed } => write!(
                f,
                "a meter is {px} px high; its frame and gap take {needed} px (4 × border), leaving no room for its fill"
            ),
            Self::ElevationStep {
                lower,
                upper,
                delta_l,
            } => write!(
                f,
                "surfaces {lower} and {upper} differ by ΔL* {delta_l:.2}; adjacent surfaces of the ladder need at least {ELEVATION_STEP} to be told apart"
            ),
            Self::BadScale { field } => write!(
                f,
                "{field} is not a usable scale: it must have a step (surfaces), be finite and never fall (spacing, radii, type_scale), and be above zero (type_scale)"
            ),
        }
    }
}

/// A skin for one subtree: the node that carries it and everything below it is styled from this
/// skin and not from the [`GooeySkin`] resource, so a preview of a skin, a tool's own chrome or a
/// vendor's window can stand next to the rest of the interface in another look.
///
/// A widget takes the nearest scope at or above it (its own included); a nested scope shadows the
/// one above; none is the resource. It is the whole skin, sizes and [`GooeySkin::text_scale`]
/// included, and [`GooeyFonts`] stays global. [`apply_skin`] restyles exactly the subtree in the
/// frame a scope is added, changed or removed, or a widget moves under or out of one, and a
/// changed resource leaves scoped widgets as they are. A scope that fails
/// [`GooeySkin::problems`] logs a warning per problem and is applied anyway. The toast stack and
/// the legend are not children of the node that spawned them: put the scope on them if they are
/// to follow it.
///
/// ```
/// use bevy::prelude::*;
/// use bevy_gooey::{GooeySkin, SkinScope, button, panel};
/// fn preview(mut commands: Commands) {
///     commands.spawn((panel(), SkinScope(GooeySkin::parchment()), children![button("Play")]));
/// }
/// ```
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct SkinScope(pub GooeySkin);

/// The fonts widgets draw with. A separate resource from [`GooeySkin`] because a font is an asset
/// handle the host loads, not data a project saves.
///
/// The default is Bevy's default font source, which draws Bevy's built-in font when the host
/// enables Bevy's `default_font` feature (on in Bevy's defaults).
#[derive(Resource, Clone, Debug, Default)]
pub struct GooeyFonts {
    /// Labels and button text.
    pub body: FontSource,
    /// Headings.
    pub heading: FontSource,
}

/// What a widget is, which decides how [`apply_skin`] styles it. Every widget function puts one on
/// its root (and on the parts it spawns); put one on your own node to have it styled the same way.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub enum Role {
    /// A full-window layout root; not drawn.
    Screen,
    /// A framed, padded column on [`GooeySkin::surface`].
    Panel,
    /// A column filled with one step of [`GooeySkin::surfaces`], lowest first; nothing else is
    /// drawn (see [`crate::surface`]).
    Surface(u8),
    /// A column of buttons.
    Menu,
    /// Heading text.
    Heading,
    /// Body text.
    Label,
    /// A button; its fill follows its state.
    Button,
    /// The text inside a button; its colour follows the button's state.
    ButtonText,
    /// A meter's track: a slot framed in [`GooeySkin::outline`].
    Meter,
    /// A meter's fill, one border inside the track's frame.
    MeterFill,
    /// A tooltip's box.
    Tooltip,
    /// A [`crate::popover`]'s panel: a card floating beside its anchor, opaque and framed in
    /// [`GooeySkin::outline`] like a dialog's, padded like a panel.
    Popover,
    /// The veil behind a modal dialog.
    Scrim,
    /// A [`crate::toggle`]'s row: a label and a switch, on no fill, its border the focus ring.
    Toggle,
    /// A toggle's track, framed in [`GooeySkin::outline`]; it is [`GooeySkin::accent`] while on.
    ToggleTrack,
    /// A toggle's knob, at the track's start while off and its end while on.
    ToggleKnob,
    /// A [`crate::slider`]'s row: a header and a rail, on no fill, its border the focus ring.
    Slider,
    /// The line a slider's label and value share.
    SliderHeader,
    /// The strip a slider's rail and thumb sit in, as high as the thumb.
    SliderTrack,
    /// A slider's rail: a slot framed in [`GooeySkin::outline`], like a meter's.
    SliderRail,
    /// A slider's fill, from the rail's start to the thumb's centre.
    SliderFill,
    /// The empty room before a slider's thumb, sized by [`crate::update_sliders`] to place it.
    SliderGap,
    /// A slider's thumb, the part a pointer grabs.
    SliderThumb,
    /// A slider's value, as text, at the end of its header.
    SliderReadout,
    /// The label of a toggle or a slider: body text that dims with a disabled control.
    ControlText,
    /// A [`crate::choice`]'s row: a label and the selected option between two arrows, on no fill,
    /// its border the focus ring.
    Choice,
    /// A choice's arrow: the target a click on steps, as wide as a button and as high as a toggle.
    ChoiceArrow,
    /// The chevron in a choice's back arrow, drawn in two borders: dimmed on the first option.
    ChoiceBack,
    /// The chevron in a choice's forward arrow: dimmed on the last option.
    ChoiceForward,
    /// The one grid cell every option of a choice is laid out in, so it is as wide as the widest.
    ChoiceCell,
    /// One option's text in a choice; only the selected one is shown.
    ChoiceOption,
    /// A [`crate::binding`] row: a label and a keycap, on no fill, its border the focus ring.
    Binding,
    /// A binding's keycap: a button's fill and size, [`GooeySkin::outline`] round it, and lit
    /// [`GooeySkin::accent`] while the row waits for a press.
    BindingCap,
    /// The name of what a binding holds, on its keycap; hidden while the row waits.
    BindingText,
    /// What a binding says while it waits, on its keycap: [`GooeySkin::accent_text`]; hidden at
    /// rest, though it keeps its room, so the keycap never changes size.
    BindingPrompt,
    /// A [`crate::prompts`] legend: a slim bar of prompts in the bottom-left corner of its parent,
    /// [`GooeySkin::surface`] in a [`GooeySkin::surface_raised`] frame, so its words read over any
    /// world; absolute, so it moves nothing.
    Prompts,
    /// One [`crate::prompt`]: a keycap and its words in a row; not drawn.
    Prompt,
    /// A prompt's keycap for a key or a mouse button: a rounded rectangle, a binding keycap's fill
    /// and [`GooeySkin::outline`] frame, with the input's name in [`GooeySkin::text`].
    PromptKey,
    /// A prompt's keycap for a pad button: the same fill, frame and text, fully round (a circle
    /// for "A", a pill for "LB").
    PromptButton,
    /// The one full-window column the toasts stand in; not drawn, it pads the toasts.
    ToastStack,
    /// A [`crate::toast`]'s room: a clipped column whose height opens and closes, so the toasts
    /// around it make way and close up; not drawn.
    Toast,
    /// A toast's body: a raised row of a mark and a message, framed in [`GooeySkin::outline`], that
    /// fades in and out.
    ToastBody,
    /// A toast's mark down its left edge: [`GooeySkin::accent`], or [`GooeySkin::danger`] for an
    /// alert, twice as wide.
    ToastMark,
    /// A toast's message: body text that fades with its toast.
    ToastText,
    /// A [`crate::list`]'s root: a column that holds the scrolling viewport and its rail; not drawn.
    List,
    /// A list's viewport: a clipped column, [`crate::ListView::rows`] rows tall, with a lane on its
    /// right for the rail.
    ListView,
    /// A list's rows, in one column the viewport scrolls: a [`GooeySkin::gap`] apart, never
    /// squeezed.
    ListRows,
    /// A list's rail, down its right edge: [`GooeySkin::surface_raised`], shown only while the list
    /// has more than it shows.
    ListRail,
    /// A list's thumb, the share of the rail the viewport is of the content: [`GooeySkin::text_muted`],
    /// [`GooeySkin::text`] under the pointer and [`GooeySkin::accent`] while dragged.
    ListThumb,
    /// A [`crate::split`]'s root: a row or a column of two panes and the divider between them; not
    /// drawn.
    Split,
    /// One pane of a split: a clipped cell the game's content fills; not drawn.
    SplitPane,
    /// A split's divider: a channel [`crate::MIN_TARGET_PX`] wide on [`GooeySkin::surface_raised`],
    /// lit like a button under the pointer, with the focus ring while it has focus.
    SplitDivider,
    /// The grip in the middle of a divider: a pill of [`GooeySkin::text_muted`], [`GooeySkin::text`]
    /// under the pointer or with focus and [`GooeySkin::accent`] while dragged.
    SplitGrip,
    /// A [`crate::dock`]'s root: fills its parent; not drawn.
    Dock,
    /// The part of a dock the layout is built in: fills the dock; not drawn.
    DockBody,
    /// The landing of a tab being carried, drawn over the dock: the part the panel will take, in
    /// [`GooeySkin::accent`] let through over the dock's panels, framed in [`GooeySkin::focus_ring`].
    DockDrop,
    /// The title of a tab being carried, beside the pointer: an opaque card in the ring's frame.
    DockGhost,
    /// A [`crate::tabs`] widget's column: the strip, a gap, the pages; not drawn.
    Tabs,
    /// The strip of tabs, in a row a [`GooeySkin::gap`] apart; not drawn.
    TabList,
    /// One tab: a button's size and look, lit [`GooeySkin::accent`] while it is the selected one.
    Tab,
    /// A tab's label: [`GooeySkin::accent_text`] on the selected tab, else [`GooeySkin::text`].
    TabText,
    /// The one grid cell every page stands in, so the widget is as large as its largest page; not
    /// drawn.
    TabPages,
    /// One page, a column of widgets a [`GooeySkin::gap`] apart; hidden unless its tab is selected.
    TabPage,
    /// A [`crate::text_input`] row: a label and a field on no fill, laid out like a toggle's row.
    TextInput,
    /// The cell a text input's field and hint stand in, so the hint can lie over the field; not
    /// drawn.
    TextCell,
    /// A text input's field: an outlined box in the panel's own colour, [`GooeySkin::outline`]
    /// round it, [`GooeySkin::text_muted`] under the pointer and [`GooeySkin::focus_ring`] while
    /// it holds focus. Its caret is [`GooeySkin::text`] and its selection [`GooeySkin::accent`].
    TextField,
    /// A text input's hint: [`GooeySkin::text_muted`] words over the field while it is empty.
    TextHint,
    /// A [`crate::menu_bar`]: a row of titles on the panel's colour, framed below.
    MenuBar,
    /// A [`crate::menu_title`]: a button's size with no frame at rest, the hovered fill under the
    /// pointer or while its menu is open, and the focus ring while it has focus.
    MenuTitle,
    /// The card a title or a [`crate::submenu`] opens: a [`Role::Popover`]'s look, shown while open.
    MenuPanel,
    /// One command of a menu: a mark, a label, a shortcut and a chevron on a row that lights as a
    /// whole under the pointer, with focus, or while its submenu is open.
    MenuItem,
    /// The slot at the start of a menu item that holds a check or a dot, so labels share one edge.
    MenuMark,
    /// A check item's tick, drawn in two borders while the item is on.
    MenuCheck,
    /// A radio item's dot, drawn while the item is on.
    MenuDot,
    /// A menu item's label.
    MenuText,
    /// A menu item's shortcut: [`GooeySkin::text_muted`] at rest and right-aligned, the label's
    /// colour while the row is lit.
    MenuShortcut,
    /// A submenu item's chevron, the choice's, pointing right.
    MenuChevron,
    /// The line between two groups of a menu: one frame weight of [`GooeySkin::outline`].
    MenuSeparator,
    /// A line in a menu that is not a command ("Nothing here"): [`GooeySkin::text_muted`].
    MenuNote,
    /// A [`crate::command_palette`]'s card: an opaque column on [`GooeySkin::outline`]'s frame, a
    /// field and its rows, a popover's look at the top of the window.
    Palette,
    /// One row of a palette: a menu row's look and size, lit as a whole, with the focus ring, while
    /// it is the one Enter runs (Bevy's `Selected`), though focus stays in the field.
    PaletteRow,
    /// The line under a palette's rows that says which of how many is lit: muted, at the end.
    PaletteCount,
    /// A palette's field row: a [`Role::TextInput`] with no label and no side padding, so its frame
    /// stands where its card's rows do.
    PaletteField,
}

type Styled = (
    Entity,
    &'static Role,
    Option<&'static mut Node>,
    Option<&'static mut BackgroundColor>,
    Option<&'static mut BorderColor>,
    Option<&'static mut TextColor>,
    Option<&'static mut TextFont>,
    Option<(
        &'static mut Popover,
        &'static mut BoxShadow,
        Option<&'static TooltipBox>,
        Option<&'static PopoverPanel>,
    )>,
    Option<&'static ComputedUiRenderTargetInfo>,
    Option<&'static mut TextCursorStyle>,
);

/// Every input a widget's look depends on, read once per run of [`apply_skin`], except the skin,
/// which is the nearest [`SkinScope`] at or above the widget (see `look_of`).
#[derive(Clone, Copy)]
struct Look<'a> {
    skin: &'a GooeySkin,
    fonts: &'a GooeyFonts,
    focused: Option<Entity>,
    /// The widget focus is on, whether or not focus is shown: a text field keeps its ring while
    /// the player types, though a pointer press hid every other one.
    editing: Option<Entity>,
}

/// `look` with the skin the entity is styled from: its own [`SkinScope`], else the nearest one above
/// it, else the resource's. `any_scope` is false when no scope stands anywhere, which skips the
/// walk up the tree.
fn look_of<'a>(
    look: Look<'a>,
    entity: Entity,
    any_scope: bool,
    scopes: &'a Query<&SkinScope>,
    parents: &Query<&ChildOf>,
) -> Look<'a> {
    if !any_scope {
        return look;
    }
    let scope = std::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .find_map(|e| scopes.get(e).ok());
    Look {
        skin: scope.map_or(look.skin, |scope| &scope.0),
        ..look
    }
}

/// The widget state a look depends on.
#[derive(Clone, Copy)]
struct State {
    pressed: bool,
    disabled: bool,
    hovered: bool,
    /// A toggle that is on.
    checked: bool,
    /// A tab that is the selected one of its strip.
    selected: bool,
    /// The entity is a dialog's scrim: a panel on it is the dialog, drawn opaque, framed, wider and
    /// roomier than a screen's.
    scrim: bool,
    /// A toast's opacity; whole for everything that is not one.
    alpha: f32,
    /// A toast of the alert kind.
    alert: bool,
    /// How much of a toast's room is open, from 0 to 1; whole for everything that is not one.
    grow: f32,
    /// How tall a toast's body stands in full, in logical px.
    body_px: f32,
    /// A choice on its first option.
    at_start: bool,
    /// A choice on its last option.
    at_end: bool,
    /// How many rows tall a list's viewport stands; none for everything that is not one.
    rows: f32,
    /// A binding row that waits for the player's next press.
    capturing: bool,
    /// The widget holds visible focus.
    focused: bool,
    /// A menu title or submenu item whose menu is open.
    open: bool,
    /// A menu item whose menu has a check or a radio item in it, so it keeps the mark slot.
    marked: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            pressed: false,
            disabled: false,
            hovered: false,
            checked: false,
            selected: false,
            scrim: false,
            alpha: 1.0,
            alert: false,
            grow: 1.0,
            body_px: 0.0,
            at_start: false,
            at_end: false,
            rows: 0.0,
            capturing: false,
            focused: false,
            open: false,
            marked: false,
        }
    }
}

/// What [`State`] is read from.
type StateQuery = (
    Has<Pressed>,
    Option<&'static SliderDragState>,
    Has<InteractionDisabled>,
    Has<Checked>,
    Has<Selected>,
    Option<&'static Hovered>,
    Option<&'static Toast>,
    Option<(&'static ChoiceValue, &'static ChoiceCount)>,
    Option<&'static ScrollbarDragState>,
    Option<&'static ListView>,
    Has<Capturing>,
    Has<MenuOpen>,
    Has<MenuMarked>,
    Option<&'static Role>,
);

/// A widget whose look may have changed since the last run.
type Restyle = (
    With<Role>,
    Or<(
        Changed<Role>,
        Changed<Pressed>,
        Changed<Hovered>,
        Changed<InteractionDisabled>,
        Added<Selected>,
        Changed<Checked>,
        Changed<SliderDragState>,
        Changed<ScrollbarDragState>,
        Changed<ListView>,
        Changed<TooltipBox>,
        Changed<Toast>,
        Changed<ChoiceValue>,
        Changed<ComputedUiRenderTargetInfo>,
        Added<Capturing>,
        Or<(Added<MenuOpen>, Added<PopoverPanel>, Added<MenuMarked>)>,
    )>,
);

/// The components whose removal restyles a widget.
type Removals<'w, 's> = (
    RemovedComponents<'w, 's, Pressed>,
    RemovedComponents<'w, 's, InteractionDisabled>,
    RemovedComponents<'w, 's, Checked>,
    RemovedComponents<'w, 's, Selected>,
    RemovedComponents<'w, 's, Capturing>,
    RemovedComponents<'w, 's, MenuOpen>,
    RemovedComponents<'w, 's, MenuMarked>,
    RemovedComponents<'w, 's, SkinScope>,
    RemovedComponents<'w, 's, ChildOf>,
);

/// Whether a role is a widget whose parts follow its state: they are restyled with it.
fn owns_parts(role: Option<&Role>) -> bool {
    matches!(
        role,
        Some(
            Role::Button
                | Role::Toggle
                | Role::Slider
                | Role::Choice
                | Role::Binding
                | Role::Toast
                | Role::TextInput
                | Role::PaletteField
                | Role::MenuItem
                | Role::PaletteRow
                | Role::SplitDivider
        )
    )
}

/// Styles every entity with a [`Role`] from [`GooeySkin`] (or the [`SkinScope`] it stands in) and
/// [`GooeyFonts`]: the one place widgets are styled. Runs in [`crate::GooeySystems::Style`].
///
/// A changed skin or font restyles every widget; otherwise only widgets whose role, press, hover,
/// disabled or on state, or scale changed are restyled (and every button, toggle and slider when
/// focus moves), in the frame the change happens, with the parts that follow them (a button's text,
/// a switch's knob, a slider's thumb); a [`SkinScope`] that is added, changed or removed restyles
/// its subtree. Lines are drawn at the scale Bevy last computed for the
/// node, so the frame a window is resized draws them at the old scale's snapping and the next frame
/// at the new one's. A changed skin that fails [`GooeySkin::problems`] logs one warning per problem
/// and is applied anyway.
#[allow(clippy::too_many_arguments)]
pub fn apply_skin(
    skin: Res<GooeySkin>,
    fonts: Res<GooeyFonts>,
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
    changed: Query<Entity, Restyle>,
    mut removed: Removals<'_, '_>,
    scopes: Query<&SkinScope>,
    changed_scopes: Query<(Entity, &SkinScope), Changed<SkinScope>>,
    moved: Query<Entity, Changed<ChildOf>>,
    states: Query<StateQuery>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    mut styled: Query<Styled>,
) {
    if skin.is_changed() {
        for problem in skin.problems() {
            warn!("bevy_gooey: {problem}");
        }
    }
    for (_, scope) in &changed_scopes {
        for problem in scope.0.problems() {
            warn!("bevy_gooey: scoped skin: {problem}");
        }
    }
    let (unpressed, enabled, unchecked, unselected, stopped, closed, unmarked, unscoped, orphaned) =
        &mut removed;
    let any_scope = !scopes.is_empty();
    let look = Look {
        skin: &skin,
        fonts: &fonts,
        focused: visible.0.then(|| focus.get()).flatten(),
        editing: focus.get(),
    };
    let state_of = |entity: Entity| {
        states
            .get(entity)
            .map(
                |(
                    pressed,
                    drag,
                    disabled,
                    checked,
                    selected,
                    hovered,
                    toast,
                    choice,
                    bar,
                    list,
                    capturing,
                    open,
                    marked,
                    role,
                )| {
                    State {
                        // `bevy_ui_widgets` marks a slider pressed for a click on its track only; a grab
                        // of the thumb is a drag, so a dragged slider reads as pressed too, and so does
                        // a dragged scrollbar thumb.
                        pressed: pressed
                            || drag.is_some_and(|d| d.dragging)
                            || bar.is_some_and(|d| d.dragging),
                        disabled,
                        hovered: hovered.is_some_and(|h| h.0),
                        checked,
                        selected,
                        scrim: role == Some(&Role::Scrim),
                        alpha: toast.map_or(1.0, Toast::alpha),
                        alert: toast.is_some_and(|t| t.kind() == ToastKind::Alert),
                        grow: toast.map_or(1.0, Toast::grow),
                        body_px: toast.map_or(0.0, Toast::body_px),
                        at_start: choice.is_some_and(|(value, _)| value.0 == 0),
                        at_end: choice.is_some_and(|(value, count)| value.0 + 1 >= count.0),
                        rows: list.map_or(0.0, |list| list.rows),
                        capturing,
                        focused: look.focused == Some(entity),
                        open,
                        marked,
                    }
                },
            )
            .unwrap_or_default()
    };
    // A part of a toggle or a slider, however deep, follows that toggle or slider's state.
    let owner_of = |entity: Entity| {
        parents
            .iter_ancestors(entity)
            .find(|&a| {
                states.get(a).is_ok_and(|(.., role)| {
                    matches!(
                        role,
                        Some(
                            Role::Toggle
                                | Role::Slider
                                | Role::Choice
                                | Role::Binding
                                | Role::TextInput
                                | Role::PaletteField
                                | Role::Toast
                                | Role::MenuItem
                                | Role::PaletteRow
                                | Role::SplitDivider
                        )
                    )
                })
            })
            .map(state_of)
            .unwrap_or_default()
    };

    if skin.is_changed() || fonts.is_changed() {
        unpressed.clear();
        enabled.clear();
        unchecked.clear();
        unselected.clear();
        stopped.clear();
        closed.clear();
        unmarked.clear();
        unscoped.clear();
        orphaned.clear();
        for item in &mut styled {
            let entity = item.0;
            let parent = parents.get(entity).map(|p| p.parent()).ok();
            restyle(
                item,
                &look_of(look, entity, any_scope, &scopes, &parents),
                state_of(entity),
                parent.map(state_of),
                owner_of(entity),
            );
        }
        return;
    }

    let mut dirty: Vec<Entity> = changed.iter().collect();
    dirty.extend(unpressed.read());
    dirty.extend(enabled.read());
    dirty.extend(unchecked.read());
    dirty.extend(unselected.read());
    // A scope that changed, went, or has a widget moved under or out of it restyles the whole
    // subtree: every widget below it may stand in another skin now.
    let mut regions: Vec<Entity> = changed_scopes.iter().map(|(e, _)| e).collect();
    regions.extend(unscoped.read());
    let orphans = orphaned.read();
    if any_scope {
        regions.extend(&moved);
        regions.extend(orphans);
    }
    for region in regions {
        dirty.push(region);
        dirty.extend(children.iter_descendants(region));
    }
    dirty.extend(stopped.read());
    dirty.extend(closed.read());
    dirty.extend(unmarked.read());
    if focus.is_changed() || visible.is_changed() {
        dirty.extend(
            styled
                .iter()
                .filter(|item| {
                    matches!(
                        item.1,
                        Role::Button
                            | Role::Toggle
                            | Role::Slider
                            | Role::Choice
                            | Role::Binding
                            | Role::Tab
                            | Role::TextField
                            | Role::MenuItem
                            | Role::MenuTitle
                            | Role::SplitDivider
                    )
                })
                .map(|item| item.0),
        );
    }
    // A button's text follows the button's state, so a dirty button dirties its text, and a dirty
    // toggle or slider every part of it.
    let texts: Vec<Entity> = dirty
        .iter()
        .filter_map(|&e| children.get(e).ok())
        .flat_map(|c| c.iter())
        .collect();
    let parts: Vec<Entity> = dirty
        .iter()
        .filter(|&&e| states.get(e).is_ok_and(|(.., role)| owns_parts(role)))
        .flat_map(|&e| children.iter_descendants(e))
        .collect();
    dirty.extend(texts);
    dirty.extend(parts);
    dirty.sort_unstable();
    dirty.dedup();
    for entity in dirty {
        let Ok(item) = styled.get_mut(entity) else {
            continue;
        };
        let parent = parents.get(entity).map(|p| p.parent()).ok();
        restyle(
            item,
            &look_of(look, entity, any_scope, &scopes, &parents),
            state_of(entity),
            parent.map(state_of),
            owner_of(entity),
        );
    }
}

fn restyle(
    (
        entity,
        role,
        node,
        background,
        border,
        text_color,
        text_font,
        floating,
        target,
        cursor,
    ): bevy::ecs::query::QueryItem<'_, '_, Styled>,
    look: &Look,
    state: State,
    parent: Option<State>,
    owner: State,
) {
    let skin = look.skin;
    let scaled = |size: f32| FontSize::Px(size * skin.text_scale);
    let tip = floating.as_ref().and_then(|(_, _, tip, _)| tip.copied());
    // Every line weight (borders, the focus ring, the meter's frame and gap) on whole device
    // pixels at the node's scale, so it is one weight on every side wherever the node lands.
    let line = snap_line_width(skin.border, target.map_or(1.0, |t| t.scale_factor()));
    // The settings controls' sizes grow with the player's text size, like the meter.
    let text_px = skin.text_size * skin.text_scale;
    let rail = skin.meter_height * skin.text_scale;
    let thumb = Vec2::new(text_px * THUMB_TEXT_SIZES, rail * THUMB_RAIL_RATIO);
    let switch = text_px * SWITCH_TEXT_SIZES;
    let knob = (switch - 4.0 * line).max(0.0);
    let chevron = text_px * CHEVRON_TEXT_SIZES;
    // A choice's arrow: a button's width, or the chevron's own if the text has outgrown it.
    let arrow = skin.target().max(chevron * CHEVRON_ARROW_FIT);
    let in_dialog = parent.is_some_and(|p| p.scrim);
    let fill = match role {
        Role::Panel if in_dialog => Some(skin.solid_surface()),
        Role::Panel => Some(skin.surface),
        Role::Button => Some(if state.disabled {
            skin.surface
        } else if state.pressed {
            skin.accent
        } else if state.hovered {
            skin.hovered()
        } else {
            skin.surface_raised
        }),
        // The selected tab is lit like everything that is on (a switch, a meter's fill): the
        // accent, with the text colour inverted on it, two cues by luminance and not by hue. The
        // others are a button's, full-contrast text on a visible fill, never faded.
        Role::Tab => Some(if state.disabled {
            skin.surface
        } else if state.selected || state.pressed {
            skin.accent
        } else if state.hovered {
            skin.hovered()
        } else {
            skin.surface_raised
        }),
        Role::Meter | Role::SliderRail => Some(skin.surface_raised),
        Role::Tooltip | Role::Popover | Role::MenuPanel | Role::MenuBar | Role::Palette => {
            Some(skin.solid_surface())
        }
        // A menu's rows have no fill at rest, so a card shows its own colour; one lights as a
        // whole under the pointer, with focus, or while its submenu is open, in a button's
        // hovered fill, and the accent while pressed. A title the same on the bar.
        Role::MenuTitle | Role::MenuItem => Some(if state.disabled {
            Color::NONE
        } else if state.pressed {
            skin.accent
        } else if state.hovered || state.focused || state.open {
            skin.hovered()
        } else {
            Color::NONE
        }),
        // A palette's row is lit only while it is the active one, never for the pointer resting on
        // it: the pointer moving over a row makes it the active one, so two rows never light.
        Role::PaletteRow => Some(if state.disabled {
            Color::NONE
        } else if state.pressed {
            skin.accent
        } else if state.selected {
            skin.hovered()
        } else {
            Color::NONE
        }),
        Role::MenuSeparator => Some(skin.outline),
        Role::MenuDot => Some(menu_ink(owner, skin)),
        Role::ToastBody => Some(faded(skin.solid_surface(), owner.alpha)),
        Role::ToastMark => Some(faded(
            if owner.alert {
                skin.danger
            } else {
                skin.accent
            },
            owner.alpha,
        )),
        Role::MeterFill => Some(skin.accent),
        Role::ListRail => Some(skin.surface_raised),
        // The divider is the list rail's colour, lit like a button under the pointer and while held.
        Role::SplitDivider => Some(if state.hovered || state.pressed {
            skin.hovered()
        } else {
            skin.surface_raised
        }),
        // The grip is the thumb's grey (3:1 on the rail), the text colour under the pointer or
        // with focus, and the accent while the divider is held.
        Role::SplitGrip => Some(if owner.disabled {
            skin.text_muted
        } else if owner.pressed {
            skin.accent
        } else if owner.hovered || owner.focused {
            skin.text
        } else {
            skin.text_muted
        }),
        // The same grey as a switch's off knob on its track, the pair `problems` already holds at
        // 3:1; the pointer brightens it to the text colour and a drag lights it.
        Role::ListThumb => Some(if state.pressed {
            skin.accent
        } else if state.hovered {
            skin.text
        } else {
            skin.text_muted
        }),
        Role::Scrim => Some(skin.scrim),
        Role::Surface(level) => Some(skin.surface_at(*level)),
        // A switch is lit by the accent while on, and its knob's place says so a second way.
        Role::ToggleTrack => Some(if owner.disabled {
            skin.surface
        } else if owner.checked {
            skin.accent
        } else {
            skin.surface_raised
        }),
        Role::ToggleKnob => Some(if owner.checked && !owner.disabled {
            skin.accent_text
        } else {
            skin.text_muted
        }),
        Role::SliderFill => Some(if owner.disabled {
            skin.text_muted
        } else {
            skin.accent
        }),
        // The thumb is the text colour with a keyline in the panel's, so one of the two contrasts
        // with the fill, the rail and the panel behind it, in either skin.
        Role::SliderThumb => Some(if owner.disabled {
            skin.text_muted
        } else if owner.pressed {
            skin.focus_ring
        } else {
            skin.text
        }),
        // The keycap is a button's fill, lit by the accent while the row waits: the language of
        // every "on" state here (a switch, the selected tab), with the text colour inverted on it.
        Role::BindingCap => Some(if owner.disabled {
            skin.surface
        } else if owner.capturing {
            skin.accent
        } else if owner.hovered {
            skin.hovered()
        } else {
            skin.surface_raised
        }),
        // The legend is the panel's colour, so its words are at the contrast a panel's are over any
        // world; a keycap is a binding keycap at rest: raised, framed in the outline.
        Role::Prompts => Some(skin.surface),
        Role::PromptKey | Role::PromptButton => Some(skin.surface_raised),
        // The landing is the accent let through to the panels under it; the ghost is as opaque as
        // a dialog's card, so its title is read at the contrast of any panel's text.
        Role::DockDrop => Some(skin.accent.with_alpha(DOCK_DROP_ALPHA)),
        Role::DockGhost => Some(skin.solid_surface()),
        // The field is an outlined box in the panel's own colour, so the typed text and the hint
        // meet on it the contrast they are checked at on the panel.
        Role::TextField => Some(skin.solid_surface()),
        Role::Screen
        | Role::Menu
        | Role::Heading
        | Role::Label
        | Role::ButtonText
        | Role::Toggle
        | Role::Slider
        | Role::Choice
        | Role::ChoiceArrow
        | Role::ChoiceBack
        | Role::ChoiceForward
        | Role::ChoiceCell
        | Role::ChoiceOption
        | Role::Binding
        | Role::BindingText
        | Role::BindingPrompt
        | Role::Prompt
        | Role::SliderHeader
        | Role::SliderTrack
        | Role::SliderGap
        | Role::SliderReadout
        | Role::ControlText
        | Role::Toast
        | Role::ToastStack
        | Role::ToastText
        | Role::List
        | Role::ListView
        | Role::ListRows
        | Role::Split
        | Role::SplitPane
        | Role::Dock
        | Role::DockBody
        | Role::Tabs
        | Role::TabList
        | Role::TabText
        | Role::TabPages
        | Role::TabPage
        | Role::TextInput
        | Role::PaletteField
        | Role::TextCell
        | Role::TextHint
        | Role::MenuMark
        | Role::MenuCheck
        | Role::MenuText
        | Role::MenuShortcut
        | Role::MenuChevron
        | Role::MenuNote
        | Role::PaletteCount => None,
    };
    if let (Some(fill), Some(mut background)) = (fill, background) {
        background.set_if_neq(BackgroundColor(fill));
    }

    let edge = match role {
        Role::Panel if in_dialog => Some(BorderColor::all(skin.outline)),
        // A popover floats over whatever is behind it, so its edge is the dialog's: the outline.
        Role::Popover | Role::MenuPanel | Role::Palette => Some(BorderColor::all(skin.outline)),
        Role::DockDrop | Role::DockGhost => Some(BorderColor::all(skin.focus_ring)),
        // The bar's one frame is the line under it, the panel's own quiet edge.
        Role::MenuBar => Some(BorderColor::all(
            skin.surface_edge.unwrap_or(skin.surface_raised),
        )),
        Role::MenuTitle | Role::MenuItem if state.focused => {
            Some(BorderColor::all(skin.focus_ring))
        }
        Role::MenuTitle | Role::MenuItem => Some(BorderColor::all(Color::NONE)),
        // Focus stays in the palette's field, so the row Enter runs carries the ring itself.
        Role::PaletteRow => Some(BorderColor::all(if state.selected {
            skin.focus_ring
        } else {
            Color::NONE
        })),
        // The tick and the arrow are drawn in borders, in the row's ink.
        Role::MenuCheck | Role::MenuChevron => Some(BorderColor::all(menu_ink(owner, skin))),
        Role::Panel | Role::Tooltip | Role::Prompts => Some(BorderColor::all(
            skin.surface_edge.unwrap_or(skin.surface_raised),
        )),
        // Framed in the outline like the other widgets whose fill is the colour of what is behind
        // them: a toast over a panel keeps its edge at 3:1.
        Role::ToastBody => Some(BorderColor::all(faded(skin.outline, owner.alpha))),
        // The keycap is framed in the outline at rest and lit, as a switch's track is framed
        // whether it is on or off: the accent fills a frame the panel can always see at 3:1.
        Role::Meter
        | Role::ToggleTrack
        | Role::SliderRail
        | Role::BindingCap
        | Role::PromptKey
        | Role::PromptButton => Some(BorderColor::all(skin.outline)),
        Role::Button | Role::Tab if look.focused == Some(entity) => {
            Some(BorderColor::all(skin.focus_ring))
        }
        Role::Button | Role::Tab => Some(BorderColor::all(Color::NONE)),
        // A row has no fill of its own, so its border speaks: the ring when focused, the outline
        // under the pointer, nothing at rest.
        Role::Toggle | Role::Slider | Role::Choice | Role::Binding => {
            Some(BorderColor::all(if look.focused == Some(entity) {
                skin.focus_ring
            } else if (state.hovered || state.pressed) && !state.disabled {
                skin.outline
            } else {
                Color::NONE
            }))
        }
        // A chevron is drawn in its borders: the text colour, the accent under the pointer, muted
        // where it cannot step (the first option's back, the last's forward) or on a disabled row.
        Role::ChoiceBack | Role::ChoiceForward => {
            let spent = if *role == Role::ChoiceBack {
                owner.at_start
            } else {
                owner.at_end
            };
            Some(BorderColor::all(if owner.disabled || spent {
                skin.text_muted
            } else if parent.is_some_and(|p| p.hovered) {
                skin.accent
            } else {
                skin.text
            }))
        }
        // The row has no frame of its own: the field speaks. Its edge is the outline at rest, the
        // muted text colour under the pointer, and the ring while it holds focus, shown or not
        // (its caret already says who has the keys, and a pointer press hides every other ring).
        Role::TextInput | Role::PaletteField => Some(BorderColor::all(Color::NONE)),
        Role::TextField => Some(BorderColor::all(if state.disabled {
            skin.outline
        } else if look.editing == Some(entity) {
            skin.focus_ring
        } else if state.hovered {
            skin.text_muted
        } else {
            skin.outline
        })),
        Role::SliderThumb => Some(BorderColor::all(skin.solid_surface())),
        Role::SplitDivider => Some(BorderColor::all(if look.focused == Some(entity) {
            skin.focus_ring
        } else {
            Color::NONE
        })),
        _ => None,
    };
    if let (Some(edge), Some(mut border)) = (edge, border) {
        border.set_if_neq(edge);
    }

    if let Some(mut node) = node {
        let mut next = node.clone();
        let radius = BorderRadius::all(Val::Px(skin.radius));
        match role {
            Role::Panel => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                // Over a dialog's scrim a panel is the dialog: a measure, and room between its
                // title, its body and its choices.
                let room = if in_dialog { DIALOG_ROOM_FACTOR } else { 1.0 };
                next.padding = UiRect::all(Val::Px(skin.pad() * room));
                next.row_gap = Val::Px(skin.gap * room);
                if in_dialog {
                    next.min_width = Val::Px(text_px * DIALOG_MIN_TEXT_SIZES);
                    next.max_width = Val::Px(text_px * DIALOG_MAX_TEXT_SIZES);
                }
            }
            Role::Menu | Role::TabPage | Role::Tabs => next.row_gap = Val::Px(skin.gap),
            // The strip's tabs stand a gap apart like a menu's buttons, and each is a button's size.
            Role::TabList => next.column_gap = Val::Px(skin.gap),
            Role::Button | Role::Tab => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad() * 1.5), Val::Px(skin.pad() * 0.6));
                // A tab shares its strip equally and never narrower than its own label and
                // padding (`Val::Auto`), so a long label is not squeezed out of its tab.
                next.min_width = if *role == Role::Tab {
                    Val::Auto
                } else {
                    Val::Px(skin.target())
                };
                next.min_height = Val::Px(skin.target());
            }
            // A frame marks the whole length at 3:1 against the panel; the fill sits one line
            // inside it, on the track, because no one colour contrasts with both.
            Role::Meter => {
                next.height = Val::Px(skin.meter_height * skin.text_scale);
                next.min_width = Val::Px(skin.text_size * skin.text_scale * METER_MIN_TEXT_SIZES);
                next.border = UiRect::all(Val::Px(line));
                next.padding = UiRect::all(Val::Px(line));
                next.border_radius = radius;
            }
            // CSS's content-edge radius: the track's minus its frame and gap, so the corners are
            // concentric. Bevy clips a child to a rectangle, never to its parent's curve, so the
            // fill is rounded itself.
            Role::MeterFill | Role::SliderFill => {
                next.border_radius =
                    BorderRadius::all(Val::Px((skin.radius - 2.0 * line).max(0.0)));
            }
            // The toasts' own vertical margins make the gap between rows and stand half a gap off
            // the stack's ends, so the stack pads its ends half a gap less than the screen's edge
            // wants and its sides in full.
            Role::ToastStack => {
                let edge = skin.pad() * 2.0;
                next.padding =
                    UiRect::axes(Val::Px(edge), Val::Px((edge - skin.gap / 2.0).max(0.0)));
            }
            // The room a toast holds in the column: its body's height and its margins while it
            // opens or closes, no limit once it has opened. The body inside is clipped to it.
            Role::Toast => {
                next.max_height = if state.grow >= 1.0 {
                    Val::Auto
                } else {
                    Val::Px((state.body_px + skin.gap) * state.grow)
                };
            }
            // The tooltip's measure and the meter's frame, a row of a mark and a message, with half
            // a gap of margin above and below so two toasts stand a gap apart. It is never squeezed
            // by the room it is clipped to: the room is measured from its full height.
            Role::ToastBody => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.max_width = Val::Px(text_px * TOAST_MEASURE_TEXT_SIZES);
                next.margin = UiRect::vertical(Val::Px(skin.gap / 2.0));
                next.flex_shrink = 0.0;
            }
            // Flush with the toast's inside edge, so its outer corners follow the frame's curve.
            Role::ToastMark => {
                let inner = Val::Px((skin.radius - line).max(0.0));
                let marks = if owner.alert { 2.0 } else { 1.0 };
                next.width = Val::Px(line * TOAST_MARK_LINES * marks);
                next.flex_shrink = 0.0;
                next.border_radius = BorderRadius {
                    top_left: inner.into(),
                    bottom_left: inner.into(),
                    top_right: Val::ZERO.into(),
                    bottom_right: Val::ZERO.into(),
                };
            }
            Role::ToastText => {
                next.padding = UiRect::axes(Val::Px(skin.pad()), Val::Px(skin.pad() * 0.5));
                next.flex_shrink = 1.0;
            }
            Role::Popover => {
                // Shown here, styled: it is born `Display::None`, so it is never drawn bare.
                next.display = Display::Flex;
                next.flex_direction = FlexDirection::Column;
                next.align_items = AlignItems::Stretch;
                // Never wider than the window leaves room for, so a long line wraps and the panel
                // can always be placed inside it.
                if let Some(window) = target {
                    next.max_width = Val::Px((window.logical_size().x - 2.0 * skin.gap).max(0.0));
                }
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::all(Val::Px(skin.pad()));
                next.row_gap = Val::Px(skin.gap);
            }
            Role::Tooltip => {
                next.padding = UiRect::axes(Val::Px(skin.pad()), Val::Px(skin.pad() * 0.5));
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.max_width = Val::Px(tooltip_width(skin, tip.unwrap_or_default(), line));
                // The line and, when the command has one, its shortcut's keycap beside it.
                next.align_items = AlignItems::Center;
                next.column_gap = Val::Px(skin.gap);
            }
            // A button's frame and room, a label left and a switch right, the row the target.
            Role::Toggle => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad() * 1.5), Val::Px(skin.pad() * 0.6));
                next.min_height = Val::Px(skin.target());
                next.column_gap = Val::Px(skin.gap);
            }
            // A framed pill, one line of frame and one of gap round the knob, which sits at the
            // start while off and the end while on.
            Role::ToggleTrack => {
                next.width = Val::Px(switch * 2.0);
                next.height = Val::Px(switch);
                next.flex_shrink = 0.0;
                next.border = UiRect::all(Val::Px(line));
                next.padding = UiRect::all(Val::Px(line));
                next.border_radius = BorderRadius::all(Val::Px(switch / 2.0));
                next.justify_content = if owner.checked {
                    JustifyContent::FlexEnd
                } else {
                    JustifyContent::FlexStart
                };
            }
            Role::ToggleKnob => {
                next.width = Val::Px(knob);
                next.height = Val::Px(knob);
                next.flex_shrink = 0.0;
                next.border_radius = BorderRadius::all(Val::Px(knob / 2.0));
            }
            // No room at the sides: `bevy_ui_widgets` turns a click into a value across the whole
            // width of this node, so the thumb must travel that width, less the border.
            Role::Slider => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(0.0), Val::Px(skin.pad() * 0.6));
                next.min_width = Val::Px(text_px * SLIDER_MIN_TEXT_SIZES);
                next.row_gap = Val::Px(skin.gap * 0.5);
            }
            // The header's text lines up with a toggle's label and a toggle's switch.
            Role::SliderHeader => next.padding = UiRect::horizontal(Val::Px(skin.pad() * 1.5)),
            Role::SliderTrack => next.height = Val::Px(thumb.y),
            // From the thumb's centre at the start to its centre at the end, so the thumb
            // overhangs the rail by half its width at each end and the focus ring has air round
            // the rail. The fill ends under the thumb.
            Role::SliderRail => {
                next.left = Val::Px(thumb.x / 2.0);
                next.right = Val::Px(thumb.x / 2.0);
                next.top = Val::Px((thumb.y - rail) / 2.0);
                next.height = Val::Px(rail);
                next.border = UiRect::all(Val::Px(line));
                next.padding = UiRect::all(Val::Px(line));
                next.border_radius = radius;
            }
            Role::SliderThumb => {
                next.width = Val::Px(thumb.x);
                next.height = Val::Px(thumb.y);
                next.flex_shrink = 0.0;
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
            }
            // A toggle's frame and room at the start. At the end the room is whatever the arrow's
            // own width leaves of a toggle's, so the forward chevron's ink stops where a switch's
            // track does and the two rows share their edges.
            Role::Choice => {
                let ink = chevron * std::f32::consts::FRAC_1_SQRT_2;
                let room = skin.pad() * 1.5;
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect {
                    left: Val::Px(room),
                    right: Val::Px((room - (arrow - ink) / 2.0).max(0.0)),
                    top: Val::ZERO,
                    bottom: Val::ZERO,
                };
                next.min_height = Val::Px(skin.target());
                next.column_gap = Val::Px(skin.gap);
            }
            // As wide as a button and as high as the toggle's row inside its frame, so the row is
            // the toggle's height and a thumb has the room NN/g asks of a stepper's buttons.
            Role::ChoiceArrow => {
                next.min_width = Val::Px(arrow);
                next.min_height = Val::Px((switch + skin.pad() * 1.2).max(skin.target()));
                next.flex_shrink = 0.0;
            }
            // Two sides of a square, turned by `choice()`: the forward chevron keeps its top and
            // right, the back one its bottom and left, so each points the way it steps.
            Role::ChoiceBack | Role::ChoiceForward => {
                next.width = Val::Px(chevron);
                next.height = Val::Px(chevron);
                next.flex_shrink = 0.0;
                let line = Val::Px(line);
                next.border = if *role == Role::ChoiceForward {
                    UiRect {
                        top: line,
                        right: line,
                        ..UiRect::ZERO
                    }
                } else {
                    UiRect {
                        bottom: line,
                        left: line,
                        ..UiRect::ZERO
                    }
                };
            }
            // The widest option sets the width, but never less than `CHOICE_MIN_TEXT_SIZES`: half a
            // gap of air stands each side of it, so the value reads as one thing with the arrows
            // round it, and a column of choices has its arrows and values in one place each.
            Role::ChoiceCell => {
                next.padding = UiRect::horizontal(Val::Px(skin.gap / 2.0));
                next.min_width = Val::Px(text_px * CHOICE_MIN_TEXT_SIZES);
                next.flex_shrink = 0.0;
            }
            // A toggle's frame and height. The keycap stands in it with `0.3` of a padding of air
            // above and below, so the focus ring never touches it: at the default skin the keycap is
            // 38.4 px (a 24 px line, that air twice and its frame) and the row 52.8, a switch's row.
            Role::Binding => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad() * 1.5), Val::Px(skin.pad() * 0.3));
                next.min_height = Val::Px(skin.target());
                next.column_gap = Val::Px(skin.gap);
            }
            // A frame and room round one line, as wide as the prompt needs and at least as wide as
            // every other keycap of a column.
            Role::BindingCap => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad() * 0.5), Val::Px(skin.pad() * 0.3));
                next.min_width = Val::Px((text_px * BINDING_CAP_TEXT_SIZES).max(skin.target()));
                next.flex_shrink = 0.0;
            }
            // A binding row's frame and padding, so the label stands where every row's does; the
            // field inside is a button's height at least (the target), and the row, with the little
            // air the field needs round it, is a toggle's. A palette's field has no label to stand
            // under, so its row has no side padding and no frame: the field's own frame is as wide
            // as the rows' lit bars under it.
            Role::TextInput | Role::PaletteField => {
                let palette = *role == Role::PaletteField;
                next.border = if palette {
                    UiRect::ZERO
                } else {
                    UiRect::all(Val::Px(line))
                };
                next.border_radius = radius;
                next.padding = UiRect::axes(
                    Val::Px(if palette { 0.0 } else { skin.pad() * 1.5 }),
                    Val::Px(skin.pad() * 0.1),
                );
                next.min_height = Val::Px(skin.target());
                next.column_gap = Val::Px(skin.gap);
            }
            // One line of text in a frame, as high as the target: the text sits at the top of its
            // box, so the padding is what makes the height (the line, the padding twice and the
            // frame twice come to `min_target`) and what centres the line. A `min_height` here is
            // read as the content's, which made the field a padding and a frame too tall.
            Role::TextField => {
                let (x, y) = text_field_inset(skin, text_px, line);
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(x), Val::Px(y));
            }
            // Over the field's first character: the same inset, inside its frame.
            Role::TextHint => {
                let (x, y) = text_field_inset(skin, text_px, line);
                next.left = Val::Px(line + x);
                next.right = Val::Px(line + x);
                next.top = Val::Px(line + y);
            }
            // The legend: a bar in its parent's bottom-left corner, inside the safe area, its
            // prompts a pair of paddings apart. Absolute, so the screen's layout is the same with
            // or without it. A flex-wrap here would break a line a float's width early.
            Role::Prompts => {
                next.left = Val::Percent(SAFE_EDGE_PERCENT);
                next.bottom = Val::Percent(SAFE_EDGE_PERCENT);
                next.max_width = Val::Percent(PROMPTS_MAX_WIDTH_PERCENT);
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad()), Val::Px(skin.pad() * 0.3));
                next.column_gap = Val::Px(skin.pad() * PROMPT_SPACING_PADDINGS);
            }
            Role::Prompt => next.column_gap = Val::Px(skin.gap),
            // A binding keycap's frame and air, at least as wide as it is high, so a single
            // character is a square (a circle for a pad button) and a longer name a wider cap.
            Role::PromptKey | Role::PromptButton => {
                let cap = text_px * LIST_LINE_TEXT_SIZES + skin.pad() * 0.6 + 2.0 * line;
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = if *role == Role::PromptButton {
                    BorderRadius::MAX
                } else {
                    radius
                };
                next.padding = UiRect::axes(Val::Px(skin.pad() * 0.5), Val::Px(skin.pad() * 0.3));
                next.min_width = Val::Px(cap);
                next.flex_shrink = 0.0;
            }
            // The rows stand `rows` of a button's height and a gap apart; to their right a lane the
            // rail's width and a gap of air, always, so rows never change width when others come
            // and go. The padding is inside the clip, so the content stops short of the rail and
            // still counts toward what scrolls.
            Role::ListView => {
                let row = (text_px * LIST_LINE_TEXT_SIZES
                    + skin.pad() * LIST_PADDING_FACTOR
                    + 2.0 * line)
                    .max(skin.target());
                next.max_height = Val::Px(list_height(state.rows, row, skin.gap));
                next.padding = UiRect::right(Val::Px(RAIL_LINES * line + skin.gap));
            }
            Role::ListRows => next.row_gap = Val::Px(skin.gap),
            // A channel the target's minimum wide (or high), as long as the split, framed only by
            // the focus ring; the grip's own size is laid out with it.
            Role::SplitDivider => {
                next.min_width = Val::Px(MIN_TARGET_PX);
                next.min_height = Val::Px(MIN_TARGET_PX);
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
            }
            Role::SplitGrip => next.border_radius = BorderRadius::MAX,
            Role::DockDrop => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
            }
            Role::DockGhost => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad() * 0.8), Val::Px(skin.pad() * 0.4));
            }
            // Flush with the list's right edge, full height; as wide as the thumb it holds.
            Role::ListRail => {
                let width = RAIL_LINES * line;
                next.width = Val::Px(width);
                next.border_radius = BorderRadius::all(Val::Px(width / 2.0));
            }
            // A row of titles, a frame only under it.
            Role::MenuBar => {
                next.border = UiRect::bottom(Val::Px(line));
                next.padding = UiRect::axes(Val::Px(skin.pad() * 0.5), Val::Px(skin.pad() * 0.3));
                next.column_gap = Val::Px(skin.gap * 0.5);
                next.align_items = AlignItems::Center;
            }
            // A button's size, frameless until the focus ring.
            Role::MenuTitle => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad() * 1.2), Val::Px(skin.pad() * 0.6));
                next.min_height = Val::Px(skin.target());
            }
            // A popover's card with a menu's room: rows touch, each carrying its own padding, so
            // the card's is half a popover's. Display is the menu's own (open or closed).
            Role::MenuPanel => {
                next.flex_direction = FlexDirection::Column;
                next.align_items = AlignItems::Stretch;
                if let Some(window) = target {
                    next.max_width = Val::Px((window.logical_size().x - 2.0 * skin.gap).max(0.0));
                }
                next.min_width = Val::Px(text_px * MENU_MIN_TEXT_SIZES);
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::all(Val::Px(skin.pad() * 0.5));
                next.row_gap = Val::Px(skin.gap * 0.25);
            }
            // A popover's card, wide enough for a line of a command and its chord at the top of
            // the window, as wide as the scrim's inside at the most, its rows touching (each row
            // carries its own padding like a menu's). Where it stands never changes while the rows
            // below the field come and go.
            Role::Palette => {
                next.flex_direction = FlexDirection::Column;
                next.align_items = AlignItems::Stretch;
                next.flex_shrink = 0.0;
                next.width = Val::Px(text_px * PALETTE_TEXT_SIZES);
                next.max_width = Val::Percent(100.0);
                next.margin = UiRect::top(Val::Vh(PALETTE_TOP_VH));
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::all(Val::Px(skin.pad() * 0.5));
                next.row_gap = Val::Px(skin.gap * 0.5);
            }
            Role::PaletteCount => {
                next.align_self = AlignSelf::FlexEnd;
                next.padding = UiRect::axes(Val::Px(skin.pad()), Val::Px(skin.pad() * 0.2));
            }
            // A button's padding and height, the label growing so shortcuts stand at the end.
            Role::MenuItem | Role::MenuNote | Role::PaletteRow => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad()), Val::Px(skin.pad() * 0.4));
                next.min_height = Val::Px(skin.target());
                next.column_gap = Val::Px(skin.gap);
                next.align_items = AlignItems::Center;
            }
            Role::MenuMark => {
                next.display = if owner.marked {
                    Display::Flex
                } else {
                    Display::None
                };
                next.width = Val::Px(text_px * MENU_MARK_TEXT_SIZES);
                next.height = Val::Px(text_px * MENU_MARK_TEXT_SIZES);
                next.flex_shrink = 0.0;
                next.justify_content = JustifyContent::Center;
                next.align_items = AlignItems::Center;
            }
            Role::MenuCheck => {
                next.display = if owner.checked {
                    Display::Flex
                } else {
                    Display::None
                };
                next.width = Val::Px(text_px * 0.4);
                next.height = Val::Px(text_px * 0.7);
                next.border = UiRect {
                    right: Val::Px(line),
                    bottom: Val::Px(line),
                    ..UiRect::ZERO
                };
                next.flex_shrink = 0.0;
            }
            Role::MenuDot => {
                next.display = if owner.checked {
                    Display::Flex
                } else {
                    Display::None
                };
                next.width = Val::Px(text_px * 0.5);
                next.height = Val::Px(text_px * 0.5);
                next.border_radius = BorderRadius::MAX;
                next.flex_shrink = 0.0;
            }
            Role::MenuText => next.flex_grow = 1.0,
            // Two gaps from the label at the least, never squeezed.
            Role::MenuShortcut => {
                next.margin = UiRect::left(Val::Px(skin.gap + text_px * MENU_CHORD_TEXT_SIZES));
                next.flex_shrink = 0.0;
            }
            Role::MenuChevron => {
                next.width = Val::Px(chevron);
                next.height = Val::Px(chevron);
                next.border = UiRect {
                    top: Val::Px(line),
                    right: Val::Px(line),
                    ..UiRect::ZERO
                };
                next.flex_shrink = 0.0;
            }
            // A line the card's width, a little in from its rows and a gap apart from them.
            Role::MenuSeparator => {
                next.height = Val::Px(line);
                next.margin = UiRect::axes(Val::Px(skin.pad() * 0.5), Val::Px(skin.gap * 0.5));
                next.flex_shrink = 0.0;
            }
            // The safe area: a long body wraps at the dialog's maximum width or squeezes it in,
            // and nothing of it ever reaches the bezel.
            Role::Scrim => next.padding = UiRect::all(Val::Percent(SAFE_EDGE_PERCENT)),
            _ => {}
        }
        node.set_if_neq(next);
    }

    let text = match role {
        Role::Heading => Some((skin.text, &look.fonts.heading, skin.heading_size)),
        Role::Label => Some((skin.text, &look.fonts.body, skin.text_size)),
        Role::ButtonText => {
            let parent = parent.unwrap_or_default();
            let color = if parent.disabled {
                skin.text_muted
            } else if parent.pressed {
                skin.accent_text
            } else {
                skin.text
            };
            Some((color, &look.fonts.body, skin.text_size))
        }
        // The selected tab's label is drawn on the accent fill, so it takes the accent's text.
        Role::TabText => {
            let parent = parent.unwrap_or_default();
            let color = if parent.disabled {
                skin.text_muted
            } else if parent.selected || parent.pressed {
                skin.accent_text
            } else {
                skin.text
            };
            Some((color, &look.fonts.body, skin.text_size))
        }
        Role::ControlText | Role::SliderReadout | Role::ChoiceOption | Role::BindingText => {
            let color = if owner.disabled {
                skin.text_muted
            } else {
                skin.text
            };
            Some((color, &look.fonts.body, skin.text_size))
        }
        // The field's text, and the hint's, are the body's: the typed text at the text colour and
        // the hint muted; a disabled field is muted whole.
        Role::TextField => {
            let color = if state.disabled || owner.disabled {
                skin.text_muted
            } else {
                skin.text
            };
            Some((color, &look.fonts.body, skin.text_size))
        }
        Role::TextHint => Some((skin.text_muted, &look.fonts.body, skin.text_size)),
        Role::MenuText => Some((menu_ink(owner, skin), &look.fonts.body, skin.text_size)),
        // Muted at rest; the label's ink once the row is lit (muted is not held against the
        // hovered fill), so a chord is always as readable as the label beside it.
        Role::MenuShortcut => {
            let color = if owner.disabled {
                skin.text_muted
            } else if owner.pressed {
                skin.accent_text
            } else if owner.hovered || owner.focused || owner.open || owner.selected {
                skin.text
            } else {
                skin.text_muted
            };
            Some((color, &look.fonts.body, skin.text_size))
        }
        Role::MenuNote | Role::PaletteCount => {
            Some((skin.text_muted, &look.fonts.body, skin.text_size))
        }
        Role::BindingPrompt => Some((skin.accent_text, &look.fonts.body, skin.text_size)),
        Role::PromptKey | Role::PromptButton | Role::DockGhost => {
            Some((skin.text, &look.fonts.body, skin.text_size))
        }
        Role::ToastText => Some((
            faded(skin.text, owner.alpha),
            &look.fonts.body,
            skin.text_size,
        )),
        _ => None,
    };
    if let Some((color, font, size)) = text {
        if let Some(mut text_color) = text_color {
            text_color.set_if_neq(TextColor(color));
        }
        if let Some(mut text_font) = text_font {
            let mut next = text_font.clone();
            next.font = font.clone();
            next.font_size = scaled(size);
            text_font.set_if_neq(next);
        }
    }
    // A text field's caret is the text colour and its selection the lit accent with the accent's
    // text, the pair every "on" state here is drawn in; an unfocused selection is not drawn.
    if let (Role::TextField, Some(mut caret)) = (role, cursor) {
        caret.set_if_neq(TextCursorStyle {
            color: skin.text,
            selection_color: skin.accent,
            unfocused_selection_color: Color::NONE,
            selected_text_color: Some(skin.accent_text),
            selection_radius: 0.0,
        });
    }

    if let Some((mut popover, mut shadow, tip, panel)) = floating {
        let next = if let Some(tip) = tip {
            // One spacing step outside the panel a widget usually sits in (its padding and border),
            // so the tooltip never straddles that panel's edge. `Popover` measures from inside the
            // widget's border, so that border is added once more.
            let gap = skin.gap + skin.pad() + line * 2.0;
            tooltip_placement(gap, skin.gap, tip.in_row)
        } else if let Some(panel) = panel {
            // Half a gap off its anchor, and a gap from the window's edge.
            placement_of(panel.placement, skin.gap * 0.5 + line, skin.gap)
        } else {
            return;
        };
        if *popover != next {
            *popover = next;
        }
        // A soft drop shadow in the background colour lifts the box above the screen.
        shadow.set_if_neq(BoxShadow::new(
            skin.background.with_alpha(0.7),
            Val::Px(0.0),
            Val::Px(skin.border * 2.0),
            Val::Px(0.0),
            Val::Px(skin.pad()),
        ));
    }
}

/// The ink of a menu row's label, tick and arrow: the text colour, the accent's text while the row
/// is pressed, muted while it is disabled.
fn menu_ink(row: State, skin: &GooeySkin) -> Color {
    if row.disabled {
        skin.text_muted
    } else if row.pressed {
        skin.accent_text
    } else {
        skin.text
    }
}

/// `color` with its own alpha times `opacity`: a toast fading in or out. Whole opacity returns the
/// colour untouched.
fn faded(color: Color, opacity: f32) -> Color {
    if opacity >= 1.0 {
        return color;
    }
    let c = Srgba::from(color);
    Color::from(Srgba {
        alpha: c.alpha * opacity.max(0.0),
        ..c
    })
}

/// A tooltip's widest box, logical px: a comfortable line of about 40 characters (24 text sizes),
/// narrowed to the room the window leaves beside a widget in a column so the tooltip wraps there
/// instead of falling back over the next widget. Below 8 text sizes of room, wrapping would make
/// a ribbon of single words, so the measure stands and `Popover` takes the next side. A shortcut's
/// keycap is never cropped: the box is at least the cap, a gap and six text sizes of words beside
/// it wide, so a long chord at a large text size makes `Popover` take another side before it makes
/// the box leave the window.
fn tooltip_width(skin: &GooeySkin, tip: TooltipBox, line: f32) -> f32 {
    let size = skin.text_size * skin.text_scale;
    let measure = size * 24.0;
    let width = if tip.in_row {
        measure
    } else {
        // The box's outer edge sits `gap + padding + border` from the widget, and `gap` from the
        // window.
        let fit = tip.room - (skin.gap + skin.pad() + line) - skin.gap;
        if fit >= size * 8.0 {
            measure.min(fit)
        } else {
            measure
        }
    };
    if tip.cap > 0.0 {
        width.max(tip.cap + skin.gap + size * 6.0 + 2.0 * (skin.pad() + line))
    } else {
        width
    }
}

/// Where a tooltip opens: across its widget's flow first (beside a widget in a column, below one in
/// a row), then the opposite side, then along the flow; `gap` from the widget and `margin` from the
/// window's edge. Centred on the widget, so it reads as attached to that widget and not to its
/// neighbour.
fn tooltip_placement(gap: f32, margin: f32, in_row: bool) -> Popover {
    use PopoverSide::{Bottom, Left, Right, Top};
    let sides = if in_row {
        [Bottom, Top, Right, Left]
    } else {
        [Right, Left, Bottom, Top]
    };
    Popover {
        positions: sides
            .into_iter()
            .map(|side| PopoverPlacement {
                side,
                align: PopoverAlign::Center,
                gap,
            })
            .collect(),
        window_margin: margin,
    }
}

/// How far a text field's text stands inside its frame, in logical px: the side, and the top, which
/// is what leaves the line in the middle of the target's height (the text sits at the top of its
/// box). Never less than the thin air a keycap has above and below.
fn text_field_inset(skin: &GooeySkin, text_px: f32, line: f32) -> (f32, f32) {
    let x = skin.pad() * 0.6;
    let centred = (skin.target() - 2.0 * line - text_px * LIST_LINE_TEXT_SIZES) / 2.0;
    (x, centred.max(skin.pad() * 0.3))
}
