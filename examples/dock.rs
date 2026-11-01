//! The dock: a workbench of panels built from a layout kept as data. Drag a tab onto another group
//! (its middle, or its tab strip to choose the place), onto a group's edge to split it or onto the
//! dock's own edge; a card follows the pointer and the part the panel will take is lit before it is
//! let go. Escape puts it back. Without a pointer, focus a tab and press Ctrl+Shift and an arrow.
//! Drag a divider to size the groups. Every change is printed as the layout to save.
//!
//! Run `cargo run --release --example dock`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `dock.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS=<step>;<step>;…` stages one step every `GOOEY_KEY_EVERY` (default 10, at least 8)
//! frames from frame 30, separated by `;`: `move:<Tab>><Tab>.<place>` carries the tab titled
//! `<Tab>` with the pointer to the group that holds the tab `<Tab>` (`place` is `into`, `strip`,
//! `left`, `right`, `top`, `bottom`) and lets go, `hold:` the same without letting go (so a still
//! shows the landing), `moveedge:<Tab>.<left|right|top|bottom>` to the dock's own edge,
//! `drag:<dx>,<dy>` the first divider, `focus:<Tab>` a click on a tab, and `chord:<right|left|up|down>`
//! Ctrl+Shift and that arrow, as real keyboard and pointer input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy_gooey::*;

/// Where a staged move lands, in terms of the group it is aimed at.
#[derive(Clone, Copy, PartialEq)]
enum Place {
    Into,
    Strip,
    Left,
    Right,
    Top,
    Bottom,
}

/// A staged step of `GOOEY_KEYS`.
#[derive(Clone)]
enum Step {
    /// Carry the tab titled `.0` to the group of the tab titled `.1`, at `.2`; let go if `.3`.
    Move(String, String, Place, bool),
    /// Carry the tab to the dock's own edge.
    Edge(String, Place),
    /// Click on the tab titled so.
    Click(String),
    /// Drag the first divider by this much, in layout px.
    Drag(Vec2),
    /// Ctrl+Shift and this arrow.
    Chord(KeyCode),
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

/// The frame `GOOEY_KEYS` starts at, after the layout has settled.
const KEY_FROM: u32 = 30;
/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

fn pair(text: &str) -> Option<Vec2> {
    let (x, y) = text.split_once(',')?;
    Some(Vec2::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
}

fn place_of(word: &str) -> Option<Place> {
    Some(match word {
        "into" => Place::Into,
        "strip" => Place::Strip,
        "left" => Place::Left,
        "right" => Place::Right,
        "top" => Place::Top,
        "bottom" => Place::Bottom,
        _ => return None,
    })
}

fn steps_of(text: &str) -> Vec<Step> {
    text.split(';')
        .filter_map(|word| {
            let word = word.trim();
            let carried = word
                .strip_prefix("move:")
                .map(|rest| (rest, true))
                .or_else(|| word.strip_prefix("hold:").map(|rest| (rest, false)));
            if let Some((rest, let_go)) = carried {
                let (from, rest) = rest.split_once('>')?;
                let (to, place) = rest.rsplit_once('.')?;
                return Some(Step::Move(from.into(), to.into(), place_of(place)?, let_go));
            }
            if let Some(rest) = word.strip_prefix("moveedge:") {
                let (tab, place) = rest.rsplit_once('.')?;
                return Some(Step::Edge(tab.into(), place_of(place)?));
            }
            if let Some(tab) = word.strip_prefix("focus:") {
                return Some(Step::Click(tab.into()));
            }
            if let Some(by) = word.strip_prefix("drag:") {
                return Some(Step::Drag(pair(by)?));
            }
            word.strip_prefix("chord:").and_then(|key| {
                Some(Step::Chord(match key {
                    "up" => KeyCode::ArrowUp,
                    "down" => KeyCode::ArrowDown,
                    "left" => KeyCode::ArrowLeft,
                    "right" => KeyCode::ArrowRight,
                    _ => return None,
                }))
            })
        })
        .collect()
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
        title: "bevy_gooey dock".into(),
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
    .add_observer(saved)
    .add_systems(Startup, setup)
    .add_systems(Update, (follow_background, capture, stage.after(capture)));
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "dock.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            step_every: env("GOOEY_KEY_EVERY").unwrap_or(10),
            steps: steps_of(&std::env::var("GOOEY_KEYS").unwrap_or_default()),
        });
    }
    app.run();
}

/// What the readout under the dock says: the panels as the layout lists them.
#[derive(Component)]
struct Readout;

fn setup(mut commands: Commands, mut panels: ResMut<PanelRegistry>) {
    commands.spawn(Camera2d);
    panels
        .register("scene", "Scene", |page| {
            page.insert(children![
                heading("Scene"),
                (
                    menu(),
                    children![
                        button("Meadow"),
                        button("Harbour"),
                        button("Mill"),
                        button("Cellar"),
                    ]
                ),
            ]);
        })
        .register("assets", "Assets", |page| {
            page.insert(children![
                heading("Assets"),
                label("meadow.ron   4 KB"),
                label("harbour.ron  9 KB"),
                label("mill.ron     2 KB"),
            ]);
        })
        .register("viewport", "Viewport", |page| {
            page.insert(children![
                heading("Viewport"),
                label("The scene is drawn here"),
                slider("Zoom", 0.6, 0.0..=1.0),
            ]);
        })
        .register("inspector", "Inspector", |page| {
            page.insert(children![
                heading("Inspector"),
                toggle("Visible", true),
                toggle("Snap to grid", false),
                choice("Quality", ["Low", "High"], 1),
            ]);
        })
        .register("log", "Log", |page| {
            page.insert(children![
                label("12:40  Loaded Meadow"),
                label("12:41  Autosaved"),
                label("12:44  Saved Harbour"),
            ]);
        })
        .register("console", "Console", |page| {
            page.insert(children![label("No errors")]);
        });
    let layout = DockLayout::split(
        SplitAxis::Horizontal,
        0.24,
        DockLayout::group(["scene", "assets"]),
        DockLayout::split(
            SplitAxis::Horizontal,
            0.66,
            DockLayout::split(
                SplitAxis::Vertical,
                0.66,
                DockLayout::group(["viewport"]),
                DockLayout::group(["log", "console"]),
            ),
            DockLayout::group(["inspector"]),
        ),
    );
    commands.spawn((
        screen(),
        children![(
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(24.0)),
                row_gap: Val::Px(12.0),
                ..default()
            },
            children![
                (
                    Node {
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        ..default()
                    },
                    children![dock(layout)],
                ),
                (
                    label("Drag a tab; Ctrl+Shift+arrow moves the focused one"),
                    Readout
                ),
            ],
        )],
    ));
}

/// Prints each layout the player makes, as a game would save it, and says so under the dock.
fn saved(changed: On<DockLayoutChanged>, mut texts: Query<&mut Text, With<Readout>>) {
    let line = format!("Layout to save: {:?}", changed.layout.panels());
    println!("gooey: {line}");
    for mut text in &mut texts {
        text.0 = line.clone();
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

/// A pointer input at `position` (window px) for the primary window.
fn write(
    pointer: &mut MessageWriter<PointerInput>,
    window: Entity,
    position: Vec2,
    action: PointerAction,
) {
    let Some(target) = WindowRef::Entity(window).normalize(None) else {
        return;
    };
    pointer.write(PointerInput::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(target),
            position,
        },
        action,
    ));
}

/// What the staged steps look up: the tabs by title and where they and their groups are laid out.
#[derive(bevy::ecs::system::SystemParam)]
struct Seen<'w, 's> {
    texts: Query<'w, 's, (&'static Text, &'static Role, &'static ChildOf)>,
    parents: Query<'w, 's, &'static ChildOf>,
    frames: Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform)>,
    dividers: Query<'w, 's, &'static UiGlobalTransform, With<SplitDivider>>,
}

impl Seen<'_, '_> {
    /// Where a tab and the group that holds it are, in window px: the tab's centre, then the
    /// group's rect (the card, three nodes up).
    fn find(&self, title: &str, scale: f32) -> Option<(Vec2, Rect)> {
        let (_, _, tab) = self
            .texts
            .iter()
            .find(|(text, role, _)| **role == Role::TabText && text.0 == title)?;
        let tab = tab.parent();
        let rect = |entity: Entity| {
            let (node, at) = self.frames.get(entity).ok()?;
            Some(Rect::from_center_size(
                at.translation / scale,
                node.size() / scale,
            ))
        };
        let mut card = tab;
        for _ in 0..3 {
            card = self.parents.get(card).ok()?.parent();
        }
        Some((rect(tab)?.center(), rect(card)?))
    }
}

/// `GOOEY_KEYS`: one step every `GOOEY_KEY_EVERY` frames, as the window's own input.
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<(Entity, &Window), With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    seen: Seen,
    mut keys: MessageWriter<KeyboardInput>,
    mut pointer: MessageWriter<PointerInput>,
    mut grabbed: Local<(Vec2, Vec2)>,
) {
    let (Some(c), Ok((window, props))) = (capture, window.single()) else {
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
    let scale = props.scale_factor();
    let ui = ui_scale.0;
    // A step carries a point from `from` to `to` (window px): onto it, press, an eased move over
    // the middle frames, and (if `let_go`) the release at the last.
    let carry = |from: Vec2, to: Vec2, let_go: bool, pointer: &mut MessageWriter<PointerInput>| {
        let last = every - 2;
        match phase {
            0 => write(
                pointer,
                window,
                from,
                PointerAction::Move { delta: Vec2::ZERO },
            ),
            1 => write(
                pointer,
                window,
                from,
                PointerAction::Press(PointerButton::Primary),
            ),
            n if n < last => {
                let t = (n - 1) as f32 / (last - 1) as f32;
                let eased = t * t * (3.0 - 2.0 * t);
                let at = from + (to - from) * eased;
                write(
                    pointer,
                    window,
                    at,
                    PointerAction::Move {
                        delta: (to - from) / last as f32,
                    },
                );
            }
            n if n == last && let_go => {
                write(
                    pointer,
                    window,
                    to,
                    PointerAction::Release(PointerButton::Primary),
                );
            }
            _ => {}
        }
    };
    match step {
        Step::Move(tab, onto, place, let_go) => {
            let (Some((from, _)), Some((_, group))) =
                (seen.find(tab, scale), seen.find(onto, scale))
            else {
                return;
            };
            if phase == 0 {
                let (w, h) = (group.width(), group.height());
                let to = match place {
                    Place::Into => group.center() + Vec2::new(0.0, h * 0.1),
                    Place::Strip => Vec2::new(group.max.x - 30.0, group.min.y + 36.0),
                    Place::Left => Vec2::new(group.min.x + w * 0.1, group.center().y),
                    Place::Right => Vec2::new(group.max.x - w * 0.1, group.center().y),
                    Place::Top => Vec2::new(group.center().x, group.min.y + h * 0.18),
                    Place::Bottom => Vec2::new(group.center().x, group.max.y - h * 0.1),
                };
                *grabbed = (from, to);
            }
            carry(grabbed.0, grabbed.1, *let_go, &mut pointer);
        }
        Step::Edge(tab, place) => {
            let Some((from, _)) = seen.find(tab, scale) else {
                return;
            };
            if phase == 0 {
                let size = window_size(props);
                let to = match place {
                    Place::Left => Vec2::new(26.0, size.y * 0.5),
                    Place::Right => Vec2::new(size.x - 26.0, size.y * 0.5),
                    Place::Top => Vec2::new(size.x * 0.5, 26.0),
                    _ => Vec2::new(size.x * 0.5, size.y - 62.0),
                };
                *grabbed = (from, to);
            }
            carry(grabbed.0, grabbed.1, true, &mut pointer);
        }
        Step::Click(tab) => {
            let Some((at, _)) = seen.find(tab, scale) else {
                return;
            };
            match phase {
                0 => write(
                    &mut pointer,
                    window,
                    at,
                    PointerAction::Move { delta: Vec2::ZERO },
                ),
                1 => write(
                    &mut pointer,
                    window,
                    at,
                    PointerAction::Press(PointerButton::Primary),
                ),
                2 => write(
                    &mut pointer,
                    window,
                    at,
                    PointerAction::Release(PointerButton::Primary),
                ),
                _ => {}
            }
        }
        Step::Drag(by) => {
            if phase == 0 {
                let Some(at) = seen.dividers.iter().next() else {
                    return;
                };
                *grabbed = (at.translation / scale, at.translation / scale + *by * ui);
            }
            carry(grabbed.0, grabbed.1, true, &mut pointer);
        }
        Step::Chord(arrow) => {
            // The modifiers go down first and the arrow a few frames later, as a hand does it.
            let (key_code, state) = match phase {
                0 => (KeyCode::ControlLeft, ButtonState::Pressed),
                1 => (KeyCode::ShiftLeft, ButtonState::Pressed),
                4 => (*arrow, ButtonState::Pressed),
                6 => (*arrow, ButtonState::Released),
                8 => (KeyCode::ShiftLeft, ButtonState::Released),
                9 => (KeyCode::ControlLeft, ButtonState::Released),
                _ => return,
            };
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
}

/// The window's size in logical px.
fn window_size(window: &Window) -> Vec2 {
    Vec2::new(window.width(), window.height())
}
