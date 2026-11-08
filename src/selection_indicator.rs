//! The selection indicator: one accent node behind a tab strip's or a toggle group's options that
//! springs from the option that was lit to the one that is, so the eye can follow which took over.
//!
//! Prior art: Rune's segmented control and tab strip (<https://github.com/anomalyco/rune> at
//! `f73f72c`, rune-kit `segmented.rs`, `shell/tab_strip.rs`). See
//! `docs/research/selection-indicator.md`.
//!
//! The indicator is two absolute nodes in the widget ([`crate::toggle_group`], [`crate::tabs`],
//! [`crate::side_tabs`]), hidden at rest: the options' own fills carry the look, as they always did.
//! One is a backdrop of the strip's own rect in the raised fill, and the other, its child, is the
//! puck in the accent. While the puck travels, [`update_selection_indicators`] marks the options
//! [`Indicating`] and [`crate::apply_skin`] draws them clear, so the backdrop and the puck are what
//! show; the puck springs from the old option's rect to the new one's on the skin's `motion.snappy`
//! spring. It lands on the lit option's rect and the options take their fills back in that frame
//! (colour motion lands them rather than easing them), so nothing jumps. Reduced motion, no motion
//! and no clock never start a travel, and a strip that reflows at rest moves nothing.

use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::{Checked, ComputedNode, Selected, UiGlobalTransform};

use crate::model::{SPRING_REST_POSITION, SpringState, spring_settled, spring_step};
use crate::motion::{MotionSettings, MotionToken};
use crate::skin::{GooeySkin, Role, SkinScope, skin_of};

/// The four numbers the puck is laid out by: left, top, width and height, logical px.
const LANES: usize = 4;

/// On an option of a [`crate::toggle_group`], a [`crate::tabs`] strip or [`crate::side_tabs`] while
/// the [`SelectionIndicator`] travels. [`crate::apply_skin`] draws a marked option with no fill, so
/// the indicator is what shows, and colour motion lands its colours in the frame the mark changes.
/// It is on every such option (false at rest) so that a change to it restyles.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct Indicating(pub bool);

/// The indicator of a [`crate::toggle_group`], [`crate::tabs`] or [`crate::side_tabs`]: an absolute
/// node in the widget's root (its last child, drawn behind the others by `ZIndex(-1)`) holding the
/// puck, shown only while the puck travels. Part of what those spawn; a game never builds one. Its
/// fields are its own; the node is the strip's backdrop, and the [`IndicatorPuck`] child is what
/// moves.
#[derive(Component, Clone, Debug, Default)]
pub struct SelectionIndicator {
    /// The option the puck last rested on or travelled to.
    lit: Option<Entity>,
    /// Where the puck is drawn now, and how fast each lane is moving.
    lanes: [SpringState; LANES],
    /// Where it is heading.
    goal: [f32; LANES],
    /// Whether the puck has been placed on an option at least once: a first lit option is not
    /// travelled to.
    placed: bool,
    /// Whether it is travelling.
    moving: bool,
}

impl SelectionIndicator {
    /// Whether the puck is travelling between two options.
    pub fn is_moving(&self) -> bool {
        self.moving
    }

    /// The option the puck rests on, or is travelling to.
    pub fn lit(&self) -> Option<Entity> {
        self.lit
    }
}

/// The accent node that moves inside a [`SelectionIndicator`]'s backdrop. Part of what
/// [`crate::toggle_group`], [`crate::tabs`] and [`crate::side_tabs`] spawn.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct IndicatorPuck;

/// The bundle of an indicator: a backdrop that is absolute, behind its siblings, clear of the
/// pointer and not drawn until the puck travels, and the puck in it.
pub(crate) fn selection_indicator() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::NONE),
        // Behind its siblings, though it is the last of them: the options' and the strip's own
        // indices are what they always were.
        ZIndex(-1),
        SelectionIndicator::default(),
        Pickable::IGNORE,
        children![(
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::NONE),
            IndicatorPuck,
            Pickable::IGNORE,
        )],
    )
}

/// What an option is read by: where it is laid out, and whether it is the lit one.
type OptionParts = (
    &'static ComputedNode,
    &'static UiGlobalTransform,
    Has<Selected>,
    Has<Checked>,
);

/// The top-left of a laid-out node, in physical px, and the scale to logical.
fn corner(node: &ComputedNode, at: &UiGlobalTransform) -> (Vec2, f32) {
    (
        at.translation - node.size() * 0.5,
        node.inverse_scale_factor(),
    )
}

/// Moves every [`SelectionIndicator`]: finds the lit option of its widget, starts a travel when a
/// different one is lit than last time, springs the puck toward the lit option's rect, and lands it.
/// Runs in [`crate::GooeySystems::Style`], after the systems that light an option and before
/// [`crate::apply_skin`], so the fills it asks for are drawn in the frame the change happens.
///
/// Reads the options' layout from the last frame. A widget with nothing lit, or whose options have
/// no layout yet, hides its indicator. Writes a node and a mark only when they change, and
/// allocates nothing, so a widget at rest costs a comparison.
#[allow(clippy::too_many_arguments)]
pub fn update_selection_indicators(
    settings: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    time: Option<Res<Time<Real>>>,
    children: Query<&Children>,
    roles: Query<&Role>,
    layouts: Query<OptionParts>,
    mut indicators: Query<
        (Entity, &mut SelectionIndicator, &mut Node, &ChildOf),
        Without<IndicatorPuck>,
    >,
    mut pucks: Query<(&mut Node, &mut BackgroundColor), With<IndicatorPuck>>,
    mut backdrops: Query<&mut BackgroundColor, Without<IndicatorPuck>>,
    mut marks: Query<&mut Indicating>,
) {
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let travels = time.is_some() && !settings.lands_travel();
    for (entity, mut state, mut node, parent) in &mut indicators {
        let host = parent.parent();
        // The widget holding the options: a group's own children, or a tab strip's.
        let holder = if roles.get(host) == Ok(&Role::Tabs) {
            children.get(host).ok().and_then(|kids| {
                kids.iter()
                    .find(|&kid| roles.get(kid) == Ok(&Role::TabList))
            })
        } else {
            Some(host)
        };
        let Some(holder) = holder else {
            continue;
        };
        let lit = children.get(holder).ok().and_then(|kids| {
            kids.iter()
                .find(|&kid| layouts.get(kid).is_ok_and(|(_, _, s, c)| s || c))
        });
        let was_moving = state.moving;
        // The puck's rect in the holder (the backdrop is the holder's rect), and the backdrop's in
        // the host.
        let goal = lit.and_then(|lit| {
            let (option, option_at, ..) = layouts.get(lit).ok()?;
            let (holder_node, holder_at, ..) = layouts.get(holder).ok()?;
            if option.size().min_element() <= 0.0 {
                return None;
            }
            let ((from, scale), (to, _)) =
                (corner(holder_node, holder_at), corner(option, option_at));
            let at = (to - from) * scale;
            Some([at.x, at.y, option.size().x * scale, option.size().y * scale])
        });
        let Some(goal) = goal else {
            if state.moving {
                *state = SelectionIndicator::default();
                mark(&mut marks, &children, holder, false);
                hide(&mut node);
            }
            continue;
        };

        if state.lit != lit {
            if state.placed && travels {
                // The lanes stay where they are drawn, so a change mid-flight turns from there.
                state.moving = true;
            } else {
                state.moving = false;
                state.lanes = goal.map(|value| SpringState {
                    value,
                    velocity: 0.0,
                });
            }
            state.lit = lit;
            state.placed = true;
        } else if !state.moving && state.goal != goal {
            // The options reflowed at rest: the rect they will be drawn at moves with them, nothing
            // travels.
            state.lanes = goal.map(|value| SpringState {
                value,
                velocity: 0.0,
            });
        }
        if state.goal != goal {
            state.goal = goal;
        }

        if state.moving {
            if travels {
                let look = skin_of(&skin, entity, &scopes, &parents);
                let spring = look
                    .motion
                    .spring(MotionToken::Snappy)
                    .scaled(settings.duration_scale);
                let mut settled = true;
                for (lane, &want) in state.lanes.iter_mut().zip(&goal) {
                    let next = spring_step(spring, *lane, want, delta);
                    if spring_settled(spring, next, want, SPRING_REST_POSITION) {
                        *lane = SpringState {
                            value: want,
                            velocity: 0.0,
                        };
                    } else {
                        *lane = next;
                        settled = false;
                    }
                }
                if settled {
                    state.moving = false;
                }
            } else {
                state.moving = false;
                state.lanes = goal.map(|value| SpringState {
                    value,
                    velocity: 0.0,
                });
            }
        }

        if state.moving != was_moving {
            mark(&mut marks, &children, holder, state.moving);
        }
        if !state.moving {
            hide(&mut node);
            continue;
        }
        // The backdrop stands where the holder does; the puck is where it is drawn.
        let Ok((holder_node, holder_at, ..)) = layouts.get(holder) else {
            continue;
        };
        let Some(host_layout) = layouts.get(host).ok() else {
            continue;
        };
        let ((host_corner, scale), (holder_corner, _)) = (
            corner(host_layout.0, host_layout.1),
            corner(holder_node, holder_at),
        );
        let at = (holder_corner - host_corner) * scale;
        let size = holder_node.size() * scale;
        let look = skin_of(&skin, entity, &scopes, &parents);
        let round = BorderRadius::all(Val::Px(look.radius));
        let next = Node {
            position_type: PositionType::Absolute,
            display: Display::Flex,
            left: Val::Px(at.x),
            top: Val::Px(at.y),
            width: Val::Px(size.x),
            height: Val::Px(size.y),
            border_radius: round,
            ..default()
        };
        if *node != next {
            *node = next;
        }
        if let Ok(mut fill) = backdrops.get_mut(entity) {
            fill.set_if_neq(BackgroundColor(look.surface_raised));
        }
        let [left, top, width, height] = state.lanes.map(|lane| lane.value);
        if let Some(puck) = children
            .get(entity)
            .ok()
            .and_then(|kids| kids.iter().find(|&kid| pucks.contains(kid)))
            && let Ok((mut puck_node, mut puck_fill)) = pucks.get_mut(puck)
        {
            let next = Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                width: Val::Px(width),
                height: Val::Px(height),
                border_radius: round,
                ..default()
            };
            if *puck_node != next {
                *puck_node = next;
            }
            puck_fill.set_if_neq(BackgroundColor(look.accent));
        }
    }
}

/// Hides an indicator at rest.
fn hide(node: &mut Mut<Node>) {
    if node.display != Display::None {
        node.display = Display::None;
    }
}

/// Sets [`Indicating`] on a holder's options.
fn mark(marks: &mut Query<&mut Indicating>, children: &Query<&Children>, holder: Entity, on: bool) {
    let Ok(kids) = children.get(holder) else {
        return;
    };
    for &kid in kids {
        if let Ok(mut mark) = marks.get_mut(kid) {
            mark.set_if_neq(Indicating(on));
        }
    }
}
