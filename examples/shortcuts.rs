//! Shortcuts: a keymap screen. Each row is a `binding` that captures a chord (Enter on it, then
//! hold Ctrl and Shift and tap a key); the line under the rows names the commands whose chords
//! clash (`model::shortcut_conflicts`); pressing a bound chord runs its command (`Shortcut::just_pressed`);
//! and the Save button's tooltip shows the Save row's chord on a keycap, so rebinding the row
//! changes the hint.
//!
//! Run `cargo run --release --example shortcuts`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `shortcuts.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10
//! frames later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from
//! `GOOEY_SHOT_FROM` (default 30) there as `f_%04d.png` (the directory must exist).
//! `GOOEY_SKIN=parchment`, `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and
//! `GOOEY_SIZE=<w>x<h>` set the start. `GOOEY_PLATFORM=mac` spells every chord the way a mac does
//! (the capture host is not a mac) and `GOOEY_GLYPHS=words` spells them in ASCII instead of Apple's
//! symbols (`cmd+s` and `shift+cmd+s` are staged keys too). `GOOEY_KEYS=enter,ctrl+shift+k,down,…`
//! stages one step every
//! `GOOEY_KEY_EVERY` (default 14) frames from frame 30: `up`, `down`, `enter`, `escape`, or a chord
//! (`ctrl`, `shift`, `alt` or `meta` joined by `+` to a letter) as real keyboard input, modifiers
//! first.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::AutoFocus;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_gooey::*;

/// The command a keymap row binds.
#[derive(Component)]
struct Command(&'static str);

/// The line that names the commands whose chords clash.
#[derive(Component)]
struct ConflictLine;

/// The line that names the command a chord just ran.
#[derive(Component)]
struct RanLine;

/// The Save button, whose tooltip follows the Save row.
#[derive(Component)]
struct SaveButton;

/// A staged step of `GOOEY_KEYS`: the keys to hold, the last of which is the one tapped.
#[derive(Clone)]
struct Step(Vec<KeyCode>);

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

/// `ctrl+shift+k` as the keys to press: the modifiers, then the letter.
fn parse_step(text: &str) -> Option<Step> {
    let mut keys = Vec::new();
    for part in text.trim().split('+') {
        keys.push(match part {
            "ctrl" => KeyCode::ControlLeft,
            "shift" => KeyCode::ShiftLeft,
            "alt" => KeyCode::AltLeft,
            "meta" => KeyCode::SuperLeft,
            "up" => KeyCode::ArrowUp,
            "down" => KeyCode::ArrowDown,
            "enter" => KeyCode::Enter,
            "escape" => KeyCode::Escape,
            "j" => KeyCode::KeyJ,
            "o" => KeyCode::KeyO,
            "k" => KeyCode::KeyK,
            "p" => KeyCode::KeyP,
            "s" => KeyCode::KeyS,
            _ => return None,
        });
    }
    Some(Step(keys))
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
        title: "bevy_gooey shortcuts".into(),
        ..default()
    };
    if let Some((w, h)) = std::env::var("GOOEY_SIZE")
        .ok()
        .and_then(|s| s.split_once('x').map(|(w, h)| (w.parse(), h.parse())))
        .and_then(|(w, h)| Some((w.ok()?, h.ok()?)))
    {
        window.resolution = WindowResolution::new(w, h).with_scale_factor_override(1.0);
    }
    // A mac's spelling on another host, for the capture: the defect is Apple's notation and the
    // fix is the host's own spelling of it.
    let mut bindings = BindingSettings::default();
    if std::env::var("GOOEY_PLATFORM").as_deref() == Ok("mac") {
        bindings.platform = Some(Platform::Mac);
    }
    if std::env::var("GOOEY_GLYPHS").as_deref() == Ok("words") {
        bindings.chord_glyphs = ChordGlyphs::Words;
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
    .insert_resource(bindings)
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            follow_background,
            show_conflicts,
            follow_save_row,
            run_commands,
            capture,
            stage.after(capture),
        ),
    );
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "shortcuts.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            step_every: env("GOOEY_KEY_EVERY").unwrap_or(14),
            steps: std::env::var("GOOEY_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter_map(parse_step)
                .collect(),
        });
    }
    app.run();
}

/// A keymap row: the command's name and the chord it is on.
fn row(name: &'static str, chord: Shortcut) -> impl Bundle {
    (binding(name, Binding::Chord(chord)), Command(name))
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let save = Shortcut::primary(KeyCode::KeyS);
    let save_all = Shortcut::new(Modifiers::PRIMARY | Modifiers::SHIFT, KeyCode::KeyS);
    let palette = Shortcut::primary(KeyCode::KeyP);
    // At the left of the window, so the Save button's tooltip, which opens beside it, has room.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(110.0),
            left: Val::Px(90.0),
            ..default()
        },
        children![(
            panel(),
            children![
                heading("Keymap"),
                (row("Save", save), AutoFocus),
                row("Save all", save_all),
                row("Palette", palette),
                (label("No conflicts"), ConflictLine),
                (label("Press a chord to run it"), RanLine),
                (
                    button("Save"),
                    SaveButton,
                    Tooltip::new("Write to disk").with_shortcut(save),
                ),
            ]
        )],
    ));
}

/// Names the commands whose chords clash, whenever a row's chord changes.
fn show_conflicts(
    changed: Query<(), (With<Command>, Changed<BindingValue>)>,
    rows: Query<(&Command, &BindingValue)>,
    mut line: Single<&mut Text, With<ConflictLine>>,
) {
    if changed.is_empty() {
        return;
    }
    let mut keymap: Vec<(&str, Binding)> = rows.iter().map(|(c, v)| (c.0, v.0)).collect();
    keymap.sort_by_key(|(name, _)| *name);
    let clashes = model::shortcut_conflicts(&keymap);
    line.0 = match clashes.first() {
        None => "No conflicts".into(),
        Some((a, b)) => format!("Conflict: {a} and {b}"),
    };
}

/// The Save button's tooltip shows the chord the Save row holds.
fn follow_save_row(
    rows: Query<(&Command, &BindingValue), Changed<BindingValue>>,
    mut tips: Query<&mut Tooltip, With<SaveButton>>,
) {
    for (command, value) in &rows {
        if let (Binding::Chord(chord), "Save") = (value.0, command.0) {
            for mut tip in &mut tips {
                tip.shortcut = Some(chord);
            }
        }
    }
}

/// Runs the command whose chord went down. Nothing runs while a row waits for a chord, or in the
/// frame after it ended (the chord that was just bound is not also a press of it).
fn run_commands(
    keys: Res<ButtonInput<KeyCode>>,
    rows: Query<(&Command, &BindingValue)>,
    waiting: Query<(), With<Capturing>>,
    mut was_waiting: Local<bool>,
    mut line: Single<&mut Text, With<RanLine>>,
) {
    let idle = waiting.is_empty() && !*was_waiting;
    *was_waiting = !waiting.is_empty();
    if !idle {
        return;
    }
    for (command, value) in &rows {
        if let Binding::Chord(chord) = value.0
            && chord.just_pressed(&keys)
        {
            line.0 = format!("Ran: {} ({})", command.0, chord.label());
            return;
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

/// `GOOEY_KEYS`: one step every `GOOEY_KEY_EVERY` frames. A step holds its modifiers from its
/// first frame, taps its last key a quarter of the way in, and lets everything go at half way,
/// as the window's own keyboard input.
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keys: MessageWriter<KeyboardInput>,
) {
    let (Some(c), Ok(window)) = (capture, window.single()) else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.step_every.max(8);
    let Some(step) = c.steps.get((since / every) as usize) else {
        return;
    };
    let Some((&last, held)) = step.0.split_last() else {
        return;
    };
    let phase = since % every;
    let mut send = |key_code, state| {
        keys.write(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
    };
    if phase == 0 {
        for &key_code in held {
            send(key_code, ButtonState::Pressed);
        }
    } else if phase == every / 4 {
        send(last, ButtonState::Pressed);
    } else if phase == every / 2 {
        send(last, ButtonState::Released);
        for &key_code in held {
            send(key_code, ButtonState::Released);
        }
    }
}
