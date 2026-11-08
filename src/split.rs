//! The split: what keeps a [`crate::split`]'s two panes in proportion, and what moves its divider
//! by pointer, key and pad. Its look is [`crate::apply_skin`]'s, like every widget's.

use bevy::input::ButtonState;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::{FocusedInput, InputFocus, InputFocusVisible};
use bevy::picking::events::{
    PointerCancel, PointerClick, PointerDrag, PointerDragEnd, PointerDragStart, PointerPress,
    PointerRelease,
};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy::ui_widgets::ValueChange;

use crate::clicks::DoublePress;
use crate::model::{
    PAD_REPEAT_DELAY, PAD_REPEAT_RATE, SPLIT_KEY_STEP_PX, repeat_steps, split_clamp, split_drag,
    split_mins,
};
use crate::nav::show_focus;
use crate::size_motion::SplitGlide;
use crate::skin::Role;
use crate::split_peek::SplitPeek;

/// How a [`crate::split`] lays its two panes out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default, Debug, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SplitAxis {
    /// Side by side: the first pane on the left, the second on the right, a vertical divider
    /// between them that Left and Right move.
    #[default]
    Horizontal,
    /// One above the other: the first pane on top, a horizontal divider that Up and Down move.
    Vertical,
}

/// A [`crate::split`]'s root: which way it lays its panes out. Part of what `split()` spawns; it
/// requires [`SplitFraction`] and [`SplitMin`], so a game sets either in the same tuple as the
/// split and its own value wins.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[require(SplitFraction, SplitMin, crate::SplitPeek)]
pub struct Split {
    axis: SplitAxis,
}

impl Split {
    pub(crate) fn new(axis: SplitAxis) -> Self {
        Self { axis }
    }

    /// The way the panes are laid out.
    pub fn axis(&self) -> SplitAxis {
        self.axis
    }
}

/// How much of a [`crate::split`] the first pane has, from 0 to 1: its share of the split's length
/// on its axis less the divider's. It is the widget's state: read it to know the layout, set it to
/// move the divider from code (the panes follow in the same frame, and no [`ValueChange`] is sent
/// for a change the game made itself) or save it, as plain data, with the player's other settings.
///
/// It is a preference: the player's drag and the keys keep it inside the panes' [`SplitMin`], and a
/// window too small for them lays the panes out as small as they must be and leaves this alone.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn remember(splits: Query<&SplitFraction, Changed<SplitFraction>>) {
///     for fraction in &splits {
///         info!("the sidebar takes {:.0}%", fraction.0 * 100.0);
///     }
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplitFraction(pub f32);

impl Default for SplitFraction {
    fn default() -> Self {
        Self(0.5)
    }
}

/// The smallest each pane of a [`crate::split`] may be, in logical pixels: the divider never takes
/// either below it, and a window too small for both shrinks the second pane first. Zero (the
/// default) lets a pane collapse to nothing.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     split(SplitAxis::Horizontal, panel(), panel()),
///     SplitFraction(0.3),
///     SplitMin::new(180.0, 320.0),
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplitMin {
    /// The first pane's least size along the axis.
    pub first: f32,
    /// The second pane's least size along the axis.
    pub second: f32,
}

impl SplitMin {
    /// Minimums of `first` and `second` logical pixels.
    pub fn new(first: f32, second: f32) -> Self {
        Self { first, second }
    }
}

/// A [`crate::split`]'s divider, the part a pointer grabs and focus rests on. Part of what
/// `split()` spawns. Give it `InteractionDisabled` to fix the layout: it still takes focus, and
/// moves nothing.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct SplitDivider {
    pub(crate) axis: SplitAxis,
}

impl SplitDivider {
    pub(crate) fn new(axis: SplitAxis) -> Self {
        Self { axis }
    }

    /// The axis of the split this divider is in.
    pub fn axis(&self) -> SplitAxis {
        self.axis
    }
}

/// Where a divider was when a drag began; its presence says a drag is going.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct SplitGrab(f32);

/// On a split whose first pane is collapsed: the fraction a restore gives back (Enter, South, a
/// double press, or docking a peek). A game reads it to know a split is collapsed and how wide it
/// will be restored; the crate puts it on and takes it off.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SplitSaved(pub(crate) f32);

impl SplitSaved {
    /// The fraction a restore gives back.
    pub fn fraction(&self) -> f32 {
        self.0
    }
}

/// A request to collapse a split's first pane to its minimum, or restore it, from its divider:
/// Enter, the pad's South button and a double click.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub(crate) struct ToggleSplit {
    #[event_target]
    pub(crate) entity: Entity,
}

/// What a divider's handlers need to know about its split.
struct Reading {
    root: Entity,
    axis: SplitAxis,
    fraction: f32,
    min: SplitMin,
    /// The room the panes share, in logical pixels: the split's length on its axis, less the
    /// divider's.
    extent: f32,
    disabled: bool,
    /// The fraction a peeking first pane docks at (the one a restore gives back), `Some` exactly
    /// while the pane floats over the second one.
    peeking: Option<f32>,
}

impl Reading {
    /// The fraction `delta` logical pixels from here, held to the minimums.
    fn moved(&self, delta: f32) -> f32 {
        split_drag(
            self.fraction,
            delta,
            self.extent,
            self.min.first,
            self.min.second,
        )
    }

    /// The fraction with the first pane as small, or as large, as it may be.
    fn end(&self, large: bool) -> f32 {
        let to = if large { 1.0 } else { 0.0 };
        split_clamp(to, self.extent, self.min.first, self.min.second)
    }
}

type Dividers = (
    &'static ChildOf,
    &'static SplitDivider,
    &'static ComputedNode,
    Has<InteractionDisabled>,
);

type Roots = (
    &'static SplitFraction,
    &'static SplitMin,
    &'static ComputedNode,
    &'static SplitPeek,
    Option<&'static SplitSaved>,
);

/// A system parameter that reads a divider's split (its fraction, minimums and room); it is in the
/// signature of [`split_by_pad`], so it is public. Nothing outside the crate has a use for it.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SplitGeometry<'w, 's> {
    dividers: Query<'w, 's, Dividers>,
    roots: Query<'w, 's, Roots>,
}

impl SplitGeometry<'_, '_> {
    /// The axis of the split `divider` is in, if it is a divider that takes input: not a disabled one.
    fn enabled(&self, divider: Entity) -> Option<SplitAxis> {
        let (_, divider, _, disabled) = self.dividers.get(divider).ok()?;
        (!disabled).then_some(divider.axis)
    }

    fn read(&self, divider: Entity) -> Option<Reading> {
        let Ok((parent, divider_axis, node, disabled)) = self.dividers.get(divider) else {
            debug!(
                "split divider {divider} is not read: it lacks a `ChildOf`, `SplitDivider` or \
                 `ComputedNode` (a node that has not been laid out has no `ComputedNode`)"
            );
            return None;
        };
        let Ok((fraction, min, root_node, peek, saved)) = self.roots.get(parent.parent()) else {
            debug!(
                "split divider {divider} is not read: its parent {} lacks a `SplitFraction`, \
                 `SplitMin` or `ComputedNode`",
                parent.parent()
            );
            return None;
        };
        let along = |size: Vec2, axis: SplitAxis| match axis {
            SplitAxis::Horizontal => size.x,
            SplitAxis::Vertical => size.y,
        };
        let axis = divider_axis.axis;
        let whole = along(root_node.size(), axis) * root_node.inverse_scale_factor();
        let bar = along(node.size(), axis) * node.inverse_scale_factor();
        let extent = (whole - bar).max(0.0);
        Some(Reading {
            root: parent.parent(),
            axis,
            fraction: fraction.0,
            min: *min,
            extent,
            disabled,
            peeking: peek
                .is_open()
                .then(|| saved.map(|s| split_clamp(s.0, extent, min.first, min.second)))
                .flatten(),
        })
    }
}

/// Reports a move of a split's divider: one [`ValueChange<f32>`] on the split, unless it is no
/// change (a step into an end says nothing).
fn report(commands: &mut Commands, reading: &Reading, to: f32, is_final: bool) {
    if to != reading.fraction {
        commands.trigger(ValueChange {
            source: reading.root,
            value: to,
            is_final,
        });
    }
}

/// Keeps the state: a [`ValueChange<f32>`] on a split becomes its [`SplitFraction`]. The split's
/// own observer.
pub(crate) fn split_self_update(
    change: On<ValueChange<f32>>,
    mut splits: Query<&mut SplitFraction, With<Split>>,
) {
    if let Ok(mut fraction) = splits.get_mut(change.source) {
        fraction.set_if_neq(SplitFraction(change.value));
    }
}

/// A press on the divider is its own: it stops here, and marks it pressed.
pub(crate) fn divider_on_press(
    mut press: On<PointerPress>,
    dividers: Query<(Has<InteractionDisabled>, Has<Pressed>), With<SplitDivider>>,
    mut commands: Commands,
) {
    press.propagate(false);
    if press.button == PointerButton::Primary
        && dividers
            .get(press.entity)
            .is_ok_and(|(disabled, pressed)| !disabled && !pressed)
    {
        commands.entity(press.entity).insert(Pressed);
    }
}

/// Releasing the pointer on the divider lets it go.
pub(crate) fn divider_on_release(mut release: On<PointerRelease>, mut commands: Commands) {
    release.propagate(false);
    commands.entity(release.entity).remove::<Pressed>();
}

/// A drag begins: the divider remembers where it started.
pub(crate) fn divider_on_drag_start(
    mut start: On<PointerDragStart>,
    geometry: SplitGeometry,
    glides: Query<&SplitGlide>,
    mut commands: Commands,
) {
    start.propagate(false);
    if start.button != PointerButton::Primary {
        debug!(
            "split divider {} is not dragged: the drag is by {:?}, not the primary button",
            start.entity, start.button
        );
        return;
    }
    if let Some(reading) = geometry.read(start.entity) {
        if reading.disabled {
            debug!(
                "split divider {} is not dragged: it is `InteractionDisabled`",
                start.entity
            );
            return;
        }
        // A drag that starts while the pane peeks docks it where it floats and drags from there.
        if let Some(docked) = reading.peeking {
            commands.trigger(ToggleSplit {
                entity: start.entity,
            });
            commands
                .entity(start.entity)
                .insert((Pressed, SplitGrab(docked)));
            return;
        }
        // A drag that starts during a collapse's slide takes the divider from where it is drawn.
        let from = glides
            .get(reading.root)
            .map_or(reading.fraction, SplitGlide::shown);
        if from != reading.fraction {
            commands.entity(reading.root).remove::<SplitGlide>();
            report(&mut commands, &reading, from, false);
        }
        commands
            .entity(start.entity)
            .insert((Pressed, SplitGrab(from)));
    }
}

/// The divider follows the pointer: the fraction is where the drag began plus the distance along the
/// axis over the room, so `d` pixels of drag change it by `d` over the room, held to the minimums.
pub(crate) fn divider_on_drag(
    mut drag: On<PointerDrag>,
    ui_scale: Res<UiScale>,
    grabs: Query<&SplitGrab>,
    geometry: SplitGeometry,
    mut commands: Commands,
) {
    drag.propagate(false);
    let Ok(grab) = grabs.get(drag.entity) else {
        debug!(
            "split divider {} ignores a drag: no drag began on it (no `SplitGrab`)",
            drag.entity
        );
        return;
    };
    let Some(reading) = geometry.read(drag.entity) else {
        return;
    };
    let along = match reading.axis {
        SplitAxis::Horizontal => drag.distance.x,
        SplitAxis::Vertical => drag.distance.y,
    } / ui_scale.0;
    let to = split_drag(
        grab.0,
        along,
        reading.extent,
        reading.min.first,
        reading.min.second,
    );
    debug!(
        "split divider {} dragged {along} layout px (pointer distance {}) of room {}: fraction {} to {to}",
        drag.entity, drag.distance, reading.extent, reading.fraction
    );
    report(&mut commands, &reading, to, false);
}

/// The drag ends: the last value is final.
pub(crate) fn divider_on_drag_end(
    mut end: On<PointerDragEnd>,
    grabs: Query<(), With<SplitGrab>>,
    geometry: SplitGeometry,
    mut commands: Commands,
) {
    end.propagate(false);
    commands.entity(end.entity).remove::<(Pressed, SplitGrab)>();
    if grabs.contains(end.entity)
        && let Some(reading) = geometry.read(end.entity)
    {
        commands.trigger(ValueChange {
            source: reading.root,
            value: reading.fraction,
            is_final: true,
        });
    }
}

/// A cancelled pointer lets the divider go where it is.
pub(crate) fn divider_on_cancel(mut cancel: On<PointerCancel>, mut commands: Commands) {
    cancel.propagate(false);
    commands
        .entity(cancel.entity)
        .remove::<(Pressed, SplitGrab)>();
}

/// A click on the divider stops there: the split under it is not clicked.
pub(crate) fn divider_on_click(mut click: On<PointerClick>) {
    click.propagate(false);
}

/// A double press collapses the first pane to its minimum, or restores it: the pointer's way to
/// size a split without a drag (WCAG 2.5.7). A press that ends a drag is never half of one
/// ([`DoublePress`] pairs only clicks that did not travel past the slop),
/// although the pointer lets go over the divider it just moved and Bevy calls that a click.
pub(crate) fn divider_on_double_press(
    mut double: On<DoublePress>,
    dividers: Query<Has<InteractionDisabled>, With<SplitDivider>>,
    mut commands: Commands,
) {
    double.propagate(false);
    if dividers.get(double.entity).is_ok_and(|disabled| !disabled) {
        commands.trigger(ToggleSplit {
            entity: double.entity,
        });
    }
}

/// The arrows along the axis step a focused divider, held ones repeating; Home and End take the
/// first pane to its smallest and largest; Enter collapses it or restores it. The other arrows
/// are not handled here: they move focus on.
pub(crate) fn divider_on_key_input(
    mut input: On<FocusedInput<KeyboardInput>>,
    geometry: SplitGeometry,
    mut commands: Commands,
) {
    if input.input.state != ButtonState::Pressed {
        return;
    }
    let Some(reading) = geometry.read(input.focused_entity) else {
        return;
    };
    let step = |sign: f32| reading.moved(sign * SPLIT_KEY_STEP_PX);
    let key = input.input.key_code;
    // A step, Home or End while the pane peeks docks it first: the next press steps from there.
    let moves = matches!(
        (reading.axis, key),
        (
            SplitAxis::Horizontal,
            KeyCode::ArrowLeft | KeyCode::ArrowRight
        ) | (SplitAxis::Vertical, KeyCode::ArrowUp | KeyCode::ArrowDown)
            | (_, KeyCode::Home | KeyCode::End)
    );
    if moves && reading.peeking.is_some() {
        input.propagate(false);
        if !reading.disabled {
            commands.trigger(ToggleSplit {
                entity: input.focused_entity,
            });
        }
        return;
    }
    let to = match (reading.axis, key) {
        (SplitAxis::Horizontal, KeyCode::ArrowLeft) | (SplitAxis::Vertical, KeyCode::ArrowUp) => {
            step(-1.0)
        }
        (SplitAxis::Horizontal, KeyCode::ArrowRight)
        | (SplitAxis::Vertical, KeyCode::ArrowDown) => step(1.0),
        (_, KeyCode::Home) => reading.end(false),
        (_, KeyCode::End) => reading.end(true),
        (_, KeyCode::Enter) => {
            input.propagate(false);
            if !reading.disabled && !input.input.repeat {
                commands.trigger(ToggleSplit {
                    entity: input.focused_entity,
                });
            }
            return;
        }
        _ => return,
    };
    input.propagate(false);
    if !reading.disabled {
        report(&mut commands, &reading, to, true);
    }
}

/// Collapses a split's first pane to its minimum, remembering where it was, or gives that back.
pub(crate) fn toggle_split(
    event: On<ToggleSplit>,
    geometry: SplitGeometry,
    saved: Query<&SplitSaved>,
    motion: Res<crate::motion::MotionSettings>,
    glides: Query<&SplitGlide>,
    mut commands: Commands,
) {
    let Some(reading) = geometry.read(event.entity) else {
        return;
    };
    if reading.disabled || reading.extent <= 0.0 {
        return;
    }
    let smallest = reading.end(false);
    let at_smallest = (reading.fraction - smallest) * reading.extent < 0.5;
    let to = if at_smallest {
        let back = saved.get(reading.root).map_or(0.5, |s| s.0);
        commands.entity(reading.root).remove::<SplitSaved>();
        split_clamp(back, reading.extent, reading.min.first, reading.min.second)
    } else {
        commands
            .entity(reading.root)
            .insert(SplitSaved(reading.fraction));
        smallest
    };
    // The panes slide there (size motion): from where they are laid out now, a glide on its way
    // included. The fraction is the new one at once.
    if !motion.lands_travel() && to != reading.fraction && reading.peeking.is_none() {
        let from = glides
            .get(reading.root)
            .map_or(reading.fraction, SplitGlide::shown);
        commands
            .entity(reading.root)
            .insert(SplitGlide::new(from, to));
    }
    report(&mut commands, &reading, to, true);
}

/// The D-pad steps a focused divider along its axis, held to repeat like a slider's, and the pad's
/// South button collapses or restores its first pane. Shows focus. Runs in
/// [`crate::GooeySystems::Navigate`]. The keys are the divider's own observer's.
pub fn split_by_pad(
    time: Res<Time<Real>>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    gamepads: Query<&Gamepad>,
    geometry: SplitGeometry,
    mut held: Local<(i8, f32)>,
    mut commands: Commands,
) {
    let down = |button| gamepads.iter().any(|g| g.pressed(button));
    let Some((divider, axis)) = focus
        .get()
        .and_then(|f| geometry.enabled(f).map(|a| (f, a)))
    else {
        *held = (0, 0.0);
        return;
    };
    if gamepads
        .iter()
        .any(|g| g.just_pressed(GamepadButton::South))
    {
        show_focus(&mut shown);
        commands.trigger(ToggleSplit { entity: divider });
    }
    let (plus, minus, across) = match axis {
        SplitAxis::Horizontal => (
            GamepadButton::DPadRight,
            GamepadButton::DPadLeft,
            [GamepadButton::DPadUp, GamepadButton::DPadDown],
        ),
        SplitAxis::Vertical => (
            GamepadButton::DPadDown,
            GamepadButton::DPadUp,
            [GamepadButton::DPadLeft, GamepadButton::DPadRight],
        ),
    };
    let direction = down(plus) as i8 - down(minus) as i8;
    if direction == 0 || across.into_iter().any(down) {
        *held = (0, 0.0);
        return;
    }
    let steps = if held.0 == direction {
        let before = held.1;
        held.1 += time.delta_secs();
        repeat_steps(before, held.1, PAD_REPEAT_DELAY, PAD_REPEAT_RATE)
    } else {
        *held = (direction, 0.0);
        1
    };
    if steps == 0 {
        return;
    }
    show_focus(&mut shown);
    if let Some(reading) = geometry.read(divider) {
        if reading.peeking.is_some() {
            // Docks the peeking pane; the held direction steps it from there.
            commands.trigger(ToggleSplit { entity: divider });
            return;
        }
        let to = reading.moved(f32::from(direction) * steps as f32 * SPLIT_KEY_STEP_PX);
        report(&mut commands, &reading, to, true);
    }
}

/// What laying a split out reads: its axis, state, minimums, size, parts and any glide on its way.
type LaidOutSplit = (
    &'static Split,
    &'static SplitFraction,
    &'static SplitMin,
    &'static ComputedNode,
    &'static Children,
    Option<&'static SplitGlide>,
    &'static SplitPeek,
);

/// Lays each split's panes out from its [`SplitFraction`] and [`SplitMin`]: the first pane grows by
/// the fraction (held to the minimums) and the second by what is left, from a basis of nothing, so
/// they share the room the divider leaves in exactly that ratio, and each pane's own minimum is
/// the one [`split_mins`] leaves for the room it has now. A window that shrinks is laid out by
/// flexbox the frame it does and the minimums follow the next. Runs in
/// [`crate::GooeySystems::Style`], before [`crate::apply_skin`]; a split that did not change costs
/// a comparison and writes nothing.
pub fn update_splits(
    splits: Query<LaidOutSplit>,
    dividers: Query<&ComputedNode, With<SplitDivider>>,
    mut parts: Query<(&Role, &mut Node)>,
) {
    for (split, fraction, min, node, children, glide, peek) in &splits {
        // A collapse or a restore on its way lays the panes out where it has got to.
        let fraction = glide.map_or(fraction.0, SplitGlide::shown);
        let along = |size: Vec2| match split.axis {
            SplitAxis::Horizontal => size.x,
            SplitAxis::Vertical => size.y,
        };
        let bar = children
            .iter()
            .find_map(|c| dividers.get(c).ok())
            .map_or(0.0, |d| along(d.size()) * d.inverse_scale_factor());
        let extent = (along(node.size()) * node.inverse_scale_factor() - bar).max(0.0);
        let first = split_clamp(fraction, extent, min.first, min.second);
        // **A split that has not been laid out yet still says what its panes may not go under.**
        //
        // `split_mins` folds each minimum into the room that is there, so with no room it answers
        // `(0, 0)` — and this system runs in `Style`, which is *before* `ui_layout_system`, so the
        // frame a split is spawned is laid out with `flex_grow: 0` on the first pane and no minimum
        // on either. Measured by a caller (Dark Factory): entering a page showed one frame of the body
        // across the whole window, the sidebar 24 px wide, then the right layout. So an extent of
        // zero keeps the minimums as asked for; the first real extent clamps them from there.
        let (min_first, min_second) = if extent > 0.0 {
            split_mins(extent, min.first, min.second)
        } else {
            (min.first.max(0.0), min.second.max(0.0))
        };
        let panes: Vec<Entity> = children
            .iter()
            .filter(|&c| parts.get(c).is_ok_and(|(role, _)| *role == Role::SplitPane))
            .collect();
        let peeking = peek.is_open();
        for (index, (pane, (grow, least))) in panes
            .into_iter()
            .zip([(first, min_first), (1.0 - first, min_second)])
            .enumerate()
        {
            let Ok((_, mut node)) = parts.get_mut(pane) else {
                continue;
            };
            let least = Val::Px(least);
            let (min_width, min_height) = match split.axis {
                SplitAxis::Horizontal => (least, Val::Px(0.0)),
                SplitAxis::Vertical => (Val::Px(0.0), least),
            };
            let mut next = node.clone();
            next.flex_grow = grow;
            next.min_width = min_width;
            next.min_height = min_height;
            if index == 1 {
                // The floating pane and the divider are out of the flow: the second pane takes the
                // rest alone (a grow under 1 would leave some of it empty), after the room the
                // collapsed pane and the divider had.
                let kept = Val::Px(peek.collapsed() + bar);
                next.margin = match (peeking, split.axis) {
                    (false, _) => UiRect::ZERO,
                    (true, SplitAxis::Horizontal) => UiRect::left(kept),
                    (true, SplitAxis::Vertical) => UiRect::top(kept),
                };
                if peeking {
                    next.flex_grow = 1.0;
                }
            }
            if index == 0 {
                // A peeking first pane floats at the split's start edge, as wide as it will be
                // restored; otherwise it is in the flow.
                let size = Val::Px(peek.width());
                (
                    next.position_type,
                    next.left,
                    next.top,
                    next.right,
                    next.bottom,
                ) = if peeking {
                    match split.axis {
                        SplitAxis::Horizontal => (
                            PositionType::Absolute,
                            Val::Px(0.0),
                            Val::Px(0.0),
                            Val::Auto,
                            Val::Px(0.0),
                        ),
                        SplitAxis::Vertical => (
                            PositionType::Absolute,
                            Val::Px(0.0),
                            Val::Px(0.0),
                            Val::Px(0.0),
                            Val::Auto,
                        ),
                    }
                } else {
                    (
                        PositionType::Relative,
                        Val::Auto,
                        Val::Auto,
                        Val::Auto,
                        Val::Auto,
                    )
                };
                (next.width, next.height) = match (peeking, split.axis) {
                    (false, _) => (Val::Auto, Val::Auto),
                    (true, SplitAxis::Horizontal) => (size, Val::Auto),
                    (true, SplitAxis::Vertical) => (Val::Auto, size),
                };
            }
            node.set_if_neq(next);
        }
        // A peeking pane's divider stands on its far edge, above it, out of the flow.
        let at = Val::Px(peek.width());
        for child in children.iter() {
            let Ok((role, mut node)) = parts.get_mut(child) else {
                continue;
            };
            if *role != Role::SplitDivider {
                continue;
            }
            let mut next = node.clone();
            (
                next.position_type,
                next.left,
                next.top,
                next.right,
                next.bottom,
            ) = match (peeking, split.axis) {
                (false, _) => (
                    PositionType::Relative,
                    Val::Auto,
                    Val::Auto,
                    Val::Auto,
                    Val::Auto,
                ),
                (true, SplitAxis::Horizontal) => (
                    PositionType::Absolute,
                    at,
                    Val::Px(0.0),
                    Val::Auto,
                    Val::Px(0.0),
                ),
                (true, SplitAxis::Vertical) => (
                    PositionType::Absolute,
                    Val::Px(0.0),
                    at,
                    Val::Px(0.0),
                    Val::Auto,
                ),
            };
            node.set_if_neq(next);
        }
    }
}
