//! The table: thousands of rows in a handful of entities. [`crate::table`] spawns a header and a
//! scrolling viewport; this module keeps a **pool** of row entities (the rows in view and one
//! more) bound to the rows of a [`RowSource`] that the scroll has put in view ([`update_tables`]),
//! sorts by a header ([`SortBy`]), keeps the player's place when the rows change under it, and
//! moves a cursor and a selection by pointer, key and pad ([`table_by_input`]). Its look is
//! [`crate::apply_skin`]'s, like every widget's.

use std::any::Any;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;

use bevy::ecs::relationship::RelatedSpawner;
use bevy::ecs::spawn::SpawnWith;
use bevy::input::ButtonInput;
use bevy::input::ButtonState;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusedInput, InputFocus, InputFocusVisible};
use bevy::picking::cursor::EntityCursor;
use bevy::picking::events::{
    PointerCancel, PointerClick, PointerDrag, PointerDragEnd, PointerDragStart, PointerPress,
    PointerRelease,
};
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{InteractionDisabled, Pressed, Selected};
use bevy::ui_widgets::{Activate, ScrollArea, ValueChange, observe};
use bevy::window::SystemCursorIcon;

use crate::clicks::DoublePress;
use crate::list::ListView;
use crate::model::{
    MIN_TARGET_PX, PAD_REPEAT_DELAY, PAD_REPEAT_RATE, SPLIT_KEY_STEP_PX, TABLE_LEAST_COLUMN,
    TableStep, column_resize, natural_cmp, repeat_steps, scroll_to_reveal, table_anchor,
    table_step, table_window, tree_indent, type_ahead,
};
use crate::motion::MotionSettings;
use crate::nav::show_focus;
use crate::scroll_glide::{ScrollGlide, glide_spring, step_glide};
use crate::skin::{GooeySkin, Role, SkinScope};
use crate::tree::{Lead, Tree, TreeView, bind_lead, spawn_lead};
use crate::type_select::{TypedPrefix, plain, starts_with_ignore_case, typed_characters};
use crate::widgets::{ONE_LINE, list_rail};

/// How many rows tall a table stands unless it carries its own [`TableRows`].
const DEFAULT_ROWS: f32 = 8.5;

/// What one change to a [`RowSource`] did to the rows the table shows, as the source reports it in
/// [`RowSource::change_since`]. Every case leaves the rows the table already holds in the order they
/// had, so the table keeps its sort, its selection, its cursor and its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RowChange {
    /// Nothing: no row's key, text or place is other than it was.
    Unchanged,
    /// The text of some cells changed (a relabel); the rows, their keys and their order did not.
    Texts,
    /// `count` rows were put in at display row `row`, the rows before it unmoved; the ones from
    /// `row` on moved down by `count`. A `count` of 0 is a change to a row that is not shown (a
    /// child added to a closed node).
    Inserted {
        /// The display row of the first row added.
        row: usize,
        /// How many rows were added.
        count: usize,
    },
}

/// The rows a [`crate::table`] shows: the one trait a game implements. The table asks for what is
/// in view, so a source of ten thousand rows costs what ten cost; it holds no copy.
///
/// Only [`len`](RowSource::len) and [`cell`](RowSource::cell) are required. A row's
/// [`key`](RowSource::key) is its identity: the selection, the cursor and the player's place in
/// the rows are kept by key, so they stay with a row when the rows are sorted or others come and
/// go. It defaults to the row's index, which is right for a source that only grows at the end; a
/// source that inserts or removes rows gives each a key of its own (an id).
///
/// ```
/// # use std::borrow::Cow;
/// # use bevy_gooey::*;
/// struct Items(Vec<(u64, String, u32)>);
///
/// impl RowSource for Items {
///     fn len(&self) -> usize {
///         self.0.len()
///     }
///     fn cell(&self, row: usize, column: usize) -> Cow<'_, str> {
///         match column {
///             0 => Cow::Borrowed(&self.0[row].1),
///             _ => Cow::Owned(self.0[row].2.to_string()),
///         }
///     }
///     fn key(&self, row: usize) -> u64 {
///         self.0[row].0
///     }
/// }
/// ```
pub trait RowSource: Any + Send + Sync {
    /// How many rows there are.
    fn len(&self) -> usize;

    /// Whether there are none.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The text of one cell. `row` is below [`len`](RowSource::len) and `column` below the
    /// number of columns the table was made with.
    fn cell(&self, row: usize, column: usize) -> Cow<'_, str>;

    /// The row's identity. Two rows never share one; the default is the index.
    fn key(&self, row: usize) -> u64 {
        row as u64
    }

    /// How two rows order by `column`, for a click on that column's header. The default is the
    /// cells' text in [`natural_cmp`] order (numbers by value, words without regard to case);
    /// override it for a column that is not text (a date, a size with a unit).
    fn compare(&self, column: usize, a: usize, b: usize) -> Ordering {
        natural_cmp(&self.cell(a, column), &self.cell(b, column))
    }

    /// A number that changes whenever the rows do, if the source can say what changed
    /// ([`change_since`](RowSource::change_since)); 0, the default, when it cannot. A source that
    /// answers draws its numbers from one counter for all of them, so two sources never agree on
    /// one by accident.
    fn revision(&self) -> u64 {
        0
    }

    /// What changed since the table last looked, when it looked at [`revision`](RowSource::revision)
    /// `seen`: `None` when the source cannot say, or more than one change happened, and the table
    /// rebuilds its order as it always did. A source that answers lets the table touch only what
    /// changed, so one row relabelled or appended costs one row, not ten thousand.
    fn change_since(&self, seen: u64) -> Option<RowChange> {
        let _ = seen;
        None
    }
}

impl RowSource for Vec<Vec<String>> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn cell(&self, row: usize, column: usize) -> Cow<'_, str> {
        Cow::Borrowed(self[row].get(column).map_or("", String::as_str))
    }
}

/// One column of a [`crate::table`].
///
/// ```
/// # use bevy_gooey::Column;
/// let name = Column::new("Name").width(14.0);
/// let notes = Column::new("Notes").unsortable();
/// assert_eq!((name.width, name.sortable, notes.sortable), (14.0, true, false));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    /// The header's text.
    pub title: String,
    /// The column's width in body text sizes (an `em`, so it grows with the player's text size and
    /// keeps its characters per cell). The last column also takes whatever room is left.
    pub width: f32,
    /// Whether a click on the header sorts by the column.
    pub sortable: bool,
}

impl Column {
    /// A sortable column eight text sizes wide.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 8.0,
            sortable: true,
        }
    }

    /// This column `width` text sizes wide.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// This column, not sortable: its header sorts nothing and sends no [`SortBy`].
    pub fn unsortable(mut self) -> Self {
        self.sortable = false;
        self
    }
}

/// A [`crate::table`]'s root: its columns and its rows. Change the rows through it
/// ([`source_mut`](Table::source_mut), [`set_source`](Table::set_source)): the table sees a
/// changed component, sorts the new rows the way they were sorted and keeps the row the player was
/// looking at where it was.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn spawn(mut commands: Commands) {
/// commands.spawn(table(
///     [Column::new("Name"), Column::new("Level").width(5.0)],
///     vec![vec!["Ash".to_string(), "3".to_string()]],
/// ));
/// # }
/// fn add_row(mut tables: Query<&mut Table>) {
///     for mut table in &mut tables {
///         if let Some(rows) = table.source_mut::<Vec<Vec<String>>>() {
///             rows.push(vec!["Birch".to_string(), "7".to_string()]);
///         }
///     }
/// }
/// ```
#[derive(Component)]
#[require(
    TableSort,
    TableSelection,
    SelectMode,
    TableCursor,
    TableWidths,
    TableRows,
    TableCache
)]
pub struct Table {
    source: Box<dyn RowSource>,
    columns: Vec<Column>,
}

impl Table {
    /// A table of `columns` over `rows`.
    pub fn new(columns: impl IntoIterator<Item = Column>, rows: impl RowSource) -> Self {
        Self {
            source: Box::new(rows),
            columns: columns.into_iter().collect(),
        }
    }

    /// The columns, in order.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// How many rows there are.
    pub fn len(&self) -> usize {
        self.source.len()
    }

    /// Whether there are no rows.
    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    /// The rows, if they are a `T`.
    pub fn source<T: RowSource>(&self) -> Option<&T> {
        (self.source.as_ref() as &dyn Any).downcast_ref()
    }

    /// The rows to change, if they are a `T`. Taking this marks the table changed.
    pub fn source_mut<T: RowSource>(&mut self) -> Option<&mut T> {
        (self.source.as_mut() as &mut dyn Any).downcast_mut()
    }

    /// Replaces the rows.
    pub fn set_source(&mut self, rows: impl RowSource) {
        self.source = Box::new(rows);
    }
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Table")
            .field("columns", &self.columns)
            .field("rows", &self.source.len())
            .finish()
    }
}

/// How many rows tall a [`crate::table`] stands; a fraction cuts the last row visible, the cut-off
/// that tells the player there is more. The default is 8.5. Part of what the table requires, so a
/// game sets its own in the same tuple: `(table(..), TableRows(12.0))`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableRows(pub f32);

impl Default for TableRows {
    fn default() -> Self {
        Self(DEFAULT_ROWS)
    }
}

/// How a [`crate::table`] is sorted: the column and which way. It is the widget's state: the
/// player's click on a header sets it (and sends [`SortBy`]); a game sets it to sort from code or
/// to start a table sorted, and the rows follow in the same frame with no event.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableSort {
    /// The column sorted by, none for the source's own order.
    pub column: Option<usize>,
    /// Largest first.
    pub descending: bool,
}

/// How many rows a [`crate::table`]'s player may select.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SelectMode {
    /// None: the cursor moves and a row can be opened, but nothing is selected.
    None,
    /// One. A click, or an arrow, selects the row and drops the last.
    #[default]
    Single,
    /// Any number. Ctrl and a click or Space toggles a row, Shift and a click or an arrow
    /// extends the selection from where it began, Ctrl and A selects every row.
    Multi,
}

/// The rows a [`crate::table`] has selected, by [`RowSource::key`]: the widget's state. A game
/// sets it (no event is sent for what the game does itself) or reads it; the player's change sends
/// `ValueChange<TableSelection>` with the whole selection.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn on_select(change: On<ValueChange<TableSelection>>) {
///     for key in change.value.iter() {
///         info!("row {key} is selected");
///     }
/// }
/// ```
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableSelection(pub BTreeSet<u64>);

impl TableSelection {
    /// A selection of the one row `key`.
    pub fn of(key: u64) -> Self {
        Self(BTreeSet::from([key]))
    }

    /// Whether the row `key` is selected.
    pub fn contains(&self, key: u64) -> bool {
        self.0.contains(&key)
    }

    /// The selected keys, smallest first.
    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        self.0.iter().copied()
    }

    /// How many rows are selected.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The row of a [`crate::table`] that keys act on, by [`RowSource::key`]: the row ringed while the
/// table has focus. Arrows and Page keys move it, Enter opens it ([`RowActivated`]), Space
/// selects it. A game sets it to move the ring from code.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableCursor(pub Option<u64>);

/// The widths of a [`crate::table`]'s columns in body text sizes, as the player has dragged them:
/// the state a game saves with the player's other settings. A column with no entry has its
/// [`Column::width`].
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableWidths(pub Vec<f32>);

/// The player picked a sortable column's header: the table is already sorted that way
/// ([`TableSort`] holds it), so this is for a game that keeps its own order or a note of the
/// choice. The first click on a column sorts it ascending and each next one turns it round. A
/// sort the game sets itself sends nothing.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn remember(sort: On<SortBy>) {
///     info!("sorted by column {}, descending: {}", sort.column, sort.descending);
/// }
/// ```
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct SortBy {
    /// The table.
    #[event_target]
    pub entity: Entity,
    /// The column sorted by.
    pub column: usize,
    /// Largest first.
    pub descending: bool,
}

/// The player opened a row of a [`crate::table`]: Enter or the pad's South button on the cursor
/// row, or a double click.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct RowActivated {
    /// The table.
    #[event_target]
    pub entity: Entity,
    /// The row's [`RowSource::key`].
    pub key: u64,
}

/// The entities a table is built of, for the systems that lay them out. Internal: part of what
/// [`crate::table`] spawns.
#[derive(Component)]
pub struct TableParts {
    view: Entity,
    rows: Entity,
    headers: Vec<HeaderParts>,
}

/// One column's header and what hangs off it.
pub(crate) struct HeaderParts {
    pub(crate) cell: Entity,
    pub(crate) edge: Option<Entity>,
    pub(crate) up: Entity,
    pub(crate) down: Entity,
}

/// A header cell: which table and column it heads.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct TableHeaderCell {
    pub(crate) table: Entity,
    pub(crate) column: usize,
}

/// A column's resize edge.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct TableEdge {
    table: Entity,
    column: usize,
}

/// Where an edge was when it was picked up, in body text sizes.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct EdgeGrab(f32);

/// One row of a table's pool, the entity a row of text is drawn on: which table, which display
/// row it shows now, that row's key and whether it is the cursor's. A table has only as many as it
/// shows, and each is rebound as the table scrolls, so what an entity shows changes: ask this
/// component which row it is (for a tooltip or a context menu on the row under the pointer), and
/// never keep the entity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableRow {
    /// The table.
    pub table: Entity,
    /// The display row shown: the row's place after sorting.
    pub index: usize,
    /// The row's [`RowSource::key`].
    pub key: u64,
    /// Whether it is the table's cursor row.
    pub cursor: bool,
}

struct PoolRow {
    row: Entity,
    /// Each column's cell box and the text in it.
    cells: Vec<(Entity, Entity)>,
    /// A tree's chevron and indent, in the first cell.
    lead: Option<Lead>,
}

/// What a table has worked out: the display order, the pool of row entities, the scroll it last
/// drew. Internal; a game reads [`TableSort`], [`TableSelection`] and [`TableCursor`].
#[derive(Component, Default)]
pub struct TableCache {
    built: bool,
    /// Display row to source row.
    order: Vec<usize>,
    /// Display row to key.
    pub(crate) keys: Vec<u64>,
    pool: Vec<PoolRow>,
    /// The display row at the top of the pool.
    first: usize,
    /// Where a range selection from the keyboard or Shift began.
    pub(crate) anchor: Option<u64>,
    /// The cursor moved by a key: scroll it into view.
    follow: bool,
    /// The body text size the widths were laid out at.
    text_px: f32,
    pitch: f32,
    /// The source's [`RowSource::revision`] this cache was last made from.
    seen: u64,
}

impl TableCache {
    pub(crate) fn index_of(&self, key: Option<u64>) -> Option<usize> {
        let key = key?;
        self.keys.iter().position(|&k| k == key)
    }

    /// Whether a press of Up (or Down) is the table's to take: it has a row to move to that way.
    pub(crate) fn takes(&self, cursor: Option<u64>, up: bool) -> bool {
        let Some(last) = self.keys.len().checked_sub(1) else {
            return false;
        };
        match self.index_of(cursor) {
            None => true,
            Some(i) if up => i > 0,
            Some(i) => i < last,
        }
    }
}

/// How a pick moves the selection.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pick {
    /// The cursor only.
    Move,
    /// This row alone.
    Only,
    /// This row in or out.
    Toggle,
    /// Everything from the anchor to this row.
    Range,
}

pub(crate) type Chosen<'a, 'w> = (
    &'a SelectMode,
    &'a mut Mut<'w, TableSelection>,
    &'a mut Mut<'w, TableCursor>,
    &'a mut Mut<'w, TableCache>,
);

/// Puts the cursor on display row `index` and the selection where `how` says; sends the one
/// `ValueChange` when the selection is different.
pub(crate) fn pick(
    entity: Entity,
    index: usize,
    how: Pick,
    follow: bool,
    (mode, selection, cursor, cache): Chosen<'_, '_>,
    commands: &mut Commands,
) {
    let Some(&key) = cache.keys.get(index) else {
        return;
    };
    let was = cache.index_of(cursor.0);
    if cursor.0 != Some(key) {
        cursor.0 = Some(key);
    }
    cache.follow = follow;
    let mut next = selection.0.clone();
    match (*mode, how) {
        (SelectMode::None, _) | (_, Pick::Move) => {}
        (SelectMode::Single, _) | (SelectMode::Multi, Pick::Only) => {
            next = BTreeSet::from([key]);
            cache.anchor = Some(key);
        }
        (SelectMode::Multi, Pick::Toggle) => {
            if !next.remove(&key) {
                next.insert(key);
            }
            cache.anchor = Some(key);
        }
        (SelectMode::Multi, Pick::Range) => {
            let from = cache.index_of(cache.anchor).or(was).unwrap_or(index);
            let (lo, hi) = (from.min(index), from.max(index));
            next = cache.keys[lo..=hi].iter().copied().collect();
            let begun = cache.keys[from];
            cache.anchor.get_or_insert(begun);
        }
    }
    if next != selection.0 {
        selection.0 = next;
        commands.trigger(ValueChange {
            source: entity,
            value: selection.clone(),
            is_final: true,
        });
    }
}

/// A click on a row: the cursor goes there and the selection by the modifiers held (Ctrl toggles,
/// Shift takes the range in a [`SelectMode::Multi`] table); a double press opens the row
/// (`row_on_double_press`).
pub(crate) fn row_on_click(
    mut click: On<PointerClick>,
    rows: Query<&TableRow>,
    keys: Res<ButtonInput<KeyCode>>,
    mut tables: Query<
        (
            &SelectMode,
            &mut TableSelection,
            &mut TableCursor,
            &mut TableCache,
        ),
        Without<InteractionDisabled>,
    >,
    mut commands: Commands,
) {
    click.propagate(false);
    if click.button != PointerButton::Primary {
        return;
    }
    let Ok(row) = rows.get(click.entity) else {
        return;
    };
    let Ok((mode, mut selection, mut cursor, mut cache)) = tables.get_mut(row.table) else {
        return;
    };
    let held = |a, b| keys.pressed(a) || keys.pressed(b);
    let how = if held(KeyCode::ControlLeft, KeyCode::ControlRight) {
        Pick::Toggle
    } else if held(KeyCode::ShiftLeft, KeyCode::ShiftRight) {
        Pick::Range
    } else {
        Pick::Only
    };
    pick(
        row.table,
        row.index,
        how,
        false,
        (mode, &mut selection, &mut cursor, &mut cache),
        &mut commands,
    );
}

/// A double press on a row opens it; its first click already moved the cursor and the selection.
pub(crate) fn row_on_double_press(
    mut double: On<DoublePress>,
    rows: Query<&TableRow>,
    tables: Query<(), (With<Table>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    double.propagate(false);
    let Ok(row) = rows.get(double.entity) else {
        return;
    };
    if tables.contains(row.table) {
        commands.trigger(RowActivated {
            entity: row.table,
            key: row.key,
        });
    }
}

/// A click on a sortable header sorts by its column, or turns the sort round.
pub(crate) fn header_on_activate(
    activate: On<Activate>,
    headers: Query<&TableHeaderCell>,
    mut tables: Query<(&Table, &mut TableSort), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok(header) = headers.get(activate.entity) else {
        return;
    };
    let Ok((table, mut sort)) = tables.get_mut(header.table) else {
        return;
    };
    if !table
        .columns
        .get(header.column)
        .is_some_and(|column| column.sortable)
    {
        return;
    }
    let descending = sort.column == Some(header.column) && !sort.descending;
    *sort = TableSort {
        column: Some(header.column),
        descending,
    };
    commands.trigger(SortBy {
        entity: header.table,
        column: header.column,
        descending,
    });
}

/// Ctrl and Left or Right on a focused header move the column's right edge, 16 px a press: the
/// way to size a column without a drag (WCAG 2.5.7).
pub(crate) fn header_on_key_input(
    mut input: On<FocusedInput<KeyboardInput>>,
    headers: Query<&TableHeaderCell>,
    keys: Res<ButtonInput<KeyCode>>,
    skin: Res<GooeySkin>,
    mut tables: Query<(&Table, &mut TableWidths)>,
) {
    if input.input.state != ButtonState::Pressed
        || !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        return;
    }
    let sign = match input.input.key_code {
        KeyCode::ArrowLeft => -1.0,
        KeyCode::ArrowRight => 1.0,
        _ => return,
    };
    input.propagate(false);
    let Ok(header) = headers.get(input.focused_entity) else {
        return;
    };
    let Ok((table, mut widths)) = tables.get_mut(header.table) else {
        return;
    };
    let text_px = (skin.text_size * skin.text_scale).max(1.0);
    let start = width_of(table, &widths, header.column);
    resize(
        &mut widths,
        table,
        header.column,
        start,
        sign * SPLIT_KEY_STEP_PX,
        text_px,
    );
}

/// The width of `column` in body text sizes: the player's, or the column's own.
fn width_of(table: &Table, widths: &TableWidths, column: usize) -> f32 {
    widths
        .0
        .get(column)
        .copied()
        .or_else(|| table.columns.get(column).map(|c| c.width))
        .unwrap_or(8.0)
        .max(TABLE_LEAST_COLUMN)
}

/// Moves `column`'s edge `by` logical pixels from `start` (text sizes), never under the least.
fn resize(
    widths: &mut Mut<TableWidths>,
    table: &Table,
    column: usize,
    start: f32,
    by: f32,
    text_px: f32,
) {
    let to = column_resize(start * text_px, by, TABLE_LEAST_COLUMN * text_px) / text_px;
    if widths.0.len() < table.columns.len() {
        let missing = table.columns.len() - widths.0.len();
        let from = widths.0.len();
        widths
            .0
            .extend((0..missing).map(|i| table.columns[from + i].width));
    }
    if let Some(slot) = widths.0.get_mut(column)
        && (*slot - to).abs() > f32::EPSILON
    {
        *slot = to;
    }
}

/// A press on an edge is not a press on its header.
pub(crate) fn edge_on_press(mut press: On<PointerPress>) {
    press.propagate(false);
}

/// Nor is the release.
pub(crate) fn edge_on_release(mut release: On<PointerRelease>) {
    release.propagate(false);
}

/// Nor the click Bevy sends to the edge a drag ends over.
pub(crate) fn edge_on_click(mut click: On<PointerClick>) {
    click.propagate(false);
}

/// Picking an edge up: it is held and remembers the width it began at.
pub(crate) fn edge_on_drag_start(
    mut start: On<PointerDragStart>,
    edges: Query<&TableEdge>,
    tables: Query<(&Table, &TableWidths)>,
    mut commands: Commands,
) {
    start.propagate(false);
    let Ok(edge) = edges.get(start.entity) else {
        return;
    };
    let Ok((table, widths)) = tables.get(edge.table) else {
        return;
    };
    commands
        .entity(start.entity)
        .insert((Pressed, EdgeGrab(width_of(table, widths, edge.column))));
}

/// The edge follows the pointer: the width it began at plus the distance, never under the least.
pub(crate) fn edge_on_drag(
    mut drag: On<PointerDrag>,
    edges: Query<(&TableEdge, &EdgeGrab)>,
    skin: Res<GooeySkin>,
    ui_scale: Option<Res<UiScale>>,
    mut tables: Query<(&Table, &mut TableWidths)>,
) {
    drag.propagate(false);
    let Ok((edge, grab)) = edges.get(drag.entity) else {
        return;
    };
    let Ok((table, mut widths)) = tables.get_mut(edge.table) else {
        return;
    };
    let text_px = (skin.text_size * skin.text_scale).max(1.0);
    let along = drag.distance.x / ui_scale.map_or(1.0, |s| s.0);
    resize(&mut widths, table, edge.column, grab.0, along, text_px);
}

/// Letting go of an edge.
pub(crate) fn edge_on_drag_end(mut end: On<PointerDragEnd>, mut commands: Commands) {
    end.propagate(false);
    commands.entity(end.entity).remove::<(Pressed, EdgeGrab)>();
}

/// A cancelled pointer lets the edge go where it is.
pub(crate) fn edge_on_cancel(mut cancel: On<PointerCancel>, mut commands: Commands) {
    cancel.propagate(false);
    commands
        .entity(cancel.entity)
        .remove::<(Pressed, EdgeGrab)>();
}

/// The bundle [`crate::table`] returns; `outlined` is a [`crate::tree`]'s: no header, the rows'
/// first column indented and led by a disclosure chevron.
pub(crate) fn table_bundle(
    columns: impl IntoIterator<Item = Column>,
    rows: impl RowSource,
    outlined: bool,
) -> impl Bundle {
    let table = Table::new(columns, rows);
    let titles: Vec<(String, bool)> = table
        .columns
        .iter()
        .map(|c| (c.title.clone(), c.sortable))
        .collect();
    (
        Node {
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip_x(),
            ..default()
        },
        Role::Table,
        table,
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            let root = parent.target_entity();
            let strip = (!outlined).then(|| {
                parent
                    .spawn((
                        Node {
                            flex_shrink: 0.0,
                            ..default()
                        },
                        Role::TableHeader,
                    ))
                    .id()
            });
            let last = titles.len().saturating_sub(1);
            let mut headers = Vec::new();
            for (column, (title, _)) in titles.iter().enumerate() {
                let Some(strip) = strip else {
                    break;
                };
                let (cell, up, down) = {
                    let world = parent.world_mut();
                    let cell = world
                        .spawn((
                            Node {
                                align_items: AlignItems::Center,
                                flex_shrink: 0.0,
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            bevy::ui_widgets::Button,
                            Role::TableHeaderCell,
                            TableHeaderCell {
                                table: root,
                                column,
                            },
                            Hovered::default(),
                            TabIndex(0),
                            AutoDirectionalNavigation::default(),
                            observe(header_on_activate),
                            observe(header_on_key_input),
                            ChildOf(strip),
                        ))
                        .id();
                    world.spawn((
                        Text::new(title.clone()),
                        ONE_LINE,
                        Role::ButtonText,
                        ChildOf(cell),
                    ));
                    let mark = |role, shift| {
                        (
                            Node {
                                display: Display::None,
                                ..default()
                            },
                            UiTransform {
                                translation: Val2::percent(0.0, shift),
                                rotation: Rot2::degrees(45.0),
                                ..default()
                            },
                            role,
                            Pickable::IGNORE,
                            ChildOf(cell),
                        )
                    };
                    let up = world
                        .spawn(mark(Role::TableSortUp, crate::widgets::CHEVRON_INK_SHIFT))
                        .id();
                    let down = world
                        .spawn(mark(
                            Role::TableSortDown,
                            -crate::widgets::CHEVRON_INK_SHIFT,
                        ))
                        .id();
                    (cell, up, down)
                };
                // The last column's edge would only size the room the table does not use.
                let edge = (column < last).then(|| {
                    parent
                        .world_mut()
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(0.0),
                                bottom: Val::Px(0.0),
                                width: Val::Px(MIN_TARGET_PX),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            Role::TableEdge,
                            TableEdge {
                                table: root,
                                column,
                            },
                            Hovered::default(),
                            EntityCursor::System(SystemCursorIcon::ColResize),
                            (
                                observe(edge_on_press),
                                observe(edge_on_release),
                                observe(edge_on_click),
                                observe(edge_on_drag_start),
                                observe(edge_on_drag),
                                observe(edge_on_drag_end),
                                observe(edge_on_cancel),
                            ),
                            ChildOf(strip),
                            children![(
                                Node {
                                    width: Val::Px(3.0),
                                    height: Val::Px(20.0),
                                    ..default()
                                },
                                Role::SplitGrip,
                                Pickable::IGNORE,
                            )],
                        ))
                        .id()
                });
                headers.push(HeaderParts {
                    cell,
                    edge,
                    up,
                    down,
                });
            }
            let body = parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    Role::List,
                ))
                .id();
            let (view, rows) = {
                let world = parent.world_mut();
                let view = world
                    .spawn((
                        Node {
                            display: Display::Block,
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        Role::TableView,
                        ListView { rows: DEFAULT_ROWS },
                        ScrollArea,
                        ChildOf(body),
                    ))
                    .id();
                let rows = world
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.0,
                            ..default()
                        },
                        Role::TableRows,
                        ChildOf(view),
                    ))
                    .id();
                world.spawn((list_rail(view), ChildOf(body)));
                (view, rows)
            };
            parent.world_mut().entity_mut(root).insert(TableParts {
                view,
                rows,
                headers,
            });
        })),
    )
}

/// What `update_tables` reads of a table.
type Laid = (
    Entity,
    Ref<'static, Table>,
    Ref<'static, TableSort>,
    Ref<'static, TableWidths>,
    &'static TableRows,
    Mut<'static, TableSelection>,
    Mut<'static, TableCursor>,
    Mut<'static, TableCache>,
    &'static TableParts,
);

/// The skin a table is drawn in: the nearest [`SkinScope`] at or above it, else the resource.
fn skin_of<'a>(
    entity: Entity,
    global: &'a GooeySkin,
    scopes: &'a Query<&SkinScope>,
    parents: &Query<&ChildOf>,
) -> &'a GooeySkin {
    if scopes.is_empty() {
        return global;
    }
    std::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .find_map(|e| scopes.get(e).ok())
        .map_or(global, |scope| &scope.0)
}

/// A cell's box: `width` logical pixels, the last column taking the room left over. Its least
/// width is nothing (`Overflow::clip` does not turn off a flex item's automatic minimum, the width
/// of its content), so a long text is cut at the column and never widens it.
fn cell_node(width: f32, last: bool) -> Node {
    Node {
        flex_basis: Val::Px(width),
        flex_grow: if last { 1.0 } else { 0.0 },
        flex_shrink: 0.0,
        min_width: Val::Px(0.0),
        overflow: Overflow::clip(),
        align_items: AlignItems::Center,
        ..default()
    }
}

/// Keeps every table drawn from its rows: the order (rebuilt when the rows or the sort change),
/// the rows in view bound to the pool's entities, the pool's size, the room above and below the
/// pool that stands for the rows not drawn, the columns' widths and the sort mark, and the scroll
/// (the anchor row kept where it was when the rows change, the cursor row kept in sight when a key
/// moved it). A table at rest costs a comparison. Runs in [`crate::GooeySystems::Style`], before
/// [`crate::apply_skin`].
#[allow(clippy::too_many_arguments)]
pub fn update_tables(
    skin: Res<GooeySkin>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    mut tables: Query<Laid>,
    mut nodes: Query<&mut Node>,
    mut scrolls: Query<(&mut ScrollPosition, &mut ListView, &mut ScrollGlide)>,
    settings: Res<MotionSettings>,
    time: Option<Res<Time<Real>>>,
    mut texts: Query<&mut Text>,
    mut rows: Query<(&mut TableRow, Has<Selected>)>,
    mut commands: Commands,
) {
    for (entity, table, sort, widths, rows_tall, mut selection, mut cursor, mut cache, parts) in
        &mut tables
    {
        let skin = skin_of(entity, &skin, &scopes, &parents);
        let text_px = (skin.text_size * skin.text_scale).max(1.0);
        let pitch = skin.table_pitch();
        let len = table.source.len();
        let columns = table.columns.len();
        let tree = table.source::<Tree>();
        let indent = tree_indent(text_px);
        let cache = &mut *cache;
        let fresh = !cache.built;
        // A source that can say what changed since the table last looked (a `Tree` relabelled or
        // appended to) is patched, not rebuilt: its keys and order are kept and only the rows the
        // change added are read. Anything else (a sort, whose order a relabel can change; a
        // removal, the first frame, a source with no answer, a count that does not add up, a row
        // put at or above the one on top, whose place the rebuild keeps) takes the rebuild.
        let sorted = sort.column.filter(|&c| c < columns).is_some();
        let patch = if fresh || sort.is_changed() || !table.is_changed() {
            None
        } else {
            match table.source.change_since(cache.seen) {
                Some(RowChange::Unchanged) => Some((0, 0)),
                Some(RowChange::Texts) if !sorted => Some((0, 0)),
                Some(RowChange::Inserted { row, count })
                    if count == 0 || (!sorted && row > cache.first && row <= cache.keys.len()) =>
                {
                    Some((row, count))
                }
                _ => None,
            }
            .filter(|&(_, count)| cache.keys.len() + count == len)
        };
        let rebuilt = fresh || sort.is_changed() || (table.is_changed() && patch.is_none());
        let Ok((mut scroll, mut view, mut glide)) = scrolls.get_mut(parts.view) else {
            continue;
        };
        if view.rows != rows_tall.0 {
            view.rows = rows_tall.0;
        }
        let tall = rows_tall.0.max(0.0);
        let view_h = (tall * pitch).min(len as f32 * pitch);
        let max = (len as f32 * pitch - view_h).max(0.0);
        let mut y = scroll.y.clamp(0.0, max);

        if let Some((row, count @ 1..)) = patch {
            // The rows the source added, at display row `row`. With no sort the order is the
            // source's own, so the new rows are the last indexes.
            let added: Vec<u64> = (row..row + count).map(|r| table.source.key(r)).collect();
            cache.keys.splice(row..row, added);
            let old = cache.order.len();
            cache.order.extend(old..len);
        }
        if rebuilt {
            let old_first = cache.first;
            let old_first_key = cache.keys.get(old_first).copied();
            let old_cursor = cache.index_of(cursor.0);
            let mut order: Vec<usize> = (0..len).collect();
            if let Some(column) = sort.column.filter(|&c| c < columns) {
                let descending = sort.descending;
                order.sort_by(|&a, &b| {
                    let by = table.source.compare(column, a, b);
                    (if descending { by.reverse() } else { by }).then(a.cmp(&b))
                });
            }
            cache.keys = order.iter().map(|&row| table.source.key(row)).collect();
            cache.order = order;
            if !fresh {
                if table.is_changed() {
                    // Rows came or went: the row at the top stays at the top.
                    let now = cache.index_of(old_first_key);
                    y = table_anchor(y, pitch, old_first, now, max);
                    // A selected row that is gone is no longer selected (a tree's node under a
                    // closed parent is not gone); a cursor row that is gone moves to where it
                    // stood, and a tree's to the nearest node of its own that is shown.
                    if !selection.0.is_empty() {
                        let live: BTreeSet<u64> = cache.keys.iter().copied().collect();
                        let kept: BTreeSet<u64> = selection
                            .0
                            .iter()
                            .copied()
                            .filter(|k| live.contains(k) || tree.is_some_and(|t| t.contains(*k)))
                            .collect();
                        if kept != selection.0 {
                            selection.0 = kept;
                        }
                    }
                    if let Some(key) = cursor.0
                        && !cache.keys.contains(&key)
                    {
                        cursor.0 = tree.and_then(|t| t.shown_ancestor(key)).or_else(|| {
                            old_cursor
                                .and_then(|i| cache.keys.get(i.min(len.saturating_sub(1))).copied())
                        });
                    }
                    if cache.anchor.is_some_and(|k| !cache.keys.contains(&k)) {
                        cache.anchor = None;
                    }
                } else {
                    // A new sort starts from the top, or from the cursor row if there is one.
                    y = 0.0;
                    cache.follow = cursor.0.is_some();
                }
            }
            cache.built = true;
        }
        if rebuilt || patch.is_some() {
            cache.seen = table.source.revision();
        }
        // The table moved the scroll itself this frame (a sort starts from the top, the rows
        // shrank): a glide that was running is over, and a request starts from `y`.
        if (scroll.y - y).abs() > 0.01 {
            scroll.y = y;
            *glide = ScrollGlide::default();
        }
        if cache.follow {
            cache.follow = false;
            if let Some(i) = cache.index_of(cursor.0) {
                let top = i as f32 * pitch;
                let base = glide.goal().unwrap_or(y);
                let want = scroll_to_reveal(top, top + pitch, view_h, base, max, pitch);
                if (want - base).abs() > 0.01 {
                    glide.go(&scroll, want);
                }
            }
        }
        // The table steps its own viewport's glide, so the rows it binds below are the rows of the
        // scroll this frame draws (the glide system skips a table's viewport).
        if glide.goal().is_some() {
            step_glide(
                &mut glide,
                &mut scroll,
                glide_spring(skin, &settings),
                !ScrollGlide::glides(&settings, time.as_deref()),
                time.as_ref().map_or(0.0, |time| time.delta_secs()),
            );
            y = scroll.y.clamp(0.0, max);
        }
        if (scroll.y - y).abs() > 0.01 {
            scroll.y = y;
        }

        let pool = (tall.ceil() as usize + 1).min(len);
        let first = table_window(y, pitch, pool, len);
        let moved = first != cache.first || pool != cache.pool.len() || rebuilt || patch.is_some();
        let resized = cache.text_px != text_px || cache.pitch != pitch;
        cache.first = first;

        let width_px = |c: usize| width_of(&table, &widths, c) * text_px;
        let last = columns.saturating_sub(1);

        // The pool: the rows' entities, as many as can be in view.
        while cache.pool.len() > pool {
            if let Some(gone) = cache.pool.pop() {
                commands.entity(gone.row).despawn();
            }
        }
        for slot in 0..cache.pool.len() {
            if !(moved || selection.is_changed() || cursor.is_changed() || resized) {
                break;
            }
            let index = first + slot;
            let (Some(&key), Some(&source)) = (cache.keys.get(index), cache.order.get(index))
            else {
                continue;
            };
            let slot_row = &cache.pool[slot];
            if let Ok((mut row, selected)) = rows.get_mut(slot_row.row) {
                let want = TableRow {
                    table: entity,
                    index,
                    key,
                    cursor: cursor.0 == Some(key),
                };
                if *row != want {
                    *row = want;
                }
                let should = selection.contains(key);
                if should != selected {
                    if should {
                        commands.entity(slot_row.row).insert(Selected);
                    } else {
                        commands.entity(slot_row.row).remove::<Selected>();
                    }
                }
            }
            if moved {
                for (c, &(_, text)) in slot_row.cells.iter().enumerate() {
                    if let Ok(mut text) = texts.get_mut(text) {
                        let now = table.source.cell(source, c);
                        if text.0 != *now {
                            text.0 = now.into_owned();
                        }
                    }
                }
            }
            if (moved || resized)
                && let (Some(tree), Some(lead)) = (tree, &slot_row.lead)
            {
                bind_lead(lead, tree.outline(source), indent, &mut nodes);
            }
        }
        while cache.pool.len() < pool {
            let index = first + cache.pool.len();
            let (Some(&key), Some(&source)) = (cache.keys.get(index), cache.order.get(index))
            else {
                break;
            };
            let row = commands
                .spawn((
                    Node {
                        align_items: AlignItems::Center,
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Role::TableRow,
                    TableRow {
                        table: entity,
                        index,
                        key,
                        cursor: cursor.0 == Some(key),
                    },
                    Hovered::default(),
                    observe(row_on_click),
                    observe(row_on_double_press),
                    ChildOf(parts.rows),
                ))
                .id();
            if selection.contains(key) {
                commands.entity(row).insert(Selected);
            }
            let mut lead = None;
            let cells = (0..columns)
                .map(|c| {
                    let mut node = cell_node(width_px(c), c == last);
                    // A tree's chevron is a row high, so a click anywhere beside the label opens.
                    node.align_self = if tree.is_some() && c == 0 {
                        AlignSelf::Stretch
                    } else {
                        AlignSelf::Auto
                    };
                    let cell = commands
                        .spawn((node, Role::TableCell, Pickable::IGNORE, ChildOf(row)))
                        .id();
                    if let (Some(tree), 0) = (tree, c) {
                        lead = Some(spawn_lead(
                            &mut commands,
                            cell,
                            tree.outline(source),
                            indent,
                        ));
                    }
                    let text = commands
                        .spawn((
                            Text::new(table.source.cell(source, c).into_owned()),
                            ONE_LINE,
                            Role::TableCellText,
                            Pickable::IGNORE,
                            ChildOf(cell),
                        ))
                        .id();
                    (cell, text)
                })
                .collect();
            cache.pool.push(PoolRow { row, cells, lead });
        }

        // The room the rows not drawn would take, above and below the pool.
        if let Ok(mut column) = nodes.get_mut(parts.rows) {
            let above = Val::Px(first as f32 * pitch);
            let below = Val::Px(len.saturating_sub(first + cache.pool.len()) as f32 * pitch);
            if column.padding.top != above || column.padding.bottom != below {
                column.padding.top = above;
                column.padding.bottom = below;
            }
        }

        if rebuilt || widths.is_changed() || resized || moved {
            let mut left = 0.0;
            for (c, header) in parts.headers.iter().enumerate() {
                let px = width_px(c);
                left += px;
                if let Ok(mut node) = nodes.get_mut(header.cell) {
                    let want = cell_node(px, c == last);
                    if node.flex_basis != want.flex_basis || node.flex_grow != want.flex_grow {
                        node.flex_basis = want.flex_basis;
                        node.flex_grow = want.flex_grow;
                    }
                }
                if let Some(edge) = header.edge
                    && let Ok(mut node) = nodes.get_mut(edge)
                {
                    let at = Val::Px(left - MIN_TARGET_PX / 2.0);
                    if node.left != at {
                        node.left = at;
                    }
                }
                let marked = sort.column == Some(c);
                for (mark, shown) in [
                    (header.up, marked && !sort.descending),
                    (header.down, marked && sort.descending),
                ] {
                    if let Ok(mut node) = nodes.get_mut(mark) {
                        let want = if shown { Display::Flex } else { Display::None };
                        if node.display != want {
                            node.display = want;
                        }
                    }
                }
                for slot in &cache.pool {
                    if let Some(&(cell, _)) = slot.cells.get(c)
                        && let Ok(mut node) = nodes.get_mut(cell)
                    {
                        let want = cell_node(px, c == last);
                        if node.flex_basis != want.flex_basis || node.flex_grow != want.flex_grow {
                            node.flex_basis = want.flex_basis;
                            node.flex_grow = want.flex_grow;
                        }
                    }
                }
            }
        }
        cache.text_px = text_px;
        cache.pitch = pitch;
    }
}

/// What [`table_by_input`] moves.
type Typed = (
    &'static Table,
    &'static TableRows,
    &'static SelectMode,
    &'static mut TableSelection,
    &'static mut TableCursor,
    &'static mut TableCache,
);

/// The keys and the pad on a focused table: Up and Down move the cursor row (the D-pad too, held
/// to repeat), Page Up and Page Down a page (the triggers too), Home and End the first and last
/// rows of the data, Space selects the cursor row (toggles it in a [`SelectMode::Multi`] table),
/// Ctrl and A select every row, Enter and the pad's South button open it ([`RowActivated`]), and
/// typing goes to the next row whose first column starts with what was typed within a second
/// (type-select, scrolled into view; a tree keeps `*`). Shift extends the selection and Ctrl moves
/// the cursor and leaves the selection where it is. Shows focus. Runs in [`crate::GooeySystems::Navigate`], after [`crate::navigate_by_input`], which
/// leaves Up and Down to a table that has a row to move to that way and lets focus leave it from
/// its first and last rows.
#[allow(clippy::too_many_arguments)]
pub fn table_by_input(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut presses: MessageReader<KeyboardInput>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    gamepads: Query<&Gamepad>,
    mut tables: Query<Typed, Without<InteractionDisabled>>,
    trees: Query<(), With<TreeView>>,
    mut prefix: ResMut<TypedPrefix>,
    mut held: Local<(i8, f32)>,
    mut commands: Commands,
) {
    let typed: Vec<KeyboardInput> = presses
        .read()
        .filter(|p| p.state == ButtonState::Pressed)
        .cloned()
        .collect();
    let target = focus.get().filter(|&f| tables.contains(f));
    let Some(entity) = target else {
        *held = (0, 0.0);
        return;
    };
    let Ok((table, rows_tall, mode, mut selection, mut cursor, mut cache)) = tables.get_mut(entity)
    else {
        return;
    };
    let page = (rows_tall.0.floor() as usize).saturating_sub(1).max(1);
    let down = |button| gamepads.iter().any(|g| g.pressed(button));
    let pad_pressed = |button| gamepads.iter().any(|g| g.just_pressed(button));
    let held_key = |a, b| keys.pressed(a) || keys.pressed(b);
    let ctrl = held_key(KeyCode::ControlLeft, KeyCode::ControlRight);
    let shift = held_key(KeyCode::ShiftLeft, KeyCode::ShiftRight);
    let how = if ctrl {
        Pick::Move
    } else if shift && *mode == SelectMode::Multi {
        Pick::Range
    } else {
        Pick::Only
    };
    let mut steps: Vec<TableStep> = Vec::new();
    let (mut toggle, mut all, mut open) = (false, false, false);
    for press in &typed {
        match press.key_code {
            KeyCode::ArrowUp => steps.push(TableStep::Up),
            KeyCode::ArrowDown => steps.push(TableStep::Down),
            KeyCode::PageUp if !ctrl => steps.push(TableStep::PageUp),
            KeyCode::PageDown if !ctrl => steps.push(TableStep::PageDown),
            KeyCode::Home => steps.push(TableStep::First),
            KeyCode::End => steps.push(TableStep::Last),
            KeyCode::Space if !press.repeat => toggle = true,
            KeyCode::KeyA if ctrl && !press.repeat => all = true,
            KeyCode::Enter if !press.repeat => open = true,
            _ => {}
        }
    }
    // The pad: the D-pad steps and repeats while held; the triggers page; South opens.
    let direction = down(GamepadButton::DPadDown) as i8 - down(GamepadButton::DPadUp) as i8;
    if direction == 0 {
        *held = (0, 0.0);
    } else {
        let count = if held.0 == direction {
            let before = held.1;
            held.1 += time.delta_secs();
            repeat_steps(before, held.1, PAD_REPEAT_DELAY, PAD_REPEAT_RATE)
        } else {
            *held = (direction, 0.0);
            1
        };
        let step = if direction > 0 {
            TableStep::Down
        } else {
            TableStep::Up
        };
        steps.extend(std::iter::repeat_n(step, count as usize));
    }
    if pad_pressed(GamepadButton::LeftTrigger2) {
        steps.push(TableStep::PageUp);
    }
    if pad_pressed(GamepadButton::RightTrigger2) {
        steps.push(TableStep::PageDown);
    }
    open |= pad_pressed(GamepadButton::South);

    // Type-select: a tree keeps `*` for opening a node's siblings.
    let mut characters = typed_characters(&typed, plain(&keys));
    if trees.contains(entity) {
        characters.retain(|c| c != '*');
    }

    let handled = !steps.is_empty() || toggle || all || open || !characters.is_empty();
    if !handled {
        return;
    }
    show_focus(&mut shown);
    let len = table.len();
    for step in steps {
        let from = cache.index_of(cursor.0);
        if let Some(to) = table_step(from, step, len, page) {
            let how = if matches!(step, TableStep::PageUp | TableStep::PageDown) && ctrl {
                Pick::Move
            } else {
                how
            };
            pick(
                entity,
                to,
                how,
                true,
                (mode, &mut selection, &mut cursor, &mut cache),
                &mut commands,
            );
        }
    }
    if toggle {
        let at = cache
            .index_of(cursor.0)
            .or(table_step(None, TableStep::Down, len, page));
        if let Some(at) = at {
            let how = if *mode == SelectMode::Multi {
                Pick::Toggle
            } else {
                Pick::Only
            };
            pick(
                entity,
                at,
                how,
                true,
                (mode, &mut selection, &mut cursor, &mut cache),
                &mut commands,
            );
        }
    }
    if !characters.is_empty() {
        let (needle, len) = prefix.needle(entity, time.elapsed_secs(), &characters);
        let found = type_ahead(cache.keys.len(), cache.index_of(cursor.0), len, |i| {
            // The cache is rebuilt in Style; rows a source lost earlier this frame are no match.
            let row = cache.order.get(i).copied().unwrap_or(i);
            row < table.len() && starts_with_ignore_case(&table.source.cell(row, 0), &needle)
        });
        if let Some(at) = found {
            pick(
                entity,
                at,
                Pick::Only,
                true,
                (mode, &mut selection, &mut cursor, &mut cache),
                &mut commands,
            );
        }
    }
    if all && *mode == SelectMode::Multi {
        let every: BTreeSet<u64> = cache.keys.iter().copied().collect();
        if every != selection.0 {
            selection.0 = every;
            commands.trigger(ValueChange {
                source: entity,
                value: selection.clone(),
                is_final: true,
            });
        }
    }
    if open && let Some(key) = cursor.0 {
        commands.trigger(RowActivated { entity, key });
    }
}
