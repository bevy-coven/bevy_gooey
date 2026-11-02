//! The status bar: left and right groups of items that hide lowest priority first when the window is
//! narrow, send `Activate`, and mark a warning or an error. Laid out for real in a headless window,
//! driven by real pointer input.

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<usize>` a game would see.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, usize)>);

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
    Changed<StatusKind>,
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

/// The items of one bar: left "main" and "2 problems"; right "UTF-8" (10), "Spaces: 4" (50) and "Ln 4, Col 12" (200).
struct Bar {
    column: Entity,
    bar: Entity,
    items: Vec<(&'static str, Entity)>,
}

fn status(app: &mut App, width: f32) -> Bar {
    let spawn = |app: &mut App, text: &'static str, priority: Option<u8>| {
        let mut item = app.world_mut().spawn(status_item(text));
        if let Some(priority) = priority {
            item.insert(StatusPriority(priority));
        }
        (text, item.id())
    };
    let main = spawn(app, "main", None);
    let problems = spawn(app, "2 problems", None);
    let utf = spawn(app, "UTF-8", Some(10));
    let spaces = spawn(app, "Spaces: 4", Some(50));
    let cursor = spawn(app, "Ln 4, Col 12", Some(200));
    let bar = app.world_mut().spawn(status_bar((), ())).id();
    // The groups are the bar's two children.
    let groups = children_of(app.world(), bar);
    app.world_mut()
        .entity_mut(groups[0])
        .add_children(&[main.1, problems.1]);
    app.world_mut()
        .entity_mut(groups[1])
        .add_children(&[utf.1, spaces.1, cursor.1]);
    let column = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(width),
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            ..default()
        })
        .add_child(bar)
        .id();
    app.world_mut().spawn(screen()).add_child(column);
    settle(app);
    Bar {
        column,
        bar,
        items: vec![main, problems, utf, spaces, cursor],
    }
}

fn resize(app: &mut App, bar: &Bar, width: f32) {
    app.world_mut()
        .entity_mut(bar.column)
        .get_mut::<Node>()
        .expect("node")
        .width = Val::Px(width);
    settle(app);
}

fn gone(app: &App, bar: &Bar) -> Vec<&'static str> {
    bar.items
        .iter()
        .filter(|(_, e)| app.world().get::<Overflowed>(*e).is_some())
        .map(|(name, _)| *name)
        .collect()
}

fn item(bar: &Bar, name: &str) -> Entity {
    bar.items
        .iter()
        .find(|(n, _)| *n == name)
        .expect("an item")
        .1
}

#[test]
fn at_800_px_the_lowest_priority_item_hides_first_and_a_click_on_one_that_is_left_activates_it() {
    let mut app = app();
    let bar = status(&mut app, 1200.0);
    assert!(gone(&app, &bar).is_empty(), "{:?}", gone(&app, &bar));
    // The bar's content is its width less its own side padding; the items stand a half gap apart.
    let skin = GooeySkin::default();
    let (gap, pad) = (skin.gap * 0.5, skin.pad());
    let widths: Vec<f32> = bar
        .items
        .iter()
        .map(|(_, e)| rect(&app, *e).width())
        .collect();
    let sum: f32 = widths.iter().sum();
    let all = sum + 4.0 * gap + pad;
    resize(&mut app, &bar, all + 1.0);
    assert!(
        gone(&app, &bar).is_empty(),
        "room for all: {:?}",
        gone(&app, &bar)
    );
    resize(&mut app, &bar, all - 1.0);
    assert_eq!(gone(&app, &bar), vec!["UTF-8"]);
    // Narrower: the next lowest of the right group, then the highest, only then the left's.
    resize(&mut app, &bar, all - widths[2] - gap - 1.0);
    assert_eq!(gone(&app, &bar), vec!["UTF-8", "Spaces: 4"]);
    resize(&mut app, &bar, 800.0);
    assert!(
        gone(&app, &bar)
            .iter()
            .all(|n| ["UTF-8", "Spaces: 4"].contains(n)),
        "800 px holds all but the lowest: {:?}",
        gone(&app, &bar)
    );
    let main = item(&bar, "main");
    click(&mut app, main);
    assert_eq!(app.world().resource::<Activations>().0, vec![main]);
}

#[test]
fn a_bar_hides_right_items_before_any_left_one_and_the_later_left_item_first() {
    let mut app = app();
    let bar = status(&mut app, 1200.0);
    resize(&mut app, &bar, 150.0);
    let hidden = gone(&app, &bar);
    assert!(
        hidden.contains(&"UTF-8")
            && hidden.contains(&"Spaces: 4")
            && hidden.contains(&"Ln 4, Col 12"),
        "{hidden:?}"
    );
    // The left group goes last, "2 problems" before "main".
    assert!(hidden.contains(&"2 problems"), "{hidden:?}");
    assert!(!hidden.contains(&"main"), "{hidden:?}");
}

#[test]
fn widening_the_window_shows_the_hidden_items_again() {
    let mut app = app();
    let bar = status(&mut app, 1200.0);
    resize(&mut app, &bar, 200.0);
    assert!(!gone(&app, &bar).is_empty());
    resize(&mut app, &bar, 1200.0);
    assert!(gone(&app, &bar).is_empty(), "{:?}", gone(&app, &bar));
}

#[test]
fn the_right_group_is_pushed_to_the_end_and_the_left_starts_at_the_start() {
    let mut app = app();
    let bar = status(&mut app, 1200.0);
    let bar_rect = rect(&app, bar.bar);
    let main = rect(&app, item(&bar, "main"));
    let cursor = rect(&app, item(&bar, "Ln 4, Col 12"));
    assert!(
        main.min.x - bar_rect.min.x < 30.0,
        "{main:?} in {bar_rect:?}"
    );
    assert!(
        bar_rect.max.x - cursor.max.x < 30.0,
        "{cursor:?} in {bar_rect:?}"
    );
}

#[test]
fn an_item_is_a_target_high_and_the_mark_shows_a_warning_and_an_error_and_not_plain_state() {
    let mut app = app();
    let bar = status(&mut app, 1200.0);
    let skin = GooeySkin::default();
    // A kind in the same tuple as the item is the item's own (the item requires one, it carries none).
    let stated = app
        .world_mut()
        .spawn((status_item("Failed"), StatusKind::Error))
        .id();
    assert_eq!(
        app.world().get::<StatusKind>(stated),
        Some(&StatusKind::Error)
    );
    let problems = item(&bar, "2 problems");
    assert!(rect(&app, problems).height() >= skin.target() - 1.0);
    let mark = |app: &App, item: Entity| {
        children_of(app.world(), item)
            .into_iter()
            .find(|&c| app.world().get::<Role>(c) == Some(&Role::StatusMark))
            .expect("a mark")
    };
    let display = |app: &App, mark: Entity| app.world().get::<Node>(mark).expect("node").display;
    let colour =
        |app: &App, mark: Entity| app.world().get::<BackgroundColor>(mark).expect("fill").0;
    assert_eq!(
        display(&app, mark(&app, problems)),
        Display::None,
        "info draws none"
    );
    app.world_mut()
        .entity_mut(problems)
        .insert(StatusKind::Warning);
    settle(&mut app);
    let m = mark(&app, problems);
    assert_ne!(display(&app, m), Display::None);
    assert_eq!(colour(&app, m), skin.accent);
    assert!(
        rect(&app, m).width() >= 2.0 && rect(&app, m).height() > 20.0,
        "{:?}",
        rect(&app, m)
    );
    app.world_mut()
        .entity_mut(problems)
        .insert(StatusKind::Error);
    settle(&mut app);
    assert_eq!(colour(&app, m), skin.danger);
    // The words stay the text colour in every kind.
    let words = children_of(app.world(), problems)
        .into_iter()
        .find(|&c| app.world().get::<Text>(c).is_some())
        .expect("words");
    assert_eq!(
        app.world().get::<TextColor>(words).expect("ink").0,
        skin.text
    );
    app.world_mut()
        .entity_mut(problems)
        .insert(StatusKind::Info);
    settle(&mut app);
    assert_eq!(display(&app, m), Display::None);
}

#[test]
fn a_status_progress_is_a_short_bar_inside_its_item_that_follows_its_value() {
    let mut app = app();
    let baking = app.world_mut().spawn(status_progress("Baking", 0.5)).id();
    let bar = app.world_mut().spawn(status_bar((), ())).id();
    let groups = children_of(app.world(), bar);
    app.world_mut().entity_mut(groups[0]).add_child(baking);
    app.world_mut().spawn(screen()).add_child(bar);
    settle(&mut app);
    let skin = GooeySkin::default();
    let track = descendants(app.world(), baking)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&Role::Meter))
        .expect("a track");
    let width = rect(&app, track).width();
    let text_px = skin.text_size * skin.text_scale;
    assert!(width >= text_px * 7.5 && width < text_px * 20.0, "{width}");
    let fill = children_of(app.world(), track)[0];
    let fraction = rect(&app, fill).width() / (width - 2.0 * 2.0);
    assert!((fraction - 0.5).abs() < 0.08, "{fraction}");
}

#[test]
fn thirty_idle_frames_write_nothing_whether_the_bar_fits_or_not() {
    let mut app = app();
    let bar = status(&mut app, 1200.0);
    resize(&mut app, &bar, 260.0);
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0);
}

#[test]
fn at_200_percent_text_the_items_that_do_not_fit_hide_and_the_rest_stay_inside_the_bar() {
    let mut app = app();
    app.insert_resource(GooeySkin {
        text_scale: 2.0,
        ..GooeySkin::default()
    });
    let mut bar = status(&mut app, 1280.0);
    let baking = app.world_mut().spawn(status_progress("Baking", 0.4)).id();
    let groups = children_of(app.world(), bar.bar);
    app.world_mut().entity_mut(groups[0]).add_child(baking);
    bar.items.insert(2, ("Baking", baking));
    settle(&mut app);
    let hidden = gone(&app, &bar);
    assert!(!hidden.is_empty(), "200% text needs more than 1280 px");
    let edge = rect(&app, bar.bar).max.x;
    for (name, entity) in &bar.items {
        if !hidden.contains(name) {
            assert!(
                rect(&app, *entity).max.x <= edge + 1.0,
                "{name} sticks out of the bar"
            );
        }
    }
}
