//! Scrolls that glide and fade at their edges: a key or Tab that moves focus (a table's cursor
//! included) scrolls on the `motion.default` spring instead of jumping, a request mid-glide keeps
//! its speed, the wheel takes over, reduced motion lands at once, and a clipped list fades each
//! edge only where content is hidden, at `hidden / 15` of full. On a 60 Hz clock the test sets.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::ecs::spawn::SpawnIter;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::touch::TouchPhase;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::Pickable;
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, ScrollPosition, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// How many writes to a fade's node the app has seen.
#[derive(Resource, Default)]
struct FadeWrites(usize);

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

fn app(motion: MotionSettings) -> App {
    let mut app = App::new();
    app.insert_resource(motion);
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
    )));
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
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
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
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

fn scroll_of(app: &App, view: Entity) -> f32 {
    app.world().get::<ScrollPosition>(view).expect("scroll").y
}

fn view_of(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<ListView>>()
        .single(app.world())
        .expect("one list")
}

/// A list of `n` buttons 4.5 rows tall, in a panel on a screen. Returns the viewport and the rows.
fn saves(app: &mut App, n: usize) -> (Entity, Vec<Entity>) {
    let root = app
        .world_mut()
        .spawn(list(
            4.5,
            Children::spawn(SpawnIter((0..n).map(|i| button(format!("Slot {i:02}"))))),
        ))
        .id();
    let card = app.world_mut().spawn(panel()).add_child(root).id();
    app.world_mut().spawn(screen()).add_child(card);
    frames(app, 8);
    let view = view_of(app);
    let column = app.world().get::<Children>(view).expect("rows")[0];
    let rows = app
        .world()
        .get::<Children>(column)
        .expect("rows")
        .iter()
        .collect();
    (view, rows)
}

fn wheel(app: &mut App, at: Vec2, lines: f32) {
    let window = window(app);
    let location = Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    };
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Scroll {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -lines,
            phase: TouchPhase::Moved,
        },
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
        app.update();
    }
    app.update();
}

#[test]
fn a_focus_moved_far_down_a_list_glides_there_instead_of_jumping() {
    let mut app = app(MotionSettings::default());
    let (view, rows) = saves(&mut app, 40);
    let jump = {
        let mut twin = self::app(MotionSettings::off());
        let (twin_view, twin_rows) = saves(&mut twin, 40);
        focus(&mut twin, twin_rows[20]);
        frames(&mut twin, 4);
        scroll_of(&twin, twin_view)
    };
    assert!(jump > 100.0, "the twin jumps a long way: {jump}");

    focus(&mut app, rows[20]);
    frames(&mut app, 3);
    let early = scroll_of(&app, view);
    assert!(
        early > 0.0 && early < jump * 0.9,
        "three frames in it is on the way, {early} of {jump}"
    );
    frames(&mut app, 120);
    assert!(
        (scroll_of(&app, view) - jump).abs() < 0.6,
        "it lands where the jump did: {} vs {jump}",
        scroll_of(&app, view)
    );
    assert_eq!(
        app.world().get::<ScrollGlide>(view).expect("glide").goal(),
        None,
        "a landed glide holds no goal"
    );
}

#[test]
fn a_new_request_mid_glide_keeps_the_speed_and_the_wheel_takes_over() {
    let mut app = app(MotionSettings::default());
    let (view, rows) = saves(&mut app, 40);
    focus(&mut app, rows[12]);
    frames(&mut app, 6);
    let before = scroll_of(&app, view);
    frames(&mut app, 1);
    let step = scroll_of(&app, view) - before;
    assert!(step > 1.0, "moving: {step}");
    // Ask for a row further on while it moves: no frame stops, and the speed stays.
    focus(&mut app, rows[18]);
    let mut at = scroll_of(&app, view);
    let mut steps = Vec::new();
    for _ in 0..6 {
        app.update();
        let now = scroll_of(&app, view);
        steps.push(now - at);
        at = now;
    }
    assert!(
        steps.iter().all(|&s| s > step * 0.5),
        "the retarget kept its speed: {steps:?} after {step}"
    );

    // A wheel notch takes over: it moves 1:1 and the glide ends where the wheel left it.
    let over = rect(&app, view).center();
    let before = scroll_of(&app, view);
    wheel(&mut app, over, 1.0);
    let after = scroll_of(&app, view);
    assert!(after != before, "the wheel moved it");
    frames(&mut app, 120);
    assert!(
        (scroll_of(&app, view) - after).abs() < 1.0,
        "no glide went on after the wheel: {} after {after}",
        scroll_of(&app, view)
    );
}

#[test]
fn one_wheel_notch_scrolls_the_same_with_the_glide_as_without() {
    let notch = |motion: MotionSettings| {
        let mut app = app(motion);
        let (view, _) = saves(&mut app, 40);
        let over = rect(&app, view).center();
        wheel(&mut app, over, 1.0);
        frames(&mut app, 4);
        scroll_of(&app, view)
    };
    let (glide, plain) = (
        notch(MotionSettings::default()),
        notch(MotionSettings::off()),
    );
    assert!(plain > 0.0, "a notch scrolls");
    assert_eq!(glide, plain, "the wheel is 1:1, with a glide or none");
}

#[test]
fn reduced_motion_and_no_motion_land_a_scroll_at_once() {
    for motion in [MotionSettings::reduced(), MotionSettings::off()] {
        let mut app = app(motion.clone());
        let (view, rows) = saves(&mut app, 40);
        focus(&mut app, rows[20]);
        frames(&mut app, 2);
        let landed = scroll_of(&app, view);
        frames(&mut app, 60);
        assert!(
            landed > 100.0,
            "it landed in two frames: {landed} under {motion:?}"
        );
        assert_eq!(scroll_of(&app, view), landed, "and stayed under {motion:?}");
    }
}

fn the_table(app: &mut App, rows: usize) -> (Entity, Entity) {
    let items: Vec<Vec<String>> = (0..rows).map(|i| vec![format!("Item {i}")]).collect();
    app.world_mut().spawn((
        panel(),
        children![table([Column::new("Name").width(14.0)], items)],
    ));
    frames(app, 8);
    let table = app
        .world_mut()
        .query_filtered::<Entity, With<Table>>()
        .single(app.world())
        .expect("one table");
    let view = app
        .world_mut()
        .query_filtered::<(Entity, &Role), With<ListView>>()
        .iter(app.world())
        .find(|(_, role)| **role == Role::TableView)
        .map(|(entity, _)| entity)
        .expect("a table viewport");
    (table, view)
}

#[test]
fn holding_down_in_a_ten_thousand_row_table_scrolls_as_one_motion() {
    let mut app = app(MotionSettings::default());
    let (table, view) = the_table(&mut app, 10_000);
    focus(&mut app, table);
    frames(&mut app, 2);
    key(&mut app, KeyCode::ArrowDown, ButtonState::Pressed);
    key(&mut app, KeyCode::ArrowDown, ButtonState::Released);
    frames(&mut app, 2);
    let pitch = app.world().resource::<GooeySkin>().table_pitch();
    let view_rect = rect(&app, view);

    // The key repeats at 30 Hz for 3 s on a 60 Hz clock.
    let mut steps = Vec::new();
    let mut at = scroll_of(&app, view);
    let mut outside = 0;
    for frame in 0..180 {
        if frame % 2 == 0 {
            key(&mut app, KeyCode::ArrowDown, ButtonState::Pressed);
            key(&mut app, KeyCode::ArrowDown, ButtonState::Released);
        }
        app.update();
        let now = scroll_of(&app, view);
        steps.push(now - at);
        at = now;
        let cursor_row = app
            .world()
            .get::<TableCursor>(table)
            .and_then(|c| c.0)
            .map(|key| key as f32)
            .unwrap_or(0.0);
        let top = cursor_row * pitch - now;
        if top < -0.5 || top + pitch > view_rect.height() + 0.5 {
            outside += 1;
        }
        // The rows drawn cover the viewport: bound to the scroll this frame, not the last.
        if frame >= 8 {
            let keys: Vec<u64> = app
                .world_mut()
                .query::<&TableRow>()
                .iter(app.world())
                .map(|row| row.index as u64)
                .collect();
            let (first, last) = (
                *keys.iter().min().expect("rows") as f32,
                *keys.iter().max().expect("rows") as f32,
            );
            assert!(
                first * pitch <= now + 0.5
                    && (last + 1.0) * pitch >= now + view_rect.height() - 0.5,
                "frame {frame}: rows {first}..={last} cover the viewport at {now}"
            );
        }
    }
    // The first frames ramp up from rest and the last frames of a key still move a row: judge the
    // steady part, the second and third seconds.
    let steady = &steps[60..170];
    let mut sorted = steady.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let median = sorted[sorted.len() / 2];
    assert!(median > 1.0, "it is moving: median step {median}");
    let worst = sorted.last().copied().expect("steps");
    assert!(
        worst <= 2.0 * median,
        "no frame's step exceeds twice the median: worst {worst}, median {median}"
    );
    assert!(
        steady.iter().all(|&s| s >= 0.0),
        "it never turns back: {steady:?}"
    );
    assert!(
        outside <= 1,
        "the cursor row stays in the viewport (it left it on {outside} frames)"
    );
    // At rest the cursor stands a row clear of the viewport's foot.
    frames(&mut app, 180);
    let cursor_row = app
        .world()
        .get::<TableCursor>(table)
        .and_then(|c| c.0)
        .expect("a cursor") as f32;
    let foot = cursor_row * pitch + pitch - scroll_of(&app, view);
    assert!(
        view_rect.height() - foot >= pitch - 0.6,
        "a row of table stays beyond the cursor: {} left",
        view_rect.height() - foot
    );
}

fn overlay(app: &mut App, view: Entity, edge: FadeEdge) -> (Entity, f32, Node) {
    let mut found = app
        .world_mut()
        .query::<(Entity, &ScrollFade, &Node)>()
        .iter(app.world())
        .filter(|(_, fade, _)| fade.view == view && fade.edge == edge)
        .map(|(entity, fade, node)| (entity, fade.strength, node.clone()))
        .collect::<Vec<_>>();
    assert_eq!(found.len(), 1, "one {edge:?} fade on the viewport");
    found.remove(0)
}

fn alpha_of(app: &App, entity: Entity) -> f32 {
    let gradient = app
        .world()
        .get::<BackgroundGradient>(entity)
        .expect("a gradient");
    match &gradient.0[0] {
        Gradient::Linear(linear) => linear.stops[0].color.alpha(),
        other => panic!("a linear gradient, not {other:?}"),
    }
}

fn set_scroll(app: &mut App, view: Entity, y: f32) {
    app.world_mut()
        .get_mut::<ScrollPosition>(view)
        .expect("scroll")
        .y = y;
    frames(app, 3);
}

#[test]
fn an_edge_fades_only_where_content_is_hidden_and_at_hidden_over_fifteen() {
    let mut app = app(MotionSettings::off());
    let (view, _) = saves(&mut app, 40);
    frames(&mut app, 3);
    let (top, strength, node) = overlay(&mut app, view, FadeEdge::Top);
    assert_eq!(strength, 0.0, "nothing is hidden above at the top");
    assert_eq!(node.display, Display::None, "so the fade draws nothing");
    let (bottom, strength, node) = overlay(&mut app, view, FadeEdge::Bottom);
    assert_eq!(strength, 1.0, "a long list hides plenty below");
    assert_eq!(node.display, Display::Flex);
    assert!(alpha_of(&app, bottom) > 0.0);
    for edge in [top, bottom] {
        assert_eq!(
            app.world().get::<Pickable>(edge),
            Some(&Pickable::IGNORE),
            "a fade never takes a click"
        );
    }

    // 15 px down the top is at full strength, half of that at half.
    set_scroll(&mut app, view, 15.0);
    let (top, strength, node) = overlay(&mut app, view, FadeEdge::Top);
    assert_eq!(strength, 1.0, "15 px hidden above is full");
    assert_eq!(node.display, Display::Flex);
    let full = alpha_of(&app, top);
    set_scroll(&mut app, view, 7.5);
    let (top, strength, _) = overlay(&mut app, view, FadeEdge::Top);
    assert!((strength - 0.5).abs() < 0.07, "half of 15 px: {strength}");
    let half = alpha_of(&app, top);
    assert!(
        (half - full * 0.5).abs() < 0.06,
        "half the strength is half the alpha: {half} of {full}"
    );
    set_scroll(&mut app, view, 0.0);
    let (_, strength, node) = overlay(&mut app, view, FadeEdge::Top);
    assert_eq!(
        (strength, node.display),
        (0.0, Display::None),
        "back at the top"
    );

    // At the end nothing is hidden below.
    set_scroll(&mut app, view, 100_000.0);
    let (_, strength, node) = overlay(&mut app, view, FadeEdge::Bottom);
    assert_eq!(
        (strength, node.display),
        (0.0, Display::None),
        "nothing hidden below at the end"
    );
}

#[test]
fn a_list_at_rest_writes_no_fade_and_a_short_list_has_none_showing() {
    let mut app = app(MotionSettings::off());
    app.init_resource::<FadeWrites>().add_systems(
        Update,
        |fades: Query<(), (With<ScrollFade>, Changed<Node>)>, mut writes: ResMut<FadeWrites>| {
            writes.0 += fades.iter().count();
        },
    );
    let (view, _) = saves(&mut app, 3);
    frames(&mut app, 3);
    for edge in [FadeEdge::Top, FadeEdge::Bottom] {
        let (_, strength, node) = overlay(&mut app, view, edge);
        assert_eq!(
            (strength, node.display),
            (0.0, Display::None),
            "a list that fits fades no {edge:?}"
        );
    }
    let before = app.world().resource::<FadeWrites>().0;
    frames(&mut app, 5);
    let after = app.world().resource::<FadeWrites>().0;
    assert_eq!(before, after, "a settled fade is not written");
}

#[test]
fn a_sort_while_scrolled_down_brings_the_cursor_row_into_view() {
    let mut app = app(MotionSettings::default());
    let (table, view) = the_table(&mut app, 300);
    focus(&mut app, table);
    frames(&mut app, 2);
    // Put the cursor on row 5, then scroll far from it.
    app.world_mut()
        .get_mut::<TableCursor>(table)
        .expect("cursor")
        .0 = Some(5);
    frames(&mut app, 4);
    app.world_mut()
        .get_mut::<ScrollPosition>(view)
        .expect("scroll")
        .y = 6_000.0;
    frames(&mut app, 4);
    // Sorting largest first puts row 5 at the foot of the 300.
    *app.world_mut().get_mut::<TableSort>(table).expect("sort") = TableSort {
        column: Some(0),
        descending: false,
    };
    frames(&mut app, 4);
    *app.world_mut().get_mut::<TableSort>(table).expect("sort") = TableSort {
        column: Some(0),
        descending: true,
    };
    frames(&mut app, 120);
    let pitch = app.world().resource::<GooeySkin>().table_pitch();
    let rows: Vec<(u64, usize)> = app
        .world_mut()
        .query::<&TableRow>()
        .iter(app.world())
        .map(|row| (row.key, row.index))
        .collect();
    assert!(!rows.is_empty(), "rows are drawn");
    let view_rect = rect(&app, view);
    let at = scroll_of(&app, view);
    let cursor = app
        .world()
        .get::<TableCursor>(table)
        .expect("cursor")
        .0
        .expect("a cursor");
    assert_eq!(cursor, 5, "the cursor stayed on its row");
    let index = rows
        .iter()
        .find(|(key, _)| *key == 5)
        .map(|&(_, index)| index)
        .expect("the cursor row is among the rows drawn");
    let top = index as f32 * pitch - at;
    assert!(
        top >= -0.5 && top + pitch <= view_rect.height() + 0.5,
        "the cursor row stands in the viewport: {top} of {} (scroll {at})",
        view_rect.height()
    );
}
