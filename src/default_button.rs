//! A dialog's default button: Return presses it when the focus is in one of the dialog's single-line
//! text fields (after the field's final change) or nowhere, so typing a name and pressing Return
//! creates it. A focused button keeps its own Return, and a multi-line text area keeps its newline.
//!
//! Reference: Rune's "Key routing" (Return presses the window's or open modal's default button unless
//! the focused element takes Return); see `docs/research/default-button.md`.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input_focus::FocusedInput;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui::{ComputedStackIndex, InteractionDisabled};
use bevy::ui_widgets::Activate;

use crate::button_variant::ButtonVariant;
use crate::nav::Dialog;

/// The dialogs already warned about for holding two default buttons.
#[derive(Resource, Default)]
pub(crate) struct WarnedDialogs(std::collections::HashSet<Entity>);

/// Marks the one button of a [`crate::dialog`] (or a [`crate::Confirm`] built with
/// [`crate::Confirm::default`]) that Return presses. It draws as [`ButtonVariant::Submit`] unless
/// the host gave it another variant. A disabled default button presses nothing, and two in one
/// dialog warn once: the first wins.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((dialog(), children![(panel(), children![
///     text_input("Name", ""),
///     (menu(), children![(button("Create"), DefaultButton), button("Cancel")]),
/// ])]));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[component(on_add = draw_as_submit)]
pub struct DefaultButton;

/// A default button with no variant of its own is the main action.
fn draw_as_submit(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    if world.get::<ButtonVariant>(entity).is_none() {
        world
            .commands()
            .entity(entity)
            .insert(ButtonVariant::Submit);
    }
}

/// The default button of the dialog `from` stands in, if it has one, and a warning (once per dialog)
/// when it has more.
fn default_of(from: Entity, lookup: &mut DefaultLookup) -> Option<Entity> {
    let mut at = from;
    let dialog = loop {
        if lookup.dialogs.contains(at) {
            break at;
        }
        at = lookup.parents.get(at).ok()?.parent();
    };
    let found: Vec<Entity> = lookup
        .children
        .iter_descendants_depth_first(dialog)
        .filter(|e| lookup.defaults.contains(*e))
        .collect();
    if found.len() > 1 && lookup.warned.0.insert(dialog) {
        warn!(
            "bevy_gooey: dialog {dialog} has {} default buttons; Return presses the first",
            found.len()
        );
    }
    found.first().copied()
}

/// What [`press_default`] reads of the tree.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct DefaultLookup<'w, 's> {
    parents: Query<'w, 's, &'static ChildOf>,
    dialogs: Query<'w, 's, (), With<Dialog>>,
    children: Query<'w, 's, &'static Children>,
    defaults: Query<'w, 's, (), With<DefaultButton>>,
    disabled: Query<'w, 's, (), With<InteractionDisabled>>,
    warned: ResMut<'w, WarnedDialogs>,
}

/// Presses the default button of the dialog `from` is in. Called after a field's final change, so
/// the game sees the value and then the press.
pub(crate) fn press_default(lookup: &mut DefaultLookup, commands: &mut Commands, from: Entity) {
    if let Some(button) = default_of(from, lookup)
        && !lookup.disabled.contains(button)
    {
        commands.trigger(Activate { entity: button });
    }
}

/// Return with nothing focused presses the default button of the dialog that is on top.
/// A global observer: Bevy sends the key to the primary window when no entity has focus.
pub(crate) fn default_on_unfocused_enter(
    input: On<FocusedInput<KeyboardInput>>,
    windows: Query<(), With<Window>>,
    focus: Res<bevy::input_focus::InputFocus>,
    keys: Res<ButtonInput<KeyCode>>,
    open: Query<(Entity, &TabGroup, &InheritedVisibility, &ComputedStackIndex), With<Dialog>>,
    mut lookup: DefaultLookup,
    mut commands: Commands,
) {
    if input.input.state != ButtonState::Pressed
        || input.input.repeat
        || !matches!(input.input.key_code, KeyCode::Enter | KeyCode::NumpadEnter)
        // A key for a focused widget bubbles up to the window; only a key that began there has no focus.
        || focus.get().is_some()
        || !windows.contains(input.focused_entity)
        || keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
            KeyCode::AltLeft,
            KeyCode::AltRight,
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
        ])
    {
        return;
    }
    // The dialog the player is looking at: the visible modal one highest in the UI stack (the rule
    // focus and Cancel use). A dialog with no default of its own presses nothing: one under it is
    // out of reach.
    let Some(top) = open
        .iter()
        .filter(|(_, group, visible, _)| group.modal && visible.get())
        .max_by_key(|(.., stack)| stack.0)
        .map(|(entity, ..)| entity)
    else {
        return;
    };
    if let Some(button) = default_of(top, &mut lookup)
        && !lookup.disabled.contains(button)
    {
        commands.trigger(Activate { entity: button });
    }
}
