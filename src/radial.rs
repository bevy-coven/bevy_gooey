//! The radial menu: what keeps a [`crate::radial`]'s state, points it at an item from a stick, the
//! pointer, the arrows and the D-pad, and lights the item. Its look is [`crate::apply_skin`]'s.

use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::{FocusedInput, InputFocus, InputFocusVisible};
use bevy::picking::events::{PointerMove, PointerPress};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Selected};
use bevy::ui_widgets::{Activate, ValueChange};

use crate::model::{RADIAL_DEAD_ZONE, RADIAL_STICK_PUSH, radial_pick, radial_step};
use crate::nav::show_focus;
use crate::skin::Role;

/// Which item a [`crate::radial`] has highlighted: an index into its items, in the order they
/// were given (item 0 is at the top, the rest follow clockwise). It is the widget's state: read it
/// to know the highlight, set it to move the highlight from code (the screen follows in the same
/// frame, and no [`ValueChange`] is sent for a change the game made itself). A value past the last
/// item is pulled back to the last.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn follow(wheels: Query<&RadialValue, Changed<RadialValue>>) {
///     for lit in &wheels {
///         info!("the wheel points at item {}", lit.0);
///     }
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct RadialValue(pub usize);

/// How many items a [`crate::radial`] has: set when it is spawned and read by the widget to know
/// where its ring closes. Leave it alone; a radial's items are fixed for its life.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct RadialCount(pub usize);

/// One item of a [`crate::radial`], by its place on the ring. Part of what `radial()` spawns; a
/// game never builds one.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct RadialSlot(pub(crate) usize);

impl RadialSlot {
    /// The item's place on the ring, from 0 at the top, clockwise.
    pub fn index(&self) -> usize {
        self.0
    }
}

/// Points a radial at item `to`: one non-final [`ValueChange`], or nothing when it is already there.
fn point_at(commands: &mut Commands, radial: Entity, now: usize, to: usize) {
    if to != now {
        commands.trigger(ValueChange {
            source: radial,
            value: to,
            is_final: false,
        });
    }
}

/// Keeps the state: a [`ValueChange<usize>`] on a radial, final or not, becomes its
/// [`RadialValue`]. The radial's own observer, like [`crate::choice`]'s.
pub(crate) fn radial_self_update(
    change: On<ValueChange<usize>>,
    mut radials: Query<&mut RadialValue>,
) {
    if let Ok(mut value) = radials.get_mut(change.source) {
        value.set_if_neq(RadialValue(change.value));
    }
}

/// Enter, Space, the pad's South button and a click on the disc (Bevy's `Button` makes each an
/// [`Activate`]) choose the highlighted item: one final [`ValueChange`], even when the highlight
/// did not move. A game that lets the player hold a button to open the wheel triggers `Activate` on
/// the radial when it is let go.
pub(crate) fn radial_on_activate(
    event: On<Activate>,
    radials: Query<&RadialValue, (With<RadialCount>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok(value) = radials.get(event.entity) {
        commands.trigger(ValueChange {
            source: event.entity,
            value: value.0,
            is_final: true,
        });
    }
}

/// The arrows step a focused radial one item (Right and Down forward, Left and Up back, wrapping
/// round) and Home and End go to the ends. A held key repeats. Tab is not handled here: it moves
/// focus on.
pub(crate) fn radial_on_key_input(
    mut input: On<FocusedInput<KeyboardInput>>,
    radials: Query<(&RadialValue, &RadialCount), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok((value, count)) = radials.get(input.focused_entity) else {
        return;
    };
    if input.input.state != ButtonState::Pressed {
        return;
    }
    let to = match input.input.key_code {
        KeyCode::ArrowRight | KeyCode::ArrowDown => radial_step(value.0, count.0, 1),
        KeyCode::ArrowLeft | KeyCode::ArrowUp => radial_step(value.0, count.0, -1),
        KeyCode::Home => 0,
        KeyCode::End => count.0.saturating_sub(1),
        _ => return,
    };
    input.propagate(false);
    point_at(&mut commands, input.focused_entity, value.0, to);
}

/// The item a point on the disc faces, from the normalized place a pointer hit reports (`-0.5` to
/// `0.5` across the node, `y` down): none inside the dead circle in the middle.
fn faced(position: Option<Vec3>, count: usize) -> Option<usize> {
    let position = position?;
    let (x, y) = (position.x, -position.y);
    // The disc's radius is 0.5 in a hit's units.
    if x.hypot(y) < RADIAL_DEAD_ZONE * 0.5 {
        return None;
    }
    radial_pick(x, y, count)
}

/// The pointer moving over the disc points the radial at the item it faces. A pointer in the
/// middle holds the highlight, and one that rests moves nothing.
pub(crate) fn radial_on_pointer_move(
    moved: On<PointerMove>,
    radials: Query<(&RadialValue, &RadialCount), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok((value, count)) = radials.get(moved.entity) else {
        return;
    };
    if let Some(to) = faced(moved.hit.position, count.0) {
        point_at(&mut commands, moved.entity, value.0, to);
    }
}

/// A press (a touch has no move before it) points the radial at the item it faces, before the click
/// that follows chooses it.
pub(crate) fn radial_on_press(
    press: On<PointerPress>,
    radials: Query<(&RadialValue, &RadialCount), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok((value, count)) = radials.get(press.entity) else {
        return;
    };
    if let Some(to) = faced(press.hit.position, count.0) {
        point_at(&mut commands, press.entity, value.0, to);
    }
}

/// Points the focused, enabled [`crate::radial`] with a pad: a stick (the left or the right, of any
/// pad, whichever is pushed further) pushed past [`RADIAL_STICK_PUSH`] highlights the item it
/// points at ([`radial_pick`]), and one inside that dead zone leaves the highlight where it is; the
/// D-pad's Right and Down step forward and Left and Up back, wrapping round. Shows focus
/// (`InputFocusVisible`). Runs in [`crate::GooeySystems::Navigate`]. South chooses, through
/// [`crate::gamepad_select`]; the keys are the radial's own observer.
pub fn radial_by_pad(
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    gamepads: Query<&Gamepad>,
    radials: Query<(&RadialValue, &RadialCount), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Some(radial) = focus.get() else {
        return;
    };
    let Ok((value, count)) = radials.get(radial) else {
        return;
    };
    let pressed = |button| gamepads.iter().any(|g| g.just_pressed(button));
    let forward = pressed(GamepadButton::DPadRight) || pressed(GamepadButton::DPadDown);
    let back = pressed(GamepadButton::DPadLeft) || pressed(GamepadButton::DPadUp);
    let stick = gamepads
        .iter()
        .flat_map(|g| [g.left_stick(), g.right_stick()])
        .max_by(|a, b| a.length_squared().total_cmp(&b.length_squared()))
        .filter(|s| s.length() >= RADIAL_STICK_PUSH);
    let to = if forward != back {
        Some(radial_step(value.0, count.0, if forward { 1 } else { -1 }))
    } else {
        stick.and_then(|s| radial_pick(s.x, s.y, count.0))
    };
    if let Some(to) = to {
        show_focus(&mut shown);
        point_at(&mut commands, radial, value.0, to);
    }
}

/// Lights each changed radial's highlighted item (Bevy's `Selected`, the lit look of a tab) and no
/// other, names it in the middle, and pulls a value past the last item back to it. Runs in
/// [`crate::GooeySystems::Style`], before [`crate::apply_skin`]: the same frame the value changes,
/// from a stick, a key, the pointer or the game. A radial whose value did not change costs
/// nothing.
pub fn update_radials(
    mut radials: Query<(Entity, &mut RadialValue, &RadialCount), Changed<RadialValue>>,
    children: Query<&Children>,
    slots: Query<(&RadialSlot, Has<Selected>)>,
    roles: Query<&Role>,
    mut texts: Query<&mut Text>,
    mut commands: Commands,
) {
    for (radial, mut value, count) in &mut radials {
        let last = count.0.saturating_sub(1);
        if value.0 > last {
            value.0 = last;
        }
        let Ok(kids) = children.get(radial) else {
            continue;
        };
        let mut label = None;
        for &kid in kids {
            let Ok((slot, lit)) = slots.get(kid) else {
                continue;
            };
            let here = slot.0 == value.0;
            match (here, lit) {
                (true, false) => {
                    commands.entity(kid).insert(Selected);
                }
                (false, true) => {
                    commands.entity(kid).remove::<Selected>();
                }
                _ => {}
            }
            if here {
                label = children
                    .iter_descendants(kid)
                    .find_map(|part| texts.get(part).ok().map(|text| text.0.clone()));
            }
        }
        let centre = kids
            .iter()
            .find(|&kid| roles.get(kid) == Ok(&Role::RadialCentre));
        if let (Some(centre), Some(label)) = (centre, label)
            && let Ok(mut text) = texts.get_mut(centre)
            && text.0 != label
        {
            text.0 = label;
        }
    }
}
