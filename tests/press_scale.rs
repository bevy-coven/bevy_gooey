//! Press scale (`PressScale`): a card held under the pointer shrinks to 0.97 about its centre
//! within the snappy spring while layout and the hit test keep its resting size, comes back on
//! release, is cancelled by a scroll under the hold, and stays still under reduced motion. Laid out
//! for real in a headless window, driven by pointer input, on a 60 Hz clock.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::math::Affine2;
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, Pressed, ScrollPosition, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

/// How near the held scale must come to `PRESS_SCALE`.
const SCALE_TOLERANCE: f32 = 0.005;

/// Every card chosen, by index.
#[derive(Resource, Default)]
struct Chosen(Vec<usize>);

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
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )))
    .insert_resource(settings)
    .init_resource::<Chosen>();
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// Three cards in a scroll view that can scroll; returns the view and the cards.
fn cards(app: &mut App) -> (Entity, Vec<Entity>) {
    let grid = app
        .world_mut()
        .spawn(card_grid((0..3).map(|i| Card::new(format!("Map {i}")))))
        .observe(
            |change: On<ValueChange<usize>>, mut chosen: ResMut<Chosen>| {
                chosen.0.push(change.value);
            },
        )
        .id();
    let view = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(600.0),
                height: Val::Px(300.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
        ))
        .add_child(grid)
        .id();
    app.world_mut().spawn(screen()).add_child(view);
    frames(app, 4);
    let cells = app
        .world()
        .get::<Children>(grid)
        .expect("cards")
        .iter()
        .collect();
    (view, cells)
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one window")
}

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let window = window(app);
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
    app.update();
}

/// A node's resting rectangle (layout), logical px: `(min, max)`.
fn resting(app: &App, entity: Entity) -> Rect {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    // The drawn centre is the resting one (the scale is about it), and layout keeps the size.
    Rect::from_center_size(at, node.size())
}

/// The scale the card is drawn at, read off its drawn transform.
fn drawn_scale(app: &App, entity: Entity) -> f32 {
    let at = Affine2::from(
        *app.world()
            .get::<UiGlobalTransform>(entity)
            .expect("placed"),
    );
    at.matrix2.x_axis.length()
}

fn held_scale(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<PressScaled>(entity)
        .expect("a press scale")
        .scale()
}

#[test]
fn a_card_held_shrinks_to_0_97_within_the_snappy_spring_and_springs_back() {
    let mut app = app(MotionSettings::default());
    let (_, cells) = cards(&mut app);
    let card = cells[0];
    assert!(
        app.world().get::<PressScale>(card).is_some(),
        "a card gives"
    );
    let size = app
        .world()
        .get::<ComputedNode>(card)
        .expect("laid out")
        .size();
    let at = resting(&app, card);
    pointer(
        &mut app,
        at.center(),
        PointerAction::Move { delta: Vec2::X },
    );
    pointer(
        &mut app,
        at.center(),
        PointerAction::Press(PointerButton::Primary),
    );
    let snappy = app.world().resource::<GooeySkin>().motion.snappy.duration;
    // The press's own frame is the spring's first step.
    frames(&mut app, (snappy / FRAME).round() as usize - 1);
    assert!(
        (held_scale(&app, card) - PRESS_SCALE).abs() <= SCALE_TOLERANCE,
        "held for {snappy} s: {}",
        held_scale(&app, card)
    );
    assert!(
        (drawn_scale(&app, card) - held_scale(&app, card)).abs() < 1e-4,
        "it is drawn at that scale"
    );
    assert_eq!(
        app.world()
            .get::<ComputedNode>(card)
            .expect("laid out")
            .size(),
        size,
        "layout keeps its resting size"
    );

    pointer(
        &mut app,
        at.center(),
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(
        app.world().resource::<Chosen>().0,
        [0],
        "the release chose it"
    );
    frames(&mut app, 60);
    assert_eq!(held_scale(&app, card), 1.0, "back at rest");
    assert!((drawn_scale(&app, card) - 1.0).abs() < 1e-6);
}

#[test]
fn a_press_at_the_edge_pixel_holds_while_the_drawing_shrinks_under_it() {
    let mut app = app(MotionSettings::default());
    let (_, cells) = cards(&mut app);
    let card = cells[0];
    let at = resting(&app, card);
    let edge = Vec2::new(at.min.x + 0.5, at.center().y);
    pointer(&mut app, edge, PointerAction::Move { delta: Vec2::X });
    pointer(&mut app, edge, PointerAction::Press(PointerButton::Primary));
    for frame in 0..30 {
        app.update();
        assert!(
            app.world().get::<Hovered>(card).is_some_and(|h| h.0),
            "the edge pixel still hits the card at frame {frame}"
        );
        assert!(app.world().get::<Pressed>(card).is_some());
    }
    assert!((held_scale(&app, card) - PRESS_SCALE).abs() <= SCALE_TOLERANCE);
}

#[test]
fn a_scroll_during_the_hold_cancels_the_press() {
    let mut app = app(MotionSettings::default());
    let (view, cells) = cards(&mut app);
    let card = cells[0];
    let at = resting(&app, card);
    pointer(
        &mut app,
        at.center(),
        PointerAction::Move { delta: Vec2::X },
    );
    pointer(
        &mut app,
        at.center(),
        PointerAction::Press(PointerButton::Primary),
    );
    frames(&mut app, 5);
    app.world_mut()
        .get_mut::<ScrollPosition>(view)
        .expect("a scroll view")
        .y = 1.0;
    frames(&mut app, 2);
    pointer(
        &mut app,
        at.center(),
        PointerAction::Release(PointerButton::Primary),
    );
    frames(&mut app, 2);
    assert!(
        app.world().resource::<Chosen>().0.is_empty(),
        "the release after a scroll chose nothing"
    );
    frames(&mut app, 60);
    assert_eq!(held_scale(&app, card), 1.0, "and it sprang back");
}

#[test]
fn reduced_motion_keeps_a_held_card_at_its_size() {
    let mut app = app(MotionSettings::reduced());
    let (_, cells) = cards(&mut app);
    let card = cells[0];
    let at = resting(&app, card);
    pointer(
        &mut app,
        at.center(),
        PointerAction::Move { delta: Vec2::X },
    );
    pointer(
        &mut app,
        at.center(),
        PointerAction::Press(PointerButton::Primary),
    );
    frames(&mut app, 20);
    assert_eq!(held_scale(&app, card), 1.0);
    assert!((drawn_scale(&app, card) - 1.0).abs() < 1e-6);
}

#[test]
fn a_button_does_not_give_and_an_idle_card_writes_nothing() {
    #[derive(Resource, Default)]
    struct Writes(u32);
    let mut app = app(MotionSettings::default());
    app.init_resource::<Writes>().add_systems(
        Last,
        |changed: Query<(), Changed<PressScaled>>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    let plain = app.world_mut().spawn(button("Save")).id();
    app.world_mut().spawn(screen()).add_child(plain);
    let (_, _) = cards(&mut app);
    frames(&mut app, 2);
    assert!(app.world().get::<PressScale>(plain).is_none());
    app.world_mut().resource_mut::<Writes>().0 = 0;
    frames(&mut app, 30);
    assert_eq!(app.world().resource::<Writes>().0, 0);
}

#[test]
fn a_slot_full_or_empty_gives_and_a_secondary_press_does_not() {
    let mut app = app(MotionSettings::default());
    let grid = app
        .world_mut()
        .spawn(slot_grid(2, [Some(Slot::new("Sword")), None]))
        .id();
    app.world_mut().spawn(screen()).add_child(grid);
    frames(&mut app, 4);
    let mut query = app.world_mut().query::<(Entity, &Role, Has<PressScale>)>();
    let slots: Vec<(Entity, Role, bool)> = query
        .iter(app.world())
        .filter(|(_, role, _)| matches!(role, Role::Slot | Role::SlotEmpty))
        .map(|(e, r, has)| (e, *r, has))
        .collect();
    assert!(slots.iter().any(|(_, r, _)| *r == Role::Slot));
    assert!(slots.iter().any(|(_, r, _)| *r == Role::SlotEmpty));
    assert!(
        slots.iter().all(|(_, _, has)| *has),
        "every slot gives: {slots:?}"
    );

    let (full, _, _) = slots[0];
    let at = resting(&app, full);
    pointer(
        &mut app,
        at.center(),
        PointerAction::Move { delta: Vec2::X },
    );
    pointer(
        &mut app,
        at.center(),
        PointerAction::Press(PointerButton::Secondary),
    );
    frames(&mut app, 20);
    assert_eq!(
        held_scale(&app, full),
        1.0,
        "a secondary press does not shrink it"
    );
    pointer(
        &mut app,
        at.center(),
        PointerAction::Release(PointerButton::Secondary),
    );
    pointer(
        &mut app,
        at.center(),
        PointerAction::Press(PointerButton::Primary),
    );
    frames(&mut app, 20);
    assert!((held_scale(&app, full) - PRESS_SCALE).abs() <= SCALE_TOLERANCE);
}
