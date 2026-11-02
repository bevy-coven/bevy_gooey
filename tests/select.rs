//! The select: a choice with eight options or fewer, a searchable list above that. Laid out for real
//! in a headless window, driven by real keyboard and pointer input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{InteractionDisabled, Selected, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<usize>` a game would see.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, usize)>);

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
    .add_observer(
        |change: On<ValueChange<usize>>, mut seen: ResMut<Changes>| {
            seen.0.push((change.source, change.value))
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

/// A select over `options` on a screen, focused, with `selected` chosen.
fn select_of(app: &mut App, options: &[&str], selected: usize) -> Entity {
    let row = app
        .world_mut()
        .spawn(select("Country", options.iter().copied(), selected))
        .id();
    let panel = app.world_mut().spawn(panel()).add_child(row).id();
    app.world_mut().spawn(screen()).add_child(panel);
    settle(app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(row, FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    app.update();
    row
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

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
    settle(app);
}

fn enter(app: &mut App) {
    tap_named(app, KeyCode::Enter, Key::Enter);
}

fn type_str(app: &mut App, text: &str) {
    let window = window(app);
    for c in text.chars() {
        let s = c.to_string();
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::KeyA,
                logical_key: Key::Character(s.as_str().into()),
                state,
                text: (state == ButtonState::Pressed).then(|| s.as_str().into()),
                repeat: false,
                window,
            });
            app.update();
        }
    }
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

fn click(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
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

/// The open list, if one is.
fn list(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<CommandPalette>>()
        .iter(world)
        .next()
}

/// The rows the list shows, top first: (words, lit, entity).
fn rows(app: &mut App, list: Entity) -> Vec<(String, bool, Entity)> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &PaletteRow, &Node, &Children, Has<Selected>)>();
    let mut found: Vec<(usize, String, bool, Entity)> = query
        .iter(world)
        .filter(|(_, row, node, ..)| row.palette == list && node.display != Display::None)
        .map(|(entity, row, _, children, lit)| {
            let words = world
                .get::<Text>(children[0])
                .map(|t| t.0.clone())
                .unwrap_or_default();
            (row.slot, words, lit, entity)
        })
        .collect();
    found.sort_by_key(|(slot, ..)| *slot);
    found
        .into_iter()
        .map(|(_, words, lit, entity)| (words, lit, entity))
        .collect()
}

fn note(app: &mut App) -> (String, bool) {
    let world = app.world_mut();
    world
        .query::<(&PaletteNote, &Node, &Text)>()
        .iter(world)
        .next()
        .map(|(_, node, text)| (text.0.clone(), node.display != Display::None))
        .expect("the note")
}

fn value(app: &App, row: Entity) -> usize {
    app.world().get::<ChoiceValue>(row).expect("a choice").0
}

#[test]
fn typing_par_in_forty_options_leaves_the_matching_ones_and_enter_selects_the_first() {
    let mut app = app();
    let row = select_of(&mut app, &COUNTRIES, 0);
    enter(&mut app);
    let list = list(&mut app).expect("Enter opens the list");
    // What the player sees is the fuzzy rule's: best first, the list's own order among equals.
    let expected = |query: &str| -> Vec<usize> {
        let mut want: Vec<(usize, u32)> = COUNTRIES
            .iter()
            .enumerate()
            .filter_map(|(i, name)| fuzzy_score(query, name).map(|score| (i, score)))
            .collect();
        want.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        want.into_iter().map(|(i, _)| i).collect()
    };
    let names = |app: &mut App, list: Entity| -> Vec<String> {
        rows(app, list).into_iter().map(|(w, ..)| w).collect()
    };
    type_str(&mut app, "pa");
    let wide = expected("pa");
    assert!(wide.len() > 3 && wide.len() < COUNTRIES.len(), "{wide:?}");
    assert_eq!(
        names(&mut app, list),
        wide.iter().map(|&i| COUNTRIES[i].to_string()).collect::<Vec<_>>()
    );
    type_str(&mut app, "r");
    let want = expected("par");
    assert_eq!(want, vec![26], "only Paraguay holds p, a, r in order");
    let shown = rows(&mut app, list);
    assert_eq!(names(&mut app, list), vec!["Paraguay".to_string()]);
    assert!(shown[0].1, "the first match is lit");
    enter(&mut app);
    assert_eq!(app.world().resource::<Changes>().0, vec![(row, want[0])]);
    assert_eq!(value(&app, row), want[0]);
    assert!(self::list(&mut app).is_none(), "the list closes");
}

#[test]
fn eight_options_are_a_choice_that_cycles_and_nine_are_a_list() {
    let mut app = app();
    let eight = select_of(&mut app, &COUNTRIES[..8], 0);
    enter(&mut app);
    assert!(list(&mut app).is_none());
    assert_eq!(app.world().resource::<Changes>().0, vec![(eight, 1)]);
    let mut app = self::app();
    let nine = select_of(&mut app, &COUNTRIES[..9], 0);
    enter(&mut app);
    assert!(list(&mut app).is_some());
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert_eq!(value(&app, nine), 0);
}

#[test]
fn nothing_matching_says_so_chooses_nothing_and_leaves_the_list_open() {
    let mut app = app();
    let row = select_of(&mut app, &COUNTRIES, 3);
    enter(&mut app);
    let list = list(&mut app).expect("open");
    type_str(&mut app, "zzzz");
    assert_eq!(note(&mut app), ("No option matches".to_string(), true));
    assert!(rows(&mut app, list).is_empty());
    enter(&mut app);
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert_eq!(value(&app, row), 3);
    assert!(self::list(&mut app).is_some(), "still open");
}

#[test]
fn escape_closes_the_list_leaves_the_value_and_gives_focus_back_to_the_row() {
    let mut app = app();
    let row = select_of(&mut app, &COUNTRIES, 5);
    enter(&mut app);
    type_str(&mut app, "par");
    tap_named(&mut app, KeyCode::Escape, Key::Escape);
    assert!(list(&mut app).is_none());
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert_eq!(value(&app, row), 5);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(row));
}

#[test]
fn the_list_opens_with_the_current_option_lit_and_the_options_in_their_own_order() {
    let mut app = app();
    let options = ["Zulu", "Alpha", "Mike", "Bravo", "Xray", "Charlie", "Yankee", "Delta", "Echo", "Foxtrot"];
    let row = select_of(&mut app, &options, 4);
    enter(&mut app);
    let list = list(&mut app).expect("open");
    let shown = rows(&mut app, list);
    assert_eq!(shown[0].0, "Zulu", "not alphabetical");
    assert_eq!(
        shown.iter().filter(|r| r.1).map(|r| r.0.as_str()).collect::<Vec<_>>(),
        vec!["Xray"]
    );
    // Enter on it chooses what the row already has: no change.
    enter(&mut app);
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert_eq!(value(&app, row), 4);
}

#[test]
fn down_moves_the_lit_option_and_enter_chooses_it() {
    let mut app = app();
    let row = select_of(&mut app, &COUNTRIES, 0);
    enter(&mut app);
    tap_named(&mut app, KeyCode::ArrowDown, Key::ArrowDown);
    tap_named(&mut app, KeyCode::ArrowDown, Key::ArrowDown);
    enter(&mut app);
    assert_eq!(app.world().resource::<Changes>().0, vec![(row, 2)]);
    assert_eq!(value(&app, row), 2);
}

#[test]
fn a_click_opens_the_list_and_a_click_on_an_option_chooses_it_once() {
    let mut app = app();
    let row = select_of(&mut app, &COUNTRIES, 0);
    let at = rect(&app, row).center();
    click(&mut app, at);
    let list = list(&mut app).expect("a click opens it");
    let shown = rows(&mut app, list);
    let third = shown[2].2;
    let at = rect(&app, third).center();
    click(&mut app, at);
    assert_eq!(app.world().resource::<Changes>().0, vec![(row, 2)]);
    assert!(self::list(&mut app).is_none());
}

#[test]
fn left_and_right_still_step_a_select_that_filters_and_open_nothing() {
    let mut app = app();
    let row = select_of(&mut app, &COUNTRIES, 0);
    tap_named(&mut app, KeyCode::ArrowRight, Key::ArrowRight);
    assert_eq!(app.world().resource::<Changes>().0, vec![(row, 1)]);
    assert!(list(&mut app).is_none());
}

#[test]
fn a_disabled_select_opens_nothing() {
    let mut app = app();
    let row = select_of(&mut app, &COUNTRIES, 0);
    app.world_mut().entity_mut(row).insert(InteractionDisabled);
    settle(&mut app);
    enter(&mut app);
    assert!(list(&mut app).is_none());
}
