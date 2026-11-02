//! The toolbar: icon buttons, a toggle group with radio behaviour, separators, and a bar that holds
//! only what its width has room for and lists the rest in a "More" menu. Laid out for real in a
//! headless window, driven by real pointer and keyboard input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{Checked, InteractionDisabled, UiGlobalTransform};
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
    Changed<ToggleGroupValue>,
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

fn picture(app: &mut App) -> IconSource {
    IconSource::Image(
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default()),
    )
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

/// A group of three icon options in a toolbar on a screen: the toolbar, the group, its options.
struct Tools {
    group: Entity,
    options: Vec<Entity>,
}

fn tools(app: &mut App, selected: usize) -> Tools {
    let options = ["Move", "Paint", "Erase"].map(|name| {
        let source = picture(app);
        GroupOption::icon(source, Tooltip::new(name))
    });
    let group = app.world_mut().spawn(toggle_group(options, selected)).id();
    let bar = app.world_mut().spawn(toolbar(())).add_child(group).id();
    app.world_mut().spawn(screen()).add_child(bar);
    settle(app);
    let options = children_of(app.world(), group);
    Tools { group, options }
}

fn lit(app: &App, option: Entity) -> bool {
    app.world().get::<Checked>(option).is_some()
}

fn fill(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

#[test]
fn activating_the_second_option_of_a_toggle_group_changes_to_one_and_unchecks_the_first() {
    let mut app = app();
    let tools = tools(&mut app, 0);
    assert!(lit(&app, tools.options[0]));
    assert!(!lit(&app, tools.options[1]));
    click(&mut app, tools.options[1]);
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(tools.group, 1)],
        "one change, from the group"
    );
    assert_eq!(
        app.world().get::<ToggleGroupValue>(tools.group),
        Some(&ToggleGroupValue(1))
    );
    assert!(!lit(&app, tools.options[0]));
    assert!(lit(&app, tools.options[1]));
    assert!(!lit(&app, tools.options[2]));
}

#[test]
fn the_option_that_is_on_is_the_accent_with_its_picture_inverted_and_the_others_a_buttons_fill() {
    let mut app = app();
    let tools = tools(&mut app, 1);
    let skin = GooeySkin::default();
    assert_eq!(fill(&app, tools.options[1]), skin.accent);
    assert_eq!(fill(&app, tools.options[0]), skin.surface_raised);
    let tint = |app: &App, option: Entity| {
        let picture = children_of(app.world(), option)
            .into_iter()
            .find(|&c| app.world().get::<Icon>(c).is_some())
            .expect("an icon");
        app.world().get::<ImageNode>(picture).expect("image").color
    };
    assert_eq!(tint(&app, tools.options[1]), skin.accent_text);
    assert_eq!(tint(&app, tools.options[0]), skin.text);
    // Moving the light moves both.
    click(&mut app, tools.options[2]);
    assert_eq!(fill(&app, tools.options[2]), skin.accent);
    assert_eq!(fill(&app, tools.options[1]), skin.surface_raised);
    assert_eq!(tint(&app, tools.options[1]), skin.text);
}

#[test]
fn choosing_the_option_that_is_already_on_says_nothing() {
    let mut app = app();
    let tools = tools(&mut app, 0);
    click(&mut app, tools.options[0]);
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert!(lit(&app, tools.options[0]));
}

#[test]
fn enter_chooses_the_focused_option_and_the_pad_is_the_same_activation() {
    let mut app = app();
    let tools = tools(&mut app, 0);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(tools.options[2], FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    app.update();
    tap(&mut app, KeyCode::Enter);
    settle(&mut app);
    assert_eq!(app.world().resource::<Changes>().0, vec![(tools.group, 2)]);
    assert!(lit(&app, tools.options[2]));
}

#[test]
fn a_value_the_game_sets_moves_the_light_with_no_event_and_one_past_the_end_is_pulled_back() {
    let mut app = app();
    let tools = tools(&mut app, 0);
    app.world_mut()
        .entity_mut(tools.group)
        .insert(ToggleGroupValue(2));
    settle(&mut app);
    assert!(lit(&app, tools.options[2]));
    assert!(!lit(&app, tools.options[0]));
    app.world_mut()
        .entity_mut(tools.group)
        .insert(ToggleGroupValue(9));
    settle(&mut app);
    assert_eq!(
        app.world().get::<ToggleGroupValue>(tools.group),
        Some(&ToggleGroupValue(2))
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn a_disabled_option_is_not_chosen() {
    let mut app = app();
    let tools = tools(&mut app, 0);
    app.world_mut()
        .entity_mut(tools.options[1])
        .insert(InteractionDisabled);
    settle(&mut app);
    click(&mut app, tools.options[1]);
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert!(lit(&app, tools.options[0]));
}

#[test]
fn an_icon_button_is_a_target_square_that_carries_its_tooltip_and_activates_once() {
    let mut app = app();
    let source = picture(&mut app);
    let save = app
        .world_mut()
        .spawn(icon_button(
            source,
            Tooltip::new("Save").with_shortcut(Shortcut::primary(KeyCode::KeyS)),
        ))
        .id();
    let bar = app.world_mut().spawn(toolbar(())).add_child(save).id();
    app.world_mut().spawn(screen()).add_child(bar);
    settle(&mut app);
    let skin = GooeySkin::default();
    let size = rect(&app, save).size();
    assert!(size.x >= skin.target() - 1.0, "{size}");
    assert!(size.y >= skin.target() - 1.0, "{size}");
    let boxes = children_of(app.world(), save)
        .into_iter()
        .filter(|&c| app.world().get::<TooltipBox>(c).is_some())
        .count();
    assert_eq!(boxes, 1, "the tooltip is its name");
    click(&mut app, save);
    assert_eq!(app.world().resource::<Activations>().0, vec![save]);
}

#[test]
fn a_separator_is_a_vertical_line_as_high_as_the_bar_less_its_margins() {
    let mut app = app();
    let a = app.world_mut().spawn(button("A")).id();
    let line = app.world_mut().spawn(toolbar_separator()).id();
    let b = app.world_mut().spawn(button("B")).id();
    let bar = app
        .world_mut()
        .spawn(toolbar(()))
        .add_children(&[a, line, b])
        .id();
    app.world_mut().spawn(screen()).add_child(bar);
    settle(&mut app);
    let (line_rect, a_rect) = (rect(&app, line), rect(&app, a));
    assert!(
        line_rect.width() >= 1.0 && line_rect.width() <= 3.0,
        "{line_rect:?}"
    );
    assert!(line_rect.height() > a_rect.height() * 0.5, "{line_rect:?}");
    assert!(line_rect.min.x > a_rect.max.x && line_rect.max.x < rect(&app, b).min.x);
}

/// A toolbar of `count` icon buttons, a separator after the fourth, in a column `width` px wide.
struct Crowd {
    column: Entity,
    bar: Entity,
    buttons: Vec<Entity>,
}

fn crowd(app: &mut App, count: usize, width: f32) -> Crowd {
    let mut kids = Vec::new();
    let mut buttons = Vec::new();
    for i in 0..count {
        let source = picture(app);
        let b = app
            .world_mut()
            .spawn(icon_button(source, Tooltip::new(format!("Tool {i}"))))
            .id();
        buttons.push(b);
        kids.push(b);
    }
    let bar = app.world_mut().spawn(toolbar(())).add_children(&kids).id();
    let column = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(width),
            ..default()
        })
        .add_child(bar)
        .id();
    app.world_mut().spawn(screen()).add_child(column);
    settle(app);
    Crowd {
        column,
        bar,
        buttons,
    }
}

fn hidden(app: &App, crowd: &Crowd) -> Vec<usize> {
    crowd
        .buttons
        .iter()
        .enumerate()
        .filter(|(_, b)| app.world().get::<Overflowed>(**b).is_some())
        .map(|(i, _)| i)
        .collect()
}

fn more_of(app: &App, bar: Entity) -> Entity {
    children_of(app.world(), bar)
        .into_iter()
        .find(|&c| app.world().get::<MenuTitle>(c).is_some())
        .expect("a More button")
}

#[test]
fn a_bar_with_room_for_everything_shows_all_and_has_no_more_button() {
    let mut app = app();
    let crowd = crowd(&mut app, 5, 1000.0);
    assert!(hidden(&app, &crowd).is_empty());
    assert!(
        children_of(app.world(), crowd.bar)
            .into_iter()
            .all(|c| app.world().get::<MenuTitle>(c).is_none())
    );
}

#[test]
fn a_narrow_bar_hides_its_last_items_and_grows_a_more_button_that_lists_exactly_those() {
    let mut app = app();
    let crowd = crowd(&mut app, 10, 300.0);
    let gone = hidden(&app, &crowd);
    assert!(!gone.is_empty() && gone.len() < 10, "{gone:?}");
    // The last ones, in order, and every one shown before them.
    let first = gone[0];
    assert_eq!(gone, (first..10).collect::<Vec<_>>());
    for &shown in &crowd.buttons[..first] {
        let r = rect(&app, shown);
        assert!(r.max.x <= rect(&app, crowd.bar).max.x + 0.5, "{r:?}");
    }
    let more = more_of(&app, crowd.bar);
    assert_ne!(
        app.world().get::<Node>(more).expect("node").display,
        Display::None
    );
    // The button is inside the bar's width, after the last shown item.
    let bar_right = rect(&app, crowd.bar).max.x;
    assert!(rect(&app, more).max.x <= bar_right + 0.5);
    // Open its menu: one row for each hidden item, named by its tooltip.
    click(&mut app, more);
    let rows: Vec<Entity> = descendants(app.world(), more)
        .into_iter()
        .filter(|&e| app.world().get::<MenuItem>(e).is_some())
        .collect();
    assert_eq!(rows.len(), gone.len());
    // Choosing the first row activates the first hidden item.
    app.world_mut().resource_mut::<Activations>().0.clear();
    let row = rows[0];
    click(&mut app, row);
    assert!(
        app.world()
            .resource::<Activations>()
            .0
            .contains(&crowd.buttons[first]),
        "the real item heard its Activate: {:?}",
        app.world().resource::<Activations>().0
    );
}

#[test]
fn widening_the_bar_shows_every_item_again_and_takes_the_more_button_away() {
    let mut app = app();
    let crowd = crowd(&mut app, 10, 300.0);
    assert!(!hidden(&app, &crowd).is_empty());
    app.world_mut()
        .entity_mut(crowd.column)
        .get_mut::<Node>()
        .expect("node")
        .width = Val::Px(1200.0);
    settle(&mut app);
    assert!(
        hidden(&app, &crowd).is_empty(),
        "{:?}",
        hidden(&app, &crowd)
    );
    let more = more_of(&app, crowd.bar);
    assert_eq!(
        app.world().get::<Node>(more).expect("node").display,
        Display::None
    );
    // And narrowing again hides them again: the widths were kept.
    app.world_mut()
        .entity_mut(crowd.column)
        .get_mut::<Node>()
        .expect("node")
        .width = Val::Px(300.0);
    settle(&mut app);
    assert!(!hidden(&app, &crowd).is_empty());
}

#[test]
fn focus_on_an_item_that_hides_goes_to_the_more_button() {
    let mut app = app();
    let crowd = crowd(&mut app, 10, 1200.0);
    let last = crowd.buttons[9];
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(last, FocusCause::Navigated);
    app.update();
    app.world_mut()
        .entity_mut(crowd.column)
        .get_mut::<Node>()
        .expect("node")
        .width = Val::Px(300.0);
    settle(&mut app);
    assert!(app.world().get::<Overflowed>(last).is_some());
    let more = more_of(&app, crowd.bar);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(more));
}

#[test]
fn a_hidden_toggle_group_lists_each_option_as_a_radio_row_with_the_one_on_checked() {
    let mut app = app();
    let mut kids = Vec::new();
    for i in 0..6 {
        let source = picture(&mut app);
        kids.push(
            app.world_mut()
                .spawn(icon_button(source, Tooltip::new(format!("Tool {i}"))))
                .id(),
        );
    }
    let options = ["Move", "Paint", "Erase"].map(|name| {
        let source = picture(&mut app);
        GroupOption::icon(source, Tooltip::new(name))
    });
    let group = app.world_mut().spawn(toggle_group(options, 1)).id();
    kids.push(group);
    let bar = app.world_mut().spawn(toolbar(())).add_children(&kids).id();
    let column = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(260.0),
            ..default()
        })
        .add_child(bar)
        .id();
    app.world_mut().spawn(screen()).add_child(column);
    settle(&mut app);
    assert!(app.world().get::<Overflowed>(group).is_some());
    let more = more_of(&app, bar);
    click(&mut app, more);
    let radios: Vec<(Entity, bool)> = descendants(app.world(), more)
        .into_iter()
        .filter(|&e| app.world().get::<MenuItem>(e).is_some())
        .rev()
        .take(3)
        .map(|e| (e, app.world().get::<Checked>(e).is_some()))
        .collect();
    assert_eq!(radios.len(), 3);
    let on: Vec<bool> = radios.iter().rev().map(|(_, on)| *on).collect();
    assert_eq!(on, vec![false, true, false], "Paint is on");
    // Choosing Erase from the menu changes the group once.
    let erase = radios[0].0;
    click(&mut app, erase);
    assert_eq!(app.world().resource::<Changes>().0, vec![(group, 2)]);
    assert_eq!(
        app.world().get::<ToggleGroupValue>(group),
        Some(&ToggleGroupValue(2))
    );
}

#[test]
fn thirty_idle_frames_write_nothing_for_a_toolbar_that_fits_or_one_that_overflows() {
    let mut app = app();
    tools(&mut app, 0);
    crowd(&mut app, 10, 300.0);
    settle(&mut app);
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0);
}

#[test]
fn an_icon_buttons_tooltip_opens_below_it_and_covers_no_neighbour() {
    let mut app = app();
    let crowd = crowd(&mut app, 4, 1000.0);
    let first = crowd.buttons[0];
    // A chord makes the box wider than the button's left edge leaves room for centred.
    app.world_mut().entity_mut(first).insert(
        Tooltip::new("Save the whole project").with_shortcut(Shortcut::primary(KeyCode::KeyS)),
    );
    settle(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(first, FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    settle(&mut app);
    let tip = children_of(app.world(), first)
        .into_iter()
        .find(|&c| app.world().get::<TooltipBox>(c).is_some())
        .expect("a tooltip box");
    let (tip, button) = (rect(&app, tip), rect(&app, first));
    assert!(tip.width() > 0.0, "the tooltip is shown: {tip:?}");
    assert!(
        tip.min.y >= button.max.y,
        "below the button, not over the bar: tip {tip:?}, button {button:?}"
    );
    for &neighbour in &crowd.buttons[1..] {
        let other = rect(&app, neighbour);
        assert!(
            tip.intersect(other).is_empty(),
            "the tooltip covers a neighbour: {tip:?} over {other:?}"
        );
    }
}
