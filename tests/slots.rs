//! The slot grid: an inventory of square slots a game fills with items, walked by the arrows, moved
//! by the crate's drag and drop. Laid out for real in a headless window, driven by real keyboard,
//! pointer and gamepad input.

use bevy::camera::NormalizedRenderTarget;
use bevy::ecs::system::RunSystemOnce;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<usize>` a game would see: the grid, the slot and `is_final`.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, usize, bool)>);

/// Every `SlotMoved` a game would see, and whether it applied the swap itself.
#[derive(Resource)]
struct Moves {
    seen: Vec<SlotMoved>,
    applied: Vec<bool>,
    apply: bool,
}

impl Default for Moves {
    fn default() -> Self {
        Self {
            seen: Vec::new(),
            applied: Vec::new(),
            apply: true,
        }
    }
}

/// Every `SlotRefused` a game would see.
#[derive(Resource, Default)]
struct Refusals(Vec<SlotRefused>);

fn app() -> App {
    let mut app = App::new();
    // Assumed owner exception (#76): colours ease on the real clock, so this app reads them landed.
    app.insert_resource(MotionSettings::off());
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
    .insert_resource(UiScaleSettings {
        reference_short_side: 720.0,
        ..default()
    })
    .init_resource::<Changes>()
    .init_resource::<Moves>()
    .init_resource::<Refusals>()
    .add_observer(|refused: On<SlotRefused>, mut seen: ResMut<Refusals>| {
        seen.0.push(refused.clone());
    })
    .add_observer(
        |change: On<ValueChange<usize>>, mut seen: ResMut<Changes>| {
            seen.0.push((change.source, change.value, change.is_final))
        },
    )
    .add_observer(
        |moved: On<SlotMoved>, mut heard: ResMut<Moves>, mut grids: Query<&mut SlotItems>| {
            heard.seen.push(moved.clone());
            if heard.apply {
                let applied = moved.apply(&mut grids);
                heard.applied.push(applied);
            }
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

/// Twelve slots: a sword, a stack of arrows, a potion and a torch, and gaps.
fn backpack() -> Vec<Option<Slot>> {
    let mut items = vec![None; 12];
    items[0] = Some(Slot::new("Sword"));
    items[1] = Some(Slot::new("Arrows").count(24));
    items[5] = Some(Slot::new("Potion"));
    items[7] = Some(Slot::new("Torch").count(1));
    items
}

struct Scene {
    before: Entity,
    grid: Entity,
    after: Entity,
}

/// A button, a grid `columns` wide of `items`, and a button, in a column at the top-left of the
/// window, in a screen (a tab group).
fn scene_of(app: &mut App, columns: usize, items: Vec<Option<Slot>>) -> Scene {
    let world = app.world_mut();
    let before = world.spawn(button("Before")).id();
    let grid = world.spawn(slot_grid(columns, items)).id();
    let after = world.spawn(button("After")).id();
    let root = world
        .spawn(screen())
        .add_children(&[before, grid, after])
        .id();
    world.entity_mut(root).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::FlexStart,
        justify_content: JustifyContent::FlexStart,
        row_gap: Val::Px(20.0),
        padding: UiRect::all(Val::Px(20.0)),
        ..default()
    });
    settle(app);
    Scene {
        before,
        grid,
        after,
    }
}

fn scene(app: &mut App) -> Scene {
    scene_of(app, 4, backpack())
}

/// The slots of a grid, in reading order.
fn cells(app: &App, grid: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(grid)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

fn parts_with(app: &App, cell: Entity, role: Role) -> Vec<Entity> {
    app.world()
        .get::<Children>(cell)
        .map(|c| {
            c.iter()
                .filter(|&e| app.world().get::<Role>(e) == Some(&role))
                .collect()
        })
        .unwrap_or_default()
}

fn text_of(app: &App, cell: Entity, role: Role) -> String {
    let part = parts_with(app, cell, role)
        .first()
        .copied()
        .expect("the part");
    app.world().get::<Text>(part).expect("text").0.clone()
}

fn display_of(app: &App, cell: Entity, role: Role) -> Display {
    let part = parts_with(app, cell, role)
        .first()
        .copied()
        .expect("the part");
    app.world().get::<Node>(part).expect("node").display
}

fn items(app: &mut App, grid: Entity) -> Mut<'_, SlotItems> {
    app.world_mut().get_mut::<SlotItems>(grid).expect("items")
}

fn names_of(items: &[Option<Slot>]) -> Vec<String> {
    items
        .iter()
        .map(|slot| slot.as_ref().map_or(String::new(), |s| s.name.clone()))
        .collect()
}

fn names(app: &App, grid: Entity) -> Vec<String> {
    names_of(&app.world().get::<SlotItems>(grid).expect("items").0)
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

fn centre(app: &App, entity: Entity) -> Vec2 {
    rect(app, entity).center() * app.world().resource::<UiScale>().0
}

fn skin(app: &App) -> GooeySkin {
    app.world().resource::<GooeySkin>().clone()
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn key_event(app: &mut App, key_code: KeyCode, logical_key: Key, state: ButtonState) {
    let window = window(app);
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key,
        state,
        text: None,
        repeat: false,
        window,
    });
    app.update();
}

fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    key_event(
        app,
        key_code,
        Key::Unidentified(NativeKey::Unidentified),
        state,
    );
}

fn tap(app: &mut App, key_code: KeyCode) {
    let logical = match key_code {
        KeyCode::Tab => Key::Tab,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::Enter => Key::Enter,
        KeyCode::Space => Key::Space,
        KeyCode::Escape => Key::Escape,
        KeyCode::ArrowUp => Key::ArrowUp,
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        _ => Key::Unidentified(NativeKey::Unidentified),
    };
    key_event(app, key_code, logical.clone(), ButtonState::Pressed);
    key_event(app, key_code, logical, ButtonState::Released);
    settle(app);
}

/// A chord as a keyboard sends it: the modifier is down a frame before the key.
fn chord(app: &mut App, modifier: KeyCode, key_code: KeyCode) {
    key(app, modifier, ButtonState::Pressed);
    tap(app, key_code);
    key(app, modifier, ButtonState::Released);
}

fn pick_up(app: &mut App) {
    chord(app, KeyCode::AltLeft, KeyCode::KeyM);
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

fn click_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::X });
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
}

/// Presses on `from`, moves to `to` and lets go there.
fn drag_to(app: &mut App, from: Entity, to: Entity) {
    let (from, to) = (centre(app, from), centre(app, to));
    drag_points(app, from, to);
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

/// The same, between two points of the window.
fn drag_points(app: &mut App, from: Vec2, to: Vec2) {
    pointer(app, from, PointerAction::Move { delta: Vec2::X });
    pointer(app, from, PointerAction::Press(PointerButton::Primary));
    pointer(app, to, PointerAction::Move { delta: to - from });
    settle(app);
    pointer(app, to, PointerAction::Release(PointerButton::Primary));
    settle(app);
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
    settle(app);
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

fn changes(app: &App) -> Vec<(Entity, usize, bool)> {
    app.world().resource::<Changes>().0.clone()
}

fn clear_changes(app: &mut App) {
    app.world_mut().resource_mut::<Changes>().0.clear();
}

fn moves(app: &App) -> Vec<SlotMoved> {
    app.world().resource::<Moves>().seen.clone()
}

fn drag_roles(app: &mut App) -> Vec<Role> {
    app.world_mut()
        .query::<&Role>()
        .iter(app.world())
        .filter(|role| matches!(role, Role::DragGhost | Role::DropMark))
        .copied()
        .collect()
}

fn tab_stops(app: &App, grid: Entity) -> Vec<i32> {
    cells(app, grid)
        .iter()
        .map(|&c| app.world().get::<TabIndex>(c).expect("a tab index").0)
        .collect()
}

fn carried(app: &App) -> bool {
    app.world().resource::<DragState>().carried_by_key()
}

// ---- what is drawn ----

#[test]
fn the_grid_is_columns_wide_and_every_slot_a_target_high_square() {
    let mut app = app();
    let ui = scene_of(&mut app, 4, backpack().into_iter().take(10).collect());
    let skin = skin(&app);
    let side = skin.text_size * skin.text_scale * 4.0;
    let slots = cells(&app, ui.grid);
    assert_eq!(slots.len(), 10);
    for &slot in &slots {
        let r = rect(&app, slot);
        assert!(
            (r.width() - side).abs() < 1.0 && (r.height() - side).abs() < 1.0,
            "{r:?} is not {side} square"
        );
        assert!(r.width() >= 44.0, "a thumb-sized target");
    }
    // Four to a row, three rows, the columns lined up.
    let at: Vec<Rect> = slots.iter().map(|&s| rect(&app, s)).collect();
    for row in 0..3 {
        for col in 0..4 {
            let Some(r) = at.get(row * 4 + col) else {
                continue;
            };
            assert!((r.min.y - at[row * 4].min.y).abs() < 1.0, "row {row}");
            assert!((r.min.x - at[col].min.x).abs() < 1.0, "column {col}");
        }
    }
    assert!(at[4].min.y > at[0].max.y && at[8].min.y > at[4].max.y);
    assert!(at[1].min.x > at[0].max.x, "a gap between slots");
    let grid = rect(&app, ui.grid);
    assert!(
        at.iter()
            .all(|r| r.min.x >= grid.min.x - 1.0 && r.max.x <= grid.max.x + 1.0)
    );
}

#[test]
fn a_slot_with_a_picture_draws_it_and_one_without_draws_its_name_and_a_count_shows_above_one() {
    let mut app = app();
    let image = app.world().resource::<AssetServer>().add(Image::default());
    let items_in = vec![
        Some(Slot::new("Sword")),
        Some(Slot::new("Arrows").count(24)),
        Some(Slot::new("Gem").icon(IconSource::Image(image)).count(3)),
        None,
    ];
    let ui = scene_of(&mut app, 4, items_in);
    let slots = cells(&app, ui.grid);

    assert_eq!(text_of(&app, slots[0], Role::SlotName), "Sword");
    assert_eq!(display_of(&app, slots[0], Role::SlotName), Display::Flex);
    assert_eq!(display_of(&app, slots[0], Role::SlotCount), Display::None);
    assert!(parts_with(&app, slots[0], Role::SlotIcon).is_empty());

    assert_eq!(text_of(&app, slots[1], Role::SlotCount), "24");
    assert_eq!(display_of(&app, slots[1], Role::SlotCount), Display::Flex);

    // The picture replaces the name, at 60% of the slot, and the count stays.
    assert_eq!(display_of(&app, slots[2], Role::SlotName), Display::None);
    let icon = parts_with(&app, slots[2], Role::SlotIcon);
    assert_eq!(icon.len(), 1);
    let (picture, slot) = (rect(&app, icon[0]), rect(&app, slots[2]));
    assert!(
        (picture.width() - slot.width() * 0.6).abs() < 1.0,
        "{picture:?} in {slot:?}"
    );
    assert!(
        picture.min.x >= slot.min.x && picture.max.x <= slot.max.x,
        "inside its slot"
    );
    assert_eq!(text_of(&app, slots[2], Role::SlotCount), "3");

    // An empty slot draws nothing.
    assert_eq!(display_of(&app, slots[3], Role::SlotName), Display::None);
    assert_eq!(display_of(&app, slots[3], Role::SlotCount), Display::None);
}

#[test]
fn a_filled_slot_is_raised_and_an_empty_one_is_the_panels_well() {
    let mut app = app();
    let ui = scene(&mut app);
    let skin = skin(&app);
    let slots = cells(&app, ui.grid);
    let fill = |app: &App, e: Entity| app.world().get::<BackgroundColor>(e).expect("fill").0;
    let edge = |app: &App, e: Entity| app.world().get::<BorderColor>(e).expect("edge").top;
    let quiet = skin.surface_edge.unwrap_or(skin.surface_raised);
    assert_eq!(app.world().get::<Role>(slots[0]), Some(&Role::Slot));
    assert_eq!(fill(&app, slots[0]), skin.surface_raised);
    assert_eq!(
        edge(&app, slots[0]),
        skin.surface_raised,
        "a filled slot's frame is its fill, so it is drawn as large as an empty one"
    );
    assert_eq!(app.world().get::<Role>(slots[2]), Some(&Role::SlotEmpty));
    assert_eq!(fill(&app, slots[2]), skin.surface);
    assert_eq!(edge(&app, slots[2]), quiet);

    // The ring is the focused slot's frame, filled or empty.
    focus(&mut app, slots[0]);
    assert_eq!(edge(&app, slots[0]), skin.focus_ring);
    assert_eq!(edge(&app, slots[2]), quiet);
    focus(&mut app, slots[2]);
    assert_eq!(edge(&app, slots[2]), skin.focus_ring);
    assert_eq!(
        edge(&app, slots[0]),
        skin.surface_raised,
        "the ring leaves with focus"
    );

    // The count's chip is the opaque surface, whatever is under it, and its words are the text's.
    let chip = parts_with(&app, slots[1], Role::SlotCount)[0];
    assert_eq!(fill(&app, chip), skin.solid_surface());
    assert_eq!(
        app.world().get::<TextColor>(chip).expect("ink").0,
        skin.text
    );
}

#[test]
fn the_name_and_the_count_are_readable_in_every_look() {
    for preset in SkinPreset::ALL {
        let mut app = app();
        app.insert_resource(preset.skin());
        let ui = scene(&mut app);
        let slots = cells(&app, ui.grid);
        let name = parts_with(&app, slots[0], Role::SlotName)[0];
        let ink = app.world().get::<TextColor>(name).expect("ink").0;
        let fill = app
            .world()
            .get::<BackgroundColor>(slots[0])
            .expect("fill")
            .0;
        let ratio = contrast_ratio(rgb(ink), rgb(fill));
        assert!(ratio >= TEXT_CONTRAST, "{} name {ratio}", preset.name());
        let chip = parts_with(&app, slots[1], Role::SlotCount)[0];
        let ink = app.world().get::<TextColor>(chip).expect("ink").0;
        let fill = app.world().get::<BackgroundColor>(chip).expect("fill").0;
        let ratio = contrast_ratio(rgb(ink), rgb(fill));
        assert!(ratio >= TEXT_CONTRAST, "{} count {ratio}", preset.name());
    }
}

#[test]
fn at_every_text_size_every_slot_is_square_and_in_its_grid() {
    for step in TEXT_SIZE_STEPS {
        let mut app = app();
        app.insert_resource(GooeySkin {
            text_scale: step,
            ..GooeySkin::default()
        });
        let ui = scene(&mut app);
        let skin = skin(&app);
        let side = skin.text_size * step * 4.0;
        let grid = rect(&app, ui.grid);
        for slot in cells(&app, ui.grid) {
            let r = rect(&app, slot);
            assert!(
                (r.width() - side).abs() < 1.0 && (r.height() - side).abs() < 1.0,
                "{step}"
            );
            assert!(
                r.max.x <= grid.max.x + 1.0 && r.max.y <= grid.max.y + 1.0,
                "{step}"
            );
        }
        // The name wraps inside its slot and never leaves it.
        let slots = cells(&app, ui.grid);
        let name = parts_with(&app, slots[5], Role::SlotName)[0];
        let (n, s) = (rect(&app, name), rect(&app, slots[5]));
        assert!(
            n.max.x <= s.max.x + 1.0 && n.min.x >= s.min.x - 1.0,
            "{step}: {n:?} in {s:?}"
        );
    }
}

// ---- the contents ----

#[test]
fn the_slots_follow_the_items_as_they_grow_shrink_and_change() {
    let mut app = app();
    let ui = scene(&mut app);
    let before = cells(&app, ui.grid);
    assert_eq!(before.len(), 12);

    // More items: new slots, the old ones the same entities.
    items(&mut app, ui.grid).0.push(Some(Slot::new("Map")));
    settle(&mut app);
    let grown = cells(&app, ui.grid);
    assert_eq!(grown.len(), 13);
    assert_eq!(&grown[..12], &before[..]);
    assert_eq!(text_of(&app, grown[12], Role::SlotName), "Map");
    assert!(
        rect(&app, grown[12]).min.y > rect(&app, grown[0]).max.y,
        "a fourth row"
    );

    // One changed: its words, its count and its look, in the same frame; the others untouched.
    items(&mut app, ui.grid).0[2] = Some(Slot::new("Bread").count(5));
    settle(&mut app);
    assert_eq!(cells(&app, ui.grid), grown);
    assert_eq!(text_of(&app, grown[2], Role::SlotName), "Bread");
    assert_eq!(text_of(&app, grown[2], Role::SlotCount), "5");
    assert_eq!(display_of(&app, grown[2], Role::SlotCount), Display::Flex);
    assert_eq!(app.world().get::<Role>(grown[2]), Some(&Role::Slot));
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(grown[2])
            .expect("fill")
            .0,
        skin(&app).surface_raised
    );

    // And emptied: the count and the name go and the well is empty.
    items(&mut app, ui.grid).0[1] = None;
    settle(&mut app);
    assert_eq!(app.world().get::<Role>(grown[1]), Some(&Role::SlotEmpty));
    assert_eq!(display_of(&app, grown[1], Role::SlotCount), Display::None);
    assert_eq!(text_of(&app, grown[1], Role::SlotName), "");

    // Fewer items: the extra slots are gone.
    items(&mut app, ui.grid).0.truncate(6);
    settle(&mut app);
    assert_eq!(cells(&app, ui.grid), grown[..6].to_vec());
    assert!(app.world().get_entity(grown[12]).is_err());
}

#[test]
fn an_icon_slot_is_named_by_its_tooltip_and_a_slot_with_words_has_none() {
    let mut app = app();
    let image = app.world().resource::<AssetServer>().add(Image::default());
    let icon = IconSource::Image(image);
    let ui = scene_of(
        &mut app,
        4,
        vec![
            Some(Slot::new("Sword")),
            Some(Slot::new("Gem").icon(icon.clone())),
            Some(
                Slot::new("Key")
                    .icon(icon.clone())
                    .tooltip("Opens the vault"),
            ),
            None,
        ],
    );
    let slots = cells(&app, ui.grid);
    let tip = |app: &App, slot: Entity| app.world().get::<Tooltip>(slot).map(|t| t.text.clone());
    assert_eq!(tip(&app, slots[0]), None);
    assert_eq!(tip(&app, slots[1]).as_deref(), Some("Gem"));
    assert_eq!(tip(&app, slots[2]).as_deref(), Some("Opens the vault"));
    assert_eq!(tip(&app, slots[3]), None);

    // Changing the item changes the line, and emptying the slot takes it away.
    items(&mut app, ui.grid).0[1] = Some(Slot::new("Ruby").icon(icon));
    settle(&mut app);
    assert_eq!(tip(&app, slots[1]).as_deref(), Some("Ruby"));
    items(&mut app, ui.grid).0[2] = None;
    settle(&mut app);
    assert_eq!(tip(&app, slots[2]), None);
    let boxes = app
        .world_mut()
        .query::<&TooltipBox>()
        .iter(app.world())
        .count();
    assert_eq!(boxes, 1, "one box for the one tooltip left");
}

#[test]
fn an_idle_grid_writes_nothing() {
    let mut app = app();
    let ui = scene(&mut app);
    let fifth = cells(&app, ui.grid)[5];
    focus(&mut app, fifth);
    settle(&mut app);
    app.world_mut().clear_trackers();
    for _ in 0..30 {
        app.update();
    }
    let world = app.world();
    for slot in cells(&app, ui.grid) {
        let e = world.entity(slot);
        assert!(!e.get_ref::<Node>().expect("n").is_changed());
        assert!(!e.get_ref::<BackgroundColor>().expect("b").is_changed());
        assert!(!e.get_ref::<TabIndex>().expect("t").is_changed());
        assert!(!e.get_ref::<Role>().expect("r").is_changed());
    }
    assert!(
        !world
            .entity(ui.grid)
            .get_ref::<SlotCursor>()
            .expect("c")
            .is_changed()
    );
}

// ---- focus ----

#[test]
fn the_grid_is_one_tab_stop_entered_on_the_cursor() {
    let mut app = app();
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);
    let mut first = vec![-1; 12];
    first[0] = 0;
    assert_eq!(tab_stops(&app, ui.grid), first);

    focus(&mut app, ui.before);
    tap(&mut app, KeyCode::Tab);
    assert_eq!(focused(&app), Some(slots[0]), "Tab enters on the cursor");
    tap(&mut app, KeyCode::Tab);
    assert_eq!(
        focused(&app),
        Some(ui.after),
        "and leaves on the next press"
    );

    // Walk to slot 6; the cursor, and the one tab stop, goes with focus.
    focus(&mut app, slots[6]);
    assert_eq!(tab_stops(&app, ui.grid)[6], 0);
    assert_eq!(
        tab_stops(&app, ui.grid).iter().filter(|&&t| t == 0).count(),
        1
    );
    assert_eq!(app.world().get::<SlotCursor>(ui.grid), Some(&SlotCursor(6)));
    focus(&mut app, ui.before);
    tap(&mut app, KeyCode::Tab);
    assert_eq!(
        focused(&app),
        Some(slots[6]),
        "Tab returns to the slot the player left"
    );

    // Shift+Tab from after the grid lands on it too.
    focus(&mut app, ui.after);
    key(&mut app, KeyCode::ShiftLeft, ButtonState::Pressed);
    tap(&mut app, KeyCode::Tab);
    key(&mut app, KeyCode::ShiftLeft, ButtonState::Released);
    assert_eq!(focused(&app), Some(slots[6]));
}

#[test]
fn the_arrows_walk_the_grid_and_leave_it_from_its_edges() {
    let mut app = app();
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);
    focus(&mut app, slots[5]);
    for (arrow, to) in [
        (KeyCode::ArrowRight, 6),
        (KeyCode::ArrowLeft, 5),
        (KeyCode::ArrowDown, 9),
        (KeyCode::ArrowUp, 5),
        (KeyCode::ArrowUp, 1),
    ] {
        tap(&mut app, arrow);
        assert_eq!(focused(&app), Some(slots[to]), "{arrow:?}");
    }
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(
        focused(&app),
        Some(ui.before),
        "Up from the first row leaves"
    );
    focus(&mut app, slots[10]);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(
        focused(&app),
        Some(ui.after),
        "Down from the last row leaves"
    );
}

#[test]
fn the_dpad_walks_the_grid() {
    let mut app = app();
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, slots[5]);
    pad_press(&mut app, pad, GamepadButton::DPadRight);
    assert_eq!(focused(&app), Some(slots[6]));
    pad_press(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(focused(&app), Some(slots[10]));
    pad_press(&mut app, pad, GamepadButton::DPadLeft);
    assert_eq!(focused(&app), Some(slots[9]));
}

#[test]
fn home_and_end_go_to_the_ends_of_the_row_and_with_ctrl_of_the_grid() {
    let mut app = app();
    let ui = scene_of(&mut app, 4, backpack().into_iter().take(10).collect());
    let slots = cells(&app, ui.grid);
    focus(&mut app, slots[5]);
    tap(&mut app, KeyCode::Home);
    assert_eq!(focused(&app), Some(slots[4]));
    tap(&mut app, KeyCode::End);
    assert_eq!(focused(&app), Some(slots[7]));
    // The last row is short: End stops at the last slot.
    focus(&mut app, slots[8]);
    tap(&mut app, KeyCode::End);
    assert_eq!(focused(&app), Some(slots[9]));
    chord(&mut app, KeyCode::ControlLeft, KeyCode::Home);
    assert_eq!(focused(&app), Some(slots[0]));
    chord(&mut app, KeyCode::ControlLeft, KeyCode::End);
    assert_eq!(focused(&app), Some(slots[9]));
    assert_eq!(app.world().get::<SlotCursor>(ui.grid), Some(&SlotCursor(9)));
}

#[test]
fn the_cursor_is_the_grids_state_a_move_reports_once_and_a_game_write_reports_nothing() {
    let mut app = app();
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);
    focus(&mut app, slots[5]);
    assert_eq!(changes(&app), vec![(ui.grid, 5, false)]);
    clear_changes(&mut app);

    // The game moves the stop: the tab index follows, no event, and focus stays where it was.
    app.world_mut().get_mut::<SlotCursor>(ui.grid).expect("c").0 = 2;
    settle(&mut app);
    assert!(changes(&app).is_empty(), "{:?}", changes(&app));
    assert_eq!(tab_stops(&app, ui.grid)[2], 0);
    assert_eq!(tab_stops(&app, ui.grid)[0], -1);

    // A value past the end is pulled back to the last slot.
    app.world_mut().get_mut::<SlotCursor>(ui.grid).expect("c").0 = 99;
    settle(&mut app);
    assert_eq!(
        app.world().get::<SlotCursor>(ui.grid),
        Some(&SlotCursor(11))
    );

    // Shrinking the grid pulls it in too.
    items(&mut app, ui.grid).0.truncate(8);
    settle(&mut app);
    assert_eq!(app.world().get::<SlotCursor>(ui.grid), Some(&SlotCursor(7)));
    assert_eq!(tab_stops(&app, ui.grid)[7], 0);
}

// ---- choosing ----

#[test]
fn enter_space_south_and_a_click_each_choose_a_slot_once_empty_or_not() {
    let mut app = app();
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);
    let pad = plug_in_pad(&mut app);

    focus(&mut app, slots[0]);
    clear_changes(&mut app);
    tap(&mut app, KeyCode::Enter);
    assert_eq!(changes(&app), vec![(ui.grid, 0, true)]);

    clear_changes(&mut app);
    tap(&mut app, KeyCode::Space);
    assert_eq!(changes(&app), vec![(ui.grid, 0, true)]);

    clear_changes(&mut app);
    pad_press(&mut app, pad, GamepadButton::South);
    assert_eq!(changes(&app), vec![(ui.grid, 0, true)]);

    // An empty slot is chosen too: it may be where the item goes.
    focus(&mut app, slots[2]);
    clear_changes(&mut app);
    tap(&mut app, KeyCode::Enter);
    assert_eq!(changes(&app), vec![(ui.grid, 2, true)]);

    // A click focuses the slot (the cursor moves, not final) and chooses it (final), once each.
    clear_changes(&mut app);
    let at = centre(&app, slots[7]);
    click_at(&mut app, at);
    let seen = changes(&app);
    assert_eq!(
        seen.iter().filter(|c| c.2).collect::<Vec<_>>(),
        [&(ui.grid, 7, true)],
        "a click is one choice: {seen:?}"
    );
    assert_eq!(app.world().get::<SlotCursor>(ui.grid), Some(&SlotCursor(7)));
}

// ---- moving ----

#[test]
fn dragging_an_item_onto_another_slot_is_one_move_and_swaps_the_two() {
    let mut app = app();
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);
    let (from, to) = (centre(&app, slots[0]), centre(&app, slots[6]));

    drag_points(&mut app, from, to);

    assert_eq!(
        moves(&app),
        vec![SlotMoved {
            entity: ui.grid,
            from: SlotRef {
                grid: ui.grid,
                index: 0
            },
            to: 6
        }]
    );
    assert_eq!(app.world().resource::<Moves>().applied, [true]);
    assert_eq!(names(&app, ui.grid)[0], "");
    assert_eq!(names(&app, ui.grid)[6], "Sword");
    // The screen follows the data in the same frame, and the emptied slot is a well again.
    assert_eq!(text_of(&app, slots[6], Role::SlotName), "Sword");
    assert_eq!(app.world().get::<Role>(slots[0]), Some(&Role::SlotEmpty));
    assert_eq!(app.world().get::<Role>(slots[6]), Some(&Role::Slot));
    assert!(
        drag_roles(&mut app).is_empty(),
        "the ghost and the mark are gone"
    );

    // Onto a filled slot it is a swap.
    drag_to(&mut app, slots[6], slots[1]);
    assert_eq!(names(&app, ui.grid)[1], "Sword");
    assert_eq!(names(&app, ui.grid)[6], "Arrows");
    assert_eq!(text_of(&app, slots[6], Role::SlotCount), "24");
}

#[test]
fn a_drop_on_the_slot_it_came_from_and_a_drag_from_an_empty_slot_move_nothing() {
    let mut app = app();
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);

    // Picked up and put back where it was.
    let sword = centre(&app, slots[0]);
    drag_points(&mut app, sword, sword + Vec2::new(8.0, 8.0));
    assert!(moves(&app).is_empty());

    // An empty slot is no source: nothing is lifted, nothing is dropped.
    let empty = centre(&app, slots[2]);
    let onto = centre(&app, slots[6]);
    pointer(&mut app, empty, PointerAction::Move { delta: Vec2::X });
    pointer(
        &mut app,
        empty,
        PointerAction::Press(PointerButton::Primary),
    );
    pointer(
        &mut app,
        onto,
        PointerAction::Move {
            delta: onto - empty,
        },
    );
    settle(&mut app);
    assert!(
        drag_roles(&mut app).is_empty(),
        "no ghost for an empty slot"
    );
    pointer(
        &mut app,
        onto,
        PointerAction::Release(PointerButton::Primary),
    );
    settle(&mut app);
    assert!(moves(&app).is_empty());
    assert_eq!(names(&app, ui.grid), names_of(&backpack()));
}

#[test]
fn an_emptied_slot_is_not_a_source_and_a_filled_one_is() {
    let mut app = app();
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);
    let source = |app: &App, slot: Entity| app.world().get::<DragPayload<SlotRef>>(slot).is_some();
    assert!(source(&app, slots[0]) && !source(&app, slots[2]));
    items(&mut app, ui.grid).0[0] = None;
    items(&mut app, ui.grid).0[2] = Some(Slot::new("Rope"));
    settle(&mut app);
    assert!(!source(&app, slots[0]) && source(&app, slots[2]));
    // Alt+M on the emptied slot lifts nothing.
    focus(&mut app, slots[0]);
    pick_up(&mut app);
    assert!(!carried(&app));
    focus(&mut app, slots[2]);
    pick_up(&mut app);
    assert!(carried(&app), "and on the filled one it does");
}

#[test]
fn an_item_dragged_to_another_grid_is_one_move_there_and_swaps_across() {
    let mut app = app();
    let ui = scene(&mut app);
    let chest = app
        .world_mut()
        .spawn(slot_grid(
            3,
            vec![Some(Slot::new("Gold").count(90)), None, None],
        ))
        .id();
    let root = app.world().get::<ChildOf>(ui.grid).expect("root").parent();
    app.world_mut().entity_mut(root).add_child(chest);
    settle(&mut app);
    let mine = cells(&app, ui.grid);
    let theirs = cells(&app, chest);

    drag_to(&mut app, mine[0], theirs[1]);

    let seen = moves(&app);
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].entity, chest,
        "the event is on the grid it landed in"
    );
    assert_eq!(
        seen[0].from,
        SlotRef {
            grid: ui.grid,
            index: 0
        }
    );
    assert_eq!(seen[0].to, 1);
    assert_eq!(names(&app, ui.grid)[0], "", "the empty slot came across");
    assert_eq!(names(&app, chest), ["Gold", "Sword", ""]);
    assert_eq!(text_of(&app, theirs[1], Role::SlotName), "Sword");
}

#[test]
fn a_game_with_rules_can_refuse_by_not_applying() {
    let mut app = app();
    app.world_mut().resource_mut::<Moves>().apply = false;
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);
    let before = names(&app, ui.grid);
    drag_to(&mut app, slots[0], slots[6]);
    assert_eq!(moves(&app).len(), 1, "the game is told");
    assert_eq!(
        names(&app, ui.grid),
        before,
        "and nothing moved until it says so"
    );
}

#[test]
fn apply_changes_nothing_for_a_slot_that_is_not_there() {
    let mut app = app();
    let ui = scene(&mut app);
    let before = names(&app, ui.grid);
    let bad = |from: usize, to: usize| SlotMoved {
        entity: ui.grid,
        from: SlotRef {
            grid: ui.grid,
            index: from,
        },
        to,
    };
    let mut results = Vec::new();
    for moved in [bad(0, 99), bad(99, 0), bad(3, 3), bad(0, 1)] {
        let applied = app
            .world_mut()
            .run_system_once(move |mut grids: Query<&mut SlotItems>| moved.apply(&mut grids))
            .expect("ran");
        results.push(applied);
    }
    assert_eq!(results, [false, false, false, true]);
    let after = names(&app, ui.grid);
    assert_eq!((after[0].as_str(), after[1].as_str()), ("Arrows", "Sword"));
    assert_eq!(&after[2..], &before[2..]);
}

#[test]
fn the_keyboard_lifts_steers_and_drops_an_item_with_no_pointer() {
    let mut app = app();
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);
    focus(&mut app, slots[0]);

    pick_up(&mut app);
    assert!(carried(&app));
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(
        app.world().resource::<DragState>().over(),
        Some(slots[4]),
        "the mark is on the slot below"
    );
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(app.world().resource::<DragState>().over(), Some(slots[5]));
    pick_up(&mut app);

    let seen = moves(&app);
    assert_eq!(seen.len(), 1);
    assert_eq!((seen[0].from.index, seen[0].to), (0, 5));
    assert_eq!(
        names(&app, ui.grid)[0],
        "Potion",
        "swapped with the potion there"
    );
    assert_eq!(names(&app, ui.grid)[5], "Sword");
    assert!(!carried(&app));
}

#[test]
fn escape_puts_a_carried_item_back() {
    let mut app = app();
    let ui = scene(&mut app);
    let slots = cells(&app, ui.grid);
    focus(&mut app, slots[0]);
    pick_up(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    tap(&mut app, KeyCode::Escape);
    assert!(moves(&app).is_empty());
    assert_eq!(names(&app, ui.grid)[0], "Sword");
    assert!(!carried(&app));
}

// ---- rules: a slot that says no before the player lets go ----

fn helmet_only(item: &Slot, slot: usize) -> bool {
    slot != 2 || item.kind.as_deref() == Some("helmet")
}

/// A sword, a helm and a potion; slot 2 takes a helmet and nothing else, slot 3 is empty.
fn kit() -> Vec<Option<Slot>> {
    vec![
        Some(Slot::new("Sword").kind("weapon")),
        Some(Slot::new("Helm").kind("helmet")),
        None,
        None,
        Some(Slot::new("Potion").kind("potion")),
        None,
    ]
}

fn ruled(app: &mut App) -> Scene {
    let ui = scene_of(app, 3, kit());
    app.world_mut()
        .entity_mut(ui.grid)
        .insert(SlotRules::new(helmet_only));
    settle(app);
    ui
}

fn refusals(app: &App) -> Vec<SlotRefused> {
    app.world().resource::<Refusals>().0.clone()
}

fn refused_roles(app: &mut App) -> Vec<Role> {
    app.world_mut()
        .query::<&Role>()
        .iter(app.world())
        .filter(|role| matches!(role, Role::DragGhostRefused | Role::DropMarkRefused))
        .copied()
        .collect()
}

/// Presses on `from`, moves to `to` and stops there, the item still in the hand.
fn hold_over(app: &mut App, from: Entity, to: Entity) -> Vec2 {
    let (from, to) = (centre(app, from), centre(app, to));
    pointer(app, from, PointerAction::Move { delta: Vec2::X });
    pointer(app, from, PointerAction::Press(PointerButton::Primary));
    pointer(app, to, PointerAction::Move { delta: to - from });
    settle(app);
    to
}

#[test]
fn a_slot_that_will_not_take_the_item_shows_the_refusal_while_it_is_held_over_it() {
    let mut app = app();
    let ui = ruled(&mut app);
    let slots = cells(&app, ui.grid);

    // The sword over the helmet slot: red while still in the hand.
    let at = hold_over(&mut app, slots[0], slots[2]);
    assert!(app.world().resource::<DragState>().refused());
    let roles = refused_roles(&mut app);
    assert!(roles.contains(&Role::DropMarkRefused), "{roles:?}");
    assert!(roles.contains(&Role::DragGhostRefused), "{roles:?}");
    // Moving on to a slot that takes it turns the look back.
    let ok = centre(&app, slots[3]);
    pointer(&mut app, ok, PointerAction::Move { delta: ok - at });
    settle(&mut app);
    assert!(!app.world().resource::<DragState>().refused());
    assert!(refused_roles(&mut app).is_empty());

    // Back over the helmet slot and let go: one refusal, nothing moved.
    pointer(&mut app, at, PointerAction::Move { delta: at - ok });
    settle(&mut app);
    pointer(&mut app, at, PointerAction::Release(PointerButton::Primary));
    settle(&mut app);
    assert!(moves(&app).is_empty());
    assert_eq!(
        refusals(&app),
        vec![SlotRefused {
            entity: ui.grid,
            from: SlotRef {
                grid: ui.grid,
                index: 0
            },
            to: 2
        }]
    );
    assert_eq!(&names(&app, ui.grid)[..3], ["Sword", "Helm", ""]);
    assert!(drag_roles(&mut app).is_empty() && refused_roles(&mut app).is_empty());
}

#[test]
fn a_slot_that_takes_the_item_gets_the_move_and_focus_follows_it() {
    let mut app = app();
    let ui = ruled(&mut app);
    let slots = cells(&app, ui.grid);

    drag_to(&mut app, slots[1], slots[2]);

    assert_eq!(moves(&app).len(), 1);
    assert!(refusals(&app).is_empty());
    assert_eq!(&names(&app, ui.grid)[1..3], ["", "Helm"]);
    assert_eq!(
        focused(&app),
        Some(slots[2]),
        "the press focused the helm's slot, and focus went with the helm"
    );
    assert_eq!(app.world().get::<SlotCursor>(ui.grid), Some(&SlotCursor(2)));
    assert_eq!(tab_stops(&app, ui.grid), [-1, -1, 0, -1, -1, -1]);
}

#[test]
fn a_swap_is_refused_when_the_item_going_back_is_not_taken_at_the_origin() {
    let mut app = app();
    let ui = ruled(&mut app);
    let slots = cells(&app, ui.grid);
    drag_to(&mut app, slots[1], slots[2]);
    clear_changes(&mut app);
    app.world_mut().resource_mut::<Moves>().seen.clear();
    app.world_mut().resource_mut::<Refusals>().0.clear();

    // The helm over the sword: the helm would go to slot 0 (it takes anything) and the sword to
    // slot 2 (a helmet only), so a swap is no.
    drag_to(&mut app, slots[2], slots[0]);
    assert!(moves(&app).is_empty());
    assert_eq!(refusals(&app).len(), 1);
    assert_eq!(&names(&app, ui.grid)[..3], ["Sword", "", "Helm"]);
    // The helm to an empty slot is a move.
    drag_to(&mut app, slots[2], slots[5]);
    assert_eq!(moves(&app).len(), 1);
    assert_eq!(names(&app, ui.grid)[5], "Helm");
}

#[test]
fn both_grids_rules_apply_to_a_drag_between_them() {
    let mut app = app();
    let ui = ruled(&mut app);
    let chest = app
        .world_mut()
        .spawn(slot_grid(
            3,
            vec![None, Some(Slot::new("Helm 2").kind("helmet")), None],
        ))
        .id();
    let root = app.world().get::<ChildOf>(ui.grid).expect("root").parent();
    app.world_mut().entity_mut(root).add_child(chest);
    settle(&mut app);
    let mine = cells(&app, ui.grid);
    let theirs = cells(&app, chest);

    // The sword to the chest (takes anything): fine. A chest helm onto the sword's old place
    // (the chest's rules are open, mine take it at 0): fine too.
    drag_to(&mut app, mine[0], theirs[0]);
    assert_eq!(moves(&app).len(), 1);
    // The chest's helm onto my filled slot 4 (a potion): the potion would go to the chest's slot
    // 1, which takes anything, so it is fine; onto my slot 2 (helmet only) with a sword it is not.
    drag_to(&mut app, theirs[0], mine[2]);
    assert_eq!(refusals(&app).len(), 1, "a sword is not a helmet");
    assert_eq!(names(&app, chest)[0], "Sword");
    drag_to(&mut app, theirs[1], mine[2]);
    assert_eq!(moves(&app).len(), 2);
    assert_eq!(names(&app, ui.grid)[2], "Helm 2");
}

#[test]
fn a_locked_slot_takes_nothing_and_gives_its_item_up_to_an_open_slot() {
    let mut app = app();
    let ui = scene_of(&mut app, 3, kit());
    app.world_mut()
        .entity_mut(ui.grid)
        .insert(SlotRules::new(|_, slot| slot != 3));
    settle(&mut app);
    let slots = cells(&app, ui.grid);
    drag_to(&mut app, slots[0], slots[3]);
    assert!(moves(&app).is_empty());
    assert_eq!(refusals(&app).len(), 1);
    // A rule written later governs the next drag.
    app.world_mut()
        .entity_mut(ui.grid)
        .insert(SlotRules::default());
    settle(&mut app);
    drag_to(&mut app, slots[0], slots[3]);
    assert_eq!(moves(&app).len(), 1);
    assert_eq!(names(&app, ui.grid)[3], "Sword");
    // And a rule that takes nothing never stops the item leaving its own slot for an open one.
    app.world_mut()
        .entity_mut(ui.grid)
        .insert(SlotRules::new(|_, slot| slot != 3));
    settle(&mut app);
    drag_to(&mut app, slots[3], slots[5]);
    assert_eq!(moves(&app).len(), 2);
    assert_eq!(names(&app, ui.grid)[5], "Sword");
}

#[test]
fn the_keyboard_carry_skips_a_slot_that_says_no_and_focus_follows_the_item() {
    let mut app = app();
    let ui = ruled(&mut app);
    let slots = cells(&app, ui.grid);
    focus(&mut app, slots[0]);

    pick_up(&mut app);
    assert!(carried(&app));
    // Right of the sword is the helm (fine, it swaps back to slot 0: a sword is taken there);
    // right of that is the helmet slot, which refuses a sword: the mark stays where it was.
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(app.world().resource::<DragState>().over(), Some(slots[1]));
    tap(&mut app, KeyCode::ArrowRight);
    let over = app.world().resource::<DragState>().over();
    assert_ne!(
        over,
        Some(slots[2]),
        "a refusing slot is never a place to put the item down"
    );
    let target = slots.iter().position(|&c| Some(c) == over).expect("a slot");
    assert_ne!(target, 1, "the mark moved on past the helm");
    pick_up(&mut app);

    let seen = moves(&app);
    assert_eq!(seen.len(), 1);
    assert_eq!((seen[0].from.index, seen[0].to), (0, target));
    assert!(refusals(&app).is_empty());
    assert_eq!(focused(&app), over, "the ring went with the sword");
    assert_eq!(
        app.world().get::<SlotCursor>(ui.grid),
        Some(&SlotCursor(target))
    );
}

#[test]
fn a_grid_in_a_column_wider_than_itself_keeps_its_slots_together() {
    let mut app = app();
    let grid = app
        .world_mut()
        .spawn(slot_grid(3, vec![None, None, None]))
        .id();
    let wide = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(600.0),
            height: Val::Px(20.0),
            ..default()
        })
        .id();
    let root = app
        .world_mut()
        .spawn(screen())
        .insert(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        })
        .id();
    let column = app
        .world_mut()
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .add_children(&[wide, grid])
        .id();
    app.world_mut().entity_mut(root).add_child(column);
    settle(&mut app);
    let slots = cells(&app, grid);
    let skin = skin(&app);
    let (a, b) = (rect(&app, slots[0]), rect(&app, slots[1]));
    let gap = b.min.x - a.max.x;
    assert!(
        (gap - skin.gap * 0.5).abs() < 1.5,
        "slots {gap} px apart in a 600 px column, not the half gap {}",
        skin.gap * 0.5
    );
}

// ---- candidates: every slot that takes the carried item, from the moment it is lifted ----

/// The indices of the slots of `grid` that carry `DropCandidate`.
fn candidate_slots(app: &App, grid: Entity) -> Vec<usize> {
    cells(app, grid)
        .iter()
        .enumerate()
        .filter(|(_, cell)| app.world().get::<DropCandidate>(**cell).is_some())
        .map(|(i, _)| i)
        .collect()
}

/// Presses on a slot and moves a little way inside it: lifted, the pointer still over its own slot.
fn lift(app: &mut App, slot: Entity) {
    let at = centre(app, slot);
    pointer(app, at, PointerAction::Move { delta: Vec2::X });
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(
        app,
        at + Vec2::Y * 20.0,
        PointerAction::Move {
            delta: Vec2::Y * 20.0,
        },
    );
    settle(app);
}

fn edge(app: &App, slot: Entity) -> Color {
    app.world().get::<BorderColor>(slot).expect("edge").top
}

#[test]
fn lifting_an_item_marks_the_slots_that_take_it_and_none_that_refuse_it() {
    let mut app = app();
    let ui = ruled(&mut app);
    let slots = cells(&app, ui.grid);
    assert!(candidate_slots(&app, ui.grid).is_empty());

    // The sword (slot 0): the helmet-only slot 2 refuses it; the helm's slot takes it because the
    // helm may go back to slot 0; the rest are free. Not its own slot.
    lift(&mut app, slots[0]);
    assert_eq!(candidate_slots(&app, ui.grid), [1, 3, 4, 5]);
    let at = centre(&app, slots[0]);
    pointer(&mut app, at, PointerAction::Release(PointerButton::Primary));
    settle(&mut app);
    assert!(
        candidate_slots(&app, ui.grid).is_empty(),
        "let go ends them"
    );

    // The helm (slot 1) goes to the helmet slot too.
    lift(&mut app, slots[1]);
    assert_eq!(candidate_slots(&app, ui.grid), [0, 2, 3, 4, 5]);
}

#[test]
fn a_filled_slot_is_a_candidate_only_if_the_swap_back_is_taken_too() {
    let mut app = app();
    let ui = ruled(&mut app);
    let slots = cells(&app, ui.grid);
    // The helm goes to the helmet slot; now the sword cannot swap with it and the helm cannot
    // swap with the sword (the sword would land in the helmet slot).
    drag_to(&mut app, slots[1], slots[2]);
    assert_eq!(&names(&app, ui.grid)[..3], ["Sword", "", "Helm"]);

    lift(&mut app, slots[0]);
    assert_eq!(
        candidate_slots(&app, ui.grid),
        [1, 3, 4, 5],
        "not the helm's slot"
    );
    let at = centre(&app, slots[0]);
    pointer(&mut app, at, PointerAction::Release(PointerButton::Primary));
    settle(&mut app);

    lift(&mut app, slots[2]);
    assert_eq!(
        candidate_slots(&app, ui.grid),
        [1, 3, 5],
        "not the sword's slot, which would send the sword to the helmet slot, nor the potion's"
    );
}

#[test]
fn a_rule_that_changes_during_the_carry_moves_the_marks() {
    let mut app = app();
    let ui = ruled(&mut app);
    let slots = cells(&app, ui.grid);
    lift(&mut app, slots[0]);
    assert!(!candidate_slots(&app, ui.grid).contains(&2));

    app.world_mut()
        .entity_mut(ui.grid)
        .insert(SlotRules::new(|_, _| true));
    settle(&mut app);
    assert_eq!(candidate_slots(&app, ui.grid), [1, 2, 3, 4, 5]);
}

#[test]
fn the_keyboard_carry_marks_the_same_slots_and_a_drop_or_escape_ends_them() {
    let mut app = app();
    let ui = ruled(&mut app);
    let slots = cells(&app, ui.grid);
    focus(&mut app, slots[0]);

    pick_up(&mut app);
    assert!(carried(&app));
    assert_eq!(candidate_slots(&app, ui.grid), [1, 3, 4, 5]);
    key(&mut app, KeyCode::Escape, ButtonState::Pressed);
    key(&mut app, KeyCode::Escape, ButtonState::Released);
    settle(&mut app);
    assert!(candidate_slots(&app, ui.grid).is_empty());

    pick_up(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    pick_up(&mut app);
    assert!(candidate_slots(&app, ui.grid).is_empty());
}

#[test]
fn a_drag_between_grids_marks_both_grids_slots() {
    let mut app = app();
    let ui = ruled(&mut app);
    let chest = app
        .world_mut()
        .spawn(slot_grid(3, vec![None, None, None]))
        .id();
    let root = app.world().get::<ChildOf>(ui.grid).expect("root").parent();
    app.world_mut().entity_mut(root).add_child(chest);
    settle(&mut app);
    let slots = cells(&app, ui.grid);
    lift(&mut app, slots[0]);
    assert_eq!(candidate_slots(&app, chest), [0, 1, 2]);
    assert_eq!(candidate_slots(&app, ui.grid), [1, 3, 4, 5]);
}

#[test]
fn a_taking_slot_is_framed_and_a_refusing_one_is_dimmed_in_every_look() {
    for preset in SkinPreset::ALL {
        let mut app = app();
        app.insert_resource(preset.skin());
        let ui = ruled(&mut app);
        let skin = skin(&app);
        let slots = cells(&app, ui.grid);
        let quiet = skin.surface_edge.unwrap_or(skin.surface_raised);
        let name = preset.name();
        let fill = |app: &App, e: Entity| app.world().get::<BackgroundColor>(e).expect("fill").0;
        let ink = |app: &App, cell: Entity| {
            let part = parts_with(app, cell, Role::SlotName)[0];
            app.world().get::<TextColor>(part).expect("ink").0
        };
        assert_eq!(edge(&app, slots[1]), skin.surface_raised);
        assert_eq!(edge(&app, slots[3]), quiet);

        // The sword: the helm's slot and the free ones take it and are framed in the outline, a
        // filled one still raised and in the text's ink; the helmet slot refuses it and keeps its
        // quiet frame.
        lift(&mut app, slots[0]);
        assert_eq!(edge(&app, slots[1]), skin.outline, "{name}: a filled taker");
        assert_eq!(edge(&app, slots[3]), skin.outline, "{name}: an empty taker");
        assert_eq!(fill(&app, slots[1]), skin.surface_raised, "{name}");
        assert_eq!(ink(&app, slots[1]), skin.text, "{name}");
        assert_eq!(
            edge(&app, slots[2]),
            quiet,
            "{name}: a refusing slot keeps its frame"
        );
        assert!(app.world().get::<DropCandidate>(slots[0]).is_none());
        assert!(app.world().get::<DropRefusing>(slots[0]).is_none());

        // A candidate that has focus wears the ring.
        focus(&mut app, slots[3]);
        assert_eq!(edge(&app, slots[3]), skin.focus_ring, "{name}");

        let at = centre(&app, slots[0]);
        pointer(&mut app, at, PointerAction::Release(PointerButton::Primary));
        settle(&mut app);
        assert_eq!(
            edge(&app, slots[1]),
            skin.surface_raised,
            "{name}: frames go with the drag"
        );
        assert_eq!(edge(&app, slots[4]), skin.surface_raised, "{name}");

        // With the helm in the helmet slot, the sword cannot swap with it: that filled slot is
        // drawn as the panel's well, its words muted, its frame the quiet one.
        drag_to(&mut app, slots[1], slots[2]);
        lift(&mut app, slots[0]);
        assert!(app.world().get::<DropRefusing>(slots[2]).is_some());
        assert_eq!(
            fill(&app, slots[2]),
            skin.surface,
            "{name}: a refusing slot is a well"
        );
        assert_eq!(edge(&app, slots[2]), quiet, "{name}");
        assert_eq!(
            ink(&app, slots[2]),
            skin.text_muted,
            "{name}: its words are muted"
        );
        let at = centre(&app, slots[0]);
        pointer(&mut app, at, PointerAction::Release(PointerButton::Primary));
        settle(&mut app);
        assert_eq!(
            fill(&app, slots[2]),
            skin.surface_raised,
            "{name}: and raised again"
        );
        assert_eq!(ink(&app, slots[2]), skin.text, "{name}");
    }
}

#[test]
fn candidates_write_nothing_while_the_carry_holds_still() {
    let mut app = app();
    let ui = ruled(&mut app);
    let slots = cells(&app, ui.grid);
    lift(&mut app, slots[0]);
    settle(&mut app);
    app.world_mut().clear_trackers();
    for _ in 0..30 {
        app.update();
    }
    for slot in cells(&app, ui.grid) {
        let e = app.world().entity(slot);
        assert!(!e.get_ref::<BorderColor>().expect("b").is_changed());
        assert!(!e.get_ref::<Node>().expect("n").is_changed());
        if let Some(mark) = e.get_ref::<DropCandidate>() {
            assert!(!mark.is_changed());
        }
    }
}
