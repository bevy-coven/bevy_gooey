//! A small tool: a library of pieces to drag, a viewport that keeps the arrow keys, and an
//! inspector with a notes field several lines high. F6 and Shift+F6 move focus from part to part;
//! with focus in the viewport the arrow keys pan it and move no focus (Tab or F6 leaves). Drag a
//! piece from the library onto the viewport to place it, onto Trash to remove the last one, or onto
//! the Lamp slot, which takes lamps only and says no to a wall.
//!
//! Run `cargo run --release --example workspace`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `workspace.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10
//! frames later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from
//! `GOOEY_SHOT_FROM` (default 30) there as `f_%04d.png` (the directory must exist).
//! `GOOEY_SKIN=parchment`, `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and
//! `GOOEY_SIZE=<w>x<h>` set the start. `GOOEY_KEYS=a;b;…` stages one step every `GOOEY_KEY_EVERY`
//! (default 24) frames from frame 30, as real input: `tab`, `up`, `down`, `left`, `right`, `enter`,
//! `f6`, `sf6` (Shift+F6), `center` (Ctrl+Enter), `escape`, `pickup` (Alt+M: lifts the focused piece,
//! and drops it again), `type:<text>` (`_` is a space; a character every
//! second frame) and `drag:<from>><to>` with `wall`, `door`, `lamp` for `<from>` and `viewport`,
//! `slot`, `trash`, and `high` and `low` (a point of the viewport above and below its middle) for
//! `<to>` (the pointer moves from the middle of one to the middle of the
//! other, holds there, and lets go).

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::InputFocus;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy_gooey::*;

/// A piece of the library: what a drag carries to the viewport and to Trash.
#[derive(Clone, Debug, PartialEq)]
struct Piece(&'static str, Color);

/// A lamp: only the Lamp slot takes it.
#[derive(Clone, Debug, PartialEq)]
struct Lamp;

/// Names an entity for the staged drags.
#[derive(Component)]
struct Mark(&'static str);

/// The viewport's readout.
#[derive(Component)]
struct Readout;

/// The line under the notes: how many lines there are and whether the edit has ended.
#[derive(Component)]
struct NotesNote;

/// A piece standing in the viewport.
#[derive(Component)]
struct Placed;

/// A staged step of `GOOEY_KEYS`.
#[derive(Clone, Debug)]
enum Step {
    Key(KeyCode, Key),
    /// A key with a modifier held, pressed and released: the modifier, then the key.
    Chord(KeyCode, Key, KeyCode, Key),
    Type(String),
    Drag(&'static str, &'static str),
}

/// Capture mode's state, present only when `GOOEY_FRAMES` is set.
#[derive(Resource)]
struct Capture {
    at: u32,
    shot: String,
    dir: Option<String>,
    from: u32,
    every: u32,
    kept: u32,
    frame: u32,
    total_ms: f64,
    counted: u32,
    step_every: u32,
    steps: Vec<Step>,
}

/// The frame `GOOEY_KEYS` starts at, after the layout has settled.
const KEY_FROM: u32 = 30;
/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

fn main() {
    let mut skin = match std::env::var("GOOEY_SKIN").as_deref() {
        Ok("parchment") => GooeySkin::parchment(),
        Ok("high_contrast") => GooeySkin::high_contrast(),
        Ok("tool") => GooeySkin::tool(),
        _ => GooeySkin::dark(),
    };
    if let Some(scale) = env("GOOEY_TEXT_SCALE") {
        skin.text_scale = scale;
    }
    let mut window = Window {
        title: "bevy_gooey workspace".into(),
        ..default()
    };
    if let Some((w, h)) = std::env::var("GOOEY_SIZE")
        .ok()
        .and_then(|s| s.split_once('x').map(|(w, h)| (w.parse(), h.parse())))
        .and_then(|(w, h)| Some((w.ok()?, h.ok()?)))
    {
        window.resolution = WindowResolution::new(w, h).with_scale_factor_override(1.0);
    }
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(window),
            ..default()
        }),
        GooeyPlugins,
    ))
    .insert_resource(skin)
    .add_systems(Startup, setup)
    .add_systems(Update, (follow_background, pan, capture))
    // Staged input is written before `PreUpdate`, as a real keyboard's and mouse's are.
    .add_systems(First, stage);
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "workspace.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            step_every: env("GOOEY_KEY_EVERY").unwrap_or(24),
            steps: steps_of(&std::env::var("GOOEY_KEYS").unwrap_or_default()),
        });
    }
    app.run();
}

fn steps_of(text: &str) -> Vec<Step> {
    let named = |name: &str| -> Option<&'static str> {
        [
            "wall", "door", "lamp", "viewport", "high", "low", "slot", "trash",
        ]
        .into_iter()
        .find(|n| *n == name)
    };
    text.split(';')
        .filter_map(|word| {
            let word = word.trim();
            if let Some(words) = word.strip_prefix("type:") {
                return Some(Step::Type(words.replace('_', " ")));
            }
            if let Some(pair) = word.strip_prefix("drag:") {
                let (from, to) = pair.split_once('>')?;
                return Some(Step::Drag(named(from)?, named(to)?));
            }
            Some(match word {
                "tab" => Step::Key(KeyCode::Tab, Key::Tab),
                "up" => Step::Key(KeyCode::ArrowUp, Key::ArrowUp),
                "down" => Step::Key(KeyCode::ArrowDown, Key::ArrowDown),
                "left" => Step::Key(KeyCode::ArrowLeft, Key::ArrowLeft),
                "right" => Step::Key(KeyCode::ArrowRight, Key::ArrowRight),
                "enter" => Step::Key(KeyCode::Enter, Key::Enter),
                "f6" => Step::Key(KeyCode::F6, Key::F6),
                "escape" => Step::Key(KeyCode::Escape, Key::Escape),
                "pickup" => Step::Chord(
                    KeyCode::AltLeft,
                    Key::Alt,
                    KeyCode::KeyM,
                    Key::Character("m".into()),
                ),
                "sf6" => Step::Chord(KeyCode::ShiftLeft, Key::Shift, KeyCode::F6, Key::F6),
                "center" => Step::Chord(
                    KeyCode::ControlLeft,
                    Key::Control,
                    KeyCode::Enter,
                    Key::Enter,
                ),
                _ => return None,
            })
        })
        .collect()
}

/// A library row: a button that carries `payload` when dragged.
fn row<T: Clone + Send + Sync + 'static>(
    name: &'static str,
    mark: &'static str,
    payload: T,
) -> impl Bundle {
    (button(name), DragPayload(payload), Mark(mark))
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let library = commands
        .spawn((
            FocusRegion::navigate(),
            Node {
                width: Val::Px(230.0),
                ..default()
            },
            children![(
                panel(),
                children![
                    heading("Library"),
                    (
                        menu(),
                        children![
                            row("Wall", "wall", Piece("Wall", Color::srgb(0.85, 0.55, 0.25))),
                            row("Door", "door", Piece("Door", Color::srgb(0.35, 0.7, 0.45))),
                            row("Lamp", "lamp", Lamp),
                        ]
                    ),
                    label("Drag a piece onto the viewport."),
                ],
            )],
        ))
        .id();
    let view = commands
        .spawn((
            viewport(),
            Mark("viewport"),
            DropTarget::<Piece>::new(),
            children![(
                Node {
                    padding: UiRect::all(Val::Px(12.0)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                children![
                    (label("Pan 0, 0  |  0 pieces"), Readout),
                    label("The arrow keys pan; F6 leaves."),
                ],
            )],
        ))
        .observe(place)
        .id();
    let inspector = commands
        .spawn((
            FocusRegion::navigate(),
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            children![(
                panel(),
                children![
                    heading("Inspector"),
                    text_area("Notes", "Walls hold the roof up.", 5),
                    (label("Ctrl+Enter saves the notes."), NotesNote),
                    (
                        panel(),
                        Mark("slot"),
                        DropTarget::<Lamp>::new(),
                        children![label("Lamp slot: lamps only")],
                    ),
                    (
                        panel(),
                        Mark("trash"),
                        DropTarget::<Piece>::new(),
                        children![label("Trash")],
                    ),
                ],
            )],
        ))
        .id();
    commands
        .spawn(screen())
        .insert(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            padding: UiRect::all(Val::Px(16.0)),
            align_items: AlignItems::Stretch,
            ..default()
        })
        .add_children(&[library, view, inspector]);
    commands.add_observer(trash);
    commands.add_observer(
        |change: On<ValueChange<String>>, mut notes: Query<&mut Text, With<NotesNote>>| {
            let lines = change.value.lines().count().max(1);
            let words = if change.is_final {
                format!("Saved {lines} lines.")
            } else {
                format!("Editing, {lines} lines.")
            };
            for mut text in &mut notes {
                text.0.clone_from(&words);
            }
        },
    );
    commands.add_observer(|dropped: On<DropRefused<Piece>>| {
        info!("the {} slot says no", dropped.payload.0);
    });
}

/// A piece dropped on the viewport stands where the pointer was let go.
fn place(
    dropped: On<Dropped<Piece>>,
    frames: Query<(&ComputedNode, &UiGlobalTransform)>,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    let Ok((node, at)) = frames.get(dropped.target) else {
        return;
    };
    let inverse = node.inverse_scale_factor();
    let corner = at.translation * inverse - node.size() * inverse / 2.0;
    let here = dropped.pointer / ui_scale.0 - corner - Vec2::splat(28.0);
    commands.entity(dropped.target).with_child((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(here.x),
            top: Val::Px(here.y),
            width: Val::Px(56.0),
            height: Val::Px(56.0),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(dropped.payload.1),
        Pickable::IGNORE,
        Placed,
    ));
}

/// A piece dropped on Trash removes the last piece placed.
fn trash(
    dropped: On<Dropped<Piece>>,
    marks: Query<&Mark>,
    placed: Query<Entity, With<Placed>>,
    mut commands: Commands,
) {
    if marks.get(dropped.target).is_ok_and(|m| m.0 == "trash")
        && let Some(last) = placed.iter().last()
    {
        commands.entity(last).despawn();
    }
}

/// The arrow keys pan the viewport while it has focus: the host reads them, gooey moves no focus.
fn pan(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    views: Query<Entity, With<Role>>,
    marks: Query<&Mark>,
    placed: Query<(), With<Placed>>,
    mut readout: Query<&mut Text, With<Readout>>,
    mut at: Local<IVec2>,
) {
    let on_view = focus
        .get()
        .is_some_and(|f| views.contains(f) && marks.get(f).is_ok_and(|m| m.0 == "viewport"));
    if on_view {
        at.x += i32::from(keys.just_pressed(KeyCode::ArrowRight))
            - i32::from(keys.just_pressed(KeyCode::ArrowLeft));
        at.y += i32::from(keys.just_pressed(KeyCode::ArrowDown))
            - i32::from(keys.just_pressed(KeyCode::ArrowUp));
    }
    let line = format!(
        "Pan {}, {}  |  {} pieces",
        at.x,
        at.y,
        placed.iter().count()
    );
    for mut text in &mut readout {
        if text.0 != line {
            text.0.clone_from(&line);
        }
    }
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

fn capture(
    capture: Option<ResMut<Capture>>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
) {
    let Some(mut c) = capture else {
        return;
    };
    c.frame += 1;
    if c.frame > WARM_FRAMES && c.frame <= c.at {
        c.total_ms += time.delta_secs_f64() * 1000.0;
        c.counted += 1;
    }
    if c.frame == c.at {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(c.shot.clone()));
        let mean = if c.counted > 0 {
            c.total_ms / f64::from(c.counted)
        } else {
            0.0
        };
        println!("gooey: frames {} mean_ms {mean:.3}", c.at);
    }
    if let Some(dir) = c.dir.clone() {
        let every = c.every.max(1);
        if c.frame >= c.from && (c.frame - c.from) % every == 0 {
            c.kept += 1;
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{dir}/f_{:04}.png", c.kept)));
        }
        if c.frame == c.at {
            println!("gooey: kept {} frames in {dir}", c.kept);
        }
    }
    if c.frame == c.at + 10 {
        exit.write(AppExit::Success);
    }
}

/// `GOOEY_KEYS`: real keyboard and pointer input, one step every `GOOEY_KEY_EVERY` frames.
#[allow(clippy::too_many_arguments)]
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<(Entity, &Window), With<PrimaryWindow>>,
    marks: Query<(&Mark, &ComputedNode, &UiGlobalTransform)>,
    mut keys: MessageWriter<KeyboardInput>,
    mut pointer: MessageWriter<PointerInput>,
    mut grabbed: Local<(Vec2, Vec2)>,
) {
    let (Some(c), Ok((window, props))) = (capture, window.single()) else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.step_every.max(8);
    let Some(step) = c.steps.get((since / every) as usize) else {
        return;
    };
    let phase = since % every;
    let mut key = |key_code: KeyCode, logical_key: Key, text: Option<&str>, state: ButtonState| {
        keys.write(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: text.map(Into::into),
            repeat: false,
            window,
        });
    };
    match step {
        Step::Key(key_code, logical) => match phase {
            0 => key(*key_code, logical.clone(), None, ButtonState::Pressed),
            n if n == every / 2 => key(*key_code, logical.clone(), None, ButtonState::Released),
            _ => {}
        },
        Step::Chord(held, held_key, key_code, logical) => match phase {
            0 => key(*held, held_key.clone(), None, ButtonState::Pressed),
            2 => key(*key_code, logical.clone(), None, ButtonState::Pressed),
            n if n == every / 2 => key(*key_code, logical.clone(), None, ButtonState::Released),
            n if n == every / 2 + 1 => key(*held, held_key.clone(), None, ButtonState::Released),
            _ => {}
        },
        Step::Type(words) => {
            // A character every second frame: pressed on the even frame, let go on the next.
            let index = (phase / 2) as usize;
            if let Some(c) = words.chars().nth(index) {
                let text = c.to_string();
                let state = if phase % 2 == 0 {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                };
                key(
                    KeyCode::KeyA,
                    Key::Character(text.as_str().into()),
                    (state == ButtonState::Pressed).then_some(text.as_str()),
                    state,
                );
            }
        }
        Step::Drag(from, to) => {
            let Some(target) = WindowRef::Entity(window).normalize(None) else {
                return;
            };
            let centre = |name: &str| {
                // `high` and `low` are points of the viewport, off its middle.
                let (mark, offset) = match name {
                    "high" => ("viewport", Vec2::new(90.0, -170.0)),
                    "low" => ("viewport", Vec2::new(-110.0, 140.0)),
                    other => (other, Vec2::ZERO),
                };
                marks
                    .iter()
                    .find(|(m, ..)| m.0 == mark)
                    .map(|(_, _, at)| at.translation / props.scale_factor() + offset)
            };
            let (position, action) = match phase {
                0 => {
                    let (Some(a), Some(b)) = (centre(from), centre(to)) else {
                        return;
                    };
                    // Over the target's upper middle: a viewport's middle is where pieces stand.
                    *grabbed = (a, b);
                    (a, PointerAction::Move { delta: Vec2::ZERO })
                }
                1 => (grabbed.0, PointerAction::Press(PointerButton::Primary)),
                n if n < every - 6 => {
                    let t = ((n - 1) as f32 / (every - 8) as f32).clamp(0.0, 1.0);
                    let eased = t * t * (3.0 - 2.0 * t);
                    let to = grabbed.0.lerp(grabbed.1, eased);
                    (
                        to,
                        PointerAction::Move {
                            delta: (grabbed.1 - grabbed.0) / every as f32,
                        },
                    )
                }
                n if n == every - 3 => (grabbed.1, PointerAction::Release(PointerButton::Primary)),
                n if n > every - 6 && n < every - 3 => {
                    (grabbed.1, PointerAction::Move { delta: Vec2::ZERO })
                }
                _ => return,
            };
            pointer.write(PointerInput::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::Window(target),
                    position,
                },
                action,
            ));
        }
    }
}
