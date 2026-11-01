//! The dock: a layout built from data, panels that keep their page when the player rearranges them,
//! and the drag, the key and the events that do it. Laid out for real in a headless window, driven
//! by real pointer and keyboard input.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every layout the dock has reported, in order.
#[derive(Resource, Default)]
struct Reports(Vec<DockLayout>);

/// Every panel the dock left out, with why.
#[derive(Resource, Default)]
struct Skipped(Vec<(String, DockSkip)>);

/// How many frames' writes to a dock's nodes the app has seen.
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
    .init_resource::<Reports>()
    .init_resource::<Skipped>()
    .init_resource::<Writes>()
    .add_observer(|changed: On<DockLayoutChanged>, mut log: ResMut<Reports>| {
        log.0.push(changed.layout.clone());
    })
    .add_observer(|skipped: On<DockPanelSkipped>, mut log: ResMut<Skipped>| {
        log.0.push((skipped.id.clone(), skipped.reason));
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

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
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

/// A modifier key pressed and, after `then`, let go.
fn with_modifiers(app: &mut App, then: impl FnOnce(&mut App)) {
    let window = window(app);
    let mods = [KeyCode::ControlLeft, KeyCode::ShiftLeft];
    for key_code in mods {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
    then(app);
    for key_code in mods {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state: ButtonState::Released,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
}

struct Rig {
    dock: Entity,
    /// How many times each panel's spawn function ran.
    spawned: Arc<AtomicUsize>,
}

/// Registers panels a to e, each a label of its title, and spawns a dock of `layout` in a 1000 by
/// 500 container.
fn rig(app: &mut App, layout: DockLayout) -> Rig {
    rig_in(app, layout, 1000.0)
}

fn rig_in(app: &mut App, layout: DockLayout, width: f32) -> Rig {
    let spawned = Arc::new(AtomicUsize::new(0));
    for (id, title) in [
        ("a", "Alpha"),
        ("b", "Bravo"),
        ("c", "Charlie"),
        ("d", "Delta"),
    ] {
        let count = spawned.clone();
        app.world_mut()
            .resource_mut::<PanelRegistry>()
            .register(id, title, move |page| {
                count.fetch_add(1, Ordering::SeqCst);
                page.with_child(label(title));
            });
    }
    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(width),
                height: Val::Px(500.0),
                ..default()
            },
            children![dock(layout)],
        ))
        .id();
    settle(app);
    let dock = app.world().get::<Children>(container).expect("dock")[0];
    Rig { dock, spawned }
}

const H: SplitAxis = SplitAxis::Horizontal;
const V: SplitAxis = SplitAxis::Vertical;

/// Alpha and Bravo on the left, Charlie on the right.
fn two() -> DockLayout {
    DockLayout::split(
        H,
        0.5,
        DockLayout::group(["a", "b"]),
        DockLayout::group(["c"]),
    )
}

fn tab(app: &mut App, title: &str) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &Text, &Role, &ChildOf)>();
    query
        .iter(app.world())
        .find(|(_, text, role, _)| **role == Role::TabText && text.0 == title)
        .map(|(_, _, _, parent)| parent.parent())
        .unwrap_or_else(|| panic!("no tab titled {title}"))
}

/// The card of the group a tab is in: tab, strip, tabs, card.
fn card(app: &App, tab: Entity) -> Entity {
    let mut at = tab;
    for _ in 0..3 {
        at = app.world().get::<ChildOf>(at).expect("parent").parent();
    }
    assert_eq!(app.world().get::<Role>(at), Some(&Role::Panel));
    at
}

fn card_of(app: &mut App, title: &str) -> Entity {
    let tab = tab(app, title);
    card(app, tab)
}

fn card_rect(app: &mut App, title: &str) -> Rect {
    let card = card_of(app, title);
    rect(app, card)
}

fn tab_rect(app: &mut App, title: &str) -> Rect {
    let tab = tab(app, title);
    rect(app, tab)
}

fn page_shown(app: &mut App, id: &str) -> bool {
    let page = page(app, id);
    shown(app, page)
}

fn page(app: &mut App, id: &str) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &DockPanel)>();
    query
        .iter(app.world())
        .find(|(_, panel)| panel.0 == id)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("no page for {id}"))
}

fn pages(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&DockPanel>();
    query.iter(app.world()).count()
}

fn reports(app: &App) -> Vec<DockLayout> {
    app.world().resource::<Reports>().0.clone()
}

fn layout(app: &App, dock: Entity) -> DockLayout {
    app.world().get::<Dock>(dock).expect("a dock").0.clone()
}

fn shown(app: &App, entity: Entity) -> bool {
    app.world()
        .get::<InheritedVisibility>(entity)
        .is_some_and(|v| v.get())
}

/// Picks `tab` up and carries it to `to` (layout px), without letting go.
fn carry(app: &mut App, tab: Entity, to: Vec2) {
    let from = at(app, rect(app, tab).center());
    grab_and_move(app, from, at(app, to));
}

/// Carries `tab` to `to` and lets go.
fn drop_on(app: &mut App, tab: Entity, to: Vec2) {
    carry(app, tab, to);
    release(app, at(app, to));
}

fn title_of(app: &App, entity: Entity) -> Option<String> {
    app.world().get::<Text>(entity).map(|t| t.0.clone())
}

fn overlay(app: &mut App) -> Option<(Entity, Node)> {
    let mut query = app.world_mut().query::<(Entity, &Role, &Node)>();
    query
        .iter(app.world())
        .find(|(_, role, _)| **role == Role::DockDrop)
        .map(|(entity, _, node)| (entity, node.clone()))
}

fn ghost(app: &mut App) -> Option<(Entity, Node)> {
    let mut query = app.world_mut().query::<(Entity, &Role, &Node)>();
    query
        .iter(app.world())
        .find(|(_, role, _)| **role == Role::DockGhost)
        .map(|(entity, _, node)| (entity, node.clone()))
}

// ---- the layout on screen ----

#[test]
fn a_layout_is_built_from_its_panels_and_shows_the_selected_one() {
    let mut app = app();
    let ui = rig(&mut app, two());
    assert_eq!(
        ui.spawned.load(Ordering::SeqCst),
        3,
        "each panel's function ran once"
    );
    assert_eq!(pages(&mut app), 3);
    for title in ["Alpha", "Bravo", "Charlie"] {
        let tab = tab(&mut app, title);
        assert!(
            rect(&app, tab).height() >= 44.0 - 1.0,
            "a tab is a target high"
        );
    }
    // Alpha is shown in the left group, Bravo hidden behind it, Charlie alone on the right.
    assert!(page_shown(&mut app, "a"));
    assert!(!page_shown(&mut app, "b"));
    assert!(page_shown(&mut app, "c"));
    // The groups split the dock in half, and fill it.
    let whole = rect(&app, ui.dock);
    let (left, right) = (card_rect(&mut app, "Alpha"), card_rect(&mut app, "Charlie"));
    assert!((whole.width() - 1000.0).abs() < 1.0 && (whole.height() - 500.0).abs() < 1.0);
    assert!(left.min.x <= whole.min.x + 1.0 && right.max.x >= whole.max.x - 1.0);
    assert!(
        (left.width() - right.width()).abs() < 14.0,
        "{left:?} {right:?}"
    );
    assert!(left.height() >= whole.height() - 1.0);
}

#[test]
fn the_panel_is_filled_by_the_games_function_and_fills_its_group() {
    let mut app = app();
    let ui = rig(&mut app, DockLayout::group(["a"]));
    let page = page(&mut app, "a");
    let (card, page) = (card_rect(&mut app, "Alpha"), rect(&app, page));
    assert!(
        page.height() > 200.0,
        "the page fills the group: {page:?} in {card:?}"
    );
    assert!(page.max.y <= card.max.y + 1.0);
    let _ = ui;
}

#[test]
fn an_unknown_and_a_repeated_panel_are_left_out_and_reported_once_each() {
    let mut app = app();
    let layout = DockLayout::split(
        H,
        0.5,
        DockLayout::group(["ghost"]),
        DockLayout::group(["a", "a", "c"]),
    );
    let ui = rig(&mut app, layout);
    // The ghost group is gone and its sibling fills the dock.
    assert_eq!(pages(&mut app), 2);
    let skipped = app.world().resource::<Skipped>().0.clone();
    assert_eq!(
        skipped,
        [
            ("ghost".to_owned(), DockSkip::Unknown),
            ("a".to_owned(), DockSkip::Duplicate)
        ]
    );
    let card = card_rect(&mut app, "Alpha");
    assert!(
        card.width() > 950.0,
        "the lone group fills the dock: {card:?}"
    );
    // Nothing more is reported while it stands.
    settle(&mut app);
    assert_eq!(app.world().resource::<Skipped>().0.len(), 2);
    let _ = ui;
}

#[test]
fn a_panel_registered_later_appears_and_one_the_game_drops_goes() {
    let mut app = app();
    let ui = rig(&mut app, DockLayout::group(["a", "late"]));
    assert_eq!(pages(&mut app), 1);
    app.world_mut()
        .resource_mut::<PanelRegistry>()
        .register("late", "Later", |page| {
            page.with_child(label("arrived"));
        });
    settle(&mut app);
    assert_eq!(pages(&mut app), 2);
    let _ = tab(&mut app, "Later");
    // The game writes a smaller layout: the page of the panel it no longer names is gone.
    app.world_mut().get_mut::<Dock>(ui.dock).expect("dock").0 = DockLayout::group(["late"]);
    settle(&mut app);
    assert_eq!(pages(&mut app), 1);
}

#[test]
fn a_layout_the_game_writes_keeps_the_panels_it_already_has() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let before = page(&mut app, "c");
    app.world_mut().get_mut::<Dock>(ui.dock).expect("dock").0 = DockLayout::split(
        V,
        0.3,
        DockLayout::group(["c", "a"]),
        DockLayout::group(["b"]),
    );
    settle(&mut app);
    assert_eq!(
        page(&mut app, "c"),
        before,
        "the page is moved, not rebuilt"
    );
    assert_eq!(ui.spawned.load(Ordering::SeqCst), 3);
    // Charlie is now over Bravo: the groups are stacked.
    let top = card_rect(&mut app, "Charlie");
    let bottom = card_rect(&mut app, "Bravo");
    assert!(top.max.y <= bottom.min.y + 1.0, "{top:?} {bottom:?}");
    assert!(
        app.world().resource::<Reports>().0.is_empty(),
        "the game's write is no report"
    );
}

// ---- dragging a tab ----

#[test]
fn dragging_a_tab_to_the_other_group_moves_it_and_reports_the_layout() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let page_before = page(&mut app, "a");
    let alpha = tab(&mut app, "Alpha");
    let right = card_rect(&mut app, "Charlie");
    drop_on(&mut app, alpha, right.center());
    let moved = DockLayout::split(
        H,
        0.5,
        DockLayout::group(["b"]),
        DockLayout::Group {
            panels: vec!["c".into(), "a".into()],
            selected: 1,
        },
    );
    assert_eq!(
        reports(&app),
        std::slice::from_ref(&moved),
        "one report, with the tab moved"
    );
    assert_eq!(layout(&app, ui.dock), moved);
    // The page is the very entity it was, and shown in its new group.
    assert_eq!(page(&mut app, "a"), page_before);
    assert_eq!(ui.spawned.load(Ordering::SeqCst), 3);
    assert!(shown(&app, page_before));
    assert!(!page_shown(&mut app, "c"));
    // The tab is in the right group now.
    let (alpha, charlie) = (tab(&mut app, "Alpha"), tab(&mut app, "Charlie"));
    assert_eq!(card(&app, alpha), card(&app, charlie));
    assert_eq!(pages(&mut app), 3);
}

#[test]
fn the_last_tab_of_a_group_leaving_collapses_the_group() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let charlie = tab(&mut app, "Charlie");
    let left = card_rect(&mut app, "Alpha");
    drop_on(&mut app, charlie, left.center());
    assert_eq!(
        layout(&app, ui.dock),
        DockLayout::Group {
            panels: vec!["a".into(), "b".into(), "c".into()],
            selected: 2
        }
    );
    let card = card_rect(&mut app, "Alpha");
    assert!(card.width() > 950.0, "the group fills the dock: {card:?}");
}

#[test]
fn dropping_on_a_groups_edge_splits_it_and_on_the_docks_edge_splits_the_whole() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let bravo = tab(&mut app, "Bravo");
    let right = card_rect(&mut app, "Charlie");
    // The bottom quarter of the right group.
    drop_on(
        &mut app,
        bravo,
        Vec2::new(right.center().x, right.max.y - 60.0),
    );
    assert_eq!(
        layout(&app, ui.dock),
        DockLayout::split(
            H,
            0.5,
            DockLayout::group(["a"]),
            DockLayout::split(V, 0.5, DockLayout::group(["c"]), DockLayout::group(["b"])),
        )
    );
    // Now Alpha to the left edge of the whole dock: a quarter of it, as a group of its own.
    let alpha = tab(&mut app, "Alpha");
    let whole = rect(&app, ui.dock);
    // Alpha is alone: its group collapses, and the layout is a split of Charlie over Bravo with
    // Alpha at the left edge.
    drop_on(
        &mut app,
        alpha,
        Vec2::new(whole.min.x + 6.0, whole.center().y),
    );
    let DockLayout::Split {
        axis,
        fraction,
        first,
        ..
    } = layout(&app, ui.dock)
    else {
        panic!("a split")
    };
    assert_eq!((axis, fraction), (H, 0.25));
    assert_eq!(*first, DockLayout::group(["a"]));
    let card = card_rect(&mut app, "Alpha");
    assert!((card.width() - 0.25 * 1000.0).abs() < 14.0, "{card:?}");
}

#[test]
fn dropping_on_the_strip_puts_the_tab_where_the_pointer_is() {
    let mut app = app();
    let ui = rig(&mut app, DockLayout::group(["a", "b", "c"]));
    let alpha = tab(&mut app, "Alpha");
    let charlie = tab_rect(&mut app, "Charlie");
    // Past the last tab's centre: Alpha goes last.
    drop_on(
        &mut app,
        alpha,
        Vec2::new(charlie.center().x + 10.0, charlie.center().y),
    );
    assert_eq!(
        layout(&app, ui.dock),
        DockLayout::Group {
            panels: vec!["b".into(), "c".into(), "a".into()],
            selected: 2
        }
    );
    // And Charlie, grabbed from the end, before the first tab.
    let charlie = tab(&mut app, "Charlie");
    let bravo = tab_rect(&mut app, "Bravo");
    drop_on(
        &mut app,
        charlie,
        Vec2::new(bravo.min.x + 4.0, bravo.center().y),
    );
    assert_eq!(
        layout(&app, ui.dock),
        DockLayout::Group {
            panels: vec!["c".into(), "b".into(), "a".into()],
            selected: 0
        }
    );
}

#[test]
fn a_drop_that_changes_nothing_reports_nothing() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let charlie = tab(&mut app, "Charlie");
    // Charlie alone, dropped on its own group's middle: nowhere to go.
    let own = rect(&app, card(&app, charlie));
    drop_on(&mut app, charlie, own.center());
    assert!(reports(&app).is_empty());
    assert_eq!(layout(&app, ui.dock), two());
    // Dropped off the dock altogether: nothing.
    let alpha = tab(&mut app, "Alpha");
    drop_on(&mut app, alpha, Vec2::new(1200.0, 650.0));
    assert!(reports(&app).is_empty());
    assert_eq!(layout(&app, ui.dock), two());
}

#[test]
fn a_carried_tab_lights_where_it_will_land_and_shows_its_title() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let (overlay_entity, idle) = overlay(&mut app).expect("the landing");
    assert_eq!(idle.display, Display::None);
    assert_eq!(ghost(&mut app).expect("the ghost").1.display, Display::None);
    let alpha = tab(&mut app, "Alpha");
    let right = card_rect(&mut app, "Charlie");
    let dock = rect(&app, ui.dock);
    // Over the right group's left quarter: the left half of that group lights.
    carry(
        &mut app,
        alpha,
        Vec2::new(right.min.x + 20.0, right.center().y),
    );
    let (_, lit) = overlay(&mut app).expect("the landing");
    assert_eq!(lit.display, Display::Flex);
    let (left, width) = match (lit.left, lit.width) {
        (Val::Px(l), Val::Px(w)) => (l, w),
        other => panic!("a pixel rect, not {other:?}"),
    };
    assert!(
        (left - (right.min.x - dock.min.x)).abs() < 1.5,
        "left {left}"
    );
    assert!((width - right.width() / 2.0).abs() < 1.5, "width {width}");
    // It is drawn: framed in the ring, tinted, and the pointer goes through it.
    let world = app.world();
    assert_eq!(
        world.get::<BorderColor>(overlay_entity).map(|b| b.top),
        Some(GooeySkin::dark().focus_ring)
    );
    assert!(
        world
            .get::<BackgroundColor>(overlay_entity)
            .is_some_and(|b| b.0.alpha() < 1.0)
    );
    assert_eq!(
        world.get::<Pickable>(overlay_entity),
        Some(&Pickable::IGNORE)
    );
    let (ghost_entity, ghost_node) = ghost(&mut app).expect("the ghost");
    assert_eq!(ghost_node.display, Display::Flex);
    assert_eq!(title_of(&app, ghost_entity).as_deref(), Some("Alpha"));
    // Over its own group's middle there is nowhere to go: no landing.
    let own = card_rect(&mut app, "Alpha");
    carry(&mut app, alpha, own.center());
    assert_eq!(
        overlay(&mut app).expect("the landing").1.display,
        Display::None
    );
    // Letting go over a landing puts them both away.
    carry(&mut app, alpha, right.center());
    let to = at(&app, right.center());
    release(&mut app, to);
    assert_eq!(
        overlay(&mut app).expect("the landing").1.display,
        Display::None
    );
    assert_eq!(ghost(&mut app).expect("the ghost").1.display, Display::None);
}

#[test]
fn back_puts_a_carried_tab_down_where_it_was() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let alpha = tab(&mut app, "Alpha");
    let right = card_rect(&mut app, "Charlie");
    carry(&mut app, alpha, right.center());
    assert_eq!(
        overlay(&mut app).expect("the landing").1.display,
        Display::Flex
    );
    // The press focused the tab; Escape is Back.
    assert_eq!(focused(&app), Some(alpha));
    tap(&mut app, KeyCode::Escape);
    assert_eq!(
        overlay(&mut app).expect("the landing").1.display,
        Display::None
    );
    let to = at(&app, right.center());
    release(&mut app, to);
    assert!(reports(&app).is_empty());
    assert_eq!(layout(&app, ui.dock), two());
}

// ---- the way that is not a drag ----

#[test]
fn ctrl_shift_and_an_arrow_moves_a_tab_to_the_next_group_and_focus_goes_with_it() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let page_before = page(&mut app, "a");
    let alpha = tab(&mut app, "Alpha");
    focus(&mut app, alpha);
    with_modifiers(&mut app, |app| tap(app, KeyCode::ArrowRight));
    settle(&mut app);
    assert_eq!(
        layout(&app, ui.dock),
        DockLayout::split(
            H,
            0.5,
            DockLayout::group(["b"]),
            DockLayout::Group {
                panels: vec!["c".into(), "a".into()],
                selected: 1
            },
        )
    );
    assert_eq!(reports(&app).len(), 1, "one move, one report");
    assert_eq!(page(&mut app, "a"), page_before);
    // Focus is on the tab of the panel that moved, ring still shown.
    let alpha = tab(&mut app, "Alpha");
    assert_eq!(focused(&app), Some(alpha));
    assert!(app.world().resource::<InputFocusVisible>().0);
    // Left again: back into the first group, as its last tab.
    with_modifiers(&mut app, |app| tap(app, KeyCode::ArrowLeft));
    settle(&mut app);
    assert_eq!(
        layout(&app, ui.dock),
        DockLayout::split(
            H,
            0.5,
            DockLayout::Group {
                panels: vec!["b".into(), "a".into()],
                selected: 1
            },
            DockLayout::group(["c"]),
        )
    );
    let alpha = tab(&mut app, "Alpha");
    assert_eq!(
        focused(&app),
        Some(alpha),
        "focus follows the second move too"
    );
}

#[test]
fn with_no_group_that_way_the_panel_goes_to_the_docks_edge_and_nothing_else_moves_focus() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let charlie = tab(&mut app, "Charlie");
    let alpha = tab(&mut app, "Alpha");
    focus(&mut app, alpha);
    // Nothing above Alpha's group: Ctrl+Shift+Up takes Alpha to the top of the dock.
    with_modifiers(&mut app, |app| tap(app, KeyCode::ArrowUp));
    settle(&mut app);
    assert_eq!(
        layout(&app, ui.dock),
        DockLayout::split(
            V,
            0.25,
            DockLayout::group(["a"]),
            DockLayout::split(H, 0.5, DockLayout::group(["b"]), DockLayout::group(["c"])),
        )
    );
    // The arrows alone still go where they always went: Down from the Alpha tab to its page.
    let alpha = tab(&mut app, "Alpha");
    focus(&mut app, alpha);
    tap(&mut app, KeyCode::ArrowDown);
    settle(&mut app);
    assert_ne!(focused(&app), Some(alpha));
    assert_eq!(reports(&app).len(), 1);
    let _ = charlie;
}

#[test]
fn a_lone_tab_with_nowhere_to_go_stays() {
    let mut app = app();
    let ui = rig(&mut app, DockLayout::group(["a"]));
    let alpha = tab(&mut app, "Alpha");
    focus(&mut app, alpha);
    with_modifiers(&mut app, |app| tap(app, KeyCode::ArrowLeft));
    settle(&mut app);
    assert!(reports(&app).is_empty());
    assert_eq!(focused(&app), Some(alpha), "the tab is still there");
    assert_eq!(layout(&app, ui.dock), DockLayout::group(["a"]));
}

// ---- what the player does to the layout, reported ----

#[test]
fn picking_a_tab_and_letting_go_of_a_divider_are_reported_with_the_layout() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let bravo = tab(&mut app, "Bravo");
    let to = at(&app, rect(&app, bravo).center());
    click(&mut app, to);
    assert_eq!(
        reports(&app),
        [DockLayout::split(
            H,
            0.5,
            DockLayout::Group {
                panels: vec!["a".into(), "b".into()],
                selected: 1
            },
            DockLayout::group(["c"]),
        )]
    );
    assert!(page_shown(&mut app, "b") && !page_shown(&mut app, "a"));
    // A divider dragged 100 px: one report, when it is let go, with the new fraction.
    let mut dividers = app.world_mut().query::<(Entity, &SplitDivider)>();
    let divider = dividers.iter(app.world()).next().expect("a divider").0;
    drag(&mut app, divider, Vec2::new(100.0, 0.0));
    let all = reports(&app);
    assert_eq!(all.len(), 2, "{all:?}");
    let DockLayout::Split { fraction, .. } = &all[1] else {
        panic!("a split")
    };
    assert!(
        (fraction - (0.5 + 100.0 / (1000.0 - 24.0))).abs() < 0.01,
        "{fraction}"
    );
    assert_eq!(layout(&app, ui.dock), all[1]);
    // The structure was not rebuilt for it: the same tab entity is still there.
    assert_eq!(tab(&mut app, "Bravo"), bravo);
}

#[test]
fn a_divider_in_a_dock_never_takes_a_pane_under_its_minimum() {
    let mut app = app();
    let ui = rig(&mut app, two());
    let mut dividers = app.world_mut().query::<(Entity, &SplitDivider)>();
    let divider = dividers.iter(app.world()).next().expect("a divider").0;
    drag(&mut app, divider, Vec2::new(-900.0, 0.0));
    let left = card_rect(&mut app, "Alpha");
    assert!((left.width() - DOCK_PANE_MIN_PX).abs() < 1.5, "{left:?}");
    let _ = ui;
}

#[test]
fn nothing_is_written_while_the_dock_stands() {
    let mut app = app();
    let _ = rig(&mut app, two());
    settle(&mut app);
    let before = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, before);
}

#[test]
fn a_strip_too_narrow_for_its_tabs_wraps_them_and_every_tab_stays_in_the_group() {
    let mut app = app();
    let ui = rig_in(&mut app, DockLayout::group(["a", "b", "c", "d"]), 260.0);
    let card = card_rect(&mut app, "Alpha");
    let mut rows = Vec::new();
    for title in ["Alpha", "Bravo", "Charlie", "Delta"] {
        let tab = tab_rect(&mut app, title);
        assert!(
            tab.min.x >= card.min.x - 0.5 && tab.max.x <= card.max.x + 0.5,
            "{title} {tab:?} is outside {card:?}"
        );
        rows.push(tab.min.y.round());
    }
    rows.dedup();
    assert!(
        rows.len() >= 2,
        "the tabs stand in more than one row: {rows:?}"
    );
    // A tab in the second row is reached by the drop's place: after every tab above it.
    let delta = tab_rect(&mut app, "Delta");
    let alpha = tab(&mut app, "Alpha");
    drop_on(&mut app, alpha, delta.center() + Vec2::new(-4.0, 0.0));
    let DockLayout::Group { panels, .. } = layout(&app, ui.dock) else {
        panic!("a group")
    };
    assert_eq!(panels, ["b", "c", "a", "d"]);
}
