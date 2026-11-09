//! The accessibility tree: what a screen reader hears of every widget.
//!
//! Bevy gives a widget an `AccessibilityNode` with the right role when it is a `Button`,
//! `Checkbox`, `Slider`, tab or text field, and keeps its `toggled`, `selected` and `disabled`
//! state. It never names them, and it knows nothing of a heading, a meter, a toast, a table or a
//! splitter. [`update_accessibility`] fills that in from what the widget already draws: the name
//! is the words on screen, so a game writes nothing twice. [`spec`] is one exhaustive `match` over
//! [`Role`]: a new role does not compile until someone says what an assistive technology sees.
//!
//! The system returns at once unless `bevy::a11y::AccessibilityRequested` is set (an assistive
//! technology connected). It reads each node through `Deref` and writes only a difference, so a
//! settled interface marks nothing changed.
//!
//! The same pass declares what an assistive technology may ask of each widget (press, focus, step,
//! set); [`crate::access_actions`] carries the requests out as the widget's own events.

use accesskit::{Action, HasPopup, Node as Node_, Role as Ak, Toggled};
use bevy::a11y::{AccessibilityNode, AccessibilityRequested};
use bevy::ecs::query::QueryData;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled, Selected};

use crate::access_actions::{Handles, MANAGED, TREE_BRANCH, actions_of};
use crate::binding::Capturing;
use crate::choice::ChoiceValue;
use crate::color::ColorSwatch;
use crate::cooldown::Cooldown;
use crate::menu::{MenuItem, MenuOpen};
use crate::meter::Meter;
use crate::model::cooldown_words;
use crate::progress::Progress;
use crate::section::{SectionHeader, SectionOpen};
use crate::skin::Role;
use crate::split::{SplitDivider, SplitFraction};
use crate::table::{Table, TableRow};
use crate::text::{AreaField, TextInputOf, TextInputValue};
use crate::toast::{Toast, ToastKind};
use crate::tooltip::Tooltip;
use crate::tree::{Tree, TreeView};

/// What a role is to an assistive technology.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spec {
    /// Decorative, or its words belong to the widget around it: no node of its own, and a node
    /// Bevy added (an `ImageNode`'s) is hidden.
    Hidden,
    /// A container with no meaning of its own. It keeps the tree nested, so the screen reader
    /// reads in layout order and not in the order the entities are stored.
    Group,
    /// A node the widget's own arm of [`describe`] fills in.
    Node,
}

/// The exhaustive table: every [`Role`] is one of three things.
fn spec(role: Role) -> Spec {
    match role {
        Role::Screen
        | Role::Panel
        | Role::Surface(_)
        | Role::Tone(_)
        | Role::Hovered
        | Role::Pressed
        | Role::Held
        | Role::HeldGround
        | Role::Unchecked
        | Role::Card
        | Role::PreviewEdge
        | Role::Menu
        | Role::Popover
        | Role::ToastStack
        | Role::ToastBody
        | Role::Prompts
        | Role::Prompt
        | Role::ShortcutLegend
        | Role::List
        | Role::ListRows
        | Role::TableView
        | Role::TableRows
        | Role::TableHeader
        | Role::Split
        | Role::SplitPane
        | Role::Dock
        | Role::DockBody
        | Role::Tabs
        | Role::TabList
        | Role::TabPages
        | Role::TextInput
        | Role::TextArea
        | Role::TextCell
        | Role::NumberPart
        | Role::NumberParts
        | Role::ColorStripBox
        | Role::Section
        | Role::SectionBody
        | Role::StatusGroup
        | Role::StatusItem
        | Role::StatusBar
        | Role::Inspector
        | Role::NotificationCenter
        | Role::Palette
        | Role::PaletteField
        | Role::SlotGrid
        | Role::CardGrid
        | Role::Led
        | Role::FieldMessage => Spec::Group,
        Role::Heading
        | Role::Label
        | Role::Button
        | Role::CardCell
        | Role::Meter
        | Role::Tooltip
        | Role::Scrim
        | Role::Toggle
        | Role::Slider
        | Role::Choice
        | Role::Binding
        | Role::PromptKey
        | Role::PromptButton
        | Role::Toast
        | Role::ToastAction
        | Role::Table
        | Role::TableRow
        | Role::TableCell
        | Role::TableEdge
        | Role::Tab
        | Role::TabPage
        | Role::TextField
        | Role::ColorSwatch
        | Role::ColorArea
        | Role::Viewport
        | Role::SplitDivider
        | Role::MenuBar
        | Role::MenuPanel
        | Role::ToggleGroup
        | Role::MenuTitle
        | Role::MenuItem
        | Role::MenuNote
        | Role::PaletteRow
        | Role::PaletteCount
        | Role::Progress
        | Role::Spinner
        | Role::Toolbar
        | Role::GroupOption
        | Role::Slot
        | Role::SlotEmpty
        | Role::Radial
        | Role::SectionNote
        | Role::TableHeaderCell
        | Role::ListView => Spec::Node,
        Role::ButtonText
        | Role::MenuTitleText
        | Role::MeterFill
        | Role::MeterTrail
        | Role::ToggleTrack
        | Role::ToggleKnob
        | Role::SliderHeader
        | Role::SliderTrack
        | Role::SliderRail
        | Role::SliderFill
        | Role::SliderGap
        | Role::SliderThumb
        | Role::SliderReadout
        | Role::ControlText
        | Role::ChoiceArrow
        | Role::ChoiceBack
        | Role::ChoiceForward
        | Role::ChoiceCell
        | Role::ChoiceOption
        | Role::BindingCap
        | Role::BindingText
        | Role::BindingPrompt
        | Role::ShortcutBadge
        | Role::ToastMark
        | Role::ToastText
        | Role::ListRail
        | Role::ListThumb
        | Role::TableSortUp
        | Role::TableSortDown
        | Role::TableCellText
        | Role::TreeTwisty
        | Role::TreeClosed
        | Role::TreeOpen
        | Role::SplitGrip
        | Role::DockDrop
        | Role::DockGhost
        | Role::DragGhost
        | Role::DragGhostRefused
        | Role::DropMark
        | Role::DropMarkRefused
        | Role::TabText
        | Role::TextHint
        | Role::MenuMark
        | Role::MenuCheck
        | Role::MenuDot
        | Role::MenuText
        | Role::MenuShortcut
        | Role::MenuChevron
        | Role::ColorSwatchHalf
        | Role::ColorAreaMarker
        | Role::ColorAreaMarkerInner
        | Role::ColorRail
        | Role::Icon
        | Role::FieldMark
        | Role::ProgressLine
        | Role::StatusMark
        | Role::MenuSeparator
        | Role::ToolbarSeparator
        | Role::RadialTrack
        | Role::SlotName
        | Role::SlotCount
        | Role::SlotIcon
        | Role::CardImage
        | Role::CardName
        | Role::CardLine
        | Role::TabBadge
        | Role::Cooldown
        | Role::CooldownText
        | Role::LedDot
        | Role::FrameArt
        | Role::HoldTrack
        | Role::HoldFill
        | Role::RadialIcon
        | Role::RadialSlot
        | Role::RadialText
        | Role::RadialCentre
        | Role::ToastActionText => Spec::Hidden,
    }
}

/// The roles whose text is a widget's own name: a button's, a tab's, a menu row's, a table cell's,
/// a toast button's. (Label, heading and control text are read by their own arms.)
fn names(role: Role) -> bool {
    matches!(
        role,
        Role::ButtonText
            | Role::MenuTitleText
            | Role::TabText
            | Role::TabBadge
            | Role::MenuText
            | Role::TableCellText
            | Role::ToastActionText
    )
}

/// What a node should say, as far as the crate owns it. `None` fields are left as they are.
#[derive(Default, Debug, PartialEq)]
struct Wanted {
    /// Set only where Bevy has not set the role already.
    role: Option<Ak>,
    name: Option<String>,
    value: Option<String>,
    /// `(value, min, max)`.
    numeric: Option<(f64, f64, f64)>,
    toggled: Option<bool>,
    selected: Option<bool>,
    expanded: Option<bool>,
    /// A tree row that can be opened and shut: it declares Expand and Collapse.
    tree_branch: bool,
    /// A tree row with no children: it is not expandable, so a state its pooled entity said while
    /// it was another node's must go.
    end_node: bool,
    /// A tree row's depth, from 0 as AccessKit counts (the platform adapters add the 1).
    level: Option<usize>,
    /// A tree row's place among the rows shown, from 0.
    position_in_set: Option<usize>,
    /// A tree's rows shown: the platform reads a row's set size from its container.
    size_of_set: Option<usize>,
    /// A table row's index among all rows, from 0.
    row_index: Option<usize>,
    row_count: Option<usize>,
    column_count: Option<usize>,
    /// A toast's live region: `Some` interrupts or waits, `None` is not a toast.
    live: Option<accesskit::Live>,
    /// A column header's sort: `Some(None)` clears the state (this column is not the one sorted
    /// by), `Some(Some(_))` sets it, and `None` is no header at all.
    sort: Option<Option<accesskit::SortDirection>>,
    popup: bool,
    modal: bool,
    hidden: bool,
    disabled: bool,
    /// What an assistive technology may ask of the widget ([`actions_of`]).
    actions: &'static [Action],
}

/// The name a crate part gives an unlabelled control, English like the crate's other words
/// ("Dismiss", "More"); a host's own [`AccessibleLabel`] replaces it.
const DIVIDER: &str = "Divider";
const COLUMN_EDGE: &str = "Resize column";
const EMPTY_SLOT: &str = "Empty slot";
const COLOUR_AREA: &str = "Saturation and brightness";
const COMMAND_FIELD: &str = "Command";

/// Everything one entity says about itself, read once.
#[derive(QueryData)]
pub(crate) struct Facts {
    label: Option<&'static AccessibleLabel>,
    seen: Option<&'static InheritedVisibility>,
    node: Option<&'static Node>,
    disabled: Has<InteractionDisabled>,
    selected: Has<Selected>,
    checked: Has<Checked>,
    open: Has<MenuOpen>,
    capturing: Has<Capturing>,
    header: Has<SectionHeader>,
    area: Has<AreaField>,
    item: Option<&'static MenuItem>,
    toast: Option<&'static Toast>,
    meter: Option<&'static Meter>,
    progress: Option<&'static Progress>,
    row: Option<&'static TableRow>,
    field: Option<&'static TextInputOf>,
    swatch: Option<&'static ColorSwatch>,
    choice: Option<&'static ChoiceValue>,
    divider: Option<&'static SplitDivider>,
    column: Option<&'static crate::table::TableHeaderCell>,
    cooldown: Option<&'static Cooldown>,
    handles: Handles,
}

/// The queries a name or a value is read through.
#[derive(SystemParam)]
pub(crate) struct Look<'w, 's> {
    sorts: Query<'w, 's, &'static crate::table::TableSort>,
    texts: Query<'w, 's, &'static Text>,
    roles: Query<'w, 's, &'static Role>,
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static ChildOf>,
    values: Query<'w, 's, &'static TextInputValue>,
    sections: Query<'w, 's, &'static SectionOpen>,
    fractions: Query<'w, 's, &'static SplitFraction>,
    trees: Query<'w, 's, (), With<TreeView>>,
    tables: Query<'w, 's, (&'static Table, Has<InteractionDisabled>)>,
    tooltips: Query<'w, 's, &'static Tooltip>,
}

impl Look<'_, '_> {
    /// An entity's place among a list's rows: the column, how many rows it holds, and which this one
    /// is. `None` outside a list.
    fn list_place(&self, entity: Entity) -> Option<(Entity, usize, usize)> {
        let mut here = entity;
        while let Ok(parent) = self.parents.get(here) {
            let up = parent.parent();
            if self.roles.get(up) == Ok(&Role::ListRows)
                && let Ok(kids) = self.children.get(up)
                && let Some(at) = kids.iter().position(|kid| kid == here)
            {
                return Some((up, kids.len(), at));
            }
            here = up;
        }
        None
    }

    /// A `ListView`'s row count and its rows' entity, read from the one `Role::ListRows` column
    /// under it (a list's viewport holds that column, and the column holds the rows).
    fn list_rows(&self, view: Entity) -> Option<(Entity, usize)> {
        let mut pending = vec![view];
        while let Some(entity) = pending.pop() {
            if self.roles.get(entity) == Ok(&Role::ListRows)
                && let Ok(kids) = self.children.get(entity)
            {
                return Some((entity, kids.len()));
            }
            if let Ok(kids) = self.children.get(entity) {
                pending.extend(kids.iter());
            }
        }
        None
    }

    /// The words of every descendant of `root` whose role `want` accepts, in layout order, joined
    /// by a space. A popup, a tooltip and a section's body are another widget's words and are
    /// not entered.
    fn words(&self, root: Entity, want: impl Fn(Role) -> bool + Copy) -> Option<String> {
        let mut out = String::new();
        self.collect(root, want, &mut out);
        let trimmed = out.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    }

    fn collect(&self, from: Entity, want: impl Fn(Role) -> bool + Copy, out: &mut String) {
        let Ok(children) = self.children.get(from) else {
            return;
        };
        for child in children {
            let Ok(&role) = self.roles.get(*child) else {
                self.collect(*child, want, out);
                continue;
            };
            if matches!(
                role,
                Role::MenuPanel | Role::Popover | Role::Tooltip | Role::SectionBody
            ) {
                continue;
            }
            if want(role)
                && let Ok(text) = self.texts.get(*child)
                && !text.0.is_empty()
            {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(&text.0);
            }
            self.collect(*child, want, out);
        }
    }

    /// The words of each of `root`'s children of `role`, joined by a comma: a list a screen reader
    /// pauses between.
    fn lines(&self, root: Entity, role: Role) -> Option<String> {
        let children = self.children.get(root).ok()?;
        let lines: Vec<&str> = children
            .iter()
            .filter(|child| self.roles.get(*child).is_ok_and(|r| *r == role))
            .filter_map(|child| self.texts.get(child).ok())
            .map(|text| text.0.trim())
            .filter(|text| !text.is_empty())
            .collect();
        (!lines.is_empty()).then(|| lines.join(", "))
    }

    /// The row's label: a toggle's, a slider's, a text field's.
    fn control_text(&self, root: Entity) -> Option<String> {
        self.words(root, |role| role == Role::ControlText)
    }

    /// A button's name: its text, else its tooltip (an icon button has no other).
    fn name_of(&self, root: Entity) -> Option<String> {
        self.words(root, names).or_else(|| {
            self.tooltips
                .get(root)
                .ok()
                .map(|tip| tip.text.clone())
                .filter(|text| !text.is_empty())
        })
    }

    fn own(&self, entity: Entity) -> Option<String> {
        self.texts
            .get(entity)
            .ok()
            .map(|text| text.0.clone())
            .filter(|text| !text.is_empty())
    }

    /// The option of a choice that is selected: the `index`th of its options, in layout order. It is
    /// read by index, not by what is shown, so a choice on a page that is hidden keeps its value.
    fn selected_option(&self, root: Entity, index: usize) -> Option<String> {
        let mut options = Vec::new();
        self.options(root, &mut options);
        options.get(index).and_then(|option| self.own(*option))
    }

    fn options(&self, from: Entity, out: &mut Vec<Entity>) {
        for child in self.children.get(from).into_iter().flat_map(|c| c.iter()) {
            if self.roles.get(child) == Ok(&Role::ChoiceOption) {
                out.push(child);
            }
            self.options(child, out);
        }
    }

    fn parent(&self, entity: Entity) -> Option<Entity> {
        self.parents.get(entity).ok().map(ChildOf::parent)
    }
}

/// What the entity of `role` says, `None` when it is decorative after all (a meter's track inside a
/// progress bar, a spinner nobody named).
fn describe(entity: Entity, role: Role, facts: &FactsItem, look: &Look) -> Option<Wanted> {
    let mut wanted = Wanted::default();
    match role {
        Role::Heading => {
            wanted.role = Some(Ak::Heading);
            wanted.name = look.own(entity);
        }
        Role::Label
        | Role::PromptKey
        | Role::PromptButton
        | Role::MenuNote
        | Role::PaletteCount
        | Role::SectionNote => {
            wanted.role = Some(Ak::Label);
            wanted.value = look.own(entity);
        }
        Role::Button => {
            wanted.name = look.name_of(entity);
            // A list's row is a button the list owns: it reads as a list item with its place.
            if let Some((_, count, at)) = look.list_place(entity) {
                wanted.role = Some(Ak::ListItem);
                wanted.position_in_set = Some(at);
                wanted.size_of_set = Some(count);
            }
            if facts.header
                && let Some(parent) = look.parent(entity)
                && let Ok(open) = look.sections.get(parent)
            {
                wanted.expanded = Some(open.0);
            }
        }
        Role::Meter => {
            let meter = facts.meter?;
            wanted.role = Some(Ak::Meter);
            wanted.numeric = Some((f64::from(meter.value), 0.0, f64::from(meter.max)));
        }
        Role::Tooltip => wanted.role = Some(Ak::Tooltip),
        Role::Scrim => {
            wanted.role = Some(Ak::Dialog);
            wanted.modal = true;
            wanted.name = look.words(entity, |role| role == Role::Heading);
        }
        Role::Toggle => wanted.name = look.control_text(entity),
        Role::Slider => {
            wanted.name = look.control_text(entity).or_else(|| {
                // A colour strip's label is the text above it, in the box that holds both.
                let parent = look.parent(entity)?;
                (look.roles.get(parent) == Ok(&Role::ColorStripBox))
                    .then(|| look.words(parent, |role| role == Role::Label))
                    .flatten()
            });
            wanted.value = look.words(entity, |role| role == Role::SliderReadout);
        }
        Role::Choice => {
            wanted.role = Some(Ak::SpinButton);
            wanted.name = look.control_text(entity);
            wanted.value = facts
                .choice
                .and_then(|value| look.selected_option(entity, value.0));
        }
        // A wheel is read as a spin button whose value is the highlighted item's name, the one
        // in its middle; the host names it with `AccessibleLabel`.
        Role::Radial => {
            wanted.role = Some(Ak::SpinButton);
            wanted.value = look.words(entity, |role| role == Role::RadialCentre);
        }
        // A slot is its item's name, and its count where there is more than one; an empty one
        // says so.
        Role::Slot => {
            wanted.name = look.words(entity, |role| role == Role::SlotName);
            wanted.value = look.words(entity, |role| role == Role::SlotCount);
        }
        Role::SlotEmpty => wanted.name = Some(EMPTY_SLOT.to_owned()),
        // A card is its name, and its quiet lines one after another as its value; chosen is
        // selected, as a tab is.
        Role::CardCell => {
            wanted.name = look.words(entity, |role| role == Role::CardName);
            wanted.value = look.lines(entity, Role::CardLine);
            wanted.selected = Some(facts.selected);
        }
        Role::Binding => {
            wanted.name = look.control_text(entity);
            wanted.value = if facts.capturing {
                look.words(entity, |role| role == Role::BindingPrompt)
            } else {
                look.words(entity, |role| role == Role::BindingText)
            };
        }
        Role::Toast => {
            let alert = facts
                .toast
                .is_some_and(|toast| toast.kind() == ToastKind::Alert);
            wanted.role = Some(if alert { Ak::Alert } else { Ak::Status });
            wanted.name = look.words(entity, |role| role == Role::ToastText);
            // A toast appears by itself: a live region is how a screen reader is told, and an alert
            // interrupts while a plain toast waits its turn (WCAG 4.1.3, Status Messages).
            wanted.live = Some(if alert {
                accesskit::Live::Assertive
            } else {
                accesskit::Live::Polite
            });
        }
        Role::ListView => {
            // A list's rows are the column under it: the set's size and each row's place come from
            // there, so a reader announces "row 3 of 40" whether or not the rows are spawned.
            wanted.role = Some(Ak::List);
            let rows = look
                .parents
                .get(entity)
                .ok()
                .and_then(|_| look.list_rows(entity));
            wanted.size_of_set = rows.map(|(_, count)| count);
        }
        Role::ToastAction => wanted.name = look.words(entity, names),
        // A table's rows are a pool of a dozen entities rebound while scrolling, so the whole
        // set's size and each row's place are said here: a reader announces "row 5 of 10,000",
        // never "row 5 of 12".
        Role::Table => {
            let rows = look.tables.get(entity).ok().map(|(table, _)| table.len());
            if look.trees.contains(entity) {
                wanted.role = Some(Ak::Tree);
                wanted.size_of_set = rows;
            } else {
                wanted.role = Some(Ak::Table);
                wanted.row_count = rows;
                wanted.column_count = look
                    .tables
                    .get(entity)
                    .ok()
                    .map(|(table, _)| table.columns().len());
            }
        }
        Role::TableRow => {
            let tree = facts.row.is_some_and(|row| look.trees.contains(row.table));
            wanted.role = Some(if tree { Ak::TreeItem } else { Ak::Row });
            wanted.selected = Some(facts.selected);
            if let Some(row) = facts.row {
                if tree {
                    wanted.position_in_set = Some(row.index);
                    tree_place(&mut wanted, row, look);
                } else {
                    wanted.row_index = Some(row.index);
                }
            }
        }
        Role::TableCell => {
            wanted.role = Some(Ak::Cell);
            wanted.name = look.words(entity, names);
        }
        Role::TableHeaderCell => {
            wanted.role = Some(Ak::ColumnHeader);
            wanted.name = look.words(entity, names);
            // The sort mark, as `update_tables` drew it: this column's direction, or neither.
            if let Some(cell) = facts.column
                && let Ok(sort) = look.sorts.get(cell.table)
            {
                wanted.sort = Some(match sort.column {
                    Some(column) if column == cell.column && sort.descending => {
                        Some(accesskit::SortDirection::Descending)
                    }
                    Some(column) if column == cell.column => {
                        Some(accesskit::SortDirection::Ascending)
                    }
                    _ => None,
                });
            }
        }
        Role::Tab => wanted.name = look.words(entity, names),
        Role::TableEdge => {
            wanted.role = Some(Ak::Splitter);
            wanted.name = Some(COLUMN_EDGE.to_owned());
        }
        Role::TabPage => wanted.role = Some(Ak::TabPanel),
        Role::TextField => {
            if facts.area {
                wanted.role = Some(Ak::MultilineTextInput);
            }
            let row = facts.field?.0;
            wanted.name = look.control_text(row).or_else(|| {
                (look.roles.get(row) == Ok(&Role::PaletteField)).then(|| COMMAND_FIELD.to_owned())
            });
            wanted.value = look.values.get(row).ok().map(|value| value.0.clone());
        }
        Role::ColorSwatch => {
            let row = facts.swatch?.0;
            wanted.role = Some(Ak::ColorWell);
            wanted.name = look.control_text(row);
            wanted.value = look.values.get(row).ok().map(|value| value.0.clone());
        }
        Role::ColorArea => {
            wanted.role = Some(Ak::Group);
            wanted.name = Some(COLOUR_AREA.to_owned());
        }
        Role::Viewport => wanted.role = Some(Ak::Pane),
        Role::SplitDivider => {
            facts.divider?;
            wanted.role = Some(Ak::Splitter);
            wanted.name = Some(DIVIDER.to_owned());
            let fraction = look
                .parent(entity)
                .and_then(|parent| look.fractions.get(parent).ok());
            wanted.numeric = fraction.map(|fraction| (f64::from(fraction.0), 0.0, 1.0));
        }
        Role::MenuBar => wanted.role = Some(Ak::MenuBar),
        Role::MenuPanel => wanted.role = Some(Ak::Menu),
        Role::MenuTitle => {
            wanted.role = Some(Ak::MenuItem);
            wanted.name = look.words(entity, names);
            wanted.popup = true;
            wanted.expanded = Some(facts.open);
        }
        Role::MenuItem => {
            let (role, popup) = facts.item.map_or((Ak::MenuItem, false), MenuItem::access);
            wanted.role = Some(role);
            wanted.name = look.words(entity, names);
            wanted.popup = popup;
            if popup {
                wanted.expanded = Some(facts.open);
            }
            if matches!(role, Ak::MenuItemCheckBox | Ak::MenuItemRadio) {
                wanted.toggled = Some(facts.checked);
            }
        }
        Role::PaletteRow => {
            wanted.role = Some(Ak::ListBoxOption);
            wanted.name = look.words(entity, names);
            wanted.selected = Some(facts.selected);
        }
        Role::Progress => {
            let progress = facts.progress?;
            wanted.role = Some(Ak::ProgressIndicator);
            wanted.numeric = Some((f64::from(progress.0), 0.0, 1.0));
            wanted.value = look.words(entity, |role| role == Role::ProgressLine);
        }
        Role::Spinner => {
            facts.label?;
            wanted.role = Some(Ak::ProgressIndicator);
        }
        Role::Toolbar => wanted.role = Some(Ak::Toolbar),
        Role::ToggleGroup => wanted.role = Some(Ak::RadioGroup),
        Role::GroupOption => {
            wanted.role = Some(Ak::RadioButton);
            wanted.name = look.name_of(entity);
            wanted.toggled = Some(facts.checked);
        }
        _ => {}
    }
    if let Some(label) = facts.label {
        wanted.name = Some(label.0.clone());
    }
    // A cooldown running is part of what the widget is: its seconds follow the name, the host's
    // own label included.
    if let Some(cooldown) = facts.cooldown
        && !cooldown.is_ready()
        && let Some(name) = wanted.name.as_mut()
    {
        name.push_str(", ");
        name.push_str(&cooldown_words(cooldown.remaining));
    }
    wanted.disabled = facts.disabled;
    wanted.actions = if wanted.tree_branch {
        TREE_BRANCH
    } else {
        actions_of(&facts.handles)
    };
    wanted.hidden = facts.seen.is_some_and(|seen| !seen.get())
        || facts.node.is_some_and(|node| node.display == Display::None);
    Some(wanted)
}

/// A tree row's depth and whether it opens, read from the [`Tree`] by the row's key: a row is a
/// pooled entity, so the node it shows changes under it. A branch that cannot be opened (the
/// tree is disabled) still says its state, and declares no action.
fn tree_place(wanted: &mut Wanted, row: &TableRow, look: &Look) {
    let Ok((table, disabled)) = look.tables.get(row.table) else {
        return;
    };
    let Some(outline) = table
        .source::<Tree>()
        .and_then(|tree| tree.outline_of(row.key))
    else {
        return;
    };
    wanted.level = Some(outline.depth);
    match outline.open {
        Some(open) => {
            wanted.expanded = Some(open);
            if !disabled {
                wanted.tree_branch = true;
            }
        }
        None => wanted.end_node = true,
    }
}

/// Whether `node` already says everything `wanted` asks.
fn says(node: &Node_, wanted: &Wanted) -> bool {
    wanted.role.is_none_or(|role| node.role() == role)
        && node.label() == wanted.name.as_deref()
        && node.value() == wanted.value.as_deref()
        && wanted.numeric.is_none_or(|(value, min, max)| {
            node.numeric_value() == Some(value)
                && node.min_numeric_value() == Some(min)
                && node.max_numeric_value() == Some(max)
        })
        && wanted
            .toggled
            .is_none_or(|on| node.toggled() == Some(Toggled::from(on)))
        && wanted
            .selected
            .is_none_or(|selected| node.is_selected() == Some(selected))
        && wanted
            .expanded
            .is_none_or(|expanded| node.is_expanded() == Some(expanded))
        && (!wanted.end_node || node.is_expanded().is_none())
        && wanted.level.is_none_or(|level| node.level() == Some(level))
        && wanted
            .position_in_set
            .is_none_or(|at| node.position_in_set() == Some(at))
        && wanted
            .size_of_set
            .is_none_or(|size| node.size_of_set() == Some(size))
        && wanted
            .row_index
            .is_none_or(|at| node.row_index() == Some(at))
        && wanted
            .row_count
            .is_none_or(|count| node.row_count() == Some(count))
        && wanted
            .column_count
            .is_none_or(|count| node.column_count() == Some(count))
        && wanted.sort.is_none_or(|sort| node.sort_direction() == sort)
        && wanted.live.is_none_or(|live| node.live() == Some(live))
        && (!wanted.popup || node.has_popup() == Some(HasPopup::Menu))
        && node.is_modal() == wanted.modal
        && node.is_hidden() == wanted.hidden
        && node.is_disabled() == wanted.disabled
        && MANAGED
            .iter()
            .all(|action| node.supports_action(*action) == wanted.actions.contains(action))
}

/// Writes what `wanted` asks, field by field.
fn write(node: &mut Node_, wanted: &Wanted) {
    if let Some(role) = wanted.role {
        node.set_role(role);
    }
    match &wanted.name {
        Some(name) => node.set_label(name.as_str()),
        None => node.clear_label(),
    }
    match &wanted.value {
        Some(value) => node.set_value(value.as_str()),
        None => node.clear_value(),
    }
    if let Some((value, min, max)) = wanted.numeric {
        node.set_numeric_value(value);
        node.set_min_numeric_value(min);
        node.set_max_numeric_value(max);
    }
    if let Some(on) = wanted.toggled {
        node.set_toggled(Toggled::from(on));
    }
    if let Some(selected) = wanted.selected {
        node.set_selected(selected);
    }
    if let Some(expanded) = wanted.expanded {
        node.set_expanded(expanded);
    }
    if wanted.end_node {
        node.clear_expanded();
    }
    if let Some(level) = wanted.level {
        node.set_level(level);
    }
    if let Some(at) = wanted.position_in_set {
        node.set_position_in_set(at);
    }
    if let Some(size) = wanted.size_of_set {
        node.set_size_of_set(size);
    }
    if let Some(at) = wanted.row_index {
        node.set_row_index(at);
    }
    if let Some(count) = wanted.row_count {
        node.set_row_count(count);
    }
    if let Some(count) = wanted.column_count {
        node.set_column_count(count);
    }
    match wanted.sort {
        Some(Some(sort)) => node.set_sort_direction(sort),
        Some(None) => node.clear_sort_direction(),
        None => {}
    }
    if let Some(live) = wanted.live {
        node.set_live(live);
    }
    if wanted.popup {
        node.set_has_popup(HasPopup::Menu);
    }
    flag(wanted.modal, node, Node_::set_modal, Node_::clear_modal);
    flag(wanted.hidden, node, Node_::set_hidden, Node_::clear_hidden);
    flag(
        wanted.disabled,
        node,
        Node_::set_disabled,
        Node_::clear_disabled,
    );
    for action in MANAGED {
        if wanted.actions.contains(&action) {
            node.add_action(action);
        } else {
            node.remove_action(action);
        }
    }
}

fn flag(on: bool, node: &mut Node_, set: fn(&mut Node_), clear: fn(&mut Node_)) {
    if on {
        set(node);
    } else {
        clear(node);
    }
}

/// Gives every widget its accessibility node and keeps its name, value and state true, while an
/// assistive technology is connected (`AccessibilityRequested`); a game without one pays for one
/// resource read. Runs last in [`crate::GooeySystems::Style`].
pub(crate) fn update_accessibility(
    requested: Option<Res<AccessibilityRequested>>,
    mut commands: Commands,
    mut nodes: Query<(Entity, &Role, Option<&mut AccessibilityNode>, Facts)>,
    look: Look,
) {
    if requested.is_some_and(|requested| !requested.get()) {
        return;
    }
    for (entity, &role, node, facts) in &mut nodes {
        let kind = spec(role);
        let wanted = match kind {
            Spec::Hidden => None,
            Spec::Group => {
                let mut group = Wanted {
                    role: Some(Ak::GenericContainer),
                    ..default()
                };
                group.name = facts.label.map(|label| label.0.clone());
                group.hidden = facts.seen.is_some_and(|seen| !seen.get())
                    || facts.node.is_some_and(|node| node.display == Display::None);
                Some(group)
            }
            Spec::Node => describe(entity, role, &facts, &look),
        };
        match (node, wanted) {
            (Some(mut node), Some(wanted)) => {
                if !says(&node, &wanted) {
                    write(&mut node, &wanted);
                }
            }
            (Some(mut node), None) => {
                if !node.is_hidden() {
                    node.set_hidden();
                }
            }
            (None, Some(wanted)) => {
                let mut node = Node_::new(wanted.role.unwrap_or(Ak::GenericContainer));
                write(&mut node, &wanted);
                commands
                    .entity(entity)
                    .insert(AccessibilityNode::from(node));
                // Bevy sets a node's bounds when its layout changes; a node added now has not
                // had one, so ask for it.
                commands.queue(move |world: &mut World| {
                    if let Some(mut computed) = world.get_mut::<ComputedNode>(entity) {
                        computed.set_changed();
                    }
                });
            }
            (None, None) => {}
        }
    }
}
