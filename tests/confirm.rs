//! `confirm`: a modal question answered by one event. Laid out for real in a headless window, driven
//! by what the player reads and presses (`bevy_gooey::testing`).
#![cfg(feature = "testing")]

use bevy::input_focus::AutoFocus;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::{click, find_by_label, focused_label, press};
use bevy_gooey::*;

/// Every answer: the dialog, the choice, whether Back gave it, and whether the dialog was still
/// there when the observer ran.
#[derive(Resource, Default)]
struct Answers(Vec<(Entity, usize, bool, bool)>);

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: bevy::window::WindowResolution::new(1280, 720)
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
    .init_resource::<Answers>()
    .add_observer(
        |chosen: On<ConfirmChosen>, dialogs: Query<&Confirm>, mut answers: ResMut<Answers>| {
            answers.0.push((
                chosen.entity,
                chosen.index,
                chosen.cancelled,
                dialogs.get(chosen.entity).is_ok(),
            ));
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

fn open(app: &mut App, question: Confirm) -> Entity {
    let dialog = app.world_mut().spawn(question).id();
    settle(app);
    dialog
}

fn quit() -> Confirm {
    confirm(
        "Leave the game?",
        "Progress since the last autosave is lost.",
        ["Leave", "Stay"],
    )
}

fn answers(app: &App) -> Vec<(usize, bool)> {
    app.world()
        .resource::<Answers>()
        .0
        .iter()
        .map(|a| (a.1, a.2))
        .collect()
}

#[test]
fn it_opens_with_focus_on_the_safe_choice_and_the_words_on_screen() {
    let mut app = app();
    open(&mut app, quit());
    assert_eq!(focused_label(&app).as_deref(), Some("Stay"));
    for words in [
        "Leave the game?",
        "Progress since the last autosave is lost.",
        "Leave",
    ] {
        assert!(find_by_label(&mut app, words).is_ok(), "{words} is drawn");
    }
}

#[test]
fn enter_on_the_safe_choice_answers_it_once_and_closes() {
    let mut app = app();
    let dialog = open(&mut app, quit());
    press(&mut app, KeyCode::Enter);
    settle(&mut app);
    assert_eq!(answers(&app), vec![(1, false)]);
    assert!(
        app.world().resource::<Answers>().0[0].3,
        "the dialog is still alive for the observer that hears the answer"
    );
    assert!(app.world().get_entity(dialog).is_err(), "answered: gone");
}

#[test]
fn a_click_on_another_choice_answers_that_one() {
    let mut app = app();
    let dialog = open(&mut app, quit());
    let leave = find_by_label(&mut app, "Leave").expect("Leave");
    click(&mut app, leave);
    settle(&mut app);
    assert_eq!(answers(&app), vec![(0, false)]);
    assert!(app.world().get_entity(dialog).is_err());
}

#[test]
fn escape_answers_with_the_safe_choice_once_and_closes() {
    let mut app = app();
    let dialog = open(&mut app, quit());
    press(&mut app, KeyCode::Escape);
    settle(&mut app);
    assert_eq!(answers(&app), vec![(1, true)]);
    assert!(app.world().get_entity(dialog).is_err());
}

#[test]
fn the_safe_choice_can_be_the_first() {
    let mut app = app();
    open(
        &mut app,
        confirm("Overwrite?", "", ["Cancel", "Overwrite"]).safe(0),
    );
    assert_eq!(focused_label(&app).as_deref(), Some("Cancel"));
    press(&mut app, KeyCode::Escape);
    settle(&mut app);
    assert_eq!(answers(&app), vec![(0, true)]);
}

#[test]
fn an_empty_body_draws_no_line_and_one_choice_is_its_own_safe_choice() {
    let mut app = app();
    let dialog = open(&mut app, confirm("Saved.", "", ["OK"]));
    let labels = app
        .world_mut()
        .query_filtered::<&Role, With<Text>>()
        .iter(app.world())
        .filter(|role| **role == Role::Label)
        .count();
    assert_eq!(labels, 0, "no body, no label");
    assert_eq!(focused_label(&app).as_deref(), Some("OK"));
    press(&mut app, KeyCode::Escape);
    settle(&mut app);
    assert_eq!(answers(&app), vec![(0, true)]);
    assert!(app.world().get_entity(dialog).is_err());
}

#[test]
fn a_question_with_no_choices_is_answered_by_escape_and_breaks_nothing() {
    let mut app = app();
    let dialog = open(
        &mut app,
        confirm("Nothing to pick", "", Vec::<String>::new()),
    );
    press(&mut app, KeyCode::Escape);
    settle(&mut app);
    assert_eq!(answers(&app), vec![(0, true)]);
    assert!(app.world().get_entity(dialog).is_err());
}

#[test]
fn focus_goes_back_to_the_button_that_opened_it() {
    let mut app = app();
    app.world_mut()
        .spawn((screen(), children![(button("Quit"), AutoFocus)]));
    settle(&mut app);
    assert_eq!(focused_label(&app).as_deref(), Some("Quit"));
    open(&mut app, quit());
    assert_eq!(focused_label(&app).as_deref(), Some("Stay"));
    press(&mut app, KeyCode::Escape);
    settle(&mut app);
    assert_eq!(focused_label(&app).as_deref(), Some("Quit"));
}

#[test]
fn arrows_cannot_leave_the_question() {
    let mut app = app();
    app.world_mut()
        .spawn((screen(), children![(button("Behind"), AutoFocus)]));
    settle(&mut app);
    open(&mut app, quit());
    for key in [
        KeyCode::ArrowDown,
        KeyCode::ArrowUp,
        KeyCode::ArrowRight,
        KeyCode::Tab,
    ] {
        press(&mut app, key);
        settle(&mut app);
        assert_ne!(focused_label(&app).as_deref(), Some("Behind"), "{key:?}");
    }
    assert!(answers(&app).is_empty(), "moving answers nothing");
}
