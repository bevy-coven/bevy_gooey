//! The combobox: a field with a list under it that narrows as the player types. Typing opens and
//! narrows the list, Down and Ctrl+N and Ctrl+P move the lit option (wrapping, skipping disabled
//! ones), Enter and a click choose, Escape closes before it cancels. Laid out for real in a
//! headless window, driven by real keyboard and pointer input.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, InteractionDisabled, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::type_text;
use bevy_gooey::*;

/// Every `ValueChange<String>` a game would see on a combobox, and every `Cancel` that reached the
/// screen.
#[derive(Resource, Default)]
struct Seen {
    changes: Vec<(Entity, String)>,
    cancels: usize,
}

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
    .init_resource::<Seen>()
    .add_observer(
        |change: On<ValueChange<String>>,
         combos: Query<(), With<Combobox>>,
         mut seen: ResMut<Seen>| {
            if combos.contains(change.source) {
                seen.changes.push((change.source, change.value.clone()));
            }
        },
    )
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        0.03,
    )));
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..14 {
        app.update();
    }
}

const COUNTRIES: [&str; 40] = [
    "Argentina",
    "Australia",
    "Austria",
    "Belgium",
    "Brazil",
    "Canada",
    "Chile",
    "China",
    "Colombia",
    "Denmark",
    "Egypt",
    "Finland",
    "France",
    "Germany",
    "Greece",
    "Hungary",
    "India",
    "Ireland",
    "Italy",
    "Japan",
    "Kenya",
    "Mexico",
    "Nepal",
    "Norway",
    "Pakistan",
    "Panama",
    "Paraguay",
    "Peru",
    "Philippines",
    "Poland",
    "Portugal",
    "Romania",
    "Spain",
    "Sweden",
    "Switzerland",
    "Thailand",
    "Turkey",
    "Ukraine",
    "Uruguay",
    "Vietnam",
];

/// A combobox over `options` in a dialog that counts the `Cancel`s that reach it, with its field
/// focused. The option's id is its label in lower case.
fn combo_of(app: &mut App, options: Vec<ComboOption>) -> Entity {
    let combo = app.world_mut().spawn(combobox("Country", options)).id();
    let card = app.world_mut().spawn(panel()).add_child(combo).id();
    app.world_mut()
        .spawn((
            dialog(),
            observe(|_: On<Cancel>, mut seen: ResMut<Seen>| seen.cancels += 1),
        ))
        .add_child(card);
    settle(app);
    focus_field(app, combo);
    combo
}

fn options_of(labels: &[&str]) -> Vec<ComboOption> {
    labels
        .iter()
        .map(|label| ComboOption::new(label.to_lowercase(), *label))
        .collect()
}

/// The text field of a combobox.
fn field_of(app: &mut App, combo: Entity) -> Entity {
    let row = app
        .world_mut()
        .query::<(Entity, &ComboField)>()
        .iter(app.world())
        .find(|(_, of)| of.0 == combo)
        .map(|(row, _)| row)
        .expect("the field row");
    app.world_mut()
        .query_filtered::<(Entity, &TextInputOf), With<bevy::ui_widgets::TextInput>>()
        .iter(app.world())
        .find(|(_, of)| of.0 == row)
        .map(|(field, _)| field)
        .expect("the field")
}

fn focus_field(app: &mut App, combo: Entity) {
    let field = field_of(app, combo);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(field, FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    app.update();
}

fn state(app: &App, combo: Entity) -> &Combobox {
    app.world().get::<Combobox>(combo).expect("a combobox")
}

fn text_of(app: &mut App, combo: Entity) -> String {
    let row = app
        .world_mut()
        .query::<(Entity, &ComboField)>()
        .iter(app.world())
        .find(|(_, of)| of.0 == combo)
        .map(|(row, _)| row)
        .expect("the field row");
    app.world()
        .get::<TextInputValue>(row)
        .expect("a value")
        .0
        .clone()
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

fn tap(app: &mut App, key_code: KeyCode, logical_key: Key) {
    key(app, key_code, logical_key.clone(), ButtonState::Pressed);
    key(app, key_code, logical_key, ButtonState::Released);
    settle(app);
}

fn down(app: &mut App) {
    tap(app, KeyCode::ArrowDown, Key::ArrowDown);
}

fn enter(app: &mut App) {
    tap(app, KeyCode::Enter, Key::Enter);
}

/// Ctrl and `letter`, as a keyboard sends them.
fn ctrl(app: &mut App, key_code: KeyCode, letter: &str) {
    key(
        app,
        KeyCode::ControlLeft,
        Key::Control,
        ButtonState::Pressed,
    );
    tap(app, key_code, Key::Character(letter.into()));
    key(
        app,
        KeyCode::ControlLeft,
        Key::Control,
        ButtonState::Released,
    );
    settle(app);
}

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let window = window(app);
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
    app.update();
}

fn click_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
}

fn centre(app: &App, entity: Entity) -> Vec2 {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    at * node.inverse_scale_factor()
}

/// The row of the open list at `slot`, if it is drawn.
fn row_at(app: &mut App, combo: Entity, slot: usize) -> Option<Entity> {
    let popover = state(app, combo).list()?;
    app.world_mut()
        .query::<(Entity, &ComboRow, &Node)>()
        .iter(app.world())
        .find(|(_, row, node)| {
            row.combo == combo
                && row.popover == popover
                && row.slot == slot
                && node.display != Display::None
        })
        .map(|(row, ..)| row)
}

fn shown_rows(app: &mut App, combo: Entity) -> usize {
    let Some(popover) = state(app, combo).list() else {
        return 0;
    };
    app.world_mut()
        .query::<(&ComboRow, &Node)>()
        .iter(app.world())
        .filter(|(row, node)| {
            row.combo == combo && row.popover == popover && node.display != Display::None
        })
        .count()
}

fn label(app: &App, combo: Entity, index: usize) -> String {
    state(app, combo).options()[index].label.clone()
}

#[test]
fn typing_par_narrows_forty_options_and_down_enter_chooses_one_with_one_change() {
    let mut app = app();
    let combo = combo_of(&mut app, options_of(&COUNTRIES));
    assert!(
        !state(&app, combo).is_open(),
        "closed until the player types or presses a key"
    );

    type_text(&mut app, "par");
    assert!(state(&app, combo).is_open(), "typing opens the list");
    let matches = state(&app, combo).matches().to_vec();
    assert_eq!(
        matches
            .iter()
            .map(|&i| label(&app, combo, i))
            .collect::<Vec<_>>(),
        vec!["Paraguay"],
        "'par' narrows the forty to the one that has it"
    );
    assert_eq!(shown_rows(&mut app, combo), 1, "one row, the rest hidden");
    assert_eq!(
        state(&app, combo).lit().map(|o| o.id.clone()),
        Some("paraguay".into())
    );

    down(&mut app);
    enter(&mut app);
    let seen = app.world().resource::<Seen>();
    assert_eq!(
        seen.changes,
        vec![(combo, "paraguay".to_owned())],
        "one ValueChange with the id"
    );
    assert_eq!(
        app.world().get::<ComboValue>(combo).expect("v").0,
        Some("paraguay".into())
    );
    assert_eq!(
        text_of(&mut app, combo),
        "Paraguay",
        "the field shows the chosen label"
    );
    assert!(!state(&app, combo).is_open(), "the list closes");
    assert_eq!(app.world().resource::<Seen>().cancels, 0);
}

#[test]
fn down_moves_among_several_matches_in_rank_order_and_enter_chooses_the_lit_one() {
    let mut app = app();
    let combo = combo_of(&mut app, options_of(&COUNTRIES));
    type_text(&mut app, "ia");
    let matches = state(&app, combo).matches().to_vec();
    assert!(
        matches.len() >= 3,
        "several countries have 'ia': {}",
        matches.len()
    );
    assert!(matches.len() < COUNTRIES.len(), "and not all of them");
    let ids: Vec<String> = matches
        .iter()
        .map(|&i| state(&app, combo).options()[i].id.clone())
        .collect();
    assert_eq!(
        state(&app, combo).lit().map(|o| o.id.clone()),
        Some(ids[0].clone())
    );
    down(&mut app);
    assert_eq!(
        state(&app, combo).lit().map(|o| o.id.clone()),
        Some(ids[1].clone())
    );
    enter(&mut app);
    assert_eq!(
        app.world().resource::<Seen>().changes,
        vec![(combo, ids[1].clone())],
        "the second match, one change"
    );
}

#[test]
fn ctrl_n_and_ctrl_p_wrap_over_the_enabled_options_and_skip_the_disabled() {
    let mut app = app();
    let mut options = options_of(&["Alpha", "Bravo", "Charlie", "Delta", "Echo"]);
    options[1] = ComboOption::new("bravo", "Bravo").disabled();
    options[4] = ComboOption::new("echo", "Echo").disabled();
    let combo = combo_of(&mut app, options);

    down(&mut app);
    assert!(state(&app, combo).is_open(), "Down opens the list");
    let lit = |app: &App| state(app, combo).lit().map(|o| o.id.clone());
    assert_eq!(
        lit(&app),
        Some("alpha".into()),
        "the first enabled option is lit"
    );
    ctrl(&mut app, KeyCode::KeyN, "n");
    assert_eq!(
        lit(&app),
        Some("charlie".into()),
        "Ctrl+N skips the disabled Bravo"
    );
    ctrl(&mut app, KeyCode::KeyN, "n");
    assert_eq!(lit(&app), Some("delta".into()));
    ctrl(&mut app, KeyCode::KeyN, "n");
    assert_eq!(
        lit(&app),
        Some("alpha".into()),
        "Ctrl+N from the last enabled row wraps to the first, past the disabled Echo"
    );
    ctrl(&mut app, KeyCode::KeyP, "p");
    assert_eq!(
        lit(&app),
        Some("delta".into()),
        "Ctrl+P wraps back to the last enabled"
    );
    assert_eq!(
        text_of(&mut app, combo),
        "",
        "the control keys typed nothing"
    );
}

#[test]
fn a_disabled_option_is_never_lit_and_refuses_a_click_and_enter() {
    let mut app = app();
    let mut options = options_of(&["Alpha", "Bravo", "Charlie"]);
    options[1] = ComboOption::new("bravo", "Bravo").disabled();
    let combo = combo_of(&mut app, options);
    down(&mut app);
    let disabled = row_at(&mut app, combo, 1).expect("Bravo's row");
    assert!(app.world().get::<InteractionDisabled>(disabled).is_some());
    let at = centre(&app, disabled);
    click_at(&mut app, at);
    assert!(
        app.world().resource::<Seen>().changes.is_empty(),
        "a click on a disabled row chooses nothing"
    );
    assert!(state(&app, combo).is_open(), "and the list stays open");
    assert_ne!(
        state(&app, combo).lit().map(|o| o.id.clone()),
        Some("bravo".into())
    );

    type_text(&mut app, "brav");
    assert_eq!(
        state(&app, combo).lit().map(|o| o.id.clone()),
        None,
        "only a disabled option matches"
    );
    enter(&mut app);
    assert!(
        app.world().resource::<Seen>().changes.is_empty(),
        "Enter chooses nothing"
    );
}

#[test]
fn a_click_on_a_row_chooses_it() {
    let mut app = app();
    let combo = combo_of(&mut app, options_of(&COUNTRIES));
    down(&mut app);
    let row = row_at(&mut app, combo, 3).expect("the fourth row is drawn");
    let at = centre(&app, row);
    click_at(&mut app, at);
    assert_eq!(
        app.world().resource::<Seen>().changes,
        vec![(combo, "belgium".to_owned())],
        "the fourth country, one change"
    );
    assert_eq!(text_of(&mut app, combo), "Belgium");
    assert!(!state(&app, combo).is_open());
}

#[test]
fn no_match_says_so_and_enter_chooses_nothing() {
    let mut app = app();
    let combo = combo_of(&mut app, options_of(&COUNTRIES));
    type_text(&mut app, "zzzz");
    assert!(state(&app, combo).is_open());
    assert!(state(&app, combo).matches().is_empty());
    assert_eq!(shown_rows(&mut app, combo), 0);
    let note = app
        .world_mut()
        .query::<(&ComboNote, &Node)>()
        .iter(app.world())
        .find(|(note, _)| note.combo == combo)
        .map(|(_, node)| node.display)
        .expect("the note");
    assert_eq!(note, Display::Flex, "\"No option matches\" shows");
    enter(&mut app);
    assert!(app.world().resource::<Seen>().changes.is_empty());
    assert!(state(&app, combo).is_open(), "and the list stays");
}

#[test]
fn escape_closes_the_list_and_a_second_escape_is_a_cancel() {
    let mut app = app();
    let combo = combo_of(&mut app, options_of(&COUNTRIES));
    app.world_mut().get_mut::<ComboValue>(combo).expect("v").0 = Some("peru".into());
    settle(&mut app);
    assert_eq!(
        text_of(&mut app, combo),
        "Peru",
        "a game's write shows the label"
    );
    assert!(
        app.world().resource::<Seen>().changes.is_empty(),
        "and sends no change"
    );

    type_text(&mut app, "x");
    assert!(state(&app, combo).is_open());
    tap(&mut app, KeyCode::Escape, Key::Escape);
    assert!(!state(&app, combo).is_open(), "Escape closes the list");
    assert_eq!(
        text_of(&mut app, combo),
        "Peru",
        "and puts the chosen label back"
    );
    assert_eq!(
        app.world().resource::<Seen>().cancels,
        0,
        "it was not a Cancel"
    );
    tap(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(
        app.world().resource::<Seen>().cancels,
        1,
        "the second Escape is one"
    );
}

#[test]
fn focus_leaving_the_field_closes_the_list_and_restores_the_label() {
    let mut app = app();
    let combo = combo_of(&mut app, options_of(&COUNTRIES));
    type_text(&mut app, "ita");
    assert!(state(&app, combo).is_open());
    app.world_mut().resource_mut::<InputFocus>().clear();
    settle(&mut app);
    assert!(!state(&app, combo).is_open());
    assert_eq!(
        text_of(&mut app, combo),
        "",
        "no choice, so the field is empty again"
    );
    assert!(app.world().resource::<Seen>().changes.is_empty());
}

#[test]
fn a_starting_value_shows_its_label_and_down_keeps_focus_with_a_button_below() {
    let mut app = app();
    let combo = app
        .world_mut()
        .spawn((
            combobox("Country", options_of(&COUNTRIES)),
            ComboValue(Some("peru".into())),
        ))
        .id();
    let below = app.world_mut().spawn(button("Next")).id();
    let card = app
        .world_mut()
        .spawn(panel())
        .add_children(&[combo, below])
        .id();
    app.world_mut().spawn(dialog()).add_child(card);
    settle(&mut app);
    assert_eq!(
        text_of(&mut app, combo),
        "Peru",
        "the starting value shows its label"
    );
    assert!(app.world().resource::<Seen>().changes.is_empty());

    focus_field(&mut app, combo);
    let field = field_of(&mut app, combo);
    down(&mut app);
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(field),
        "Down walks the list; it does not move focus to the button below"
    );
    assert!(state(&app, combo).is_open(), "Down opens the list");
    assert_eq!(
        state(&app, combo).lit().map(|o| o.id.clone()),
        Some("peru".into()),
        "on the chosen option"
    );
    down(&mut app);
    assert_eq!(
        state(&app, combo).lit().map(|o| o.id.clone()),
        Some("philippines".into())
    );
}
