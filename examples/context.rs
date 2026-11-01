//! Context menus: a menu that belongs to a thing. A right-click on a crate in the world opens the
//! crate menu at the pointer (near the window's edge it flips up and to the left); a right-click on
//! a row of the inventory opens the row menu (one menu for every row, with a submenu); Shift+F10 or
//! the Menu key on a focused row opens the same menu below it. The line at the bottom says what was
//! chosen and for which thing (`ContextTarget`). Escape or a press elsewhere closes the menu.
//!
//! Run `cargo run --release --example context`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `context.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS=down,context,enter,…` stages one step every `GOOEY_KEY_EVERY` (default 10) frames
//! from frame 30: `up`, `down`, `left`, `right`, `enter`, `escape` and `menu` (the Menu key) as real
//! keyboard input, `shiftf10` (Shift+F10), `context` (a right-click at the next point of
//! `GOOEY_POINTS=x,y;x,y;…`) and `away` (a primary click on the empty ground at the bottom left).

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::AutoFocus;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::sprite::{SpritePickingMode, SpritePickingSettings};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy_gooey::*;

/// A staged step of `GOOEY_KEYS`.
#[derive(Clone, Copy)]
enum Step {
    Key(KeyCode),
    /// Shift+F10.
    ShiftF10,
    /// A right-click at a point.
    Context(Vec2),
    /// A primary click on the empty ground.
    Away,
}

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
    step_every: u32,
    steps: Vec<Step>,
}

/// The frame `GOOEY_KEYS` starts at, after the layout and autofocus have settled.
const KEY_FROM: u32 = 30;
/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

/// The line at the bottom that says what was chosen.
#[derive(Component)]
struct Status;

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
        title: "bevy_gooey context menus".into(),
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
    .insert_resource(SpritePickingSettings {
        picking_mode: SpritePickingMode::BoundingBox,
        ..default()
    })
    .add_systems(Startup, setup)
    .add_systems(Update, (follow_background, capture, stage.after(capture)));
    if let Some(at) = env("GOOEY_FRAMES") {
        let mut points = std::env::var("GOOEY_POINTS")
            .unwrap_or_default()
            .split(';')
            .filter_map(|p| p.split_once(','))
            .filter_map(|(x, y)| Some(Vec2::new(x.trim().parse().ok()?, y.trim().parse().ok()?)))
            .collect::<Vec<_>>()
            .into_iter();
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "context.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            step_every: env("GOOEY_KEY_EVERY").unwrap_or(10),
            steps: std::env::var("GOOEY_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|k| match k.trim() {
                    "up" => Some(Step::Key(KeyCode::ArrowUp)),
                    "down" => Some(Step::Key(KeyCode::ArrowDown)),
                    "left" => Some(Step::Key(KeyCode::ArrowLeft)),
                    "right" => Some(Step::Key(KeyCode::ArrowRight)),
                    "enter" => Some(Step::Key(KeyCode::Enter)),
                    "escape" => Some(Step::Key(KeyCode::Escape)),
                    "menu" => Some(Step::Key(KeyCode::ContextMenu)),
                    "shiftf10" => Some(Step::ShiftF10),
                    "context" => points.next().map(Step::Context),
                    "away" => Some(Step::Away),
                    _ => None,
                })
                .collect(),
        });
    }
    app.run();
}

/// What a command of either menu does: say what was chosen and for which thing.
fn chose(verb: &'static str) -> impl Bundle {
    observe(
        move |activate: On<Activate>,
              parents: Query<&ChildOf>,
              targets: Query<&ContextTarget>,
              names: Query<&Name>,
              mut status: Query<&mut Text, With<Status>>| {
            let thing = ContextTarget::of(activate.entity, &parents, &targets)
                .and_then(|t| names.get(t).ok())
                .map_or("nothing", |n| n.as_str());
            for mut line in &mut status {
                line.0 = format!("{verb}: {thing}");
            }
        },
    )
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    // One menu for every crate in the world, and one for every row of the inventory.
    let crate_menu = commands
        .spawn(context_menu(children![
            (
                menu_item("Inspect").with_shortcut(Shortcut::new(Modifiers::NONE, KeyCode::KeyI)),
                chose("Inspected")
            ),
            (menu_item("Duplicate"), chose("Duplicated")),
            (check_item("Locked", true), chose("Toggled lock on")),
            separator(),
            (menu_item("Delete"), chose("Deleted")),
        ]))
        .id();
    let row_menu = commands
        .spawn(context_menu(children![
            (menu_item("Use"), chose("Used")),
            (menu_item("Drop"), chose("Dropped")),
            separator(),
            submenu(
                "Move to",
                children![
                    (menu_item("Chest 1"), chose("Moved to chest 1")),
                    (menu_item("Chest 2"), chose("Moved to chest 2")),
                ]
            ),
        ]))
        .id();
    // The world: three crates under the interface, each picked with `Pickable`.
    for (name, at, colour) in [
        (
            "Crate A",
            Vec2::new(150.0, -100.0),
            Color::srgb(0.55, 0.36, 0.22),
        ),
        (
            "Crate B",
            Vec2::new(330.0, 40.0),
            Color::srgb(0.36, 0.5, 0.35),
        ),
        (
            "Crate C",
            Vec2::new(170.0, 180.0),
            Color::srgb(0.35, 0.42, 0.6),
        ),
        (
            "Crate D",
            Vec2::new(520.0, -230.0),
            Color::srgb(0.6, 0.4, 0.5),
        ),
    ] {
        commands.spawn((
            Name::new(name),
            Sprite::from_color(colour, Vec2::splat(120.0)),
            Transform::from_xyz(at.x, at.y, 0.0),
            Pickable::default(),
            ContextMenu(crate_menu),
        ));
    }
    // The inventory: a panel of rows at the left.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Px(16.0),
            ..default()
        },
        children![(
            panel(),
            children![
                heading("Inventory"),
                (
                    button("Iron sword"),
                    Name::new("Iron sword"),
                    AutoFocus,
                    ContextMenu(row_menu)
                ),
                (
                    button("Healing herb"),
                    Name::new("Healing herb"),
                    ContextMenu(row_menu)
                ),
                (
                    button("Old map"),
                    Name::new("Old map"),
                    ContextMenu(row_menu)
                ),
            ]
        )],
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(16.0),
            left: Val::Px(16.0),
            ..default()
        },
        children![(label("Right-click a thing, or Shift+F10 on a row"), Status)],
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

/// `GOOEY_KEYS`: one step every `GOOEY_KEY_EVERY` frames. A key is pressed and then released as
/// the window's own keyboard input; a click moves the pointer first, then presses, then releases,
/// as the window's own pointer input.
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keys: MessageWriter<KeyboardInput>,
    mut pointer: MessageWriter<PointerInput>,
) {
    let (Some(c), Ok(window)) = (capture, window.single()) else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.step_every.max(4);
    let Some(&step) = c.steps.get((since / every) as usize) else {
        return;
    };
    let phase = since % every;
    let key = |key_code: KeyCode, state: ButtonState, keys: &mut MessageWriter<KeyboardInput>| {
        keys.write(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
    };
    match step {
        Step::Key(key_code) => {
            let state = match phase {
                0 => ButtonState::Pressed,
                n if n == every / 2 => ButtonState::Released,
                _ => return,
            };
            key(key_code, state, &mut keys);
        }
        Step::ShiftF10 => match phase {
            0 => key(KeyCode::ShiftLeft, ButtonState::Pressed, &mut keys),
            1 => key(KeyCode::F10, ButtonState::Pressed, &mut keys),
            n if n == every / 2 => {
                key(KeyCode::F10, ButtonState::Released, &mut keys);
                key(KeyCode::ShiftLeft, ButtonState::Released, &mut keys);
            }
            _ => {}
        },
        Step::Context(_) | Step::Away => {
            let (at, button) = match step {
                Step::Context(at) => (at, PointerButton::Secondary),
                _ => (Vec2::new(8.0, 400.0), PointerButton::Primary),
            };
            let action = match phase {
                0 => PointerAction::Move { delta: Vec2::ZERO },
                n if n == every / 2 => PointerAction::Press(button),
                n if n == every / 2 + 1 => PointerAction::Release(button),
                _ => return,
            };
            let Some(target) = WindowRef::Entity(window).normalize(None) else {
                return;
            };
            pointer.write(PointerInput::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::Window(target),
                    position: at,
                },
                action,
            ));
        }
    }
}
