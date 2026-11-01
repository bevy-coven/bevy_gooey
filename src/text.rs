//! The text input: what keeps a [`crate::text_input`] row's state, tells the game about each edit
//! and tells the host when a field has the player. The field itself is Bevy's headless `TextInput`;
//! its look is [`crate::apply_skin`]'s, like every widget's.

use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::{AcquireFocus, FocusCause, FocusedInput, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit, TextEditChange, TextReadWriteMode};
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{TextInput, ValueChange};

/// What a [`crate::text_input`] row holds: the text in its field. It is the widget's state: read it
/// to know what the player typed, set it to change the field from code (the field shows the text
/// in the same frame, the caret at its end, and no [`ValueChange`] is sent for a change the game
/// made itself). A value longer than the row's [`TextLimit`] is shown whole: a limit stops what the
/// player types, never what the game sets.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn save_name(names: Query<&TextInputValue, Changed<TextInputValue>>) {
///     for name in &names {
///         info!("the player is called {}", name.0);
///     }
/// }
/// ```
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct TextInputValue(pub String);

/// The words a [`crate::text_input`] field shows, muted, while it is empty ("Enter a name"): a hint
/// at what to type, never the field's label (the row's own label stays outside the field and always
/// shows). Put it on the row, at spawn or later.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct TextHint(pub String);

/// The most characters a [`crate::text_input`] field takes from the player, typed or pasted. Put it
/// on the row, at spawn or later. It never cuts a [`TextInputValue`] the game set.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct TextLimit(pub usize);

/// On a [`crate::text_input`] row exactly while its field has focus: the player is typing. A game's
/// own hotkeys wait on it ([`is_text_editing`]), and a host that opens the platform's on-screen
/// keyboard for a pad player does it when this is added. It is put on and taken off in
/// [`crate::GooeySystems::Navigate`], so a system that must see the frame's change runs after that
/// set, and one that must not act on the press that ends the edit (Escape) runs before it.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct TextEditing;

/// A run condition: whether any [`crate::text_input`] has the player's keys. A game gates its own
/// key handling on `not(is_text_editing)`, so typing a name does not also walk the character.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn pause() {}
/// fn plug(app: &mut App) {
///     app.add_systems(Update, pause.run_if(not(is_text_editing)));
/// }
/// ```
pub fn is_text_editing(editing: Query<(), With<TextEditing>>) -> bool {
    !editing.is_empty()
}

/// Which [`crate::text_input`] row a field or its hint belongs to. Part of what `text_input()`
/// spawns, a game never builds one; read it to find the row from the field a Bevy event names
/// (`FocusGained`, `TextEditChange`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextInputOf(pub Entity);

/// On a [`crate::text_input`] row whose text changed since the last time an edit ended: when
/// focus leaves it, that is a final change. Put on and taken off by the row's own systems.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TextEdited;

/// The field's own edit notice becomes the game's: one [`ValueChange<String>`] on the row for each
/// edit that changed the text, and none for a caret that only moved. A global observer.
pub(crate) fn text_on_edit(
    change: On<TextEditChange>,
    fields: Query<(&EditableText, &TextInputOf)>,
    rows: Query<&TextInputValue>,
    mut commands: Commands,
) {
    let Ok((text, &TextInputOf(row))) = fields.get(change.event_target()) else {
        return;
    };
    let Ok(held) = rows.get(row) else {
        return;
    };
    let now = text.value().to_string();
    if now != held.0 {
        commands.entity(row).insert(TextEdited);
        commands.trigger(ValueChange {
            source: row,
            value: now,
            is_final: false,
        });
    }
}

/// Keeps the state: a [`ValueChange<String>`] on a row becomes its [`TextInputValue`]. The row's
/// own observer, like Bevy's `checkbox_self_update`.
pub(crate) fn text_self_update(
    change: On<ValueChange<String>>,
    mut rows: Query<&mut TextInputValue>,
) {
    if let Ok(mut held) = rows.get_mut(change.source)
        && held.0 != change.value
    {
        held.0.clone_from(&change.value);
    }
}

/// Ends an edit: the text as it stands, once, `is_final`.
fn finish(commands: &mut Commands, row: Entity, value: String) {
    commands.entity(row).remove::<TextEdited>();
    commands.trigger(ValueChange {
        source: row,
        value,
        is_final: true,
    });
}

/// Enter in a focused field is a submit: a final change with the text, whether or not it changed.
/// The field stays focused (Up, Down and Tab leave it). Bevy's handler lets Enter bubble ("to allow
/// for tab navigation and submit actions"), and holds it back while an IME is composing, which this
/// does too.
pub(crate) fn text_on_key_input(
    input: On<FocusedInput<KeyboardInput>>,
    fields: Query<(&EditableText, &TextInputOf), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    if input.input.state != ButtonState::Pressed
        || input.input.repeat
        || !matches!(input.input.key_code, KeyCode::Enter | KeyCode::NumpadEnter)
    {
        return;
    }
    let Ok((text, &TextInputOf(row))) = fields.get(input.focused_entity) else {
        return;
    };
    if !text.is_composing() {
        finish(&mut commands, row, text.value().to_string());
    }
}

/// A press on the row, its label or its padding focuses its field, as a click on an HTML label
/// does, so the whole row is the target. Bevy's `AcquireFocus` bubbles up from the press; this
/// takes it at the row, before it reaches the window and clears focus.
pub(crate) fn text_row_on_acquire(
    mut acquire: On<AcquireFocus>,
    fields: Query<(Entity, &TextInputOf), (With<TextInput>, Without<InteractionDisabled>)>,
    mut focus: ResMut<InputFocus>,
) {
    let row = acquire.focused_entity;
    let Some((field, _)) = fields.iter().find(|(_, of)| of.0 == row) else {
        return;
    };
    acquire.propagate(false);
    if focus.get() != Some(field) {
        focus.set(field, FocusCause::Pressed);
    }
}

/// Puts [`TextEditing`] on the row whose field has focus and takes it off the row that lost it,
/// which ends that row's edit (a final change, if the text changed since the last one). Focus on
/// the row itself (an `AutoFocus` on it, or a game that set it there) goes to its field. Runs in
/// [`crate::GooeySystems::Navigate`], and only when focus moved.
pub fn track_text_editing(
    mut focus: ResMut<InputFocus>,
    fields: Query<(Entity, &TextInputOf), With<TextInput>>,
    editing: Query<(Entity, &TextInputValue, Has<TextEdited>), With<TextEditing>>,
    mut commands: Commands,
) {
    if !focus.is_changed() {
        return;
    }
    let mut current = focus.get();
    if let Some(row) = current
        && let Some((field, _)) = fields.iter().find(|(_, of)| of.0 == row)
    {
        focus.set(field, FocusCause::Auto);
        current = Some(field);
    }
    let now = current.and_then(|f| fields.get(f).ok()).map(|(_, of)| of.0);
    for (row, value, edited) in &editing {
        if Some(row) != now {
            commands.entity(row).remove::<TextEditing>();
            if edited {
                finish(&mut commands, row, value.0.clone());
            }
        }
    }
    if let Some(row) = now
        && !editing.contains(row)
    {
        commands.entity(row).insert(TextEditing);
    }
}

/// What [`update_text_inputs`] reads on a row: the text, the hint, the limit and whether it is
/// disabled.
type Rows = (
    Entity,
    &'static TextInputValue,
    Option<&'static TextHint>,
    Option<&'static TextLimit>,
    Has<InteractionDisabled>,
);

/// A row whose field or hint may be out of date.
type Stale = Or<(
    Changed<TextInputValue>,
    Changed<TextHint>,
    Changed<TextLimit>,
    Added<InteractionDisabled>,
)>;

/// Brings a row's parts up to what the row says: a [`TextInputValue`] the game set goes into the
/// field (caret at the end), the hint shows exactly while the value is empty, the limit goes to
/// the field, and a disabled row disables its field. Writes only what differs, so the field's own
/// edits (which already match the row) cost nothing. Runs in [`crate::GooeySystems::Style`], before
/// [`crate::apply_skin`].
pub fn update_text_inputs(
    rows: Query<Rows, Stale>,
    mut fields: Query<
        (
            Entity,
            &TextInputOf,
            &mut EditableText,
            Has<InteractionDisabled>,
        ),
        With<TextInput>,
    >,
    mut hints: Query<(&TextInputOf, &mut Text, &mut Node), Without<TextInput>>,
    mut enabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    for (row, value, hint, limit, disabled) in &rows {
        for (field, of, mut text, field_disabled) in &mut fields {
            if of.0 != row {
                continue;
            }
            if text.value().to_string() != value.0 {
                text.editor_mut().set_text(&value.0);
                text.queue_edit(TextEdit::TextEnd(false));
            }
            let max = limit.map(|l| l.0);
            if text.max_characters != max {
                text.max_characters = max;
            }
            if disabled && !field_disabled {
                commands
                    .entity(field)
                    .insert((InteractionDisabled, TextReadWriteMode::ReadOnly));
            }
        }
        for (of, mut shown, mut node) in &mut hints {
            if of.0 != row {
                continue;
            }
            let words = hint.map_or("", |h| h.0.as_str());
            if shown.0 != words {
                words.clone_into(&mut shown.0);
            }
            let display = if value.0.is_empty() && !words.is_empty() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
    for row in enabled.read() {
        for (field, of, _, field_disabled) in &fields {
            if of.0 == row && field_disabled {
                commands
                    .entity(field)
                    .remove::<InteractionDisabled>()
                    .insert(TextReadWriteMode::Editable);
            }
        }
    }
}
