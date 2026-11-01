//! A popover as a game sees it, laid out in a headless window and driven by real pointer and
//! keyboard input: placed on the side asked for, flipped or slid to stay inside the window, closed
//! once by a press outside or by Back, never pressing the anchor it is a child of, holding focus
//! only when modal, and drawn as a card in the skin's colours.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{AutoFocus, InputFocus};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedUiRenderTargetInfo, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// What a restyle writes.
type Written = Or<(
    Changed<Node>,
    Changed<BackgroundColor>,
    Changed<BorderColor>,
)>;

#[derive(Resource, Default)]
struct Writes(u32);

/// Every `PopoverClosed` a game's observer heard, in order.
#[derive(Resource, Default)]
struct Closed(Vec<(Entity, CloseReason)>);

/// Every button activated, by its label.
#[derive(Resource, Default)]
struct Activated(Vec<&'static str>);

fn app_with(skin: GooeySkin) -> App {
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
    .init_resource::<Writes>()
    .init_resource::<Closed>()
    .init_resource::<Activated>()
    .insert_resource(skin)
    .add_observer(|closed: On<PopoverClosed>, mut list: ResMut<Closed>| {
        list.0.push((closed.entity, closed.reason));
    })
    .add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn app() -> App {
    app_with(GooeySkin::default())
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// Where `entity` is drawn, in the interface's logical px.
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

/// The window, in the interface's logical px, as a popover is held inside it.
fn window(app: &App, entity: Entity) -> Rect {
    let target = app
        .world()
        .get::<ComputedUiRenderTargetInfo>(entity)
        .expect("a target");
    Rect::from_corners(Vec2::ZERO, target.logical_size())
}

/// Bevy's `Popover` aligns to the anchor's padding box, a frame inside its edge.
const EDGE: f32 = 4.0;

fn gone(app: &App, entity: Entity) -> bool {
    app.world().get_entity(entity).is_err()
}

/// A button at `left`, `top` of the window: an anchor.
fn anchor_at(app: &mut App, name: &'static str, left: Val, top: Val) -> Entity {
    let world = app.world_mut();
    let anchor = world
        .spawn(button(name))
        .observe(move |_: On<Activate>, mut log: ResMut<Activated>| log.0.push(name)) // a game's own
        .id();
    let mut node = world.get_mut::<Node>(anchor).expect("a node");
    node.position_type = PositionType::Absolute;
    node.left = left;
    node.top = top;
    anchor
}

/// A popover as a game writes it: a line and two buttons.
fn open(app: &mut App, anchor: Entity, placement: Placement) -> Entity {
    app.world_mut()
        .spawn((
            popover(anchor, placement),
            children![
                label("A line of text that makes the card wide"),
                button("Copy"),
                button("Paste"),
            ],
        ))
        .id()
}

fn open_modal(app: &mut App, anchor: Entity, placement: Placement) -> Entity {
    app.world_mut()
        .spawn((
            popover_modal(anchor, placement),
            children![button("New"), button("Open")],
        ))
        .id()
}

fn mouse(app: &mut App, at: Vec2, action: PointerAction) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window");
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
    mouse(app, at, PointerAction::Move { delta: Vec2::ZERO });
    for _ in 0..2 {
        app.update();
    }
}

/// A primary click: the mouse moves there and settles, then presses and releases.
fn click_at(app: &mut App, at: Vec2) {
    point_at(app, at);
    for action in [
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        mouse(app, at, action);
        app.update();
    }
    settle(app);
}

/// A click on the middle of `entity`.
fn click_on(app: &mut App, entity: Entity) {
    let at = rect(app, entity).center();
    click_at(app, at);
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
    settle(app);
}

fn focus(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn inside(app: &App, entity: Option<Entity>, root: Entity) -> bool {
    let mut at = entity;
    while let Some(e) = at {
        if e == root {
            return true;
        }
        at = app.world().get::<ChildOf>(e).map(ChildOf::parent);
    }
    false
}

fn closed(app: &App) -> Vec<(Entity, CloseReason)> {
    app.world().resource::<Closed>().0.clone()
}

/// The `Text` node under `root` that reads `text`.
fn text_node(app: &mut App, root: Entity, text: &str) -> Entity {
    let world = app.world_mut();
    let mut found = None;
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if let Some(t) = world.get::<Text>(e)
            && t.0 == text
        {
            found = Some(e);
        }
        if let Some(kids) = world.get::<Children>(e) {
            stack.extend(kids.iter());
        }
    }
    found.expect("a widget with that text")
}

/// The button under `root` labelled `text`.
fn child_named(app: &mut App, root: Entity, text: &str) -> Entity {
    let text = text_node(app, root, text);
    app.world()
        .get::<ChildOf>(text)
        .map(ChildOf::parent)
        .expect("a button around its text")
}

#[test]
fn a_popover_anchored_at_the_right_edge_is_inside_the_window_and_a_press_outside_closes_it_once() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(1130.0), Val::Px(300.0));
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Right);
    settle(&mut app);
    let (panel_rect, anchor_rect) = (rect(&app, panel), rect(&app, anchor));
    let win = window(&app, panel);
    assert!(
        panel_rect.min.x >= win.min.x && panel_rect.max.x <= win.max.x,
        "the panel is inside the window: {panel_rect:?} in {win:?}"
    );
    assert!(
        panel_rect.max.x <= anchor_rect.min.x,
        "flipped to the left of the anchor: {panel_rect:?} against {anchor_rect:?}"
    );
    click_at(&mut app, Vec2::new(300.0, 100.0));
    assert!(gone(&app, panel), "a press outside closed it");
    assert_eq!(closed(&app), vec![(panel, CloseReason::Outside)]);
    click_at(&mut app, Vec2::new(300.0, 100.0));
    assert_eq!(closed(&app).len(), 1, "once");
}

#[test]
fn each_side_opens_on_that_side_when_there_is_room_aligned_to_the_anchors_edge() {
    for (placement, at) in [
        (Placement::Below, (400.0, 300.0)),
        (Placement::Above, (400.0, 300.0)),
        (Placement::Right, (400.0, 300.0)),
        (Placement::Left, (600.0, 300.0)),
    ] {
        let mut app = app();
        let anchor = anchor_at(&mut app, "Edit", Val::Px(at.0), Val::Px(at.1));
        settle(&mut app);
        let panel = open(&mut app, anchor, placement);
        settle(&mut app);
        let (p, a) = (rect(&app, panel), rect(&app, anchor));
        match placement {
            Placement::Below => {
                assert!(p.min.y >= a.max.y, "{placement:?}: below {p:?} {a:?}");
                assert!(
                    (p.min.x - a.min.x).abs() <= EDGE,
                    "{placement:?}: left edges"
                );
            }
            Placement::Above => {
                assert!(p.max.y <= a.min.y, "{placement:?}: above {p:?} {a:?}");
                assert!(
                    (p.min.x - a.min.x).abs() <= EDGE,
                    "{placement:?}: left edges"
                );
            }
            Placement::Right => {
                assert!(p.min.x >= a.max.x, "{placement:?}: right of {p:?} {a:?}");
                assert!(
                    (p.min.y - a.min.y).abs() <= EDGE,
                    "{placement:?}: top edges"
                );
            }
            Placement::Left => {
                assert!(p.max.x <= a.min.x, "{placement:?}: left of {p:?} {a:?}");
                assert!(
                    (p.min.y - a.min.y).abs() <= EDGE,
                    "{placement:?}: top edges"
                );
            }
            Placement::AtPointer => unreachable!(),
        }
        let gap = if matches!(placement, Placement::Below | Placement::Above) {
            (p.min.y - a.max.y).max(a.min.y - p.max.y)
        } else {
            (p.min.x - a.max.x).max(a.min.x - p.max.x)
        };
        assert!(gap > 0.0, "{placement:?}: never touching the anchor: {gap}");
    }
}

#[test]
fn a_popover_with_no_room_below_opens_above() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(400.0), Val::Px(670.0));
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Below);
    settle(&mut app);
    let (p, a) = (rect(&app, panel), rect(&app, anchor));
    assert!(p.max.y <= a.min.y, "above the anchor: {p:?} {a:?}");
    assert!(p.min.y >= 0.0);
}

#[test]
fn a_popover_wider_than_the_room_beside_its_anchors_start_slides_to_its_end() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(1150.0), Val::Px(300.0));
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Below);
    settle(&mut app);
    let (p, a) = (rect(&app, panel), rect(&app, anchor));
    assert!(p.min.y >= a.max.y, "still below: {p:?} {a:?}");
    assert!(
        p.max.x <= window(&app, panel).max.x,
        "inside the window: {p:?}"
    );
    assert!(
        (p.max.x - a.max.x).abs() <= EDGE,
        "right edges together: {p:?} {a:?}"
    );
}

#[test]
fn a_popover_at_the_pointer_opens_there_and_flips_at_the_windows_edges() {
    for (at, flips_x, flips_y) in [
        (Vec2::new(400.0, 300.0), false, false),
        (Vec2::new(1270.0, 300.0), true, false),
        (Vec2::new(400.0, 715.0), false, true),
        (Vec2::new(1270.0, 715.0), true, true),
    ] {
        let mut app = app();
        let region = app
            .world_mut()
            .spawn(Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            })
            .id();
        settle(&mut app);
        point_at(&mut app, at);
        let panel = open(&mut app, region, Placement::AtPointer);
        settle(&mut app);
        let p = rect(&app, panel);
        let win = window(&app, panel);
        assert!(
            p.min.x >= win.min.x
                && p.max.x <= win.max.x
                && p.min.y >= win.min.y
                && p.max.y <= win.max.y,
            "inside the window at {at}: {p:?}"
        );
        if flips_x {
            assert!(
                (p.max.x - at.x).abs() <= 1.0,
                "right edge at the pointer {at}: {p:?}"
            );
        } else {
            assert!(
                (p.min.x - at.x).abs() <= 1.0,
                "left edge at the pointer {at}: {p:?}"
            );
        }
        if flips_y {
            assert!(
                (p.max.y - at.y).abs() <= 1.0,
                "bottom edge at the pointer {at}: {p:?}"
            );
        } else {
            assert!(
                (p.min.y - at.y).abs() <= 1.0,
                "top edge at the pointer {at}: {p:?}"
            );
        }
    }
}

#[test]
fn a_press_on_the_region_a_pointer_popover_opened_in_closes_it_and_the_point_goes_with_it() {
    let mut app = app();
    let region = app
        .world_mut()
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        })
        .id();
    settle(&mut app);
    point_at(&mut app, Vec2::new(400.0, 300.0));
    let panel = open(&mut app, region, Placement::AtPointer);
    settle(&mut app);
    assert_eq!(
        app.world_mut()
            .query::<&PopoverPoint>()
            .iter(app.world())
            .count(),
        1
    );
    click_at(&mut app, Vec2::new(900.0, 500.0));
    assert!(gone(&app, panel));
    assert_eq!(closed(&app), vec![(panel, CloseReason::Outside)]);
    assert_eq!(
        app.world_mut()
            .query::<&PopoverPoint>()
            .iter(app.world())
            .count(),
        0,
        "the point is removed with its popover"
    );
}

#[test]
fn a_press_inside_the_panel_closes_nothing_and_never_presses_the_anchor() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(400.0), Val::Px(300.0));
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Below);
    settle(&mut app);
    // The panel's own padding, then its line of text.
    let p = rect(&app, panel);
    click_at(&mut app, p.min + Vec2::new(4.0, 4.0));
    let line = text_node(&mut app, panel, "A line of text that makes the card wide");
    click_on(&mut app, line);
    assert!(!gone(&app, panel), "inside presses close nothing");
    assert!(closed(&app).is_empty());
    assert!(
        app.world().resource::<Activated>().0.is_empty(),
        "the anchor was never activated: {:?}",
        app.world().resource::<Activated>().0
    );
    assert!(app.world().get::<bevy::ui::Pressed>(anchor).is_none());
}

#[test]
fn a_button_in_the_panel_activates_and_the_panel_stays() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(400.0), Val::Px(300.0));
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Below);
    settle(&mut app);
    let copy = child_named(&mut app, panel, "Copy");
    app.world_mut()
        .entity_mut(copy)
        .observe(|_: On<Activate>, mut log: ResMut<Activated>| log.0.push("Copy"));
    click_on(&mut app, copy);
    assert_eq!(app.world().resource::<Activated>().0, vec!["Copy"]);
    assert!(!gone(&app, panel));
}

#[test]
fn a_press_on_the_anchor_of_a_side_popover_is_not_outside_so_its_own_toggle_works() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(400.0), Val::Px(300.0));
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Below);
    settle(&mut app);
    click_on(&mut app, anchor);
    assert!(
        !gone(&app, panel),
        "the anchor's own press leaves it to the game"
    );
    assert!(closed(&app).is_empty());
    assert_eq!(
        app.world().resource::<Activated>().0,
        vec!["Edit"],
        "the anchor still activates"
    );
}

#[test]
fn the_press_that_opens_a_popover_is_not_outside() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(400.0), Val::Px(300.0));
    settle(&mut app);
    point_at(&mut app, Vec2::new(900.0, 600.0));
    // Spawned in the frame the press arrives, as a game's observer of that press would.
    let panel = open(&mut app, anchor, Placement::Below);
    mouse(
        &mut app,
        Vec2::new(900.0, 600.0),
        PointerAction::Press(PointerButton::Primary),
    );
    settle(&mut app);
    assert!(!gone(&app, panel), "the opening press is not outside");
    mouse(
        &mut app,
        Vec2::new(900.0, 600.0),
        PointerAction::Release(PointerButton::Primary),
    );
    app.update();
    click_at(&mut app, Vec2::new(900.0, 600.0));
    assert!(gone(&app, panel), "the next one is");
}

#[test]
fn a_despawned_anchor_takes_its_popover_and_a_resize_places_it_again() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Percent(80.0), Val::Px(300.0));
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Right);
    settle(&mut app);
    for (width, height) in [(1280.0, 720.0), (700.0, 520.0), (1600.0, 900.0)] {
        let mut windows = app
            .world_mut()
            .query_filtered::<&mut Window, With<PrimaryWindow>>();
        windows
            .single_mut(app.world_mut())
            .expect("one primary window")
            .resolution
            .set(width, height);
        settle(&mut app);
        let (p, win) = (rect(&app, panel), window(&app, panel));
        assert!(
            p.min.x >= win.min.x - 0.5 && p.max.x <= win.max.x + 0.5,
            "inside a {width}x{height} window: {p:?} in {win:?}"
        );
    }
    app.world_mut().entity_mut(anchor).despawn();
    settle(&mut app);
    assert!(gone(&app, panel), "the anchor going took the popover");
    assert!(closed(&app).is_empty(), "nothing the player did");
}

#[test]
fn a_press_inside_a_nested_popover_keeps_both_and_one_outside_closes_the_outer_once() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(300.0), Val::Px(200.0));
    settle(&mut app);
    let outer = open(&mut app, anchor, Placement::Below);
    settle(&mut app);
    let more = child_named(&mut app, outer, "Copy");
    let inner = open(&mut app, more, Placement::Right);
    settle(&mut app);
    let line = child_named(&mut app, inner, "Paste");
    click_on(&mut app, line);
    assert!(
        !gone(&app, outer) && !gone(&app, inner),
        "a press inside both"
    );
    // Inside the outer panel, outside the inner one: only the inner closes.
    let o = rect(&app, outer);
    click_at(&mut app, o.min + Vec2::new(4.0, 4.0));
    assert!(!gone(&app, outer));
    assert!(gone(&app, inner));
    assert_eq!(closed(&app), vec![(inner, CloseReason::Outside)]);
    // Outside both.
    let first_inner = inner;
    let inner = open(&mut app, more, Placement::Right);
    settle(&mut app);
    click_at(&mut app, Vec2::new(1000.0, 650.0));
    assert!(gone(&app, outer) && gone(&app, inner));
    assert_eq!(
        closed(&app),
        vec![
            (first_inner, CloseReason::Outside),
            (outer, CloseReason::Outside)
        ],
        "the outer one is reported once; the one under it goes with it"
    );
}

#[test]
fn a_modal_popover_takes_focus_keeps_it_closes_on_escape_once_and_gives_focus_back() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "File", Val::Px(400.0), Val::Px(200.0));
    app.world_mut().entity_mut(anchor).insert(AutoFocus);
    let _other = anchor_at(&mut app, "Other", Val::Px(700.0), Val::Px(200.0));
    settle(&mut app);
    assert_eq!(focus(&app), Some(anchor));
    let panel = open_modal(&mut app, anchor, Placement::Below);
    settle(&mut app);
    let new = child_named(&mut app, panel, "New");
    assert_eq!(focus(&app), Some(new), "focus moved to its first widget");
    for key in [
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowUp,
        KeyCode::ArrowRight,
    ] {
        tap(&mut app, key);
        assert!(
            inside(&app, focus(&app), panel),
            "{key:?} kept focus inside the panel: {:?}",
            focus(&app)
        );
    }
    tap(&mut app, KeyCode::Escape);
    assert!(gone(&app, panel));
    assert_eq!(
        closed(&app),
        vec![(panel, CloseReason::Cancel)],
        "one event"
    );
    assert_eq!(focus(&app), Some(anchor), "focus went back to the anchor");
}

#[test]
fn a_non_modal_popover_leaves_focus_alone_and_closes_on_escape_from_its_anchor() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "File", Val::Px(400.0), Val::Px(200.0));
    app.world_mut().entity_mut(anchor).insert(AutoFocus);
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Below);
    settle(&mut app);
    assert_eq!(focus(&app), Some(anchor), "focus did not move");
    tap(&mut app, KeyCode::Escape);
    assert!(gone(&app, panel));
    assert_eq!(closed(&app), vec![(panel, CloseReason::Cancel)]);
    assert_eq!(focus(&app), Some(anchor));
}

#[test]
fn escape_from_a_widget_the_popover_is_not_anchored_to_closes_nothing() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "File", Val::Px(400.0), Val::Px(200.0));
    let elsewhere = anchor_at(&mut app, "Elsewhere", Val::Px(700.0), Val::Px(200.0));
    app.world_mut().entity_mut(elsewhere).insert(AutoFocus);
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Below);
    settle(&mut app);
    tap(&mut app, KeyCode::Escape);
    assert!(
        !gone(&app, panel),
        "Back is the screen's here, not the popover's"
    );
    assert!(closed(&app).is_empty());
}

#[test]
fn a_popover_is_never_drawn_bare_and_is_a_card_in_the_skins_colours() {
    let rgb = |c: Color| {
        let c = Srgba::from(c);
        [c.red, c.green, c.blue]
    };
    for (name, skin) in [
        ("dark", GooeySkin::dark()),
        ("parchment", GooeySkin::parchment()),
        ("high contrast", GooeySkin::high_contrast()),
    ] {
        let mut app = app_with(skin.clone());
        let anchor = anchor_at(&mut app, "Edit", Val::Px(400.0), Val::Px(300.0));
        settle(&mut app);
        let panel = open(&mut app, anchor, Placement::Below);
        assert_eq!(
            app.world().get::<Node>(panel).expect("a node").display,
            Display::None,
            "{name}: born hidden, so never drawn unstyled"
        );
        settle(&mut app);
        let node = app.world().get::<Node>(panel).expect("a node").clone();
        assert_eq!(node.display, Display::Flex, "{name}: shown once styled");
        assert_eq!(node.padding, UiRect::all(Val::Px(skin.padding)));
        assert_eq!(node.row_gap, Val::Px(skin.gap));
        let fill = app.world().get::<BackgroundColor>(panel).expect("a fill").0;
        assert_eq!(fill, skin.solid_surface(), "{name}: opaque");
        assert_eq!(Srgba::from(fill).alpha, 1.0);
        let edge = app.world().get::<BorderColor>(panel).expect("an edge").top;
        assert_eq!(edge, skin.outline, "{name}: the dialog's edge");
        let ratio = contrast_ratio(rgb(edge), rgb(fill));
        assert!(
            ratio >= NON_TEXT_CONTRAST,
            "{name}: the edge is {ratio:.2}:1 against the card"
        );
        let line = text_node(&mut app, panel, "A line of text that makes the card wide");
        let text = app.world().get::<TextColor>(line).expect("text colour").0;
        let ratio = contrast_ratio(rgb(text), rgb(fill));
        assert!(
            ratio >= TEXT_CONTRAST,
            "{name}: text is {ratio:.2}:1 on the card"
        );
        assert_ne!(
            app.world().get::<GlobalZIndex>(panel).expect("a z").0,
            0,
            "{name}: above the screen"
        );
    }
}

#[test]
fn a_popover_over_a_dialog_stands_above_it_and_a_tooltip_above_both() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(400.0), Val::Px(300.0));
    let dialog_z = app
        .world_mut()
        .spawn(dialog())
        .get::<GlobalZIndex>()
        .expect("a z")
        .0;
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Below);
    let z = app.world().get::<GlobalZIndex>(panel).expect("a z").0;
    assert!(z > dialog_z, "above a dialog: {z} over {dialog_z}");
    assert!(z < 200, "under a tooltip");
}

#[test]
fn an_open_popover_idling_writes_nothing() {
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(400.0), Val::Px(300.0));
    settle(&mut app);
    open(&mut app, anchor, Placement::Below);
    settle(&mut app);
    let before = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<Writes>().0,
        before,
        "thirty idle frames restyled nothing"
    );
}

#[test]
fn a_popover_stays_inside_the_window_at_large_text_and_on_every_side() {
    for scale in [1.0, 1.5, 2.0] {
        for placement in [
            Placement::Below,
            Placement::Above,
            Placement::Right,
            Placement::Left,
        ] {
            for (x, y) in [
                (5.0, 5.0),
                (1100.0, 5.0),
                (5.0, 650.0),
                (1100.0, 650.0),
                (600.0, 340.0),
            ] {
                let mut app = app_with(GooeySkin {
                    text_scale: scale,
                    ..GooeySkin::default()
                });
                let anchor = anchor_at(&mut app, "Edit", Val::Px(x), Val::Px(y));
                // The far corners are the anchor's own far edges, 5 px from the window's.
                {
                    let mut node = app.world_mut().get_mut::<Node>(anchor).expect("a node");
                    if x > 1000.0 {
                        (node.left, node.right) = (Val::Auto, Val::Px(5.0));
                    }
                    if y > 600.0 {
                        (node.top, node.bottom) = (Val::Auto, Val::Px(5.0));
                    }
                }
                settle(&mut app);
                let panel = app
                    .world_mut()
                    .spawn((
                        popover(anchor, placement),
                        children![label("Cut and paste"), button("Copy"), button("Paste")],
                    ))
                    .id();
                settle(&mut app);
                let (p, win) = (rect(&app, panel), window(&app, panel));
                assert!(
                    p.min.x >= win.min.x - 0.5
                        && p.max.x <= win.max.x + 0.5
                        && p.min.y >= win.min.y - 0.5
                        && p.max.y <= win.max.y + 0.5,
                    "{placement:?} from ({x}, {y}) at {scale}x text: {p:?} in {win:?}"
                );
            }
        }
    }
}

#[test]
fn a_popover_is_drawn_only_once_it_stands_at_its_final_size_and_place() {
    // Bevy lays a new node's text out a frame late: a card drawn in its first frame is as narrow
    // as its padding, at the place its width moves it from.
    let mut app = app();
    let anchor = anchor_at(&mut app, "Edit", Val::Px(1150.0), Val::Px(300.0));
    settle(&mut app);
    let panel = open(&mut app, anchor, Placement::Below);
    let mut frames = Vec::new();
    for _ in 0..8 {
        app.update();
        let shown =
            *app.world().get::<Visibility>(panel).expect("visibility") != Visibility::Hidden;
        frames.push((shown, rect(&app, panel)));
    }
    let (_, last) = *frames.last().expect("frames");
    assert!(
        frames.iter().any(|(shown, _)| *shown),
        "it is shown within eight frames"
    );
    for (i, (shown, at)) in frames.iter().enumerate() {
        assert!(
            !shown
                || (at.min - last.min).abs().max_element() <= 0.5
                    && (at.max - last.max).abs().max_element() <= 0.5,
            "frame {i} is drawn at {at:?}, its final place is {last:?}"
        );
    }
}
