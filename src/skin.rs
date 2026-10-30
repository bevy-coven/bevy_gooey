//! The skin: the look of every widget, as data, and the one system that applies it.

use std::fmt;

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{FontSize, FontSource};
use bevy::ui::{InteractionDisabled, Pressed};

use crate::model::{
    COMFORT_TARGET_PX, MIN_TARGET_PX, NON_TEXT_CONTRAST, TEXT_CONTRAST, composite, contrast_ratio,
};

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
    /// A button at rest, a meter's track, a tooltip and a panel's border.
    pub surface_raised: Color,
    /// Headings, labels and button text.
    pub text: Color,
    /// The text of a disabled button.
    pub text_muted: Color,
    /// A pressed button and a meter's fill.
    pub accent: Color,
    /// Text drawn on [`GooeySkin::accent`].
    pub accent_text: Color,
    /// A colour for destructive actions, for the host to use.
    pub danger: Color,
    /// The border of the focused button while focus is visible.
    pub focus_ring: Color,
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
    /// The border width of panels and buttons, logical px. A button's focus ring is drawn in it.
    pub border: f32,
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
            scrim: Color::BLACK,
            text_size: 20.0,
            heading_size: 32.0,
            gap: 10.0,
            padding: 14.0,
            radius: 8.0,
            border: 3.0,
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
    /// order: low contrast pairs, then a too-small target, then unusable sizes. Empty for a good
    /// skin; both presets are good.
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
        let positive = [
            ("text_size", self.text_size),
            ("heading_size", self.heading_size),
            ("text_scale", self.text_scale),
            ("min_target", self.min_target),
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
                "{field} is not a usable size: it must be finite, and above zero for a text size, text_scale or min_target, at least zero otherwise"
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
    /// A meter's track.
    Meter,
    /// A meter's fill.
    MeterFill,
    /// A tooltip's box.
    Tooltip,
    /// The veil behind a modal dialog.
    Scrim,
}

type Styled = (
    Entity,
    &'static Role,
    Option<&'static mut Node>,
    Option<&'static mut BackgroundColor>,
    Option<&'static mut BorderColor>,
    Option<&'static mut TextColor>,
    Option<&'static mut TextFont>,
);

/// Every input a widget's look depends on, read once per run of [`apply_skin`].
struct Look<'a> {
    skin: &'a GooeySkin,
    fonts: &'a GooeyFonts,
    focused: Option<Entity>,
}

/// The widget state a look depends on.
#[derive(Clone, Copy, Default)]
struct State {
    pressed: bool,
    disabled: bool,
    hovered: bool,
    /// The entity is a dialog's scrim: a panel on it is drawn opaque.
    scrim: bool,
}

/// A widget whose look may have changed since the last run.
type Restyle = (
    With<Role>,
    Or<(
        Changed<Role>,
        Changed<Pressed>,
        Changed<Hovered>,
        Changed<InteractionDisabled>,
    )>,
);

/// Styles every entity with a [`Role`] from [`GooeySkin`] and [`GooeyFonts`]: the one place
/// widgets are styled. Runs in [`crate::GooeySystems::Style`].
///
/// A changed skin or font restyles every widget; otherwise only widgets whose role, press, hover or
/// disabled state changed are restyled (and every button when focus moves), in the frame the change
/// happens. A changed skin that fails [`GooeySkin::problems`] logs one warning per problem and is
/// applied anyway.
#[allow(clippy::too_many_arguments)]
pub fn apply_skin(
    skin: Res<GooeySkin>,
    fonts: Res<GooeyFonts>,
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
    changed: Query<Entity, Restyle>,
    mut unpressed: RemovedComponents<Pressed>,
    mut enabled: RemovedComponents<InteractionDisabled>,
    states: Query<(
        Has<Pressed>,
        Has<InteractionDisabled>,
        Option<&Hovered>,
        Option<&Role>,
    )>,
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
            .map(|(pressed, disabled, hovered, role)| State {
                pressed,
                disabled,
                hovered: hovered.is_some_and(|h| h.0),
                scrim: role == Some(&Role::Scrim),
            })
            .unwrap_or_default()
    };

    if skin.is_changed() || fonts.is_changed() {
        unpressed.clear();
        enabled.clear();
        for item in &mut styled {
            let entity = item.0;
            let parent = parents.get(entity).map(|p| p.parent()).ok();
            restyle(item, &look, state_of(entity), parent.map(state_of));
        }
        return;
    }

    let mut dirty: Vec<Entity> = changed.iter().collect();
    dirty.extend(unpressed.read());
    dirty.extend(enabled.read());
    if focus.is_changed() || visible.is_changed() {
        dirty.extend(
            styled
                .iter()
                .filter(|item| *item.1 == Role::Button)
                .map(|item| item.0),
        );
    }
    // A button's text follows the button's state, so a dirty button dirties its text.
    let texts: Vec<Entity> = dirty
        .iter()
        .filter_map(|&e| children.get(e).ok())
        .flat_map(|c| c.iter())
        .collect();
    dirty.extend(texts);
    dirty.sort_unstable();
    dirty.dedup();
    for entity in dirty {
        let Ok(item) = styled.get_mut(entity) else {
            continue;
        };
        let parent = parents.get(entity).map(|p| p.parent()).ok();
        restyle(item, &look, state_of(entity), parent.map(state_of));
    }
}

fn restyle(
    (entity, role, node, background, border, text_color, text_font): bevy::ecs::query::QueryItem<
        '_,
        '_,
        Styled,
    >,
    look: &Look,
    state: State,
    parent: Option<State>,
) {
    let skin = look.skin;
    let scaled = |size: f32| FontSize::Px(size * skin.text_scale);
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
        Role::Meter | Role::Tooltip => Some(skin.surface_raised),
        Role::MeterFill => Some(skin.accent),
        Role::Scrim => Some(skin.scrim),
        Role::Screen | Role::Menu | Role::Heading | Role::Label | Role::ButtonText => None,
    };
    if let (Some(fill), Some(mut background)) = (fill, background) {
        background.set_if_neq(BackgroundColor(fill));
    }

    let edge = match role {
        Role::Panel => Some(BorderColor::all(skin.surface_raised)),
        Role::Button if look.focused == Some(entity) => Some(BorderColor::all(skin.focus_ring)),
        Role::Button => Some(BorderColor::all(Color::NONE)),
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
                next.border = UiRect::all(Val::Px(skin.border));
                next.border_radius = radius;
                next.row_gap = Val::Px(skin.gap);
            }
            Role::Menu => next.row_gap = Val::Px(skin.gap),
            Role::Button => {
                next.border = UiRect::all(Val::Px(skin.border));
                next.border_radius = radius;
                next.padding =
                    UiRect::axes(Val::Px(skin.padding * 1.5), Val::Px(skin.padding * 0.6));
                next.min_width = Val::Px(skin.min_target);
                next.min_height = Val::Px(skin.min_target);
            }
            Role::Meter => next.border_radius = radius,
            Role::Tooltip => {
                next.padding = UiRect::all(Val::Px(skin.padding * 0.5));
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
}
