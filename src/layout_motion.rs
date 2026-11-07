//! Layout motion: a widget that layout moves travels to its new place instead of teleporting.
//!
//! [`MovesSmoothly`] on a node says: when layout puts its top-left corner somewhere else inside its
//! parent, draw it where it was and let a spring carry it to the new place. Layout is at the new
//! place from the first frame; only the drawing travels, and with it the picking, since both read
//! the node's [`UiGlobalTransform`]. Children ride their parent. A node whose own size changes
//! with its top-left corner standing still does not move. A move caused by a new window size or
//! scale, a scroll of the parent, or the node's first frame lands at once, and so does every move
//! under reduced or no motion ([`MotionSettings::lands_travel`]).
//!
//! A move also lands while a node is settling in: in its first [`LAYOUT_QUIET_FRAMES`] frames, in
//! the frames after a new window size, scale or skin, and when it is given a new parent, because a
//! fitting pass (a toolbar's overflow, a status bar hiding what does not fit) reads one frame's
//! layout and moves things in the next.
//!
//! The crate marks tabs; a game marks its own. A dock's tabs are spawned again on every change to
//! the dock's layout, so they are new on the only frames they move and do not travel yet (G-63). Rows of a list, a table or a tree are
//! pooled (a reused row is a new row) and are not marked. Toasts glide on their own room already,
//! and toolbar and status items are moved by a fitting pass, so none of them is marked.

use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::ui::{
    ComputedNode, ComputedUiRenderTargetInfo, FixedNode, ScrollPosition, UiGlobalTransform,
    UiTransform,
};

use crate::model::{SPRING_REST_POSITION, SpringState, spring_settled, spring_step};
use crate::motion::{MotionSettings, MotionToken};
use crate::skin::{GooeySkin, Role, SkinScope, skin_of};

/// Travel to a new place in layout on the skin's default spring, instead of jumping there.
///
/// When layout puts the node's top-left corner somewhere else inside its parent, the node is drawn
/// where it was and a spring carries it to the new place, keeping its speed if it is moved again on
/// the way. Layout is at the new place from the first frame: only the drawing travels, and the
/// picking with it. Children ride the node, and a node that only grows does not move. A move lands
/// at once in the node's first [`LAYOUT_QUIET_FRAMES`] frames, in those after a new window size,
/// scale, skin or parent, on a scroll of its parent, and under reduced or no motion
/// ([`MotionSettings::lands_travel`]). [`update_layout_motion`] does it; [`LayoutTravel`] says how
/// far from its place the node is drawn.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // A row of cards that slide over when one is taken out.
/// commands.spawn((Node::default(), children![(panel(), MovesSmoothly), (panel(), MovesSmoothly)]));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(LayoutTravel)]
pub struct MovesSmoothly;

/// What [`update_layout_motion`] keeps of a [`MovesSmoothly`] node: where layout last put its
/// top-left corner inside its parent, logical px, and how far from there it is drawn now.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutTravel {
    /// The corner's place inside the parent at the last look, logical px; `None` before the first.
    placed: Option<Vec2>,
    /// How far from its layout place the node is drawn, logical px, and how fast that changes.
    offset: [SpringState; 2],
    /// Frames left in which a move lands instead of travelling.
    quiet: u8,
    /// The parent the place was read in.
    parent: Option<Entity>,
}

/// How many frames a move lands after a node first appears, gets a new parent, or sees a new
/// window size, scale or skin: the frame of the change and the one after, in which a fitting pass
/// that read the first moves things again.
pub const LAYOUT_QUIET_FRAMES: u8 = 2;

impl LayoutTravel {
    /// How far from where layout puts it the node is drawn now, logical px.
    pub fn offset(&self) -> Vec2 {
        Vec2::new(self.offset[0].value, self.offset[1].value)
    }

    fn at_rest(&self) -> bool {
        self.offset == [SpringState::default(); 2]
    }

    fn land(&mut self) {
        self.offset = [SpringState::default(); 2];
    }
}

/// A node that just got its role.
type NewPart = (Entity, &'static Role);

/// A node that just got its role and is not marked yet.
type NewPartFilter = (Added<Role>, Without<MovesSmoothly>);

/// Gives the crate's own moving parts [`MovesSmoothly`] the frame they get their role: tabs. A game
/// that takes it off one keeps it off, and its motion stops. Runs in
/// [`crate::GooeySystems::Style`].
pub fn mark_moving_parts(mut commands: Commands, added: Query<NewPart, NewPartFilter>) {
    for (entity, role) in &added {
        if *role == Role::Tab {
            commands.entity(entity).insert(MovesSmoothly);
        }
    }
}

/// What the layout motion reads of a moving node.
type Traveller = (
    Entity,
    &'static mut LayoutTravel,
    &'static ComputedNode,
    &'static UiTransform,
    Ref<'static, ComputedUiRenderTargetInfo>,
    &'static ChildOf,
);

/// The shape of the tree a move is drawn over.
type Frames<'w, 's> = (
    Query<'w, 's, &'static ComputedNode>,
    Query<'w, 's, Ref<'static, ScrollPosition>>,
    Query<'w, 's, &'static Children>,
    Query<'w, 's, &'static ChildOf>,
    Query<'w, 's, (), With<FixedNode>>,
);

/// The lists a run reuses, so a move allocates nothing per frame.
#[derive(Default)]
pub struct Scratch {
    moves: Vec<(Entity, Vec2)>,
    stack: Vec<Entity>,
}

/// Moves every [`MovesSmoothly`] node's drawing toward where layout has just put it. Runs in
/// `PostUpdate` after Bevy's layout and before its clipping, so the frame a move happens is drawn
/// from the old place.
///
/// Each frame it reads where layout put the node's top-left corner inside its parent, in the
/// parent's own space and without the node's own [`UiTransform`], so a [`crate::MotionTarget`] on
/// the node or a scale on an ancestor is not read as a move. When the corner moved it starts, or
/// retargets keeping the speed, a spring from the old place, and it adds the drawn offset to the
/// [`UiGlobalTransform`] of the node and every node under it that layout places from it (not a
/// [`FixedNode`], which layout places on the window). A node at rest, or hidden
/// (`Display::None` lays it out empty), is read and not written.
#[allow(clippy::too_many_arguments)]
pub fn update_layout_motion(
    settings: Res<MotionSettings>,
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    time: Option<Res<Time<Real>>>,
    mut travellers: Query<Traveller, With<MovesSmoothly>>,
    frames: Frames,
    mut globals: Query<&mut UiGlobalTransform>,
    mut scratch: Local<Scratch>,
) {
    let (nodes, scrolls, children, parents, fixed) = frames;
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let lands = settings.lands_travel() || time.is_none();
    let Scratch { moves, stack } = &mut *scratch;
    moves.clear();
    // Every corner is read before any drawing is moved, so a moving parent's own offset is never
    // read into its child's place.
    for (entity, mut travel, node, own, target, parent) in &mut travellers {
        let (Ok(parent_node), Ok(parent_at), Ok(at)) = (
            nodes.get(parent.parent()),
            globals.get(parent.parent()),
            globals.get(entity),
        ) else {
            continue;
        };
        // A hidden node (or one under a hidden parent) is laid out empty at its parent's corner:
        // that is no place to travel from, so it starts over when it is shown.
        if node.is_empty() || parent_node.is_empty() {
            if travel.placed.is_some() || !travel.at_rest() {
                travel.placed = None;
                travel.land();
            }
            continue;
        }
        let parent_space = Affine2::from(*parent_at);
        if parent_space.matrix2.determinant() == 0.0 {
            continue;
        }
        let physical = target.physical_size().as_vec2();
        let own_shift = own.translation.resolve(
            target.scale_factor(),
            node.size(),
            physical,
            node.em_size,
            node.rem_size,
        );
        // The node's centre in its parent's space, as layout placed it, then its top-left corner
        // from the parent's: physical px, then logical.
        let centre = parent_space.inverse().transform_point2(at.translation) - own_shift;
        let scale = node.inverse_scale_factor();
        let placed = (centre - node.size() / 2.0 + parent_node.size() / 2.0) * scale;
        if !placed.is_finite() {
            continue;
        }
        let scrolled = scrolls.get(parent.parent()).is_ok_and(|s| s.is_changed());
        if travel.placed.is_none()
            || travel.parent != Some(parent.parent())
            || target.is_changed()
            || skin.is_changed()
        {
            travel.quiet = LAYOUT_QUIET_FRAMES;
            travel.parent = Some(parent.parent());
            travel.placed = None;
        }
        let quiet = travel.quiet > 0;
        if quiet {
            travel.quiet -= 1;
        }
        match travel.placed {
            Some(was) if was != placed => {
                if lands || quiet || scrolled {
                    travel.land();
                } else {
                    let jump = was - placed;
                    travel.offset[0].value += jump.x;
                    travel.offset[1].value += jump.y;
                }
                travel.placed = Some(placed);
            }
            Some(_) => {}
            None => travel.placed = Some(placed),
        }
        if travel.at_rest() {
            continue;
        }
        if lands {
            travel.land();
            continue;
        }
        let spring = skin_of(&skin, entity, &scopes, &parents)
            .motion
            .spring(MotionToken::Default)
            .scaled(settings.duration_scale);
        for lane in &mut travel.offset {
            let next = spring_step(spring, *lane, 0.0, delta);
            *lane = if spring_settled(spring, next, 0.0, SPRING_REST_POSITION) {
                SpringState::default()
            } else {
                next
            };
        }
        if !travel.at_rest() {
            // The offset is in the parent's space: turned and scaled with it on the screen.
            moves.push((entity, parent_space.matrix2 * (travel.offset() / scale)));
        }
    }
    for &(entity, shift) in moves.iter() {
        stack.clear();
        stack.push(entity);
        while let Some(node) = stack.pop() {
            // A node layout does not place from this one keeps its own place, and so does all of
            // what is under it.
            if node != entity && fixed.contains(node) {
                continue;
            }
            let Ok(mut at) = globals.get_mut(node) else {
                continue;
            };
            let mut moved = Affine2::from(*at);
            moved.translation += shift;
            *at = moved.into();
            if let Ok(kids) = children.get(node) {
                stack.extend(kids.iter());
            }
        }
    }
}
