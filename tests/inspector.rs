#![cfg(feature = "reflect")]
//! The reflect inspector: the widgets a reflected component's fields become, edits written back to
//! it, and rows that follow it. Laid out for real in a headless window, driven by real pointer and
//! keyboard input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{Checked, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;
use std::ops::RangeInclusive;

/// Every `InspectorChange` a game would see.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, String)>);

/// How many frames a `Changed<Rig>` was seen in, as a game's own system would.
#[derive(Resource, Default)]
struct RigChanges(u32);

/// How many entities' fills, borders, nodes or visibility were written, frame after frame.
#[derive(Resource, Default)]
struct Writes(u32);

type Written = Or<(
    Changed<BackgroundColor>,
    Changed<BorderColor>,
    Changed<Node>,
    Changed<Visibility>,
    Changed<NumberValue>,
    Changed<ChoiceValue>,
    Changed<VecValue>,
    Changed<TextInputValue>,
    Changed<ColorValue>,
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
    .init_resource::<Changes>()
    .init_resource::<Writes>()
    .init_resource::<RigChanges>()
    .add_observer(|change: On<InspectorChange>, mut seen: ResMut<Changes>| {
        seen.0.push((change.entity, change.path.clone()))
    })
    .add_systems(
        Update,
        |rigs: Query<(), Changed<Rig>>, mut seen: ResMut<RigChanges>| {
            seen.0 += rigs.iter().count() as u32
        },
    )
    .register_type::<Rig>()
    .register_type::<Mode>()
    .register_type::<Nested>()
    .register_type::<Weather>()
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

#[derive(Reflect, Default, Clone, Copy, PartialEq, Debug)]
enum Mode {
    #[default]
    Idle,
    Run,
    Fly,
}

#[derive(Reflect, Default, Clone, PartialEq, Debug)]
struct Nested {
    gain: f32,
    label: String,
}

/// The acceptance's struct: a bool, a unit enum, an `f32` with a range, a `Vec3` and a nested
/// struct; and the other kinds an inspector draws.
#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct Rig {
    /// Is the rig powered.
    /// Off means nothing moves.
    powered: bool,
    mode: Mode,
    #[reflect(@RangeInclusive::<f32>::new(0.0, 10.0))]
    speed: f32,
    position: Vec3,
    nested: Nested,
}

#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct Extras {
    count: u32,
    #[reflect(@InspectStep(0.5))]
    trim: f64,
    tint: Color,
    flat: Vec2,
    name: String,
    tag: Option<u8>,
    list: Vec<u8>,
}

#[derive(Resource, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Resource, Default)]
struct Weather {
    rain: bool,
}

/// An inspector of `Rig` on a screen: the inspector, the entity it inspects.
struct Ui {
    inspector: Entity,
    target: Entity,
}

fn rig() -> Rig {
    Rig {
        powered: true,
        mode: Mode::Run,
        speed: 4.0,
        position: Vec3::new(1.0, 2.0, 3.0),
        nested: Nested {
            gain: 0.5,
            label: "alpha".into(),
        },
    }
}

fn ui(app: &mut App) -> Ui {
    let target = app.world_mut().spawn(rig()).id();
    let inspector = app
        .world_mut()
        .spawn(inspector(InspectTarget::component::<Rig>(target)))
        .id();
    app.world_mut().spawn(screen()).add_child(inspector);
    settle(app);
    Ui { inspector, target }
}

/// What a row is, by the state it keeps.
fn kind(app: &App, row: Entity) -> &'static str {
    let w = app.world();
    if w.get::<SectionOpen>(row).is_some() {
        "section"
    } else if w.get::<VecValue>(row).is_some() {
        "vec"
    } else if w.get::<NumberValue>(row).is_some() {
        "number"
    } else if w.get::<ChoiceValue>(row).is_some() {
        "choice"
    } else if w.get::<ColorValue>(row).is_some() {
        "colour"
    } else if w.get::<TextInputValue>(row).is_some() {
        "text"
    } else if w.get::<Role>(row) == Some(&Role::Toggle) {
        "toggle"
    } else if w.get::<Text>(row).is_some() {
        "line"
    } else {
        "?"
    }
}

fn kinds(app: &App, parent: Entity) -> Vec<&'static str> {
    children_of(app.world(), parent)
        .into_iter()
        .map(|c| kind(app, c))
        .collect()
}

/// The rows of the field called `path`.
fn row(app: &App, ui: &Ui, path: &str) -> Entity {
    app.world()
        .iter_entities()
        .find(|e| {
            e.get::<InspectRow>()
                .is_some_and(|r| r.inspector == ui.inspector && r.path == path)
        })
        .map(|e| e.id())
        .unwrap_or_else(|| panic!("no row for {path}"))
}

fn rig_of(app: &App, ui: &Ui) -> Rig {
    app.world().get::<Rig>(ui.target).expect("a rig").clone()
}

fn texts(app: &App, root: Entity) -> Vec<String> {
    descendants(app.world(), root)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect()
}

fn height(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<ComputedNode>(entity)
        .map_or(0.0, |n| n.size().y)
}

fn changed(app: &mut App) -> Vec<String> {
    let seen = std::mem::take(&mut app.world_mut().resource_mut::<Changes>().0);
    seen.into_iter().map(|(_, path)| path).collect()
}

#[test]
fn a_struct_yields_a_toggle_a_choice_a_number_a_vec_and_a_section_and_editing_the_number_changes_it()
 {
    let mut app = app();
    let ui = ui(&mut app);
    assert_eq!(
        kinds(&app, ui.inspector),
        vec!["toggle", "choice", "number", "vec", "section"]
    );
    let section = children_of(app.world(), ui.inspector)[4];
    let body = children_of(app.world(), section)[1];
    assert_eq!(kinds(&app, body), vec!["number", "text"]);
    // Shown as the component has them.
    let speed = row(&app, &ui, "speed");
    assert_eq!(
        app.world().get::<NumberValue>(speed),
        Some(&NumberValue(4.0))
    );
    assert!(
        app.world()
            .get::<Checked>(row(&app, &ui, "powered"))
            .is_some()
    );
    assert_eq!(
        app.world().get::<ChoiceValue>(row(&app, &ui, "mode")),
        Some(&ChoiceValue(1))
    );
    assert_eq!(
        app.world().get::<VecValue>(row(&app, &ui, "position")),
        Some(&VecValue(vec![1.0, 2.0, 3.0]))
    );
    // The player's edit of the number, as the field reports it.
    app.world_mut().trigger(ValueChange {
        source: speed,
        value: 7.5_f32,
        is_final: true,
    });
    settle(&mut app);
    assert_eq!(rig_of(&app, &ui).speed, 7.5);
    assert_eq!(changed(&mut app), vec!["speed".to_string()]);
}

#[test]
fn each_kind_of_edit_writes_its_field_and_says_so_once_with_the_fields_path() {
    let mut app = app();
    let ui = ui(&mut app);
    // A real click on the switch.
    let powered = row(&app, &ui, "powered");
    click(&mut app, powered);
    assert!(!rig_of(&app, &ui).powered);
    assert_eq!(changed(&mut app), vec!["powered".to_string()]);
    // A real Right on the focused choice: Run to Fly.
    let mode = row(&app, &ui, "mode");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(mode, FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    app.update();
    tap(&mut app, KeyCode::ArrowRight);
    settle(&mut app);
    assert_eq!(rig_of(&app, &ui).mode, Mode::Fly);
    assert_eq!(changed(&mut app), vec!["mode".to_string()]);
    // One component of the vector.
    let position = row(&app, &ui, "position");
    app.world_mut().trigger(VecChange {
        entity: position,
        index: 1,
        value: 9.0,
        values: vec![1.0, 9.0, 3.0],
        is_final: true,
    });
    settle(&mut app);
    assert_eq!(rig_of(&app, &ui).position, Vec3::new(1.0, 9.0, 3.0));
    assert_eq!(changed(&mut app), vec!["position".to_string()]);
    // The nested struct's own fields are reached by their path.
    let gain = row(&app, &ui, "nested.gain");
    app.world_mut().trigger(ValueChange {
        source: gain,
        value: 2.5_f32,
        is_final: true,
    });
    let label = row(&app, &ui, "nested.label");
    app.world_mut().trigger(ValueChange {
        source: label,
        value: "beta".to_string(),
        is_final: true,
    });
    settle(&mut app);
    assert_eq!(rig_of(&app, &ui).nested.gain, 2.5);
    assert_eq!(rig_of(&app, &ui).nested.label, "beta");
    assert_eq!(
        changed(&mut app),
        vec!["nested.gain".to_string(), "nested.label".to_string()]
    );
}

#[test]
fn a_value_the_field_already_has_is_not_written_and_says_nothing() {
    let mut app = app();
    let ui = ui(&mut app);
    app.update();
    let speed = row(&app, &ui, "speed");
    app.world_mut().trigger(ValueChange {
        source: speed,
        value: 4.0_f32,
        is_final: true,
    });
    settle(&mut app);
    assert!(changed(&mut app).is_empty());
    assert_eq!(
        app.world().resource::<RigChanges>().0,
        1,
        "only the spawn: the component was not written"
    );
}

#[test]
fn an_edit_is_a_change_the_games_changed_filter_sees_and_idle_frames_are_not() {
    let mut app = app();
    let ui = ui(&mut app);
    settle(&mut app);
    app.world_mut().resource_mut::<RigChanges>().0 = 0;
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(app.world().resource::<RigChanges>().0, 0, "idle");
    let speed = row(&app, &ui, "speed");
    app.world_mut().trigger(ValueChange {
        source: speed,
        value: 6.0_f32,
        is_final: true,
    });
    settle(&mut app);
    assert_eq!(
        app.world().resource::<RigChanges>().0,
        1,
        "once, for the edit"
    );
}

#[test]
fn a_number_has_its_attributes_range_and_the_types_step_and_a_doc_comment_is_a_tooltip() {
    let mut app = app();
    let ui = ui(&mut app);
    let speed = row(&app, &ui, "speed");
    assert_eq!(
        app.world().get::<NumberRange>(speed),
        Some(&NumberRange {
            min: 0.0,
            max: 10.0
        })
    );
    let gain = row(&app, &ui, "nested.gain");
    assert_eq!(
        app.world().get::<NumberRange>(gain).map(|r| r.min),
        Some(f32::NEG_INFINITY),
        "no attribute, no range"
    );
    assert_eq!(app.world().get::<NumberStep>(gain), Some(&NumberStep(0.1)));
    assert_eq!(
        app.world().get::<NumberStep>(speed),
        Some(&NumberStep(0.1)),
        "a hundredth of 0 to 10"
    );
    let powered = row(&app, &ui, "powered");
    let tip = app
        .world()
        .get::<Tooltip>(powered)
        .expect("the doc comment is the tooltip");
    assert_eq!(tip.text, "Is the rig powered. Off means nothing moves.");
    assert!(
        app.world().get::<Tooltip>(speed).is_none(),
        "no doc, no tooltip"
    );
}

#[test]
fn every_other_kind_is_the_widget_of_its_type_and_an_integer_stays_an_integer() {
    let mut app = app();
    let target = app
        .world_mut()
        .spawn(Extras {
            count: 3,
            trim: 1.5,
            tint: Color::srgb(1.0, 0.5, 0.0),
            flat: Vec2::new(4.0, 5.0),
            name: "crate".into(),
            tag: Some(7),
            list: vec![1, 2],
        })
        .id();
    app.register_type::<Extras>();
    let inspector = app
        .world_mut()
        .spawn(inspector(InspectTarget::component::<Extras>(target)))
        .id();
    app.world_mut().spawn(screen()).add_child(inspector);
    settle(&mut app);
    assert_eq!(
        kinds(&app, inspector),
        vec!["number", "number", "colour", "vec", "text", "line", "line"]
    );
    let ui = Ui { inspector, target };
    let count = row(&app, &ui, "count");
    assert_eq!(app.world().get::<NumberStep>(count), Some(&NumberStep(1.0)));
    assert_eq!(
        app.world().get::<NumberStep>(row(&app, &ui, "trim")),
        Some(&NumberStep(0.5)),
        "the attribute's step"
    );
    app.world_mut().trigger(ValueChange {
        source: count,
        value: 5.7_f32,
        is_final: true,
    });
    settle(&mut app);
    assert_eq!(app.world().get::<Extras>(target).expect("extras").count, 6);
    assert_eq!(
        app.world().get::<NumberValue>(count),
        Some(&NumberValue(6.0)),
        "the row shows what the field now holds"
    );
    // A colour, and a vector of two.
    let tint = row(&app, &ui, "tint");
    app.world_mut().trigger(ValueChange {
        source: tint,
        value: Srgba::new(0.0, 1.0, 0.0, 1.0),
        is_final: true,
    });
    let flat = row(&app, &ui, "flat");
    app.world_mut().trigger(VecChange {
        entity: flat,
        index: 0,
        value: 8.0,
        values: vec![8.0, 5.0],
        is_final: true,
    });
    settle(&mut app);
    let extras = app.world().get::<Extras>(target).expect("extras");
    assert_eq!(extras.tint, Color::Srgba(Srgba::new(0.0, 1.0, 0.0, 1.0)));
    assert_eq!(extras.flat, Vec2::new(8.0, 5.0));
    // The lines for what has no widget say what the value is and follow it.
    let words = texts(&app, inspector);
    assert!(
        words
            .iter()
            .any(|t| t.starts_with("Tag: ") && t.contains('7')),
        "{words:?}"
    );
    assert!(words.iter().any(|t| t.starts_with("List: ")), "{words:?}");
    app.world_mut()
        .get_mut::<Extras>(target)
        .expect("extras")
        .tag = None;
    settle(&mut app);
    let words = texts(&app, inspector);
    assert!(
        words
            .iter()
            .any(|t| t.starts_with("Tag: ") && t.contains("None")),
        "{words:?}"
    );
}

#[test]
fn a_value_the_game_writes_moves_every_row_with_no_event() {
    let mut app = app();
    let ui = ui(&mut app);
    {
        let mut rig = app.world_mut().get_mut::<Rig>(ui.target).expect("a rig");
        rig.powered = false;
        rig.mode = Mode::Idle;
        rig.speed = 9.0;
        rig.position = Vec3::new(-1.0, 0.0, 1.0);
        rig.nested.gain = 3.0;
        rig.nested.label = "omega".into();
    }
    settle(&mut app);
    assert!(
        app.world()
            .get::<Checked>(row(&app, &ui, "powered"))
            .is_none()
    );
    assert_eq!(
        app.world().get::<ChoiceValue>(row(&app, &ui, "mode")),
        Some(&ChoiceValue(0))
    );
    assert_eq!(
        app.world().get::<NumberValue>(row(&app, &ui, "speed")),
        Some(&NumberValue(9.0))
    );
    assert_eq!(
        app.world().get::<VecValue>(row(&app, &ui, "position")),
        Some(&VecValue(vec![-1.0, 0.0, 1.0]))
    );
    assert_eq!(
        app.world()
            .get::<NumberValue>(row(&app, &ui, "nested.gain")),
        Some(&NumberValue(3.0))
    );
    assert_eq!(
        app.world()
            .get::<TextInputValue>(row(&app, &ui, "nested.label")),
        Some(&TextInputValue("omega".into()))
    );
    assert!(
        changed(&mut app).is_empty(),
        "the game's own writes say nothing"
    );
}

#[test]
fn a_resource_is_inspected_like_a_component() {
    let mut app = app();
    app.insert_resource(Weather { rain: true });
    let inspector = app
        .world_mut()
        .spawn(inspector(InspectTarget::resource::<Weather>()))
        .id();
    app.world_mut().spawn(screen()).add_child(inspector);
    settle(&mut app);
    assert_eq!(kinds(&app, inspector), vec!["toggle"]);
    let ui = Ui {
        inspector,
        target: Entity::PLACEHOLDER,
    };
    let rain = row(&app, &ui, "rain");
    click(&mut app, rain);
    assert!(!app.world().resource::<Weather>().rain);
    assert_eq!(changed(&mut app), vec!["rain".to_string()]);
    app.world_mut().resource_mut::<Weather>().rain = true;
    settle(&mut app);
    assert!(app.world().get::<Checked>(rain).is_some());
}

#[test]
fn a_type_that_cannot_be_reflected_as_a_component_says_so_instead_of_a_panel() {
    // Reflected, but with no `#[reflect(Component)]`: the registry cannot reach it on an entity.
    #[derive(Component, Reflect, Default)]
    struct Unreachable {
        x: f32,
    }
    let mut app = app();
    app.register_type::<Unreachable>();
    let target = app.world_mut().spawn(Unreachable { x: 1.0 }).id();
    let inspector = app
        .world_mut()
        .spawn(inspector(InspectTarget::component::<Unreachable>(target)))
        .id();
    app.world_mut().spawn(screen()).add_child(inspector);
    settle(&mut app);
    assert_eq!(kinds(&app, inspector), vec!["line"]);
    assert!(
        texts(&app, inspector)[0].starts_with("Nothing to inspect"),
        "{:?}",
        texts(&app, inspector)
    );
}

#[test]
fn the_rows_stand_a_gap_apart_a_target_high_and_inside_the_screen() {
    let mut app = app();
    let ui = ui(&mut app);
    let skin = GooeySkin::default();
    let rows = children_of(app.world(), ui.inspector);
    for pair in rows.windows(2) {
        let (a, b) = (rect(&app, pair[0]), rect(&app, pair[1]));
        assert!(
            (b.min.y - a.max.y - skin.gap).abs() < 1.5,
            "rows are a gap apart: {} px",
            b.min.y - a.max.y
        );
    }
    for &r in &rows {
        assert!(height(&app, r) >= skin.target() - 1.0 || kind(&app, r) == "section");
    }
    let inspector = rect(&app, ui.inspector);
    assert!(inspector.min.x >= 0.0 && inspector.max.x <= 1280.0 && inspector.max.y <= 720.0);
}

#[test]
fn thirty_idle_frames_write_nothing_and_leave_the_component_alone() {
    let mut app = app();
    let _ = ui(&mut app);
    app.update();
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0);
    let rig = app
        .world_mut()
        .query::<Ref<Rig>>()
        .single(app.world())
        .expect("one rig");
    assert!(!rig.is_changed());
}

#[test]
fn a_text_field_being_typed_in_is_not_overwritten_by_the_old_value() {
    let mut app = app();
    let ui = ui(&mut app);
    let label = row(&app, &ui, "nested.label");
    app.world_mut().entity_mut(label).insert(TextEditing);
    app.world_mut()
        .get_mut::<Rig>(ui.target)
        .expect("rig")
        .nested
        .label = "elsewhere".into();
    settle(&mut app);
    assert_eq!(
        app.world().get::<TextInputValue>(label),
        Some(&TextInputValue("alpha".into())),
        "left alone while its field has focus"
    );
    app.world_mut().entity_mut(label).remove::<TextEditing>();
    settle(&mut app);
    assert_eq!(
        app.world().get::<TextInputValue>(label),
        Some(&TextInputValue("elsewhere".into()))
    );
}
