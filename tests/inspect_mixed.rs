#![cfg(feature = "reflect")]
//! A reflect inspector over a multi-selection (G-67): `InspectMixed` on the inspector's root names the
//! reflect paths whose values differ across the host's selection, and their rows read mixed until the
//! player edits them or the list stops naming them. Laid out for real in a headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `InspectorChange` a game would see.
#[derive(Resource, Default)]
struct Changes(Vec<String>);

/// How many times a `MixedValue` or `VecMixed` was written, frame after frame.
#[derive(Resource, Default)]
struct MixedWrites(u32);

type MixedWritten = Or<(Changed<MixedValue>, Changed<VecMixed>)>;

#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct Aim {
    yaw: f32,
    x: f32,
    position: Vec3,
    tint: Color,
    powered: bool,
}

fn aim() -> Aim {
    Aim {
        yaw: 12.5,
        x: 3.0,
        position: Vec3::new(1.0, 2.0, 3.0),
        tint: Color::srgb(0.2, 0.4, 0.6),
        powered: true,
    }
}

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
    .init_resource::<MixedWrites>()
    .add_observer(|change: On<InspectorChange>, mut seen: ResMut<Changes>| {
        seen.0.push(change.path.clone())
    })
    .add_systems(
        Update,
        |changed: Query<(), MixedWritten>, mut writes: ResMut<MixedWrites>| {
            writes.0 += changed.iter().count() as u32
        },
    )
    .register_type::<Aim>();
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

struct Ui {
    inspector: Entity,
    target: Entity,
}

/// An inspector of `Aim` that starts with `mixed` as its list (none when `None`).
fn ui(app: &mut App, mixed: Option<&[&str]>) -> Ui {
    let target = app.world_mut().spawn(aim()).id();
    let mut inspector = app
        .world_mut()
        .spawn(inspector(InspectTarget::component::<Aim>(target)));
    if let Some(list) = mixed {
        inspector.insert(InspectMixed(list.iter().map(|p| (*p).to_owned()).collect()));
    }
    let inspector = inspector.id();
    app.world_mut().spawn(screen()).add_child(inspector);
    settle(app);
    Ui { inspector, target }
}

fn set_list(app: &mut App, ui: &Ui, list: &[&str]) {
    app.world_mut()
        .entity_mut(ui.inspector)
        .insert(InspectMixed(list.iter().map(|p| (*p).to_owned()).collect()));
    settle(app);
}

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

fn texts(app: &App, root: Entity) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(text) = app.world().get::<Text>(entity) {
            found.push(text.0.clone());
        }
        if let Some(children) = app.world().get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    found
}

fn shows_the_hint(app: &App, row: Entity) -> bool {
    texts(app, row).iter().any(|t| t == MIXED_HINT)
}

fn mixed(app: &App, row: Entity) -> bool {
    app.world().get::<MixedValue>(row).is_some()
}

fn changed(app: &mut App) -> Vec<String> {
    std::mem::take(&mut app.world_mut().resource_mut::<Changes>().0)
}

#[test]
fn a_listed_number_reads_mixed_and_an_edit_sends_one_change_and_takes_it_off() {
    let mut app = app();
    let ui = ui(&mut app, Some(&["yaw"]));
    let (yaw, x) = (row(&app, &ui, "yaw"), row(&app, &ui, "x"));

    assert!(mixed(&app, yaw), "the listed row is mixed");
    assert!(
        shows_the_hint(&app, yaw),
        "and its field shows the hint: {:?}",
        texts(&app, yaw)
    );
    assert!(!mixed(&app, x), "an unlisted row is not");
    assert!(
        !shows_the_hint(&app, x),
        "and shows its number: {:?}",
        texts(&app, x)
    );
    assert_eq!(app.world().get::<NumberValue>(x), Some(&NumberValue(3.0)));

    // The player types 90 in the yaw field, as the field reports it.
    app.world_mut().trigger(ValueChange {
        source: yaw,
        value: 90.0_f32,
        is_final: true,
    });
    settle(&mut app);
    assert_eq!(
        changed(&mut app),
        vec!["yaw".to_string()],
        "one change, for yaw"
    );
    assert_eq!(app.world().get::<Aim>(ui.target).map(|a| a.yaw), Some(90.0));
    assert!(!mixed(&app, yaw), "the row no longer reads mixed");
    assert!(!shows_the_hint(&app, yaw), "{:?}", texts(&app, yaw));
    assert_eq!(
        app.world().get::<NumberValue>(yaw),
        Some(&NumberValue(90.0))
    );
    // The host's list still names it until the host applies the edit and takes it out: the row stays as
    // the player left it.
    settle(&mut app);
    assert!(!mixed(&app, yaw));
}

#[test]
fn taking_a_path_out_of_the_list_shows_the_components_own_value() {
    let mut app = app();
    let ui = ui(&mut app, Some(&["yaw", "x"]));
    let (yaw, x) = (row(&app, &ui, "yaw"), row(&app, &ui, "x"));
    assert!(mixed(&app, yaw) && mixed(&app, x));

    set_list(&mut app, &ui, &["x"]);
    assert!(!mixed(&app, yaw), "yaw is back to its value");
    assert!(!shows_the_hint(&app, yaw), "{:?}", texts(&app, yaw));
    assert_eq!(
        app.world().get::<NumberValue>(yaw),
        Some(&NumberValue(12.5))
    );
    assert!(mixed(&app, x), "x stays mixed");

    // A list that empties puts every row back, and so does taking the component off.
    set_list(&mut app, &ui, &["yaw", "x"]);
    assert!(mixed(&app, yaw) && mixed(&app, x));
    app.world_mut()
        .entity_mut(ui.inspector)
        .remove::<InspectMixed>();
    settle(&mut app);
    assert!(
        !mixed(&app, yaw) && !mixed(&app, x),
        "no list: nothing is mixed"
    );
    assert!(!shows_the_hint(&app, yaw) && !shows_the_hint(&app, x));
}

#[test]
fn a_vector_marks_its_listed_components_and_a_colour_reads_mixed() {
    let mut app = app();
    let ui = ui(&mut app, Some(&["position.y", "tint"]));
    let (position, tint) = (row(&app, &ui, "position"), row(&app, &ui, "tint"));
    assert_eq!(
        app.world().get::<VecMixed>(position),
        Some(&VecMixed(vec![false, true, false])),
        "only the y component of the vector"
    );
    assert!(mixed(&app, tint), "the colour row");

    // The vector's own path marks every component.
    set_list(&mut app, &ui, &["position"]);
    assert_eq!(
        app.world().get::<VecMixed>(position),
        Some(&VecMixed(vec![true, true, true]))
    );
    assert!(!mixed(&app, tint), "the colour is back to its value");

    set_list(&mut app, &ui, &[]);
    assert_eq!(
        app.world().get::<VecMixed>(position),
        None,
        "no component is mixed"
    );
}

#[test]
fn a_path_the_target_does_not_have_and_a_row_with_no_mixed_state_are_left_alone() {
    let mut app = app();
    let ui = ui(&mut app, Some(&["nothing.here", "powered"]));
    for path in ["yaw", "x", "tint"] {
        assert!(!mixed(&app, row(&app, &ui, path)), "{path}");
    }
    assert_eq!(
        app.world().get::<VecMixed>(row(&app, &ui, "position")),
        None
    );
    // A toggle has no mixed state in this crate: it keeps the component's value.
    assert!(
        app.world()
            .get::<bevy::ui::Checked>(row(&app, &ui, "powered"))
            .is_some()
    );
}

#[test]
fn a_list_that_has_not_changed_writes_nothing() {
    let mut app = app();
    let _ui = ui(&mut app, Some(&["yaw", "position.x", "tint"]));
    app.world_mut().resource_mut::<MixedWrites>().0 = 0;
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(app.world().resource::<MixedWrites>().0, 0);
}

#[test]
fn a_row_the_player_edited_stays_as_they_left_it_when_the_list_changes_for_another_path() {
    let mut app = app();
    let ui = ui(&mut app, Some(&["yaw", "x"]));
    let (yaw, x) = (row(&app, &ui, "yaw"), row(&app, &ui, "x"));
    app.world_mut().trigger(ValueChange {
        source: yaw,
        value: 90.0_f32,
        is_final: true,
    });
    settle(&mut app);
    assert!(!mixed(&app, yaw));

    // The host's list still names yaw and gains another path: yaw's membership did not change, so the
    // row is not marked again over the player's edit.
    set_list(&mut app, &ui, &["yaw", "x", "position.y"]);
    assert!(!mixed(&app, yaw), "the edited row is not re-marked");
    assert!(mixed(&app, x), "x was never edited and stays mixed");
    assert_eq!(
        app.world().get::<VecMixed>(row(&app, &ui, "position")),
        Some(&VecMixed(vec![false, true, false])),
        "the new path is marked"
    );

    // Once the host takes yaw out and puts it back (the next selection), it reads mixed again.
    set_list(&mut app, &ui, &["x", "position.y"]);
    set_list(&mut app, &ui, &["yaw", "x", "position.y"]);
    assert!(mixed(&app, yaw), "a path added to the list reads mixed");
}
