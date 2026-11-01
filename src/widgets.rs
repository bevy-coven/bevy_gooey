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
use bevy::text::{EditableText, Justify, LineBreak, TextCursorStyle};
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{Checked, FixedNode};
use bevy::ui_widgets::{
    Checkbox, ControlOrientation, ScrollArea, Scrollbar, ScrollbarThumb, Slider, SliderOrientation,
    SliderRange, SliderStep, SliderThumb, SliderValue, TabActivation, TabList, TextInput,
    TrackClick, checkbox_self_update, observe, slider_self_update,
};
use bevy::window::SystemCursorIcon;

use crate::binding::{
    Binding, BindingValue, binding_on_activate, binding_on_key_input, binding_self_update,
};
use crate::choice::{
    ChoiceCount, ChoiceValue, StepBy, arrow_on_click, arrow_on_press, choice_on_activate,
    choice_on_key_input, choice_on_move, choice_self_update,
};
use crate::list::ListView;
use crate::menu::{MenuBar, MenuItem, MenuTitle, bar_on_cancel, menu_panel};
use crate::meter::Meter;
use crate::model::{PadStyle, meter_fraction, slider_fraction, slider_text};
use crate::nav::Dialog;
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
use crate::prompt::Prompt;
use crate::skin::Role;
use crate::split::{
    Split, SplitAxis, SplitDivider, divider_on_cancel, divider_on_click, divider_on_drag,
    divider_on_drag_end, divider_on_drag_start, divider_on_key_input, divider_on_press,
    divider_on_release, split_self_update, toggle_split,
};
use crate::tabs::{TabsCount, TabsValue, tabs_on_select, tabs_self_update};
use crate::text::{
    TextHint, TextInputOf, TextInputValue, text_on_key_input, text_row_on_acquire, text_self_update,
};
use crate::toast::{Toast, ToastKind};

/// A control's label is one line: the row is as wide as its label and its control need, so a label
/// that wraps was wrapped by a rounding error in the row's width (a label 210 px wide in a row that
/// came out a hair under 210 stood on two lines at 125% text, and the row was twice as tall).
pub(crate) const ONE_LINE: TextLayout = TextLayout::new(Justify::Left, LineBreak::NoWrap);

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

/// A text field's caret, as a share of the font size: 2 px at the body size, a hair thinner than
/// Bevy's 0.2 so it does not cover a letter's stem.
const TEXT_CARET_WIDTH: f32 = 0.1;

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

/// A framed, padded column on the skin's surface colour.
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

/// A HUD meter (health, stamina, progress): a framed track with a fill `value / max` of its length.
/// The frame marks the whole length against the panel; the fill sits one border inside it, with
/// concentric rounded corners. It is [`crate::GooeySkin::meter_height`] high times the text scale,
/// so it grows with the player's size option. Change its [`Meter`] component to move it.
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
        children![(
            Node {
                width: Val::Percent(meter_fraction(value, max) * 100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Role::MeterFill
        )],
    )
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
            (Text::new(label), ONE_LINE, Role::ControlText),
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
                    (Text::new(label), ONE_LINE, Role::ControlText),
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
            (
                Text::new(label),
                ONE_LINE,
                Role::ControlText,
                Pickable::IGNORE
            ),
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
            (
                Text::new(label),
                ONE_LINE,
                Role::ControlText,
                Pickable::IGNORE
            ),
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

/// The row under [`text_input`]: a label if it has one and the field, which is `chars` character
/// advances wide, or fills the room its parent leaves when `chars` is `None` (a command palette's
/// field has no label and the card's whole width).
pub(crate) fn text_row(label: Option<String>, value: String, chars: Option<f32>) -> impl Bundle {
    let fills = chars.is_none();
    (
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            width: if fills {
                Val::Percent(100.0)
            } else {
                Val::Auto
            },
            ..default()
        },
        if fills {
            Role::PaletteField
        } else {
            Role::TextInput
        },
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
            row.spawn((
                Node {
                    flex_grow: if fills { 1.0 } else { 0.0 },
                    ..default()
                },
                Role::TextCell,
            ))
            .with_children(|cell| {
                cell.spawn((
                    Node {
                        flex_shrink: 0.0,
                        flex_grow: if fills { 1.0 } else { 0.0 },
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    TextInput,
                    EditableText {
                        visible_width: chars,
                        cursor_width: TEXT_CARET_WIDTH,
                        ..EditableText::new(&value)
                    },
                    TextLayout::no_wrap(),
                    TextCursorStyle::default(),
                    Role::TextField,
                    TextInputOf(root),
                    Hovered::default(),
                    TabIndex(0),
                    AutoDirectionalNavigation::default(),
                    observe(text_on_key_input),
                ));
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

/// The legend of a screen: a slim bar of [`prompt`]s in its parent's bottom-left corner, 5% in from
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
    (
        Node {
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            ..default()
        },
        Role::Prompts,
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
            // The thumb has no `Node`: Bevy lays it out itself, after layout, from the viewport.
            // The rail is hidden until `update_lists` finds the rows do not fit.
            parent.spawn((
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
            ));
        })),
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
    let (grow, least) = if fill { (1.0, Val::Px(0.0)) } else { (0.0, Val::Auto) };
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
                Node {
                    display: Display::Grid,
                    grid_template_columns: vec![RepeatedGridTrack::flex(
                        u16::try_from(count).unwrap_or(u16::MAX),
                        1.0,
                    )],
                    ..default()
                },
                TabList {
                    orientation: ControlOrientation::Horizontal,
                    activation: TabActivation::Automatic,
                },
                Role::TabList,
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
                    overflow: if fill { Overflow::clip() } else { Overflow::visible() },
                    ..default()
                },
                Role::TabPages,
                pages,
            ));
        })),
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
        Hovered::default(),
        AutoDirectionalNavigation::default(),
        extra,
        children![(Text::new(label), Role::TabText)],
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
        CommandPalette::default(),
        observe(palette_on_cancel),
        observe(palette_on_acquire),
        observe(palette_on_click),
        Children::spawn(SpawnWith(|scrim: &mut RelatedSpawner<ChildOf>| {
            let palette = scrim.target_entity();
            scrim
                .spawn((Node::default(), Role::Palette))
                .with_children(|card| {
                    card.spawn((
                        text_row(None, String::new(), None),
                        PaletteInput(palette),
                        TextHint(PALETTE_HINT.into()),
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
                            Text::new(NO_MATCH),
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
        children![(Text::new(label), Role::ButtonText), menu_panel(items)],
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
/// read, fades out and removes itself ([`Toast`], [`crate::ToastSettings`]).
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
/// announced to screen readers. See [`Toast`] for its lifecycle.
pub fn toast(text: impl Into<String>) -> impl Bundle {
    toast_of(ToastKind::Info, text.into())
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
pub fn toast_alert(text: impl Into<String>) -> impl Bundle {
    toast_of(ToastKind::Alert, text.into())
}

/// A toast's nodes: a clipped column that holds the room (its height is opened and closed by
/// [`crate::apply_skin`], and starts closed, so a toast never takes room before it is styled) around
/// the body, a framed row of a mark and the message. Everything starts transparent and
/// transparent to the pointer.
fn toast_of(kind: ToastKind, text: String) -> impl Bundle {
    (
        Toast::new(kind, &text),
        Node {
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            max_height: Val::Px(0.0),
            ..default()
        },
        Role::Toast,
        Pickable::IGNORE,
        children![(
            Node {
                flex_direction: FlexDirection::Row,
                ..default()
            },
            Role::ToastBody,
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            Pickable::IGNORE,
            children![
                (
                    Node::default(),
                    Role::ToastMark,
                    BackgroundColor(Color::NONE),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(text),
                    TextColor(Color::NONE),
                    Role::ToastText,
                    Pickable::IGNORE,
                ),
            ],
        )],
    )
}
