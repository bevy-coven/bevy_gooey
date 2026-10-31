//! Every widget on one screen.
//!
//! - Arrows or the D-pad move focus; Enter, Space or the pad's South button activate.
//! - **Options** switches between the dark and the parchment skin; **Shift+T** toggles a 1.5x text
//!   scale; **Quit** opens a modal dialog that holds focus (Stay first; Tab, arrows and the D-pad
//!   stay inside it; closing it puts focus back on Quit); focusing or hovering **Options** shows
//!   its tooltip beside it (`GOOEY_KEYS=down` stages it).
//! - The health meter drains and refills (`GOOEY_HEALTH=<value>` holds it at that value instead).
//!   Each `UiFeedback` is logged where a game would play a sound.
//! - **Subtitles** is a switch and **Volume** a slider (Left and Right step it, Home and End jump,
//!   the D-pad too, held to repeat; a click or a drag sets it): each logs the value it settles on.
//! - Resize the window: the whole interface keeps its share of it (twice the size at 2560x1440
//!   as at 1280x720).
//!
//! Capture mode, for a still and a frame time: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `gallery.png`), prints `gooey: frames <n> mean_ms <x>` (the mean frame time
//! over frames 20..n) and exits 10 frames later. `GOOEY_SKIN=parchment` starts in parchment;
//! `GOOEY_TEXT_SCALE=<f>` starts at that text scale; `GOOEY_SIZE=<w>x<h>` sets the window size.
//! `GOOEY_KEYS=down,down,enter,…` stages a state before the shot: from frame 30 it presses one key
//! every 10 frames, as real keyboard input (`up`, `down`, `left`, `right`, `enter`, `tab`), or
//! clicks the window's empty top-left corner, as real pointer input (`click`). `untip` takes
//! Options' `Tooltip` off and `retip` puts a new line on it, as a game does when a hint goes stale.
//! `hover`, `grab`, `drag` and `drop` work Volume's thumb with the pointer: move onto it, press and
//! hold, move 60 px right, let go.
//! `GOOEY_WORLD=1` puts a sprite under the interface, standing in for the game world: the
//! interface lets the pointer through its empty part, so a click on it (`click` hits it, at the
//! empty top-left corner) turns it warm and logs `gooey: world`.
//! A toast stands in the stack at the bottom centre (`GOOEY_TOAST_ANCHOR=topright` moves it): Play
//! says "Game saved", the Subtitles switch and the Volume slider say what they settled on, and
//! pressing Play four times shows the stack's cap. In capture mode toasts stand 100 times as long
//! as usual, so a still holds them; `GOOEY_KEYS` tokens `toast`, `alert` and `long` spawn one.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::AutoFocus;
use bevy::picking::events::PointerPress;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy_gooey::*;

/// The health meter.
#[derive(Component)]
struct Health;

/// `GOOEY_HEALTH`: the value the meter holds instead of draining, for a still.
#[derive(Resource)]
struct HeldHealth(Option<f32>);

/// The open Quit dialog.
#[derive(Component)]
struct QuitDialog;

/// Capture mode's state, present only when `GOOEY_FRAMES` is set.
#[derive(Resource)]
struct Capture {
    at: u32,
    shot: String,
    frame: u32,
    total_ms: f64,
    counted: u32,
    /// `GOOEY_KEYS`, staged one every [`KEY_EVERY`] frames from [`KEY_FROM`].
    keys: Vec<Staged>,
}

/// One staged input of `GOOEY_KEYS`.
#[derive(Clone, Copy)]
enum Staged {
    /// A key, pressed and released.
    Key(KeyCode),
    /// A primary click on the window's empty top-left corner, where no widget is.
    Click,
    /// Remove the `Tooltip` from the widget that has one.
    Untip,
    /// Insert a new `Tooltip` over the widget's old one.
    Retip,
    /// Move the pointer onto the Volume slider's thumb.
    Hover,
    /// Press the primary button on the thumb and keep it down.
    Grab,
    /// Move the held pointer 60 px right of where the thumb was grabbed.
    Drag,
    /// Let go.
    Drop,
    /// Spawn a short toast.
    Toast,
    /// Spawn an alert toast.
    Alert,
    /// Spawn a toast long enough to wrap.
    Long,
}

/// The frame `GOOEY_KEYS` starts pressing at, after the layout and autofocus have settled.
const KEY_FROM: u32 = 30;
/// Frames between two staged key presses; the release comes halfway.
const KEY_EVERY: u32 = 10;

/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;

/// The world's colour before and after a click on it, `GOOEY_WORLD`.
const WORLD_IDLE: Color = Color::srgb(0.10, 0.19, 0.24);
const WORLD_PRESSED: Color = Color::srgb(0.42, 0.20, 0.10);

fn main() {
    let mut skin = match std::env::var("GOOEY_SKIN").as_deref() {
        Ok("parchment") => GooeySkin::parchment(),
        _ => GooeySkin::dark(),
    };
    if let Some(scale) = std::env::var("GOOEY_TEXT_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        skin.text_scale = scale;
    }
    let mut window = Window {
        title: "bevy_gooey gallery".into(),
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
    .insert_resource(toast_settings())
    .insert_resource(HeldHealth(
        std::env::var("GOOEY_HEALTH")
            .ok()
            .and_then(|s| s.parse().ok()),
    ))
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            drain_health,
            toggle_text_scale,
            follow_background,
            log_feedback,
            capture,
            stage_keys.after(capture),
        ),
    );
    if let Some(at) = std::env::var("GOOEY_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "gallery.png".into()),
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            keys: std::env::var("GOOEY_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|k| match k.trim() {
                    "up" => Some(Staged::Key(KeyCode::ArrowUp)),
                    "down" => Some(Staged::Key(KeyCode::ArrowDown)),
                    "left" => Some(Staged::Key(KeyCode::ArrowLeft)),
                    "right" => Some(Staged::Key(KeyCode::ArrowRight)),
                    "enter" => Some(Staged::Key(KeyCode::Enter)),
                    "tab" => Some(Staged::Key(KeyCode::Tab)),
                    "click" => Some(Staged::Click),
                    "untip" => Some(Staged::Untip),
                    "retip" => Some(Staged::Retip),
                    "hover" => Some(Staged::Hover),
                    "grab" => Some(Staged::Grab),
                    "drag" => Some(Staged::Drag),
                    "drop" => Some(Staged::Drop),
                    "toast" => Some(Staged::Toast),
                    "alert" => Some(Staged::Alert),
                    "long" => Some(Staged::Long),
                    _ => None,
                })
                .collect(),
        });
    }
    if std::env::var_os("GOOEY_WORLD").is_some() {
        app.add_systems(Startup, spawn_world);
    }
    app.run();
}

/// `GOOEY_TOAST_ANCHOR` (`topleft`, `topcenter`, `topright`, `bottomleft`, `bottomcenter`,
/// `bottomright`) places the stack; in capture mode every toast stands 100 times as long, which is
/// the player's `duration_scale` option at work.
fn toast_settings() -> ToastSettings {
    let anchor = match std::env::var("GOOEY_TOAST_ANCHOR").as_deref() {
        Ok("topleft") => ToastAnchor::TopLeft,
        Ok("topcenter") => ToastAnchor::TopCenter,
        Ok("topright") => ToastAnchor::TopRight,
        Ok("bottomleft") => ToastAnchor::BottomLeft,
        Ok("bottomright") => ToastAnchor::BottomRight,
        _ => ToastAnchor::BottomCenter,
    };
    ToastSettings {
        anchor,
        duration_scale: if std::env::var_os("GOOEY_FRAMES").is_some() {
            100.0
        } else {
            1.0
        },
        ..default()
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            panel(),
            children![
                heading("bevy_gooey"),
                (
                    menu(),
                    children![
                        (
                            button("Play"),
                            AutoFocus,
                            observe(|_: On<Activate>, mut commands: Commands| {
                                info!("gooey: play");
                                commands.spawn(toast("Game saved"));
                            }),
                        ),
                        (
                            button("Options"),
                            Tooltip("Sound, controls and display".into()),
                            observe(switch_skin),
                        ),
                        (button("Quit"), observe(open_quit_dialog)),
                    ]
                ),
                label("Health"),
                (meter(100.0, 100.0), Health),
                (
                    toggle("Subtitles", true),
                    observe(|change: On<ValueChange<bool>>, mut commands: Commands| {
                        info!("gooey: subtitles {}", change.value);
                        commands.spawn(toast(if change.value {
                            "Subtitles on"
                        } else {
                            "Subtitles off"
                        }));
                    }),
                ),
                (
                    slider("Volume", 0.7, 0.0..=1.0),
                    observe(|change: On<ValueChange<f32>>, mut commands: Commands| {
                        if change.is_final {
                            info!("gooey: volume {:.2}", change.value);
                            commands.spawn(toast(format!("Volume {:.0}%", change.value * 100.0)));
                        }
                    }),
                ),
            ]
        )],
    ));
}

/// `GOOEY_WORLD`: the game world, a sprite bigger than any window, under the interface.
fn spawn_world(mut commands: Commands) {
    commands
        .spawn((
            Sprite::from_color(WORLD_IDLE, Vec2::splat(8000.0)),
            // Bevy's sprite picking considers only sprites that opt in.
            Pickable::default(),
        ))
        .observe(press_world);
}

/// What a game does when its world is clicked: here, turn warm and log it.
fn press_world(press: On<PointerPress>, mut sprites: Query<&mut Sprite>) {
    if let Ok(mut sprite) = sprites.get_mut(press.entity) {
        sprite.color = WORLD_PRESSED;
    }
    info!("gooey: world");
}

/// Options: dark to parchment and back, keeping the player's text scale.
fn switch_skin(_: On<Activate>, mut skin: ResMut<GooeySkin>) {
    let next = if skin.background == GooeySkin::parchment().background {
        GooeySkin::dark()
    } else {
        GooeySkin::parchment()
    };
    *skin = GooeySkin {
        text_scale: skin.text_scale,
        ..next
    };
}

fn open_quit_dialog(_: On<Activate>, open: Query<(), With<QuitDialog>>, mut commands: Commands) {
    if !open.is_empty() {
        return;
    }
    commands.spawn((
        dialog(),
        QuitDialog,
        children![(
            panel(),
            children![
                label("Leave the game?"),
                (
                    menu(),
                    children![
                        (
                            button("Leave"),
                            observe(|_: On<Activate>, mut exit: MessageWriter<AppExit>| {
                                exit.write(AppExit::Success);
                            }),
                        ),
                        (button("Stay"), AutoFocus, observe(close_quit_dialog)),
                    ]
                ),
            ]
        )],
    ));
}

/// Stay: closing the dialog is all it takes; focus goes back to Quit on its own.
fn close_quit_dialog(
    _: On<Activate>,
    dialogs: Query<Entity, With<QuitDialog>>,
    mut commands: Commands,
) {
    for dialog in &dialogs {
        commands.entity(dialog).despawn();
    }
}

/// The meter drains by 10 a second and refills when empty, or holds `GOOEY_HEALTH`.
fn drain_health(
    time: Res<Time>,
    held: Res<HeldHealth>,
    mut meters: Query<&mut Meter, With<Health>>,
) {
    for mut meter in &mut meters {
        if let Some(value) = held.0 {
            let max = meter.max;
            meter.set_if_neq(Meter { value, max });
            continue;
        }
        meter.value -= 10.0 * time.delta_secs();
        if meter.value <= 0.0 {
            meter.value = meter.max;
        }
    }
}

/// Shift+T: the player's text-size option, 1.0 or 1.5.
fn toggle_text_scale(keys: Res<ButtonInput<KeyCode>>, mut skin: ResMut<GooeySkin>) {
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if shift && keys.just_pressed(KeyCode::KeyT) {
        skin.text_scale = if skin.text_scale > 1.0 { 1.0 } else { 1.5 };
    }
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

/// Where a game would play a sound.
fn log_feedback(mut feedback: MessageReader<UiFeedback>) {
    for f in feedback.read() {
        info!("gooey: feedback {:?}", f.kind);
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
    if c.frame == c.at + 10 {
        exit.write(AppExit::Success);
    }
}

/// `GOOEY_KEYS`: one input every [`KEY_EVERY`] frames, pressed and then released as the window's
/// own keyboard or pointer input, so Bevy's input dispatch, picking, Tab navigation and buttons all
/// see it.
fn stage_keys(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    tipped: Query<Entity, With<Tooltip>>,
    thumbs: Query<(&Role, &UiGlobalTransform)>,
    mut commands: Commands,
    mut keys: MessageWriter<KeyboardInput>,
    mut pointer: MessageWriter<PointerInput>,
) {
    let (Some(c), Ok(window)) = (capture, window.single()) else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let state = match since % KEY_EVERY {
        0 => ButtonState::Pressed,
        n if n == KEY_EVERY / 2 => ButtonState::Released,
        _ => return,
    };
    match c.keys.get((since / KEY_EVERY) as usize) {
        Some(&Staged::Key(key_code)) => {
            keys.write(KeyboardInput {
                key_code,
                logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window,
            });
        }
        Some(Staged::Click) => {
            let Some(target) = WindowRef::Entity(window).normalize(None) else {
                return;
            };
            let at = Location {
                target: NormalizedRenderTarget::Window(target),
                position: Vec2::splat(4.0),
            };
            let actions: &[PointerAction] = match state {
                ButtonState::Pressed => &[
                    PointerAction::Move { delta: Vec2::ZERO },
                    PointerAction::Press(PointerButton::Primary),
                ],
                ButtonState::Released => &[PointerAction::Release(PointerButton::Primary)],
            };
            for &action in actions {
                pointer.write(PointerInput::new(PointerId::Mouse, at.clone(), action));
            }
        }
        Some(Staged::Untip) if state == ButtonState::Pressed => {
            for widget in &tipped {
                commands.entity(widget).remove::<Tooltip>();
            }
        }
        Some(Staged::Retip) if state == ButtonState::Pressed => {
            for widget in &tipped {
                commands.entity(widget).insert(Tooltip(
                    "Unlocked: sound, controls, display and cheats".into(),
                ));
            }
        }
        Some(&step @ (Staged::Hover | Staged::Grab | Staged::Drag | Staged::Drop))
            if state == ButtonState::Pressed =>
        {
            let Some(target) = WindowRef::Entity(window).normalize(None) else {
                return;
            };
            let Some((_, thumb)) = thumbs.iter().find(|(role, _)| **role == Role::SliderThumb)
            else {
                return;
            };
            let reach = if matches!(step, Staged::Drag) {
                60.0
            } else {
                0.0
            };
            let at = Location {
                target: NormalizedRenderTarget::Window(target),
                position: thumb.translation + Vec2::new(reach, 0.0),
            };
            let actions: &[PointerAction] = match step {
                Staged::Grab => &[
                    PointerAction::Move { delta: Vec2::ZERO },
                    PointerAction::Press(PointerButton::Primary),
                ],
                Staged::Drop => &[PointerAction::Release(PointerButton::Primary)],
                _ => &[PointerAction::Move {
                    delta: Vec2::new(reach, 0.0),
                }],
            };
            for &action in actions {
                pointer.write(PointerInput::new(PointerId::Mouse, at.clone(), action));
            }
        }
        Some(Staged::Toast) if state == ButtonState::Pressed => {
            commands.spawn(toast("Game saved"));
        }
        Some(Staged::Alert) if state == ButtonState::Pressed => {
            commands.spawn(toast_alert("Inventory full"));
        }
        Some(Staged::Long) if state == ButtonState::Pressed => {
            commands.spawn(toast(
                "Quest updated: the lighthouse keeper wants three crates of lamp oil by morning",
            ));
        }
        _ => {}
    }
}
