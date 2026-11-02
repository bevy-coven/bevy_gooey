//! Shortcuts: a key chord as one value, its platform-correct words, and the one rule that decides
//! whether it was just pressed. The arithmetic (the order and the spelling, the conflicts) is
//! [`crate::model`]'s, engine-free.

use std::ops::BitOr;

use bevy::prelude::*;

use crate::model::{is_modifier_key, shortcut_label};

/// The platform a chord is spelled for: a mac says "⇧⌘P", every other platform "Ctrl+Shift+P".
/// [`Platform::current`] is the one this build runs on; a test or an editor that shows another
/// platform's keymap names it.
///
/// A web build running in a browser on a mac cannot know (the compile target is `wasm32`), and
/// says "Ctrl": a game that can ask the browser passes [`Platform::Mac`] to
/// [`Shortcut::label_on`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, PartialEq, Hash, Clone)]
pub enum Platform {
    /// macOS: Command is the primary modifier, labels use Apple's symbols and order.
    Mac,
    /// Windows, Linux and the rest: Ctrl is the primary modifier, labels spell the words.
    Other,
}

impl Platform {
    /// The platform this build was compiled for.
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Other
        }
    }
}

/// The modifier keys held with a key, as the *physical* keys (either side of the keyboard counts):
/// a saved keymap says what was pressed, so a mac player's Control-S and Command-S are two
/// different chords. [`Modifiers::PRIMARY`] is the platform's own "primary" (Command on a mac,
/// Ctrl elsewhere), which is what a game binds "Save" to once for every platform.
///
/// Combine them with `|`:
///
/// ```
/// # use bevy_gooey::*;
/// let chord = Modifiers::CTRL | Modifiers::SHIFT;
/// assert!(chord.ctrl && chord.shift && !chord.alt && !chord.meta);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Default, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Modifiers {
    /// Control, on either side.
    pub ctrl: bool,
    /// Shift, on either side.
    pub shift: bool,
    /// Alt (Option on a mac), on either side.
    pub alt: bool,
    /// The Windows or Super key, or Command on a mac, on either side.
    pub meta: bool,
}

impl Modifiers {
    /// No modifier held.
    pub const NONE: Self = Self::of(false, false, false, false);
    /// Control.
    pub const CTRL: Self = Self::of(true, false, false, false);
    /// Shift.
    pub const SHIFT: Self = Self::of(false, true, false, false);
    /// Alt (Option on a mac).
    pub const ALT: Self = Self::of(false, false, true, false);
    /// The Windows or Super key, Command on a mac.
    pub const META: Self = Self::of(false, false, false, true);
    /// The platform's primary modifier: Command on a mac, Ctrl elsewhere
    /// ([`Modifiers::primary_on`] for the current compile target).
    pub const PRIMARY: Self = Self::primary_on(Platform::current());

    const fn of(ctrl: bool, shift: bool, alt: bool, meta: bool) -> Self {
        Self {
            ctrl,
            shift,
            alt,
            meta,
        }
    }

    /// The primary modifier of `platform`: Command on a mac, Ctrl elsewhere.
    pub const fn primary_on(platform: Platform) -> Self {
        match platform {
            Platform::Mac => Self::META,
            Platform::Other => Self::CTRL,
        }
    }

    /// The modifiers held right now on `keys`, either side of the keyboard counting.
    pub fn held(keys: &ButtonInput<KeyCode>) -> Self {
        let either = |left, right| keys.pressed(left) || keys.pressed(right);
        Self {
            ctrl: either(KeyCode::ControlLeft, KeyCode::ControlRight),
            shift: either(KeyCode::ShiftLeft, KeyCode::ShiftRight),
            alt: either(KeyCode::AltLeft, KeyCode::AltRight),
            meta: either(KeyCode::SuperLeft, KeyCode::SuperRight),
        }
    }

    /// Whether none is held.
    pub const fn is_empty(self) -> bool {
        !(self.ctrl || self.shift || self.alt || self.meta)
    }
}

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self::of(
            self.ctrl | other.ctrl,
            self.shift | other.shift,
            self.alt | other.alt,
            self.meta | other.meta,
        )
    }
}

/// A key chord: a key and the modifiers held with it. `Eq + Hash`, serde under the `serde` feature,
/// so a game saves its keymap as data and [`crate::model::shortcut_conflicts`] finds two commands
/// on one chord.
///
/// A modifier is never the key of a shortcut (a press of Ctrl alone is not a shortcut): build a
/// chord from a non-modifier key; [`Shortcut::is_valid`] checks one loaded from a file.
///
/// It is also a component: put one on a widget (`(button("Save"), Shortcut::primary(KeyCode::KeyS))`)
/// and [`ShortcutBadges`] shows its label on the widget while the player holds the modifier. The
/// crate never runs the chord: the game compares [`Shortcut::just_pressed`] itself.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// let palette = Shortcut::new(Modifiers::CTRL | Modifiers::SHIFT, KeyCode::KeyP);
/// assert_eq!(palette.label_on(Platform::Other), "Ctrl+Shift+P");
/// assert_eq!(palette.label_on(Platform::Mac), "⌃⇧P");
/// // One "Save" for every platform: Command-S on a mac, Ctrl+S elsewhere.
/// let save = Shortcut::primary(KeyCode::KeyS);
/// assert_eq!(save.label(), if cfg!(target_os = "macos") { "⌘S" } else { "Ctrl+S" });
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Debug, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Shortcut {
    /// The modifiers held with the key.
    pub modifiers: Modifiers,
    /// The key, by its place on the keyboard (so the chord is the same on every layout, as
    /// [`crate::Binding::Key`] is).
    pub key: KeyCode,
}

impl Shortcut {
    /// `key` with `modifiers` held.
    pub const fn new(modifiers: Modifiers, key: KeyCode) -> Self {
        Self { modifiers, key }
    }

    /// `key` with the platform's primary modifier: Command-`key` on a mac, Ctrl+`key` elsewhere.
    pub const fn primary(key: KeyCode) -> Self {
        Self::new(Modifiers::PRIMARY, key)
    }

    /// Whether this is a shortcut: its key is not itself a modifier. Chords the crate builds
    /// ([`crate::binding`] rows) always are; a value read from a file may not be.
    pub fn is_valid(&self) -> bool {
        !is_modifier_key(&format!("{:?}", self.key))
    }

    /// The chord as this build's platform spells it: "Ctrl+Shift+P", or "⇧⌘P" on a mac.
    /// Apple's order is Control, Option, Shift, Command; Microsoft's is Windows logo, Ctrl, Alt,
    /// Shift. A hint, a tooltip and a [`crate::binding`] row all say it this way.
    pub fn label(&self) -> String {
        self.label_on(Platform::current())
    }

    /// The chord as `platform` spells it.
    pub fn label_on(&self, platform: Platform) -> String {
        let Modifiers {
            ctrl,
            shift,
            alt,
            meta,
        } = self.modifiers;
        shortcut_label(
            &format!("{:?}", self.key),
            [ctrl, shift, alt, meta],
            platform == Platform::Mac,
        )
    }

    /// Whether this chord went down this frame: the key was just pressed with exactly these
    /// modifiers held (either side of the keyboard counts). Exactly: Ctrl+Shift+S does not
    /// trigger Ctrl+S, so two chords on one key never both fire. The one rule every handler of
    /// chords shares.
    pub fn just_pressed(&self, keys: &ButtonInput<KeyCode>) -> bool {
        keys.just_pressed(self.key) && Modifiers::held(keys) == self.modifiers
    }
}
