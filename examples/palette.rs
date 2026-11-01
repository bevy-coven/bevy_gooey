//! Command palette: Ctrl+Shift+P (Cmd+Shift+P on a Mac) opens a field over every command of a small
//! editor, each with its chord and, when it cannot run, why. Type a few letters ("opn prj"), move
//! with Up and Down, run with Enter or a click, close with Escape. The line under the title says
//! which command ran last; commands run since the start come first the next time.
//!
//! Run `cargo run --release --example palette`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `palette.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS=palette,type:opn prj,down,enter,…` stages one step every `GOOEY_KEY_EVERY` (default
//! 10) frames from frame 30: `palette` opens it, `type:<text>` types the text, and `up`, `down`,
//! `enter`, `escape`, `pagedown` and `pageup` are real keyboard input.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_gooey::*;

/// A staged step of `GOOEY_KEYS`.
#[derive(Clone, Debug)]
enum Step {
    Open,
    Type(String),
    Key(KeyCode, Key),
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

/// The frame `GOOEY_KEYS` starts at, after the layout and autofocus have settled.
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
        title: "bevy_gooey palette".into(),
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
    .insert_resource(commands_of_an_editor())
    .add_observer(on_run)
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            follow_background,
            open_palette.run_if(not(is_text_editing)),
            capture,
            stage.after(capture),
        ),
    );
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "palette.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            step_every: env("GOOEY_KEY_EVERY").unwrap_or(10),
            steps: std::env::var("GOOEY_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|k| match k.trim() {
                    "palette" => Some(Step::Open),
                    "up" => Some(Step::Key(KeyCode::ArrowUp, Key::ArrowUp)),
                    "down" => Some(Step::Key(KeyCode::ArrowDown, Key::ArrowDown)),
                    "enter" => Some(Step::Key(KeyCode::Enter, Key::Enter)),
                    "escape" => Some(Step::Key(KeyCode::Escape, Key::Escape)),
                    "pagedown" => Some(Step::Key(KeyCode::PageDown, Key::PageDown)),
                    "pageup" => Some(Step::Key(KeyCode::PageUp, Key::PageUp)),
                    other => other
                        .strip_prefix("type:")
                        .map(|text| Step::Type(text.to_owned())),
                })
                .collect(),
        });
    }
    app.run();
}

/// What a small level editor can do: a few dozen commands in four categories, two of them off.
fn commands_of_an_editor() -> CommandList {
    let primary = Shortcut::primary;
    let shifted = |key| Shortcut::new(Modifiers::PRIMARY | Modifiers::SHIFT, key);
    let c = |id: &str, title: &str, category: &str| PaletteCommand::new(id, title).category(category);
    CommandList(vec![
        c("file.new", "New Project", "File").shortcut(primary(KeyCode::KeyN)),
        c("file.open", "Open Project", "File").shortcut(primary(KeyCode::KeyO)),
        c("file.reopen", "Reopen Last Project", "File"),
        c("file.save", "Save", "File")
            .shortcut(primary(KeyCode::KeyS))
            .disabled("No project is open"),
        c("file.save_as", "Save As...", "File")
            .shortcut(shifted(KeyCode::KeyS))
            .disabled("No project is open"),
        c("file.export", "Export Level", "File").shortcut(primary(KeyCode::KeyE)),
        c("file.settings", "Open Project Settings", "File"),
        c("file.quit", "Quit", "File").shortcut(primary(KeyCode::KeyQ)),
        c("edit.undo", "Undo", "Edit").shortcut(primary(KeyCode::KeyZ)),
        c("edit.redo", "Redo", "Edit")
            .shortcut(shifted(KeyCode::KeyZ))
            .disabled("Nothing to redo"),
        c("edit.cut", "Cut", "Edit").shortcut(primary(KeyCode::KeyX)),
        c("edit.copy", "Copy", "Edit").shortcut(primary(KeyCode::KeyC)),
        c("edit.paste", "Paste", "Edit").shortcut(primary(KeyCode::KeyV)),
        c("edit.duplicate", "Duplicate", "Edit").shortcut(primary(KeyCode::KeyD)),
        c("edit.delete", "Delete Selection", "Edit").shortcut(Shortcut::new(
            Modifiers::NONE,
            KeyCode::Delete,
        )),
        c("edit.prefs", "Open Preferences", "Edit").shortcut(primary(KeyCode::Comma)),
        c("view.grid", "Toggle Grid", "View").shortcut(primary(KeyCode::KeyG)),
        c("view.snap", "Toggle Snap to Grid", "View"),
        c("view.inspector", "Show Inspector", "View").shortcut(primary(KeyCode::KeyI)),
        c("view.console", "Show Console", "View").shortcut(primary(KeyCode::Backquote)),
        c("view.zoom_in", "Zoom In", "View").shortcut(primary(KeyCode::Equal)),
        c("view.zoom_out", "Zoom Out", "View").shortcut(primary(KeyCode::Minus)),
        c("view.zoom_fit", "Zoom to Fit", "View").shortcut(primary(KeyCode::Digit0)),
        c("view.fullscreen", "Toggle Fullscreen", "View").shortcut(Shortcut::new(
            Modifiers::NONE,
            KeyCode::F11,
        )),
        c("level.add_light", "Add Light", "Level"),
        c("level.add_camera", "Add Camera", "Level"),
        c("level.add_spawn", "Add Spawn Point", "Level"),
        c("level.add_trigger", "Add Trigger Volume", "Level"),
        c("level.play", "Play Level", "Level").shortcut(Shortcut::new(Modifiers::NONE, KeyCode::F5)),
        c("level.stop", "Stop Playing", "Level")
            .shortcut(shifted(KeyCode::F5))
            .disabled("The level is not playing"),
        c("level.bake", "Bake Lighting", "Level"),
        c("level.validate", "Check Level for Problems", "Level"),
    ])
}

/// The line the screen reports the last command on.
#[derive(Component)]
struct Last;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            Node {
                margin: UiRect::all(Val::Px(24.0)),
                align_self: AlignSelf::FlexStart,
                ..default()
            },
            Pickable::IGNORE,
            children![(
                panel(),
                children![
                    heading("Level editor"),
                    (label("Last command: none"), Last),
                    label(format!(
                        "Press {} to search every command.",
                        Shortcut::new(Modifiers::PRIMARY | Modifiers::SHIFT, KeyCode::KeyP)
                            .label()
                    )),
                ],
            )],
        )],
    ));
}

/// What the game does with a chosen command: here, says its id.
fn on_run(run: On<RunCommand>, list: Res<CommandList>, mut last: Single<&mut Text, With<Last>>) {
    let title = list
        .0
        .iter()
        .find(|c| c.id == run.id)
        .map_or(run.id.clone(), PaletteCommand::label);
    last.0 = format!("Last command: {title}");
}

/// The game's own chord opens the palette.
fn open_palette(
    keys: Res<ButtonInput<KeyCode>>,
    open: Query<(), With<CommandPalette>>,
    mut commands: Commands,
) {
    let chord = Shortcut::new(Modifiers::PRIMARY | Modifiers::SHIFT, KeyCode::KeyP);
    if chord.just_pressed(&keys) && open.is_empty() {
        commands.spawn(command_palette());
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

/// `GOOEY_KEYS`: one step every `GOOEY_KEY_EVERY` frames. A key is pressed and then released as
/// the window's own keyboard input; typing sends one press with its text per character at once.
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keys: MessageWriter<KeyboardInput>,
    mut commands: Commands,
) {
    let (Some(c), Ok(window)) = (capture, window.single()) else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.step_every.max(4);
    let Some(step) = c.steps.get((since / every) as usize) else {
        return;
    };
    let state = match since % every {
        0 => ButtonState::Pressed,
        n if n == every / 2 => ButtonState::Released,
        _ => return,
    };
    match step {
        Step::Open => {
            if state == ButtonState::Pressed {
                commands.spawn(command_palette());
            }
        }
        Step::Type(text) => {
            for character in text.chars() {
                let s = character.to_string();
                keys.write(KeyboardInput {
                    key_code: KeyCode::KeyA,
                    logical_key: Key::Character(s.as_str().into()),
                    state,
                    text: (state == ButtonState::Pressed).then(|| s.as_str().into()),
                    repeat: false,
                    window,
                });
            }
        }
        Step::Key(key_code, logical_key) => {
            keys.write(KeyboardInput {
                key_code: *key_code,
                logical_key: logical_key.clone(),
                state,
                text: None,
                repeat: false,
                window,
            });
        }
    }
}
