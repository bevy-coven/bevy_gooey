//! The split: two panes and a divider the player moves by pointer, key and pad, never past a pane's
//! minimum. Laid out for real in a headless window, driven by real keyboard, pointer and gamepad
//! input.

use bevy::camera::{ImageRenderTarget, NormalizedRenderTarget, RenderTarget};
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::render_resource::TextureFormat;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{
    ComputedNode, ComputedUiTargetCamera, InteractionDisabled, IsDefaultUiCamera, UiGlobalTransform,
};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<f32>` the app has seen: the source, the value and `is_final`.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, f32, bool)>);

/// How many frames' writes to a split's nodes the app has seen.
#[derive(Resource, Default)]
struct Writes(u32);

fn app_sized(width: u32, height: u32) -> App {
    let mut app = App::new();
    // Assumed owner exception (#76): these tests rest the pointer on a collapsed divider for real
    // milliseconds, which would float the pane over the second one; `tests/split_peek.rs` proves the
    // peek on a clock it sets.
    app.insert_resource(SplitPeekSettings {
        enabled: false,
        ..default()
    });
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(width, height)
                        .with_scale_factor_override(1.0),
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
    // The instant path: this file asserts layout a frame or two after a change, on the real-time
    // clock (assumed owner exception, bevy_gooey#76).
    .insert_resource(MotionSettings::off())
    .init_resource::<Changes>()
    .init_resource::<Writes>()
    .add_observer(|change: On<ValueChange<f32>>, mut log: ResMut<Changes>| {
        log.0.push((change.source, change.value, change.is_final));
    })
    .add_systems(
        Update,
        |nodes: Query<(), (Changed<Node>, With<Role>)>, mut writes: ResMut<Writes>| {
            writes.0 += nodes.iter().count() as u32;
        },
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn app() -> App {
    app_sized(1280, 720)
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// A split, its two panes and its divider, in a container of fixed size.
struct Rig {
    container: Entity,
    split: Entity,
    first: Entity,
    divider: Entity,
    second: Entity,
    /// The panels inside the panes.
    a: Entity,
    b: Entity,
}

/// The divider's thickness, `MIN_TARGET_PX`: the room the panes share is the split less this.
const BAR: f32 = 24.0;

fn rig_with(
    app: &mut App,
    axis: SplitAxis,
    (width, height): (f32, f32),
    fraction: f32,
    min: SplitMin,
) -> Rig {
    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(width),
                height: Val::Px(height),
                ..default()
            },
            children![(
                split(
                    axis,
                    (panel(), children![label("A")]),
                    (panel(), children![label("B")]),
                ),
                SplitFraction(fraction),
                min,
            )],
        ))
        .id();
    settle(app);
    let split = child_of(app, container);
    let parts: Vec<Entity> = app
        .world()
        .get::<Children>(split)
        .expect("the split's children")
        .iter()
        .collect();
    let first = parts[0];
    let divider = parts[1];
    let second = parts[2];
    let a = child_of(app, first);
    let b = child_of(app, second);
    Rig {
        container,
        split,
        first,
        divider,
        second,
        a,
        b,
    }
}

fn rig(app: &mut App, axis: SplitAxis, fraction: f32, min: SplitMin) -> Rig {
    rig_with(app, axis, (800.0, 400.0), fraction, min)
}

fn child_of(app: &App, parent: Entity) -> Entity {
    app.world()
        .get::<Children>(parent)
        .expect("children")
        .iter()
        .next()
        .expect("a child")
}

/// Where `entity` is drawn, in logical px (the layout's).
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

fn fraction(app: &App, split: Entity) -> f32 {
    app.world().get::<SplitFraction>(split).expect("state").0
}

fn set_fraction(app: &mut App, split: Entity, to: f32) {
    app.world_mut()
        .get_mut::<SplitFraction>(split)
        .expect("state")
        .0 = to;
    settle(app);
}

fn changes(app: &App) -> Vec<(Entity, f32, bool)> {
    app.world().resource::<Changes>().0.clone()
}

fn clear_changes(app: &mut App) {
    app.world_mut().resource_mut::<Changes>().0.clear();
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

/// A pointer position for a point of the layout: the layout is the window's size over `UiScale`.
fn at(app: &App, layout: Vec2) -> Vec2 {
    layout * app.world().resource::<UiScale>().0
}

/// Where the pointer is: the window unless the test draws the interface into an image.
#[derive(Resource)]
struct PointerOn(NormalizedRenderTarget);

fn location(app: &mut App, at: Vec2) -> Location {
    if let Some(on) = app.world().get_resource::<PointerOn>() {
        return Location {
            target: on.0.clone(),
            position: at,
        };
    }
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

/// A press and release on `at` without moving.
fn click(app: &mut App, at: Vec2) {
    point_at(app, at);
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
}

/// Drags the divider `by` logical px (layout units) along the axis and lets go.
fn drag(app: &mut App, divider: Entity, by: Vec2) {
    let from = at(app, rect(app, divider).center());
    let to = from + at(app, by);
    grab_and_move(app, from, to);
    release(app, to);
}

fn tap_with(app: &mut App, key_code: KeyCode, repeat: bool) {
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat,
            window,
        });
        app.update();
    }
}

fn tap(app: &mut App, key_code: KeyCode) {
    tap_with(app, key_code, false);
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    settle(app);
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
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

fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
}

/// The room the panes share on the axis: the split's length less the divider's.
fn room(app: &App, ui: &Rig, axis: SplitAxis) -> f32 {
    let whole = rect(app, ui.split).size();
    let bar = rect(app, ui.divider).size();
    match axis {
        SplitAxis::Horizontal => whole.x - bar.x,
        SplitAxis::Vertical => whole.y - bar.y,
    }
}

fn along(size: Vec2, axis: SplitAxis) -> f32 {
    match axis {
        SplitAxis::Horizontal => size.x,
        SplitAxis::Vertical => size.y,
    }
}

fn pane(app: &App, entity: Entity, axis: SplitAxis) -> f32 {
    along(rect(app, entity).size(), axis)
}

fn fill_of(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

const H: SplitAxis = SplitAxis::Horizontal;
const V: SplitAxis = SplitAxis::Vertical;

// ---- the layout ----

#[test]
fn the_panes_share_the_room_the_divider_leaves_in_the_ratio_of_the_fraction() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.25, SplitMin::default());
    let room = room(&app, &ui, H);
    assert!((room - (800.0 - BAR)).abs() < 0.6, "the room is {room}");
    assert!((pane(&app, ui.first, H) - 0.25 * room).abs() < 1.0);
    assert!((pane(&app, ui.second, H) - 0.75 * room).abs() < 1.0);
    // The divider is the target's minimum wide and as long as the split.
    let bar = rect(&app, ui.divider);
    assert!((bar.width() - BAR).abs() < 0.6, "{} wide", bar.width());
    assert!((bar.height() - 400.0).abs() < 0.6);
    // It stands between the panes, touching both.
    assert!((rect(&app, ui.first).max.x - bar.min.x).abs() < 1.0);
    assert!((bar.max.x - rect(&app, ui.second).min.x).abs() < 1.0);
}

#[test]
fn a_vertical_split_stacks_its_panes_and_the_fraction_is_the_top_ones() {
    let mut app = app();
    let ui = rig(&mut app, V, 0.25, SplitMin::default());
    let room = room(&app, &ui, V);
    assert!((room - (400.0 - BAR)).abs() < 0.6, "the room is {room}");
    assert!((pane(&app, ui.first, V) - 0.25 * room).abs() < 1.0);
    let (top, bar, bottom) = (
        rect(&app, ui.first),
        rect(&app, ui.divider),
        rect(&app, ui.second),
    );
    assert!(top.max.y <= bar.min.y + 1.0 && bar.max.y <= bottom.min.y + 1.0);
    assert!((bar.height() - BAR).abs() < 0.6 && (bar.width() - 800.0).abs() < 0.6);
}

#[test]
fn a_panel_in_a_pane_fills_it_on_both_axes() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.3, SplitMin::default());
    for (pane, panel) in [(ui.first, ui.a), (ui.second, ui.b)] {
        let (pane, panel) = (rect(&app, pane), rect(&app, panel));
        assert!(
            (pane.width() - panel.width()).abs() < 1.0,
            "{pane:?} {panel:?}"
        );
        assert!(
            (pane.height() - panel.height()).abs() < 1.0,
            "{pane:?} {panel:?}"
        );
    }
}

#[test]
fn content_wider_than_its_pane_is_cut_and_never_widens_it() {
    let mut app = app();
    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(800.0),
                height: Val::Px(400.0),
                ..default()
            },
            children![(
                split(
                    H,
                    (
                        panel(),
                        children![label(
                            "A line of text that is a good deal wider than the pane it is in"
                        )]
                    ),
                    (panel(), children![label("B")]),
                ),
                SplitFraction(0.2),
            )],
        ))
        .id();
    settle(&mut app);
    let split = child_of(&app, container);
    let first = app.world().get::<Children>(split).expect("children")[0];
    let room = 800.0 - BAR;
    assert!(
        (pane(&app, first, H) - 0.2 * room).abs() < 1.0,
        "the first pane is {}, not {}",
        pane(&app, first, H),
        0.2 * room
    );
}

#[test]
fn a_smaller_split_keeps_its_ratio_and_its_state() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.25, SplitMin::default());
    app.world_mut()
        .get_mut::<Node>(ui.container)
        .expect("node")
        .width = Val::Px(400.0);
    settle(&mut app);
    let room = room(&app, &ui, H);
    assert!((room - (400.0 - BAR)).abs() < 0.6);
    assert!((pane(&app, ui.first, H) - 0.25 * room).abs() < 1.0);
    assert_eq!(fraction(&app, ui.split), 0.25, "the state is untouched");
}

// ---- the drag ----

/// Draws the interface into an image of the window's size, as a host that renders its whole
/// interface to a surface does, and puts the pointer on that image.
fn draw_into_an_image(app: &mut App, scale_factor: f32) {
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            1280,
            720,
            TextureFormat::Bgra8UnormSrgb,
            None,
        ));
    let target = RenderTarget::Image(ImageRenderTarget {
        handle,
        scale_factor,
    });
    let normalized = target.normalize(None).expect("an image target");
    app.world_mut().spawn((Camera2d, target, IsDefaultUiCamera));
    app.world_mut().insert_resource(PointerOn(normalized));
}

#[test]
fn a_drag_in_an_interface_drawn_into_an_image_moves_the_divider() {
    let mut app = app();
    draw_into_an_image(&mut app, 1.0);
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    let camera = app
        .world()
        .get::<ComputedUiTargetCamera>(ui.divider)
        .and_then(ComputedUiTargetCamera::get)
        .expect("a camera draws the divider");
    assert!(
        matches!(
            app.world().get::<RenderTarget>(camera),
            Some(RenderTarget::Image(_))
        ),
        "the interface is drawn into the image"
    );
    let room = room(&app, &ui, H);
    drag(&mut app, ui.divider, Vec2::new(100.0, 0.0));
    let want = 0.5 + 100.0 / room;
    assert!(
        (fraction(&app, ui.split) - want).abs() < 0.002,
        "dragged 100 px of {room}: {} of {want}; changes {:?}",
        fraction(&app, ui.split),
        changes(&app)
    );
}

#[test]
fn a_drag_of_one_move_a_frame_in_an_image_at_a_scale_with_minimums_moves_the_divider() {
    // The shape a host that draws into an image at 1.5x, under a UI scale of 1.2, with minimums of
    // 96 px, sees: the pointer moves a few pixels a frame.
    let mut app = app_sized(1536, 864);
    settle(&mut app);
    assert!((app.world().resource::<UiScale>().0 - 1.2).abs() < 1e-3);
    draw_into_an_image(&mut app, 1.5);
    let ui = rig(&mut app, H, 0.22, SplitMin::new(96.0, 96.0));
    let room = room(&app, &ui, H);
    let from = at(&app, rect(&app, ui.divider).center());
    point_at(&mut app, from);
    pointer(&mut app, from, PointerAction::Press(PointerButton::Primary));
    let mut now = from;
    for _ in 0..10 {
        let next = now + Vec2::new(12.0, 0.0);
        pointer(&mut app, next, PointerAction::Move { delta: next - now });
        now = next;
    }
    release(&mut app, now);
    let want = 0.22 + (now.x - from.x) / 1.2 / room;
    assert!(
        (fraction(&app, ui.split) - want).abs() < 0.002,
        "dragged {} px of {room}: {} of {want}; changes {:?}",
        now.x - from.x,
        fraction(&app, ui.split),
        changes(&app)
    );
}

#[test]
fn dragging_the_divider_100_px_changes_the_fraction_by_100_over_the_room() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    let room = room(&app, &ui, H);
    drag(&mut app, ui.divider, Vec2::new(100.0, 0.0));
    let want = 0.5 + 100.0 / room;
    assert!(
        (fraction(&app, ui.split) - want).abs() < 0.002,
        "dragged 100 px of {room}: {} of {want}",
        fraction(&app, ui.split)
    );
    // And the panes followed it.
    assert!((pane(&app, ui.first, H) - want * room).abs() < 1.0);
    // Back the other way by 250.
    drag(&mut app, ui.divider, Vec2::new(-250.0, 0.0));
    let want = want - 250.0 / room;
    assert!(
        (fraction(&app, ui.split) - want).abs() < 0.002,
        "after the second drag: {} of {want}; divider at {:?}; changes {:?}",
        fraction(&app, ui.split),
        rect(&app, ui.divider),
        changes(&app)
    );
}

#[test]
fn the_same_drag_at_a_ui_scale_of_one_and_a_half_is_the_same_share() {
    // A 1920x1080 window draws the interface at 1.5x: 100 layout px is 150 window px.
    let mut app = app_sized(1920, 1080);
    settle(&mut app);
    assert!((app.world().resource::<UiScale>().0 - 1.5).abs() < 1e-3);
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    let room = room(&app, &ui, H);
    assert!(
        (room - (800.0 - BAR)).abs() < 0.6,
        "the layout is in logical px"
    );
    drag(&mut app, ui.divider, Vec2::new(100.0, 0.0));
    let want = 0.5 + 100.0 / room;
    assert!(
        (fraction(&app, ui.split) - want).abs() < 0.002,
        "{} of {want}",
        fraction(&app, ui.split)
    );
}

#[test]
fn a_vertical_divider_is_dragged_down_the_axis() {
    let mut app = app();
    let ui = rig(&mut app, V, 0.5, SplitMin::default());
    let room = room(&app, &ui, V);
    // Sideways does nothing, 60 px down moves it.
    drag(&mut app, ui.divider, Vec2::new(90.0, 0.0));
    assert_eq!(fraction(&app, ui.split), 0.5);
    drag(&mut app, ui.divider, Vec2::new(0.0, 60.0));
    assert!((fraction(&app, ui.split) - (0.5 + 60.0 / room)).abs() < 0.002);
}

#[test]
fn a_drag_never_takes_a_pane_below_its_minimum() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.5, SplitMin::new(200.0, 150.0));
    drag(&mut app, ui.divider, Vec2::new(5000.0, 0.0));
    assert!(
        pane(&app, ui.second, H) >= 150.0 - 0.6,
        "the second pane is {}",
        pane(&app, ui.second, H)
    );
    assert!(
        (pane(&app, ui.second, H) - 150.0).abs() < 1.0,
        "and it stops there"
    );
    drag(&mut app, ui.divider, Vec2::new(-5000.0, 0.0));
    assert!(pane(&app, ui.first, H) >= 200.0 - 0.6);
    assert!((pane(&app, ui.first, H) - 200.0).abs() < 1.0);
    // The state says the same: 200 of 776.
    assert!((fraction(&app, ui.split) - 200.0 / (800.0 - BAR)).abs() < 0.002);
}

#[test]
fn a_split_too_small_for_both_minimums_takes_the_room_from_the_second_pane_first() {
    let mut app = app();
    // 300 px wide: 276 of room for minimums of 200 and 200.
    let ui = rig_with(
        &mut app,
        H,
        (300.0, 200.0),
        0.5,
        SplitMin::new(200.0, 200.0),
    );
    assert!(
        (pane(&app, ui.first, H) - 200.0).abs() < 1.0,
        "the first keeps its minimum"
    );
    assert!(
        (pane(&app, ui.second, H) - 76.0).abs() < 1.5,
        "the second gives way"
    );
    // Smaller than the first's minimum alone: the first gets all the room, the second none.
    app.world_mut()
        .get_mut::<Node>(ui.container)
        .expect("node")
        .width = Val::Px(150.0);
    settle(&mut app);
    assert!(
        (pane(&app, ui.first, H) - 126.0).abs() < 1.5,
        "{}",
        pane(&app, ui.first, H)
    );
    assert!(
        pane(&app, ui.second, H) < 1.0,
        "{}",
        pane(&app, ui.second, H)
    );
    // A split that shrank and grew back is where it was.
    app.world_mut()
        .get_mut::<Node>(ui.container)
        .expect("node")
        .width = Val::Px(800.0);
    settle(&mut app);
    assert!((pane(&app, ui.first, H) - 0.5 * (800.0 - BAR)).abs() < 1.0);
}

#[test]
fn a_drag_reports_each_move_and_a_last_final_one() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    clear_changes(&mut app);
    let from = rect(&app, ui.divider).center();
    grab_and_move(&mut app, from, from + Vec2::new(40.0, 0.0));
    let during = changes(&app);
    assert!(!during.is_empty(), "the drag said something");
    assert!(
        during
            .iter()
            .all(|&(source, _, last)| source == ui.split && !last)
    );
    pointer(
        &mut app,
        from + Vec2::new(40.0, 0.0),
        PointerAction::Release(PointerButton::Primary),
    );
    settle(&mut app);
    let all = changes(&app);
    let (source, value, last) = *all.last().expect("a last report");
    assert!(last && source == ui.split, "the release is final");
    assert_eq!(
        value,
        fraction(&app, ui.split),
        "and it carries where the divider is"
    );
    assert_eq!(all.iter().filter(|c| c.2).count(), 1);
}

#[test]
fn a_value_the_game_sets_moves_the_panes_and_says_nothing() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    clear_changes(&mut app);
    set_fraction(&mut app, ui.split, 0.8);
    assert!(changes(&app).is_empty());
    assert!((pane(&app, ui.first, H) - 0.8 * (800.0 - BAR)).abs() < 1.0);
}

#[test]
fn a_second_split_in_a_pane_moves_alone() {
    let mut app = app();
    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(800.0),
                height: Val::Px(400.0),
                ..default()
            },
            children![(
                split(H, split(V, panel(), panel()), panel(),),
                SplitFraction(0.6),
            )],
        ))
        .id();
    settle(&mut app);
    let outer = child_of(&app, container);
    let outer_first = app.world().get::<Children>(outer).expect("children")[0];
    let inner = child_of(&app, outer_first);
    let inner_divider = app.world().get::<Children>(inner).expect("children")[1];
    drag(&mut app, inner_divider, Vec2::new(0.0, 50.0));
    assert_eq!(fraction(&app, outer), 0.6, "the outer split stayed");
    assert!(
        fraction(&app, inner) > 0.5,
        "the inner one moved: {}",
        fraction(&app, inner)
    );
    // The inner split fills the outer's first pane.
    assert!((rect(&app, inner).width() - rect(&app, outer_first).width()).abs() < 1.0);
}

// ---- the keys ----

#[test]
fn left_and_right_step_a_horizontal_divider_by_sixteen_px_and_move_no_focus() {
    let mut app = app();
    // A button right of the divider: Right would move focus to it if it were not the divider's.
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    let button = app.world_mut().spawn(button("Beside")).id();
    app.world_mut().entity_mut(ui.b).add_child(button);
    settle(&mut app);
    focus(&mut app, ui.divider);
    let room = room(&app, &ui, H);
    clear_changes(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    assert!((fraction(&app, ui.split) - (0.5 + 16.0 / room)).abs() < 1e-4);
    assert_eq!(focused(&app), Some(ui.divider), "focus stayed");
    tap(&mut app, KeyCode::ArrowLeft);
    tap(&mut app, KeyCode::ArrowLeft);
    assert!((fraction(&app, ui.split) - (0.5 - 16.0 / room)).abs() < 1e-4);
    assert_eq!(focused(&app), Some(ui.divider));
    assert_eq!(changes(&app).len(), 3, "one report a press");
    assert!(changes(&app).iter().all(|c| c.2), "a key's report is final");
    // The panes follow.
    assert!((pane(&app, ui.first, H) - (0.5 - 16.0 / room) * room).abs() < 1.0);
}

#[test]
fn a_held_key_repeats() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    focus(&mut app, ui.divider);
    let room = room(&app, &ui, H);
    tap(&mut app, KeyCode::ArrowRight);
    tap_with(&mut app, KeyCode::ArrowRight, true);
    tap_with(&mut app, KeyCode::ArrowRight, true);
    assert!((fraction(&app, ui.split) - (0.5 + 48.0 / room)).abs() < 1e-4);
}

#[test]
fn up_and_down_step_a_vertical_divider_and_left_and_right_do_nothing_to_it() {
    let mut app = app();
    let ui = rig(&mut app, V, 0.5, SplitMin::default());
    focus(&mut app, ui.divider);
    let room = room(&app, &ui, V);
    tap(&mut app, KeyCode::ArrowRight);
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(fraction(&app, ui.split), 0.5, "the other axis is not its");
    tap(&mut app, KeyCode::ArrowDown);
    assert!((fraction(&app, ui.split) - (0.5 + 16.0 / room)).abs() < 1e-4);
    tap(&mut app, KeyCode::ArrowUp);
    tap(&mut app, KeyCode::ArrowUp);
    assert!((fraction(&app, ui.split) - (0.5 - 16.0 / room)).abs() < 1e-4);
    assert_eq!(focused(&app), Some(ui.divider));
}

#[test]
fn home_and_end_take_the_first_pane_to_its_smallest_and_largest() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.5, SplitMin::new(120.0, 90.0));
    focus(&mut app, ui.divider);
    tap(&mut app, KeyCode::Home);
    assert!((pane(&app, ui.first, H) - 120.0).abs() < 1.0);
    tap(&mut app, KeyCode::End);
    assert!((pane(&app, ui.second, H) - 90.0).abs() < 1.0);
    // At an end a step further changes nothing and says nothing.
    clear_changes(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    tap(&mut app, KeyCode::End);
    assert!(changes(&app).is_empty(), "{:?}", changes(&app));
}

#[test]
fn enter_collapses_the_first_pane_to_its_minimum_and_gives_it_back() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.35, SplitMin::new(100.0, 0.0));
    focus(&mut app, ui.divider);
    tap(&mut app, KeyCode::Enter);
    assert!(
        (pane(&app, ui.first, H) - 100.0).abs() < 1.0,
        "collapsed to its minimum"
    );
    tap(&mut app, KeyCode::Enter);
    assert!(
        (fraction(&app, ui.split) - 0.35).abs() < 1e-4,
        "restored where it was"
    );
    // With no minimum the pane goes to nothing.
    set_fraction(&mut app, ui.split, 0.5);
    app.world_mut()
        .entity_mut(ui.split)
        .insert(SplitMin::default());
    settle(&mut app);
    tap(&mut app, KeyCode::Enter);
    assert!(pane(&app, ui.first, H) < 1.0);
    assert_eq!(focused(&app), Some(ui.divider));
}

// ---- the pad ----

#[test]
fn the_dpad_steps_the_divider_and_south_collapses_and_restores_it() {
    let mut app = app();
    let pad = plug_in_pad(&mut app);
    let ui = rig(&mut app, H, 0.5, SplitMin::new(100.0, 100.0));
    focus(&mut app, ui.divider);
    let room = room(&app, &ui, H);
    press(&mut app, pad, GamepadButton::DPadRight);
    assert!((fraction(&app, ui.split) - (0.5 + 16.0 / room)).abs() < 1e-4);
    press(&mut app, pad, GamepadButton::DPadLeft);
    press(&mut app, pad, GamepadButton::DPadLeft);
    assert!((fraction(&app, ui.split) - (0.5 - 16.0 / room)).abs() < 1e-4);
    // Up and Down on a horizontal split's divider are not its: they move nothing here.
    let before = fraction(&app, ui.split);
    press(&mut app, pad, GamepadButton::DPadUp);
    assert_eq!(fraction(&app, ui.split), before);
    assert_eq!(focused(&app), Some(ui.divider));
    press(&mut app, pad, GamepadButton::South);
    assert!(
        (pane(&app, ui.first, H) - 100.0).abs() < 1.0,
        "South collapsed it"
    );
    press(&mut app, pad, GamepadButton::South);
    assert!(
        (fraction(&app, ui.split) - before).abs() < 1e-4,
        "and gave it back"
    );
}

#[test]
fn the_dpad_steps_a_vertical_divider_up_and_down() {
    let mut app = app();
    let pad = plug_in_pad(&mut app);
    let ui = rig(&mut app, V, 0.5, SplitMin::default());
    focus(&mut app, ui.divider);
    let room = room(&app, &ui, V);
    press(&mut app, pad, GamepadButton::DPadDown);
    assert!((fraction(&app, ui.split) - (0.5 + 16.0 / room)).abs() < 1e-4);
    press(&mut app, pad, GamepadButton::DPadLeft);
    assert!((fraction(&app, ui.split) - (0.5 + 16.0 / room)).abs() < 1e-4);
}

// ---- the pointer without a drag ----

#[test]
fn a_double_click_collapses_the_first_pane_and_a_second_gives_it_back() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.4, SplitMin::new(80.0, 0.0));
    let centre = rect(&app, ui.divider).center();
    click(&mut app, centre);
    assert_eq!(fraction(&app, ui.split), 0.4, "one click moves nothing");
    click(&mut app, centre);
    assert!(
        (pane(&app, ui.first, H) - 80.0).abs() < 1.0,
        "a double click collapses it: the pane is {} wide",
        pane(&app, ui.first, H)
    );
    // The divider has moved with the pane. A third click in the same burst is not another
    // double click.
    let centre = rect(&app, ui.divider).center();
    click(&mut app, centre);
    assert!((pane(&app, ui.first, H) - 80.0).abs() < 1.0);
    // After the interval the crate counts clicks from one again. Both this crate's `DoublePress`
    // (on `Time<Real>`) and Bevy's picking interval (on `Instant::now`) are wall clocks, so a host
    // under load made a fixed sleep a flake: instead, move the pointer off the divider for one
    // frame with the clock stepped past the interval, which is what a player's own pause does.
    app.world_mut()
        .resource_mut::<MotionSettings>()
        .duration_scale = 1.0;
    let away = rect(&app, ui.a).center();
    let seconds = app
        .world()
        .resource::<PointerSettings>()
        .double_click_seconds;
    app.world_mut()
        .insert_resource(Time::<Real>::default());
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(std::time::Duration::from_secs_f32(seconds * 4.0));
    point_at(&mut app, away);
    app.update();
    click(&mut app, centre);
    click(&mut app, centre);
    assert!(
        (fraction(&app, ui.split) - 0.4).abs() < 1e-3,
        "and again gives it back"
    );
}

#[test]
fn letting_go_of_a_drag_is_not_a_click() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.5, SplitMin::new(100.0, 0.0));
    // Two drags in a row, each ending on the divider: neither collapses the pane.
    drag(&mut app, ui.divider, Vec2::new(60.0, 0.0));
    let after_first = fraction(&app, ui.split);
    drag(&mut app, ui.divider, Vec2::new(60.0, 0.0));
    assert!(fraction(&app, ui.split) > after_first);
    assert!(
        pane(&app, ui.first, H) > 300.0,
        "the pane was dragged out, not collapsed"
    );
}

#[test]
fn a_press_on_the_divider_focuses_it_and_hides_the_ring_until_a_key() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    let centre = rect(&app, ui.divider).center();
    click(&mut app, centre);
    assert_eq!(focused(&app), Some(ui.divider));
    assert!(!app.world().resource::<InputFocusVisible>().0);
}

// ---- disabled ----

#[test]
fn a_disabled_divider_takes_focus_and_moves_nothing() {
    let mut app = app();
    let pad = plug_in_pad(&mut app);
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    app.world_mut()
        .entity_mut(ui.divider)
        .insert(InteractionDisabled);
    settle(&mut app);
    clear_changes(&mut app);
    drag(&mut app, ui.divider, Vec2::new(100.0, 0.0));
    focus(&mut app, ui.divider);
    for key in [
        KeyCode::ArrowRight,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Enter,
    ] {
        tap(&mut app, key);
    }
    press(&mut app, pad, GamepadButton::DPadRight);
    press(&mut app, pad, GamepadButton::South);
    assert_eq!(fraction(&app, ui.split), 0.5);
    assert!(changes(&app).is_empty());
    assert_eq!(focused(&app), Some(ui.divider));
}

// ---- the look ----

#[test]
fn the_divider_is_a_rail_with_a_grip_that_the_pointer_and_a_drag_light() {
    let mut app = app();
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    let skin = GooeySkin::default();
    let grip = app.world().get::<Children>(ui.divider).expect("children")[0];
    assert_eq!(fill_of(&app, ui.divider), skin.surface_raised);
    assert_eq!(fill_of(&app, grip), skin.text_muted);
    let centre = rect(&app, ui.divider).center();
    point_at(&mut app, centre);
    settle(&mut app);
    assert!(app.world().get::<Hovered>(ui.divider).expect("hover").0);
    assert_eq!(
        fill_of(&app, ui.divider),
        skin.hovered(),
        "the rail under the pointer"
    );
    assert_eq!(fill_of(&app, grip), skin.text, "the grip under the pointer");
    grab_and_move(&mut app, centre, centre + Vec2::new(30.0, 0.0));
    assert_eq!(fill_of(&app, grip), skin.accent, "while held");
    release(&mut app, centre + Vec2::new(30.0, 0.0));
    // The grip is a short pill in the middle of the channel.
    let (bar, pill) = (rect(&app, ui.divider), rect(&app, grip));
    assert!((pill.center() - bar.center()).length() < 1.0);
    assert!((pill.width() - 4.0).abs() < 0.6 && (pill.height() - 40.0).abs() < 0.6);
    // And a vertical one's lies along its axis.
    let ui = rig(&mut app, V, 0.5, SplitMin::default());
    let grip = app.world().get::<Children>(ui.divider).expect("children")[0];
    let pill = rect(&app, grip);
    assert!(pill.width() > pill.height());
}

#[test]
fn the_ring_shows_on_a_focused_divider_and_nowhere_else_in_every_preset() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app();
        app.insert_resource(skin.clone());
        let ui = rig(&mut app, H, 0.5, SplitMin::default());
        let ring = |app: &App| {
            app.world()
                .get::<BorderColor>(ui.divider)
                .expect("a border")
                .top
        };
        assert_eq!(
            ring(&app),
            Color::NONE,
            "{}: no ring at rest",
            preset.name()
        );
        focus(&mut app, ui.divider);
        assert_eq!(ring(&app), skin.focus_ring, "{}: the ring", preset.name());
        let line = app
            .world()
            .get::<ComputedNode>(ui.divider)
            .expect("laid out")
            .border
            .min_inset
            .x;
        assert!(
            line >= skin.border.floor(),
            "{}: a ring {line} px wide, the skin's border is {}",
            preset.name(),
            skin.border
        );
    }
}

#[test]
fn the_grip_reads_against_the_rail_in_every_preset() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app();
        app.insert_resource(skin.clone());
        let ui = rig(&mut app, H, 0.5, SplitMin::default());
        let grip = app.world().get::<Children>(ui.divider).expect("children")[0];
        let (rail, rest) = (fill_of(&app, ui.divider), fill_of(&app, grip));
        assert!(
            contrast_ratio(rgb(rest), rgb(rail)) >= NON_TEXT_CONTRAST,
            "{}: the grip at rest is {:.2}:1",
            preset.name(),
            contrast_ratio(rgb(rest), rgb(rail))
        );
        // Under the pointer the rail is lit and the grip, now the text colour, still reads.
        let centre = rect(&app, ui.divider).center();
        point_at(&mut app, centre);
        settle(&mut app);
        let (rail, lit) = (fill_of(&app, ui.divider), fill_of(&app, grip));
        assert!(
            contrast_ratio(rgb(lit), rgb(rail)) >= NON_TEXT_CONTRAST,
            "{}: the grip under the pointer is {:.2}:1",
            preset.name(),
            contrast_ratio(rgb(lit), rgb(rail))
        );
    }
}

#[test]
fn the_divider_is_the_same_target_in_the_tool_look_and_at_200_percent_text() {
    let mut app = app();
    app.insert_resource(GooeySkin::tool());
    let ui = rig(&mut app, H, 0.5, SplitMin::default());
    assert!((rect(&app, ui.divider).width() - BAR).abs() < 0.6);
    app.insert_resource(GooeySkin {
        text_scale: 2.0,
        ..default()
    });
    settle(&mut app);
    assert!(
        (rect(&app, ui.divider).width() - BAR).abs() < 0.6,
        "the text size does not widen it"
    );
}

#[test]
fn an_idle_split_writes_nothing() {
    let mut app = app();
    let _ui = rig(&mut app, H, 0.5, SplitMin::new(100.0, 100.0));
    settle(&mut app);
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0);
}

/// **The frame a split is spawned in already has its panes in their ratio.**
///
/// Measured by a caller of this crate (bevy_grimoire's Cauldron): entering the project page showed one
/// frame with the body across the whole window and the sidebar a few pixels wide, then the right
/// layout. `update_splits` runs in `GooeySystems::Style`, which is after layout, and a split spawned
/// this frame has no `ComputedNode` size yet — so `extent` is 0, `split_clamp` returns the fraction
/// unchanged, and the panes are laid out with `flex_grow: 0` and `0` before the first corrected write.
///
/// The ratio is read from the panes the frame the split exists, with **no** `settle` between.
#[test]
fn the_frame_a_split_is_spawned_in_has_its_panes_in_their_ratio() {
    let mut app = app_sized(800, 400);
    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(800.0),
                height: Val::Px(400.0),
                ..default()
            },
            children![(
                split(
                    SplitAxis::Horizontal,
                    (panel(), children![label("A")]),
                    (panel(), children![label("B")]),
                ),
                SplitFraction(0.0),
                SplitMin::new(240.0, 400.0),
            )],
        ))
        .id();

    // One frame: the spawn, its layout, and `Style`.
    app.update();
    let split = child_of(&app, container);
    let parts: Vec<Entity> = app
        .world()
        .get::<Children>(split)
        .expect("the split's children")
        .iter()
        .collect();
    let (first, second) = (parts[0], parts[2]);
    let a = rect(&app, first);
    let b = rect(&app, second);

    assert!(
        a.width() >= 240.0 - 1.0,
        "on the frame it was spawned the first pane is {} px wide, under its own 240 px minimum: \
         `update_splits` had no extent to clamp against and wrote `flex_grow: 0`. The second pane is \
         {} px wide where the split is 800 px (panes {a:?} and {b:?})",
        a.width(),
        b.width()
    );
    assert!(
        b.width() > 0.0,
        "on the frame it was spawned the second pane is {} px wide: one pane cannot be nothing \
         (panes {a:?} and {b:?})",
        b.width()
    );
    // **The panes and the divider account for the split**, measured from the divider's own rect rather
    // than from `BAR`: the grip is `MIN_TARGET_PX` wide but the divider node is laid out inside that,
    // and a nominal constant would make this assertion about the constant.
    let divider = rect(&app, parts[1]);
    assert!(
        (a.width() + b.width() + divider.width() - 800.0).abs() < 2.0,
        "the two panes and the divider take {} px of an 800 px split on its first frame ({a:?}, {b:?}, divider {divider:?})",
        a.width() + b.width() + divider.width()
    );
}

#[test]
fn f6_reaches_a_collapsed_pane_and_peeks_it() {
    let mut app = app();
    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(800.0),
                height: Val::Px(400.0),
                ..default()
            },
            children![(
                split(
                    H,
                    (panel(), children![button("Sidebar action")]),
                    (panel(), children![button("Main action")]),
                ),
                SplitFraction(0.4),
                SplitMin::new(80.0, 0.0),
            )],
        ))
        .id();
    settle(&mut app);
    let split = child_of(&app, container);
    let divider = {
        let mut stack = vec![split];
        let mut found = None;
        while let Some(entity) = stack.pop() {
            if let Some(kids) = app.world().get::<Children>(entity) {
                for &kid in &**kids {
                    if app.world().get::<Role>(kid) == Some(&Role::SplitDivider) {
                        found = Some(kid);
                    }
                    stack.push(kid);
                }
            }
        }
        found.expect("a divider")
    };
    // Collapse the first pane from its divider.
    let centre = rect(&app, divider).center();
    click(&mut app, centre);
    click(&mut app, centre);
    settle(&mut app);
    assert!(
        (pane(&app, child_of(&app, split), H) - 80.0).abs() < 1.0,
        "the first pane is collapsed"
    );
    // F6 from the second pane's button reaches the collapsed pane's own button (its first stop),
    // and landing there peeks the pane, so the player sees the widget they landed on.
    let main = {
        let mut stack = vec![
            *app.world()
                .get::<Children>(split)
                .expect("panes")
                .last()
                .expect("two"),
        ];
        let mut found = None;
        while let Some(entity) = stack.pop() {
            if app.world().get::<Role>(entity) == Some(&Role::Button) {
                found = Some(entity);
            }
            if let Some(kids) = app.world().get::<Children>(entity) {
                stack.extend(kids.iter());
            }
        }
        found.expect("the second pane's button")
    };
    focus(&mut app, main);
    tap(&mut app, KeyCode::F6);
    settle(&mut app);
    let reached = focused(&app);
    let in_first = reached.is_some_and(|e| {
        let mut at = e;
        loop {
            match app.world().get::<ChildOf>(at) {
                Some(parent) if parent.parent() == child_of(&app, split) => break true,
                Some(parent) => at = parent.parent(),
                None => break false,
            }
        }
    });
    assert!(
        in_first,
        "F6 goes to the collapsed pane's first widget, not past it: {reached:?}"
    );
    let peek = app.world().get::<SplitPeek>(split).expect("a peek");
    assert!(
        peek.is_open() || !peek_enabled(&app),
        "and the pane peeks around it, so the widget is on screen"
    );
}

/// Whether the app's split peeks at all (it is off under no motion, as the crate's tests run).
fn peek_enabled(app: &App) -> bool {
    app.world()
        .get_resource::<SplitPeekSettings>()
        .is_some_and(|s| s.enabled)
}
