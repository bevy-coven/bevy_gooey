//! Every widget on one screen.
//!
//! - Arrows or the D-pad move focus; Enter, Space or the pad's South button activate.
//! - **Options** switches between the dark and the parchment skin; **Shift+T** toggles a 1.5x text
//!   scale; **Quit** opens a modal dialog that holds focus (Stay first; Tab, arrows and the D-pad
//!   stay inside it; **Escape** or the pad's B closes it, as Stay does, and either puts focus back
//!   on Quit); focusing or hovering **Options** shows its tooltip beside it (`GOOEY_KEYS=down`
//!   stages it).
//! - The health meter drains and refills (`GOOEY_HEALTH=<value>` holds it at that value instead;
//!   `GOOEY_KEYS` tokens `hit` and `heal` take 40 off or add 40 and hold it there: a hit leaves a
//!   trail of the lost length, a heal eases up).
//!   Each `UiFeedback` is logged where a game would play a sound.
//! - **Audio**, **Display** and **Controls** are the tabs of one settings widget: click a tab, Left
//!   and Right on a focused tab, or Ctrl+PageDown and Ctrl+PageUp (on a pad, `LB` and `RB`) turn its
//!   page, and each turn logs the page. Audio holds **Volume**, a slider, and **Subtitles**, a
//!   switch; Display holds **Quality** and **Window**, choices; Controls holds **Jump**, a key row
//!   (Enter starts a capture, the next key is the new binding, Escape exits, and it logs and toasts
//!   the key), and **Use**, a pad row (it waits for a pad button). Left and
//!   Right step a focused choice (it stops at its ends; Home and End jump; Enter, Space and South
//!   cycle it round; a click on an arrow steps it) and a focused slider (Home and End jump, the
//!   D-pad too, held to repeat; a click or a drag sets it): each logs the value it settles on.
//! - Resize the window: the whole interface keeps its share of it (twice the size at 2560x1440
//!   as at 1280x720).
//!
//! Capture mode, for a still and a frame time: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_FIXED_CLOCK=1` makes every frame 1/30 s (a deterministic recording). **It also pins the frame
//! time**: a run with it reports `mean_ms 33.333`, which is the clock, not a measurement, and the line
//! says so. Take the frame time from a run **without** it (`docs/research/perf-gate.md`).
//! `GOOEY_SHOT` (default `gallery.png`), prints `gooey: frames <n> mean_ms <x>` (the mean frame time
//! over frames 20..n) and exits 10 frames later. `GOOEY_SHOT_DIR=<dir>` also keeps every
//! `GOOEY_SHOT_EVERY`th frame (`GOOEY_SHOT_FROM`, default frame 30) there as `f_%04d.png`, and prints
//! `gooey: kept <n> frames in <dir>`: those frames are a GIF's, and the still stays one file.
//! `GOOEY_A11Y=1` stands in for a connected screen reader and prints the accessibility tree at the
//! capture frame, one `gooey: a11y` line per node: its role, name, value and flags.
//! `GOOEY_SKIN=parchment`, `high_contrast`, `tool`, `desk` or `desk_light` starts in that skin;
//! `GOOEY_TEXT_SCALE=<f>` starts at that text scale; `GOOEY_SIZE=<w>x<h>` sets the window size;
//! `GOOEY_MOTION=<f>` sets `MotionSettings::duration_scale` (`0` is no motion, `3` a slow
//! entrance for a GIF); `GOOEY_MOTION_REDUCED=1` turns on `MotionSettings::reduced`.
//! `GOOEY_KEYS=down,down,enter,…` stages a state before the shot: from frame 30 it presses one key
//! every 10 frames, as real keyboard input (`up`, `down`, `left`, `right`, `enter`, `tab`, `escape`), or
//! clicks the window's empty top-left corner, as real pointer input (`click`). `untip` takes
//! Options' `Tooltip` off and `retip` puts a new line on it, as a game does when a hint goes stale.
//! `hover`, `grab`, `drag` and `drop` work Volume's thumb with the pointer: move onto it, press and
//! hold, move 60 px right, let go.
//! `GOOEY_KEYS` token `legend` moves the Select and Back legend to the next `PromptsAnchor`.
//! `GOOEY_WORLD=1` puts a sprite under the interface, standing in for the game world: the
//! interface lets the pointer through its empty part, so a click on it (`click` hits it, at the
//! empty top-left corner) turns it warm and logs `gooey: world`.
//! Toasts stand in a stack at the bottom right, clear of the panel (`GOOEY_TOAST_ANCHOR=topright`
//! moves it; the crate's own default is the bottom centre): Play says "Game saved", the Subtitles
//! switch and the Volume slider say what they settled on, and pressing Play four times shows the
//! stack's cap. In capture mode toasts stand 100 times as long as usual, so a still holds them;
//! `GOOEY_KEYS` tokens `toast`, `alert` and `long` spawn one.
//! The **Load game** list holds twelve save slots, four and a half rows tall: it scrolls with the
//! wheel, a drag on its thumb and a click on its rail, and follows focus (the next row stays in
//! view); PageUp and PageDown move a viewport. `GOOEY_KEYS` tokens `slots` (focus the first slot),
//! `pagedown`, `pageup` and `wheel` (one notch over the list) stage it. `nexttab` and `prevtab`
//! press Ctrl+PageDown and Ctrl+PageUp, and `tab1`, `tab2` and `tab3` click that tab. `q` and `f` press
//! those keys, to bind one to Jump (`GOOEY_KEYS=tab3,down,enter,q`).
//! The bottom-left corner holds the legend, **Select** and **Back** ([`prompts`]), which also stands
//! in the Quit dialog: the keycaps are the keyboard's ("Enter", "Esc") until a pad button is
//! pressed, then the pad's, round ("A", "B"), until the next key. `GOOEY_KEYS` tokens `plug`
//! (connect a gamepad) and `pad` (press its X button, which nothing acts on) stage that.
//! The **Save as** field under the list is a [`text_input`]: click it or Tab to it and type, Enter
//! saves (a toast), and Up leaves it. `GOOEY_KEYS` tokens `name` (focus it) and `type=Aria` (type
//! those characters, one every beat) stage that.

use bevy::a11y::{AccessibilityNode, AccessibilityRequested};
use bevy::camera::NormalizedRenderTarget;
use bevy::ecs::spawn::SpawnIter;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::touch::TouchPhase;
use bevy::input_focus::{AutoFocus, FocusCause, InputFocus, InputFocusVisible};
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

/// A row of the Load game list.
#[derive(Component)]
struct SaveSlot;

/// The Quality choice's options.
const QUALITY: [&str; 4] = ["Low", "Medium", "High", "Ultra"];

/// The Load game list's rows.
const SLOTS: [&str; 12] = [
    "Autosave",
    "Meadow, 12:40",
    "Harbour, 09:15",
    "Mill, 03:02",
    "Cellar, 01:48",
    "Orchard, 00:20",
    "Chapel, 00:12",
    "Quarry, 00:09",
    "Marsh, 00:07",
    "Lighthouse, 00:05",
    "Ridge, 00:03",
    "Delta, 00:01",
];

/// Capture mode's state, present only when `GOOEY_FRAMES` is set.
#[derive(Resource)]
struct Capture {
    at: u32,
    shot: String,
    /// `GOOEY_SHOT_DIR`: keep every [`Self::every`]th frame from [`Self::from`] as
    /// `<dir>/f_%04d.png`, for a GIF. `GOOEY_SHOT` still writes its one still at [`Self::at`].
    dir: Option<String>,
    from: u32,
    every: u32,
    kept: u32,
    frame: u32,
    total_ms: f64,
    counted: u32,
    /// `GOOEY_KEYS`, staged one every [`Self::key_every`] frames from [`KEY_FROM`].
    keys: Vec<Staged>,
    /// `GOOEY_KEY_EVERY`: frames between two staged inputs ([`KEY_EVERY`]). A GIF of a walk down a
    /// panel wants a slower beat than a still's state-setting does.
    key_every: u32,
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
    /// Add this to the health meter's value and hold it there: a hit (negative) or a heal.
    Health(f32),
    /// Spawn a short toast.
    Toast,
    /// Spawn an alert toast.
    Alert,
    /// Spawn a toast long enough to wrap.
    Long,
    /// A primary click on the Quality choice's back arrow.
    Back,
    /// A primary click on the Quality choice's forward arrow.
    Forward,
    /// Focus the first row of the Load game list, showing focus.
    Slots,
    /// One notch of the wheel down, over the Load game list.
    Wheel,
    /// Ctrl+PageDown, the keyboard's next tab (the pad's `RB`).
    NextTab,
    /// Ctrl+PageUp, the keyboard's previous tab (the pad's `LB`).
    PrevTab,
    /// A primary click on the tab at this place in the strip, from the left.
    Tab(usize),
    /// Move every legend to the next `PromptsAnchor`: bottom-left, bottom-centre, bottom-right,
    /// top-left, top-centre, top-right, and round again.
    Legend,
    /// Plug in a gamepad.
    Plug,
    /// Press and release the plugged-in gamepad's West button (X on an Xbox pad), which no widget
    /// acts on: the player has picked up the pad.
    Pad,
    /// Focus the Save-as field, showing focus.
    Name,
    /// One character typed, as the platform's keyboard reports it.
    Char(char),
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
        Ok("high_contrast") => GooeySkin::high_contrast(),
        Ok("tool") => GooeySkin::tool(),
        Ok("desk") => GooeySkin::desk(),
        Ok("desk_light") => GooeySkin::desk_light(),
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
    .insert_resource(MotionSettings {
        duration_scale: std::env::var("GOOEY_MOTION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0),
        reduced: std::env::var("GOOEY_MOTION_REDUCED").is_ok(),
    })
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
            dump_accessibility.after(capture),
            stage_keys.after(capture),
            stage_legend.after(capture),
        ),
    );
    if let Some(at) = std::env::var("GOOEY_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        // GOOEY_FIXED_CLOCK=1: every frame is 1/30 s of game time, whatever the host's load, so a
        // recording of an easing interface is the same each time (the frame time then reads 33.333).
        if std::env::var_os("GOOEY_FIXED_CLOCK").is_some() {
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_micros(33_333),
            ));
        }
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "gallery.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: std::env::var("GOOEY_SHOT_FROM")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(KEY_FROM),
            every: std::env::var("GOOEY_SHOT_EVERY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            key_every: std::env::var("GOOEY_KEY_EVERY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(KEY_EVERY),
            keys: std::env::var("GOOEY_KEYS")
                .unwrap_or_default()
                .split(',')
                .flat_map(|k| -> Vec<Staged> {
                    let k = k.trim();
                    if let Some(word) = k.strip_prefix("type=") {
                        return word.chars().map(Staged::Char).collect();
                    }
                    let staged = match k {
                        "up" => Some(Staged::Key(KeyCode::ArrowUp)),
                        "down" => Some(Staged::Key(KeyCode::ArrowDown)),
                        "left" => Some(Staged::Key(KeyCode::ArrowLeft)),
                        "right" => Some(Staged::Key(KeyCode::ArrowRight)),
                        "enter" => Some(Staged::Key(KeyCode::Enter)),
                        "escape" => Some(Staged::Key(KeyCode::Escape)),
                        "tab" => Some(Staged::Key(KeyCode::Tab)),
                        "space" => Some(Staged::Key(KeyCode::Space)),
                        "q" => Some(Staged::Key(KeyCode::KeyQ)),
                        "f" => Some(Staged::Key(KeyCode::KeyF)),
                        "home" => Some(Staged::Key(KeyCode::Home)),
                        "end" => Some(Staged::Key(KeyCode::End)),
                        "pagedown" => Some(Staged::Key(KeyCode::PageDown)),
                        "pageup" => Some(Staged::Key(KeyCode::PageUp)),
                        "slots" => Some(Staged::Slots),
                        "wheel" => Some(Staged::Wheel),
                        "nexttab" => Some(Staged::NextTab),
                        "prevtab" => Some(Staged::PrevTab),
                        "tab1" => Some(Staged::Tab(0)),
                        "tab2" => Some(Staged::Tab(1)),
                        "tab3" => Some(Staged::Tab(2)),
                        "back" => Some(Staged::Back),
                        "forward" => Some(Staged::Forward),
                        "click" => Some(Staged::Click),
                        "untip" => Some(Staged::Untip),
                        "retip" => Some(Staged::Retip),
                        "hover" => Some(Staged::Hover),
                        "grab" => Some(Staged::Grab),
                        "drag" => Some(Staged::Drag),
                        "drop" => Some(Staged::Drop),
                        "hit" => Some(Staged::Health(-40.0)),
                        "heal" => Some(Staged::Health(40.0)),
                        "toast" => Some(Staged::Toast),
                        "alert" => Some(Staged::Alert),
                        "long" => Some(Staged::Long),
                        "plug" => Some(Staged::Plug),
                        "legend" => Some(Staged::Legend),
                        "name" => Some(Staged::Name),
                        "pad" => Some(Staged::Pad),
                        _ => None,
                    };
                    staged.into_iter().collect()
                })
                .collect(),
        });
    }
    if std::env::var_os("GOOEY_WORLD").is_some() {
        app.add_systems(Startup, spawn_world);
    }
    if std::env::var_os("GOOEY_A11Y").is_some() {
        app.world().resource::<AccessibilityRequested>().set(true);
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
        Ok("bottomcenter") => ToastAnchor::BottomCenter,
        _ => ToastAnchor::BottomRight,
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
            Node {
                column_gap: Val::Px(24.0),
                row_gap: Val::Px(24.0),
                // Two auto-width panels side by side at 100%, stacked when the player's text size
                // makes them wider than the window: a gallery that overflows its own window is not
                // the 200% promise the crate makes.
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                max_width: Val::Percent(100.0),
                ..default()
            },
            children![
                (
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
                                    Tooltip::new("Sound, controls and display"),
                                    observe(switch_skin),
                                ),
                                (button("Quit"), observe(open_quit_dialog)),
                            ]
                        ),
                        label("Health"),
                        (meter(100.0, 100.0), Health),
                        (
                            tabs(
                                ["Audio", "Display", "Controls"],
                                0,
                                children![
                                    tab_page(children![
                                        (
                                            slider("Volume", 0.7, 0.0..=1.0),
                                            observe(
                                                |change: On<ValueChange<f32>>,
                                                 mut commands: Commands| {
                                                    if change.is_final {
                                                        info!("gooey: volume {:.2}", change.value);
                                                        commands.spawn(toast(format!(
                                                            "Volume {:.0}%",
                                                            change.value * 100.0
                                                        )));
                                                    }
                                                }
                                            ),
                                        ),
                                        (
                                            toggle("Subtitles", true),
                                            observe(
                                                |change: On<ValueChange<bool>>,
                                                 mut commands: Commands| {
                                                    info!("gooey: subtitles {}", change.value);
                                                    commands.spawn(toast(if change.value {
                                                        "Subtitles on"
                                                    } else {
                                                        "Subtitles off"
                                                    }));
                                                }
                                            ),
                                        ),
                                    ]),
                                    tab_page(children![
                                        (
                                            choice("Quality", QUALITY, 1),
                                            observe(
                                                |change: On<ValueChange<usize>>,
                                                 mut commands: Commands| {
                                                    info!("gooey: quality {}", change.value);
                                                    commands.spawn(toast(format!(
                                                        "Quality {}",
                                                        QUALITY.get(change.value).copied().unwrap_or("?")
                                                    )));
                                                }
                                            ),
                                        ),
                                        skin_preset("Theme"),
                                    ]),
                                    tab_page(children![
                                        (
                                            binding("Jump", Binding::Key(KeyCode::Space)),
                                            observe(
                                                |change: On<ValueChange<Binding>>,
                                                 mut commands: Commands| {
                                                    let name = change.value.label(PadStyle::Xbox);
                                                    info!("gooey: jump is {name}");
                                                    commands.spawn(toast(format!("Jump is {name}")));
                                                }
                                            ),
                                        ),
                                        binding("Use", Binding::Pad(GamepadButton::South)),
                                    ]),
                                ]
                            ),
                            observe(|change: On<ValueChange<usize>>| {
                                info!("gooey: tab {}", change.value);
                            }),
                        ),
                    ]
                ),
                (
                    panel(),
                    children![
                        label("Load game"),
                        list(
                            4.5,
                            Children::spawn(SpawnIter(SLOTS.iter().map(|&name| (
                                button(name),
                                SaveSlot,
                                observe(move |_: On<Activate>, mut commands: Commands| {
                                    info!("gooey: load {name}");
                                    commands.spawn(toast(format!("Loaded {name}")));
                                }),
                            ))))
                        ),
                        (
                            text_input("Save as", ""),
                            TextHint("Name this save".into()),
                            TextLimit(16),
                            observe(|change: On<ValueChange<String>>, mut commands: Commands| {
                                if change.is_final && !change.value.is_empty() {
                                    info!("gooey: save as {}", change.value);
                                    commands.spawn(toast(format!("Saved as {}", change.value)));
                                }
                            }),
                        ),
                    ]
                ),
            ]
        ),
        (
            prompts(),
            children![prompt("Select", Prompt::SELECT), prompt("Back", Prompt::BACK)],
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
        confirm(
            "Leave the game?",
            "Progress since the last autosave is lost.",
            ["Leave", "Stay"],
        ),
        QuitDialog,
        observe(
            |chosen: On<ConfirmChosen>, mut exit: MessageWriter<AppExit>| {
                if chosen.cancelled {
                    info!("cancel: the Quit dialog is closed");
                } else if chosen.index == 0 {
                    exit.write(AppExit::Success);
                }
            },
        ),
        children![(
            prompts(),
            children![
                prompt("Select", Prompt::SELECT),
                prompt("Back", Prompt::BACK)
            ],
        )],
    ));
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

/// `GOOEY_A11Y`: at the capture frame, print every node of the accessibility tree, nested as the
/// screen reader is told it.
fn dump_accessibility(
    capture: Option<Res<Capture>>,
    requested: Option<Res<AccessibilityRequested>>,
    nodes: Query<(
        Entity,
        &AccessibilityNode,
        Option<&Children>,
        Option<&ChildOf>,
    )>,
) {
    let Some(capture) = capture else {
        return;
    };
    if capture.frame != capture.at || !requested.is_some_and(|r| r.get()) {
        return;
    }
    fn walk(
        entity: Entity,
        depth: usize,
        nodes: &Query<(
            Entity,
            &AccessibilityNode,
            Option<&Children>,
            Option<&ChildOf>,
        )>,
    ) {
        let Ok((_, node, children, _)) = nodes.get(entity) else {
            return;
        };
        let mut flags = String::new();
        for (on, name) in [
            (node.is_hidden(), " hidden"),
            (node.is_disabled(), " disabled"),
            (node.is_modal(), " modal"),
        ] {
            if on {
                flags.push_str(name);
            }
        }
        if let Some(on) = node.is_selected() {
            flags.push_str(if on { " selected" } else { " unselected" });
        }
        if let Some(on) = node.is_expanded() {
            flags.push_str(if on { " expanded" } else { " collapsed" });
        }
        if let Some(toggled) = node.toggled() {
            flags.push_str(&format!(" {toggled:?}"));
        }
        if let Some(n) = node.numeric_value() {
            flags.push_str(&format!(" #{n}"));
        }
        println!(
            "gooey: a11y {}{:?} {:?} {:?}{flags}",
            "  ".repeat(depth),
            node.role(),
            node.label().unwrap_or(""),
            node.value().unwrap_or("")
        );
        for child in children.into_iter().flat_map(|c| c.iter()) {
            walk(child, depth + 1, nodes);
        }
    }
    for (entity, _, _, parent) in &nodes {
        if parent.is_none_or(|parent| nodes.get(parent.parent()).is_err()) {
            walk(entity, 0, &nodes);
        }
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
        // A fixed clock pins `Time<Real>`'s delta, so its mean is a constant, not a measurement: say
        // so, or a later pass quotes 33.333 as a baseline (docs/research/perf-gate.md).
        let pinned = std::env::var_os("GOOEY_FIXED_CLOCK").is_some();
        let note = if pinned {
            " (fixed clock: not a measurement)"
        } else {
            ""
        };
        println!("gooey: frames {} mean_ms {mean:.3}{note}", c.at);
    }
    if let Some(dir) = c.dir.clone() {
        let every = c.every.max(1);
        let due = c.frame >= c.from && (c.frame - c.from) % every == 0;
        if due {
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

/// `GOOEY_KEYS` token `legend`: every legend to the next anchor, a live change of its component.
fn stage_legend(capture: Option<Res<Capture>>, mut legends: Query<&mut PromptsAnchor>) {
    let Some(c) = capture else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.key_every.max(2);
    if since % every != 0 || !matches!(c.keys.get((since / every) as usize), Some(Staged::Legend)) {
        return;
    }
    for mut anchor in &mut legends {
        *anchor = match *anchor {
            PromptsAnchor::BottomLeft => PromptsAnchor::BottomCenter,
            PromptsAnchor::BottomCenter => PromptsAnchor::BottomRight,
            PromptsAnchor::BottomRight => PromptsAnchor::TopLeft,
            PromptsAnchor::TopLeft => PromptsAnchor::TopCenter,
            PromptsAnchor::TopCenter => PromptsAnchor::TopRight,
            PromptsAnchor::TopRight => PromptsAnchor::BottomLeft,
        };
    }
}

/// `GOOEY_KEYS`: one input every `GOOEY_KEY_EVERY` frames ([`KEY_EVERY`]), pressed and then released as the window's
/// own keyboard or pointer input, so Bevy's input dispatch, picking, Tab navigation and buttons all
/// see it.
#[allow(clippy::too_many_arguments)]
fn stage_keys(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    tipped: Query<Entity, With<Tooltip>>,
    thumbs: Query<(&Role, &UiGlobalTransform)>,
    inputs: Query<Entity, With<bevy::ui_widgets::TextInput>>,
    slots: Query<(Entity, &UiGlobalTransform), With<SaveSlot>>,
    mut focus: ResMut<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    mut held: ResMut<HeldHealth>,
    health: Query<&Meter, With<Health>>,
    mut commands: Commands,
    mut keys: MessageWriter<KeyboardInput>,
    mut pointer: MessageWriter<PointerInput>,
    pads: Query<Entity, With<Gamepad>>,
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
        Some(&arrow @ (Staged::Back | Staged::Forward)) => {
            let Some(target) = WindowRef::Entity(window).normalize(None) else {
                return;
            };
            let arrows = thumbs
                .iter()
                .filter(|(role, _)| **role == Role::ChoiceArrow);
            let x = |(_, at): &(&Role, &UiGlobalTransform)| at.translation.x;
            let picked = if matches!(arrow, Staged::Back) {
                arrows.min_by(|a, b| x(a).total_cmp(&x(b)))
            } else {
                arrows.max_by(|a, b| x(a).total_cmp(&x(b)))
            };
            let Some((_, arrow)) = picked else {
                return;
            };
            let at = Location {
                target: NormalizedRenderTarget::Window(target),
                position: arrow.translation,
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
                commands.entity(widget).insert(Tooltip::new(
                    "Unlocked: sound, controls, display and cheats",
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
        Some(Staged::Name) if state == ButtonState::Pressed => {
            if let Some(field) = inputs.iter().next() {
                focus.set(field, FocusCause::Navigated);
                shown.0 = true;
            }
        }
        Some(&Staged::Char(c)) => {
            let text = c.to_string();
            keys.write(KeyboardInput {
                key_code: KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Unidentified),
                logical_key: Key::Character(text.as_str().into()),
                state,
                text: (state == ButtonState::Pressed).then(|| text.as_str().into()),
                repeat: false,
                window,
            });
        }
        Some(Staged::Slots) if state == ButtonState::Pressed => {
            let first = slots
                .iter()
                .min_by(|a, b| a.1.translation.y.total_cmp(&b.1.translation.y));
            if let Some((slot, _)) = first {
                focus.set(slot, FocusCause::Navigated);
                shown.0 = true;
            }
        }
        Some(Staged::Wheel) if state == ButtonState::Pressed => {
            let Some(target) = WindowRef::Entity(window).normalize(None) else {
                return;
            };
            let Some((_, list)) = thumbs.iter().find(|(role, _)| **role == Role::ListView) else {
                return;
            };
            let at = Location {
                target: NormalizedRenderTarget::Window(target),
                position: list.translation,
            };
            pointer.write(PointerInput::new(
                PointerId::Mouse,
                at.clone(),
                PointerAction::Move { delta: Vec2::ZERO },
            ));
            pointer.write(PointerInput::new(
                PointerId::Mouse,
                at,
                PointerAction::Scroll {
                    unit: MouseScrollUnit::Line,
                    x: 0.0,
                    y: -1.0,
                    phase: TouchPhase::Moved,
                },
            ));
        }
        Some(&turn @ (Staged::NextTab | Staged::PrevTab)) => {
            let page = if matches!(turn, Staged::NextTab) {
                KeyCode::PageDown
            } else {
                KeyCode::PageUp
            };
            for key_code in [KeyCode::ControlLeft, page] {
                keys.write(KeyboardInput {
                    key_code,
                    logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                    state,
                    text: None,
                    repeat: false,
                    window,
                });
            }
        }
        Some(&Staged::Tab(place)) => {
            let Some(target) = WindowRef::Entity(window).normalize(None) else {
                return;
            };
            let mut tabs: Vec<&UiGlobalTransform> = thumbs
                .iter()
                .filter(|(role, _)| **role == Role::Tab)
                .map(|(_, at)| at)
                .collect();
            tabs.sort_by(|a, b| a.translation.x.total_cmp(&b.translation.x));
            let Some(tab) = tabs.get(place) else {
                return;
            };
            let at = Location {
                target: NormalizedRenderTarget::Window(target),
                position: tab.translation,
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
        Some(Staged::Plug) if state == ButtonState::Pressed => {
            let pad = commands.spawn_empty().id();
            connections.write(GamepadConnectionEvent::new(
                pad,
                GamepadConnection::Connected {
                    name: "Gallery pad".into(),
                    vendor_id: None,
                    product_id: None,
                },
            ));
        }
        Some(Staged::Pad) => {
            if let Some(pad) = pads.iter().next() {
                let value = if state == ButtonState::Pressed {
                    1.0
                } else {
                    0.0
                };
                pad_input.write(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                    pad,
                    GamepadButton::West,
                    value,
                )));
            }
        }
        Some(Staged::Health(by)) if state == ButtonState::Pressed => {
            if let Ok(meter) = health.single() {
                held.0 = Some((held.0.unwrap_or(meter.value) + by).clamp(0.0, meter.max));
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
                "Quest updated: bring the lighthouse keeper three crates of lamp oil",
            ));
        }
        _ => {}
    }
}
