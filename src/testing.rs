//! Test helpers (cargo feature `testing`): drive a gooey interface in a headless [`App`] by what
//! the player reads, not by entity ids.
//!
//! A player finds "Options" by its words, clicks it where it is drawn and sees where focus went.
//! These helpers do the same through the real input path (`PointerInput`, `MouseButtonInput`,
//! `KeyboardInput`), so a test cannot pass while a button is covered, hidden, unfocusable or
//! mislabelled, which a test that triggers an entity directly can.
//!
//! The app needs a primary window (Bevy's `WindowPlugin`, with `WinitPlugin` and a renderer left
//! out is enough), [`crate::GooeyPlugins`] and one `update` after the interface is spawned, so it
//! has been laid out.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_gooey::testing::{FindError, click, find_by_label, focused_label};
//!
//! // `app` is a headless app with `GooeyPlugins` and the interface under test, updated once.
//! fn clicking_options_focuses_it(app: &mut App) -> Result<(), FindError> {
//!     let options = find_by_label(app, "Options")?;
//!     click(app, options);
//!     assert_eq!(focused_label(app).as_deref(), Some("Options"));
//!     Ok(())
//! }
//! ```

use std::fmt;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButtonInput;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef};

use crate::skin::Role;
use crate::text::TextInputOf;

/// Updates run after each action, so the input is read in `PreUpdate`, its observers run, and the
/// crate's `Style` pass lands. A design default; the crate's own tests settle for four.
const SETTLE_FRAMES: usize = 2;

/// Why [`find_by_label`] did not return one widget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindError {
    /// No visible widget reads `label`.
    NotFound {
        /// The words looked for.
        label: String,
    },
    /// More than one visible widget reads `label`: the test must say which, because a silent first
    /// match would pass for the wrong one.
    Ambiguous {
        /// The words looked for.
        label: String,
        /// Each match as `Entity (Role)`.
        matches: Vec<(Entity, String)>,
    },
}

impl fmt::Display for FindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { label } => write!(
                f,
                "no visible widget reads \"{label}\" (has the app updated since the interface was spawned?)"
            ),
            Self::Ambiguous { label, matches } => {
                write!(f, "{} widgets read \"{label}\":", matches.len())?;
                for (entity, what) in matches {
                    write!(f, " {entity} ({what})")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for FindError {}

/// The one visible widget whose words are `label` (compared after trimming).
///
/// The words are a `Text` node's string. The widget is that node's nearest ancestor that takes
/// focus (a button, a toggle, a slider: anything with a `TabIndex`), or the node itself when
/// nothing above it does (a heading). A hidden page, a `Display::None` subtree and an option a
/// choice is not showing are not visible, so a "Back" on another tab is no match. Two widgets with
/// the same label are an [`FindError::Ambiguous`] naming both; one widget whose label appears twice
/// inside it (a name and its tooltip) is one match.
pub fn find_by_label(app: &mut App, label: &str) -> Result<Entity, FindError> {
    let world = app.world_mut();
    let mut texts = world.query::<(Entity, &Text, &InheritedVisibility, &ComputedNode)>();
    let mut found = Vec::new();
    for (entity, text, shown, node) in texts.iter(world) {
        if text.0.trim() != label.trim() || !shown.get() {
            continue;
        }
        let size = node.size();
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let widget = std::iter::once(entity)
            .chain(ancestors(world, entity))
            .find(|e| world.entity(*e).contains::<TabIndex>())
            .unwrap_or(entity);
        if !found.contains(&widget) {
            found.push(widget);
        }
    }
    found.sort();
    match found.as_slice() {
        [] => Err(FindError::NotFound {
            label: label.to_owned(),
        }),
        [one] => Ok(*one),
        many => Err(FindError::Ambiguous {
            label: label.to_owned(),
            matches: many
                .iter()
                .map(|e| {
                    let entity = world.entity(*e);
                    let what = entity
                        .get::<Name>()
                        .map(|n| n.to_string())
                        .or_else(|| entity.get::<Role>().map(|r| format!("{r:?}")))
                        .unwrap_or_else(|| "no role".to_owned());
                    (*e, what)
                })
                .collect(),
        }),
    }
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("a test app has one primary window")
}

fn settle(app: &mut App) {
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
}

fn location(app: &mut App, at: Vec2) -> Location {
    let window = window(app);
    Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    }
}

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let at = location(app, at);
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, at, action));
    app.update();
}

/// Click `entity` with the primary button, where it is drawn: the pointer moves to the centre of
/// its laid-out box, then presses and releases, as a mouse does.
///
/// # Panics
///
/// When `entity` has not been laid out (no `ComputedNode`): update the app first.
pub fn click(app: &mut App, entity: Entity) {
    let centre = {
        let world = app.world();
        world
            .get::<ComputedNode>(entity)
            .expect("click needs a laid-out node: update the app after spawning it");
        world
            .get::<UiGlobalTransform>(entity)
            .expect("click needs a placed node: update the app after spawning it")
            .translation
    };
    // Layout is in physical pixels, the pointer in the window's logical ones (the window's scale
    // factor alone: `UiScale` is already in the layout).
    let primary = window(app);
    let scale = app
        .world()
        .get::<Window>(primary)
        .map_or(1.0, |w| w.scale_factor());
    let at = centre / scale;
    pointer(app, at, PointerAction::Move { delta: Vec2::ONE });
    settle(app);
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        });
        let action = match state {
            ButtonState::Pressed => PointerAction::Press(PointerButton::Primary),
            ButtonState::Released => PointerAction::Release(PointerButton::Primary),
        };
        pointer(app, at, action);
    }
    settle(app);
}

/// The logical key a physical one produces on a QWERTY keyboard, for the keys a UI reads by name
/// (Enter, Escape, the arrows, Tab ...) and the letters and digits.
fn logical_key(key: KeyCode) -> Key {
    match key {
        KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
        KeyCode::Escape => Key::Escape,
        KeyCode::Tab => Key::Tab,
        KeyCode::Space => Key::Space,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::ArrowUp => Key::ArrowUp,
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Key::Shift,
        KeyCode::ControlLeft | KeyCode::ControlRight => Key::Control,
        KeyCode::AltLeft | KeyCode::AltRight => Key::Alt,
        KeyCode::SuperLeft | KeyCode::SuperRight => Key::Super,
        KeyCode::F6 => Key::F6,
        KeyCode::F10 => Key::F10,
        other => {
            let name = format!("{other:?}");
            if let Some(letter) = name.strip_prefix("Key") {
                Key::Character(letter.to_lowercase().as_str().into())
            } else if let Some(digit) = name.strip_prefix("Digit") {
                Key::Character(digit.into())
            } else {
                Key::Unidentified(NativeKey::Unidentified)
            }
        }
    }
}

/// Press and release `key`, as a keyboard does: focus's widget hears it as a real key press.
pub fn press(app: &mut App, key: KeyCode) {
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: key,
            logical_key: logical_key(key),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
    settle(app);
}

/// Type `text` into whatever has focus, one key press and release per character, each carrying the
/// text it produces (as a platform reports it), so a text field takes it.
pub fn type_text(app: &mut App, text: &str) {
    let window = window(app);
    for c in text.chars() {
        let s = c.to_string();
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::KeyA,
                logical_key: Key::Character(s.as_str().into()),
                state,
                text: (state == ButtonState::Pressed).then(|| s.as_str().into()),
                repeat: false,
                window,
            });
            app.update();
        }
    }
    settle(app);
}

/// The entity that has input focus, if any. With nothing focused Bevy may report the window:
/// [`focused_label`] is `None` for that.
pub fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

/// The words of `entity`: its own `Text`, else the first `Text` below it in tree order. A text
/// field reads as its row (the label before the field), not as the field it is.
pub fn label_of(app: &App, entity: Entity) -> Option<String> {
    let world = app.world();
    let entity = world.get::<TextInputOf>(entity).map_or(entity, |row| row.0);
    std::iter::once(entity)
        .chain(descendants(world, entity))
        .find_map(|e| {
            world
                .get::<Text>(e)
                .map(|t| t.0.trim().to_owned())
                .filter(|t| !t.is_empty())
        })
}

/// The words of the focused widget ([`label_of`]), or `None` when nothing but the window has focus.
pub fn focused_label(app: &App) -> Option<String> {
    label_of(app, focused(app)?)
}

/// `entity`'s parents, nearest first.
fn ancestors(world: &World, entity: Entity) -> Vec<Entity> {
    let mut up = Vec::new();
    let mut at = entity;
    while let Some(parent) = world.entity(at).get::<ChildOf>() {
        at = parent.parent();
        up.push(at);
    }
    up
}

/// Everything below `entity`, depth first in child order: the order the player reads a widget.
fn descendants(world: &World, entity: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack: Vec<Entity> = world
        .entity(entity)
        .get::<Children>()
        .map(|c| c.iter().rev().collect())
        .unwrap_or_default();
    while let Some(at) = stack.pop() {
        out.push(at);
        if let Some(children) = world.entity(at).get::<Children>() {
            stack.extend(children.iter().rev());
        }
    }
    out
}
