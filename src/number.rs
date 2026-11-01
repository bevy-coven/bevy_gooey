//! The number field: a [`crate::text_input`] row whose text is a number the player may also scrub
//! with a drag and step with a key or the D-pad, and the vec field that holds several. The text
//! row does the typing; this keeps the number, applies what is typed, and sends the changes.

use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::{FocusedInput, InputFocus};
use bevy::picking::events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::ValueChange;

use crate::model::{NumberError, format_number, parse_number, scrub_value, step_value};
use crate::text::{TextEdited, TextHint, TextInputOf, TextInputValue};

/// What a [`crate::number_field`] holds: its number. It is the widget's state: read it to know the
/// number, set it to change the field from code (the field shows it in the same frame and no
/// [`ValueChange`] is sent for a change the game made itself). The player's own changes are one
/// `ValueChange<f32>` on the row each: `is_final` is false while a drag is scrubbing it and true
/// when the drag ends, on Enter, when focus leaves after an edit and on a key or D-pad step.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn on_health(health: Query<&NumberValue, Changed<NumberValue>>) {
///     for health in &health {
///         info!("health is {}", health.0);
///     }
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct NumberValue(pub f32);

/// How far one step moves a [`crate::number_field`]: a pixel of drag (Shift a tenth of it, Ctrl ten
/// times it), an Up or Down press (Shift ten) and a D-pad press. It also sets the digits the field
/// shows: a step of 0.1 shows one. Put it on the row, at spawn or later; on a [`crate::vec_field`]
/// it is every component's.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct NumberStep(pub f32);

impl Default for NumberStep {
    fn default() -> Self {
        Self(1.0)
    }
}

/// The least and the greatest a player can make a [`crate::number_field`]: a drag, a step and a
/// typed number are all held to it. A value the game sets is shown as it is. On a
/// [`crate::vec_field`] it is every component's.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct NumberRange {
    /// The least.
    pub min: f32,
    /// The greatest.
    pub max: f32,
}

impl Default for NumberRange {
    fn default() -> Self {
        Self {
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
        }
    }
}

/// The words a mixed number field shows, muted, in its empty field. Two hyphens, because the
/// em dash a mixed value is usually drawn with is not in every font and the host supplies the
/// font: a missing glyph would be a box.
pub const MIXED_HINT: &str = "--";

/// On a [`crate::number_field`] whose value is not one number: a multi-selection whose members
/// differ. The field is empty and shows a muted [`MIXED_HINT`]; [`NumberValue`] is the number a drag or a
/// step starts from. Any change the player makes (a scrub, a step, a typed number) takes the
/// marker off and is the one value for every member; Enter on the untouched field changes
/// nothing. The game puts it back when the selection still differs.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct MixedValue;

/// What a [`crate::vec_field`] holds: one number per component, in order. Like [`NumberValue`] it
/// is the state: a game's write shows at once and sends nothing, the player's change is a
/// [`VecChange`].
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct VecValue(pub Vec<f32>);

/// On a [`crate::vec_field`]: which components of a multi-selection differ (`true` shows the
/// [`MIXED_HINT`] of [`MixedValue`] in that component). A component the player changes is taken off.
/// A missing entry is `false`.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct VecMixed(pub Vec<bool>);

/// A change the player made to one component of a [`crate::vec_field`], sent on the vec row. A
/// game that edits a mixed multi-selection applies `value` to component `index` of every member
/// and leaves the others alone; `values` is the whole vector as it now stands.
#[derive(EntityEvent, Clone, Debug, PartialEq)]
pub struct VecChange {
    /// The vec row.
    #[event_target]
    pub entity: Entity,
    /// The component that changed.
    pub index: usize,
    /// Its new value.
    pub value: f32,
    /// Every component now.
    pub values: Vec<f32>,
    /// False while a drag is still scrubbing it, true once the player is done.
    pub is_final: bool,
}

/// Text the player typed that a [`crate::number_field`] would not take, sent on the row with the
/// reason ([`NumberError`] reads as words). The field shows the number again. Nothing is sent for
/// a number held to the range: that is taken, as the range's end.
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq)]
pub struct NumberRefused {
    /// The number field's row.
    #[event_target]
    pub entity: Entity,
    /// What the player typed.
    pub text: String,
    /// Why it was refused.
    pub reason: NumberError,
}

/// On a number field's text field (the one a [`crate::number_field`] row types in), which steps on
/// Up and Down. Part of what `number_field()` spawns, a game never builds one.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct NumberInput;

/// A drag of a number row that has begun: the value it began at, and whether it has changed it.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct NumberGrab {
    start: f32,
    moved: bool,
}

/// On each component's number row of a [`crate::vec_field`]: the vec row it belongs to, and its
/// place. Part of what `vec_field()` spawns, a game never builds one.
#[derive(Component, Clone, Copy, Debug)]
pub struct VecPart {
    /// The vec row.
    pub root: Entity,
    /// The component's place, from 0.
    pub index: usize,
}

/// A request to step a number field by `steps` of its step (negative goes down), sent to its text
/// field. A step onto the end it is already at is no change and says nothing.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub(crate) struct NumberStepBy {
    #[event_target]
    pub(crate) entity: Entity,
    pub(crate) steps: f32,
}

/// What a number row's text is for a value: its digits, or nothing while the value is mixed (the
/// hint shows [`MIXED_HINT`]).
fn text_of(value: f32, step: f32, mixed: bool) -> String {
    if mixed {
        String::new()
    } else {
        format_number(value, step)
    }
}

/// Keeps the state: a [`ValueChange<f32>`] on a number row becomes its [`NumberValue`], and the
/// value is one number again. The row's own observer.
pub(crate) fn number_self_update(
    change: On<ValueChange<f32>>,
    mut rows: Query<&mut NumberValue>,
    mut commands: Commands,
) {
    if let Ok(mut held) = rows.get_mut(change.source) {
        held.set_if_neq(NumberValue(change.value));
        commands.entity(change.source).remove::<MixedValue>();
    }
}

/// What a number row reads: its number, step and range, and whether it is mixed.
type Reading = (
    &'static NumberValue,
    &'static NumberStep,
    &'static NumberRange,
);

/// A press that begins on the row itself, its label or its padding, scrubs it: the drag starts
/// from the value the row holds. A press that begins in the field is the field's (the caret). The
/// press had focused the field (a click on a label does, as in a form); a scrub is not typing, so
/// it lets that go, and the field shows no caret and no ring while the number rolls.
pub(crate) fn number_on_drag_start(
    mut start: On<PointerDragStart>,
    rows: Query<&NumberValue, Without<InteractionDisabled>>,
    fields: Query<(Entity, &TextInputOf), With<NumberInput>>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    if start.button != PointerButton::Primary {
        return;
    }
    if let Ok(value) = rows.get(start.entity) {
        start.propagate(false);
        if let Some(held) = focus.get()
            && fields.get(held).is_ok_and(|(_, of)| of.0 == start.entity)
        {
            focus.clear();
        }
        commands.entity(start.entity).insert(NumberGrab {
            start: value.0,
            moved: false,
        });
    }
}

/// The row follows the pointer: the value is where the drag began plus the distance along the
/// row over `UiScale`, times the step (Shift a tenth, Ctrl ten times), held to the range.
pub(crate) fn number_on_drag(
    mut drag: On<PointerDrag>,
    ui_scale: Res<UiScale>,
    keys: Res<ButtonInput<KeyCode>>,
    mut grabs: Query<(&mut NumberGrab, Reading)>,
    mut commands: Commands,
) {
    let Ok((mut grab, (value, step, range))) = grabs.get_mut(drag.entity) else {
        return;
    };
    drag.propagate(false);
    let held = |a, b| keys.pressed(a) || keys.pressed(b);
    let to = scrub_value(
        grab.start,
        drag.distance.x / ui_scale.0,
        step.0,
        range.min,
        range.max,
        held(KeyCode::ShiftLeft, KeyCode::ShiftRight),
        held(KeyCode::ControlLeft, KeyCode::ControlRight),
    );
    if to != value.0 {
        grab.moved = true;
        commands.trigger(ValueChange {
            source: drag.entity,
            value: to,
            is_final: false,
        });
    }
}

/// Lets go: a drag that changed the value ends with one final change, the value as it stands.
pub(crate) fn number_on_drag_end(
    mut end: On<PointerDragEnd>,
    grabs: Query<(&NumberGrab, &NumberValue)>,
    mut commands: Commands,
) {
    let Ok((grab, value)) = grabs.get(end.entity) else {
        return;
    };
    end.propagate(false);
    if grab.moved {
        commands.trigger(ValueChange {
            source: end.entity,
            value: value.0,
            is_final: true,
        });
    }
    commands.entity(end.entity).remove::<NumberGrab>();
}

/// A cancelled pointer lets the number go where it is, as letting go does.
pub(crate) fn number_on_cancel(
    mut cancel: On<PointerCancel>,
    grabs: Query<(&NumberGrab, &NumberValue)>,
    mut commands: Commands,
) {
    let Ok((grab, value)) = grabs.get(cancel.entity) else {
        return;
    };
    cancel.propagate(false);
    if grab.moved {
        commands.trigger(ValueChange {
            source: cancel.entity,
            value: value.0,
            is_final: true,
        });
    }
    commands.entity(cancel.entity).remove::<NumberGrab>();
}

/// A number row's text ends an edit (Enter, or focus leaving after a change): a number is held to
/// the range and is the new value; anything else is refused with its reason and the field shows
/// the number again. The text is written back as the field shows the number (`2.50` is `2.5`).
pub(crate) fn number_on_text(
    change: On<ValueChange<String>>,
    rows: Query<(
        Reading,
        Has<MixedValue>,
        &TextInputValue,
        Has<InteractionDisabled>,
    )>,
    mut commands: Commands,
) {
    if !change.is_final {
        return;
    }
    let row = change.source;
    let Ok(((value, step, range), mixed, shown, disabled)) = rows.get(row) else {
        return;
    };
    if disabled || (mixed && change.value.trim().is_empty()) {
        return;
    }
    match parse_number(&change.value) {
        Ok(typed) => {
            let typed = crate::model::hold(typed, range.min, range.max);
            if mixed || typed != value.0 {
                commands.trigger(ValueChange {
                    source: row,
                    value: typed,
                    is_final: true,
                });
            }
            // Queued, so it lands after the text row's own observer has taken the typed text.
            let text = format_number(typed, step.0);
            if shown.0 != text {
                commands.entity(row).insert(TextInputValue(text));
            }
        }
        Err(reason) => {
            let text = text_of(value.0, step.0, mixed);
            if shown.0 != text {
                commands.entity(row).insert(TextInputValue(text));
            }
            commands.trigger(NumberRefused {
                entity: row,
                text: change.value.clone(),
                reason,
            });
        }
    }
}

/// Up and Down on the focused field step the number (Shift ten steps), held keys repeating, as a
/// spinbutton's do; the other arrows, Home and End are the caret's. Escape puts the field back to
/// the number and ends the edit with no change (the blur it causes sends nothing); the press goes
/// on, so inside a dialog it is still Back.
pub(crate) fn number_on_key_input(
    mut input: On<FocusedInput<KeyboardInput>>,
    keys: Res<ButtonInput<KeyCode>>,
    fields: Query<&TextInputOf, (With<NumberInput>, Without<InteractionDisabled>)>,
    mut rows: Query<(Reading, Has<MixedValue>, &mut TextInputValue)>,
    mut commands: Commands,
) {
    if input.input.state != ButtonState::Pressed {
        return;
    }
    let Ok(&TextInputOf(row)) = fields.get(input.focused_entity) else {
        return;
    };
    let ten = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        10.0
    } else {
        1.0
    };
    match input.input.key_code {
        KeyCode::ArrowUp | KeyCode::ArrowDown => {
            input.propagate(false);
            let up = input.input.key_code == KeyCode::ArrowUp;
            commands.trigger(NumberStepBy {
                entity: input.focused_entity,
                steps: if up { ten } else { -ten },
            });
        }
        KeyCode::Escape => {
            if let Ok(((value, step, _), mixed, mut shown)) = rows.get_mut(row) {
                let text = text_of(value.0, step.0, mixed);
                if shown.0 != text {
                    shown.0 = text;
                }
                commands.entity(row).remove::<TextEdited>();
            }
        }
        _ => {}
    }
}

/// Steps a number field: one final change with the number moved, held to the range, and none when
/// it was already at that end (a mixed value always changes: it becomes one).
pub(crate) fn number_on_step(
    step: On<NumberStepBy>,
    fields: Query<&TextInputOf, With<NumberInput>>,
    rows: Query<(Reading, Has<MixedValue>), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok(&TextInputOf(row)) = fields.get(step.entity) else {
        return;
    };
    let Ok(((value, by, range), mixed)) = rows.get(row) else {
        return;
    };
    let to = step_value(value.0, step.steps, by.0, range.min, range.max);
    if to != value.0 || mixed {
        commands.trigger(ValueChange {
            source: row,
            value: to,
            is_final: true,
        });
    }
}

/// What [`update_numbers`] reads on a number row.
type Numbers = (
    Entity,
    Ref<'static, NumberValue>,
    Ref<'static, NumberStep>,
    Has<MixedValue>,
    Option<Ref<'static, MixedValue>>,
    &'static mut TextInputValue,
    &'static mut TextHint,
);

/// Brings a number row's text up to its number: a [`NumberValue`], [`NumberStep`] or
/// [`MixedValue`] that changed writes the digits (or nothing and the [`MIXED_HINT`], while mixed) into
/// the row's text, which the text input then shows. Writes only what differs, so a scrub that
/// the field already shows costs nothing. Runs in [`crate::GooeySystems::Style`], before
/// [`crate::update_text_inputs`].
pub fn update_numbers(
    mut rows: Query<Numbers, With<NumberRange>>,
    mut unmixed: RemovedComponents<MixedValue>,
) {
    let unmixed: Vec<Entity> = unmixed.read().collect();
    for (entity, value, step, mixed, mark, mut text, mut hint) in &mut rows {
        if !(value.is_changed()
            || step.is_changed()
            || mark.is_some_and(|m| m.is_added())
            || unmixed.contains(&entity))
        {
            continue;
        }
        let digits = text_of(value.0, step.0, mixed);
        if text.0 != digits {
            text.0 = digits;
        }
        let words = if mixed { MIXED_HINT } else { "" };
        if hint.0 != words {
            hint.0 = words.to_string();
        }
    }
}

/// A component of a vec field changed by the player: the vec row's value, and one [`VecChange`].
pub(crate) fn vec_part_on_change(
    change: On<ValueChange<f32>>,
    parts: Query<&VecPart>,
    mut roots: Query<(&mut VecValue, Option<&mut VecMixed>)>,
    mut commands: Commands,
) {
    let Ok(&VecPart { root, index }) = parts.get(change.source) else {
        return;
    };
    let Ok((mut values, mixed)) = roots.get_mut(root) else {
        return;
    };
    if let Some(slot) = values.0.get_mut(index) {
        *slot = change.value;
    }
    if let Some(mut mixed) = mixed
        && mixed.0.get(index).copied().unwrap_or(false)
    {
        mixed.0[index] = false;
    }
    commands.trigger(VecChange {
        entity: root,
        index,
        value: change.value,
        values: values.0.clone(),
        is_final: change.is_final,
    });
}

/// What [`update_vecs`] reads on a vec row.
type Vecs = (
    Ref<'static, VecValue>,
    Ref<'static, NumberStep>,
    Ref<'static, NumberRange>,
    Option<Ref<'static, VecMixed>>,
);

/// What [`update_vecs`] writes on a component's number row.
type Components = (
    Entity,
    &'static VecPart,
    &'static mut NumberValue,
    &'static mut NumberStep,
    &'static mut NumberRange,
    Has<MixedValue>,
);

/// Brings a vec field's components up to the row: a [`VecValue`], [`NumberStep`], [`NumberRange`]
/// or [`VecMixed`] the game set goes into each component's number row (which shows it, with no
/// event). Runs in [`crate::GooeySystems::Style`], before [`update_numbers`].
pub fn update_vecs(
    roots: Query<Vecs, Without<VecPart>>,
    mut parts: Query<Components>,
    mut commands: Commands,
) {
    for (entity, part, mut value, mut step, mut range, mixed) in &mut parts {
        let Ok((values, by, limits, marks)) = roots.get(part.root) else {
            continue;
        };
        if !(values.is_changed()
            || by.is_changed()
            || limits.is_changed()
            || marks.as_ref().is_some_and(|m| m.is_changed()))
        {
            continue;
        }
        if let Some(&number) = values.0.get(part.index) {
            value.set_if_neq(NumberValue(number));
        }
        step.set_if_neq(*by);
        range.set_if_neq(*limits);
        let wanted = marks
            .as_ref()
            .is_some_and(|m| m.0.get(part.index).copied().unwrap_or(false));
        if wanted != mixed {
            if wanted {
                commands.entity(entity).insert(MixedValue);
            } else {
                commands.entity(entity).remove::<MixedValue>();
            }
        }
    }
}
