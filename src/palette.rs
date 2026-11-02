//! The command palette: a modal field over a searchable list of every command, each with its chord
//! and, when it is off, why.
//!
//! The game keeps its commands in [`CommandList`] and spawns [`crate::command_palette`] when its
//! own chord is pressed. The palette is a [`crate::dialog`] (a modal scope, focus in and back,
//! [`Cancel`]) holding a [`crate::text_input`]-style field and [`PALETTE_ROWS`] reused rows: typing
//! spawns and despawns nothing, [`update_palette`] only rewrites the rows' text. Focus stays in the
//! field while Up and Down move the lit row ([`palette_by_input`]), as the WAI-ARIA combobox does.
//! Enter, South or a click on an enabled row despawns the palette and sends one [`RunCommand`].

use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::{AcquireFocus, FocusCause, InputFocus};
use bevy::picking::events::PointerMove;
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};
use bevy::ui::{ComputedNode, InteractionDisabled, Selected, UiGlobalTransform};
use bevy::ui_widgets::{Activate, TextInput, ValueChange};

use crate::cancel::Cancel;
use crate::choice::ChoiceValue;
use crate::feedback::{UiFeedback, UiFeedbackKind};
use crate::model::{PAD_REPEAT_DELAY, PAD_REPEAT_RATE, fuzzy_score, repeat_steps};
use crate::shortcut::Shortcut;
use crate::skin::SAFE_EDGE_PERCENT;
use crate::text::{TextInputOf, TextInputValue};

/// How many rows a palette shows at once. They are reused: the list scrolls under them as the lit
/// row moves, so a thousand commands cost the same entities as eight.
pub const PALETTE_ROWS: usize = 8;

/// What a palette says while nothing matches the query.
pub(crate) const NO_MATCH: &str = "No command matches";

/// The words in an empty palette's field.
pub(crate) const PALETTE_HINT: &str = "Type a command";

/// One command a [`command_palette`](crate::command_palette) lists.
///
/// Build it with [`PaletteCommand::new`] and chain the rest; it is `#[non_exhaustive]`, so a field
/// added later does not break a game's list. The row shows `"Category: Title"` and the palette
/// matches that text, so "file open" finds "File: Open Project".
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// let open = PaletteCommand::new("project.open", "Open Project")
///     .category("File")
///     .shortcut(Shortcut::primary(KeyCode::KeyO));
/// let save = PaletteCommand::new("project.save", "Save").disabled("No project is open");
/// assert_eq!(open.label(), "File: Open Project");
/// assert!(save.enabled.is_err());
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct PaletteCommand {
    /// What [`RunCommand`] carries: the game's own name for the command. Unique in a list.
    pub id: String,
    /// The command in the player's words, as a menu item would say it.
    pub title: String,
    /// The group it belongs to ("File"), shown before the title; `None` shows the title alone.
    pub category: Option<String>,
    /// The chord that does the same, shown at the row's end, in the platform's words.
    pub shortcut: Option<Shortcut>,
    /// `Ok` when the command can run now; `Err(reason)` when it cannot, which the row shows in
    /// place of the chord. A disabled command is listed, can be lit and runs nothing.
    pub enabled: Result<(), String>,
}

impl PaletteCommand {
    /// An enabled command with no category and no chord.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            category: None,
            shortcut: None,
            enabled: Ok(()),
        }
    }

    /// Puts the command under a category.
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Gives the command the chord that does the same.
    pub fn shortcut(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Turns the command off, with the reason the row shows.
    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        self.enabled = Err(reason.into());
        self
    }

    /// The text a row shows and a query is matched against: `"Category: Title"`, or the title.
    pub fn label(&self) -> String {
        match &self.category {
            Some(category) => format!("{category}: {}", self.title),
            None => self.title.clone(),
        }
    }
}

/// Every command a [`command_palette`](crate::command_palette) lists. It is the one list: the
/// game fills it once and changes it as commands come and go or switch on and off, and an open
/// palette follows the change in the same frame.
#[derive(Resource, Clone, Debug, Default)]
pub struct CommandList(pub Vec<PaletteCommand>);

/// The ids of the commands the palette ran, the most recent first, [`PaletteHistory::LIMIT`] at
/// most. An empty query lists them first, and so does a query, among the commands that match it.
/// Save it with the game's settings (it is serde data with the `serde` feature) to keep recents
/// between runs, or clear it to forget them.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PaletteHistory(pub Vec<String>);

impl PaletteHistory {
    /// The most ids kept.
    pub const LIMIT: usize = 10;

    /// Puts `id` first, once, dropping the oldest past [`Self::LIMIT`].
    pub fn record(&mut self, id: &str) {
        self.0.retain(|seen| seen != id);
        self.0.insert(0, id.to_owned());
        self.0.truncate(Self::LIMIT);
    }
}

/// The player chose a command in a palette: observe it with `app.add_observer(|run: On<RunCommand>
/// | ..)` and do what `id` names. The palette is already despawned and focus is back where it was,
/// so a command may open a dialog or move focus. Sent once per choice, never for a disabled
/// command.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn plug(app: &mut App) {
/// app.add_observer(|run: On<RunCommand>| match run.id.as_str() {
///     "project.open" => info!("open a project"),
///     other => warn!("no command {other}"),
/// });
/// # }
/// ```
#[derive(Event, Clone, Debug, PartialEq, Eq)]
pub struct RunCommand {
    /// The [`PaletteCommand::id`] that was chosen.
    pub id: String,
}

/// An open [`command_palette`](crate::command_palette): which of its matches is lit and which
/// matches there are. Its fields are the palette's own; read them through the methods.
#[derive(Component, Clone, Debug)]
pub struct CommandPalette {
    selected: usize,
    top: usize,
    /// How many of [`PALETTE_ROWS`] fit the window at the current text size.
    cap: usize,
    query: String,
    matches: Vec<usize>,
    /// The options of the [`crate::select`] this palette chooses for, when it is not the game's
    /// command palette: the list it matches, in place of [`CommandList`].
    source: Option<PaletteSource>,
}

/// What a select's list is made of: its options as commands (the index is the id) and the row
/// whose value the choice sets.
#[derive(Clone, Debug)]
struct PaletteSource {
    options: Vec<PaletteCommand>,
    select: Entity,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self {
            selected: 0,
            top: 0,
            cap: PALETTE_ROWS,
            query: String::new(),
            matches: Vec::new(),
            source: None,
        }
    }
}

impl CommandPalette {
    /// The state of the list a [`crate::select`] opens: `options` in their own order, the one at
    /// `selected` lit, a choice of one sets `select`'s value.
    /// Whether this is the list of the select `select`.
    pub(crate) fn is_for(&self, select: Entity) -> bool {
        self.source.as_ref().is_some_and(|source| source.select == select)
    }

    pub(crate) fn for_select(select: Entity, options: &[String], selected: usize) -> Self {
        Self {
            selected,
            source: Some(PaletteSource {
                options: options
                    .iter()
                    .enumerate()
                    .map(|(index, label)| PaletteCommand::new(index.to_string(), label.clone()))
                    .collect(),
                select,
            }),
            ..Self::default()
        }
    }
}

impl CommandPalette {
    /// How many rows the palette shows at once: [`PALETTE_ROWS`], or fewer while large text or a
    /// short window leave no room for all of them.
    pub fn visible_rows(&self) -> usize {
        self.cap
    }

    /// The index into [`CommandList`] of every command that matches the query, in the order the
    /// palette lists them.
    pub fn results(&self) -> &[usize] {
        &self.matches
    }

    /// Which of [`CommandPalette::results`] is lit: the one Enter runs.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The lit command, if any command matches.
    pub fn selected_command<'a>(&self, list: &'a CommandList) -> Option<&'a PaletteCommand> {
        self.matches
            .get(self.selected)
            .and_then(|&index| list.0.get(index))
    }
}

/// On a palette's field row: which palette it is in. Part of what [`crate::command_palette`]
/// spawns; a game never builds one.
#[derive(Component, Clone, Copy, Debug)]
pub struct PaletteInput(pub Entity);

/// One of a palette's reused rows: its palette and its place among the rows shown. Part of what
/// [`crate::command_palette`] spawns.
#[derive(Component, Clone, Copy, Debug)]
pub struct PaletteRow {
    /// The palette the row is in.
    pub palette: Entity,
    /// Its place among the rows shown, from the top.
    pub slot: usize,
}

/// On the line that says "No command matches". Part of what [`crate::command_palette`] spawns.
#[derive(Component, Clone, Copy, Debug)]
pub struct PaletteNote(pub Entity);

/// On the line that says which of how many is lit. Part of what [`crate::command_palette`] spawns.
#[derive(Component, Clone, Copy, Debug)]
pub struct PaletteCount(pub Entity);

/// Back out: Escape (the dialog's [`Cancel`]) closes the palette and gives focus back. A global
/// `Cancel` that reaches the palette is the palette's; the game needs no observer.
pub(crate) fn palette_on_cancel(mut cancel: On<Cancel>, mut commands: Commands) {
    cancel.propagate(false);
    commands.entity(cancel.entity).despawn();
}

/// A click on the scrim outside the card closes the palette. A click inside bubbles up with its
/// original target in the card, so it is not this.
pub(crate) fn palette_on_click(click: On<PointerClick>, mut commands: Commands) {
    if click.original_event_target() == click.entity {
        commands.entity(click.entity).despawn();
    }
}

/// A press anywhere in the palette (a row, the card's padding, the scrim) keeps focus in the field:
/// the request stops here instead of clearing focus at the window.
pub(crate) fn palette_on_acquire(
    mut acquire: On<AcquireFocus>,
    inputs: Query<(&PaletteInput, Entity)>,
    fields: Query<(Entity, &TextInputOf), With<TextInput>>,
    mut focus: ResMut<InputFocus>,
) {
    let palette = acquire.focused_entity;
    acquire.propagate(false);
    let Some((field, _)) = fields
        .iter()
        .find(|(_, of)| inputs.get(of.0).is_ok_and(|(input, _)| input.0 == palette))
    else {
        return;
    };
    if focus.get() != Some(field) {
        focus.set(field, FocusCause::Pressed);
    }
}

/// The pointer moving over a row makes it the lit one, as a menu follows the pointer. A pointer
/// that rests while the list changes under it moves nothing.
pub(crate) fn palette_on_move(
    moved: On<PointerMove>,
    rows: Query<&PaletteRow>,
    mut palettes: Query<&mut CommandPalette>,
) {
    let Ok(row) = rows.get(moved.entity) else {
        return;
    };
    let Ok(mut palette) = palettes.get_mut(row.palette) else {
        return;
    };
    let index = palette.top + row.slot;
    if index < palette.matches.len() && palette.selected != index {
        palette.selected = index;
    }
}

/// A click on an enabled row runs it. Bevy's `Button` sends no `Activate` for a disabled one.
pub(crate) fn palette_on_activate(
    activate: On<Activate>,
    rows: Query<&PaletteRow>,
    mut palettes: Query<&mut CommandPalette>,
    list: Res<CommandList>,
    mut history: ResMut<PaletteHistory>,
    mut commands: Commands,
) {
    let Ok(row) = rows.get(activate.entity) else {
        return;
    };
    let Ok(mut palette) = palettes.get_mut(row.palette) else {
        return;
    };
    let index = palette.top + row.slot;
    if index < palette.matches.len() {
        palette.selected = index;
    }
    run_selected(row.palette, &palette, &list, &mut history, &mut commands);
}

/// Runs the lit command if it is enabled: records it, despawns the palette (so focus goes back
/// first) and sends [`RunCommand`]. `false` when there is nothing to run.
fn run_selected(
    palette: Entity,
    state: &CommandPalette,
    list: &CommandList,
    history: &mut PaletteHistory,
    commands: &mut Commands,
) -> bool {
    if let Some(source) = &state.source {
        // A select's list: the choice is the option's index, sent as the row's own change unless
        // it is the option the row already has. The palette goes first so focus is back on the row.
        let Some(&index) = state.matches.get(state.selected) else {
            return false;
        };
        let select = source.select;
        commands.entity(palette).despawn();
        commands.queue(move |world: &mut World| {
            let now = world.get::<ChoiceValue>(select).map(|value| value.0);
            if now.is_some_and(|now| now != index) {
                world.trigger(ValueChange {
                    source: select,
                    value: index,
                    is_final: true,
                });
            }
        });
        return true;
    }
    let Some(command) = state.selected_command(list) else {
        return false;
    };
    if command.enabled.is_err() {
        return false;
    }
    let id = command.id.clone();
    history.record(&id);
    commands.entity(palette).despawn();
    commands.trigger(RunCommand { id });
    true
}

/// The keys of a palette, with focus in its field: Down and Up move the lit row (wrapping), PageDown
/// and PageUp a page (stopping at the ends), Enter runs it; on a pad the D-pad moves it (a held
/// direction repeats on real time) and South runs it. A held key repeats by the keyboard's own
/// repeat. Left, Right, Home, End and every character are the field's own.
///
/// The field takes Up and Down itself (a single line sends the caret to its start), so after a move
/// the caret is put back at the end: what the player types next is appended. The palette's field is
/// the only widget in its modal scope, so [`crate::navigate_by_input`] has nowhere to move focus to
/// and the arrows are the palette's alone. Runs in [`crate::GooeySystems::Navigate`].
#[allow(clippy::too_many_arguments)]
pub fn palette_by_input(
    mut keys: MessageReader<KeyboardInput>,
    gamepads: Query<&Gamepad>,
    time: Res<Time<Real>>,
    focus: Res<InputFocus>,
    list: Res<CommandList>,
    mut history: ResMut<PaletteHistory>,
    parents: Query<&ChildOf>,
    mut palettes: Query<&mut CommandPalette>,
    mut fields: Query<&mut EditableText, With<TextInput>>,
    mut feedback: MessageWriter<UiFeedback>,
    mut held: Local<(i8, f32)>,
    mut commands: Commands,
) {
    let pressed: Vec<(KeyCode, bool)> = keys
        .read()
        .filter(|key| key.state == ButtonState::Pressed)
        .map(|key| (key.key_code, key.repeat))
        .collect();
    let field = focus.get().filter(|&f| fields.contains(f));
    let palette = field.and_then(|f| {
        parents
            .iter_ancestors(f)
            .find(|&ancestor| palettes.contains(ancestor))
    });
    let (Some(field), Some(palette)) = (field, palette) else {
        *held = (0, 0.0);
        return;
    };
    let Ok(mut state) = palettes.get_mut(palette) else {
        return;
    };

    let down = |button| gamepads.iter().any(|g| g.pressed(button));
    let pad_direction = down(GamepadButton::DPadDown) as i8 - down(GamepadButton::DPadUp) as i8;
    let pad_steps = if pad_direction == 0 {
        *held = (0, 0.0);
        0
    } else if held.0 == pad_direction {
        let before = held.1;
        held.1 += time.delta_secs();
        repeat_steps(before, held.1, PAD_REPEAT_DELAY, PAD_REPEAT_RATE)
    } else {
        *held = (pad_direction, 0.0);
        1
    };

    let count = state.matches.len();
    let page = state.cap.max(1);
    let mut selected = state.selected;
    let mut typed_arrow = false;
    let mut run = gamepads
        .iter()
        .any(|g| g.just_pressed(GamepadButton::South));
    for (key, repeat) in pressed {
        match key {
            KeyCode::ArrowDown if count > 0 => {
                selected = (selected + 1) % count;
                typed_arrow = true;
            }
            KeyCode::ArrowUp if count > 0 => {
                selected = (selected + count - 1) % count;
                typed_arrow = true;
            }
            KeyCode::PageDown if count > 0 => selected = (selected + page).min(count - 1),
            KeyCode::PageUp if count > 0 => selected = selected.saturating_sub(page),
            KeyCode::Enter | KeyCode::NumpadEnter if !repeat => run = true,
            _ => {}
        }
    }
    if count > 0 && pad_steps > 0 {
        for _ in 0..pad_steps {
            selected = if pad_direction > 0 {
                (selected + 1) % count
            } else {
                (selected + count - 1) % count
            };
        }
    }
    if selected != state.selected {
        state.selected = selected;
        feedback.write(UiFeedback {
            entity: palette,
            kind: UiFeedbackKind::Focus,
        });
    }
    if typed_arrow && let Ok(mut text) = fields.get_mut(field) {
        text.queue_edit(TextEdit::TextEnd(false));
    }
    // Enter while an input method is composing commits the composition, not the command.
    let composing = fields.get(field).is_ok_and(|text| text.is_composing());
    if run && !composing && run_selected(palette, &state, &list, &mut history, &mut commands) {
        feedback.write(UiFeedback {
            entity: palette,
            kind: UiFeedbackKind::Activate,
        });
    }
}

/// The commands that match `query`, as indices into `list`, in the order a palette lists them:
/// recently run first (history order), then by score, best first, then by label. An empty query
/// matches everything and orders by recency, then by label.
///
/// With `ordered` (a select's options) ties and the empty query keep the list's own order, not the
/// alphabet's: the player knows that order.
fn rank(list: &[PaletteCommand], history: &[String], query: &str, ordered: bool) -> Vec<usize> {
    let labels: Vec<String> = list.iter().map(PaletteCommand::label).collect();
    let keys: Vec<String> = labels.iter().map(|label| label.to_lowercase()).collect();
    let mut scored: Vec<(usize, u32, usize)> = labels
        .iter()
        .enumerate()
        .filter_map(|(index, label)| {
            let score = fuzzy_score(query, label)?;
            let recent = history
                .iter()
                .position(|id| *id == list[index].id)
                .unwrap_or(usize::MAX);
            Some((index, score, recent))
        })
        .collect();
    let searching = !query.trim().is_empty();
    scored.sort_by(|a, b| {
        a.2.cmp(&b.2)
            .then_with(|| {
                if searching {
                    b.1.cmp(&a.1)
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then_with(|| {
                if ordered {
                    a.0.cmp(&b.0)
                } else {
                    keys[a.0].cmp(&keys[b.0])
                }
            })
    });
    scored.into_iter().map(|(index, ..)| index).collect()
}

/// What [`update_palette`] writes the count line from.
type CountParts = (
    &'static PaletteCount,
    &'static mut Node,
    &'static mut Text,
    &'static ComputedNode,
);

/// The count line is neither a row nor the note.
type CountOnly = (Without<PaletteRow>, Without<PaletteNote>);

/// What [`update_palette`] writes a row from, and measures the room by.
type RowParts = (
    Entity,
    &'static PaletteRow,
    &'static Children,
    &'static mut Node,
    &'static ComputedNode,
    Has<Selected>,
    Has<InteractionDisabled>,
);

/// The share of a count line's room kept free when working out how many rows fit, as a share of a
/// row's height: the count only shows when there are more matches than rows, and the rows that fit
/// must not depend on whether it does.
const COUNT_RESERVE: f32 = 0.75;

/// How far, in physical pixels, layout's rounding can move the measured height of a row.
const ROUNDING_PX: f32 = 1.5;

/// How many rows fit the window, in the units of one layout (physical px): what is left of the
/// window under the card's top and above the safe edge, once the card's own room (its padding, the
/// field and the count) is taken out, divided by a row's height. At least one and at most
/// [`PALETTE_ROWS`]: a window too small for one row shows one, cut.
///
/// Layout rounds a row's height by up to a pixel according to where it stands, and the room is
/// measured from the rows shown, so in a window where the answer is a whole number of rows to
/// within that pixel it would change with every row that scrolls in. `held`, the answer last time,
/// is kept while the room is within `ROUNDING_PX` of what it needs, so the card does not grow and
/// shrink a row as the lit row walks down the list.
fn rows_that_fit(
    window: Vec2,
    card_top: f32,
    card_height: f32,
    rows_height: f32,
    count_height: f32,
    row_height: f32,
    held: usize,
) -> usize {
    let chrome = card_height - rows_height - count_height;
    let room = window.y
        - window.x * SAFE_EDGE_PERCENT / 100.0
        - card_top
        - chrome
        - row_height * COUNT_RESERVE;
    let exact = room / row_height;
    let slack = ROUNDING_PX / row_height;
    let fit = exact.floor().max(1.0);
    let held_rows = held as f32;
    let fit = if held_rows > fit && exact + slack >= held_rows
        || held_rows < fit && exact - slack < held_rows + 1.0
    {
        held_rows
    } else {
        fit
    };
    (fit as usize).clamp(1, PALETTE_ROWS)
}

/// Ranks the commands against what the field holds and shows them: the lit row (the first match
/// whenever the query changes), the window of rows that keeps it in view (up to [`PALETTE_ROWS`],
/// fewer when text is large or the window short, so the card never leaves the window), each row's
/// words (the label, and the chord or the reason the command is off), the "No command matches" line
/// and the count. Writes only what differs, and runs only for a palette whose field, lit row, list,
/// history or size changed. Runs in [`crate::GooeySystems::Style`], before [`crate::apply_skin`],
/// the frame the change happens.
#[allow(clippy::too_many_arguments)]
pub fn update_palette(
    list: Res<CommandList>,
    history: Res<PaletteHistory>,
    inputs: Query<(Entity, Ref<TextInputValue>, &PaletteInput)>,
    mut palettes: Query<(Entity, &mut CommandPalette)>,
    parents: Query<&ChildOf>,
    layouts: Query<(Ref<ComputedNode>, &UiGlobalTransform)>,
    mut rows: Query<RowParts>,
    mut notes: Query<(&PaletteNote, &mut Node), Without<PaletteRow>>,
    mut counts: Query<CountParts, CountOnly>,
    mut texts: Query<&mut Text, (Without<PaletteCount>, Without<PaletteNote>)>,
    mut commands: Commands,
) {
    // The card is the field row's parent.
    let card_of = |input: Entity| parents.get(input).map(ChildOf::parent).ok();
    let resized = |entity: Entity, input: Entity| {
        layouts.get(entity).is_ok_and(|(node, _)| node.is_changed())
            || card_of(input)
                .and_then(|card| layouts.get(card).ok())
                .is_some_and(|(node, _)| node.is_changed())
    };
    let dirty: Vec<Entity> = palettes
        .iter_mut()
        .filter(|(entity, palette)| {
            palette.is_added()
                || palette.is_changed()
                || list.is_changed()
                || history.is_changed()
                || inputs.iter().any(|(input, value, of)| {
                    of.0 == *entity && (value.is_changed() || resized(*entity, input))
                })
        })
        .map(|(entity, _)| entity)
        .collect();
    for entity in dirty {
        let Some((input, query)) = inputs
            .iter()
            .find(|(_, _, of)| of.0 == entity)
            .map(|(input, value, _)| (input, value.0.trim().to_owned()))
        else {
            continue;
        };
        let Ok((_, mut state)) = palettes.get_mut(entity) else {
            continue;
        };
        // Our own writes below are not a change for the next run: only the player's (a lit row
        // moved, the field edited), the game's (the list) and the layout's are.
        let state = state.bypass_change_detection();
        // A select's list matches its own options and has no recents; the game's palette the
        // command list.
        let own = state.source.clone();
        let items: &[PaletteCommand] = own.as_ref().map_or(&list.0, |own| &own.options);
        let recents: &[String] = if own.is_some() { &[] } else { &history.0 };

        // How many rows the last layout had room for, measured from the rows it showed.
        let shown: Vec<f32> = rows
            .iter()
            .filter(|(_, part, _, node, ..)| {
                part.palette == entity && node.display != Display::None
            })
            .map(|(_, _, _, _, computed, ..)| computed.size().y)
            .collect();
        let count_height: f32 = counts
            .iter()
            .filter(|(part, node, ..)| part.0 == entity && node.display != Display::None)
            .map(|(.., computed)| computed.size().y)
            .sum();
        // A row's height is the mean of the rows shown: layout rounds each to whole pixels, by
        // where it stands, so the first row alone says 19 where the mean says 18.4.
        let row_height = shown.iter().sum::<f32>() / shown.len().max(1) as f32;
        if let (true, Some(card), Some(window)) = (
            row_height > 0.0,
            card_of(input).and_then(|card| layouts.get(card).ok()),
            layouts.get(entity).ok(),
        ) && window.0.size().min_element() > 0.0
            && card.0.size().y > 0.0
        {
            let card_top = card.1.translation.y - card.0.size().y / 2.0;
            state.cap = rows_that_fit(
                window.0.size(),
                card_top,
                card.0.size().y,
                shown.iter().sum(),
                count_height,
                row_height,
                state.cap,
            );
        }

        let matches = rank(items, recents, &query, own.is_some());
        if query != state.query {
            state.query = query;
            state.selected = 0;
            state.top = 0;
        }
        state.matches = matches;
        let count = state.matches.len();
        let cap = state.cap;
        state.selected = state.selected.min(count.saturating_sub(1));
        if state.selected < state.top {
            state.top = state.selected;
        } else if state.selected >= state.top + cap {
            state.top = state.selected + 1 - cap;
        }
        state.top = state.top.min(count.saturating_sub(cap));

        for (row, part, children, mut node, _, lit, disabled) in &mut rows {
            if part.palette != entity {
                continue;
            }
            let index = state.top + part.slot;
            let command = state
                .matches
                .get(index)
                .and_then(|&i| items.get(i))
                .filter(|_| part.slot < cap);
            let display = if command.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
            let (title, hint) = command.map_or((String::new(), String::new()), |command| {
                let hint = match (&command.enabled, &command.shortcut) {
                    (Err(reason), _) => reason.clone(),
                    (Ok(()), Some(shortcut)) => shortcut.label(),
                    (Ok(()), None) => String::new(),
                };
                (command.label(), hint)
            });
            for (child, words) in children.iter().zip([title, hint]) {
                if let Ok(mut text) = texts.get_mut(child)
                    && text.0 != words
                {
                    text.0 = words;
                }
            }
            let want_lit = command.is_some() && index == state.selected;
            if want_lit && !lit {
                commands.entity(row).insert(Selected);
            } else if !want_lit && lit {
                commands.entity(row).remove::<Selected>();
            }
            let want_off = command.is_some_and(|c| c.enabled.is_err());
            if want_off && !disabled {
                commands.entity(row).insert(InteractionDisabled);
            } else if !want_off && disabled {
                commands.entity(row).remove::<InteractionDisabled>();
            }
        }
        for (note, mut node) in &mut notes {
            if note.0 != entity {
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
        for (part, mut node, mut text, _) in &mut counts {
            if part.0 != entity {
                continue;
            }
            let display = if count > cap {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
            let words = if count > cap {
                format!("{} of {count}", state.selected + 1)
            } else {
                String::new()
            };
            if text.0 != words {
                text.0 = words;
            }
        }
    }
}
