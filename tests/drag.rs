//! Drag and drop with payloads: a source carries a typed value, a target takes it, and a drop is one
//! event. Laid out for real in a headless window, driven by real pointer and keyboard input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

#[derive(Clone, Debug, PartialEq)]
struct Piece(u32);

#[derive(Clone, Debug, PartialEq)]
struct Light(&'static str);

/// Everything a game would hear, in order.
#[derive(Resource, Default)]
struct Heard {
    dropped: Vec<(Entity, Entity, Piece, Vec2)>,
    lights: Vec<(Entity, Light)>,
    refused: Vec<(Entity, Piece)>,
    world: Vec<(Piece, Vec2)>,
    cancels: u32,
    activations: u32,
}

struct Scene {
    wall: Entity,
    lamp: Entity,
    slot: Entity,
    small: Entity,
    lights: Entity,
    both: Entity,
    panel: Entity,
}

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
    .init_resource::<Heard>()
    .add_observer(|on: On<Dropped<Piece>>, mut heard: ResMut<Heard>| {
        heard
            .dropped
            .push((on.target, on.source, on.payload.clone(), on.pointer));
    })
    .add_observer(|on: On<Dropped<Light>>, mut heard: ResMut<Heard>| {
        heard.lights.push((on.target, on.payload.clone()));
    })
    .add_observer(|on: On<DropRefused<Piece>>, mut heard: ResMut<Heard>| {
        heard.refused.push((on.target, on.payload.clone()));
    })
    .add_observer(|on: On<DroppedOnWorld<Piece>>, mut heard: ResMut<Heard>| {
        heard.world.push((on.payload.clone(), on.pointer));
    })
    .add_observer(|cancel: On<Cancel>, mut heard: ResMut<Heard>| {
        // A global observer hears a bubbling event at every step: count it once, where it starts.
        if cancel.entity == cancel.original_event_target() {
            heard.cancels += 1;
        }
    });
    app.world_mut().spawn(Camera2d);
    app
}

/// Two sources (a Wall carrying `Piece(7)` and a Lamp carrying a `Light`) on the left, five targets
/// on the right: a slot that takes any piece, one that takes pieces under 10, one that takes only
/// lights, one that takes both, and a plain panel that is not a target. The rest of the screen is
/// empty: the world.
fn scene(app: &mut App) -> Scene {
    let world = app.world_mut();
    let wall = world.spawn((button("Wall"), DragPayload(Piece(7)))).id();
    let lamp = world
        .spawn((button("Lamp"), DragPayload(Light("warm"))))
        .id();
    let sources = world.spawn(panel()).add_children(&[wall, lamp]).id();
    let slot = world
        .spawn((
            panel(),
            DropTarget::<Piece>::new(),
            children![label("Any piece")],
        ))
        .id();
    let small = world
        .spawn((
            panel(),
            DropTarget::<Piece>::accepting(|piece| piece.0 < 10),
            children![label("Small pieces")],
        ))
        .id();
    let lights = world
        .spawn((
            panel(),
            DropTarget::<Light>::new(),
            children![label("Lights")],
        ))
        .id();
    let both = world
        .spawn((
            panel(),
            DropTarget::<Light>::new(),
            DropTarget::<Piece>::new(),
            children![label("Both")],
        ))
        .id();
    let plain = world
        .spawn((panel(), children![label("Not a target")]))
        .id();
    let targets = world
        .spawn(panel())
        .add_children(&[slot, small, lights, both, plain])
        .id();
    let root = world.spawn(screen()).add_children(&[sources, targets]).id();
    world.entity_mut(root).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(60.0),
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::FlexStart,
        ..default()
    });
    settle(app);
    Scene {
        wall,
        lamp,
        slot,
        small,
        lights,
        both,
        panel: plain,
    }
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn rect(app: &App, entity: Entity) -> Rect {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    let scale = node.inverse_scale_factor();
    Rect::from_center_size(at * scale, node.size() * scale)
}

/// The window position of the middle of `entity`.
fn centre(app: &App, entity: Entity) -> Vec2 {
    rect(app, entity).center() * app.world().resource::<UiScale>().0
}

fn location(app: &mut App, at: Vec2) -> Location {
    let window = window(app);
    Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    }
}

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let at = location(app, at);
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, at, action));
    app.update();
}

fn point_at(app: &mut App, to: Vec2) {
    pointer(app, to, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
}

/// Presses on `from`, moves to `to` and leaves the pointer pressed there.
fn grab_and_move(app: &mut App, from: Vec2, to: Vec2) {
    point_at(app, from);
    pointer(app, from, PointerAction::Press(PointerButton::Primary));
    pointer(app, to, PointerAction::Move { delta: to - from });
    settle(app);
}

fn release(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
}

fn drag(app: &mut App, from: Entity, to: Vec2) {
    let start = centre(app, from);
    grab_and_move(app, start, to);
    release(app, to);
}

fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    let window = window(app);
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

fn heard(app: &App) -> &Heard {
    app.world().resource::<Heard>()
}

fn parts<C: Component>(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .iter(app.world())
        .collect()
}

/// The ghost and the mark a drag draws: every node with one of the drag roles.
fn drag_roles(app: &mut App) -> Vec<(Entity, Role)> {
    app.world_mut()
        .query::<(Entity, &Role)>()
        .iter(app.world())
        .filter(|(_, role)| {
            matches!(
                role,
                Role::DragGhost | Role::DragGhostRefused | Role::DropMark | Role::DropMarkRefused
            )
        })
        .map(|(entity, role)| (entity, *role))
        .collect()
}

#[test]
fn dragging_a_source_onto_a_target_sends_one_dropped() {
    let mut app = app();
    let s = scene(&mut app);
    let to = centre(&app, s.slot);

    drag(&mut app, s.wall, to);

    let heard = heard(&app);
    assert_eq!(heard.dropped.len(), 1, "exactly one Dropped");
    let (target, source, payload, pointer) = &heard.dropped[0];
    assert_eq!((*target, *source), (s.slot, s.wall));
    assert_eq!(*payload, Piece(7));
    assert!(
        (*pointer - to).length() < 1.0,
        "the pointer is where it was let go"
    );
    assert!(heard.refused.is_empty() && heard.world.is_empty() && heard.lights.is_empty());
}

#[test]
fn letting_go_over_empty_space_sends_the_pointer_position_to_the_world() {
    let mut app = app();
    let s = scene(&mut app);
    let empty = Vec2::new(600.0, 660.0);

    drag(&mut app, s.wall, empty);

    let heard = heard(&app);
    assert_eq!(heard.world.len(), 1);
    assert_eq!(heard.world[0].0, Piece(7));
    assert!((heard.world[0].1 - empty).length() < 1.0);
    assert!(heard.dropped.is_empty() && heard.refused.is_empty());
}

#[test]
fn a_target_of_another_type_refuses_visibly_and_sends_one_refusal() {
    let mut app = app();
    let s = scene(&mut app);
    let over = centre(&app, s.lights);
    let start = centre(&app, s.wall);

    grab_and_move(&mut app, start, over);

    let roles = drag_roles(&mut app);
    assert!(
        roles.iter().any(|(_, r)| *r == Role::DropMarkRefused)
            && roles.iter().any(|(_, r)| *r == Role::DragGhostRefused),
        "the mark and the ghost say no while the pointer is over it: {roles:?}"
    );
    assert!(app.world().resource::<DragState>().refused());
    assert_eq!(app.world().resource::<DragState>().over(), Some(s.lights));

    release(&mut app, over);

    let heard = heard(&app);
    assert_eq!(heard.refused, vec![(s.lights, Piece(7))]);
    assert!(heard.dropped.is_empty() && heard.world.is_empty());
    assert!(
        drag_roles(&mut app).is_empty(),
        "the ghost and the mark are gone"
    );
}

#[test]
fn a_targets_own_test_can_refuse_and_accept() {
    let mut app = app();
    let s = scene(&mut app);
    // Piece(7) is under 10: taken.
    let to = centre(&app, s.small);
    drag(&mut app, s.wall, to);
    assert_eq!(heard(&app).dropped.len(), 1);
    assert_eq!(heard(&app).dropped[0].0, s.small);

    // A bigger piece is refused by the same target.
    app.world_mut()
        .entity_mut(s.wall)
        .insert(DragPayload(Piece(50)));
    drag(&mut app, s.wall, to);
    assert_eq!(heard(&app).dropped.len(), 1, "no second Dropped");
    assert_eq!(heard(&app).refused, vec![(s.small, Piece(50))]);
}

#[test]
fn one_node_can_take_two_types() {
    let mut app = app();
    let s = scene(&mut app);
    let to = centre(&app, s.both);

    drag(&mut app, s.wall, to);
    drag(&mut app, s.lamp, to);

    assert_eq!(heard(&app).dropped.len(), 1);
    assert_eq!(heard(&app).dropped[0].0, s.both);
    assert_eq!(heard(&app).lights, vec![(s.both, Light("warm"))]);
    assert!(heard(&app).refused.is_empty());
}

#[test]
fn the_ghost_and_the_mark_follow_while_carrying_and_are_gone_after() {
    let mut app = app();
    let s = scene(&mut app);
    let over = centre(&app, s.slot);
    let start = centre(&app, s.wall);

    grab_and_move(&mut app, start, over);

    assert!(app.world().get::<Dragging>(s.wall).is_some());
    let roles = drag_roles(&mut app);
    let ghost = roles
        .iter()
        .find(|(_, r)| *r == Role::DragGhost)
        .expect("a ghost")
        .0;
    let mark = roles
        .iter()
        .find(|(_, r)| *r == Role::DropMark)
        .expect("a mark")
        .0;
    assert_eq!(
        app.world().get::<Text>(ghost).map(|t| t.0.as_str()),
        Some("Wall")
    );
    assert_eq!(app.world().get::<Pickable>(ghost), Some(&Pickable::IGNORE));
    assert_eq!(app.world().get::<Pickable>(mark), Some(&Pickable::IGNORE));
    let ghost_at = rect(&app, ghost);
    assert!(
        ghost_at.min.x > over.x && ghost_at.min.y > over.y,
        "the ghost stands beside the pointer, not under it: {ghost_at:?} at {over}"
    );
    let marked = rect(&app, mark);
    let target = rect(&app, s.slot);
    assert!(
        (marked.min - target.min).length() < 1.5 && (marked.max - target.max).length() < 1.5,
        "the mark is the target's rect: {marked:?} against {target:?}"
    );

    release(&mut app, over);

    assert!(app.world().get::<Dragging>(s.wall).is_none());
    assert!(drag_roles(&mut app).is_empty());
}

#[test]
fn escape_puts_it_down_and_sends_nothing() {
    let mut app = app();
    let s = scene(&mut app);
    let over = centre(&app, s.slot);
    let start = centre(&app, s.wall);
    grab_and_move(&mut app, start, over);
    assert!(!drag_roles(&mut app).is_empty());

    key(&mut app, KeyCode::Escape, ButtonState::Pressed);
    key(&mut app, KeyCode::Escape, ButtonState::Released);
    settle(&mut app);
    assert!(
        drag_roles(&mut app).is_empty(),
        "the ghost and the mark are gone"
    );
    assert!(app.world().get::<Dragging>(s.wall).is_none());

    release(&mut app, over);

    let heard = heard(&app);
    assert!(heard.dropped.is_empty() && heard.world.is_empty() && heard.refused.is_empty());
    assert_eq!(heard.cancels, 0, "that Escape was the drag's, not a Back");
}

#[test]
fn escape_with_nothing_carried_is_still_a_back() {
    let mut app = app();
    let s = scene(&mut app);
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(s.wall, bevy::input_focus::FocusCause::Navigated);
    settle(&mut app);

    key(&mut app, KeyCode::Escape, ButtonState::Pressed);
    key(&mut app, KeyCode::Escape, ButtonState::Released);

    assert_eq!(heard(&app).cancels, 1);
}

#[test]
fn letting_go_over_a_panel_that_is_not_a_target_sends_nothing() {
    let mut app = app();
    let s = scene(&mut app);
    let over = centre(&app, s.panel);

    drag(&mut app, s.wall, over);

    let heard = heard(&app);
    assert!(heard.dropped.is_empty() && heard.world.is_empty() && heard.refused.is_empty());
    assert!(drag_roles(&mut app).is_empty());
}

#[test]
fn a_small_move_is_a_click_not_a_drag() {
    let mut app = app();
    let s = scene(&mut app);
    let start = centre(&app, s.wall);

    grab_and_move(&mut app, start, start + Vec2::new(3.0, 0.0));

    assert!(app.world().get::<Dragging>(s.wall).is_none());
    assert!(
        drag_roles(&mut app).is_empty(),
        "no ghost for a shaking hand"
    );
    let slot = centre(&app, s.slot);
    release(&mut app, slot);
    let heard = heard(&app);
    assert!(heard.dropped.is_empty() && heard.world.is_empty());
}

#[test]
fn a_ghost_has_words_at_the_contrast_of_a_panels_in_every_look() {
    for preset in SkinPreset::ALL {
        let mut app = app();
        app.insert_resource(preset.skin());
        let s = scene(&mut app);
        let over = centre(&app, s.lights);
        let start = centre(&app, s.wall);
        grab_and_move(&mut app, start, over);

        for (entity, role) in drag_roles(&mut app) {
            if !matches!(role, Role::DragGhost | Role::DragGhostRefused) {
                continue;
            }
            let fill = app
                .world()
                .get::<BackgroundColor>(entity)
                .expect("filled")
                .0
                .to_srgba();
            let ink = app
                .world()
                .get::<TextColor>(entity)
                .expect("inked")
                .0
                .to_srgba();
            let ratio = contrast_ratio(
                [fill.red, fill.green, fill.blue],
                [ink.red, ink.green, ink.blue],
            );
            assert!(ratio >= 4.5, "{}: ghost words {ratio:.2}:1", preset.name());
            let frame = app
                .world()
                .get::<BorderColor>(entity)
                .expect("framed")
                .top
                .to_srgba();
            let edge = contrast_ratio(
                [fill.red, fill.green, fill.blue],
                [frame.red, frame.green, frame.blue],
            );
            assert!(edge >= 3.0, "{}: ghost frame {edge:.2}:1", preset.name());
        }
    }
}

#[test]
fn nothing_is_written_while_idle() {
    let mut app = app();
    let _ = scene(&mut app);
    app.world_mut().clear_trackers();

    for _ in 0..30 {
        app.update();
    }

    assert!(!app.world().resource_ref::<DragState>().is_changed());
    assert!(parts::<Dragging>(&mut app).is_empty());
}

#[test]
fn letting_go_over_a_world_sprite_is_the_world_too() {
    let mut app = app();
    let s = scene(&mut app);
    // A pickable sprite behind the interface, in the middle of the window's empty part.
    app.world_mut().spawn((
        Sprite::from_color(Color::WHITE, Vec2::splat(300.0)),
        Transform::from_xyz(0.0, -200.0, 0.0),
        Pickable::default(),
    ));
    settle(&mut app);
    let over = Vec2::new(640.0, 560.0);

    drag(&mut app, s.wall, over);

    let heard = heard(&app);
    assert_eq!(heard.world.len(), 1, "one DroppedOnWorld over a sprite");
    assert!((heard.world[0].1 - over).length() < 1.0);
    assert!(heard.dropped.is_empty() && heard.refused.is_empty());
}

// ---- Carrying without a pointer ------------------------------------------------------------

/// Focuses `entity` as the arrow keys would.
fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(entity, bevy::input_focus::FocusCause::Navigated);
    settle(app);
}

fn tap(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Pressed);
    key(app, key_code, ButtonState::Released);
}

/// A chord as a keyboard sends it: the modifier is down a frame before the key.
fn chord(app: &mut App, modifier: KeyCode, key_code: KeyCode) {
    key(app, modifier, ButtonState::Pressed);
    tap(app, key_code);
    key(app, modifier, ButtonState::Released);
}

/// The default pick-up key.
fn pick_up(app: &mut App) {
    chord(app, KeyCode::AltLeft, KeyCode::KeyM);
}

fn plug_in_pad(app: &mut App) -> Entity {
    let pad = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(GamepadConnectionEvent::new(
        pad,
        GamepadConnection::Connected {
            name: "Test pad".into(),
            vendor_id: None,
            product_id: None,
        },
    ));
    app.update();
    pad
}

fn pad_press(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
}

fn over(app: &App) -> Option<Entity> {
    app.world().resource::<DragState>().over()
}

fn carried(app: &App) -> bool {
    app.world().resource::<DragState>().carried_by_key()
}

fn count_activations(app: &mut App, entity: Entity) {
    app.world_mut()
        .entity_mut(entity)
        .observe(|_: On<Activate>, mut heard: ResMut<Heard>| heard.activations += 1);
}

#[test]
fn a_focused_source_is_lifted_steered_and_dropped_from_the_keyboard() {
    let mut app = app();
    let s = scene(&mut app);
    focus(&mut app, s.wall);

    pick_up(&mut app);
    assert!(carried(&app), "Alt+M lifts the focused source");
    assert!(app.world().get::<Dragging>(s.wall).is_some());
    assert_eq!(
        over(&app),
        None,
        "nothing is marked until the player steers"
    );
    assert_eq!(drag_roles(&mut app).len(), 2, "a ghost and a mark");
    assert!(heard(&app).dropped.is_empty());

    // The nearest target to the right of a source on the left is the top one.
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(over(&app), Some(s.slot));
    // Down goes slot, small and then past the lights (which take no piece) to both.
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(over(&app), Some(s.small));
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(
        over(&app),
        Some(s.both),
        "a target that refuses the piece is skipped"
    );
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(over(&app), Some(s.small));
    tap(&mut app, KeyCode::ArrowDown);

    // The ghost stands beside the marked target, as it does beside a pointer.
    settle(&mut app);
    let marked = centre(&app, s.both);
    let ghost = drag_roles(&mut app)
        .into_iter()
        .find(|(_, role)| *role == Role::DragGhost)
        .map(|(entity, _)| entity)
        .expect("a ghost");
    let node = app.world().get::<Node>(ghost).expect("a node");
    let scale = app.world().resource::<UiScale>().0;
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        panic!("the ghost is placed in pixels");
    };
    assert!((Vec2::new(left, top) * scale - marked).length() < 40.0 * scale);

    pick_up(&mut app);

    let heard = heard(&app);
    assert_eq!(heard.dropped.len(), 1, "exactly one Dropped");
    let (target, source, payload, pointer) = &heard.dropped[0];
    assert_eq!((*target, *source), (s.both, s.wall));
    assert_eq!(*payload, Piece(7));
    assert!(
        (*pointer - marked).length() < 1.0,
        "the pointer is the target's middle"
    );
    assert!(heard.refused.is_empty() && heard.world.is_empty() && heard.lights.is_empty());
    assert!(!carried(&app));
    assert!(app.world().get::<Dragging>(s.wall).is_none());
    assert!(drag_roles(&mut app).is_empty(), "the ghost and the mark go");
}

#[test]
fn the_keys_that_steer_move_no_focus_and_send_no_back() {
    let mut app = app();
    let s = scene(&mut app);
    focus(&mut app, s.wall);
    pick_up(&mut app);

    for code in [
        KeyCode::ArrowRight,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::PageDown,
    ] {
        tap(&mut app, code);
    }

    let focused = app
        .world()
        .resource::<bevy::input_focus::InputFocus>()
        .get();
    assert_eq!(focused, Some(s.wall), "focus stays on the source");
    assert_eq!(heard(&app).cancels, 0);
    assert!(carried(&app));
}

#[test]
fn a_button_source_is_not_also_pressed_by_carrying() {
    let mut app = app();
    let s = scene(&mut app);
    count_activations(&mut app, s.wall);
    focus(&mut app, s.wall);

    pick_up(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    pick_up(&mut app);

    assert_eq!(heard(&app).dropped.len(), 1);
    assert_eq!(heard(&app).activations, 0);
}

#[test]
fn escape_puts_a_carried_payload_down_and_sends_nothing() {
    let mut app = app();
    let s = scene(&mut app);
    focus(&mut app, s.wall);
    pick_up(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(over(&app), Some(s.slot));

    tap(&mut app, KeyCode::Escape);

    {
        let heard = heard(&app);
        assert!(heard.dropped.is_empty() && heard.refused.is_empty() && heard.world.is_empty());
        assert_eq!(heard.cancels, 0, "that Escape was no Back");
    }
    assert!(!carried(&app));
    assert!(app.world().get::<Dragging>(s.wall).is_none());
    assert!(drag_roles(&mut app).is_empty());
    // Back works again afterwards.
    tap(&mut app, KeyCode::Escape);
    assert_eq!(heard(&app).cancels, 1);
}

#[test]
fn dropping_with_nothing_marked_sends_nothing() {
    let mut app = app();
    let s = scene(&mut app);
    focus(&mut app, s.wall);
    pick_up(&mut app);

    pick_up(&mut app);

    let heard = heard(&app);
    assert!(heard.dropped.is_empty() && heard.refused.is_empty() && heard.world.is_empty());
    assert!(!carried(&app));
    assert!(drag_roles(&mut app).is_empty());
}

#[test]
fn only_targets_that_take_the_payload_are_offered() {
    let mut app = app();
    let s = scene(&mut app);
    // A piece of 70 is refused by the "small pieces" target, which a pointer would see in red.
    let big = app
        .world_mut()
        .spawn((button("Big"), DragPayload(Piece(70))))
        .id();
    app.world_mut().entity_mut(s.wall).add_child(big);
    settle(&mut app);
    focus(&mut app, big);
    pick_up(&mut app);

    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(over(&app), Some(s.slot));
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(
        over(&app),
        Some(s.both),
        "small pieces refuses 70, lights take no piece"
    );
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(over(&app), Some(s.both), "nothing is below the last one");
    assert!(!app.world().resource::<DragState>().refused());
}

#[test]
fn a_light_goes_to_the_targets_that_take_lights() {
    let mut app = app();
    let s = scene(&mut app);
    focus(&mut app, s.lamp);
    pick_up(&mut app);

    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(over(&app), Some(s.lights));
    pick_up(&mut app);

    assert_eq!(heard(&app).lights, vec![(s.lights, Light("warm"))]);
    assert!(heard(&app).dropped.is_empty());
}

#[test]
fn page_keys_step_through_the_targets_in_reading_order_and_wrap() {
    let mut app = app();
    let s = scene(&mut app);
    focus(&mut app, s.wall);
    pick_up(&mut app);

    let mut seen = Vec::new();
    for _ in 0..4 {
        tap(&mut app, KeyCode::PageDown);
        seen.push(over(&app));
    }
    assert_eq!(
        seen,
        [Some(s.slot), Some(s.small), Some(s.both), Some(s.slot)]
    );
    tap(&mut app, KeyCode::PageUp);
    assert_eq!(over(&app), Some(s.both));
}

#[test]
fn the_pad_lifts_with_west_steers_with_the_dpad_and_drops_with_south() {
    let mut app = app();
    let s = scene(&mut app);
    let pad = plug_in_pad(&mut app);
    count_activations(&mut app, s.wall);
    focus(&mut app, s.wall);

    pad_press(&mut app, pad, GamepadButton::West);
    assert!(carried(&app));
    pad_press(&mut app, pad, GamepadButton::DPadRight);
    assert_eq!(over(&app), Some(s.slot));
    pad_press(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(over(&app), Some(s.small));
    pad_press(&mut app, pad, GamepadButton::South);

    let heard = heard(&app);
    assert_eq!(heard.dropped.len(), 1);
    assert_eq!(heard.dropped[0].0, s.small);
    assert_eq!(heard.activations, 0, "South dropped it and pressed nothing");
    assert!(!carried(&app));
}

#[test]
fn east_puts_a_pad_carried_payload_down() {
    let mut app = app();
    let s = scene(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, s.wall);
    pad_press(&mut app, pad, GamepadButton::West);
    pad_press(&mut app, pad, GamepadButton::DPadRight);

    pad_press(&mut app, pad, GamepadButton::East);

    assert!(!carried(&app));
    assert!(heard(&app).dropped.is_empty());
    assert_eq!(heard(&app).cancels, 0);
}

#[test]
fn picking_up_needs_a_focused_source() {
    let mut app = app();
    let s = scene(&mut app);
    // Nothing focused, then a panel that is not a source.
    pick_up(&mut app);
    assert!(!carried(&app));
    focus(&mut app, s.slot);
    pick_up(&mut app);
    assert!(!carried(&app));
    assert!(drag_roles(&mut app).is_empty());
}

#[test]
fn a_source_that_goes_while_it_is_carried_ends_the_carry() {
    let mut app = app();
    let s = scene(&mut app);
    focus(&mut app, s.wall);
    pick_up(&mut app);
    tap(&mut app, KeyCode::ArrowRight);

    app.world_mut().entity_mut(s.wall).despawn();
    settle(&mut app);

    assert!(!carried(&app));
    assert!(drag_roles(&mut app).is_empty());
    assert!(heard(&app).dropped.is_empty());
}

#[test]
fn the_pick_up_key_is_the_games_to_change() {
    let mut app = app();
    let s = scene(&mut app);
    app.world_mut().resource_mut::<DragSettings>().pick_up =
        Shortcut::new(Modifiers::NONE, KeyCode::F2);
    focus(&mut app, s.wall);

    pick_up(&mut app);
    assert!(!carried(&app), "Alt+M no longer lifts");
    tap(&mut app, KeyCode::F2);
    assert!(carried(&app));
    tap(&mut app, KeyCode::ArrowRight);
    tap(&mut app, KeyCode::F2);

    assert_eq!(heard(&app).dropped.len(), 1);
}

#[test]
fn a_pointer_drag_is_not_disturbed_by_the_keys() {
    let mut app = app();
    let s = scene(&mut app);
    let to = centre(&app, s.slot);
    let from = centre(&app, s.wall);
    grab_and_move(&mut app, from, to);
    assert!(app.world().resource::<DragState>().is_dragging());

    pick_up(&mut app);
    tap(&mut app, KeyCode::ArrowDown);

    assert!(!carried(&app), "the pointer's drag stays the pointer's");
    assert_eq!(over(&app), Some(s.slot));
    release(&mut app, to);
    assert_eq!(heard(&app).dropped.len(), 1);
}

// ---- candidates: the places that take what is carried, from the moment it is lifted ----

fn candidates(app: &mut App) -> Vec<Entity> {
    let mut found: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<DropCandidate>>()
        .iter(app.world())
        .collect();
    found.sort();
    found
}

fn refusers(app: &mut App) -> Vec<Entity> {
    let mut found: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<DropRefusing>>()
        .iter(app.world())
        .collect();
    found.sort();
    found
}

fn sorted(mut entities: Vec<Entity>) -> Vec<Entity> {
    entities.sort();
    entities
}

#[test]
fn lifting_a_source_marks_exactly_the_targets_that_take_its_payload() {
    let mut app = app();
    let s = scene(&mut app);
    assert!(candidates(&mut app).is_empty());

    // The wall (Piece 7) is taken by "any piece", "small pieces" and "both", and by nothing else:
    // not the lights, not the plain panel, not the wall itself.
    let from = centre(&app, s.wall);
    grab_and_move(&mut app, from, from + Vec2::new(0.0, 200.0));
    assert!(app.world().resource::<DragState>().is_dragging());
    assert_eq!(
        candidates(&mut app),
        sorted(vec![s.slot, s.small, s.both]),
        "the pointer is over none of them yet"
    );
    // The one target that takes no piece refuses it; a panel that is no target is neither.
    assert_eq!(refusers(&mut app), vec![s.lights]);

    // A target's own test counts: a piece of 50 is refused by "small pieces".
    release(&mut app, from + Vec2::new(0.0, 200.0));
    assert!(
        candidates(&mut app).is_empty(),
        "let go takes every mark off"
    );
    app.world_mut()
        .entity_mut(s.wall)
        .insert(DragPayload(Piece(50)));
    grab_and_move(&mut app, from, from + Vec2::new(0.0, 200.0));
    assert_eq!(candidates(&mut app), sorted(vec![s.slot, s.both]));
    assert_eq!(refusers(&mut app), sorted(vec![s.small, s.lights]));
    release(&mut app, from + Vec2::new(0.0, 200.0));
    assert!(refusers(&mut app).is_empty());

    // A light goes to the two that take lights.
    let from = centre(&app, s.lamp);
    grab_and_move(&mut app, from, from + Vec2::new(0.0, 200.0));
    assert_eq!(candidates(&mut app), sorted(vec![s.lights, s.both]));
    assert_eq!(refusers(&mut app), sorted(vec![s.slot, s.small]));
}

#[test]
fn escape_and_a_keyboard_carry_end_and_make_the_same_marks() {
    let mut app = app();
    let s = scene(&mut app);
    focus(&mut app, s.wall);

    pick_up(&mut app);
    assert!(carried(&app));
    assert_eq!(candidates(&mut app), sorted(vec![s.slot, s.small, s.both]));

    key(&mut app, KeyCode::Escape, ButtonState::Pressed);
    key(&mut app, KeyCode::Escape, ButtonState::Released);
    settle(&mut app);
    assert!(!carried(&app));
    assert!(candidates(&mut app).is_empty());

    // A drop from the keyboard ends them too.
    pick_up(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    pick_up(&mut app);
    assert!(candidates(&mut app).is_empty());
    assert_eq!(heard(&app).dropped.len(), 1);
}

#[test]
fn a_press_that_never_lifts_marks_nothing_and_a_disabled_target_is_no_candidate() {
    let mut app = app();
    let s = scene(&mut app);
    let from = centre(&app, s.wall);
    point_at(&mut app, from);
    pointer(&mut app, from, PointerAction::Press(PointerButton::Primary));
    pointer(
        &mut app,
        from + Vec2::new(2.0, 0.0),
        PointerAction::Move {
            delta: Vec2::X * 2.0,
        },
    );
    settle(&mut app);
    assert!(!app.world().resource::<DragState>().is_dragging());
    assert!(candidates(&mut app).is_empty(), "a small move is a click");
    release(&mut app, from);

    app.world_mut()
        .entity_mut(s.small)
        .insert(bevy::ui::InteractionDisabled);
    grab_and_move(&mut app, from, from + Vec2::new(0.0, 200.0));
    assert_eq!(candidates(&mut app), sorted(vec![s.slot, s.both]));
    assert!(
        !refusers(&mut app).contains(&s.small),
        "a disabled target is neither: it is not offered and not dimmed as a refusal"
    );
}

#[test]
fn candidates_write_nothing_while_the_carry_holds_still() {
    let mut app = app();
    let s = scene(&mut app);
    let from = centre(&app, s.wall);
    grab_and_move(&mut app, from, from + Vec2::new(0.0, 200.0));
    settle(&mut app);
    app.world_mut().clear_trackers();
    for _ in 0..30 {
        app.update();
    }
    for target in [s.slot, s.small, s.both] {
        let e = app.world().entity(target);
        assert!(!e.get_ref::<DropCandidate>().expect("marked").is_changed());
        assert!(!e.get_ref::<BorderColor>().expect("border").is_changed());
    }
}
