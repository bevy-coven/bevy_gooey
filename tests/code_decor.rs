//! What a host draws onto the code editor (G-72), headless and driven by real keys on a clock the
//! test sets: syntax colours read from the glyph runs, edits from elsewhere that keep the player's
//! caret and fade, completion chosen by typing and Enter, and diagnostics with their note.

use std::ops::Range;
use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::text::TextSpan;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

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
    )));
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn editor_of(app: &mut App, text: &str) -> Entity {
    let world = app.world_mut();
    let editor = world.spawn(code_editor("Script", text)).id();
    let host = world
        .spawn(Node {
            width: Val::Px(800.0),
            height: Val::Px(480.0),
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

fn key(app: &mut App, key_code: KeyCode, logical_key: Key, text: Option<&str>, state: ButtonState) {
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

fn tap(app: &mut App, key_code: KeyCode, logical_key: Key) {
    key(
        app,
        key_code,
        logical_key.clone(),
        None,
        ButtonState::Pressed,
    );
    key(app, key_code, logical_key, None, ButtonState::Released);
    settle(app);
}

fn type_str(app: &mut App, text: &str) {
    for c in text.chars() {
        let s = c.to_string();
        key(
            app,
            KeyCode::KeyA,
            Key::Character(s.as_str().into()),
            Some(&s),
            ButtonState::Pressed,
        );
        key(
            app,
            KeyCode::KeyA,
            Key::Character(s.as_str().into()),
            None,
            ButtonState::Released,
        );
    }
    settle(app);
}

fn chord(app: &mut App, modifier: KeyCode, key_code: KeyCode, logical_key: Key) {
    key(
        app,
        modifier,
        Key::Unidentified(NativeKey::Unidentified),
        None,
        ButtonState::Pressed,
    );
    key(
        app,
        key_code,
        logical_key.clone(),
        None,
        ButtonState::Pressed,
    );
    key(app, key_code, logical_key, None, ButtonState::Released);
    key(
        app,
        modifier,
        Key::Unidentified(NativeKey::Unidentified),
        None,
        ButtonState::Released,
    );
    settle(app);
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

fn value(app: &App, editor: Entity) -> &CodeEditorValue {
    app.world().get::<CodeEditorValue>(editor).expect("value")
}

fn shown(app: &App, entity: Entity) -> bool {
    app.world()
        .get::<Node>(entity)
        .is_some_and(|node| node.display != Display::None)
}

/// Every span under the editor that holds words, with its role and colour.
fn spans(app: &App, editor: Entity) -> Vec<(String, Role, Color)> {
    let world = app.world();
    descendants(world, editor)
        .into_iter()
        .filter_map(|e| {
            let text = world.get::<TextSpan>(e)?;
            (!text.0.is_empty()).then(|| {
                (
                    text.0.clone(),
                    *world.get::<Role>(e).expect("a role"),
                    world.get::<TextColor>(e).expect("a colour").0,
                )
            })
        })
        .collect()
}

#[test]
fn every_glyph_run_takes_the_colour_of_its_role() {
    let mut app = app_with(MotionSettings::off());
    let editor = editor_of(&mut app, "name: \"Ada\" # who");
    let revision = value(&app, editor).revision();
    app.world_mut().entity_mut(editor).insert(Highlights::new(
        revision,
        [
            (0..4, SyntaxRole::Key),
            (6..11, SyntaxRole::String),
            (12..17, SyntaxRole::Comment),
        ],
    ));
    settle(&mut app);
    let skin = app.world().resource::<GooeySkin>().clone();
    let runs = spans(&app, editor);
    let find = |text: &str| {
        runs.iter()
            .find(|(t, ..)| t == text)
            .unwrap_or_else(|| panic!("a run {text:?} in {runs:?}"))
            .clone()
    };
    assert_eq!(find("name").2, skin.syntax(SyntaxRole::Key));
    assert_eq!(find("name").1, Role::CodeSyntax(SyntaxRole::Key));
    assert_eq!(find("\"Ada\"").2, skin.syntax(SyntaxRole::String));
    assert_eq!(find("# who").2, skin.syntax(SyntaxRole::Comment));
    assert_eq!(
        find("# who").2,
        skin.text_muted,
        "a comment is the muted text"
    );
    assert_ne!(find("name").2, find("\"Ada\"").2, "roles are told apart");
    // The plain text between them keeps the text colour.
    assert_eq!(find(": ").2, skin.text);
}

#[test]
fn decorations_for_an_old_revision_are_dropped_never_drawn_on_new_text() {
    let mut app = app_with(MotionSettings::off());
    let editor = editor_of(&mut app, "abc def");
    let revision = value(&app, editor).revision();
    app.world_mut()
        .entity_mut(editor)
        .insert(Highlights::new(revision, [(0..3, SyntaxRole::Keyword)]));
    settle(&mut app);
    assert!(!spans(&app, editor).is_empty());
    type_str(&mut app, "x");
    assert_ne!(value(&app, editor).revision(), revision);
    assert!(
        spans(&app, editor).is_empty(),
        "the old spans are gone: {:?}",
        spans(&app, editor)
    );
    // Spans for the new revision are drawn again.
    let now = value(&app, editor).revision();
    app.world_mut()
        .entity_mut(editor)
        .insert(Highlights::new(now, [(0..4, SyntaxRole::Number)]));
    settle(&mut app);
    assert_eq!(
        spans(&app, editor)[0].1,
        Role::CodeSyntax(SyntaxRole::Number)
    );
}

#[test]
fn an_edit_from_elsewhere_keeps_the_caret_on_its_character_and_its_lines_are_tinted_then_fade() {
    let mut app = app_with(MotionSettings::default());
    let editor = editor_of(&mut app, "one\ntwo\nthree");
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(2, 2), CodePos::new(2, 2));
    settle(&mut app);
    let ada = Color::srgb(0.9, 0.4, 0.2);
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .apply_external_edit(
            &[CodeEdit::new(0..0, "zero\n")],
            CodeAuthor::new("Ada", ada),
        )
        .expect("a valid edit");
    app.update();
    app.update();
    let buffer = app
        .world()
        .get::<CodeEditor>(editor)
        .expect("editor")
        .buffer();
    assert_eq!(buffer.text(), "zero\none\ntwo\nthree");
    assert_eq!(
        buffer.caret(),
        CodePos::new(3, 2),
        "the caret is on the same character"
    );
    assert_eq!(
        value(&app, editor).text(),
        "zero\none\ntwo\nthree",
        "the host's copy follows"
    );
    let tint = |app: &App| -> Vec<f32> {
        descendants(app.world(), editor)
            .into_iter()
            .filter(|&e| app.world().get::<Role>(e) == Some(&Role::CodeExternal) && shown(app, e))
            .filter_map(|e| app.world().get::<BackgroundColor>(e).map(|b| b.0.alpha()))
            .collect()
    };
    let first = tint(&app);
    assert_eq!(first.len(), 1, "one changed line is tinted");
    assert!(first[0] > 0.1, "tinted: {first:?}");
    for _ in 0..60 {
        app.update();
    }
    let mid = tint(&app);
    assert!(
        mid[0] < first[0] && mid[0] > 0.0,
        "fading: {first:?} then {mid:?}"
    );
    for _ in 0..140 {
        app.update();
    }
    assert!(tint(&app).is_empty(), "the tint has faded away");
}

#[test]
fn an_edit_inside_the_players_selection_keeps_the_selection_on_the_same_text() {
    let mut app = app_with(MotionSettings::off());
    let editor = editor_of(&mut app, "let value = 10;");
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(0, 4), CodePos::new(0, 9));
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .apply_external_edit(
            &[CodeEdit::new(4..4, "mut ")],
            CodeAuthor::new("Bo", Color::WHITE),
        )
        .expect("a valid edit");
    settle(&mut app);
    let editor_ref = app.world().get::<CodeEditor>(editor).expect("editor");
    assert_eq!(editor_ref.buffer().selected_text(), "value");
}

#[derive(Resource, Default)]
struct Asked(Vec<(String, CompletionTrigger)>);

#[test]
fn completion_is_asked_for_answered_narrowed_by_typing_and_chosen_with_enter() {
    let mut app = app_with(MotionSettings::off());
    app.init_resource::<Asked>();
    app.add_observer(
        |request: On<CodeCompletionRequested>, mut asked: ResMut<Asked>, mut commands: Commands| {
            asked.0.push((request.prefix.clone(), request.trigger));
            commands.entity(request.entity).insert(CodeCompletions {
                anchor: request.offset - request.prefix.len(),
                items: vec![
                    CompletionItem::new("map").detail("a table"),
                    CompletionItem::new("max").detail("the larger"),
                    CompletionItem::new("match").detail("a pattern"),
                    CompletionItem::new("mix").insert("mix()"),
                ],
            });
        },
    );
    let editor = editor_of(&mut app, "");
    type_str(&mut app, "m");
    assert_eq!(
        app.world().resource::<Asked>().0,
        vec![("m".to_owned(), CompletionTrigger::Typing)]
    );
    let labels = |app: &App| -> Vec<String> {
        descendants(app.world(), editor)
            .into_iter()
            .filter(|&e| app.world().get::<Role>(e) == Some(&Role::CodeCompletionLabel))
            .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
            .filter(|t| !t.is_empty())
            .collect()
    };
    assert_eq!(
        labels(&app).len(),
        4,
        "the list is open under the caret: {:?}",
        labels(&app)
    );
    type_str(&mut app, "ax");
    assert_eq!(
        app.world().resource::<Asked>().0.len(),
        1,
        "typing on narrows locally, it asks no more"
    );
    assert_eq!(
        labels(&app),
        vec!["max".to_owned()],
        "narrowed by what was typed"
    );
    tap(&mut app, KeyCode::Enter, Key::Enter);
    assert_eq!(
        value(&app, editor).text(),
        "max",
        "Enter chose it, and wrote no line break"
    );
    assert!(labels(&app).is_empty(), "the list closed");
}

#[test]
fn up_and_down_move_the_lit_row_and_escape_closes_the_list_before_focus_goes() {
    let mut app = app_with(MotionSettings::off());
    app.add_observer(
        |request: On<CodeCompletionRequested>, mut commands: Commands| {
            commands.entity(request.entity).insert(CodeCompletions {
                anchor: request.offset - request.prefix.len(),
                items: vec![
                    CompletionItem::new("alpha"),
                    CompletionItem::new("also"),
                    CompletionItem::new("alto"),
                ],
            });
        },
    );
    let editor = editor_of(&mut app, "");
    type_str(&mut app, "a");
    tap(&mut app, KeyCode::ArrowDown, Key::ArrowDown);
    let lit = |app: &App| -> String {
        let world = app.world();
        let row = descendants(world, editor)
            .into_iter()
            .find(|&e| world.get::<Role>(e) == Some(&Role::CodeCompletionRowLit))
            .expect("a lit row");
        let label = world
            .get::<Children>(row)
            .and_then(|kids| kids.iter().next())
            .expect("its label");
        world.get::<Text>(label).expect("words").0.clone()
    };
    let second = {
        let world = app.world();
        let panel = descendants(world, editor)
            .into_iter()
            .find(|&e| world.get::<Role>(e) == Some(&Role::CodeCompletion))
            .expect("the list");
        let labels: Vec<String> = world
            .get::<Children>(panel)
            .expect("rows")
            .iter()
            .filter_map(|row| {
                let label = world.get::<Children>(row)?.iter().next()?;
                let text = world.get::<Text>(label)?.0.clone();
                (!text.is_empty()).then_some(text)
            })
            .collect();
        labels[1].clone()
    };
    assert_eq!(lit(&app), second, "Down moved the lit row to the second");
    tap(&mut app, KeyCode::Tab, Key::Tab);
    assert_eq!(value(&app, editor).text(), second, "Tab chooses too");
    type_str(&mut app, " a");
    tap(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(editor),
        "the first Escape closed the list"
    );
    tap(&mut app, KeyCode::Escape, Key::Escape);
    assert_ne!(app.world().resource::<InputFocus>().get(), Some(editor));
}

#[test]
fn a_diagnostic_is_a_squiggle_under_its_range_a_gutter_mark_and_a_note_on_ctrl_dot() {
    let mut app = app_with(MotionSettings::off());
    let editor = editor_of(&mut app, "let x = oops;\nok");
    let revision = value(&app, editor).revision();
    app.world_mut().entity_mut(editor).insert(Diagnostics::new(
        revision,
        [Diagnostic::new(
            8..12,
            CodeSeverity::Error,
            "no such name `oops`",
        )],
    ));
    settle(&mut app);
    let by_role = |app: &App, want: Role| -> Vec<Entity> {
        descendants(app.world(), editor)
            .into_iter()
            .filter(|&e| app.world().get::<Role>(e) == Some(&want) && shown(app, e))
            .collect()
    };
    let squiggles = by_role(&app, Role::CodeSquiggle(CodeSeverity::Error));
    assert_eq!(squiggles.len(), 1);
    assert_eq!(
        app.world().get::<Text>(squiggles[0]).expect("text").0,
        "~~~~",
        "one wave glyph a column"
    );
    let skin = app.world().resource::<GooeySkin>().clone();
    assert_eq!(
        app.world()
            .get::<TextColor>(squiggles[0])
            .expect("colour")
            .0,
        skin.severity(CodeSeverity::Error)
    );
    assert_eq!(
        by_role(&app, Role::CodeMark(CodeSeverity::Error)).len(),
        1,
        "a mark in the gutter"
    );
    assert!(by_role(&app, Role::CodeNoteText).is_empty());
    // Put the caret in the range and press Ctrl+. : the message shows in a note.
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(0, 9), CodePos::new(0, 9));
    settle(&mut app);
    chord(
        &mut app,
        KeyCode::ControlLeft,
        KeyCode::Period,
        Key::Character(".".into()),
    );
    let note = by_role(&app, Role::CodeNoteText);
    assert_eq!(note.len(), 1);
    assert_eq!(
        app.world().get::<Text>(note[0]).expect("text").0,
        "no such name `oops`"
    );
    // The next key puts it away.
    tap(&mut app, KeyCode::ArrowRight, Key::ArrowRight);
    assert!(by_role(&app, Role::CodeNoteText).is_empty());
}

#[test]
fn a_new_answer_with_other_items_replaces_the_list_whatever_the_prefix() {
    let mut app = app_with(MotionSettings::off());
    app.add_observer(
        |request: On<CodeCompletionRequested>, mut commands: Commands| {
            commands.entity(request.entity).insert(CodeCompletions {
                anchor: request.offset - request.prefix.len(),
                items: vec![
                    CompletionItem::new("map"),
                    CompletionItem::new("max"),
                    CompletionItem::new("match"),
                    CompletionItem::new("mix"),
                ],
            });
        },
    );
    let editor = editor_of(&mut app, "");
    type_str(&mut app, "m");
    let labels = |app: &App| -> Vec<String> {
        descendants(app.world(), editor)
            .into_iter()
            .filter(|&e| app.world().get::<Role>(e) == Some(&Role::CodeCompletionLabel))
            .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
            .filter(|t| !t.is_empty())
            .collect()
    };
    assert_eq!(labels(&app).len(), 4);
    // The host answers again for the same word with other items (a slower answer, a refined one):
    // the prefix is the same, the rows are the new items' and Enter chooses from them.
    app.world_mut().entity_mut(editor).insert(CodeCompletions {
        anchor: 0,
        items: vec![
            CompletionItem::new("minor"),
            CompletionItem::new("alpha"),
            CompletionItem::new("major"),
        ],
    });
    settle(&mut app);
    let mut shown = labels(&app);
    shown.sort();
    assert_eq!(
        shown,
        vec!["major".to_owned(), "minor".to_owned()],
        "the new items that match, not the old indices"
    );
    tap(&mut app, KeyCode::Enter, Key::Enter);
    let chosen = value(&app, editor).text().to_owned();
    assert!(
        chosen == "minor" || chosen == "major",
        "Enter chose one of the new items: {chosen:?}"
    );
}

#[test]
fn a_diagnostic_for_an_old_revision_is_dropped_never_drawn_on_new_text() {
    let mut app = app_with(MotionSettings::off());
    let editor = editor_of(&mut app, "let x = oops;");
    let revision = value(&app, editor).revision();
    app.world_mut().entity_mut(editor).insert(Diagnostics::new(
        revision,
        [Diagnostic::new(8..12, CodeSeverity::Error, "no such name")],
    ));
    settle(&mut app);
    let marks = |app: &App| -> usize {
        descendants(app.world(), editor)
            .into_iter()
            .filter(|&e| {
                matches!(
                    app.world().get::<Role>(e),
                    Some(Role::CodeSquiggle(_) | Role::CodeMark(_))
                ) && shown(app, e)
            })
            .count()
    };
    assert_eq!(marks(&app), 2, "a squiggle and a gutter mark");
    type_str(&mut app, "z");
    assert_ne!(value(&app, editor).revision(), revision);
    assert_eq!(
        marks(&app),
        0,
        "the old diagnostic is gone with the old text"
    );
    // The ctrl+. note is gone with it too.
    chord(
        &mut app,
        KeyCode::ControlLeft,
        KeyCode::Period,
        Key::Character(".".into()),
    );
    assert!(
        descendants(app.world(), editor)
            .into_iter()
            .all(|e| app.world().get::<Role>(e) != Some(&Role::CodeNoteText) || !shown(&app, e))
    );
}

#[test]
fn an_edit_from_elsewhere_that_names_text_that_is_not_there_is_refused_and_changes_nothing() {
    let mut app = app_with(MotionSettings::off());
    let editor = editor_of(&mut app, "let é = 1;\nlast");
    let revision = value(&app, editor).revision();
    let author = || CodeAuthor::new("Ada", Color::WHITE);
    let refusals = [
        (
            vec![CodeEdit::new(Range { start: 6, end: 2 }, "x")],
            "starts after it ends",
        ),
        (vec![CodeEdit::new(0..99, "x")], "past the text"),
        // Byte 5 is inside the é (bytes 4 and 5).
        (vec![CodeEdit::new(0..5, "x")], "inside a character"),
        (
            vec![CodeEdit::new(0..3, "x"), CodeEdit::new(2..3, "y")],
            "overlap",
        ),
    ];
    for (edits, why) in refusals {
        let outcome = app
            .world_mut()
            .get_mut::<CodeEditor>(editor)
            .expect("editor")
            .apply_external_edit(&edits, author());
        let error = outcome.expect_err(why);
        assert!(error.to_string().contains(why), "{error} for {why}");
        settle(&mut app);
        assert_eq!(value(&app, editor).text(), "let é = 1;\nlast");
        assert_eq!(value(&app, editor).revision(), revision, "nothing happened");
    }
}

#[derive(Resource, Default)]
struct HoverAsked(Vec<(usize, u64)>);

#[test]
fn a_hover_is_asked_for_by_offset_and_revision_answered_by_the_host_and_dropped_with_the_text() {
    let mut app = app_with(MotionSettings::off());
    app.init_resource::<HoverAsked>();
    app.add_observer(
        |request: On<CodeHoverRequested>, mut asked: ResMut<HoverAsked>| {
            asked.0.push((request.offset, request.revision));
        },
    );
    let editor = editor_of(&mut app, "let x = oops;");
    let revision = value(&app, editor).revision();
    let note_text = |app: &App| -> Vec<String> {
        descendants(app.world(), editor)
            .into_iter()
            .filter(|&e| app.world().get::<Role>(e) == Some(&Role::CodeNoteText) && shown(app, e))
            .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
            .collect()
    };
    // The caret on a word, Ctrl+. : the editor asks the host what it says about that offset.
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(0, 9), CodePos::new(0, 9));
    settle(&mut app);
    chord(
        &mut app,
        KeyCode::ControlLeft,
        KeyCode::Period,
        Key::Character(".".into()),
    );
    assert_eq!(
        app.world().resource::<HoverAsked>().0,
        vec![(9, revision)],
        "asked once, for the offset and the revision"
    );
    assert!(
        note_text(&app).is_empty(),
        "nothing to say until the host answers"
    );
    settle(&mut app);
    assert_eq!(
        app.world().resource::<HoverAsked>().0.len(),
        1,
        "a caret at rest asks once"
    );
    // The host answers for that revision: its words show in a note, and stay while the caret is
    // over the range.
    app.world_mut().entity_mut(editor).insert(CodeHover {
        revision,
        range: 8..12,
        text: "oops: a name nobody defined".to_owned(),
    });
    settle(&mut app);
    assert_eq!(
        note_text(&app),
        vec!["oops: a name nobody defined".to_owned()]
    );
    // The text moves on: an answer for the old revision is never drawn on the new text.
    type_str(&mut app, "z");
    assert_ne!(value(&app, editor).revision(), revision);
    settle(&mut app);
    assert!(
        note_text(&app).is_empty(),
        "an answer for an old revision is dropped"
    );
}

#[test]
fn every_syntax_colour_holds_the_contrast_bar_in_every_preset() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        assert!(
            skin.problems().is_empty(),
            "{}: {:?}",
            preset.name(),
            skin.problems()
        );
    }
    assert!(
        GooeySkin::high_contrast()
            .high_contrast_problems()
            .is_empty()
    );
}
