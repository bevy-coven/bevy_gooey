//! The select: a [`crate::choice`] that, past a few options, opens a searchable list. The list is
//! the command palette's card over the select's own options (`palette.rs`), so the field, the lit
//! row, the keys, the scrolling and the look are one path.

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;

use crate::choice::ChoiceValue;
use crate::palette::CommandPalette;
use crate::widgets::select_list;

/// A [`crate::select`] shows a list to filter when it has more options than this; with this many or
/// fewer it is a plain [`crate::choice`] that cycles.
pub const SELECT_FILTER_ABOVE: usize = 8;

/// The words in an open select's empty field.
pub(crate) const SELECT_HINT: &str = "Type to filter";

/// What an open select says while nothing matches.
pub(crate) const SELECT_NO_MATCH: &str = "No option matches";

/// The options of a [`crate::select`], as text. Part of what it spawns; the row's value is its
/// [`ChoiceValue`](crate::ChoiceValue).
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Select {
    options: Vec<String>,
}

impl Select {
    pub(crate) fn new(options: Vec<String>) -> Self {
        Self { options }
    }

    /// The options, in the order they were given.
    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// Whether it opens a list: it has more than [`SELECT_FILTER_ABOVE`] options.
    pub fn filters(&self) -> bool {
        self.options.len() > SELECT_FILTER_ABOVE
    }
}

/// Enter, Space, South or a click on a select that filters opens its list, once: a second press
/// while one is open is the list's own.
pub(crate) fn select_on_activate(
    activate: On<Activate>,
    selects: Query<(&Select, &ChoiceValue), Without<InteractionDisabled>>,
    open: Query<&CommandPalette>,
    mut commands: Commands,
) {
    let Ok((select, value)) = selects.get(activate.entity) else {
        return;
    };
    if !select.filters() || open.iter().any(|list| list.is_for(activate.entity)) {
        return;
    }
    commands.spawn(select_list(activate.entity, select.options(), value.0));
}
