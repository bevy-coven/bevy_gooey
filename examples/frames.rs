//! Art frames: a panel and its buttons drawn from nine-slice images. `GooeyFrames` holds the art; the
//! skin's colours tint it, so the hover, press and disabled states, the focus ring and every
//! contrast guarantee are the flat look's. The example draws its own art in code (a bevelled plate
//! with rounded corners): the panel, the buttons, the meter, the name field and the tooltips; a game loads painted images instead. Enter
//! on the last button takes the frames away and gives the flat look back.
//!
//! Run `cargo run --release --example frames`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `frames.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png`. `GOOEY_SKIN=parchment`, `GOOEY_TEXT_SCALE=<f>` and
//! `GOOEY_SIZE=<w>x<h>` set the start. `GOOEY_KEYS=down,enter,…` presses one key every
//! `GOOEY_KEY_EVERY` (default 10) frames from frame 30 as real keyboard input (`up`, `down`, `left`,
//! `right`, `enter`, `escape`, `home`, `end`).

use bevy::asset::RenderAssetUsages;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
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
        title: "bevy_gooey frames".into(),
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
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "frames.png".into()),
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
                    "pad" => Some(Staged::Pad(GamepadButton::West)),
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

/// The side of the art, image pixels, and the size of its corners.
const ART_SIDE: u32 = 32;
const ART_SLICE: u32 = 10;

/// A bevelled plate in light neutrals (the skin tints it): a bright rim, a darker inner groove, a
/// soft centre with a faint grain, rounded corners and a rivet in each. `grain` varies the centre.
fn plate(radius: f32, rivets: bool, grain: f32) -> Image {
    let side = ART_SIDE as usize;
    let mut pixels = vec![0u8; side * side * 4];
    for y in 0..side {
        for x in 0..side {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            // Distance inside the rounded rectangle's edge.
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
                (0.84, 1.0)
            } else if depth < 3.6 {
                (0.42, 1.0)
            } else {
                let noise = ((x * 7 + y * 13) % 5) as f32 / 5.0;
                (0.97 - grain * noise * 0.12, 1.0)
            };
            let rivet = rivets && {
                let (rx, ry) = (
                    fx.min(side as f32 - fx) - radius * 0.62,
                    fy.min(side as f32 - fy) - radius * 0.62,
                );
                rx * rx + ry * ry < 1.6
            };
            let value = if rivet { 0.3 } else { value };
            let byte = (value * 255.0) as u8;
            let at = (y * side + x) * 4;
            pixels[at..at + 4].copy_from_slice(&[byte, byte, byte, (alpha * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: ART_SIDE,
            height: ART_SIDE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// The screen: a panel of buttons, the last of which takes the frames away.
fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut frames: ResMut<GooeyFrames>,
) {
    commands.spawn(Camera2d);
    frames.panel = Some(Frame::new(
        images.add(plate(9.0, true, 1.0)),
        ART_SLICE as f32,
    ));
    frames.button = Some(Frame::new(
        images.add(plate(6.0, false, 0.0)),
        ART_SLICE as f32,
    ));
    frames.meter = Some(Frame::new(
        images.add(plate(6.0, false, 0.0)),
        ART_SLICE as f32 * 0.6,
    ));
    frames.field = Some(Frame::new(
        images.add(plate(5.0, false, 0.0)),
        ART_SLICE as f32 * 0.6,
    ));
    frames.popup = Some(Frame::new(
        images.add(plate(7.0, false, 1.0)),
        ART_SLICE as f32,
    ));
    commands.spawn((
        screen(),
        children![
            (
                Node {
                    height: Val::Percent(100.0),
                    align_items: AlignItems::FlexStart,
                    padding: UiRect::top(Val::Vh(4.0)),
                    ..default()
                },
                children![(
                    panel(),
                    children![
                        heading("Camp"),
                        (
                            button("Rest"),
                            AutoFocus,
                            Tooltip::new("Sleep until morning")
                        ),
                        (
                            button("Craft"),
                            Tooltip::new("Make tools from what you carry")
                        ),
                        (button("Map"), Tooltip::new("Where the camp stands")),
                        text_input("Name", "Camp 1"),
                        label("Stamina"),
                        meter(70.0, 100.0),
                        (
                            button("Flat look"),
                            ButtonVariant::Outline,
                            observe(|_: On<Activate>, mut frames: ResMut<GooeyFrames>| {
                                *frames = GooeyFrames::default();
                            })
                        ),
                    ]
                )],
            ),
            (
                prompts(),
                children![
                    prompt("Select", Prompt::SELECT),
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
                    name: "Options pad".into(),
                    vendor_id: None,
                    product_id: None,
                },
            ));
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
