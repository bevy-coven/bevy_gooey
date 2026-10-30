//! Every widget on one screen.
//!
//! - Arrows or the D-pad move focus; Enter, Space or the pad's South button activate.
//! - **Options** switches between the dark and the parchment skin; **Shift+T** toggles a 1.5x text
//!   scale; **Quit** opens a modal dialog that holds focus (Stay first; Tab, arrows and the D-pad
//!   stay inside it; closing it puts focus back on Quit); focusing or hovering **Options** shows
//!   its tooltip beside it (`GOOEY_KEYS=down` stages it).
//! - The health meter drains and refills (`GOOEY_HEALTH=<value>` holds it at that value instead).
//!   Each `UiFeedback` is logged where a game would play a sound.
//!
//! Capture mode, for a still and a frame time: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `gallery.png`), prints `gooey: frames <n> mean_ms <x>` (the mean frame time
//! over frames 20..n) and exits 10 frames later. `GOOEY_SKIN=parchment` starts in parchment;
//! `GOOEY_TEXT_SCALE=<f>` starts at that text scale; `GOOEY_SIZE=<w>x<h>` sets the window size.
//! `GOOEY_KEYS=down,down,enter,…` stages a state before the shot: from frame 30 it presses one key
//! every 10 frames, as real keyboard input (`up`, `down`, `left`, `right`, `enter`, `tab`).

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{AutoFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_gooey::*;

/// The health meter.
#[derive(Component)]
struct Health;

/// `GOOEY_HEALTH`: the value the meter holds instead of draining, for a still.
#[derive(Resource)]
struct HeldHealth(Option<f32>);

/// The open Quit dialog.
#[derive(Component)]
struct QuitDialog;

/// Capture mode's state, present only when `GOOEY_FRAMES` is set.
#[derive(Resource)]
struct Capture {
    at: u32,
    shot: String,
    frame: u32,
    total_ms: f64,
    counted: u32,
    /// `GOOEY_KEYS`, pressed one every [`KEY_EVERY`] frames from [`KEY_FROM`].
    keys: Vec<KeyCode>,
}

/// The frame `GOOEY_KEYS` starts pressing at, after the layout and autofocus have settled.
const KEY_FROM: u32 = 30;
/// Frames between two staged key presses; the release comes halfway.
const KEY_EVERY: u32 = 10;

/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;

fn main() {
    let mut skin = match std::env::var("GOOEY_SKIN").as_deref() {
        Ok("parchment") => GooeySkin::parchment(),
        _ => GooeySkin::dark(),
    };
    if let Some(scale) = std::env::var("GOOEY_TEXT_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        skin.text_scale = scale;
    }
    let mut window = Window {
        title: "bevy_gooey gallery".into(),
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
    .insert_resource(ClearColor(skin.background))
    .insert_resource(skin)
    .insert_resource(InputFocusVisible(true))
    .insert_resource(HeldHealth(
        std::env::var("GOOEY_HEALTH")
            .ok()
            .and_then(|s| s.parse().ok()),
    ))
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            drain_health,
            toggle_text_scale,
            follow_background,
            log_feedback,
            capture,
            stage_keys.after(capture),
        ),
    );
    if let Some(at) = std::env::var("GOOEY_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "gallery.png".into()),
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            keys: std::env::var("GOOEY_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|k| match k.trim() {
                    "up" => Some(KeyCode::ArrowUp),
                    "down" => Some(KeyCode::ArrowDown),
                    "left" => Some(KeyCode::ArrowLeft),
                    "right" => Some(KeyCode::ArrowRight),
                    "enter" => Some(KeyCode::Enter),
                    "tab" => Some(KeyCode::Tab),
                    _ => None,
                })
                .collect(),
        });
    }
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            panel(),
            children![
                heading("bevy_gooey"),
                (
                    menu(),
                    children![
                        (
                            button("Play"),
                            AutoFocus,
                            observe(|_: On<Activate>| info!("gooey: play")),
                        ),
                        (
                            button("Options"),
                            Tooltip("Sound, controls and display".into()),
                            observe(switch_skin),
                        ),
                        (button("Quit"), observe(open_quit_dialog)),
                    ]
                ),
                label("Health"),
                (meter(100.0, 100.0), Health),
            ]
        )],
    ));
}

/// Options: dark to parchment and back, keeping the player's text scale.
fn switch_skin(_: On<Activate>, mut skin: ResMut<GooeySkin>) {
    let next = if skin.background == GooeySkin::parchment().background {
        GooeySkin::dark()
    } else {
        GooeySkin::parchment()
    };
    *skin = GooeySkin {
        text_scale: skin.text_scale,
        ..next
    };
}

fn open_quit_dialog(_: On<Activate>, open: Query<(), With<QuitDialog>>, mut commands: Commands) {
    if !open.is_empty() {
        return;
    }
    commands.spawn((
        dialog(),
        QuitDialog,
        children![(
            panel(),
            children![
                label("Leave the game?"),
                (
                    menu(),
                    children![
                        (
                            button("Leave"),
                            observe(|_: On<Activate>, mut exit: MessageWriter<AppExit>| {
                                exit.write(AppExit::Success);
                            }),
                        ),
                        (button("Stay"), AutoFocus, observe(close_quit_dialog)),
                    ]
                ),
            ]
        )],
    ));
}

/// Stay: closing the dialog is all it takes; focus goes back to Quit on its own.
fn close_quit_dialog(
    _: On<Activate>,
    dialogs: Query<Entity, With<QuitDialog>>,
    mut commands: Commands,
) {
    for dialog in &dialogs {
        commands.entity(dialog).despawn();
    }
}

/// The meter drains by 10 a second and refills when empty, or holds `GOOEY_HEALTH`.
fn drain_health(
    time: Res<Time>,
    held: Res<HeldHealth>,
    mut meters: Query<&mut Meter, With<Health>>,
) {
    for mut meter in &mut meters {
        if let Some(value) = held.0 {
            let max = meter.max;
            meter.set_if_neq(Meter { value, max });
            continue;
        }
        meter.value -= 10.0 * time.delta_secs();
        if meter.value <= 0.0 {
            meter.value = meter.max;
        }
    }
}

/// Shift+T: the player's text-size option, 1.0 or 1.5.
fn toggle_text_scale(keys: Res<ButtonInput<KeyCode>>, mut skin: ResMut<GooeySkin>) {
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if shift && keys.just_pressed(KeyCode::KeyT) {
        skin.text_scale = if skin.text_scale > 1.0 { 1.0 } else { 1.5 };
    }
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

/// Where a game would play a sound.
fn log_feedback(mut feedback: MessageReader<UiFeedback>) {
    for f in feedback.read() {
        info!("gooey: feedback {:?}", f.kind);
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
    if c.frame == c.at + 10 {
        exit.write(AppExit::Success);
    }
}

/// `GOOEY_KEYS`: one key every [`KEY_EVERY`] frames, pressed and then released as the window's
/// own keyboard input, so Bevy's input dispatch, Tab navigation and buttons all see it.
fn stage_keys(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut input: MessageWriter<KeyboardInput>,
) {
    let (Some(c), Ok(window)) = (capture, window.single()) else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let state = match since % KEY_EVERY {
        0 => ButtonState::Pressed,
        n if n == KEY_EVERY / 2 => ButtonState::Released,
        _ => return,
    };
    if let Some(&key_code) = c.keys.get((since / KEY_EVERY) as usize) {
        input.write(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
    }
}
