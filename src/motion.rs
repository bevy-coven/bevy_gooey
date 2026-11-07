//! Motion: a node that has somewhere to be goes there on a spring, and one setting turns every
//! motion of the crate off.
//!
//! [`MotionTarget`] is the one track: a component on any `Node` that says the pose it wants (a
//! translation, a scale, a rotation) and which of the skin's springs ([`MotionTokens`]) takes it
//! there. A new target starts a new segment from where the node is *drawn* and how fast it is going,
//! so nothing jumps when a motion is interrupted. The crate gives one to a dialog's card as it opens
//! (with [`MotionTarget::entering_from`]) and drives every popover panel's slide the same way; a game
//! puts its own on its panels. [`MotionSettings`] is the player's "reduce motion" (WCAG 2.3.3): at
//! zero nothing starts and a running track lands in the frame it is seen.

use bevy::prelude::*;

use crate::model::{
    DIALOG_SLIDE_PX, POPOVER_SLIDE_PX, Spring, SpringState, spring_settled, spring_step,
};
use crate::popover::PopoverPanel;
use crate::skin::{GooeySkin, Role, SkinScope, skin_of};

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

/// Which of the skin's springs a motion uses ([`MotionTokens`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default, Debug, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MotionToken {
    /// Small controls answering a press: the quick one.
    Snappy,
    /// The default state change.
    #[default]
    Default,
    /// Larger things that arrive and settle.
    Calm,
    /// A thing that leads another (a drag's follower, a pointer-tied value).
    Lead,
}

/// The skin's motion palette: four springs and a fade length, so a whole interface has one feel
/// ([`GooeySkin::motion`](crate::GooeySkin::motion)). Every spring has bounce 0 by default:
/// utility UI arrives and stops. [`MotionSettings::duration_scale`] multiplies every length here.
///
/// ```
/// use bevy_gooey::{GooeySkin, MotionToken};
/// let skin = GooeySkin::dark();
/// assert_eq!(skin.motion.spring(MotionToken::Default).duration, 0.3);
/// assert!(skin.problems().is_empty());
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct MotionTokens {
    /// 0.25 s, bounce 0.
    pub snappy: Spring,
    /// 0.3 s, bounce 0.
    pub default: Spring,
    /// 0.4 s, bounce 0.
    pub calm: Spring,
    /// 0.2 s, bounce 0.
    pub lead: Spring,
    /// How long an opacity change takes, seconds. Default: 0.15. Zero or above, finite.
    pub fade_seconds: f32,
}

impl Default for MotionTokens {
    fn default() -> Self {
        Self {
            snappy: Spring {
                duration: 0.25,
                bounce: 0.0,
            },
            default: Spring {
                duration: 0.3,
                bounce: 0.0,
            },
            calm: Spring {
                duration: 0.4,
                bounce: 0.0,
            },
            lead: Spring {
                duration: 0.2,
                bounce: 0.0,
            },
            fade_seconds: 0.15,
        }
    }
}

impl MotionTokens {
    /// The spring a token names.
    pub fn spring(&self, token: MotionToken) -> Spring {
        match token {
            MotionToken::Snappy => self.snappy,
            MotionToken::Default => self.default,
            MotionToken::Calm => self.calm,
            MotionToken::Lead => self.lead,
        }
    }

    /// The name of the first field that cannot be used: a spring that is not
    /// [`Spring::is_valid`], or a fade that is negative or not finite.
    pub fn problem(&self) -> Option<&'static str> {
        [
            ("motion.snappy", self.snappy),
            ("motion.default", self.default),
            ("motion.calm", self.calm),
            ("motion.lead", self.lead),
        ]
        .into_iter()
        .find(|(_, spring)| !spring.is_valid())
        .map(|(field, _)| field)
        .or_else(|| {
            (!self.fade_seconds.is_finite() || self.fade_seconds < 0.0)
                .then_some("motion.fade_seconds")
        })
    }
}

/// How near a track's scale or rotation must be to its target, in the unit of the lane (a factor, a
/// radian), to count as there: a hundredth of a pixel across a hundred-pixel node.
const TRACK_REST_UNITLESS: f32 = 1.0e-4;

/// The pose a [`MotionTarget`] asks for, lane by lane: translation x and y (logical px), scale x and
/// y, rotation (radians, clockwise).
const LANES: usize = 5;

/// A node's place to be, and the spring that takes it there: a track on its [`UiTransform`].
///
/// Change the fields and the node, wherever it is drawn now and however fast it is going, moves to the
/// new pose on [`MotionTarget::spring`] (a name in [`GooeySkin::motion`]); nothing jumps, because
/// the new segment starts from the drawn pose and its velocity. The *first* target a node has lands
/// where it says, with no motion from spawn, unless it is given a place to come from with
/// [`MotionTarget::entering_from`]. Only the picture moves: layout, focus and the hit test see the
/// node where layout puts it. The clock is [`Time<Real>`] times [`MotionSettings::duration_scale`],
/// so a paused game's menu still moves, and with [`MotionSettings::off`] every track lands in the
/// frame it is seen.
///
/// The track owns the node's [`UiTransform`] while it runs. A game that writes the transform itself
/// wins: the track is dropped the next time it looks. Add [`InstantMotion`] to land the next change at
/// once instead.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::{MotionTarget, MotionToken};
/// # fn spawn(mut commands: Commands) {
/// // Rises 12 logical px into place.
/// commands.spawn((Node::default(), MotionTarget::default().entering_from(Vec2::new(0.0, 12.0))));
/// // Later: a panel that slides 300 px right when asked, with the calm spring.
/// let panel = commands.spawn((Node::default(), MotionTarget::default())).id();
/// commands.entity(panel).insert(MotionTarget {
///     translation: Vec2::new(300.0, 0.0),
///     spring: MotionToken::Calm,
///     ..default()
/// });
/// # }
/// ```
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(MotionTrack)]
pub struct MotionTarget {
    /// Where the node's picture is moved to, logical px from where layout puts it (positive `y` is
    /// down). Default: nowhere.
    pub translation: Vec2,
    /// The node's scale, 1 is its size. Default: 1.
    pub scale: Vec2,
    /// The node's rotation, radians clockwise. Default: 0.
    pub rotation: f32,
    /// Which of [`GooeySkin::motion`]'s springs takes it there. Default: [`MotionToken::Default`].
    pub spring: MotionToken,
    /// Where the *first* pose is drawn from, as a translation in logical px: the node is put there
    /// the frame it appears and moves to its target, so an entrance is a first target that does not
    /// land. Read once, when the track starts; later targets ignore it.
    pub enter_from: Option<Vec2>,
}

impl Default for MotionTarget {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
            spring: MotionToken::Default,
            enter_from: None,
        }
    }
}

impl MotionTarget {
    /// The same track, entering from `from` (logical px, positive `y` below its place) with the
    /// [`MotionToken::Snappy`] spring, the one the crate gives a dialog's card. Pair it with the
    /// first target a node gets.
    pub fn entering_from(self, from: Vec2) -> Self {
        Self {
            enter_from: Some(from),
            spring: MotionToken::Snappy,
            ..self
        }
    }

    /// The pose as lanes.
    fn lanes(&self) -> [f32; LANES] {
        [
            self.translation.x,
            self.translation.y,
            self.scale.x,
            self.scale.y,
            self.rotation,
        ]
    }
}

/// Marks a [`MotionTarget`] whose *next* change lands at once, with no motion: the marker is taken
/// off in that frame. One change that has no travel (a window restored, a list rebuilt), without
/// switching motion off for the player.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct InstantMotion;

/// What a track draws now: the lanes' positions and velocities, the target they were last moving
/// toward and the transform it wrote (to see a game's own write). Internal; a game reads
/// [`UiTransform`] and writes [`MotionTarget`].
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct MotionTrack {
    lanes: [SpringState; LANES],
    /// The pose the lanes were last moving toward; `None` until the track has started.
    heading: Option<[f32; LANES]>,
    /// The transform the track last wrote, to tell a game's own write from its own.
    drawn: Option<UiTransform>,
    /// Whether every lane is at rest on its target: a settled track is skipped before any write.
    settled: bool,
}

impl MotionTrack {
    /// The transform a pose draws.
    fn transform_of(pose: [f32; LANES]) -> UiTransform {
        UiTransform {
            translation: Val2::px(pose[0], pose[1]),
            scale: Vec2::new(pose[2], pose[3]),
            rotation: Rot2::radians(pose[4]),
        }
    }

    fn shown(&self) -> [f32; LANES] {
        self.lanes.map(|lane| lane.value)
    }

    /// Puts every lane on `pose`, going nowhere.
    fn land(&mut self, pose: [f32; LANES]) {
        self.lanes = pose.map(|value| SpringState {
            value,
            velocity: 0.0,
        });
        self.heading = Some(pose);
        self.settled = true;
    }
}

/// Whether a lane is at rest on its target, by the lane's own tolerance.
fn lane_at_rest(spring: Spring, lane: usize, state: SpringState, target: f32) -> bool {
    let tolerance = if lane < 2 {
        crate::model::SPRING_REST_POSITION
    } else {
        TRACK_REST_UNITLESS
    };
    spring_settled(spring, state, target, tolerance)
}

/// How a popover panel's slide stands: it starts [`POPOVER_SLIDE_PX`] closer to its anchor and
/// settles at its gap on the skin's [`MotionToken::Snappy`] spring.
///
/// Bevy's `Popover` owns the panel's `UiTransform` (it measures where the panel is and corrects
/// it every frame), so a translation written there is undone; the gap to the anchor is what the
/// placement is made of, and [`crate::apply_skin`] writes it, less [`PopoverSlide::reach`]. A
/// panel that flips to the anchor's other side is moved in the right direction there too. The crate
/// adds it; a game's own panel uses [`MotionTarget`].
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PopoverSlide {
    reach: SpringState,
    /// Whether the slide has reached its end: the component is taken off a frame after, so the
    /// restyle for its last value is seen.
    spent: bool,
}

impl PopoverSlide {
    /// How much closer to its anchor than its final gap the panel stands, logical px.
    pub fn reach(&self) -> f32 {
        self.reach.value
    }
}

/// What a new node's role and parent say about it.
type NewCard = (Entity, &'static Role, &'static ChildOf);

/// A node that has just got a role or a parent and no motion of its own yet.
type NewCardFilter = (Or<(Added<Role>, Added<ChildOf>)>, Without<MotionTarget>);

/// Gives a dialog's card and each popover panel that appeared this frame its entrance, unless
/// motion is off or the card already has a [`MotionTarget`]. A card is a `Panel` or `Palette`
/// whose parent is a scrim. Runs in [`crate::GooeySystems::Style`], before [`update_motion`].
pub fn begin_entrances(
    settings: Res<MotionSettings>,
    mut commands: Commands,
    popovers: Query<Entity, (Added<PopoverPanel>, Without<PopoverSlide>)>,
    cards: Query<NewCard, NewCardFilter>,
    roles: Query<&Role>,
) {
    if settings.is_off() {
        return;
    }
    for entity in &popovers {
        commands.entity(entity).insert(PopoverSlide {
            reach: SpringState {
                value: POPOVER_SLIDE_PX,
                velocity: 0.0,
            },
            spent: false,
        });
    }
    for (entity, role, parent) in &cards {
        if matches!(role, Role::Panel | Role::Palette)
            && roles.get(parent.parent()).is_ok_and(|r| *r == Role::Scrim)
        {
            commands
                .entity(entity)
                .insert(MotionTarget::default().entering_from(Vec2::new(0.0, DIALOG_SLIDE_PX)));
        }
    }
}

/// Moves every [`MotionTarget`] and every [`PopoverSlide`] to where real time puts it. With
/// [`MotionSettings`] off (or no clock) every track lands at once. Writes a node's transform only
/// when it differs, and a settled track with an unchanged target is skipped before anything is
/// read. Runs in [`crate::GooeySystems::Style`], before layout.
#[allow(clippy::too_many_arguments)]
pub fn update_motion(
    settings: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    time: Option<Res<Time<Real>>>,
    mut commands: Commands,
    mut tracks: Query<(
        Entity,
        Ref<MotionTarget>,
        &mut MotionTrack,
        &mut UiTransform,
        Option<&Visibility>,
        Has<InstantMotion>,
    )>,
    mut popovers: Query<(Entity, &mut PopoverSlide, Option<&Visibility>)>,
) {
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    // With no clock the track jumps: nothing can say how long it has been.
    let off = settings.is_off() || time.is_none();
    for (entity, target, mut track, mut transform, visibility, instant) in &mut tracks {
        if track.settled && !target.is_changed() {
            continue;
        }
        let want = target.lanes();
        if !want.iter().all(|lane| lane.is_finite()) {
            warn!(
                "a MotionTarget on {entity} has a pose that is not a number; it keeps moving to the last one"
            );
            continue;
        }
        // A game that wrote the transform since the track did has taken it over.
        if track.drawn.is_some_and(|drawn| drawn != *transform) {
            commands
                .entity(entity)
                .remove::<(MotionTarget, MotionTrack, InstantMotion)>();
            continue;
        }
        let new_heading = track.heading != Some(want);
        let starting = track.heading.is_none();
        if off || (instant && new_heading && !starting) {
            track.land(want);
            if instant {
                commands.entity(entity).remove::<InstantMotion>();
            }
        } else if starting {
            match target.enter_from {
                Some(from) => {
                    let mut pose = want;
                    pose[0] += from.x;
                    pose[1] += from.y;
                    track.lanes = pose.map(|value| SpringState {
                        value,
                        velocity: 0.0,
                    });
                    track.heading = Some(want);
                    track.settled = false;
                }
                None => track.land(want),
            }
        } else if new_heading {
            track.heading = Some(want);
            track.settled = false;
        }
        if !track.settled && visibility != Some(&Visibility::Hidden) {
            let look = skin_of(&skin, entity, &scopes, &parents);
            let spring = look
                .motion
                .spring(target.spring)
                .scaled(settings.duration_scale);
            let mut settled = true;
            for lane in 0..LANES {
                let state = spring_step(spring, track.lanes[lane], want[lane], delta);
                track.lanes[lane] = if lane_at_rest(spring, lane, state, want[lane]) {
                    SpringState {
                        value: want[lane],
                        velocity: 0.0,
                    }
                } else {
                    settled = false;
                    state
                };
            }
            track.settled = settled;
        }
        let drawn = MotionTrack::transform_of(track.shown());
        if *transform != drawn {
            *transform = drawn;
        }
        if track.drawn != Some(drawn) {
            track.drawn = Some(drawn);
        }
    }
    let spring = skin
        .motion
        .spring(MotionToken::Snappy)
        .scaled(settings.duration_scale);
    for (entity, mut slide, visibility) in &mut popovers {
        // A spent slide is taken off a frame after it reaches its end, so the restyle that
        // [`crate::apply_skin`] makes for the change to 0 sees it.
        if slide.spent {
            commands.entity(entity).remove::<PopoverSlide>();
            continue;
        }
        let next = if off {
            SpringState::default()
        } else if visibility == Some(&Visibility::Hidden) {
            slide.reach
        } else {
            let state = spring_step(spring, slide.reach, 0.0, delta);
            if spring_settled(spring, state, 0.0, crate::model::SPRING_REST_POSITION) {
                SpringState::default()
            } else {
                state
            }
        };
        if slide.reach != next {
            slide.reach = next;
        }
        if next == SpringState::default() && !slide.spent {
            slide.spent = true;
        }
    }
}
