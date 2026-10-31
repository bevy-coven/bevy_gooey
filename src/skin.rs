//! The skin: the look of every widget, as data, and the one system that applies it.

use std::fmt;

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{FontSize, FontSource};
use bevy::ui::{Checked, ComputedUiRenderTargetInfo, InteractionDisabled, Pressed};
use bevy::ui_widgets::SliderDragState;
use bevy::ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide};

use crate::model::{
    COMFORT_TARGET_PX, MIN_TARGET_PX, NON_TEXT_CONTRAST, TEXT_CONTRAST, composite, contrast_ratio,
    snap_line_width,
};
use crate::toast::{Toast, ToastKind};
use crate::tooltip::TooltipBox;

/// A meter's least length, in body text sizes: 8 × 20 px = 160 px at the default size, growing
/// with [`GooeySkin::text_scale`].
const METER_MIN_TEXT_SIZES: f32 = 8.0;

/// A slider's least width, in body text sizes: 10 × 20 px = 200 px at the default size.
const SLIDER_MIN_TEXT_SIZES: f32 = 10.0;

/// A toggle's track is this many body text sizes high and twice as wide.
const SWITCH_TEXT_SIZES: f32 = 1.5;

/// A slider's thumb is this many body text sizes wide, and this many rails high.
const THUMB_TEXT_SIZES: f32 = 1.0;

/// A slider's thumb stands this many rails tall, a little over the rail on each side.
const THUMB_RAIL_RATIO: f32 = 1.25;

/// A toast's longest line, in body text sizes: the tooltip's measure, about 40 characters.
const TOAST_MEASURE_TEXT_SIZES: f32 = 24.0;

/// An information toast's mark is this many lines wide, and an alert's twice that: the kind is the
/// mark's width first and its colour second, as a switch's state is its knob's place.
const TOAST_MARK_LINES: f32 = 4.0;

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
    /// A player option: every text size is multiplied by it.
    pub text_scale: f32,
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
            scrim: Color::srgba(0.08, 0.05, 0.02, 0.55),
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
            scrim: Color::BLACK,
            text_size: 20.0,
            heading_size: 32.0,
            gap: 10.0,
            padding: 14.0,
            radius: 8.0,
            border: 3.0,
            meter_height: 24.0,
            min_target: COMFORT_TARGET_PX,
            text_scale: 1.0,
        }
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
    /// then unusable sizes. Empty for a good skin; both presets are good.
    pub fn problems(&self) -> Vec<SkinProblem> {
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
        ];
        for (pair, a, b, needed) in pairs {
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
        if self.min_target < MIN_TARGET_PX {
            out.push(SkinProblem::TargetTooSmall {
                px: self.min_target,
            });
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
        out
    }
}

impl Default for GooeySkin {
    fn default() -> Self {
        Self::dark()
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
    /// [`GooeySkin::min_target`] is below [`MIN_TARGET_PX`].
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
        }
    }
}

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
        &'static TooltipBox,
        &'static mut Popover,
        &'static mut BoxShadow,
    )>,
    Option<&'static ComputedUiRenderTargetInfo>,
);

/// Every input a widget's look depends on, read once per run of [`apply_skin`].
struct Look<'a> {
    skin: &'a GooeySkin,
    fonts: &'a GooeyFonts,
    focused: Option<Entity>,
}

/// The widget state a look depends on.
#[derive(Clone, Copy)]
struct State {
    pressed: bool,
    disabled: bool,
    hovered: bool,
    /// A toggle that is on.
    checked: bool,
    /// The entity is a dialog's scrim: a panel on it is drawn opaque.
    scrim: bool,
    /// A toast's opacity; whole for everything that is not one.
    alpha: f32,
    /// A toast of the alert kind.
    alert: bool,
    /// How much of a toast's room is open, from 0 to 1; whole for everything that is not one.
    grow: f32,
    /// How tall a toast's body stands in full, in logical px.
    body_px: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            pressed: false,
            disabled: false,
            hovered: false,
            checked: false,
            scrim: false,
            alpha: 1.0,
            alert: false,
            grow: 1.0,
            body_px: 0.0,
        }
    }
}

/// What [`State`] is read from.
type StateQuery = (
    Has<Pressed>,
    Option<&'static SliderDragState>,
    Has<InteractionDisabled>,
    Has<Checked>,
    Option<&'static Hovered>,
    Option<&'static Toast>,
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
        Changed<Checked>,
        Changed<SliderDragState>,
        Changed<TooltipBox>,
        Changed<Toast>,
        Changed<ComputedUiRenderTargetInfo>,
    )>,
);

/// Whether a role is a widget whose parts follow its state: they are restyled with it.
fn owns_parts(role: Option<&Role>) -> bool {
    matches!(
        role,
        Some(Role::Button | Role::Toggle | Role::Slider | Role::Toast)
    )
}

/// Styles every entity with a [`Role`] from [`GooeySkin`] and [`GooeyFonts`]: the one place
/// widgets are styled. Runs in [`crate::GooeySystems::Style`].
///
/// A changed skin or font restyles every widget; otherwise only widgets whose role, press, hover,
/// disabled or on state, or scale changed are restyled (and every button, toggle and slider when
/// focus moves), in the frame the change happens, with the parts that follow them (a button's text,
/// a switch's knob, a slider's thumb). Lines are drawn at the scale Bevy last computed for the
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
    mut unpressed: RemovedComponents<Pressed>,
    mut enabled: RemovedComponents<InteractionDisabled>,
    mut unchecked: RemovedComponents<Checked>,
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
    let look = Look {
        skin: &skin,
        fonts: &fonts,
        focused: visible.0.then(|| focus.get()).flatten(),
    };
    let state_of = |entity: Entity| {
        states
            .get(entity)
            .map(
                |(pressed, drag, disabled, checked, hovered, toast, role)| State {
                    // `bevy_ui_widgets` marks a slider pressed for a click on its track only; a grab
                    // of the thumb is a drag, so a dragged slider reads as pressed too.
                    pressed: pressed || drag.is_some_and(|d| d.dragging),
                    disabled,
                    hovered: hovered.is_some_and(|h| h.0),
                    checked,
                    scrim: role == Some(&Role::Scrim),
                    alpha: toast.map_or(1.0, Toast::alpha),
                    alert: toast.is_some_and(|t| t.kind() == ToastKind::Alert),
                    grow: toast.map_or(1.0, Toast::grow),
                    body_px: toast.map_or(0.0, Toast::body_px),
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
                    matches!(role, Some(Role::Toggle | Role::Slider | Role::Toast))
                })
            })
            .map(state_of)
            .unwrap_or_default()
    };

    if skin.is_changed() || fonts.is_changed() {
        unpressed.clear();
        enabled.clear();
        unchecked.clear();
        for item in &mut styled {
            let entity = item.0;
            let parent = parents.get(entity).map(|p| p.parent()).ok();
            restyle(
                item,
                &look,
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
    if focus.is_changed() || visible.is_changed() {
        dirty.extend(
            styled
                .iter()
                .filter(|item| matches!(item.1, Role::Button | Role::Toggle | Role::Slider))
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
            &look,
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
        tooltip,
        target,
    ): bevy::ecs::query::QueryItem<'_, '_, Styled>,
    look: &Look,
    state: State,
    parent: Option<State>,
    owner: State,
) {
    let skin = look.skin;
    let scaled = |size: f32| FontSize::Px(size * skin.text_scale);
    let tip = tooltip.as_ref().map(|(tip, ..)| **tip);
    // Every line weight (borders, the focus ring, the meter's frame and gap) on whole device
    // pixels at the node's scale, so it is one weight on every side wherever the node lands.
    let line = snap_line_width(skin.border, target.map_or(1.0, |t| t.scale_factor()));
    // The settings controls' sizes grow with the player's text size, like the meter.
    let text_px = skin.text_size * skin.text_scale;
    let rail = skin.meter_height * skin.text_scale;
    let thumb = Vec2::new(text_px * THUMB_TEXT_SIZES, rail * THUMB_RAIL_RATIO);
    let switch = text_px * SWITCH_TEXT_SIZES;
    let knob = (switch - 4.0 * line).max(0.0);
    let fill = match role {
        Role::Panel if parent.is_some_and(|p| p.scrim) => Some(skin.solid_surface()),
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
        Role::Meter | Role::SliderRail => Some(skin.surface_raised),
        Role::Tooltip => Some(skin.solid_surface()),
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
        Role::Scrim => Some(skin.scrim),
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
        Role::Screen
        | Role::Menu
        | Role::Heading
        | Role::Label
        | Role::ButtonText
        | Role::Toggle
        | Role::Slider
        | Role::SliderHeader
        | Role::SliderTrack
        | Role::SliderGap
        | Role::SliderReadout
        | Role::ControlText
        | Role::Toast
        | Role::ToastStack
        | Role::ToastText => None,
    };
    if let (Some(fill), Some(mut background)) = (fill, background) {
        background.set_if_neq(BackgroundColor(fill));
    }

    let edge = match role {
        Role::Panel | Role::Tooltip => Some(BorderColor::all(skin.surface_raised)),
        // Framed in the outline like the other widgets whose fill is the colour of what is behind
        // them: a toast over a panel keeps its edge at 3:1.
        Role::ToastBody => Some(BorderColor::all(faded(skin.outline, owner.alpha))),
        Role::Meter | Role::ToggleTrack | Role::SliderRail => Some(BorderColor::all(skin.outline)),
        Role::Button if look.focused == Some(entity) => Some(BorderColor::all(skin.focus_ring)),
        Role::Button => Some(BorderColor::all(Color::NONE)),
        // A row has no fill of its own, so its border speaks: the ring when focused, the outline
        // under the pointer, nothing at rest.
        Role::Toggle | Role::Slider => Some(BorderColor::all(if look.focused == Some(entity) {
            skin.focus_ring
        } else if (state.hovered || state.pressed) && !state.disabled {
            skin.outline
        } else {
            Color::NONE
        })),
        Role::SliderThumb => Some(BorderColor::all(skin.solid_surface())),
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
                next.padding = UiRect::all(Val::Px(skin.padding));
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.row_gap = Val::Px(skin.gap);
            }
            Role::Menu => next.row_gap = Val::Px(skin.gap),
            Role::Button => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding =
                    UiRect::axes(Val::Px(skin.padding * 1.5), Val::Px(skin.padding * 0.6));
                next.min_width = Val::Px(skin.min_target);
                next.min_height = Val::Px(skin.min_target);
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
                let edge = skin.padding * 2.0;
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
                next.padding = UiRect::axes(Val::Px(skin.padding), Val::Px(skin.padding * 0.5));
                next.flex_shrink = 1.0;
            }
            Role::Tooltip => {
                next.padding = UiRect::axes(Val::Px(skin.padding), Val::Px(skin.padding * 0.5));
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.max_width = Val::Px(tooltip_width(skin, tip.unwrap_or_default(), line));
            }
            // A button's frame and room, a label left and a switch right, the row the target.
            Role::Toggle => {
                next.border = UiRect::all(Val::Px(line));
                next.border_radius = radius;
                next.padding =
                    UiRect::axes(Val::Px(skin.padding * 1.5), Val::Px(skin.padding * 0.6));
                next.min_height = Val::Px(skin.min_target);
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
                next.padding = UiRect::axes(Val::Px(0.0), Val::Px(skin.padding * 0.6));
                next.min_width = Val::Px(text_px * SLIDER_MIN_TEXT_SIZES);
                next.row_gap = Val::Px(skin.gap * 0.5);
            }
            // The header's text lines up with a toggle's label and a toggle's switch.
            Role::SliderHeader => next.padding = UiRect::horizontal(Val::Px(skin.padding * 1.5)),
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
        Role::ControlText | Role::SliderReadout => {
            let color = if owner.disabled {
                skin.text_muted
            } else {
                skin.text
            };
            Some((color, &look.fonts.body, skin.text_size))
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

    if let Some((tip, mut popover, mut shadow)) = tooltip {
        // One spacing step outside the panel a widget usually sits in (its padding and border), so
        // the tooltip never straddles that panel's edge. `Popover` measures from inside the
        // widget's border, so that border is added once more.
        let gap = skin.gap + skin.padding + line * 2.0;
        let next = tooltip_placement(gap, skin.gap, tip.in_row);
        if *popover != next {
            *popover = next;
        }
        // A soft drop shadow in the background colour lifts the tooltip above the screen.
        shadow.set_if_neq(BoxShadow::new(
            skin.background.with_alpha(0.7),
            Val::Px(0.0),
            Val::Px(skin.border * 2.0),
            Val::Px(0.0),
            Val::Px(skin.padding),
        ));
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
/// a ribbon of single words, so the measure stands and `Popover` takes the next side.
fn tooltip_width(skin: &GooeySkin, tip: TooltipBox, line: f32) -> f32 {
    let size = skin.text_size * skin.text_scale;
    let measure = size * 24.0;
    if tip.in_row {
        return measure;
    }
    // The box's outer edge sits `gap + padding + border` from the widget, and `gap` from the window.
    let fit = tip.room - (skin.gap + skin.padding + line) - skin.gap;
    if fit >= size * 8.0 {
        measure.min(fit)
    } else {
        measure
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
