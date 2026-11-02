//! What an assistive technology can do to a widget it reads: press it, focus it, step it, set it.
//!
//! [`update_accessibility`](crate::access::update_accessibility) declares a node's [`Action`]s
//! from [`actions_of`], and [`act_on_requests`] turns each `bevy::a11y::ActionRequest` into the
//! event the keyboard, the pad or the pointer already sends (`Activate`, `ToggleChecked`,
//! `SetSliderValue`, `MoveChoice`, `ValueChange<String>`), so a game's observer hears the same
//! change whichever input made it and nothing acts twice. One table answers both questions: an
//! action is declared only when it is handled, and a request for one that is not declared is
//! ignored.

use accesskit::{Action, ActionData};
use bevy::a11y::ActionRequest;
use bevy::ecs::query::QueryData;
use bevy::ecs::system::SystemParam;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{
    Activate, Button, Checkbox, SelectedTab, SetSliderValue, Slider, SliderValueChange, Tab,
    TabList, ToggleChecked, ValueChange,
};

use crate::choice::{ChoiceValue, Move, MoveChoice};
use crate::color::{AreaStepBy, ColorArea};
use crate::menu::{MenuItem, MenuOpen, MenuTitle};
use crate::nav::show_focus;
use crate::number::{NumberInput, NumberStepBy};
use crate::section::{SectionHeader, SectionOpen};
use crate::text::TextInputOf;

/// Every action the crate ever declares: a node's set is exactly some of these, so a write can
/// clear the ones a widget no longer takes.
pub(crate) const MANAGED: [Action; 7] = [
    Action::Click,
    Action::Focus,
    Action::Increment,
    Action::Decrement,
    Action::SetValue,
    Action::Expand,
    Action::Collapse,
];

const FOCUS: &[Action] = &[Action::Focus];
const PRESS: &[Action] = &[Action::Focus, Action::Click];
const STEP: &[Action] = &[Action::Focus, Action::Increment, Action::Decrement];
const SET: &[Action] = &[
    Action::Focus,
    Action::Increment,
    Action::Decrement,
    Action::SetValue,
];
const TYPE: &[Action] = &[Action::Focus, Action::SetValue];
const SPIN: &[Action] = &[
    Action::Focus,
    Action::Click,
    Action::Increment,
    Action::Decrement,
];
const FOLD: &[Action] = &[
    Action::Focus,
    Action::Click,
    Action::Expand,
    Action::Collapse,
];

/// The components that decide what a widget can be asked to do.
#[derive(QueryData)]
pub(crate) struct Handles {
    disabled: Has<InteractionDisabled>,
    button: Has<Button>,
    checkbox: Has<Checkbox>,
    tab: Has<Tab>,
    slider: Has<Slider>,
    number: Has<NumberInput>,
    area: Has<ColorArea>,
    choice: Has<ChoiceValue>,
    field: Option<&'static TextInputOf>,
    header: Has<SectionHeader>,
    title: Has<MenuTitle>,
    item: Option<&'static MenuItem>,
    open: Has<MenuOpen>,
    stop: Has<TabIndex>,
}

/// The actions `handles` takes: none while disabled. The one table the declaration and the
/// handler share.
pub(crate) fn actions_of(handles: &HandlesItem) -> &'static [Action] {
    if handles.disabled {
        &[]
    } else if handles.slider || handles.number {
        SET
    } else if handles.area {
        STEP
    } else if handles.field.is_some() {
        TYPE
    } else if handles.choice {
        SPIN
    } else if folds(handles) {
        FOLD
    } else if handles.button || handles.checkbox || handles.tab {
        PRESS
    } else if handles.stop {
        FOCUS
    } else {
        &[]
    }
}

/// A section's header or a menu that opens: the widgets Expand and Collapse mean something to.
fn folds(handles: &HandlesItem) -> bool {
    handles.header || handles.title || handles.item.is_some_and(|item| item.access().1)
}

/// What a request reads besides the widget: where it sits and the state of the strip or section
/// it belongs to.
#[derive(SystemParam)]
pub(crate) struct Around<'w, 's> {
    parents: Query<'w, 's, &'static ChildOf>,
    lists: Query<'w, 's, &'static SelectedTab, With<TabList>>,
    sections: Query<'w, 's, &'static SectionOpen>,
}

/// Does what an assistive technology asked of a widget, through the widget's own event. A request
/// for a node that is gone, is no widget, or has not declared the action is ignored (a screen
/// reader holds a tree one frame old). Runs in [`crate::GooeySystems::Navigate`].
pub(crate) fn act_on_requests(
    mut requests: MessageReader<ActionRequest>,
    mut focus: ResMut<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    widgets: Query<Handles>,
    around: Around,
    mut commands: Commands,
) {
    for request in requests.read() {
        let Some(entity) = Entity::try_from_bits(request.target_node.0) else {
            continue;
        };
        let Ok(handles) = widgets.get(entity) else {
            continue;
        };
        let action = request.action;
        if !actions_of(&handles).contains(&action) {
            continue;
        }
        match action {
            Action::Focus => {
                focus.set(entity, FocusCause::Navigated);
                show_focus(&mut shown);
            }
            Action::Click => {
                if handles.checkbox {
                    commands.trigger(ToggleChecked { entity });
                } else if handles.tab {
                    select_tab(entity, &around, &mut commands);
                } else {
                    commands.trigger(Activate { entity });
                }
            }
            Action::Increment | Action::Decrement => {
                let direction = if action == Action::Increment { 1 } else { -1 };
                if handles.number {
                    commands.trigger(NumberStepBy {
                        entity,
                        steps: direction as f32,
                    });
                } else if handles.area {
                    commands.trigger(AreaStepBy {
                        entity,
                        steps: direction as f32,
                    });
                } else if handles.slider {
                    commands.trigger(SetSliderValue {
                        entity,
                        change: SliderValueChange::RelativeStep(direction as f32),
                    });
                } else if handles.choice {
                    commands.trigger(MoveChoice {
                        entity,
                        to: Move::Step(direction),
                    });
                }
            }
            Action::SetValue => match &request.data {
                Some(ActionData::NumericValue(value)) => {
                    if handles.slider {
                        commands.trigger(SetSliderValue {
                            entity,
                            change: SliderValueChange::Absolute(*value as f32),
                        });
                    } else if handles.number
                        && let Some(&TextInputOf(row)) = handles.field
                    {
                        // The text row's own end-of-edit path parses, holds to the range, reports
                        // and refuses, as a typed number does.
                        commands.trigger(ValueChange {
                            source: row,
                            value: value.to_string(),
                            is_final: true,
                        });
                    }
                }
                Some(ActionData::Value(text)) => {
                    if let Some(&TextInputOf(row)) = handles.field {
                        commands.trigger(ValueChange {
                            source: row,
                            value: text.to_string(),
                            is_final: true,
                        });
                    }
                }
                _ => {}
            },
            Action::Expand | Action::Collapse => {
                let open = if handles.header {
                    around
                        .parents
                        .get(entity)
                        .ok()
                        .and_then(|parent| around.sections.get(parent.parent()).ok())
                        .is_some_and(|open| open.0)
                } else {
                    handles.open
                };
                if open == (action == Action::Collapse) {
                    commands.trigger(Activate { entity });
                }
            }
            _ => {}
        }
    }
}

/// A tab clicked by an assistive technology: the message Bevy's own click observer sends its strip.
fn select_tab(tab: Entity, around: &Around, commands: &mut Commands) {
    let Ok(strip) = around.parents.get(tab).map(ChildOf::parent) else {
        return;
    };
    if around
        .lists
        .get(strip)
        .is_ok_and(|selected| selected.0 != Some(tab))
    {
        commands.trigger(ValueChange::<Option<Entity>> {
            source: strip,
            value: Some(tab),
            is_final: true,
        });
    }
}
