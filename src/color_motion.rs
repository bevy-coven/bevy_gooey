//! Colour motion: a widget's colours spring to the ones [`crate::apply_skin`] gives it, in
//! premultiplied Oklab, instead of repainting in one frame.
//!
//! Every styled node ([`Role`]) carries a [`ColorTrack`]. When a colour of its background, its
//! border, its text or its icon's tint is written with a new value, the track takes that value as
//! its target, puts back the colour it showed and moves from there, on the skin's springs: a state
//! change (hover, focus, selection, checked, disabled) on [`MotionTokens::snappy`], a skin change on
//! [`MotionTokens::calm`], text and icon tints in half their ground's time so a label leads its fill.
//! A press lands at once and its release eases. Reference: Rune's README, "Animation" (colours on
//! springs in premultiplied Oklab, text at half the duration, a theme switch on calm); see
//! `docs/research/color-motion.md`.

use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::{ScrollbarDragState, SliderDragState};

use crate::MotionTokens;
use crate::model::{Spring, SpringState, spring_settled, spring_step};
use crate::motion::MotionSettings;
use crate::selection_indicator::Indicating;
use crate::skin::{GooeySkin, Role, SkinScope, skin_of};
use crate::table::TableRow;

/// How near a colour channel must be to its target, in premultiplied Oklab units, to count as
/// there: a quarter of one 8-bit sRGB step of lightness, so the last frame cannot be seen to snap.
const COLOR_REST_OKLAB: f32 = 1.0 / 1024.0;

/// An alpha this small has no colour to show: the straight colour is taken from the target.
const ALPHA_EPSILON: f32 = 1.0e-4;

/// How a label's and an icon's colours run against their ground's: in half the time (Rune).
const TEXT_DURATION_FACTOR: f32 = 0.5;

/// How far up the tree a press, a drag or a rebound table row is looked for: a toggle's, a slider's
/// or a table row's parts follow its state.
const HELD_DEPTH: usize = 3;

/// How many frames a node's colours land from its first: its skin is written in the first, and a
/// widget a host spawns late in a frame is styled in the next (the convention of layout motion).
const QUIET_FRAMES: u8 = 2;

/// The colours a track moves, in order: background, border top, right, bottom, left, text, icon.
const LANES: usize = 7;
const BACKGROUND: usize = 0;
const BORDER: [usize; 4] = [1, 2, 3, 4];
const TEXT: usize = 5;
const ICON: usize = 6;

/// One colour on its way: what it was last written as, where it is going, and the spring state of
/// each channel of premultiplied Oklab (L, a and b times alpha, then alpha).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Lane {
    /// The value the component holds as this track left it; `None` until the track has seen it.
    written: Option<Color>,
    /// The colour it is moving to (the last one written by anyone else).
    target: Color,
    channels: [SpringState; 4],
    spring: Option<Spring>,
    moving: bool,
}

impl Lane {
    fn land(&mut self, color: Color) {
        self.written = Some(color);
        self.target = color;
        self.channels = premultiplied(color).map(|value| SpringState {
            value,
            velocity: 0.0,
        });
        self.moving = false;
    }

    /// The target's alpha changed while the colour is on its way (a fade over a change): the colour
    /// goes on to the same target at the new alpha, from where it is drawn.
    fn follow_alpha(&mut self, now: Color) {
        let old = self.channels[3].value;
        let new = premultiplied(now)[3];
        if old > ALPHA_EPSILON {
            // A channel is lightness (or a, or b) times alpha. The straight value keeps its place
            // and its speed through the change of alpha: d(c)/dt = alpha * dL/dt + L * d(alpha)/dt,
            // so the new speed is the old one with the alpha's own part taken out, scaled to the
            // new alpha. Scaling the speed by `new / old` instead made a colour that moves while it
            // fades in from nearly nothing start a hundred times too fast and overshoot its target
            // (a menu's lit row flashed twice as bright as its colour as the menu faded in).
            let ratio = new / old;
            let alpha_speed = self.channels[3].velocity;
            for channel in &mut self.channels[..3] {
                let straight = channel.value / old;
                channel.velocity = ratio * (channel.velocity - straight * alpha_speed);
                channel.value *= ratio;
            }
        }
        self.channels[3] = SpringState {
            value: new,
            velocity: 0.0,
        };
        self.target = now;
    }

    /// One frame of the lane, given what the component holds now: the colour to write, if any.
    fn frame(&mut self, now: Color, choice: Choice, dt: f32) -> Option<Color> {
        let Some(written) = self.written else {
            self.land(now);
            return None;
        };
        if now != written && now != self.target {
            // A new colour from the skin, the widget or the game. A change of alpha alone is a fade,
            // which presence and toasts already run: it is followed, never held back.
            let fade = same_hue(now, self.target);
            match choice.spring {
                Some(_) if fade && self.moving => self.follow_alpha(now),
                Some(spring) if !fade => {
                    self.target = now;
                    self.spring = Some(spring);
                    self.moving = true;
                }
                _ => {
                    self.land(now);
                    return None;
                }
            }
        }
        if self.moving && choice.spring.is_none() {
            // Motion was switched off, or the widget was pressed, while this was on its way.
            let target = self.target;
            self.land(target);
            return (target != now).then_some(target);
        }
        let spring = self.spring.filter(|_| self.moving)?;
        let goal = premultiplied(self.target);
        let mut settled = true;
        for (state, &goal) in self.channels.iter_mut().zip(&goal) {
            let next = spring_step(spring, *state, goal, dt);
            if spring_settled(spring, next, goal, COLOR_REST_OKLAB) {
                *state = SpringState {
                    value: goal,
                    velocity: 0.0,
                };
            } else {
                settled = false;
                *state = next;
            }
        }
        let shown = if settled {
            self.moving = false;
            self.target
        } else {
            straight(self.channels.map(|c| c.value), self.target)
        };
        self.written = Some(shown);
        (shown != now).then_some(shown)
    }
}

/// What a styled node's colours are drawn as while they move to the skin's: the track
/// [`update_color_motion`] keeps on every [`Role`]. Internal; a game reads the colour components.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct ColorTrack {
    lanes: [Lane; LANES],
    /// How many frames the track has run, up to [`QUIET_FRAMES`]: the colours of those land.
    age: u8,
}

impl ColorTrack {
    /// Whether any colour of the node is on its way.
    pub fn is_moving(&self) -> bool {
        self.lanes.iter().any(|lane| lane.moving)
    }
}

/// How this frame's new colours move: on which spring, or `None` to land.
#[derive(Clone, Copy)]
struct Choice {
    spring: Option<Spring>,
}

/// The colours that are data, not the skin's: drawn as they are written, never eased.
fn is_data(role: &Role) -> bool {
    matches!(
        role,
        Role::Tone(_) | Role::ColorSwatchHalf | Role::ColorArea | Role::ColorRail | Role::LedDot
    )
}

/// Whether two colours differ in alpha alone.
fn same_hue(a: Color, b: Color) -> bool {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    a.red == b.red && a.green == b.green && a.blue == b.blue
}

/// A colour as premultiplied Oklab: L, a and b times alpha, then alpha.
fn premultiplied(color: Color) -> [f32; 4] {
    let lab = Oklaba::from(color);
    [
        lab.lightness * lab.alpha,
        lab.a * lab.alpha,
        lab.b * lab.alpha,
        lab.alpha,
    ]
}

/// Premultiplied Oklab back to a colour; with no alpha left, the target's straight colour.
fn straight(channels: [f32; 4], target: Color) -> Color {
    let alpha = channels[3].clamp(0.0, 1.0);
    if alpha < ALPHA_EPSILON {
        return Color::Oklaba(Oklaba::from(target).with_alpha(alpha));
    }
    Color::Oklaba(Oklaba::new(
        channels[0] / alpha,
        channels[1] / alpha,
        channels[2] / alpha,
        alpha,
    ))
}

/// What holds a node's colours still: a selection indicator that took an option over or gave it
/// back, a press, a slider's or a scrollbar's drag, a rebound table row.
type Holds<'w, 's> = (
    Query<'w, 's, Ref<'static, Indicating>>,
    Query<'w, 's, (), With<Pressed>>,
    Query<'w, 's, &'static SliderDragState>,
    Query<'w, 's, &'static ScrollbarDragState>,
    Query<'w, 's, Ref<'static, TableRow>>,
);

/// A track's pieces of a node: its role, its track and every colour it may carry.
type Colored = (
    Entity,
    &'static Role,
    &'static mut ColorTrack,
    Option<&'static mut BackgroundColor>,
    Option<&'static mut BorderColor>,
    Option<&'static mut TextColor>,
    Option<&'static mut ImageNode>,
    Option<&'static InheritedVisibility>,
);

/// Moves every styled node's colours toward the last ones written to it, on the skin's springs and
/// the clock of [`Time<Real>`] times [`MotionSettings::duration_scale`]. A node's first two frames,
/// a node that is not drawn, a pressed or dragged one, one whose table row was rebound, a fade's
/// change of alpha and the colours that are data land (and so does a colour on its way when motion
/// is switched off). Reduced motion eases over [`MotionTokens::fade_seconds`]. Writes only a colour
/// that differs, and reads nothing of a settled node, so a settled interface writes nothing. Runs
/// after [`crate::GooeySystems::Style`].
#[allow(clippy::too_many_arguments)]
pub fn update_color_motion(
    settings: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    time: Option<Res<Time<Real>>>,
    scopes: Query<&SkinScope>,
    changed_scopes: Query<(), Changed<SkinScope>>,
    parents: Query<&ChildOf>,
    holds: Holds,
    mut tracks: Query<Colored>,
) {
    let (indicating, pressed, sliders, bars, rows) = holds;
    let dt = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let off = settings.is_off() || time.is_none();
    let reskinned = skin.is_changed() || !changed_scopes.is_empty();
    let held_at = |at: Entity| {
        pressed.contains(at)
            || sliders.get(at).is_ok_and(|d| d.dragging)
            || bars.get(at).is_ok_and(|d| d.dragging)
            || rows.get(at).is_ok_and(|r| r.is_changed())
    };
    for (entity, role, mut track, background, border, text, image, visible) in &mut tracks {
        // A settled node whose colours nobody wrote costs nothing: it is skipped before any lookup.
        if track.age >= QUIET_FRAMES
            && !track.is_moving()
            && !background.as_ref().is_some_and(|c| c.is_changed())
            && !border.as_ref().is_some_and(|c| c.is_changed())
            && !text.as_ref().is_some_and(|c| c.is_changed())
            && !image.as_ref().is_some_and(|c| c.is_changed())
        {
            continue;
        }
        let quiet = track.age < QUIET_FRAMES;
        if quiet {
            track.age += 1;
        }
        let held = || {
            // An option the selection indicator has just taken over, or given back, lands: the
            // indicator is what moves, and the fill must not fade over it.
            if indicating.get(entity).is_ok_and(|mark| mark.is_changed()) {
                return true;
            }
            let mut at = entity;
            for _ in 0..=HELD_DEPTH {
                if held_at(at) {
                    return true;
                }
                match parents.get(at) {
                    Ok(parent) => at = parent.parent(),
                    Err(_) => return false,
                }
            }
            false
        };
        let drawn = visible.is_none_or(|v| v.get());
        let ground = if off || quiet || is_data(role) || !drawn || held() {
            None
        } else {
            let motion: &MotionTokens = &skin_of(&skin, entity, &scopes, &parents).motion;
            if settings.reduced {
                (motion.fade_seconds > 0.0).then(|| {
                    Spring {
                        duration: motion.fade_seconds,
                        bounce: 0.0,
                    }
                    .scaled(settings.duration_scale)
                })
            } else if reskinned {
                Some(motion.calm.scaled(settings.duration_scale))
            } else {
                Some(motion.snappy.scaled(settings.duration_scale))
            }
        };
        let fill = Choice { spring: ground };
        let label = Choice {
            spring: ground.map(|spring| spring.scaled(TEXT_DURATION_FACTOR)),
        };
        let lanes = &mut track.lanes;
        if let Some(mut background) = background
            && let Some(color) = lanes[BACKGROUND].frame(background.0, fill, dt)
        {
            background.0 = color;
        }
        if let Some(mut border) = border {
            let sides = [border.top, border.right, border.bottom, border.left];
            let next: [Option<Color>; 4] =
                std::array::from_fn(|side| lanes[BORDER[side]].frame(sides[side], fill, dt));
            if next.iter().any(Option::is_some) {
                let [top, right, bottom, left] =
                    std::array::from_fn(|side| next[side].unwrap_or(sides[side]));
                *border = BorderColor {
                    top,
                    right,
                    bottom,
                    left,
                };
            }
        }
        if let Some(mut text) = text
            && let Some(color) = lanes[TEXT].frame(text.0, label, dt)
        {
            text.0 = color;
        }
        if let Some(mut image) = image
            && let Some(color) = lanes[ICON].frame(image.color, label, dt)
        {
            image.color = color;
        }
    }
}
