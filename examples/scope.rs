//! A subtree in a skin of its own: the left panel is the interface in the player's skin, with the
//! options rows that change it; the right panel carries a `SkinScope` and stays in parchment at the
//! default text size whatever the left one does.
//!
//! - **Theme** and **Text size** edit the `GooeySkin` resource, and everything outside a scope
//!   answers the same frame: the left panel, its rows and the legend. The right panel is one
//!   component, `SkinScope(GooeySkin::parchment())`, on its root: its buttons, switch, slider and
//!   text are styled from that skin and from nothing else.
//! - Removing the scope (Enter on the right panel's Button) hands the panel back to the player's
//!   skin in the same frame, so the two panels are one family again.
//! - A tool shows a game's skin in a preview panel next to its own chrome the same way, and a game
//!   previews the theme it is about to apply.
//!
//! Run `cargo run --release --example scope`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `options.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png`. `GOOEY_SKIN=parchment`, `GOOEY_TEXT_SCALE=<f>` and
//! `GOOEY_SIZE=<w>x<h>` set the start. `GOOEY_KEYS=down,right,…` presses one key every
//! `GOOEY_KEY_EVERY` (default 10) frames from frame 30 as real keyboard input (`up`, `down`, `left`,
//! `right`, `enter`, `escape`, `home`, `end`), `plug` connects a gamepad, `pad` presses its X button
//! (the player has picked up the pad) and `padup`, `paddown`, `padleft` and `padright` its D-pad.

use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
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
        _ => GooeySkin::dark(),
    };
    if let Some(scale) = env("GOOEY_TEXT_SCALE") {
        skin.text_scale = scale;
    }
    let mut window = Window {
        title: "bevy_gooey options".into(),
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
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "options.png".into()),
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

/// The screen: the options panel on the left and the scoped panel on the right, near the top so the
/// largest text size stands clear of the legend, and the legend in the corner.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![
            (
                Node {
                    height: Val::Percent(100.0),
                    align_items: AlignItems::FlexStart,
                    column_gap: Val::Px(24.0),
                    padding: UiRect::top(Val::Vh(4.0)),
                    ..default()
                },
                children![
                    (
                        panel(),
                        children![
                            heading("Options"),
                            (text_size("Text size"), AutoFocus),
                            skin_preset("Theme"),
                            toggle("Subtitles", true),
                            slider("Volume", 70.0, 0.0..=100.0),
                            button("Play"),
                        ]
                    ),
                    (
                        panel(),
                        SkinScope(GooeySkin::parchment()),
                        children![
                            heading("Preview"),
                            toggle("Subtitles", true),
                            slider("Volume", 70.0, 0.0..=100.0),
                            (button("Play"), observe(unscope)),
                        ]
                    ),
                ],
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

/// Activating the preview's button takes the scope off its panel.
fn unscope(
    activate: On<Activate>,
    parents: Query<&ChildOf>,
    scoped: Query<(), With<SkinScope>>,
    mut commands: Commands,
) {
    for ancestor in parents.iter_ancestors(activate.event_target()) {
        if scoped.contains(ancestor) {
            commands.entity(ancestor).remove::<SkinScope>();
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
