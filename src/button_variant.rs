//! Button variants: a button says what it does. A [`ButtonVariant`] on a [`crate::button`] or
//! [`crate::icon_button`] picks its fill and ink at rest, under the pointer and held down; [`Loading`]
//! swaps its label for a spinner without changing its width and turns its input off.
//!
//! The colours are accessors over the skin's existing fields ([`GooeySkin::variant_fill`],
//! [`GooeySkin::variant_ink`]), so a preset or a skin file needs no new field, and
//! [`GooeySkin::problems`] checks every variant's ink on its fill. Reference: Rune's
//! `ButtonVariant` (`crates/rune-kit/src/actions.rs:27-39`); see `docs/research/button-variant.md`.

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;

use crate::model::{cie_lightness, composite, contrast_ratio, lift_toward};
use crate::progress::Spinner;
use crate::skin::{GooeySkin, Role};

/// How far toward the text a variant's fill moves under the pointer and held down, in CIE L\*
/// ([`lift_toward`]): the primary button's own step, and twice it.
const LIFT_L: f32 = crate::model::PRIMARY_LIFT_L;

/// The hue Warning takes from the danger colour, degrees of Oklch: amber.
const WARNING_HUE_DEG: f32 = 80.0;

/// How many times a colour's chroma is pulled in to fit sRGB, and by how much each time.
const GAMUT_STEPS: usize = 40;
const GAMUT_PULL: f32 = 0.9;

/// How much of its hover fill a [`ButtonVariant::GhostFaint`] shows: half.
const FAINT_HOVER_ALPHA: f32 = 0.5;

/// What a button says it does. Absent is [`ButtonVariant::Neutral`], the plain button. Changing it
/// restyles the button. [`crate::PrimaryButton`] is the same look as [`ButtonVariant::Submit`].
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((button("Delete"), ButtonVariant::Danger));
/// commands.spawn((button("Cancel"), ButtonVariant::Ghost));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ButtonVariant {
    /// The plain button: raised fill, the accent when held.
    #[default]
    Neutral,
    /// The screen's main action: the accent fill, lifting under the pointer.
    Submit,
    /// The text colour as a fill, with the ground's colour as its ink: the strongest button that is
    /// not the accent.
    Contrast,
    /// A destructive action: the danger colour.
    Danger,
    /// A cautionary action: the danger colour turned to amber.
    Warning,
    /// No fill at rest, a frame, and the hovered fill under the pointer.
    Outline,
    /// No fill and no frame at rest; the hovered fill under the pointer. For a toolbar.
    Ghost,
    /// A ghost whose label is the muted ink until the pointer comes.
    GhostMuted,
    /// A ghost that answers the pointer with half the fill: the quietest button.
    GhostFaint,
}

impl ButtonVariant {
    /// Every variant, plain first.
    pub const ALL: [ButtonVariant; 9] = [
        ButtonVariant::Neutral,
        ButtonVariant::Submit,
        ButtonVariant::Contrast,
        ButtonVariant::Danger,
        ButtonVariant::Warning,
        ButtonVariant::Outline,
        ButtonVariant::Ghost,
        ButtonVariant::GhostMuted,
        ButtonVariant::GhostFaint,
    ];

    /// Whether the fill is a colour of its own, not the ground showing through.
    pub fn is_filled(self) -> bool {
        matches!(
            self,
            ButtonVariant::Neutral
                | ButtonVariant::Submit
                | ButtonVariant::Contrast
                | ButtonVariant::Danger
                | ButtonVariant::Warning
        )
    }

    /// Whether the variant draws a frame at rest.
    pub fn is_framed(self) -> bool {
        matches!(self, ButtonVariant::Outline)
    }
}

/// Which of its three looks a button is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonPhase {
    /// Nothing is happening to it.
    Rest,
    /// The pointer is on it.
    Hover,
    /// It is held down.
    Pressed,
}

impl ButtonPhase {
    /// Every phase, in order.
    pub const ALL: [ButtonPhase; 3] = [ButtonPhase::Rest, ButtonPhase::Hover, ButtonPhase::Pressed];
}

impl GooeySkin {
    /// The fill of a button of `variant` in `phase`. A variant with no fill at rest answers
    /// [`Color::NONE`]; the ground shows through.
    ///
    /// ```
    /// use bevy_gooey::{ButtonPhase, ButtonVariant, GooeySkin};
    /// let skin = GooeySkin::dark();
    /// let fills = ButtonPhase::ALL.map(|phase| skin.variant_fill(ButtonVariant::Danger, phase));
    /// assert!(fills[0] != fills[1] && fills[1] != fills[2]);
    /// ```
    pub fn variant_fill(&self, variant: ButtonVariant, phase: ButtonPhase) -> Color {
        use ButtonPhase::{Hover, Pressed, Rest};
        use ButtonVariant::*;
        match (variant, phase) {
            (Neutral, Rest) => self.surface_raised,
            (Neutral, Hover) => self.hovered(),
            (Neutral, Pressed) => self.accent,
            (Submit, Rest) => self.accent,
            (Submit, Hover) => self.primary_hovered(),
            (Submit, Pressed) => self.primary_pressed(),
            (Contrast, Rest) => self.text,
            (Contrast, Hover) => self.lifted_from(self.text, self.surface, 1.0),
            (Contrast, Pressed) => self.lifted_from(self.text, self.surface, 2.0),
            (Danger, Rest) => self.danger,
            (Danger, Hover) => self.lifted_from(self.danger, self.text, 1.0),
            (Danger, Pressed) => self.lifted_from(self.danger, self.text, 2.0),
            (Warning, Rest) => self.warning(),
            (Warning, Hover) => self.lifted_from(self.warning(), self.text, 1.0),
            (Warning, Pressed) => self.lifted_from(self.warning(), self.text, 2.0),
            (Outline | Ghost | GhostMuted | GhostFaint, Rest) => Color::NONE,
            (GhostFaint, Hover) => {
                let fill = self.hovered();
                fill.with_alpha(fill.alpha() * FAINT_HOVER_ALPHA)
            }
            (Outline | Ghost | GhostMuted, Hover) => self.hovered(),
            // Held, a quiet button sinks: the panel moved away from the text, darker under light text
            // and lighter under dark, so it differs from rest on the panel it stands on (a skin's
            // `pressed` is the panel itself unless it says otherwise, which is no change) and its
            // ink only reads better.
            (Outline | Ghost | GhostMuted | GhostFaint, Pressed) => self.recessed(),
        }
    }

    /// The ink of a button of `variant` in `phase`: the label's colour, and an icon's tint. It reads
    /// on [`GooeySkin::variant_fill`] (over the panel, for a variant with no fill), which
    /// [`GooeySkin::problems`] checks.
    ///
    /// ```
    /// use bevy_gooey::{ButtonPhase, ButtonVariant, GooeySkin};
    /// let skin = GooeySkin::dark();
    /// assert_eq!(skin.variant_ink(ButtonVariant::Submit, ButtonPhase::Rest), skin.accent_text);
    /// ```
    pub fn variant_ink(&self, variant: ButtonVariant, phase: ButtonPhase) -> Color {
        use ButtonPhase::Rest;
        use ButtonVariant::*;
        match variant {
            Neutral => match phase {
                ButtonPhase::Pressed => self.accent_text,
                _ => self.text,
            },
            Submit => self.accent_text,
            Contrast | Danger | Warning => self.ink_on(self.variant_fill(variant, phase)),
            Outline | Ghost => self.text,
            GhostMuted if phase == Rest => self.text_muted,
            GhostMuted => self.text,
            GhostFaint if phase == Rest => self.text_muted,
            GhostFaint => self.text,
        }
    }

    /// The caution colour: [`GooeySkin::danger`]'s lightness and chroma at an amber hue.
    pub fn warning(&self) -> Color {
        let mut lch = Oklcha::from(self.danger);
        lch.hue = WARNING_HUE_DEG;
        // Amber at the danger colour's chroma can fall outside sRGB: pull the chroma in until it fits,
        // so the colour that is checked is the colour that is drawn.
        for _ in 0..GAMUT_STEPS {
            let c = Srgba::from(Color::from(lch));
            if [c.red, c.green, c.blue]
                .iter()
                .all(|v| (0.0..=1.0).contains(v))
            {
                break;
            }
            lch.chroma *= GAMUT_PULL;
        }
        Color::from(lch)
    }

    /// The panel's fill moved one lift away from the text: toward black under light text, toward white
    /// under dark. A panel that is already as far as it goes (black under white text) moves toward
    /// the text instead, which a skin that dark has the contrast to spare for.
    fn recessed(&self) -> Color {
        let ground = self.solid_surface();
        let away = if cie_lightness(rgb(self.text)) > cie_lightness(rgb(ground)) {
            Color::BLACK
        } else {
            Color::WHITE
        };
        let sunk = self.lifted_from(ground, away, 1.0);
        let moved = (cie_lightness(rgb(sunk)) - cie_lightness(rgb(ground))).abs();
        if moved >= crate::model::STATE_STEP {
            sunk
        } else {
            self.lifted_from(ground, self.text, 1.0)
        }
    }

    /// `from` moved `steps` lifts toward `toward`, at its own alpha.
    fn lifted_from(&self, from: Color, toward: Color, steps: f32) -> Color {
        let [r, g, b] = lift_toward(rgb(from), rgb(toward), LIFT_L * steps);
        Color::srgba(r, g, b, from.alpha())
    }

    /// The first of the panel, the ground, the text, black and white that reads on `fill` at 7:1, so
    /// a skin's own colours are preferred; failing all, whichever reads best.
    fn ink_on(&self, fill: Color) -> Color {
        let backing = rgb(fill);
        let candidates = [
            self.surface,
            self.background,
            self.text,
            Color::BLACK,
            Color::WHITE,
        ];
        candidates
            .into_iter()
            .find(|c| contrast_ratio(rgb(*c), backing) >= crate::model::ENHANCED_CONTRAST)
            .or_else(|| {
                candidates.into_iter().max_by(|a, b| {
                    contrast_ratio(rgb(*a), backing).total_cmp(&contrast_ratio(rgb(*b), backing))
                })
            })
            .unwrap_or(self.text)
    }

    /// What a variant's ink is read against: its fill over the panel.
    fn variant_backing(&self, variant: ButtonVariant, phase: ButtonPhase) -> [f32; 3] {
        let fill = self.variant_fill(variant, phase);
        composite(rgb(fill), fill.alpha(), rgb(self.solid_surface()))
    }

    /// The contrast pairs [`GooeySkin::problems`] adds for every variant: its ink on its fill at
    /// rest, under the pointer and held down, as text, and the lightness step between its fills
    /// where it has them.
    pub(crate) fn variant_pairs(&self) -> Vec<(&'static str, [f32; 3], [f32; 3])> {
        let mut out = Vec::new();
        // The plain button and Submit are checked by the pairs `problems()` has always had (text on the
        // hovered fill, accent_text on the accent and its lifts).
        for variant in ButtonVariant::ALL
            .into_iter()
            .filter(|v| !matches!(v, ButtonVariant::Neutral | ButtonVariant::Submit))
        {
            for phase in ButtonPhase::ALL {
                out.push((
                    pair_name(variant, phase),
                    rgb(self.variant_ink(variant, phase)),
                    self.variant_backing(variant, phase),
                ));
            }
        }
        out
    }

    /// The lightness steps [`GooeySkin::problems`] holds a variant's fills to: every step of a variant
    /// that is a fill of its own (the plain button and the faint ghost answer the pointer with the
    /// skin's own hovered fill, which is not this change's to refuse), and rest against held for a
    /// quiet one, whose hover fill may be as near the panel as the skin makes it.
    pub(crate) fn variant_steps(&self) -> Vec<(&'static str, f32)> {
        let lightness = |v, p| cie_lightness(self.variant_backing(v, p));
        let mut out = Vec::new();
        for variant in ButtonVariant::ALL {
            let [rest, hover, pressed] = ButtonPhase::ALL.map(|p| lightness(variant, p));
            match variant {
                ButtonVariant::Contrast | ButtonVariant::Danger | ButtonVariant::Warning => {
                    out.push((step_name(variant, 0), (rest - hover).abs()));
                    out.push((step_name(variant, 1), (hover - pressed).abs()));
                }
                ButtonVariant::Outline
                | ButtonVariant::Ghost
                | ButtonVariant::GhostMuted
                | ButtonVariant::GhostFaint => {
                    out.push((step_name(variant, 2), (rest - pressed).abs()));
                }
                ButtonVariant::Neutral | ButtonVariant::Submit => {}
            }
        }
        out
    }
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

macro_rules! names {
    ($($variant:ident => $label:literal),* $(,)?) => {
        fn pair_name(variant: ButtonVariant, phase: ButtonPhase) -> &'static str {
            match (variant, phase) {
                $(
                    (ButtonVariant::$variant, ButtonPhase::Rest) => concat!($label, " ink on its fill at rest"),
                    (ButtonVariant::$variant, ButtonPhase::Hover) => concat!($label, " ink on its fill hovered"),
                    (ButtonVariant::$variant, ButtonPhase::Pressed) => concat!($label, " ink on its fill pressed"),
                )*
            }
        }
        fn step_name(variant: ButtonVariant, which: u8) -> &'static str {
            match (variant, which) {
                $(
                    (ButtonVariant::$variant, 0) => concat!($label, " at rest and hovered"),
                    (ButtonVariant::$variant, 1) => concat!($label, " hovered and pressed"),
                    (ButtonVariant::$variant, _) => concat!($label, " at rest and pressed"),
                )*
            }
        }
    };
}
names! {
    Neutral => "neutral",
    Submit => "submit",
    Contrast => "contrast",
    Danger => "danger",
    Warning => "warning",
    Outline => "outline",
    Ghost => "ghost",
    GhostMuted => "ghost muted",
    GhostFaint => "ghost faint",
}

/// A button that is busy: its label is hidden (its width stays) behind a [`Spinner`], and it takes no
/// input, so a click sends no [`Activate`](bevy::ui_widgets::Activate). It looks as it did, not
/// disabled. Set it back to `false` and everything returns.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((button("Save"), ButtonVariant::Submit, Loading(true)));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Loading(pub bool);

/// What [`update_loading`] did to a loading button, so it can be undone: it holds the button's input
/// and hides its label, and `own_disabled` is whether the host had already disabled it.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct LoadingHold {
    own_disabled: bool,
    /// What it hid, with the visibility each had, to put back.
    hidden: Vec<(Entity, Visibility)>,
}

/// The spinner [`update_loading`] puts over a loading button's label.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoadingSpinner;

type Loaded = (
    Entity,
    Ref<'static, Loading>,
    Has<LoadingHold>,
    Has<InteractionDisabled>,
);

/// A host that disables a loading button while it loads keeps it disabled when the load ends.
pub fn note_host_disable(
    inserted: On<Insert<InteractionDisabled>>,
    mut holds: Query<&mut LoadingHold>,
) {
    if let Ok(mut hold) = holds.get_mut(inserted.entity)
        && !hold.own_disabled
    {
        hold.own_disabled = true;
    }
}

/// Puts a spinner over the label of every button whose [`Loading`] is true, and takes it away when it
/// is false or the component is gone. Runs in [`crate::GooeySystems::Style`], before
/// [`crate::apply_skin`].
pub fn update_loading(
    mut commands: Commands,
    buttons: Query<Loaded, Changed<Loading>>,
    unloaded: Query<Entity, (With<LoadingHold>, Without<Loading>)>,
    children: Query<&Children>,
    mut parts: Query<(&Role, &mut Visibility), Without<LoadingSpinner>>,
    spinners: Query<Entity, With<LoadingSpinner>>,
    holds: Query<&LoadingHold>,
) {
    let release = |commands: &mut Commands,
                   parts: &mut Query<(&Role, &mut Visibility), Without<LoadingSpinner>>,
                   entity: Entity| {
        let Ok(hold) = holds.get(entity) else { return };
        commands.entity(entity).remove::<LoadingHold>();
        if !hold.own_disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
        for (kid, prior) in &hold.hidden {
            if let Ok((_, mut visibility)) = parts.get_mut(*kid) {
                *visibility = *prior;
            }
        }
        for kid in children.get(entity).into_iter().flat_map(|c| c.iter()) {
            if spinners.get(kid).is_ok() {
                commands.entity(kid).despawn();
            }
        }
    };
    for entity in &unloaded {
        release(&mut commands, &mut parts, entity);
    }
    for (entity, loading, held, disabled) in &buttons {
        if loading.0 && !held {
            let mut hidden = Vec::new();
            for kid in children.get(entity).into_iter().flat_map(|c| c.iter()) {
                if let Ok((role, mut visibility)) = parts.get_mut(kid)
                    && matches!(role, Role::ButtonText | Role::Icon)
                {
                    hidden.push((kid, *visibility));
                    *visibility = Visibility::Hidden;
                }
            }
            // The button's own `InteractionDisabled` goes in first: the observer that notes a host's
            // disabling only counts one made after the hold is on.
            if !disabled {
                commands.entity(entity).insert(InteractionDisabled);
            }
            commands.entity(entity).insert(LoadingHold {
                own_disabled: disabled,
                hidden,
            });
            commands.entity(entity).with_child((
                Node {
                    // PLACES-ITSELF-OK: the spinner stands over the label's own room, centred, taking none.
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Role::Spinner,
                Spinner,
                LoadingSpinner,
                Pickable::IGNORE,
            ));
        } else if !loading.0 && held {
            release(&mut commands, &mut parts, entity);
        }
    }
}
