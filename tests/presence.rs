//! Presence as a game sees it, in a headless window on a clock the test sets: a dialog that is
//! dismissed keeps its card and its veil drawn while they thin out, a press during the exit reaches
//! the screen under it, `Stay` turns the fade round from where it is drawn, `Opacity` multiplies a
//! whole subtree, and nothing is written once everything has settled.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedUiRenderTargetInfo, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::press;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 0.016;

/// Everything a restyle or a fade writes.
type Written = Or<(
    Changed<BackgroundColor>,
    Changed<BorderColor>,
    Changed<TextColor>,
    Changed<UiTransform>,
    Changed<Opacity>,
)>;

#[derive(Resource, Default)]
struct Writes(u32);

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1280, 720).with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>(),
        GooeyPlugins,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )))
    .init_resource::<Writes>()
    .add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    app.world_mut().spawn(Camera2d);
    // The first update only starts the clock.
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// One Update, no clock: enough for a command queue and a restyle.
fn tick(app: &mut App) {
    app.update();
}

/// Where `entity` is in the window, logical px (the scale factor is 1).
fn placed(app: &App, entity: Entity) -> Vec2 {
    let centre = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("laid out")
        .translation;
    // Layout is in physical px; the pointer is in the target's logical ones.
    let scale = app
        .world()
        .get::<ComputedUiRenderTargetInfo>(entity)
        .map_or(1.0, |target| target.scale_factor());
    centre / scale
}

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("a window");
    let location = Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    };
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, location, action));
}

fn point_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ONE });
    frames(app, 2);
}

/// The screen the dialog stands on, and the Quit button that opens it.
/// The open Quit dialog.
#[derive(Component)]
struct QuitDialog;

/// The dialog's card: its `panel()`.
fn the_card(app: &App, dialog: Entity) -> Entity {
    app.world()
        .get::<Children>(dialog)
        .expect("a card")
        .iter()
        .find(|&child| app.world().get::<Role>(child) == Some(&Role::Panel))
        .expect("the dialog's panel")
}

/// The opacity of a widget's fill as it is drawn, 0 to 1.
fn fill_alpha(app: &App, entity: Entity) -> f32 {
    Srgba::from(
        app.world()
            .get::<BackgroundColor>(entity)
            .expect("a fill")
            .0,
    )
    .alpha
}

#[test]
fn escape_keeps_the_card_drawn_while_it_thins_out_and_then_it_is_gone() {
    let mut app = app();
    let dialog = spawn_dialog(&mut app);
    let card = the_card(&app, dialog);
    let open_fill = fill_alpha(&app, card);
    assert!(open_fill > 0.0, "the card is drawn while it is open");

    press(&mut app, KeyCode::Escape);
    tick(&mut app);
    assert!(
        app.world().get_entity(dialog).is_ok(),
        "the dialog is still drawn the frame Back closes it"
    );
    let mut seen = Vec::new();
    for _ in 0..15 {
        if app.world().get_entity(dialog).is_err() {
            break;
        }
        seen.push(fill_alpha(&app, card));
        app.update();
    }
    assert!(
        seen.len() >= 3,
        "the exit is more than a frame long: {seen:?}"
    );
    assert!(
        seen.iter().all(|a| *a > 0.0 && *a < 1.0),
        "every frame of the exit is partway: {seen:?}"
    );
    assert!(
        seen.windows(2).all(|pair| pair[1] <= pair[0]),
        "the card thins out: {seen:?}"
    );
    assert!(
        app.world().get_entity(dialog).is_err(),
        "gone within `motion.default` plus a frame"
    );
}

/// A dialog as a game writes it, open in its first frame.
fn spawn_dialog(app: &mut App) -> Entity {
    let dialog = app
        .world_mut()
        .spawn((
            confirm("Leave the game?", "Progress is lost.", ["Leave", "Stay"]),
            QuitDialog,
        ))
        .id();
    frames(app, 2);
    dialog
}

#[test]
fn a_press_during_the_exit_lands_on_what_is_under_the_scrim() {
    let mut app = app();
    let quit = app.world_mut().spawn(button("Options")).id();
    let screen = app.world_mut().spawn(screen()).id();
    app.world_mut().entity_mut(screen).add_child(quit);
    frames(&mut app, 3);
    let dialog = spawn_dialog(&mut app);
    let at = placed(&app, quit);
    press(&mut app, KeyCode::Escape);
    tick(&mut app);
    assert!(app.world().get_entity(dialog).is_ok(), "still drawn");

    point_at(&mut app, at);
    // The hit is the topmost node the pointer finds, which for a button is the words in it: it is
    // the screen's, not the exit's, when it or an ancestor is the button.
    let hit = app
        .world()
        .resource::<HoverMap>()
        .get(&PointerId::Mouse)
        .and_then(|hits| hits.keys().next().copied())
        .expect("the pointer is over the screen's button");
    let world = app.world();
    let mut node = hit;
    let under_quit = loop {
        if node == quit {
            break true;
        }
        match world.get::<ChildOf>(node) {
            Some(parent) => node = parent.parent(),
            None => break false,
        }
    };
    assert!(
        under_quit,
        "the exit takes no pointer, so the screen under it is what is hit: {hit:?} is not under {quit:?}"
    );
    assert_ne!(hit, dialog, "and nothing of the exit is hit at all");
}

/// The largest change of a fade's opacity between two frames of a run, and how many frames it took.
fn largest_step(app: &mut App, panel: Entity) -> (f32, usize) {
    let mut seen = Vec::new();
    for _ in 0..40 {
        if app.world().get_entity(panel).is_err() {
            break;
        }
        seen.push(fill_alpha(app, panel));
        app.update();
    }
    let step = seen
        .windows(2)
        .map(|pair| (pair[0] - pair[1]).abs())
        .fold(0.0f32, f32::max);
    (step, seen.len())
}

#[test]
fn stay_turns_the_fade_round_from_where_it_is_drawn() {
    // An uninterrupted exit, from its first frame: the largest step it takes.
    let mut plain = app();
    let going_dialog = spawn_dialog(&mut plain);
    let going_card = the_card(&plain, going_dialog);
    plain.world_mut().entity_mut(going_dialog).insert(Leave);
    let (exit_step, frames) = largest_step(&mut plain, going_card);
    assert!(
        plain.world().get_entity(going_dialog).is_err(),
        "an exit ends"
    );
    assert!(frames > 1 && exit_step > 0.0, "the exit moves");

    // The same exit, taken back after 0.05 s: three frames at 60 Hz.
    let mut app = app();
    let dialog = spawn_dialog(&mut app);
    let card = the_card(&app, dialog);
    app.world_mut().entity_mut(dialog).insert(Leave);
    for _ in 0..3 {
        app.update();
    }
    let partway = fill_alpha(&app, card);
    assert!(partway > 0.0 && partway < 1.0, "partway out: {partway}");

    app.world_mut().entity_mut(dialog).insert(Stay);
    let mut back = vec![partway];
    for _ in 0..30 {
        app.update();
        back.push(fill_alpha(&app, card));
    }
    assert!(
        app.world().get_entity(dialog).is_ok(),
        "it came back instead of going"
    );
    let back_step = back
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0f32, f32::max);
    assert!(
        back_step <= exit_step + 1e-6,
        "no step on the way back is larger than the exit's: {back_step} against {exit_step}"
    );
    assert!(
        back.iter().any(|alpha| (*alpha - 1.0).abs() < 1e-3),
        "it comes all the way back: {back:?}"
    );
    assert!(
        app.world().get::<Leaving>(dialog).is_none(),
        "the exit is over"
    );
    assert!(app.world().get::<Stay>(dialog).is_none(), "and spent");
}

#[test]
fn an_opacity_halves_a_whole_subtree_and_leaves_the_hosts_own_text_alone() {
    let mut app = app();
    let panel = app
        .world_mut()
        .spawn((
            panel(),
            Opacity(0.5),
            children![heading("Halved"), button("Press")],
        ))
        .id();
    // A host's own text, with no `Role`, inside the faded panel.
    let host_text = app.world_mut().spawn(Text::new("the host's own")).id();
    app.world_mut().entity_mut(panel).add_child(host_text);
    frames(&mut app, 3);
    let skin_alpha = Srgba::from(GooeySkin::dark().surface).alpha;
    let fill = app.world().get::<BackgroundColor>(panel).expect("fill").0;
    assert!(
        (Srgba::from(fill).alpha - skin_alpha * 0.5).abs() < 1e-3,
        "the panel's own fill is half what the skin says: {fill:?}"
    );
    let card = app.world().get::<Children>(panel).expect("children")[1];
    let button_fill = app.world().get::<BackgroundColor>(card).expect("fill").0;
    assert!(
        (Srgba::from(button_fill).alpha - 0.5).abs() < 1e-3,
        "the button's fill is half, not whole: {button_fill:?}"
    );
    let words = app.world().get::<Children>(card).expect("a label")[0];
    let ink = app.world().get::<TextColor>(words).expect("ink").0;
    assert!(
        (Srgba::from(ink).alpha - 0.5).abs() < 1e-3,
        "and so is its text"
    );
    // The host's own text has no `Role`: the crate leaves its colour exactly as Bevy spawned it.
    assert_eq!(
        app.world().get::<TextColor>(host_text).map(|ink| ink.0),
        Some(TextColor::default().0),
        "a host's own text is the host's"
    );
}

#[test]
fn with_motion_off_a_leave_is_gone_in_the_frame_it_is_asked() {
    let mut app = app();
    app.insert_resource(MotionSettings::off());
    let dialog = spawn_dialog(&mut app);
    press(&mut app, KeyCode::Escape);
    tick(&mut app);
    assert!(
        app.world().get_entity(dialog).is_err(),
        "no exit at all with the player's motion off"
    );
}

#[test]
fn a_second_leave_is_one_exit_and_a_plain_despawn_is_still_immediate() {
    let mut app = app();
    let dialog = spawn_dialog(&mut app);
    press(&mut app, KeyCode::Escape);
    tick(&mut app);
    assert!(app.world().get::<Leaving>(dialog).is_some());
    app.world_mut().entity_mut(dialog).insert(Leave);
    tick(&mut app);
    let exits = {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<Leaving>>()
            .iter(world)
            .count()
    };
    assert_eq!(exits, 1, "one exit, not two");
    // A plain despawn removes at once, as every test that closes a dialog that way relies on.
    let plain = spawn_dialog(&mut app);
    app.world_mut().entity_mut(plain).despawn();
    tick(&mut app);
    assert!(app.world().get_entity(plain).is_err());
}

#[test]
fn a_settled_interface_writes_nothing_for_thirty_frames() {
    let mut app = app();
    let _dialog = spawn_dialog(&mut app);
    // The card's entrance settles first: an exit is not the only motion on screen.
    frames(&mut app, 40);
    let before = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
        let writes = app.world().resource::<Writes>().0 - before;
        assert_eq!(writes, 0, "an open, settled dialog writes nothing");
    }
}

#[test]
fn a_despawned_parent_takes_a_leaving_child_at_once() {
    let mut app = app();
    let parent = app.world_mut().spawn(Node::default()).id();
    let child = app.world_mut().spawn((panel(), ChildOf(parent))).id();
    frames(&mut app, 2);
    app.world_mut().entity_mut(child).insert(Leave);
    tick(&mut app);
    assert!(app.world().get::<Leaving>(child).is_some(), "it is leaving");
    app.world_mut().entity_mut(parent).despawn();
    tick(&mut app);
    assert!(
        app.world().get_entity(child).is_err(),
        "the parent going takes the exit with it"
    );
}

#[test]
fn a_focus_that_was_inside_an_exit_is_let_go() {
    let mut app = app();
    let panel = app
        .world_mut()
        .spawn((panel(), children![button("Inner")]))
        .id();
    frames(&mut app, 2);
    let inner = app.world().get::<Children>(panel).expect("a button")[0];
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(inner, FocusCause::Navigated);
    app.world_mut().entity_mut(panel).insert(Leave);
    tick(&mut app);
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        None,
        "nothing inside an exit keeps focus"
    );
}
