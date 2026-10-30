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
//!   (a HUD bar driven by [`Meter`]), [`Tooltip`] and the modal [`dialog`]. Each is a free function
//!   returning a bundle, so they compose inside `children![]` and with [`observe`]; each carries a
//!   [`Role`] that [`apply_skin`] styles it by.
//! - Navigation — [`GooeyNavPlugin`]: arrows and the D-pad move focus to the nearest widget
//!   ([`navigate_by_input`]), the pad's South button activates it ([`gamepad_select`]), Tab cycles,
//!   and stays inside an open dialog. Enter and Space are Bevy's own button handling.
//! - Feedback — [`UiFeedback`] is written for every focus move and activation, for the host to
//!   turn into sound.
//!
//! [`model`] holds the engine-free maths under the skin: [`contrast_ratio`], [`composite`] and
//! [`meter_fraction`].
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
//! Sizes are logical pixels. `UiScale` is the host's, and multiplies all of them.

#![warn(missing_docs)]

pub mod model;

mod feedback;
mod meter;
mod nav;
mod skin;
mod tooltip;
mod widgets;

pub use bevy::ui_widgets::{Activate, observe};
pub use feedback::{UiFeedback, UiFeedbackKind, feedback_on_focus};
pub use meter::{Meter, update_meters};
pub use model::{
    COMFORT_TARGET_PX, MIN_TARGET_PX, NON_TEXT_CONTRAST, TEXT_CONTRAST, composite, contrast_ratio,
    meter_fraction, relative_luminance,
};
pub use nav::{Dialog, focus_opened_dialogs, gamepad_select, navigate_by_input};
pub use skin::{GooeyFonts, GooeySkin, Role, SkinProblem, apply_skin};
pub use tooltip::Tooltip;
pub use widgets::{button, dialog, heading, label, menu, meter, panel, screen};

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
    /// The skin and the meters: every widget's look.
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
            .init_resource::<InputFocus>()
            .init_resource::<InputFocusVisible>()
            .add_message::<UiFeedback>()
            .register_type::<GooeySkin>()
            .register_type::<Role>()
            .register_type::<Meter>()
            .register_type::<Tooltip>()
            .register_type::<UiFeedbackKind>()
            .add_observer(feedback::feedback_on_activate)
            .add_systems(
                Update,
                (apply_skin, update_meters).in_set(GooeySystems::Style),
            );
    }
}

/// Keyboard and gamepad navigation: arrows and the D-pad move focus, the pad's South button
/// activates, Tab cycles, and focus moves are written as [`UiFeedback`]. An open [`dialog`] holds
/// focus: it takes it on opening ([`focus_opened_dialogs`]), Tab and the arrows stay inside it, and
/// closing it gives focus back to the widget that opened it.
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
            .add_message::<UiFeedback>()
            .register_type::<Dialog>()
            .add_observer(nav::remember_opener)
            .add_observer(nav::return_focus)
            .add_systems(
                Update,
                (
                    focus_opened_dialogs,
                    navigate_by_input,
                    gamepad_select,
                    feedback_on_focus,
                )
                    .chain()
                    .in_set(GooeySystems::Navigate),
            );
    }
}

/// Every plugin in the crate: [`GooeyStylePlugin`] and [`GooeyNavPlugin`].
pub struct GooeyPlugins;

impl PluginGroup for GooeyPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(GooeyStylePlugin)
            .add(GooeyNavPlugin)
    }
}
