//! Test helpers (cargo feature `testing`): drive a gooey interface in a headless [`App`] by what
//! the player reads, not by entity ids.
//!
//! A player finds "Options" by its words, clicks it where it is drawn and sees where focus went.
//! These helpers do the same through the real input path (`PointerInput`, `MouseButtonInput`,
//! `KeyboardInput`), so a test cannot pass while a button is covered, hidden, unfocusable or
//! mislabelled, which a test that triggers an entity directly can.
//!
//! The app needs [`crate::GooeyPlugins`], a camera that draws the interface and one `update` after
//! the interface is spawned, so it has been laid out. A click goes to the render target of the camera
//! that draws the widget, as the picking backend matches pointers to cameras: a primary window
//! (Bevy's `WindowPlugin`, with `WinitPlugin` and a renderer left out is enough) or an `Image`, so
//! a test of an interface drawn into a texture needs no window at all. Bevy's own key handling of
//! a focused widget (Enter on a button, a character in a field) is dispatched only while a primary
//! window exists, so a test that presses keys on such a widget wants one.
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

use bevy::camera::{NormalizedRenderTarget, RenderTarget};
use bevy::ecs::change_detection::Tick;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButtonInput;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{AcquireFocus, InputFocus, InputFocusVisible};
use bevy::picking::events::PointerPress;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, ComputedUiTargetCamera, UiGlobalTransform};
use bevy::window::PrimaryWindow;

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

/// The window the keyboard's and the mouse's messages name: the primary window, or none when a
/// test draws only into images (the messages' `window` is then a placeholder, which nothing reads).
fn primary_window(app: &mut App) -> Option<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .ok()
}

/// What a pointer needs to land on a widget: the render target of the camera that draws it, that
/// target's scale factor (a window's, an image's) and the window the input messages name.
struct Surface {
    target: NormalizedRenderTarget,
    scale: f32,
    window: Entity,
}

/// Marks an app whose clicks are given focus by [`windowless_click_to_focus`].
#[derive(Resource)]
struct WindowlessFocus;

/// Bevy gives a clicked widget focus only while a primary window exists (`click_to_focus` looks the
/// window up to name it in the request), so in an app that draws only into images a click would
/// reach the widget and leave focus where it was. This is that observer without the window: the
/// same hidden focus ring, the same bubbling `AcquireFocus` that the widgets' own resolvers answer.
fn windowless_click_to_focus(
    press: On<PointerPress>,
    mut shown: ResMut<InputFocusVisible>,
    windows: Query<(), With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if press.entity == press.original_event_target() && windows.is_empty() {
        if shown.0 {
            shown.0 = false;
        }
        commands.trigger(AcquireFocus {
            focused_entity: press.entity,
            window: Entity::PLACEHOLDER,
        });
    }
}

/// The surface `entity` is drawn on, found the way the picking backend matches pointers to
/// cameras: its `ComputedUiTargetCamera` (a `UiTargetCamera`, or the default UI camera), then that
/// camera's `RenderTarget`.
fn surface_of(app: &mut App, entity: Entity) -> Surface {
    let primary = primary_window(app);
    let world = app.world();
    let camera = world
        .get::<ComputedUiTargetCamera>(entity)
        .and_then(ComputedUiTargetCamera::get)
        .unwrap_or_else(|| {
            panic!(
                "{entity} is drawn by no camera: spawn a camera for the interface (a `Camera2d`, or \
                 a `UiTargetCamera` on its root) and update the app"
            )
        });
    let target = world
        .get::<RenderTarget>(camera)
        .unwrap_or_else(|| panic!("the camera {camera} that draws {entity} has no `RenderTarget`"))
        .normalize(primary)
        .unwrap_or_else(|| {
            panic!(
                "the camera {camera} that draws {entity} renders to a window, and the app has none"
            )
        });
    let (scale, window) = match &target {
        NormalizedRenderTarget::Window(window) => (
            world
                .get::<Window>(window.entity())
                .map_or(1.0, |w| w.scale_factor()),
            window.entity(),
        ),
        NormalizedRenderTarget::Image(image) => {
            (image.scale_factor, primary.unwrap_or(Entity::PLACEHOLDER))
        }
        _ => (1.0, primary.unwrap_or(Entity::PLACEHOLDER)),
    };
    Surface {
        target,
        scale,
        window,
    }
}

fn settle(app: &mut App) {
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
}

fn pointer(app: &mut App, surface: &Surface, at: Vec2, action: PointerAction) {
    let location = Location {
        target: surface.target.clone(),
        position: at,
    };
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, location, action));
    app.update();
}

/// Click `entity` with the primary button, where it is drawn: the pointer moves to the centre of
/// its laid-out box, then presses and releases, as a mouse does.
///
/// # Panics
///
/// When `entity` has not been laid out (no `ComputedNode`): update the app first. When no camera
/// draws it, or the camera renders to a window the app does not have, with a message that names
/// which.
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
    // Layout is in physical pixels, the pointer in the target's logical ones (the target's scale
    // factor alone: `UiScale` is already in the layout).
    let surface = surface_of(app, entity);
    if primary_window(app).is_none() && !app.world().contains_resource::<WindowlessFocus>() {
        app.world_mut().insert_resource(WindowlessFocus);
        app.add_observer(windowless_click_to_focus);
    }
    let at = centre / surface.scale;
    pointer(app, &surface, at, PointerAction::Move { delta: Vec2::ONE });
    settle(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window: surface.window,
        });
        let action = match state {
            ButtonState::Pressed => PointerAction::Press(PointerButton::Primary),
            ButtonState::Released => PointerAction::Release(PointerButton::Primary),
        };
        pointer(app, &surface, at, action);
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
    let window = primary_window(app).unwrap_or(Entity::PLACEHOLDER);
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
    let window = primary_window(app).unwrap_or(Entity::PLACEHOLDER);
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

/// What one frame of a [`MotionTrace`] saw of the entity: each field is `None` when the entity has
/// no such component, on every frame.
#[derive(Clone, Debug, PartialEq)]
pub struct TraceFrame {
    /// Seconds of [`Time<Real>`] since the trace began watching: 0 for the first frame.
    pub seconds: f32,
    /// Seconds this frame took (the update that produced it); 0 for the first frame.
    pub delta: f32,
    /// The entity's [`UiTransform`], what a game and this crate's motion write.
    pub transform: Option<UiTransform>,
    /// Where the node is drawn, the centre of its [`UiGlobalTransform`], in logical px.
    pub position: Option<Vec2>,
    /// How big it is laid out, [`ComputedNode::size`], in logical px.
    pub size: Option<Vec2>,
    /// Its [`BackgroundColor`].
    pub background: Option<Color>,
    /// Its [`BorderColor`].
    pub border: Option<BorderColor>,
    /// Its [`TextColor`].
    pub text: Option<Color>,
    /// How many of [`UiTransform`], [`BackgroundColor`], [`BorderColor`] and [`TextColor`] were
    /// written (by change tick, whether or not the value differed) in the update that produced
    /// this frame. 0 for the first frame. Layout's own outputs are not counted.
    pub writes: usize,
}

impl TraceFrame {
    /// Whether `other` looks the same: every recorded value, not the clock or the writes.
    fn looks_like(&self, other: &Self) -> bool {
        self.transform == other.transform
            && self.position == other.position
            && self.size == other.size
            && self.background == other.background
            && self.border == other.border
            && self.text == other.text
    }
}

/// The entity a [`MotionTrace`] watched is gone: it was despawned after `frame` frames were
/// recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceEnded {
    /// How many frames the trace holds: the first frame is frame 0.
    pub frame: usize,
}

/// A frame-by-frame record of one entity's pose, for tests of motion: whether it is continuous,
/// settles, is still when it should be, and writes nothing at rest.
///
/// Drive the app on a clock the test sets (`TimeUpdateStrategy::ManualDuration`), so a frame is a
/// known length: [`MotionTrace::watch`] records the entity as it is, [`MotionTrace::step`] runs
/// frames and records each. The questions then read the record. A step in the middle of a slide, a
/// speed that jumps, or a write after the motion is over, which a hand-sampled test passes, show.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_gooey::testing::MotionTrace;
///
/// // `app` runs on a manual clock; `card` was spawned and the app updated once.
/// fn the_card_settles(app: &mut App, card: Entity) {
///     let mut trace = MotionTrace::watch(app, card);
///     trace.step(app, 30);
///     assert!(trace.max_step_px() < 12.0);
///     assert!(trace.settled_after_seconds().is_some_and(|s| s < 0.3));
///     trace.step(app, 30);
///     assert_eq!(trace.writes_at_rest(30), 0);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct MotionTrace {
    entity: Entity,
    frames: Vec<TraceFrame>,
    ended: Option<TraceEnded>,
    began: f32,
}

/// The components whose change tick counts as the entity being written.
fn written_since(world: &World, entity: Entity, since: Tick, now: Tick) -> usize {
    fn changed<T: Component>(world: &World, entity: Entity, since: Tick, now: Tick) -> usize {
        world
            .get_entity(entity)
            .ok()
            .and_then(|e| e.get_ref::<T>())
            .is_some_and(|c| c.last_changed().is_newer_than(since, now)) as usize
    }
    changed::<UiTransform>(world, entity, since, now)
        + changed::<BackgroundColor>(world, entity, since, now)
        + changed::<BorderColor>(world, entity, since, now)
        + changed::<TextColor>(world, entity, since, now)
}

fn real_seconds(world: &World) -> f32 {
    world
        .get_resource::<Time<Real>>()
        .map_or(0.0, |time| time.elapsed_secs())
}

fn sample(
    world: &World,
    entity: Entity,
    began: f32,
    delta: f32,
    writes: usize,
) -> Option<TraceFrame> {
    let e = world.get_entity(entity).ok()?;
    let node = e.get::<ComputedNode>();
    let scale = node.map_or(1.0, |n| n.inverse_scale_factor());
    Some(TraceFrame {
        seconds: real_seconds(world) - began,
        delta,
        transform: e.get::<UiTransform>().copied(),
        position: e.get::<UiGlobalTransform>().map(|t| t.translation * scale),
        size: node.map(|n| n.size() * scale),
        background: e.get::<BackgroundColor>().map(|c| c.0),
        border: e.get::<BorderColor>().copied(),
        text: e.get::<TextColor>().map(|c| c.0),
        writes,
    })
}

impl MotionTrace {
    /// Starts watching `entity`: its pose as it is now is frame 0. An entity that does not exist
    /// is an ended trace of no frames.
    pub fn watch(app: &mut App, entity: Entity) -> Self {
        let world = app.world();
        let began = real_seconds(world);
        let first = sample(world, entity, began, 0.0, 0);
        Self {
            entity,
            ended: first.is_none().then_some(TraceEnded { frame: 0 }),
            frames: first.into_iter().collect(),
            began,
        }
    }

    /// Runs `frames` updates and records the entity after each. An entity despawned along the way
    /// ends the trace ([`MotionTrace::ended`]); the remaining updates still run, so the app goes on
    /// as the test expects, and nothing panics.
    pub fn step(&mut self, app: &mut App, frames: usize) {
        for _ in 0..frames {
            // The change tick of the first write in an update is the one read before it, so a
            // write is "since" the tick before that.
            let before = app.world().read_change_tick();
            let since = Tick::new(before.get().wrapping_sub(1));
            let clock = real_seconds(app.world());
            app.update();
            if self.ended.is_some() {
                continue;
            }
            let world = app.world();
            let now = world.read_change_tick();
            let writes = written_since(world, self.entity, since, now);
            let delta = real_seconds(world) - clock;
            match sample(world, self.entity, self.began, delta, writes) {
                Some(frame) => self.frames.push(frame),
                None => {
                    self.ended = Some(TraceEnded {
                        frame: self.frames.len(),
                    });
                }
            }
        }
    }

    /// The frames recorded, the first at index 0.
    pub fn frames(&self) -> &[TraceFrame] {
        &self.frames
    }

    /// Whether the entity was despawned during the trace, and after how many frames.
    pub fn ended(&self) -> Option<TraceEnded> {
        self.ended
    }

    /// Whether nothing visible changed between the first frame and the second: an entity that is
    /// simply there does not animate for having appeared. A slide that starts on its first frame
    /// is not still, by design; read its start from [`MotionTrace::frames`]. A trace of fewer than
    /// two frames is still.
    pub fn still_on_first_frame(&self) -> bool {
        match self.frames.as_slice() {
            [first, second, ..] => first.looks_like(second),
            _ => true,
        }
    }

    /// The farthest the node was drawn to move between two frames, logical px: 0 with no layout.
    pub fn max_step_px(&self) -> f32 {
        self.steps()
            .into_iter()
            .map(|(step, _)| step)
            .fold(0.0, f32::max)
    }

    /// The biggest change of speed from one frame to the next, logical px a second, 0 with fewer
    /// than two steps: a smooth motion has none that stands out, a snap has one.
    pub fn max_speed_change_px_per_s(&self) -> f32 {
        let speeds: Vec<f32> = self
            .steps()
            .into_iter()
            .filter(|(_, delta)| *delta > 0.0)
            .map(|(step, delta)| step / delta)
            .collect();
        speeds
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max)
    }

    /// Seconds from the first frame until the last one in which anything changed, or `None` when
    /// the entity was still changing on the final frame: not yet settled.
    pub fn settled_after_seconds(&self) -> Option<f32> {
        let last = self.frames.len().checked_sub(1)?;
        let moved = self
            .frames
            .windows(2)
            .rposition(|w| !w[0].looks_like(&w[1]))
            .map(|i| i + 1);
        match moved {
            None => Some(0.0),
            Some(i) if i == last => None,
            Some(i) => Some(self.frames[i].seconds),
        }
    }

    /// How many writes ([`TraceFrame::writes`]) the last `frames` frames hold: 0 when a settled
    /// interface writes nothing.
    pub fn writes_at_rest(&self, frames: usize) -> usize {
        self.frames
            .iter()
            .rev()
            .take(frames)
            .map(|frame| frame.writes)
            .sum()
    }

    /// The distance drawn between each two frames and the time between them.
    fn steps(&self) -> Vec<(f32, f32)> {
        self.frames
            .windows(2)
            .filter_map(|w| Some((w[1].position?.distance(w[0].position?), w[1].delta)))
            .collect()
    }
}
