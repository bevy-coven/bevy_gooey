//! A hold-to-confirm button guarding a delete, and the player's setting that turns the hold into a
//! tap.
//!
//! - **Hold to delete** is `hold_button("Hold to delete", 1.2)`: hold Enter, Space or the pad's
//!   South (or the primary mouse button over it) and the bar under the label fills; at the end one
//!   `HoldConfirmed` arrives and the line under it counts the deletions. Let go early and the bar
//!   unwinds four times as fast; a new press resumes from where it stands.
//! - **Hold to confirm** is a plain `choice` that writes `HoldSettings::duration_scale`: Normal
//!   (1.0), Short (0.4) or Tap (0): a tap confirms when the key is let go, XAG 107's "a single
//!   input press instead of an input hold".
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `hold.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later, on a clock that moves 1/30 s a frame so a hold takes the same frames every run;
//! `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png`. `GOOEY_SKIN=parchment`, `GOOEY_TEXT_SCALE=<f>` and
//! `GOOEY_SIZE=<w>x<h>` set the start. `GOOEY_KEYS=up,hold:20,…` stages one input after another from
//! frame 30 as real keyboard input: `up`, `down`, `left`, `right`, `enter` (a tap), and `hold:<n>`
//! (Enter held for `n` frames), each followed by `GOOEY_KEY_EVERY` (default 10) idle frames.

use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::AutoFocus;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::time::TimeUpdateStrategy;
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
    /// `(frame, key, state)` of every staged key event.
    timeline: Vec<(u32, KeyCode, ButtonState)>,
}

/// The line under the button.
#[derive(Component)]
struct Status;

/// How many times the save has been deleted.
#[derive(Resource, Default)]
struct Deleted(u32);

/// The frame `GOOEY_KEYS` starts at, after the layout and autofocus have settled.
const KEY_FROM: u32 = 30;
/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;
/// The hold's scale for each option of the setting row.
const SCALES: [f32; 3] = [1.0, 0.4, 0.0];

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

/// `GOOEY_KEYS` as a timeline: a key is pressed at its slot's start and let go half a slot later; a
/// `hold:<n>` is pressed and let go `n` frames later; each slot is followed by `every` frames.
fn timeline(keys: &str, every: u32) -> Vec<(u32, KeyCode, ButtonState)> {
    let every = every.max(2);
    let mut at = KEY_FROM;
    let mut out = Vec::new();
    for token in keys.split(',').map(str::trim) {
        let (key_code, frames) = match token {
            "up" => (KeyCode::ArrowUp, every / 2),
            "down" => (KeyCode::ArrowDown, every / 2),
            "left" => (KeyCode::ArrowLeft, every / 2),
            "right" => (KeyCode::ArrowRight, every / 2),
            "enter" => (KeyCode::Enter, every / 2),
            other => match other.strip_prefix("hold:").and_then(|n| n.parse().ok()) {
                Some(n) => (KeyCode::Enter, n),
                None => continue,
            },
        };
        out.push((at, key_code, ButtonState::Pressed));
        out.push((at + frames, key_code, ButtonState::Released));
        at += frames + every;
    }
    out
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
        title: "bevy_gooey hold".into(),
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
    .init_resource::<Deleted>()
    .insert_resource(ClearColor(skin.background))
    .insert_resource(skin)
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (follow_background, capture, stage_keys.after(capture)),
    );
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_micros(
            33_333,
        )))
        .insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "hold.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            timeline: timeline(
                &std::env::var("GOOEY_KEYS").unwrap_or_default(),
                env("GOOEY_KEY_EVERY").unwrap_or(10),
            ),
        });
    }
    app.run();
}

/// The screen: one panel near the top with the save's name, the setting row, the hold button, a
/// plain button beside the safe answer, and the line that says what happened.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            Node {
                height: Val::Percent(100.0),
                align_items: AlignItems::FlexStart,
                padding: UiRect::top(Val::Vh(4.0)),
                ..default()
            },
            children![(
                panel(),
                children![
                    heading("Save slot 2"),
                    label("Chapter 4, 12 h 40 min"),
                    (
                        choice("Hold to confirm", ["Normal", "Short", "Tap"], 0),
                        observe(|change: On<ValueChange<usize>>, mut commands: Commands| {
                            let scale = SCALES[change.value.min(SCALES.len() - 1)];
                            commands.insert_resource(HoldSettings {
                                duration_scale: scale,
                            });
                        }),
                    ),
                    (
                        hold_button("Hold to delete", 1.2),
                        AutoFocus,
                        observe(
                            |_: On<HoldConfirmed>,
                             mut deleted: ResMut<Deleted>,
                             mut status: Single<&mut Text, With<Status>>| {
                                deleted.0 += 1;
                                status.0 = format!("Deleted: {}", deleted.0);
                            },
                        ),
                    ),
                    button("Keep the save"),
                    (label("Nothing deleted yet"), Status),
                ]
            )],
        )],
    ));
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

/// `GOOEY_KEYS`: the timeline's key events as the window's own keyboard input.
fn stage_keys(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keys: MessageWriter<KeyboardInput>,
) {
    let (Some(c), Ok(window)) = (capture, window.single()) else {
        return;
    };
    for &(_, key_code, state) in c.timeline.iter().filter(|(at, ..)| *at == c.frame) {
        keys.write(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
    }
}
