//! UI feedback: a message for every focus move and activation, for the host to turn into sound.

use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ValueChange};

use crate::skin::Role;

/// Something a player did to a widget, written the frame it happens. The crate plays no sound: read
/// these and play your own, so every action answers on a second channel.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn click(mut feedback: MessageReader<UiFeedback>) {
///     for f in feedback.read() {
///         match f.kind {
///             UiFeedbackKind::Focus => { /* a soft tick */ }
///             UiFeedbackKind::Activate => { /* a firm click */ }
///         }
///     }
/// }
/// ```
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiFeedback {
    /// The widget.
    pub entity: Entity,
    /// What happened to it.
    pub kind: UiFeedbackKind,
}

/// What happened, in a [`UiFeedback`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq, Clone)]
pub enum UiFeedbackKind {
    /// Focus moved onto the widget.
    Focus,
    /// The widget was activated (a button clicked, Enter, Space or the pad's South button), a
    /// toggle flipped, or a choice changed its option.
    Activate,
}

/// Writes [`UiFeedbackKind::Activate`] when a button is activated. A global observer.
pub(crate) fn feedback_on_activate(
    event: On<Activate>,
    roles: Query<&Role>,
    mut out: MessageWriter<UiFeedback>,
) {
    if roles.get(event.entity) == Ok(&Role::Button) {
        out.write(UiFeedback {
            entity: event.entity,
            kind: UiFeedbackKind::Activate,
        });
    }
}

/// Writes [`UiFeedbackKind::Activate`] when a toggle flips. A global observer.
pub(crate) fn feedback_on_toggle(
    event: On<ValueChange<bool>>,
    roles: Query<&Role>,
    mut out: MessageWriter<UiFeedback>,
) {
    if roles.get(event.source) == Ok(&Role::Toggle) {
        out.write(UiFeedback {
            entity: event.source,
            kind: UiFeedbackKind::Activate,
        });
    }
}

/// Writes [`UiFeedbackKind::Activate`] when a choice changes its option, by a step, a jump or a
/// cycle, or a tabs widget turns its page: once per change, never for a press at an end that
/// changes nothing. A global observer.
pub(crate) fn feedback_on_choice(
    event: On<ValueChange<usize>>,
    roles: Query<&Role>,
    mut out: MessageWriter<UiFeedback>,
) {
    if matches!(roles.get(event.source), Ok(Role::Choice | Role::Tabs)) {
        out.write(UiFeedback {
            entity: event.source,
            kind: UiFeedbackKind::Activate,
        });
    }
}

/// Writes [`UiFeedbackKind::Focus`] when focus moves onto a button, toggle, slider, choice or tab.
/// Runs in [`crate::GooeySystems::Navigate`], after [`crate::navigate_by_input`].
pub fn feedback_on_focus(
    focus: Res<InputFocus>,
    roles: Query<&Role>,
    mut out: MessageWriter<UiFeedback>,
) {
    if !focus.is_changed() {
        return;
    }
    if let Some(entity) = focus.get()
        && matches!(
            roles.get(entity),
            Ok(Role::Button | Role::Toggle | Role::Slider | Role::Choice | Role::Tab)
        )
    {
        out.write(UiFeedback {
            entity,
            kind: UiFeedbackKind::Focus,
        });
    }
}
