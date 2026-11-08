//! Scrolls that glide and fade at their edges: a scroll the crate makes (a key, the pad, Tab or a
//! table's cursor moving a row) travels there on the skin's `motion.default` spring instead of
//! jumping, and a clipped list draws a fade over each edge that hides content.
//!
//! A scroll the player makes is not eased: the wheel, a trackpad and a drag on the rail write
//! `ScrollPosition` themselves, which ends a glide and moves 1:1 as it always did. A request made
//! while one is running keeps the glide's speed and changes its goal, so holding a key to cross a
//! long table scrolls as one motion. Reduced motion, no motion and no clock land a scroll at once
//! ([`MotionSettings::lands_travel`]).
//!
//! The fade is [`ScrollEdgeFade`]: two overlays beside the viewport, each as opaque as its edge
//! hides content, at strength `hidden / 15 px` up to 1 (Rune's ramp). Both are `Pickable::IGNORE`,
//! and an edge with nothing hidden under it is `Display::None`, so a list at rest or at its end
//! draws and writes nothing.
//!
//! Prior art: Rune (<https://github.com/anomalyco/rune> at `f73f72c`): `scroll_to`'s glide and
//! `crates/rune/src/runtime/scroll/edge.rs` (`FADE_IN`, 15). See `docs/research/scroll-glide.md`.

use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, ScrollPosition, UiGlobalTransform};

use crate::model::{SPRING_REST_POSITION, Spring, SpringState, spring_settled, spring_step};
use crate::motion::{MotionSettings, MotionToken};
use crate::skin::{GooeySkin, Role, SkinScope, skin_of};

/// How many logical pixels of hidden content make an edge's fade full strength; below that it is
/// that fraction of full. Rune's `FADE_IN`, so a list that has scrolled a hair does not show a
/// hard band.
pub const SCROLL_FADE_RAMP_PX: f32 = 15.0;

/// How far in from an edge the fade reaches, in logical pixels: a row and a bit.
pub const SCROLL_FADE_PX: f32 = 24.0;

/// How much of the skin's `motion.default` spring a scroll's glide takes: a third, 0.1 s at the
/// default. A key repeats at about 30 Hz, a row each, so a held key moves the goal 1,300 px a second
/// down a table of 44 px rows, and a critically damped spring trails a goal moving at speed `v` by
/// `2v / omega`: at the whole 0.3 s that is 125 px, the cursor row two rows past the viewport's foot.
/// At a third it is 42 px, inside the row of context a request already keeps.
pub const SCROLL_SPRING_FRACTION: f32 = 1.0 / 3.0;

/// A scroll gliding to where the crate asked it to go. It is on every [`crate::ListView`] and
/// holds no goal while the list is at rest. To scroll a list from code and have it glide, call
/// [`ScrollGlide::go`]; to jump, write the `ScrollPosition`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollGlide {
    /// Where the glide is heading, if it is running.
    goal: Option<f32>,
    /// The scroll the glide shows now, and how fast it is moving.
    shown: SpringState,
    /// The scroll the glide last wrote: a `ScrollPosition` that reads otherwise was written by the
    /// wheel, a drag or the game, and that ends the glide.
    drawn: f32,
}

impl ScrollGlide {
    /// Where a running glide is heading, or `None` at rest.
    pub fn goal(&self) -> Option<f32> {
        self.goal
    }

    /// Whether `settings` and the clock let a scroll travel (otherwise it lands at once).
    pub(crate) fn glides(settings: &MotionSettings, time: Option<&Time<Real>>) -> bool {
        time.is_some() && !settings.lands_travel()
    }

    /// Asks for the scroll to glide to `to`, from where `scroll` is. A glide already running keeps
    /// its speed and takes the new goal.
    pub fn go(&mut self, scroll: &ScrollPosition, to: f32) {
        if !to.is_finite() {
            return;
        }
        if self.goal.is_none() || (scroll.y - self.drawn).abs() > TAKEN_OVER {
            self.shown = SpringState {
                value: scroll.y,
                velocity: 0.0,
            };
            self.drawn = scroll.y;
        }
        self.goal = Some(to);
    }
}

/// How far `ScrollPosition` may differ from what a glide wrote and still be the glide's own: a
/// hundredth of a pixel, far above float noise and far below any wheel step.
const TAKEN_OVER: f32 = 0.01;

/// The spring a glide runs on: the skin's `motion.default` times `duration_scale`, a third as long
/// ([`SCROLL_SPRING_FRACTION`]).
pub(crate) fn glide_spring(skin: &GooeySkin, settings: &MotionSettings) -> Spring {
    let spring = skin
        .motion
        .spring(MotionToken::Default)
        .scaled(settings.duration_scale);
    Spring {
        duration: spring.duration * SCROLL_SPRING_FRACTION,
        ..spring
    }
}

/// One frame of a glide: lands the scroll when `lands`, ends the glide when something else wrote
/// the `ScrollPosition` since it did, otherwise steps the spring `delta` seconds toward the goal.
/// Does nothing for a glide at rest.
pub(crate) fn step_glide(
    glide: &mut ScrollGlide,
    scroll: &mut ScrollPosition,
    spring: Spring,
    lands: bool,
    delta: f32,
) {
    let Some(goal) = glide.goal else {
        return;
    };
    if (scroll.y - glide.drawn).abs() > TAKEN_OVER {
        glide.goal = None;
        return;
    }
    if lands {
        if scroll.y != goal {
            scroll.y = goal;
        }
        *glide = ScrollGlide::default();
        return;
    }
    let next = spring_step(spring, glide.shown, goal, delta);
    if spring_settled(spring, next, goal, SPRING_REST_POSITION) {
        scroll.y = goal;
        *glide = ScrollGlide::default();
    } else {
        glide.shown = next;
        glide.drawn = next.value;
        scroll.y = next.value;
    }
}

/// Moves every running [`ScrollGlide`] a frame on the skin's `motion.default` spring (a third as
/// long, [`SCROLL_SPRING_FRACTION`]), and ends one whose `ScrollPosition` something else wrote (the
/// wheel, a drag on the rail, the game). Lands the scroll at once under reduced motion, no motion,
/// or with no clock. A list at rest is skipped before anything is read. A table's viewport is moved
/// by [`crate::update_tables`] itself, so the rows it binds are the rows of the scroll it draws.
/// Runs in [`crate::GooeySystems::Style`], after the systems that ask.
pub fn update_scroll_glides(
    settings: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    time: Option<Res<Time<Real>>>,
    mut glides: Query<(Entity, &mut ScrollGlide, &mut ScrollPosition, Option<&Role>)>,
) {
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let lands = !ScrollGlide::glides(&settings, time.as_deref());
    for (entity, mut glide, mut scroll, role) in &mut glides {
        if glide.goal.is_none() || role == Some(&Role::TableView) {
            continue;
        }
        let look = skin_of(&skin, entity, &scopes, &parents);
        step_glide(
            &mut glide,
            &mut scroll,
            glide_spring(look, &settings),
            lands,
            delta,
        );
    }
}

/// Fades the edges of a scrolling viewport where content is hidden under them: `ScrollEdgeFade(24.0)`
/// reaches 24 logical pixels in from the top while the viewport is scrolled down, and from the
/// bottom while there is more below. At each edge the fade is `hidden px / 15` of full, up to 1
/// ([`SCROLL_FADE_RAMP_PX`]). A list and a table carry one; take it off with `0.0` or by removing
/// it.
///
/// The overlays are two nodes beside the viewport (children of its parent), spawned the first time
/// the component is seen and despawned when it is gone. They are the colour of the nearest
/// ancestor background, or the skin's surface, and `Pickable::IGNORE`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct ScrollEdgeFade(pub f32);

impl Default for ScrollEdgeFade {
    fn default() -> Self {
        Self(SCROLL_FADE_PX)
    }
}

/// Which edge of its viewport an overlay fades.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum FadeEdge {
    /// Over the first visible row, while the content is scrolled down.
    Top,
    /// Over the last visible row, while there is more below.
    Bottom,
}

/// One edge's overlay, and what it last drew (to write nothing while it is unchanged).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ScrollFade {
    /// The viewport it fades.
    pub view: Entity,
    /// The edge it is on.
    pub edge: FadeEdge,
    /// The strength it was last drawn at: 0 hidden, 1 full.
    pub strength: f32,
    drawn: Option<Drawn>,
}

/// What an overlay last wrote.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Drawn {
    strength: f32,
    color: Color,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

/// The overlays a viewport's [`ScrollEdgeFade`] made, so they can be taken down again.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ScrollFadeParts {
    top: Entity,
    bottom: Entity,
}

/// The strength of a fade for `hidden` logical pixels of content under the edge: 0 for none, 1
/// from [`SCROLL_FADE_RAMP_PX`] up.
///
/// ```
/// # use bevy_gooey::*;
/// assert_eq!(scroll_fade_strength(0.0), 0.0);
/// assert_eq!(scroll_fade_strength(7.5), 0.5);
/// assert_eq!(scroll_fade_strength(400.0), 1.0);
/// ```
pub fn scroll_fade_strength(hidden: f32) -> f32 {
    if hidden.is_nan() {
        return 0.0;
    }
    (hidden / SCROLL_FADE_RAMP_PX).clamp(0.0, 1.0)
}

/// Spawns the two overlays for each viewport that has a [`ScrollEdgeFade`] and none yet, and takes
/// them down when the component is gone, the viewport is, or its fade is zero. Runs in
/// [`crate::GooeySystems::Style`].
pub fn manage_scroll_fades(
    mut commands: Commands,
    new: Query<(Entity, &ScrollEdgeFade, &ChildOf), Without<ScrollFadeParts>>,
    kept: Query<(Entity, &ScrollFadeParts, Option<&ScrollEdgeFade>)>,
    fades: Query<(Entity, &ScrollFade)>,
    views: Query<(), With<ScrollEdgeFade>>,
) {
    for (view, fade, parent) in &new {
        if fade.0 <= 0.0 || !fade.0.is_finite() {
            continue;
        }
        let part = |edge| {
            (
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    ..default()
                },
                BackgroundGradient::default(),
                Pickable::IGNORE,
                ScrollFade {
                    view,
                    edge,
                    strength: 0.0,
                    drawn: None,
                },
            )
        };
        let top = commands.spawn(part(FadeEdge::Top)).id();
        let bottom = commands.spawn(part(FadeEdge::Bottom)).id();
        commands
            .entity(parent.parent())
            .add_children(&[top, bottom]);
        commands
            .entity(view)
            .insert(ScrollFadeParts { top, bottom });
    }
    for (view, parts, fade) in &kept {
        if fade.is_none_or(|fade| fade.0 <= 0.0 || !fade.0.is_finite()) {
            commands.entity(parts.top).despawn();
            commands.entity(parts.bottom).despawn();
            commands.entity(view).remove::<ScrollFadeParts>();
        }
    }
    for (overlay, fade) in &fades {
        if !views.contains(fade.view) {
            commands.entity(overlay).try_despawn();
        }
    }
}

/// Sets each overlay from its viewport: where it stands (the viewport's own box, less the rail's
/// lane), how strong it is (the content hidden under its edge over [`SCROLL_FADE_RAMP_PX`]) and
/// the colour it fades from. Writes an overlay only when something it shows changed; an edge with
/// nothing hidden is `Display::None`. Reads the viewport's layout from the last frame, so a fade
/// follows a scroll one frame late. Runs in [`crate::GooeySystems::Style`].
pub fn update_scroll_fades(
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    views: Query<(&ScrollEdgeFade, &ComputedNode, &UiGlobalTransform, &ChildOf)>,
    boxes: Query<(&ComputedNode, &UiGlobalTransform), Without<ScrollFade>>,
    backgrounds: Query<&BackgroundColor>,
    mut overlays: Query<(&mut ScrollFade, &mut Node, &mut BackgroundGradient)>,
) {
    for (mut fade, mut node, mut gradient) in &mut overlays {
        let Ok((reach, view, transform, parent)) = views.get(fade.view) else {
            continue;
        };
        let reach = reach.0;
        let scale = view.inverse_scale_factor();
        let max =
            ((view.content_size().y + view.scrollbar_size.y - view.size().y).max(0.0)) * scale;
        let scrolled = (view.scroll_position.y * scale).clamp(0.0, max);
        let hidden = match fade.edge {
            FadeEdge::Top => scrolled,
            FadeEdge::Bottom => max - scrolled,
        };
        let strength = scroll_fade_strength(hidden);
        let drawn = if strength > 0.0 {
            let Ok((parent_node, parent_transform)) = boxes.get(parent.parent()) else {
                continue;
            };
            // An absolute node is placed from its parent's padding edge: inside the border.
            let view_corner = transform.translation - view.size() * 0.5;
            let parent_corner = parent_transform.translation - parent_node.size() * 0.5;
            let inside = parent_node.border.min_inset;
            let at = (view_corner - parent_corner - inside) * scale;
            let height = reach.min(view.size().y * scale);
            // The rail's lane is the viewport's right padding: the fade stops short of it.
            let width = (view.size().x - view.padding.max_inset.x).max(0.0) * scale;
            let top = match fade.edge {
                FadeEdge::Top => at.y,
                FadeEdge::Bottom => at.y + view.size().y * scale - height,
            };
            let look = skin_of(&skin, fade.view, &scopes, &parents);
            let base = std::iter::once(parent.parent())
                .chain(parents.iter_ancestors(parent.parent()))
                .find_map(|e| {
                    backgrounds
                        .get(e)
                        .ok()
                        .map(|background| background.0)
                        .filter(|color| color.alpha() > 0.0)
                })
                .unwrap_or(look.surface);
            Some(Drawn {
                strength,
                color: base,
                left: at.x,
                top,
                width,
                height,
            })
        } else {
            None
        };
        if fade.drawn == drawn {
            continue;
        }
        fade.drawn = drawn;
        fade.strength = strength;
        match drawn {
            None => {
                if node.display != Display::None {
                    node.display = Display::None;
                }
            }
            Some(drawn) => {
                let next = Node {
                    display: Display::Flex,
                    position_type: PositionType::Absolute,
                    left: Val::Px(drawn.left),
                    top: Val::Px(drawn.top),
                    width: Val::Px(drawn.width),
                    height: Val::Px(drawn.height),
                    ..default()
                };
                if *node != next {
                    *node = next;
                }
                let from = drawn.color.with_alpha(drawn.color.alpha() * drawn.strength);
                let to = drawn.color.with_alpha(0.0);
                let angle = match fade.edge {
                    FadeEdge::Top => LinearGradient::TO_BOTTOM,
                    FadeEdge::Bottom => LinearGradient::TO_TOP,
                };
                let stops = vec![
                    ColorStop::new(from, Val::Percent(0.0)),
                    ColorStop::new(to, Val::Percent(100.0)),
                ];
                *gradient = BackgroundGradient(vec![
                    LinearGradient {
                        color_space: InterpolationColorSpace::Srgba,
                        angle,
                        stops,
                    }
                    .into(),
                ]);
            }
        }
    }
}
