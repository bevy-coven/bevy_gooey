//! Code editor navigation (G-73), headless: folding moves the lines after a fold up by the folded
//! height, find counts its matches and one undo takes back Replace all, bracket matching, indent
//! guides, and the minimap with its band and drag.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::{click, find_by_label};
use bevy_gooey::*;

fn app() -> App {
    let mut app = App::new();
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

fn editor_of(app: &mut App, text: &str, extra: impl Bundle) -> Entity {
    let world = app.world_mut();
    let editor = world.spawn((code_editor("Script", text), extra)).id();
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

fn key(app: &mut App, key_code: KeyCode, logical_key: Key, state: ButtonState) {
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

fn chord(app: &mut App, mods: &[KeyCode], key_code: KeyCode, logical_key: Key) {
    for &m in mods {
        key(
            app,
            m,
            Key::Unidentified(NativeKey::Unidentified),
            ButtonState::Pressed,
        );
    }
    key(app, key_code, logical_key.clone(), ButtonState::Pressed);
    key(app, key_code, logical_key, ButtonState::Released);
    for &m in mods.iter().rev() {
        key(
            app,
            m,
            Key::Unidentified(NativeKey::Unidentified),
            ButtonState::Released,
        );
    }
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

fn shown_by_role(app: &App, editor: Entity, role: Role) -> Vec<Entity> {
    descendants(app.world(), editor)
        .into_iter()
        .filter(|&e| app.world().get::<Role>(e) == Some(&role))
        .filter(|&e| {
            app.world()
                .get::<Node>(e)
                .is_some_and(|n| n.display != Display::None)
        })
        .collect()
}

/// The row showing `text`: its entity.
fn row_with(app: &App, editor: Entity, text: &str) -> Entity {
    shown_by_role(app, editor, Role::CodeText)
        .into_iter()
        .find(|&e| {
            app.world().get::<Visibility>(e) != Some(&Visibility::Hidden)
                && app.world().get::<Text>(e).is_some_and(|t| t.0 == text)
        })
        .unwrap_or_else(|| panic!("a row showing {text:?}"))
}

fn lines(n: usize) -> String {
    (0..n).map(|i| format!("l{i}\n")).collect()
}

#[test]
fn folding_a_range_moves_the_lines_after_it_up_by_the_folded_height() {
    let mut app = app();
    let editor = editor_of(&mut app, &lines(12), ());
    let revision = app
        .world()
        .get::<CodeEditorValue>(editor)
        .expect("v")
        .revision();
    app.world_mut()
        .entity_mut(editor)
        .insert(FoldRanges::new(revision, std::iter::once(1..5)));
    settle(&mut app);
    let step = rect(&app, row_with(&app, editor, "l1")).min.y
        - rect(&app, row_with(&app, editor, "l0")).min.y;
    let before = rect(&app, row_with(&app, editor, "l5")).min.y
        - rect(&app, row_with(&app, editor, "l1")).min.y;
    assert!(
        (before - 4.0 * step).abs() < 0.5,
        "l5 is four rows under l1"
    );
    // A chevron stands on the header.
    assert_eq!(shown_by_role(&app, editor, Role::CodeChevron).len(), 1);
    // Ctrl+Shift+[ folds the block at the caret's line.
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(1, 0), CodePos::new(1, 0));
    settle(&mut app);
    chord(
        &mut app,
        &[KeyCode::ControlLeft, KeyCode::ShiftLeft],
        KeyCode::BracketLeft,
        Key::Character("{".into()),
    );
    let after = rect(&app, row_with(&app, editor, "l5")).min.y
        - rect(&app, row_with(&app, editor, "l1")).min.y;
    assert!(
        (after - step).abs() < 0.5,
        "l5 moved up by the folded height: {before} to {after}"
    );
    // The folded lines are not drawn, and the numbers skip them.
    let words: Vec<String> = shown_by_role(&app, editor, Role::CodeText)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect();
    assert!(!words.contains(&"l2".to_owned()) && words.contains(&"l5".to_owned()));
    let numbers: Vec<String> = shown_by_role(&app, editor, Role::CodeNumber)
        .into_iter()
        .chain(shown_by_role(&app, editor, Role::CodeNumberLit))
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect();
    assert!(
        numbers.contains(&"6".to_owned()) && !numbers.contains(&"3".to_owned()),
        "{numbers:?}"
    );
    // The caret cannot go into the fold: Down from the header lands on the line after it.
    chord(&mut app, &[], KeyCode::ArrowDown, Key::ArrowDown);
    assert_eq!(
        app.world()
            .get::<CodeEditor>(editor)
            .expect("e")
            .buffer()
            .caret()
            .line,
        5
    );
    // Ctrl+Shift+] opens it again.
    chord(
        &mut app,
        &[KeyCode::ControlLeft, KeyCode::ShiftLeft],
        KeyCode::BracketRight,
        Key::Character("}".into()),
    );
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(1, 0), CodePos::new(1, 0));
    chord(
        &mut app,
        &[KeyCode::ControlLeft, KeyCode::ShiftLeft],
        KeyCode::BracketRight,
        Key::Character("}".into()),
    );
    let reopened = rect(&app, row_with(&app, editor, "l5")).min.y
        - rect(&app, row_with(&app, editor, "l1")).min.y;
    assert!((reopened - 4.0 * step).abs() < 0.5);
}

#[test]
fn find_counts_its_matches_and_one_undo_takes_back_replace_all() {
    let mut app = app();
    let editor = editor_of(&mut app, "cat dog cat\nbird Cat\ncatalog", ());
    app.world_mut()
        .get_mut::<CodeFind>(editor)
        .expect("find")
        .show_replace = true;
    app.world_mut()
        .get_mut::<CodeFind>(editor)
        .expect("find")
        .open = true;
    {
        let mut find = app.world_mut().get_mut::<CodeFind>(editor).expect("find");
        find.query = "cat".into();
        find.replacement = "fox".into();
    }
    settle(&mut app);
    {
        let find = app.world().get::<CodeFind>(editor).expect("find");
        assert_eq!(find.count(), 4, "cat, cat, Cat and the one in catalog");
        assert_eq!(find.error(), None);
    }
    // Whole word and case narrow it.
    app.world_mut()
        .get_mut::<CodeFind>(editor)
        .expect("find")
        .options
        .whole_word = true;
    settle(&mut app);
    assert_eq!(
        app.world().get::<CodeFind>(editor).expect("find").count(),
        3
    );
    app.world_mut()
        .get_mut::<CodeFind>(editor)
        .expect("find")
        .options
        .case_sensitive = true;
    settle(&mut app);
    assert_eq!(
        app.world().get::<CodeFind>(editor).expect("find").count(),
        2
    );
    app.world_mut()
        .get_mut::<CodeFind>(editor)
        .expect("find")
        .options = FindOptions::default();
    settle(&mut app);
    // The matches in view are drawn as bands.
    assert!(
        shown_by_role(&app, editor, Role::CodeFindMatch).len()
            + shown_by_role(&app, editor, Role::CodeFindMatchLit).len()
            >= 4
    );
    // Replace all, through the bar's own button.
    let button = find_by_label(&mut app, "Replace all").expect("the button");
    click(&mut app, button);
    settle(&mut app);
    let text = app
        .world()
        .get::<CodeEditorValue>(editor)
        .expect("v")
        .text()
        .to_owned();
    assert_eq!(text, "fox dog fox\nbird fox\nfoxalog");
    // One undo takes the whole replace back.
    chord(
        &mut app,
        &[KeyCode::ControlLeft],
        KeyCode::KeyZ,
        Key::Character("z".into()),
    );
    assert!(app.world().resource::<InputFocus>().get().is_some());
    let editor_ref = app.world().get::<CodeEditor>(editor).expect("editor");
    assert_eq!(
        editor_ref.buffer().text(),
        "cat dog cat\nbird Cat\ncatalog",
        "{:?}",
        editor_ref.buffer().text()
    );
}

#[test]
fn a_regular_expression_is_searched_and_a_bad_one_is_an_error_not_a_panic() {
    let mut app = app();
    let editor = editor_of(&mut app, "a1 b22 c333", ());
    {
        let mut find = app.world_mut().get_mut::<CodeFind>(editor).expect("find");
        find.open = true;
        find.options.regex = true;
        find.query = r"\d+".into();
    }
    settle(&mut app);
    assert_eq!(
        app.world().get::<CodeFind>(editor).expect("find").count(),
        3
    );
    app.world_mut()
        .get_mut::<CodeFind>(editor)
        .expect("find")
        .query = "(".into();
    settle(&mut app);
    let find = app.world().get::<CodeFind>(editor).expect("find");
    assert_eq!(find.count(), 0);
    assert!(find.error().is_some());
}

#[test]
fn the_matching_bracket_is_lit_and_indent_guides_stand_at_each_level() {
    let mut app = app();
    let editor = editor_of(&mut app, "fn f() {\n    if x {\n        y\n    }\n}\n", ());
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(0, 7), CodePos::new(0, 7));
    settle(&mut app);
    assert_eq!(
        shown_by_role(&app, editor, Role::CodeBracket).len(),
        2,
        "both brackets of the pair"
    );
    // Two levels deep at the deepest row: guides at level one and level two.
    let guides = shown_by_role(&app, editor, Role::CodeGuide);
    assert!(guides.len() >= 2, "{} guides", guides.len());
    let xs: Vec<f32> = guides.iter().map(|&g| rect(&app, g).min.x).collect();
    assert!(
        xs.iter().any(|&x| (x - xs[0]).abs() > 1.0),
        "guides at more than one level: {xs:?}"
    );
}

#[test]
fn the_minimap_draws_a_bar_a_line_a_band_for_the_part_in_view_and_a_drag_scrolls() {
    let mut app = app();
    let text: String = (0..400)
        .map(|i| format!("{}\n", "x".repeat(i % 30)))
        .collect();
    let editor = editor_of(&mut app, &text, CodeMinimap);
    settle(&mut app);
    settle(&mut app);
    let bars = shown_by_role(&app, editor, Role::CodeMinimapBar);
    assert!(bars.len() > 20, "bars for the lines: {}", bars.len());
    let band = shown_by_role(&app, editor, Role::CodeMinimapBand);
    assert_eq!(band.len(), 1);
    let before = rect(&app, band[0]);
    // Pressing the minimap far down scrolls the text there, and the band follows.
    let map = shown_by_role(&app, editor, Role::CodeMinimap)[0];
    let map_rect = rect(&app, map);
    let at = Vec2::new(
        map_rect.center().x,
        map_rect.min.y + map_rect.height() * 0.9,
    );
    let window_entity = window(&mut app);
    let location = Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window_entity)
                .normalize(None)
                .expect("a window"),
        ),
        position: at,
    };
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
        app.update();
    }
    settle(&mut app);
    let scrolled = app
        .world()
        .get::<CodeEditor>(editor)
        .expect("editor")
        .scroll()
        .y;
    assert!(scrolled > 500.0, "the press scrolled the text: {scrolled}");
    let after = rect(&app, shown_by_role(&app, editor, Role::CodeMinimapBand)[0]);
    assert!(
        after.min.y > before.min.y + 20.0,
        "the band moved with it: {before:?} to {after:?}"
    );
}

fn shown_words(app: &App, editor: Entity) -> Vec<String> {
    shown_by_role(app, editor, Role::CodeText)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect()
}

#[test]
fn a_fold_follows_its_lines_when_the_text_above_it_changes() {
    let mut app = app();
    let editor = editor_of(&mut app, &lines(12), ());
    let value = |app: &App| {
        app.world()
            .get::<CodeEditorValue>(editor)
            .expect("v")
            .revision()
    };
    let revision = value(&app);
    app.world_mut()
        .entity_mut(editor)
        .insert(FoldRanges::new(revision, std::iter::once(1..5)));
    settle(&mut app);
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(1, 0), CodePos::new(1, 0));
    settle(&mut app);
    chord(
        &mut app,
        &[KeyCode::ControlLeft, KeyCode::ShiftLeft],
        KeyCode::BracketLeft,
        Key::Character("{".into()),
    );
    let hidden = |app: &App| -> Vec<String> {
        let words = shown_words(app, editor);
        (0..12)
            .map(|i| format!("l{i}"))
            .filter(|word| !words.contains(word))
            .collect()
    };
    assert_eq!(
        hidden(&app),
        vec!["l2", "l3", "l4"],
        "lines 2 to 4 are folded"
    );
    // The player adds a line above the fold: the fold stays on the same text, whatever the host
    // has not yet said about the new revision.
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(0, 0), CodePos::new(0, 0));
    settle(&mut app);
    chord(&mut app, &[], KeyCode::Enter, Key::Enter);
    assert_eq!(
        hidden(&app),
        vec!["l2", "l3", "l4"],
        "the fold moved down a line with its text"
    );
    // The host answers for the new revision, one line lower: nothing changes.
    let revision = value(&app);
    app.world_mut()
        .entity_mut(editor)
        .insert(FoldRanges::new(revision, std::iter::once(2..6)));
    settle(&mut app);
    assert_eq!(hidden(&app), vec!["l2", "l3", "l4"]);
    // An edit from elsewhere above it moves it too.
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .apply_external_edit(
            &[CodeEdit::new(0..0, "x\ny\n")],
            CodeAuthor::new("Ada", Color::WHITE),
        )
        .expect("a valid edit");
    settle(&mut app);
    assert_eq!(hidden(&app), vec!["l2", "l3", "l4"]);
    // The host replacing the whole text drops the folds: the new text is not folded anywhere.
    app.world_mut()
        .get_mut::<CodeEditorValue>(editor)
        .expect("v")
        .set(lines(12));
    settle(&mut app);
    assert!(hidden(&app).is_empty(), "nothing is folded in a new text");
}

#[test]
fn ctrl_f_opens_the_bar_seeded_with_the_selected_word_and_ctrl_alt_f_adds_the_replace_field() {
    let mut app = app();
    let editor = editor_of(&mut app, "let found = 1;\nfound", ());
    app.world_mut()
        .get_mut::<CodeEditor>(editor)
        .expect("editor")
        .select(CodePos::new(0, 4), CodePos::new(0, 9));
    settle(&mut app);
    assert!(!app.world().get::<CodeFind>(editor).expect("find").open);
    chord(
        &mut app,
        &[KeyCode::ControlLeft],
        KeyCode::KeyF,
        Key::Character("f".into()),
    );
    {
        let find = app.world().get::<CodeFind>(editor).expect("find");
        assert!(find.open, "Ctrl+F opens the bar");
        assert_eq!(find.query, "found", "seeded with the selected word");
        assert!(!find.show_replace);
        assert_eq!(find.count(), 2);
    }
    assert!(find_by_label(&mut app, "Replace all").is_err());
    chord(
        &mut app,
        &[KeyCode::ControlLeft, KeyCode::AltLeft],
        KeyCode::KeyF,
        Key::Character("f".into()),
    );
    assert!(
        app.world()
            .get::<CodeFind>(editor)
            .expect("find")
            .show_replace,
        "Ctrl+Alt+F shows the replace field"
    );
    assert!(find_by_label(&mut app, "Replace all").is_ok());
}

#[test]
fn a_pattern_the_engine_cannot_search_is_an_error_the_bar_shows_not_no_matches() {
    let buffer = CodeBuffer::new("a1 b22\nline 3");
    let regex = FindOptions {
        regex: true,
        ..FindOptions::default()
    };
    // An escape the engine does not know, and one that can never match on a line.
    for bad in [r"\1", r"\p{L}", r"\x41", r"a\nb"] {
        assert!(
            matches!(find_all(&buffer, bad, regex), Err(FindError::Regex(_))),
            "{bad}"
        );
    }
    assert_eq!(find_all(&buffer, r"\d+", regex).expect("fine").len(), 3);
    // A pattern that backtracks without end is refused, never "no matches".
    let spun = CodeBuffer::new(&format!("{}\nb", "a".repeat(60)));
    assert_eq!(
        find_all(&spun, "(a*)*b", regex),
        Err(FindError::TooMuchWork)
    );
    assert_eq!(
        replace_edits(&spun, "(a*)*b", "x", regex),
        Err(FindError::TooMuchWork)
    );
    // Many lines, each cheap alone: the scan as a whole has a budget, and the first
    // pathological line ends it at once.
    let many: String = (0..2_000)
        .map(|_| format!("{}\n", "a".repeat(40)))
        .collect();
    assert_eq!(
        find_all(&CodeBuffer::new(&many), "(a*)*b", regex),
        Err(FindError::TooMuchWork)
    );
    // A long line under a pattern that recurses is refused, not a stack overflow.
    let long = CodeBuffer::new(&"ab".repeat(30_000));
    assert_eq!(find_all(&long, "(?:ab)+", regex), Err(FindError::TooDeep));
    // The bar says so.
    let mut app = app();
    let editor = editor_of(&mut app, &format!("{}\nb", "a".repeat(60)), ());
    {
        let mut find = app.world_mut().get_mut::<CodeFind>(editor).expect("find");
        find.open = true;
        find.options.regex = true;
        find.query = "(a*)*b".into();
    }
    settle(&mut app);
    let find = app.world().get::<CodeFind>(editor).expect("find");
    assert_eq!(find.count(), 0);
    assert_eq!(find.error(), Some("this pattern takes too long to search"));
}

#[test]
fn the_minimap_band_is_dragged_after_the_press_with_no_second_press() {
    let mut app = app();
    let text: String = (0..400)
        .map(|i| format!("{}\n", "x".repeat(i % 30)))
        .collect();
    let editor = editor_of(&mut app, &text, CodeMinimap);
    settle(&mut app);
    settle(&mut app);
    let map_rect = rect(&app, shown_by_role(&app, editor, Role::CodeMinimap)[0]);
    let window_entity = window(&mut app);
    let location = |at: Vec2| Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window_entity)
                .normalize(None)
                .expect("a window"),
        ),
        position: at,
    };
    let scroll = |app: &App| {
        app.world()
            .get::<CodeEditor>(editor)
            .expect("editor")
            .scroll()
            .y
    };
    let at = |share: f32| {
        Vec2::new(
            map_rect.center().x,
            map_rect.min.y + map_rect.height() * share,
        )
    };
    let send = |app: &mut App, at: Vec2, action: PointerAction| {
        app.world_mut()
            .write_message(PointerInput::new(PointerId::Mouse, location(at), action));
        app.update();
    };
    send(&mut app, at(0.1), PointerAction::Move { delta: Vec2::ZERO });
    send(
        &mut app,
        at(0.1),
        PointerAction::Press(PointerButton::Primary),
    );
    settle(&mut app);
    let pressed = scroll(&app);
    // The button stays down and the pointer travels: the band goes with it.
    for share in [0.3, 0.5, 0.7, 0.9] {
        send(
            &mut app,
            at(share),
            PointerAction::Move {
                delta: Vec2::new(0.0, map_rect.height() * 0.2),
            },
        );
    }
    settle(&mut app);
    let dragged = scroll(&app);
    assert!(
        dragged > pressed + 400.0,
        "the drag scrolled on from the press: {pressed} to {dragged}"
    );
    send(
        &mut app,
        at(0.9),
        PointerAction::Release(PointerButton::Primary),
    );
}
