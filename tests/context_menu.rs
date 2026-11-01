//! Context menus as a game sees them, laid out in a headless window and driven by real keyboard
//! and pointer input: a secondary click on a picked world sprite opens its menu at the pointer,
//! Shift+F10 and the Menu key open the focused widget's below it, a second click moves it, one menu
//! serves many things and says which it was opened for, and Escape, an outside press and a choice
//! close it with focus given back.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::sprite::{SpritePickingMode, SpritePickingSettings};
use bevy::ui::UiGlobalTransform;
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

/// Every command chosen, by its label, with the thing its menu was opened for.
#[derive(Resource, Default)]
struct Chosen(Vec<(&'static str, Option<Entity>)>);

/// Every `PopoverClosed` a game's observer heard, with why.
#[derive(Resource, Default)]
struct Closed(Vec<(Entity, CloseReason)>);

/// Every thing a menu was opened for, in order (`On<Add<ContextTarget>>`).
#[derive(Resource, Default)]
struct Opened(Vec<Entity>);

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
    .init_resource::<Writes>()
    .init_resource::<Chosen>()
    .init_resource::<Closed>()
    .init_resource::<Opened>()
    .insert_resource(SpritePickingSettings {
        picking_mode: SpritePickingMode::BoundingBox,
        ..default()
    })
    .add_observer(|closed: On<PopoverClosed>, mut list: ResMut<Closed>| {
        list.0.push((closed.entity, closed.reason))
    })
    .add_observer(
        |add: On<Add<ContextTarget>>, targets: Query<&ContextTarget>, mut list: ResMut<Opened>| {
            if let Ok(target) = targets.get(add.entity) {
                list.0.push(target.0);
            }
        },
    )
    .add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

/// A command that records its label and what its menu was opened for.
fn chose(label: &'static str) -> impl Bundle {
    observe(
        move |activate: On<Activate>,
              parents: Query<&ChildOf>,
              targets: Query<&ContextTarget>,
              mut log: ResMut<Chosen>| {
            let target = ContextTarget::of(activate.entity, &parents, &targets);
            log.0.push((label, target));
        },
    )
}

/// A context menu of three commands.
fn menu_of_three(app: &mut App) -> Entity {
    let menu = app
        .world_mut()
        .spawn(context_menu(children![
            (
                menu_item("Rename").with_shortcut(Shortcut::new(Modifiers::NONE, KeyCode::F2)),
                chose("Rename")
            ),
            (menu_item("Duplicate"), chose("Duplicate")),
            separator(),
            (menu_item("Delete"), chose("Delete")),
        ]))
        .id();
    settle(app);
    menu
}

/// A world sprite (`size` px square, at window offset `from_centre`) that opens `menu`.
fn sprite_with(app: &mut App, menu: Entity, from_centre: Vec2) -> Entity {
    let thing = app
        .world_mut()
        .spawn((
            Sprite::from_color(Color::srgb(0.6, 0.3, 0.3), Vec2::splat(200.0)),
            Transform::from_xyz(from_centre.x, -from_centre.y, 0.0),
            Pickable::default(),
            ContextMenu(menu),
        ))
        .id();
    settle(app);
    thing
}

/// A button at `at` that opens `menu`.
fn button_with(app: &mut App, menu: Entity, at: Vec2) -> Entity {
    let thing = app
        .world_mut()
        .spawn((button("Row"), ContextMenu(menu)))
        .id();
    app.world_mut()
        .entity_mut(thing)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.position_type = PositionType::Absolute;
            node.left = Val::Px(at.x);
            node.top = Val::Px(at.y);
        });
    settle(app);
    thing
}

fn window_entity(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    let window = window_entity(app);
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

fn tap(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Pressed);
    key(app, key_code, ButtonState::Released);
    settle(app);
}

/// Shift+F10, or any key with Shift held.
fn tap_with_shift(app: &mut App, key_code: KeyCode) {
    key(app, KeyCode::ShiftLeft, ButtonState::Pressed);
    key(app, key_code, ButtonState::Pressed);
    key(app, key_code, ButtonState::Released);
    key(app, KeyCode::ShiftLeft, ButtonState::Released);
    settle(app);
}

fn focus(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn focus_on(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    settle(app);
}

fn is_open(app: &App, menu: Entity) -> bool {
    app.world().get::<PopoverPanel>(menu).is_some()
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

/// The `MenuItem` reading `text` in `menu`.
fn item(app: &mut App, text: &str) -> Entity {
    let world = app.world_mut();
    let label = world
        .query::<(Entity, &Text, &Role)>()
        .iter(world)
        .find(|(_, t, r)| t.0 == text && **r == Role::MenuText)
        .map(|(e, ..)| e)
        .unwrap_or_else(|| panic!("no menu item reading {text:?}"));
    app.world()
        .get::<ChildOf>(label)
        .map(ChildOf::parent)
        .expect("a row")
}

fn mouse(app: &mut App, at: Vec2, action: PointerAction) {
    let window = window_entity(app);
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
    for _ in 0..3 {
        app.update();
    }
}

fn click_with(app: &mut App, at: Vec2, button: PointerButton) {
    point_at(app, at);
    for action in [PointerAction::Press(button), PointerAction::Release(button)] {
        mouse(app, at, action);
        app.update();
    }
    settle(app);
}

fn right_click(app: &mut App, at: Vec2) {
    click_with(app, at, PointerButton::Secondary);
}

fn left_click(app: &mut App, at: Vec2) {
    click_with(app, at, PointerButton::Primary);
}

fn chosen(app: &App) -> Vec<(&'static str, Option<Entity>)> {
    app.world().resource::<Chosen>().0.clone()
}

/// The issue's acceptance, half one: a right-click on a picked world entity opens its menu at the
/// pointer.
#[test]
fn a_right_click_on_a_picked_world_sprite_opens_its_menu_at_the_pointer() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    let thing = sprite_with(&mut app, menu, Vec2::new(-300.0, 0.0));
    assert!(!is_open(&app, menu), "closed at rest");
    let at = Vec2::new(300.0, 300.0);
    right_click(&mut app, at);
    assert!(is_open(&app, menu), "the right-click opened it");
    assert_eq!(
        app.world().get::<ContextTarget>(menu),
        Some(&ContextTarget(thing)),
        "it knows what it was opened for"
    );
    let card = rect(&app, menu);
    assert!(
        (card.min - at).abs().max_element() <= 1.0,
        "the card's corner is at the pointer {at}: {card:?}"
    );
    assert_eq!(
        focus(&app),
        Some(item(&mut app, "Rename")),
        "focus is on the first item"
    );
}

/// The issue's acceptance, half two: Shift+F10 opens the focused button's menu.
#[test]
fn shift_f10_opens_the_focused_buttons_menu_below_it() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    let row = button_with(&mut app, menu, Vec2::new(200.0, 200.0));
    focus_on(&mut app, row);
    tap_with_shift(&mut app, KeyCode::F10);
    assert!(is_open(&app, menu), "Shift+F10 opened it");
    assert_eq!(
        app.world().get::<ContextTarget>(menu),
        Some(&ContextTarget(row))
    );
    let (card, anchor) = (rect(&app, menu), rect(&app, row));
    assert!(
        card.min.y >= anchor.max.y && (card.min.x - anchor.min.x).abs() <= 1.0,
        "below the button, starting at its edge: {card:?} {anchor:?}"
    );
    assert_eq!(focus(&app), Some(item(&mut app, "Rename")));
    // Enter chooses it, once, closes the menu and puts focus back on the button.
    tap(&mut app, KeyCode::Enter);
    assert_eq!(chosen(&app), vec![("Rename", Some(row))]);
    assert!(!is_open(&app, menu), "choosing closed it");
    assert_eq!(focus(&app), Some(row), "focus is back where it was");
    assert!(
        app.world().get::<ContextTarget>(menu).is_none(),
        "closed, it is for nothing"
    );
}

#[test]
fn the_menu_key_opens_it_too_and_escape_gives_focus_back() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    let row = button_with(&mut app, menu, Vec2::new(200.0, 200.0));
    focus_on(&mut app, row);
    tap(&mut app, KeyCode::ContextMenu);
    assert!(is_open(&app, menu), "the Menu key opened it");
    tap(&mut app, KeyCode::Escape);
    assert!(!is_open(&app, menu), "Escape closed it");
    assert_eq!(focus(&app), Some(row));
    assert_eq!(
        app.world().resource::<Closed>().0,
        vec![(menu, CloseReason::Cancel)]
    );
    assert!(chosen(&app).is_empty());
}

#[test]
fn the_menu_of_the_nearest_thing_above_the_focus_opens() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    // A row that has the menu, and a button inside it that has focus.
    let row = app
        .world_mut()
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(100.0),
                top: Val::Px(100.0),
                ..default()
            },
            ContextMenu(menu),
        ))
        .id();
    let inner = app.world_mut().spawn((button("Inside"), ChildOf(row))).id();
    settle(&mut app);
    focus_on(&mut app, inner);
    tap_with_shift(&mut app, KeyCode::F10);
    assert!(is_open(&app, menu));
    assert_eq!(
        app.world().get::<ContextTarget>(menu),
        Some(&ContextTarget(row)),
        "the thing that has the menu, not the widget that had focus"
    );
    let (card, anchor) = (rect(&app, menu), rect(&app, inner));
    assert!(card.min.y >= anchor.max.y, "below the focused widget");
}

#[test]
fn a_second_right_click_moves_the_menu_and_one_menu_serves_two_things() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    let a = sprite_with(&mut app, menu, Vec2::new(-300.0, 0.0));
    let b = sprite_with(&mut app, menu, Vec2::new(300.0, 0.0));
    let first = Vec2::new(300.0, 250.0);
    right_click(&mut app, first);
    assert_eq!(
        app.world().get::<ContextTarget>(menu),
        Some(&ContextTarget(a))
    );
    // A second click on the same thing, elsewhere on it.
    let second = Vec2::new(380.0, 400.0);
    right_click(&mut app, second);
    assert!(is_open(&app, menu), "still one open menu");
    let card = rect(&app, menu);
    assert!(
        (card.min - second).abs().max_element() <= 1.0,
        "moved to the second click {second}: {card:?}"
    );
    // A click on the other thing: the same menu, for it.
    right_click(&mut app, Vec2::new(900.0, 300.0));
    assert_eq!(
        app.world().get::<ContextTarget>(menu),
        Some(&ContextTarget(b)),
        "now for the other sprite"
    );
    assert_eq!(
        app.world().resource::<Opened>().0,
        vec![a, a, b],
        "each opening announced what it was for, before it was drawn"
    );
}

#[test]
fn a_right_click_on_the_open_menu_does_nothing() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    sprite_with(&mut app, menu, Vec2::new(-300.0, 0.0));
    right_click(&mut app, Vec2::new(300.0, 250.0));
    let before = rect(&app, menu);
    let on_card = before.center();
    right_click(&mut app, on_card);
    assert!(is_open(&app, menu));
    assert_eq!(rect(&app, menu), before, "not moved");
    assert!(app.world().resource::<Closed>().0.is_empty());
}

#[test]
fn only_a_secondary_click_opens_it() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    sprite_with(&mut app, menu, Vec2::new(-300.0, 0.0));
    left_click(&mut app, Vec2::new(300.0, 250.0));
    click_with(&mut app, Vec2::new(300.0, 250.0), PointerButton::Middle);
    assert!(!is_open(&app, menu));
    // And a secondary click on empty ground opens nothing.
    right_click(&mut app, Vec2::new(1000.0, 650.0));
    assert!(!is_open(&app, menu));
}

#[test]
fn a_right_click_on_a_ui_row_opens_its_menu() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    let row = button_with(&mut app, menu, Vec2::new(200.0, 200.0));
    let at = rect(&app, row).center();
    right_click(&mut app, at);
    assert!(is_open(&app, menu));
    assert_eq!(
        app.world().get::<ContextTarget>(menu),
        Some(&ContextTarget(row))
    );
    // Choosing a command reports the row it was for.
    let delete = item(&mut app, "Delete");
    let at = rect(&app, delete).center();
    left_click(&mut app, at);
    assert_eq!(chosen(&app), vec![("Delete", Some(row))]);
    assert!(!is_open(&app, menu));
}

#[test]
fn a_press_outside_closes_it_once() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    sprite_with(&mut app, menu, Vec2::new(-300.0, 0.0));
    right_click(&mut app, Vec2::new(300.0, 250.0));
    left_click(&mut app, Vec2::new(1100.0, 650.0));
    assert!(!is_open(&app, menu));
    assert_eq!(
        app.world().resource::<Closed>().0,
        vec![(menu, CloseReason::Outside)]
    );
    assert!(chosen(&app).is_empty());
}

#[test]
fn near_the_windows_edge_the_menu_flips_and_stays_inside() {
    for at in [Vec2::new(420.0, 440.0), Vec2::new(430.0, 458.0)] {
        let mut app = app();
        let menu = menu_of_three(&mut app);
        // A sprite that covers the lower right of the window.
        app.world_mut().spawn((
            Sprite::from_color(Color::srgb(0.3, 0.4, 0.6), Vec2::splat(2000.0)),
            Pickable::default(),
            ContextMenu(menu),
        ));
        settle(&mut app);
        let corner = Vec2::new(1270.0, 700.0) + (at - Vec2::new(420.0, 440.0)) * 0.0;
        right_click(&mut app, corner);
        let card = rect(&app, menu);
        assert!(
            card.min.x >= 0.0 && card.max.x <= 1280.0 && card.min.y >= 0.0 && card.max.y <= 720.0,
            "inside the window: {card:?}"
        );
        assert!(
            (card.max.x - corner.x).abs() <= 1.0 && (card.max.y - corner.y).abs() <= 1.0,
            "flipped up and to the left of the pointer {corner}: {card:?}"
        );
    }
}

#[test]
fn shift_f10_never_focuses_a_menu_bar_and_does_nothing_without_a_menu() {
    let mut app = app();
    let bar = app
        .world_mut()
        .spawn(menu_bar(children![menu_title(
            "File",
            children![menu_item("New")]
        )]))
        .id();
    let plain = app.world_mut().spawn(button("Plain")).id();
    settle(&mut app);
    focus_on(&mut app, plain);
    tap_with_shift(&mut app, KeyCode::F10);
    assert_eq!(focus(&app), Some(plain), "Shift+F10 is not F10");
    // Plain F10 still takes the bar.
    tap(&mut app, KeyCode::F10);
    let title = app
        .world()
        .get::<Children>(bar)
        .and_then(|c| c.iter().next())
        .expect("a title");
    assert_eq!(focus(&app), Some(title));
}

#[test]
fn a_menu_the_game_despawned_opens_nothing_and_breaks_nothing() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    let thing = sprite_with(&mut app, menu, Vec2::new(-300.0, 0.0));
    let row = button_with(&mut app, menu, Vec2::new(900.0, 100.0));
    app.world_mut().entity_mut(menu).despawn();
    settle(&mut app);
    right_click(&mut app, Vec2::new(300.0, 300.0));
    focus_on(&mut app, row);
    tap_with_shift(&mut app, KeyCode::F10);
    assert!(app.world().get_entity(thing).is_ok());
    assert!(app.world().resource::<Opened>().0.is_empty());
}

#[test]
fn nothing_is_written_while_it_is_closed_or_open_and_idle() {
    let mut app = app();
    let menu = menu_of_three(&mut app);
    sprite_with(&mut app, menu, Vec2::new(-300.0, 0.0));
    let idle = |app: &mut App| {
        app.world_mut().resource_mut::<Writes>().0 = 0;
        for _ in 0..30 {
            app.update();
        }
        app.world().resource::<Writes>().0
    };
    assert_eq!(idle(&mut app), 0, "closed");
    right_click(&mut app, Vec2::new(300.0, 300.0));
    assert_eq!(idle(&mut app), 0, "open");
}
