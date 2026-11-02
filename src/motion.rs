//! Motion: a panel that arrives slides into place instead of appearing whole, and one setting turns
//! every motion of the crate off.
//!
//! [`SlideIn`] is the one token: a component on any `Node` that eases its [`UiTransform`] translation
//! from an offset to nothing on real time and then removes itself. The crate puts it on a dialog's
//! card and on every popover panel (menus, context menus, select lists, the colour picker) as they
//! open; a game puts it on its own panels. [`MotionSettings`] is the player's "reduce motion"
//! (WCAG 2.3.3): at zero nothing starts and a running slide ends in the frame it is seen.

use bevy::prelude::*;

use crate::model::{
    DIALOG_SLIDE_PX, DIALOG_SLIDE_SECONDS, POPOVER_SLIDE_PX, POPOVER_SLIDE_SECONDS, ease_out,
};
use crate::popover::{Placement, PopoverPanel};
use crate::skin::Role;

/// How much motion the player wants: the one switch for every motion the crate draws.
///
/// Every motion lasts its own length times [`MotionSettings::duration_scale`]; at 0
/// ([`MotionSettings::off`]) none starts and a running one ends at once. A game wires this to its
/// "Reduce motion" option: an entrance is never essential, and WCAG 2.2 SC 2.3.3 asks that motion
/// triggered by interaction can be disabled. The crate cannot read the operating system's own
/// preference, Bevy having no way to.
///
/// ```
/// # use bevy_gooey::MotionSettings;
/// let mut settings = MotionSettings::default();
/// assert_eq!(settings.duration_scale, 1.0);
/// settings = MotionSettings::off();
/// assert!(settings.is_off());
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct MotionSettings {
    /// The multiplier on every motion's length. 1 is as designed, 2 half as fast, 0 no motion.
    /// Default: 1.
    pub duration_scale: f32,
}

impl Default for MotionSettings {
    fn default() -> Self {
        Self {
            duration_scale: 1.0,
        }
    }
}

impl MotionSettings {
    /// No motion at all: the reduced-motion setting.
    pub const fn off() -> Self {
        Self {
            duration_scale: 0.0,
        }
    }

    /// Whether motion is turned off (a scale that is zero, negative or not a number).
    pub fn is_off(&self) -> bool {
        self.duration_scale.is_nan() || self.duration_scale <= 0.0
    }
}

/// A panel arriving: its [`UiTransform`] translation eases out from `from` to nothing over `seconds`
/// of real time (so a paused game's menu still moves), and then the component is removed and the
/// translation is left at zero. Only the picture moves: layout, focus and the hit test see the
/// panel where it ends up from its first frame, and it writes the translation only, never the scale.
///
/// The crate adds it to a dialog's card and to every popover panel as it opens, unless
/// [`MotionSettings`] is off. It does not run while the node is `Visibility::Hidden`, so a panel
/// that is drawn a frame after it is spawned still shows all of its slide. A game that wants it on
/// its own panel inserts it:
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::SlideIn;
/// # fn spawn(mut commands: Commands) {
/// // Rises 12 logical px over a fifth of a second.
/// commands.spawn((Node::default(), SlideIn::new(Vec2::new(0.0, 12.0), 0.2)));
/// # }
/// ```
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct SlideIn {
    /// Where the node starts, in logical px from where it ends: positive `y` is below it.
    pub from: Vec2,
    /// How long the slide takes, seconds, before [`MotionSettings::duration_scale`].
    pub seconds: f32,
    age: f32,
}

impl SlideIn {
    /// A slide from `from` (logical px, positive `y` down) lasting `seconds`.
    pub const fn new(from: Vec2, seconds: f32) -> Self {
        Self {
            from,
            seconds,
            age: 0.0,
        }
    }
}

/// Where a popover on `placement` starts: [`POPOVER_SLIDE_PX`] on the side of its anchor, so it
/// leaves the anchor as it opens.
fn popover_start(placement: Placement) -> Vec2 {
    match placement {
        Placement::Above => Vec2::new(0.0, POPOVER_SLIDE_PX),
        Placement::Right => Vec2::new(-POPOVER_SLIDE_PX, 0.0),
        Placement::Left => Vec2::new(POPOVER_SLIDE_PX, 0.0),
        Placement::Below | Placement::AtPointer => Vec2::new(0.0, -POPOVER_SLIDE_PX),
    }
}

/// Gives a dialog's card and each popover panel that appeared this frame its [`SlideIn`], unless
/// motion is off or the panel already has one. A card is a `Panel` or `Palette` whose parent is a
/// scrim. Runs in [`crate::GooeySystems::Style`], before [`update_motion`].
pub fn begin_entrances(
    settings: Res<MotionSettings>,
    mut commands: Commands,
    popovers: Query<(Entity, &PopoverPanel), (Added<PopoverPanel>, Without<SlideIn>)>,
    cards: Query<(Entity, &Role, &ChildOf), (Or<(Added<Role>, Added<ChildOf>)>, Without<SlideIn>)>,
    roles: Query<&Role>,
) {
    if settings.is_off() {
        return;
    }
    for (entity, panel) in &popovers {
        commands.entity(entity).insert(SlideIn::new(
            popover_start(panel.placement),
            POPOVER_SLIDE_SECONDS,
        ));
    }
    for (entity, role, parent) in &cards {
        if matches!(role, Role::Panel | Role::Palette)
            && roles.get(parent.parent()).is_ok_and(|r| *r == Role::Scrim)
        {
            commands.entity(entity).insert(SlideIn::new(
                Vec2::new(0.0, DIALOG_SLIDE_PX),
                DIALOG_SLIDE_SECONDS,
            ));
        }
    }
}

/// Moves every [`SlideIn`] to where real time puts it and takes a spent one off, leaving its
/// translation at zero; with [`MotionSettings`] off every one ends at once. Writes the translation
/// only when it differs. Runs in [`crate::GooeySystems::Style`], before layout.
pub fn update_motion(
    settings: Res<MotionSettings>,
    time: Option<Res<Time<Real>>>,
    mut commands: Commands,
    mut slides: Query<(Entity, &mut SlideIn, &mut UiTransform, Option<&Visibility>)>,
) {
    let delta = time.map_or(0.0, |time| time.delta_secs());
    let off = settings.is_off();
    for (entity, mut slide, mut transform, visibility) in &mut slides {
        let length = slide.seconds * settings.duration_scale;
        let done = off || length <= 0.0 || slide.age >= length;
        if !done && visibility != Some(&Visibility::Hidden) {
            slide.age += delta;
        }
        let progress = if done || slide.age >= length {
            1.0
        } else {
            ease_out(slide.age, length)
        };
        let left = 1.0 - progress;
        let want = Val2::px(slide.from.x * left, slide.from.y * left);
        if transform.translation != want {
            transform.translation = want;
        }
        if progress >= 1.0 {
            commands.entity(entity).remove::<SlideIn>();
        }
    }
}
