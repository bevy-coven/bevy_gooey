//! The vertical tab strip: a sidebar's navigation whose tabs stand one above the other with their
//! labels at the start and a badge at the end, Up and Down on a tab moving and selecting once,
//! Left and Right leaving it for the page beside it, and pages that stand anywhere following its
//! `TabsValue`. Laid out in a headless window and driven by real key and pad input.

use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{Selected, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

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
    ));
    app.world_mut().spawn(Camera2d);
    app.init_resource::<Heard>();
    app
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// Every `ValueChange<usize>` the strip sent.
#[derive(Resource, Default)]
struct Heard(Vec<usize>);

/// A sidebar 240 px wide holding the strip, and a pane beside it holding the three pages, each a
/// button: `(strip, tabs, pages, page buttons)`.
struct Sidebar {
    strip: Entity,
    tabs: Vec<Entity>,
    pages: Vec<Entity>,
    buttons: Vec<Entity>,
}

fn sidebar(app: &mut App) -> Sidebar {
    let world = app.world_mut();
    let strip = world
        .spawn(side_tabs(
            [
                "Projects".into(),
                "Learn".into(),
                SideTab::new("What's new").badge("3"),
            ],
            0,
        ))
        .observe(|change: On<ValueChange<usize>>, mut heard: ResMut<Heard>| {
            heard.0.push(change.value);
        })
        .id();
    let side = world
        .spawn(Node {
            width: Val::Px(240.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .add_child(strip)
        .id();
    let pane = world
        .spawn(Node {
            flex_grow: 1.0,
            ..default()
        })
        .id();
    let mut pages = Vec::new();
    let mut buttons = Vec::new();
    for (index, name) in ["New project", "Tutorials", "Notes"]
        .into_iter()
        .enumerate()
    {
        let button = world.spawn(button(name)).id();
        let page = world
            .spawn(side_tab_page(strip, index, ()))
            .add_child(button)
            .id();
        world.entity_mut(pane).add_child(page);
        pages.push(page);
        buttons.push(button);
    }
    let root = world
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            column_gap: Val::Px(24.0),
            ..default()
        })
        .add_children(&[side, pane])
        .id();
    world.spawn(screen()).add_child(root);
    settle(app);
    let list = app.world().get::<Children>(strip).expect("the list")[0];
    let tabs = app
        .world()
        .get::<Children>(list)
        .expect("tabs")
        .iter()
        .collect();
    Sidebar {
        strip,
        tabs,
        pages,
        buttons,
    }
}

/// A node's rectangle in the window, logical px: `(left, top, right, bottom)`.
fn rect(app: &App, entity: Entity) -> (f32, f32, f32, f32) {
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
    (
        at.x - size.x / 2.0,
        at.y - size.y / 2.0,
        at.x + size.x / 2.0,
        at.y + size.y / 2.0,
    )
}

fn tap(app: &mut App, key_code: KeyCode) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("a window");
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

fn shown(app: &App, page: Entity) -> bool {
    app.world().get::<Visibility>(page) == Some(&Visibility::Inherited)
}

#[test]
fn the_tabs_stand_one_above_the_other_as_wide_as_the_strip_labels_first_badge_last() {
    let mut app = app();
    let ui = sidebar(&mut app);
    let rects: Vec<_> = ui.tabs.iter().map(|&t| rect(&app, t)).collect();
    for pair in rects.windows(2) {
        assert!(
            pair[1].1 >= pair[0].3 - 0.5,
            "one above the other: {pair:?}"
        );
        assert!((pair[1].0 - pair[0].0).abs() <= 0.5, "from one edge");
    }
    for r in &rects {
        assert!(
            (r.2 - r.0 - 240.0).abs() <= 1.0,
            "as wide as the strip: {r:?}"
        );
    }
    let skin = GooeySkin::default();
    let parts = app.world().get::<Children>(ui.tabs[2]).expect("parts");
    let label = rect(&app, parts[0]);
    let badge = rect(&app, parts[1]);
    let tab = rects[2];
    assert!(
        label.0 - tab.0 <= skin.padding * 2.0,
        "the label at the start: {label:?} in {tab:?}"
    );
    assert!(
        tab.2 - badge.2 <= skin.padding * 2.0 && badge.0 > label.2,
        "the badge at the end: {badge:?} in {tab:?}"
    );
    // The badge's count is in the tab's name.
    let words = app.world().get::<Text>(parts[1]).map(|t| t.0.clone());
    assert_eq!(words.as_deref(), Some("3"));
}

#[test]
fn up_and_down_on_a_tab_select_once_and_the_page_beside_follows() {
    let mut app = app();
    let ui = sidebar(&mut app);
    assert!(shown(&app, ui.pages[0]) && !shown(&app, ui.pages[1]) && !shown(&app, ui.pages[2]));
    assert!(app.world().get::<Selected>(ui.tabs[0]).is_some());
    focus_on(&mut app, ui.tabs[0]);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(ui.tabs[1]), "one tab down, not two");
    assert_eq!(app.world().get::<TabsValue>(ui.strip), Some(&TabsValue(1)));
    assert_eq!(app.world().resource::<Heard>().0, vec![1]);
    assert!(shown(&app, ui.pages[1]) && !shown(&app, ui.pages[0]));
    assert!(app.world().get::<Selected>(ui.tabs[1]).is_some());
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(focused(&app), Some(ui.tabs[0]));
    assert_eq!(app.world().resource::<Heard>().0, vec![1, 0]);
    assert!(shown(&app, ui.pages[0]));
}

#[test]
fn left_and_right_leave_the_strip_for_the_page_and_back_to_the_selected_tab() {
    let mut app = app();
    let ui = sidebar(&mut app);
    focus_on(&mut app, ui.tabs[0]);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(focused(&app), Some(ui.buttons[0]), "into the page beside");
    assert_eq!(app.world().get::<TabsValue>(ui.strip), Some(&TabsValue(0)));
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(focused(&app), Some(ui.tabs[0]), "back to the selected tab");
    assert!(app.world().resource::<Heard>().0.is_empty());
}

#[test]
fn a_page_the_game_turns_away_from_gives_focus_to_the_page_shown() {
    let mut app = app();
    let ui = sidebar(&mut app);
    focus_on(&mut app, ui.buttons[0]);
    app.world_mut().entity_mut(ui.strip).insert(TabsValue(2));
    settle(&mut app);
    assert!(shown(&app, ui.pages[2]) && !shown(&app, ui.pages[0]));
    assert_eq!(
        focused(&app),
        Some(ui.buttons[2]),
        "focus on the page shown"
    );
    assert!(
        app.world().resource::<Heard>().0.is_empty(),
        "no event for the game's turn"
    );
    // A page spawned later stands hidden or shown by the value it finds.
    let late = app.world_mut().spawn(side_tab_page(ui.strip, 2, ())).id();
    let early = app.world_mut().spawn(side_tab_page(ui.strip, 0, ())).id();
    settle(&mut app);
    assert!(shown(&app, late) && !shown(&app, early));
}

#[test]
fn the_pads_up_and_down_step_a_vertical_tab() {
    let mut app = app();
    let ui = sidebar(&mut app);
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
    focus_on(&mut app, ui.tabs[0]);
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad,
                GamepadButton::DPadDown,
                value,
            )));
        app.update();
    }
    settle(&mut app);
    assert_eq!(app.world().get::<TabsValue>(ui.strip), Some(&TabsValue(1)));
    assert_eq!(focused(&app), Some(ui.tabs[1]), "one step");
    assert_eq!(app.world().resource::<Heard>().0, vec![1]);
}
