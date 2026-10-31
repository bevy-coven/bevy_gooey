//! Game interfaces made easy for Bevy: a skin, ready-made game widgets and keyboard and gamepad
//! navigation, on Bevy's own headless widgets (`bevy_ui_widgets`) and input focus
//! (`bevy_input_focus`).
//!
//! A menu is a few nested calls, every widget looks like one family, and a pad or a keyboard reaches
//! everything a mouse does:
//!
//! - [`GooeySkin`] — the look, as plain data: colours, sizes and a player's [`GooeySkin::text_scale`].
//!   [`GooeyStylePlugin`] restyles everything on screen the frame it changes, and
//!   [`GooeySkin::problems`] checks it against the WCAG 2.2 contrast and target-size rules. Two
//!   presets, [`GooeySkin::dark`] and [`GooeySkin::parchment`]. Fonts are the host's, in
//!   [`GooeyFonts`].
//! - The widgets — [`screen`], [`panel`], [`menu`], [`heading`], [`label`], [`button`], [`meter`]
//!   (a HUD bar driven by [`Meter`]), [`toggle`] and [`slider`] (an options screen's switch and
//!   scrubber, reporting through [`ValueChange`]), [`toast`] and [`toast_alert`] (a line of news
//!   spawned from any system, stood in one [`ToastStack`] and timed by [`ToastSettings`]),
//!   [`Tooltip`] and the modal [`dialog`]. Each is a free function returning a bundle, so they
//!   compose inside `children![]` and with [`observe`]; each carries a [`Role`] that
//!   [`apply_skin`] styles it by. A [`screen`] is layout: its empty part lets the pointer through
//!   to the game under a HUD, while every widget on it takes the pointer. A toast takes no input
//!   and no focus.
//! - Navigation — [`GooeyNavPlugin`]: arrows and the D-pad move focus to the nearest widget
//!   ([`navigate_by_input`]), the pad's South button activates it ([`gamepad_select`]), Tab cycles.
//!   Keys and the pad show the focus ring; a pointer press hides it. With nothing focused (a click
//!   on empty space, a despawned widget), the first arrow or D-pad press lands where the screen
//!   starts. An open [`dialog`] holds focus: it takes it, keeps arrows, D-pad and Tab inside, and
//!   gives it back to its opener on closing. A [`Tooltip`] opens when its widget has visible focus,
//!   as it does under the pointer. Enter and Space are Bevy's own button handling. A focused
//!   [`slider`] keeps Left and Right (the D-pad steps it, held to repeat: [`gamepad_adjust`]); Up
//!   and Down still move on.
//! - Scale — [`GooeyScalePlugin`]: the whole interface keeps its share of the screen at every
//!   resolution, so text legible at 720p is legible on a 1440p monitor and a 4K television
//!   ([`UiScaleSettings`], [`scale_with_window`]).
//! - Feedback — [`UiFeedback`] is written for every focus move onto a widget and every activation
//!   or flip, for the host to turn into sound.
//!
//! [`model`] holds the engine-free maths under the skin: [`contrast_ratio`], [`composite`],
//! [`meter_fraction`], [`slider_fraction`], [`window_ui_scale`] and [`toast_hold`].
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

mod controls;
mod feedback;
mod meter;
mod nav;
mod scale;
mod skin;
mod toast;
mod tooltip;
mod widgets;

pub use bevy::ui::Checked;
pub use bevy::ui_widgets::{Activate, SliderRange, SliderStep, SliderValue, ValueChange, observe};
pub use controls::update_sliders;
pub use feedback::{UiFeedback, UiFeedbackKind, feedback_on_focus};
pub use meter::{Meter, update_meters};
pub use model::{
    COMFORT_TARGET_PX, MIN_TARGET_PX, NON_TEXT_CONTRAST, PAD_REPEAT_DELAY, PAD_REPEAT_RATE,
    TEXT_CONTRAST, composite, contrast_ratio, meter_fraction, relative_luminance, repeat_steps,
    slider_fraction, slider_text, snap_line_width, toast_alpha, toast_hold, toast_life, toast_lift,
    window_ui_scale,
};
pub use nav::{
    Dialog, FocusScopes, focus_opened_dialogs, gamepad_adjust, gamepad_select, navigate_by_input,
};
pub use scale::{UiScaleSettings, scale_with_window};
pub use skin::{GooeyFonts, GooeySkin, Role, SkinProblem, apply_skin};
pub use toast::{Toast, ToastAnchor, ToastKind, ToastSettings, ToastStack, update_toasts};
pub use tooltip::{Tooltip, TooltipBox, show_tooltips, update_tooltip_text};
pub use widgets::{
    button, dialog, heading, label, menu, meter, panel, screen, slider, toast, toast_alert, toggle,
};

use bevy::app::PluginGroupBuilder;
use bevy::input_focus::directional_navigation::DirectionalNavigationPlugin;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;

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
            .init_resource::<GooeyFonts>()
            .init_resource::<ToastSettings>()
            .init_resource::<InputFocus>()
            .init_resource::<InputFocusVisible>()
            .add_message::<UiFeedback>()
            .register_type::<GooeySkin>()
            .register_type::<Role>()
            .register_type::<Meter>()
            .register_type::<Tooltip>()
            .register_type::<Toast>()
            .register_type::<ToastStack>()
            .register_type::<ToastSettings>()
            .register_type::<ToastAnchor>()
            .register_type::<ToastKind>()
            .register_type::<UiFeedbackKind>()
            .add_observer(feedback::feedback_on_activate)
            .add_observer(feedback::feedback_on_toggle)
            .add_systems(
                Update,
                (
                    update_toasts,
                    update_tooltip_text,
                    show_tooltips,
                    apply_skin,
                    update_meters,
                    update_sliders,
                )
                    .chain()
                    .in_set(GooeySystems::Style),
            );
    }
}

/// Keyboard and gamepad navigation: arrows and the D-pad move focus, the pad's South button
/// activates, Tab cycles, and focus moves are written as [`UiFeedback`]. Keys and the pad show
/// focus (Bevy's `InputFocusVisible`; a pointer press hides it), and a direction press with nothing
/// focused lands where the screen starts. An open [`dialog`] holds focus: it takes it on opening
/// ([`focus_opened_dialogs`]), Tab and the arrows stay inside it, and closing it gives focus back
/// to the widget that opened it.
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
            .init_resource::<InputFocusVisible>()
            .add_message::<UiFeedback>()
            .register_type::<Dialog>()
            .add_observer(nav::remember_opener)
            .add_observer(nav::return_focus)
            .add_systems(
                Update,
                (
                    focus_opened_dialogs,
                    navigate_by_input,
                    gamepad_adjust,
                    gamepad_select,
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
