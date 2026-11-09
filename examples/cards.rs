//! A project browser: the screen's one main action as a `primary_button` beside a plain one, and
//! a `card_grid` of maps, each a picture (here a flat swatch; a game loads a thumbnail) above a name
//! and two quiet lines, in a row that wraps to fill the panel. The grid is one tab stop the arrows
//! walk; Enter chooses a card, which is lit in the accent; "Ruins" is dimmed and cannot be chosen.
//!
//! Run `cargo run --release --example cards`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `cards.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS=tab,…` presses one key every `GOOEY_KEY_EVERY` (default 10) frames from frame 30
//! as real keyboard input (`tab`, `up`, `down`, `left`, `right`, `enter`).
//! `GOOEY_ART=1` draws each card from a `GooeyFrames::card` (a rounded plate made here) instead of a
//! flat fill, so the demo shows what art does to a card.

use bevy::asset::RenderAssetUsages;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
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

/// The frame `GOOEY_KEYS` starts pressing at, after the layout has settled.
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
        title: "bevy_gooey cards".into(),
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
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "cards.png".into()),
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
                    "tab" => Some(KeyCode::Tab),
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

/// The side of a card's art, image pixels, and the size of its corners.
const CARD_ART_SIDE: u32 = 32;
const CARD_ART_SLICE: u32 = 9;

/// A rounded plate for a card's edge in light neutrals (the skin tints it by the card's state): a
/// bright rim, a darker groove inside it and a soft centre. Drawn here, so the example needs no art
/// files.
fn plate() -> Image {
    let side = CARD_ART_SIDE as usize;
    let radius = CARD_ART_SLICE as f32;
    let mut pixels = vec![0u8; side * side * 4];
    for y in 0..side {
        for x in 0..side {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let (dx, dy) = (
                (fx - side as f32 / 2.0).abs() - (side as f32 / 2.0 - radius),
                (fy - side as f32 / 2.0).abs() - (side as f32 / 2.0 - radius),
            );
            let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() + dx.max(dy).min(0.0);
            let depth = radius - outside;
            let (value, alpha) = if depth < 0.0 {
                (0.0, 0.0)
            } else if depth < 1.2 {
                (1.0, 1.0)
            } else if depth < 2.4 {
                (0.86, 1.0)
            } else if depth < 3.4 {
                (0.46, 1.0)
            } else {
                (0.96, 1.0)
            };
            let byte = (value * 255.0) as u8;
            let at = (y * side + x) * 4;
            pixels[at..at + 4].copy_from_slice(&[byte, byte, byte, (alpha * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: CARD_ART_SIDE,
            height: CARD_ART_SIDE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// A flat swatch standing in for a map's thumbnail.
fn swatch(images: &mut Assets<Image>, rgb: [u8; 3]) -> Handle<Image> {
    images.add(Image::new_fill(
        Extent3d {
            width: 16,
            height: 9,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[rgb[0], rgb[1], rgb[2], 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

/// A panel of the two actions over the maps.
fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut frames: ResMut<GooeyFrames>,
) {
    commands.spawn(Camera2d);
    if std::env::var_os("GOOEY_ART").is_some() {
        frames.card = Some(Frame::new(images.add(plate()), CARD_ART_SLICE as f32));
    }
    let maps = [
        Card::new("Harbour")
            .image(swatch(&mut images, [52, 96, 128]))
            .line("10 x 10 m, 4 m tall")
            .line("Opened 2 hours ago"),
        Card::new("Keep")
            .image(swatch(&mut images, [120, 96, 64]))
            .line("24 x 18 m")
            .line("13 pieces"),
        Card::new("Orchard")
            .image(swatch(&mut images, [72, 120, 64]))
            .line("40 x 40 m")
            .line("Opened yesterday"),
        Card::new("Ruins").line("Missing assets").disabled(),
        Card::new("Mill").line("16 x 12 m").line("7 pieces"),
    ];
    // The panel is 80% of the window wide, so the grid has a width to wrap in; `panel()` sizes
    // itself to its content otherwise.
    let maps_panel = commands
        .spawn((
            panel(),
            children![
                heading("Maps"),
                (
                    Node {
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                    children![primary_button("New map"), button("Open...")],
                ),
                (card_grid(maps), CardSelection(Some(1))),
            ],
        ))
        .insert(Node {
            width: Val::Percent(80.0),
            ..default()
        })
        .id();
    commands.spawn(screen()).add_child(maps_panel);
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
