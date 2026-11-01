//! The number field and the vec field: a text row whose number the player scrubs, types and steps,
//! held to a range, with a mixed value and the reason a typed word is refused. Laid out for real in
//! a headless window, driven by real pointer, keyboard and gamepad input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedNode, InteractionDisabled, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<f32>` a game would see: the row, the value and whether the change ended.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, f32, bool)>);

/// Every `VecChange` a game would see: the component, its value, the whole vector and `is_final`.
#[derive(Resource, Default)]
struct VecChanges(Vec<(usize, f32, Vec<f32>, bool)>);

/// Every `NumberRefused`: the text and the reason.
#[derive(Resource, Default)]
struct Refusals(Vec<(String, NumberError)>);

/// How many frames' writes to what a number row shows the app has seen.
#[derive(Resource, Default)]
struct Writes(u32);

/// What a frame at rest must not write.
type Written = Or<(
    Changed<Text>,
    Changed<TextInputValue>,
    Changed<NumberValue>,
    Changed<BackgroundColor>,
    Changed<BorderColor>,
    Changed<Node>,
    Changed<TextColor>,
)>;

fn app_sized(width: u32, height: u32) -> App {
    let mut app = App::new();
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
    .init_resource::<Changes>()
    .init_resource::<VecChanges>()
    .init_resource::<Refusals>()
    .init_resource::<Writes>()
    .add_observer(|change: On<ValueChange<f32>>, mut seen: ResMut<Changes>| {
        seen.0.push((change.source, change.value, change.is_final));
    })
    .add_observer(|change: On<VecChange>, mut seen: ResMut<VecChanges>| {
        seen.0.push((
            change.index,
            change.value,
            change.values.clone(),
            change.is_final,
        ));
    })
    .add_observer(|refused: On<NumberRefused>, mut seen: ResMut<Refusals>| {
        seen.0.push((refused.text.clone(), refused.reason));
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
    app_sized(1280, 720)
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// A panel with a number row (health, 0 to 100 by 0.1), a button under it and a second row, as a
/// game's inspector lays them out.
struct Rig {
    health: Entity,
    other: Entity,
    button: Entity,
}

fn rig(app: &mut App) -> Rig {
    rig_with(app, 25.0, 0.1, 0.0..=100.0)
}

fn rig_with(app: &mut App, value: f32, step: f32, range: std::ops::RangeInclusive<f32>) -> Rig {
    let health = app
        .world_mut()
        .spawn(number_field("Health", value, step, range))
        .id();
    let other = app
        .world_mut()
        .spawn(number_field("Armour", 5.0, 1.0, 0.0..=10.0))
        .id();
    let button = app.world_mut().spawn(button("Apply")).id();
    app.world_mut()
        .spawn(panel())
        .add_children(&[health, other, button]);
    settle(app);
    Rig {
        health,
        other,
        button,
    }
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            for child in children.iter() {
                found.push(child);
                stack.push(child);
            }
        }
    }
    found
}

fn part(app: &App, root: Entity, role: Role) -> Entity {
    descendants(app.world(), root)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&role))
        .unwrap_or_else(|| panic!("a {role:?} under the widget"))
}

fn field(app: &App, row: Entity) -> Entity {
    part(app, row, Role::TextField)
}

/// The row's label: the text that is the scrub handle.
fn label_of(app: &App, row: Entity) -> Entity {
    part(app, row, Role::ControlText)
}

fn rect(app: &App, entity: Entity) -> Rect {
    let size = app
        .world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size();
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    Rect::from_center_size(at, size)
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn key_event(app: &mut App, key_code: KeyCode, logical_key: Key, state: ButtonState, repeat: bool) {
    let window = window(app);
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key,
        state,
        text: None,
        repeat,
        window,
    });
    app.update();
}

/// A named key pressed and released, one update each.
fn tap_named(app: &mut App, key_code: KeyCode, logical_key: Key) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        key_event(app, key_code, logical_key.clone(), state, false);
    }
    settle(app);
}

fn tap(app: &mut App, key_code: KeyCode) {
    let key = match key_code {
        KeyCode::ArrowUp => Key::ArrowUp,
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::Enter => Key::Enter,
        KeyCode::Escape => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::End => Key::End,
        _ => Key::Unidentified(NativeKey::Unidentified),
    };
    tap_named(app, key_code, key);
}

/// A key held (down without up) and later let go.
fn hold(app: &mut App, key_code: KeyCode) {
    key_event(
        app,
        key_code,
        Key::Unidentified(NativeKey::Unidentified),
        ButtonState::Pressed,
        false,
    );
}

fn let_go(app: &mut App, key_code: KeyCode) {
    key_event(
        app,
        key_code,
        Key::Unidentified(NativeKey::Unidentified),
        ButtonState::Released,
        false,
    );
}

fn type_char(app: &mut App, c: char) {
    let window = window(app);
    let text = c.to_string();
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: Key::Character(text.as_str().into()),
            state,
            text: (state == ButtonState::Pressed).then(|| text.as_str().into()),
            repeat: false,
            window,
        });
        app.update();
    }
}

fn type_str(app: &mut App, text: &str) {
    for c in text.chars() {
        type_char(app, c);
    }
    settle(app);
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
fn click_at(app: &mut App, at: Vec2) {
    point_at(app, at);
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    release(app, at);
}

/// Drags from the middle of `from` by `by` pointer px along x, and lets go.
fn drag_from(app: &mut App, from: Entity, by: f32) {
    let start = rect(app, from).center();
    let to = start + Vec2::new(by, 0.0);
    grab_and_move(app, start, to);
    release(app, to);
}

/// Drags a row by its label.
fn scrub(app: &mut App, row: Entity, by: i32) {
    let label = label_of(app, row);
    drag_from(app, label, by as f32);
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    settle(app);
}

fn focus_field(app: &mut App, row: Entity) {
    let field = field(app, row);
    focus(app, field);
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
    settle(app);
}

fn value(app: &App, row: Entity) -> f32 {
    app.world().get::<NumberValue>(row).expect("a number").0
}

/// The text the row holds, which the field itself must show after a frame.
fn text(app: &App, row: Entity) -> String {
    app.world()
        .get::<TextInputValue>(row)
        .expect("a text")
        .0
        .clone()
}

/// What the field itself holds.
fn typed(app: &App, row: Entity) -> String {
    let field = field(app, row);
    app.world()
        .get::<bevy::text::EditableText>(field)
        .expect("an editable text")
        .value()
        .to_string()
}

fn changes(app: &App) -> Vec<(f32, bool)> {
    app.world()
        .resource::<Changes>()
        .0
        .iter()
        .map(|(_, v, f)| (*v, *f))
        .collect()
}

fn forget(app: &mut App) {
    app.world_mut().resource_mut::<Changes>().0.clear();
    app.world_mut().resource_mut::<VecChanges>().0.clear();
    app.world_mut().resource_mut::<Refusals>().0.clear();
}

fn refusals(app: &App) -> Vec<(String, NumberError)> {
    app.world().resource::<Refusals>().0.clone()
}

/// Types `words` into `row`'s field in place of what it holds, and leaves it focused.
fn enter_text(app: &mut App, row: Entity, words: &str) {
    focus_field(app, row);
    tap(app, KeyCode::End);
    for _ in 0..16 {
        tap(app, KeyCode::Backspace);
    }
    type_str(app, words);
}

// ---------------------------------------------------------------------------------------------

#[test]
fn dragging_the_label_50_px_at_a_step_of_a_tenth_changes_the_value_by_5() {
    let mut app = app();
    let ui = rig_with(&mut app, 0.0, 0.1, 0.0..=100.0);
    forget(&mut app);
    scrub(&mut app, ui.health, 50);
    assert!(
        (value(&app, ui.health) - 5.0).abs() < 1e-4,
        "{}",
        value(&app, ui.health)
    );
    let seen = changes(&app);
    assert_eq!(seen.last(), Some(&(5.0, true)), "{seen:?}");
    assert_eq!(
        seen.iter().filter(|(_, f)| *f).count(),
        1,
        "one final change: {seen:?}"
    );
    assert!(
        seen[..seen.len() - 1].iter().all(|(_, f)| !f),
        "the others are not final: {seen:?}"
    );
    // The field shows what the number is.
    assert_eq!(text(&app, ui.health), "5.0");
    assert_eq!(typed(&app, ui.health), "5.0");
}

#[test]
fn a_scrub_is_not_typing_so_it_takes_the_field_off_focus_and_the_host_off_the_keys() {
    let mut app = app();
    let ui = rig(&mut app);
    focus_field(&mut app, ui.health);
    assert!(app.world().get::<TextEditing>(ui.health).is_some());
    let label = label_of(&app, ui.health);
    let start = rect(&app, label).center();
    grab_and_move(&mut app, start, start + Vec2::new(30.0, 0.0));
    assert_ne!(
        focused(&app),
        Some(field(&app, ui.health)),
        "the caret is gone"
    );
    assert!(
        app.world().get::<TextEditing>(ui.health).is_none(),
        "the game's hotkeys are back while the number rolls"
    );
    release(&mut app, start + Vec2::new(30.0, 0.0));
    assert!((value(&app, ui.health) - 28.0).abs() < 1e-3);
    // A click on the label without a drag still focuses the field, as a form's label does.
    click_at(&mut app, start);
    assert_eq!(focused(&app), Some(field(&app, ui.health)));
}

#[test]
fn the_same_drag_at_a_ui_scale_of_one_and_a_half_is_the_same_change() {
    // A 1920x1080 window draws the interface at 1.5x: 50 layout px is 75 window px.
    let mut app = app_sized(1920, 1080);
    settle(&mut app);
    assert!((app.world().resource::<UiScale>().0 - 1.5).abs() < 1e-3);
    let ui = rig_with(&mut app, 0.0, 0.1, 0.0..=100.0);
    scrub(&mut app, ui.health, 75);
    assert!(
        (value(&app, ui.health) - 5.0).abs() < 1e-4,
        "{}",
        value(&app, ui.health)
    );
}

#[test]
fn a_drag_left_goes_down_and_never_past_the_ends_of_the_range() {
    let mut app = app();
    let ui = rig(&mut app);
    scrub(&mut app, ui.health, -50);
    assert!((value(&app, ui.health) - 20.0).abs() < 1e-4);
    scrub(&mut app, ui.health, -2000);
    assert_eq!(value(&app, ui.health), 0.0, "held at the least");
    scrub(&mut app, ui.health, 5000);
    assert_eq!(value(&app, ui.health), 100.0, "held at the greatest");
    assert_eq!(text(&app, ui.health), "100.0");
}

#[test]
fn shift_scrubs_a_tenth_as_fast_and_ctrl_ten_times() {
    let mut app = app();
    let ui = rig_with(&mut app, 0.0, 0.1, 0.0..=100.0);
    hold(&mut app, KeyCode::ShiftLeft);
    scrub(&mut app, ui.health, 50);
    let_go(&mut app, KeyCode::ShiftLeft);
    assert!(
        (value(&app, ui.health) - 0.5).abs() < 1e-4,
        "{}",
        value(&app, ui.health)
    );
    hold(&mut app, KeyCode::ControlLeft);
    scrub(&mut app, ui.health, 5);
    let_go(&mut app, KeyCode::ControlLeft);
    assert!(
        (value(&app, ui.health) - 5.5).abs() < 1e-4,
        "{}",
        value(&app, ui.health)
    );
}

#[test]
fn a_drag_that_begins_in_the_field_is_the_fields_and_scrubs_nothing() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    let field = field(&app, ui.health);
    drag_from(&mut app, field, 80.0);
    assert_eq!(value(&app, ui.health), 25.0);
    assert!(changes(&app).is_empty(), "{:?}", changes(&app));
}

#[test]
fn a_drag_that_ends_where_it_began_says_nothing() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    scrub(&mut app, ui.health, 0);
    assert!(changes(&app).is_empty(), "{:?}", changes(&app));
}

#[test]
fn a_number_typed_and_entered_is_one_final_change_and_the_field_shows_it_as_a_number() {
    let mut app = app();
    let ui = rig(&mut app);
    enter_text(&mut app, ui.health, "2.50");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(value(&app, ui.health), 2.5);
    assert_eq!(changes(&app), vec![(2.5, true)]);
    assert_eq!(text(&app, ui.health), "2.5", "2.50 is shown as 2.5");
    assert_eq!(typed(&app, ui.health), "2.5");
    assert!(refusals(&app).is_empty());
}

#[test]
fn focusing_the_field_selects_its_number_so_typing_replaces_it() {
    let mut app = app();
    let ui = rig(&mut app);
    focus_field(&mut app, ui.health);
    type_str(&mut app, "7.5");
    assert_eq!(typed(&app, ui.health), "7.5", "not 25.07.5");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(value(&app, ui.health), 7.5);
}

#[test]
fn a_number_outside_the_range_is_held_to_its_end() {
    let mut app = app();
    let ui = rig(&mut app);
    enter_text(&mut app, ui.health, "150");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(value(&app, ui.health), 100.0);
    assert_eq!(changes(&app), vec![(100.0, true)]);
    assert_eq!(text(&app, ui.health), "100.0");
    enter_text(&mut app, ui.health, "-3");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(value(&app, ui.health), 0.0);
    assert!(refusals(&app).is_empty(), "held, not refused");
}

#[test]
fn an_expression_and_other_words_are_refused_with_their_reason_and_the_number_stays() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    enter_text(&mut app, ui.health, "1.5*2");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(value(&app, ui.health), 25.0);
    assert_eq!(
        refusals(&app),
        vec![("1.5*2".to_string(), NumberError::Expression)]
    );
    assert_eq!(
        text(&app, ui.health),
        "25.0",
        "the field shows the number again"
    );
    assert_eq!(typed(&app, ui.health), "25.0");
    assert!(changes(&app).is_empty(), "{:?}", changes(&app));
    forget(&mut app);
    enter_text(&mut app, ui.health, "abc");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(
        refusals(&app),
        vec![("abc".to_string(), NumberError::NotANumber)]
    );
    forget(&mut app);
    enter_text(&mut app, ui.health, "");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(refusals(&app), vec![(String::new(), NumberError::Empty)]);
    assert_eq!(value(&app, ui.health), 25.0);
    assert_eq!(typed(&app, ui.health), "25.0");
    assert_eq!(
        NumberError::Expression.to_string(),
        "type a number, not an expression"
    );
}

#[test]
fn leaving_after_an_edit_applies_it_once_and_leaving_after_none_says_nothing() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    focus_field(&mut app, ui.health);
    focus(&mut app, ui.button);
    assert!(changes(&app).is_empty());
    enter_text(&mut app, ui.health, "40");
    focus(&mut app, ui.button);
    assert_eq!(changes(&app), vec![(40.0, true)]);
    assert_eq!(value(&app, ui.health), 40.0);
    assert_eq!(text(&app, ui.health), "40.0");
}

#[test]
fn escape_puts_the_number_back_and_sends_nothing() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    enter_text(&mut app, ui.health, "99");
    tap(&mut app, KeyCode::Escape);
    assert_eq!(value(&app, ui.health), 25.0);
    assert_eq!(text(&app, ui.health), "25.0");
    assert_eq!(typed(&app, ui.health), "25.0");
    assert!(changes(&app).is_empty(), "{:?}", changes(&app));
    assert!(refusals(&app).is_empty());
}

#[test]
fn up_and_down_step_the_number_and_move_no_focus() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    focus_field(&mut app, ui.health);
    let field = field(&app, ui.health);
    tap(&mut app, KeyCode::ArrowUp);
    assert!((value(&app, ui.health) - 25.1).abs() < 1e-4);
    assert_eq!(text(&app, ui.health), "25.1");
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowDown);
    assert!((value(&app, ui.health) - 24.9).abs() < 1e-4);
    assert_eq!(focused(&app), Some(field), "focus stayed on the field");
    assert_eq!(
        changes(&app),
        vec![(25.1, true), (25.0, true), (24.9, true)]
    );
}

#[test]
fn shift_with_up_steps_ten_and_a_held_key_repeats() {
    let mut app = app();
    let ui = rig(&mut app);
    focus_field(&mut app, ui.health);
    hold(&mut app, KeyCode::ShiftLeft);
    tap(&mut app, KeyCode::ArrowUp);
    let_go(&mut app, KeyCode::ShiftLeft);
    assert!((value(&app, ui.health) - 26.0).abs() < 1e-4);
    // The platform's own repeat: three more presses of one held key.
    for _ in 0..3 {
        key_event(
            &mut app,
            KeyCode::ArrowDown,
            Key::ArrowDown,
            ButtonState::Pressed,
            true,
        );
    }
    key_event(
        &mut app,
        KeyCode::ArrowDown,
        Key::ArrowDown,
        ButtonState::Released,
        false,
    );
    settle(&mut app);
    assert!((value(&app, ui.health) - 25.7).abs() < 1e-4);
}

#[test]
fn a_step_never_goes_past_the_range_and_says_nothing_at_the_end() {
    let mut app = app();
    let ui = rig_with(&mut app, 99.95, 0.1, 0.0..=100.0);
    focus_field(&mut app, ui.health);
    forget(&mut app);
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(value(&app, ui.health), 100.0);
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(changes(&app), vec![(100.0, true)], "one change, then none");
}

#[test]
fn left_and_right_are_the_carets_and_move_no_focus_and_no_number() {
    let mut app = app();
    let ui = rig(&mut app);
    focus_field(&mut app, ui.health);
    let field = field(&app, ui.health);
    forget(&mut app);
    tap(&mut app, KeyCode::ArrowLeft);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(focused(&app), Some(field));
    assert_eq!(value(&app, ui.health), 25.0);
    assert!(changes(&app).is_empty());
}

#[test]
fn the_d_pad_steps_with_left_and_right_and_moves_focus_with_up_and_down() {
    let mut app = app();
    let ui = rig(&mut app);
    let pad = plug_in_pad(&mut app);
    focus_field(&mut app, ui.health);
    let field = field(&app, ui.health);
    press(&mut app, pad, GamepadButton::DPadRight);
    assert!((value(&app, ui.health) - 25.1).abs() < 1e-4);
    press(&mut app, pad, GamepadButton::DPadLeft);
    press(&mut app, pad, GamepadButton::DPadLeft);
    assert!((value(&app, ui.health) - 24.9).abs() < 1e-4);
    assert_eq!(focused(&app), Some(field), "left and right never leave");
    press(&mut app, pad, GamepadButton::DPadDown);
    assert_ne!(focused(&app), Some(field), "down moved focus on");
    assert!((value(&app, ui.health) - 24.9).abs() < 1e-4);
}

#[test]
fn a_value_the_game_sets_shows_at_once_and_says_nothing() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    app.world_mut()
        .get_mut::<NumberValue>(ui.health)
        .expect("a number")
        .0 = 61.25;
    settle(&mut app);
    assert_eq!(
        text(&app, ui.health),
        "61.25",
        "more digits than the step's, so the text is true"
    );
    assert_eq!(typed(&app, ui.health), "61.25");
    assert!(changes(&app).is_empty());
    app.world_mut()
        .entity_mut(ui.health)
        .insert(NumberStep(1.0));
    settle(&mut app);
    assert_eq!(text(&app, ui.health), "61.25");
    app.world_mut()
        .get_mut::<NumberValue>(ui.health)
        .expect("a number")
        .0 = 7.0;
    settle(&mut app);
    assert_eq!(
        text(&app, ui.health),
        "7",
        "a whole-number step shows a whole number"
    );
}

#[test]
fn a_mixed_value_is_an_empty_field_with_a_dash_and_a_scrub_makes_it_one_value() {
    let mut app = app();
    let ui = rig(&mut app);
    app.world_mut().entity_mut(ui.health).insert(MixedValue);
    settle(&mut app);
    assert_eq!(text(&app, ui.health), "");
    assert_eq!(typed(&app, ui.health), "");
    let hint = part(&app, ui.health, Role::TextHint);
    assert_eq!(
        app.world().get::<Text>(hint).expect("text").0,
        MIXED_HINT,
        "the muted hint"
    );
    assert_eq!(
        app.world().get::<Node>(hint).expect("node").display,
        Display::Flex,
        "shown while empty"
    );
    forget(&mut app);
    scrub(&mut app, ui.health, 100);
    assert!(app.world().get::<MixedValue>(ui.health).is_none());
    assert!(
        (value(&app, ui.health) - 35.0).abs() < 1e-3,
        "from the representative 25"
    );
    assert_eq!(text(&app, ui.health), "35.0");
    assert_eq!(
        app.world().get::<Text>(hint).expect("text").0,
        "",
        "no dash once it is one value"
    );
    assert!(
        changes(&app)
            .last()
            .is_some_and(|(v, f)| (*v - 35.0).abs() < 1e-3 && *f)
    );
}

#[test]
fn enter_on_an_untouched_mixed_field_changes_nothing_and_a_typed_number_or_a_step_sets_it() {
    let mut app = app();
    let ui = rig(&mut app);
    app.world_mut().entity_mut(ui.health).insert(MixedValue);
    settle(&mut app);
    forget(&mut app);
    focus_field(&mut app, ui.health);
    tap(&mut app, KeyCode::Enter);
    assert!(changes(&app).is_empty(), "{:?}", changes(&app));
    assert!(refusals(&app).is_empty());
    assert!(app.world().get::<MixedValue>(ui.health).is_some());
    type_str(&mut app, "8");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(changes(&app), vec![(8.0, true)]);
    assert!(app.world().get::<MixedValue>(ui.health).is_none());
    assert_eq!(text(&app, ui.health), "8.0");
    // A step on another mixed row is one change.
    app.world_mut().entity_mut(ui.other).insert(MixedValue);
    settle(&mut app);
    forget(&mut app);
    focus_field(&mut app, ui.other);
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(changes(&app), vec![(6.0, true)]);
    assert!(app.world().get::<MixedValue>(ui.other).is_none());
}

#[test]
fn a_disabled_row_does_not_scrub_step_or_take_text() {
    let mut app = app();
    let ui = rig(&mut app);
    app.world_mut()
        .entity_mut(ui.health)
        .insert(InteractionDisabled);
    settle(&mut app);
    forget(&mut app);
    scrub(&mut app, ui.health, 50);
    assert_eq!(value(&app, ui.health), 25.0);
    focus_field(&mut app, ui.health);
    tap(&mut app, KeyCode::ArrowUp);
    type_str(&mut app, "9");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(value(&app, ui.health), 25.0);
    assert!(changes(&app).is_empty(), "{:?}", changes(&app));
}

#[test]
fn the_row_is_a_target_high_and_never_changes_size_while_the_number_scrubs() {
    let mut app = app();
    let ui = rig_with(&mut app, 0.0, 0.1, -100000.0..=100000.0);
    let before = rect(&app, ui.health).size();
    let field_before = rect(&app, field(&app, ui.health)).size();
    assert!(before.y >= 44.0 - 1e-3, "{before}");
    for to in [99999.9, -12345.6, 0.5] {
        app.world_mut()
            .get_mut::<NumberValue>(ui.health)
            .expect("a number")
            .0 = to;
        settle(&mut app);
        assert_eq!(rect(&app, ui.health).size(), before, "at {to}");
        assert_eq!(
            rect(&app, field(&app, ui.health)).size(),
            field_before,
            "at {to}"
        );
    }
}

#[test]
fn number_field_is_a_text_field_for_the_host_and_text_editing_is_on_while_it_is_focused() {
    let mut app = app();
    let ui = rig(&mut app);
    assert!(app.world().get::<TextEditing>(ui.health).is_none());
    focus_field(&mut app, ui.health);
    assert!(app.world().get::<TextEditing>(ui.health).is_some());
    focus(&mut app, ui.button);
    assert!(app.world().get::<TextEditing>(ui.health).is_none());
}

#[test]
fn a_selected_number_is_drawn_in_its_selection_ink_only_while_the_field_has_focus() {
    // Bevy applies `selected_text_color` with or without focus; the accent's dark ink with no
    // highlight behind it made a number the player had focused and left unreadable.
    let mut app = app();
    let ui = rig(&mut app);
    let ink = |app: &App| {
        app.world()
            .get::<bevy::text::TextCursorStyle>(field(app, ui.health))
            .expect("a cursor style")
            .selected_text_color
    };
    focus_field(&mut app, ui.health);
    assert_eq!(
        ink(&app),
        Some(app.world().resource::<GooeySkin>().accent_text),
        "focused, the selection is lit and its text is the accent's"
    );
    focus(&mut app, ui.button);
    assert_eq!(ink(&app), None, "away, it is the field's own colour");
    // And after a scrub, which takes focus off the field with its text still selected.
    let label = label_of(&app, ui.other);
    let start = rect(&app, label).center();
    focus_field(&mut app, ui.other);
    grab_and_move(&mut app, start, start + Vec2::new(20.0, 0.0));
    release(&mut app, start + Vec2::new(20.0, 0.0));
    assert_eq!(
        app.world()
            .get::<bevy::text::TextCursorStyle>(field(&app, ui.other))
            .expect("a cursor style")
            .selected_text_color,
        None
    );
}

#[test]
fn thirty_idle_frames_write_nothing() {
    let mut app = app();
    let ui = rig(&mut app);
    let vec = app
        .world_mut()
        .spawn(vec_field("Position", [1.0, 2.0, 3.0], 0.1, -10.0..=10.0))
        .id();
    let _ = (ui, vec);
    settle(&mut app);
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0);
}

// ---------------------------------------------------------------------------------------------
// The vec field.

fn vec_rig(app: &mut App) -> (Entity, Vec<Entity>) {
    let row = app
        .world_mut()
        .spawn(vec_field("Position", [1.0, 2.0, 3.0], 0.5, -100.0..=100.0))
        .id();
    settle(app);
    let parts: Vec<Entity> = {
        let mut found: Vec<(usize, Entity)> = app
            .world_mut()
            .query::<(Entity, &VecPart)>()
            .iter(app.world())
            .filter(|(_, p)| p.root == row)
            .map(|(e, p)| (p.index, e))
            .collect();
        found.sort();
        found.into_iter().map(|(_, e)| e).collect()
    };
    (row, parts)
}

#[test]
fn a_vec_field_is_a_label_and_a_row_of_fields_each_with_its_letter_and_the_rows_numbers() {
    let mut app = app();
    let (row, parts) = vec_rig(&mut app);
    assert_eq!(parts.len(), 3);
    assert_eq!(
        app.world().get::<VecValue>(row).expect("values").0,
        vec![1.0, 2.0, 3.0]
    );
    let letters: Vec<String> = parts
        .iter()
        .map(|&p| {
            let label = label_of(&app, p);
            app.world().get::<Text>(label).expect("text").0.clone()
        })
        .collect();
    assert_eq!(letters, vec!["X", "Y", "Z"]);
    let shown: Vec<String> = parts.iter().map(|&p| typed(&app, p)).collect();
    assert_eq!(shown, vec!["1.0", "2.0", "3.0"]);
    // Side by side, each a target high, the letter at least 24 px of handle.
    let rects: Vec<Rect> = parts.iter().map(|&p| rect(&app, p)).collect();
    assert!(rects[0].max.x <= rects[1].min.x + 1.0 && rects[1].max.x <= rects[2].min.x + 1.0);
    for r in &rects {
        assert!(r.height() >= 44.0 - 1e-3, "{r:?}");
    }
    let letter = label_of(&app, parts[0]);
    let handle = rect(&app, parts[0]).min.x..rect(&app, letter).max.x;
    assert!(handle.end - handle.start >= 24.0, "{handle:?}");
}

#[test]
fn dragging_a_letter_changes_that_component_and_sends_one_vec_change_each_with_the_vector() {
    let mut app = app();
    let (row, parts) = vec_rig(&mut app);
    forget(&mut app);
    scrub(&mut app, parts[1], 20);
    // 20 px at a step of 0.5 is 10: 2 to 12.
    assert_eq!(
        app.world().get::<VecValue>(row).expect("values").0,
        vec![1.0, 12.0, 3.0]
    );
    let seen = app.world().resource::<VecChanges>().0.clone();
    let last = seen.last().expect("a change");
    assert_eq!(last, &(1, 12.0, vec![1.0, 12.0, 3.0], true), "{seen:?}");
    assert!(seen.iter().all(|(i, ..)| *i == 1));
    assert_eq!(typed(&app, parts[1]), "12.0");
    assert_eq!(typed(&app, parts[0]), "1.0", "the others stand");
}

#[test]
fn a_component_typed_stepped_and_held_to_the_range_is_the_vec_rows_change() {
    let mut app = app();
    let (row, parts) = vec_rig(&mut app);
    forget(&mut app);
    enter_text(&mut app, parts[2], "500");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(
        app.world().get::<VecValue>(row).expect("values").0,
        vec![1.0, 2.0, 100.0]
    );
    forget(&mut app);
    focus_field(&mut app, parts[0]);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(
        app.world().resource::<VecChanges>().0,
        vec![(0, 0.5, vec![0.5, 2.0, 100.0], true)]
    );
}

#[test]
fn a_vector_the_game_sets_shows_in_its_components_and_says_nothing() {
    let mut app = app();
    let (row, parts) = vec_rig(&mut app);
    forget(&mut app);
    app.world_mut().get_mut::<VecValue>(row).expect("values").0 = vec![-4.5, 0.0, 9.0];
    settle(&mut app);
    let shown: Vec<String> = parts.iter().map(|&p| typed(&app, p)).collect();
    assert_eq!(shown, vec!["-4.5", "0.0", "9.0"]);
    assert!(app.world().resource::<VecChanges>().0.is_empty());
    assert!(changes(&app).is_empty() || changes(&app).iter().all(|_| false));
}

#[test]
fn a_mixed_component_is_empty_alone_and_the_players_change_to_it_takes_it_off() {
    let mut app = app();
    let (row, parts) = vec_rig(&mut app);
    app.world_mut()
        .entity_mut(row)
        .insert(VecMixed(vec![false, true, false]));
    settle(&mut app);
    assert_eq!(typed(&app, parts[0]), "1.0");
    assert_eq!(typed(&app, parts[1]), "", "the mixed one is empty");
    assert_eq!(typed(&app, parts[2]), "3.0");
    forget(&mut app);
    scrub(&mut app, parts[1], 4);
    assert_eq!(
        app.world().get::<VecMixed>(row).expect("mixed").0,
        vec![false, false, false]
    );
    assert_eq!(typed(&app, parts[1]), "4.0", "2 plus 4 px of 0.5");
    let seen = app.world().resource::<VecChanges>().0.clone();
    assert_eq!(seen.last().map(|(i, ..)| *i), Some(1), "{seen:?}");
}

#[test]
fn a_vec_field_keeps_its_row_whole_at_200_percent_text() {
    let mut app = app_sized(1280, 720);
    app.world_mut().resource_mut::<GooeySkin>().text_scale = 2.0;
    let (row, parts) = vec_rig(&mut app);
    settle(&mut app);
    let whole = rect(&app, row);
    for &p in &parts {
        let r = rect(&app, p);
        assert!(
            r.min.x >= whole.min.x - 1.0 && r.max.x <= whole.max.x + 1.0,
            "{r:?} inside {whole:?}"
        );
    }
}
