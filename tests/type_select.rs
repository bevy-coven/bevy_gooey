//! Type-select (G-56): in a tree, a table, a list, an open menu and a select that does not filter,
//! typing a name goes to it. Laid out for real in a headless window, driven by real
//! `KeyboardInput` on a clock the test sets.

use std::time::Duration;

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::ValueChange;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::{find_by_label, focused, press, type_text};
use bevy_gooey::*;

/// One frame of the test's clock.
const FRAME_MS: u64 = 16;

#[derive(Resource, Default)]
struct Changes(Vec<usize>);

fn app() -> App {
    let mut app = App::new();
    // Assumed owner exception (#76): a scroll glides on the real clock, and this app reads the pool
    // the scroll has landed on.
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
        FRAME_MS,
    )))
    .init_resource::<Changes>()
    .add_observer(
        |change: On<ValueChange<usize>>, mut seen: ResMut<Changes>| {
            seen.0.push(change.value);
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

/// Runs frames until longer than the type-select pause has passed.
fn pause(app: &mut App) {
    let frames = (TYPE_AHEAD_SECS * 1000.0) as u64 / FRAME_MS + 10;
    for _ in 0..frames {
        app.update();
    }
}

fn focus_on(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    settle(app);
}

fn the<T: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<T>>()
        .single(app.world())
        .expect("exactly one")
}

fn cursor(app: &mut App, table: Entity) -> Option<u64> {
    app.world().get::<TableCursor>(table).expect("a cursor").0
}

/// 200 roots: six fruit and bird names among 194 zones, in this order.
fn two_hundred() -> Vec<TreeNode> {
    let named = ["Apple", "Parrot", "Party", "Owl", "Peach", "Plum"];
    let mut nodes: Vec<TreeNode> = named
        .iter()
        .enumerate()
        .map(|(i, name)| TreeNode::new(i as u64, *name))
        .collect();
    nodes.extend((named.len()..200).map(|i| TreeNode::new(i as u64, format!("Zone {i:03}"))));
    nodes
}

#[test]
fn typing_in_a_tree_goes_to_the_prefix_cycles_a_letter_and_starts_over_after_a_pause() {
    let mut app = app();
    app.world_mut().spawn((
        panel(),
        children![tree(two_hundred()), text_input("Name", "")],
    ));
    settle(&mut app);
    let tree = the::<Table>(&mut app);
    focus_on(&mut app, tree);
    type_text(&mut app, "par");
    assert_eq!(cursor(&mut app, tree), Some(1), "\"par\" lands on Parrot");
    pause(&mut app);
    type_text(&mut app, "p");
    assert_eq!(
        cursor(&mut app, tree),
        Some(2),
        "after a pause \"p\" is a new prefix: the next p, Party"
    );
    type_text(&mut app, "p");
    assert_eq!(cursor(&mut app, tree), Some(4), "\"p\" again cycles: Peach");
    type_text(&mut app, "p");
    assert_eq!(cursor(&mut app, tree), Some(5), "and again: Plum");
    pause(&mut app);
    type_text(&mut app, "a");
    assert_eq!(
        cursor(&mut app, tree),
        Some(0),
        "a new prefix wraps to Apple"
    );
    assert_eq!(
        focused(&app),
        Some(tree),
        "type-select keeps focus on the tree"
    );
    // A text field takes the characters instead.
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::ui_widgets::TextInput>>()
        .single(app.world())
        .expect("one text field");
    focus_on(&mut app, field);
    type_text(&mut app, "pl");
    assert_eq!(
        cursor(&mut app, tree),
        Some(0),
        "typing in a field moves no tree"
    );
    let typed: Vec<String> = app
        .world_mut()
        .query::<&TextInputValue>()
        .iter(app.world())
        .map(|value| value.0.clone())
        .collect();
    assert_eq!(typed, vec!["pl".to_owned()], "the field has the characters");
}

#[test]
fn typing_in_a_ten_thousand_row_table_scrolls_to_the_row_and_puts_the_cursor_on_it() {
    let mut app = app();
    let rows: Vec<Vec<String>> = (0..10_000)
        .map(|i| {
            if i == 9_000 {
                vec!["Zebra".to_owned()]
            } else {
                vec![format!("Item {i:05}")]
            }
        })
        .collect();
    app.world_mut().spawn((
        panel(),
        children![table([Column::new("Name").width(12.0)], rows)],
    ));
    settle(&mut app);
    let table = the::<Table>(&mut app);
    focus_on(&mut app, table);
    type_text(&mut app, "zeb");
    assert_eq!(cursor(&mut app, table), Some(9_000));
    let shown = app
        .world_mut()
        .query::<&TableRow>()
        .iter(app.world())
        .any(|row| row.key == 9_000);
    assert!(shown, "the row is scrolled into the pool that draws it");
    assert_eq!(focused(&app), Some(table));
}

#[test]
fn typing_in_a_list_moves_focus_to_the_row_and_space_is_no_type_select_character() {
    let mut app = app();
    app.world_mut().spawn((
        panel(),
        children![list(
            4.5,
            children![
                button("Apple"),
                button("Cave"),
                button("Cube"),
                button("Desk"),
                button("Cedar"),
                button("Elm"),
            ]
        )],
    ));
    settle(&mut app);
    let apple = find_by_label(&mut app, "Apple").unwrap();
    focus_on(&mut app, apple);
    type_text(&mut app, "c");
    assert_eq!(focused(&app), find_by_label(&mut app, "Cave").ok());
    type_text(&mut app, "c");
    assert_eq!(focused(&app), find_by_label(&mut app, "Cube").ok());
    pause(&mut app);
    type_text(&mut app, "ce");
    assert_eq!(
        focused(&app),
        find_by_label(&mut app, "Cedar").ok(),
        "a two-letter prefix, the fifth row, scrolled to"
    );
    let scrolled = app
        .world_mut()
        .query_filtered::<&ScrollPosition, With<ListView>>()
        .single(app.world())
        .expect("one list")
        .y;
    assert!(
        scrolled > 0.0,
        "the list scrolled to the fifth row: {scrolled}"
    );
    // A space is no part of a prefix: from Apple, "c", " ", "e" is "ce", Cedar (with the space
    // nothing would match and focus would stay on Cave).
    pause(&mut app);
    let apple = find_by_label(&mut app, "Apple").unwrap();
    focus_on(&mut app, apple);
    type_text(&mut app, "c e");
    assert_eq!(
        focused(&app),
        find_by_label(&mut app, "Cedar").ok(),
        "the space was not typed into the prefix"
    );
}

#[test]
fn a_text_field_in_a_list_takes_the_characters() {
    let mut app = app();
    app.world_mut().spawn((
        panel(),
        children![list(
            4.5,
            children![text_input("Name", ""), button("Banana"), button("Cherry")]
        )],
    ));
    settle(&mut app);
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::ui_widgets::TextInput>>()
        .single(app.world())
        .expect("one text field");
    focus_on(&mut app, field);
    type_text(&mut app, "b");
    assert_eq!(focused(&app), Some(field), "focus stays in the field");
    let typed: Vec<String> = app
        .world_mut()
        .query::<&TextInputValue>()
        .iter(app.world())
        .map(|value| value.0.clone())
        .collect();
    assert_eq!(typed, vec!["b".to_owned()]);
}

#[test]
fn typing_in_an_open_menu_moves_to_the_item() {
    let mut app = app();
    app.world_mut()
        .spawn(screen())
        .with_child(menu_bar(children![menu_title(
            "File",
            children![
                menu_item("New"),
                menu_item("Open..."),
                menu_item("Save"),
                menu_item("Save as..."),
                menu_item("Quit"),
            ]
        )]));
    settle(&mut app);
    press(&mut app, KeyCode::F10);
    press(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), find_by_label(&mut app, "New").ok());
    type_text(&mut app, "s");
    assert_eq!(focused(&app), find_by_label(&mut app, "Save").ok());
    type_text(&mut app, "s");
    assert_eq!(focused(&app), find_by_label(&mut app, "Save as...").ok());
    pause(&mut app);
    type_text(&mut app, "q");
    assert_eq!(focused(&app), find_by_label(&mut app, "Quit").ok());
}

#[test]
fn typing_on_a_select_that_does_not_filter_steps_it_to_the_option_once() {
    let mut app = app();
    app.world_mut().spawn((
        panel(),
        children![select("Quality", ["Low", "Medium", "High", "Ultra"], 0)],
    ));
    settle(&mut app);
    let quality = the::<Select>(&mut app);
    focus_on(&mut app, quality);
    type_text(&mut app, "h");
    assert_eq!(
        app.world().get::<ChoiceValue>(quality).map(|v| v.0),
        Some(2)
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![2],
        "one change, as a step"
    );
    type_text(&mut app, "x");
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![2],
        "no match changes nothing"
    );
}
