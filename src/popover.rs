//! Popovers: a panel that opens beside a widget, stays inside the window, and closes when the
//! player backs out or presses anywhere else.

use bevy::picking::events::{PointerClick, PointerPress, PointerRelease};
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{PointerAction, PointerId, PointerInput, PointerLocation};
use bevy::prelude::*;
use bevy::ui::{FixedNode, UiScale};
use bevy::ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide};

use crate::cancel::Cancel;

/// Where a [`popover`](crate::popover) opens, relative to the widget it is anchored to.
///
/// The four sides name the side of the anchor the panel prefers. When it does not fit there it is
/// placed on the opposite side, and along the anchor's other edge when that fits it, so a panel
/// is never drawn outside the window while any placement keeps it inside (a panel larger than the
/// window takes the least clipped one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Default, Clone, PartialEq)]
pub enum Placement {
    /// Under the anchor, its left edge on the anchor's, or its right edge on the anchor's when
    /// that leaves more of it in the window; above the anchor when there is no room below.
    #[default]
    Below,
    /// Over the anchor, aligned as [`Placement::Below`] is; below it when there is no room above.
    Above,
    /// Right of the anchor, its top edge on the anchor's, or its bottom edge on the anchor's when
    /// that leaves more of it in the window; left of the anchor when there is no room.
    Right,
    /// Left of the anchor, aligned as [`Placement::Right`] is; right of it when there is no room.
    Left,
    /// At the pointer: below it and to its right, flipped to its left and above it at the window's
    /// edges. The anchor is the widget (or panel) the pointer was over; a pointer press on it
    /// closes the popover like a press anywhere else. For a context menu.
    AtPointer,
}

/// A popover's panel: what a game queries to find the one open for an anchor. Added by
/// [`popover`](crate::popover) and [`popover_modal`](crate::popover_modal); a game never builds one.
///
/// The panel is a child of its anchor, so the anchor going away takes the popover with it. To close
/// one from code, despawn it: no [`PopoverClosed`] is sent for what the game itself did.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct PopoverPanel {
    /// The widget the popover is anchored to.
    pub anchor: Entity,
    /// Where it opens.
    pub placement: Placement,
    /// Whether it holds focus ([`popover_modal`](crate::popover_modal)).
    pub modal: bool,
    /// False until the first frame it has been alive: the press that opened it is never "outside".
    armed: bool,
    /// False until it has been styled and laid out once: it is not drawn before (see
    /// [`reveal_popovers`]).
    shown: bool,
}

/// Why a popover closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum CloseReason {
    /// The player pressed Escape or the pad's East button ([`Cancel`]).
    Cancel,
    /// The player pressed the pointer somewhere outside the panel.
    Outside,
}

/// A popover is closing because of the player, and is despawned right after this event is
/// delivered. Observe it on the popover (`commands.spawn((popover(..), observe(..)))`) or globally.
/// A popover the game despawns, or whose anchor was despawned, sends nothing.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, trigger: Entity) {
/// commands.spawn((
///     popover(trigger, Placement::Below),
///     observe(|closed: On<PopoverClosed>| info!("closed by {:?}", closed.reason)),
///     children![button("Copy"), button("Paste")],
/// ));
/// # }
/// ```
#[derive(EntityEvent, Clone, Debug, Reflect)]
#[reflect(Event, Clone, Debug)]
pub struct PopoverClosed {
    /// The popover.
    #[event_target]
    pub entity: Entity,
    /// The widget it was anchored to.
    pub anchor: Entity,
    /// What closed it.
    pub reason: CloseReason,
}

/// The zero-size node at the pointer that a [`Placement::AtPointer`] popover is anchored to: a
/// child of the popover's anchor, laid out on the window, removed with its popover. A game never
/// builds one.
#[derive(Component, Clone, Copy, Debug)]
pub struct PopoverPoint;

impl PopoverPanel {
    pub(crate) fn new(anchor: Entity, placement: Placement, modal: bool) -> Self {
        Self {
            anchor,
            placement,
            modal,
            armed: false,
            shown: false,
        }
    }
}

/// The candidate positions Bevy's `Popover` chooses from for a placement: the preferred side
/// aligned to the anchor's start, to its end, then centred on it (the shift: a panel wider than the
/// room on one side of the anchor slides along its edge), then the opposite side the same three
/// ways. The first with the least of it outside the window wins, so a placement that fits is
/// never left for another.
pub(crate) fn candidates(placement: Placement, gap: f32) -> Vec<PopoverPlacement> {
    use PopoverSide::{Bottom, Left, Right, Top};
    let (side, mirror) = match placement {
        Placement::Below | Placement::AtPointer => (Bottom, Top),
        Placement::Above => (Top, Bottom),
        Placement::Right => (Right, Left),
        Placement::Left => (Left, Right),
    };
    [side, mirror]
        .into_iter()
        .flat_map(|side| {
            [PopoverAlign::Start, PopoverAlign::End, PopoverAlign::Center]
                .into_iter()
                .map(move |align| PopoverPlacement { side, align, gap })
        })
        .collect()
}

/// A popover's `Popover` positions for a skin's gap and window margin.
pub(crate) fn placement_of(placement: Placement, gap: f32, margin: f32) -> Popover {
    Popover {
        positions: candidates(
            placement,
            if placement == Placement::AtPointer {
                0.0
            } else {
                gap
            },
        ),
        window_margin: margin,
    }
}

/// Triggers [`PopoverClosed`] on `popover` and despawns it. A popover already gone is left alone.
fn close(commands: &mut Commands, popover: Entity, anchor: Entity, reason: CloseReason) {
    commands.trigger(PopoverClosed {
        entity: popover,
        anchor,
        reason,
    });
    commands.entity(popover).try_despawn();
}

/// A [`Cancel`] reaching a popover closes it and goes no further: Escape closed the popover, so
/// the screen under it does not also pause.
pub(crate) fn popover_on_cancel(
    mut cancel: On<Cancel>,
    panels: Query<&PopoverPanel>,
    mut commands: Commands,
) {
    if let Ok(panel) = panels.get(cancel.entity) {
        cancel.propagate(false);
        close(
            &mut commands,
            cancel.entity,
            panel.anchor,
            CloseReason::Cancel,
        );
    }
}

/// A press, release or click on the panel stops at it, so the anchor (a button, say) under it in
/// the hierarchy is never pressed or activated by a press on its popover.
pub(crate) fn stop_press(mut press: On<PointerPress>) {
    press.propagate(false);
}

/// See [`stop_press`].
pub(crate) fn stop_release(mut release: On<PointerRelease>) {
    release.propagate(false);
}

/// See [`stop_press`].
pub(crate) fn stop_click(mut click: On<PointerClick>) {
    click.propagate(false);
}

/// A press on the panel's own padding or text asks for focus; the request stops here, so it does
/// not bubble to the anchor (a button, which would take focus) or to the window (which would
/// clear it). Focus stays where it is.
pub(crate) fn stop_acquire(mut acquire: On<bevy::input_focus::AcquireFocus>) {
    acquire.propagate(false);
}

/// Puts each new [`Placement::AtPointer`] popover at the pointer: a zero-size node, child of the
/// anchor, laid out on the window and not the anchor (`FixedNode`), left and top at the pointer's
/// position, with the panel moved under it. Bevy's `Popover` then places the panel against that
/// point and flips it at the window's edge like any other. Runs in
/// [`crate::GooeySystems::Navigate`].
pub fn anchor_pointer_popovers(
    new: Query<(Entity, &PopoverPanel), Added<PopoverPanel>>,
    pointers: Query<&PointerLocation>,
    ui_scale: Option<Res<UiScale>>,
    mut commands: Commands,
) {
    for (panel, info) in &new {
        if info.placement != Placement::AtPointer {
            continue;
        }
        // The pointer's position is in window logical px; the interface's are those over its scale.
        let scale = ui_scale.as_ref().map_or(1.0, |s| s.0).max(f32::EPSILON);
        let Some(at) = pointers
            .iter()
            .find_map(|p| p.location.as_ref())
            .map(|l| l.position / scale)
        else {
            continue;
        };
        let point = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(at.x),
                    top: Val::Px(at.y),
                    width: Val::Px(0.0),
                    height: Val::Px(0.0),
                    ..default()
                },
                FixedNode,
                PopoverPoint,
                ChildOf(info.anchor),
            ))
            .id();
        commands.entity(panel).insert(ChildOf(point));
    }
}

/// Removes the pointer node of a popover that is gone.
pub fn drop_pointer_anchors(
    points: Query<Entity, (With<PopoverPoint>, Without<Children>)>,
    mut commands: Commands,
) {
    for point in &points {
        commands.entity(point).try_despawn();
    }
}

/// Shows each popover the frame after it was styled. A new popover is born hidden: Bevy lays out
/// a node's text a frame after it first appears, so a card drawn in its first frame would be as
/// narrow as its padding and stand where the next frame's width moves it from. [`crate::apply_skin`]
/// shows it (`Display::Flex`) in the frame it is spawned, the layout of that frame measures it, and
/// this reveals it in the next, at its final place. Runs in [`crate::GooeySystems::Navigate`].
pub fn reveal_popovers(mut panels: Query<(&Node, &mut PopoverPanel, &mut Visibility)>) {
    for (node, mut panel, mut visibility) in &mut panels {
        if !panel.shown && node.display == Display::Flex {
            panel.shown = true;
            *visibility = Visibility::Inherited;
        }
    }
}

/// What a popover is judged by when a press lands: the panel, and whether it still needs arming.
type Open = (Entity, &'static mut PopoverPanel);

/// Closes every popover a pointer press landed outside of, with [`CloseReason::Outside`], once
/// each. Outside is anywhere that is not the panel or, for a popover on a side of its anchor, the
/// anchor itself (a game's own toggle on the anchor opens and closes with one rule); the press that
/// opened a popover, in the frame it opened, is not outside. The press goes on to whatever it
/// landed on. Runs in [`crate::GooeySystems::Navigate`].
pub fn close_popovers_on_press(
    mut input: MessageReader<PointerInput>,
    hover: Option<Res<HoverMap>>,
    mut panels: Query<Open>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let pressed: Vec<PointerId> = input
        .read()
        .filter(|i| matches!(i.action, PointerAction::Press(_)))
        .map(|i| i.pointer_id)
        .collect();
    let mut closed: Vec<Entity> = Vec::new();
    // Outermost first, so a popover a parent popover holds is closed by the parent's close.
    let mut open: Vec<(Entity, Entity, Placement, bool)> = Vec::new();
    for (entity, mut panel) in &mut panels {
        let armed = panel.armed;
        if !armed {
            panel.armed = true;
        }
        open.push((entity, panel.anchor, panel.placement, armed));
    }
    let Some(hover) = hover else {
        return;
    };
    open.sort_by_key(|&(entity, ..)| parents.iter_ancestors(entity).count());
    for (entity, anchor, placement, armed) in open {
        if !armed || parents.iter_ancestors(entity).any(|a| closed.contains(&a)) {
            continue;
        }
        let outside = pressed.iter().any(|pointer| {
            let hits = hover.get(pointer);
            let within = |root: Entity| {
                hits.is_some_and(|hits| {
                    hits.keys()
                        .any(|&e| e == root || parents.iter_ancestors(e).any(|a| a == root))
                })
            };
            !within(entity) && !(placement != Placement::AtPointer && within(anchor))
        });
        if outside {
            closed.push(entity);
            close(&mut commands, entity, anchor, CloseReason::Outside);
        }
    }
}
