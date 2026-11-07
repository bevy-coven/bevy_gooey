//! Collapsible sections: a header that folds its body away from layout and from focus, a state that
//! survives a respawn by id, "Empty" for a section with nothing in it. Laid out for real in a
//! headless window, driven by real pointer and keyboard input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{InteractionDisabled, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<bool>` a game would see.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, bool)>);

/// Every `Activate` a game would see.
#[derive(Resource, Default)]
struct Activations(Vec<Entity>);

/// How many entities' fills, borders, nodes or visibility were written, frame after frame.
#[derive(Resource, Default)]
struct Writes(u32);

type Written = Or<(
    Changed<BackgroundColor>,
    Changed<BorderColor>,
    Changed<Node>,
    Changed<Visibility>,
    Changed<SectionOpen>,
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
    // The instant path: this file asserts layout a frame or two after a change, on the real-time
    // clock (assumed owner exception, bevy_gooey#76).
    .insert_resource(MotionSettings::off())
    .init_resource::<Changes>()
    .init_resource::<Activations>()
    .init_resource::<Writes>()
    .add_observer(|change: On<ValueChange<bool>>, mut seen: ResMut<Changes>| {
        seen.0.push((change.source, change.value))
    })
    .add_observer(|activate: On<Activate>, mut seen: ResMut<Activations>| {
        seen.0.push(activate.entity)
    })
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
    for _ in 0..8 {
        app.update();
    }
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

/// A press of Tab, which Bevy's navigation reads by its logical key.
fn tab(app: &mut App) {
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Tab,
            logical_key: Key::Tab,
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
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

fn click(app: &mut App, entity: Entity) {
    let at = rect(app, entity).center();
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
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

/// The widgets of a panel of two sections and a button either side.
struct Ui {
    panel: Entity,
    before: Entity,
    /// The first section (id "a", two widgets), its header, body and the widgets in it.
    a: Entity,
    a_header: Entity,
    a_body: Entity,
    a_items: Vec<Entity>,
    /// The second section (id "b"), with nothing in it.
    b: Entity,
    b_header: Entity,
    b_body: Entity,
    b_note: Entity,
    after: Entity,
}

fn ui_with(app: &mut App, a_open: bool, b_open: bool) -> Ui {
    let panel = app
        .world_mut()
        .spawn((
            panel(),
            children![
                button("Before"),
                (
                    section(
                        "Physics",
                        a_open,
                        children![slider("Gravity", 9.8, 0.0..=20.0), toggle("Wind", false)],
                    ),
                    SectionId::new("a"),
                ),
                (section("Debug", b_open, ()), SectionId::new("b")),
                button("After"),
            ],
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(panel);
    settle(app);
    let kids = children_of(app.world(), panel);
    let [before, a, b, after] = kids[..] else {
        panic!("four children, got {}", kids.len());
    };
    let [a_header, a_body, _] = children_of(app.world(), a)[..] else {
        panic!("a section is a header, a body and a note");
    };
    let [b_header, b_body, b_note] = children_of(app.world(), b)[..] else {
        panic!("a section is a header, a body and a note");
    };
    let a_items = children_of(app.world(), a_body);
    Ui {
        panel,
        before,
        a,
        a_header,
        a_body,
        a_items,
        b,
        b_header,
        b_body,
        b_note,
        after,
    }
}

fn ui(app: &mut App) -> Ui {
    ui_with(app, true, true)
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    app.update();
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn height(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<ComputedNode>(entity)
        .map_or(0.0, |n| n.size().y)
}

fn is_open(app: &App, section: Entity) -> bool {
    app.world().get::<SectionOpen>(section).expect("state").0
}

fn texts(app: &App, root: Entity) -> Vec<(Entity, String)> {
    descendants(app.world(), root)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| (e, t.0.clone())))
        .collect()
}

#[test]
fn toggling_hides_the_children_from_layout_and_focus_and_the_state_survives_a_respawn() {
    let mut app = app();
    let ui = ui(&mut app);
    assert!(is_open(&app, ui.a));
    assert!(
        height(&app, ui.a_body) > 40.0,
        "open: the widgets have room"
    );
    for &item in &ui.a_items {
        assert!(height(&app, item) > 10.0);
    }

    click(&mut app, ui.a_header);
    assert!(!is_open(&app, ui.a));
    assert_eq!(height(&app, ui.a_body), 0.0, "closed: no layout");
    for &item in &ui.a_items {
        assert_eq!(height(&app, item), 0.0, "closed: a child is not laid out");
    }
    // Focus never rests in it: Tab round the screen twice and the arrows down it.
    let hidden = descendants(app.world(), ui.a_body);
    focus(&mut app, ui.before);
    let mut visited = Vec::new();
    for step in 0..8 {
        tab(&mut app);
        assert!(
            !focused(&app).is_some_and(|f| hidden.contains(&f)),
            "Tab {step} stopped in a closed section"
        );
        visited.extend(focused(&app));
    }
    for stop in [ui.a_header, ui.b_header, ui.after, ui.before] {
        assert!(visited.contains(&stop), "Tab walks the screen: {visited:?}");
    }
    focus(&mut app, ui.before);
    for step in 0..4 {
        tap(&mut app, KeyCode::ArrowDown);
        settle(&mut app);
        assert!(
            !focused(&app).is_some_and(|f| hidden.contains(&f)),
            "Down {step} stopped in a closed section"
        );
    }

    // The same id after a despawn and a respawn: closed, though spawned open.
    app.world_mut().entity_mut(ui.panel).despawn();
    settle(&mut app);
    let again = ui_with(&mut app, true, true);
    assert!(!is_open(&app, again.a), "the state came back");
    assert_eq!(height(&app, again.a_body), 0.0);
    assert!(is_open(&app, again.b), "another id is not touched");
    click(&mut app, again.a_header);
    assert!(is_open(&app, again.a));
    assert!(height(&app, again.a_body) > 40.0);
}

#[test]
fn enter_space_and_a_click_toggle_once_each_with_one_change_and_a_game_write_sends_none() {
    let mut app = app();
    let ui = ui(&mut app);
    focus(&mut app, ui.a_header);
    tap(&mut app, KeyCode::Enter);
    settle(&mut app);
    assert!(!is_open(&app, ui.a));
    assert_eq!(app.world().resource::<Changes>().0, vec![(ui.a, false)]);
    tap(&mut app, KeyCode::Space);
    settle(&mut app);
    assert!(is_open(&app, ui.a));
    click(&mut app, ui.a_header);
    assert!(!is_open(&app, ui.a));
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(ui.a, false), (ui.a, true), (ui.a, false)]
    );

    app.world_mut().resource_mut::<Changes>().0.clear();
    app.world_mut().entity_mut(ui.a).insert(SectionOpen(true));
    settle(&mut app);
    assert!(height(&app, ui.a_body) > 40.0, "the game's write opens it");
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn a_disabled_header_does_not_toggle() {
    let mut app = app();
    let ui = ui(&mut app);
    app.world_mut()
        .entity_mut(ui.a_header)
        .insert(InteractionDisabled);
    settle(&mut app);
    click(&mut app, ui.a_header);
    assert!(is_open(&app, ui.a));
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn focus_inside_a_body_that_closes_goes_to_its_header() {
    let mut app = app();
    let ui = ui(&mut app);
    focus(&mut app, ui.a_items[1]);
    assert_eq!(focused(&app), Some(ui.a_items[1]));
    app.world_mut().entity_mut(ui.a).insert(SectionOpen(false));
    settle(&mut app);
    assert_eq!(focused(&app), Some(ui.a_header));
}

#[test]
fn a_section_with_nothing_in_it_says_empty_until_something_is() {
    let mut app = app();
    let ui = ui(&mut app);
    let words = texts(&app, ui.b);
    assert!(words.iter().any(|(_, t)| t == "Empty"), "{words:?}");
    assert!(height(&app, ui.b_note) > 10.0, "shown");
    assert_eq!(height(&app, ui.b_body), 0.0, "nothing to lay out");
    // A section with widgets in it says nothing of the kind.
    let note_of_a = children_of(app.world(), ui.a)[2];
    assert_eq!(height(&app, note_of_a), 0.0);
    // Closed, it hides the note too.
    click(&mut app, ui.b_header);
    assert!(!is_open(&app, ui.b));
    assert_eq!(height(&app, ui.b_note), 0.0);
    click(&mut app, ui.b_header);
    assert!(height(&app, ui.b_note) > 10.0);
    // A child added later takes it away.
    let child = app.world_mut().spawn(toggle("Late", false)).id();
    app.world_mut().entity_mut(ui.b_body).add_child(child);
    settle(&mut app);
    assert_eq!(height(&app, ui.b_note), 0.0);
    assert!(height(&app, child) > 10.0);
}

#[test]
fn the_header_is_a_target_high_with_its_words_and_chevron_readable_in_every_look() {
    for skin in [
        GooeySkin::dark(),
        GooeySkin::parchment(),
        GooeySkin::high_contrast(),
        GooeySkin::tool(),
    ] {
        let mut app = app();
        app.insert_resource(skin.clone());
        let ui = ui(&mut app);
        let header = rect(&app, ui.a_header);
        assert!(
            header.height() >= skin.target() - 1.0,
            "{} px high",
            header.height()
        );
        assert!(
            (header.width() - rect(&app, ui.panel).width()).abs() < skin.pad() * 2.0 + 8.0,
            "the header spans its panel"
        );
        let fill = Srgba::from(
            app.world()
                .get::<BackgroundColor>(ui.a_header)
                .expect("fill")
                .0,
        );
        let title = texts(&app, ui.a_header)
            .into_iter()
            .find(|(_, t)| t == "Physics")
            .expect("the title")
            .0;
        let ink = Srgba::from(app.world().get::<TextColor>(title).expect("ink").0);
        let ratio = contrast_ratio(
            [ink.red, ink.green, ink.blue],
            [fill.red, fill.green, fill.blue],
        );
        assert!(ratio >= 4.5, "title {ratio}");
        // Open: the chevron that points down is laid out and the one that points right is not.
        let chevrons: Vec<(Role, f32)> = descendants(app.world(), ui.a_header)
            .into_iter()
            .filter_map(|e| {
                let role = *app.world().get::<Role>(e)?;
                matches!(role, Role::TreeClosed | Role::TreeOpen).then(|| (role, height(&app, e)))
            })
            .collect();
        assert_eq!(chevrons.len(), 2);
        for (role, h) in chevrons {
            assert_eq!(h > 0.0, role == Role::TreeOpen, "{role:?} {h}");
        }
    }
}

#[test]
fn a_nested_section_is_the_same_widget_and_hides_with_its_parent() {
    let mut app = app();
    let inner = app
        .world_mut()
        .spawn(section("Inner", true, children![toggle("Deep", false)]))
        .id();
    let outer = app.world_mut().spawn(section("Outer", true, ())).id();
    // The outer's body is where the inner goes.
    let body = children_of(app.world(), outer)[1];
    app.world_mut().entity_mut(body).add_child(inner);
    app.world_mut().spawn(screen()).add_child(outer);
    settle(&mut app);
    let inner_header = children_of(app.world(), inner)[0];
    let inner_body = children_of(app.world(), inner)[1];
    let deep = children_of(app.world(), inner_body)[0];
    assert!(height(&app, deep) > 10.0);
    click(&mut app, inner_header);
    assert!(!is_open(&app, inner));
    assert!(is_open(&app, outer), "the outer is its own");
    assert_eq!(height(&app, deep), 0.0);
    click(&mut app, inner_header);
    app.world_mut().entity_mut(outer).insert(SectionOpen(false));
    settle(&mut app);
    assert_eq!(
        height(&app, inner_header),
        0.0,
        "an outer closed takes the inner away"
    );
    assert!(is_open(&app, inner), "and leaves its state");
}

#[test]
fn a_state_the_game_stored_is_the_start_and_the_player_writes_it_back() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<SectionStates>()
        .set("a", false);
    let ui = ui(&mut app);
    assert!(!is_open(&app, ui.a), "the stored state wins over `open`");
    assert!(app.world().resource::<Changes>().0.is_empty(), "no event");
    click(&mut app, ui.a_header);
    assert_eq!(
        app.world().resource::<SectionStates>().open("a"),
        Some(true)
    );
    app.world_mut().entity_mut(ui.a).insert(SectionOpen(false));
    settle(&mut app);
    assert_eq!(
        app.world().resource::<SectionStates>().open("a"),
        Some(false),
        "a game's write is kept too"
    );
    assert_eq!(app.world().resource::<SectionStates>().open("nobody"), None);
}

#[test]
fn a_section_with_no_id_remembers_nothing() {
    let mut app = app();
    let section = app
        .world_mut()
        .spawn(section("Loose", false, children![toggle("x", false)]))
        .id();
    app.world_mut().spawn(screen()).add_child(section);
    settle(&mut app);
    let header = children_of(app.world(), section)[0];
    click(&mut app, header);
    assert!(is_open(&app, section));
    assert_eq!(
        *app.world().resource::<SectionStates>(),
        SectionStates::default()
    );
}

#[test]
fn thirty_idle_frames_write_nothing_open_or_closed() {
    let mut app = app();
    let ui = ui(&mut app);
    click(&mut app, ui.b_header);
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0);
}

#[test]
fn at_200_percent_text_nothing_leaves_its_section_and_the_title_stays_on_one_line() {
    let mut app = app();
    app.insert_resource(GooeySkin {
        text_scale: 2.0,
        ..GooeySkin::default()
    });
    let ui = ui(&mut app);
    let panel = rect(&app, ui.panel);
    for entity in descendants(app.world(), ui.panel) {
        if height(&app, entity) == 0.0 {
            continue;
        }
        let r = rect(&app, entity);
        assert!(
            r.min.x >= panel.min.x - 1.5 && r.max.x <= panel.max.x + 1.5,
            "{entity} at {r:?} leaves the panel {panel:?}"
        );
    }
    let title = texts(&app, ui.a_header)
        .into_iter()
        .find(|(_, t)| t == "Physics")
        .expect("title")
        .0;
    assert!(
        height(&app, title) < GooeySkin::default().text_size * 2.0 * 1.6,
        "one line, {} px",
        height(&app, title)
    );
    let _ = (ui.before, ui.after, ui.b_header);
}
