//! A strip of tabs that is too wide for the room it is given: at 200% text the six labels do not
//! fit across the panel, so the strip lays itself out in as many equal columns as the room holds
//! and wraps into rows -- one column while a label alone is nearly the panel's width, two when
//! they are not. Down walks the strip, the ring stepping from row to row with nothing outside the
//! panel.
//!
//! Run `cargo run --release --example strip`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `strip.png`), `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th
//! frame from `GOOEY_SHOT_FROM` there as `f_%04d.png`, `GOOEY_TEXT_SCALE=<f>`
//! (default 2.0) and `GOOEY_SIZE=<w>x<h>` set the start.
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::WindowResolution;
use bevy_gooey::*;

fn main() {
    let w = std::env::var("GOOEY_SIZE").unwrap_or_else(|_| "760x720".into());
    let (w, h) = w.split_once('x').unwrap_or(("760", "720"));
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "strip".into(),
                resolution: WindowResolution::new(
                    w.parse().unwrap_or(760),
                    h.parse().unwrap_or(720),
                )
                .with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }),
        GooeyPlugins,
    ));
    let mut skin = GooeySkin::parchment();
    if let Ok(scale) = std::env::var("GOOEY_TEXT_SCALE") {
        skin.text_scale = scale.parse().unwrap_or(2.0);
    }
    app.insert_resource(skin);
    app.add_systems(Startup, setup);
    app.add_systems(Update, (shot, walk));
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            Node {
                width: Val::Percent(92.0),
                padding: UiRect::top(Val::Vh(8.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            children![(
                panel(),
                children![tabs(
                    [
                        "Interface",
                        "Gameplay",
                        "Audio",
                        "Video",
                        "Controls",
                        "Advanced"
                    ],
                    0,
                    children![
                        tab_page(children![label("a")]),
                        tab_page(children![label("b")]),
                        tab_page(children![label("c")]),
                        tab_page(children![label("d")]),
                        tab_page(children![label("e")]),
                        tab_page(children![label("f")]),
                    ]
                )]
            )],
        )],
    ));
}

fn shot(
    mut n: Local<u32>,
    mut kept: Local<u32>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    *n += 1;
    // Frames into a directory for a GIF: `GOOEY_SHOT_DIR`, from `GOOEY_SHOT_FROM` every
    // `GOOEY_SHOT_EVERY` frames.
    if let Ok(dir) = std::env::var("GOOEY_SHOT_DIR") {
        let from: u32 = std::env::var("GOOEY_SHOT_FROM")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(28);
        let every: u32 = std::env::var("GOOEY_SHOT_EVERY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);
        if *n >= from && (*n - from).is_multiple_of(every.max(1)) {
            *kept += 1;
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{dir}/f_{:04}.png", *kept)));
        }
    }
    if *n == 60 {
        let path = std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "/tmp/g81/strip.png".into());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if *n > 70 {
        exit.write(AppExit::Success);
    }
}

/// Down walks the strip row by row, so a clip shows the ring crossing a row break.
fn walk(mut n: Local<u32>, mut keys: MessageWriter<bevy::input::keyboard::KeyboardInput>) {
    *n += 1;
    if *n > 30 && n.is_multiple_of(12) && *n < 100 {
        for (state, key_code) in [
            (bevy::input::ButtonState::Pressed, KeyCode::ArrowDown),
            (bevy::input::ButtonState::Released, KeyCode::ArrowDown),
        ] {
            keys.write(bevy::input::keyboard::KeyboardInput {
                key_code,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state,
                repeat: false,
                text: None,
                window: Entity::PLACEHOLDER,
            });
        }
    }
}
