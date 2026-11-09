//! The skin: the look of every widget, as data, and the one system that applies it.

use std::collections::BTreeMap;
use std::fmt;

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{FontSize, FontSource, TextCursorStyle};
use bevy::ui::{Checked, ComputedUiRenderTargetInfo, InteractionDisabled, Pressed, Selected};
use bevy::ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide};
use bevy::ui_widgets::{ScrollbarDragState, SliderDragState};

use crate::binding::Capturing;
use crate::button_variant::{ButtonPhase, ButtonVariant, LoadingHold};
use crate::choice::{ChoiceCount, ChoiceValue};
use crate::color_motion::ColorTrack;
use crate::control_size::ControlSize;
use crate::drag::{DropCandidate, DropRefusing};
use crate::elevation::{Elevation, ElevationLevel};
use crate::field::{FieldProblem, FieldProblemKind, ProblemRoom};
use crate::icon::{IconSize, IconTone};
use crate::list::ListView;
use crate::menu::{MenuMarked, MenuOpen};
use crate::model::{
    COMFORT_TARGET_PX, ELEVATION_STEP, ENHANCED_CONTRAST, MIN_TARGET_PX, NON_TEXT_CONTRAST,
    PRIMARY_LIFT_L, STATE_STEP, TEXT_CONTRAST, cie_lightness, composite, contrast_ratio,
    lift_toward, list_height, snap_line_width, snap_text_size, tree_indent,
};
use crate::popover::{PopoverPanel, placement_of};
use crate::presence::PresenceAlpha;
use crate::prompt::PromptsAnchor;
use crate::selection_indicator::Indicating;
use crate::status::StatusKind;
use crate::table::TableRow;
use crate::toast::{Toast, ToastKind};
use crate::tooltip::TooltipBox;
use crate::{MotionTokens, PopoverSlide};

/// How far a skin's held colours step back toward its background when it sets none: 0.6, so a
/// world glyph left at two fifths of the way from the background still reads and no longer competes
/// with an overlay. See [`GooeySkin::held_fade`].
const HELD_FADE: f32 = 0.6;

/// A meter's least length, in body text sizes: 8 × 20 px = 160 px at the default size, growing
/// with [`GooeySkin::text_scale`].
const METER_MIN_TEXT_SIZES: f32 = 8.0;

/// A progress bar's least length, in body text sizes, so its line of words (the longest is about
/// 25 sizes) never widens it as the numbers change.
const PROGRESS_MIN_TEXT_SIZES: f32 = 28.0;
/// A progress inside a status item is a short bar: this many body text sizes wide at the least.
const STATUS_PROGRESS_TEXT_SIZES: f32 = 8.0;

/// A spinner's side, in body text sizes.
const SPINNER_TEXT_SIZES: f32 = 1.5;

/// A slider's least width, in body text sizes: 10 × 20 px = 200 px at the default size.
const SLIDER_MIN_TEXT_SIZES: f32 = 10.0;

/// The least width of a choice's value cell (its option, the air round it and nothing of the
/// arrows), in body text sizes: 8 × 20 px = 160 px at the default size, which holds "PlayStation"
/// and "Fullscreen" in the host's font with room, so choices stacked in a panel have their arrows
/// and values aligned instead of each at its widest option's width.
const CHOICE_MIN_TEXT_SIZES: f32 = 8.0;

/// A [`crate::radial`]'s disc is this many body text sizes across: wide enough that eight short
/// labels on its ring (at 36% of the diameter from the middle) stand clear of one another, and
/// still a fraction of a 720p window at the default size (26 × 16 px = 416 px).
const RADIAL_DIAMETER_TEXT_SIZES: f32 = 26.0;

/// A picture on a [`crate::radial`]'s ring is this many of the wheel's text sizes square (48 px by
/// default): big enough to tell eight weapons apart, small enough that the ring's items stand
/// clear of one another.
const RADIAL_ICON_TEXT_SIZES: f32 = 3.0;

/// A [`crate::slot_grid`]'s slot is this many body text sizes square (4 × 16 px = 64 px by default,
/// over the 44 px target), so the grid follows the player's text size.
const SLOT_TEXT_SIZES: f32 = 4.0;

/// The share of a slot's side its picture takes.
const SLOT_ICON_SHARE: f32 = 0.6;

/// A card's width, in body text sizes: room for a name of about 25 characters on a line and a
/// picture large enough to recognise.
const CARD_TEXT_SIZES: f32 = 14.0;

/// A card's picture's width over its height: 16:9, a screen's shape, as a thumbnail of a scene is.
const CARD_IMAGE_ASPECT: f32 = 16.0 / 9.0;

/// How much of a disabled card's picture shows: half, dimmed with its words.
const CARD_DIM_ALPHA: f32 = 0.5;

/// How much of the icon a [`crate::Cooldown`]'s unrecovered wedge covers: the skin's background at
/// this alpha, so the icon is dimmed and still recognisable.
const COOLDOWN_DIM_ALPHA: f32 = 0.66;

/// The alpha of the hand at the sweep's edge.
const COOLDOWN_EDGE_ALPHA: f32 = 0.7;

/// The alpha of the chip the cooldown's seconds are drawn on. Not 1: the icon shows faintly
/// through, so the chip reads as part of the cover; [`GooeySkin::problems`] measures the words
/// against the chip over the worst icon.
const COOLDOWN_CHIP_ALPHA: f32 = 0.92;

/// The most of the window's shorter side a [`crate::radial`]'s disc takes: 92%, which at 720p is a
/// 662 px wheel and caps its text at 25 px (125% of the default), so the wheel is whole on the
/// screen at 150% and 200% text too.
const RADIAL_WINDOW_SHARE: f32 = 0.92;

/// A toggle's track is this many body text sizes high and twice as wide.
const SWITCH_TEXT_SIZES: f32 = 1.5;

/// A slider's thumb is this many body text sizes wide, and this many rails high.
const THUMB_TEXT_SIZES: f32 = 1.0;

/// How high the picker's colour area is, in logical px; it is as wide as the picker. A pointer needs
/// a plane to aim at, not one that grows with the text, so 200% text does not push the strips off
/// a 720p window.
const COLOR_AREA_HEIGHT_PX: f32 = 136.0;

/// How wide the ring on the colour area is, in body text sizes.
const COLOR_MARKER_TEXT_SIZES: f32 = 1.3;

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
/// How much of the ring's colour the landing of a carried tab lets over the panels it covers.
const DOCK_DROP_ALPHA: f32 = 0.22;

/// How much of the ring's (or the danger's) colour a drop mark lets through: less than a dock's landing,
/// because a target may be the whole viewport and what is under the mark must stay readable.
const DROP_MARK_ALPHA: f32 = 0.12;
/// How much of the raised fill a table's banded row has: half, so the band is a hint under the text
/// and never a second look.
const TABLE_BAND_ALPHA: f32 = 0.5;
/// A table's sort mark is this share of a choice's chevron: it stands beside a title, not in a
/// target of its own.
const TABLE_SORT_MARK_SCALE: f32 = 0.8;

/// An LED's dot is this many body text sizes square.
pub(crate) const LED_TEXT_SIZES: f32 = 1.1;

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
    /// The host's own semantic colours, by name: a suggestion ink, a map's grid line, a rarity.
    /// Read one with [`GooeySkin::tone`], draw a node with it by [`Role::Tone`], check the pairs a
    /// host relies on with [`GooeySkin::tone_problems`]. Empty in every preset; a skin file that
    /// carries tones lists them in name order. Not checked by [`GooeySkin::problems`]: the crate
    /// cannot know which pairs a host draws.
    pub tones: BTreeMap<String, Color>,
    /// The fill of a row, chip or field under the pointer; `None` draws
    /// [`GooeySkin::surface_raised`] a third of the way to [`GooeySkin::accent`]. Read it with
    /// [`GooeySkin::hovered`]: every hovered widget of the crate draws it, and [`Role::Hovered`]
    /// draws it for a host's own.
    pub hover: Option<Color>,
    /// The fill of a host's own control with the pointer down on it, recessed against
    /// [`GooeySkin::hovered`]; `None` draws [`GooeySkin::solid_surface`], the panel it sinks into.
    /// Read it with [`GooeySkin::pressed`]; [`Role::Pressed`] draws it. The crate's own pressed
    /// button is [`GooeySkin::accent`] and is not changed by this.
    pub pressed: Option<Color>,
    /// A world gizmo's or glyph's colour while the key that asks the world to step back is held;
    /// `None` is [`GooeySkin::text`] faded toward [`GooeySkin::held_ground`] by
    /// [`GooeySkin::held_fade`], so the glyph loses contrast with the ground it stands on in a dark
    /// skin and a light one alike. Read it with [`GooeySkin::held`]; [`Role::Held`] draws it.
    pub held: Option<Color>,
    /// The ground that steps back with [`GooeySkin::held`]; `None` is
    /// [`GooeySkin::solid_surface`] faded toward [`GooeySkin::background`] by
    /// [`GooeySkin::held_fade`]. Read it with [`GooeySkin::held_ground`]; [`Role::HeldGround`]
    /// draws it.
    pub held_ground: Option<Color>,
    /// How far [`GooeySkin::held`] and [`GooeySkin::held_ground`] step back when they are not set,
    /// from 0 (no change) to 1 (the ground gone into the background, the glyph into the ground):
    /// both come off this one fade, so a held ground and a held glyph compose. Outside 0 to 1 or not finite is [`SkinProblem::BadSize`].
    pub held_fade: f32,
    /// The motion palette: four springs (snappy, default, calm, lead) and a fade length, so every
    /// motion of the interface shares one feel. Default: [`MotionTokens::default`]. A spring that
    /// has no motion, or a fade that is negative or not finite, is [`SkinProblem::BadMotion`]. Not
    /// reflected (a spring is plain maths in `model`).
    #[reflect(ignore)]
    pub motion: MotionTokens,
    /// The mark of a thing that has not been judged yet (a check not run, a box not ticked by the
    /// tool); `None` is [`GooeySkin::text_muted`]. Read it with [`GooeySkin::unchecked`];
    /// [`Role::Unchecked`] draws it. Checked against the panel at the 3:1 graphic minimum.
    pub unchecked: Option<Color>,
    /// The ground of a card the scrim leaves lit while it holds the rest back; `None` is
    /// [`GooeySkin::solid_surface`], the dialog panel's own fill. Read it with
    /// [`GooeySkin::card`]; [`Role::Card`] draws it. Text is checked on it.
    pub card: Option<Color>,
    /// The edge of a row's preview box; `None` is [`GooeySkin::outline`]. Read it with
    /// [`GooeySkin::preview_edge`]; [`Role::PreviewEdge`] draws it as a border. Checked against the
    /// panel at the 3:1 graphic minimum.
    pub preview_edge: Option<Color>,
    /// The shadows a surface casts at each height it stands at: [`Elevation::raised`] under a
    /// panel and a card, [`Elevation::floating`] under a popover, a menu, a tooltip and the
    /// colour picker, [`Elevation::overlay`] under a dialog's card, a toast and the command
    /// palette. Each a stack of CSS-style [`crate::ShadowLayer`]s in logical px. `dark` (the
    /// default skin), `parchment` and `tool` carry Rune's neutral stacks and `high_contrast` none,
    /// as its edges carry its depth; a theme step ([`GooeySkin::with_colours_of`]) carries them. A negative or non-number blur, or an offset or spread that is not
    /// finite, is [`SkinProblem::BadSize`] (`elevation.<level>`).
    pub elevation: Elevation,
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
            elevation: Elevation::on_dark(),
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
            elevation: Elevation::on_light(),
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
            elevation: Elevation::on_dark(),
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
            tones: BTreeMap::new(),
            hover: None,
            pressed: None,
            held: None,
            held_ground: None,
            held_fade: HELD_FADE,
            motion: MotionTokens::default(),
            unchecked: None,
            card: None,
            preview_edge: None,
            elevation: Elevation::none(),
        }
    }

    /// The colour of the tone called `name` ([`GooeySkin::tones`]), or `None` when the skin holds
    /// none by that name: never a default colour. Names are case-sensitive and not translated.
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use bevy_gooey::GooeySkin;
    /// let mut skin = GooeySkin::dark();
    /// skin.tones.insert("suggest".into(), Color::srgb(0.6, 0.8, 0.5));
    /// assert_eq!(skin.tone("suggest"), Some(Color::srgb(0.6, 0.8, 0.5)));
    /// assert_eq!(skin.tone("Suggest"), None);
    /// ```
    pub fn tone(&self, name: &str) -> Option<Color> {
        self.tones.get(name).copied()
    }

    /// The colour a [`GooeySkin::tone_problems`] pair names: a tone, else one of the skin's own
    /// colours (`background`, `surface`, `surface_raised`, `text`, `text_muted`, `accent`,
    /// `accent_text`, `danger`, `focus_ring`, `outline`). `surface` is the opaque surface a
    /// panel's text is checked on.
    fn colour_named(&self, name: &str) -> Option<Color> {
        self.tone(name).or(match name {
            "background" => Some(self.background),
            "surface" => Some(self.solid_surface()),
            "surface_raised" => Some(self.surface_raised),
            "text" => Some(self.text),
            "text_muted" => Some(self.text_muted),
            "accent" => Some(self.accent),
            "accent_text" => Some(self.accent_text),
            "danger" => Some(self.danger),
            "focus_ring" => Some(self.focus_ring),
            "outline" => Some(self.outline),
            _ => None,
        })
    }

    /// Check the ink and ground pairs a host's own chrome draws, with the arithmetic
    /// [`GooeySkin::problems`] uses for the crate's: each `(pair, ink, ground, needed)` names two
    /// colours, each a tone of this skin or one of its own (`"text"`, `"surface"`, `"accent"`, ...,
    /// see [`GooeySkin::tones`]), and the contrast `needed` between them (4.5 for text, 3.0 for a
    /// graphic, [`ENHANCED_CONTRAST`] for a high-contrast look). One
    /// [`SkinProblem::LowContrast`] named `pair` for each that falls short; one
    /// [`SkinProblem::UnknownTone`] for each name that is neither, since a pair that cannot be read
    /// must not pass. Alpha is ignored, as in [`GooeySkin::problems`].
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use bevy_gooey::{GooeySkin, SkinProblem};
    /// let mut skin = GooeySkin::dark();
    /// skin.tones.insert("suggest".into(), Color::srgb(0.9, 0.9, 0.5));
    /// let pairs = [("suggest on surface", "suggest", "surface", 4.5)];
    /// assert!(skin.tone_problems(&pairs).is_empty());
    /// let pairs = [("suggest on text", "suggest", "text", 4.5)];
    /// assert!(matches!(skin.tone_problems(&pairs)[0], SkinProblem::LowContrast { .. }));
    /// ```
    pub fn tone_problems(&self, pairs: &[(&'static str, &str, &str, f32)]) -> Vec<SkinProblem> {
        let mut out = Vec::new();
        for &(pair, ink, ground, needed) in pairs {
            let (Some(a), Some(b)) = (self.colour_named(ink), self.colour_named(ground)) else {
                for name in [ink, ground] {
                    if self.colour_named(name).is_none() {
                        out.push(SkinProblem::UnknownTone {
                            name: name.to_owned(),
                        });
                    }
                }
                continue;
            };
            let ratio = contrast_ratio(rgb(a), rgb(b));
            // A NaN colour gives a NaN ratio, which is reported, not passed.
            if ratio.is_nan() || ratio < needed {
                out.push(SkinProblem::LowContrast {
                    pair,
                    ratio,
                    needed,
                });
            }
        }
        out
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

    /// A hovered button's fill: [`GooeySkin::hover`] when the skin sets it, else
    /// [`GooeySkin::surface_raised`] a third of the way to [`GooeySkin::accent`].
    /// [`apply_skin`] and [`GooeySkin::problems`] both use it, so the colour checked is the colour
    /// drawn.
    pub fn hovered(&self) -> Color {
        self.hover
            .unwrap_or_else(|| self.surface_raised.mix(&self.accent, 0.35))
    }

    /// The fill of a host's own control with the pointer down: [`GooeySkin::pressed`] (the field)
    /// when the skin sets it, else [`GooeySkin::solid_surface`]. A skin that sets none still
    /// answers.
    ///
    /// ```
    /// use bevy_gooey::GooeySkin;
    /// let skin = GooeySkin::dark();
    /// assert_eq!(skin.pressed, None);
    /// assert_eq!(skin.pressed(), skin.solid_surface());
    /// ```
    pub fn pressed(&self) -> Color {
        self.pressed.unwrap_or_else(|| self.solid_surface())
    }

    /// A [`crate::primary_button`]'s fill under the pointer: [`GooeySkin::accent`] moved toward
    /// [`GooeySkin::text`] by [`crate::PRIMARY_LIFT_L`] of CIE L\* ([`crate::lift_toward`]). A step
    /// in lightness, not a fixed mix, so the lift is seen in a skin whose accent is already near
    /// its text; the text is the ink furthest from the accent's own words, so
    /// [`GooeySkin::accent_text`] stays readable, and [`GooeySkin::problems`] checks both.
    ///
    /// ```
    /// use bevy_gooey::GooeySkin;
    /// let skin = GooeySkin::dark();
    /// assert_ne!(skin.primary_hovered(), skin.accent);
    /// ```
    pub fn primary_hovered(&self) -> Color {
        self.lifted(PRIMARY_LIFT_L)
    }

    /// A [`crate::primary_button`]'s fill held down: [`GooeySkin::accent`] moved twice
    /// [`crate::PRIMARY_LIFT_L`] toward [`GooeySkin::text`], a step past
    /// [`GooeySkin::primary_hovered`].
    pub fn primary_pressed(&self) -> Color {
        self.lifted(PRIMARY_LIFT_L * 2.0)
    }

    /// The accent moved `step_l` of CIE L\* toward the text, at the accent's own alpha.
    fn lifted(&self, step_l: f32) -> Color {
        let [red, green, blue] = lift_toward(rgb(self.accent), rgb(self.text), step_l);
        Color::srgba(red, green, blue, self.accent.alpha())
    }

    /// The colour of a world gizmo or glyph while the player holds the key that asks the world to
    /// step back: [`GooeySkin::held`] (the field), else [`GooeySkin::text`] faded toward
    /// [`GooeySkin::held_ground`] by [`GooeySkin::held_fade`].
    ///
    /// ```
    /// use bevy_gooey::GooeySkin;
    /// let skin = GooeySkin { held_fade: 0.0, ..GooeySkin::dark() };
    /// assert_eq!(skin.held(), skin.text);
    /// let gone = GooeySkin { held_fade: 1.0, ..GooeySkin::dark() };
    /// assert_eq!(gone.held(), gone.held_ground());
    /// ```
    pub fn held(&self) -> Color {
        self.held
            .unwrap_or_else(|| self.text.mix(&self.held_ground(), self.fade()))
    }

    /// The ground that steps back with [`GooeySkin::held`]: [`GooeySkin::held_ground`] (the
    /// field), else [`GooeySkin::solid_surface`] faded toward [`GooeySkin::background`] by the same
    /// [`GooeySkin::held_fade`], so the two compose.
    pub fn held_ground(&self) -> Color {
        self.held_ground
            .unwrap_or_else(|| self.solid_surface().mix(&self.background, self.fade()))
    }

    /// The mark of a thing not judged yet: [`GooeySkin::unchecked`] (the field), else
    /// [`GooeySkin::text_muted`].
    pub fn unchecked(&self) -> Color {
        self.unchecked.unwrap_or(self.text_muted)
    }

    /// The ground of a card the scrim leaves lit: [`GooeySkin::card`] (the field), else
    /// [`GooeySkin::solid_surface`].
    pub fn card(&self) -> Color {
        self.card.unwrap_or_else(|| self.solid_surface())
    }

    /// The edge of a row's preview box: [`GooeySkin::preview_edge`] (the field), else
    /// [`GooeySkin::outline`].
    pub fn preview_edge(&self) -> Color {
        self.preview_edge.unwrap_or(self.outline)
    }

    /// [`GooeySkin::held_fade`] held to 0 to 1 (a NaN is no fade), so a bad value never extrapolates
    /// a colour; [`GooeySkin::problems`] reports it.
    fn fade(&self) -> f32 {
        if self.held_fade.is_nan() {
            0.0
        } else {
            self.held_fade.clamp(0.0, 1.0)
        }
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

    /// The cover of a [`crate::Cooldown`]'s unrecovered wedge: [`GooeySkin::background`], the
    /// colour of the screen behind the interface, at `COOLDOWN_DIM_ALPHA`: the widget sinks back
    /// into the screen. A cover in the widget's own surface would be the same value as the widget.
    pub(crate) fn cooldown_dim(&self) -> Color {
        let [r, g, b] = rgb(self.background);
        Color::srgba(r, g, b, COOLDOWN_DIM_ALPHA)
    }

    /// The thin hand at the sweep's leading edge, at `COOLDOWN_EDGE_ALPHA`: whichever of
    /// [`GooeySkin::text`] and [`GooeySkin::solid_surface`] stands further from the dim, so it is
    /// light in a dark skin and in a parchment one, where the text is dark like the dim.
    pub(crate) fn cooldown_edge(&self) -> Color {
        let dim = rgb(self.background);
        let [text, surface] = [rgb(self.text), rgb(self.solid_surface())];
        let [r, g, b] = if contrast_ratio(text, dim) >= contrast_ratio(surface, dim) {
            text
        } else {
            surface
        };
        Color::srgba(r, g, b, COOLDOWN_EDGE_ALPHA)
    }

    /// The chip a [`crate::Cooldown`]'s seconds are drawn on: [`GooeySkin::solid_surface`] at
    /// `COOLDOWN_CHIP_ALPHA`.
    fn cooldown_chip(&self) -> Color {
        self.solid_surface().with_alpha(COOLDOWN_CHIP_ALPHA)
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
                "accent_text on primary hovered",
                rgb(self.accent_text),
                rgb(self.primary_hovered()),
                TEXT_CONTRAST,
            ),
            (
                "accent_text on primary pressed",
                rgb(self.accent_text),
                rgb(self.primary_pressed()),
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
            ("text on pressed", text, rgb(self.pressed()), TEXT_CONTRAST),
            ("text on card", text, rgb(self.card()), TEXT_CONTRAST),
            (
                "unchecked on surface",
                rgb(self.unchecked()),
                surface,
                NON_TEXT_CONTRAST,
            ),
            (
                "preview_edge on surface",
                rgb(self.preview_edge()),
                surface,
                NON_TEXT_CONTRAST,
            ),
            // The chip over an icon as dark, and as light, as an icon can be: the worse of the two.
            (
                "text on cooldown chip",
                text,
                [[0.0; 3], [1.0; 3]]
                    .map(|icon| composite(surface, COOLDOWN_CHIP_ALPHA, icon))
                    .into_iter()
                    .min_by(|a, b| contrast_ratio(text, *a).total_cmp(&contrast_ratio(text, *b)))
                    .unwrap_or(surface),
                TEXT_CONTRAST,
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
        // Every button variant's ink on its fill, at rest, hovered and pressed.
        let variants = self
            .variant_pairs()
            .into_iter()
            .map(|(pair, ink, backing)| (pair, ink, backing, TEXT_CONTRAST));
        for (pair, a, b, needed) in pairs.into_iter().chain(edge).chain(variants) {
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
        // Pressed against hovered, the pair a recessed row relies on: not a contrast (WCAG 1.4.11
        // asks none of two states that do not appear side by side) but a lightness step.
        let delta = (cie_lightness(rgb(self.pressed())) - cie_lightness(rgb(self.hovered()))).abs();
        // A NaN colour gives a NaN step, which is reported, not passed.
        if delta.is_nan() || delta < STATE_STEP {
            out.push(SkinProblem::Indistinguishable {
                pair: "pressed and hovered",
                delta_l: delta,
                needed: STATE_STEP,
            });
        }
        // Every variant's fills, each a step from the next: the button answers the pointer.
        for (pair, delta) in self.variant_steps() {
            if delta.is_nan() || delta < STATE_STEP {
                out.push(SkinProblem::Indistinguishable {
                    pair,
                    delta_l: delta,
                    needed: STATE_STEP,
                });
            }
        }
        // A primary button's three fills, each a step from the next: the lift under the pointer and
        // the press are how the player sees the button answer.
        let lightness = |c: Color| cie_lightness(rgb(c));
        for (pair, a, b) in [
            (
                "primary at rest and hovered",
                self.accent,
                self.primary_hovered(),
            ),
            (
                "primary hovered and pressed",
                self.primary_hovered(),
                self.primary_pressed(),
            ),
        ] {
            let delta = (lightness(a) - lightness(b)).abs();
            if delta.is_nan() || delta < STATE_STEP {
                out.push(SkinProblem::Indistinguishable {
                    pair,
                    delta_l: delta,
                    needed: STATE_STEP,
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
        if !(0.0..=1.0).contains(&self.held_fade) {
            // Out of range or NaN (a NaN is in no range).
            out.push(SkinProblem::BadSize { field: "held_fade" });
        }
        if let Some(field) = self.motion.problem() {
            out.push(SkinProblem::BadMotion { field });
        }
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
        for field in self.elevation.unusable() {
            out.push(SkinProblem::BadSize { field });
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

    /// The height of one row of a [`crate::table`], logical px: a button's, so a thumb or a pad's
    /// focus jump hits it, and a whole number of pixels so ten thousand of them add up to what the
    /// layout lays out. Every row is exactly this tall: it is the one fact that lets a table draw
    /// only the rows in view.
    pub fn table_pitch(&self) -> f32 {
        let text = self.text_size * self.text_scale;
        (text * LIST_LINE_TEXT_SIZES + self.pad() * LIST_PADDING_FACTOR)
            .max(self.target())
            .ceil()
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
            hover: look.hover,
            pressed: look.pressed,
            held: look.held,
            held_ground: look.held_ground,
            unchecked: look.unchecked,
            card: look.card,
            preview_edge: look.preview_edge,
            elevation: look.elevation.clone(),
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
                skin.hovered(),
                skin.pressed(),
                skin.held(),
                skin.held_ground(),
                skin.unchecked(),
                skin.card(),
                skin.preview_edge(),
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

/// How much of the ink the unlit three quarters of a busy button's spinner keep.
const SPINNER_TRACE_ALPHA: f32 = 0.35;

/// Which look a button's state is showing.
fn phase_of(state: &State) -> ButtonPhase {
    if state.pressed {
        ButtonPhase::Pressed
    } else if state.hovered || state.open {
        ButtonPhase::Hover
    } else {
        ButtonPhase::Rest
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
    /// A size field is not finite, or is at or below zero where it must be positive, or below zero;
    /// [`GooeySkin::held_fade`] is outside 0 to 1.
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
    /// Two state fills a reader must tell apart differ by less than `needed` in CIE L\*: the
    /// skin's pressed fill and its hovered fill ([`STATE_STEP`]), so a recessed row would look the
    /// same under the pointer as with it down.
    Indistinguishable {
        /// Which two, as "`<a>` and `<b>`".
        pair: &'static str,
        /// The lightness step they have.
        delta_l: f32,
        /// The step they need.
        needed: f32,
    },
    /// A [`GooeySkin::motion`] field cannot be used: a spring whose duration is not above zero or
    /// whose bounce is not between −1 and 1, or a fade length that is negative or not finite.
    BadMotion {
        /// The field's name, such as `motion.default`.
        field: &'static str,
    },
    /// A pair given to [`GooeySkin::tone_problems`] names a colour the skin does not hold: neither
    /// one of its [`GooeySkin::tones`] nor one of its own colours. The pair is not checked.
    UnknownTone {
        /// The name that matched nothing.
        name: String,
    },
    /// A seed given to [`GooeySkin::from_seed`] is not opaque: a skin is derived from three
    /// opaque colours.
    TranslucentSeed {
        /// Which seed: `background`, `text` or `accent`.
        seed: &'static str,
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
                "{field} is not a usable size: it must be finite, and above zero for a text size, text_scale, min_target or meter_height, from 0 to 1 for held_fade, at least zero otherwise"
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
            Self::TranslucentSeed { seed } => {
                write!(
                    f,
                    "the {seed} seed is not opaque: a skin is derived from opaque colours"
                )
            }
            Self::BadMotion { field } => write!(
                f,
                "{field} is not usable motion: a spring needs a duration above zero and a bounce between -1 and 1, a fade a length of zero or more"
            ),
            Self::BadScale { field } => write!(
                f,
                "{field} is not a usable scale: it must have a step (surfaces), be finite and never fall (spacing, radii, type_scale), and be above zero (type_scale)"
            ),
            Self::Indistinguishable {
                pair,
                delta_l,
                needed,
            } => write!(
                f,
                "{pair} differ by ΔL* {delta_l:.2}; two states a player must tell apart need at least {needed}"
            ),
            Self::UnknownTone { name } => write!(
                f,
                "\"{name}\" is neither a tone of the skin nor one of its own colours"
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

/// Marks a [`crate::button`] as the screen's main action: [`apply_skin`] fills it with
/// [`GooeySkin::accent`] and labels it in [`GooeySkin::accent_text`] at rest,
/// [`GooeySkin::primary_hovered`] under the pointer and [`GooeySkin::primary_pressed`] held down; a
/// disabled one is dimmed like any button. [`crate::primary_button`] spawns a button with it;
/// inserting or removing it restyles the button, so the main action can move.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct PrimaryButton;

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
#[require(ColorTrack)]
pub enum Role {
    /// A full-window layout root; not drawn.
    Screen,
    /// A framed, padded column on [`GooeySkin::surface`].
    Panel,
    /// A column filled with one step of [`GooeySkin::surfaces`], lowest first; nothing else is
    /// drawn (see [`crate::surface`]).
    Surface(u8),
    /// A node filled with the host's own tone of that name, [`GooeySkin::tone`], and nothing else
    /// (give it its own `Node`): restyled in the frame the skin changes. A name the skin does not
    /// hold draws no fill at all, transparent, never a stand-in colour.
    Tone(&'static str),
    /// A node filled with [`GooeySkin::hovered`], a host's own row, chip or field under the
    /// pointer, and nothing else (give it its own `Node`). The host swaps it with [`Role::Pressed`]
    /// as the pointer goes down and up; the frame after, it is restyled.
    Hovered,
    /// A node filled with [`GooeySkin::pressed`], a host's own control with the pointer down,
    /// recessed against [`Role::Hovered`] (checked: [`SkinProblem::Indistinguishable`]).
    Pressed,
    /// A node filled with [`GooeySkin::held`]: a gizmo or glyph while the key that asks the world
    /// to step back is held. It composes with [`Role::HeldGround`] by one
    /// [`GooeySkin::held_fade`].
    Held,
    /// A node filled with [`GooeySkin::held_ground`]: the ground that steps back with
    /// [`Role::Held`].
    HeldGround,
    /// A node filled with [`GooeySkin::unchecked`]: the mark of a thing not judged yet.
    Unchecked,
    /// A node filled with [`GooeySkin::card`]: the ground of a card the scrim leaves lit.
    Card,
    /// A node whose border is [`GooeySkin::preview_edge`] and that has no fill: the edge of a
    /// row's preview box (give it its own `Node` with a border width).
    PreviewEdge,
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
    /// A meter's trail: the length the fill just lost, behind it in [`GooeySkin::danger`] while it
    /// holds and drains. Zero wide at rest.
    MeterTrail,
    /// The track under a [`crate::hold_button`]'s label: a [`Role::Meter`]'s frame and height with
    /// no least width, so the button is as wide as its label.
    HoldTrack,
    /// The fill of a [`Role::HoldTrack`], as wide as the hold has come, one border inside it.
    HoldFill,
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
    /// A [`crate::ShortcutBadges`] badge: a small chip with a chord's words on the right of the
    /// widget it names, inside its frame, in the smallest step of [`GooeySkin::type_scale`]. It is
    /// the inverse of a panel: [`GooeySkin::text`] as the fill and the surface under a panel's
    /// text as the ink (the pair [`GooeySkin::problems`] holds at 4.5:1), so it stands off the
    /// fill of any button, hovered, pressed or lit, in every look.
    ShortcutBadge,
    /// The card a [`crate::ShortcutLegend`] draws in the middle of the window while the shortcut
    /// overlay is up: [`GooeySkin::solid_surface`] in an [`GooeySkin::outline`] frame (opaque, so
    /// its words read over any empty ground), a grid of keycaps and their words.
    ShortcutLegend,
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
    /// A [`crate::table`]'s root: a header over the viewport; not drawn.
    Table,
    /// A table's header strip: a row of [`Role::TableHeaderCell`]s, as wide as its rows (the same
    /// lane on its right as the rail's); not drawn.
    TableHeader,
    /// One column's header: a button's look and size with the column's title, the sort mark
    /// after it, and the focus ring.
    TableHeaderCell,
    /// The mark of an ascending sort, after a header's title: a chevron pointing up in the
    /// title's colour.
    TableSortUp,
    /// The mark of a descending sort: a chevron pointing down.
    TableSortDown,
    /// A column's resize edge: [`crate::MIN_TARGET_PX`] wide over the border between two headers,
    /// a [`Role::SplitGrip`] in its middle.
    TableEdge,
    /// A table's viewport: a clipped column [`crate::TableRows`] rows tall, with a lane on its
    /// right for the rail; the same as a list's.
    TableView,
    /// The column of the pool's rows inside the viewport, padded above and below by the height
    /// of the rows not drawn; not drawn.
    TableRows,
    /// One row of the pool: no fill at rest (a quiet band on every second row), [`GooeySkin::hovered`]
    /// under the pointer, [`GooeySkin::accent`] while selected, the focus ring while it is the
    /// cursor of a focused table.
    TableRow,
    /// One cell's box: a column wide, one line high, cutting what does not fit; not drawn.
    TableCell,
    /// A cell's text: [`GooeySkin::text`], or [`GooeySkin::accent_text`] on a selected row.
    TableCellText,
    /// A [`crate::tree`] row's indent and click target, a row high and [`crate::tree_indent`]
    /// wide; not drawn.
    TreeTwisty,
    /// The chevron of a closed node, pointing right, in the row's ink.
    TreeClosed,
    /// The chevron of an open node, pointing down, in the row's ink.
    TreeOpen,
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
    /// [`GooeySkin::focus_ring`], let through over the dock's panels and framed in it.
    DockDrop,
    /// The title of a tab being carried, beside the pointer: an opaque card in the ring's frame.
    DockGhost,
    /// What a [`crate::DragPayload`] source carries, beside the pointer: an opaque card in the
    /// ring's frame.
    DragGhost,
    /// The same card while the node under the pointer refuses what is carried: framed in
    /// [`GooeySkin::danger`].
    DragGhostRefused,
    /// The mark on the [`crate::DropTarget`] under the pointer: [`GooeySkin::focus_ring`] let
    /// through, and framed in it.
    DropMark,
    /// The mark while the target refuses what is carried: [`GooeySkin::danger`], let through and
    /// framed.
    DropMarkRefused,
    /// A [`crate::tabs`] widget's column: the strip, a gap, the pages; not drawn.
    Tabs,
    /// The strip of tabs, in a row a [`GooeySkin::gap`] apart; not drawn.
    TabList,
    /// One tab: a button's size and look, lit [`GooeySkin::accent`] while it is the selected one.
    Tab,
    /// A tab's label: [`GooeySkin::accent_text`] on the selected tab, else [`GooeySkin::text`].
    TabText,
    /// A [`crate::radial`]'s disc: a round [`GooeySkin::surface`] with the items on a ring inside
    /// it, its border the focus ring. As wide as it is high, in text sizes, so it follows the
    /// player's text scale.
    Radial,
    /// The quiet circle the items of a radial stand on: a line in [`GooeySkin::surface_raised`],
    /// drawn behind them, so the ring reads as a ring and not as eight floating labels.
    RadialTrack,
    /// One item of a radial: a button's fill and size, centred on its place on the ring, lit
    /// [`GooeySkin::accent`] while it is the highlighted one.
    RadialSlot,
    /// An item's label: [`GooeySkin::accent_text`] on the highlighted item, else
    /// [`GooeySkin::text`].
    RadialText,
    /// An item's picture ([`crate::RadialItem::icon`]): 3 of the wheel's text sizes square, the
    /// image tinted like an [`Role::Icon`] (the accent's text on the highlighted item).
    RadialIcon,
    /// The highlighted item's name, in the middle of the disc in the heading's size.
    RadialCentre,
    /// A [`crate::slot_grid`]'s grid: its slots in a row-and-column grid half a
    /// [`GooeySkin::gap`] apart; not drawn.
    SlotGrid,
    /// A slot that holds an item: a button's fill, [`GooeySkin::text_size`] times four square, the
    /// focus ring while it has focus, the accent while pressed.
    Slot,
    /// An empty slot: the panel's fill with its quiet frame, the same size as a filled one.
    SlotEmpty,
    /// A slot's item name: drawn only while the slot has no picture.
    SlotName,
    /// A slot's count, a chip on the opaque surface in its bottom-right corner, drawn above one.
    SlotCount,
    /// A slot's picture, 60% of the slot's side, tinted like an [`Role::Icon`].
    SlotIcon,
    /// A [`crate::card_grid`]: its cards in a row a [`GooeySkin::gap`] apart that wraps to fill its
    /// width; not drawn.
    CardGrid,
    /// A card: a column 14 body text sizes wide on the opaque surface in an
    /// [`GooeySkin::outline`] frame; [`GooeySkin::hovered`] under the pointer, the accent when
    /// chosen or pressed, the panel's fill when disabled, the focus ring while it has focus.
    CardCell,
    /// A [`crate::side_tabs`] tab's badge at its end ("3"): a pill of the opaque surface with the
    /// text's ink in the skin's small size, the same on a lit tab.
    TabBadge,
    /// A card's picture across its width at 16:9, on a [`GooeySkin::surface_raised`] placeholder;
    /// half transparent on a disabled card.
    CardImage,
    /// A card's name, in the body's size and [`GooeySkin::text`].
    CardName,
    /// A card's quiet line, in the skin's small size and [`GooeySkin::text_muted`] (the text on a
    /// hovered card, the accent's text on a chosen one).
    CardLine,
    /// A [`crate::Cooldown`]'s cover: the owner's whole padding box, dimmed from twelve o'clock
    /// round to where the clear wedge has opened; the sweep is data, written by
    /// [`crate::update_cooldowns`].
    Cooldown,
    /// The seconds left on a [`Role::Cooldown`], on a chip of the opaque surface in its middle.
    CooldownText,
    /// The art layer of a framed widget ([`crate::GooeyFrames`]): fills it, draws under its other
    /// children; the tint is data written by [`crate::update_frames`].
    FrameArt,
    /// An [`crate::led`]'s row: its dot, its label and its state word, a half gap apart.
    Led,
    /// An LED's dot: a square of `LED_TEXT_SIZES` body text sizes, its fill, frame, silhouette and
    /// glow data written by [`crate::update_leds`] from the [`crate::LedState`].
    LedDot,
    /// The one grid cell every page stands in, so the widget is as large as its largest page; not
    /// drawn.
    TabPages,
    /// One page, a column of widgets a [`GooeySkin::gap`] apart; hidden unless its tab is selected.
    TabPage,
    /// A [`crate::text_input`] row: a label and a field on no fill, laid out like a toggle's row.
    TextInput,
    /// A [`crate::text_area`] row: a label over a field several lines high, on no fill.
    TextArea,
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
    /// A menu bar title's words: a button's text colour (at rest, pressed, disabled) on the
    /// title's own fill, under a role of its own so that a host counting [`Role::ButtonText`] to
    /// find a dialog's answers does not count the bar.
    MenuTitleText,
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
    /// One component of a [`crate::vec_field`]: a text row with no frame and no padding on its right,
    /// its label (the axis letter) the scrub handle on its left.
    NumberPart,
    /// The row of a [`crate::vec_field`]'s components, a gap apart; not drawn.
    NumberParts,
    /// The button of a [`crate::color_field`] that opens its picker: a target square in the
    /// outline's frame, the ring when focused.
    ColorSwatch,
    /// One half of a colour swatch, filled with the colour itself (data, not a skin colour).
    ColorSwatchHalf,
    /// The picker's saturation and value area: a frame, a focus ring, and inside it the colour's
    /// own hue and its gradients (data, not a skin colour).
    ColorArea,
    /// A [`crate::viewport`]: an unfilled frame in [`GooeySkin::outline`], [`GooeySkin::focus_ring`]
    /// while it has focus; what it shows is the host's.
    Viewport,
    /// The ring on the picker's area, outside: the panel's colour, so the ring shows on any fill.
    ColorAreaMarker,
    /// The ring on the picker's area, inside: the text colour.
    ColorAreaMarkerInner,
    /// A picker's strip and the label over it: a column, the label outside the slider so the
    /// slider's focus ring never touches the text and the label starts where the area does.
    ColorStripBox,
    /// A colour strip's rail, like a slider's rail but with a gradient of the colour's own for its
    /// fill.
    ColorRail,
    /// An [`crate::icon`]: a square a step of [`GooeySkin::type_scale`] on a side, its image
    /// tinted by its [`crate::IconTone`].
    Icon,
    /// The message of a [`crate::FieldProblem`], under its row: absolute, so it takes no room of
    /// its own (the row keeps the room below itself).
    FieldMessage,
    /// The bar at the start of a problem's message, in the problem's colour.
    FieldMark,
    /// A [`crate::progress`]'s root: a column of a meter's track and a line of words.
    Progress,
    /// The words under a progress bar: body text.
    ProgressLine,
    /// A [`crate::spinner`]: a ring of [`GooeySkin::outline`] with one quarter in the
    /// [`GooeySkin::accent`].
    Spinner,
    /// A [`crate::toolbar`]'s root: a full-width row of items on the menu bar's solid surface, one
    /// quiet line under it.
    Toolbar,
    /// The line between two groups of a toolbar: a vertical rule of [`GooeySkin::outline`].
    ToolbarSeparator,
    /// A [`crate::toggle_group`]'s row of options; not drawn.
    ToggleGroup,
    /// One option of a toggle group: a tab's look, lit [`GooeySkin::accent`] while it is the one
    /// on.
    GroupOption,
    /// A [`crate::status_bar`]'s root: a full-width row on the menu bar's surface, one quiet line
    /// over it.
    StatusBar,
    /// One side of a status bar: a row of items; not drawn.
    StatusGroup,
    /// One [`crate::status_item`]: a button's states without its fill at rest, a target high.
    StatusItem,
    /// The mark at the start of a status item of a [`crate::StatusKind`] other than info: a bar
    /// in [`GooeySkin::accent`] or [`GooeySkin::danger`].
    StatusMark,
    /// A [`crate::section`]'s root: a column of its header, its body and its note; not drawn.
    Section,
    /// A section's body: the game's widgets, indented under the header; not drawn.
    SectionBody,
    /// A `inspector`'s root: a column of the rows built from a reflected value, a gap
    /// apart; not drawn.
    Inspector,
    /// A [`crate::notification_center`]'s root: a column holding the log's list; not drawn.
    NotificationCenter,
    /// A button in a toast: a button's look, fading with its toast. Part of a toast that was given
    /// one with [`crate::ToastBundle::action`] or [`crate::ToastBundle::sticky`].
    ToastAction,
    /// The words of a toast's button.
    ToastActionText,
    /// A section's "Empty": [`GooeySkin::text_muted`], in the body's place while it holds nothing.
    SectionNote,
}

type Styled = (
    Entity,
    &'static Role,
    Option<&'static mut Node>,
    Option<&'static mut BackgroundColor>,
    Option<&'static mut BorderColor>,
    Option<&'static mut TextColor>,
    Option<&'static mut TextFont>,
    // A box is recognised by *what it is* (a tooltip's box, or a popover's panel), never by carrying a
    // `Popover`: the placement below is written by inserting one, so a box can be spawned without it and
    // Bevy's `position_popover` never sees an unlaid-out anchor (which it panics on — iteration 97).
    Option<(
        Option<&'static mut Popover>,
        Option<&'static TooltipBox>,
        Option<(&'static PopoverPanel, Option<&'static PopoverSlide>)>,
    )>,
    Option<&'static mut BoxShadow>,
    Option<&'static ComputedUiRenderTargetInfo>,
    Option<&'static mut TextCursorStyle>,
    Option<&'static mut ImageNode>,
    Option<&'static IconTone>,
    Option<&'static IconSize>,
    Option<&'static PresenceAlpha>,
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

/// The skin `entity` is styled from: its own [`SkinScope`], else the nearest one above it, else
/// `skin`. For a system outside [`apply_skin`] that writes a colour as data.
pub(crate) fn skin_of<'a>(
    skin: &'a GooeySkin,
    entity: Entity,
    scopes: &'a Query<&SkinScope>,
    parents: &Query<&ChildOf>,
) -> &'a GooeySkin {
    std::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .find_map(|e| scopes.get(e).ok())
        .map_or(skin, |scope| &scope.0)
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
    /// A row of a table that is an odd one of the display, drawn with a quiet band.
    band: bool,
    /// A drop target that takes what is being carried: a slot draws it framed.
    candidate: bool,
    /// A drop target that refuses what is being carried: a slot draws it dimmed.
    refusing: bool,
    /// The kind of the [`crate::FieldProblem`] on a text row, if it has one.
    problem: Option<FieldProblemKind>,
    /// How tall the problem's message stands, logical px.
    problem_room: f32,
    /// The kind of a status item.
    status: Option<StatusKind>,
    /// Where a prompts legend stands.
    anchor: PromptsAnchor,
    /// The entity is a status item: a progress in it is a short bar.
    status_item: bool,
    /// A button that is the screen's main action ([`PrimaryButton`]) or a [`ButtonVariant::Submit`].
    primary: bool,
    /// What the button says it does ([`ButtonVariant`]); a [`PrimaryButton`] is a Submit.
    variant: ButtonVariant,
    /// The button is busy ([`crate::Loading`]): its spinner draws in its ink.
    holding: bool,
    /// A selection indicator is travelling over this option's strip ([`Indicating`]): the option
    /// draws clear, so the indicator is what shows.
    indicating: bool,
    /// The control's own size, when it has one ([`ControlSize`]).
    control: Option<ControlSize>,
    /// A text area: its field holds lines, so a size is its least height and not one line's centre.
    multi_line: bool,
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
            band: false,
            candidate: false,
            refusing: false,
            problem: None,
            problem_room: 0.0,
            status: None,
            status_item: false,
            anchor: PromptsAnchor::default(),
            primary: false,
            variant: ButtonVariant::Neutral,
            holding: false,
            indicating: false,
            control: None,
            multi_line: false,
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
    (
        Has<MenuMarked>,
        Option<(&'static FieldProblem, &'static ProblemRoom)>,
        Option<&'static StatusKind>,
        Has<DropCandidate>,
        Has<DropRefusing>,
        Option<&'static PromptsAnchor>,
        Has<PrimaryButton>,
        Option<&'static ControlSize>,
        Option<&'static ButtonVariant>,
        Has<LoadingHold>,
        Option<&'static Indicating>,
    ),
    Option<&'static TableRow>,
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
        Or<(
            Added<MenuOpen>,
            Added<PopoverPanel>,
            Changed<PopoverSlide>,
            Added<MenuMarked>,
            Changed<TableRow>,
            Changed<FieldProblem>,
            Changed<ProblemRoom>,
            Changed<IconTone>,
            Changed<IconSize>,
            Changed<StatusKind>,
            Added<DropCandidate>,
            Added<DropRefusing>,
            Changed<PromptsAnchor>,
            Added<PrimaryButton>,
            Or<(
                Changed<PresenceAlpha>,
                Changed<ControlSize>,
                Changed<ButtonVariant>,
                Added<LoadingHold>,
                Changed<Indicating>,
            )>,
        )>,
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
    RemovedComponents<'w, 's, FieldProblem>,
    RemovedComponents<'w, 's, DropCandidate>,
    RemovedComponents<'w, 's, DropRefusing>,
    RemovedComponents<'w, 's, PrimaryButton>,
    RemovedComponents<'w, 's, ControlSize>,
    RemovedComponents<'w, 's, ButtonVariant>,
    RemovedComponents<'w, 's, LoadingHold>,
);

/// Whether a role is a widget whose parts follow its state: they are restyled with it.
fn owns_parts(role: Option<&Role>) -> bool {
    matches!(
        role,
        Some(
            Role::Button
                | Role::CardCell
                | Role::Toggle
                | Role::Slider
                | Role::Choice
                | Role::Binding
                | Role::Radial
                | Role::Toast
                | Role::TextInput
                | Role::TextArea
                | Role::PaletteField
                | Role::NumberPart
                | Role::MenuItem
                | Role::PaletteRow
                | Role::TableRow
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
/// its subtree. Lines and text sizes are set at the scale Bevy last computed
/// for the node, so the frame a window is resized draws them at the old scale's snapping and the next frame
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
    roles: Query<&Role>,
    anchors: Query<&ComputedNode>,
    mut commands: Commands,
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
    let (
        unpressed,
        enabled,
        unchecked,
        unselected,
        stopped,
        closed,
        unmarked,
        unscoped,
        orphaned,
        unproblemed,
        undropped,
        unrefused,
        unprimed,
        unsized_controls,
        unvaried,
        unheld,
    ) = &mut removed;
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
                    (
                        marked,
                        problem,
                        status,
                        candidate,
                        refusing,
                        anchor,
                        primary,
                        control,
                        variant,
                        holding,
                        indicating,
                    ),
                    row,
                    role,
                )| {
                    State {
                        // `bevy_ui_widgets` marks a slider pressed for a click on its track only; a grab
                        // of the thumb is a drag, so a dragged slider reads as pressed too, and so does
                        // a dragged scrollbar thumb.
                        pressed: pressed
                            || drag.is_some_and(|d| d.dragging)
                            || bar.is_some_and(|d| d.dragging),
                        disabled: disabled && !holding,
                        holding,
                        indicating: indicating.is_some_and(|mark| mark.0),
                        hovered: hovered.is_some_and(|h| h.0),
                        checked,
                        selected: selected || (checked && role == Some(&Role::GroupOption)),
                        scrim: role == Some(&Role::Scrim),
                        alpha: toast.map_or(1.0, Toast::alpha),
                        alert: toast.is_some_and(|t| t.kind() == ToastKind::Alert),
                        grow: toast.map_or(1.0, Toast::grow),
                        body_px: toast.map_or(0.0, Toast::body_px),
                        at_start: choice.is_some_and(|(value, _)| value.0 == 0),
                        at_end: choice.is_some_and(|(value, count)| value.0 + 1 >= count.0),
                        rows: list.map_or(0.0, |list| list.rows),
                        capturing,
                        focused: look.focused == Some(entity)
                            || row.is_some_and(|r| r.cursor && look.focused == Some(r.table)),
                        open,
                        marked,
                        candidate,
                        refusing,
                        problem: problem.map(|(problem, _)| problem.kind),
                        problem_room: problem.map_or(0.0, |(_, room)| room.0),
                        status: status.copied(),
                        anchor: anchor.copied().unwrap_or_default(),
                        status_item: role == Some(&Role::StatusItem),
                        band: row.is_some_and(|r| r.index % 2 == 1),
                        primary: primary || variant.is_some_and(|v| *v == ButtonVariant::Submit),
                        variant: variant.copied().unwrap_or(if primary {
                            ButtonVariant::Submit
                        } else {
                            ButtonVariant::Neutral
                        }),
                        control: control.copied(),
                        multi_line: role == Some(&Role::TextArea),
                    }
                },
            )
            .unwrap_or_default()
    };
    // A part of a toggle or a slider, however deep, follows that toggle or slider's state.
    let owner_of = |entity: Entity| {
        let is_owner = |a: &Entity| {
            states.get(*a).is_ok_and(|(.., role)| {
                matches!(
                    role,
                    Some(
                        Role::Toggle
                            | Role::Slider
                            | Role::Choice
                            | Role::Binding
                            | Role::Radial
                            | Role::TextInput
                            | Role::TextArea
                            | Role::PaletteField
                            | Role::NumberPart
                            | Role::Toast
                            | Role::MenuItem
                            | Role::PaletteRow
                            | Role::TableRow
                            | Role::TableHeaderCell
                            | Role::TableEdge
                            | Role::SplitDivider
                    )
                )
            })
        };
        let mut owners = parents.iter_ancestors(entity).filter(is_owner);
        let Some(first) = owners.next() else {
            return State::default();
        };
        let mut owner = state_of(first);
        // A part of a part (a vec field's component inside the vec field) takes the size the
        // outer control was given.
        if owner.control.is_none() {
            owner.control = owners.find_map(|a| state_of(a).control);
        }
        owner
    };

    if skin.is_changed() || fonts.is_changed() {
        unpressed.clear();
        enabled.clear();
        unchecked.clear();
        unselected.clear();
        stopped.clear();
        closed.clear();
        unmarked.clear();
        unproblemed.clear();
        undropped.clear();
        unrefused.clear();
        unprimed.clear();
        unsized_controls.clear();
        unvaried.clear();
        unheld.clear();
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
                on_screen(parent, &roles, &parents),
                parent.is_some_and(|p| {
                    anchors
                        .get(p)
                        .is_ok_and(|node| node.size().x > 1.0 && node.size().y > 1.0)
                }),
                &mut commands,
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
    dirty.extend(unproblemed.read());
    dirty.extend(undropped.read());
    dirty.extend(unrefused.read());
    dirty.extend(unprimed.read());
    dirty.extend(unsized_controls.read());
    dirty.extend(unvaried.read());
    dirty.extend(unheld.read());
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
                            | Role::Radial
                            | Role::Slot
                            | Role::SlotEmpty
                            | Role::CardCell
                            | Role::Tab
                            | Role::GroupOption
                            | Role::StatusItem
                            | Role::ToastAction
                            | Role::TextField
                            | Role::MenuItem
                            | Role::MenuTitle
                            | Role::TableRow
                            | Role::TableHeaderCell
                            | Role::SplitDivider
                            | Role::ColorSwatch
                            | Role::ColorArea
                            | Role::Viewport
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
            on_screen(parent, &roles, &parents),
            parent.is_some_and(|p| {
                anchors
                    .get(p)
                    .is_ok_and(|node| node.size().x > 1.0 && node.size().y > 1.0)
            }),
            &mut commands,
        );
    }
}

/// The colour a [`crate::FieldProblem`] of `kind` is drawn in: the bar beside its message and the
/// field's frame.
fn problem_colour(skin: &GooeySkin, kind: Option<FieldProblemKind>) -> Color {
    match kind {
        Some(FieldProblemKind::Error) => skin.danger,
        Some(FieldProblemKind::Warning) => skin.accent,
        None => Color::NONE,
    }
}

/// Whether `parent` is a screen or a plain layout node standing on one: the screen's own card, as a
/// game writes `screen() { Node { .. } children![panel()] }` (the gallery does exactly that).
fn on_screen(parent: Option<Entity>, roles: &Query<&Role>, parents: &Query<&ChildOf>) -> bool {
    let Some(parent) = parent else {
        return false;
    };
    if roles.get(parent) == Ok(&Role::Screen) {
        return true;
    }
    if roles.get(parent).is_ok() {
        return false;
    }
    parents
        .get(parent)
        .is_ok_and(|up| roles.get(up.parent()) == Ok(&Role::Screen))
}

#[allow(clippy::too_many_arguments)]
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
        shadow,
        target,
        cursor,
        mut image,
        tone,
        icon_size,
        presence,
    ): bevy::ecs::query::QueryItem<'_, '_, Styled>,
    look: &Look,
    state: State,
    parent: Option<State>,
    owner: State,
    // The node's parent is a `Role::Screen` or a plain layout node under one: a panel there is the
    // screen's own card, the only panel the window's width can crowd (a dock group's, a split pane's
    // and a dialog's all stand in a container that sizes them).
    parent_screen: bool,
    // The box's anchor is laid out: Bevy's `position_popover` panics on an anchor whose border is as
    // big as its size, which is what every widget looks like on the frame it is spawned.
    anchor_sized: bool,
    commands: &mut Commands,
) {
    let skin = look.skin;
    let scale = target.map_or(1.0, |t| t.scale_factor());
    // How opaque this node is drawn: the product of every `Opacity` at and above it (a fade), and
    // whole for a node outside one.
    let alpha = presence.map_or(1.0, |alpha| alpha.0);
    // Every text size on whole device pixels at the node's scale (the window's times `UiScale`),
    // rounded up, so glyphs are set at a whole pixel size and are not softened between two.
    let scaled = |size: f32| FontSize::Px(snap_text_size(size * skin.text_scale, scale));
    let tip = floating.as_ref().and_then(|(_, tip, _)| tip.copied());
    // Every line weight (borders, the focus ring, the meter's frame and gap) on whole device
    // pixels at the node's scale, so it is one weight on every side wherever the node lands.
    let line = snap_line_width(skin.border, scale);
    // The settings controls' sizes grow with the player's text size, like the meter.
    let text_px = skin.text_size * skin.text_scale;
    // The body text as it is set (`scaled`): what a line of it is high, for a height that holds
    // one. Widths keep `text_px`.
    let set_px = snap_text_size(text_px, scale);
    // A control's height: its own `ControlSize`, never under the skin's target, else the target.
    let height_of = |state: &State| {
        state
            .control
            .map_or(skin.target(), |size| size.height(skin.target()))
    };
    let control = height_of(&state);
    let owner_control = height_of(&owner);
    // The height a field's line is centred in: a text area's lines start at its top, as before.
    let field_height = if owner.multi_line {
        skin.target()
    } else {
        owner_control
    };
    // A sized control's air above and below: what centres `content` logical px in its height inside
    // its frame, so the size is the height; an unsized control keeps the skin's own air.
    let air = |content: f32, skins_own: f32| {
        if state.control.is_some() {
            ((control - content - 2.0 * line) / 2.0).max(0.0)
        } else {
            skins_own
        }
    };
    // A wheel is the player's text size until 26 of them would pass `RADIAL_WINDOW_SHARE` of the
    // window's shorter side: past that the whole wheel (disc, labels and name) stops growing, so
    // it is always whole on the screen, which 200% text on a 720p window would not be.
    let wheel_px = target.map_or(text_px, |t| {
        let size = t.logical_size();
        text_px.min(RADIAL_WINDOW_SHARE * size.x.min(size.y) / RADIAL_DIAMETER_TEXT_SIZES)
    });
    let rail = skin.meter_height * skin.text_scale;
    let thumb = Vec2::new(text_px * THUMB_TEXT_SIZES, rail * THUMB_RAIL_RATIO);
    let switch = text_px * SWITCH_TEXT_SIZES;
    let knob = (switch - 4.0 * line).max(0.0);
    let chevron = text_px * CHEVRON_TEXT_SIZES;
    // A choice's arrow: a button's width, or the chevron's own if the text has outgrown it.
    let arrow = skin.target().max(chevron * CHEVRON_ARROW_FIT);
    let in_dialog = parent.is_some_and(|p| p.scrim);
    // The room a row keeps under itself for its problem's message: the message, the line its top
    // overlaps (it is placed from the padding box) and a quarter gap of air.
    let problem_room = Val::Px(if state.problem.is_some() {
        state.problem_room + line + skin.gap * 0.25
    } else {
        0.0
    });
    let fill = match role {
        Role::Panel if in_dialog => Some(skin.solid_surface()),
        Role::Panel => Some(skin.surface),
        Role::Button if !state.disabled && !matches!(state.variant, ButtonVariant::Neutral | ButtonVariant::Submit) => {
            Some(skin.variant_fill(state.variant, phase_of(&state)))
        }
        Role::Button | Role::TableHeaderCell => Some(if state.disabled {
            skin.surface
        } else if state.primary {
            // The main action is the accent at rest and lifts from it, where a plain button
            // rests quiet and turns to the accent when pressed.
            if state.pressed {
                skin.primary_pressed()
            } else if state.hovered || state.open {
                skin.primary_hovered()
            } else {
                skin.accent
            }
        } else if state.pressed {
            skin.accent
        } else if state.hovered || state.open {
            skin.hovered()
        } else {
            skin.surface_raised
        }),
        // The disc is the panel's ground; an item rests on a button's fill and the highlighted
        // one is lit like a selected tab, two cues by luminance and not by hue.
        Role::Radial => Some(skin.surface),
        Role::RadialSlot => Some(if owner.disabled {
            skin.surface
        } else if state.selected {
            skin.accent
        } else {
            skin.surface_raised
        }),
        // A slot is a button's well: raised when it holds an item, the panel's own fill when it is
        // empty (its quiet frame is the border), the hovered fill under the pointer and the
        // accent while pressed. The count's chip is the opaque surface, whatever is under it.
        // A slot that refuses what is carried is the panel's well, items and all: only the
        // places that take it stay raised.
        // A card rests on the opaque surface so its quiet lines read at 4.5:1, lifts under the
        // pointer, and is the accent when chosen, like a selected tab.
        Role::CardCell => Some(if state.disabled {
            skin.surface
        } else if state.pressed || state.selected {
            skin.accent
        } else if state.hovered {
            skin.hovered()
        } else {
            skin.solid_surface()
        }),
        Role::CardImage => Some(skin.surface_raised),
        Role::TabBadge => Some(skin.solid_surface()),
        Role::Slot => Some(if state.refusing {
            skin.surface
        } else if state.pressed {
            skin.accent
        } else if state.hovered {
            skin.hovered()
        } else {
            skin.surface_raised
        }),
        Role::SlotEmpty => Some(if state.hovered {
            skin.surface_raised
        } else {
            skin.surface
        }),
        Role::SlotCount => Some(skin.solid_surface()),
        Role::CooldownText => Some(skin.cooldown_chip()),
        // The selected tab is lit like everything that is on (a switch, a meter's fill): the
        // accent, with the text colour inverted on it, two cues by luminance and not by hue. The
        // others are a button's, full-contrast text on a visible fill, never faded.
        // While the selection indicator travels the options are clear: its backdrop is the raised
        // fill they had, and its puck the accent.
        Role::Tab | Role::GroupOption if state.indicating && !state.disabled && !state.pressed => {
            Some(Color::NONE)
        }
        Role::Tab | Role::GroupOption => Some(if state.disabled {
            skin.surface
        } else if state.selected || state.pressed {
            skin.accent
        } else if state.hovered {
            skin.hovered()
        } else {
            skin.surface_raised
        }),
        // A status item is lit like a menu title: no fill at rest, the hovered fill under the
        // pointer or with focus, the accent while pressed.
        Role::StatusItem => Some(if state.disabled {
            Color::NONE
        } else if state.pressed {
            skin.accent
        } else if state.hovered || state.focused {
            skin.hovered()
        } else {
            Color::NONE
        }),
        // The mark says what kind the item is by a shape and a colour: none for plain state.
        Role::StatusMark => Some(match parent.and_then(|p| p.status) {
            Some(StatusKind::Warning) => skin.accent,
            Some(StatusKind::Error) => skin.danger,
            _ => Color::NONE,
        }),
        Role::ToolbarSeparator => Some(skin.outline),
        Role::Meter | Role::HoldTrack | Role::SliderRail => Some(skin.surface_raised),
        // A swatch's frame shows through its halves while the colour is mixed.
        Role::ColorSwatch => Some(skin.surface_raised),
        Role::Tooltip | Role::Popover | Role::MenuPanel | Role::MenuBar | Role::Palette
        | Role::Toolbar | Role::StatusBar => {
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
        // A toast's button is a button's fill, fading with its toast.
        Role::ToastAction => Some(faded(
            if state.disabled {
                skin.surface
            } else if state.pressed {
                skin.accent
            } else if state.hovered {
                skin.hovered()
            } else {
                skin.surface_raised
            },
            owner.alpha,
        )),
        Role::ToastBody => Some(faded(skin.solid_surface(), owner.alpha)),
        Role::ToastMark => Some(faded(
            if owner.alert {
                skin.danger
            } else {
                skin.accent
            },
            owner.alpha,
        )),
        Role::MeterFill | Role::HoldFill => Some(skin.accent),
        Role::MeterTrail => Some(skin.danger),
        Role::FieldMark => Some(problem_colour(skin, owner.problem)),
        Role::ListRail => Some(skin.surface_raised),
        // A row has no fill at rest but a quiet band on every second one (never the only mark of
        // anything): lit by the pointer like a button, and by the accent while selected, with
        // the text colour inverted on it.
        Role::TableRow => Some(if state.disabled {
            Color::NONE
        } else if state.selected {
            skin.accent
        } else if state.hovered {
            skin.hovered()
        } else if state.band {
            skin.surface_raised.with_alpha(TABLE_BAND_ALPHA)
        } else {
            Color::NONE
        }),
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
        Role::Tone(name) => Some(skin.tone(name).unwrap_or(Color::NONE)),
        Role::Hovered => Some(skin.hovered()),
        Role::Pressed => Some(skin.pressed()),
        Role::Held => Some(skin.held()),
        Role::HeldGround => Some(skin.held_ground()),
        Role::Unchecked => Some(skin.unchecked()),
        Role::Card => Some(skin.card()),
        // An edge alone: a fill left by a role the node had before is cleared.
        Role::PreviewEdge => Some(Color::NONE),
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
        Role::ShortcutBadge => Some(skin.text),
        Role::ShortcutLegend => Some(skin.solid_surface()),
        // The landing is the ring's colour let through to the panels under it, so fill and frame are one; the ghost is as opaque as
        // a dialog's card, so its title is read at the contrast of any panel's text.
        Role::DockDrop => Some(skin.focus_ring.with_alpha(DOCK_DROP_ALPHA)),
        Role::DropMark => Some(skin.focus_ring.with_alpha(DROP_MARK_ALPHA)),
        Role::DropMarkRefused => Some(skin.danger.with_alpha(DROP_MARK_ALPHA)),
        Role::DockGhost | Role::DragGhost | Role::DragGhostRefused => Some(skin.solid_surface()),
        // The field is an outlined box in the panel's own colour, so the typed text and the hint
        // meet on it the contrast they are checked at on the panel.
        Role::TextField => Some(skin.solid_surface()),
        Role::Screen
        | Role::Menu
        | Role::Heading
        | Role::Label
        | Role::ButtonText
        | Role::MenuTitleText
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
        | Role::ToastActionText
        | Role::List
        | Role::ListView
        | Role::ListRows
        | Role::Table
        | Role::TableHeader
        | Role::TableSortUp
        | Role::TableSortDown
        | Role::TableEdge
        | Role::TableView
        | Role::TableRows
        | Role::TableCell
        | Role::TableCellText
        | Role::TreeTwisty
        | Role::Section
        | Role::SectionBody
        | Role::SectionNote
        | Role::Inspector
        | Role::NotificationCenter
        | Role::TreeClosed
        | Role::TreeOpen
        | Role::Split
        | Role::SplitPane
        | Role::Dock
        | Role::DockBody
        | Role::Tabs
        | Role::TabList
        | Role::TabText
        | Role::SlotGrid
        | Role::SlotName
        | Role::SlotIcon
        | Role::CardGrid
        | Role::CardName
        | Role::CardLine
        | Role::Cooldown
        | Role::Led
        | Role::LedDot
        | Role::FrameArt
        | Role::RadialIcon
        | Role::RadialText
        | Role::RadialCentre
        | Role::RadialTrack
        | Role::TabPages
        | Role::TabPage
        | Role::TextInput
        | Role::TextArea
        | Role::PaletteField
        | Role::NumberPart
        | Role::NumberParts
        // The colour parts are filled by the colour itself, a value, never by the skin.
        | Role::ColorSwatchHalf
        | Role::ColorStripBox
        | Role::ColorArea
        | Role::Viewport
        | Role::ColorAreaMarker
        | Role::ColorAreaMarkerInner
        | Role::ColorRail
        | Role::TextCell
        | Role::TextHint
        | Role::MenuMark
        | Role::MenuCheck
        | Role::MenuText
        | Role::MenuShortcut
        | Role::MenuChevron
        | Role::MenuNote
        | Role::Icon
        | Role::FieldMessage
        | Role::Progress
        | Role::ProgressLine
        | Role::Spinner
        | Role::ToggleGroup
        | Role::StatusGroup
        | Role::PaletteCount => None,
    };
    if let (Some(fill), Some(mut background)) = (fill, background) {
        background.set_if_neq(BackgroundColor(faded(fill, alpha)));
    }

    let edge = match role {
        Role::Panel if in_dialog => Some(BorderColor::all(skin.outline)),
        // A popover floats over whatever is behind it, so its edge is the dialog's: the outline.
        Role::Popover | Role::MenuPanel | Role::Palette => Some(BorderColor::all(skin.outline)),
        Role::DockDrop | Role::DockGhost | Role::DropMark | Role::DragGhost => {
            Some(BorderColor::all(skin.focus_ring))
        }
        Role::DropMarkRefused | Role::DragGhostRefused => Some(BorderColor::all(skin.danger)),
        // The bar's one frame is the line under it, the panel's own quiet edge.
        Role::MenuBar | Role::Toolbar | Role::StatusBar => Some(BorderColor::all(
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
        | Role::HoldTrack
        | Role::ToggleTrack
        | Role::SliderRail
        | Role::ColorRail
        | Role::BindingCap
        | Role::PromptKey
        | Role::PromptButton
        | Role::ShortcutLegend => Some(BorderColor::all(skin.outline)),
        Role::ShortcutBadge => Some(BorderColor::all(skin.text)),
        Role::PreviewEdge => Some(BorderColor::all(skin.preview_edge())),
        Role::ToastAction => Some(BorderColor::all(if look.focused == Some(entity) {
            faded(skin.focus_ring, owner.alpha)
        } else {
            Color::NONE
        })),
        Role::Button | Role::Tab | Role::GroupOption | Role::StatusItem | Role::TableHeaderCell
            if look.focused == Some(entity) =>
        {
            Some(BorderColor::all(skin.focus_ring))
        }
        Role::Button if state.variant.is_framed() && !state.disabled => {
            Some(BorderColor::all(skin.outline))
        }
        Role::Button | Role::Tab | Role::GroupOption | Role::StatusItem | Role::TableHeaderCell => {
            Some(BorderColor::all(Color::NONE))
        }
        // The ring on the cursor's row; a sort mark is drawn in its header's ink.
        Role::TableRow => Some(BorderColor::all(if state.focused {
            skin.focus_ring
        } else {
            Color::NONE
        })),
        Role::TableSortUp | Role::TableSortDown => Some(BorderColor::all(menu_ink(owner, skin))),
        // A node's chevron is drawn in its borders, in the row's ink: inverted on the accent.
        // A section's header is the chevron's parent: its press inverts the ink and its disabling mutes it.
        Role::TreeClosed | Role::TreeOpen => Some(BorderColor::all(
            if owner.selected || parent.is_some_and(|p| p.pressed && !p.disabled) {
                skin.accent_text
            } else if parent.is_some_and(|p| p.disabled) {
                skin.text_muted
            } else {
                skin.text
            },
        )),
        // The ring when focused. A filled slot's frame is its own fill (a fill is drawn inside the
        // border, so a clear frame would draw the well a border's width smaller than an empty
        // one beside it), an empty one's the quiet edge.
        Role::Slot | Role::SlotEmpty | Role::CardCell if look.focused == Some(entity) => {
            Some(BorderColor::all(skin.focus_ring))
        }
        Role::CardCell => Some(BorderColor::all(skin.outline)),
        Role::Slot => Some(BorderColor::all(if state.refusing {
            skin.surface_edge.unwrap_or(skin.surface_raised)
        } else if state.candidate {
            skin.outline
        } else if state.pressed {
            skin.accent
        } else if state.hovered {
            skin.hovered()
        } else {
            skin.surface_raised
        })),
        Role::SlotEmpty => Some(BorderColor::all(if state.candidate {
            skin.outline
        } else {
            skin.surface_edge.unwrap_or(skin.surface_raised)
        })),
        Role::RadialTrack => Some(BorderColor::all(skin.surface_raised)),
        // The disc is framed like a panel and wears the ring while it has focus.
        Role::Radial => Some(BorderColor::all(if look.focused == Some(entity) {
            skin.focus_ring
        } else {
            skin.surface_edge.unwrap_or(skin.surface_raised)
        })),
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
        Role::TextInput | Role::TextArea | Role::PaletteField | Role::NumberPart => {
            Some(BorderColor::all(Color::NONE))
        }
        Role::TextField => Some(BorderColor::all(if state.disabled {
            skin.outline
        } else if look.editing == Some(entity) {
            skin.focus_ring
        } else if owner.problem == Some(FieldProblemKind::Error) {
            skin.danger
        } else if state.hovered {
            skin.text_muted
        } else {
            skin.outline
        })),
        // A quarter of the ring is lit; the others are the outline, 3:1 on the panel.
        Role::Spinner => Some(match parent {
            // In a busy button the quarter is the button's own ink, and the rest of the ring a pale
            // trace of it, so it reads on the button's fill whatever that is.
            Some(p) if p.holding => {
                let ink = skin.variant_ink(p.variant, phase_of(&p));
                let trace = ink.with_alpha(ink.alpha() * SPINNER_TRACE_ALPHA);
                BorderColor {
                    top: ink,
                    right: trace,
                    bottom: trace,
                    left: trace,
                }
            }
            _ => BorderColor {
                top: skin.accent,
                right: skin.outline,
                bottom: skin.outline,
                left: skin.outline,
            },
        }),
        Role::SliderThumb => Some(BorderColor::all(skin.solid_surface())),
        // The swatch and the area are framed in the outline and wear the ring when focused: what
        // they hold is any colour, so the frame is what a panel's 3:1 is measured on.
        Role::ColorSwatch | Role::ColorArea | Role::Viewport => {
            Some(BorderColor::all(if look.focused == Some(entity) {
                skin.focus_ring
            } else {
                skin.outline
            }))
        }
        // A ring of two tones, so it shows on any colour under it.
        Role::ColorAreaMarker => Some(BorderColor::all(skin.solid_surface())),
        Role::ColorAreaMarkerInner => Some(BorderColor::all(skin.text)),
        Role::SplitDivider => Some(BorderColor::all(if look.focused == Some(entity) {
            skin.focus_ring
        } else {
            Color::NONE
        })),
        _ => None,
    };
    if let (Some(edge), Some(mut border)) = (edge, border) {
        border.set_if_neq(faded_edge(edge, alpha));
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
                } else if parent_screen {
                    // A screen's own card never grows past the window less the legend's own safe edge
                    // on each side, so at large text it wraps rather than running off the screen or
                    // crowding the edge (2 * gap measured 12 px of margin at 150%, tighter than the
                    // legend beside it). A panel in a dock group, a split pane or a dialog is sized by
                    // its container and is left alone (iteration 93 measured a cap there breaking a
                    // dock drag).
                    if let Some(window) = target {
                        let edge = window.logical_size().x * SAFE_EDGE_PERCENT / 100.0;
                        next.max_width = Val::Px((window.logical_size().x - 2.0 * edge).max(0.0));
                    }
                }
            }
            // A disc as wide as it is high, in text sizes, so it grows with the player's text scale;
            // the middle holds the highlighted item's name and the items stand on a ring round it.
            Role::Radial => {
                let diameter = wheel_px * RADIAL_DIAMETER_TEXT_SIZES;
                next.width = Val::Px(diameter);
                next.height = Val::Px(diameter);
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = BorderRadius::MAX;
                next.flex_shrink = 0.0;
            }
            Role::RadialTrack => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = BorderRadius::MAX;
            }
            // An item is a button's size and look, its middle on its place on the ring (the place
            // and the half-size pull back are the widget's own).
            Role::RadialSlot => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad() * 1.5), Val::Px(skin.pad() * 0.6));
                next.min_width = Val::Px(skin.target());
                next.min_height = Val::Px(skin.target());
                next.justify_content = JustifyContent::Center;
                next.align_items = AlignItems::Center;
                next.flex_shrink = 0.0;
            }
            Role::CardGrid => {
                next.flex_direction = FlexDirection::Row;
                next.flex_wrap = FlexWrap::Wrap;
                next.width = Val::Percent(100.0);
                next.align_items = AlignItems::FlexStart;
                next.column_gap = Val::Px(skin.gap);
                next.row_gap = Val::Px(skin.gap);
            }
            Role::CardCell => {
                next.flex_direction = FlexDirection::Column;
                next.width = Val::Px(text_px * CARD_TEXT_SIZES);
                next.flex_shrink = 0.0;
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::all(Val::Px(skin.pad()));
                next.row_gap = Val::Px(skin.gap * 0.25);
                next.overflow = Overflow::clip();
            }
            Role::TabBadge => {
                next.padding = UiRect::horizontal(Val::Px(skin.pad() * 0.5));
                next.margin = UiRect::left(Val::Px(skin.gap));
                next.border_radius = BorderRadius::MAX;
            }
            Role::CardImage => {
                next.width = Val::Percent(100.0);
                next.aspect_ratio = Some(CARD_IMAGE_ASPECT);
                next.margin = UiRect::bottom(Val::Px(skin.gap * 0.25));
                next.border_radius = BorderRadius::all(Val::Px(skin.radius * 0.5));
            }
            Role::SlotGrid => {
                // Tracks stand at the start and are never stretched: a grid in a column wider than
                // its slots (a stretched flex child) keeps the slots together.
                next.justify_content = JustifyContent::Start;
                next.align_content = AlignContent::Start;
                next.column_gap = Val::Px(skin.gap * 0.5);
                next.row_gap = Val::Px(skin.gap * 0.5);
            }
            // A square well, a few text sizes on a side, that clips what it holds.
            Role::Slot | Role::SlotEmpty => {
                let side = Val::Px(text_px * SLOT_TEXT_SIZES);
                next.width = side;
                next.height = side;
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::all(Val::Px(skin.gap * 0.25));
                next.justify_content = JustifyContent::Center;
                next.align_items = AlignItems::Center;
                next.overflow = Overflow::clip();
                next.flex_shrink = 0.0;
            }
            Role::SlotCount => {
                next.right = Val::Px(line + 1.0);
                next.bottom = Val::Px(line + 1.0);
                next.padding = UiRect::horizontal(Val::Px(skin.gap * 0.5));
                next.border_radius = BorderRadius::all(Val::Px(skin.radius * 0.5));
            }
            // The cover is the owner's whole padding box, the chip centred in it.
            Role::Cooldown => {
                next.position_type = PositionType::Absolute;
                next.left = Val::Px(0.0);
                next.top = Val::Px(0.0);
                next.right = Val::Px(0.0);
                next.bottom = Val::Px(0.0);
                next.align_items = AlignItems::Center;
                next.justify_content = JustifyContent::Center;
                next.border_radius = radius;
            }
            Role::Led => {
                next.align_items = AlignItems::Center;
                next.column_gap = Val::Px(skin.gap * 0.5);
            }
            Role::LedDot => {
                let side = Val::Px(skin.text_size * skin.text_scale * LED_TEXT_SIZES);
                next.width = side;
                next.height = side;
                next.flex_shrink = 0.0;
                next.border = UiRect::all(Val::Px(line));
            }
            Role::CooldownText => {
                next.padding = UiRect::axes(Val::Px(skin.gap * 0.5), Val::Px(skin.gap * 0.25));
                next.border_radius = BorderRadius::all(Val::Px(skin.radius * 0.5));
            }
            Role::SlotIcon => {
                let side = Val::Px(text_px * SLOT_TEXT_SIZES * SLOT_ICON_SHARE);
                next.width = side;
                next.height = side;
                next.flex_shrink = 0.0;
            }
            // A picture's item is as wide as it is high: the negative margin takes back the
            // sides' extra padding an item with words has, so the pill round it is a square.
            Role::RadialIcon => {
                let side = Val::Px(wheel_px * RADIAL_ICON_TEXT_SIZES);
                next.width = side;
                next.height = side;
                next.margin = UiRect::horizontal(Val::Px(-skin.pad() * 0.9));
                next.flex_shrink = 0.0;
            }
            Role::Menu | Role::TabPage | Role::Tabs | Role::Inspector => {
                next.row_gap = Val::Px(skin.gap)
            }
            // A section's header stands half a gap off its body, and what is in the body is
            // indented one padding and a half, a gap apart like a panel's rows.
            Role::Section => next.row_gap = Val::Px(skin.gap * 0.5),
            Role::SectionBody => {
                next.row_gap = Val::Px(skin.gap);
                next.padding = UiRect::left(Val::Px(skin.pad() * 1.5));
            }
            Role::SectionNote => next.padding = UiRect::left(Val::Px(skin.pad() * 1.5)),
            // The strip's tabs stand a gap apart like a menu's buttons, and each is a button's size.
            // The strip keeps every row it wraps to: a dock group's page area grows under it, and a
            // strip that could shrink was cut off by its own card (measured at 200% text in a narrow
            // group: three rows, the second and third outside the card).
            Role::TabList => {
                next.column_gap = Val::Px(skin.gap);
                next.row_gap = Val::Px(skin.gap);
                next.flex_shrink = 0.0;
            }
            Role::Button
            | Role::Tab
            | Role::GroupOption
            | Role::TableHeaderCell
            | Role::ToastAction => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(
                    Val::Px(skin.pad() * 1.5),
                    Val::Px(air(set_px * LIST_LINE_TEXT_SIZES, skin.pad() * 0.6)),
                );
                // A tab shares its strip equally and never narrower than its own label and
                // padding (`Val::Auto`), so a long label is not squeezed out of its tab.
                next.min_width = if *role == Role::Tab {
                    Val::Auto
                } else {
                    Val::Px(skin.target())
                };
                // An icon beside the label stands half a gap from it; a lone label has no use for it.
                if *role == Role::Button {
                    next.column_gap = Val::Px(skin.gap * 0.5);
                }
                next.min_height = Val::Px(control);
                // A toast's button never gives up its padded label to a long message (which
                // shrinks and wraps), stands a gap from its neighbours on the row, and keeps its
                // own height beside a wrapped message instead of stretching to it.
                if *role == Role::ToastAction {
                    next.flex_shrink = 0.0;
                    next.align_self = AlignSelf::Center;
                    next.margin = UiRect::horizontal(Val::Px(skin.gap * 0.5));
                }
            }
            // A bar is a row of items centred on its line, never squeezed, with one quiet line on
            // the side that faces the rest of the screen.
            Role::Toolbar | Role::StatusBar => {
                next.border = if *role == Role::Toolbar {
                    UiRect::bottom(Val::Px(line))
                } else {
                    UiRect::top(Val::Px(line))
                };
                next.padding = UiRect::axes(Val::Px(skin.pad() * 0.5), Val::Px(skin.pad() * 0.3));
                next.column_gap = Val::Px(skin.gap * 0.5);
                next.align_items = AlignItems::Center;
                next.flex_shrink = 0.0;
            }
            Role::StatusGroup | Role::ToggleGroup => {
                next.column_gap = Val::Px(
                    skin.gap
                        * if *role == Role::ToggleGroup {
                            0.25
                        } else {
                            0.5
                        },
                );
                next.align_items = AlignItems::Center;
            }
            Role::ToolbarSeparator => {
                next.width = Val::Px(line);
                next.align_self = AlignSelf::Stretch;
                next.margin = UiRect::axes(Val::Px(skin.gap * 0.25), Val::Px(skin.pad() * 0.3));
                next.flex_shrink = 0.0;
            }
            Role::StatusItem => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad()), Val::Px(skin.pad() * 0.3));
                next.column_gap = Val::Px(skin.gap * 0.5);
                next.align_items = AlignItems::Center;
                next.justify_content = JustifyContent::Center;
                next.min_height = Val::Px(skin.target());
            }
            Role::StatusMark => {
                next.width = Val::Px(line * 2.0);
                next.align_self = AlignSelf::Stretch;
                next.flex_shrink = 0.0;
                next.border_radius = radius;
                let shown = parent.is_some_and(|p| p.status.is_some_and(|k| k != StatusKind::Info));
                next.display = if shown { Display::Flex } else { Display::None };
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
            // A hold button's bar: the meter's height and frame, no least width, a gap under the
            // label and the label's width across.
            Role::HoldTrack => {
                next.height = Val::Px(skin.meter_height * skin.text_scale);
                next.border = UiRect::all(Val::Px(line));
                next.padding = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.align_self = AlignSelf::Stretch;
                next.flex_shrink = 0.0;
                next.margin = UiRect::top(Val::Px(skin.gap * 0.5));
            }
            // CSS's content-edge radius: the track's minus its frame and gap, so the corners are
            // concentric. Bevy clips a child to a rectangle, never to its parent's curve, so the
            // fill is rounded itself.
            Role::MeterFill | Role::MeterTrail | Role::HoldFill | Role::SliderFill => {
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
                next.padding = UiRect::axes(
                    Val::Px(skin.pad() * 1.5),
                    Val::Px(air(
                        (set_px * LIST_LINE_TEXT_SIZES).max(switch),
                        skin.pad() * 0.6,
                    )),
                );
                next.min_height = Val::Px(control);
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
            Role::SliderRail | Role::ColorRail => {
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
                next.min_height = Val::Px(control);
                next.column_gap = Val::Px(skin.gap);
            }
            // As wide as a button and as high as the toggle's row inside its frame, so the row is
            // the toggle's height and a thumb has the room NN/g asks of a stepper's buttons.
            Role::ChoiceArrow => {
                next.min_width = Val::Px(arrow);
                // A sized choice's arrow fills the row inside its frame, so the row is the size.
                next.min_height = Val::Px(if owner.control.is_some() {
                    (owner_control - 2.0 * line).max(0.0)
                } else {
                    (switch + skin.pad() * 1.2).max(skin.target())
                });
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
            Role::TextInput | Role::TextArea | Role::PaletteField => {
                next.margin = UiRect::bottom(problem_room);
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
                next.min_height = Val::Px(control);
                next.column_gap = Val::Px(skin.gap);
                // A text area's label stands over its field.
                next.row_gap = Val::Px(if *role == Role::TextArea {
                    skin.gap * 0.5
                } else {
                    0.0
                });
            }
            // A vec field's component: the label is the scrub handle, so the row's own padding on
            // its left, a pad wide, makes the handle (padding, letter, gap) a target of at least 24 px
            // (WCAG 2.5.8); no frame, and none on the right, so the three fields stand a gap apart.
            Role::NumberPart => {
                next.margin = UiRect::bottom(problem_room);
                next.border = UiRect::ZERO;
                next.padding = UiRect {
                    left: Val::Px(skin.pad()),
                    right: Val::ZERO,
                    top: Val::ZERO,
                    bottom: Val::ZERO,
                };
                next.min_height = Val::Px(owner_control);
                next.column_gap = Val::Px(skin.gap * 0.5);
            }
            Role::NumberParts => next.column_gap = Val::Px(skin.gap),
            // A target square in the outline's frame, holding the colour in two halves.
            Role::ColorStripBox => next.row_gap = Val::Px(skin.gap * 0.5),
            Role::ColorSwatch => {
                next.width = Val::Px(skin.target());
                next.height = Val::Px(skin.target());
                next.flex_shrink = 0.0;
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.overflow = Overflow::clip();
            }
            // The plane fills the picker's width; the frame is the one line the pointer's position
            // is measured inside of.
            Role::Viewport => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
            }
            Role::ColorArea => {
                next.min_height = Val::Px(COLOR_AREA_HEIGHT_PX);
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
            }
            // Two rings, the panel's colour outside the text colour, so the pair shows on any fill.
            Role::ColorAreaMarker => {
                let size = Val::Px(text_px * COLOR_MARKER_TEXT_SIZES);
                next.width = size;
                next.height = size;
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = BorderRadius::MAX;
            }
            Role::ColorAreaMarkerInner => {
                next.width = Val::Percent(100.0);
                next.height = Val::Percent(100.0);
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = BorderRadius::MAX;
            }
            // One line of text in a frame, as high as the target: the text sits at the top of its
            // box, so the padding is what makes the height (the line, the padding twice and the
            // frame twice come to `min_target`) and what centres the line. A `min_height` here is
            // read as the content's, which made the field a padding and a frame too tall.
            Role::TextField => {
                let (x, y) = text_field_inset(skin, field_height, set_px, line);
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(x), Val::Px(y));
            }
            // Over the field's first character: the same inset, inside its frame.
            Role::TextHint => {
                let (x, y) = text_field_inset(skin, field_height, set_px, line);
                next.left = Val::Px(line + x);
                next.right = Val::Px(line + x);
                next.top = Val::Px(line + y);
            }
            // The legend: a bar in its parent's corner or edge (its anchor), inside the safe area, its
            // prompts a pair of paddings apart. Absolute, so the screen's layout is the same with
            // or without it. A flex-wrap here would break a line a float's width early.
            Role::Prompts => {
                let edge = Val::Percent(SAFE_EDGE_PERCENT);
                let anchor = state.anchor;
                // Only one inset on an axis, or taffy stretches the bar between the two; a
                // centred bar is `left: 50%` pulled back by `place_prompts`' translation.
                next.left = if anchor.is_centred() {
                    Val::Percent(50.0)
                } else if anchor.at_right() {
                    Val::Auto
                } else {
                    edge
                };
                next.right = if anchor.at_right() { edge } else { Val::Auto };
                next.bottom = if anchor.at_bottom() { edge } else { Val::Auto };
                next.top = if anchor.at_bottom() { Val::Auto } else { edge };
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
                let cap = set_px * LIST_LINE_TEXT_SIZES + skin.pad() * 0.6 + 2.0 * line;
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
            // A badge sits in the widget's padding box, inside its frame, so the focus ring (the
            // frame) is never covered; its text is the smallest of the type scale and its own
            // frame one device pixel, so it is smaller than the least target it stands in.
            Role::ShortcutBadge => {
                let hair = snap_line_width(1.0, target.map_or(1.0, |t| t.scale_factor()));
                next.position_type = PositionType::Absolute;
                next.right = Val::Px(skin.pad() * 0.5);
                next.align_self = AlignSelf::Center;
                next.border = UiRect::all(Val::Px(hair));
                next.border_radius = radius;
                next.padding = UiRect::axes(Val::Px(skin.pad() * 0.3), Val::Px(skin.pad() * 0.1));
                next.flex_shrink = 0.0;
            }
            // A card of rows in the middle of its parent: a pair of paddings between the keycap
            // column and the words, a gap between rows, the dialog's frame and air.
            Role::ShortcutLegend => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding = UiRect::all(Val::Px(skin.pad() * 1.5));
                next.column_gap = Val::Px(skin.pad() * PROMPT_SPACING_PADDINGS);
                next.row_gap = Val::Px(skin.gap);
                next.max_width = Val::Percent(PROMPTS_MAX_WIDTH_PERCENT);
            }
            // The rows stand `rows` of a button's height and a gap apart; to their right a lane the
            // rail's width and a gap of air, always, so rows never change width when others come
            // and go. The padding is inside the clip, so the content stops short of the rail and
            // still counts toward what scrolls.
            Role::ListView | Role::TableView => {
                // A table's rows touch and are one pitch tall; a list's are a button's height and a
                // gap apart.
                let (row, gap) = if *role == Role::TableView {
                    (skin.table_pitch(), 0.0)
                } else {
                    let row = (set_px * LIST_LINE_TEXT_SIZES
                        + skin.pad() * LIST_PADDING_FACTOR
                        + 2.0 * line)
                        .max(skin.target());
                    (row, skin.gap)
                };
                next.max_height = Val::Px(list_height(state.rows, row, gap));
                next.padding = UiRect::right(Val::Px(RAIL_LINES * line + skin.gap));
            }
            Role::ListRows => next.row_gap = Val::Px(skin.gap),
            // The header is as wide as the rows: the same lane on its right as the viewport's.
            Role::TableHeader => {
                next.padding = UiRect::right(Val::Px(RAIL_LINES * line + skin.gap));
            }
            // One pitch high, the ring inside its box so a row never changes size.
            Role::TableRow => {
                next.height = Val::Px(skin.table_pitch());
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
            }
            // The text stands where a header's title does: a frame and a padding in.
            Role::TableCell => {
                next.padding = UiRect::axes(Val::Px(skin.pad() * 1.5), Val::ZERO);
            }
            // A chevron of two borders, turned an eighth of a turn by the widget: up keeps its top
            // and left, down its bottom and right.
            Role::TableSortUp | Role::TableSortDown => {
                let size = chevron * TABLE_SORT_MARK_SCALE;
                next.width = Val::Px(size);
                next.height = Val::Px(size);
                next.margin = UiRect::left(Val::Px(skin.gap));
                next.flex_shrink = 0.0;
                let line = Val::Px(line);
                next.border = if *role == Role::TableSortUp {
                    UiRect {
                        top: line,
                        left: line,
                        ..UiRect::ZERO
                    }
                } else {
                    UiRect {
                        bottom: line,
                        right: line,
                        ..UiRect::ZERO
                    }
                };
            }
            // The box a chevron sits in: as wide as one level of indent, the row's height (the
            // widget stretches it), its click target. The widget sets how far in it stands.
            Role::TreeTwisty => {
                next.width = Val::Px(tree_indent(text_px));
                next.flex_shrink = 0.0;
            }
            // The same turned square as a header's sort mark: closed keeps its top and right (it
            // points right), open its bottom and right (it points down).
            Role::TreeClosed | Role::TreeOpen => {
                let size = chevron;
                next.width = Val::Px(size);
                next.height = Val::Px(size);
                next.flex_shrink = 0.0;
                // Air either side, for the turned square's corners and, in a section's header,
                // for the title; in a tree's twisty, which centres it, it moves nothing.
                next.margin = UiRect::horizontal(Val::Px(size * 0.5));
                let line = Val::Px(line);
                next.border = if *role == Role::TreeClosed {
                    UiRect {
                        top: line,
                        right: line,
                        ..UiRect::ZERO
                    }
                } else {
                    UiRect {
                        bottom: line,
                        right: line,
                        ..UiRect::ZERO
                    }
                };
            }
            // A channel the target's minimum wide (or high), as long as the split, framed only by
            // the focus ring; the grip's own size is laid out with it.
            Role::SplitDivider => {
                next.min_width = Val::Px(MIN_TARGET_PX);
                next.min_height = Val::Px(MIN_TARGET_PX);
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
            }
            Role::SplitGrip => next.border_radius = BorderRadius::MAX,
            Role::DockDrop | Role::DropMark | Role::DropMarkRefused => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
            }
            Role::DockGhost | Role::DragGhost | Role::DragGhostRefused => {
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
            // A step of the type scale square, as tall as the text of the same step.
            Role::Icon => {
                let side = Val::Px(skin.type_size(icon_size.map_or(2, |size| size.0)));
                next.width = side;
                next.height = side;
                next.flex_shrink = 0.0;
            }
            // Under its row, from the row's left edge to its right, its top a line and a quarter
            // gap below the padding box so it clears the field's frame.
            Role::FieldMessage => {
                // Indented as the row's label is, so the bar and the words start under it.
                next.padding = UiRect::horizontal(Val::Px(skin.pad() * 1.5));
                next.column_gap = Val::Px(skin.gap * 0.5);
                next.margin = UiRect::top(Val::Px(line + skin.gap * 0.25));
            }
            Role::FieldMark => {
                next.width = Val::Px(line * 2.0);
                next.flex_shrink = 0.0;
                next.border_radius = radius;
            }
            Role::Progress => {
                next.row_gap = Val::Px(skin.gap * 0.5);
                next.min_width = Val::Px(
                    text_px
                        * if parent.is_some_and(|p| p.status_item) {
                            STATUS_PROGRESS_TEXT_SIZES
                        } else {
                            PROGRESS_MIN_TEXT_SIZES
                        },
                );
            }
            Role::Spinner => {
                let side = Val::Px(text_px * SPINNER_TEXT_SIZES);
                next.width = side;
                next.height = side;
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = BorderRadius::MAX;
                next.flex_shrink = 0.0;
            }
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

    // A card's picture is drawn as it is, and let half through on a disabled card.
    if let (Role::CardImage, Some(image)) = (role, image.as_mut()) {
        let color = if parent.is_some_and(|p| p.disabled) {
            Color::WHITE.with_alpha(CARD_DIM_ALPHA)
        } else {
            Color::WHITE
        };
        let color = faded(color, alpha);
        if image.color != color {
            image.color = color;
        }
    }

    // An icon is its image times a colour of the skin: the text's, following the widget it stands
    // in (muted when disabled, the accent's text pressed), or a fixed tone.
    if let (Role::Icon | Role::SlotIcon | Role::RadialIcon, Some(mut image)) = (role, image) {
        let tone = tone.copied().unwrap_or_default();
        let parent = parent.unwrap_or_default();
        let disabled = parent.disabled || owner.disabled || parent.refusing;
        let color = match tone {
            IconTone::Text if disabled => skin.text_muted,
            IconTone::Text
                if !matches!(
                    parent.variant,
                    ButtonVariant::Neutral | ButtonVariant::Submit
                ) =>
            {
                skin.variant_ink(parent.variant, phase_of(&parent))
            }
            IconTone::Text
                if parent.pressed || parent.selected || parent.primary || owner.pressed =>
            {
                skin.accent_text
            }
            IconTone::Text => skin.text,
            IconTone::Muted => skin.text_muted,
            IconTone::Accent => skin.accent,
            IconTone::Danger => skin.danger,
        };
        let color = faded(color, alpha);
        if image.color != color {
            image.color = color;
        }
    }

    let text = match role {
        Role::Heading => Some((skin.text, &look.fonts.heading, skin.heading_size)),
        Role::Label | Role::ProgressLine => Some((skin.text, &look.fonts.body, skin.text_size)),
        Role::SectionNote => Some((skin.text_muted, &look.fonts.body, skin.text_size)),
        // The label of a toast's button follows the button: inverted on the accent, faded with
        // the toast.
        Role::ToastActionText => {
            let parent = parent.unwrap_or_default();
            let color = if parent.pressed {
                skin.accent_text
            } else {
                skin.text
            };
            Some((faded(color, owner.alpha), &look.fonts.body, skin.text_size))
        }
        Role::ButtonText | Role::MenuTitleText => {
            let parent = parent.unwrap_or_default();
            let color = if parent.disabled {
                skin.text_muted
            } else if !matches!(
                parent.variant,
                ButtonVariant::Neutral | ButtonVariant::Submit
            ) {
                skin.variant_ink(parent.variant, phase_of(&parent))
            } else if parent.pressed || parent.primary {
                skin.accent_text
            } else {
                skin.text
            };
            Some((color, &look.fonts.body, skin.text_size))
        }
        // An item's label is drawn on the accent fill when it is the highlighted one.
        Role::RadialText => {
            let parent = parent.unwrap_or_default();
            let color = if owner.disabled {
                skin.text_muted
            } else if parent.selected {
                skin.accent_text
            } else {
                skin.text
            };
            Some((color, &look.fonts.body, wheel_px / skin.text_scale))
        }
        // The name in the middle is read under the thumb, so it is a heading's size.
        Role::RadialCentre => Some((
            if owner.disabled {
                skin.text_muted
            } else {
                skin.text
            },
            &look.fonts.heading,
            skin.heading_size * wheel_px / text_px,
        )),
        // A badge's count is read on its own pill, lit tab or not.
        Role::TabBadge => Some((skin.text, &look.fonts.body, skin.type_scale[1])),
        // A card's words follow the card: muted when disabled, the accent's text when chosen or
        // pressed; a line is quiet at rest and the text under the pointer, where the muted ink
        // is not held to the hovered fill.
        Role::CardName | Role::CardLine => {
            let parent = parent.unwrap_or_default();
            let color = if parent.disabled {
                skin.text_muted
            } else if parent.pressed || parent.selected {
                skin.accent_text
            } else if *role == Role::CardName || parent.hovered {
                skin.text
            } else {
                skin.text_muted
            };
            let size = if *role == Role::CardName {
                skin.text_size
            } else {
                skin.type_scale[1]
            };
            Some((color, &look.fonts.body, size))
        }
        // A slot's name is a button's text, inverted on the accent while pressed.
        Role::SlotName => {
            let parent = parent.unwrap_or_default();
            let color = if parent.pressed {
                skin.accent_text
            } else if parent.refusing {
                skin.text_muted
            } else {
                skin.text
            };
            Some((color, &look.fonts.body, skin.type_scale[1]))
        }
        // The count is read on the opaque surface's chip, never on the item.
        Role::SlotCount => Some((skin.text, &look.fonts.body, skin.type_scale[2])),
        // The seconds are read on the chip, never on the icon the sweep dims.
        Role::CooldownText => Some((skin.text, &look.fonts.body, skin.type_scale[1])),
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
        // A cell's text is the row's ink: inverted on the accent of a selected row.
        Role::TableCellText => {
            let color = if owner.selected {
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
        Role::ShortcutBadge => Some((skin.solid_surface(), &look.fonts.body, skin.type_scale[0])),
        Role::PromptKey
        | Role::PromptButton
        | Role::DockGhost
        | Role::DragGhost
        | Role::DragGhostRefused => Some((skin.text, &look.fonts.body, skin.text_size)),
        Role::ToastText => Some((
            faded(skin.text, owner.alpha),
            &look.fonts.body,
            skin.text_size,
        )),
        _ => None,
    };
    if let Some((color, font, size)) = text {
        if let Some(mut text_color) = text_color {
            text_color.set_if_neq(TextColor(faded(color, alpha)));
        }
        if let Some(mut text_font) = text_font {
            let mut next = text_font.clone();
            next.font = font.clone();
            next.font_size = scaled(size);
            text_font.set_if_neq(next);
        }
    }
    // A text field's caret is the text colour and its selection the lit accent with the accent's
    // text, the pair every "on" state here is drawn in. An unfocused selection is not drawn, and
    // neither is its text colour: Bevy applies `selected_text_color` whether or not the field has
    // focus, so the accent's dark ink on no highlight left the selected text unreadable (a number
    // field selects its text on focus, and a field a player Tabs away from kept it selected).
    if let (Role::TextField, Some(mut caret)) = (role, cursor) {
        caret.set_if_neq(TextCursorStyle {
            color: faded(skin.text, alpha),
            selection_color: faded(skin.accent, alpha),
            unfocused_selection_color: Color::NONE,
            selected_text_color: (look.editing == Some(entity))
                .then(|| faded(skin.accent_text, alpha)),
            selection_radius: 0.0,
        });
    }

    // The height the widget stands at, drawn from the skin's stacks; a toast's shadow fades with
    // the toast.
    if let (Some(mut shadow), Some(level)) = (shadow, level_of(*role, in_dialog)) {
        // Its alpha follows the node's own: a toast's by `state.alpha`, everything's by the
        // `Opacity` at and above it.
        shadow.set_if_neq(skin.elevation.box_shadow(level, state.alpha * alpha));
    }

    if let Some((popover, tip, panel)) = floating {
        if !anchor_sized {
            return;
        }
        let next = if let Some(tip) = tip {
            // One spacing step outside the panel a widget usually sits in (its padding and border),
            // so the tooltip never straddles that panel's edge. `Popover` measures from inside the
            // widget's border, so that border is added once more.
            let gap = skin.gap + skin.pad() + line * 2.0;
            tooltip_placement(gap, skin.gap, tip.in_row)
        } else if let Some(panel) = panel {
            // Half a gap off its anchor, and a gap from the window's edge; while it opens it
            // starts that much closer to the anchor ([`PopoverSlide`]).
            let (panel, slide) = panel;
            let reach = slide.map_or(0.0, PopoverSlide::reach);
            placement_of(panel.placement, skin.gap * 0.5 + line - reach, skin.gap)
        } else {
            return;
        };
        // The placement is written by inserting a `Popover`, so a box is born without one and Bevy
        // never places it before its anchor is laid out — that panic is the reason this is an insert
        // rather than a field the widget must already carry (iteration 97).
        match popover {
            Some(mut popover) if *popover != next => *popover = next,
            Some(_) => {}
            None => {
                commands.entity(entity).insert(next);
            }
        }
    }
}

/// The height a widget of `role` stands at, if it casts a shadow: a panel over a dialog's scrim
/// is the dialog, an overlay.
fn level_of(role: Role, in_dialog: bool) -> Option<ElevationLevel> {
    match role {
        Role::Panel if in_dialog => Some(ElevationLevel::Overlay),
        Role::Panel | Role::CardCell => Some(ElevationLevel::Raised),
        Role::Tooltip | Role::Popover | Role::MenuPanel => Some(ElevationLevel::Floating),
        Role::Palette | Role::Toast => Some(ElevationLevel::Overlay),
        _ => None,
    }
}

/// Whether a widget of `role` casts a shadow at some height ([`crate::elevation::give_elevation`]
/// gives it its `BoxShadow`).
pub(crate) fn elevated(role: Role) -> bool {
    level_of(role, false).is_some() || level_of(role, true).is_some()
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

/// `edge` with every side's alpha times `opacity`: a frame fading with the node under it.
fn faded_edge(edge: BorderColor, opacity: f32) -> BorderColor {
    if opacity >= 1.0 {
        return edge;
    }
    BorderColor {
        top: faded(edge.top, opacity),
        right: faded(edge.right, opacity),
        bottom: faded(edge.bottom, opacity),
        left: faded(edge.left, opacity),
    }
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
    // Centred on the widget first. Below or above a widget in a row, a box too wide to be centred
    // inside the window (the first button of a bar, with a chord) slides along the widget's edge
    // before the other side is tried: flipped beside the widget it would cover its neighbours.
    let aligns: &[PopoverAlign] = if in_row {
        &[PopoverAlign::Center, PopoverAlign::Start, PopoverAlign::End]
    } else {
        &[PopoverAlign::Center]
    };
    Popover {
        positions: sides
            .into_iter()
            .flat_map(|side| {
                aligns
                    .iter()
                    .map(move |&align| PopoverPlacement { side, align, gap })
            })
            .collect(),
        window_margin: margin,
    }
}

/// How far a text field's text stands inside its frame, in logical px: the side, and the top, which
/// is what leaves the line in the middle of `height` (the control's height, the target unless a
/// [`ControlSize`] says more; the text sits at the top of its box). Never less than the thin air a keycap has above and below.
fn text_field_inset(skin: &GooeySkin, height: f32, text_px: f32, line: f32) -> (f32, f32) {
    let x = skin.pad() * 0.6;
    let centred = (height - 2.0 * line - text_px * LIST_LINE_TEXT_SIZES) / 2.0;
    (x, centred.max(skin.pad() * 0.3))
}
