//! The text area: a text input several lines high. Enter adds a line, Ctrl+Enter or Cmd+Enter ends
//! the edit, Up and Down are the caret's, the paste respects `TextLimit`, and the field never grows
//! while the player types. Laid out for real in a headless window, driven by real keyboard input.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::text::{EditableText, TextEdit};
use bevy::ui::{InteractionDisabled, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<String>` a game would see: the row, the text and whether the edit ended.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, String, bool)>);

/// How many frames' writes the app has seen to what a field's look and state change.
#[derive(Resource, Default)]
struct Writes(u32);

type Written = Or<(
    Changed<Text>,
    Changed<TextInputValue>,
    Changed<BackgroundColor>,
    Changed<BorderColor>,
    Changed<Node>,
    Changed<TextColor>,
)>;

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
    .init_resource::<Changes>()
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

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
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

fn focus_on(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    settle(app);
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn value(app: &App, row: Entity) -> String {
    app.world()
        .get::<TextInputValue>(row)
        .expect("a text row")
        .0
        .clone()
}

fn changes(app: &App) -> Vec<(Entity, String, bool)> {
    app.world().resource::<Changes>().0.clone()
}

/// A modifier held (or let go) in its own frame, so it is down when the next key is read.
fn modifier(app: &mut App, key_code: KeyCode, logical: Key, state: ButtonState) {
    let window = window(app);
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: logical,
        state,
        text: None,
        repeat: false,
        window,
    });
    app.update();
}

fn chord_enter(app: &mut App, key_code: KeyCode, logical: Key) {
    modifier(app, key_code, logical.clone(), ButtonState::Pressed);
    enter(app);
    modifier(app, key_code, logical, ButtonState::Released);
    settle(app);
}

/// A panel of a toggle, a three-row notes area, a five-row one and a Back button.
struct Form {
    above: Entity,
    notes: Entity,
    tall: Entity,
    back: Entity,
}

fn form(app: &mut App) -> Form {
    let world = app.world_mut();
    let above = world.spawn(toggle("Subtitles", true)).id();
    let notes = world.spawn(text_area("Notes", "", 3)).id();
    let tall = world.spawn(text_area("Story", "", 5)).id();
    let back = world.spawn(button("Back")).id();
    let card = world
        .spawn(panel())
        .add_children(&[above, notes, tall, back])
        .id();
    world.spawn(screen()).add_child(card);
    settle(app);
    Form {
        above,
        notes,
        tall,
        back,
    }
}

#[test]
fn the_field_is_as_many_lines_high_as_asked_and_the_label_is_over_it() {
    let mut app = app();
    let f = form(&mut app);
    let h3 = rect(&app, field(&app, f.notes)).height();
    let h5 = rect(&app, field(&app, f.tall)).height();

    let line = (h5 - h3) / 2.0;
    assert!(line > 8.0, "a line is {line} px");
    assert!(h3 > 3.0 * line, "three lines and a frame: {h3} px");
    let label = part(&app, f.notes, Role::ControlText);
    assert!(
        rect(&app, label).max.y <= rect(&app, field(&app, f.notes)).min.y + 1.0,
        "the label stands over the field"
    );
}

#[test]
fn enter_adds_a_line_and_is_no_final_change() {
    let mut app = app();
    let f = form(&mut app);
    let notes_field = field(&app, f.notes);
    focus_on(&mut app, notes_field);

    type_str(&mut app, "ab");
    enter(&mut app);
    type_str(&mut app, "cd");

    assert_eq!(value(&app, f.notes), "ab\ncd");
    let seen = changes(&app);
    assert!(!seen.is_empty());
    assert!(seen.iter().all(|(row, _, last)| *row == f.notes && !last));
}

#[test]
fn ctrl_enter_and_cmd_enter_end_the_edit_with_one_final_change() {
    for (key_code, logical) in [
        (KeyCode::ControlLeft, Key::Control),
        (KeyCode::SuperLeft, Key::Super),
    ] {
        let mut app = app();
        let f = form(&mut app);
        let notes_field = field(&app, f.notes);
        focus_on(&mut app, notes_field);
        type_str(&mut app, "hello");
        enter(&mut app);
        type_str(&mut app, "world");
        let before = changes(&app).len();

        chord_enter(&mut app, key_code, logical);

        let seen = changes(&app);
        assert_eq!(seen.len(), before + 1, "one change for the chord");
        let last = seen.last().expect("a change");
        assert_eq!(last, &(f.notes, "hello\nworld".to_owned(), true));
        assert_eq!(value(&app, f.notes), "hello\nworld", "no line added");
        assert_eq!(
            focused(&app),
            Some(field(&app, f.notes)),
            "the field is kept"
        );
    }
}

#[test]
fn up_and_down_move_the_caret_between_lines_and_never_focus() {
    let mut app = app();
    let f = form(&mut app);
    let notes_field = field(&app, f.notes);
    focus_on(&mut app, notes_field);
    type_str(&mut app, "a");
    enter(&mut app);
    type_str(&mut app, "b");

    arrow(&mut app, KeyCode::ArrowUp);
    assert_eq!(focused(&app), Some(notes_field), "Up moved focus");
    type_str(&mut app, "X");
    assert_eq!(value(&app, f.notes), "aX\nb", "the caret went up a line");

    arrow(&mut app, KeyCode::ArrowDown);
    arrow(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(notes_field), "Down moved focus");
}

#[test]
fn tab_and_escape_leave_the_field() {
    let mut app = app();
    let f = form(&mut app);
    let notes_field = field(&app, f.notes);
    focus_on(&mut app, notes_field);

    tap(&mut app, KeyCode::Tab);
    settle(&mut app);
    assert_ne!(focused(&app), Some(notes_field), "Tab never traps");
    assert!(focused(&app).is_some());

    focus_on(&mut app, notes_field);
    escape(&mut app);
    assert_ne!(focused(&app), Some(notes_field), "Escape blurs");
}

#[test]
fn a_huge_paste_never_passes_the_limit_and_a_small_one_is_taken() {
    let mut app = app();
    let f = form(&mut app);
    app.world_mut().entity_mut(f.notes).insert(TextLimit(100));
    let notes_field = field(&app, f.notes);
    focus_on(&mut app, notes_field);
    let paste = |app: &mut App, text: String| {
        app.world_mut()
            .get_mut::<EditableText>(notes_field)
            .expect("a field")
            .queue_edit(TextEdit::Insert(text.into()));
        settle(app);
    };

    paste(&mut app, "x\n".repeat(5000));
    assert_eq!(
        value(&app, f.notes).chars().count(),
        0,
        "Bevy ignores an edit that would pass the limit"
    );

    paste(&mut app, "ok\n".repeat(10));
    assert_eq!(
        value(&app, f.notes).chars().count(),
        30,
        "a paste that fits is taken"
    );

    paste(&mut app, "y".repeat(71));
    assert_eq!(
        value(&app, f.notes).chars().count(),
        30,
        "30 and 71 pass 100"
    );
}

#[test]
fn the_field_never_grows_however_much_is_typed() {
    let mut app = app();
    let f = form(&mut app);
    let notes_field = field(&app, f.notes);
    let before = rect(&app, notes_field);
    let card_before = rect(&app, f.back);
    focus_on(&mut app, notes_field);

    for line in 0..12 {
        type_str(
            &mut app,
            &format!("line {line} with some words in it that go on"),
        );
        enter(&mut app);
    }
    settle(&mut app);

    let after = rect(&app, notes_field);
    assert!(
        (after.height() - before.height()).abs() < 0.5
            && (after.width() - before.width()).abs() < 0.5,
        "{before:?} became {after:?}"
    );
    assert!(
        (rect(&app, f.back).min - card_before.min).length() < 0.5,
        "nothing moved"
    );
}

#[test]
fn a_long_line_wraps_inside_the_field() {
    let mut app = app();
    let f = form(&mut app);
    let notes_field = field(&app, f.notes);
    focus_on(&mut app, notes_field);
    let width = rect(&app, notes_field).width();

    type_str(&mut app, &"word ".repeat(60));

    let editable = app
        .world()
        .get::<EditableText>(notes_field)
        .expect("a field");
    assert!(
        editable.editor().raw_text().len() > 200 && rect(&app, notes_field).width() == width,
        "typing a long line did not widen the field"
    );
}

#[test]
fn the_arrow_keys_stay_in_the_area_and_the_d_pad_leaves_it() {
    use bevy::input::gamepad::{
        GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
    };
    let mut app = app();
    let f = form(&mut app);
    let notes_field = field(&app, f.notes);
    let tall_field = field(&app, f.tall);
    focus_on(&mut app, notes_field);

    arrow(&mut app, KeyCode::ArrowLeft);
    arrow(&mut app, KeyCode::ArrowDown);
    arrow(&mut app, KeyCode::ArrowUp);
    assert_eq!(
        focused(&app),
        Some(notes_field),
        "arrow keys are the caret's"
    );

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
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad,
                GamepadButton::DPadDown,
                value,
            )));
        app.update();
    }
    assert_eq!(
        focused(&app),
        Some(tall_field),
        "a pad has no caret to move"
    );
    let _ = (f.above, f.back);
}

#[test]
fn a_disabled_area_takes_nothing() {
    let mut app = app();
    let f = form(&mut app);
    app.world_mut()
        .entity_mut(f.notes)
        .insert(InteractionDisabled);
    settle(&mut app);
    let notes_field = field(&app, f.notes);
    focus_on(&mut app, notes_field);

    type_str(&mut app, "nope");
    enter(&mut app);

    assert_eq!(value(&app, f.notes), "");
    assert!(changes(&app).is_empty());
}

#[test]
fn at_200_percent_text_nothing_leaves_the_panel() {
    let mut app = app();
    let mut skin = GooeySkin::dark();
    skin.text_scale = 2.0;
    app.insert_resource(skin);
    let world = app.world_mut();
    let notes = world.spawn(text_area("Notes", "", 4)).id();
    let card = world.spawn(panel()).add_child(notes).id();
    world.spawn(screen()).add_child(card);
    settle(&mut app);

    let card_rect = rect(&app, card);
    for part in [notes, field(&app, notes)] {
        let r = rect(&app, part);
        assert!(
            r.min.x >= card_rect.min.x - 1.0
                && r.max.x <= card_rect.max.x + 1.0
                && r.min.y >= card_rect.min.y - 1.0
                && r.max.y <= card_rect.max.y + 1.0,
            "{r:?} leaves {card_rect:?}"
        );
    }
}

#[test]
fn nothing_is_written_while_idle() {
    let mut app = app();
    let f = form(&mut app);
    let notes_field = field(&app, f.notes);
    focus_on(&mut app, notes_field);
    type_str(&mut app, "a");
    settle(&mut app);
    let before = app.world().resource::<Writes>().0;

    for _ in 0..30 {
        app.update();
    }

    assert_eq!(app.world().resource::<Writes>().0, before);
}
