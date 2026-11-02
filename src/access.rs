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

use accesskit::{HasPopup, Node as Node_, Role as Ak, Toggled};
use bevy::a11y::{AccessibilityNode, AccessibilityRequested};
use bevy::ecs::query::QueryData;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled, Selected};

use crate::binding::Capturing;
use crate::choice::ChoiceValue;
use crate::color::ColorSwatch;
use crate::menu::{MenuItem, MenuOpen};
use crate::meter::Meter;
use crate::progress::Progress;
use crate::section::{SectionHeader, SectionOpen};
use crate::skin::Role;
use crate::split::{SplitDivider, SplitFraction};
use crate::table::TableRow;
use crate::text::{AreaField, TextInputOf, TextInputValue};
use crate::toast::{Toast, ToastKind};
use crate::tooltip::Tooltip;
use crate::tree::TreeView;

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
        | Role::Menu
        | Role::Popover
        | Role::ToastStack
        | Role::ToastBody
        | Role::Prompts
        | Role::Prompt
        | Role::List
        | Role::ListView
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
        | Role::FieldMessage => Spec::Group,
        Role::Heading
        | Role::Label
        | Role::Button
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
        | Role::TableHeaderCell
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
        | Role::Radial
        | Role::SectionNote => Spec::Node,
        Role::ButtonText
        | Role::MeterFill
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
            | Role::TabText
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
    popup: bool,
    modal: bool,
    hidden: bool,
    disabled: bool,
}

/// The name a crate part gives an unlabelled control, English like the crate's other words
/// ("Dismiss", "More"); a host's own [`AccessibleLabel`] replaces it.
const DIVIDER: &str = "Divider";
const COLUMN_EDGE: &str = "Resize column";
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
}

/// The queries a name or a value is read through.
#[derive(SystemParam)]
pub(crate) struct Look<'w, 's> {
    texts: Query<'w, 's, &'static Text>,
    roles: Query<'w, 's, &'static Role>,
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static ChildOf>,
    values: Query<'w, 's, &'static TextInputValue>,
    sections: Query<'w, 's, &'static SectionOpen>,
    fractions: Query<'w, 's, &'static SplitFraction>,
    trees: Query<'w, 's, (), With<TreeView>>,
    tooltips: Query<'w, 's, &'static Tooltip>,
}

impl Look<'_, '_> {
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
        }
        Role::ToastAction => wanted.name = look.words(entity, names),
        Role::Table => {
            wanted.role = Some(if look.trees.contains(entity) {
                Ak::Tree
            } else {
                Ak::Table
            });
        }
        Role::TableRow => {
            let tree = facts.row.is_some_and(|row| look.trees.contains(row.table));
            wanted.role = Some(if tree { Ak::TreeItem } else { Ak::Row });
            wanted.selected = Some(facts.selected);
        }
        Role::TableCell => {
            wanted.role = Some(Ak::Cell);
            wanted.name = look.words(entity, names);
        }
        Role::TableHeaderCell | Role::Tab => wanted.name = look.words(entity, names),
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
    wanted.disabled = facts.disabled;
    wanted.hidden = facts.seen.is_some_and(|seen| !seen.get())
        || facts.node.is_some_and(|node| node.display == Display::None);
    Some(wanted)
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
        && (!wanted.popup || node.has_popup() == Some(HasPopup::Menu))
        && node.is_modal() == wanted.modal
        && node.is_hidden() == wanted.hidden
        && node.is_disabled() == wanted.disabled
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
