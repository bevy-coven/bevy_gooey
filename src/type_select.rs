//! Type-select: in a list, a table, a tree, a menu or a select that does not filter, typing a
//! name goes to it. The prefix is the characters typed within [`TYPE_AHEAD_SECS`] of each other,
//! matched against the start of each row's label without regard to case; a longer pause, or focus
//! moving to another group, starts it over; one letter typed again goes on to the next row that
//! starts with it ([`type_ahead`]). Space is not a type-select character: it presses. Measured from
//! Rune's focus runtime (`crates/rune/src/runtime/input/focus.rs` at `f73f72c`, MIT:
//! `TYPE_SELECT_PAUSE = 1.0`, whitespace ignored, Cmd, Ctrl, Alt and Super pass); research:
//! `docs/research/type-select.md`.
//!
//! A table's rows (and so a tree's) are its own, in `crate::table_by_input`; this module holds the
//! prefix both share, and the system for the widgets that are entities of their own: a list's
//! focusable rows, an open menu's items and a select's options.

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;

use crate::binding::Capturing;
use crate::choice::{ChoiceValue, Move, MoveChoice};
use crate::drag::DragState;
use crate::list::ListView;
use crate::menu::{MenuItem, MenuPanel};
use crate::model::{TYPE_AHEAD_SECS, type_ahead};
use crate::nav::show_focus;
use crate::select::Select;
use crate::table::Table;
use crate::text::TextEditing;

/// The prefix typed so far, shared by every widget that takes type-select: one keyboard, one
/// prefix. `GooeyNavPlugin` inits it; it is in the signature of [`crate::table_by_input`], so it
/// is public. Nothing outside the crate has a use for it.
#[derive(Resource, Default, Debug)]
pub struct TypedPrefix {
    text: String,
    /// When the last character came, on `Time<Real>`.
    at: f32,
    /// The group the prefix was typed in: focus moving to another starts over.
    scope: Option<Entity>,
}

impl TypedPrefix {
    /// Adds `characters` typed at `now` in `scope`, and returns what to look for: the prefix, or
    /// its one letter when the prefix is that letter repeated (the next row with it), with how
    /// many characters it has.
    pub(crate) fn needle(&mut self, scope: Entity, now: f32, characters: &str) -> (String, usize) {
        if now - self.at > TYPE_AHEAD_SECS || self.scope != Some(scope) {
            self.text.clear();
        }
        self.at = now;
        self.scope = Some(scope);
        self.text
            .extend(characters.chars().flat_map(char::to_lowercase));
        let needle = match self.text.chars().next() {
            Some(first) if self.text.chars().all(|c| c == first) => first.to_string(),
            _ => self.text.clone(),
        };
        let len = needle.chars().count();
        (needle, len)
    }
}

/// The characters `presses` type, for type-select: printable text only, no whitespace (Space
/// presses), and nothing while Ctrl, Alt or Super is held (`plain` false), which are shortcuts.
pub(crate) fn typed_characters(presses: &[KeyboardInput], plain: bool) -> String {
    if !plain {
        return String::new();
    }
    presses
        .iter()
        .filter(|press| press.state == ButtonState::Pressed)
        .filter_map(|press| match &press.logical_key {
            Key::Character(c) => Some(c.as_str()),
            _ => None,
        })
        .flat_map(str::chars)
        .filter(|c| !c.is_control() && !c.is_whitespace())
        .collect()
}

/// Whether `label` starts with `prefix` (already lower case), without regard to case.
pub(crate) fn starts_with_ignore_case(label: &str, prefix: &str) -> bool {
    let mut letters = label.chars().flat_map(char::to_lowercase);
    prefix.chars().all(|p| letters.next() == Some(p))
}

/// Whether no Ctrl, Alt or Super key is held.
pub(crate) fn plain(keys: &ButtonInput<KeyCode>) -> bool {
    let held = |a, b| keys.pressed(a) || keys.pressed(b);
    !held(KeyCode::ControlLeft, KeyCode::ControlRight)
        && !held(KeyCode::AltLeft, KeyCode::AltRight)
        && !held(KeyCode::SuperLeft, KeyCode::SuperRight)
}

/// What the widgets that are entities of their own read to type-select.
#[derive(SystemParam)]
pub(crate) struct Groups<'w, 's> {
    parents: Query<'w, 's, &'static ChildOf>,
    children: Query<'w, 's, &'static Children>,
    lists: Query<'w, 's, (), With<ListView>>,
    panels: Query<'w, 's, (), With<MenuPanel>>,
    items: Query<'w, 's, &'static MenuItem>,
    stops: Query<'w, 's, (&'static InheritedVisibility, Has<InteractionDisabled>), With<TabIndex>>,
    texts: Query<'w, 's, &'static Text>,
    selects: Query<'w, 's, (&'static Select, &'static ChoiceValue), Without<InteractionDisabled>>,
    tables: Query<'w, 's, (), With<Table>>,
    waiting: Query<'w, 's, (), With<Capturing>>,
    editing: Query<'w, 's, (), With<TextEditing>>,
    drag: Option<Res<'w, DragState>>,
}

impl Groups<'_, '_> {
    /// The keyboard is someone else's: a binding row waits for a press, a text field has focus, or
    /// a payload is carried by key (`crate::not_capturing` and `crate::is_text_editing`, read here
    /// rather than as run conditions so the characters of those frames are read and dropped, not
    /// left for the next frame to type-select).
    fn keyboard_taken(&self) -> bool {
        !self.waiting.is_empty()
            || !self.editing.is_empty()
            || self.drag.as_ref().is_some_and(|drag| drag.carried_by_key())
    }

    /// The nearest `wanted` at or above `entity`.
    fn up(&self, entity: Entity, wanted: impl Fn(Entity) -> bool) -> Option<Entity> {
        std::iter::once(entity)
            .chain(self.parents.iter_ancestors(entity))
            .find(|&e| wanted(e))
    }

    /// Every entity under `root`, depth first in child order: the order the player reads them.
    fn below(&self, root: Entity) -> Vec<Entity> {
        let mut out = Vec::new();
        let mut stack: Vec<Entity> = self
            .children
            .get(root)
            .map(|kids| kids.iter().rev().collect())
            .unwrap_or_default();
        while let Some(entity) = stack.pop() {
            out.push(entity);
            if let Ok(kids) = self.children.get(entity) {
                stack.extend(kids.iter().rev());
            }
        }
        out
    }

    /// The words a list row says: its own text, else the first text under it.
    fn words(&self, entity: Entity) -> Option<String> {
        std::iter::once(entity)
            .chain(self.below(entity))
            .find_map(|e| self.texts.get(e).ok())
            .map(|text| text.0.clone())
    }

    /// An open menu's items, under its own panel and not under a submenu's.
    fn menu_items(&self, panel: Entity) -> Vec<(Entity, String)> {
        self.below(panel)
            .into_iter()
            .filter(|&e| self.up(e, |p| self.panels.contains(p)) == Some(panel))
            .filter_map(|e| {
                self.items
                    .get(e)
                    .ok()
                    .map(|item| (e, item.label().to_owned()))
            })
            .collect()
    }

    /// A list's rows that take focus and are shown.
    fn list_rows(&self, list: Entity) -> Vec<(Entity, String)> {
        self.below(list)
            .into_iter()
            .filter(|&e| {
                self.stops
                    .get(e)
                    .is_ok_and(|(seen, disabled)| seen.get() && !disabled)
            })
            .filter_map(|e| self.words(e).map(|words| (e, words)))
            .collect()
    }
}

/// Type-select in an open menu, a list and a select that does not filter (a table's and a tree's
/// rows are `crate::table_by_input`'s): moves focus to the next item whose label starts with the
/// typed prefix, or steps the select to that option with its one `ValueChange<usize>`. Shows
/// focus; a list scrolls the row into view by following it (`crate::scroll_to_focus`). Runs in
/// [`crate::GooeySystems::Navigate`]; it reads every frame's keys and acts on none while a binding
/// row waits, a text field has focus (the field takes the characters) or a payload is carried.
#[allow(clippy::too_many_arguments)]
pub(crate) fn type_select_by_input(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut presses: MessageReader<KeyboardInput>,
    mut focus: ResMut<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    mut prefix: ResMut<TypedPrefix>,
    groups: Groups,
    mut commands: Commands,
) {
    let typed: Vec<KeyboardInput> = presses.read().cloned().collect();
    if groups.keyboard_taken() {
        return;
    }
    let characters = typed_characters(&typed, plain(&keys));
    let Some(focused) = focus.get() else {
        return;
    };
    // A table's rows, and a tree's, are `table_by_input`'s.
    if characters.is_empty() || groups.tables.contains(focused) {
        return;
    }
    let now = time.elapsed_secs();
    if let Ok((select, value)) = groups.selects.get(focused) {
        if select.filters() {
            return;
        }
        let (needle, len) = prefix.needle(focused, now, &characters);
        let options = select.options();
        if let Some(to) = type_ahead(options.len(), Some(value.0), len, |i| {
            starts_with_ignore_case(&options[i], &needle)
        }) {
            show_focus(&mut shown);
            commands.trigger(MoveChoice {
                entity: focused,
                to: Move::To(to),
            });
        }
        return;
    }
    let (scope, rows) = if groups.items.contains(focused)
        && let Some(panel) = groups.up(focused, |e| groups.panels.contains(e))
    {
        (panel, groups.menu_items(panel))
    } else if let Some(list) = groups.up(focused, |e| groups.lists.contains(e)) {
        (list, groups.list_rows(list))
    } else {
        return;
    };
    let (needle, len) = prefix.needle(scope, now, &characters);
    let from = rows.iter().position(|(e, _)| *e == focused);
    if let Some(to) = type_ahead(rows.len(), from, len, |i| {
        starts_with_ignore_case(&rows[i].1, &needle)
    }) && rows[to].0 != focused
    {
        focus.set(rows[to].0, FocusCause::Navigated);
        show_focus(&mut shown);
    }
}
