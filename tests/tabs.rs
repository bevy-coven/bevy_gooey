//! Tabs: a strip of labels over pages of widgets. They keep their state, show one page, turn it for
//! a click, the arrows, the D-pad, the bumpers and Ctrl+Page, never leave focus where it cannot be
//! seen, and never move under the player's thumb. Laid out for real in a headless window, driven by
//! real keyboard, pointer and gamepad input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{Selected, UiGlobalTransform};
use bevy::ui_widgets::{SelectedTab, SliderValue};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<usize>` a game would see, from the tabs and from a choice alike.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, usize)>);

/// Every `UiFeedback` written.
#[derive(Resource, Default)]
struct Feedback(Vec<UiFeedback>);

/// Every button activated.
#[derive(Resource, Default)]
struct Activations(Vec<Entity>);

/// How many frames' writes the app has seen to what a page turn changes.
#[derive(Resource, Default)]
struct Writes(u32);

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
    .init_resource::<Feedback>()
    .init_resource::<Activations>()
    .init_resource::<Writes>()
    .add_observer(
        |change: On<ValueChange<usize>>, mut seen: ResMut<Changes>| {
            seen.0.push((change.source, change.value))
        },
    )
    .add_observer(|activate: On<Activate>, mut seen: ResMut<Activations>| {
        seen.0.push(activate.entity)
    })
    .add_systems(
        Update,
        (
            |mut messages: MessageReader<UiFeedback>, mut seen: ResMut<Feedback>| {
                seen.0.extend(messages.read().copied());
            },
            |changed: Query<
                (),
                Or<(
                    Changed<Visibility>,
                    Changed<SelectedTab>,
                    Changed<TabsValue>,
                    Changed<BackgroundColor>,
                    Changed<BorderColor>,
                    Changed<Node>,
                )>,
            >,
             mut writes: ResMut<Writes>| writes.0 += changed.iter().count() as u32,
        ),
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

/// A settings screen as a game builds it: three tabs over three pages, and a Back button under
/// them, in a panel on a screen.
struct Settings {
    root: Entity,
    tabs: Vec<Entity>,
    pages: Vec<Entity>,
    volume: Entity,
    subtitles: Entity,
    quality: Entity,
    rebind: Entity,
    back: Entity,
}

fn settings(app: &mut App) -> Settings {
    settings_of(app, 0)
}

fn settings_of(app: &mut App, selected: usize) -> Settings {
    let card = app
        .world_mut()
        .spawn((
            panel(),
            children![
                heading("Settings"),
                tabs(
                    ["Audio", "Display", "Controls"],
                    selected,
                    children![
                        tab_page(children![
                            slider("Volume", 0.7, 0.0..=1.0),
                            toggle("Subtitles", true)
                        ]),
                        tab_page(children![choice("Quality", ["Low", "Medium", "High"], 1)]),
                        tab_page(children![button("Rebind keys")]),
                    ]
                ),
                button("Back"),
            ],
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(app);
    let root = part(app, card, Role::Tabs);
    let back = *children_of(app.world(), card)
        .iter()
        .find(|&&c| app.world().get::<Role>(c) == Some(&Role::Button))
        .expect("a Back button");
    Settings {
        root,
        tabs: parts(app, root, Role::Tab),
        pages: pages(app, root),
        volume: part(app, root, Role::Slider),
        subtitles: part(app, root, Role::Toggle),
        quality: part(app, root, Role::Choice),
        rebind: part(app, root, Role::Button),
        back,
    }
}

fn children_of(world: &World, entity: Entity) -> Vec<Entity> {
    world
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        for child in children_of(world, entity) {
            found.push(child);
            stack.push(child);
        }
    }
    found
}

/// Every part of `root` with `role`, left to right.
fn parts(app: &App, root: Entity, role: Role) -> Vec<Entity> {
    let mut found: Vec<Entity> = descendants(app.world(), root)
        .into_iter()
        .filter(|&e| app.world().get::<Role>(e) == Some(&role))
        .collect();
    found.sort_by(|&a, &b| rect(app, a).center().x.total_cmp(&rect(app, b).center().x));
    found
}

fn part(app: &App, root: Entity, role: Role) -> Entity {
    *parts(app, root, role)
        .first()
        .unwrap_or_else(|| panic!("a {role:?} under the widget"))
}

/// The pages of a tabs widget, in the order they were given.
fn pages(app: &App, root: Entity) -> Vec<Entity> {
    let container = part(app, root, Role::TabPages);
    children_of(app.world(), container)
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

fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    let window = window(app);
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

fn tap(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Pressed);
    key(app, key_code, ButtonState::Released);
}

/// Holds `held` while `key_code` is tapped, then lets both go.
fn chord(app: &mut App, held: KeyCode, key_code: KeyCode) {
    key(app, held, ButtonState::Pressed);
    tap(app, key_code);
    key(app, held, ButtonState::Released);
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

fn click_at(app: &mut App, at: Vec2) {
    point_at(app, at);
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
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

fn pad_button(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
    app.world_mut()
        .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            pad, button, value,
        )));
    app.update();
}

/// Presses and releases `button` on `pad`, one update each, then lets the screen settle.
fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    pad_button(app, pad, button, 1.0);
    pad_button(app, pad, button, 0.0);
    settle(app);
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    app.update();
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn ring_visible(app: &App) -> bool {
    app.world().resource::<InputFocusVisible>().0
}

fn selected(app: &App, ui: &Settings) -> usize {
    app.world().get::<TabsValue>(ui.root).expect("a value").0
}

fn set_page(app: &mut App, ui: &Settings, page: usize) {
    app.world_mut()
        .get_mut::<TabsValue>(ui.root)
        .expect("a value")
        .0 = page;
    settle(app);
}

/// The index of the page that is shown, or panics if it is not exactly one.
fn shown(app: &App, ui: &Settings) -> usize {
    let on: Vec<usize> = ui
        .pages
        .iter()
        .enumerate()
        .filter(|&(_, &page)| app.world().get::<InheritedVisibility>(page).is_some_and(|v| v.get()))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(on.len(), 1, "exactly one page is shown, not {on:?}");
    on[0]
}

/// The tabs lit as selected.
fn lit(app: &App, ui: &Settings) -> Vec<usize> {
    ui.tabs
        .iter()
        .enumerate()
        .filter(|&(_, &tab)| app.world().get::<Selected>(tab).is_some())
        .map(|(index, _)| index)
        .collect()
}

fn changes_of(app: &App, root: Entity) -> Vec<usize> {
    app.world()
        .resource::<Changes>()
        .0
        .iter()
        .filter(|&&(source, _)| source == root)
        .map(|&(_, value)| value)
        .collect()
}

fn activations(app: &App) -> usize {
    app.world()
        .resource::<Feedback>()
        .0
        .iter()
        .filter(|f| f.kind == UiFeedbackKind::Activate)
        .count()
}

fn focus_feedback_on(app: &App, entity: Entity) -> usize {
    app.world()
        .resource::<Feedback>()
        .0
        .iter()
        .filter(|f| f.kind == UiFeedbackKind::Focus && f.entity == entity)
        .count()
}

fn background(app: &App, entity: Entity) -> Color {
    app.world().get::<BackgroundColor>(entity).expect("a fill").0
}

fn label_colour(app: &App, tab: Entity) -> Color {
    let text = children_of(app.world(), tab)[0];
    app.world().get::<TextColor>(text).expect("a colour").0
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

/// The widgets Tab, the arrows or a click could reach on the pages that are not shown.
fn hidden_stops(app: &App, ui: &Settings) -> Vec<Entity> {
    let on = shown_index(app, ui);
    ui.pages
        .iter()
        .enumerate()
        .filter(|&(index, _)| index != on)
        .flat_map(|(_, &page)| descendants(app.world(), page))
        .filter(|&e| app.world().get::<TabIndex>(e).is_some())
        .collect()
}

fn shown_index(app: &App, ui: &Settings) -> usize {
    app.world().get::<TabsValue>(ui.root).expect("a value").0
}

fn first_widget(ui: &Settings, page: usize) -> Entity {
    [ui.volume, ui.quality, ui.rebind][page]
}

// ---- the state and the look ----

#[test]
fn the_first_page_shows_and_the_others_wait() {
    let mut app = app();
    let ui = settings(&mut app);
    assert_eq!(ui.tabs.len(), 3, "three tabs");
    assert_eq!(ui.pages.len(), 3, "three pages");
    assert_eq!(selected(&app, &ui), 0);
    assert_eq!(shown(&app, &ui), 0, "the first page is shown, the rest hidden");
    assert_eq!(lit(&app, &ui), [0], "only the first tab is lit");
    assert!(
        app.world().resource::<Changes>().0.is_empty(),
        "spawning says nothing"
    );
    // A page that is not shown holds no tab stop anyone can reach.
    for stop in hidden_stops(&app, &ui) {
        assert!(
            !app.world().get::<InheritedVisibility>(stop).expect("seen").get(),
            "{stop:?} on a page that is not shown is hidden"
        );
    }
}

#[test]
fn the_page_a_game_picks_shows_first_and_a_value_past_the_end_shows_the_last() {
    let mut app = app();
    let ui = settings_of(&mut app, 2);
    assert_eq!((selected(&app, &ui), shown(&app, &ui)), (2, 2));
    assert_eq!(lit(&app, &ui), [2]);

    let mut app = app_sized(1280, 720);
    let ui = settings(&mut app);
    set_page(&mut app, &ui, 9);
    assert_eq!(selected(&app, &ui), 2, "pulled back to the last tab");
    assert_eq!(shown(&app, &ui), 2);
    assert_eq!(lit(&app, &ui), [2]);
    assert!(
        app.world().resource::<Changes>().0.is_empty(),
        "a change the game made says nothing"
    );
}

#[test]
fn turning_a_page_moves_nothing() {
    let mut app = app();
    let ui = settings(&mut app);
    let strip = part(&app, ui.root, Role::TabList);
    let before = (
        rect(&app, ui.root),
        rect(&app, strip),
        ui.tabs.iter().map(|&t| rect(&app, t)).collect::<Vec<_>>(),
    );
    let page0 = rect(&app, ui.pages[0]);
    for page in [1, 2, 0] {
        set_page(&mut app, &ui, page);
        let at = format!("page {page}");
        assert_eq!(rect(&app, ui.root), before.0, "{at}: the widget");
        assert_eq!(rect(&app, strip), before.1, "{at}: the strip");
        for (i, &tab) in ui.tabs.iter().enumerate() {
            assert_eq!(rect(&app, tab), before.2[i], "{at}: tab {i}");
        }
        for (i, &p) in ui.pages.iter().enumerate() {
            assert_eq!(rect(&app, p), page0, "{at}: page {i} is as large as the others");
        }
    }
    // The pages stand under the strip, a gap clear of it.
    let gap = GooeySkin::default().gap;
    assert!(
        (page0.min.y - rect(&app, strip).max.y - gap).abs() <= 1.0,
        "the page stands a gap below the strip"
    );
}

#[test]
fn tabs_are_equal_targets_a_thumb_can_hit() {
    for (width, height, least) in [(1280, 720, 44.0), (2560, 1440, 88.0)] {
        let mut app = app_sized(width, height);
        let ui = settings(&mut app);
        let strip = rect(&app, part(&app, ui.root, Role::TabList));
        let sizes: Vec<Vec2> = ui.tabs.iter().map(|&t| rect(&app, t).size()).collect();
        for (i, size) in sizes.iter().enumerate() {
            assert!(size.y >= least, "{width}x{height}: tab {i} is {} px high", size.y);
            assert!(size.x >= least, "{width}x{height}: tab {i} is {} px wide", size.x);
            assert!(
                (size.x - sizes[0].x).abs() <= 1.0,
                "{width}x{height}: the tabs share the strip equally"
            );
        }
        let inside = ui.tabs.iter().all(|&t| {
            let r = rect(&app, t);
            r.min.x >= strip.min.x - 1.0 && r.max.x <= strip.max.x + 1.0
        });
        assert!(inside, "{width}x{height}: the tabs stay inside the strip");
    }
}

#[test]
fn the_selected_tab_is_lit_and_the_others_stay_readable_in_both_skins() {
    for skin in [GooeySkin::dark(), GooeySkin::parchment()] {
        assert!(skin.problems().is_empty(), "the preset holds its own rules");
        let mut app = app();
        app.insert_resource(skin.clone());
        let ui = settings(&mut app);
        let on = background(&app, ui.tabs[0]);
        let off = background(&app, ui.tabs[1]);
        assert_eq!(on, skin.accent, "the selected tab is the accent");
        assert_eq!(off, skin.surface_raised, "the others are a button's grey");
        assert_eq!(label_colour(&app, ui.tabs[0]), skin.accent_text);
        assert_eq!(label_colour(&app, ui.tabs[1]), skin.text);
        // Two cues by luminance: the fills differ by 3:1, and every label reads at 4.5:1 on its own.
        assert!(contrast_ratio(rgb(on), rgb(off)) >= NON_TEXT_CONTRAST, "selected against the rest");
        assert!(contrast_ratio(rgb(skin.accent_text), rgb(on)) >= TEXT_CONTRAST);
        assert!(contrast_ratio(rgb(skin.text), rgb(off)) >= TEXT_CONTRAST);
        // Choosing another tab swaps them.
        set_page(&mut app, &ui, 1);
        assert_eq!(background(&app, ui.tabs[1]), skin.accent);
        assert_eq!(background(&app, ui.tabs[0]), skin.surface_raised);
        assert_eq!(label_colour(&app, ui.tabs[1]), skin.accent_text);
        assert_eq!(label_colour(&app, ui.tabs[0]), skin.text);
    }
}

#[test]
fn the_pointer_lights_a_tab_that_is_not_selected() {
    let mut app = app();
    let ui = settings(&mut app);
    let skin = GooeySkin::default();
    let at = rect(&app, ui.tabs[2]).center();
    point_at(&mut app, at);
    settle(&mut app);
    assert_eq!(background(&app, ui.tabs[2]), skin.hovered());
    assert_eq!(background(&app, ui.tabs[0]), skin.accent, "the selected tab stays lit");
}

#[test]
fn a_tabs_focus_ring_is_one_whole_line_on_every_side_at_every_scale() {
    for (width, height, device) in [(1280, 720, 3.0), (1920, 1080, 4.0), (2560, 1440, 6.0)] {
        let mut app = app_sized(width, height);
        let ui = settings(&mut app);
        focus(&mut app, ui.tabs[0]);
        settle(&mut app);
        for &tab in &ui.tabs {
            let node = app.world().get::<ComputedNode>(tab).expect("laid out");
            assert_eq!(
                (node.border.min_inset, node.border.max_inset),
                (Vec2::splat(device), Vec2::splat(device)),
                "{width}x{height}: a tab's border"
            );
        }
        let ring = *app.world().get::<BorderColor>(ui.tabs[0]).expect("a border");
        assert_eq!(ring, BorderColor::all(GooeySkin::default().focus_ring));
        let quiet = *app.world().get::<BorderColor>(ui.tabs[1]).expect("a border");
        assert_eq!(quiet, BorderColor::all(Color::NONE), "no ring on the others");
    }
}

// ---- the pointer ----

#[test]
fn a_click_on_a_tab_turns_the_page_and_says_so_once() {
    let mut app = app();
    let ui = settings(&mut app);
    let at = rect(&app, ui.tabs[2]).center();
    click_at(&mut app, at);
    assert_eq!(selected(&app, &ui), 2);
    assert_eq!(shown(&app, &ui), 2);
    assert_eq!(lit(&app, &ui), [2]);
    assert_eq!(changes_of(&app, ui.root), [2], "one change, with the place of the tab");
    assert_eq!(activations(&app), 1, "one answer for the sound");
    assert_eq!(focused(&app), Some(ui.tabs[2]), "focus is on the tab clicked");
    assert!(!ring_visible(&app), "a pointer press hides the ring");

    // A click on the tab already selected is no change.
    let at = rect(&app, ui.tabs[2]).center();
    click_at(&mut app, at);
    assert_eq!(changes_of(&app, ui.root), [2]);
    assert_eq!(activations(&app), 1);
}

#[test]
fn a_page_that_is_shown_takes_the_pointer_and_the_ones_under_it_do_not() {
    let mut app = app();
    let ui = settings(&mut app);
    set_page(&mut app, &ui, 2);
    // The Volume slider (page 0) lies under the Rebind button (page 2): same cell, same place.
    let volume_at = app.world().get::<SliderValue>(ui.volume).expect("a value").0;
    let at = rect(&app, ui.rebind).center();
    click_at(&mut app, at);
    assert_eq!(
        app.world().resource::<Activations>().0,
        [ui.rebind],
        "the button on the shown page was pressed"
    );
    assert_eq!(
        app.world().get::<SliderValue>(ui.volume).expect("a value").0,
        volume_at,
        "the slider on the page under it was not"
    );
}

// ---- the keyboard ----

#[test]
fn left_and_right_on_a_tab_turn_the_page_round_the_strip_and_focus_goes_with_it() {
    let mut app = app();
    let ui = settings(&mut app);
    focus(&mut app, ui.tabs[0]);

    tap(&mut app, KeyCode::ArrowLeft);
    settle(&mut app);
    assert_eq!(selected(&app, &ui), 2, "Left from the first tab goes to the last");
    assert_eq!(focused(&app), Some(ui.tabs[2]), "one press moved focus once");
    assert_eq!((shown(&app, &ui), lit(&app, &ui)), (2, vec![2]));

    tap(&mut app, KeyCode::ArrowRight);
    settle(&mut app);
    assert_eq!(selected(&app, &ui), 0, "Right from the last tab goes to the first");
    assert_eq!(focused(&app), Some(ui.tabs[0]));

    tap(&mut app, KeyCode::End);
    settle(&mut app);
    assert_eq!((selected(&app, &ui), focused(&app)), (2, Some(ui.tabs[2])));
    tap(&mut app, KeyCode::Home);
    settle(&mut app);
    assert_eq!((selected(&app, &ui), focused(&app)), (0, Some(ui.tabs[0])));

    assert_eq!(changes_of(&app, ui.root), [2, 0, 2, 0], "each turn said once");
    assert!(ring_visible(&app), "keys show the ring");
    assert_eq!(focus_feedback_on(&app, ui.tabs[2]), 2);
}

#[test]
fn up_leaves_the_strip_for_what_is_above_and_down_enters_the_page() {
    let mut app = app();
    let ui = settings(&mut app);
    focus(&mut app, ui.tabs[0]);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(ui.volume), "Down goes into the page shown");
    assert_eq!(selected(&app, &ui), 0, "and turns nothing");
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(focused(&app), Some(ui.tabs[0]), "Up comes back to the strip");
}

#[test]
fn up_from_a_page_lands_on_the_selected_tab_not_the_nearest() {
    let mut app = app();
    // A page whose only widget is a button at its left edge: the nearest tab above is the first.
    let card = app
        .world_mut()
        .spawn((
            panel(),
            children![tabs(
                ["One", "Two", "Three"],
                2,
                children![
                    tab_page(children![(Node::default(), children![button("A")])]),
                    tab_page(children![(Node::default(), children![button("B")])]),
                    tab_page(children![(Node::default(), children![button("C")])]),
                ]
            )],
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);
    let tabs_ = parts(&app, card, Role::Tab);
    let buttons: Vec<Entity> = descendants(app.world(), card)
        .into_iter()
        .filter(|&e| app.world().get::<Role>(e) == Some(&Role::Button))
        .collect();
    let c = *buttons
        .iter()
        .find(|&&b| {
            let text = children_of(app.world(), b)[0];
            app.world().get::<Text>(text).is_some_and(|t| t.0 == "C")
        })
        .expect("the third page's button");
    assert!(
        rect(&app, c).center().x < rect(&app, tabs_[1]).center().x,
        "the button stands under the first tab, not the third"
    );
    focus(&mut app, c);
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(
        focused(&app),
        Some(tabs_[2]),
        "focus enters the strip on the selected tab"
    );
    // And it was not turned by getting there.
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn tab_and_the_arrows_never_land_on_a_page_that_is_not_shown() {
    let mut app = app();
    let ui = settings(&mut app);
    let hidden = hidden_stops(&app, &ui);
    assert!(!hidden.is_empty(), "there are widgets on the other pages");
    focus(&mut app, ui.tabs[0]);
    for step in 0..8 {
        tap(&mut app, KeyCode::Tab);
        let at = focused(&app).expect("focus stays on something");
        assert!(!hidden.contains(&at), "Tab step {step} landed on a hidden page");
    }
    for key_code in [
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowUp,
        KeyCode::ArrowUp,
        KeyCode::ArrowUp,
        KeyCode::ArrowUp,
    ] {
        tap(&mut app, key_code);
        let at = focused(&app).expect("focus stays on something");
        assert!(!hidden.contains(&at), "{key_code:?} landed on a hidden page");
    }
    // Down from the last widget of the page reaches Back, which is outside the tabs.
    focus(&mut app, ui.subtitles);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(ui.back));
}

// ---- the pad ----

#[test]
fn the_bumpers_turn_the_page_from_inside_it_and_focus_lands_on_the_new_page() {
    let mut app = app();
    let ui = settings(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.volume);

    press(&mut app, pad, GamepadButton::RightTrigger);
    assert_eq!(selected(&app, &ui), 1, "RB is the next tab");
    assert_eq!(shown(&app, &ui), 1);
    assert_eq!(
        focused(&app),
        Some(ui.quality),
        "focus is on the new page's first widget"
    );
    assert!(ring_visible(&app), "and the ring shows it");

    press(&mut app, pad, GamepadButton::RightTrigger);
    assert_eq!((selected(&app, &ui), focused(&app)), (2, Some(ui.rebind)));
    press(&mut app, pad, GamepadButton::RightTrigger);
    assert_eq!(
        (selected(&app, &ui), focused(&app)),
        (0, Some(ui.volume)),
        "RB from the last tab goes round to the first"
    );
    press(&mut app, pad, GamepadButton::LeftTrigger);
    assert_eq!((selected(&app, &ui), focused(&app)), (2, Some(ui.rebind)), "LB goes back, round");
    assert_eq!(changes_of(&app, ui.root), [1, 2, 0, 2]);
    assert_eq!(activations(&app), 4, "a turn is one answer each");
    // The triggers are the list's: they turn no page.
    press(&mut app, pad, GamepadButton::RightTrigger2);
    press(&mut app, pad, GamepadButton::LeftTrigger2);
    assert_eq!(selected(&app, &ui), 2);
}

#[test]
fn the_bumpers_with_focus_on_a_tab_keep_focus_on_the_strip() {
    let mut app = app();
    let ui = settings(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.tabs[0]);
    press(&mut app, pad, GamepadButton::RightTrigger);
    assert_eq!((selected(&app, &ui), focused(&app)), (1, Some(ui.tabs[1])));
    press(&mut app, pad, GamepadButton::LeftTrigger);
    press(&mut app, pad, GamepadButton::LeftTrigger);
    assert_eq!((selected(&app, &ui), focused(&app)), (2, Some(ui.tabs[2])));
}

#[test]
fn a_page_with_nothing_to_focus_takes_focus_to_the_selected_tab() {
    let mut app = app();
    let card = app
        .world_mut()
        .spawn((
            panel(),
            children![tabs(
                ["Game", "Credits"],
                0,
                children![
                    tab_page(children![button("Resume")]),
                    tab_page(children![label("Made with Bevy")]),
                ]
            )],
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);
    let tabs_ = parts(&app, card, Role::Tab);
    let resume = part(&app, card, Role::Button);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, resume);
    press(&mut app, pad, GamepadButton::RightTrigger);
    assert_eq!(focused(&app), Some(tabs_[1]), "no widget on the page: the tab it belongs to");
}

#[test]
fn the_bumpers_reach_tabs_from_a_button_beside_them_only_when_there_is_one_to_turn() {
    let mut app = app();
    let ui = settings(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.back);
    press(&mut app, pad, GamepadButton::RightTrigger);
    assert_eq!(selected(&app, &ui), 1, "Back is outside the tabs, and they are the only ones in view");
    assert_eq!(focused(&app), Some(ui.back), "focus did not move");

    // With nothing focused there is nothing to turn from.
    app.world_mut().resource_mut::<InputFocus>().clear();
    press(&mut app, pad, GamepadButton::RightTrigger);
    assert_eq!(selected(&app, &ui), 1);

    // A second tabs widget in view makes the choice the player's: focus outside both turns neither.
    let other = app
        .world_mut()
        .spawn(tabs(["X", "Y"], 0, children![tab_page(children![]), tab_page(children![])]))
        .id();
    app.world_mut()
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        })
        .add_child(other);
    settle(&mut app);
    focus(&mut app, ui.back);
    press(&mut app, pad, GamepadButton::RightTrigger);
    assert_eq!(selected(&app, &ui), 1, "two to choose from: neither turns");
    assert_eq!(app.world().get::<TabsValue>(other).expect("a value").0, 0);
    // Focus inside one of them turns that one.
    focus(&mut app, ui.volume);
    press(&mut app, pad, GamepadButton::LeftTrigger);
    assert_eq!(selected(&app, &ui), 0);
    assert_eq!(app.world().get::<TabsValue>(other).expect("a value").0, 0);
}

#[test]
fn an_open_dialog_keeps_the_bumpers_off_the_tabs_behind_it() {
    let mut app = app();
    let ui = settings(&mut app);
    let pad = plug_in_pad(&mut app);
    let leave = app.world_mut().spawn(button("Leave")).id();
    let card = app.world_mut().spawn(panel()).add_child(leave).id();
    app.world_mut().spawn(dialog()).add_child(card);
    settle(&mut app);
    assert_eq!(focused(&app), Some(leave), "the dialog took focus");
    press(&mut app, pad, GamepadButton::RightTrigger);
    assert_eq!(selected(&app, &ui), 0, "the tabs behind the scrim are not the dialog's");
}

#[test]
fn the_d_pad_steps_a_focused_tab_and_the_arrows_of_the_pad_never_leave_the_strip() {
    let mut app = app();
    let ui = settings(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.tabs[0]);
    press(&mut app, pad, GamepadButton::DPadRight);
    assert_eq!((selected(&app, &ui), focused(&app)), (1, Some(ui.tabs[1])));
    press(&mut app, pad, GamepadButton::DPadLeft);
    press(&mut app, pad, GamepadButton::DPadLeft);
    assert_eq!((selected(&app, &ui), focused(&app)), (2, Some(ui.tabs[2])), "wraps");
    assert_eq!(changes_of(&app, ui.root), [1, 0, 2], "each press one turn");
    press(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(focused(&app), Some(ui.rebind), "Down goes into the page shown");
    assert_eq!(selected(&app, &ui), 2);
}

// ---- Ctrl+Page ----

#[test]
fn ctrl_page_down_and_up_turn_the_page_and_the_page_keys_alone_do_not() {
    let mut app = app();
    let ui = settings(&mut app);
    focus(&mut app, ui.volume);

    tap(&mut app, KeyCode::PageDown);
    settle(&mut app);
    assert_eq!(selected(&app, &ui), 0, "PageDown is the list's");

    chord(&mut app, KeyCode::ControlLeft, KeyCode::PageDown);
    settle(&mut app);
    assert_eq!((selected(&app, &ui), focused(&app)), (1, Some(ui.quality)));
    chord(&mut app, KeyCode::ControlRight, KeyCode::PageUp);
    settle(&mut app);
    assert_eq!((selected(&app, &ui), focused(&app)), (0, Some(ui.volume)));
    chord(&mut app, KeyCode::ControlLeft, KeyCode::PageUp);
    settle(&mut app);
    assert_eq!(selected(&app, &ui), 2, "round the strip");
    assert_eq!(changes_of(&app, ui.root), [1, 0, 2]);
}

#[test]
fn ctrl_page_down_turns_the_page_and_never_also_pages_a_list() {
    let mut app = app();
    let card = app
        .world_mut()
        .spawn((
            panel(),
            children![tabs(
                ["Saves", "Other"],
                0,
                children![
                    tab_page(children![list(
                        4.5,
                        Children::spawn(bevy::ecs::spawn::SpawnIter(
                            (0..12).map(|i| button(format!("Slot {i}")))
                        ))
                    )]),
                    tab_page(children![button("Other")]),
                ]
            )],
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);
    let rows = parts(&app, card, Role::Button);
    let slot0 = *rows.iter().min_by(|&&a, &&b| rect(&app, a).min.y.total_cmp(&rect(&app, b).min.y)).expect("rows");
    focus(&mut app, slot0);
    chord(&mut app, KeyCode::ControlLeft, KeyCode::PageDown);
    settle(&mut app);
    let tabs_ = parts(&app, card, Role::TabList);
    assert_eq!(
        app.world().get::<TabsValue>(part(&app, card, Role::Tabs)).expect("a value").0,
        1
    );
    let view = part(&app, card, Role::ListView);
    assert_eq!(
        app.world().get::<ScrollPosition>(view).expect("a scroll").y,
        0.0,
        "the list was not paged"
    );
    let _ = tabs_;
}

// ---- code ----

#[test]
fn a_page_set_from_code_shows_in_the_same_update_says_nothing_and_pulls_focus_out_of_the_old_page() {
    let mut app = app();
    let ui = settings(&mut app);
    focus(&mut app, ui.subtitles);
    app.world_mut().get_mut::<TabsValue>(ui.root).expect("a value").0 = 1;
    app.update();
    assert_eq!(shown(&app, &ui), 1, "the page turned in the update the value changed");
    assert_eq!(lit(&app, &ui), [1]);
    settle(&mut app);
    assert_eq!(focused(&app), Some(ui.quality), "focus was in the page that went away");
    assert!(app.world().resource::<Changes>().0.is_empty(), "the game made it, so it says nothing");
    assert_eq!(activations(&app), 0);

    // Focus outside the tabs is left alone.
    focus(&mut app, ui.back);
    set_page(&mut app, &ui, 2);
    assert_eq!(focused(&app), Some(ui.back));
}

#[test]
fn a_focus_the_pointer_set_is_never_taken_from_a_tab() {
    let mut app = app();
    let ui = settings(&mut app);
    let at = rect(&app, ui.tabs[1]).center();
    click_at(&mut app, at);
    assert_eq!(focused(&app), Some(ui.tabs[1]));
    assert!(!ring_visible(&app));
    settle(&mut app);
    assert_eq!(focused(&app), Some(ui.tabs[1]), "the tab keeps focus, ring hidden");
    assert_eq!(first_widget(&ui, 1), ui.quality);
}

// ---- cost ----

#[test]
fn idle_tabs_write_nothing() {
    let mut app = app();
    let ui = settings(&mut app);
    focus(&mut app, ui.tabs[0]);
    settle(&mut app);
    app.world_mut().insert_resource(Writes::default());
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0, "no tab, page or fill was written");
    let _ = ui;
}

#[test]
fn a_long_label_widens_the_strip_and_is_never_squeezed_out_of_its_tab() {
    for (width, height) in [(1280, 720), (2560, 1440)] {
        let mut app = app_sized(width, height);
        // Pages narrower than the labels: the strip, not the pages, sets the widget's width.
        let card = app
            .world_mut()
            .spawn((
                panel(),
                children![tabs(
                    ["Graphics", "Accessibility", "Controls"],
                    0,
                    children![
                        tab_page(children![label("A")]),
                        tab_page(children![label("B")]),
                        tab_page(children![label("C")]),
                    ]
                )],
            ))
            .id();
        app.world_mut().spawn(screen()).add_child(card);
        settle(&mut app);
        let scale = height as f32 / 720.0;
        let pad = GooeySkin::default().padding * 1.5 * scale;
        let strip = rect(&app, part(&app, card, Role::TabList));
        for tab in parts(&app, card, Role::Tab) {
            let text = rect(&app, children_of(app.world(), tab)[0]);
            let tab = rect(&app, tab);
            assert!(
                tab.width() >= text.width() + 2.0 * pad - 1.0,
                "{width}x{height}: a tab {} px wide holds a label {} px wide and its padding",
                tab.width(),
                text.width()
            );
            assert!(
                tab.min.x >= strip.min.x - 1.0 && tab.max.x <= strip.max.x + 1.0,
                "{width}x{height}: the tab stays inside the strip"
            );
        }
    }
}

#[test]
fn zz_debug() {
    for (w, h) in [(1280, 720)] {
        let mut app = app_sized(w, h);
        let card = app.world_mut().spawn((panel(), children![tabs(["Graphics","Accessibility","Controls"],0,children![tab_page(children![label("A")]),tab_page(children![label("B")]),tab_page(children![label("C")])])])).id();
        app.world_mut().spawn(screen()).add_child(card);
        settle(&mut app);
        let strip = part(&app, card, Role::TabList);
        eprintln!("strip {:?} panel {:?}", rect(&app, strip), rect(&app, card));
        for t in parts(&app, card, Role::Tab) { let tx = children_of(app.world(), t)[0]; eprintln!("  tab {:?} text {:?}", rect(&app, t), rect(&app, tx)); }
        let ui = settings(&mut app_sized(w,h));
        let _ = ui;
    }
    let mut app = app_sized(1280,720);
    let ui = settings(&mut app);
    for &t in &ui.tabs { eprintln!("settings tab {:?}", rect(&app, t)); }
}
