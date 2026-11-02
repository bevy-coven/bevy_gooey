//! The shortcut overlay: a small editor's commands, each with a chord. Hold **Alt** for a third of a
//! second and every command's chord shows on its button; let go and they go. Tab and Enter work as
//! in any gooey screen, and the ring is never covered.
//!
//! Capture mode, as the other examples: `GOOEY_FRAMES`, `GOOEY_SHOT`, `GOOEY_SHOT_DIR`,
//! `GOOEY_SHOT_FROM`, `GOOEY_SHOT_EVERY`, `GOOEY_SKIN`, `GOOEY_TEXT_SCALE`, `GOOEY_SIZE` and
//! `GOOEY_KEYS` (steps separated by `;`: `tab`, `enter`, `alt` (hold Alt down), `unalt` (let it go),
//! `shift` and `unshift` (hold or release Shift: then Alt+Shift is not the hold)), one every
//! `GOOEY_KEY_EVERY` frames.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_gooey::*;

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

/// A staged step of `GOOEY_KEYS`.
#[derive(Clone, Debug)]
enum Step {
    /// A key pressed and let go.
    Tap(KeyCode, Key),
    /// A key pressed and kept down.
    Down(KeyCode),
    /// A key let go.
    Up(KeyCode),
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
        title: "bevy_gooey shortcut badges".into(),
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
    .add_systems(Update, (follow_background, capture))
    // Staged keys are written before `PreUpdate`, as a real keyboard's are.
    .add_systems(First, stage);
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "badges.png".into()),
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
    text.split(';')
        .filter_map(|word| {
            Some(match word.trim() {
                "tab" => Step::Tap(KeyCode::Tab, Key::Tab),
                "enter" => Step::Tap(KeyCode::Enter, Key::Enter),
                "alt" => Step::Down(KeyCode::AltLeft),
                "unalt" => Step::Up(KeyCode::AltLeft),
                "shift" => Step::Down(KeyCode::ShiftLeft),
                "unshift" => Step::Up(KeyCode::ShiftLeft),
                _ => return None,
            })
        })
        .collect()
}

/// One command: a button with the chord the editor binds it to. The chord is a component on the
/// widget, which is all the overlay needs; the editor runs the chord itself.
fn command(name: &'static str, chord: Shortcut) -> impl Bundle {
    (
        button(name),
        chord,
        observe(
            move |_: On<Activate>, mut ran: Single<&mut Text, With<Ran>>| {
                ran.0 = format!("Ran: {name}");
            },
        ),
    )
}

/// The line that says which command last ran.
#[derive(Component)]
struct Ran;

/// A level editor's command panel: a column of commands as wide as a toolbar's menu (a chord sits
/// at the right of its button and a short label stays clear of it, at any text size). Redo is disabled: its chord does
/// nothing, so it shows no badge, and Help has no chord.
fn setup(mut commands: Commands, skin: Res<GooeySkin>) {
    commands.spawn(Camera2d);
    let primary = Shortcut::primary;
    let card = commands
        .spawn((
            panel(),
            children![
                heading("Level editor"),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        width: Val::Px(400.0 * skin.text_scale),
                        ..default()
                    },
                    children![
                        command("New", primary(KeyCode::KeyN)),
                        command("Open", primary(KeyCode::KeyO)),
                        command("Save", primary(KeyCode::KeyS)),
                        command(
                            "Save all",
                            Shortcut::new(Modifiers::PRIMARY | Modifiers::SHIFT, KeyCode::KeyS)
                        ),
                        command("Undo", primary(KeyCode::KeyZ)),
                        (
                            command(
                                "Redo",
                                Shortcut::new(Modifiers::PRIMARY | Modifiers::SHIFT, KeyCode::KeyZ)
                            ),
                            Disabled::new("Nothing to redo"),
                        ),
                        button("Help"),
                    ],
                ),
                (label("Hold Alt to see the chords."), Ran),
            ],
        ))
        .id();
    commands
        .spawn(screen())
        .insert(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .add_child(card);
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

/// `GOOEY_KEYS`: real keyboard input, one step every `GOOEY_KEY_EVERY` frames.
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
    let phase = since % every;
    let mut key = |key_code: KeyCode, logical_key: Key, state: ButtonState| {
        keys.write(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: None,
            repeat: false,
            window,
        });
    };
    match step {
        Step::Tap(key_code, logical) => match phase {
            0 => key(*key_code, logical.clone(), ButtonState::Pressed),
            n if n == every / 2 => key(*key_code, logical.clone(), ButtonState::Released),
            _ => {}
        },
        Step::Down(key_code) if phase == 0 => key(
            *key_code,
            Key::Unidentified(NativeKey::Unidentified),
            ButtonState::Pressed,
        ),
        Step::Up(key_code) if phase == 0 => key(
            *key_code,
            Key::Unidentified(NativeKey::Unidentified),
            ButtonState::Released,
        ),
        _ => {}
    }
}
