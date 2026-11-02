//! The state roles: the colours a host composes its own controls from, each drawn by its `Role`
//! on a plain node: a row under the pointer (`Hovered`), the same row with the pointer down
//! (`Pressed`, sunk below it), the card the scrim leaves lit (`Card`), the mark of a thing not
//! judged yet (`Unchecked`), the edge of a preview box (`PreviewEdge`), and a world glyph and its
//! ground stepped back together (`Held`, `HeldGround`) beside the same pair at rest. Step the Theme
//! row and every state changes with the look in the same frame; `problems()` checks pressed against
//! hovered by a CIE L* step.
//!
//! Run `cargo run --release --example states`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `states.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS=right,…` presses one key every `GOOEY_KEY_EVERY` (default 10) frames from frame 30
//! as real keyboard input (`up`, `down`, `left`, `right`, `enter`).

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::AutoFocus;
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
        title: "bevy_gooey states".into(),
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
    .add_systems(Update, (capture, stage_keys.after(capture)));
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "states.png".into()),
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

/// The ground fills the window; one panel holds the swatches and the Theme row.
fn setup(mut commands: Commands, skin: Res<GooeySkin>) {
    commands.spawn(Camera2d);
    let gap = skin.space(3);
    let corner = skin.corner(1);
    let swatch = |role: Role, border: f32| {
        (
            role,
            Node {
                width: Val::Px(120.0),
                height: Val::Px(72.0),
                border: UiRect::all(Val::Px(border)),
                border_radius: BorderRadius::all(Val::Px(corner)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        )
    };
    let cell = |role: Role, border: f32, name: &'static str| {
        (
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(skin.space(1)),
                ..default()
            },
            children![swatch(role, border), label(name)],
        )
    };
    // A world glyph on its ground: the pair at rest and the pair stepped back by one fade.
    let glyph = |ground: Role, ink: Color| {
        (
            swatch(ground, 0.0),
            children![(
                Node {
                    width: Val::Px(36.0),
                    height: Val::Px(36.0),
                    border_radius: BorderRadius::all(Val::Px(corner)),
                    ..default()
                },
                BackgroundColor(ink),
            )],
        )
    };
    let held = (
        Node {
            column_gap: Val::Px(gap),
            ..default()
        },
        children![
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(skin.space(1)),
                    ..default()
                },
                children![glyph(Role::Surface(1), skin.text), label("World")],
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(skin.space(1)),
                    ..default()
                },
                children![glyph(Role::HeldGround, skin.held()), label("Held")],
            ),
        ],
    );
    commands.spawn((
        Role::Surface(0),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            panel(),
            Node {
                width: Val::Px(700.0),
                ..default()
            },
            children![
                heading("State roles"),
                (
                    Node {
                        column_gap: Val::Px(gap),
                        row_gap: Val::Px(gap),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    },
                    children![
                        cell(Role::Hovered, 0.0, "Hovered"),
                        cell(Role::Pressed, 0.0, "Pressed"),
                        cell(Role::Card, 0.0, "Card"),
                        cell(Role::Unchecked, 0.0, "Unchecked"),
                        cell(Role::PreviewEdge, 3.0, "Preview edge"),
                    ],
                ),
                held,
                (skin_preset("Theme"), AutoFocus),
            ],
        )],
    ));
    commands.spawn((
        prompts(),
        children![
            prompt("Select", Prompt::SELECT),
            prompt("Back", Prompt::BACK)
        ],
    ));
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
