//! The combobox: a text field with a list of options under it that narrows as the player types, a
//! desktop picker where a [`crate::select`] opens a card over the screen.
//!
//! Prior art: Rune's `Combobox` (<https://github.com/anomalyco/rune> at `f73f72c`, rune-kit
//! README "Components"). See `docs/research/combobox.md`.
//!
//! The widget is a wrapper holding a [`crate::text_input`]-style row and, while it is open, a
//! [`crate::popover`] under that row holding a [`crate::list`] of one row per option. Typing never
//! spawns or despawns a row: [`update_comboboxes`] ranks the options against what the field holds
//! with the palette's matcher ([`crate::model::fuzzy_score`]) and rewrites the rows' words, hiding
//! the rows past the last match. Focus stays in the field; Up, Down, Ctrl+N and Ctrl+P move the lit
//! row ([`combobox_by_input`]), a click lights and chooses one, and the list scrolls to keep the lit
//! row in sight ([`crate::ScrollGlide`]).

use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::events::{PointerClick, PointerMove};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};
use bevy::ui::{ComputedNode, InteractionDisabled, ScrollPosition, Selected, UiGlobalTransform};
use bevy::ui_widgets::{Activate, TextInput, ValueChange, observe};

use crate::cancel::Cancel;
use crate::list::{ListView, span};
use crate::model::{fuzzy_score, scroll_to_reveal};
use crate::motion::MotionSettings;
use crate::nav::show_focus;
use crate::popover::{Placement, PopoverClosed};
use crate::scroll_glide::ScrollGlide;
use crate::text::{TextHint, TextInputOf, TextInputValue};

/// How many rows tall an open combobox's list stands; the half row cut at its foot says the list
/// goes on.
pub const COMBO_ROWS: f32 = 6.5;

/// The words in an empty combobox's field.
pub(crate) const COMBO_HINT: &str = "Type to filter";

/// What an open combobox says while nothing matches.
pub(crate) const COMBO_NO_MATCH: &str = "No option matches";

/// How many rows of list stay beyond the lit row when the list scrolls to it: one.
const COMBO_CONTEXT_ROWS: f32 = 1.0;

/// One choice in a [`crate::combobox`].
///
/// Build it with [`ComboOption::new`] and chain the rest; it is `#[non_exhaustive]`, so a field
/// added later does not break a game's list. The row shows the label and, muted at its end, the
/// detail; the list matches the label.
///
/// ```
/// # use bevy_gooey::*;
/// let paris = ComboOption::new("fr-par", "Paris").detail("France");
/// let atlantis = ComboOption::new("atl", "Atlantis").disabled();
/// assert_eq!(paris.id, "fr-par");
/// assert!(paris.enabled && !atlantis.enabled);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct ComboOption {
    /// What a choice sends: the game's own name for the option. Unique in a list.
    pub id: String,
    /// The option in the player's words; what the field shows once it is chosen.
    pub label: String,
    /// Words after the label, muted ("France" after "Paris"); never matched.
    pub detail: Option<String>,
    /// Whether it can be chosen. A disabled option is listed and matched, is never lit and refuses
    /// Enter and a click.
    pub enabled: bool,
}

impl ComboOption {
    /// An enabled option with no detail.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            detail: None,
            enabled: true,
        }
    }

    /// Puts words after the label.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Turns the option off.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// What a [`crate::combobox`] holds: the id of the option chosen, or `None` before one is. It is
/// the widget's state: read it to know the choice, set it to choose from code (the field shows the
/// option's label in the same frame and no [`ValueChange`] is sent for what the game did itself).
/// An id no option has is refused with a warning and shown as an empty field.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct ComboValue(pub Option<String>);

/// The state of a [`crate::combobox`]: its options, and, while its list is open, which of them
/// match and which is lit. Read it through the methods.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(from_reflect = false)]
#[reflect(Component)]
#[require(ComboValue)]
pub struct Combobox {
    #[reflect(ignore)]
    options: Vec<ComboOption>,
    /// The open list, if there is one.
    popover: Option<Entity>,
    /// What the options are matched against: what the player typed since the list opened, or `None`
    /// for every option.
    query: Option<String>,
    /// Indices into `options`, in the order the list shows them.
    matches: Vec<usize>,
    /// The lit match: an index into `matches`, always an enabled option.
    lit: Option<usize>,
    /// The text last written into the field by this widget: a field that reads otherwise was typed in.
    written: String,
    /// The id the field was last made to show; `None` until the first frame.
    shown: Option<Option<String>>,
    /// The lit row has moved and the list has not yet scrolled to it.
    reveal: bool,
}

impl Combobox {
    fn new(options: Vec<ComboOption>) -> Self {
        Self {
            options,
            popover: None,
            query: None,
            matches: Vec::new(),
            lit: None,
            written: String::new(),
            shown: None,
            reveal: false,
        }
    }

    /// The options, in the order they were given.
    pub fn options(&self) -> &[ComboOption] {
        &self.options
    }

    /// The popover of the open list, or `None` while it is closed.
    pub fn list(&self) -> Option<Entity> {
        self.popover
    }

    /// Whether the list is open.
    pub fn is_open(&self) -> bool {
        self.popover.is_some()
    }

    /// The index into [`Combobox::options`] of every option that matches, in the order the list
    /// shows them. Empty while the list is closed.
    pub fn matches(&self) -> &[usize] {
        &self.matches
    }

    /// The option the lit row stands for (the one Enter chooses), if one is lit.
    pub fn lit(&self) -> Option<&ComboOption> {
        self.lit
            .and_then(|lit| self.matches.get(lit))
            .and_then(|&index| self.options.get(index))
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.options.iter().position(|option| option.id == id)
    }

    /// The first enabled match at or after `from`, going `step` (1 or -1) and wrapping.
    fn next_enabled(&self, from: usize, step: isize) -> Option<usize> {
        let count = self.matches.len();
        (0..count)
            .map(|offset| {
                (from as isize + step * offset as isize).rem_euclid(count as isize) as usize
            })
            .find(|&slot| self.options[self.matches[slot]].enabled)
    }
}

/// On a combobox's field row: the combobox it belongs to. Part of what [`crate::combobox`] spawns.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct ComboField(pub Entity);

/// On a combobox's text field while its list is open: Enter is the list's, so the field does not
/// also submit its dialog. Put on and taken off by [`update_comboboxes`].
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct ComboListOpen;

/// One of an open combobox's reused rows: its combobox, the popover it is in and its place among the
/// rows shown. Part of what the list spawns.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct ComboRow {
    /// The combobox the row is in.
    pub combo: Entity,
    /// The popover the row belongs to.
    pub popover: Entity,
    /// Its place among the matches, from the top.
    pub slot: usize,
}

/// On the line that says "No option matches". Part of what the list spawns.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct ComboNote {
    /// The combobox the line is in.
    pub combo: Entity,
    /// The popover it belongs to.
    pub popover: Entity,
}

/// A combobox's wrapper: a labelled field and, once it is typed in or opened, a list of `options`
/// under it.
pub(crate) fn combobox_bundle(label: String, options: Vec<ComboOption>) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Combobox::new(options),
        observe(combo_on_cancel),
        Children::spawn(bevy::ecs::spawn::SpawnWith(
            move |parent: &mut bevy::ecs::relationship::RelatedSpawner<ChildOf>| {
                let combo = parent.target_entity();
                parent.spawn((
                    crate::widgets::combo_field_row(label),
                    ComboField(combo),
                    TextHint(COMBO_HINT.into()),
                    observe(combo_on_click),
                ));
            },
        )),
    )
}

/// The popover under a combobox's field: a list of `rows` reused rows and the no-match line.
fn list_bundle(combo: Entity, popover: Entity, rows: usize) -> impl Bundle {
    children![
        crate::widgets::list(
            COMBO_ROWS,
            Children::spawn(bevy::ecs::spawn::SpawnIter(
                (0..rows).map(move |slot| combo_row(combo, popover, slot))
            )),
        ),
        (
            Node {
                display: Display::None,
                ..default()
            },
            Text::new(COMBO_NO_MATCH),
            crate::skin::Role::MenuNote,
            crate::widgets::ONE_LINE,
            ComboNote { combo, popover },
            Pickable::IGNORE,
        ),
    ]
}

/// One reused row: the option's words and, muted at the end, its detail.
fn combo_row(combo: Entity, popover: Entity, slot: usize) -> impl Bundle {
    (
        Node {
            display: Display::None,
            ..default()
        },
        bevy::ui_widgets::Button,
        crate::skin::Role::PaletteRow,
        ComboRow {
            combo,
            popover,
            slot,
        },
        observe(combo_row_on_activate),
        observe(combo_row_on_move),
        children![
            (
                Text::new(String::new()),
                crate::skin::Role::MenuText,
                crate::widgets::ONE_LINE,
                Pickable::IGNORE
            ),
            (
                Text::new(String::new()),
                crate::skin::Role::MenuShortcut,
                crate::widgets::ONE_LINE,
                Pickable::IGNORE
            ),
        ],
    )
}

/// Ranks `options` against `query`: by score, best first, the list's own order among equals; with
/// no query, the list's own order.
fn rank(options: &[ComboOption], query: Option<&str>) -> Vec<usize> {
    let query = query.map(str::trim).filter(|query| !query.is_empty());
    let mut scored: Vec<(usize, u32)> = options
        .iter()
        .enumerate()
        .filter_map(|(index, option)| match query {
            Some(query) => fuzzy_score(query, &option.label).map(|score| (index, score)),
            None => Some((index, 0)),
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    scored.into_iter().map(|(index, _)| index).collect()
}

/// Opens the list: a popover under the field row with one row per option, every option listed and
/// the lit row on the chosen one (or the first enabled one).
fn open(
    commands: &mut Commands,
    combo: Entity,
    state: &mut Combobox,
    field_row: Entity,
    chosen: Option<&str>,
) {
    if state.popover.is_some() {
        return;
    }
    state.query = None;
    state.matches = rank(&state.options, None);
    state.lit = chosen
        .and_then(|id| state.index_of(id))
        .and_then(|index| state.matches.iter().position(|&m| m == index))
        .filter(|&slot| state.options[state.matches[slot]].enabled)
        .or_else(|| state.next_enabled(0, 1));
    let popover = commands.spawn_empty().id();
    commands.entity(popover).insert((
        crate::widgets::popover(field_row, Placement::Below),
        list_bundle(combo, popover, state.options.len()),
        observe(combo_on_closed),
    ));
    state.popover = Some(popover);
    state.reveal = true;
}

/// Closes the list without choosing: the popover leaves and the field shows the chosen label again.
fn close(commands: &mut Commands, state: &mut Combobox) {
    if let Some(popover) = state.popover.take() {
        commands.entity(popover).insert(crate::Leave);
    }
    state.query = None;
    state.matches.clear();
    state.lit = None;
}

/// A press outside the list closed it (the popover is already going): the widget forgets the list,
/// and [`update_comboboxes`] puts the chosen label back in the field.
fn combo_on_closed(closed: On<PopoverClosed>, mut combos: Query<&mut Combobox>) {
    for mut state in &mut combos {
        if state.popover == Some(closed.entity) {
            state.popover = None;
            state.query = None;
            state.matches.clear();
            state.lit = None;
        }
    }
}

/// Escape in the field blurs it (Bevy's text input lets go of focus and lets the key bubble on). With
/// the list open the first Escape is the list's: focus is put back on the field, and the `Cancel` the
/// press then sends reaches the list's popover and closes it. A second Escape, with the list closed,
/// is the field's own: it blurs, and the dialog or screen hears its `Cancel` as for any text field.
/// Runs in `PreUpdate`, after the keys have been dispatched.
pub fn keep_focus_on_escape(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut focus: ResMut<InputFocus>,
    fields: Query<&TextInputOf, With<TextInput>>,
    rows: Query<&ComboField>,
    combos: Query<&Combobox>,
    mut last: Local<Option<Entity>>,
) {
    let now = focus.get();
    if keys.is_some_and(|keys| keys.just_pressed(KeyCode::Escape))
        && let Some(field) = *last
        && now != Some(field)
        && fields
            .get(field)
            .ok()
            .and_then(|of| rows.get(of.0).ok())
            .and_then(|of| combos.get(of.0).ok())
            .is_some_and(Combobox::is_open)
    {
        focus.set(field, FocusCause::Navigated);
        return;
    }
    *last = now;
}

/// A `Cancel` that reaches the combobox itself (the list was not yet drawn for the popover to take
/// it) with the list open closes it and goes no further; with it closed it goes on up.
fn combo_on_cancel(
    mut cancel: On<Cancel>,
    mut combos: Query<&mut Combobox>,
    mut commands: Commands,
) {
    if let Ok(mut state) = combos.get_mut(cancel.entity)
        && state.is_open()
    {
        cancel.propagate(false);
        close(&mut commands, &mut state);
    }
}

/// A click on the field opens the list.
fn combo_on_click(
    click: On<PointerClick>,
    fields: Query<&ComboField>,
    mut combos: Query<(&mut Combobox, &ComboValue)>,
    mut commands: Commands,
) {
    let Ok(&ComboField(combo)) = fields.get(click.entity) else {
        return;
    };
    if let Ok((mut state, value)) = combos.get_mut(combo) {
        open(
            &mut commands,
            combo,
            &mut state,
            click.entity,
            value.0.as_deref(),
        );
    }
}

/// The pointer moving over an enabled row lights it, as a menu follows the pointer.
fn combo_row_on_move(
    moved: On<PointerMove>,
    rows: Query<&ComboRow>,
    mut combos: Query<&mut Combobox>,
) {
    let Ok(row) = rows.get(moved.entity) else {
        return;
    };
    let Ok(mut state) = combos.get_mut(row.combo) else {
        return;
    };
    if state.popover != Some(row.popover) || state.lit == Some(row.slot) {
        return;
    }
    let enabled = state
        .matches
        .get(row.slot)
        .is_some_and(|&index| state.options[index].enabled);
    if enabled {
        state.lit = Some(row.slot);
    }
}

/// A click on an enabled row chooses it.
fn combo_row_on_activate(
    activate: On<Activate>,
    rows: Query<&ComboRow>,
    mut combos: Query<(&mut Combobox, &mut ComboValue)>,
    fields: Query<(Entity, &ComboField)>,
    mut texts: Query<&mut TextInputValue>,
    mut commands: Commands,
) {
    let Ok(row) = rows.get(activate.entity) else {
        return;
    };
    let Ok((mut state, mut value)) = combos.get_mut(row.combo) else {
        return;
    };
    if state.popover != Some(row.popover) {
        return;
    }
    let Some(&index) = state.matches.get(row.slot) else {
        return;
    };
    let field_row = fields.iter().find(|(_, of)| of.0 == row.combo).map(|f| f.0);
    choose(
        &mut commands,
        row.combo,
        &mut state,
        &mut value,
        index,
        field_row.and_then(|row| texts.get_mut(row).ok()),
    );
}

/// Chooses option `index`: the field shows its label, the list closes, and one
/// [`ValueChange<String>`] with the id is sent on the combobox unless it was already the choice.
fn choose(
    commands: &mut Commands,
    combo: Entity,
    state: &mut Combobox,
    value: &mut ComboValue,
    index: usize,
    text: Option<Mut<TextInputValue>>,
) {
    let option = state.options[index].clone();
    if !option.enabled {
        return;
    }
    close(commands, state);
    state.written = option.label.clone();
    state.shown = Some(Some(option.id.clone()));
    if let Some(mut text) = text
        && text.0 != option.label
    {
        text.0 = option.label.clone();
    }
    let changed = value.0.as_deref() != Some(option.id.as_str());
    if changed {
        value.0 = Some(option.id.clone());
        commands.trigger(ValueChange {
            source: combo,
            value: option.id,
            is_final: true,
        });
    }
}

/// The keys of a combobox, with focus in its field: Down and Up (Ctrl+N and Ctrl+P) open the list
/// and then move the lit row, skipping disabled options and wrapping at the ends, and Enter chooses
/// the lit one. Every other key is the field's own: characters narrow the list, Left, Right, Home
/// and End move the caret. The field takes Up and Down itself (a single line sends the caret to its
/// start), so after a move the caret is put back at the end. Enter while an input method is
/// composing commits the composition, not the choice. A pad has no way to type, so it chooses with
/// a [`crate::select`]. Runs in [`crate::GooeySystems::Navigate`].
#[allow(clippy::too_many_arguments)]
pub fn combobox_by_input(
    mut keys: MessageReader<KeyboardInput>,
    held: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    parents: Query<&ChildOf>,
    mut combos: Query<(&mut Combobox, &mut ComboValue)>,
    rows: Query<(Entity, &ComboField)>,
    of: Query<&TextInputOf>,
    mut fields: Query<&mut EditableText, With<TextInput>>,
    mut texts: Query<&mut TextInputValue>,
    mut commands: Commands,
) {
    let pressed: Vec<(KeyCode, bool)> = keys
        .read()
        .filter(|key| key.state == ButtonState::Pressed)
        .map(|key| (key.key_code, key.repeat))
        .collect();
    let Some(field) = focus.get().filter(|&f| fields.contains(f)) else {
        return;
    };
    let Some(field_row) = of.get(field).ok().map(|of| of.0) else {
        return;
    };
    let Some(combo) = rows
        .get(field_row)
        .ok()
        .map(|(_, of)| of.0)
        .or_else(|| parents.iter_ancestors(field).find(|a| combos.contains(*a)))
    else {
        return;
    };
    let Ok((mut state, mut value)) = combos.get_mut(combo) else {
        return;
    };
    let ctrl = held.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let composing = fields.get(field).is_ok_and(|text| text.is_composing());
    let mut moved = false;
    for (key, repeat) in pressed {
        let step = match key {
            KeyCode::ArrowDown => Some(1),
            KeyCode::ArrowUp => Some(-1),
            KeyCode::KeyN if ctrl => Some(1),
            KeyCode::KeyP if ctrl => Some(-1),
            _ => None,
        };
        if let Some(step) = step {
            show_focus(&mut shown);
            if !state.is_open() {
                open(
                    &mut commands,
                    combo,
                    &mut state,
                    field_row,
                    value.0.as_deref(),
                );
            } else if let Some(lit) = state.lit {
                let next = (lit as isize + step).rem_euclid(state.matches.len() as isize) as usize;
                state.lit = state.next_enabled(next, step);
                state.reveal = true;
            } else {
                state.lit = state.next_enabled(0, step);
                state.reveal = true;
            }
            moved = true;
        } else if matches!(key, KeyCode::Enter | KeyCode::NumpadEnter)
            && !repeat
            && !composing
            && state.is_open()
            && let Some(slot) = state.lit
            && let Some(&index) = state.matches.get(slot)
        {
            let text = texts.get_mut(field_row).ok();
            choose(&mut commands, combo, &mut state, &mut value, index, text);
        }
    }
    if moved && let Ok(mut text) = fields.get_mut(field) {
        text.queue_edit(TextEdit::TextEnd(false));
    }
}

/// Puts `label` in the field: the row's value and the field's own text together, so no edit event in
/// the frame between them reads the old text against the new value as something the player typed.
fn show(
    label: &str,
    state: &mut Combobox,
    text: &mut TextInputValue,
    editor: Option<Mut<EditableText>>,
) {
    if text.0 != label {
        text.0 = label.to_owned();
    }
    if let Some(mut editor) = editor
        && editor.value() != label
    {
        editor.editor_mut().set_text(label);
        editor.queue_edit(TextEdit::TextEnd(false));
    }
    state.written = label.to_owned();
}

/// What [`update_comboboxes`] writes a row from.
type RowParts = (
    Entity,
    &'static ComboRow,
    &'static Children,
    &'static mut Node,
    Has<Selected>,
    Has<InteractionDisabled>,
);

/// What it scrolls a list by.
type ViewParts = (
    Entity,
    &'static ComputedNode,
    &'static UiGlobalTransform,
    &'static mut ScrollPosition,
    &'static mut ScrollGlide,
);

/// A popover's node: not a row, not the note.
type PopoverOnly = (
    Without<ComboRow>,
    Without<ComboNote>,
    With<crate::PopoverPanel>,
);

/// Keeps every combobox true to its field and its list:
/// - a game's write to [`ComboValue`] shows the option's label in the field;
/// - what the player types opens the list (once) and narrows it, lighting the first enabled match;
/// - focus leaving the field closes it, and a closed list puts the chosen label back in the field;
/// - the rows are rewritten to the matches (their words, the lit one, the disabled ones), the rows
///   past the last match are hidden, the list says "No option matches" when nothing does, the lit
///   row is scrolled into sight and the popover is at least as wide as the field.
///
/// Writes only what differs. Runs in [`crate::GooeySystems::Style`], before [`crate::apply_skin`].
#[allow(clippy::too_many_arguments)]
pub fn update_comboboxes(
    focus: Res<InputFocus>,
    settings: Res<MotionSettings>,
    time: Option<Res<Time<Real>>>,
    mut combos: Query<(Entity, &mut Combobox, &ComboValue)>,
    mut field_rows: Query<(Entity, &ComboField, &mut TextInputValue)>,
    inputs: Query<(Entity, &TextInputOf, Has<ComboListOpen>), With<TextInput>>,
    mut editors: Query<&mut EditableText, With<TextInput>>,
    layouts: Query<(&ComputedNode, &UiGlobalTransform)>,
    parents: Query<&ChildOf>,
    mut rows: Query<RowParts>,
    mut notes: Query<(&ComboNote, &mut Node), Without<ComboRow>>,
    mut label_texts: Query<&mut Text>,
    mut views: Query<ViewParts, With<ListView>>,
    mut popovers: Query<&mut Node, PopoverOnly>,
    mut commands: Commands,
) {
    for (combo, mut state, value) in &mut combos {
        let Some((field_row, _, mut typed)) =
            field_rows.iter_mut().find(|(_, of, _)| of.0 == combo)
        else {
            continue;
        };
        let field = inputs
            .iter()
            .find(|(_, of, _)| of.0 == field_row)
            .map(|(field, ..)| field);
        let focused = field.is_some_and(|field| focus.get() == Some(field));
        let label_of = |state: &Combobox, id: &Option<String>| -> String {
            id.as_deref()
                .and_then(|id| state.index_of(id))
                .map(|index| state.options[index].label.clone())
                .unwrap_or_default()
        };

        // A game's write (or the first frame) shows the chosen label.
        if state.shown.as_ref() != Some(&value.0) {
            if let Some(id) = &value.0
                && state.index_of(id).is_none()
            {
                warn!(
                    "the combobox {combo} was given the id {id:?}, which none of its options has"
                );
            }
            let label = label_of(&state, &value.0);
            show(
                &label,
                &mut state,
                &mut typed,
                field.and_then(|f| editors.get_mut(f).ok()),
            );
            state.shown = Some(value.0.clone());
        } else if typed.0 != state.written && focused {
            // The player typed: open the list on it and narrow it.
            let query = typed.0.clone();
            state.written = query.clone();
            if !state.is_open() {
                open(&mut commands, combo, &mut state, field_row, None);
            }
            state.query = Some(query);
            state.matches = rank(&state.options, state.query.as_deref());
            state.lit = state.next_enabled(0, 1);
            state.reveal = true;
        }

        // Focus in the field keeps the list; anywhere else closes it. A closed list puts the chosen
        // label back in the field.
        if state.is_open() && !focused {
            close(&mut commands, &mut state);
        }
        if !state.is_open() {
            let label = label_of(&state, &value.0);
            if state.written != label {
                show(
                    &label,
                    &mut state,
                    &mut typed,
                    field.and_then(|f| editors.get_mut(f).ok()),
                );
            }
        }
        if let Some((field, _, listed_now)) = field.and_then(|f| inputs.get(f).ok()) {
            let listed = state.is_open();
            if listed && !listed_now {
                commands.entity(field).insert(ComboListOpen);
            } else if !listed && listed_now {
                commands.entity(field).remove::<ComboListOpen>();
            }
        }

        let Some(popover) = state.popover else {
            continue;
        };
        let count = state.matches.len();
        for (row, part, children, mut node, lit, disabled) in &mut rows {
            if part.combo != combo || part.popover != popover {
                continue;
            }
            let option = state
                .matches
                .get(part.slot)
                .map(|&index| &state.options[index]);
            let display = if option.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
            let words = option.map_or((String::new(), String::new()), |option| {
                (
                    option.label.clone(),
                    option.detail.clone().unwrap_or_default(),
                )
            });
            for (child, words) in children.iter().zip([words.0, words.1]) {
                if let Ok(mut text) = label_texts.get_mut(child)
                    && text.0 != words
                {
                    text.0 = words;
                }
            }
            let want_lit = option.is_some() && state.lit == Some(part.slot);
            if want_lit && !lit {
                commands.entity(row).insert(Selected);
            } else if !want_lit && lit {
                commands.entity(row).remove::<Selected>();
            }
            let want_off = option.is_some_and(|option| !option.enabled);
            if want_off && !disabled {
                commands.entity(row).insert(InteractionDisabled);
            } else if !want_off && disabled {
                commands.entity(row).remove::<InteractionDisabled>();
            }
        }
        for (note, mut node) in &mut notes {
            if note.combo != combo || note.popover != popover {
                continue;
            }
            let display = if count == 0 {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
        // At least as wide as the field, so the list does not breathe as the matches change.
        if let (Ok((field_node, _)), Ok(mut node)) =
            (layouts.get(field_row), popovers.get_mut(popover))
        {
            let width = Val::Px((field_node.size().x * field_node.inverse_scale_factor()).round());
            if node.min_width != width {
                node.min_width = width;
            }
        }
        // Scroll the lit row into sight once its row has been laid out.
        if state.reveal
            && let Some(slot) = state.lit
        {
            let row = rows
                .iter()
                .find(|(_, part, ..)| {
                    part.combo == combo && part.popover == popover && part.slot == slot
                })
                .map(|(row, ..)| row);
            let view = views
                .iter_mut()
                .find(|(view, ..)| parents.iter_ancestors(*view).any(|a| a == popover));
            if let (Some(row), Some((_, view, view_transform, mut scroll, mut glide))) = (row, view)
                && let Ok((node, transform)) = layouts.get(row)
                && node.size().y > 0.0
                && view.size().y > 0.0
            {
                let scale = view.inverse_scale_factor();
                let (top, height) = span(node, transform);
                let (view_top, view_height) = span(view, view_transform);
                let laid_out = view.scroll_position.y * scale;
                let max = (view.content_size().y + view.scrollbar_size.y - view.size().y).max(0.0)
                    * scale;
                let from_start = top - view_top + laid_out;
                let base = glide.goal().unwrap_or(laid_out);
                let want = scroll_to_reveal(
                    from_start,
                    from_start + height,
                    view_height,
                    base,
                    max,
                    COMBO_CONTEXT_ROWS * height,
                );
                if (want - glide.goal().unwrap_or(scroll.y)).abs() > 0.5 {
                    if ScrollGlide::glides(&settings, time.as_deref()) {
                        glide.go(&scroll, want);
                    } else {
                        scroll.y = want;
                    }
                }
                state.reveal = false;
            }
        }
    }
}
