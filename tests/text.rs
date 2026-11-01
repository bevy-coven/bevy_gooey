//! The text input: a row with a field the player types in. It keeps its text, tells the game of
//! each edit and of the edit's end, never traps focus, tells the host while it holds the keys, and
//! never changes size while the player types. Laid out for real in a headless window, driven by
//! real keyboard, pointer and gamepad input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButtonInput;
use bevy::input_focus::{AutoFocus, FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui::{InteractionDisabled, UiGlobalTransform};
use bevy::ui_widgets::TextInput;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<String>` a game would see: the row, the text and whether the edit ended.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, String, bool)>);

/// How many `Cancel`s the screen heard.
#[derive(Resource, Default)]
struct Cancels(u32);

/// How many frames' writes the app has seen to what a text field's look and state change.
#[derive(Resource, Default)]
struct Writes(u32);

/// What a frame at rest must not write.
type Written = Or<(
    Changed<Text>,
    Changed<TextInputValue>,
    Changed<BackgroundColor>,
    Changed<BorderColor>,
    Changed<Node>,
    Changed<TextColor>,
)>;

/// A headless app with a `width` × `height` window, so UI layout and picking run.
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
    .init_resource::<Cancels>()
    .init_resource::<Writes>()
    .add_observer(
        |change: On<ValueChange<String>>, mut seen: ResMut<Changes>| {
            seen.0
                .push((change.source, change.value.clone(), change.is_final))
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

fn app() -> App {
    app_sized(1280, 720)
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// A settings screen as a game builds it: a toggle above a name field above a seed field above a
/// Back button, in a panel on a screen that counts the `Cancel`s it hears.
struct Settings {
    card: Entity,
    name: Entity,
    seed: Entity,
    above: Entity,
    back: Entity,
}

fn settings(app: &mut App) -> Settings {
    let world = app.world_mut();
    let above = world.spawn(toggle("Subtitles", true)).id();
    let name = world
        .spawn((
            text_input("Name", ""),
            TextHint("Enter a name".into()),
            TextLimit(12),
        ))
        .id();
    let seed = world.spawn(text_input("Seed", "")).id();
    let back = world.spawn(button("Back")).id();
    let card = world
        .spawn(panel())
        .add_children(&[above, name, seed, back])
        .id();
    let screen = world.spawn(screen()).add_child(card).id();
    world
        .entity_mut(screen)
        .observe(|_: On<Cancel>, mut seen: ResMut<Cancels>| seen.0 += 1);
    settle(app);
    Settings {
        card,
        name,
        seed,
        above,
        back,
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

/// The part of `root` with `role`.
fn part(app: &App, root: Entity, role: Role) -> Entity {
    descendants(app.world(), root)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&role))
        .unwrap_or_else(|| panic!("a {role:?} under the widget"))
}

fn field(app: &App, row: Entity) -> Entity {
    part(app, row, Role::TextField)
}

/// Where `entity` is drawn, in logical px (the window's scale factor and `UiScale` are 1).
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

/// A key pressed and released, one update each.
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

/// A named key (Enter, Backspace, the arrows) pressed and released, one update each.
fn tap_named(app: &mut App, key_code: KeyCode, logical_key: Key) {
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: logical_key.clone(),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
}

/// One character typed: the key press as a platform reports it, with the text it produces.
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

fn enter(app: &mut App) {
    tap_named(app, KeyCode::Enter, Key::Enter);
}

fn backspace(app: &mut App) {
    tap_named(app, KeyCode::Backspace, Key::Backspace);
}

fn arrow(app: &mut App, key_code: KeyCode) {
    let key = match key_code {
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::ArrowUp => Key::ArrowUp,
        _ => Key::ArrowDown,
    };
    tap_named(app, key_code, key);
    settle(app);
}

fn escape(app: &mut App) {
    tap_named(app, KeyCode::Escape, Key::Escape);
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

fn point_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
}

/// A primary click at `at`: the pointer moved there, pressed and released.
fn click(app: &mut App, at: Vec2) {
    point_at(app, at);
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        });
        let action = match state {
            ButtonState::Pressed => PointerAction::Press(PointerButton::Primary),
            ButtonState::Released => PointerAction::Release(PointerButton::Primary),
        };
        pointer(app, at, action);
    }
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

/// Presses and releases `button` on `pad`, one update each.
fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    settle(app);
}

/// Focuses the field of `row`, shown.
fn focus_field(app: &mut App, row: Entity) {
    let field = field(app, row);
    focus(app, field);
}

/// A primary click on the middle of `entity`.
fn click_on(app: &mut App, entity: Entity) {
    let at = rect(app, entity).center();
    click(app, at);
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn text(app: &App, row: Entity) -> String {
    app.world()
        .get::<TextInputValue>(row)
        .expect("a value")
        .0
        .clone()
}

/// What the field itself holds, which the row's value must always equal after a frame.
fn typed(app: &App, row: Entity) -> String {
    let field = field(app, row);
    app.world()
        .get::<EditableText>(field)
        .expect("an editable text")
        .value()
        .to_string()
}

/// Every change the game saw, as (text, final).
fn changes(app: &App) -> Vec<(String, bool)> {
    app.world()
        .resource::<Changes>()
        .0
        .iter()
        .map(|(_, text, is_final)| (text.clone(), *is_final))
        .collect()
}

fn forget(app: &mut App) {
    app.world_mut().resource_mut::<Changes>().0.clear();
}

fn cancels(app: &App) -> u32 {
    app.world().resource::<Cancels>().0
}

fn editing(app: &App, row: Entity) -> bool {
    app.world().get::<TextEditing>(row).is_some()
}

fn hint_shown(app: &App, row: Entity) -> bool {
    let hint = part(app, row, Role::TextHint);
    app.world()
        .get::<Node>(hint)
        .is_some_and(|n| n.display != Display::None)
}

fn rgb(color: Color) -> [f32; 3] {
    let c = color.to_srgba();
    [c.red, c.green, c.blue]
}

fn background(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

fn text_colour(app: &App, entity: Entity) -> Color {
    app.world().get::<TextColor>(entity).expect("a colour").0
}

fn border_colour(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BorderColor>(entity)
        .expect("a border colour")
        .top
}

fn contrast(a: Color, b: Color) -> f32 {
    contrast_ratio(rgb(a), rgb(b))
}

// ---------------------------------------------------------------------------------------------

#[test]
fn a_field_stands_right_of_its_label_a_target_high_and_the_hint_shows_while_it_is_empty() {
    let mut app = app();
    let s = settings(&mut app);
    let label = part(&app, s.name, Role::ControlText);
    let field = field(&app, s.name);
    let field_box = rect(&app, field);
    assert!(
        rect(&app, label).max.x < field_box.min.x,
        "label, then field"
    );
    assert!(
        field_box.height() >= 44.0,
        "a target: {}",
        field_box.height()
    );
    assert!(
        field_box.width() > 200.0,
        "16 characters: {}",
        field_box.width()
    );
    // The label stands where the other rows' labels do.
    let toggle_label = part(&app, s.above, Role::ControlText);
    assert!(
        (rect(&app, label).min.x - rect(&app, toggle_label).min.x).abs() <= 1.0,
        "{} vs {}",
        rect(&app, label).min.x,
        rect(&app, toggle_label).min.x
    );
    // The field and the row's right edge line up across the two fields.
    assert!((rect(&app, field).max.x - rect(&app, self::field(&app, s.seed)).max.x).abs() <= 1.0);
    // The row is a toggle's height, the field a target high inside it, the hint over its text.
    assert!(
        (rect(&app, s.name).height() - rect(&app, s.above).height()).abs() <= 1.0,
        "{} vs {}",
        rect(&app, s.name).height(),
        rect(&app, s.above).height()
    );
    let hint_box = rect(&app, part(&app, s.name, Role::TextHint));
    assert!(
        (hint_box.min.x - (field_box.min.x + 3.0 + 8.4)).abs() <= 1.0,
        "{hint_box:?}"
    );
    assert!(
        (hint_box.center().y - rect(&app, label).center().y).abs() <= 1.0,
        "the hint and the label share a line"
    );
    assert!(hint_shown(&app, s.name));
    assert!(!hint_shown(&app, s.seed), "no hint was given");
    let hint = part(&app, s.name, Role::TextHint);
    assert_eq!(
        app.world().get::<Text>(hint).map(|t| t.0.as_str()),
        Some("Enter a name")
    );
}

#[test]
fn typing_changes_the_value_and_each_edit_is_one_change_that_is_not_final() {
    let mut app = app();
    let s = settings(&mut app);
    focus_field(&mut app, s.name);
    type_str(&mut app, "Ari");
    assert_eq!(text(&app, s.name), "Ari");
    assert_eq!(typed(&app, s.name), "Ari");
    assert_eq!(
        changes(&app),
        vec![
            ("A".to_string(), false),
            ("Ar".to_string(), false),
            ("Ari".to_string(), false)
        ]
    );
    assert!(
        !hint_shown(&app, s.name),
        "the hint goes with the first letter"
    );
    backspace(&mut app);
    settle(&mut app);
    assert_eq!(text(&app, s.name), "Ar");
    assert_eq!(changes(&app).last(), Some(&("Ar".to_string(), false)));
}

#[test]
fn the_caret_moving_says_nothing_and_enter_submits_once_with_the_text_and_stays() {
    let mut app = app();
    let s = settings(&mut app);
    focus_field(&mut app, s.name);
    type_str(&mut app, "Ari");
    forget(&mut app);
    arrow(&mut app, KeyCode::ArrowLeft);
    arrow(&mut app, KeyCode::ArrowLeft);
    assert_eq!(changes(&app), vec![], "a caret that moves is no edit");
    assert_eq!(
        focused(&app),
        Some(field(&app, s.name)),
        "Left never leaves"
    );
    enter(&mut app);
    settle(&mut app);
    assert_eq!(changes(&app), vec![("Ari".to_string(), true)]);
    assert_eq!(
        focused(&app),
        Some(field(&app, s.name)),
        "Enter stays in the field"
    );
    enter(&mut app);
    settle(&mut app);
    assert_eq!(
        changes(&app),
        vec![("Ari".to_string(), true), ("Ari".to_string(), true)],
        "each Enter is a submit, changed or not"
    );
}

#[test]
fn leaving_after_an_edit_ends_it_once_and_leaving_after_none_says_nothing() {
    let mut app = app();
    let s = settings(&mut app);
    focus_field(&mut app, s.name);
    forget(&mut app);
    arrow(&mut app, KeyCode::ArrowDown);
    assert_eq!(changes(&app), vec![], "no edit, no ending");
    focus_field(&mut app, s.name);
    type_str(&mut app, "Z");
    forget(&mut app);
    focus(&mut app, s.back);
    assert_eq!(
        changes(&app),
        vec![("Z".to_string(), true)],
        "the edit ended once"
    );
    focus_field(&mut app, s.name);
    focus(&mut app, s.back);
    assert_eq!(
        changes(&app),
        vec![("Z".to_string(), true)],
        "and not again"
    );
}

#[test]
fn a_value_the_game_sets_shows_at_once_and_says_nothing() {
    let mut app = app();
    let s = settings(&mut app);
    app.world_mut()
        .get_mut::<TextInputValue>(s.name)
        .expect("a value")
        .0 = "Marguerite".into();
    settle(&mut app);
    assert_eq!(typed(&app, s.name), "Marguerite");
    assert!(!hint_shown(&app, s.name));
    assert_eq!(
        changes(&app),
        vec![],
        "a change the game made is not news to it"
    );
    // The caret is at the end: typing appends.
    focus_field(&mut app, s.name);
    type_str(&mut app, "!");
    assert_eq!(
        text(&app, s.name),
        "Marguerite!"[..11].to_string(),
        "limit 12 keeps 11 + 1"
    );
    app.world_mut()
        .get_mut::<TextInputValue>(s.name)
        .expect("a value")
        .0 = String::new();
    settle(&mut app);
    assert_eq!(typed(&app, s.name), "");
    assert!(hint_shown(&app, s.name), "empty again, the hint is back");
}

#[test]
fn the_limit_stops_the_player_and_never_cuts_what_the_game_set() {
    let mut app = app();
    let s = settings(&mut app);
    let long = "a value the game set is longer than twelve";
    app.world_mut()
        .get_mut::<TextInputValue>(s.name)
        .expect("a value")
        .0 = long.into();
    settle(&mut app);
    assert_eq!(typed(&app, s.name), long, "never cut");
    app.world_mut()
        .get_mut::<TextInputValue>(s.name)
        .expect("a value")
        .0 = String::new();
    settle(&mut app);
    focus_field(&mut app, s.name);
    type_str(&mut app, "abcdefghijklmnop");
    assert_eq!(text(&app, s.name), "abcdefghijkl", "12 characters");
    assert_eq!(typed(&app, s.name), "abcdefghijkl");
    // Sixteen characters were typed, twelve were taken: twelve changes, none for the refused four.
    assert_eq!(changes(&app).len(), 12);
}

#[test]
fn up_and_down_tab_and_the_d_pad_leave_the_field_and_left_and_right_never_do() {
    let mut app = app();
    let s = settings(&mut app);
    let name = field(&app, s.name);
    let seed = field(&app, s.seed);
    focus(&mut app, name);
    arrow(&mut app, KeyCode::ArrowRight);
    arrow(&mut app, KeyCode::ArrowLeft);
    assert_eq!(focused(&app), Some(name), "Left and Right are the caret's");
    arrow(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(seed), "Down moves on");
    arrow(&mut app, KeyCode::ArrowUp);
    assert_eq!(focused(&app), Some(name), "Up moves back");
    arrow(&mut app, KeyCode::ArrowUp);
    assert_eq!(
        focused(&app),
        Some(s.above),
        "Up leaves the first field for the toggle"
    );
    focus(&mut app, name);
    tap(&mut app, KeyCode::Tab);
    settle(&mut app);
    assert_eq!(focused(&app), Some(seed), "Tab leaves");
}

#[test]
fn a_keys_left_and_right_are_the_carets_but_the_d_pad_moves_on() {
    let mut app = app();
    let pad = plug_in_pad(&mut app);
    let ok = app.world_mut().spawn(button("OK")).id();
    let row = app.world_mut().spawn(text_input("Port", "")).id();
    let beside = app
        .world_mut()
        .spawn(Node {
            column_gap: Val::Px(12.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .add_children(&[row, ok])
        .id();
    let card = app.world_mut().spawn(panel()).add_child(beside).id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);
    let port = field(&app, row);
    focus(&mut app, port);
    arrow(&mut app, KeyCode::ArrowRight);
    assert_eq!(
        focused(&app),
        Some(port),
        "Right is the caret's, the button beside it waits"
    );
    press(&mut app, pad, GamepadButton::DPadRight);
    settle(&mut app);
    assert_eq!(focused(&app), Some(ok), "the D-pad moves on to the button");
}

#[test]
fn a_click_on_the_label_the_field_or_the_rows_edge_focuses_the_field() {
    let mut app = app();
    let s = settings(&mut app);
    let name = field(&app, s.name);
    let label_box = rect(&app, part(&app, s.name, Role::ControlText));
    click(&mut app, label_box.center());
    assert_eq!(focused(&app), Some(name), "the label");
    focus(&mut app, s.back);
    click_on(&mut app, name);
    assert_eq!(focused(&app), Some(name), "the field");
    focus(&mut app, s.back);
    let row = rect(&app, s.name);
    click(&mut app, Vec2::new(row.min.x + 4.0, row.center().y));
    assert_eq!(focused(&app), Some(name), "the row's padding");
    click(&mut app, Vec2::new(5.0, 5.0));
    assert_eq!(focused(&app), None, "empty space blurs");
}

#[test]
fn the_frame_is_lit_while_the_field_holds_focus_even_after_a_pointer_press_hid_every_ring() {
    let mut app = app();
    let s = settings(&mut app);
    let name = field(&app, s.name);
    let skin = app.world().resource::<GooeySkin>().clone();
    assert_eq!(border_colour(&app, name), skin.outline);
    click_on(&mut app, name);
    assert_eq!(focused(&app), Some(name));
    assert!(
        !app.world().resource::<InputFocusVisible>().0,
        "a press hides rings"
    );
    assert_eq!(
        border_colour(&app, name),
        skin.focus_ring,
        "the field's stays"
    );
    focus(&mut app, s.back);
    assert_eq!(
        border_colour(&app, name),
        skin.text_muted,
        "the pointer is still over it"
    );
    point_at(&mut app, Vec2::new(5.0, 5.0));
    settle(&mut app);
    assert_eq!(border_colour(&app, name), skin.outline, "and then at rest");
}

#[test]
fn text_editing_is_on_the_row_exactly_while_its_field_has_focus() {
    let mut app = app();
    let s = settings(&mut app);
    assert!(!editing(&app, s.name));
    focus_field(&mut app, s.name);
    assert!(editing(&app, s.name));
    assert!(!editing(&app, s.seed));
    focus_field(&mut app, s.seed);
    assert!(!editing(&app, s.name) && editing(&app, s.seed));
    focus(&mut app, s.back);
    assert!(!editing(&app, s.seed));
    let mut ids =
        bevy::ecs::system::RunSystemOnce::run_system_once(&mut *app.world_mut(), is_text_editing);
    assert!(matches!(ids, Ok(false)));
    focus_field(&mut app, s.seed);
    ids = bevy::ecs::system::RunSystemOnce::run_system_once(&mut *app.world_mut(), is_text_editing);
    assert!(matches!(ids, Ok(true)));
}

#[test]
fn focus_on_the_row_goes_to_its_field() {
    let mut app = app();
    let s = settings(&mut app);
    app.world_mut().entity_mut(s.seed).insert(AutoFocus);
    settle(&mut app);
    assert_eq!(focused(&app), Some(field(&app, s.seed)));
    assert!(editing(&app, s.seed));
}

#[test]
fn escape_ends_the_edit_and_sends_no_cancel_where_no_dialog_is_open() {
    let mut app = app();
    let s = settings(&mut app);
    focus_field(&mut app, s.name);
    type_str(&mut app, "Ari");
    forget(&mut app);
    escape(&mut app);
    assert_eq!(focused(&app), None, "Bevy's own: Escape blurs");
    assert_eq!(changes(&app), vec![("Ari".to_string(), true)]);
    assert_eq!(cancels(&app), 0, "Escape stays the game's pause key");
    assert!(!editing(&app, s.name));
}

#[test]
fn a_disabled_field_takes_no_character_and_a_row_disabled_after_the_fact_disables_it() {
    let mut app = app();
    let s = settings(&mut app);
    app.world_mut()
        .entity_mut(s.name)
        .insert(InteractionDisabled);
    settle(&mut app);
    assert!(
        app.world()
            .get::<InteractionDisabled>(field(&app, s.name))
            .is_some()
    );
    focus_field(&mut app, s.name);
    type_str(&mut app, "x");
    assert_eq!(text(&app, s.name), "");
    app.world_mut()
        .entity_mut(s.name)
        .remove::<InteractionDisabled>();
    settle(&mut app);
    assert!(
        app.world()
            .get::<InteractionDisabled>(field(&app, s.name))
            .is_none()
    );
    type_str(&mut app, "x");
    assert_eq!(text(&app, s.name), "x");
}

#[test]
fn a_field_is_a_text_input_and_a_hint_is_not() {
    let mut app = app();
    let s = settings(&mut app);
    let field = field(&app, s.name);
    assert!(app.world().get::<TextInput>(field).is_some());
    let hint = part(&app, s.name, Role::TextHint);
    assert!(app.world().get::<TextInput>(hint).is_none());
    assert_eq!(
        app.world().get::<TextInputOf>(hint).map(|o| o.0),
        Some(s.name)
    );
    let _ = s.card;
}

#[test]
fn a_dialog_that_holds_a_field_opens_on_it_and_escape_is_back_with_one_cancel() {
    let mut app = app();
    let s = settings(&mut app);
    let save = app.world_mut().spawn(text_input("Name your save", "")).id();
    let ok = app.world_mut().spawn(button("OK")).id();
    let choices = app.world_mut().spawn(menu()).add_child(ok).id();
    let card = app
        .world_mut()
        .spawn(panel())
        .add_children(&[save, choices])
        .id();
    let dialog = app.world_mut().spawn(dialog()).add_child(card).id();
    app.world_mut()
        .entity_mut(dialog)
        .observe(|_: On<Cancel>, mut seen: ResMut<Cancels>| seen.0 += 1);
    settle(&mut app);
    assert_eq!(
        focused(&app),
        Some(field(&app, save)),
        "a dialog opens on its first widget, here a field"
    );
    assert!(editing(&app, save));
    type_str(&mut app, "Run 1");
    assert_eq!(text(&app, save), "Run 1");
    arrow(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(ok), "Down reaches OK inside the dialog");
    arrow(&mut app, KeyCode::ArrowUp);
    assert_eq!(focused(&app), Some(field(&app, save)));
    escape(&mut app);
    assert_eq!(cancels(&app), 1, "Escape in a dialog is Back, once");
    assert_eq!(
        text(&app, save),
        "Run 1",
        "what Back does with the text is the game's"
    );
    let _ = s;
}

#[test]
fn text_hint_frame_ring_and_selection_are_held_to_contrast_in_every_preset() {
    for preset in [
        GooeySkin::dark(),
        GooeySkin::parchment(),
        GooeySkin::high_contrast(),
    ] {
        let mut app = app();
        app.insert_resource(preset.clone());
        let s = settings(&mut app);
        assert_eq!(preset.problems(), vec![]);
        let name = field(&app, s.name);
        let hint = part(&app, s.name, Role::TextHint);
        let fill = background(&app, name);
        let panel_fill = preset.solid_surface();
        assert_eq!(fill, panel_fill, "the field is the panel's own colour");
        let typed = contrast(text_colour(&app, name), fill);
        let hinted = contrast(text_colour(&app, hint), fill);
        let frame = contrast(border_colour(&app, name), panel_fill);
        assert!(typed >= 4.5, "typed text {typed}");
        assert!(hinted >= 4.5, "hint {hinted}");
        assert!(frame >= 3.0, "resting frame {frame}");
        focus_field(&mut app, s.name);
        let ring = contrast(border_colour(&app, name), panel_fill);
        assert!(ring >= 3.0, "focused frame {ring}");
        let cursor = *app
            .world()
            .get::<TextCursorStyle>(name)
            .expect("a cursor style");
        assert!(contrast(cursor.color, fill) >= 3.0, "caret");
        assert!(contrast(cursor.selection_color, fill) >= 3.0, "selection");
        let selected = cursor
            .selected_text_color
            .expect("selected text has a colour");
        assert!(
            contrast(selected, cursor.selection_color) >= 4.5,
            "selected text"
        );
        assert_eq!(cursor.unfocused_selection_color, Color::NONE);
    }
}

#[test]
fn typing_a_long_line_moves_nothing_at_any_scale() {
    for (width, height, scale) in [(1280, 720, 1.0), (2560, 1440, 1.0), (1280, 720, 1.5)] {
        let mut app = app_sized(width, height);
        app.insert_resource(GooeySkin {
            text_scale: scale,
            ..default()
        });
        let s = settings(&mut app);
        let watched = [s.card, s.name, s.seed, field(&app, s.seed), s.back];
        let before: Vec<Rect> = watched.iter().map(|&e| rect(&app, e)).collect();
        focus_field(&mut app, s.seed);
        type_str(&mut app, &"0123456789".repeat(6));
        assert_eq!(text(&app, s.seed).len(), 60);
        let after: Vec<Rect> = watched.iter().map(|&e| rect(&app, e)).collect();
        for ((b, a), e) in before.iter().zip(&after).zip(watched) {
            assert!(
                (b.min - a.min).abs().max_element() < 0.5
                    && (b.size() - a.size()).abs().max_element() < 0.5,
                "{e:?} moved at {width}x{height} text {scale}: {b:?} -> {a:?}"
            );
        }
    }
}

#[test]
fn a_line_that_fills_a_limit_of_sixteen_shows_whole_with_its_caret() {
    // Bevy keeps one character of room after the caret for the next one: a field only 16
    // characters wide scrolled the first letter of a full 16-character name away.
    for (width, height, scale) in [(1280, 720, 1.0), (2560, 1440, 1.0), (1280, 720, 1.5)] {
        let mut app = app_sized(width, height);
        app.insert_resource(GooeySkin {
            text_scale: scale,
            ..default()
        });
        let s = settings(&mut app);
        app.world_mut().entity_mut(s.seed).insert(TextLimit(16));
        focus_field(&mut app, s.seed);
        type_str(&mut app, "0000000000000000");
        assert_eq!(text(&app, s.seed).len(), 16);
        let field = field(&app, s.seed);
        let offset = app
            .world()
            .get::<EditableText>(field)
            .expect("a field")
            .viewport
            .offset
            .x;
        assert!(
            offset.abs() < 0.5,
            "the first letter scrolled away by {offset} px at {width}x{height}, text {scale}"
        );
    }
}

#[test]
fn nothing_is_written_while_idle_or_while_the_caret_blinks() {
    let mut app = app();
    let s = settings(&mut app);
    for _ in 0..30 {
        app.update();
    }
    let at_rest = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, at_rest, "idle");
    focus_field(&mut app, s.name);
    type_str(&mut app, "abc");
    let typing = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, typing, "focused, idle");
}

#[test]
fn a_tooltip_on_the_row_opens_while_its_field_has_visible_focus() {
    let mut app = app();
    let s = settings(&mut app);
    app.world_mut()
        .entity_mut(s.seed)
        .insert(Tooltip::new("A number, or leave it blank"));
    settle(&mut app);
    let seed = s.seed;
    let open = |app: &mut App| {
        let mut boxes = app.world_mut().query::<(&ChildOf, &Node, &TooltipBox)>();
        boxes
            .iter(app.world())
            .find(|(owner, ..)| owner.parent() == seed)
            .map(|(_, node, _)| node.display != Display::None)
            .expect("a tooltip box on the row")
    };
    assert!(!open(&mut app), "closed at rest");
    focus_field(&mut app, s.seed);
    assert!(open(&mut app), "open while the field has shown focus");
    focus(&mut app, s.back);
    assert!(!open(&mut app), "closed when focus leaves");
}

#[test]
fn every_change_names_the_row_not_the_field() {
    let mut app = app();
    let s = settings(&mut app);
    focus_field(&mut app, s.name);
    type_str(&mut app, "ab");
    enter(&mut app);
    settle(&mut app);
    let seen = &app.world().resource::<Changes>().0;
    assert_eq!(seen.len(), 3);
    assert!(seen.iter().all(|(source, ..)| *source == s.name));
}
