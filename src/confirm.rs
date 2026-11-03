//! `confirm`: a modal question with its choices, answered by one event.
//!
//! The component builds the same dialog a game writes by hand (`dialog()`, a `panel()` of a
//! `heading`, a `label` and a `menu` of `button`s) once, from its `on_add` hook, and closes it when
//! it is answered. Research: `docs/research/confirm.md`.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::input_focus::AutoFocus;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::cancel::Cancel;
use crate::nav::Dialog;
use crate::skin::Role;
use crate::widgets::{button, heading, label, menu, panel};

/// A modal question: a title, a line of body and the choices, as one component. Spawn it, observe
/// [`ConfirmChosen`], and nothing else: it is a [`dialog`](crate::dialog) (focus goes in and stays
/// in, and comes back to the opener), its panel is the dialog's, and it despawns itself when it is
/// answered.
///
/// One choice is the **safe** one, the last by default ([`Confirm::safe`] moves it): it has focus
/// when the dialog opens, so Enter, Space and South never destroy anything by reflex, and Escape
/// and a pad's East answer with it. Name each choice for what it does ("Leave", "Stay"), not "Yes"
/// and "No", and put the safe one last.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn quit(mut commands: Commands) {
///     commands.spawn((
///         confirm("Leave the game?", "Progress since the last autosave is lost.", ["Leave", "Stay"]),
///         observe(|chosen: On<ConfirmChosen>, mut exit: MessageWriter<AppExit>| {
///             if chosen.index == 0 {
///                 exit.write(AppExit::Success);
///             }
///         }),
///     ));
/// }
/// ```
///
/// For a dialog with content of its own (a slider, a text field), use [`dialog`](crate::dialog)
/// and [`panel`] directly.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Debug, Clone)]
#[component(on_add = build)]
#[require(
    Node = scrim(),
    Role = Role::Scrim,
    Dialog,
    GlobalZIndex = GlobalZIndex(100),
    TabGroup = TabGroup { order: 0, modal: true },
)]
pub struct Confirm {
    title: String,
    body: String,
    choices: Vec<String>,
    safe: usize,
}

/// The veil: the whole window, its contents centred (the same node [`dialog`](crate::dialog) is).
fn scrim() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

impl Confirm {
    /// A question with `title` as its main instruction, `body` for what the choice costs (empty:
    /// no line) and `choices` as its buttons, left to right in the order given; the last is the
    /// safe one.
    pub fn new<I, S>(title: impl Into<String>, body: impl Into<String>, choices: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let choices: Vec<String> = choices.into_iter().map(Into::into).collect();
        Self {
            title: title.into(),
            body: body.into(),
            safe: choices.len().saturating_sub(1),
            choices,
        }
    }

    /// Makes choice `index` the safe one: the one that has focus when the dialog opens and the one
    /// Escape and a pad's East answer with. An index past the last choice is the last.
    pub fn safe(mut self, index: usize) -> Self {
        self.safe = index.min(self.choices.len().saturating_sub(1));
        self
    }
}

/// A modal question and its choices: see [`Confirm`], which this builds.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(confirm("Delete this save?", "It cannot be brought back.", ["Delete", "Keep"]));
/// # }
/// ```
pub fn confirm<I, S>(title: impl Into<String>, body: impl Into<String>, choices: I) -> Confirm
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    Confirm::new(title, body, choices)
}

/// The player answered a [`Confirm`]: sent once, on the dialog, in the flush that despawns it.
/// `index` is the choice's place in the list it was built from; `cancelled` is true when Escape or a
/// pad's East answered, with the safe choice's index.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn on_chosen(chosen: On<ConfirmChosen>) {
///     info!("choice {} on {}", chosen.index, chosen.entity);
/// }
/// ```
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq, Reflect)]
#[reflect(Event, Clone, Debug)]
pub struct ConfirmChosen {
    /// The confirmation dialog, still alive while an observer of it runs.
    #[event_target]
    pub entity: Entity,
    /// Which choice, counted from the first.
    pub index: usize,
    /// Escape or a pad's East answered, with the safe choice.
    pub cancelled: bool,
}

/// On a confirmation's button: the dialog it answers and its place in the list.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct ConfirmButton {
    /// The [`Confirm`] dialog.
    pub dialog: Entity,
    /// The choice's index.
    pub index: usize,
}

/// Builds the panel in this flush, before any frame can draw the dialog empty.
fn build(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world.commands().queue(move |world: &mut World| {
        let Some(confirm) = world.get::<Confirm>(entity).cloned() else {
            return;
        };
        let card = world.spawn((panel(), ChildOf(entity))).id();
        world.spawn((heading(confirm.title), ChildOf(card)));
        if !confirm.body.is_empty() {
            world.spawn((label(confirm.body), ChildOf(card)));
        }
        let row = world.spawn((menu(), ChildOf(card))).id();
        for (index, text) in confirm.choices.into_iter().enumerate() {
            let choice = world
                .spawn((
                    button(text),
                    ConfirmButton {
                        dialog: entity,
                        index,
                    },
                    ChildOf(row),
                ))
                .id();
            if index == confirm.safe {
                world.entity_mut(choice).insert(AutoFocus);
            }
        }
    });
}

/// A confirmation's button was activated (Enter, Space, South or a click): answer, close. A global
/// observer.
pub(crate) fn confirm_on_activate(
    activate: On<Activate>,
    buttons: Query<&ConfirmButton>,
    dialogs: Query<(), With<Confirm>>,
    mut commands: Commands,
) {
    let Ok(button) = buttons.get(activate.entity) else {
        return;
    };
    if dialogs.get(button.dialog).is_err() {
        return;
    }
    answer(&mut commands, button.dialog, button.index, false);
}

/// Back reached a confirmation: answer with its safe choice, close. A global observer; a `Cancel`
/// ends at the dialog, so this sees it once.
pub(crate) fn confirm_on_cancel(
    mut cancel: On<Cancel>,
    dialogs: Query<&Confirm>,
    mut commands: Commands,
) {
    let Ok(confirm) = dialogs.get(cancel.entity) else {
        return;
    };
    cancel.propagate(false);
    answer(&mut commands, cancel.entity, confirm.safe, true);
}

fn answer(commands: &mut Commands, dialog: Entity, index: usize, cancelled: bool) {
    commands.trigger(ConfirmChosen {
        entity: dialog,
        index,
        cancelled,
    });
    commands.entity(dialog).despawn();
}
