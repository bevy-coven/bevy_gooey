//! A collapsed split's first pane peeks (#69): the pointer resting on its edge floats it over the
//! second pane, which does not move; the pointer leaving, or Escape, takes it away; a toggle or a
//! drag docks it where it floats. On a clock the test sets (30 ms a frame).

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{BoxShadow, ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

const FRAME: f32 = 0.03;
/// The divider's thickness.
const BAR: f32 = 24.0;

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
    ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )));
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

struct Rig {
    split: Entity,
    first: Entity,
    divider: Entity,
    second: Entity,
}

/// An 800 px split of two panels, the first at 40% with a minimum of 100 px, collapsed by Enter on
/// its divider and settled.
fn collapsed(app: &mut App) -> Rig {
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
                    (panel(), children![label("A"), button("Sidebar")]),
                    (panel(), children![label("B")]),
                ),
                SplitFraction(0.4),
                SplitMin::new(100.0, 100.0),
            )],
        ))
        .id();
    frames(app, 10);
    let split = app.world().get::<Children>(container).expect("kids")[0];
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(split)
        .expect("kids")
        .iter()
        .collect();
    let rig = Rig {
        split,
        first: kids[0],
        divider: kids[1],
        second: kids[2],
    };
    focus(app, rig.divider);
    tap(app, KeyCode::Enter);
    frames(app, 40);
    assert!(
        app.world().get::<SplitSaved>(split).is_some(),
        "it is collapsed"
    );
    // The pointer is somewhere that is not the split.
    point_at(app, Vec2::new(1000.0, 600.0));
    frames(app, 10);
    rig
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    frames(app, 3);
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn tap(app: &mut App, key_code: KeyCode) {
    let window = window(app);
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

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let window = window(app);
    let target = WindowRef::Entity(window)
        .normalize(None)
        .expect("a window ref");
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(target),
            position: at,
        },
        action,
    ));
    app.update();
}

fn point_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
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

fn peek(app: &App, rig: &Rig) -> SplitPeek {
    *app.world().get::<SplitPeek>(rig.split).expect("a peek")
}

fn floating(app: &App, rig: &Rig) -> bool {
    app.world()
        .get::<Node>(rig.first)
        .expect("node")
        .position_type
        == PositionType::Absolute
}

fn fraction(app: &App, rig: &Rig) -> f32 {
    app.world()
        .get::<SplitFraction>(rig.split)
        .expect("state")
        .0
}

/// The second pane's size and place, to compare before and during a peek.
fn second_pane(app: &App, rig: &Rig) -> (Vec2, Vec2) {
    let r = rect(app, rig.second);
    (r.size(), r.center())
}

/// Rests the pointer on the divider for `seconds`.
fn rest_on_divider(app: &mut App, rig: &Rig, seconds: f32) {
    let at = rect(app, rig.divider).center();
    point_at(app, at);
    frames(app, (seconds / FRAME).ceil() as usize);
}

#[test]
fn the_pointer_resting_on_the_collapsed_edge_floats_the_pane_over_the_second_pane_which_stays() {
    let mut app = app(MotionSettings::default());
    let rig = collapsed(&mut app);
    let before = second_pane(&app, &rig);
    // Less than the delay: nothing.
    let at = rect(&app, rig.divider).center();
    point_at(&mut app, at);
    assert!(!peek(&app, &rig).is_open());
    // Past it: the pane floats.
    frames(&mut app, 6);
    assert!(peek(&app, &rig).is_open(), "{:?}", peek(&app, &rig));
    assert!(floating(&app, &rig));
    frames(&mut app, 30);
    // As wide as a restore would make it, drawn from the split's start edge, over the second pane.
    let room = 800.0 - BAR;
    let width = rect(&app, rig.first).width();
    assert!(
        (width - 0.4 * room).abs() < 1.5,
        "the floating pane is {width} wide, a restore gives {}",
        0.4 * room
    );
    let (pane, second) = (rect(&app, rig.first), rect(&app, rig.second));
    assert!((pane.min.x - rect(&app, rig.split).min.x).abs() < 1.0);
    assert!(
        pane.max.x > second.min.x + 20.0,
        "it covers the second pane's start"
    );
    // And the second pane has not moved or changed size.
    let during = second_pane(&app, &rig);
    assert!(
        (during.0 - before.0).length() < 0.6,
        "{during:?} against {before:?}"
    );
    assert!(
        (during.1 - before.1).length() < 0.6,
        "{during:?} against {before:?}"
    );
    assert!(app.world().get::<SplitSaved>(rig.split).is_some());
    // The floating pane is opaque (the second pane's words must not show through it) and casts a
    // shadow; its divider stands on its far edge.
    let fill = app
        .world()
        .get::<BackgroundColor>(rig.first)
        .expect("a fill while it floats")
        .0;
    assert_eq!(Srgba::from(fill).alpha, 1.0);
    assert!(app.world().get::<BoxShadow>(rig.first).is_some());
    let (pane, bar) = (rect(&app, rig.first), rect(&app, rig.divider));
    assert!(
        (bar.min.x - pane.max.x).abs() < 1.5,
        "the divider is on the pane's far edge"
    );
}

#[test]
fn a_pointer_that_only_passes_the_edge_peeks_nothing() {
    let mut app = app(MotionSettings::default());
    let rig = collapsed(&mut app);
    let at = rect(&app, rig.divider).center();
    point_at(&mut app, at);
    frames(&mut app, 1);
    point_at(&mut app, Vec2::new(1000.0, 600.0));
    frames(&mut app, 20);
    assert!(!peek(&app, &rig).is_open());
    assert!(!floating(&app, &rig));
}

#[test]
fn the_peek_stays_while_the_pointer_is_on_it_and_goes_after_the_grace_when_it_leaves() {
    let mut app = app(MotionSettings::default());
    let rig = collapsed(&mut app);
    rest_on_divider(&mut app, &rig, 0.3);
    frames(&mut app, 20);
    assert!(peek(&app, &rig).is_open());
    // Onto the floating pane itself (hoverable): it holds, however long.
    let pane = rect(&app, rig.first);
    point_at(&mut app, pane.center());
    frames(&mut app, 40);
    assert!(
        peek(&app, &rig).is_open(),
        "the pointer on the pane holds it"
    );
    // Away: still there inside the grace, gone after it and its slide.
    point_at(&mut app, Vec2::new(1000.0, 600.0));
    frames(&mut app, 2);
    assert!(peek(&app, &rig).is_open(), "inside the grace");
    frames(&mut app, 40);
    assert!(!peek(&app, &rig).is_open());
    assert!(!floating(&app, &rig), "back in the flow");
    assert!(
        app.world().get::<SplitSaved>(rig.split).is_some(),
        "still collapsed"
    );
    assert!(rect(&app, rig.first).width() < 101.0);
    assert_eq!(
        app.world()
            .get::<UiTransform>(rig.first)
            .expect("node")
            .translation,
        Val2::ZERO,
        "no slide is left on the pane"
    );
}

#[test]
fn escape_closes_a_peek_with_the_pointer_still_on_it() {
    let mut app = app(MotionSettings::default());
    let rig = collapsed(&mut app);
    rest_on_divider(&mut app, &rig, 0.3);
    frames(&mut app, 20);
    assert!(peek(&app, &rig).is_open());
    tap(&mut app, KeyCode::Escape);
    frames(&mut app, 40);
    assert!(!floating(&app, &rig));
}

#[test]
fn toggling_a_peek_docks_it_at_the_width_it_floats_with_no_glide() {
    let mut app = app(MotionSettings::default());
    let rig = collapsed(&mut app);
    rest_on_divider(&mut app, &rig, 0.3);
    frames(&mut app, 30);
    let width = rect(&app, rig.first).width();
    focus(&mut app, rig.divider);
    tap(&mut app, KeyCode::Enter);
    frames(&mut app, 2);
    assert!(
        app.world().get::<SplitSaved>(rig.split).is_none(),
        "restored"
    );
    assert!(
        app.world().get::<SplitGlide>(rig.split).is_none(),
        "no glide: it is already there"
    );
    assert!(!floating(&app, &rig), "in the flow");
    let docked = rect(&app, rig.first).width();
    assert!(
        (docked - width).abs() < 1.5,
        "docked at {docked}, it floated at {width}"
    );
    assert!((fraction(&app, &rig) - 0.4).abs() < 0.01);
}

#[test]
fn a_drag_that_starts_on_the_divider_of_a_peek_docks_it_and_drags_from_there() {
    let mut app = app(MotionSettings::default());
    let rig = collapsed(&mut app);
    rest_on_divider(&mut app, &rig, 0.3);
    frames(&mut app, 30);
    // The divider stands on the floating pane's far edge, above it, where a press reaches it.
    let at = rect(&app, rig.divider).center();
    point_at(&mut app, at);
    pointer(&mut app, at, PointerAction::Press(PointerButton::Primary));
    pointer(
        &mut app,
        at + Vec2::new(40.0, 0.0),
        PointerAction::Move {
            delta: Vec2::new(40.0, 0.0),
        },
    );
    pointer(
        &mut app,
        at + Vec2::new(40.0, 0.0),
        PointerAction::Release(PointerButton::Primary),
    );
    frames(&mut app, 10);
    assert!(app.world().get::<SplitSaved>(rig.split).is_none(), "docked");
    assert!(!floating(&app, &rig));
    let room = 800.0 - BAR;
    assert!(
        (fraction(&app, &rig) - (0.4 + 40.0 / room)).abs() < 0.01,
        "dragged on from the docked width: {}",
        fraction(&app, &rig)
    );
}

#[test]
fn an_open_split_never_peeks() {
    let mut app = app(MotionSettings::default());
    let rig = collapsed(&mut app);
    focus(&mut app, rig.divider);
    tap(&mut app, KeyCode::Enter);
    frames(&mut app, 40);
    assert!(app.world().get::<SplitSaved>(rig.split).is_none());
    rest_on_divider(&mut app, &rig, 0.5);
    frames(&mut app, 10);
    assert!(!peek(&app, &rig).is_open());
}

#[test]
fn reduced_motion_fades_the_pane_in_and_never_slides_it() {
    let mut app = app(MotionSettings::reduced());
    let rig = collapsed(&mut app);
    let at = rect(&app, rig.divider).center();
    point_at(&mut app, at);
    let mut seen = false;
    for _ in 0..14 {
        app.update();
        seen |= app.world().get::<Opacity>(rig.first).is_some();
        assert_eq!(
            app.world()
                .get::<UiTransform>(rig.first)
                .expect("node")
                .translation,
            Val2::ZERO
        );
    }
    assert!(peek(&app, &rig).is_open());
    assert!(seen, "it faded in");
}

#[test]
fn no_motion_opens_it_after_the_dwell_and_closes_it_after_the_grace_with_nothing_drawn_between() {
    let mut app = app(MotionSettings::off());
    let rig = collapsed(&mut app);
    rest_on_divider(&mut app, &rig, 0.2);
    assert!(peek(&app, &rig).is_open());
    assert!(floating(&app, &rig));
    point_at(&mut app, Vec2::new(1000.0, 600.0));
    frames(&mut app, 12);
    assert!(!floating(&app, &rig));
}

#[test]
fn a_collapsed_idle_split_writes_nothing() {
    let mut app = app(MotionSettings::default());
    let rig = collapsed(&mut app);
    frames(&mut app, 10);
    for _ in 0..20 {
        app.update();
        for e in [rig.split, rig.first, rig.second, rig.divider] {
            let r = app.world().entity(e);
            if let Some(node) = r.get_ref::<Node>() {
                assert!(!node.is_changed(), "a settled split's node is written");
            }
        }
    }
}

#[test]
fn a_disabled_divider_does_not_peek() {
    let mut app = app(MotionSettings::default());
    let rig = collapsed(&mut app);
    app.world_mut()
        .entity_mut(rig.divider)
        .insert(bevy::ui::InteractionDisabled);
    rest_on_divider(&mut app, &rig, 0.5);
    assert!(!peek(&app, &rig).is_open());
}
