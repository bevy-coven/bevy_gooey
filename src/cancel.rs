//! Cancel: Escape and the pad's B back out of a dialog or a screen.

use bevy::ecs::query::QueryData;
use bevy::ecs::traversal::Traversal;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;

use crate::nav::{Dialog, FocusScopes, show_focus};

/// The player asked to back out: Escape, or any gamepad's East button (B on Xbox, Circle on
/// PlayStation), pressed once. Nothing is closed for you; say what leaving means by observing it
/// on the dialog or the screen the player is in.
///
/// It starts at the focused widget and bubbles up through its parents (Bevy's own event
/// propagation), so an observer on the [`dialog`](crate::dialog) or the [`screen`](crate::screen)
/// hears it, and the innermost layer hears it first. An observer that handled it calls
/// `cancel.propagate(false)`, so a layer above does not also act. It never leaves a
/// [`dialog`](crate::dialog): a modal dialog is its own scope, so a Back inside it does not reach
/// the screen under it, even one the dialog was spawned beneath. With no widget focused, an open
/// dialog still hears it; with no dialog open and nothing focused, nothing is sent, and Escape is
/// the game's own.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     dialog(),
///     observe(|cancel: On<Cancel>, mut commands: Commands| {
///         commands.entity(cancel.entity).despawn();
///     }),
///     children![(panel(), children![label("Leave?"), button("Stay")])],
/// ));
/// # }
/// ```
///
/// The keyboard and the pad are one path, so a game handles one event. Draw the pad's prompt
/// yourself ("B Back"); with it on screen a Back button is not also needed.
#[derive(EntityEvent, Clone, Debug, Reflect)]
#[entity_event(propagate = CancelScope, auto_propagate)]
#[reflect(Event, Clone, Debug)]
pub struct Cancel {
    /// The entity the event is at: the one an observer on it is watching, and, in an observer on
    /// the dialog or the screen, that dialog or screen. `On::original_event_target` is the widget
    /// that had focus (or the dialog, when nothing did).
    #[event_target]
    pub entity: Entity,
}

/// How a [`Cancel`] travels: to the parent, and no further than a [`dialog`](crate::dialog).
#[derive(QueryData)]
pub struct CancelScope {
    child_of: Option<&'static ChildOf>,
    dialog: Has<Dialog>,
}

impl Traversal<Cancel> for CancelScope {
    fn traverse(item: Self::Item<'_, '_>, _: &Cancel) -> Option<Entity> {
        let CancelScopeItem { child_of, dialog } = item;
        if dialog {
            return None;
        }
        child_of.map(ChildOf::parent)
    }
}

/// Sends a [`Cancel`] when Escape or any gamepad's East button is just pressed, and shows focus
/// (`InputFocusVisible`). Runs in [`crate::GooeySystems::Navigate`].
///
/// It is sent to the focused widget when that is a live, visible tab stop in the scope the player
/// sees (the open [`dialog`](crate::dialog) on top, or the screen when none is open); otherwise to
/// the open dialog, so a dialog can always be dismissed; otherwise nowhere, and a press does
/// nothing here. The window Bevy focuses at start-up is not a widget. A press repeated by the
/// keyboard does not send another.
pub fn cancel_by_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    scopes: FocusScopes,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::Escape)
        && !gamepads.iter().any(|g| g.just_pressed(GamepadButton::East))
    {
        return;
    }
    let Some(entity) = scopes.cancel_target(focus.get()) else {
        return;
    };
    show_focus(&mut shown);
    commands.trigger(Cancel { entity });
}
