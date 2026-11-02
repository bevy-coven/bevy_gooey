//! The status bar: the components of [`crate::status_bar`]'s items. What holds them to the bar's
//! width is [`crate::fit_bars`]; their look is [`crate::apply_skin`]'s.

use bevy::prelude::*;

/// A [`crate::status_bar`]'s root. Query for it to find the bar.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct StatusBar;

/// One side of a [`crate::status_bar`]: the left group (the workspace's state) grows to fill the
/// bar; the right group (the context's) is pushed to its end.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusGroup {
    pub(crate) left: bool,
}

/// How much a [`crate::status_item`] means, shown as a mark at its start: [`StatusKind::Info`]
/// draws none; a warning's is [`crate::GooeySkin::accent`] and an error's
/// [`crate::GooeySkin::danger`]. The words never change colour (4.5:1 against the bar in every
/// kind, and the mark is a shape, not only a hue). Change it to change the mark in the same frame.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StatusKind {
    /// Plain state.
    #[default]
    Info,
    /// Something to look at.
    Warning,
    /// Something is wrong.
    Error,
}

/// Which items of a narrow [`crate::status_bar`] stay: the **lowest** priority hides first, and an
/// item of the right group before any of the left's. Higher is kept longer; the default is 128.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((status_item("main"), StatusPriority(200)));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct StatusPriority(pub u8);

impl Default for StatusPriority {
    fn default() -> Self {
        Self(128)
    }
}
