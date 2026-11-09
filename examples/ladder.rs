//! The surface ladder and the design tokens: five steps of `GooeySkin::surfaces`, each `surface(n)`
//! nested in the one below on the skin's own spacing and corner tokens, each labelled with its CIE L*
//! and the step it makes over the one under it. Step the Theme row (the innermost card) and the
//! ladder changes with the look in the same frame; every adjacent step is at least L* 2 apart
//! (`GooeySkin::problems` checks it), in every preset.
//!
//! Run `cargo run --release --example ladder`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_FIXED_CLOCK=1` makes every frame 1/30 s of game time (a deterministic recording; the
//! frame time then reads 33.333). `GOOEY_SHOT` (default `ladder.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS=down,right,…` presses one key every `GOOEY_KEY_EVERY` (default 10) frames from frame
//! 30 as real keyboard input (`up`, `down`, `left`, `right`, `enter`).

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::AutoFocus;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_gooey::*;

/// The steps drawn, the ground and four cards.
const LEVELS: u8 = 5;

/// Which step of the ladder a label names.
#[derive(Component)]
struct StepLabel(u8);

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
    key_every: u32,
    keys: Vec<KeyCode>,
}

/// The frame `GOOEY_KEYS` starts pressing at, after the layout and autofocus have settled.
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
        title: "bevy_gooey ladder".into(),
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
    .add_systems(Update, (label_steps, capture, stage_keys.after(capture)));
    if let Some(at) = env("GOOEY_FRAMES") {
        // GOOEY_FIXED_CLOCK=1: every frame is 1/30 s of game time, whatever the host's load, so a
        // recording of an easing interface is the same each time (the frame time then reads 33.333).
        if std::env::var_os("GOOEY_FIXED_CLOCK").is_some() {
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_micros(33_333),
            ));
        }
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "ladder.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            key_every: env("GOOEY_KEY_EVERY").unwrap_or(10),
            keys: std::env::var("GOOEY_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|k| match k.trim() {
                    "up" => Some(KeyCode::ArrowUp),
                    "down" => Some(KeyCode::ArrowDown),
                    "left" => Some(KeyCode::ArrowLeft),
                    "right" => Some(KeyCode::ArrowRight),
                    "enter" => Some(KeyCode::Enter),
                    _ => None,
                })
                .collect(),
        });
    }
    app.run();
}

/// The ground fills the window; four cards nest in it, `surface(1)` outermost, each padded and
/// rounded from the skin's tokens. The Theme row stands in the innermost card.
fn setup(mut commands: Commands, skin: Res<GooeySkin>) {
    commands.spawn(Camera2d);
    let ground = commands
        .spawn((
            Role::Surface(0),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();
    let mut inside: Option<Entity> = None;
    for level in (1..LEVELS).rev() {
        let card = commands
            .spawn((
                Role::Surface(level),
                Node {
                    flex_direction: FlexDirection::Column,
                    width: if level == 1 {
                        Val::Px(640.0)
                    } else {
                        Val::Auto
                    },
                    padding: UiRect::all(Val::Px(skin.space(3))),
                    row_gap: Val::Px(skin.space(2)),
                    border_radius: BorderRadius::all(Val::Px(skin.corner(1))),
                    ..default()
                },
            ))
            .id();
        if level == 1 {
            let title = commands.spawn(heading("Surface ladder")).id();
            commands.entity(card).add_child(title);
        }
        let name = commands.spawn((label(""), StepLabel(level))).id();
        commands.entity(card).add_child(name);
        match inside {
            Some(inner) => {
                commands.entity(card).add_child(inner);
            }
            // The innermost card holds the one control: the Theme row, with the focus.
            None => {
                let theme = commands.spawn((skin_preset("Theme"), AutoFocus)).id();
                commands.entity(card).add_child(theme);
            }
        }
        inside = Some(card);
    }
    if let Some(outer) = inside {
        commands.entity(ground).add_child(outer);
    }
    commands.spawn((
        prompts(),
        children![
            prompt("Select", Prompt::SELECT),
            prompt("Back", Prompt::BACK)
        ],
    ));
}

/// Each card says which step it is, its L* and how far it stands above the step it sits in; redone
/// when the skin changes, so stepping the Theme row relabels the ladder.
fn label_steps(skin: Res<GooeySkin>, mut labels: Query<(&StepLabel, &mut Text)>) {
    if !skin.is_changed() {
        return;
    }
    let lightness = |level: u8| {
        let c = Srgba::from(skin.surface_at(level));
        cie_lightness([c.red, c.green, c.blue])
    };
    for (StepLabel(level), mut text) in &mut labels {
        let l = lightness(*level);
        let step = l - lightness(level - 1);
        text.0 = format!(
            "Surface {level}   L* {l:.1}   {step:+.1} over {}",
            level - 1
        );
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

/// `GOOEY_KEYS`: one key every `GOOEY_KEY_EVERY` frames, pressed and then released as the window's
/// own keyboard input, so Bevy's input dispatch and the crate's navigation see it.
fn stage_keys(
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
    let every = c.key_every.max(2);
    let state = match since % every {
        0 => ButtonState::Pressed,
        n if n == every / 2 => ButtonState::Released,
        _ => return,
    };
    if let Some(&key_code) = c.keys.get((since / every) as usize) {
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
