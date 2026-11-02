//! Motion as a game sees it, in the same headless window on a clock the test sets
//! (`TimeUpdateStrategy::ManualDuration`, 30 ms a frame): a dialog's card rises into place and
//! settles, a popover slides out of its anchor only once it is drawn, a paused game's menu still
//! moves, one setting turns every motion off at once or at the start, a game's own `SlideIn` works
//! and is never replaced, and nothing is written once a slide is spent.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// How many `UiTransform`s changed.
#[derive(Resource, Default)]
struct Moves(u32);

/// One frame of the test clock, seconds.
const FRAME: f32 = 0.03;

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
    ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )));
    app.init_resource::<Moves>().add_systems(
        Update,
        |changed: Query<(), Changed<UiTransform>>, mut moves: ResMut<Moves>| {
            moves.0 += changed.iter().count() as u32
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

/// A dialog as a game writes it, and its card.
fn confirm(app: &mut App) -> Entity {
    let card = app
        .world_mut()
        .spawn((
            dialog(),
            children![(
                panel(),
                children![
                    heading("Leave the game?"),
                    (menu(), children![button("Leave")])
                ]
            )],
        ))
        .id();
    app.world()
        .get::<Children>(card)
        .and_then(|c| c.first().copied())
        .expect("the dialog has its card")
}

/// An anchor and a popover opened on it.
fn popped(app: &mut App, placement: Placement) -> Entity {
    let anchor = app.world_mut().spawn(button("Menu")).id();
    app.world_mut()
        .spawn((
            popover(anchor, placement),
            children![button("Copy"), button("Paste")],
        ))
        .id()
}

/// Where `entity`'s picture is moved to, in logical px.
fn offset(app: &App, entity: Entity) -> Vec2 {
    let at = app
        .world()
        .get::<UiTransform>(entity)
        .expect("a node")
        .translation;
    let px = |v: Val| match v {
        Val::Px(px) => px,
        Val::Auto => 0.0,
        other => panic!("not a px translation: {other:?}"),
    };
    Vec2::new(px(at.x), px(at.y))
}

fn sliding(app: &App, entity: Entity) -> bool {
    app.world().get::<SlideIn>(entity).is_some()
}

#[test]
fn a_dialogs_card_rises_from_below_and_settles_in_the_time_it_has() {
    let mut app = app();
    let card = confirm(&mut app);
    app.update();
    let first = offset(&app, card);
    assert_eq!(first.x, 0.0);
    assert!(
        first.y > DIALOG_SLIDE_PX * 0.5 && first.y < DIALOG_SLIDE_PX,
        "the first frame is partway: {first:?}"
    );
    let mut last = first.y;
    // 0.18 s is six frames of 30 ms; every frame before the end is lower than the one before.
    for _ in 0..4 {
        app.update();
        let now = offset(&app, card).y;
        assert!(now < last && now > 0.0, "{now} after {last}");
        last = now;
    }
    frames(&mut app, 3);
    assert_eq!(offset(&app, card), Vec2::ZERO);
    assert!(!sliding(&app, card), "a spent slide takes itself off");
}

#[test]
fn with_motion_off_a_card_is_in_place_in_its_first_frame_and_never_slides() {
    let mut app = app();
    app.insert_resource(MotionSettings::off());
    let card = confirm(&mut app);
    app.update();
    assert_eq!(offset(&app, card), Vec2::ZERO);
    assert!(!sliding(&app, card));
}

#[test]
fn turning_motion_off_ends_a_running_slide_in_the_next_frame() {
    let mut app = app();
    let card = confirm(&mut app);
    frames(&mut app, 2);
    assert!(offset(&app, card).y > 0.0);
    app.insert_resource(MotionSettings::off());
    app.update();
    assert_eq!(offset(&app, card), Vec2::ZERO);
    assert!(!sliding(&app, card));
}

#[test]
fn a_slower_scale_takes_proportionally_longer() {
    let mut app = app();
    app.insert_resource(MotionSettings {
        duration_scale: 2.0,
    });
    let card = confirm(&mut app);
    // 0.18 s × 2 is twelve frames: still moving after eight, settled after thirteen.
    frames(&mut app, 8);
    assert!(offset(&app, card).y > 0.0);
    frames(&mut app, 5);
    assert_eq!(offset(&app, card), Vec2::ZERO);
}

#[test]
fn a_paused_games_dialog_still_slides() {
    let mut app = app();
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    let card = confirm(&mut app);
    frames(&mut app, 2);
    let mid = offset(&app, card).y;
    assert!(mid > 0.0 && mid < DIALOG_SLIDE_PX, "{mid}");
    frames(&mut app, 6);
    assert_eq!(offset(&app, card), Vec2::ZERO);
}

#[test]
fn the_command_palettes_card_rises_like_a_dialogs() {
    let mut app = app();
    let palette = app.world_mut().spawn(command_palette()).id();
    frames(&mut app, 2);
    let card = app
        .world_mut()
        .query_filtered::<Entity, With<SlideIn>>()
        .iter(app.world())
        .find(|&e| {
            app.world()
                .get::<ChildOf>(e)
                .is_some_and(|p| p.parent() == palette)
        })
        .expect("the palette's card slides");
    assert!(offset(&app, card).y > 0.0);
}

#[test]
fn a_panel_on_a_screen_does_not_slide() {
    let mut app = app();
    let screen = app
        .world_mut()
        .spawn((screen(), children![(panel(), children![label("HUD")])]))
        .id();
    frames(&mut app, 3);
    let card = app.world().get::<Children>(screen).expect("child")[0];
    assert!(!sliding(&app, card));
    assert_eq!(offset(&app, card), Vec2::ZERO);
}

#[test]
fn a_popover_slides_out_of_its_anchor_and_only_once_it_is_drawn() {
    let mut app = app();
    let panel = popped(&mut app, Placement::Below);
    let mut hidden_frames = 0;
    let mut revealed_at = None;
    for _ in 0..4 {
        app.update();
        let hidden =
            *app.world().get::<Visibility>(panel).expect("visibility") == Visibility::Hidden;
        if hidden {
            hidden_frames += 1;
            assert_eq!(
                offset(&app, panel),
                Vec2::new(0.0, -POPOVER_SLIDE_PX),
                "a hidden panel's clock waits"
            );
        } else if revealed_at.is_none() {
            revealed_at = Some(offset(&app, panel));
        }
    }
    assert!(hidden_frames >= 1, "a popover is born hidden");
    let shown = revealed_at.expect("it is revealed");
    assert!(
        shown.y < 0.0 && shown.y > -POPOVER_SLIDE_PX,
        "the frame it is drawn it is already moving: {shown:?}"
    );
    frames(&mut app, 6);
    assert_eq!(offset(&app, panel), Vec2::ZERO);
    assert!(!sliding(&app, panel));
}

#[test]
fn a_popover_leaves_the_side_of_its_anchor_it_opens_on() {
    for (placement, sign) in [
        (Placement::Below, (0.0, -1.0)),
        (Placement::Above, (0.0, 1.0)),
        (Placement::Right, (-1.0, 0.0)),
        (Placement::Left, (1.0, 0.0)),
    ] {
        let mut app = app();
        let panel = popped(&mut app, placement);
        let at = app.world().get::<SlideIn>(panel);
        // The component is queued by the first update's restyle.
        assert!(at.is_none());
        app.update();
        let from = app.world().get::<SlideIn>(panel).expect("a slide").from;
        assert_eq!(
            from,
            Vec2::new(sign.0 * POPOVER_SLIDE_PX, sign.1 * POPOVER_SLIDE_PX),
            "{placement:?}"
        );
    }
}

#[test]
fn a_game_can_slide_its_own_panel_and_the_crate_never_replaces_it() {
    let mut app = app();
    let own = app
        .world_mut()
        .spawn((Node::default(), SlideIn::new(Vec2::new(-30.0, 0.0), 0.09)))
        .id();
    app.update();
    let first = offset(&app, own);
    assert!(first.x < 0.0 && first.x > -30.0, "{first:?}");
    frames(&mut app, 4);
    assert_eq!(offset(&app, own), Vec2::ZERO);
    assert!(!sliding(&app, own));

    // A card that comes with a slide of its own keeps it.
    let card = app
        .world_mut()
        .spawn((
            dialog(),
            children![(panel(), SlideIn::new(Vec2::new(0.0, 40.0), 0.3))],
        ))
        .id();
    app.update();
    let card = app.world().get::<Children>(card).expect("a card")[0];
    assert_eq!(
        app.world().get::<SlideIn>(card).expect("kept").from,
        Vec2::new(0.0, 40.0)
    );
}

#[test]
fn nothing_is_written_once_a_slide_is_spent() {
    let mut app = app();
    confirm(&mut app);
    frames(&mut app, 12);
    let before = app.world().resource::<Moves>().0;
    for _ in 0..30 {
        app.update();
        let moved = app.world().resource::<Moves>().0 - before;
        assert_eq!(moved, 0, "an idle interface writes no transform");
    }
}
