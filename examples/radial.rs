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

use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
    RawGamepadButtonChangedEvent, RawGamepadEvent,
};
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

/// The screen: the wheel in the middle of the window and a line over it saying what is equipped.
/// A game opens its wheel when the player holds a button and gives it focus (`AutoFocus` here).
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                children![
                    (heading("Equipped: nothing"), Equipped),
                    (
                        radial(
                            [
                                "Pistol", "Rifle", "Shotgun", "Grenade", "Knife", "Bow", "Torch",
                                "Medkit"
                            ],
                            0
                        ),
                        AutoFocus,
                        observe(
                            |change: On<ValueChange<usize>>,
                             mut line: Single<&mut Text, With<Equipped>>| {
                                if change.is_final {
                                    const NAMES: [&str; 8] = [
                                        "Pistol", "Rifle", "Shotgun", "Grenade", "Knife", "Bow",
                                        "Torch", "Medkit",
                                    ];
                                    line.0 = format!("Equipped: {}", NAMES[change.value]);
                                }
                            }
                        ),
                    )
                ],
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
