//! A weapon wheel with no wiring: `radial(items, selected)` is the whole widget.
//!
//! - A stick (either one) points at the item it faces and South (Enter, a click) chooses it; the
//!   arrows and the D-pad step round the ring; the pointer over a wedge points at it. The lit item
//!   is named in the middle, where the thumb is. The one observer in this file hears the final
//!   `ValueChange<usize>` and says what is equipped.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `radial.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png`. `GOOEY_SKIN=parchment`, `GOOEY_TEXT_SCALE=<f>` and
//! `GOOEY_SIZE=<w>x<h>` set the start. `GOOEY_KEYS=plug,stick:90,…` stages one input every
//! `GOOEY_KEY_EVERY` (default 10) frames from frame 30: the keys `up`, `down`, `left`, `right`,
//! `enter`, `home`, `end`, `plug` (connects a gamepad), `south` and the D-pad `padup`, `paddown`,
//! `padleft`, `padright`; `stick:<degrees>` holds the left stick fully toward that angle (0 is up,
//! 90 is right) and `rest` lets it go.

use bevy::asset::RenderAssetUsages;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
    RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::AutoFocus;
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
    keys: Vec<Staged>,
}

/// One staged input of `GOOEY_KEYS`.
#[derive(Clone, Copy)]
enum Staged {
    Key(KeyCode),
    Plug,
    /// A gamepad button, pressed and released.
    Pad(GamepadButton),
    /// The left stick held toward an angle, in degrees clockwise from up; `None` lets it go.
    Stick(Option<f32>),
}

/// The line that says what the player chose.
#[derive(Component)]
struct Equipped;

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
        title: "bevy_gooey radial".into(),
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
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (follow_background, capture, stage_keys.after(capture)),
    );
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "radial.png".into()),
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
                    "up" => Some(Staged::Key(KeyCode::ArrowUp)),
                    "down" => Some(Staged::Key(KeyCode::ArrowDown)),
                    "left" => Some(Staged::Key(KeyCode::ArrowLeft)),
                    "right" => Some(Staged::Key(KeyCode::ArrowRight)),
                    "enter" => Some(Staged::Key(KeyCode::Enter)),
                    "escape" => Some(Staged::Key(KeyCode::Escape)),
                    "home" => Some(Staged::Key(KeyCode::Home)),
                    "end" => Some(Staged::Key(KeyCode::End)),
                    "plug" => Some(Staged::Plug),
                    "south" => Some(Staged::Pad(GamepadButton::South)),
                    "rest" => Some(Staged::Stick(None)),
                    k if k.starts_with("stick:") => {
                        k[6..].parse().ok().map(|a| Staged::Stick(Some(a)))
                    }
                    "padup" => Some(Staged::Pad(GamepadButton::DPadUp)),
                    "paddown" => Some(Staged::Pad(GamepadButton::DPadDown)),
                    "padleft" => Some(Staged::Pad(GamepadButton::DPadLeft)),
                    "padright" => Some(Staged::Pad(GamepadButton::DPadRight)),
                    _ => None,
                })
                .collect(),
        });
    }
    app.run();
}

/// The weapons, in ring order.
const NAMES: [&str; 8] = [
    "Pistol", "Rifle", "Shotgun", "Grenade", "Knife", "Bow", "Torch", "Medkit",
];

/// The side of one glyph cell, px.
const CELL: u32 = 48;

/// A white-on-transparent atlas of eight glyphs, one per weapon, drawn with distance fields. A game
/// ships its own art; the skin tints whatever is white.
fn glyphs() -> Image {
    type Lines = Vec<(Vec2, Vec2)>;
    let line = |a: [f32; 2], b: [f32; 2]| (Vec2::from(a), Vec2::from(b));
    let ring = |centre: [f32; 2], radius: f32, from: f32, to: f32| -> Lines {
        let steps = 24;
        (0..steps)
            .map(|i| {
                let at = |k: usize| {
                    let angle = (from + (to - from) * k as f32 / steps as f32).to_radians();
                    Vec2::new(
                        centre[0] + radius * angle.cos(),
                        centre[1] + radius * angle.sin(),
                    )
                };
                (at(i), at(i + 1))
            })
            .collect()
    };
    let glyphs: [Lines; 8] = [
        // Pistol: a slide and a grip.
        vec![
            line([6.0, 14.0], [42.0, 14.0]),
            line([6.0, 14.0], [6.0, 24.0]),
            line([6.0, 24.0], [42.0, 24.0]),
            line([42.0, 14.0], [42.0, 24.0]),
            line([10.0, 24.0], [6.0, 42.0]),
            line([6.0, 42.0], [18.0, 42.0]),
            line([18.0, 42.0], [22.0, 24.0]),
            line([24.0, 24.0], [26.0, 32.0]),
            line([26.0, 32.0], [32.0, 32.0]),
            line([32.0, 32.0], [32.0, 24.0]),
        ],
        // Rifle: a long barrel, a stock and a magazine.
        vec![
            line([4.0, 22.0], [44.0, 22.0]),
            line([4.0, 22.0], [4.0, 34.0]),
            line([4.0, 34.0], [18.0, 28.0]),
            line([18.0, 28.0], [26.0, 28.0]),
            line([26.0, 28.0], [26.0, 40.0]),
            line([26.0, 22.0], [26.0, 28.0]),
        ],
        // Shotgun: two barrels and a stock.
        vec![
            line([14.0, 17.0], [44.0, 17.0]),
            line([14.0, 23.0], [44.0, 23.0]),
            line([14.0, 17.0], [14.0, 23.0]),
            line([14.0, 23.0], [6.0, 36.0]),
            line([6.0, 36.0], [14.0, 36.0]),
            line([14.0, 36.0], [20.0, 23.0]),
        ],
        // Grenade: a round body, a neck and a lever.
        {
            let mut lines = ring([24.0, 28.0], 13.0, 0.0, 360.0);
            lines.push(line([20.0, 15.0], [28.0, 15.0]));
            lines.push(line([24.0, 15.0], [24.0, 8.0]));
            lines.push(line([24.0, 8.0], [34.0, 8.0]));
            lines
        },
        // Knife: a blade and a handle.
        vec![
            line([8.0, 40.0], [30.0, 8.0]),
            line([30.0, 8.0], [36.0, 26.0]),
            line([36.0, 26.0], [18.0, 40.0]),
            line([8.0, 40.0], [18.0, 40.0]),
            line([32.0, 30.0], [44.0, 42.0]),
        ],
        // Bow: an arc and its string.
        {
            let mut lines = ring([16.0, 24.0], 18.0, -70.0, 70.0);
            lines.push(line([22.0, 5.0], [22.0, 43.0]));
            lines
        },
        // Torch: a handle and a flame.
        vec![
            line([20.0, 28.0], [28.0, 28.0]),
            line([20.0, 28.0], [22.0, 46.0]),
            line([28.0, 28.0], [26.0, 46.0]),
            line([22.0, 46.0], [26.0, 46.0]),
            line([24.0, 3.0], [31.0, 12.0]),
            line([31.0, 12.0], [31.0, 20.0]),
            line([31.0, 20.0], [24.0, 26.0]),
            line([24.0, 26.0], [17.0, 20.0]),
            line([17.0, 20.0], [17.0, 12.0]),
            line([17.0, 12.0], [24.0, 3.0]),
        ],
        // Medkit: a case with a cross.
        vec![
            line([6.0, 14.0], [42.0, 14.0]),
            line([42.0, 14.0], [42.0, 40.0]),
            line([42.0, 40.0], [6.0, 40.0]),
            line([6.0, 40.0], [6.0, 14.0]),
            line([18.0, 14.0], [18.0, 8.0]),
            line([18.0, 8.0], [30.0, 8.0]),
            line([30.0, 8.0], [30.0, 14.0]),
            line([24.0, 21.0], [24.0, 33.0]),
            line([18.0, 27.0], [30.0, 27.0]),
        ],
    ];
    let side = CELL as usize;
    let mut data = vec![0u8; side * side * 4 * glyphs.len()];
    for (cell, lines) in glyphs.iter().enumerate() {
        for y in 0..side {
            for x in 0..side {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let distance = lines
                    .iter()
                    .map(|&(a, b)| {
                        let ab = b - a;
                        let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
                        p.distance(a + ab * t)
                    })
                    .fold(f32::MAX, f32::min);
                let alpha = (2.4 - distance).clamp(0.0, 1.0);
                let at = (y * side * glyphs.len() + cell * side + x) * 4;
                data[at..at + 4].copy_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
            }
        }
    }
    Image::new(
        Extent3d {
            width: CELL * glyphs.len() as u32,
            height: CELL,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// The screen: the wheel in the middle of the window and a line over it saying what is equipped.
/// A game opens its wheel when the player holds a button and gives it focus (`AutoFocus` here).
fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn(Camera2d);
    // `GOOEY_PICTURES=1` draws the weapons as pictures; the names are then read in the middle.
    let pictures = std::env::var("GOOEY_PICTURES").is_ok_and(|v| v != "0");
    let image = images.add(glyphs());
    let layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(CELL),
        NAMES.len() as u32,
        1,
        None,
        None,
    ));
    let items: Vec<RadialItem> = NAMES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let item = RadialItem::new(*name);
            if pictures {
                item.icon(IconSource::Atlas {
                    image: image.clone(),
                    layout: layout.clone(),
                    index,
                })
            } else {
                item
            }
        })
        .collect();
    commands.spawn((
        screen(),
        children![
            // The status line is a HUD corner of its own, so the wheel has the middle to itself and
            // is as large as the window allows at any text size.
            (
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(4.0),
                    left: Val::Percent(4.0),
                    ..default()
                },
                children![(
                    panel(),
                    children![label("Equipped"), (heading("Nothing"), Equipped)]
                )],
            ),
            (
                radial(items, 0),
                AutoFocus,
                observe(
                    |change: On<ValueChange<usize>>,
                     mut line: Single<&mut Text, With<Equipped>>| {
                        if change.is_final {
                            line.0 = NAMES[change.value].to_string();
                        }
                    }
                ),
            ),
            (
                prompts(),
                children![
                    prompt("Choose", Prompt::SELECT),
                    prompt("Back", Prompt::BACK)
                ],
            ),
        ],
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

/// `GOOEY_KEYS`: one input every `GOOEY_KEY_EVERY` frames, pressed and then released as the window's
/// own keyboard or gamepad input, so Bevy's input dispatch and the crate's navigation see it.
fn stage_keys(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    pads: Query<Entity, With<Gamepad>>,
    mut commands: Commands,
    mut keys: MessageWriter<KeyboardInput>,
    mut connections: MessageWriter<GamepadConnectionEvent>,
    mut pad_input: MessageWriter<RawGamepadEvent>,
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
    match c.keys.get((since / every) as usize) {
        Some(&Staged::Key(key_code)) => {
            keys.write(KeyboardInput {
                key_code,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window,
            });
        }
        Some(Staged::Plug) if state == ButtonState::Pressed => {
            let pad = commands.spawn_empty().id();
            connections.write(GamepadConnectionEvent::new(
                pad,
                GamepadConnection::Connected {
                    name: "Radial pad".into(),
                    vendor_id: None,
                    product_id: None,
                },
            ));
        }
        Some(&Staged::Stick(angle)) if state == ButtonState::Pressed => {
            if let Some(pad) = pads.iter().next() {
                let (x, y) = angle.map_or((0.0, 0.0), |deg| {
                    let r = deg.to_radians();
                    (r.sin(), r.cos())
                });
                for (axis, value) in [(GamepadAxis::LeftStickX, x), (GamepadAxis::LeftStickY, y)] {
                    pad_input.write(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                        pad, axis, value,
                    )));
                }
            }
        }
        Some(&Staged::Pad(button)) => {
            if let Some(pad) = pads.iter().next() {
                let value = if state == ButtonState::Pressed {
                    1.0
                } else {
                    0.0
                };
                pad_input.write(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                    pad, button, value,
                )));
            }
        }
        _ => {}
    }
}
