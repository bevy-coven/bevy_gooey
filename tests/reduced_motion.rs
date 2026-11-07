//! Reduced motion (`MotionSettings::reduced`): nothing travels, and fades still run. A dialog's card
//! and a popover are in place on their first frame and fade in over the skin's `fade_seconds`; a
//! slide running when the player turns it on lands in the next frame while an exit keeps fading; a
//! leave and a stay during the fade-in, or a host's own opacity, never leave it stuck; a meter's
//! fill lands; off still wins; and the Motion option row steps Full, Reduced and Off,
//! writing only `MotionSettings`. In a headless window on a clock the test sets.

use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds: 60 Hz, so "±1 frame" is a sixtieth of a second.
const FRAME: f32 = 1.0 / 60.0;

/// How many frames the test waits for anything to settle: a second.
const PATIENCE_FRAMES: usize = 60;

/// How many frames each resource was written in.
#[derive(Resource, Default)]
struct Writes {
    motion: u32,
    skin: u32,
    scale: u32,
    binding: u32,
}

fn app(settings: MotionSettings) -> App {
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
    ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )))
    .insert_resource(settings)
    .init_resource::<Writes>()
    .add_systems(
        Update,
        (
            |motion: Res<MotionSettings>, mut writes: ResMut<Writes>| {
                writes.motion += u32::from(motion.is_changed())
            },
            |skin: Res<GooeySkin>, mut writes: ResMut<Writes>| {
                writes.skin += u32::from(skin.is_changed())
            },
            |scale: Res<UiScaleSettings>, mut writes: ResMut<Writes>| {
                writes.scale += u32::from(scale.is_changed())
            },
            |binding: Res<BindingSettings>, mut writes: ResMut<Writes>| {
                writes.binding += u32::from(binding.is_changed())
            },
        ),
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

/// A dialog as a game writes it, spawned without a frame run, and its card.
fn confirm(app: &mut App) -> Entity {
    let dialog = app
        .world_mut()
        .spawn((
            dialog(),
            children![(
                panel(),
                children![heading("Leave the game?"), button("Leave")]
            )],
        ))
        .id();
    app.world()
        .get::<Children>(dialog)
        .and_then(|c| c.first().copied())
        .expect("the dialog has its card")
}

/// Where `entity`'s picture is moved to, logical px (0 when it is not moved).
fn offset(app: &App, entity: Entity) -> Vec2 {
    let at = app
        .world()
        .get::<UiTransform>(entity)
        .map_or(Val2::ZERO, |t| t.translation);
    let px = |v: Val| match v {
        Val::Px(px) => px,
        _ => 0.0,
    };
    Vec2::new(px(at.x), px(at.y))
}

/// How opaque `entity` is drawn: its own `Opacity`, else whole.
fn opacity(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<Opacity>(entity)
        .map_or(1.0, Opacity::factor)
}

fn fill_alpha(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
        .alpha()
}

/// Frames from now until `entity` is drawn whole, stepping one at a time; also checks the fade
/// never falls and that the node's picture stands still while it is drawn (a dialog's card at no
/// offset; a popover where its placement puts it).
fn frames_to_whole(app: &mut App, entity: Entity) -> usize {
    let mut last = opacity(app, entity);
    let mut drawn_at = (last > 0.0).then(|| offset(app, entity));
    for frame in 1..=PATIENCE_FRAMES {
        app.update();
        let now = opacity(app, entity);
        assert!(
            now >= last,
            "the fade fell from {last} to {now} at frame {frame}"
        );
        if now > 0.0 {
            let at = offset(app, entity);
            if let Some(was) = drawn_at {
                assert_eq!(at, was, "nothing travels while it fades in (frame {frame})");
            }
            drawn_at = Some(at);
        }
        if now >= 1.0 && app.world().get::<Opacity>(entity).is_none() {
            return frame;
        }
        last = now;
    }
    panic!("not drawn whole after {PATIENCE_FRAMES} frames");
}

#[test]
fn a_dialogs_card_is_in_place_on_its_first_frame_and_fades_in_over_the_fade() {
    let mut whole = app(MotionSettings::default());
    let reference = confirm(&mut whole);
    frames(&mut whole, PATIENCE_FRAMES);
    let settled_alpha = fill_alpha(&whole, reference);

    let mut app = app(MotionSettings::reduced());
    let card = confirm(&mut app);
    app.update();
    assert_eq!(
        offset(&app, card),
        Vec2::ZERO,
        "in place on its first frame"
    );
    assert!(
        app.world().get::<MotionTarget>(card).is_none(),
        "reduced motion gives the card no slide"
    );
    let first = opacity(&app, card);
    assert!(first < 0.2, "the first frame is near invisible: {first}");
    assert!(
        fill_alpha(&app, card) < settled_alpha,
        "the fill is drawn faded too"
    );

    let fade = app.world().resource::<GooeySkin>().motion.fade_seconds;
    let seconds = (frames_to_whole(&mut app, card) + 1) as f32 * FRAME;
    assert!(
        (seconds - fade).abs() <= FRAME + 1e-4,
        "whole after {seconds} s; the fade is {fade} s, ±1 frame"
    );
    assert_eq!(
        fill_alpha(&app, card),
        settled_alpha,
        "drawn as the skin says"
    );
}

#[test]
fn a_popover_opens_in_place_and_fades_in() {
    let mut app = app(MotionSettings::reduced());
    let anchor = app.world_mut().spawn(button("Menu")).id();
    let mut node = app.world_mut().get_mut::<Node>(anchor).expect("a node");
    node.position_type = PositionType::Absolute;
    node.left = Val::Px(600.0);
    node.top = Val::Px(300.0);
    let panel = app
        .world_mut()
        .spawn((
            popover(anchor, Placement::Below),
            children![button("Copy"), button("Paste")],
        ))
        .id();
    app.update();
    assert!(
        app.world().get::<PopoverSlide>(panel).is_none(),
        "reduced motion gives a popover no slide"
    );
    assert!(opacity(&app, panel) < 1.0, "it starts faded");
    // The fade waits while the panel is hidden (its first frame), then lasts the fade.
    let taken = frames_to_whole(&mut app, panel);
    let fade = app.world().resource::<GooeySkin>().motion.fade_seconds;
    let seconds = taken as f32 * FRAME;
    assert!(
        seconds <= fade + 2.0 * FRAME + 1e-4,
        "whole after {seconds} s, the fade is {fade} s"
    );
    assert!(
        seconds >= fade - FRAME - 1e-4,
        "not before the fade: {seconds} s"
    );
}

#[test]
fn turning_reduced_on_lands_a_running_slide_at_once_and_lets_an_exit_finish_its_fade() {
    let mut app = app(MotionSettings::default());
    let card = confirm(&mut app);
    frames(&mut app, 2);
    assert_ne!(offset(&app, card), Vec2::ZERO, "the card is rising");

    let leaving = app.world_mut().spawn(panel()).id();
    app.world_mut().spawn(screen()).add_child(leaving);
    frames(&mut app, 2);
    app.world_mut().entity_mut(leaving).insert(Leave);
    frames(&mut app, 2);
    let mid = opacity(&app, leaving);
    assert!(mid > 0.0 && mid < 1.0, "the exit is under way: {mid}");

    *app.world_mut().resource_mut::<MotionSettings>() = MotionSettings::reduced();
    app.update();
    assert_eq!(
        offset(&app, card),
        Vec2::ZERO,
        "the slide lands in the next frame"
    );
    let next = opacity(&app, leaving);
    assert!(
        next > 0.0 && next < mid,
        "the exit keeps fading, not cut: {mid} then {next}"
    );
    frames(&mut app, PATIENCE_FRAMES);
    assert!(
        app.world().get_entity(leaving).is_err(),
        "and despawns once it has faded"
    );
}

#[test]
fn a_leave_during_the_fade_in_fades_out_from_where_it_had_got_to() {
    let mut app = app(MotionSettings::reduced());
    let card = confirm(&mut app);
    frames(&mut app, 3);
    let got_to = opacity(&app, card);
    assert!(got_to > 0.0 && got_to < 1.0, "fading in: {got_to}");
    let dialog = app.world().get::<ChildOf>(card).expect("a parent").parent();
    app.world_mut().entity_mut(card).insert(Leave);
    app.update();
    assert!(
        opacity(&app, card) <= got_to,
        "it leaves from where it was, never jumping whole"
    );
    frames(&mut app, PATIENCE_FRAMES);
    assert!(app.world().get_entity(card).is_err(), "and is gone");
    assert!(app.world().get_entity(dialog).is_ok());
}

#[test]
fn a_leave_then_a_stay_during_the_fade_in_brings_the_card_back_whole() {
    let mut whole = app(MotionSettings::default());
    let reference = confirm(&mut whole);
    frames(&mut whole, PATIENCE_FRAMES);
    let settled_alpha = fill_alpha(&whole, reference);

    let mut app = app(MotionSettings::reduced());
    let card = confirm(&mut app);
    frames(&mut app, 3);
    app.world_mut().entity_mut(card).insert(Leave);
    app.update();
    app.world_mut().entity_mut(card).insert(Stay);
    frames(&mut app, PATIENCE_FRAMES);
    assert!(
        app.world().get::<Opacity>(card).is_none(),
        "the arrival's opacity is not kept as the card's own"
    );
    assert_eq!(fill_alpha(&app, card), settled_alpha, "drawn whole again");
}

#[test]
fn a_hosts_opacity_written_during_the_fade_in_is_kept() {
    let mut app = app(MotionSettings::reduced());
    let card = confirm(&mut app);
    frames(&mut app, 3);
    app.world_mut().entity_mut(card).insert(Opacity(0.5));
    frames(&mut app, PATIENCE_FRAMES);
    assert_eq!(
        app.world().get::<Opacity>(card),
        Some(&Opacity(0.5)),
        "the host took the card's opacity over"
    );
}

#[test]
fn a_meters_fill_lands_on_its_value() {
    let mut app = app(MotionSettings::reduced());
    // A gain: at full motion the fill rises to it on the snappy spring (a loss is exact anyway).
    let track = app.world_mut().spawn(meter(30.0, 100.0)).id();
    let panel = app.world_mut().spawn(panel()).add_child(track).id();
    app.world_mut().spawn(screen()).add_child(panel);
    frames(&mut app, 3);
    app.world_mut().entity_mut(track).insert(Meter {
        value: 80.0,
        max: 100.0,
    });
    app.update();
    let fill = app
        .world()
        .get::<Children>(track)
        .and_then(|c| {
            c.iter()
                .find(|e| app.world().get::<Role>(*e) == Some(&Role::MeterFill))
        })
        .expect("the fill");
    let Val::Percent(width) = app.world().get::<Node>(fill).expect("a node").width else {
        panic!("the fill's width is a percentage");
    };
    assert!(
        (width - 80.0).abs() < 1e-3,
        "the fill is at the value in the frame it changes: {width}%"
    );
}

#[test]
fn off_wins_over_reduced_and_nothing_fades() {
    let mut app = app(MotionSettings {
        duration_scale: 0.0,
        reduced: true,
    });
    let card = confirm(&mut app);
    app.update();
    assert_eq!(offset(&app, card), Vec2::ZERO);
    assert!(
        app.world().get::<Opacity>(card).is_none(),
        "drawn whole on its first frame"
    );
}

fn tap(app: &mut App, key_code: KeyCode) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window");
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
}

fn selected(app: &App, row: Entity) -> usize {
    app.world().get::<ChoiceValue>(row).expect("a value").0
}

#[test]
fn the_motion_row_steps_full_reduced_off_writing_only_motion_settings() {
    // A player's own slower motion, which Full and Reduced keep.
    let mut app = app(MotionSettings {
        duration_scale: 2.0,
        reduced: false,
    });
    let row = app.world_mut().spawn(motion_row("Motion")).id();
    let card = app.world_mut().spawn(panel()).add_child(row).id();
    app.world_mut().spawn(screen()).add_child(card);
    frames(&mut app, 6);
    assert_eq!(selected(&app, row), 0, "a hand-written scale shows as Full");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(row, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    app.update();
    *app.world_mut().resource_mut::<Writes>() = Writes::default();

    let mut seen = Vec::new();
    for _ in 0..2 {
        tap(&mut app, KeyCode::ArrowRight);
        seen.push(app.world().resource::<MotionSettings>().clone());
    }
    assert_eq!(
        seen,
        [
            MotionSettings {
                duration_scale: 2.0,
                reduced: true
            },
            MotionSettings::off(),
        ],
        "Full → Reduced keeps the player's scale, then Off"
    );
    assert_eq!(selected(&app, row), 2);
    let writes = app.world().resource::<Writes>();
    assert_eq!(
        (writes.motion, writes.skin, writes.scale, writes.binding),
        (2, 0, 0, 0),
        "two steps, two writes of MotionSettings, and nothing else"
    );

    // Back from Off: the designed lengths again.
    tap(&mut app, KeyCode::Home);
    assert_eq!(
        *app.world().resource::<MotionSettings>(),
        MotionSettings::default()
    );

    // A setting the game writes moves the row, with no event.
    *app.world_mut().resource_mut::<MotionSettings>() = MotionSettings::reduced();
    app.update();
    assert_eq!(selected(&app, row), 1);
}
