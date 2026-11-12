//! The code editor (G-71), laid out for real in a headless window and driven by real keyboard,
//! pointer and wheel input on a clock the test sets: typing, selecting by word and line, undo and
//! redo with the host's value read after each step, a paste as one step, CRLF kept, a tab drawn at
//! its width, a caret that travels and blinks, a view that glides, and a 20,000-line file that costs
//! what a 20-line one does.

use std::time::{Duration, Instant};

use bevy::camera::NormalizedRenderTarget;
use bevy::clipboard::Clipboard;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<String>` the host heard: the editor, the whole text and whether it was final.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, String, bool)>);

const WIDTH: f32 = 800.0;
/// The host column's height: twenty lines of the dark skin's 14 px... at the default 16 px text.
const HEIGHT: f32 = 480.0;

fn app_with(motion: MotionSettings) -> App {
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
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )))
    .init_resource::<Changes>()
    .add_observer(
        |change: On<ValueChange<String>>, mut seen: ResMut<Changes>| {
            seen.0
                .push((change.source, change.value.clone(), change.is_final));
        },
    );
    app.world_mut().spawn(Camera2d);
    app
}

/// Motion off: the view and the caret land at once, so a test reads where they are.
fn app() -> App {
    app_with(MotionSettings::off())
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// An editor of `text` in a host column of a known size, focused.
fn editor_of(app: &mut App, text: &str) -> Entity {
    let world = app.world_mut();
    let editor = world.spawn(code_editor("Script", text)).id();
    let host = world
        .spawn(Node {
            width: Val::Px(WIDTH),
            height: Val::Px(HEIGHT),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .add_child(editor)
        .id();
    world.spawn(screen()).add_child(host);
    settle(app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    settle(app);
    editor
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn key_event(
    app: &mut App,
    key_code: KeyCode,
    logical_key: Key,
    text: Option<&str>,
    state: ButtonState,
) {
    let window = window(app);
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key,
        state,
        text: text.map(Into::into),
        repeat: false,
        window,
    });
    app.update();
}

fn tap_named(app: &mut App, key_code: KeyCode, logical_key: Key) {
    key_event(
        app,
        key_code,
        logical_key.clone(),
        None,
        ButtonState::Pressed,
    );
    key_event(app, key_code, logical_key, None, ButtonState::Released);
}

fn type_str(app: &mut App, text: &str) {
    for c in text.chars() {
        let s = c.to_string();
        let key = if c == ' ' {
            KeyCode::Space
        } else {
            KeyCode::KeyA
        };
        key_event(
            app,
            key,
            Key::Character(s.as_str().into()),
            Some(&s),
            ButtonState::Pressed,
        );
        key_event(
            app,
            key,
            Key::Character(s.as_str().into()),
            None,
            ButtonState::Released,
        );
    }
    settle(app);
}

/// A key with the modifier keys held.
fn chord(app: &mut App, mods: &[KeyCode], key_code: KeyCode, logical_key: Key) {
    for &m in mods {
        key_event(
            app,
            m,
            Key::Unidentified(NativeKey::Unidentified),
            None,
            ButtonState::Pressed,
        );
    }
    key_event(
        app,
        key_code,
        logical_key.clone(),
        None,
        ButtonState::Pressed,
    );
    key_event(app, key_code, logical_key, None, ButtonState::Released);
    for &m in mods.iter().rev() {
        key_event(
            app,
            m,
            Key::Unidentified(NativeKey::Unidentified),
            None,
            ButtonState::Released,
        );
    }
    settle(app);
}

const CTRL: KeyCode = KeyCode::ControlLeft;
const SHIFT: KeyCode = KeyCode::ShiftLeft;

fn ctrl(app: &mut App, letter: KeyCode, name: &str) {
    chord(app, &[CTRL], letter, Key::Character(name.into()));
}

fn enter(app: &mut App) {
    tap_named(app, KeyCode::Enter, Key::Enter);
    settle(app);
}

fn arrow(app: &mut App, mods: &[KeyCode], key_code: KeyCode) {
    let key = match key_code {
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::ArrowUp => Key::ArrowUp,
        _ => Key::ArrowDown,
    };
    chord(app, mods, key_code, key);
}

fn text_of(app: &App, editor: Entity) -> String {
    app.world()
        .get::<CodeEditorValue>(editor)
        .expect("a value")
        .text()
        .to_owned()
}

fn state(app: &App, editor: Entity) -> &CodeEditor {
    app.world().get::<CodeEditor>(editor).expect("an editor")
}

fn selected(app: &App, editor: Entity) -> String {
    state(app, editor).buffer().selected_text()
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

/// The text rows, in line order: the visible `CodeText` nodes (the hidden one measures a column).
fn rows(app: &App, root: Entity) -> Vec<Entity> {
    let world = app.world();
    let mut found: Vec<(f32, Entity)> = descendants(world, root)
        .into_iter()
        .filter(|&e| world.get::<Role>(e) == Some(&Role::CodeText))
        .filter(|&e| world.get::<Visibility>(e) != Some(&Visibility::Hidden))
        .filter(|&e| {
            world
                .get::<Node>(e)
                .is_some_and(|n| n.display != Display::None)
        })
        .map(|e| (rect(app, e).min.y, e))
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().map(|(_, e)| e).collect()
}

fn row_texts(app: &App, root: Entity) -> Vec<String> {
    rows(app, root)
        .into_iter()
        .map(|e| app.world().get::<Text>(e).expect("text").0.clone())
        .collect()
}

fn part(app: &App, root: Entity, role: Role) -> Vec<Entity> {
    descendants(app.world(), root)
        .into_iter()
        .filter(|&e| app.world().get::<Role>(e) == Some(&role))
        .collect()
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

/// One column of the face, in px: the hidden probe's width over its ten digits.
fn advance(app: &App, root: Entity) -> f32 {
    let probe = descendants(app.world(), root)
        .into_iter()
        .find(|&e| {
            app.world().get::<Role>(e) == Some(&Role::CodeText)
                && app.world().get::<Visibility>(e) == Some(&Visibility::Hidden)
        })
        .expect("the measuring node");
    rect(app, probe).width() / 10.0
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

/// A primary click at `at`, pressed and released.
fn click(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
}

/// Where the pointer must be to land on column `col` of the row `row` of the view.
fn on_text(app: &App, root: Entity, row: usize, col: usize) -> Vec2 {
    let line = rect(app, rows(app, root)[row]);
    Vec2::new(
        line.min.x + (col as f32 + 0.1) * advance(app, root),
        line.center().y,
    )
}

fn wheel(app: &mut App, over: Vec2, y: f32, unit: MouseScrollUnit) {
    pointer(app, over, PointerAction::Move { delta: Vec2::ZERO });
    let window = window(app);
    app.world_mut().write_message(MouseWheel {
        unit,
        x: 0.0,
        y,
        window,
        phase: bevy::input::touch::TouchPhase::Moved,
    });
    app.update();
}

fn lines(n: usize) -> String {
    (0..n)
        .map(|i| format!("line number {i} of the file\n"))
        .collect()
}

// -- the acceptance: typing, selecting by word and line, undo and redo, the value after each step --

#[test]
fn typing_selecting_by_word_and_line_undo_and_redo_each_read_in_the_value() {
    let mut app = app();
    let editor = editor_of(&mut app, "");
    type_str(&mut app, "let speed = 3;");
    assert_eq!(text_of(&app, editor), "let speed = 3;");
    enter(&mut app);
    type_str(&mut app, "let jump = 9;");
    assert_eq!(text_of(&app, editor), "let speed = 3;\nlet jump = 9;");

    // Select by word: Ctrl+Shift+Left takes "9;" back to the start of "9".
    chord(&mut app, &[CTRL, SHIFT], KeyCode::ArrowLeft, Key::ArrowLeft);
    assert_eq!(selected(&app, editor), ";");
    chord(&mut app, &[CTRL, SHIFT], KeyCode::ArrowLeft, Key::ArrowLeft);
    assert_eq!(selected(&app, editor), "9;");
    chord(&mut app, &[CTRL, SHIFT], KeyCode::ArrowLeft, Key::ArrowLeft);
    assert_eq!(selected(&app, editor), "= 9;");

    // Typing over a selection replaces it, and is undone as a step.
    type_str(&mut app, "!");
    assert_eq!(text_of(&app, editor), "let speed = 3;\nlet jump !");
    ctrl(&mut app, KeyCode::KeyZ, "z");
    assert_eq!(text_of(&app, editor), "let speed = 3;\nlet jump = 9;");
    assert_eq!(selected(&app, editor), "= 9;");
    ctrl(&mut app, KeyCode::KeyY, "y");
    assert_eq!(text_of(&app, editor), "let speed = 3;\nlet jump !");

    // Back to the typed line, then undo takes it back a run at a time: what was typed groups by
    // class (letters and digits, blanks, anything else), so `;`, `9`, a blank, a blank, `=`, ...
    ctrl(&mut app, KeyCode::KeyZ, "z");
    assert_eq!(text_of(&app, editor), "let speed = 3;\nlet jump = 9;");
    for want in [
        "let speed = 3;\nlet jump = 9",
        "let speed = 3;\nlet jump = ",
        "let speed = 3;\nlet jump =",
        "let speed = 3;\nlet jump ",
        "let speed = 3;\nlet jump",
        "let speed = 3;\nlet ",
        "let speed = 3;\nlet",
    ] {
        ctrl(&mut app, KeyCode::KeyZ, "z");
        assert_eq!(text_of(&app, editor), want);
    }
    // The line break is a step of its own, and every step redoes (Shift+Ctrl+Z as well).
    ctrl(&mut app, KeyCode::KeyZ, "z");
    assert_eq!(text_of(&app, editor), "let speed = 3;\n");
    ctrl(&mut app, KeyCode::KeyZ, "z");
    assert_eq!(text_of(&app, editor), "let speed = 3;");
    chord(
        &mut app,
        &[CTRL, SHIFT],
        KeyCode::KeyZ,
        Key::Character("z".into()),
    );
    assert_eq!(text_of(&app, editor), "let speed = 3;\n");
    ctrl(&mut app, KeyCode::KeyY, "y");
    assert_eq!(text_of(&app, editor), "let speed = 3;\nlet");
}

#[test]
fn a_press_selects_a_word_on_two_and_a_line_on_three() {
    let mut app = app();
    let editor = editor_of(&mut app, "alpha beta\ngamma delta\nthird");
    // One click puts the caret there.
    let at = on_text(&app, editor, 0, 7);
    click(&mut app, at);
    assert_eq!(state(&app, editor).buffer().caret(), CodePos::new(0, 7));
    assert!(selected(&app, editor).is_empty());
    // Two quick clicks select the word under the pointer.
    click(&mut app, at);
    assert_eq!(selected(&app, editor), "beta");
    // The third selects the line.
    click(&mut app, at);
    assert_eq!(selected(&app, editor), "alpha beta\n");
    // A click elsewhere, after the double-click time, is a single again.
    for _ in 0..40 {
        app.update();
    }
    let elsewhere = on_text(&app, editor, 1, 1);
    click(&mut app, elsewhere);
    assert!(selected(&app, editor).is_empty());
    assert_eq!(state(&app, editor).buffer().caret(), CodePos::new(1, 1));
}

#[test]
fn a_drag_selects_and_shift_arrows_extend_by_character_and_line() {
    let mut app = app();
    let editor = editor_of(&mut app, "one two\nthree four\nfive");
    let from = on_text(&app, editor, 0, 4);
    let to = on_text(&app, editor, 1, 5);
    pointer(&mut app, from, PointerAction::Move { delta: Vec2::ZERO });
    pointer(&mut app, from, PointerAction::Press(PointerButton::Primary));
    pointer(&mut app, to, PointerAction::Move { delta: to - from });
    pointer(&mut app, to, PointerAction::Move { delta: Vec2::ZERO });
    settle(&mut app);
    pointer(&mut app, to, PointerAction::Release(PointerButton::Primary));
    settle(&mut app);
    assert_eq!(selected(&app, editor), "two\nthree");
    arrow(&mut app, &[SHIFT], KeyCode::ArrowDown);
    assert_eq!(selected(&app, editor), "two\nthree four\nfive");
    arrow(&mut app, &[], KeyCode::ArrowLeft);
    // A plain arrow over a selection collapses it to that end.
    assert!(selected(&app, editor).is_empty());
    assert_eq!(state(&app, editor).buffer().caret(), CodePos::new(0, 4));
}

// -- the value and the events --

#[test]
fn every_edit_is_one_value_change_with_the_whole_text_and_save_is_final() {
    let mut app = app();
    let editor = editor_of(&mut app, "ab");
    arrow(&mut app, &[], KeyCode::ArrowRight);
    arrow(&mut app, &[], KeyCode::ArrowRight);
    app.world_mut().resource_mut::<Changes>().0.clear();
    type_str(&mut app, "c");
    type_str(&mut app, "d");
    let seen = app.world().resource::<Changes>().0.clone();
    assert_eq!(
        seen,
        vec![
            (editor, "abc".to_owned(), false),
            (editor, "abcd".to_owned(), false)
        ],
        "one change for each edit, with the whole text, none final"
    );
    // A caret move is not a change.
    arrow(&mut app, &[], KeyCode::ArrowLeft);
    assert_eq!(app.world().resource::<Changes>().0.len(), 2);
    // Ctrl+S is one final change, with the text.
    ctrl(&mut app, KeyCode::KeyS, "s");
    let seen = app.world().resource::<Changes>().0.clone();
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[2], (editor, "abcd".to_owned(), true));
    // The revision moved once per edit and not for the save.
    assert_eq!(
        app.world()
            .get::<CodeEditorValue>(editor)
            .expect("v")
            .revision(),
        2
    );
}

#[test]
fn the_host_replaces_the_text_with_set_and_the_editor_shows_it_with_no_event() {
    let mut app = app();
    let editor = editor_of(&mut app, "old text");
    type_str(&mut app, "x");
    app.world_mut().resource_mut::<Changes>().0.clear();
    app.world_mut()
        .get_mut::<CodeEditorValue>(editor)
        .expect("value")
        .set("brand new\nsecond");
    settle(&mut app);
    assert_eq!(state(&app, editor).buffer().text(), "brand new\nsecond");
    assert_eq!(row_texts(&app, editor)[0], "brand new");
    assert!(
        app.world().resource::<Changes>().0.is_empty(),
        "a host's write sends nothing"
    );
    // The history is the old text's: nothing to undo.
    ctrl(&mut app, KeyCode::KeyZ, "z");
    assert_eq!(text_of(&app, editor), "brand new\nsecond");
}

// -- clipboard, indentation, line endings --

#[test]
fn a_paste_is_one_undo_step_and_copy_and_cut_fill_the_clipboard() {
    let mut app = app();
    let editor = editor_of(&mut app, "one two");
    ctrl(&mut app, KeyCode::KeyA, "a");
    assert_eq!(selected(&app, editor), "one two");
    ctrl(&mut app, KeyCode::KeyC, "c");
    arrow(&mut app, &[], KeyCode::ArrowRight);
    // Paste it three times, one step each, over a clipboard of several lines.
    app.world_mut()
        .resource_mut::<Clipboard>()
        .set_text("a\nb\nc")
        .expect("the in-process clipboard");
    ctrl(&mut app, KeyCode::KeyV, "v");
    assert_eq!(text_of(&app, editor), "one twoa\nb\nc");
    ctrl(&mut app, KeyCode::KeyZ, "z");
    assert_eq!(
        text_of(&app, editor),
        "one two",
        "a paste of three lines is one undo step"
    );
    ctrl(&mut app, KeyCode::KeyY, "y");
    assert_eq!(text_of(&app, editor), "one twoa\nb\nc");
    // Cut takes the selection out and puts it on the clipboard.
    ctrl(&mut app, KeyCode::KeyA, "a");
    ctrl(&mut app, KeyCode::KeyX, "x");
    assert_eq!(text_of(&app, editor), "");
    ctrl(&mut app, KeyCode::KeyV, "v");
    assert_eq!(text_of(&app, editor), "one twoa\nb\nc");
}

#[test]
fn crlf_endings_are_kept_through_an_edit_and_a_new_line_takes_them() {
    let mut app = app();
    let editor = editor_of(&mut app, "first\r\nsecond\r\n");
    ctrl(&mut app, KeyCode::KeyA, "a");
    arrow(&mut app, &[], KeyCode::ArrowRight);
    type_str(&mut app, "!");
    assert_eq!(text_of(&app, editor), "first\r\nsecond\r\n!");
    enter(&mut app);
    type_str(&mut app, "?");
    assert_eq!(text_of(&app, editor), "first\r\nsecond\r\n!\r\n?");
}

#[test]
fn tab_indents_shift_tab_outdents_and_a_tab_is_drawn_at_its_configured_width() {
    let mut app = app();
    let editor = editor_of(&mut app, "a\n\tb");
    // A tab character is drawn as the spaces to the next four-column stop.
    assert_eq!(row_texts(&app, editor)[1], "    b");
    app.world_mut().entity_mut(editor).insert(CodeIndentation {
        indent: CodeIndent::Tab,
        tab_width: 8,
    });
    settle(&mut app);
    assert_eq!(
        row_texts(&app, editor)[1],
        "        b",
        "eight columns when the editor is set to eight"
    );
    // Tab types an indent, and the keyboard is not trapped: focus stays and Escape lets go.
    app.world_mut().entity_mut(editor).insert(CodeIndentation {
        indent: CodeIndent::Spaces(2),
        tab_width: 4,
    });
    settle(&mut app);
    tap_named(&mut app, KeyCode::Tab, Key::Tab);
    settle(&mut app);
    assert_eq!(text_of(&app, editor), "  a\n\tb");
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(editor),
        "Tab does not leave"
    );
    chord(&mut app, &[SHIFT], KeyCode::Tab, Key::Tab);
    assert_eq!(text_of(&app, editor), "a\n\tb");
    tap_named(&mut app, KeyCode::Escape, Key::Escape);
    settle(&mut app);
    assert_ne!(
        app.world().resource::<InputFocus>().get(),
        Some(editor),
        "Escape gives focus up"
    );
}

#[test]
fn enter_carries_the_indent_of_the_line() {
    let mut app = app();
    let editor = editor_of(&mut app, "    fn x() {");
    arrow(&mut app, &[CTRL], KeyCode::End);
    tap_named(&mut app, KeyCode::End, Key::End);
    enter(&mut app);
    assert_eq!(text_of(&app, editor), "    fn x() {\n    ");
    type_str(&mut app, "y");
    assert_eq!(text_of(&app, editor), "    fn x() {\n    y");
}

// -- the view --

#[test]
fn the_rows_are_a_pool_so_a_big_file_costs_what_a_small_one_does() {
    let mut small = app();
    let small_editor = editor_of(&mut small, &lines(20));
    let mut big = app();
    let big_editor = editor_of(&mut big, &lines(20_000));
    let count = |app: &App, editor: Entity| descendants(app.world(), editor).len();
    assert_eq!(
        count(&big, big_editor),
        count(&small, small_editor),
        "the same entities for 20 lines and 20,000"
    );
    assert!(row_texts(&big, big_editor)[0].starts_with("line number 0 "));
}

#[test]
fn ctrl_end_scrolls_to_the_last_line_and_it_is_visible() {
    let mut app = app();
    let editor = editor_of(&mut app, &lines(20_000));
    arrow(&mut app, &[CTRL], KeyCode::End);
    settle(&mut app);
    let shown = row_texts(&app, editor);
    assert!(
        shown
            .iter()
            .any(|row| row.starts_with("line number 19999 of")),
        "the last line is drawn: {:?}",
        shown.last()
    );
    // It is inside the editor's box.
    let box_ = rect(&app, editor);
    let last = rows(&app, editor)
        .into_iter()
        .find(|&e| {
            app.world()
                .get::<Text>(e)
                .is_some_and(|t| t.0.starts_with("line number 19999 of"))
        })
        .expect("the last line's row");
    assert!(
        rect(&app, last).max.y <= box_.max.y + 0.5,
        "the last line is in the view"
    );
    assert_eq!(state(&app, editor).buffer().caret().line, 20_000);
}

#[test]
fn the_wheel_scrolls_and_the_view_glides_there_on_the_clock() {
    let mut app = app_with(MotionSettings::default());
    let editor = editor_of(&mut app, &lines(500));
    for _ in 0..60 {
        app.update();
    }
    let over = rect(&app, editor).center();
    let before = state(&app, editor).scroll().y;
    wheel(&mut app, over, -10.0, MouseScrollUnit::Line);
    let one_frame = state(&app, editor).scroll().y;
    for _ in 0..90 {
        app.update();
    }
    let rested = state(&app, editor).scroll().y;
    assert!(
        rested > before + 100.0,
        "ten lines of wheel moved the text: {before} to {rested}"
    );
    assert!(
        one_frame < rested - 1.0,
        "it glides: a frame in it had gone {one_frame} of {rested}"
    );
    // The rows are the lines in view.
    let first = state(&app, editor).visible_lines().start;
    assert!(row_texts(&app, editor)[0].starts_with(&format!("line number {first} of")));
}

#[test]
fn the_caret_travels_on_a_spring_blinks_and_lands_at_once_under_reduced_motion() {
    let mut app = app_with(MotionSettings::default());
    let editor = editor_of(&mut app, "abcdefghij");
    for _ in 0..90 {
        app.update();
    }
    let start = state(&app, editor).caret_at();
    for _ in 0..8 {
        arrow(&mut app, &[], KeyCode::ArrowRight);
    }
    // `arrow` settles, so it has landed by now; type far to see it move: a jump of one key.
    let _ = start;
    let mut mid = None;
    tap_named(&mut app, KeyCode::End, Key::End);
    for frame in 0..60 {
        app.update();
        let now = state(&app, editor).caret_at();
        if frame == 1 {
            mid = Some(now);
        }
    }
    let end = state(&app, editor).caret_at();
    let mid = mid.expect("a frame in");
    assert!(
        mid.x < end.x,
        "the caret was still on its way: {mid} of {end}"
    );
    // It blinks: lit and dark both show over a second.
    let caret = part(&app, editor, Role::CodeCaret)[0];
    let (mut lit, mut dark) = (false, false);
    for _ in 0..90 {
        app.update();
        match app.world().get::<Visibility>(caret) {
            Some(Visibility::Hidden) => dark = true,
            _ => lit = true,
        }
    }
    assert!(lit && dark, "the caret blinks: lit {lit}, dark {dark}");
    // Typing restarts the blink: lit in the frame of a key.
    type_str(&mut app, "x");
    assert_ne!(
        app.world().get::<Visibility>(caret),
        Some(&Visibility::Hidden)
    );
}

#[test]
fn under_reduced_motion_the_caret_is_steady_and_everything_lands_at_once() {
    let mut app = app_with(MotionSettings::reduced());
    let editor = editor_of(&mut app, &lines(300));
    let caret = part(&app, editor, Role::CodeCaret)[0];
    for _ in 0..90 {
        app.update();
        assert_ne!(
            app.world().get::<Visibility>(caret),
            Some(&Visibility::Hidden),
            "steady"
        );
    }
    arrow(&mut app, &[CTRL], KeyCode::End);
    let shown = state(&app, editor).scroll().y;
    assert!(shown > 1000.0, "the view is at the end at once: {shown}");
}

#[test]
fn a_ten_thousand_character_line_is_drawn_by_the_columns_in_view_only() {
    let mut app = app();
    let long = "x".repeat(10_000);
    let editor = editor_of(&mut app, &long);
    let cols = row_texts(&app, editor)[0].len();
    assert!(
        cols < 400,
        "a row draws the view's columns, not the line: {cols}"
    );
    tap_named(&mut app, KeyCode::End, Key::End);
    settle(&mut app);
    type_str(&mut app, "y");
    assert_eq!(text_of(&app, editor).len(), 10_001);
    // The view followed the caret to the end of the line.
    let first = row_texts(&app, editor)[0].clone();
    assert!(
        first.ends_with("xy") || first.ends_with('y'),
        "the end of the line is in view"
    );
    assert!(state(&app, editor).scroll().x > 1000.0);
}

#[test]
fn a_20000_line_file_scrolls_within_the_perf_budget_of_a_20_line_one() {
    let mut small = app();
    editor_of(&mut small, &lines(20));
    let mut big = app();
    editor_of(&mut big, &lines(20_000));
    let measure = |app: &mut App| -> f64 {
        let mut times = Vec::new();
        for round in 0..3 {
            for frame in 0..40 {
                // The wheel turns every frame, so the view moves in every one of them.
                let y = if (round + frame / 10) % 2 == 0 {
                    -3.0
                } else {
                    3.0
                };
                let window = window(app);
                app.world_mut().write_message(MouseWheel {
                    unit: MouseScrollUnit::Line,
                    x: 0.0,
                    y,
                    window,
                    phase: bevy::input::touch::TouchPhase::Moved,
                });
                let began = Instant::now();
                app.update();
                times.push(began.elapsed().as_secs_f64());
            }
        }
        times.sort_by(|a, b| a.total_cmp(b));
        times[times.len() / 2]
    };
    // Warm both, then take the better of three measurements of each (load makes a run slower,
    // never faster).
    measure(&mut small);
    measure(&mut big);
    let small_ms = (0..3).map(|_| measure(&mut small)).fold(f64::MAX, f64::min);
    let big_ms = (0..3).map(|_| measure(&mut big)).fold(f64::MAX, f64::min);
    let ratio = big_ms / small_ms.max(1e-9);
    assert!(
        ratio <= f64::from(PERF_BUDGET),
        "20,000 lines took {:.2} ms a frame to 20 lines' {:.2} ms: {ratio:.2}x, budget {PERF_BUDGET}x",
        big_ms * 1000.0,
        small_ms * 1000.0
    );
}

// -- the look --

#[test]
fn the_selection_is_a_band_per_line_the_current_line_is_lit_and_numbers_follow() {
    let mut app = app();
    let editor = editor_of(&mut app, "short\na longer line here\nend");
    let step = advance(&app, editor);
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(0, 2), CodePos::new(1, 6));
    settle(&mut app);
    let bands: Vec<_> = part(&app, editor, Role::CodeSelection)
        .into_iter()
        .filter(|&e| {
            app.world()
                .get::<Node>(e)
                .is_some_and(|n| n.display != Display::None)
        })
        .map(|e| rect(&app, e))
        .collect();
    assert_eq!(bands.len(), 2, "two lines are selected, a band each");
    // The first band runs from column 2 to the end of "short" and one more (the line break).
    let expected_first = (5 - 2 + 1) as f32 * step;
    let expected_second = 6.0 * step;
    let mut widths: Vec<f32> = bands.iter().map(Rect::width).collect();
    widths.sort_by(|a, b| a.total_cmp(b));
    let mut want = [expected_first, expected_second];
    want.sort_by(|a, b| a.total_cmp(b));
    for (got, want) in widths.iter().zip(want) {
        assert!((got - want).abs() < 1.5, "band of {got} px, wanted {want}");
    }
    // The line numbers are in the gutter, the caret's line's number in the lit role.
    let numbers =
        part(&app, editor, Role::CodeNumber).len() + part(&app, editor, Role::CodeNumberLit).len();
    assert!(numbers >= 3);
    assert_eq!(part(&app, editor, Role::CodeNumberLit).len(), 1);
    let lit = part(&app, editor, Role::CodeNumberLit)[0];
    assert_eq!(app.world().get::<Text>(lit).expect("text").0, "2");
    let line = part(&app, editor, Role::CodeLineLit)[0];
    let band = rect(&app, line);
    let caret_row = rect(&app, rows(&app, editor)[1]);
    assert!(
        (band.center().y - caret_row.center().y).abs() < 1.0,
        "the current line's band is behind it"
    );
}

#[test]
fn the_text_is_drawn_in_the_monospace_face_and_colours_follow_the_skin() {
    let mut app = app();
    let editor = editor_of(&mut app, "text");
    let row = rows(&app, editor)[0];
    let skin = app.world().resource::<GooeySkin>().clone();
    assert_eq!(
        app.world().get::<TextColor>(row).expect("colour").0,
        skin.text
    );
    let caret = part(&app, editor, Role::CodeCaret)[0];
    assert_eq!(
        app.world().get::<BackgroundColor>(caret).expect("fill").0,
        skin.text
    );
    let number = part(&app, editor, Role::CodeNumber)[0];
    assert_eq!(
        app.world().get::<TextColor>(number).expect("colour").0,
        skin.text_muted
    );
}

#[test]
fn the_selection_keeps_the_text_readable_in_every_preset() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let rgb = |c: Color| {
            let c = c.to_srgba();
            [c.red, c.green, c.blue]
        };
        let ground = rgb(skin.solid_surface());
        let selected = composite(rgb(skin.accent), CODE_SELECTION_ALPHA, ground);
        let ratio = contrast_ratio(rgb(skin.text), selected);
        assert!(
            ratio >= model::TEXT_CONTRAST,
            "{}: text on the selection is {ratio:.2}:1",
            preset.name()
        );
    }
}
