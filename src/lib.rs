//! Game interfaces made easy for Bevy: a skin, ready-made game widgets and keyboard and gamepad
//! navigation, on Bevy's own headless widgets (`bevy_ui_widgets`) and input focus
//! (`bevy_input_focus`).
//!
//! A menu is a few nested calls, every widget looks like one family, and a pad or a keyboard reaches
//! everything a mouse does:
//!
//! - [`GooeySkin`] — the look, as plain data: colours, sizes and a player's [`GooeySkin::text_scale`].
//!   [`GooeyStylePlugin`] restyles everything on screen the frame it changes, and
//!   [`GooeySkin::problems`] checks it against the WCAG 2.2 contrast and target-size rules. Three
//!   presets, [`GooeySkin::dark`], [`GooeySkin::parchment`] and [`GooeySkin::high_contrast`] (every
//!   pair at 7:1, checked by [`GooeySkin::high_contrast_problems`]). Fonts are the host's, in
//!   [`GooeyFonts`].
//! - The widgets — [`screen`], [`panel`], [`menu`], [`heading`], [`label`], [`button`],
//!   [`hold_button`] (a button confirmed only by holding it: [`HoldConfirmed`], with the player's
//!   [`HoldSettings`] to shorten the hold or make it a tap), [`meter`]
//!   (a HUD bar driven by [`Meter`]), [`toggle`], [`slider`] and [`choice`] (an options screen's
//!   switch, scrubber and stepper, reporting through [`ValueChange`]), [`binding`] (a controls
//!   screen's row: the player's next key, mouse button or pad button is the new [`Binding`]),
//!   [`text_input`] (a row with a field the player types in, kept in [`TextInputValue`]; the
//!   field never traps focus, and [`TextEditing`] and [`is_text_editing`] tell a game when it
//!   has the player's keys; [`text_area`] is the same several lines high), [`viewport`] and
//!   [`FocusRegion`] (a tool's parts F6 walks between, one keeping the arrow keys for the host),
//!   [`DragPayload`] and [`DropTarget`] (typed drag and drop: [`Dropped`], [`DroppedOnWorld`]),
//!   [`slot_grid`] (an inventory of [`Slot`]s kept in [`SlotItems`], moved by that drag and drop:
//!   [`SlotMoved`]),
//!   [`number_field`] and [`vec_field`] (a number the player scrubs with a
//!   drag on its label, types, or steps with Up, Down and the D-pad, held to a [`NumberRange`];
//!   kept in [`NumberValue`], with [`MixedValue`] for a multi-selection that differs),
//!   [`text_size`], [`interface_size`], [`pad_style`] and [`skin_preset`]
//!   (the player's own text size, interface size, controller and theme: a line each, editing
//!   [`GooeySkin::text_scale`], [`UiScaleSettings::scale`],
//!   [`BindingSettings::pad_style`] and the skin's colours and following them),
//!   [`tabs`] (its pages, each a [`tab_page`], turned by a click, the arrows and the pad's
//!   bumpers), [`list`] (rows that scroll, following focus), [`split`] (two panes and a divider the
//!   player drags or steps with the arrows, kept in [`SplitFraction`] and never below
//!   [`SplitMin`]), [`dock`] (tab groups and splits of the panels in a [`PanelRegistry`], kept as
//!   a [`DockLayout`] and rearranged by dragging a tab), [`prompts`] and [`prompt`] (the "Esc
//!   Back" hints in the corner, in the words of the device the player used last, which
//!   [`InputDevice`] tracks), [`toast`] and
//!   [`toast_alert`] (a line of news spawned from any system, stood in one [`ToastStack`] and
//!   timed by [`ToastSettings`]),
//!   [`Tooltip`] (with a command's [`Shortcut`] on a keycap: "Ctrl+S", or "⌘S" on a mac — and
//!   [`ChordGlyphs`] spells it "Cmd+S" for a host whose font has no symbols), the
//!   modal [`dialog`] and the anchored [`popover`] ([`popover_modal`] holds
//!   focus; [`Placement`] says where it opens, [`PopoverClosed`] why it closed), and the
//!   [`menu_bar`] of [`menu_title`]s with [`menu_item`], [`check_item`], [`radio_item`],
//!   [`submenu`] and [`separator`] rows (chords right-aligned, [`Disabled`] items that say why), or a
//!   [`context_menu`] that any entity opens with [`ContextMenu`] on a right-click or Shift+F10. Each is a free function returning a bundle, so they
//!   compose inside `children![]` and with [`observe`]; each carries a [`Role`] that
//!   [`apply_skin`] styles it by. A [`screen`] is layout: its empty part lets the pointer through
//!   to the game under a HUD, while every widget on it takes the pointer. A [`WorldAnchor`] on a
//!   root node makes it follow a point in the world (a health bar, a prompt, a beacon pinned to the
//!   screen's edge). A toast takes no input and no focus.
//! - Navigation — [`GooeyNavPlugin`]: arrows and the D-pad move focus to the nearest widget
//!   ([`navigate_by_input`]), the pad's South button activates it ([`gamepad_select`]), Tab cycles.
//!   Keys and the pad show the focus ring; a pointer press hides it. With nothing focused (a click
//!   on empty space, a despawned widget), the first arrow or D-pad press lands where the screen
//!   starts. An open [`dialog`] holds focus: it takes it, keeps arrows, D-pad and Tab inside, and
//!   gives it back to its opener on closing. A [`Tooltip`] opens when its widget has visible focus,
//!   as it does under the pointer. Enter and Space are Bevy's own button handling. A focused
//!   [`slider`] or [`choice`] keeps Left and Right (the D-pad steps it, held to repeat:
//!   [`gamepad_adjust`]); Up and Down still move on. `LB` and `RB` turn the page of a [`tabs`]
//!   widget from anywhere in it ([`step_tabs`]), and focus never rests on a page that is not shown
//!   ([`land_tab_focus`]). A [`list`] scrolls to keep the focused row and
//!   the next one in sight ([`scroll_to_focus`]), and PageUp and PageDown ([`page_lists`]) move a
//!   viewport at a time. Escape and a pad's B are one [`Cancel`] event, sent to the focused widget
//!   (or the open dialog) and bubbling up to the layer the game observes ([`cancel_by_input`]),
//!   except while a [`binding`] row waits for a press: then nothing else hears the player
//!   ([`not_capturing`], [`bind_by_input`]).
//! - Scale — [`GooeyScalePlugin`]: the whole interface keeps its share of the screen at every
//!   resolution, so text legible at 720p is legible on a 1440p monitor and a 4K television
//!   ([`UiScaleSettings`], [`scale_with_window`]).
//! - Feedback — [`UiFeedback`] is written for every focus move onto a widget and every activation
//!   or flip, for the host to turn into sound.
//!
//! [`model`] holds the engine-free maths under the skin: [`contrast_ratio`], [`composite`],
//! [`meter_fraction`], [`slider_fraction`], [`scroll_to_reveal`], [`tab_step`],
//! [`window_ui_scale`] and [`toast_hold`].
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_gooey::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((DefaultPlugins, GooeyPlugins))
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Camera2d);
//!     commands.spawn((
//!         screen(),
//!         children![(
//!             panel(),
//!             children![
//!                 heading("Paused"),
//!                 (
//!                     menu(),
//!                     children![
//!                         (button("Resume"), observe(|_: On<Activate>| info!("resume"))),
//!                         button("Quit"),
//!                     ]
//!                 ),
//!             ]
//!         )],
//!     ));
//! }
//! ```
//!
//! Every plugin is independent; adding some and not others is supported. None of them adds
//! `UiWidgetsPlugins`, `InputFocusPlugin` or `InputDispatchPlugin`: `DefaultPlugins` adds those,
//! and adding one twice panics. A host without `DefaultPlugins` adds them itself.
//!
//! # What this crate does not own
//!
//! World-space speech balloons and dialogue belong to `bevy_speech_bubbles`; the game's states and
//! pausing, to the game. Audio is the host's: it hears [`UiFeedback`]. Localization is the host's:
//! every widget takes the string to show. Fonts are the host's: it fills [`GooeyFonts`].
//!
//! # Units
//!
//! Sizes are logical pixels at a window whose shorter side is
//! [`UiScaleSettings::reference_short_side`] (720). [`GooeyScalePlugin`] sets Bevy's `UiScale` from
//! the window, so a 1920x1080 window draws them at 1.5x; a host that drives `UiScale` itself leaves
//! that plugin out (`GooeyPlugins.build().disable::<GooeyScalePlugin>()`). Line weights (borders,
//! the focus ring, a meter's frame and gap) are the exception to "multiplied": they are drawn a
//! whole number of device pixels wide at whatever scale the node has, rounded down
//! ([`snap_line_width`]), so each is one weight on all four sides.

#![warn(missing_docs)]

pub mod model;
#[cfg(feature = "testing")]
pub mod testing;

mod access;
mod access_actions;
mod anchor;
mod badges;
mod binding;
mod cancel;
mod choice;
mod color;
mod confirm;
mod context;
mod controls;
mod cooldown;
mod dock;
mod dock_layout;
mod drag;
mod feedback;
mod field;
mod hold;
mod icon;
#[cfg(feature = "reflect")]
mod inspector;
mod list;
mod menu;
mod meter;
mod motion;
mod nav;
mod notification;
mod number;
mod options;
mod palette;
mod popover;
mod progress;
mod prompt;
mod radial;
mod region;
mod scale;
mod section;
mod select;
mod shortcut;
mod skin;
mod slots;
mod split;
mod status;
mod table;
mod tabs;
mod text;
mod toast;
mod toolbar;
mod tooltip;
mod tree;
mod widgets;

pub use anchor::{AnchorPinned, OffScreen, WorldAnchor, WorldAnchors, update_world_anchors};
pub use badges::{
    BadgesShown, LegendRow, Reveal, ShortcutBadge, ShortcutBadges, ShortcutLegend,
    ShortcutLegendCard, update_shortcut_badges, update_shortcut_legend,
};
pub use bevy::ui::Checked;
pub use bevy::ui_widgets::{Activate, SliderRange, SliderStep, SliderValue, ValueChange, observe};
pub use binding::{
    Binding, BindingSettings, BindingValue, Capturing, bind_by_input, catch_pointer, not_capturing,
    update_bindings,
};
pub use cancel::{Cancel, CancelScope, cancel_by_input};
pub use choice::{ChoiceCount, ChoiceValue, update_choices};
pub use color::{
    AreaMarker, ColorArea, ColorPicker, ColorRail, ColorRefused, ColorStrip, ColorSwatch,
    ColorValue, SwatchHalf, update_colors,
};
pub use confirm::{Confirm, ConfirmButton, ConfirmChosen, confirm};
pub use context::{ContextMenu, ContextTarget, context_by_key};
pub use controls::update_sliders;
pub use cooldown::{Cooldown, CooldownReady, update_cooldowns};
pub use dock::{Dock, DockLayoutChanged, DockPanel, DockPanelSkipped, PanelRegistry};
pub use dock_layout::{DockDropTarget, DockLayout, DockSkip};
pub use drag::{
    DRAG_LIFT_PX, DragLabel, DragPayload, DragSettings, DragState, Dragging, DropCandidate,
    DropRefused, DropRefusing, DropTarget, Dropped, DroppedOnWorld, cancel_drags, is_dragging,
};
pub use feedback::{UiFeedback, UiFeedbackKind, feedback_on_focus};
pub use field::{FieldProblem, FieldProblemKind, update_field_problems};
pub use hold::{
    HoldConfirmed, HoldProgress, HoldSettings, HoldToConfirm, hold_by_input, update_hold_bars,
};
pub use icon::{Icon, IconMissing, IconSize, IconSource, IconTone, update_icons};
#[cfg(feature = "reflect")]
pub use inspector::{
    InspectRow, InspectStep, InspectTarget, Inspector, InspectorChange, build_inspectors,
    inspector, sync_inspectors,
};
pub use list::{ListView, page_lists, scroll_to_focus, update_lists};
pub use menu::{
    Disabled, MenuBar, MenuItem, MenuMarked, MenuOpen, MenuPanel, MenuTitle, fit_menus,
    focus_menu_bar, menu_by_input, scroll_menus_to_focus, switch_menus_on_hover,
};
pub use meter::{Meter, update_meters};
pub use model::{
    COMFORT_TARGET_PX, COOLDOWN_WHOLE_SECONDS_FROM, ChordGlyphs, DIALOG_SLIDE_PX,
    DIALOG_SLIDE_SECONDS, DOCK_EDGE_FRACTION, DOCK_EDGE_SHARE, DOCK_PANE_MIN_PX, DockZone,
    ELEVATION_STEP, ENHANCED_CONTRAST, HOLD_SCALE_MAX, HOLD_UNWIND_RATE, HexError,
    INTERFACE_SIZE_STEPS, METER_GAIN_TAU_SECONDS, METER_SNAP, METER_TRAIL_HOLD_SECONDS,
    METER_TRAIL_RATE, MIN_TARGET_PX, MeterShown, NON_TEXT_CONTRAST, NumberError, PAD_REPEAT_DELAY,
    PAD_REPEAT_RATE, PAD_STYLES, POPOVER_SLIDE_PX, POPOVER_SLIDE_SECONDS, PadStyle,
    RADIAL_DEAD_ZONE, RADIAL_STICK_PUSH, SPINNER_PERIOD_SECS, SPLIT_KEY_STEP_PX, STATE_STEP,
    STICK_PUSH, TABLE_LEAST_COLUMN, TEXT_CONTRAST, TEXT_SIZE_STEPS, TYPE_AHEAD_SECS, TableStep,
    Toward, area_values, bar_fit, box_outside, choice_next, choice_step, cie_lightness, clock_text,
    column_resize, composite, contrast_ratio, cooldown_sweep, cooldown_text, cooldown_words,
    dock_drop_rect, dock_edge_zone, dock_neighbour, dock_tab_slot, dock_zone, ease_out, eta_words,
    format_number, fuzzy_score, hex_text, hold_seconds, hold_step, hsv_to_rgb, is_modifier_key,
    key_name, list_height, meter_fraction, meter_motion, mouse_name, natural_cmp, nearest_step,
    number_decimals, pad_name, page_target, parse_hex, parse_number, pin_to_edge, progress_text,
    radial_pick, radial_place, radial_step, relative_luminance, repeat_steps, rgb_to_hsv,
    scroll_to_reveal, scrub_value, shortcut_conflicts, shortcut_label, slider_fraction,
    slider_text, slot_edge, snap_line_width, snap_to_row, spinner_angle, split_clamp, split_drag,
    split_mins, step_value, tab_step, table_anchor, table_step, table_window, target_step,
    target_toward, toast_alpha, toast_grow, toast_hold, toast_life, tree_ends, tree_heir,
    tree_indent, tree_parent, tree_visible, type_ahead, window_ui_scale,
};
pub use motion::{MotionSettings, PopoverSlide, SlideIn, begin_entrances, update_motion};
pub use nav::{
    Dialog, FocusScopes, focus_opened_dialogs, gamepad_adjust, gamepad_select, navigate_by_input,
};
pub use notification::{NotificationCenter, NotificationRow, update_notification_centers};
pub use number::{
    MIXED_HINT, MixedValue, NumberInput, NumberRange, NumberRefused, NumberStep, NumberValue,
    VecChange, VecMixed, VecPart, VecValue, update_numbers, update_vecs,
};
pub use options::{OptionRow, update_option_rows};
pub use palette::{
    CommandList, CommandPalette, PALETTE_ROWS, PaletteCommand, PaletteCount, PaletteHistory,
    PaletteInput, PaletteNote, PaletteRow, RunCommand, palette_by_input, update_palette,
};
pub use popover::{
    CloseReason, Placement, PopoverClosed, PopoverPanel, PopoverPoint, anchor_pointer_popovers,
    close_popovers_on_press, drop_pointer_anchors, reveal_popovers,
};
pub use progress::{
    Progress, ProgressClamped, ProgressText, Spinner, update_progress, update_spinners,
};
pub use prompt::{
    InputDevice, Prompt, PromptsAnchor, centre_keycaps, place_prompts, track_input_device,
    update_prompts,
};
pub use radial::{RadialCount, RadialItem, RadialSlot, RadialValue, radial_by_pad, update_radials};
pub use region::{ArrowPolicy, FocusRegion, cycle_regions};
pub use scale::{UiScaleSettings, scale_with_window};
pub use section::{SectionHeader, SectionId, SectionOpen, SectionStates, update_sections};
pub use select::{SELECT_FILTER_ABOVE, Select};
pub use shortcut::{ChordText, Modifiers, Platform, Shortcut, update_chord_texts};
pub use skin::{
    Density, GooeyFonts, GooeySkin, Role, SkinPreset, SkinProblem, SkinScope, apply_skin,
};
pub use slots::{
    Slot, SlotCursor, SlotGrid, SlotItems, SlotMoved, SlotRef, SlotRefused, SlotRules,
    update_slot_grids,
};
pub use split::{
    Split, SplitAxis, SplitDivider, SplitFraction, SplitGeometry, SplitMin, split_by_pad,
    update_splits,
};
pub use status::{StatusBar, StatusItem, StatusKind, StatusPriority};
pub use table::{
    Column, RowActivated, RowSource, SelectMode, SortBy, Table, TableCursor, TableRow, TableRows,
    TableSelection, TableSort, TableWidths, table_by_input, update_tables,
};
pub use tabs::{TabsCount, TabsValue, land_tab_focus, step_tabs, update_tabs};
pub use text::{
    TextEdited, TextEditing, TextHint, TextInputOf, TextInputValue, TextLimit, is_text_editing,
    track_text_editing, update_text_inputs,
};
pub use toast::{
    Toast, ToastAction, ToastAnchor, ToastBundle, ToastButton, ToastKind, ToastLog, ToastLogEntry,
    ToastSettings, ToastStack, update_toasts,
};
pub use toolbar::{
    GroupOption, Overflowed, ToggleGroupCount, ToggleGroupValue, Toolbar, fit_bars,
    update_toggle_groups,
};
pub use tooltip::{Tooltip, TooltipBox, show_tooltips, update_tooltip_text};
pub use tree::{Tree, TreeNode, TreeToggled, tree_by_input, update_trees};
pub use widgets::{
    binding, button, check_item, choice, color_field, command_palette, context_menu, dialog, dock,
    heading, hold_button, icon, icon_button, interface_size, label, list, menu, menu_bar,
    menu_item, menu_title, meter, notification_center, number_field, pad_style, panel, popover,
    popover_modal, progress, prompt, prompts, prompts_at, radial, radio_item, screen, section,
    select, separator, skin_preset, slider, slot_grid, spinner, split, status_bar, status_item,
    status_progress, submenu, surface, tab_page, table, tabs, text_area, text_input, text_size,
    toast, toast_alert, toggle, toggle_group, toolbar, toolbar_separator, tree, vec_field,
    viewport,
};

use bevy::app::PluginGroupBuilder;
use bevy::input_focus::directional_navigation::DirectionalNavigationPlugin;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::ui::UiSystems;

/// The order the crate's systems run in `Update`.
///
/// [`GooeySystems::Navigate`] moves focus and activates before [`GooeySystems::Style`] draws, so a
/// focus move is drawn in the very frame it happens.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GooeySystems {
    /// Arrow-key, D-pad and pad-button navigation, and the focus feedback.
    Navigate,
    /// The skin, the meters and the interface's scale: every widget's look.
    Style,
}

/// Chains the crate's sets, and does it idempotently, so each plugin may call it.
fn chain_sets(app: &mut App) {
    app.configure_sets(
        Update,
        (GooeySystems::Navigate, GooeySystems::Style).chain(),
    );
}

/// Styles every widget from [`GooeySkin`] and [`GooeyFonts`], moves [`Meter`] fills, and writes
/// [`UiFeedback`] for activations.
///
/// Inserts the default skin and fonts unless the host already has them.
pub struct GooeyStylePlugin;

impl Plugin for GooeyStylePlugin {
    fn build(&self, app: &mut App) {
        chain_sets(app);
        app.init_resource::<GooeySkin>()
            .init_resource::<InputDevice>()
            .init_resource::<GooeyFonts>()
            .init_resource::<PanelRegistry>()
            .init_resource::<ToastSettings>()
            .init_resource::<MotionSettings>()
            .init_resource::<ShortcutBadges>()
            .init_resource::<BadgesShown>()
            .init_resource::<ShortcutLegend>()
            .init_resource::<ToastLog>()
            .init_resource::<CommandList>()
            .init_resource::<PaletteHistory>()
            .init_resource::<BindingSettings>()
            .init_resource::<SectionStates>()
            .init_resource::<InputFocus>()
            .init_resource::<InputFocusVisible>()
            .add_message::<UiFeedback>()
            .register_type::<GooeySkin>()
            .register_type::<SkinScope>()
            .register_type::<Role>()
            .register_type::<Meter>()
            .register_type::<WorldAnchor>()
            .register_type::<AnchorPinned>()
            .register_type::<OffScreen>()
            .register_type::<Icon>()
            .register_type::<IconTone>()
            .register_type::<IconSize>()
            .register_type::<IconMissing>()
            .register_type::<FieldProblem>()
            .register_type::<Progress>()
            .register_type::<ProgressText>()
            .register_type::<ProgressClamped>()
            .register_type::<Spinner>()
            .register_type::<Toolbar>()
            .register_type::<ToggleGroupValue>()
            .register_type::<ToggleGroupCount>()
            .register_type::<SectionOpen>()
            .register_type::<SectionId>()
            .register_type::<StatusBar>()
            .register_type::<StatusItem>()
            .register_type::<StatusKind>()
            .register_type::<StatusPriority>()
            .register_type::<Binding>()
            .register_type::<Shortcut>()
            .register_type::<ShortcutBadges>()
            .register_type::<Reveal>()
            .register_type::<BadgesShown>()
            .register_type::<ShortcutLegend>()
            .register_type::<LegendRow>()
            .register_type::<Modifiers>()
            .register_type::<ChordText>()
            .register_type::<Platform>()
            .register_type::<InputDevice>()
            .register_type::<Prompt>()
            .register_type::<BindingSettings>()
            .register_type::<Capturing>()
            .register_type::<TextInputValue>()
            .register_type::<TextHint>()
            .register_type::<TextLimit>()
            .register_type::<TextEditing>()
            .register_type::<NumberValue>()
            .register_type::<NumberStep>()
            .register_type::<NumberRange>()
            .register_type::<MixedValue>()
            .register_type::<ColorValue>()
            .register_type::<VecValue>()
            .register_type::<VecMixed>()
            .register_type::<ChoiceValue>()
            .register_type::<TabsValue>()
            .register_type::<TabsCount>()
            .register_type::<ChoiceCount>()
            .register_type::<RadialValue>()
            .register_type::<RadialCount>()
            .register_type::<RadialSlot>()
            .register_type::<Slot>()
            .register_type::<SlotItems>()
            .register_type::<SlotGrid>()
            .register_type::<SlotCursor>()
            .register_type::<SlotRef>()
            .register_type::<ListView>()
            .register_type::<Split>()
            .register_type::<SplitAxis>()
            .register_type::<SplitFraction>()
            .register_type::<SplitMin>()
            .register_type::<TableSort>()
            .register_type::<SelectMode>()
            .register_type::<TableRows>()
            .register_type::<SplitDivider>()
            .register_type::<Tooltip>()
            .register_type::<Placement>()
            .register_type::<PopoverPanel>()
            .register_type::<MenuBar>()
            .register_type::<MenuTitle>()
            .register_type::<MenuPanel>()
            .register_type::<MenuOpen>()
            .register_type::<ContextMenu>()
            .register_type::<ContextTarget>()
            .register_type::<CloseReason>()
            .register_type::<PopoverClosed>()
            .register_type::<Cooldown>()
            .register_type::<CooldownReady>()
            .register_type::<Toast>()
            .register_type::<ToastStack>()
            .register_type::<ToastSettings>()
            .register_type::<MotionSettings>()
            .register_type::<HoldToConfirm>()
            .register_type::<HoldProgress>()
            .register_type::<HoldConfirmed>()
            .register_type::<HoldSettings>()
            .register_type::<SlideIn>()
            .register_type::<ToastAnchor>()
            .register_type::<ToastKind>()
            .register_type::<ToastLog>()
            .register_type::<ToastLogEntry>()
            .register_type::<PaletteHistory>()
            .register_type::<UiFeedbackKind>()
            .add_observer(feedback::feedback_on_activate)
            .add_observer(hold::feedback_on_hold)
            .add_observer(feedback::feedback_on_toggle)
            .add_observer(feedback::feedback_on_choice)
            .add_observer(toolbar::more_on_activate)
            .add_observer(toast::toast_on_activate)
            .add_observer(toast::toast_button_removed)
            .add_observer(text::text_on_edit)
            .add_observer(dock::dock_on_select)
            .add_observer(dock::dock_on_resize)
            .add_systems(
                Update,
                (
                    (
                        (begin_entrances, update_motion).chain(),
                        update_toasts,
                        update_notification_centers,
                        update_slot_grids,
                        update_icons,
                        update_field_problems,
                        update_progress,
                        update_spinners,
                        update_cooldowns,
                        update_hold_bars,
                        (update_toggle_groups, fit_bars).chain(),
                        update_sections,
                        (
                            update_shortcut_badges,
                            update_shortcut_legend,
                            update_chord_texts,
                        )
                            .chain(),
                        (drag::update_drag_visuals, drag::update_drop_candidates).chain(),
                    )
                        .chain(),
                    update_tooltip_text,
                    show_tooltips,
                    update_bindings,
                    (update_prompts, place_prompts).chain(),
                    (update_vecs, update_numbers, update_colors).chain(),
                    update_text_inputs,
                    update_palette,
                    update_option_rows,
                    (update_choices, update_radials).chain(),
                    dock::update_docks,
                    dock::update_dock_drag,
                    update_tabs,
                    update_splits,
                    update_trees,
                    update_tables,
                    apply_skin,
                    update_meters,
                    update_sliders,
                    (update_lists, access::update_accessibility).chain(),
                )
                    .chain()
                    .in_set(GooeySystems::Style),
            );
        // The one system that is not in `Update`: it needs this frame's transforms.
        anchor::add_systems(app);
        // So the keycap's centring: it needs this frame's shaped text.
        app.add_systems(
            PostUpdate,
            centre_keycaps
                .in_set(UiSystems::PostLayout)
                .after(bevy::ui::widget::text_system),
        );
        #[cfg(feature = "reflect")]
        app.add_systems(
            Update,
            (build_inspectors, sync_inspectors)
                .chain()
                .in_set(GooeySystems::Style)
                .before(update_toasts),
        );
    }
}

/// Keyboard and gamepad navigation: arrows and the D-pad move focus, the pad's South button
/// activates, Tab cycles, and focus moves are written as [`UiFeedback`]. Keys and the pad show
/// focus (Bevy's `InputFocusVisible`; a pointer press hides it), and a direction press with nothing
/// focused lands where the screen starts. An open [`dialog`] holds focus: it takes it on opening
/// ([`focus_opened_dialogs`]), Tab and the arrows stay inside it, and closing it gives focus back
/// to the widget that opened it. Escape and the pad's East button (B) send a [`Cancel`] to the
/// focused widget or the open dialog ([`cancel_by_input`]), for the game to observe.
///
/// Adds Bevy's `TabNavigationPlugin` and `DirectionalNavigationPlugin` unless the host already has.
pub struct GooeyNavPlugin;

impl Plugin for GooeyNavPlugin {
    fn build(&self, app: &mut App) {
        chain_sets(app);
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<DirectionalNavigationPlugin>() {
            app.add_plugins(DirectionalNavigationPlugin);
        }
        app.init_resource::<InputFocus>()
            .init_resource::<HoldSettings>()
            .init_resource::<InputDevice>()
            .init_resource::<BindingSettings>()
            .init_resource::<CommandList>()
            .init_resource::<PaletteHistory>()
            .init_resource::<InputFocusVisible>()
            .add_message::<UiFeedback>()
            .add_message::<bevy::a11y::ActionRequest>()
            .register_type::<Dialog>()
            .register_type::<FocusRegion>()
            .register_type::<DragLabel>()
            .register_type::<Dragging>()
            .register_type::<DropCandidate>()
            .register_type::<DropRefusing>()
            .init_resource::<DragState>()
            .init_resource::<DragSettings>()
            .register_type::<DragSettings>()
            .add_observer(drag::drag_on_start)
            .add_observer(drag::drag_on_drag)
            .add_observer(drag::drag_on_end)
            .add_observer(drag::drag_on_pointer_cancel)
            .register_type::<ArrowPolicy>()
            .register_type::<Cancel>()
            .register_type::<Confirm>()
            .register_type::<ConfirmButton>()
            .register_type::<ConfirmChosen>()
            .add_observer(confirm::confirm_on_activate)
            .add_observer(confirm::confirm_on_cancel)
            .add_observer(menu::menu_on_activate)
            .add_observer(context::context_on_click)
            .add_observer(nav::remember_opener)
            .add_observer(nav::return_focus)
            .add_systems(
                Update,
                (
                    track_input_device,
                    track_text_editing,
                    close_popovers_on_press,
                    anchor_pointer_popovers,
                    drop_pointer_anchors,
                    reveal_popovers,
                    focus_opened_dialogs,
                    (land_tab_focus, dock::land_dock_focus).chain(),
                    (
                        step_tabs.run_if(not_capturing),
                        cycle_regions.run_if(not_capturing),
                    )
                        .chain(),
                    (
                        focus_menu_bar.run_if(not_capturing),
                        menu_by_input.run_if(not_capturing),
                        context_by_key.run_if(not_capturing),
                        switch_menus_on_hover,
                        fit_menus,
                    )
                        .chain(),
                    (
                        navigate_by_input.run_if(not_capturing),
                        table_by_input.run_if(not_capturing),
                        tree_by_input.run_if(not_capturing),
                    )
                        .chain(),
                    (
                        gamepad_adjust.run_if(not_capturing),
                        radial_by_pad.run_if(not_capturing),
                        split_by_pad.run_if(not_capturing),
                    )
                        .chain(),
                    (
                        gamepad_select.run_if(not_capturing),
                        hold_by_input.run_if(not_capturing),
                        access_actions::act_on_requests,
                    )
                        .chain(),
                    (
                        drag::carry_by_input.run_if(binding::no_row_waits),
                        cancel_drags.run_if(binding::no_row_waits),
                        cancel_by_input
                            .run_if(not_capturing)
                            .run_if(not(is_dragging)),
                    )
                        .chain(),
                    (page_lists.run_if(not_capturing), palette_by_input).chain(),
                    bind_by_input,
                    catch_pointer,
                    (drag::track_drag, scroll_to_focus).chain(),
                    scroll_menus_to_focus,
                    feedback_on_focus,
                )
                    .chain()
                    .in_set(GooeySystems::Navigate),
            );
    }
}

/// Draws the whole interface in proportion to the window: sets Bevy's `UiScale` from the primary
/// window's shorter side and [`UiScaleSettings`] ([`scale_with_window`]), so a 1440p window draws
/// everything twice the size a 720p one does.
///
/// It owns `UiScale`: a player's interface size goes in [`UiScaleSettings::scale`]. A host that sets
/// `UiScale` itself leaves this plugin out.
pub struct GooeyScalePlugin;

impl Plugin for GooeyScalePlugin {
    fn build(&self, app: &mut App) {
        chain_sets(app);
        app.init_resource::<UiScaleSettings>()
            .init_resource::<UiScale>()
            .register_type::<UiScaleSettings>()
            .add_systems(Update, scale_with_window.in_set(GooeySystems::Style));
    }
}

/// Every plugin in the crate: [`GooeyStylePlugin`], [`GooeyNavPlugin`] and [`GooeyScalePlugin`].
pub struct GooeyPlugins;

impl PluginGroup for GooeyPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(GooeyStylePlugin)
            .add(GooeyNavPlugin)
            .add(GooeyScalePlugin)
    }
}
