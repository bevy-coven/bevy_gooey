//! The choice: what keeps a [`crate::choice`] row's state and moves it. Its look is
//! [`crate::apply_skin`]'s, like every widget's.

use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::FocusedInput;
use bevy::picking::events::{PointerClick, PointerPress};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ValueChange};

use crate::model::{choice_next, choice_step};
use crate::select::Select;
use crate::skin::Role;

/// Which option a [`crate::choice`] has selected: an index into its options, in the order they were
/// given. It is the widget's state: read it to know the setting, set it to move the choice from
/// code (the screen follows in the same frame, and no [`ValueChange`] is sent for a change the game
/// made itself). A value past the last option is pulled back to the last.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn apply(choices: Query<&ChoiceValue, Changed<ChoiceValue>>) {
///     for selected in &choices {
///         info!("quality is option {}", selected.0);
///     }
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct ChoiceValue(pub usize);

/// How many options a [`crate::choice`] has: set when it is spawned and read by the widget to know
/// where its ends are. Leave it alone; a choice's options are fixed for its life.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct ChoiceCount(pub usize);

/// An arrow's target on a choice: which way a click on it steps, `-1` or `1`. Part of the arrow
/// `choice()` spawns; a game never builds one.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct StepBy(pub(crate) i32);

/// Where a choice moves to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Move {
    /// That many options, clamped to the ends.
    Step(i32),
    /// The first option.
    First,
    /// The last option.
    Last,
    /// The next option, wrapping from the last to the first.
    Next,
    /// That option (held to the last); type-select's move.
    To(usize),
}

/// A request to move a choice. A step past an end, or onto the option already selected, is no
/// change and says nothing.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub(crate) struct MoveChoice {
    #[event_target]
    pub(crate) entity: Entity,
    pub(crate) to: Move,
}

/// Turns a [`MoveChoice`] into the one [`ValueChange<usize>`] it makes, or none.
pub(crate) fn choice_on_move(
    event: On<MoveChoice>,
    choices: Query<(&ChoiceValue, &ChoiceCount), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok((value, count)) = choices.get(event.entity) else {
        return;
    };
    let len = count.0;
    let last = len.saturating_sub(1);
    let selected = value.0.min(last);
    let to = match event.to {
        Move::Step(delta) => choice_step(selected, len, delta),
        Move::First => 0,
        Move::Last => last,
        Move::Next => choice_next(selected, len),
        Move::To(index) => index.min(last),
    };
    if to != value.0 {
        commands.trigger(ValueChange {
            source: event.entity,
            value: to,
            is_final: true,
        });
    }
}

/// Keeps the state: a [`ValueChange<usize>`] on a choice becomes its [`ChoiceValue`]. The choice's
/// own observer, like Bevy's `checkbox_self_update`.
pub(crate) fn choice_self_update(
    change: On<ValueChange<usize>>,
    mut choices: Query<&mut ChoiceValue>,
) {
    if let Ok(mut value) = choices.get_mut(change.source) {
        value.set_if_neq(ChoiceValue(change.value));
    }
}

/// Enter, Space, the pad's South button and a click on the row (Bevy's `Button` makes each an
/// [`Activate`]) cycle to the next option, wrapping round.
pub(crate) fn choice_on_activate(
    event: On<Activate>,
    selects: Query<&Select>,
    mut commands: Commands,
) {
    // A select with a list opens it instead (`select_on_activate`).
    if selects.get(event.entity).is_ok_and(Select::filters) {
        return;
    }
    commands.trigger(MoveChoice {
        entity: event.entity,
        to: Move::Next,
    });
}

/// Left and Right step a focused choice, Home and End jump to its ends. A held key repeats, like a
/// slider's. Up, Down and Tab are not handled here: they move focus on.
pub(crate) fn choice_on_key_input(
    mut input: On<FocusedInput<KeyboardInput>>,
    choices: Query<(), (With<ChoiceCount>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if !choices.contains(input.focused_entity) || input.input.state != ButtonState::Pressed {
        return;
    }
    let to = match input.input.key_code {
        KeyCode::ArrowLeft => Move::Step(-1),
        KeyCode::ArrowRight => Move::Step(1),
        KeyCode::Home => Move::First,
        KeyCode::End => Move::Last,
        _ => return,
    };
    input.propagate(false);
    commands.trigger(MoveChoice {
        entity: input.focused_entity,
        to,
    });
}

/// A press on an arrow is the arrow's own: it stops here, so the row is not pressed and a release
/// over the arrow never reads as a click on the row. Bevy's `AcquireFocus` is triggered from the
/// press's original target, so the row still takes focus.
pub(crate) fn arrow_on_press(mut press: On<PointerPress>) {
    press.propagate(false);
}

/// A click on an arrow steps its choice one option that way, once, on release. On an arrow that
/// cannot step (the first option's back, the last option's forward) it does nothing, and it never
/// reaches the row either way.
pub(crate) fn arrow_on_click(
    mut click: On<PointerClick>,
    arrows: Query<&StepBy>,
    parents: Query<&ChildOf>,
    choices: Query<(), (With<ChoiceCount>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    let Ok(step) = arrows.get(click.entity) else {
        return;
    };
    click.propagate(false);
    if let Some(choice) = parents
        .iter_ancestors(click.entity)
        .find(|&ancestor| choices.contains(ancestor))
    {
        commands.trigger(MoveChoice {
            entity: choice,
            to: Move::Step(step.0),
        });
    }
}

/// Shows each changed choice's selected option and no other, and pulls a value past the last
/// option back to it. Runs in [`crate::GooeySystems::Style`], before [`crate::apply_skin`]: the
/// same frame the value changes, from a key, the pad, a click or the game.
///
/// Every option is laid out in the one grid cell, so the cell is as wide as the widest and
/// stepping never moves the arrows; the ones not selected are hidden, which keeps their room. A
/// choice whose value did not change costs nothing.
pub fn update_choices(
    mut choices: Query<(Entity, &mut ChoiceValue, &ChoiceCount), Changed<ChoiceValue>>,
    children: Query<&Children>,
    mut parts: Query<(&Role, &mut Visibility)>,
) {
    for (choice, mut value, count) in &mut choices {
        let last = count.0.saturating_sub(1);
        if value.0 > last {
            value.0 = last;
        }
        let options: Vec<Entity> = children
            .iter_descendants(choice)
            .filter(|&part| {
                parts
                    .get(part)
                    .is_ok_and(|(role, _)| *role == Role::ChoiceOption)
            })
            .collect();
        for (index, option) in options.into_iter().enumerate() {
            let Ok((_, mut visibility)) = parts.get_mut(option) else {
                continue;
            };
            let shown = if index == value.0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            visibility.set_if_neq(shown);
        }
    }
}
