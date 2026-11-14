//! Ready-made widgets: free functions returning a bundle, so they compose inside `children![]` and
//! with [`crate::observe`]. Each carries a [`Role`], and [`crate::apply_skin`] styles it from the
//! skin; none of them takes a colour or a size.

use std::ops::RangeInclusive;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::relationship::RelatedSpawner;
use bevy::ecs::spawn::{SpawnIter, SpawnRelatedBundle, SpawnWith, SpawnableList};
use bevy::ecs::world::DeferredWorld;
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::picking::cursor::EntityCursor;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{EditableText, Justify, LineBreak, TextCursorStyle, TextEdit};
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{Checked, FixedNode};
use bevy::ui_widgets::{
    Checkbox, ControlOrientation, ScrollArea, Scrollbar, ScrollbarThumb, SelectAllOnFocus, Slider,
    SliderOrientation, SliderRange, SliderStep, SliderThumb, SliderValue, TabActivation, TabList,
    TextInput, TrackClick, checkbox_self_update, observe, slider_self_update,
};
use bevy::window::SystemCursorIcon;

use crate::binding::{
    Binding, BindingValue, binding_on_activate, binding_on_key_input, binding_self_update,
};
use crate::card::Card;
use crate::choice::{
    ChoiceCount, ChoiceValue, StepBy, arrow_on_click, arrow_on_press, choice_on_activate,
    choice_on_key_input, choice_on_move, choice_self_update,
};
use crate::color::{
    ColorSwatch, ColorValue, SwatchHalf, color_on_key_input, color_on_text, color_self_update,
    swatch_on_activate,
};
use crate::combobox::ComboOption;
use crate::dock::Dock;
use crate::dock_layout::DockLayout;
use crate::hold::{
    HoldToConfirm, hold_on_cancel, hold_on_drag_end, hold_on_press, hold_on_release,
};
use crate::icon::{Icon, IconSource};
use crate::led::{Led, LedState, LedWord};
use crate::list::ListView;
use crate::menu::{MenuBar, MenuItem, MenuTitle, bar_on_cancel, menu_panel};
use crate::meter::Meter;
use crate::model::{
    PadStyle, format_number, hex_text, meter_fraction, radial_place, slider_fraction, slider_text,
};
use crate::nav::Dialog;
use crate::number::{
    NumberInput, NumberRange, NumberStep, NumberValue, VecPart, VecValue, number_on_cancel,
    number_on_drag, number_on_drag_end, number_on_drag_start, number_on_key_input, number_on_step,
    number_on_text, number_self_update, vec_part_on_change,
};
use crate::options::{OptionRow, option_on_change};
use crate::palette::{
    CommandPalette, NO_MATCH, PALETTE_HINT, PALETTE_ROWS, PaletteCount, PaletteInput, PaletteNote,
    PaletteRow, palette_on_acquire, palette_on_activate, palette_on_cancel, palette_on_click,
    palette_on_move,
};
use crate::popover::{
    Placement, PopoverPanel, placement_of, popover_on_cancel, stop_acquire, stop_click, stop_press,
    stop_release,
};
use crate::progress::{Progress, Spinner};
use crate::prompt::{Prompt, PromptsAnchor};
use crate::radial::{
    RadialCount, RadialItem, RadialSlot, RadialValue, radial_on_activate, radial_on_key_input,
    radial_on_pointer_move, radial_on_press, radial_self_update,
};
use crate::region::FocusRegion;
use crate::section::{SectionHeader, SectionOpen, header_on_activate, section_self_update};
use crate::select::{SELECT_HINT, SELECT_NO_MATCH, Select, select_on_activate};
use crate::selection_indicator::{Indicating, selection_indicator};
use crate::skin::Role;
use crate::slots::{Slot, SlotGrid, SlotItems};
use crate::split::{
    Split, SplitAxis, SplitDivider, collapse_split, divider_on_cancel, divider_on_click,
    divider_on_double_press, divider_on_drag, divider_on_drag_end, divider_on_drag_start,
    divider_on_key_input, divider_on_press, divider_on_release, split_self_update, toggle_split,
};
use crate::status::{StatusBar, StatusGroup, StatusItem};
use crate::table::{Column, RowSource, table_bundle};
use crate::tabs::{TabsCount, TabsValue, tabs_on_select, tabs_self_update};
use crate::text::{
    AreaField, TextHint, TextInputOf, TextInputValue, text_on_key_input, text_row_on_acquire,
    text_self_update,
};
use crate::toast::{ToastBundle, ToastKind};
use crate::toolbar::{
    GroupIndex, GroupOption, OptionContent, ToggleGroupCount, ToggleGroupValue, Toolbar,
    group_self_update, option_on_activate,
};
use crate::tooltip::Tooltip;
use crate::tree::{Tree, TreeNode, TreeView};

/// The label of a control (the words beside a toggle, a slider, a choice or a binding row).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ControlText;

/// A control's label is one line: the row is as wide as its label and its control need, so a label
/// that wraps was wrapped by a rounding error in the row's width (a label 210 px wide in a row that
/// came out a hair under 210 stood on two lines at 125% text, and the row was twice as tall).
pub(crate) const ONE_LINE: TextLayout = TextLayout::new(Justify::Left, LineBreak::NoWrap);

/// Lets a control's label break where something else bounds it: inside a dialog.
///
/// A control's label is [`ONE_LINE`] because its row is as wide as its label and its control need,
/// and that is how a panel sized by its widest child comes out right. A dialog's panel is bounded
/// instead (a `Role::Panel` under a `Role::Scrim` is given a `max_width`), and a label that cannot
/// shrink runs out of it: measured, a **708 px** slider label in a row ending at 976 px started at
/// 328 and ended at **1036**, 60 px outside a panel at 280..1000. So a label under a dialog's panel
/// takes `WordBoundary` and the rest keep their exact one-line width.
///
/// It runs after [`crate::apply_skin`], which writes the rest of the text. Breaking is not free — a
/// flex item's automatic minimum drops to its longest word, so the same label shrank to 210 px in a
/// 539 px row and wrapped 3 px short of its 213 px line — which is why it is scoped to a dialog and
/// not the label's bundle.
pub(crate) fn wrap_bounded_labels(
    mut labels: Query<(&ChildOf, &mut TextLayout), With<ControlText>>,
    roles: Query<&Role>,
    parents: Query<&ChildOf>,
) {
    for (parent, mut layout) in &mut labels {
        let mut bounded = false;
        let mut up = Some(parent.parent());
        let mut guard = 0;
        while let Some(entity) = up {
            guard += 1;
            if guard > 16 {
                break;
            }
            if roles.get(entity) == Ok(&Role::Scrim) {
                bounded = true;
                break;
            }
            up = parents.get(entity).map(|p| p.parent()).ok();
        }
        let linebreak = if bounded {
            LineBreak::WordBoundary
        } else {
            LineBreak::NoWrap
        };
        // `TextLayout` is not `PartialEq`; the one field that differs is compared itself.
        if layout.linebreak != linebreak {
            layout.linebreak = linebreak;
        }
    }
}

/// A control's label, as a control row draws it: the words, [`Role::ControlText`] and the marker
/// [`wrap_bounded_labels`] writes through.
pub(crate) fn wrapping_label(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ONE_LINE,
        Role::ControlText,
        ControlText,
        Pickable::IGNORE,
    )
}

/// How many keyboard or D-pad steps span a [`slider`]'s range.
const SLIDER_STEPS: f32 = 20.0;

/// The least length of a list's thumb, in logical px (about half a comfortable target), so a long
/// list leaves something to grab.
const LIST_MIN_THUMB: f32 = 24.0;

/// A [`split`] divider's grip, the pill in the middle of its channel: its thickness and its length,
/// in logical px. A design default: a quiet pill, the length of a thumb's grab.
const SPLIT_GRIP_THICKNESS: f32 = 4.0;
const SPLIT_GRIP_LENGTH: f32 = 40.0;

/// How many characters of the body text a [`text_input`]'s field shows at once ("0"'s width each):
/// 16 and one more, because Bevy keeps one character of room after the caret for the next one to be
/// typed, so a name that fills a [`TextLimit`](crate::TextLimit) of 16 shows whole with its caret
/// instead of scrolling its first letter away. Longer text scrolls in it, so the row never changes
/// size.
const TEXT_INPUT_CHARS: f32 = 17.0;

/// How many character advances wide a [`text_area`]'s field is: a text input's and a half.
const TEXT_AREA_CHARS: f32 = 26.0;

/// How far a filling area's rail stands from its field, logical px: the lane is beside the text, not
/// over it. A layout gap is the builder's, like every other `Node` in a text row; the rail's own look
/// stays `apply_skin`'s.
const AREA_RAIL_GAP_PX: f32 = 4.0;

/// A text field's caret, as a share of the font size: 2 px at the body size, a hair thinner than
/// Bevy's 0.2 so it does not cover a letter's stem.
const TEXT_CARET_WIDTH: f32 = 0.1;

/// How many characters of the body text a [`number_field`]'s field shows: "-12345.67" and the
/// caret's room. A number that needs more scrolls in it, so the row never changes size while the
/// player scrubs.
const NUMBER_FIELD_CHARS: f32 = 10.0;

/// The same for each component of a [`vec_field`], three of which stand in a row.
const VEC_FIELD_CHARS: f32 = 8.0;

/// How many characters wide a colour field is: `#RRGGBBAA` and the room Bevy keeps after a caret at its end.
const COLOR_FIELD_CHARS: f32 = 10.0;

/// How far a choice's chevron is moved to put its ink in the middle of its arrow, in percent of
/// its own width. A square turned an eighth of a turn has its two drawn sides on one side of its
/// centre: they run from the centre line out to the corner, which is half a diagonal away, so
/// the ink's middle is a quarter of the diagonal, `1 / (2 √2)` of the side, off the centre.
pub(crate) const CHEVRON_INK_SHIFT: f32 = 50.0 * std::f32::consts::FRAC_1_SQRT_2;

/// A toggle's state at spawn. A bundle cannot carry [`Checked`] only when the toggle starts on
/// (`Option<Checked>` is not a bundle), so this puts it on the entity as it is added and goes.
#[derive(Component)]
#[component(on_add = start_state)]
struct StartsOn(bool);

fn start_state(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let on = world.get::<StartsOn>(entity).is_some_and(|start| start.0);
    let mut commands = world.commands();
    let mut toggle = commands.entity(entity);
    toggle.remove::<StartsOn>();
    if on {
        toggle.insert(Checked);
    }
}

/// A full-window column that centres its children: the root of a menu screen or a HUD. It is a tab
/// group, so Tab cycles through the buttons on it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((screen(), children![label("Hello")]));
/// # }
/// ```
///
/// It is layout, not a surface: its empty part is [`Pickable::IGNORE`], so a HUD built on it lets
/// the pointer through to the sprites, meshes and other UI roots under it, while the widgets on it
/// ([`panel`], [`button`], [`meter`]…) still take the pointer. A click on the empty part still
/// blurs the interface: it lands on whatever is below, or on the window, and Bevy clears focus
/// there. A menu screen that must swallow the pointer opts back in on the entity (a tuple bundle
/// cannot carry two `Pickable`s, so `(screen(), Pickable::default())` panics):
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(screen()).insert(Pickable::default());
/// # }
/// ```
pub fn screen() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Role::Screen,
        TabGroup::new(0),
        Pickable::IGNORE,
    )
}

/// A framed, padded column on the skin's surface colour. One standing directly on a [`screen`] is as
/// wide as its widest row up to the window less two gaps, then wraps; a panel inside a dock group, a
/// [`split`] pane or a dialog is sized by that container and has no such cap.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((panel(), children![heading("Inventory"), label("Empty")]));
/// # }
/// ```
pub fn panel() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::Panel,
    )
}

/// A [`panel`] with a width of its own, for a dialog's card or a form on a screen: logical px
/// ([`CardWidth::Px`](crate::CardWidth::Px)) or body text sizes
/// ([`CardWidth::Text`](crate::CardWidth::Text), which follows the player's text size). It is the
/// width and nothing else: a card never outgrows the room it stands in (`max_width: 100%`) and has
/// no least width, so it may be narrower than a dialog's usual 21 text sizes. A direct child of a
/// [`dialog`] is still the dialog; its frame, padding and fill are unchanged.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     dialog(),
///     children![(sized_panel(CardWidth::Px(520.0)), children![heading("New project"), button("Create")])],
/// ));
/// # }
/// ```
pub fn sized_panel(width: crate::CardWidth) -> impl Bundle {
    (panel(), width)
}

/// A column filled with one step of the skin's surface ladder, [`crate::GooeySkin::surfaces`], and
/// nothing else: no padding, frame or corner, which a host takes from the skin's tokens
/// ([`crate::GooeySkin::space`], [`crate::GooeySkin::corner`]). `level` 0 is the lowest step; a level past the top
/// is the top.
///
/// A tool nests its grounds: a panel's ground, a row on it, a field in the row, each a step above the
/// one it stands in.
///
/// A bundle cannot carry a `Node` twice: for a surface with its own padding, size or corners, spawn
/// your own `Node` with [`Role::Surface`] instead (`(Role::Surface(2), Node { padding: .. , ..default() })`).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((surface(1), children![(surface(2), children![label("A row")])]));
/// # }
/// ```
pub fn surface(level: u8) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::Surface(level),
    )
}

/// A row of widgets with the skin's gap between them: a toolbar's line, a label beside its value, a
/// list row. The gap is `gap` multiples of [`GooeySkin::gap`](crate::GooeySkin::gap), so `row(1.0)`
/// sits on the same rhythm as a panel's own children; `row(0.0)` is no gap.
///
/// A row is as wide as the room it is given and never pushes out of it: when its children do not
/// fit, every [`cell`] in it shrinks by the same share of its width (`flex_shrink: 1` on each, which
/// flexbox divides in proportion to their widths), and an [`image_box`] never shrinks at all, so a
/// picture keeps its size and the cells give way around it. A cell that shrinks cuts what does not
/// fit rather than running onto its neighbour.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((row(1.0), children![icon_button(IconSource::Image(default()), Tooltip::new("Grid")), button("Bake")]));
/// # }
/// ```
pub fn row(gap: f32) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            ..default()
        },
        Role::Row,
        crate::LayoutGap(gap),
    )
}

/// A [`row`] with a [`RowLayout`](crate::RowLayout): the cross-axis alignment, the wrap, the main-axis spread and the
/// shrink a host used to patch over a spawned row. It is one bundle, so it stands where a patched `Node`
/// cannot: in a `children![..]` list or a `Children::spawn(SpawnWith(..))`.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     row_with(
///         1.0,
///         RowLayout { align: Some(AlignItems::Center), wrap: Some(FlexWrap::Wrap), ..default() },
///     ),
///     children![label("Modified;"), button("Reset")],
/// ));
/// # }
/// ```
pub fn row_with(gap: f32, layout: crate::RowLayout) -> impl Bundle {
    (row(gap), layout)
}

/// A [`column()`] with a [`RowLayout`](crate::RowLayout): `align` is across the column's width (its children stretch
/// by default), `justify` along its height.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     column_with(1.0, RowLayout { align: Some(AlignItems::Center), ..default() }),
///     children![heading("Saved"), label("All changes are on disk")],
/// ));
/// # }
/// ```
pub fn column_with(gap: f32, layout: crate::RowLayout) -> impl Bundle {
    (column(gap), layout)
}

/// A row of buttons: `gap` multiples of the skin's gap apart (a tight `0.5` is the PLAY, BUILD
/// line), centred on one line, wrapping onto the next when the room is too narrow for them, a
/// gap below. A [`RowLayout`](crate::RowLayout) beside it changes only the fields it sets (`Some`): `justify` alone
/// keeps the centring and the wrapping.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((button_row(0.5), children![button("PLAY"), button("BUILD")]));
/// # }
/// ```
pub fn button_row(gap: f32) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            ..default()
        },
        Role::ButtonRow,
        crate::LayoutGap(gap),
    )
}

/// A stack of rows, the skin's gap between them: a panel's body, a form, a list of readings.
/// `gap` is in multiples of [`GooeySkin::gap`](crate::GooeySkin::gap), as for [`row`]. Its children
/// stretch to its width; like a [`row`] it is as wide as the room it is given and no wider, so a
/// long row inside it is cut rather than pushing the column past its host.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((column(1.0), children![label("Rig"), row(1.0), label("Slots")]));
/// # }
/// ```
pub fn column(gap: f32) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            ..default()
        },
        Role::Column,
        crate::LayoutGap(gap),
    )
}

/// A column that is a flex item of its own: it takes the room its row leaves, gives way to its
/// neighbours down to nothing (`min_width` 0), and stacks its children `gap` multiples of the
/// skin's gap apart, stretched across it. Text in it wraps inside it, so a name and its details
/// stand in a column beside a fixed picture or a [`count_text`] and never push the row out.
/// [`fill_text`] is a text that does this alone; this is the column that holds several parts.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     row(1.0),
///     children![
///         glyph_button(">", "Open"),
///         (fill_column(0.5), children![heading("agent_view.rs"), label("12 problems in this file")]),
///     ],
/// ));
/// # }
/// ```
pub fn fill_column(gap: f32) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            ..default()
        },
        Role::FillColumn,
        crate::LayoutGap(gap),
    )
}

/// A vertical region that takes the height its column leaves and can shrink to nothing
/// (`flex_grow` 1, `min_height` 0), for what holds a [`tree`] or a [`list`]: those scroll inside
/// it, so it is the viewport and a tall tree never makes its host taller. A flex item of a column;
/// its children stretch across it, and a [`RowLayout`](crate::RowLayout) beside it centres or
/// spreads them (a loading screen's centred cell).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     column(1.0),
///     children![
///         heading("Files"),
///         (fill_region(), children![tree([TreeNode::new(1, "src")])]),
///     ],
/// ));
/// # }
/// ```
pub fn fill_region() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            ..default()
        },
        Role::FillRegion,
    )
}

/// A node placed in its host by its edges: absolute, so it takes no room in the flow and the host's
/// other children stand where they would without it, `inset` from the host's sides. A side left
/// [`Val::Auto`] is not pinned (`UiRect::default()` is zero on every side, not `Auto`: start from
/// `UiRect::all(Val::Auto)`), so a node pinned at one corner is the size of what it holds. Use it for
/// a mark, a label or a drag rectangle at a pixel place of a pane; a node that follows a point in
/// the world is a [`WorldAnchor`](crate::WorldAnchor) instead (do not put both on one node).
///
/// It is layout, not a surface: it is [`Pickable::IGNORE`] like [`screen`], so the pointer goes
/// through it to what is under it; for one that must take the pointer, `.insert(Pickable::default())`
/// on the entity after spawning it. A [`RowLayout`](crate::RowLayout) beside it lays out what it holds.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // The compass: 12 px in from the host's top right corner.
/// let corner = UiRect { top: Val::Px(12.0), right: Val::Px(12.0), ..UiRect::all(Val::Auto) };
/// commands.spawn((placed(corner), children![label("N")]));
/// # }
/// ```
pub fn placed(inset: UiRect) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::Placed,
        crate::PlacedAt(inset),
        Pickable::IGNORE,
    )
}

/// A node placed `left` and `top` logical px in from its host's top left corner: a drag rectangle
/// or an area's label. [`placed`] with only those two sides pinned.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((placed_at(30.0, 40.0), children![label("Stage 2")]));
/// # }
/// ```
pub fn placed_at(left: f32, top: f32) -> impl Bundle {
    placed(UiRect {
        left: Val::Px(left),
        top: Val::Px(top),
        ..UiRect::all(Val::Auto)
    })
}

/// A node that fills its host: absolute at `0` on every side, so it takes no room and covers
/// exactly the rectangle its host has, whatever size that is: an empty state over a viewport, a
/// scrim over a pane. [`placed`] with every side pinned at zero; like it, `Pickable::IGNORE`.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     cover(),
///     RowLayout { align: Some(AlignItems::Center), justify: Some(JustifyContent::Center), ..default() },
///     children![label("Nothing here yet")],
/// ));
/// # }
/// ```
pub fn cover() -> impl Bundle {
    placed(UiRect::all(Val::ZERO))
}

/// Nothing, that takes the room its row or column leaves: `flex_grow` 1 with no content, so what
/// is before it stands at the start and what is after it at the end, and two of them centre what is
/// between. It draws nothing and is `Pickable::IGNORE` like [`screen`] (it is a gap, not a surface).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     row(1.0),
///     children![button("Back"), spacer(), button("Next")],
/// ));
/// # }
/// ```
pub fn spacer() -> impl Bundle {
    (Node::default(), Role::Spacer, Pickable::IGNORE)
}

/// A [`fill_region`] that stacks its children `gap` multiples of the skin's gap apart (a page with a
/// group gap): `fill_region` is `fill_region_with(0.0)`. The gap is the same [`LayoutGap`](crate::LayoutGap)
/// a [`column`] has. Pair it with [`Shown`](crate::Shown) for a page that can be hidden.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     fill_region_with(1.0),
///     Shown(true),
///     children![heading("Home"), label("Recent projects")],
/// ));
/// # }
/// ```
pub fn fill_region_with(gap: f32) -> impl Bundle {
    (fill_region(), crate::LayoutGap(gap))
}

/// A plain flex column of a size you give it: a [`Sizing`](crate::Sizing) with a width and a height,
/// their least and greatest, and whether it clips. For a holder whose height is a number or a bar
/// that is the full height of its host with a border (add a [`Shape`](crate::Shape)); a
/// [`row`] or [`column`] takes the same `Sizing` as a component. Not drawn, and it takes the pointer
/// like any container.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     sized(Sizing {
///         width: Some(Extent::Px(48.0)),
///         height: Some(Extent::Percent(100.0)),
///         ..default()
///     }),
///     Shape { border: Sides { right: Some(1.0), ..default() }, ..default() },
///     children![icon_button(IconSource::Image(default()), Tooltip::new("Files"))],
/// ));
/// # }
/// ```
pub fn sized(sizing: crate::Sizing) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::Sized,
        sizing,
    )
}

/// A box of a fixed width, in body text sizes, so a label column lines up down a list of rows: the
/// words start at one edge whatever they say, and what does not fit is cut rather than drawn over
/// the next column. What it holds stands in a row, stretched to its height (a label and a value side
/// by side); put a [`column()`] inside it for a stack.
///
/// A width of zero (or less) collapses the cell: it takes no room, **no gap stands where it would
/// be** (a gap is only drawn between two cells that are there), and nothing it holds is drawn. In a
/// [`row`] narrower than its children every cell shrinks by the same share of its width and they
/// never overlap; an [`image_box`] beside them never shrinks.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((row(1.0), children![(cell(7.0), children![label("0 - clip 14")]), label("free x1.00")]));
/// # }
/// ```
pub fn cell(width: f32) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            ..default()
        },
        Role::Cell,
        crate::CellWidth(width),
    )
}

/// A picture of a fixed size, so a plot, a thumbnail or a chart holds its room while it is being
/// written: the box is `size` logical px whatever its image is, and an image that has not loaded
/// draws nothing and still keeps the box. It is never squeezed by a narrow row: the cells beside it
/// give way first, and it is cut at its own box rather than growing.
///
/// **It takes no pointer** (`Pickable::IGNORE`, as every picture in this crate does), so a plot that
/// reads the cursor keeps a wrapper of its own with `RelativeCursorPosition` and `Hovered` on it, or
/// an [`overlay`]'s first child is that wrapper.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, plot: Handle<Image>) {
/// commands.spawn(image_box(plot, Vec2::new(320.0, 160.0)));
/// # }
/// ```
pub fn image_box(image: Handle<Image>, size: Vec2) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        ImageNode {
            image,
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        Role::ImageBox,
        crate::BoxSize(size),
        Pickable::IGNORE,
    )
}

/// A picture sized by its aspect ratio: `width` logical px wide while its host has the room, and no
/// wider than the host (`max_width: 100%`) when it does not, its height following the width at
/// `aspect` (width over height, `2.0` is twice as wide as high), so a splash shrinks with a narrow
/// window and never distorts. [`image_box`] is a fixed size; this is the one that gives way. Like it,
/// the picture is `Pickable::IGNORE`, and an image that has not loaded draws nothing and keeps the
/// box. [`AspectBox`](crate::AspectBox) is the component to change it afterwards.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, splash: Handle<Image>) {
/// commands.spawn(aspect_image(splash, 480.0, 16.0 / 9.0));
/// # }
/// ```
pub fn aspect_image(image: Handle<Image>, width: f32, aspect: f32) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        ImageNode {
            image,
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        Role::AspectImage,
        crate::AspectBox {
            width,
            ratio: aspect,
        },
        Pickable::IGNORE,
    )
}

/// A box its first child sizes, with every later child stacked over it: a hover image over a plot,
/// a readout over a chart, a badge over a thumbnail. The stacked children fill the box and take no
/// room of their own (the crate stacks them), so the overlay is exactly as large as what it
/// covers.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, plot: Handle<Image>, hover: Handle<Image>) {
/// commands.spawn((overlay(), children![
///     image_box(plot, Vec2::new(320.0, 160.0)),
///     image_box(hover, Vec2::new(320.0, 160.0)),
/// ]));
/// # }
/// ```
pub fn overlay() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            // Its first child gives it its size, so it never stretches to a taller sibling's.
            align_self: AlignSelf::FlexStart,
            ..default()
        },
        Role::Overlay,
    )
}

/// A column of buttons, stretched to the widest.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((menu(), children![button("New game"), button("Continue")]));
/// # }
/// ```
pub fn menu() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            ..default()
        },
        Role::Menu,
    )
}

/// Heading text, in the skin's heading size and font.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(heading("Paused"));
/// # }
/// ```
pub fn heading(text: impl Into<String>) -> impl Bundle {
    (Text::new(text), Role::Heading)
}

/// Body text, in the skin's text size and font.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(label("Health"));
/// # }
/// ```
pub fn label(text: impl Into<String>) -> impl Bundle {
    (Text::new(text), Role::Label)
}

/// A button: Bevy's headless [`bevy::ui_widgets::Button`], reachable by Tab and by arrows and the
/// D-pad, styled by its state. Observe [`crate::Activate`] on it to act.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((button("Resume"), observe(|_: On<Activate>| info!("resume"))));
/// # }
/// ```
pub fn button(text: impl Into<String>) -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        bevy::ui_widgets::Button,
        Role::Button,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        children![(Text::new(text), Role::ButtonText)],
    )
}

/// The screen's one main action ("Create", "Start", "Save"): a [`button`] filled with
/// [`crate::GooeySkin::accent`] and labelled in [`crate::GooeySkin::accent_text`] at rest, so a
/// first-time player sees the one thing to press. Under the pointer it is
/// [`crate::GooeySkin::primary_hovered`], held down [`crate::GooeySkin::primary_pressed`], and
/// disabled it is dimmed like any button. It is a [`button`] with the [`crate::PrimaryButton`]
/// marker, so focus, `Activate`, a screen reader and every test that finds a button treat it as one;
/// put the marker on any other button to make that the main action instead. Give a screen one.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     menu(),
///     children![
///         (primary_button("New project"), observe(|_: On<Activate>| info!("new"))),
///         button("Open..."),
///     ],
/// ));
/// # }
/// ```
pub fn primary_button(text: impl Into<String>) -> impl Bundle {
    (button(text), crate::PrimaryButton)
}

/// A button that confirms only when it is held, for an act the player cannot take back (delete a
/// save, quit without saving, abandon a run). Hold Enter, Space or the pad's South while it has
/// focus, or the primary pointer button while the pointer is over it, for `seconds`: a bar under
/// the label fills at an even rate and, at the end, one [`crate::HoldConfirmed`] is sent. Letting
/// go early (or the pointer leaving, or focus moving) confirms nothing and the bar unwinds four
/// times as fast as it filled; a new press resumes from where it stands.
///
/// It is a [`button`] to the skin and sends no [`crate::Activate`]: observe
/// [`crate::HoldConfirmed`]. A hold is a motor barrier (XAG 107), so the player can shorten it or
/// replace it with a tap through [`crate::HoldSettings`], and a screen reader's Click confirms at
/// once. Put what the button does in its label ("Hold to delete"): the bar is the only sign.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     hold_button("Hold to delete save", 1.2),
///     observe(|_: On<HoldConfirmed>| info!("deleted")),
/// ));
/// # }
/// ```
pub fn hold_button(text: impl Into<String>, seconds: f32) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        HoldToConfirm { seconds },
        Role::Button,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        bevy::a11y::AccessibilityNode(accesskit::Node::new(accesskit::Role::Button)),
        observe(hold_on_press),
        observe(hold_on_release),
        observe(hold_on_drag_end),
        observe(hold_on_cancel),
        children![
            (Text::new(text), Role::ButtonText),
            (
                Node::default(),
                Role::HoldTrack,
                children![(Node::default(), Role::HoldFill)]
            )
        ],
    )
}

/// A HUD meter (health, stamina, progress): a framed track with a fill `value / max` of its length.
/// The frame marks the whole length against the panel; the fill sits one border inside it, with
/// concentric rounded corners. It is [`crate::GooeySkin::meter_height`] high times the text scale,
/// so it grows with the player's size option. Change its [`Meter`] component to move it: a loss
/// drops the fill at once and leaves the lost length behind it, in the skin's danger colour, for
/// a moment before it drains; a gain eases up ([`crate::MotionSettings::off`] makes both instant).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(meter(75.0, 100.0));
/// # }
/// ```
pub fn meter(value: f32, max: f32) -> impl Bundle {
    (
        Node::default(),
        Role::Meter,
        Meter { value, max },
        children![
            (
                Node {
                    width: Val::Percent(meter_fraction(value, max) * 100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                // Over the trail, which starts at the bar's left edge and shows only past the fill.
                ZIndex(1),
                Role::MeterFill
            ),
            (
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                Role::MeterTrail
            )
        ],
    )
}

/// Work that takes ten seconds or more (a save, a bake, a download): a track like a meter's with
/// its fill `fraction` of the length (clamped to `0.0..=1.0`; anything else is reported once, see
/// [`crate::ProgressClamped`]) and, under it, the words of its [`crate::ProgressText`]: which step
/// of how many and about how long is left, "3 of 7: Measuring meshes, about 20 s left". Change its
/// [`crate::Progress`] to move it; edit the `ProgressText` in place.
///
/// Pick the indicator by how long the work takes (Nielsen's response-time limits): under about a
/// second nothing, a [`spinner`] for two to ten seconds, this for ten and more. There is no static
/// variant.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     progress(3.0 / 7.0),
///     ProgressText {
///         step: Some((3, 7, "Measuring meshes".into())),
///         eta_seconds: Some(20.0),
///     },
/// ));
/// # }
/// ```
pub fn progress(fraction: f32) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::Progress,
        Progress(fraction),
        children![
            (
                Node::default(),
                Role::Meter,
                children![(
                    Node {
                        width: Val::Percent(meter_fraction(fraction, 1.0) * 100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    Role::MeterFill
                )],
            ),
            (
                // No width of its own and the bar's width as its least: the words never decide
                // how wide the bar is, so it keeps its size as they change, and a long line
                // wraps inside it.
                Node {
                    display: Display::None,
                    width: Val::Px(0.0),
                    min_width: Val::Percent(100.0),
                    ..default()
                },
                Text::new(String::new()),
                Role::ProgressLine,
            ),
        ],
    )
}

/// Work whose length is not known, or that takes two to ten seconds: a ring with one lit quarter
/// turning once every [`crate::model::SPINNER_PERIOD_SECS`] on real time, so it keeps turning
/// while the game is paused. For ten seconds and more use a [`progress`]; there is no static
/// variant.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(spinner());
/// # }
/// ```
pub fn spinner() -> impl Bundle {
    (Node::default(), Role::Spinner, Spinner)
}

/// A small image the skin sizes and tints: `source` is the host's own art, a whole image or one
/// cell of an atlas. It is a step of the skin's [`crate::GooeySkin::type_scale`] square (the
/// body's line by default, [`crate::IconSize`]), so it is as tall as the text beside it at any text
/// size, and tinted by its [`crate::IconTone`]: the image is multiplied by a colour of the skin,
/// so draw it white on transparent. Inside a [`button`] it follows the button's state as the text
/// does. An image that is missing draws an empty box of the right size and reports once
/// ([`crate::IconMissing`]).
///
/// An icon takes no input and no focus, so a click on one lands on the button it is in. Put a
/// visible label beside it: an icon alone is a guess.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, assets: Res<AssetServer>) {
/// let delete = commands.spawn(button("Delete")).id();
/// commands.spawn((
///     icon(IconSource::Image(assets.load("trash.png"))),
///     IconTone::Danger,
///     ChildOf(delete),
/// ));
/// # }
/// ```
pub fn icon(source: IconSource) -> impl Bundle {
    (Node::default(), Role::Icon, Icon(source), Pickable::IGNORE)
}

/// A switch for an option that takes effect the moment it flips (Fullscreen, Subtitles): a row with
/// the label on the left and a framed track with a sliding knob on the right. It is Bevy's headless
/// [`bevy::ui_widgets::Checkbox`], so a click anywhere on the row, Enter, Space and the pad's South
/// button flip it, and Tab, the arrows and the D-pad reach it. It keeps its own state, [`Checked`]
/// on the entity; observe `ValueChange<bool>` on it to apply the setting. The knob's place says
/// whether it is on, so the state survives a colour-blind player; the label never changes with it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     toggle("Subtitles", true),
///     observe(|change: On<ValueChange<bool>>| info!("subtitles: {}", change.value)),
/// ));
/// # }
/// ```
pub fn toggle(label: impl Into<String>, on: bool) -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
        Checkbox,
        StartsOn(on),
        Role::Toggle,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        observe(checkbox_self_update),
        children![
            wrapping_label(label),
            (
                Node {
                    align_items: AlignItems::Center,
                    ..default()
                },
                Role::ToggleTrack,
                children![(Node::default(), Role::ToggleKnob)],
            ),
        ],
    )
}

/// A slider for a value the player scrubs (volume, brightness, sensitivity): a row with the label
/// and the value on one line and a rail under them with a thumb on it. It is Bevy's headless
/// [`bevy::ui_widgets::Slider`], and the whole row is the target: a click anywhere on it puts the
/// thumb under the pointer and a drag goes on from there. With it focused, Left and Right (keys or
/// the D-pad, held to repeat) step the value by a twentieth of the range, Home and End jump to the
/// ends, and Up and Down still move focus on, so one press is enough on a pad.
///
/// The value reads as a percentage for a range of 0 to 1 and as the number otherwise
/// ([`crate::slider_text`]). It keeps its own value, [`SliderValue`] on the entity; observe
/// `ValueChange<f32>` on it to apply the setting as it changes (`is_final` is false while a drag is
/// going). To step by something else, insert a [`SliderStep`] after spawning.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     slider("Music", 0.7, 0.0..=1.0),
///     observe(|change: On<ValueChange<f32>>| info!("music: {}", change.value)),
/// ));
/// # }
/// ```
pub fn slider(label: impl Into<String>, value: f32, range: RangeInclusive<f32>) -> impl Bundle {
    let (min, max) = range.into_inner();
    let fraction = slider_fraction(value, min, max);
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Slider {
            track_click: TrackClick::Snap,
            orientation: SliderOrientation::Horizontal,
        },
        SliderValue(value),
        SliderRange::new(min, max),
        SliderStep((max - min) / SLIDER_STEPS),
        Role::Slider,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        observe(slider_self_update),
        children![
            (
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Role::SliderHeader,
                children![
                    wrapping_label(label),
                    (
                        Text::new(slider_text(value, min, max)),
                        ONE_LINE,
                        Role::SliderReadout
                    ),
                ],
            ),
            (
                Node {
                    align_items: AlignItems::Center,
                    ..default()
                },
                Role::SliderTrack,
                children![
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        Role::SliderRail,
                        children![(
                            Node {
                                width: Val::Percent(fraction * 100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            Role::SliderFill,
                        )],
                    ),
                    (
                        Node {
                            flex_grow: fraction,
                            ..default()
                        },
                        Role::SliderGap,
                    ),
                    (Node::default(), Role::SliderThumb, SliderThumb),
                ],
            ),
        ],
    )
}

/// A choice for a setting that takes one of a few named values (Low, Medium, High; Easy, Normal,
/// Hard): a row with the label on the left and the selected option on the right between two
/// arrows. It is Bevy's headless [`bevy::ui_widgets::Button`] with a state, so Tab, the arrows and
/// the D-pad reach it, and with it focused Left and Right (keys or the D-pad, held to repeat) step
/// to the previous and next option and stop at the ends, Home and End jump to the first and last,
/// and Up and Down still move focus on, so a pad steps a column of settings with no mode to enter.
/// A click on an arrow steps once, and Enter, Space, the pad's South button and a click anywhere
/// else on the row cycle to the next option, wrapping from the last to the first, so every option is
/// reachable by one repeated press. The arrow that cannot step is dimmed.
///
/// Every option takes the same room, the widest's, so stepping never moves the arrows under the
/// player's thumb. It keeps its own state, [`ChoiceValue`] on the entity (the index of the
/// selected option); observe `ValueChange<usize>` on it to apply the setting when it changes.
/// Observe that, not `Activate`: only a cycle is an `Activate`, a step is not. A `selected` past the
/// last option reads as the last.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     choice("Quality", ["Low", "Medium", "High"], 1),
///     observe(|change: On<ValueChange<usize>>| info!("quality: option {}", change.value)),
/// ));
/// # }
/// ```
pub fn choice<S: Into<String>>(
    label: impl Into<String>,
    options: impl IntoIterator<Item = S>,
    selected: usize,
) -> impl Bundle {
    let options: Vec<String> = options.into_iter().map(Into::into).collect();
    let count = options.len();
    let selected = selected.min(count.saturating_sub(1));
    (
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
        bevy::ui_widgets::Button,
        ChoiceValue(selected),
        ChoiceCount(count),
        Role::Choice,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        observe(choice_self_update),
        observe(choice_on_move),
        observe(choice_on_activate),
        observe(choice_on_key_input),
        children![
            wrapping_label(label),
            (
                Node {
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    choice_arrow(-1),
                    (
                        Node {
                            display: Display::Grid,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Role::ChoiceCell,
                        Pickable::IGNORE,
                        Children::spawn(SpawnIter(options.into_iter().enumerate().map(
                            move |(index, text)| {
                                (
                                    Node {
                                        grid_row: GridPlacement::start(1),
                                        grid_column: GridPlacement::start(1),
                                        justify_self: JustifySelf::Center,
                                        ..default()
                                    },
                                    Text::new(text),
                                    ONE_LINE,
                                    Role::ChoiceOption,
                                    Pickable::IGNORE,
                                    if index == selected {
                                        Visibility::Inherited
                                    } else {
                                        Visibility::Hidden
                                    },
                                )
                            },
                        ))),
                    ),
                    choice_arrow(1),
                ],
            ),
        ],
    )
}

/// One of the options screen's rows for the settings this crate itself reads: a [`choice`] over
/// the steps of `kind`, which edits one field of its resource and shows it.
fn option_row(kind: OptionRow, label: impl Into<String>) -> impl Bundle {
    (
        choice(label, kind.options(), 0),
        kind,
        observe(option_on_change),
    )
}

/// The player's **text size**: a [`choice`] row over 100%, 125%, 150% and 200%
/// ([`TEXT_SIZE_STEPS`](crate::TEXT_SIZE_STEPS)) that sets
/// [`GooeySkin::text_scale`](crate::GooeySkin::text_scale), so every
/// heading, label, button, dialog, toast and legend grows the frame the player steps it (WCAG 2.2 SC
/// 1.4.4 and Xbox Accessibility Guideline 101 ask for 200%). `label` is the row's name, in the
/// host's language.
///
/// The skin is the one place the setting lives. The row moves to the step nearest the skin's
/// `text_scale` the frame it is spawned and whenever the skin changes, with no
/// [`ValueChange`](crate::ValueChange), so a
/// loaded save shows on it; a value between two steps shows the nearest and is left alone until
/// the player steps. A player's step writes `text_scale` and no other field. It is a [`choice`] in
/// every other way: it takes `Tooltip`, `InteractionDisabled` and a game's own
/// `ValueChange<usize>` observer (the index into `TEXT_SIZE_STEPS`), and a game that wants other
/// steps writes a `choice` and its own observer. Needs [`GooeyStylePlugin`](crate::GooeyStylePlugin).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((panel(), children![heading("Display"), text_size("Text size")]));
/// # }
/// ```
pub fn text_size(label: impl Into<String>) -> impl Bundle {
    option_row(OptionRow::TextSize, label)
}

/// The player's **motion**: a [`choice`] row over Full, Reduced and Off
/// ([`MOTION_STEPS`](crate::MOTION_STEPS)) that sets [`MotionSettings`](crate::MotionSettings).
/// Reduced stops everything that travels, scales or turns and keeps fades
/// ([`MotionSettings::reduced`](crate::MotionSettings::reduced)); Off stops fades too (WCAG 2.2
/// SC 2.3.3). `label` is the row's name, in the host's language; the steps are English words, and
/// a host that localises writes its own [`choice`] with
/// [`with_motion_step`](crate::with_motion_step).
///
/// It behaves as [`text_size`] does: the resource is the one place the setting lives, the row
/// follows it with no event (a hand-written scale shows the step it is in,
/// [`motion_step`](crate::motion_step)), and a step writes `MotionSettings` and nothing else. Full
/// and Reduced keep the player's own `duration_scale`.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((panel(), children![heading("Accessibility"), motion_row("Motion")]));
/// # }
/// ```
pub fn motion_row(label: impl Into<String>) -> impl Bundle {
    option_row(OptionRow::Motion, label)
}

/// The player's **interface size**: a [`choice`] row over 75%, 100%, 125% and 150%
/// ([`INTERFACE_SIZE_STEPS`](crate::INTERFACE_SIZE_STEPS)) that sets
/// [`UiScaleSettings::scale`](crate::UiScaleSettings::scale),
/// which enlarges everything, where [`text_size`] enlarges text alone: on top of the window's own
/// share, so 125% is 1.25 times what the window would draw. `label` is the row's name, in the host's
/// language.
///
/// It behaves as [`text_size`] does: the scale is the one place the setting lives, the row follows
/// it with no event, and a step writes `scale` and nothing else (not `reference_short_side`).
/// Needs [`GooeyStylePlugin`](crate::GooeyStylePlugin) for the row and
/// [`GooeyScalePlugin`](crate::GooeyScalePlugin) for the setting; without the second the row steps
/// and nothing is drawn differently.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((panel(), children![heading("Display"), interface_size("Interface size")]));
/// # }
/// ```
pub fn interface_size(label: impl Into<String>) -> impl Bundle {
    option_row(OptionRow::InterfaceSize, label)
}

/// The player's **controller**: a [`choice`] row over Xbox, PlayStation and Switch
/// ([`PAD_STYLES`](crate::PAD_STYLES)) that sets
/// [`BindingSettings::pad_style`](crate::BindingSettings::pad_style), so every
/// [`binding`] row and [`prompts`] legend names a pad's buttons as the player's controller prints
/// them ("A" or "Cross", "B" or "Circle"). `label` is the row's name, in the host's language; the
/// options are the makers' names. The crate does not detect a pad's make: the setting starts at the
/// host's [`BindingSettings`](crate::BindingSettings) (Xbox by default) and this row is how the
/// player changes it.
///
/// It behaves as [`text_size`] does: the resource is the one place the setting lives, the row
/// follows it with no event, and a step writes `pad_style` and nothing else.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((panel(), children![heading("Controls"), pad_style("Controller")]));
/// # }
/// ```
pub fn pad_style(label: impl Into<String>) -> impl Bundle {
    option_row(OptionRow::PadStyle, label)
}

/// The player's **theme**: a [`choice`] row over Dark, Parchment, High contrast and Tool
/// ([`SkinPreset::ALL`](crate::SkinPreset::ALL)) that sets the colours of
/// [`GooeySkin`](crate::GooeySkin), so the whole interface changes look in the frame the player
/// steps it, and the high-contrast one holds every colour to 7:1 ([`GooeySkin::high_contrast`](crate::GooeySkin::high_contrast);
/// Xbox Accessibility Guideline 102 asks a game for such a mode). `label` is the row's name, in the
/// host's language; the options are English words ([`SkinPreset::name`](crate::SkinPreset::name)),
/// and a host that localises writes a `choice` over its own strings with
/// [`SkinPreset::ALL`](crate::SkinPreset::ALL) and
/// [`GooeySkin::with_colours_of`](crate::GooeySkin::with_colours_of).
///
/// It behaves as [`text_size`] does: the skin is the one place the setting lives, the row follows it
/// with no event (the preset whose colours are nearest, so a skin the game tuned shows the nearest
/// and is left alone until the player steps), and a step writes the colours and no other field: the
/// player's text size, and a game's own gap and padding, stay as they were. A game that starts in
/// high contrast because the platform asks inserts `GooeySkin::high_contrast()` and the row shows it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((panel(), children![heading("Display"), skin_preset("Theme")]));
/// # }
/// ```
pub fn skin_preset(label: impl Into<String>) -> impl Bundle {
    option_row(OptionRow::SkinPreset, label)
}

/// One of a choice's two arrows: a target as wide as a button, with the chevron drawn in it. The
/// chevron is a square with two borders turned an eighth of a turn, so it is a line of the skin's
/// weight and not a glyph the host's font may lack; it is moved by the share of its own width that
/// puts its ink, not its square, in the middle of the target.
fn choice_arrow(step: i32) -> impl Bundle {
    let (mark, shift) = if step < 0 {
        (Role::ChoiceBack, CHEVRON_INK_SHIFT)
    } else {
        (Role::ChoiceForward, -CHEVRON_INK_SHIFT)
    };
    (
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        Role::ChoiceArrow,
        StepBy(step),
        Hovered::default(),
        observe(arrow_on_press),
        observe(arrow_on_click),
        children![(
            Node::default(),
            UiTransform {
                translation: Val2::percent(shift, 0.0),
                rotation: Rot2::degrees(45.0),
                ..default()
            },
            mark,
            Pickable::IGNORE,
        )],
    )
}

/// How far from the middle of a [`radial`] the items stand, as a share of its width: 36%, which
/// keeps an item of eight characters at the side of the ring inside the disc's edge.
const RADIAL_RING_FRACTION: f32 = 0.36;

/// A **radial menu**: a weapon wheel, an emote wheel, a build menu. The items stand on a ring
/// round a disc, item 0 at the top and the rest clockwise, and the player points at one and
/// chooses it. With a pad, push a stick (either one) toward the item, and press South; with a
/// pointer, move over the item's side of the disc and click, the whole wedge being the target; with
/// a keyboard, step with the arrows (Right and Down forward, Left and Up back, wrapping round),
/// Home and End, and press Enter or Space. The highlighted item is lit and its name is read in
/// the middle of the disc, where the thumb is. A stick that is only brushed, or a pointer in the
/// middle, leaves the highlight where it is.
///
/// It is Bevy's headless [`bevy::ui_widgets::Button`] and a tab stop, so a game that opens it for
/// the player gives it focus (`AutoFocus`). It keeps its state in [`RadialValue`] (the highlighted
/// item) and sends `ValueChange<usize>`: with `is_final: false` as the highlight moves, and with
/// `is_final: true` for the item chosen, even when the highlight did not move. Apply the choice on
/// the final one. To choose when a hold button is let go ("release to select"), trigger `Activate`
/// on the radial then. The disc is [`Role::Radial`] and 26 text sizes across, so it grows with
/// [`GooeySkin::text_scale`](crate::GooeySkin::text_scale); three to eight short labels read best.
/// An item is a name, or a [`RadialItem`] with a picture to draw in its place (the name is then
/// read in the middle). Name the wheel for a screen reader with `AccessibleLabel`.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     radial(["Pistol", "Rifle", "Shotgun", "Grenade"], 0),
///     observe(|change: On<ValueChange<usize>>| {
///         if change.is_final {
///             info!("equip item {}", change.value);
///         }
///     }),
/// ));
/// # }
/// ```
pub fn radial<I: Into<RadialItem>>(
    items: impl IntoIterator<Item = I>,
    selected: usize,
) -> impl Bundle {
    let items: Vec<RadialItem> = items.into_iter().map(Into::into).collect();
    let count = items.len();
    let selected = selected.min(count.saturating_sub(1));
    let name = items
        .get(selected)
        .map(|item| item.label.clone())
        .unwrap_or_default();
    let middle = |ring: f32, (x, y): (f32, f32)| Node {
        position_type: PositionType::Absolute,
        left: Val::Percent(50.0 + x * ring * 100.0),
        top: Val::Percent(50.0 - y * ring * 100.0),
        ..default()
    };
    let pull_back = UiTransform {
        translation: Val2::percent(-50.0, -50.0),
        ..default()
    };
    (
        Node::default(),
        bevy::ui_widgets::Button,
        RadialValue(selected),
        RadialCount(count),
        Role::Radial,
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        observe(radial_self_update),
        observe(radial_on_activate),
        observe(radial_on_key_input),
        observe(radial_on_pointer_move),
        observe(radial_on_press),
        Children::spawn((
            Spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    width: Val::Percent(RADIAL_RING_FRACTION * 200.0),
                    height: Val::Percent(RADIAL_RING_FRACTION * 200.0),
                    ..default()
                },
                pull_back,
                Role::RadialTrack,
                Pickable::IGNORE,
            )),
            SpawnIter(items.into_iter().enumerate().map(move |(index, item)| {
                let RadialItem { label, icon } = item;
                (
                    middle(RADIAL_RING_FRACTION, radial_place(index, count)),
                    pull_back,
                    Role::RadialSlot,
                    RadialSlot(index, label.clone()),
                    Pickable::IGNORE,
                    Children::spawn(SpawnWith(move |slot: &mut RelatedSpawner<ChildOf>| {
                        match icon {
                            Some(source) => slot.spawn((
                                Node::default(),
                                Role::RadialIcon,
                                Icon(source),
                                Pickable::IGNORE,
                            )),
                            None => slot.spawn((
                                Text::new(label),
                                ONE_LINE,
                                Role::RadialText,
                                Pickable::IGNORE,
                            )),
                        };
                    })),
                )
            })),
            Spawn((
                middle(0.0, (0.0, 0.0)),
                pull_back,
                Text::new(name),
                ONE_LINE,
                Role::RadialCentre,
                Pickable::IGNORE,
            )),
        )),
    )
}

/// An inventory: a grid of square slots, `columns` to a row, each holding an item or empty
/// (`None`), in reading order. A backpack, a hotbar, a chest, a shop shelf. The contents are data,
/// [`SlotItems`] on the root: change it and the grid follows in the same frame, however many
/// slots it has. A [`Slot`] is an item's name, its picture, its count and a tooltip line; a slot
/// with a picture draws it (tinted by the skin, so draw it white on transparent) and one without
/// draws its name, and a count above one sits in the corner.
///
/// Every slot is a real widget: Bevy's headless [`bevy::ui_widgets::Button`] with a focus ring and
/// a tooltip. The grid is one tab stop ([`SlotCursor`](crate::SlotCursor): Tab enters on the slot the player last had
/// and leaves on the next press), the arrows and the D-pad walk it, Home and End go to the ends of
/// the row and with Ctrl of the grid. Choosing a slot (Enter, Space, South or a click) sends one
/// `ValueChange<usize>` with `is_final: true` on the grid, empty or not; moving focus onto a slot
/// sends one with `is_final: false`.
///
/// Moving an item is the crate's own drag and drop: the pointer drags a filled slot onto any
/// slot (of this grid or another), and the keyboard and gamepad lift, steer and drop it
/// ([`DragSettings`](crate::DragSettings)). The drop is one [`SlotMoved`](crate::SlotMoved) on the target grid; [`SlotMoved::apply`](crate::SlotMoved::apply)
/// swaps the two slots, which is all a game with no rules needs. A slot is
/// [`GooeySkin::text_size`](crate::GooeySkin::text_size) times four square, so it follows the
/// player's text size. Wrap it in a [`list`] to scroll a long one.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands
///     .spawn(slot_grid(
///         5,
///         [
///             Some(Slot::new("Sword")),
///             Some(Slot::new("Arrows").count(24)),
///             None,
///             None,
///             None,
///         ],
///     ))
///     .observe(|moved: On<SlotMoved>, mut grids: Query<&mut SlotItems>| {
///         moved.apply(&mut grids);
///     })
///     .observe(|chosen: On<ValueChange<usize>>| {
///         if chosen.is_final {
///             info!("use slot {}", chosen.value);
///         }
///     });
/// # }
/// ```
pub fn slot_grid(columns: usize, items: impl IntoIterator<Item = Option<Slot>>) -> impl Bundle {
    let columns = columns.max(1);
    (
        Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::auto(columns as u16),
            ..default()
        },
        Role::SlotGrid,
        SlotGrid { columns },
        SlotItems(items.into_iter().collect()),
    )
}

/// A wrapping grid of [`Card`]s, each a picture above a name and a few quiet lines: a project
/// browser, a level select, a save list with thumbnails. The cards are a row that wraps to fill
/// the grid's width, every card the same width (14 body text sizes), and the grid is one tab stop
/// ([`crate::CardCursor`]) whose cards the arrows and the D-pad walk. Enter, Space, South or a
/// click choose a card: it is drawn lit ([`crate::CardSelection`]) and one final
/// `ValueChange<usize>` goes to the grid; focus moving between cards sends a non-final one. A
/// [`Card::disabled`] card is dimmed and never chosen. Each card is a [`crate::CardRef`] entity,
/// Bevy's `Button`, so it takes a [`crate::ContextMenu`] or a [`crate::Tooltip`] like any button.
/// The cards are spawned once; respawn the grid to change them.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, maps: Res<AssetServer>) {
/// commands.spawn((
///     card_grid([
///         Card::new("Harbour").image(maps.load("harbour.png")).line("10 x 10 m, 4 m tall"),
///         Card::new("Keep").line("13 pieces"),
///         Card::new("Ruins").line("Missing assets").disabled(),
///     ]),
///     observe(|change: On<ValueChange<usize>>| {
///         if change.is_final {
///             info!("open map {}", change.value);
///         }
///     }),
/// ));
/// # }
/// ```
pub fn card_grid(cards: impl IntoIterator<Item = Card>) -> impl Bundle {
    (
        Node::default(),
        Role::CardGrid,
        crate::CardGrid,
        Children::spawn(SpawnWith(crate::card::spawn_cards(
            cards.into_iter().collect(),
        ))),
    )
}

/// A row that rebinds one action: the action's name on the left and a keycap on the right showing
/// the key, mouse button or pad button it is bound to ("Space", "Right Click", "A"). It is Bevy's
/// headless [`bevy::ui_widgets::Button`], so Tab, the arrows and the D-pad reach it, and Enter,
/// Space, the pad's South button or a click start a capture: the keycap lights and says what it
/// waits for, and the player's next press is the new binding. A key row hears any key or mouse
/// button (the primary one too: the pointer cannot press anything while a row waits), and a pad row
/// any pad button; Escape (and, on a pad row, Start) exits without binding, and cannot be bound
/// (see [`crate::BindingSettings`]). While a row waits nothing else hears the player: arrows,
/// Tab, the D-pad and Back move and close nothing.
///
/// The row keeps its state in [`BindingValue`] and sends `ValueChange<Binding>` once for each
/// change; observe that to store the player's choice. Pressing the input the row already holds ends
/// the capture with no change. Which action an input belongs to, and what to do when two actions
/// share one, is the game's: [`Binding`] is `Eq + Hash`, and setting another row's [`BindingValue`]
/// swaps. The row never changes size, waiting or not.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     binding("Jump", Binding::Key(KeyCode::Space)),
///     observe(|change: On<ValueChange<Binding>>| info!("jump is now {:?}", change.value)),
/// ));
/// # }
/// ```
pub fn binding(label: impl Into<String>, value: Binding) -> impl Bundle {
    let part = |text: String, role: Role, visibility: Visibility| {
        (
            Node {
                grid_row: GridPlacement::start(1),
                grid_column: GridPlacement::start(1),
                justify_self: JustifySelf::Center,
                ..default()
            },
            Text::new(text),
            ONE_LINE,
            role,
            visibility,
            Pickable::IGNORE,
        )
    };
    (
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
        bevy::ui_widgets::Button,
        BindingValue(value),
        Role::Binding,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        observe(binding_self_update),
        observe(binding_on_activate),
        observe(binding_on_key_input),
        children![
            wrapping_label(label),
            (
                Node {
                    display: Display::Grid,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Role::BindingCap,
                Pickable::IGNORE,
                // Both texts stand in the one cell, the name and the prompt, so the keycap is as
                // wide as the wider; `update_bindings` fills them and shows one.
                children![
                    part(
                        value.label(PadStyle::default()),
                        Role::BindingText,
                        Visibility::Inherited
                    ),
                    part(String::new(), Role::BindingPrompt, Visibility::Hidden),
                ],
            ),
        ],
    )
}

/// A row for a line of text the player types (a name, a seed, a server address): the label on the
/// left, outside the field and always visible, and a framed field on the right that holds 16
/// characters, and the caret's room, at the body size and scrolls past that, so the row never
/// changes size while the player types. The field is Bevy's headless `TextInput`, so it has a
/// caret, selection, copy and paste and the platform's input methods; a click on the field, the
/// label or the row's padding focuses it, and a focused field takes the keys at once, with no step
/// to begin typing.
///
/// Left, Right, Home and End are the caret's; Up, Down and Tab leave the field, and so does the
/// D-pad (a pad cannot type: the platform's keyboard types through the same key events, and a
/// host opens it when [`TextEditing`](crate::TextEditing) is added to the row). Escape ends the
/// edit and, inside a [`dialog`], is Back as everywhere ([`Cancel`](crate::Cancel)). A field's
/// frame stays lit while it holds focus, even after a pointer press hid every other ring.
///
/// The row keeps its state in [`TextInputValue`] and sends `ValueChange<String>` for each edit
/// that changes the text (`is_final` is false), and once more with `is_final` when the edit ends:
/// Enter (a submit, even with nothing changed) or focus leaving the field after a change. Add
/// [`TextHint`](crate::TextHint) for muted words while the field is empty and
/// [`TextLimit`](crate::TextLimit) for a most characters, on the row:
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     text_input("Name", ""),
///     TextHint("Enter a name".into()),
///     TextLimit(16),
///     observe(|change: On<ValueChange<String>>| {
///         if change.is_final {
///             info!("the player is called {}", change.value);
///         }
///     }),
/// ));
/// # }
/// ```
pub fn text_input(label: impl Into<String>, value: impl Into<String>) -> impl Bundle {
    text_row(Some(label.into()), value.into(), Some(TEXT_INPUT_CHARS))
}

/// A frame for what the host draws itself (a 3D view, a map, a canvas): it fills the room its
/// parent leaves, is a tab stop, shows the focus ring while it has focus, and is a
/// [`FocusRegion::host`]: with focus in it the arrow keys are the host's (read them from
/// `ButtonInput<KeyCode>`), Tab and F6 still leave, so the keyboard is never trapped. Put what
/// draws into it (a `ViewportNode`, an image) as children or on the entity itself, and
/// `.insert(DropTarget::<T>::new())` makes it a place pieces are dropped.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn pan(keys: Res<ButtonInput<KeyCode>>, focus: Res<bevy::input_focus::InputFocus>, view: Single<Entity, With<Role>>) {
///     // Read the arrows when the viewport has focus; gooey moves no focus meanwhile.
///     if focus.get() == Some(*view) && keys.pressed(KeyCode::ArrowLeft) {
///         // pan left
///     }
/// }
/// # fn setup(mut commands: Commands) { commands.spawn(viewport()); }
/// ```
pub fn viewport() -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            align_self: AlignSelf::Stretch,
            ..default()
        },
        Role::Viewport,
        FocusRegion::host(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
    )
}

/// A row with a text field several lines high: the label over a framed field that wraps its
/// words at its width and scrolls past `rows` lines, for notes and descriptions. It is a
/// [`text_input`] in every other way: [`crate::TextInputValue`] is the text, a game's write shows in
/// the same frame with no event, [`crate::TextHint`], [`crate::TextLimit`] (a paste is cut at it),
/// [`crate::FieldProblem`] and [`crate::TextEditing`] go on the row, and every edit is one
/// [`ValueChange<String>`](bevy::ui_widgets::ValueChange) on it.
///
/// **Enter adds a line** and Up and Down move the caret between lines. **Ctrl+Enter or Cmd+Enter
/// ends the edit**: one final change with the text, the field kept. Tab, and Escape, leave the
/// field as in a [`text_input`] (it never traps the keyboard); a pad's D-pad moves focus off it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn setup(mut commands: Commands) {
/// commands.spawn((
///     text_area("Notes", "", 5),
///     TextLimit(500),
///     observe(|change: On<ValueChange<String>>| {
///         if change.is_final {
///             info!("saved {} characters", change.value.chars().count());
///         }
///     }),
/// ));
/// # }
/// ```
pub fn text_area(label: impl Into<String>, value: impl Into<String>, rows: usize) -> impl Bundle {
    text_row_as(
        Role::TextArea,
        Some(label.into()),
        value.into(),
        Some(TEXT_AREA_CHARS),
        RowKind::Area(rows.max(1)),
    )
}

/// A [`text_area`] that fills its host: the field is as tall as the room the row's parent gives it
/// (and its `rows` a least height), its own rail runs down its right while there is more text than
/// it draws, and the wheel, the rail and the arrow keys scroll it. It is the fixed area in every
/// other way — `TextInputValue`, `ValueChange<String>`, `TextLimit`, `TextHint`, `FieldProblem`,
/// `TextEditing` — and a very long line wraps rather than panning.
///
/// A row already carries its `Node`, so the room comes from the host: spawn a column with the height
/// to give and put the row in it — the row takes what the column leaves after its label.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn setup(mut commands: Commands) {
/// commands
///     .spawn(Node {
///         flex_direction: FlexDirection::Column,
///         width: Val::Percent(100.0),
///         height: Val::Percent(100.0),
///         ..default()
///     })
///     .with_children(|host| {
///         host.spawn((
///             text_area_fill("Guide", "", 10),
///             observe(|change: On<ValueChange<String>>| {
///                 if change.is_final {
///                     info!("{} characters", change.value.chars().count());
///                 }
///             }),
///         ));
///     });
/// # }
/// ```
pub fn text_area_fill(
    label: impl Into<String>,
    value: impl Into<String>,
    rows: usize,
) -> impl Bundle {
    (
        text_row_as(
            Role::TextArea,
            Some(label.into()),
            value.into(),
            Some(TEXT_AREA_CHARS),
            RowKind::Fill(rows.max(1)),
        ),
        crate::text_area::TextAreaFill,
    )
}

/// The row under [`text_input`]: a label if it has one and the field, which is `chars` character
/// advances wide, or fills the room its parent leaves when `chars` is `None` (a command palette's
/// field has no label and the card's whole width).
pub(crate) fn text_row(label: Option<String>, value: String, chars: Option<f32>) -> impl Bundle {
    let role = if chars.is_none() {
        Role::PaletteField
    } else {
        Role::TextInput
    };
    text_row_as(role, label, value, chars, RowKind::Text)
}

/// What a text row is for: text, a number (Up and Down step its field and the text cursor is over
/// it, so the row's own resize cursor, the scrub's, is on its label and padding only), a colour
/// (a swatch between the label and the field, and Escape puts the colour's text back) or a filling
/// area (a field as tall as its host, with a rail of its own).
#[derive(Clone, Copy)]
enum RowKind {
    Text,
    /// A field `usize` lines high: Enter adds a line.
    Area(usize),
    /// A field as tall as its host, scrolled by its rail and the wheel: `<rows>` is its least
    /// height.
    Fill(usize),
    Number,
    Colour(Srgba),
}

/// [`text_row`] with its row's `role` and what it is `kind` of row.
fn text_row_as(
    role: Role,
    label: Option<String>,
    value: String,
    chars: Option<f32>,
    kind: RowKind,
) -> impl Bundle {
    let fills = chars.is_none();
    let area = matches!(kind, RowKind::Area(_) | RowKind::Fill(_));
    let filling = matches!(kind, RowKind::Fill(_));
    (
        Node {
            flex_direction: if area {
                FlexDirection::Column
            } else {
                FlexDirection::Row
            },
            justify_content: if area {
                JustifyContent::FlexStart
            } else {
                JustifyContent::SpaceBetween
            },
            align_items: if area {
                AlignItems::Stretch
            } else {
                AlignItems::Center
            },
            width: if fills || filling {
                Val::Percent(100.0)
            } else {
                Val::Auto
            },
            // A filling area takes the room its host leaves and gives nothing back: it is the one
            // widget whose height is its parent's, so a scroll pane of text grows with the window.
            flex_grow: if filling { 1.0 } else { 0.0 },
            align_self: if filling {
                AlignSelf::Stretch
            } else {
                AlignSelf::Auto
            },
            min_height: Val::ZERO,
            ..default()
        },
        role,
        TextInputValue(value.clone()),
        observe(text_self_update),
        observe(text_row_on_acquire),
        Children::spawn(SpawnWith(move |row: &mut RelatedSpawner<ChildOf>| {
            let root = row.target_entity();
            if let Some(label) = label {
                row.spawn((
                    Text::new(label),
                    ONE_LINE,
                    Role::ControlText,
                    Pickable::IGNORE,
                ));
            }
            if let RowKind::Colour(colour) = kind {
                row.spawn(color_swatch(root, colour));
            }
            row.spawn((
                // A filling area's cell is a row stretched to the row's height, holding its field
                // and its rail side by side, and it never shrinks under its content; a fixed row's
                // cell is the one line's box, exactly as every text row's was (a `min_width` of 0
                // here let a hex field shrink and scroll by 114 px, which `tests/color.rs` holds
                // against).
                if filling {
                    Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Stretch,
                        min_height: Val::ZERO,
                        ..default()
                    }
                } else {
                    Node {
                        flex_grow: if fills { 1.0 } else { 0.0 },
                        ..default()
                    }
                },
                Role::TextCell,
            ))
            .with_children(|cell| {
                // A filling area's field is its cell's height (the cell stretches to the host): the
                // text scrolls inside it rather than making it as tall as the text. Nothing else
                // about a fixed row's field changes.
                let field_node = if filling {
                    Node {
                        flex_shrink: 1.0,
                        flex_grow: 1.0,
                        min_width: Val::ZERO,
                        min_height: Val::ZERO,
                        overflow: Overflow::clip(),
                        ..default()
                    }
                } else {
                    Node {
                        flex_shrink: 0.0,
                        flex_grow: if fills { 1.0 } else { 0.0 },
                        overflow: Overflow::clip(),
                        ..default()
                    }
                };
                let mut text = EditableText {
                    visible_width: chars,
                    visible_lines: match kind {
                        RowKind::Area(rows) | RowKind::Fill(rows) => Some(rows as f32),
                        _ => Some(1.0),
                    },
                    allow_newlines: area,
                    cursor_width: TEXT_CARET_WIDTH,
                    ..EditableText::new(&value)
                };
                if filling {
                    // A document opens at its top, caret and view: `EditableText::new` puts the
                    // caret at the text's end, which a long text would scroll to at once.
                    text.queue_edit(TextEdit::TextStart(false));
                }
                let mut field = cell.spawn((
                    field_node,
                    TextInput,
                    text,
                    if filling {
                        // A very long line wraps at the field's width (`WordOrCharacter`: a word
                        // with no break opportunity in it still breaks), so there is no horizontal
                        // scrolling to find.
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter)
                    } else if area {
                        TextLayout::default()
                    } else {
                        TextLayout::no_wrap()
                    },
                    TextCursorStyle::default(),
                    Role::TextField,
                    TextInputOf(root),
                    Hovered::default(),
                    TabIndex(0),
                    AutoDirectionalNavigation::default(),
                    observe(text_on_key_input),
                ));
                match kind {
                    RowKind::Text => {}
                    RowKind::Area(_) => {
                        field.insert(AreaField);
                    }
                    RowKind::Fill(_) => {
                        field.insert((AreaField, crate::text_area::AreaCaret::default()));
                    }
                    RowKind::Number => {
                        field.insert((
                            NumberInput,
                            SelectAllOnFocus,
                            EntityCursor::System(SystemCursorIcon::Text),
                            observe(number_on_key_input),
                            observe(number_on_step),
                        ));
                    }
                    RowKind::Colour(_) => {
                        field.insert((SelectAllOnFocus, observe(color_on_key_input)));
                    }
                }
                if let RowKind::Fill(_) = kind {
                    // The rail stands beside the field, not over it: a lane the text does not run
                    // under, and the one node a press on the lane lands on (a later sibling draws
                    // over an earlier one, so an absolute rail would be under the field).
                    let field_id = field.id();
                    cell.spawn((
                        Node {
                            flex_shrink: 0.0,
                            align_self: AlignSelf::Stretch,
                            margin: UiRect::left(Val::Px(AREA_RAIL_GAP_PX)),
                            ..default()
                        },
                        Role::ListRail,
                        crate::text_area::AreaRail(field_id),
                        Hovered::default(),
                        Visibility::Hidden,
                        children![(
                            Node {
                                width: Val::Percent(100.0),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Role::ListThumb,
                            crate::text_area::AreaThumb(field_id),
                            // The rail is the lane: a press anywhere on it scrolls there, so the
                            // thumb never takes the pointer.
                            Pickable::IGNORE,
                            Hovered::default(),
                        )],
                    ));
                }
                cell.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        display: Display::None,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextLayout::no_wrap(),
                    Role::TextHint,
                    TextInputOf(root),
                    Pickable::IGNORE,
                ));
            });
        })),
    )
}

/// What a number row adds to its text row: the number and how it moves, the hint that shows a
/// mixed value, the scrub (a drag that begins on the row's label or padding) and the observers that
/// turn typing, steps and drags into one [`ValueChange<f32>`].
fn number_parts(value: f32, step: f32, range: RangeInclusive<f32>) -> impl Bundle {
    (
        NumberValue(value),
        NumberStep(step),
        NumberRange {
            min: *range.start(),
            max: *range.end(),
        },
        TextHint(String::new()),
        EntityCursor::System(SystemCursorIcon::ColResize),
        (
            observe(number_self_update),
            observe(number_on_text),
            observe(number_on_drag_start),
            observe(number_on_drag),
            observe(number_on_drag_end),
            observe(number_on_cancel),
        ),
    )
}

/// A row with a number: a label and a framed field the player types a number in, scrubs with a
/// drag and steps with a key or the D-pad. `step` is how far one pixel of drag, one Up or Down
/// press or one D-pad press moves it (and the digits the field shows: 0.1 shows one), and `range`
/// is the least and greatest the player can make it.
///
/// Press on the label (or the row's padding) and drag left or right to scrub: 50 px at a step of
/// 0.1 is 5.0, Shift takes a tenth of that rate and Ctrl ten times it. Click the field and type;
/// Enter, or leaving the field, applies it (a number outside the range is held to its end;
/// anything that is not a number, an expression such as `1.5*2` included, is refused with its
/// reason as a [`NumberRefused`](crate::NumberRefused)); Escape puts the old number back. Up and
/// Down on the focused field step it (Shift ten steps) and Left, Right, Home and End are the
/// caret's; the D-pad's Left and Right step it and its Up and Down move focus; Tab leaves. A
/// drag is never the only way (WCAG 2.5.7): the typed field is the other.
///
/// The row keeps its number in [`NumberValue`] and sends `ValueChange<f32>` for each change the
/// player makes: `is_final` is false while a drag is scrubbing and true when it ends, on Enter,
/// when focus leaves after an edit and for a step. Put [`MixedValue`](crate::MixedValue) on the row
/// for a multi-selection whose members differ: the field is empty and shows a muted
/// [`MIXED_HINT`](crate::MIXED_HINT).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     number_field("Health", 100.0, 0.5, 0.0..=250.0),
///     observe(|change: On<ValueChange<f32>>| {
///         if change.is_final {
///             info!("health is {}", change.value);
///         }
///     }),
/// ));
/// # }
/// ```
pub fn number_field(
    label: impl Into<String>,
    value: f32,
    step: f32,
    range: RangeInclusive<f32>,
) -> impl Bundle {
    (
        text_row_as(
            Role::TextInput,
            Some(label.into()),
            format_number(value, step),
            Some(NUMBER_FIELD_CHARS),
            RowKind::Number,
        ),
        number_parts(value, step, range),
    )
}

/// A row with a label and one [`number_field`] for each of `N` numbers (X, Y, Z and W, then 5, 6 …),
/// as a position or a scale: each component's letter is its scrub handle, and each has the
/// typing, steps, range and refusals of a number field. `step` and `range` are every component's.
///
/// The row keeps its numbers in [`VecValue`](crate::VecValue), and a change the player makes is a
/// [`VecChange`](crate::VecChange) on the row: the component, its new value and the whole vector.
/// For a multi-selection whose members differ in some components, put
/// [`VecMixed`](crate::VecMixed) on the row: those components are empty with a muted
/// [`MIXED_HINT`](crate::MIXED_HINT), and a
/// change to one is for that component only.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     vec_field("Position", [0.0, 1.5, -2.0], 0.1, -1000.0..=1000.0),
///     observe(|change: On<VecChange>| {
///         if change.is_final {
///             info!("position is {:?}", change.values);
///         }
///     }),
/// ));
/// # }
/// ```
pub fn vec_field<const N: usize>(
    label: impl Into<String>,
    values: [f32; N],
    step: f32,
    range: RangeInclusive<f32>,
) -> impl Bundle {
    let label = label.into();
    let (min, max) = (*range.start(), *range.end());
    (
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
        Role::TextInput,
        VecValue(values.to_vec()),
        NumberStep(step),
        NumberRange { min, max },
        Children::spawn(SpawnWith(move |row: &mut RelatedSpawner<ChildOf>| {
            let root = row.target_entity();
            row.spawn((
                Text::new(label),
                ONE_LINE,
                Role::ControlText,
                Pickable::IGNORE,
            ));
            row.spawn((Node::default(), Role::NumberParts))
                .with_children(|parts| {
                    for (index, value) in values.into_iter().enumerate() {
                        parts.spawn((
                            text_row_as(
                                Role::NumberPart,
                                Some(axis_name(index)),
                                format_number(value, step),
                                Some(VEC_FIELD_CHARS),
                                RowKind::Number,
                            ),
                            number_parts(value, step, min..=max),
                            VecPart { root, index },
                            observe(vec_part_on_change),
                        ));
                    }
                });
        })),
    )
}

/// A row with a colour: a label, a swatch and a framed field showing the colour as hex, which the
/// player types (`#ff8800`, `#f80`, `ff8800`; `#ff880080` with an alpha) or picks. Enter or leaving
/// the field applies what was typed; anything that is not a hex colour is refused with its reason
/// as a [`ColorRefused`](crate::ColorRefused) and the field shows the colour again; Escape puts
/// the old colour back.
///
/// The swatch is a button (Tab, Enter, Space, South or a click): it opens a picker below it and a
/// second press closes it, Escape too. The picker is a saturation and value area (a pointer press
/// and drag, or the arrows, Shift ten times as far) over three strips for hue, brightness and
/// alpha, which are sliders (Left, Right, Home, End and the D-pad); a pad sets the saturation with
/// the D-pad's Left and Right on the area and the rest with the strips. The colour applies as it
/// is picked and there is nothing to confirm. The swatch is split: the colour opaque on the left
/// and at its alpha on the right.
///
/// The row keeps its colour in [`ColorValue`](crate::ColorValue) and sends `ValueChange<Srgba>`
/// for each change the player makes: `is_final` is false while a drag is rolling the colour and
/// true when it ends, on Enter, when focus leaves after an edit and for a key or D-pad step. Put
/// [`MixedValue`](crate::MixedValue) on the row for a multi-selection whose members differ: the
/// field is empty and shows a muted [`MIXED_HINT`](crate::MIXED_HINT).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     color_field("Tint", Srgba::hex("#ff8800").unwrap()),
///     observe(|change: On<ValueChange<Srgba>>| {
///         if change.is_final {
///             info!("tint is {}", hex_text(change.value.to_f32_array()));
///         }
///     }),
/// ));
/// # }
/// ```
pub fn color_field(label: impl Into<String>, value: Srgba) -> impl Bundle {
    (
        text_row_as(
            Role::TextInput,
            Some(label.into()),
            hex_text(value.to_f32_array()),
            Some(COLOR_FIELD_CHARS),
            RowKind::Colour(value),
        ),
        ColorValue(value),
        TextHint(String::new()),
        observe(color_self_update),
        observe(color_on_text),
    )
}

/// The swatch of a [`color_field`]: a button, a target square, split in two halves (the colour
/// opaque and at its alpha). Pressing it opens the picker. The auto margin on its left pushes it
/// and the field to the row's right edge.
fn color_swatch(row: Entity, value: Srgba) -> impl Bundle {
    (
        Node {
            margin: UiRect::left(Val::Auto),
            ..default()
        },
        bevy::ui_widgets::Button,
        Role::ColorSwatch,
        ColorSwatch(row),
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        observe(swatch_on_activate),
        children![
            (
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Role::ColorSwatchHalf,
                SwatchHalf { row, alpha: false },
                BackgroundColor(Color::Srgba(value.with_alpha(1.0))),
                Pickable::IGNORE,
            ),
            (
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Role::ColorSwatchHalf,
                SwatchHalf { row, alpha: true },
                BackgroundColor(Color::Srgba(value)),
                Pickable::IGNORE,
            ),
        ],
    )
}

/// The letter of a vec field's `index`th component: X, Y, Z, W, and then its number from 5.
fn axis_name(index: usize) -> String {
    match index {
        0 => "X".into(),
        1 => "Y".into(),
        2 => "Z".into(),
        3 => "W".into(),
        n => (n + 1).to_string(),
    }
}

/// One hint of what an input does: a keycap and the words for it ("Esc  Back"). The keycap shows
/// the input of the device the player used last, the keyboard's or the pad's from `keys`, in the
/// words of a [`binding`] row ("Esc", "Enter", "A" or "Cross"), and follows the player the frame
/// they pick up the other ([`InputDevice`](crate::InputDevice)). A key is a rounded keycap and a
/// pad button is fully round, in the same colours.
///
/// The words are yours (the crate has none to translate). [`Prompt::SELECT`] and [`Prompt::BACK`]
/// are the two the crate itself acts on: Enter and the pad's South button select, Escape and the
/// pad's East button send a [`Cancel`](crate::Cancel). For an action of the game's own, build the
/// pair with [`Prompt::new`]. A prompt is a hint, not a control: the pointer goes through it and it
/// is never a tab stop. It stands anywhere; put it in a [`prompts`] legend for the corner of the
/// screen.
///
/// A game that lets the player rebind the action keeps the prompt true by setting the [`Prompt`]
/// component (the keycap follows in the same frame):
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     prompts(),
///     children![
///         prompt("Select", Prompt::SELECT),
///         prompt("Back", Prompt::BACK),
///         prompt(
///             "Inventory",
///             Prompt::new(Binding::Key(KeyCode::KeyI), Binding::Pad(GamepadButton::North)),
///         ),
///     ],
/// ));
/// # }
/// ```
pub fn prompt(label: impl Into<String>, keys: Prompt) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            ..default()
        },
        Role::Prompt,
        keys,
        Pickable::IGNORE,
        children![
            (
                // Named for the keyboard's input; `update_prompts` writes the device's.
                Text::new(keys.keyboard.label(PadStyle::default())),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                Role::PromptKey,
                Pickable::IGNORE,
            ),
            (Text::new(label), Role::Label, Pickable::IGNORE),
        ],
    )
}

/// The legend of a screen: a slim bar of [`prompt`]s in its parent's bottom-left corner (see
/// [`prompts_at`] for another), 5% in from
/// the edges (the safe area of a television), where Xbox's guideline 112 puts "select" and "back"
/// on every screen, in the same place and order. It is a bar on the skin's panel colour, so its
/// words read over any game world. It is laid out apart from its parent's other children (absolute,
/// so it moves nothing) and is no wider than 90% of its parent: a longer legend squeezes its words
/// and never leaves the window. The pointer goes through it.
///
/// Put it in the full-window node of a screen ([`screen`]) or of a [`dialog`], which stands over the
/// screen's own and shows the hints that apply in the dialog:
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     screen(),
///     children![
///         (panel(), children![heading("Settings"), button("Back")]),
///         (prompts(), children![prompt("Select", Prompt::SELECT), prompt("Back", Prompt::BACK)]),
///     ],
/// ));
/// # }
/// ```
pub fn prompts() -> impl Bundle {
    prompts_at(PromptsAnchor::default())
}

/// A [`prompts`] legend in the corner or at the edge the game chooses: `prompts_at(
/// PromptsAnchor::BottomRight)` for a game whose own HUD holds the bottom-left. Everything else
/// is [`prompts`]'s: 5% in from the edges it touches, no wider than 90% of its parent, nothing
/// else moves. Choose one anchor and use it on every screen and dialog, which is what Xbox's
/// guideline 112 asks. The anchor is a component: insert another and the legend moves.
pub fn prompts_at(anchor: PromptsAnchor) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            ..default()
        },
        Role::Prompts,
        anchor,
        Pickable::IGNORE,
    )
}

/// A column of rows that scrolls when they do not all fit: save slots, an inventory, a key-binding
/// screen, a long settings page. `rows` is how many rows tall it stands, in the units of a button
/// and the gap beside it, so `4.5` shows four rows and half of the fifth, which cuts it, the sign
/// that more is below. The rows are the list's children, in `children![…]`: any widget, each its
/// own tab stop.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     panel(),
///     children![
///         heading("Load game"),
///         list(
///             4.5,
///             children![
///                 button("Meadow, 12:40"),
///                 button("Harbour, 09:15"),
///                 button("Mill, 03:02"),
///                 button("Cellar, 01:48"),
///                 button("Orchard, 00:20"),
///                 button("Autosave"),
///             ]
///         ),
///     ],
/// ));
/// # }
/// ```
///
/// The wheel and a trackpad scroll it, a drag on the thumb moves it and a click on the rail pages a
/// viewport, and it follows focus: moving to a row with a key, the D-pad or Tab scrolls it by the
/// least that leaves the next row whole and the one after it cut in sight, and PageUp, PageDown and
/// the pad's triggers move a viewport at a time. A click on a half-visible row focuses it and does
/// not scroll under the pointer. The rail shows only while the rows do not fit. A list shows every
/// row it holds, so it is for tens of rows, not thousands.
///
/// The entity this returns holds the rows' viewport, a [`ListView`] (its first child), and the
/// rail; to scroll from code, set the `ScrollPosition` on the [`ListView`].
pub fn list<L>(rows: f32, items: SpawnRelatedBundle<ChildOf, L>) -> impl Bundle
where
    L: SpawnableList<ChildOf> + Send + Sync + 'static,
{
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::List,
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            let view = parent
                .spawn((
                    Node {
                        display: Display::Block,
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    Role::ListView,
                    ListView { rows },
                    ScrollArea,
                    // The rows sit in one column that never shrinks: a viewport holding them
                    // directly squeezes them to fit its height, down to their `min_height`, and a
                    // button squeezed by a few pixels clips its own text.
                    children![(
                        Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.0,
                            ..default()
                        },
                        Role::ListRows,
                        items,
                    )],
                ))
                .id();
            parent.spawn(list_rail(view));
        })),
    )
}

/// A list's rail: the scrollbar down the viewport's right edge. The thumb has no `Node`: Bevy
/// lays it out itself, after layout, from the viewport. The rail is hidden until `update_lists`
/// finds the rows do not fit.
pub(crate) fn list_rail(view: Entity) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            right: Val::Px(0.0),
            display: Display::None,
            ..default()
        },
        Role::ListRail,
        Scrollbar::new(view, ControlOrientation::Vertical, LIST_MIN_THUMB),
        children![(
            Role::ListThumb,
            ScrollbarThumb {
                border_radius: BorderRadius::MAX,
                border: UiRect::ZERO,
            },
            Hovered::default(),
        )],
    )
}

/// A table of ten thousand rows in a dozen entities: a header of `columns` over rows from a
/// [`RowSource`], of which only the rows in view are spawned (the rows in view and one more,
/// whatever the length). A click on a sortable header sorts by the column (the first click
/// ascending, each next one turning it round) and sends [`SortBy`](crate::SortBy); a column's
/// right edge is dragged to size it (Ctrl and Left or Right on its focused header does it 16 px a
/// press, for no drag); a click selects a row (see [`SelectMode`](crate::SelectMode): one by
/// default, `Multi` for Ctrl and Shift) and sends `ValueChange<TableSelection>`
/// ([`TableSelection`](crate::TableSelection)), and a double click or Enter opens it
/// ([`RowActivated`](crate::RowActivated)). Rows change through the [`Table`](crate::Table)
/// component and the player keeps their place.
///
/// The table is one tab stop with a cursor row, like a list box: Up, Down, the Page keys, Home and
/// End (the first and last rows of the data) move it, Space selects it, and the D-pad, the
/// triggers and South do the same on a pad. Every row is a button's height and every cell one
/// line, cut at its column's edge: a row never changes size, which is what lets the table skip the
/// rows it does not draw. It stands [`TableRows`](crate::TableRows) rows tall (8.5 unless the game
/// says) with a rail that shows while there is more.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// let rows: Vec<Vec<String>> = (0..10_000)
///     .map(|i| vec![format!("Item {i}"), format!("{}", i * 7 % 100)])
///     .collect();
/// commands.spawn((
///     panel(),
///     children![(
///         table([Column::new("Name").width(14.0), Column::new("Qty").width(6.0)], rows),
///         SelectMode::Multi,
///         observe(|sort: On<SortBy>| info!("sorted by {}", sort.column)),
///         observe(|opened: On<RowActivated>| info!("opened row {}", opened.key)),
///     )],
/// ));
/// # }
/// ```
pub fn table(columns: impl IntoIterator<Item = Column>, rows: impl RowSource) -> impl Bundle {
    table_bundle(columns, rows, false)
}

/// A tree: nodes that open and close, as many as you have, in a handful of entities. A node is a
/// [`TreeNode`] (a key that is its identity, a label, its children, open or closed to start); the
/// player opens one with a click on its chevron or Right and closes it with a click or Left, and
/// the tree shows only the nodes whose parents are open, a row each, a button's height, the
/// children indented one level more. A closed node keeps its descendants' state (reopened, they
/// are as they were). It is the [`table`]'s rows in one column: the same scrolling, virtualized
/// pool (an outliner of 10,000 nodes is a screenful of entities), the same cursor and selection
/// by key ([`SelectMode`](crate::SelectMode) `Single` by default, `Multi` for Ctrl and Shift), the
/// same `ValueChange<TableSelection>` ([`TableSelection`](crate::TableSelection)) and
/// [`RowActivated`](crate::RowActivated) for Enter and a double click.
///
/// One tab stop with a cursor row. Up, Down, Home, End and the Page keys move it over the nodes
/// shown; Right opens a closed node and goes to the first child of an open one; Left closes an
/// open node and goes to the parent of any other; `*` opens every sibling of the cursor node; and
/// typing goes to the next node whose label starts with what was typed (within a second). The
/// D-pad and South do the same on a pad. [`TreeToggled`](crate::TreeToggled) tells the game what
/// the player opened or closed. Nodes change through the [`Table`](crate::Table) component
/// (`table.source_mut::<Tree>()`: [`Tree::insert`], [`Tree::remove`], [`Tree::set_open`]); a
/// removed node that was selected passes the selection to its next sibling, else the one before,
/// else its parent.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     panel(),
///     children![(
///         tree([TreeNode::new(1, "World").expanded().children([
///             TreeNode::new(2, "Camera"),
///             TreeNode::new(3, "Lights").child(TreeNode::new(4, "Sun")),
///         ])]),
///         SelectMode::Multi,
///         observe(|picked: On<ValueChange<TableSelection>>| info!("{:?}", picked.value.0)),
///     )],
/// ));
/// # }
/// ```
pub fn tree(nodes: impl IntoIterator<Item = TreeNode>) -> impl Bundle {
    (
        table_bundle([Column::new("").unsortable()], Tree::new(nodes), true),
        TreeView,
    )
}

/// Tabs: a strip of one- or two-word labels above pages of widgets, one page shown at a time. The
/// i-th label opens the i-th page, each a [`tab_page`] in `children![…]`; `selected` is the page
/// shown first. A settings screen is the use: Audio, Display and Controls, each its own page of
/// [`toggle`]s, [`slider`]s and [`choice`]s.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     panel(),
///     children![
///         heading("Settings"),
///         (
///             tabs(
///                 ["Audio", "Display"],
///                 0,
///                 children![
///                     tab_page(children![slider("Volume", 0.7, 0.0..=1.0), toggle("Subtitles", true)]),
///                     tab_page(children![choice("Quality", ["Low", "High"], 1)]),
///                 ]
///             ),
///             observe(|change: On<ValueChange<usize>>| info!("page {}", change.value)),
///         ),
///     ],
/// ));
/// # }
/// ```
///
/// The selected tab is lit and its page shown; a click on a tab, Left and Right on a focused tab
/// (they wrap), and the D-pad's Left and Right select it. **`LB` and `RB`** (and Ctrl+PageUp and
/// Ctrl+PageDown) turn the page from anywhere in the widget, so a pad player never has to go back
/// to the strip ([`step_tabs`](crate::step_tabs)); focus stays in the page and lands on the new
/// page's first widget ([`land_tab_focus`](crate::land_tab_focus)). Down from a tab goes into the
/// page and Up from the page goes to the selected tab.
///
/// Every page takes the room of the largest, so turning a page moves nothing: keep a handful of
/// short labels, and let the pages be about the same height. The entity this returns holds
/// [`TabsValue`](crate::TabsValue), the selected index (read it, or set it to turn the page from
/// code), and reports each change a player makes as a [`ValueChange<usize>`](crate::ValueChange)
/// on itself.
pub fn tabs<S: Into<String>>(
    labels: impl IntoIterator<Item = S>,
    selected: usize,
    pages: impl Bundle,
) -> impl Bundle {
    tabs_in(
        labels.into_iter().map(|label| (label.into(), ())),
        selected,
        pages,
        false,
    )
}

/// [`tabs`] with a bundle on each tab (the dock's marker and drag observers), and, with `fill`, a
/// widget that takes all the room its parent gives it, its page area included, so a panel in a
/// dock group can fill the group.
pub(crate) fn tabs_in<B: Bundle>(
    labels: impl IntoIterator<Item = (String, B)>,
    selected: usize,
    pages: impl Bundle,
    fill: bool,
) -> impl Bundle {
    let labels: Vec<(String, B)> = labels.into_iter().collect();
    let count = labels.len();
    let selected = selected.min(count.saturating_sub(1));
    let (grow, least) = if fill {
        (1.0, Val::Px(0.0))
    } else {
        (0.0, Val::Auto)
    };
    (
        Node {
            flex_direction: FlexDirection::Column,
            flex_grow: grow,
            min_width: least,
            min_height: least,
            ..default()
        },
        Role::Tabs,
        TabsValue(selected),
        TabsCount(count),
        observe(tabs_self_update),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            // Bevy's list: Left and Right on a tab move focus and select it at once, and a click
            // selects. Its answer is an entity; `tabs_on_select` turns it into the place. One
            // column per tab, `1fr` each, so the columns are equal, as wide as the widest label
            // and its padding, and the strip is never narrower than that: the panel it stands in
            // grows to fit its labels and never squeezes one.
            parent.spawn((
                if fill {
                    // A dock's strip keeps its tabs as wide as their labels, left-aligned, and
                    // wraps to a second line when a narrow group cannot hold them: every tab is
                    // always there to see and to reach.
                    Node {
                        flex_wrap: FlexWrap::Wrap,
                        align_content: AlignContent::FlexStart,
                        ..default()
                    }
                } else {
                    // Equal cells, as many to a row as the strip's width holds: `strip_columns`
                    // counts them from the laid-out width of the labels, and one `1fr` track per
                    // column makes every cell of a row the same width -- which `FlexWrap` could not
                    // (a wrapped row's cells came out `[312, 284]` px) and `auto-fit` could not (it
                    // collapsed to one column).
                    Node {
                        display: Display::Grid,
                        grid_template_columns: vec![RepeatedGridTrack::flex(
                            u16::try_from(count).unwrap_or(u16::MAX),
                            1.0,
                        )],
                        ..default()
                    }
                },
                TabList {
                    orientation: ControlOrientation::Horizontal,
                    activation: TabActivation::Automatic,
                },
                Role::TabList,
                Indicating::default(),
                observe(tabs_on_select),
                Children::spawn(SpawnIter(
                    labels.into_iter().map(|(label, extra)| tab(label, extra)),
                )),
            ));
            // One grid cell for every page, so the widget is as large as the largest.
            parent.spawn((
                Node {
                    display: Display::Grid,
                    flex_grow: grow,
                    min_width: least,
                    min_height: least,
                    overflow: if fill {
                        Overflow::clip()
                    } else {
                        Overflow::visible()
                    },
                    ..default()
                },
                Role::TabPages,
                pages,
            ));
            parent.spawn(selection_indicator());
        })),
    )
}

/// A vertical strip of tabs, a sidebar's navigation (Projects, Learn, What's new): Bevy's
/// `TabList` with `ControlOrientation::Vertical`, so Up and Down on a focused tab move focus and
/// select it at once, and a click selects; the D-pad's Up and Down step it, LB and RB (and
/// Ctrl+PageUp and Ctrl+PageDown) turn it from anywhere inside, and Left and Right move focus on,
/// into the page beside it. Each tab's label is left-aligned with an optional
/// [`crate::SideTab::badge`] at its end, the tabs are as wide as the strip, and the selected one is
/// lit like a [`tabs`] tab.
///
/// Its pages are not inside it: spawn each with [`side_tab_page`] wherever it belongs (the pane
/// beside a sidebar), naming this widget, and the selected one shows while the others hide. The
/// entity this returns holds [`crate::TabsValue`] (read it, or set it to turn the page from code)
/// and reports each change the player makes as a `ValueChange<usize>` on itself.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// let nav = commands
///     .spawn(side_tabs(["Projects".into(), "Learn".into(), SideTab::new("What's new").badge("3")], 0))
///     .id();
/// commands.spawn(side_tab_page(nav, 0, children![label("Your projects")]));
/// commands.spawn(side_tab_page(nav, 1, children![label("Tutorials")]));
/// commands.spawn(side_tab_page(nav, 2, children![label("Release notes")]));
/// # }
/// ```
pub fn side_tabs(tabs: impl IntoIterator<Item = crate::SideTab>, selected: usize) -> impl Bundle {
    side_strip(AlignSelf::Auto, tabs, selected, side_tab)
}

/// What [`side_tabs`] and [`side_tabs_icons`] share: a `Role::Tabs` root holding the strip's state,
/// the strip (Bevy's vertical `TabList`, first child) that is its own scrolling viewport
/// ([`crate::TabScroll`]), the rail that shows while the tabs do not fit, and the sliding
/// indicator (last child). The root is never taller than a host's height (`max_height` 100%), so a
/// strip with more tabs than room scrolls and one with room is as tall as its tabs.
fn side_strip<F, B>(
    align: AlignSelf,
    tabs: impl IntoIterator<Item = crate::SideTab>,
    selected: usize,
    make: F,
) -> impl Bundle
where
    F: Fn(crate::SideTab) -> B + Send + Sync + 'static,
    B: Bundle,
{
    let tabs: Vec<crate::SideTab> = tabs.into_iter().collect();
    let count = tabs.len();
    let selected = selected.min(count.saturating_sub(1));
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_self: align,
            min_height: Val::Px(0.0),
            max_height: Val::Percent(100.0),
            ..default()
        },
        Role::Tabs,
        TabsValue(selected),
        TabsCount(count),
        observe(tabs_self_update),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            let strip = parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        // The viewport: it takes what the root leaves it and scrolls the rest.
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    TabList {
                        orientation: ControlOrientation::Vertical,
                        activation: TabActivation::Automatic,
                    },
                    Role::TabList,
                    crate::TabScroll::default(),
                    Indicating::default(),
                    observe(tabs_on_select),
                    Children::spawn(SpawnIter(tabs.into_iter().map(make))),
                ))
                .id();
            parent.spawn(list_rail(strip));
            parent.spawn(selection_indicator());
        })),
    )
}

/// One tab of a [`side_tabs`] strip: a tab whose label stands at its start and whose badge, if
/// any, at its end.
fn side_tab(tab: crate::SideTab) -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            // A strip that scrolls squeezes nothing: each tab keeps the size it has.
            flex_shrink: 0.0,
            ..default()
        },
        bevy::ui_widgets::Tab,
        Role::Tab,
        Indicating::default(),
        crate::VerticalTab,
        Hovered::default(),
        AutoDirectionalNavigation::default(),
        Children::spawn(SpawnWith(move |parts: &mut RelatedSpawner<ChildOf>| {
            parts.spawn((Text::new(tab.label), Role::TabText));
            if let Some(badge) = tab.badge {
                parts.spawn((Text::new(badge), Role::TabBadge, ONE_LINE));
            }
        })),
    )
}

/// A [`side_tabs`] strip of pictures: an activity bar's workspaces, a narrow column of tabs where each
/// is its [`crate::SideTab::icon`], its name opens as a tooltip on hover and on focus, and its
/// [`crate::SideTab::badge`] is a small pill at its top-right corner (a count, or "New"). A tab with
/// no picture shows its label's first letter, and a badge of zero shows nothing.
///
/// Everything else is [`side_tabs`]'s: the entity holds [`crate::TabsValue`] and reports each change
/// the player makes as a `ValueChange<usize>` on itself, Up and Down (and the D-pad) step it, Left and
/// Right leave it for the page beside it, [`side_tab_page`] spawns a page anywhere, and the selected
/// tab keeps the sliding indicator.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, server: Res<AssetServer>) {
/// let bar = commands
///     .spawn(side_tabs_icons(
///         [
///             SideTab::new("Files").icon(IconSource::Image(server.load("files.png"))),
///             SideTab::new("Search").icon(IconSource::Image(server.load("search.png"))),
///             SideTab::new("Source Control").icon(IconSource::Image(server.load("branch.png"))),
///             SideTab::new("Problems").count(4).icon(IconSource::Image(server.load("warn.png"))),
///         ],
///         0,
///     ))
///     .id();
/// commands.spawn(side_tab_page(bar, 0, children![label("Your files")]));
/// # }
/// ```
pub fn side_tabs_icons(
    tabs: impl IntoIterator<Item = crate::SideTab>,
    selected: usize,
) -> impl Bundle {
    // A narrow strip beside its pages: as wide as its widest square, not stretched to the host.
    // Its list stretches its tabs, so the strip is as wide as its widest square and the sliding
    // indicator has a box to cover; each tab centres its own picture.
    side_strip(AlignSelf::FlexStart, tabs, selected, icon_tab)
}

/// One tab of a [`side_tabs_icons`] strip: a square target holding the picture (or the label's first
/// letter), its name on a tooltip, and a badge at its top-right corner.
fn icon_tab(tab: crate::SideTab) -> impl Bundle {
    // The name a reader hears: the label, and the badge's count after it — the corner pill is
    // decoration to the eye but it is the one thing that says what still needs the player.
    let name = match &tab.badge {
        Some(badge) => format!("{} ({badge})", tab.label),
        None => tab.label.clone(),
    };
    let initial = tab
        .label
        .chars()
        .next()
        .map(|first| first.to_uppercase().to_string())
        .unwrap_or_default();
    let picture = tab.icon.clone();
    (
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            // A strip that scrolls squeezes nothing: each tab keeps the size it has.
            flex_shrink: 0.0,
            ..default()
        },
        bevy::ui_widgets::Tab,
        Role::IconTab,
        crate::IconTab,
        Indicating::default(),
        crate::VerticalTab,
        Hovered::default(),
        AutoDirectionalNavigation::default(),
        Tooltip::new(&tab.label),
        // A name is the tab's accessible name whatever it draws: the picture is decoration and the
        // letter a stand-in, so neither is read twice.
        AccessibleLabel(name),
        Children::spawn(SpawnWith(move |parts: &mut RelatedSpawner<ChildOf>| {
            match picture {
                Some(source) => {
                    // `icon()` carries `Pickable::IGNORE` itself: a picture is decoration.
                    parts.spawn(icon(source));
                }
                None => {
                    parts.spawn((Text::new(initial), Role::TabText, ONE_LINE));
                }
            }
            if let Some(badge) = tab.badge {
                // The pill's corner is `apply_skin`'s (`Role::CornerBadge`): one owner of where it
                // stands, so a host's own skin can move it.
                parts.spawn((
                    Text::new(badge),
                    Role::CornerBadge,
                    crate::CornerBadge,
                    ONE_LINE,
                    Pickable::IGNORE,
                ));
            }
        })),
    )
}

/// A page of a [`side_tabs`] strip, spawned wherever the host wants it: a column of widgets spaced
/// like a [`menu`], shown while `tabs` (the entity [`side_tabs`] returned) has `index` selected and
/// hidden otherwise. Focus left on a page the strip turns away from moves to the shown page's first
/// widget, or to the selected tab.
pub fn side_tab_page(tabs: Entity, index: usize, widgets: impl Bundle) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::TabPage,
        crate::TabPageOf { tabs, index },
        Visibility::Hidden,
        widgets,
    )
}

/// One tab of a strip: a button-sized target with its label, which Bevy's `TabList` selects on a
/// click and on Left and Right.
fn tab(label: String, extra: impl Bundle) -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        bevy::ui_widgets::Tab,
        Role::Tab,
        Indicating::default(),
        Hovered::default(),
        AutoDirectionalNavigation::default(),
        extra,
        // One line: a tab is a label's height, and a label that wrapped would make one tab
        // taller than the strip it stands in.
        children![(Text::new(label), ONE_LINE, Role::TabText)],
    )
}

/// One page of [`tabs`]: a column of widgets spaced like a [`menu`], shown while its tab is
/// selected. Its widgets are the page's children, in `children![…]`. It stands hidden until
/// [`update_tabs`](crate::update_tabs) shows it, so a page never draws on top of another.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(tabs(
///     ["Game", "Video"],
///     0,
///     children![
///         tab_page(children![button("Resume"), button("Save")]),
///         tab_page(children![toggle("Fullscreen", false)]),
///     ],
/// ));
/// # }
/// ```
pub fn tab_page(widgets: impl Bundle) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            grid_row: GridPlacement::start(1),
            grid_column: GridPlacement::start(1),
            ..default()
        },
        Role::TabPage,
        Visibility::Hidden,
        widgets,
    )
}

/// Two panes side by side or one above the other, with a divider between them that the player
/// moves: a docked layout, an inventory beside a map, a log under a viewport. The panes' content is
/// yours, any bundle each (a [`panel`] fills its pane on both axes); the divider is a channel with a
/// grip in the middle, a target the pointer lands on without aiming, that takes focus like any
/// widget.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     split(
///         SplitAxis::Horizontal,
///         (panel(), children![heading("Items")]),
///         (panel(), children![heading("Map")]),
///     ),
///     SplitFraction(0.3),
///     SplitMin::new(200.0, 320.0),
///     observe(|change: On<ValueChange<f32>>| info!("the split is at {}", change.value)),
/// ));
/// # }
/// ```
///
/// The split fills its parent, so stand it in a node of the size you want (a [`screen`] is the
/// window). It keeps its state in [`SplitFraction`](crate::SplitFraction) (the first pane's share of
/// the room the divider leaves; 0.5 until you insert another with it, as above) and each pane's
/// least size in [`SplitMin`](crate::SplitMin) (none until you say). It reports each change a
/// player makes as a
/// [`ValueChange<f32>`](crate::ValueChange) on itself, `is_final` false while a drag goes on; a
/// value the game sets moves the panes with no event.
///
/// A drag of `d` pixels changes the fraction by `d` over the room, and never takes a pane below its
/// minimum; a window too small for both minimums takes the room from the second pane first. With
/// the divider focused the arrows along the split's axis (Left and Right for
/// [`SplitAxis::Horizontal`], Up and Down for [`SplitAxis::Vertical`]) and the D-pad move it
/// ([`SPLIT_KEY_STEP_PX`](crate::SPLIT_KEY_STEP_PX) a press, a held direction repeating), Home and
/// End take the first pane to its smallest and largest, and Enter, the pad's South button and a
/// double click collapse the first pane to its minimum or give it back ([`split_by_pad`](crate::split_by_pad)); the other
/// arrows move focus on. Give the divider `InteractionDisabled` to fix the layout.
pub fn split(axis: SplitAxis, first: impl Bundle, second: impl Bundle) -> impl Bundle {
    split_in(axis, children![first], children![second])
}

/// [`split`] with each pane's children given as a bundle of its own (`children![..]`, or the
/// dock's adoption of a node it built), instead of one child each.
pub(crate) fn split_in(axis: SplitAxis, first: impl Bundle, second: impl Bundle) -> impl Bundle {
    (
        Node {
            flex_direction: match axis {
                SplitAxis::Horizontal => FlexDirection::Row,
                SplitAxis::Vertical => FlexDirection::Column,
            },
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Split::new(axis),
        Role::Split,
        observe(split_self_update),
        observe(collapse_split),
        children![split_pane(first), split_divider(axis), split_pane(second)],
    )
}

/// One pane of a [`split`]: a clipped cell of one `1fr` track, so content sized to its own content
/// fills the pane, and one too big for it is cut and never widens it. Its share is
/// [`update_splits`](crate::update_splits)'. `content` is the pane's children.
fn split_pane(content: impl Bundle) -> impl Bundle {
    (
        Node {
            display: Display::Grid,
            grid_template_columns: vec![GridTrack::flex(1.0)],
            grid_template_rows: vec![GridTrack::flex(1.0)],
            flex_basis: Val::Px(0.0),
            flex_grow: 0.5,
            overflow: Overflow::clip(),
            ..default()
        },
        Role::SplitPane,
        // A pane is a part of the screen F6 can reach, the same as a viewport or a toolbar: a
        // collapsed pane's first widget is then a region's start, and landing on it peeks the pane
        // (the widget the player lands on is the one they see).
        FocusRegion::navigate(),
        // **No `Hovered` here.** A pane that carried one marked itself as an interface node for every host that asks "is any
        // `Hovered` node under the pointer" to tell the interface from the world behind it: a pane that holds a game's viewport
        // then swallowed every click meant for the world. The peek puts `Hovered` on the first pane only while the split is
        // collapsed (`update_split_peeks`), and a collapsed pane has no size to swallow anything with.
        content,
    )
}

/// A [`split`]'s divider: a focusable channel with a grip, whose pointer and key handling is
/// `split.rs`'s.
fn split_divider(axis: SplitAxis) -> impl Bundle {
    let (width, height) = match axis {
        SplitAxis::Horizontal => (SPLIT_GRIP_THICKNESS, SPLIT_GRIP_LENGTH),
        SplitAxis::Vertical => (SPLIT_GRIP_LENGTH, SPLIT_GRIP_THICKNESS),
    };
    let cursor = match axis {
        SplitAxis::Horizontal => SystemCursorIcon::ColResize,
        SplitAxis::Vertical => SystemCursorIcon::RowResize,
    };
    (
        (
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            SplitDivider::new(axis),
            Role::SplitDivider,
            Hovered::default(),
            TabIndex(0),
            AutoDirectionalNavigation::default(),
            EntityCursor::System(cursor),
        ),
        (
            observe(divider_on_press),
            observe(divider_on_release),
            observe(divider_on_drag_start),
            observe(divider_on_drag),
            observe(divider_on_drag_end),
            observe(divider_on_cancel),
            observe(divider_on_click),
            observe(divider_on_double_press),
            observe(divider_on_key_input),
            observe(toggle_split),
        ),
        children![(
            Node {
                width: Val::Px(width),
                height: Val::Px(height),
                ..default()
            },
            Role::SplitGrip,
            Pickable::IGNORE,
        )],
    )
}

/// A modal dialog: a veil over the whole window that centres its children and holds focus. When it
/// opens, focus moves to its first button (or to the one carrying Bevy's `AutoFocus`, the way to
/// pick the least destructive default); Tab, the arrows and the D-pad stay inside it; and when it
/// closes, focus returns to the button that opened it. Its contents are yours, in `children![…]`;
/// close it with `commands.entity(dialog).despawn()`, from a button's `Activate` and from an
/// observer of [`crate::Cancel`] on the dialog (Escape and the pad's B send one).
///
/// A [`panel`] that is a direct child of the dialog is the dialog, and looks like one: 21 to 36 body
/// text sizes wide (420 to 720 px at the default size, wider with a larger
/// [`text_scale`](crate::GooeySkin::text_scale)), one and a half times a screen panel's padding and
/// gap, an opaque fill, and its frame in [`outline`](crate::GooeySkin::outline). Put a [`heading`]
/// in it for the main instruction, a [`label`] for what the choice costs, and a [`menu`] of the
/// choices. The scrim keeps the window's outer 5% clear, so a long body wraps and the dialog never
/// reaches the bezel.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy::input_focus::AutoFocus;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     dialog(),
///     observe(|cancel: On<Cancel>, mut commands: Commands| commands.entity(cancel.entity).despawn()),
///     children![(panel(), children![
///         heading("Leave the game?"),
///         label("Progress since the last autosave is lost."),
///         (menu(), children![button("Leave"), (button("Stay"), AutoFocus)]),
///     ])],
/// ));
/// # }
/// ```
pub fn dialog() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Role::Scrim,
        Dialog::default(),
        GlobalZIndex(100),
        TabGroup {
            order: 0,
            modal: true,
        },
    )
}

/// A workbench of panels the player arranges: a tree of splits and tab groups built from a
/// [`DockLayout`](crate::DockLayout), which is plain data (serde with the `serde` feature), so it can be saved,
/// loaded and reset. What a panel shows is spawned once by the function registered under its id in
/// the [`PanelRegistry`](crate::PanelRegistry); the dock builds the tree, and a panel's page is moved, never rebuilt,
/// when the layout changes.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn ui(mut commands: Commands, mut panels: ResMut<PanelRegistry>) {
///     panels
///         .register("scene", "Scene", |page| {
///             page.with_child(label("Nothing selected"));
///         })
///         .register("assets", "Assets", |page| {
///             page.with_child(button("Import"));
///         })
///         .register("log", "Log", |page| {
///             page.with_child(label("Ready"));
///         });
///     commands.spawn((
///         dock(DockLayout::split(
///             SplitAxis::Horizontal,
///             0.3,
///             DockLayout::group(["scene", "assets"]),
///             DockLayout::group(["log"]),
///         )),
///         observe(|changed: On<DockLayoutChanged>| info!("save {:?}", changed.layout)),
///     ));
/// }
/// ```
///
/// The dock fills its parent. Every group is a [`panel`] with a strip of tabs; each divider is a
/// [`split`]'s, with its keys and its 96 px minimum pane. The player rearranges the layout by
/// picking a tab up: a card with its title follows the pointer, and the part of the dock the panel
/// will take is lit before it is let go (on a group's strip, into the group at that place; over
/// its middle, as a tab; over an edge, in a group of its own on that side; at the dock's own
/// edge, a quarter of the dock). Escape puts it back. Without a pointer, **Ctrl+Shift and an
/// arrow** on a focused tab moves its panel to the nearest group that way, or to that edge of the
/// dock. Each change is one [`DockLayoutChanged`](crate::DockLayoutChanged) with the layout now
/// shown; write a layout to [`Dock`](crate::Dock) to load or reset it.
///
/// A panel id the registry does not know, or one the layout names twice, is left out and reported
/// once as a [`DockPanelSkipped`](crate::DockPanelSkipped); a group left with no panel collapses.
pub fn dock(layout: DockLayout) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Role::Dock,
        Dock(layout),
        children![dock_body(), dock_drop(), dock_ghost()],
    )
}

/// The part of a dock the layout is built in: one grid cell of `1fr`, so the node the layout is
/// built as fills the dock whatever it is. [`dock`] spawns one, and so does
/// `update_docks` for a host that spawned a bare [`Dock`](crate::Dock) on a node of its own (a
/// bottom strip, a sidebar), beside [`dock_drop`] and [`dock_ghost`], which a tab drag needs.
pub(crate) fn dock_body() -> impl Bundle {
    (
        Node {
            display: Display::Grid,
            grid_template_columns: vec![GridTrack::flex(1.0)],
            grid_template_rows: vec![GridTrack::flex(1.0)],
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Role::DockBody,
    )
}

/// The part of the dock the panel a tab is carried over will take, lit while the tab is carried:
/// absolute and hidden at rest, so it moves nothing, and drawn by `update_dock_drag`.
pub(crate) fn dock_drop() -> impl Bundle {
    (
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            ..default()
        },
        Role::DockDrop,
        Pickable::IGNORE,
    )
}

/// The card that follows the pointer while a tab is carried, with the panel's title on it.
pub(crate) fn dock_ghost() -> impl Bundle {
    (
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            ..default()
        },
        Text::new(""),
        Role::DockGhost,
        Pickable::IGNORE,
    )
}

/// A command palette: a modal field over a searchable list of the game's [`crate::CommandList`], each
/// command with its chord and, when it is off, why. Spawn it when the player presses the game's
/// own chord (Ctrl+Shift+P, say); it takes focus, and everything else is the palette's.
///
/// The player types a few letters of a command ("opn prj" finds "File: Open Project"; recently run
/// commands come first), Up and Down move the lit row while focus stays in the field, Enter or a
/// click runs it, Escape or a click outside closes it. Running sends one [`crate::RunCommand`] with the
/// command's id and despawns the palette, focus back where it was; a command that is off shows its
/// reason where its chord would be and runs nothing. See [`crate::PaletteCommand`] for the list's items.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn setup(mut commands: Commands) {
///     commands.insert_resource(CommandList(vec![
///         PaletteCommand::new("project.open", "Open Project")
///             .category("File")
///             .shortcut(Shortcut::primary(KeyCode::KeyO)),
///         PaletteCommand::new("project.save", "Save").disabled("No project is open"),
///     ]));
/// }
///
/// fn open_palette(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
///     if Shortcut::new(Modifiers::PRIMARY | Modifiers::SHIFT, KeyCode::KeyP).just_pressed(&keys) {
///         commands.spawn(command_palette());
///     }
/// }
///
/// fn plug(app: &mut App) {
///     app.add_systems(Startup, setup)
///         .add_systems(Update, open_palette.run_if(not(is_text_editing)))
///         .add_observer(|run: On<RunCommand>| info!("run {}", run.id));
/// }
/// ```
///
/// Up to eight rows show at once and scroll under the lit one (fewer when large text or a short
/// window leaves no room, so the card never leaves the window). The field's hint ("Type a command")
/// and the line for no match ("No command matches") are English, as a menu's "Nothing here" is.
pub fn command_palette() -> impl Bundle {
    palette_card(CommandPalette::default(), PALETTE_HINT, NO_MATCH)
}

/// A searchable list: the command palette's card over `state`, with `hint` in its field and
/// `no_match` for an empty result. A [`select`] opens one over its own options.
fn palette_card(state: CommandPalette, hint: &'static str, no_match: &'static str) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::FlexStart,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Role::Scrim,
        Dialog::default(),
        GlobalZIndex(100),
        TabGroup {
            order: 0,
            modal: true,
        },
        state,
        observe(palette_on_cancel),
        observe(palette_on_acquire),
        observe(palette_on_click),
        Children::spawn(SpawnWith(move |scrim: &mut RelatedSpawner<ChildOf>| {
            let palette = scrim.target_entity();
            scrim
                .spawn((Node::default(), Role::Palette))
                .with_children(|card| {
                    card.spawn((
                        text_row(None, String::new(), None),
                        PaletteInput(palette),
                        TextHint(hint.into()),
                    ));
                    card.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    })
                    .with_children(|rows| {
                        for slot in 0..PALETTE_ROWS {
                            rows.spawn(palette_row(palette, slot));
                        }
                        rows.spawn((
                            Node {
                                display: Display::None,
                                ..default()
                            },
                            Text::new(no_match),
                            Role::MenuNote,
                            ONE_LINE,
                            PaletteNote(palette),
                            Pickable::IGNORE,
                        ));
                    });
                    card.spawn((
                        Node {
                            display: Display::None,
                            ..default()
                        },
                        Text::new(String::new()),
                        Role::PaletteCount,
                        ONE_LINE,
                        PaletteCount(palette),
                        Pickable::IGNORE,
                    ));
                });
        })),
    )
}

/// One of a palette's reused rows: a command's words and its chord or reason, hidden until
/// [`update_palette`] gives it a command.
fn palette_row(palette: Entity, slot: usize) -> impl Bundle {
    (
        Node {
            display: Display::None,
            ..default()
        },
        bevy::ui_widgets::Button,
        Role::PaletteRow,
        PaletteRow { palette, slot },
        observe(palette_on_activate),
        observe(palette_on_move),
        children![
            (
                Text::new(String::new()),
                Role::MenuText,
                ONE_LINE,
                Pickable::IGNORE
            ),
            (
                Text::new(String::new()),
                Role::MenuShortcut,
                ONE_LINE,
                Pickable::IGNORE
            ),
        ],
    )
}

/// A panel that opens beside `anchor`, stays inside the window, and closes when the player
/// presses anywhere else or backs out.
///
/// Spawn it as a child of nothing: the bundle makes it a child of the anchor, so despawning the
/// anchor closes it. Put any widgets in `children![…]`: a [`menu`] of buttons, a [`list`], a
/// [`label`]. It is a card in the skin's look (an opaque fill, a frame in
/// [`outline`](crate::GooeySkin::outline), a shadow) over everything but a [`dialog`] it was
/// opened from and a [`Tooltip`](crate::Tooltip), and it is placed by Bevy's `Popover`: on the
/// [`Placement`] side of the anchor, flipped to the other side or slid along the anchor's edge
/// when the window leaves no room, and placed again when the window is resized.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, edit_button: Entity) {
/// commands.spawn((
///     popover(edit_button, Placement::Below),
///     children![button("Copy"), button("Paste")],
/// ));
/// # }
/// ```
///
/// It does not take focus: it is for a combo box's list, a hover card, a panel the player reads.
/// Escape (or the pad's B) closes it from the anchor or from inside it, and a pointer press
/// anywhere that is not the panel or the anchor closes it, each sending one
/// [`PopoverClosed`](crate::PopoverClosed)
/// before it is despawned; the press goes on to what it landed on. A press on the anchor does not
/// close it, so the anchor's own `Activate` can toggle it: despawn the popover there when one is
/// open (it is the [`PopoverPanel`](crate::PopoverPanel) whose `anchor` is the button). A press on the panel never
/// presses the anchor, even though the panel is its child. For a menu that takes focus, use
/// [`popover_modal`].
pub fn popover(anchor: Entity, placement: Placement) -> impl Bundle {
    popover_panel(anchor, placement, false)
}

/// A [`popover`] that holds focus, as a menu does: focus moves into it when it opens (to its first
/// widget, or the one carrying Bevy's `AutoFocus`), Tab, the arrows and the D-pad stay inside it,
/// Escape or the pad's B closes it, and when it closes focus goes back to the widget that was
/// focused when it opened. It is the same scope as a [`dialog`] without the veil.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, file_button: Entity) {
/// commands.spawn((
///     popover_modal(file_button, Placement::Below),
///     observe(|closed: On<PopoverClosed>| info!("menu closed: {:?}", closed.reason)),
///     children![button("New"), button("Open"), button("Save")],
/// ));
/// # }
/// ```
pub fn popover_modal(anchor: Entity, placement: Placement) -> impl Bundle {
    (
        popover_panel(anchor, placement, true),
        Dialog::default(),
        TabGroup {
            order: 0,
            modal: true,
        },
    )
}

/// What [`popover`] and [`popover_modal`] share. Born `Display::None` and hidden: [`crate::apply_skin`]
/// shows it, styled, in the frame it is spawned, and [`crate::reveal_popovers`] draws it the frame
/// after, once its text is laid out, so it is never drawn bare or at the wrong width.
fn popover_panel(anchor: Entity, placement: Placement, modal: bool) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        Visibility::Hidden,
        Role::Popover,
        placement_of(placement, 0.0, 0.0),
        // Laid out on the window, not inside its anchor: its width is the room the window leaves
        // and not the anchor's (a narrow button would wrap every line to a word), and no clip of a
        // scrolling list or a panel it hangs from reaches it.
        FixedNode,
        BoxShadow::default(),
        // Over a dialog (100), under a tooltip (200).
        GlobalZIndex(150),
        PopoverPanel::new(anchor, placement, modal),
        ChildOf(anchor),
        observe(popover_on_cancel),
        (
            observe(stop_press),
            observe(stop_release),
            observe(stop_click),
            observe(stop_acquire),
        ),
    )
}

/// A menu bar: a row of [`menu_title`]s across the top of its parent, each opening a menu of
/// commands. F10, or Alt pressed and let go on its own, puts focus on the first title; Left and
/// Right walk the titles (wrapping), Down, Enter, Space or a click opens one with focus on its
/// first item, a pointer moving onto another title switches menus while one is open, and Escape
/// (or the pad's B) closes the menu and then gives focus back to where F10 took it from. Inside a
/// menu Right opens a [`submenu`] or moves on to the next title's menu, Left closes a submenu or
/// moves to the previous title's, and Up, Down, Home and End walk the items.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(menu_bar(children![
///     menu_title(
///         "File",
///         children![
///             (
///                 menu_item("Open...").with_shortcut(Shortcut::primary(KeyCode::KeyO)),
///                 observe(|_: On<Activate>| info!("open")),
///             ),
///             separator(),
///             submenu("Recent", children![menu_item("level-1.ron")]),
///         ]
///     ),
///     menu_title(
///         "View",
///         children![
///             check_item("Grid", true),
///             separator(),
///             radio_item("Small", false),
///             radio_item("Large", true),
///         ]
///     ),
/// ]));
/// # }
/// ```
///
/// The bar is the panel's colour with one line under it and blocks the pointer. It is a row of
/// whatever you pass, so it takes any widget beside the titles (a [`label`] pushed right, say).
pub fn menu_bar(titles: impl Bundle) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            ..default()
        },
        Role::MenuBar,
        MenuBar::default(),
        observe(bar_on_cancel),
        titles,
    )
}

/// A title of a [`menu_bar`]: a button with the menu `items` (a `children![]` of [`menu_item`],
/// [`check_item`], [`radio_item`], [`separator`] and [`submenu`]) that opens under it. The menu is
/// spawned once with the title and shown by opening it, so an `observe(..)` on an item stays
/// where you wrote it. Its title stays lit while its menu is open.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(menu_bar(children![menu_title(
///     "Edit",
///     children![menu_item("Undo"), menu_item("Redo")]
/// )]));
/// # }
/// ```
pub fn menu_title(label: impl Into<String>, items: impl Bundle) -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        bevy::ui_widgets::Button,
        Role::MenuTitle,
        MenuTitle,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        children![(Text::new(label), Role::MenuTitleText), menu_panel(items)],
    )
}

/// A command in a menu: a row with `label` that sends [`Activate`](crate::Activate) when chosen by Enter, Space, the
/// pad's South button or a click, and closes the menu. Chain [`MenuItem::with_shortcut`] to show
/// its chord at the end of the row, and add [`Disabled`](crate::Disabled) to make it unavailable
/// with a reason.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     menu_item("Save").with_shortcut(Shortcut::primary(KeyCode::KeyS)),
///     observe(|_: On<Activate>| info!("save")),
/// ));
/// # }
/// ```
pub fn menu_item(label: impl Into<String>) -> MenuItem {
    MenuItem::command(label)
}

/// A menu item that is on or off: it keeps its state in Bevy's [`Checked`] (a tick in the mark
/// slot while on) and flips it on every choice, sending a [`ValueChange<bool>`](crate::ValueChange) with the new value.
/// The menu closes, like any command.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     check_item("Show grid", true),
///     observe(|change: On<ValueChange<bool>>| info!("grid: {}", change.value)),
/// ));
/// # }
/// ```
pub fn check_item(label: impl Into<String>, checked: bool) -> MenuItem {
    MenuItem::check(label, checked)
}

/// One of a run of choices in a menu, exactly one of which is on: choosing it sets its own
/// [`Checked`] (a dot in the mark slot) and clears the [`radio_item`]s next to it, up to the
/// nearest [`separator`] or other kind of row either way, and sends a [`ValueChange<bool>`](crate::ValueChange) with
/// `true` for it. Choosing the one that is on changes nothing and sends nothing.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((Node::default(), children![
///     radio_item("Small", false),
///     radio_item("Large", true),
/// ]));
/// # }
/// ```
pub fn radio_item(label: impl Into<String>, checked: bool) -> MenuItem {
    MenuItem::radio(label, checked)
}

/// A menu item that opens another menu `items` to its right (Right, Enter, Space or a click opens
/// it; Left or Escape closes it and puts focus back on the item). A submenu with nothing in it
/// says "Nothing here". Add [`Disabled`](crate::Disabled) to the item to make it unavailable.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(submenu("Recent", children![menu_item("level-1.ron"), menu_item("level-2.ron")]));
/// # }
/// ```
pub fn submenu(label: impl Into<String>, items: impl Bundle) -> impl Bundle {
    (MenuItem::submenu(label), children![menu_panel(items)])
}

/// A menu that belongs to a thing: spawn it once, anywhere, and name it with
/// [`ContextMenu`](crate::ContextMenu) on whatever should open it. A secondary click on the thing
/// opens it at the pointer (flipping at the window's edge), and Shift+F10 or the Menu key with
/// focus on the thing, or a widget under it, opens it below that widget. `items` are the rows of a
/// menu ([`menu_item`], [`check_item`], [`radio_item`], [`submenu`], [`separator`]), live
/// entities, so a game's `observe(..)` sits on the item; [`ContextTarget`](crate::ContextTarget)
/// says which thing it was opened for. Closed, it is drawn nowhere and in no focus scope; open, it
/// is a modal popover, so Escape, B or a press outside close it and give focus back.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn setup(mut commands: Commands) {
/// let menu = commands
///     .spawn(context_menu(children![
///         menu_item("Rename").with_shortcut(Shortcut::new(Modifiers::NONE, KeyCode::F2)),
///         separator(),
///         menu_item("Delete"),
///     ]))
///     .id();
/// commands.spawn((button("Row"), ContextMenu(menu)));
/// # }
/// ```
pub fn context_menu(items: impl Bundle) -> impl Bundle {
    menu_panel(items)
}

/// A line between two groups of a menu. It is not focusable and the pointer goes through it to
/// the card, and it ends a run of [`radio_item`]s.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((Node::default(), children![menu_item("Open"), separator(), menu_item("Quit")]));
/// # }
/// ```
pub fn separator() -> impl Bundle {
    (Node::default(), Role::MenuSeparator, Pickable::IGNORE)
}

/// A line of news at the edge of the screen: "Game saved", "Quest updated". Spawn it from any
/// system, with no layout and nothing else to set up; it stands as long as its message takes to
/// read, fades out and removes itself ([`crate::Toast`], [`crate::ToastSettings`]).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn save(mut commands: Commands) {
///     commands.spawn(toast("Game saved"));
/// }
/// ```
///
/// It takes no input and no focus, so it never gets in the way of what is under it; it is not
/// announced to screen readers. See [`Toast`](crate::Toast) for its lifecycle. Add a button with
/// [`ToastBundle::action`] (`toast("Item deleted").action("Undo", "undo")`, which then has one
/// thing that takes input: the button) and make it stand until dismissed with
/// [`ToastBundle::sticky`]; identical toasts arriving close together are merged into one that
/// reads "x3", and every toast is kept in the [`ToastLog`](crate::ToastLog).
pub fn toast(text: impl Into<String>) -> ToastBundle {
    ToastBundle::new(ToastKind::Info, text.into())
}

/// A [`toast`] for a refusal or a failure ("Inventory full", "Could not save"): the same line with
/// a mark in [`crate::GooeySkin::danger`] where an ordinary one's is the accent. The words say it
/// too; the mark is never the only sign.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn full(mut commands: Commands) {
///     commands.spawn(toast_alert("Inventory full"));
/// }
/// ```
pub fn toast_alert(text: impl Into<String>) -> ToastBundle {
    ToastBundle::new(ToastKind::Alert, text.into())
}

/// A tool's bar: a full-width row of [`icon_button`]s, [`toggle_group`]s, [`toolbar_separator`]s and
/// any other widget, on the menu bar's surface with one quiet line under it. When the items do not
/// fit the bar's width the **last** ones hide and a "More" button appears whose menu lists them;
/// choosing a row there activates the item ([`fit_bars`](crate::fit_bars)).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, server: Res<AssetServer>) {
/// let glyph = |name: &str| IconSource::Image(server.load(name.to_string()));
/// commands.spawn(toolbar(children![
///     (
///         icon_button(glyph("save.png"), Tooltip::new("Save").with_shortcut(Shortcut::primary(KeyCode::KeyS))),
///         observe(|_: On<Activate>| info!("save")),
///     ),
///     toolbar_separator(),
///     (
///         toggle_group(
///             [
///                 GroupOption::icon(glyph("move.png"), Tooltip::new("Move")),
///                 GroupOption::icon(glyph("paint.png"), Tooltip::new("Paint")),
///             ],
///             0,
///         ),
///         observe(|mode: On<ValueChange<usize>>| info!("tool {}", mode.value)),
///     ),
/// ]));
/// # }
/// ```
///
/// The bar fills its parent's width and its items never decide it, so a window narrower than the
/// items overflows them instead of widening the bar. Put it where the width is the window's or a
/// panel's own (a [`screen`], a split pane, a column other widgets size): alone in a parent that
/// sizes to its content it has no width to fill. Every control is a tab stop and the arrows and
/// the D-pad move focus along the row.
pub fn toolbar(items: impl Bundle) -> impl Bundle {
    (
        // No width of its own and its parent's as its least: the items never decide how wide the
        // bar is (a bar sized by its items would be as wide as them in a window too narrow for
        // them, and never overflow), and the bar fills the room its parent has.
        Node {
            width: Val::Px(0.0),
            min_width: Val::Percent(100.0),
            ..default()
        },
        Role::Toolbar,
        Toolbar::default(),
        items,
    )
}

/// A toolbar's button: a target-sized square with a picture and no words, so `tooltip` is its
/// name, shown on hover and on visible focus (with its shortcut, if it has one) and listed in the
/// bar's "More" menu when it does not fit. Observe [`Activate`](crate::Activate) on it like any
/// [`button`]; the picture is tinted by the button's state ([`icon`]).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, server: Res<AssetServer>) {
/// commands.spawn((
///     icon_button(IconSource::Image(server.load("undo.png")), Tooltip::new("Undo")),
///     observe(|_: On<Activate>| info!("undo")),
/// ));
/// # }
/// ```
pub fn icon_button(source: IconSource, tooltip: Tooltip) -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        bevy::ui_widgets::Button,
        Role::Button,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        tooltip,
        children![icon(source)],
    )
}

/// A button for a mark too small to press: a caret, a close cross, a chevron. Its hit area is the
/// skin's [`target`](crate::GooeySkin::target) on each side (a [`ControlSize`](crate::ControlSize)
/// taller) **whatever the size of the glyph**, with the glyph centred in it and no padding of its
/// own, so a 6 px mark is as easy to hit as a 48 px one. `name` is what a screen reader says and
/// the tooltip's line; the glyph is only what is drawn. Observe [`Activate`](crate::Activate) on it
/// like any [`button`]; it has a button's fill, frame, focus ring and states.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     glyph_button("v", "Fold the group"),
///     observe(|_: On<Activate>| info!("fold")),
/// ));
/// # }
/// ```
pub fn glyph_button(glyph: impl Into<String>, name: impl Into<String>) -> impl Bundle {
    let name = name.into();
    (
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        bevy::ui_widgets::Button,
        Role::GlyphButton,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        Tooltip::new(name.clone()),
        AccessibleLabel(name),
        children![(Text::new(glyph), ONE_LINE, Role::ButtonText)],
    )
}

/// A tab for a strip the host selects itself (a workspace's modes, a pane's kinds): its own inset
/// and, along its bottom, an edge that is **always there** and lit in the accent while the tab
/// carries Bevy's [`Selected`](bevy::ui::Selected), so selecting another changes a colour and moves
/// no neighbour. The selected tab is the surface under it, the others a button's raised fill,
/// lifted under the pointer. Put the tabs in a [`row`] and write `Selected` on the one that is on
/// (and take it off the one that was); listen for `PointerClick` on a tab for the player.
///
/// It is not a button: it takes no focus and answers no Enter, so a host whose keys already move
/// the strip (chords, say) is not heard twice. For a strip that owns its keys and its pages use
/// [`tabs`].
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((row(0.0), children![
///     (edge_tab("Meshes"), bevy::ui::Selected),
///     edge_tab("Tiles"),
/// ]));
/// # }
/// ```
pub fn edge_tab(label: impl Into<String>) -> impl Bundle {
    (
        Node::default(),
        Role::EdgeTab,
        Hovered::default(),
        children![(Text::new(label), ONE_LINE, Role::TabText)],
    )
}

/// A full-width row that is a button: a grouped list's header that folds the group, a "show more"
/// line. It is as wide as its host, stands half a gap off what is above it, and has a button's
/// fill, frame, focus ring and states, so it shades under the pointer exactly as a list's row does.
/// Fill it with words ([`fill_text`], [`count_text`], or a mark as `(Text::new("v"),
/// Role::ButtonText)`); they follow the band's state. Observe
/// [`Activate`](crate::Activate) on it to fold or open.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     band(),
///     children![
///         (Text::new("v"), Role::ButtonText),
///         fill_text("kenney_prototype-kit"),
///         count_text("145"),
///     ],
///     observe(|_: On<Activate>| info!("fold")),
/// ));
/// # }
/// ```
pub fn band() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        },
        bevy::ui_widgets::Button,
        Role::Band,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
    )
}

/// Text that is a flex item of its own and takes the room its row leaves: it grows, gives way to
/// its neighbours and has no least width, and wraps at a word or, for one that is longer than the
/// room, at a letter. So a long name wraps **inside its own column** and never pushes its row wider
/// or out of its host. Beside it put a [`count_text`], which keeps its own.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((row(1.0), children![fill_text("low_poly_furniture/glb/"), count_text("8")]));
/// # }
/// ```
pub fn fill_text(text: impl Into<String>) -> impl Bundle {
    (
        Text::new(text),
        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
        Role::FillText,
        Pickable::IGNORE,
    )
}

/// A [`label`] with a [`TextFlow`](crate::TextFlow): whether it grows, gives way, its least width
/// and where it stands on the cross axis, each only if set.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     row(1.0),
///     children![label_with("Title", TextFlow { grow: Some(1.0), min_width: Some(Extent::Px(0.0)), ..default() })],
/// ));
/// # }
/// ```
pub fn label_with(text: impl Into<String>, flow: crate::TextFlow) -> impl Bundle {
    (label(text), flow)
}

/// A [`fill_text`] with a [`TextFlow`](crate::TextFlow): it keeps growing and giving way unless the
/// flow says otherwise.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     row(1.0),
///     children![fill_text_with("receipt", TextFlow { align: Some(AlignSelf::Center), ..default() })],
/// ));
/// # }
/// ```
pub fn fill_text_with(text: impl Into<String>, flow: crate::TextFlow) -> impl Bundle {
    (fill_text(text), flow)
}

/// Text that is a flex item of its own and keeps its words whole: it never shrinks and never wraps,
/// so a tally ("145", "8 pack(s), 120 mesh(es)") holds a column of its own at the end of a row
/// whose [`fill_text`] gives way to it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((row(1.0), children![fill_text("Electronics"), count_text("145")]));
/// # }
/// ```
pub fn count_text(text: impl Into<String>) -> impl Bundle {
    (Text::new(text), ONE_LINE, Role::CountText, Pickable::IGNORE)
}

/// A row of options of which exactly one is on, like a toolbar's tool modes: the one on is lit in
/// the accent, like the selected tab. Choosing another with Enter, Space, South or a click sends
/// one [`ValueChange<usize>`](crate::ValueChange) on the group (the index of the option) and moves
/// the light; choosing the one that is on sends nothing. The state is
/// [`ToggleGroupValue`](crate::ToggleGroupValue): set it to change the mode from code, and the
/// group follows with no event.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     toggle_group([GroupOption::text("Edit"), GroupOption::text("Play")], 0),
///     observe(|mode: On<ValueChange<usize>>| info!("mode {}", mode.value)),
/// ));
/// # }
/// ```
pub fn toggle_group(
    options: impl IntoIterator<Item = GroupOption>,
    selected: usize,
) -> impl Bundle {
    let options: Vec<GroupOption> = options.into_iter().collect();
    let count = options.len();
    let selected = selected.min(count.saturating_sub(1));
    (
        Node::default(),
        Role::ToggleGroup,
        Indicating::default(),
        ToggleGroupValue(selected),
        ToggleGroupCount(count),
        observe(group_self_update),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            for (index, option) in options.into_iter().enumerate() {
                let mut entity = parent.spawn((
                    Node {
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    bevy::ui_widgets::Button,
                    Role::GroupOption,
                    Indicating::default(),
                    GroupIndex(index),
                    Hovered::default(),
                    TabIndex(0),
                    AutoDirectionalNavigation::default(),
                    observe(option_on_activate),
                ));
                if let Some(tooltip) = option.tooltip {
                    entity.insert(tooltip);
                }
                match option.content {
                    OptionContent::Text(label) => {
                        entity.with_child((Text::new(label), Role::TabText, ONE_LINE));
                    }
                    OptionContent::Icon(source) => {
                        entity.with_child(icon(source));
                    }
                }
            }
            parent.spawn(selection_indicator());
        })),
    )
}

/// A thin vertical line between two groups of a [`toolbar`]. (The menu's [`separator`] is the
/// horizontal one.)
pub fn toolbar_separator() -> impl Bundle {
    (Node::default(), Role::ToolbarSeparator, Pickable::IGNORE)
}

/// A tool's status line along the foot of the window: workspace state on the left, the context's
/// on the right, each a row of [`status_item`]s. When the window is too narrow the lowest
/// [`StatusPriority`](crate::StatusPriority) hides first, the right group's before the left's
/// ([`fit_bars`](crate::fit_bars)); a hidden item returns when the room does.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(status_bar(
///     children![status_item("main"), (status_item("2 problems"), StatusKind::Warning)],
///     children![(status_item("UTF-8"), StatusPriority(10)), status_item("Ln 4, Col 12")],
/// ));
/// # }
/// ```
pub fn status_bar(left: impl Bundle, right: impl Bundle) -> impl Bundle {
    (
        // As a toolbar's: the items never decide the bar's width, its parent's room does.
        Node {
            width: Val::Px(0.0),
            min_width: Val::Percent(100.0),
            ..default()
        },
        Role::StatusBar,
        StatusBar,
        children![
            (
                Node {
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    ..default()
                },
                Role::StatusGroup,
                StatusGroup { left: true },
                left,
            ),
            (
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Role::StatusGroup,
                StatusGroup { left: false },
                right,
            ),
        ],
    )
}

/// One entry of a [`status_bar`]: a short line of text a target high that sends
/// [`Activate`](crate::Activate) on Enter, Space, South or a click, and takes a
/// [`Tooltip`](crate::Tooltip), a [`StatusKind`](crate::StatusKind) and a
/// [`StatusPriority`](crate::StatusPriority). Edit its [`Text`] child to change the words.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     status_item("Saving..."),
///     StatusKind::Warning,
///     Tooltip::new("Open the notification log"),
///     observe(|_: On<Activate>| info!("log")),
/// ));
/// # }
/// ```
pub fn status_item(text: impl Into<String>) -> impl Bundle {
    (
        Node::default(),
        bevy::ui_widgets::Button,
        Role::StatusItem,
        StatusItem,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        children![status_mark(), (Text::new(text), Role::ButtonText, ONE_LINE),],
    )
}

/// The mark at the start of a [`status_item`], on its own: a thin bar the height of its line that
/// is the accent for a [`StatusKind::Warning`](crate::StatusKind::Warning) and the danger colour
/// for an error and is not shown at all for plain information. It reads the
/// [`StatusKind`](crate::StatusKind) of **the node it stands in**, so a job row that is not a status
/// item puts `StatusKind` on its own row and a `status_mark()` inside it. It takes no pointer.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     row(1.0),
///     StatusKind::Error,
///     children![status_mark(), fill_text("bake_navmesh failed: 3 errors")],
/// ));
/// # }
/// ```
pub fn status_mark() -> impl Bundle {
    (Node::default(), Role::StatusMark, Pickable::IGNORE)
}

/// A [`status_item`] that also shows a short [`progress`] bar after its words, for a task that
/// runs while the player works: change the [`Progress`](crate::Progress) on the bar (the item's
/// second child's child) or query for it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(status_progress("Baking", 0.4));
/// # }
/// ```
pub fn status_progress(text: impl Into<String>, fraction: f32) -> impl Bundle {
    (
        Node::default(),
        bevy::ui_widgets::Button,
        Role::StatusItem,
        StatusItem,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        children![
            status_mark(),
            (Text::new(text), Role::ButtonText, ONE_LINE),
            progress(fraction),
        ],
    )
}

/// A button's look with none of its behaviour: the fill, frame, height and hover of a [`button`]
/// and its words, but not Bevy's `Button`, so it sends no [`Activate`](crate::Activate), takes no
/// focus (no tab stop, no Enter or Space) and has no pressed state. It is for the control that is
/// not an action of the interface's own: a "Cancel" on a job row whose host listens for the
/// pointer click itself (`On<PointerClick>` through [`observe`](crate::observe)) and keeps it
/// off the `Activate` bus that screen-wide handlers hear. For anything a keyboard or a pad should
/// reach, use [`button`].
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     plain_button("Cancel"),
///     observe(|_: On<PointerClick>| info!("cancel the job")),
/// ));
/// # }
/// ```
pub fn plain_button(text: impl Into<String>) -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        Role::Button,
        Hovered::default(),
        children![(Text::new(text), Role::ButtonText)],
    )
}

/// A glowing dot that says whether something is live, with its label and the state in words: an
/// ["Agent  live" row](crate::LedState). Write [`Led`](crate::Led) on it to change the state; each
/// state has its own silhouette (a hollow ring, a disc, a rounded square, a diamond) and its word, so
/// colour is never the only signal, and the glow of a busy one pulses unless motion is reduced.
/// What "live" means is the game's: the widget draws the state it is given.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, agent: Query<Entity, With<Led>>) {
/// commands.spawn(led("Agent", LedState::Busy));
/// // Later, when the checks pass:
/// for entity in &agent {
///     commands.entity(entity).insert(Led(LedState::Live));
/// }
/// # }
/// ```
pub fn led(label: impl Into<String>, state: LedState) -> impl Bundle {
    (
        Node::default(),
        Role::Led,
        Led(state),
        Pickable::IGNORE,
        children![
            (
                Node::default(),
                Role::LedDot,
                BackgroundColor::default(),
                BorderColor::default(),
                BoxShadow::default(),
                Pickable::IGNORE,
            ),
            (Text::new(label), Role::Label, ONE_LINE, Pickable::IGNORE),
            (
                Text::new(state.word()),
                Role::Label,
                ONE_LINE,
                LedWord,
                Pickable::IGNORE,
            ),
        ],
    )
}

/// A labelled field with a list of options under it that narrows as the player types: a desktop
/// picker, where a [`select`] opens a card over the screen. The field shows the chosen option's
/// label; typing, Down, Up or a click opens the list, typing narrows it with the command palette's
/// fuzzy matcher ("par" finds "Paraguay"), Down and Up (or Ctrl+N and Ctrl+P) move the lit option
/// and wrap past the ends, skipping disabled ones, and Enter or a click on an option chooses it:
/// the field shows its label, the list closes and one [`ValueChange<String>`](crate::ValueChange)
/// with the option's id is sent on the combobox (none when it was already the choice). Escape with
/// the list open closes it and puts the chosen label back; a second Escape is an ordinary
/// [`Cancel`](crate::Cancel). A press outside, or focus leaving the field, closes it too.
///
/// The state is [`ComboValue`](crate::ComboValue), the chosen id: set it to choose from code. With
/// no option matching the list says "No option matches" and Enter chooses nothing. The list is a
/// [`popover`] of a [`list`]: it scrolls, shows [`COMBO_ROWS`](crate::COMBO_ROWS) rows at a time and
/// builds one row per option, so it is for tens or hundreds of options, not thousands. It is typed:
/// a pad that cannot type chooses with a [`select`].
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     combobox("Country", [
///         ComboOption::new("py", "Paraguay").detail("South America"),
///         ComboOption::new("pa", "Panama"),
///         ComboOption::new("fr", "France"),
///     ]),
///     ComboValue(Some("fr".into())),
///     observe(|change: On<ValueChange<String>>| info!("country {}", change.value)),
/// ));
/// # }
/// ```
pub fn combobox(
    label: impl Into<String>,
    options: impl IntoIterator<Item = ComboOption>,
) -> impl Bundle {
    crate::combobox::combobox_bundle(label.into(), options.into_iter().collect())
}

/// The field row of a [`combobox`]: a [`text_input`]'s row, labelled.
pub(crate) fn combo_field_row(label: String) -> impl Bundle {
    text_row(Some(label), String::new(), Some(TEXT_INPUT_CHARS))
}

/// A row that chooses one of a list of options: a [`choice`] with up to eight, and above eight a
/// searchable list. The row looks and steps like a choice (Left and Right, Home and End, the
/// D-pad), and its state is the same [`ChoiceValue`](crate::ChoiceValue); with more than
/// [`SELECT_FILTER_ABOVE`](crate::SELECT_FILTER_ABOVE) options Enter, Space, South or a click
/// opens a list instead of cycling: type to narrow it, Up and Down (or the D-pad) move the lit
/// option, Enter, South or a click chooses it and Escape leaves the value alone. Choosing sends one
/// [`ValueChange<usize>`](crate::ValueChange) on the row, with the option's index in the list you
/// gave, and none when the lit option is the one already chosen.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     select("Language", ["English", "Deutsch", "Español", "Français", "Italiano", "Nederlands",
///         "Polski", "Português", "Svenska", "Türkçe"], 0),
///     observe(|change: On<ValueChange<usize>>| info!("language {}", change.value)),
/// ));
/// # }
/// ```
///
/// While the list is open the options are matched by the same fuzzy rule as the command palette
/// ("par" finds "Paraguay"), keeping the list's own order among equal matches; with none the card
/// says "No option matches" and Enter chooses nothing. Like the palette it is a card at the top of
/// the window and not under the row.
pub fn select<S: Into<String>>(
    label: impl Into<String>,
    options: impl IntoIterator<Item = S>,
    selected: usize,
) -> impl Bundle {
    let options: Vec<String> = options.into_iter().map(Into::into).collect();
    (
        choice(label, options.clone(), selected),
        Select::new(options),
        observe(select_on_activate),
    )
}

/// The list a [`select`] opens: the palette's card over the select's own options, with the one the
/// row has lit.
pub(crate) fn select_list(select: Entity, options: &[String], selected: usize) -> impl Bundle {
    palette_card(
        CommandPalette::for_select(select, options, selected),
        SELECT_HINT,
        SELECT_NO_MATCH,
    )
}

/// A collapsible section: a header you press to fold the group of widgets under it away, so a long
/// inspector or settings screen stays scannable. `children` is any bundle (`children![..]`); a
/// section inside it is a nested section. Enter, Space, South and a click on the header toggle it
/// and send one [`ValueChange<bool>`](crate::ValueChange) on the section (the new state); the state
/// is [`SectionOpen`](crate::SectionOpen), which the game may set (no event). A closed section is
/// only its header: its widgets are not laid out and focus never stops in them, and focus inside
/// one that closes goes to the header. A section with nothing in it shows "Empty".
///
/// Add a [`SectionId`](crate::SectionId) and the section keeps its state across a despawn and a
/// respawn ([`SectionStates`](crate::SectionStates)): `open` is only its first choice.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     panel(),
///     children![
///         (
///             section("Physics", true, children![slider("Gravity", 9.8, 0.0..=20.0), toggle("Wind", false)]),
///             SectionId::new("physics"),
///         ),
///         section("Debug", false, children![toggle("Show colliders", false)]),
///     ],
/// ));
/// # }
/// ```
pub fn section(title: impl Into<String>, open: bool, children: impl Bundle) -> impl Bundle {
    let title = title.into();
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::Section,
        SectionOpen(open),
        observe(section_self_update),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            parent.spawn((
                Node {
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    ..default()
                },
                bevy::ui_widgets::Button,
                Role::Button,
                SectionHeader,
                Hovered::default(),
                TabIndex(0),
                AutoDirectionalNavigation::default(),
                observe(header_on_activate),
                Children::spawn(SpawnWith(move |header: &mut RelatedSpawner<ChildOf>| {
                    let chevron = |role, at: Vec2, shown: bool| {
                        (
                            Node {
                                display: if shown { Display::Flex } else { Display::None },
                                ..default()
                            },
                            UiTransform {
                                translation: Val2::percent(at.x, at.y),
                                rotation: Rot2::degrees(45.0),
                                ..default()
                            },
                            role,
                            Pickable::IGNORE,
                        )
                    };
                    header.spawn(chevron(
                        Role::TreeClosed,
                        Vec2::new(-CHEVRON_INK_SHIFT, 0.0),
                        !open,
                    ));
                    header.spawn(chevron(
                        Role::TreeOpen,
                        Vec2::new(0.0, -CHEVRON_INK_SHIFT),
                        open,
                    ));
                    header.spawn((
                        Text::new(title),
                        Role::ButtonText,
                        ONE_LINE,
                        Pickable::IGNORE,
                    ));
                })),
            ));
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    display: if open { Display::Flex } else { Display::None },
                    ..default()
                },
                if open {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
                Role::SectionBody,
                children,
            ));
            parent.spawn((
                Node {
                    display: Display::None,
                    ..default()
                },
                Text::new("Empty"),
                Role::SectionNote,
                Pickable::IGNORE,
            ));
        })),
    )
}

/// The log of every toast, as a list the player can scroll and read: "0:07  Item deleted", the
/// newest first, so a toast that faded before it was read can be found. Put it in a panel or a
/// dialog of your own; it is a [`list`] of buttons (rows are tab stops, so the keys and the D-pad
/// scroll it like any list), and an alert's words start with "! ".
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((panel(), children![heading("Notifications"), notification_center()]));
/// # }
/// ```
///
/// It reads the [`ToastLog`](crate::ToastLog), which keeps the last 200 toasts; clear it to
/// empty the center.
pub fn notification_center() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::NotificationCenter,
        crate::notification::NotificationCenter,
        children![list(NOTIFICATION_ROWS, children![])],
    )
}

/// How many rows of the log the center shows before it scrolls.
const NOTIFICATION_ROWS: f32 = 5.5;
