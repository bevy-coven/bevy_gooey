use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
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
    step_every: u32,
    steps: Vec<Step>,
}

/// A staged step of `GOOEY_KEYS`.
#[derive(Clone, Debug)]
enum Step {
    Key(KeyCode, Key),
    /// Focus the first button of a toast, as Tab would after a few presses.
    FocusToast,
}

/// The frame `GOOEY_KEYS` starts at, after the layout has settled.
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
        title: "bevy_gooey notifications".into(),
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
    .add_observer(hear)
    .add_systems(Startup, setup)
    .add_systems(Update, (follow_background, capture, apply_steps))
    // Staged keys are written before `PreUpdate`, as a real keyboard's are.
    .add_systems(First, stage);
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "notifications.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            step_every: env("GOOEY_KEY_EVERY").unwrap_or(24),
            steps: steps_of(&std::env::var("GOOEY_KEYS").unwrap_or_default()),
        });
    }
    app.run();
}

fn steps_of(text: &str) -> Vec<Step> {
    text.split(';')
        .filter_map(|word| {
            Some(match word.trim() {
                "toast" => Step::FocusToast,
                "tab" => Step::Key(KeyCode::Tab, Key::Tab),
                "up" => Step::Key(KeyCode::ArrowUp, Key::ArrowUp),
                "down" => Step::Key(KeyCode::ArrowDown, Key::ArrowDown),
                "left" => Step::Key(KeyCode::ArrowLeft, Key::ArrowLeft),
                "right" => Step::Key(KeyCode::ArrowRight, Key::ArrowRight),
                "enter" => Step::Key(KeyCode::Enter, Key::Enter),
                "space" => Step::Key(KeyCode::Space, Key::Space),
                _ => return None,
            })
        })
        .collect()
}

/// The line that says which toast action the game heard last.
#[derive(Component)]
struct Heard;

/// A game's screen: three buttons that spawn toasts, what the game heard, and the log.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let actions = commands
        .spawn((
            panel(),
            children![
                heading("Inventory"),
                (
                    button("Delete item"),
                    observe(|_: On<Activate>, mut commands: Commands| {
                        commands.spawn(toast("Item deleted").action("Undo", "undo"));
                    }),
                ),
                (
                    button("Save game"),
                    observe(|_: On<Activate>, mut commands: Commands| {
                        commands.spawn(toast("Game saved"));
                    }),
                ),
                (
                    button("Break something"),
                    observe(|_: On<Activate>, mut commands: Commands| {
                        commands.spawn(
                            toast_alert("Could not write the save")
                                .action("Show", "show")
                                .sticky(),
                        );
                    }),
                ),
                (label("Heard: nothing"), Heard),
            ],
        ))
        .id();
    let log = commands
        .spawn((
            panel(),
            children![heading("Notifications"), notification_center()],
        ))
        .id();
    commands
        .spawn(screen())
        .insert(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::FlexStart,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(24.0),
            padding: UiRect::top(Val::Px(24.0)),
            ..default()
        })
        .add_children(&[actions, log]);
}

/// What the game does with a toast's button: it hears one `ToastAction` and writes it down.
fn hear(action: On<ToastAction>, mut heard: Query<&mut Text, With<Heard>>) {
    for mut text in &mut heard {
        text.0 = format!("Heard: {}", action.id);
    }
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

/// The step that is not a key: focus a toast's button.
fn apply_steps(
    capture: Option<Res<Capture>>,
    buttons: Query<Entity, With<ToastButton>>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
) {
    let Some(c) = capture else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.step_every.max(8);
    if since % every != 0 {
        return;
    }
    if let Some(Step::FocusToast) = c.steps.get((since / every) as usize)
        && let Some(button) = buttons.iter().next()
    {
        focus.set(button, FocusCause::Navigated);
        visible.0 = true;
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

/// `GOOEY_KEYS`: real keyboard input, one step every `GOOEY_KEY_EVERY` frames.
fn stage(
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
    let every = c.step_every.max(8);
    let Some(step) = c.steps.get((since / every) as usize) else {
        return;
    };
    let phase = since % every;
    let mut key = |key_code: KeyCode, logical_key: Key, text: Option<&str>, state: ButtonState| {
        keys.write(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: text.map(Into::into),
            repeat: false,
            window,
        });
    };
    match step {
        Step::FocusToast => {}
        Step::Key(key_code, logical) => match phase {
            0 => key(*key_code, logical.clone(), None, ButtonState::Pressed),
            n if n == every / 2 => key(*key_code, logical.clone(), None, ButtonState::Released),
            _ => {}
        },
    }
}
