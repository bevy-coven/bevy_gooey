//! Focus regions: the parts of a screen F6 cycles through, and the ones that keep the arrow keys for
//! the host. A tool's viewport owns the arrows (it pans, it orbits), yet its panels still need the
//! keyboard: a [`FocusRegion`] says which part is which.

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;

use crate::nav::{FocusScopes, show_focus};
use crate::tabs::StopQuery;

/// Who the arrow keys belong to while focus is in a [`FocusRegion`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default, Debug, PartialEq, Hash, Clone)]
pub enum ArrowPolicy {
    /// The crate's: the arrow keys move focus between widgets, as outside any region.
    #[default]
    Navigate,
    /// The host's: with focus on the region or inside it, the arrow keys move no focus
    /// ([`crate::navigate_by_input`] leaves them alone) and stay readable in
    /// `ButtonInput<KeyCode>`, so a viewport can pan on them. The D-pad still moves focus (a pad
    /// has no Tab), and so does Tab and F6.
    Host,
}

/// A part of the screen focus can be cycled to: put it on a panel, a viewport or a toolbar.
///
/// - **F6** moves focus to the first widget of the next region, **Shift+F6** to the previous one,
///   wrapping. Regions are visited in the order they stand in the UI tree (the order the game
///   spawned them), a region with nothing focusable in it is skipped, and a region that is itself a
///   tab stop (a viewport with a `TabIndex`) is its own first widget. An open
///   [`crate::dialog`] holds focus over every region: F6 cycles the dialog's own regions only.
/// - [`ArrowPolicy::Host`] leaves the arrow keys to the host while focus is in the region. Tab still
///   leaves it, so it never traps the keyboard (WCAG 2.1.2); tell the player the way out (F6) where
///   the viewport is.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn setup(mut commands: Commands) {
/// commands.spawn((
///     Node::default(),
///     FocusRegion::host(),
///     TabIndex(0), // the viewport is itself a tab stop: click or F6 puts focus on it
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct FocusRegion {
    /// Who the arrow keys belong to while focus is inside.
    pub arrows: ArrowPolicy,
}

impl FocusRegion {
    /// A region that keeps the arrow keys for the host ([`ArrowPolicy::Host`]).
    pub const fn host() -> Self {
        Self {
            arrows: ArrowPolicy::Host,
        }
    }

    /// A region that only takes part in F6 ([`ArrowPolicy::Navigate`]).
    pub const fn navigate() -> Self {
        Self {
            arrows: ArrowPolicy::Navigate,
        }
    }
}

/// F6 and Shift+F6 (and nothing held but Shift) move focus to the next or previous
/// [`FocusRegion`]'s first widget, and show focus (`InputFocusVisible`). Runs in
/// [`crate::GooeySystems::Navigate`] with the other navigation, gated by [`crate::not_capturing`].
pub fn cycle_regions(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    scopes: FocusScopes,
    stops: Query<StopQuery>,
) {
    if !keys.just_pressed(KeyCode::F6)
        || keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::AltLeft,
            KeyCode::AltRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ])
    {
        return;
    }
    let backwards = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if let Some(to) = scopes.region_stop(focus.get(), backwards, &stops) {
        show_focus(&mut shown);
        focus.set(to, bevy::input_focus::FocusCause::Navigated);
    }
}
