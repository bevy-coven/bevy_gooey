//! Drag and drop with payloads: a source carries a typed value ([`DragPayload`]), a target takes it
//! ([`DropTarget`]), and a drop is one event ([`Dropped`], [`DropRefused`], [`DroppedOnWorld`]).
//! Nothing here moves a widget: the game does, in the observer of the drop.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::resource::IsResource;
use bevy::ecs::world::DeferredWorld;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart};
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, InteractionDisabled, UiGlobalTransform, UiScale};

use crate::model::{Toward, target_step, target_toward};
use crate::nav::show_focus;
use crate::shortcut::{Modifiers, Shortcut};
use crate::skin::Role;

/// How far, in logical pixels, a press must move before it is a drag: a click with a shaking hand
/// is still a click.
pub const DRAG_LIFT_PX: f32 = 6.0;

/// The `GlobalZIndex` of the ghost and the landing mark: over a dialog (100) and a toast (150),
/// under a binding's catcher.
const DRAG_Z: i32 = 900;

/// How far the ghost stands from the pointer, so the pointer sees what it is over.
const GHOST_OFFSET_PX: f32 = 14.0;

/// What the carried thing is, for the ghost beside the pointer. Without one the ghost shows the first
/// text the source holds (a row's name, a button's label).
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct DragLabel(pub String);

/// The keys that carry a payload without a pointer, so a drag is never the only way (WCAG 2.2,
/// 2.5.7): a keyboard or gamepad player focuses a source ([`DragPayload`]), presses
/// [`DragSettings::pick_up`] (a pad's [`DragSettings::pick_up_button`]), steers the mark with the
/// arrows or the D-pad (PageUp and PageDown, LB and RB step through the targets in reading
/// order), and drops with the same key again (a pad's South too). Escape or East puts it down. A
/// game that wants other keys writes this resource; a keymap the player edits is this value.
///
/// The pick-up key is not Enter, Space or South on purpose: Bevy's `Button` activates on those, and
/// a source that is a [`button`](crate::button) would be both pressed and lifted.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// let settings = DragSettings {
///     pick_up: Shortcut::new(Modifiers::NONE, KeyCode::F2),
///     ..default()
/// };
/// assert_eq!(settings.pick_up_button, GamepadButton::West);
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct DragSettings {
    /// The chord that lifts the focused source and, while one is carried, puts it down on the
    /// marked target. Default: Alt+M.
    pub pick_up: Shortcut,
    /// The pad button that does the same, on any gamepad. Default: West (X on an Xbox pad).
    pub pick_up_button: GamepadButton,
}

impl Default for DragSettings {
    fn default() -> Self {
        Self {
            pick_up: Shortcut::new(Modifiers::ALT, KeyCode::KeyM),
            pick_up_button: GamepadButton::West,
        }
    }
}

/// Makes a node a drag source: pressing it and moving more than [`DRAG_LIFT_PX`] picks `T` up. The
/// node needs [`Pickable`] (every gooey widget has it; a sprite or mesh needs it added).
///
/// While it is carried the source has [`Dragging`]. A drag ends in exactly one of: [`Dropped`] on
/// a target that takes `T`, [`DropRefused`] over a target that does not, [`DroppedOnWorld`] over
/// nothing the interface draws, or nothing (released over a panel that is not a target, or
/// Escape). A source that is also a [`bevy_ui_widgets::Button`](bevy::ui_widgets::Button) is
/// activated by a click as before, and by a drag that ends on itself.
///
/// Removing the `DragPayload` makes the node a source no more: pressing it lifts nothing (an
/// inventory slot that was emptied). A node carries one payload type at a time.
///
/// Dragging is a shortcut and never the only way in (WCAG 2.2, 2.5.7): offer the same move as a
/// command, a [`crate::context_menu`] item for instance.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// #[derive(Clone)]
/// struct Piece(u32);
///
/// # fn setup(mut commands: Commands) {
/// commands.spawn((button("Wall"), DragPayload(Piece(7))));
/// commands
///     .spawn((panel(), DropTarget::<Piece>::new()))
///     .observe(|dropped: On<Dropped<Piece>>| {
///         info!("piece {} dropped at {}", dropped.payload.0, dropped.pointer);
///     });
/// # }
/// ```
#[derive(Component, Clone, Debug, PartialEq, Eq)]
#[component(on_add = payload_added::<T>, on_remove = payload_removed)]
#[require(DragSource)]
pub struct DragPayload<T: Clone + Send + Sync + 'static>(pub T);

/// Makes a node a place a `T` may be dropped. A node may carry several, one for each type it takes.
/// The type is the match: a [`DragPayload<A>`] over a node with only a `DropTarget<B>` is refused,
/// visibly, and so is one the target's own test ([`DropTarget::accepting`]) says no to.
#[derive(Component, Clone, Debug)]
#[component(on_add = target_added::<T>)]
#[require(DropZone)]
pub struct DropTarget<T: Clone + Send + Sync + 'static> {
    accepts: Option<fn(&T) -> bool>,
}

impl<T: Clone + Send + Sync + 'static> DropTarget<T> {
    /// A target that takes every `T`.
    pub const fn new() -> Self {
        Self { accepts: None }
    }

    /// A target that takes a `T` when `accepts` says so: a slot that holds one kind of item, a
    /// folder that is not the dragged one.
    pub const fn accepting(accepts: fn(&T) -> bool) -> Self {
        Self {
            accepts: Some(accepts),
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Default for DropTarget<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A `T` was dropped on a [`DropTarget<T>`] that takes it. An event on the target, so
/// `commands.spawn(..).observe(|on: On<Dropped<T>>| ..)` hears the drops on that node.
#[derive(EntityEvent, Clone, Debug, PartialEq)]
pub struct Dropped<T: Clone + Send + Sync + 'static> {
    /// The target.
    #[event_target]
    pub target: Entity,
    /// The source it was carried from.
    pub source: Entity,
    /// What was carried.
    pub payload: T,
    /// Where the pointer was let go, in logical window pixels (what `Camera::viewport_to_world`
    /// takes).
    pub pointer: Vec2,
}

/// A `T` was let go over a [`DropZone`](DropTarget) that does not take it: another type, or
/// [`DropTarget::accepting`] said no. An event on the target. Nothing moved.
#[derive(EntityEvent, Clone, Debug, PartialEq)]
pub struct DropRefused<T: Clone + Send + Sync + 'static> {
    /// The target that refused.
    #[event_target]
    pub target: Entity,
    /// The source.
    pub source: Entity,
    /// What was carried.
    pub payload: T,
    /// Where the pointer was let go, in logical window pixels.
    pub pointer: Vec2,
}

/// A `T` was let go over nothing the interface draws (the game world under a HUD): a host raycasts
/// from `pointer` to see what is there. Sent as a global event.
#[derive(Event, Clone, Debug, PartialEq)]
pub struct DroppedOnWorld<T: Clone + Send + Sync + 'static> {
    /// The source.
    pub source: Entity,
    /// What was carried.
    pub payload: T,
    /// Where the pointer was let go, in logical window pixels.
    pub pointer: Vec2,
}

/// On a source exactly while it is carried (the pointer has moved past [`DRAG_LIFT_PX`] and not let
/// go): dim it, or hide it, or ignore it. The ghost and the mark are the crate's.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct Dragging;

/// The type-erased half of a source: how to send its typed events. Put on by [`DragPayload`].
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct DragSource {
    world_drop: Option<fn(&mut Commands, EntityRef, Vec2)>,
    refused: Option<fn(&mut Commands, Entity, EntityRef, Vec2)>,
}

/// One type a [`DropZone`] takes: whether a source's payload is that type and passes the test, and
/// how to deliver it.
#[derive(Clone, Copy)]
struct Handler {
    accepts: fn(EntityRef, EntityRef) -> Option<bool>,
    deliver: fn(&mut Commands, EntityRef, EntityRef, Vec2) -> bool,
}

/// The type-erased half of a target: every type it takes. Put on by [`DropTarget`].
#[derive(Component, Default, Clone)]
pub(crate) struct DropZone {
    handlers: Vec<Handler>,
}

impl DropZone {
    /// Whether the zone takes `source`'s payload: some handler's type matches and passes its test.
    fn takes(&self, zone: EntityRef, source: EntityRef) -> bool {
        self.handlers
            .iter()
            .any(|h| (h.accepts)(zone, source) == Some(true))
    }
}

fn payload_added<T: Clone + Send + Sync + 'static>(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
) {
    if let Some(mut source) = world.get_mut::<DragSource>(entity) {
        source.world_drop = Some(world_drop::<T>);
        source.refused = Some(refused::<T>);
    }
}

/// A node that loses its payload is no longer a source: pressing it lifts nothing (an emptied
/// inventory slot).
fn payload_removed(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world.commands().entity(entity).try_remove::<DragSource>();
}

fn target_added<T: Clone + Send + Sync + 'static>(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
) {
    if let Some(mut zone) = world.get_mut::<DropZone>(entity) {
        zone.handlers.push(Handler {
            accepts: accepts::<T>,
            deliver: deliver::<T>,
        });
    }
}

fn accepts<T: Clone + Send + Sync + 'static>(zone: EntityRef, source: EntityRef) -> Option<bool> {
    let payload = source.get::<DragPayload<T>>()?;
    let target = zone.get::<DropTarget<T>>()?;
    Some(target.accepts.is_none_or(|accepts| accepts(&payload.0)))
}

fn deliver<T: Clone + Send + Sync + 'static>(
    commands: &mut Commands,
    zone: EntityRef,
    source: EntityRef,
    pointer: Vec2,
) -> bool {
    if accepts::<T>(zone, source) != Some(true) {
        return false;
    }
    let Some(payload) = source.get::<DragPayload<T>>() else {
        return false;
    };
    commands.trigger(Dropped {
        target: zone.id(),
        source: source.id(),
        payload: payload.0.clone(),
        pointer,
    });
    true
}

fn refused<T: Clone + Send + Sync + 'static>(
    commands: &mut Commands,
    target: Entity,
    source: EntityRef,
    pointer: Vec2,
) {
    if let Some(payload) = source.get::<DragPayload<T>>() {
        commands.trigger(DropRefused {
            target,
            source: source.id(),
            payload: payload.0.clone(),
            pointer,
        });
    }
}

fn world_drop<T: Clone + Send + Sync + 'static>(
    commands: &mut Commands,
    source: EntityRef,
    pointer: Vec2,
) {
    if let Some(payload) = source.get::<DragPayload<T>>() {
        commands.trigger(DroppedOnWorld {
            source: source.id(),
            payload: payload.0.clone(),
            pointer,
        });
    }
}

/// The one drag in progress, if any: a resource, because one pointer carries one thing.
/// [`GooeyNavPlugin`](crate::GooeyNavPlugin) adds it; read it to know what the pointer holds.
#[derive(Resource, Clone, Debug, Default)]
pub struct DragState {
    drag: Option<Drag>,
}

/// A press on a source, from the press to the let go.
#[derive(Clone, Debug)]
struct Drag {
    source: Entity,
    pointer_id: PointerId,
    /// The pointer, in logical window pixels.
    pointer: Vec2,
    /// Whether the pointer has moved far enough for this to be a drag.
    lifted: bool,
    /// The ghost's words.
    label: String,
    /// The target under the pointer, and whether it refuses the payload.
    over: Option<Entity>,
    refused: bool,
    /// Escape or East was pressed this frame: [`track_drag`] puts it down once the frame's Back
    /// has been left alone.
    cancelled: bool,
    /// Lifted by [`carry_by_input`], not by the pointer: the target under the mark is the one
    /// the arrows chose, not the one under the pointer.
    keyboard: bool,
}

impl DragState {
    /// The source being carried, once it is lifted.
    pub fn source(&self) -> Option<Entity> {
        self.drag.as_ref().filter(|d| d.lifted).map(|d| d.source)
    }

    /// The [`DropTarget`] node under the pointer while a drag is lifted.
    pub fn over(&self) -> Option<Entity> {
        self.drag.as_ref().filter(|d| d.lifted).and_then(|d| d.over)
    }

    /// Whether the node under the pointer refuses what is carried.
    pub fn refused(&self) -> bool {
        self.drag.as_ref().is_some_and(|d| d.lifted && d.refused)
    }

    /// Whether a drag is lifted.
    pub fn is_dragging(&self) -> bool {
        self.drag.as_ref().is_some_and(|d| d.lifted)
    }

    /// Whether the carried thing was lifted from the keyboard or a pad (see [`DragSettings`]):
    /// the crate's own input systems stand still then, so the arrow that steers the mark never
    /// also moves focus.
    pub fn carried_by_key(&self) -> bool {
        self.drag.as_ref().is_some_and(|d| d.lifted && d.keyboard)
    }
}

/// A run condition: whether something is being dragged.
pub fn is_dragging(state: Option<Res<DragState>>) -> bool {
    state.is_some_and(|s| s.is_dragging())
}

/// A press on a source that moves picks nothing up yet: it is remembered, and lifted at
/// [`DRAG_LIFT_PX`]. A global observer.
pub(crate) fn drag_on_start(
    mut start: On<PointerDragStart>,
    sources: Query<(), With<DragSource>>,
    mut state: ResMut<DragState>,
) {
    if !sources.contains(start.entity) || start.button != PointerButton::Primary {
        return;
    }
    start.propagate(false);
    if state.drag.is_none() {
        state.drag = Some(Drag {
            source: start.entity,
            pointer_id: start.pointer.id,
            pointer: start.pointer.position,
            lifted: false,
            label: String::new(),
            over: None,
            refused: false,
            cancelled: false,
            keyboard: false,
        });
    }
}

/// The pointer follows; far enough, the source is lifted. A global observer.
pub(crate) fn drag_on_drag(
    mut drag: On<PointerDrag>,
    mut state: ResMut<DragState>,
    labels: Query<&DragLabel>,
    texts: Query<&Text>,
    kids: Query<&Children>,
    mut commands: Commands,
) {
    let Some(held) = state.drag.as_mut().filter(|d| d.source == drag.entity) else {
        return;
    };
    drag.propagate(false);
    held.pointer = drag.pointer.position;
    if !held.lifted && drag.distance.length() >= DRAG_LIFT_PX {
        held.lifted = true;
        held.label = labels
            .get(held.source)
            .map(|label| label.0.clone())
            .ok()
            .or_else(|| first_text(held.source, &texts, &kids))
            .unwrap_or_default();
        commands.entity(held.source).insert(Dragging);
    }
}

/// The words of the first `Text` at or under `entity`, depth first.
fn first_text(entity: Entity, texts: &Query<&Text>, kids: &Query<&Children>) -> Option<String> {
    if let Ok(text) = texts.get(entity) {
        return Some(text.0.clone());
    }
    kids.get(entity)
        .ok()?
        .iter()
        .find_map(|kid| first_text(kid, texts, kids))
}

/// The entity the pointer is over, front first.
fn front_hit(hover: &HoverMap, pointer: PointerId) -> Option<Entity> {
    hover
        .get(&pointer)?
        .iter()
        .min_by(|a, b| {
            a.1.depth
                .partial_cmp(&b.1.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(entity, _)| *entity)
}

/// The nearest [`DropZone`] at or above `entity`.
fn zone_of(entity: Entity, zones: &Query<&DropZone>, parents: &Query<&ChildOf>) -> Option<Entity> {
    std::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .find(|&e| zones.contains(e))
}

/// Which target is under the pointer while a drag is lifted, and whether it refuses what is
/// carried, so the mark and the ghost say so before the pointer is let go. Runs in
/// [`crate::GooeySystems::Navigate`]; costs nothing while nothing is dragged.
pub(crate) fn track_drag(
    mut state: ResMut<DragState>,
    hover: Option<Res<HoverMap>>,
    zones: Query<&DropZone>,
    parents: Query<&ChildOf>,
    entities: Query<EntityRef, Without<IsResource>>,
    mut commands: Commands,
) {
    if state.drag.as_ref().is_some_and(|d| d.cancelled)
        && let Some(held) = state.drag.take()
    {
        commands.entity(held.source).remove::<Dragging>();
        return;
    }
    let Some(held) = state.drag.as_mut() else {
        return;
    };
    if held.keyboard {
        return;
    }
    if !held.lifted {
        return;
    }
    let over = hover
        .as_ref()
        .and_then(|hover| front_hit(hover, held.pointer_id))
        .and_then(|front| zone_of(front, &zones, &parents));
    let refused = over.is_some_and(|zone| {
        let (Ok(zone_ref), Ok(source), Ok(handlers)) = (
            entities.get(zone),
            entities.get(held.source),
            zones.get(zone),
        ) else {
            return true;
        };
        !handlers.takes(zone_ref, source)
    });
    if held.over != over {
        held.over = over;
    }
    if held.refused != refused {
        held.refused = refused;
    }
}

/// Let go: the drop. One of [`Dropped`], [`DropRefused`], [`DroppedOnWorld`], or nothing (over an
/// interface node that takes no drops). A global observer.
#[allow(clippy::too_many_arguments)]
pub(crate) fn drag_on_end(
    mut end: On<PointerDragEnd>,
    mut state: ResMut<DragState>,
    hover: Option<Res<HoverMap>>,
    zones: Query<&DropZone>,
    parents: Query<&ChildOf>,
    nodes: Query<(), With<Node>>,
    entities: Query<EntityRef, Without<IsResource>>,
    mut commands: Commands,
) {
    if state.drag.as_ref().is_none_or(|d| d.source != end.entity) {
        return;
    }
    end.propagate(false);
    let Some(held) = state.drag.take() else {
        return;
    };
    commands.entity(held.source).remove::<Dragging>();
    if !held.lifted {
        return;
    }
    let pointer = end.pointer.position;
    let Ok(source) = entities.get(held.source) else {
        return;
    };
    let Some(drag_source) = source.get::<DragSource>().copied() else {
        return;
    };
    let front = hover
        .as_ref()
        .and_then(|hover| front_hit(hover, held.pointer_id));
    match front {
        Some(front) => {
            if let Some(zone) = zone_of(front, &zones, &parents)
                && let (Ok(zone_ref), Ok(handlers)) = (entities.get(zone), zones.get(zone))
            {
                release_on(
                    &mut commands,
                    zone,
                    zone_ref,
                    handlers,
                    source,
                    drag_source,
                    pointer,
                );
            } else if !nodes.contains(front)
                && let Some(world_drop) = drag_source.world_drop
            {
                // Something the interface does not draw (a sprite, a mesh): the game world.
                world_drop(&mut commands, source, pointer);
            }
        }
        None => {
            if let Some(world_drop) = drag_source.world_drop {
                world_drop(&mut commands, source, pointer);
            }
        }
    }
}

/// Lets go of `source` over `zone`: one [`Dropped`] if a type the zone takes matches, otherwise
/// one [`DropRefused`].
fn release_on(
    commands: &mut Commands,
    zone: Entity,
    zone_ref: EntityRef,
    handlers: &DropZone,
    source: EntityRef,
    drag_source: DragSource,
    pointer: Vec2,
) {
    let delivered = handlers
        .handlers
        .iter()
        .any(|h| (h.deliver)(commands, zone_ref, source, pointer));
    if !delivered && let Some(refused) = drag_source.refused {
        refused(commands, zone, source, pointer);
    }
}

/// A cancelled pointer (the touch was taken over) puts the thing down. A global observer.
pub(crate) fn drag_on_pointer_cancel(
    cancel: On<PointerCancel>,
    mut state: ResMut<DragState>,
    mut commands: Commands,
) {
    if let Some(held) = state.drag.as_ref()
        && held.source == cancel.entity
    {
        commands.entity(held.source).remove::<Dragging>();
        state.drag = None;
    }
}

/// Escape or a pad's East button puts a carried thing down where it was: nothing is sent, and the
/// press is no Back ([`crate::cancel_by_input`] does not run while a drag is lifted; the drag
/// itself is put down by `track_drag` after it, in the same frame). Runs in
/// [`crate::GooeySystems::Navigate`], before `cancel_by_input`.
pub fn cancel_drags(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut state: ResMut<DragState>,
) {
    if !state.is_dragging() {
        return;
    }
    if (keys.just_pressed(KeyCode::Escape)
        || gamepads.iter().any(|g| g.just_pressed(GamepadButton::East)))
        && let Some(held) = state.drag.as_mut()
    {
        held.cancelled = true;
    }
}

/// A node's rect in the interface's own logical pixels, the space `Node` positions are in.
fn rect_of(node: &ComputedNode, at: &UiGlobalTransform) -> Rect {
    let inverse = node.inverse_scale_factor();
    Rect::from_center_size(at.translation * inverse, node.size() * inverse)
}

fn rect_array(rect: Rect) -> [f32; 4] {
    [rect.min.x, rect.min.y, rect.max.x, rect.max.y]
}

/// Carries a payload without a pointer (see [`DragSettings`]): the pick-up key on a focused
/// source lifts it, the arrows or the D-pad move the mark to the nearest target that takes the
/// payload in that direction, PageUp, PageDown, LB and RB step through the targets in reading
/// order, and the pick-up key again (a pad's South too) drops it on the marked one. With nothing
/// marked a drop does nothing; Escape or East ([`cancel_drags`]) puts the payload down.
///
/// Runs in [`crate::GooeySystems::Navigate`] before [`cancel_drags`]; while a payload is carried
/// by it [`crate::not_capturing`] is false, so the key that steers never also moves focus.
#[allow(clippy::too_many_arguments)]
pub(crate) fn carry_by_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    settings: Res<DragSettings>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    mut state: ResMut<DragState>,
    ui_scale: Option<Res<UiScale>>,
    parents: Query<&ChildOf>,
    words: (Query<&DragLabel>, Query<&Text>, Query<&Children>),
    zones: Query<
        (
            Entity,
            &DropZone,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
        ),
        Without<InteractionDisabled>,
    >,
    frames: Query<(&ComputedNode, &UiGlobalTransform)>,
    entities: Query<EntityRef, Without<IsResource>>,
    mut commands: Commands,
) {
    let pad = |button: GamepadButton| gamepads.iter().any(|g| g.just_pressed(button));
    let key = |code: KeyCode| keys.just_pressed(code);
    let pick = settings.pick_up.just_pressed(&keys) || pad(settings.pick_up_button);
    let scale = ui_scale.as_ref().map_or(1.0, |s| s.0).max(f32::EPSILON);

    let Some(held) = state.drag.as_ref() else {
        // Nothing carried: the pick-up key on a focused source lifts it.
        if !pick {
            return;
        }
        let (labels, texts, kids) = &words;
        let Some(source) = focus.get().and_then(|focused| {
            std::iter::once(focused)
                .chain(parents.iter_ancestors(focused))
                .find(|&e| {
                    entities.get(e).is_ok_and(|e| {
                        e.contains::<DragSource>() && !e.contains::<InteractionDisabled>()
                    })
                })
        }) else {
            return;
        };
        let label = labels
            .get(source)
            .map(|label| label.0.clone())
            .ok()
            .or_else(|| first_text(source, texts, kids))
            .unwrap_or_default();
        let pointer = frames
            .get(source)
            .map_or(Vec2::ZERO, |(node, at)| rect_of(node, at).center() * scale);
        state.drag = Some(Drag {
            source,
            pointer_id: PointerId::Mouse,
            pointer,
            lifted: true,
            label,
            over: None,
            refused: false,
            cancelled: false,
            keyboard: true,
        });
        commands.entity(source).insert(Dragging);
        show_focus(&mut shown);
        return;
    };
    if !held.keyboard || held.cancelled {
        return;
    }
    let source = held.source;
    let (Ok(source_ref), Ok((source_node, source_at))) = (entities.get(source), frames.get(source))
    else {
        // The source was despawned while it was carried.
        state.drag = None;
        return;
    };

    if pick || pad(GamepadButton::South) {
        let over = held.over;
        state.drag = None;
        commands.entity(source).remove::<Dragging>();
        let pointer = over
            .and_then(|over| frames.get(over).ok())
            .map_or(Vec2::ZERO, |(node, at)| rect_of(node, at).center() * scale);
        if let Some(zone) = over
            && let (Ok(zone_ref), Ok((_, handlers, ..))) = (entities.get(zone), zones.get(zone))
            && let Some(drag_source) = source_ref.get::<DragSource>().copied()
        {
            release_on(
                &mut commands,
                zone,
                zone_ref,
                handlers,
                source_ref,
                drag_source,
                pointer,
            );
        }
        return;
    }

    let toward = [
        (KeyCode::ArrowUp, GamepadButton::DPadUp, Toward::Up),
        (KeyCode::ArrowDown, GamepadButton::DPadDown, Toward::Down),
        (KeyCode::ArrowLeft, GamepadButton::DPadLeft, Toward::Left),
        (KeyCode::ArrowRight, GamepadButton::DPadRight, Toward::Right),
    ]
    .into_iter()
    .find(|&(k, b, _)| key(k) || pad(b))
    .map(|(_, _, toward)| toward);
    let step = if key(KeyCode::PageDown) || pad(GamepadButton::RightTrigger) {
        Some(true)
    } else if key(KeyCode::PageUp) || pad(GamepadButton::LeftTrigger) {
        Some(false)
    } else {
        None
    };
    if toward.is_none() && step.is_none() {
        return;
    }

    // Only the targets that take the payload and are on screen are offered.
    let mut candidates: Vec<(Entity, Rect)> = zones
        .iter()
        .filter(|&(entity, handlers, node, _, visible)| {
            entity != source
                && visible.get()
                && node.size().cmpgt(Vec2::ZERO).all()
                && entities
                    .get(entity)
                    .is_ok_and(|zone_ref| handlers.takes(zone_ref, source_ref))
        })
        .map(|(entity, _, node, at, _)| (entity, rect_of(node, at)))
        .collect();
    candidates.sort_by_key(|&(entity, _)| entity);
    let rects: Vec<[f32; 4]> = candidates.iter().map(|&(_, r)| rect_array(r)).collect();
    let current = state
        .drag
        .as_ref()
        .and_then(|d| d.over)
        .and_then(|over| candidates.iter().position(|&(e, _)| e == over));
    let chosen = match (toward, step) {
        (Some(toward), _) => {
            let from =
                current.map_or_else(|| rect_array(rect_of(source_node, source_at)), |i| rects[i]);
            target_toward(from, &rects, toward)
        }
        (None, Some(forward)) => target_step(&rects, current, forward),
        (None, None) => None,
    };
    if let Some(index) = chosen
        && let Some(held) = state.drag.as_mut()
    {
        let (entity, rect) = candidates[index];
        held.over = Some(entity);
        held.refused = false;
        held.pointer = rect.center() * scale;
    }
}

/// The ghost: the carried thing's words in a card beside the pointer.
#[derive(Component)]
pub(crate) struct GhostPart;

/// The mark: the frame on the target under the pointer.
#[derive(Component)]
pub(crate) struct MarkPart;

/// The ghost's parts and the mark's, as `update_drag_visuals` writes them.
type GhostParts = (
    Entity,
    &'static mut Node,
    &'static mut Role,
    &'static mut Text,
);
type MarkParts = (Entity, &'static mut Node, &'static mut Role);
type IsGhost = (With<GhostPart>, Without<MarkPart>);
type IsMark = (With<MarkPart>, Without<GhostPart>);
type Laid = (Without<GhostPart>, Without<MarkPart>);

/// Draws a drag: spawns the ghost and the mark when a source is lifted, keeps them at the pointer
/// and on the target under it, in the accept or the refuse look, and despawns them when it ends.
/// Runs in [`crate::GooeySystems::Style`], before [`crate::apply_skin`]; costs nothing while
/// nothing is dragged.
pub(crate) fn update_drag_visuals(
    state: Option<Res<DragState>>,
    ui_scale: Option<Res<UiScale>>,
    mut ghosts: Query<GhostParts, IsGhost>,
    mut marks: Query<MarkParts, IsMark>,
    frames: Query<(&ComputedNode, &UiGlobalTransform), Laid>,
    mut commands: Commands,
) {
    let held = state
        .as_ref()
        .and_then(|state| state.drag.as_ref())
        .filter(|d| d.lifted);
    let Some(held) = held else {
        for (entity, ..) in &ghosts {
            commands.entity(entity).despawn();
        }
        for (entity, ..) in &marks {
            commands.entity(entity).despawn();
        }
        return;
    };
    let scale = ui_scale.as_ref().map_or(1.0, |s| s.0).max(f32::EPSILON);
    let at = held.pointer / scale + Vec2::splat(GHOST_OFFSET_PX);
    let (ghost_role, mark_role) = if held.refused {
        (Role::DragGhostRefused, Role::DropMarkRefused)
    } else {
        (Role::DragGhost, Role::DropMark)
    };
    match ghosts.iter_mut().next() {
        Some((_, mut node, mut role, mut text)) => {
            let mut next = node.clone();
            next.left = Val::Px(at.x);
            next.top = Val::Px(at.y);
            node.set_if_neq(next);
            role.set_if_neq(ghost_role);
            text.set_if_neq(Text::new(held.label.clone()));
        }
        None => {
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(at.x),
                    top: Val::Px(at.y),
                    ..default()
                },
                Text::new(held.label.clone()),
                ghost_role,
                Pickable::IGNORE,
                GlobalZIndex(DRAG_Z),
                GhostPart,
            ));
        }
    }
    let rect = held.over.and_then(|over| {
        let (node, at) = frames.get(over).ok()?;
        Some(rect_of(node, at))
    });
    match marks.iter_mut().next() {
        Some((_, mut node, mut role)) => {
            let mut next = node.clone();
            match rect {
                Some(rect) => {
                    next.display = Display::Flex;
                    next.left = Val::Px(rect.min.x);
                    next.top = Val::Px(rect.min.y);
                    next.width = Val::Px(rect.width());
                    next.height = Val::Px(rect.height());
                }
                None => next.display = Display::None,
            }
            node.set_if_neq(next);
            role.set_if_neq(mark_role);
        }
        None => {
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    ..default()
                },
                mark_role,
                Pickable::IGNORE,
                GlobalZIndex(DRAG_Z - 1),
                MarkPart,
            ));
        }
    }
}
