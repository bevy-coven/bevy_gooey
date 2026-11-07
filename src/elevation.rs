//! Elevation as data: a skin's shadow stacks for the three heights a surface stands at
//! ([`Elevation`]), each a list of [`ShadowLayer`]s in CSS terms, drawn by `apply_skin` as Bevy's
//! `BoxShadow`.
//!
//! Rune's elevation tokens are the model (`crates/rune-kit/src/kit.rs` `Elevation`,
//! `crates/rune-kit/src/neutral.rs` at `f73f72c`, MIT): `raised` is a 1 px drop shadow blurred 2 px,
//! `floating` 4 px blurred 8, `overlay` 8 px blurred 16, each with a half-pixel ring in the card
//! border's colour. Research: `docs/research/elevation.md`.

use bevy::color::Oklcha;
use bevy::prelude::*;
use bevy::ui::{BoxShadow, ShadowStyle};

use crate::model::shadow_sigma;
use crate::skin::{Role, elevated};

/// One layer of a shadow, in CSS's terms and logical pixels: `x y blur spread color`, as
/// `box-shadow: 0 8px 24px 0 rgb(0 0 0 / 0.4)` reads. `blur` is CSS's blur radius (twice the
/// Gaussian's standard deviation); a layer with no blur is a hard ring.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct ShadowLayer {
    /// Rightward offset, logical px.
    pub x: f32,
    /// Downward offset, logical px.
    pub y: f32,
    /// CSS blur radius, logical px; zero or more.
    pub blur: f32,
    /// How much the shadow grows (or, negative, shrinks) past each side of the box, logical px, as
    /// CSS's spread: exact on the left and right. Bevy's box shadow grows the height in proportion
    /// to the width, so on a box that is not square the top and bottom grow by `spread` times its
    /// height over its width.
    pub spread: f32,
    /// The shadow's colour, its alpha included.
    pub color: Color,
}

impl Default for ShadowLayer {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            blur: 0.0,
            spread: 0.0,
            color: Color::NONE,
        }
    }
}

impl ShadowLayer {
    /// A drop shadow `y` px down, blurred `blur` px (CSS), in `color`.
    pub fn drop(y: f32, blur: f32, color: Color) -> Self {
        Self {
            y,
            blur,
            color,
            ..default()
        }
    }

    /// A hard ring `spread` px wide around the box, in `color`.
    pub fn ring(spread: f32, color: Color) -> Self {
        Self {
            spread,
            color,
            ..default()
        }
    }

    /// This layer as Bevy's [`ShadowStyle`]: the offsets as they are; the spread doubled, as Bevy
    /// grows the box's whole width by its `spread_radius` (half on each side) where CSS grows each
    /// side by it; CSS's blur radius as the Gaussian's standard deviation Bevy's box-shadow shader
    /// takes ([`shadow_sigma`]); its colour's alpha times `opacity`.
    pub fn to_bevy(&self, opacity: f32) -> ShadowStyle {
        let c = Srgba::from(self.color);
        ShadowStyle {
            color: Color::Srgba(Srgba {
                alpha: c.alpha * opacity.clamp(0.0, 1.0),
                ..c
            }),
            x_offset: Val::Px(self.x),
            y_offset: Val::Px(self.y),
            spread_radius: Val::Px(self.spread * SPREAD_PER_SIDE),
            blur_radius: Val::Px(shadow_sigma(self.blur)),
        }
    }
}

/// Bevy's `spread_radius` grows a box shadow's whole width (half on each side); CSS's spread grows
/// each side (`bevy_ui_render` 0.20 `box_shadow.rs`: `shadow_size = size + spread`).
const SPREAD_PER_SIDE: f32 = 2.0;

/// Which height a surface stands at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElevationLevel {
    /// A panel and a card on the screen.
    Raised,
    /// A popover, a menu, a tooltip and a colour picker over the screen.
    Floating,
    /// A dialog's card, a toast and the command palette over everything.
    Overlay,
}

/// A skin's shadows for each height ([`crate::GooeySkin::elevation`]): stacks of layers, drawn in
/// order. An empty stack draws no shadow; [`crate::GooeySkin::high_contrast`] has none, as its
/// edges carry its depth.
///
/// ```
/// use bevy_gooey::{Elevation, GooeySkin};
/// assert_eq!(GooeySkin::high_contrast().elevation, Elevation::none());
/// assert_eq!(GooeySkin::dark().elevation.overlay[0].blur, 16.0);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Debug, PartialEq, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct Elevation {
    /// A panel and a card on the screen.
    pub raised: Vec<ShadowLayer>,
    /// A popover, a menu, a tooltip and a colour picker.
    pub floating: Vec<ShadowLayer>,
    /// A dialog's card, a toast and the command palette.
    pub overlay: Vec<ShadowLayer>,
}

/// The drop shadows' offsets and blurs, logical px, for raised, floating and overlay (Rune's
/// neutral kit).
const DROPS: [(f32, f32); 3] = [(1.0, 2.0), (4.0, 8.0), (8.0, 16.0)];
/// The hairline ring around every elevated surface, logical px (Rune's `ring`).
const RING_SPREAD: f32 = 0.5;

impl Elevation {
    /// No shadows at any height.
    pub fn none() -> Self {
        Self::default()
    }

    /// Rune's neutral stacks, each a drop shadow in `shadow` and a hairline ring in `ring`.
    pub fn stacks(shadow: Color, ring: Color) -> Self {
        let stack = |(y, blur): (f32, f32)| {
            vec![
                ShadowLayer::drop(y, blur, shadow),
                ShadowLayer::ring(RING_SPREAD, ring),
            ]
        };
        Self {
            raised: stack(DROPS[0]),
            floating: stack(DROPS[1]),
            overlay: stack(DROPS[2]),
        }
    }

    /// The stacks for a dark skin: black shadows at 0.42 and a white ring at 0.05 (Rune's dark
    /// neutral theme).
    pub fn on_dark() -> Self {
        Self::stacks(
            Color::srgba(0.0, 0.0, 0.0, 0.42),
            Color::srgba(1.0, 1.0, 1.0, 0.05),
        )
    }

    /// The stacks for a light skin: a blue-black shadow at 0.16 and a black ring at 0.1 (Rune's
    /// light neutral theme, `oklch(0.2 0.02 265 / 0.16)`).
    pub fn on_light() -> Self {
        Self::stacks(
            Color::from(Oklcha::new(0.2, 0.02, 265.0, 0.16)),
            Color::srgba(0.0, 0.0, 0.0, 0.1),
        )
    }

    /// The stack for `level`.
    pub fn at(&self, level: ElevationLevel) -> &[ShadowLayer] {
        match level {
            ElevationLevel::Raised => &self.raised,
            ElevationLevel::Floating => &self.floating,
            ElevationLevel::Overlay => &self.overlay,
        }
    }

    /// The stack for `level` as the Bevy component that draws it, each colour's alpha times
    /// `opacity`.
    pub fn box_shadow(&self, level: ElevationLevel, opacity: f32) -> BoxShadow {
        BoxShadow(
            self.at(level)
                .iter()
                .map(|layer| layer.to_bevy(opacity))
                .collect(),
        )
    }

    /// The fields of every layer that cannot be drawn: a blur that is negative or not a number, an
    /// offset or spread that is not finite. Named `elevation.<level>`.
    pub(crate) fn unusable(&self) -> Vec<&'static str> {
        [
            ("elevation.raised", &self.raised),
            ("elevation.floating", &self.floating),
            ("elevation.overlay", &self.overlay),
        ]
        .into_iter()
        .filter(|(_, stack)| {
            stack.iter().any(|layer| {
                !(layer.blur >= 0.0 && layer.blur.is_finite())
                    || !layer.x.is_finite()
                    || !layer.y.is_finite()
                    || !layer.spread.is_finite()
            })
        })
        .map(|(field, _)| field)
        .collect()
    }
}

/// A widget whose role changed and that has no `BoxShadow` yet.
type Unshadowed = (Changed<Role>, Without<BoxShadow>);

/// A widget whose role changed and that wears a shadow this crate gave it.
type Given = (Changed<Role>, With<Elevated>);

/// On a widget whose `BoxShadow` this crate gave it for its role, so the shadow can be taken back
/// when the role no longer casts one (a host's own shadows are never touched).
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct Elevated;

/// Gives every widget that stands at a height its `BoxShadow` (a host's own `Role::Panel` too), so
/// `apply_skin` can draw its stack. Runs in `GooeySystems::Style` chained before `apply_skin`, so
/// the insert lands while the role is still a change `apply_skin` restyles.
pub(crate) fn give_elevation(
    roles: Query<(Entity, &Role), Unshadowed>,
    mut given: Query<(Entity, &Role, &mut BoxShadow), Given>,
    mut commands: Commands,
) {
    for (entity, role) in &roles {
        if elevated(*role) {
            commands
                .entity(entity)
                .try_insert((BoxShadow::default(), Elevated));
        }
    }
    // A role that no longer stands at a height gives back the shadow it was given.
    for (entity, role, mut shadow) in &mut given {
        if !elevated(*role) {
            shadow.set_if_neq(BoxShadow::default());
            commands.entity(entity).try_remove::<Elevated>();
        }
    }
}
