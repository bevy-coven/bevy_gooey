//! The command palette as a game sees it, laid out in a headless window and driven by real keyboard,
//! pointer and gamepad input: "opn prj" finds "Open Project" among 200 commands and Enter sends one
//! `RunCommand`; recents come first; a disabled command says why and runs nothing; Up and Down move
//! the lit row while focus stays in the field; a click runs a row; Escape closes with focus given
//! back; and the card stays inside the window at 200% text.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseButtonInput;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::text::EditableText;
use bevy::ui::{InteractionDisabled, Selected, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every id a `RunCommand` carried, in order.
#[derive(Resource, Default)]
struct Ran(Vec<String>);

#[derive(Resource, Default)]
struct Writes(u32);

/// What a frame at rest must not write.
type Written = Or<(
    Changed<Text>,
    Changed<TextColor>,
    Changed<BackgroundColor>,
    Changed<BorderColor>,
    Changed<Node>,
    Changed<CommandPalette>,
    Changed<TextInputValue>,
)>;

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
    .init_resource::<Ran>()
    .init_resource::<Writes>()
    .add_observer(|run: On<RunCommand>, mut ran: ResMut<Ran>| ran.0.push(run.id.clone()))
    .add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn app() -> App {
    app_sized(1280, 720)
}

fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

/// An editor's commands: 200 in all, a handful that a query could confuse and the rest generated.
fn two_hundred() -> Vec<PaletteCommand> {
    let mut list = vec![
        PaletteCommand::new("project.reopen", "Reopen Project").category("File"),
        PaletteCommand::new("project.settings", "Open Project Settings").category("File"),
        PaletteCommand::new("project.open", "Open Project")
            .category("File")
            .shortcut(Shortcut::primary(KeyCode::KeyO)),
        PaletteCommand::new("project.close", "Close Project").category("File"),
        PaletteCommand::new("prefs.open", "Open Preferences").category("Edit"),
    ];
    let verbs = [
        "Save",
        "Export",
        "Import",
        "Rename",
        "Delete",
        "Duplicate",
        "Select",
        "Toggle",
        "Rotate",
        "Hide",
    ];
    let nouns = [
        "Scene", "Layer", "Brush", "Camera", "Light", "Material", "Mesh", "Sound", "Script",
        "Prefab", "Texture", "Terrain", "Skybox", "Curve", "Bone", "Decal", "Volume", "Probe",
        "Marker", "Group",
    ];
    for verb in verbs {
        for noun in nouns {
            list.push(
                PaletteCommand::new(format!("{verb}.{noun}"), format!("{verb} {noun}"))
                    .category("Edit"),
            );
        }
    }
    list.truncate(200);
    assert_eq!(list.len(), 200);
    list
}

fn set_commands(app: &mut App, list: Vec<PaletteCommand>) {
    app.world_mut().insert_resource(CommandList(list));
}

fn open(app: &mut App) -> Entity {
    let palette = app.world_mut().spawn(command_palette()).id();
    settle(app);
    palette
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

fn down(app: &mut App) {
    tap_named(app, KeyCode::ArrowDown, Key::ArrowDown);
}

fn up(app: &mut App) {
    tap_named(app, KeyCode::ArrowUp, Key::ArrowUp);
}

fn enter(app: &mut App) {
    tap_named(app, KeyCode::Enter, Key::Enter);
}

fn escape(app: &mut App) {
    tap_named(app, KeyCode::Escape, Key::Escape);
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

/// The pointer moved onto `at`.
fn point_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ONE });
    settle(app);
}

fn click(app: &mut App, at: Vec2) {
    point_at(app, at);
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        });
        let action = match state {
            ButtonState::Pressed => PointerAction::Press(PointerButton::Primary),
            ButtonState::Released => PointerAction::Release(PointerButton::Primary),
        };
        pointer(app, at, action);
    }
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

fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
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

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn ran(app: &App) -> Vec<String> {
    app.world().resource::<Ran>().0.clone()
}

/// One row as the player reads it.
#[derive(Debug, PartialEq)]
struct Shown {
    label: String,
    hint: String,
    lit: bool,
    off: bool,
    entity: Entity,
}

/// The rows showing under `palette`, top first.
fn rows(app: &mut App, palette: Entity) -> Vec<Shown> {
    let world = app.world_mut();
    let mut query = world.query::<(
        Entity,
        &PaletteRow,
        &Node,
        &Children,
        Has<Selected>,
        Has<InteractionDisabled>,
    )>();
    let all: Vec<_> = query
        .iter(world)
        .filter(|(_, row, node, ..)| row.palette == palette && node.display != Display::None)
        .map(|(entity, row, _, children, lit, off)| {
            (
                entity,
                row.slot,
                children.iter().collect::<Vec<_>>(),
                lit,
                off,
            )
        })
        .collect();
    let mut found: Vec<(usize, Shown)> = Vec::new();
    for (entity, slot, children, lit, off) in all {
        let words = |child: Entity| {
            world
                .get::<Text>(child)
                .map(|t| t.0.clone())
                .unwrap_or_default()
        };
        found.push((
            slot,
            Shown {
                label: words(children[0]),
                hint: words(children[1]),
                lit,
                off,
                entity,
            },
        ));
    }
    found.sort_by_key(|(slot, _)| *slot);
    found.into_iter().map(|(_, row)| row).collect()
}

fn labels(app: &mut App, palette: Entity) -> Vec<String> {
    rows(app, palette).into_iter().map(|r| r.label).collect()
}

fn lit(app: &mut App, palette: Entity) -> Vec<String> {
    rows(app, palette)
        .into_iter()
        .filter(|r| r.lit)
        .map(|r| r.label)
        .collect()
}

fn gone(app: &App, entity: Entity) -> bool {
    app.world().get_entity(entity).is_err()
}

fn field_text(app: &mut App) -> String {
    let world = app.world_mut();
    world
        .query::<&EditableText>()
        .iter(world)
        .next()
        .expect("a field")
        .value()
        .to_string()
}

fn field_entity(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<EditableText>>()
        .iter(world)
        .next()
        .expect("a field")
}

fn card_entity(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<Node>>()
        .iter(world)
        .find(|&e| world.get::<Role>(e) == Some(&Role::Palette))
        .expect("the card")
}

fn count_text(app: &mut App) -> String {
    let world = app.world_mut();
    world
        .query::<(&PaletteCount, &Text)>()
        .iter(world)
        .next()
        .map(|(_, t)| t.0.clone())
        .unwrap_or_default()
}

fn note_shown(app: &mut App) -> bool {
    let world = app.world_mut();
    world
        .query::<(&PaletteNote, &Node)>()
        .iter(world)
        .next()
        .is_some_and(|(_, n)| n.display != Display::None)
}

fn selected(app: &App, palette: Entity) -> usize {
    app.world()
        .get::<CommandPalette>(palette)
        .expect("a palette")
        .selected()
}

fn result_count(app: &App, palette: Entity) -> usize {
    app.world()
        .get::<CommandPalette>(palette)
        .expect("a palette")
        .results()
        .len()
}

// --- the acceptance ----------------------------------------------------------------------------

#[test]
fn opn_prj_ranks_open_project_first_among_200_and_enter_runs_it_once() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let opener = app.world_mut().spawn(button("Menu")).id();
    settle(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(opener, FocusCause::Navigated);
    settle(&mut app);

    let palette = open(&mut app);
    // Focus went into the field, ready for the first letter.
    assert_eq!(focused(&app), Some(field_entity(&mut app)));
    type_str(&mut app, "opn prj");
    let shown = labels(&mut app, palette);
    assert_eq!(shown[0], "File: Open Project", "{shown:?}");
    assert_eq!(
        rows(&mut app, palette)[0].hint,
        Shortcut::primary(KeyCode::KeyO).label()
    );

    enter(&mut app);
    assert_eq!(ran(&app), ["project.open"]);
    assert!(gone(&app, palette), "running a command closes the palette");
    assert_eq!(focused(&app), Some(opener), "focus goes back where it was");
    // Enter with nothing open runs nothing more.
    enter(&mut app);
    assert_eq!(ran(&app).len(), 1);
    assert_eq!(
        app.world().resource::<PaletteHistory>().0,
        ["project.open"],
        "what ran is remembered"
    );
}

#[test]
fn an_empty_query_lists_recents_first_then_alphabetical() {
    let mut app = app();
    set_commands(
        &mut app,
        vec![
            PaletteCommand::new("c", "Charlie"),
            PaletteCommand::new("a", "alpha"),
            PaletteCommand::new("b", "Bravo"),
            PaletteCommand::new("d", "Delta"),
        ],
    );
    app.world_mut()
        .insert_resource(PaletteHistory(vec!["d".into(), "c".into()]));
    let palette = open(&mut app);
    assert_eq!(
        labels(&mut app, palette),
        ["Delta", "Charlie", "alpha", "Bravo"]
    );
    assert_eq!(lit(&mut app, palette), ["Delta"], "the first row is lit");
}

#[test]
fn a_recent_command_that_matches_comes_before_a_better_score() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    app.world_mut()
        .insert_resource(PaletteHistory(vec!["project.reopen".into()]));
    let palette = open(&mut app);
    type_str(&mut app, "opn prj");
    let shown = labels(&mut app, palette);
    assert_eq!(shown[0], "File: Reopen Project", "{shown:?}");
    assert_eq!(shown[1], "File: Open Project", "{shown:?}");
}

#[test]
fn nothing_matching_says_so_and_enter_runs_nothing() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let palette = open(&mut app);
    assert!(!note_shown(&mut app));
    type_str(&mut app, "zzzz");
    assert!(labels(&mut app, palette).is_empty());
    assert!(note_shown(&mut app));
    enter(&mut app);
    assert!(ran(&app).is_empty());
    assert!(!gone(&app, palette), "the palette stays open");
    // Changing the query brings the rows back and takes the line away.
    for _ in 0..4 {
        tap_named(&mut app, KeyCode::Backspace, Key::Backspace);
    }
    type_str(&mut app, "open pro");
    assert!(!note_shown(&mut app));
    assert!(!labels(&mut app, palette).is_empty());
}

// --- disabled commands -------------------------------------------------------------------------

#[test]
fn a_disabled_command_shows_its_reason_and_runs_nothing() {
    let mut app = app();
    set_commands(
        &mut app,
        vec![
            PaletteCommand::new("save", "Save")
                .category("File")
                .shortcut(Shortcut::primary(KeyCode::KeyS))
                .disabled("No project is open"),
            PaletteCommand::new("open", "Open").category("File"),
        ],
    );
    let palette = open(&mut app);
    let shown = rows(&mut app, palette);
    assert_eq!(shown[0].label, "File: Open");
    assert_eq!(shown[1].label, "File: Save");
    // The reason stands where the chord would, and the chord is not drawn.
    assert_eq!(shown[1].hint, "No project is open");
    assert!(shown[1].off && !shown[0].off);

    down(&mut app);
    assert_eq!(
        lit(&mut app, palette),
        ["File: Save"],
        "a disabled row can be lit"
    );
    enter(&mut app);
    assert!(
        ran(&app).is_empty(),
        "Enter on a disabled command runs nothing"
    );
    let row = rows(&mut app, palette)[1].entity;
    let at = rect(&app, row).center();
    click(&mut app, at);
    assert!(
        ran(&app).is_empty(),
        "a click on a disabled command runs nothing"
    );
    assert!(!gone(&app, palette));
    assert_eq!(
        focused(&app),
        Some(field_entity(&mut app)),
        "a click on it leaves focus in the field"
    );
    type_str(&mut app, "x");
    assert_eq!(
        field_text(&mut app),
        "x",
        "and the field still takes the keys"
    );
}

#[test]
fn switching_a_command_on_lets_the_open_palette_run_it() {
    let mut app = app();
    set_commands(
        &mut app,
        vec![PaletteCommand::new("save", "Save").disabled("No project is open")],
    );
    let palette = open(&mut app);
    enter(&mut app);
    assert!(ran(&app).is_empty());
    app.world_mut().resource_mut::<CommandList>().0[0].enabled = Ok(());
    settle(&mut app);
    assert!(!rows(&mut app, palette)[0].off);
    enter(&mut app);
    assert_eq!(ran(&app), ["save"]);
}

// --- the keys ----------------------------------------------------------------------------------

#[test]
fn up_and_down_move_the_lit_row_while_focus_stays_in_the_field() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let palette = open(&mut app);
    type_str(&mut app, "open");
    let field = field_entity(&mut app);
    let first = labels(&mut app, palette);
    assert!(first.len() >= 3, "{first:?}");

    down(&mut app);
    assert_eq!(lit(&mut app, palette), [first[1].clone()]);
    assert_eq!(focused(&app), Some(field), "focus never leaves the field");
    down(&mut app);
    assert_eq!(lit(&mut app, palette), [first[2].clone()]);
    up(&mut app);
    up(&mut app);
    assert_eq!(lit(&mut app, palette), [first[0].clone()]);
    // Up from the first wraps to the last match.
    up(&mut app);
    let count = result_count(&app, palette);
    assert_eq!(selected(&app, palette), count - 1);
    // And Down from the last wraps to the first.
    down(&mut app);
    assert_eq!(lit(&mut app, palette), [first[0].clone()]);
    // Typing after an arrow appends: the caret was not left at the start of the text.
    up(&mut app);
    down(&mut app);
    up(&mut app);
    type_str(&mut app, "x");
    assert_eq!(field_text(&mut app), "openx");
    assert_eq!(focused(&app), Some(field));
}

#[test]
fn the_lit_row_is_always_one_of_the_rows_shown_and_the_count_says_where() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let palette = open(&mut app);
    type_str(&mut app, "e");
    assert_eq!(lit(&mut app, palette).len(), 1);
    assert_eq!(rows(&mut app, palette).len(), PALETTE_ROWS);
    let total = result_count(&app, palette);
    assert_eq!(count_text(&mut app), format!("1 of {total}"));
    for step in 1..=12 {
        down(&mut app);
        assert_eq!(lit(&mut app, palette).len(), 1, "after {step} presses");
        assert_eq!(count_text(&mut app), format!("{} of {total}", step + 1));
    }
    // PageDown goes a page and stops at the end; PageUp comes back.
    tap_named(&mut app, KeyCode::PageDown, Key::PageDown);
    assert_eq!(selected(&app, palette), 12 + PALETTE_ROWS);
    assert_eq!(lit(&mut app, palette).len(), 1);
    for _ in 0..40 {
        tap_named(&mut app, KeyCode::PageDown, Key::PageDown);
    }
    assert_eq!(selected(&app, palette), total - 1);
    assert_eq!(lit(&mut app, palette).len(), 1);
    tap_named(&mut app, KeyCode::PageUp, Key::PageUp);
    assert_eq!(selected(&app, palette), total - 1 - PALETTE_ROWS);
}

#[test]
fn a_query_that_changes_lights_the_first_match_again() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let palette = open(&mut app);
    for _ in 0..5 {
        down(&mut app);
    }
    type_str(&mut app, "op");
    assert_eq!(selected(&app, palette), 0);
    let first = labels(&mut app, palette)[0].clone();
    assert_eq!(lit(&mut app, palette), [first]);
}

#[test]
fn enter_runs_the_lit_row_not_the_first() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let palette = open(&mut app);
    type_str(&mut app, "open");
    let second = labels(&mut app, palette)[1].clone();
    down(&mut app);
    enter(&mut app);
    let id = {
        let list = app.world().resource::<CommandList>();
        list.0
            .iter()
            .find(|c| c.label() == second)
            .expect("the command")
            .id
            .clone()
    };
    assert_eq!(ran(&app), [id]);
    assert!(gone(&app, palette));
}

#[test]
fn escape_closes_the_palette_and_gives_focus_back() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let opener = app.world_mut().spawn(button("Menu")).id();
    settle(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(opener, FocusCause::Navigated);
    settle(&mut app);
    let palette = open(&mut app);
    type_str(&mut app, "op");
    escape(&mut app);
    assert!(gone(&app, palette));
    assert!(ran(&app).is_empty());
    assert_eq!(focused(&app), Some(opener));
}

// --- the pointer -------------------------------------------------------------------------------

#[test]
fn the_pointer_lights_the_row_it_moves_over_and_a_click_runs_it_once() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let palette = open(&mut app);
    type_str(&mut app, "open");
    let shown = rows(&mut app, palette);
    let third = &shown[2];
    let at = rect(&app, third.entity).center();
    let label = third.label.clone();
    point_at(&mut app, at);
    assert_eq!(lit(&mut app, palette), [label.clone()]);
    click(&mut app, at);
    let id = {
        let list = app.world().resource::<CommandList>();
        list.0
            .iter()
            .find(|c| c.label() == label)
            .expect("the command")
            .id
            .clone()
    };
    assert_eq!(ran(&app), [id]);
    assert!(gone(&app, palette));
}

#[test]
fn a_click_on_the_cards_padding_keeps_focus_in_the_field_and_one_on_the_scrim_closes() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let palette = open(&mut app);
    type_str(&mut app, "ab");
    let field = field_entity(&mut app);
    let card = card_entity(&mut app);
    let card_rect = rect(&app, card);
    // Just inside the card's top-left corner: its padding.
    click(&mut app, card_rect.min + Vec2::splat(3.0));
    assert!(!gone(&app, palette));
    assert_eq!(focused(&app), Some(field));
    // The scrim, well outside the card.
    click(
        &mut app,
        Vec2::new(card_rect.min.x * 0.4, card_rect.max.y + 100.0),
    );
    assert!(gone(&app, palette));
    assert!(ran(&app).is_empty());
}

// --- the pad -----------------------------------------------------------------------------------

#[test]
fn the_d_pad_moves_the_lit_row_and_south_runs_it() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let palette = open(&mut app);
    type_str(&mut app, "open");
    let shown = labels(&mut app, palette);
    let pad = plug_in_pad(&mut app);
    press(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(lit(&mut app, palette), [shown[1].clone()]);
    press(&mut app, pad, GamepadButton::DPadDown);
    press(&mut app, pad, GamepadButton::DPadUp);
    assert_eq!(lit(&mut app, palette), [shown[1].clone()]);
    // Focus did not move off the field.
    assert_eq!(focused(&app), Some(field_entity(&mut app)));
    press(&mut app, pad, GamepadButton::South);
    assert_eq!(ran(&app).len(), 1);
    assert!(gone(&app, palette));
}

// --- the list changing -------------------------------------------------------------------------

#[test]
fn a_shorter_list_pulls_the_lit_row_back_in() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let palette = open(&mut app);
    type_str(&mut app, "e");
    for _ in 0..20 {
        down(&mut app);
    }
    app.world_mut().resource_mut::<CommandList>().0.truncate(5);
    settle(&mut app);
    let count = result_count(&app, palette);
    assert!(selected(&app, palette) < count.max(1));
    assert_eq!(rows(&mut app, palette).len(), count.min(PALETTE_ROWS));
    assert!(lit(&mut app, palette).len() <= 1);
    assert_eq!(lit(&mut app, palette).len(), usize::from(count > 0));
}

// --- looks -------------------------------------------------------------------------------------

#[test]
fn the_card_and_its_rows_stay_inside_the_window_at_every_text_size() {
    for scale in [1.0, 1.5, 2.0] {
        let mut app = app();
        app.world_mut().resource_mut::<GooeySkin>().text_scale = scale;
        let mut list = two_hundred();
        list[2] = PaletteCommand::new("project.open", "Open Project")
            .category("File")
            .shortcut(Shortcut::new(
                Modifiers::PRIMARY | Modifiers::SHIFT,
                KeyCode::KeyO,
            ));
        list[3] = PaletteCommand::new("project.close", "Close Project")
            .category("File")
            .disabled("Nothing is open");
        set_commands(&mut app, list);
        let palette = open(&mut app);
        type_str(&mut app, "proj");
        let window = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let card = card_entity(&mut app);
        let card_rect = rect(&app, card);
        let inside = |outer: Rect, inner: Rect| {
            inner.min.x >= outer.min.x - 1.0
                && inner.min.y >= outer.min.y - 1.0
                && inner.max.x <= outer.max.x + 1.0
                && inner.max.y <= outer.max.y + 1.0
        };
        assert!(inside(window, card_rect), "card {card_rect:?} at {scale}");
        // The card stands 5% of the width (the scrim's safe edge) and 5% of the height down, and
        // stays there while rows come and go.
        assert!((card_rect.min.y - (64.0 + 36.0)).abs() < 2.0, "{card_rect:?}");
        let shown = rows(&mut app, palette);
        assert!(shown.len() >= 2);
        for row in &shown {
            let row_rect = rect(&app, row.entity);
            assert!(
                inside(card_rect, row_rect),
                "{} at {scale}: {row_rect:?} in {card_rect:?}",
                row.label
            );
            // Its words: the label and the chord or reason, each inside the row, not on each other.
            let children: Vec<Entity> = app
                .world()
                .get::<Children>(row.entity)
                .expect("children")
                .iter()
                .collect();
            let title = rect(&app, children[0]);
            assert!(inside(row_rect, title), "{} at {scale}", row.label);
            if !row.hint.is_empty() {
                let hint = rect(&app, children[1]);
                assert!(inside(row_rect, hint), "{} hint at {scale}", row.label);
                assert!(title.max.x <= hint.min.x + 1.0, "{} at {scale}", row.label);
            }
            assert!(row_rect.height() >= 43.0, "a row is a target high");
        }
        let top_before = card_rect.min.y;
        type_str(&mut app, "ect");
        assert!((rect(&app, card).min.y - top_before).abs() < 1.0);
    }
}

#[test]
fn the_lit_row_and_its_words_keep_their_contrast_in_every_look() {
    for preset in SkinPreset::ALL {
        let mut app = app();
        *app.world_mut().resource_mut::<GooeySkin>() = preset.skin();
        let floor = if preset == SkinPreset::HighContrast {
            ENHANCED_CONTRAST
        } else {
            TEXT_CONTRAST
        };
        set_commands(
            &mut app,
            vec![
                PaletteCommand::new("open", "Open")
                    .category("File")
                    .shortcut(Shortcut::primary(KeyCode::KeyO)),
                PaletteCommand::new("save", "Save")
                    .category("File")
                    .shortcut(Shortcut::primary(KeyCode::KeyS)),
                PaletteCommand::new("quit", "Quit").disabled("Nothing to quit"),
            ],
        );
        let palette = open(&mut app);
        let skin = app.world().resource::<GooeySkin>().clone();
        let card = skin.solid_surface();
        let rgb = |c: Color| {
            let c = c.to_srgba();
            [c.red, c.green, c.blue]
        };
        // The first row is lit; the last, disabled, is lit after two presses.
        for presses in [0, 2] {
            for _ in 0..presses {
                down(&mut app);
            }
            for (index, row) in rows(&mut app, palette).iter().enumerate() {
                let children: Vec<Entity> = app
                    .world()
                    .get::<Children>(row.entity)
                    .expect("children")
                    .iter()
                    .collect();
                let fill = if row.lit && !row.off {
                    skin.hovered()
                } else {
                    card
                };
                for (part, child) in children.iter().enumerate() {
                    let ink = app.world().get::<TextColor>(*child).expect("ink").0;
                    let words = &app.world().get::<Text>(*child).expect("text").0;
                    if words.is_empty() {
                        continue;
                    }
                    let ratio = contrast_ratio(rgb(ink), rgb(fill));
                    assert!(
                        ratio >= floor,
                        "{preset:?} row {index} part {part} ({words}): {ratio:.2}:1"
                    );
                }
                if row.lit {
                    let ring = app
                        .world()
                        .get::<BorderColor>(row.entity)
                        .expect("a ring")
                        .top;
                    assert!(
                        contrast_ratio(rgb(ring), rgb(card)) >= NON_TEXT_CONTRAST,
                        "{preset:?}: the lit row's ring"
                    );
                }
            }
        }
    }
}

#[test]
fn an_idle_palette_writes_nothing() {
    let mut app = app();
    set_commands(&mut app, two_hundred());
    let palette = open(&mut app);
    type_str(&mut app, "open");
    down(&mut app);
    settle(&mut app);
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0);
    assert!(!gone(&app, palette));
}
