//! The accessibility tree: what a screen reader is told of every widget, in the same headless window.
#![cfg(feature = "testing")]

use accesskit::{Role as Ak, Toggled};
use bevy::a11y::{AccessibilityNode, AccessibilityRequested, ActionRequest};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, SliderValue, ValueChange};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::{click, find_by_label};
use bevy_gooey::*;

/// How many accessibility nodes changed in the last frame.
#[derive(Resource, Default)]
struct NodesChanged(usize);

/// What a game's observers heard.
#[derive(Resource, Default)]
struct Seen {
    toggles: Vec<bool>,
    numbers: Vec<f32>,
    indices: Vec<usize>,
    texts: Vec<String>,
    activations: usize,
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
    .init_resource::<NodesChanged>()
    .init_resource::<Seen>()
    .add_observer(|change: On<ValueChange<bool>>, mut seen: ResMut<Seen>| {
        seen.toggles.push(change.value);
    })
    .add_observer(|change: On<ValueChange<f32>>, mut seen: ResMut<Seen>| {
        seen.numbers.push(change.value);
    })
    .add_observer(|change: On<ValueChange<usize>>, mut seen: ResMut<Seen>| {
        seen.indices.push(change.value);
    })
    .add_observer(|change: On<ValueChange<String>>, mut seen: ResMut<Seen>| {
        seen.texts.push(change.value.clone());
    })
    .add_observer(|_: On<Activate>, mut seen: ResMut<Seen>| {
        seen.activations += 1;
    })
    .add_systems(
        Last,
        |changed: Query<(), Changed<AccessibilityNode>>, mut count: ResMut<NodesChanged>| {
            count.0 = changed.iter().count();
        },
    );
    app.world_mut().spawn(Camera2d);
    // An assistive technology is connected.
    app.world().resource::<AccessibilityRequested>().set(true);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn node(app: &App, entity: Entity) -> &accesskit::Node {
    &app.world()
        .get::<AccessibilityNode>(entity)
        .unwrap_or_else(|| panic!("{entity:?} has no accessibility node"))
        .0
}

/// A screen holding `widgets`, settled.
fn screen_with(app: &mut App, widgets: impl Bundle) -> Entity {
    let root = app.world_mut().spawn(screen()).id();
    let panel = app.world_mut().spawn((panel(), ChildOf(root))).id();
    app.world_mut().spawn((widgets, ChildOf(panel)));
    settle(app);
    root
}

#[test]
fn a_toggle_is_a_check_box_with_its_label_and_its_state_which_follows_a_click() {
    let mut app = app();
    screen_with(&mut app, toggle("Subtitles", false));
    let toggle = find_by_label(&mut app, "Subtitles").unwrap();
    let before = node(&app, toggle);
    assert_eq!(before.role(), Ak::CheckBox);
    assert_eq!(before.label(), Some("Subtitles"));
    assert_eq!(before.toggled(), Some(Toggled::False));

    click(&mut app, toggle);
    settle(&mut app);
    let after = node(&app, toggle);
    assert_eq!(
        after.toggled(),
        Some(Toggled::True),
        "checked after the click"
    );
    assert_eq!(after.label(), Some("Subtitles"));

    click(&mut app, toggle);
    settle(&mut app);
    assert_eq!(node(&app, toggle).toggled(), Some(Toggled::False));
}

#[test]
fn a_slider_has_its_label_its_number_and_its_words() {
    let mut app = app();
    screen_with(&mut app, slider("Music", 0.7, 0.0..=1.0));
    let slider = find_by_label(&mut app, "Music").unwrap();
    let node = node(&app, slider);
    assert_eq!(node.role(), Ak::Slider);
    assert_eq!(node.label(), Some("Music"));
    assert_eq!(node.value(), Some("70%"));
    assert!((node.numeric_value().unwrap() - 0.7).abs() < 1e-6);
    assert_eq!(node.min_numeric_value(), Some(0.0));
    assert_eq!(node.max_numeric_value(), Some(1.0));
}

#[test]
fn a_choice_reads_its_shown_option_and_follows_the_game() {
    let mut app = app();
    screen_with(&mut app, choice("Quality", ["Low", "Medium", "High"], 1));
    let choice = find_by_label(&mut app, "Quality").unwrap();
    assert_eq!(node(&app, choice).role(), Ak::SpinButton);
    assert_eq!(node(&app, choice).label(), Some("Quality"));
    assert_eq!(node(&app, choice).value(), Some("Medium"));

    app.world_mut().entity_mut(choice).insert(ChoiceValue(2));
    settle(&mut app);
    assert_eq!(node(&app, choice).value(), Some("High"));
}

#[test]
fn a_radial_is_a_spin_button_valued_with_the_highlighted_name_and_follows_the_game() {
    let mut app = app();
    screen_with(&mut app, radial(["Pistol", "Rifle", "Shotgun"], 1));
    let wheel = app
        .world_mut()
        .query_filtered::<Entity, With<RadialCount>>()
        .single(app.world())
        .unwrap();
    assert_eq!(node(&app, wheel).role(), Ak::SpinButton);
    assert_eq!(node(&app, wheel).value(), Some("Rifle"));

    app.world_mut().entity_mut(wheel).insert(RadialValue(2));
    settle(&mut app);
    assert_eq!(node(&app, wheel).value(), Some("Shotgun"));
}

#[test]
fn a_slot_is_a_button_named_by_its_item_with_its_count_and_an_empty_one_says_so() {
    let mut app = app();
    let image = app.world().resource::<AssetServer>().add(Image::default());
    screen_with(
        &mut app,
        slot_grid(
            3,
            [
                Some(Slot::new("Arrows").count(24)),
                Some(Slot::new("Gem").icon(IconSource::Image(image))),
                None,
            ],
        ),
    );
    let grid = app
        .world_mut()
        .query_filtered::<Entity, With<SlotGrid>>()
        .single(app.world())
        .unwrap();
    let slots: Vec<Entity> = app.world().get::<Children>(grid).unwrap().iter().collect();
    assert_eq!(node(&app, slots[0]).role(), Ak::Button);
    assert_eq!(node(&app, slots[0]).label(), Some("Arrows"));
    assert_eq!(node(&app, slots[0]).value(), Some("24"));
    // A picture slot is named by its item too, though the name is not drawn.
    assert_eq!(node(&app, slots[1]).label(), Some("Gem"));
    assert_eq!(node(&app, slots[1]).value(), None);
    assert_eq!(node(&app, slots[2]).label(), Some("Empty slot"));

    // The names follow the data.
    app.world_mut()
        .get_mut::<SlotItems>(grid)
        .unwrap()
        .0
        .swap(0, 2);
    settle(&mut app);
    assert_eq!(node(&app, slots[0]).label(), Some("Empty slot"));
    assert_eq!(node(&app, slots[2]).label(), Some("Arrows"));
    assert_eq!(node(&app, slots[2]).value(), Some("24"));
}

#[test]
fn a_button_is_named_by_its_text_and_an_icon_button_by_its_tooltip() {
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let button = app
        .world_mut()
        .spawn((button("Resume"), ChildOf(root)))
        .id();
    let icon_button = app
        .world_mut()
        .spawn((
            icon_button(
                IconSource::Image(Handle::default()),
                Tooltip::new("Zoom in"),
            ),
            ChildOf(root),
        ))
        .id();
    settle(&mut app);
    assert_eq!(node(&app, button).role(), Ak::Button);
    assert_eq!(node(&app, button).label(), Some("Resume"));
    assert_eq!(node(&app, icon_button).role(), Ak::Button);
    assert_eq!(node(&app, icon_button).label(), Some("Zoom in"));
}

#[test]
fn an_icon_is_hidden_and_a_disabled_control_says_so() {
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let icon_button = app
        .world_mut()
        .spawn((
            icon_button(IconSource::Image(Handle::default()), Tooltip::new("Pan")),
            ChildOf(root),
        ))
        .id();
    let button = app.world_mut().spawn((button("Buy"), ChildOf(root))).id();
    settle(&mut app);
    let icon = app
        .world()
        .entity(icon_button)
        .get::<Children>()
        .unwrap()
        .iter()
        .find(|child| app.world().get::<Icon>(*child).is_some())
        .unwrap();
    assert!(
        node(&app, icon).is_hidden(),
        "an icon beside a label adds nothing"
    );
    assert!(!node(&app, button).is_disabled());

    app.world_mut()
        .entity_mut(button)
        .insert(InteractionDisabled);
    settle(&mut app);
    assert!(node(&app, button).is_disabled());
    app.world_mut()
        .entity_mut(button)
        .remove::<InteractionDisabled>();
    settle(&mut app);
    assert!(!node(&app, button).is_disabled());
}

#[test]
fn text_a_meter_and_a_progress_bar_are_a_heading_a_label_and_numbers() {
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let heading = app
        .world_mut()
        .spawn((heading("Paused"), ChildOf(root)))
        .id();
    let label = app.world_mut().spawn((label("Health"), ChildOf(root))).id();
    let meter = app
        .world_mut()
        .spawn((meter(30.0, 120.0), ChildOf(root)))
        .id();
    let progress = app.world_mut().spawn((progress(0.25), ChildOf(root))).id();
    settle(&mut app);
    assert_eq!(node(&app, heading).role(), Ak::Heading);
    assert_eq!(node(&app, heading).label(), Some("Paused"));
    assert_eq!(node(&app, label).role(), Ak::Label);
    assert_eq!(node(&app, label).value(), Some("Health"));
    assert_eq!(node(&app, meter).role(), Ak::Meter);
    assert_eq!(node(&app, meter).numeric_value(), Some(30.0));
    assert_eq!(node(&app, meter).max_numeric_value(), Some(120.0));
    assert_eq!(node(&app, progress).role(), Ak::ProgressIndicator);
    assert_eq!(node(&app, progress).numeric_value(), Some(0.25));

    app.world_mut().entity_mut(meter).insert(Meter {
        value: 60.0,
        max: 120.0,
    });
    settle(&mut app);
    assert_eq!(node(&app, meter).numeric_value(), Some(60.0));
}

#[test]
fn a_dialog_is_a_modal_dialog_named_by_its_heading() {
    let mut app = app();
    let root = app.world_mut().spawn(dialog()).id();
    let panel = app.world_mut().spawn((panel(), ChildOf(root))).id();
    app.world_mut()
        .spawn((heading("Quit the game?"), ChildOf(panel)));
    app.world_mut().spawn((button("Quit"), ChildOf(panel)));
    settle(&mut app);
    let node = node(&app, root);
    assert_eq!(node.role(), Ak::Dialog);
    assert!(node.is_modal());
    assert_eq!(node.label(), Some("Quit the game?"));
}

#[test]
fn a_text_field_is_named_by_its_row_and_holds_its_text() {
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let name = app
        .world_mut()
        .spawn((text_input("Name", "Ada"), ChildOf(root)))
        .id();
    let notes = app
        .world_mut()
        .spawn((text_area("Notes", "two\nlines", 3), ChildOf(root)))
        .id();
    settle(&mut app);
    let field_of = |app: &mut App, row: Entity| {
        app.world_mut()
            .query::<(Entity, &TextInputOf)>()
            .iter(app.world())
            .find(|(field, of)| of.0 == row && app.world().get::<TabIndex>(*field).is_some())
            .map(|(field, _)| field)
            .unwrap()
    };
    let name_field = field_of(&mut app, name);
    assert_eq!(node(&app, name_field).role(), Ak::TextInput);
    assert_eq!(node(&app, name_field).label(), Some("Name"));
    assert_eq!(node(&app, name_field).value(), Some("Ada"));
    let notes_field = field_of(&mut app, notes);
    assert_eq!(node(&app, notes_field).role(), Ak::MultilineTextInput);
    assert_eq!(node(&app, notes_field).label(), Some("Notes"));

    app.world_mut()
        .entity_mut(name)
        .insert(TextInputValue("Grace".into()));
    settle(&mut app);
    assert_eq!(node(&app, name_field).value(), Some("Grace"));
}

#[test]
fn a_page_that_is_not_shown_and_a_closed_menu_are_hidden() {
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    app.world_mut().spawn((
        tabs(
            ["Video", "Audio"],
            0,
            children![
                tab_page(children![label("video page")]),
                tab_page(children![
                    label("audio page"),
                    choice("Quality", ["Low", "Medium"], 1)
                ]),
            ],
        ),
        ChildOf(root),
    ));
    let bar = app
        .world_mut()
        .spawn((
            menu_bar(children![menu_title(
                "File",
                children![menu_item("Open"), check_item("Grid", true)]
            )]),
            ChildOf(root),
        ))
        .id();
    settle(&mut app);
    let world = app.world_mut();
    let pages: Vec<Entity> = world
        .query::<(Entity, &Role)>()
        .iter(world)
        .filter(|(_, role)| **role == Role::TabPage)
        .map(|(e, _)| e)
        .collect();
    assert_eq!(pages.len(), 2);
    let shown: Vec<bool> = pages
        .iter()
        .map(|page| !node(&app, *page).is_hidden())
        .collect();
    assert_eq!(shown.iter().filter(|s| **s).count(), 1, "one page is read");
    for page in pages {
        assert_eq!(node(&app, page).role(), Ak::TabPanel);
    }
    // A choice on the page that is hidden still holds its value.
    let world = app.world_mut();
    let quality = world
        .query::<(Entity, &Role)>()
        .iter(world)
        .find(|(_, role)| **role == Role::Choice)
        .map(|(e, _)| e)
        .unwrap();
    assert_eq!(node(&app, quality).value(), Some("Medium"));
    assert!(node(&app, quality).is_hidden());

    let video = find_by_label(&mut app, "Video").unwrap();
    assert_eq!(node(&app, video).role(), Ak::Tab);
    assert_eq!(node(&app, video).label(), Some("Video"));
    assert_eq!(node(&app, video).is_selected(), Some(true));
    let audio = find_by_label(&mut app, "Audio").unwrap();
    assert_eq!(node(&app, audio).is_selected(), Some(false));

    let world = app.world_mut();
    let panels: Vec<Entity> = world
        .query::<(Entity, &Role)>()
        .iter(world)
        .filter(|(_, role)| **role == Role::MenuPanel)
        .map(|(e, _)| e)
        .collect();
    assert_eq!(panels.len(), 1);
    assert!(
        node(&app, panels[0]).is_hidden(),
        "a closed menu is not read"
    );
    assert_eq!(node(&app, panels[0]).role(), Ak::Menu);
    assert_eq!(node(&app, bar).role(), Ak::MenuBar);

    let title = find_by_label(&mut app, "File").unwrap();
    assert_eq!(node(&app, title).role(), Ak::MenuItem);
    assert_eq!(node(&app, title).is_expanded(), Some(false));
    click(&mut app, title);
    settle(&mut app);
    assert_eq!(node(&app, title).is_expanded(), Some(true));
    assert!(!node(&app, panels[0]).is_hidden(), "an open menu is read");
    // Its rows: a command, and a check that is checked.
    let open = find_by_label(&mut app, "Open").unwrap();
    assert_eq!(node(&app, open).role(), Ak::MenuItem);
    assert_eq!(node(&app, open).label(), Some("Open"));
    let grid = find_by_label(&mut app, "Grid").unwrap();
    assert_eq!(node(&app, grid).role(), Ak::MenuItemCheckBox);
    assert_eq!(node(&app, grid).toggled(), Some(Toggled::True));
}

#[test]
fn a_toast_is_a_status_and_an_alert_is_an_alert() {
    let mut app = app();
    app.world_mut().spawn(toast("Saved"));
    app.world_mut().spawn(toast_alert("Disk full"));
    settle(&mut app);
    let world = app.world_mut();
    let toasts: Vec<(Entity, Role)> = world
        .query::<(Entity, &Role)>()
        .iter(world)
        .filter(|(_, role)| **role == Role::Toast)
        .map(|(e, r)| (e, *r))
        .collect();
    assert_eq!(toasts.len(), 2);
    let mut seen: Vec<(Ak, String)> = toasts
        .iter()
        .map(|(e, _)| {
            let n = node(&app, *e);
            (n.role(), n.label().unwrap_or_default().to_owned())
        })
        .collect();
    seen.sort_by_key(|(_, label)| label.clone());
    assert_eq!(
        seen,
        vec![
            (Ak::Alert, "Disk full".to_owned()),
            (Ak::Status, "Saved".to_owned())
        ]
    );
    // A toast appears by itself, so it is a live region: an alert interrupts and a plain toast
    // waits its turn. Without this a screen reader never heard either.
    for (entity, _) in &toasts {
        let node = node(&app, *entity);
        let live = node.live();
        let label = node.label().unwrap_or_default();
        match label {
            "Disk full" => assert_eq!(live, Some(accesskit::Live::Assertive)),
            _ => assert_eq!(live, Some(accesskit::Live::Polite)),
        }
    }
}

#[test]
fn a_table_a_split_and_a_section_header_say_their_state() {
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let table = app
        .world_mut()
        .spawn((
            table(
                [Column::new("Name"), Column::new("Qty")],
                vec![
                    vec!["Rope".to_string(), "2".to_string()],
                    vec!["Lamp".to_string(), "1".to_string()],
                ],
            ),
            ChildOf(root),
        ))
        .id();
    let split = app
        .world_mut()
        .spawn((
            split(SplitAxis::Horizontal, label("left"), label("right")),
            ChildOf(root),
        ))
        .id();
    let section = app
        .world_mut()
        .spawn((
            section("Physics", true, children![label("gravity")]),
            ChildOf(root),
        ))
        .id();
    settle(&mut app);
    assert_eq!(node(&app, table).role(), Ak::Table);
    let world = app.world_mut();
    let rows: Vec<Entity> = world
        .query::<(Entity, &Role)>()
        .iter(world)
        .filter(|(_, role)| **role == Role::TableRow)
        .map(|(e, _)| e)
        .collect();
    assert!(rows.len() >= 2);
    for row in rows {
        assert_eq!(node(&app, row).role(), Ak::Row);
    }
    let name = find_by_label(&mut app, "Name").unwrap();
    assert_eq!(node(&app, name).label(), Some("Name"));
    let world = app.world_mut();
    let divider = world
        .query::<(Entity, &SplitDivider)>()
        .iter(world)
        .find(|(e, _)| {
            world
                .get::<ChildOf>(*e)
                .is_some_and(|c| c.parent() == split)
        })
        .map(|(e, _)| e)
        .unwrap();
    assert_eq!(node(&app, divider).role(), Ak::Splitter);
    assert_eq!(node(&app, divider).numeric_value(), Some(0.5));
    assert_eq!(node(&app, divider).label(), Some("Divider"));

    let header = find_by_label(&mut app, "Physics").unwrap();
    assert_eq!(node(&app, header).is_expanded(), Some(true));
    app.world_mut()
        .entity_mut(section)
        .insert(SectionOpen(false));
    settle(&mut app);
    assert_eq!(node(&app, header).is_expanded(), Some(false));
}

#[test]
fn a_binding_row_reads_its_key_and_what_it_waits_for() {
    let mut app = app();
    screen_with(&mut app, binding("Jump", Binding::Key(KeyCode::Space)));
    let row = find_by_label(&mut app, "Jump").unwrap();
    assert_eq!(node(&app, row).label(), Some("Jump"));
    assert_eq!(node(&app, row).value(), Some("Space"));
    click(&mut app, row);
    settle(&mut app);
    assert_ne!(
        node(&app, row).value(),
        Some("Space"),
        "it waits for a press"
    );
}

#[test]
fn every_focus_stop_of_a_busy_screen_has_a_role_and_a_name() {
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let column = app.world_mut().spawn((panel(), ChildOf(root))).id();
    app.world_mut().entity_mut(column).with_children(|p| {
        p.spawn(heading("Settings"));
        p.spawn(button("Resume"));
        p.spawn(toggle("Subtitles", true));
        p.spawn(slider("Music", 0.5, 0.0..=1.0));
        p.spawn(choice("Quality", ["Low", "High"], 0));
        p.spawn(text_size("Text size"));
        p.spawn(binding("Jump", Binding::Key(KeyCode::Space)));
        p.spawn(text_input("Name", "Ada"));
        p.spawn(number_field("Health", 50.0, 1.0, 0.0..=100.0));
        p.spawn(color_field("Tint", Srgba::new(0.2, 0.4, 0.8, 1.0)));
        p.spawn(select("Map", ["a", "b"], 0));
        p.spawn(toggle_group(
            [GroupOption::text("Day"), GroupOption::text("Night")],
            0,
        ));
    });
    settle(&mut app);
    let world = app.world_mut();
    let stops: Vec<Entity> = world
        .query_filtered::<Entity, With<TabIndex>>()
        .iter(world)
        .collect();
    assert!(stops.len() >= 12, "{} focus stops", stops.len());
    let mut bare = Vec::new();
    for stop in stops {
        let Some(node) = app.world().get::<AccessibilityNode>(stop) else {
            bare.push(format!("{stop:?}: no node"));
            continue;
        };
        let named = node.label().is_some_and(|l| !l.is_empty());
        if node.role() == Ak::Unknown || !named {
            bare.push(format!(
                "{stop:?}: {:?} {:?} {:?}",
                node.role(),
                node.label(),
                app.world().get::<Role>(stop)
            ));
        }
    }
    assert!(bare.is_empty(), "unnamed focus stops: {bare:#?}");
}

#[test]
fn with_no_screen_reader_nothing_is_written_and_a_late_one_gets_every_name() {
    let mut app = app();
    app.world().resource::<AccessibilityRequested>().set(false);
    screen_with(&mut app, toggle("Subtitles", true));
    let toggle = find_by_label(&mut app, "Subtitles").unwrap();
    assert_eq!(node(&app, toggle).label(), None, "nobody is listening");

    app.world().resource::<AccessibilityRequested>().set(true);
    settle(&mut app);
    assert_eq!(node(&app, toggle).label(), Some("Subtitles"));
    assert_eq!(node(&app, toggle).toggled(), Some(Toggled::True));
}

#[test]
fn a_settled_interface_marks_no_node_changed() {
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let column = app.world_mut().spawn((panel(), ChildOf(root))).id();
    app.world_mut().entity_mut(column).with_children(|p| {
        p.spawn(heading("Settings"));
        p.spawn(button("Resume"));
        p.spawn(toggle("Subtitles", true));
        p.spawn(slider("Music", 0.5, 0.0..=1.0));
        p.spawn(choice("Quality", ["Low", "High"], 0));
        p.spawn(meter(5.0, 10.0));
        p.spawn(outline());
        p.spawn(table(
            [Column::new("Name")],
            vec![vec!["Rope".to_string()], vec!["Lamp".to_string()]],
        ));
        p.spawn(radial(["Sword", "Bow"], 0));
    });
    for _ in 0..30 {
        app.update();
    }
    for _ in 0..10 {
        app.update();
        assert_eq!(app.world().resource::<NodesChanged>().0, 0);
    }
}

#[test]
fn a_hosts_own_label_wins_over_the_crates_words() {
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let button = app
        .world_mut()
        .spawn((
            button("OK"),
            AccessibleLabel::new("Accept the terms"),
            ChildOf(root),
        ))
        .id();
    settle(&mut app);
    assert_eq!(node(&app, button).label(), Some("Accept the terms"));
}

// --- Actions: what an assistive technology can do to what it reads ---------------------------

/// Asks, as a platform's screen reader does, for `action` on `target`, and lets the frame run.
fn act(
    app: &mut App,
    target: Entity,
    action: accesskit::Action,
    data: Option<accesskit::ActionData>,
) {
    app.world_mut()
        .write_message(ActionRequest(accesskit::ActionRequest {
            action,
            target_tree: accesskit::TreeId::ROOT,
            target_node: accesskit::NodeId(target.to_bits()),
            data,
        }));
    settle(app);
}

fn seen(app: &App) -> &Seen {
    app.world().resource::<Seen>()
}

fn only<T: Component>(app: &mut App) -> Entity {
    let world = app.world_mut();
    let mut found = world.query_filtered::<Entity, With<T>>();
    let all: Vec<Entity> = found.iter(world).collect();
    assert_eq!(all.len(), 1, "exactly one");
    all[0]
}

#[test]
fn a_node_declares_exactly_the_actions_its_widget_takes() {
    use accesskit::Action as A;
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let column = app.world_mut().spawn((panel(), ChildOf(root))).id();
    app.world_mut().entity_mut(column).with_children(|p| {
        p.spawn(heading("Settings"));
        p.spawn(button("Resume"));
        p.spawn(toggle("Subtitles", true));
        p.spawn(slider("Music", 0.5, 0.0..=1.0));
        p.spawn(choice("Quality", ["Low", "High"], 0));
        p.spawn(text_input("Name", "Ada"));
        p.spawn(section("Audio", false, children![]));
        p.spawn((button("Locked"), InteractionDisabled));
    });
    settle(&mut app);
    let declared = |app: &mut App, label: &str| {
        let entity = find_by_label(app, label).unwrap();
        let node = node(app, entity);
        [
            A::Click,
            A::Focus,
            A::Increment,
            A::Decrement,
            A::SetValue,
            A::Expand,
            A::Collapse,
        ]
        .into_iter()
        .filter(|a| node.supports_action(*a))
        .collect::<Vec<_>>()
    };
    assert_eq!(declared(&mut app, "Resume"), [A::Click, A::Focus]);
    assert_eq!(declared(&mut app, "Subtitles"), [A::Click, A::Focus]);
    assert_eq!(
        declared(&mut app, "Music"),
        [A::Focus, A::Increment, A::Decrement, A::SetValue]
    );
    assert_eq!(
        declared(&mut app, "Quality"),
        [A::Click, A::Focus, A::Increment, A::Decrement]
    );
    let name = {
        let world = app.world_mut();
        let mut fields = world.query_filtered::<
            (Entity, &TextInputOf),
            (With<bevy::ui_widgets::TextInput>, Without<NumberInput>),
        >();
        let (field, _) = fields.iter(world).next().expect("a text field");
        field
    };
    let declared_by = |app: &App, entity: Entity| {
        [A::Focus, A::SetValue, A::Click, A::Increment]
            .into_iter()
            .filter(|a| node(app, entity).supports_action(*a))
            .collect::<Vec<_>>()
    };
    assert_eq!(declared_by(&app, name), [A::Focus, A::SetValue]);
    assert_eq!(
        declared(&mut app, "Audio"),
        [A::Click, A::Focus, A::Expand, A::Collapse]
    );
    assert_eq!(
        declared(&mut app, "Locked"),
        [],
        "a disabled widget takes nothing"
    );
    let heading = find_heading(&mut app);
    assert!(
        !node(&app, heading).supports_action(A::Focus),
        "a heading is read, not used"
    );
}

fn find_heading(app: &mut App) -> Entity {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &Role)>();
    q.iter(world)
        .find(|(_, role)| **role == Role::Heading)
        .map(|(e, _)| e)
        .expect("a heading")
}

#[test]
fn clicking_a_toggle_flips_it_once_and_a_disabled_one_stays() {
    use accesskit::Action as A;
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    app.world_mut()
        .spawn((toggle("Subtitles", false), ChildOf(root)));
    app.world_mut()
        .spawn((toggle("Shake", false), InteractionDisabled, ChildOf(root)));
    settle(&mut app);
    let subtitles = find_by_label(&mut app, "Subtitles").unwrap();
    act(&mut app, subtitles, A::Click, None);
    assert_eq!(seen(&app).toggles, [true], "one change");
    assert_eq!(node(&app, subtitles).toggled(), Some(Toggled::True));
    act(&mut app, subtitles, A::Click, None);
    assert_eq!(seen(&app).toggles, [true, false]);

    let shake = find_by_label(&mut app, "Shake").unwrap();
    act(&mut app, shake, A::Click, None);
    assert_eq!(
        seen(&app).toggles.len(),
        2,
        "a disabled toggle is not flipped"
    );
}

#[test]
fn clicking_a_button_activates_it_once_and_a_disabled_one_never() {
    use accesskit::Action as A;
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    app.world_mut().spawn((button("Resume"), ChildOf(root)));
    app.world_mut()
        .spawn((button("Quit"), InteractionDisabled, ChildOf(root)));
    settle(&mut app);
    let resume = find_by_label(&mut app, "Resume").unwrap();
    act(&mut app, resume, A::Click, None);
    assert_eq!(seen(&app).activations, 1);
    let quit = find_by_label(&mut app, "Quit").unwrap();
    act(&mut app, quit, A::Click, None);
    assert_eq!(seen(&app).activations, 1);
}

#[test]
fn a_slider_is_set_stepped_and_held_to_its_range() {
    use accesskit::{Action as A, ActionData as D};
    let mut app = app();
    screen_with(&mut app, slider("Music", 0.5, 0.0..=1.0));
    let music = find_by_label(&mut app, "Music").unwrap();
    let value = |app: &App| app.world().get::<SliderValue>(music).unwrap().0;

    act(&mut app, music, A::SetValue, Some(D::NumericValue(0.25)));
    assert!((value(&app) - 0.25).abs() < 1e-6, "{}", value(&app));
    assert_eq!(seen(&app).numbers.len(), 1, "one change");
    assert!((node(&app, music).numeric_value().unwrap() - 0.25).abs() < 1e-6);

    act(&mut app, music, A::Increment, None);
    assert!(
        (value(&app) - 0.30).abs() < 1e-4,
        "one step up: {}",
        value(&app)
    );
    act(&mut app, music, A::Decrement, None);
    act(&mut app, music, A::Decrement, None);
    assert!((value(&app) - 0.20).abs() < 1e-4, "{}", value(&app));

    act(&mut app, music, A::SetValue, Some(D::NumericValue(7.0)));
    assert_eq!(value(&app), 1.0, "held to the range");
    act(&mut app, music, A::SetValue, Some(D::Value("loud".into())));
    assert_eq!(value(&app), 1.0, "words are not a number");
}

#[test]
fn a_choice_steps_one_option_and_stops_at_its_end() {
    use accesskit::Action as A;
    let mut app = app();
    screen_with(&mut app, choice("Quality", ["Low", "Medium", "High"], 1));
    let quality = find_by_label(&mut app, "Quality").unwrap();
    act(&mut app, quality, A::Increment, None);
    assert_eq!(seen(&app).indices, [2]);
    assert_eq!(node(&app, quality).value(), Some("High"));
    act(&mut app, quality, A::Increment, None);
    assert_eq!(seen(&app).indices, [2], "the end says nothing");
    act(&mut app, quality, A::Decrement, None);
    assert_eq!(seen(&app).indices, [2, 1]);
}

#[test]
fn a_text_field_takes_the_words_it_is_set_to_and_a_number_field_holds_them_to_its_range() {
    use accesskit::{Action as A, ActionData as D};
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let column = app.world_mut().spawn((panel(), ChildOf(root))).id();
    app.world_mut().entity_mut(column).with_children(|p| {
        p.spawn(text_input("Name", "Ada"));
        p.spawn(number_field("Health", 50.0, 1.0, 0.0..=100.0));
    });
    settle(&mut app);
    let world = app.world_mut();
    let mut fields = world.query::<(Entity, &TextInputOf, Has<NumberInput>)>();
    let fields: Vec<(Entity, Entity, bool)> = fields
        .iter(world)
        .map(|(e, row, number)| (e, row.0, number))
        .collect();
    let (name, name_row, _) = *fields.iter().find(|f| !f.2).expect("a text field");
    let (health, health_row, _) = *fields.iter().find(|f| f.2).expect("a number field");

    act(&mut app, name, A::SetValue, Some(D::Value("Grace".into())));
    assert_eq!(seen(&app).texts.first().map(String::as_str), Some("Grace"));
    assert_eq!(
        app.world().get::<TextInputValue>(name_row).unwrap().0,
        "Grace"
    );

    act(&mut app, health, A::SetValue, Some(D::NumericValue(150.0)));
    assert_eq!(seen(&app).numbers, [100.0], "held to the range");
    assert_eq!(app.world().get::<NumberValue>(health_row).unwrap().0, 100.0);
    act(&mut app, health, A::Decrement, None);
    assert_eq!(seen(&app).numbers, [100.0, 99.0]);
}

#[test]
fn clicking_a_tab_selects_its_page_once_and_the_selected_one_says_nothing() {
    use accesskit::Action as A;
    let mut app = app();
    screen_with(
        &mut app,
        tabs(
            ["Video", "Audio"],
            0,
            children![tab_page(children![]), tab_page(children![])],
        ),
    );
    let audio = find_by_label(&mut app, "Audio").unwrap();
    act(&mut app, audio, A::Click, None);
    assert_eq!(seen(&app).indices, [1]);
    act(&mut app, audio, A::Click, None);
    assert_eq!(seen(&app).indices, [1], "already shown");
    let video = find_by_label(&mut app, "Video").unwrap();
    act(&mut app, video, A::Click, None);
    assert_eq!(seen(&app).indices, [1, 0]);
}

#[test]
fn a_section_opens_on_expand_and_shuts_on_collapse_and_never_toggles_for_the_state_it_is_in() {
    use accesskit::Action as A;
    let mut app = app();
    screen_with(&mut app, section("Audio", false, children![]));
    let header = find_by_label(&mut app, "Audio").unwrap();
    act(&mut app, header, A::Collapse, None);
    assert!(seen(&app).toggles.is_empty());
    assert_eq!(node(&app, header).is_expanded(), Some(false));
    act(&mut app, header, A::Expand, None);
    assert_eq!(node(&app, header).is_expanded(), Some(true));
    act(&mut app, header, A::Expand, None);
    assert_eq!(node(&app, header).is_expanded(), Some(true), "still open");
    act(&mut app, header, A::Collapse, None);
    assert_eq!(node(&app, header).is_expanded(), Some(false));
}

#[test]
fn focus_lands_where_tab_would_and_shows_the_ring() {
    use accesskit::Action as A;
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    app.world_mut().spawn((button("One"), ChildOf(root)));
    app.world_mut().spawn((button("Two"), ChildOf(root)));
    settle(&mut app);
    let two = find_by_label(&mut app, "Two").unwrap();
    assert!(!app.world().resource::<InputFocusVisible>().0);
    act(&mut app, two, A::Focus, None);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(two));
    assert!(
        app.world().resource::<InputFocusVisible>().0,
        "the ring shows"
    );
}

#[test]
fn a_request_for_a_node_that_is_gone_or_never_a_widget_does_nothing() {
    use accesskit::Action as A;
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let gone = app.world_mut().spawn((button("Old"), ChildOf(root))).id();
    app.world_mut().spawn((heading("Settings"), ChildOf(root)));
    settle(&mut app);
    let window = only::<bevy::window::PrimaryWindow>(&mut app);
    let title = find_heading(&mut app);
    app.world_mut().despawn(gone);
    settle(&mut app);
    for target in [gone, window, title, root] {
        for action in [A::Click, A::Focus, A::Increment, A::SetValue, A::Expand] {
            act(&mut app, target, action, None);
        }
    }
    assert_eq!(seen(&app).activations, 0);
    assert!(app.world().resource::<InputFocus>().get() != Some(title));
}

// --- A virtualized table and tree: a row knows its place and level -----------------------------

/// The shown row of `table` for `key`, once the pool has bound it.
fn row_of(app: &mut App, key: u64) -> Entity {
    let world = app.world_mut();
    let rows: Vec<(Entity, u64)> = world
        .query::<(Entity, &TableRow)>()
        .iter(world)
        .map(|(entity, row)| (entity, row.key))
        .collect();
    let (entity, _) = rows
        .into_iter()
        .filter(|(entity, _)| {
            world
                .get::<AccessibilityNode>(*entity)
                .is_some_and(|node| !node.0.is_hidden())
        })
        .find(|(_, k)| *k == key)
        .unwrap_or_else(|| panic!("no shown row for key {key}"));
    entity
}

/// A tree of A { B { D }, C } and E, with B shut: A, B, C and E are shown.
fn outline() -> impl Bundle {
    tree([
        TreeNode::new(1, "A")
            .expanded()
            .child(TreeNode::new(2, "B").child(TreeNode::new(4, "D")))
            .child(TreeNode::new(3, "C")),
        TreeNode::new(5, "E"),
    ])
}

#[derive(Resource, Default)]
struct Toggles(Vec<(u64, bool)>);

#[test]
fn a_tree_row_says_its_level_whether_it_is_open_and_its_place_among_the_rows_shown() {
    let mut app = app();
    screen_with(&mut app, outline());
    let tree = only::<Table>(&mut app);
    assert_eq!(node(&app, tree).role(), Ak::Tree);
    assert_eq!(node(&app, tree).size_of_set(), Some(4), "the rows shown");
    // (key, level, expanded, position): AccessKit counts both from 0.
    for (key, level, open, at) in [
        (1, 0, Some(true), 0),
        (2, 1, Some(false), 1),
        (3, 1, None, 2),
        (5, 0, None, 3),
    ] {
        let row = row_of(&mut app, key);
        let n = node(&app, row);
        assert_eq!(n.role(), Ak::TreeItem, "key {key}");
        assert_eq!(n.level(), Some(level), "level of {key}");
        assert_eq!(n.is_expanded(), open, "expanded of {key}");
        assert_eq!(n.position_in_set(), Some(at), "position of {key}");
    }
}

#[test]
fn a_row_the_pool_rebinds_from_a_branch_to_an_end_node_drops_its_expanded_state() {
    let mut app = app();
    screen_with(&mut app, outline());
    let tree = only::<Table>(&mut app);
    // Closing A leaves A and E: the pool's rows are rebound, B's row is now E's.
    if let Some(mut table) = app.world_mut().get_mut::<Table>(tree)
        && let Some(rows) = table.source_mut::<Tree>()
    {
        rows.set_open(1, false);
    }
    settle(&mut app);
    assert_eq!(node(&app, tree).size_of_set(), Some(2));
    let e = row_of(&mut app, 5);
    assert_eq!(
        node(&app, e).is_expanded(),
        None,
        "an end node is not expandable"
    );
    assert_eq!(node(&app, e).position_in_set(), Some(1));
    let a = row_of(&mut app, 1);
    assert_eq!(node(&app, a).is_expanded(), Some(false));
}

#[test]
fn an_assistive_technology_opens_and_closes_a_tree_node_through_the_trees_own_event() {
    use accesskit::Action as A;
    let mut app = app();
    app.init_resource::<Toggles>()
        .add_observer(|t: On<TreeToggled>, mut log: ResMut<Toggles>| {
            log.0.push((t.key, t.open));
        });
    screen_with(&mut app, outline());
    let tree = only::<Table>(&mut app);
    let supports = |app: &App, row: Entity| {
        [A::Expand, A::Collapse, A::Click, A::Focus]
            .into_iter()
            .filter(|a| node(app, row).supports_action(*a))
            .collect::<Vec<_>>()
    };
    let b = row_of(&mut app, 2);
    let c = row_of(&mut app, 3);
    assert_eq!(supports(&app, b), [A::Expand, A::Collapse]);
    assert_eq!(supports(&app, c), [], "an end node has nothing to open");

    act(&mut app, b, A::Expand, None);
    assert_eq!(app.world().resource::<Toggles>().0, [(2, true)]);
    assert_eq!(node(&app, tree).size_of_set(), Some(5), "D is shown");
    let d = row_of(&mut app, 4);
    assert_eq!(node(&app, d).level(), Some(2));
    assert_eq!(node(&app, d).position_in_set(), Some(2));

    let b = row_of(&mut app, 2);
    act(&mut app, b, A::Expand, None);
    assert_eq!(app.world().resource::<Toggles>().0.len(), 1, "already open");
    let c = row_of(&mut app, 3);
    act(&mut app, c, A::Collapse, None);
    assert_eq!(app.world().resource::<Toggles>().0.len(), 1, "an end node");
    let b = row_of(&mut app, 2);
    act(&mut app, b, A::Collapse, None);
    assert_eq!(app.world().resource::<Toggles>().0, [(2, true), (2, false)]);
    assert_eq!(node(&app, tree).size_of_set(), Some(4));
}

#[test]
fn a_disabled_tree_declares_and_takes_no_expand() {
    use accesskit::Action as A;
    let mut app = app();
    app.init_resource::<Toggles>()
        .add_observer(|t: On<TreeToggled>, mut log: ResMut<Toggles>| {
            log.0.push((t.key, t.open));
        });
    screen_with(&mut app, (outline(), InteractionDisabled));
    let b = row_of(&mut app, 2);
    assert!(!node(&app, b).supports_action(A::Expand));
    act(&mut app, b, A::Expand, None);
    assert!(app.world().resource::<Toggles>().0.is_empty());
}

#[test]
fn a_table_says_how_many_rows_and_columns_it_has_and_each_row_its_index() {
    let mut app = app();
    let rows: Vec<Vec<String>> = (0..40)
        .map(|i| vec![format!("item {i}"), i.to_string()])
        .collect();
    screen_with(
        &mut app,
        table([Column::new("Name"), Column::new("Qty")], rows),
    );
    let table = only::<Table>(&mut app);
    assert_eq!(
        node(&app, table).row_count(),
        Some(40),
        "all of them, not the pool"
    );
    assert_eq!(node(&app, table).column_count(), Some(2));
    for key in [0, 1, 5] {
        let row = row_of(&mut app, key);
        assert_eq!(node(&app, row).row_index(), Some(key as usize), "row {key}");
    }
}

#[test]
fn a_radial_declares_and_takes_increment_and_decrement_and_wraps_like_the_arrow_keys() {
    use accesskit::Action as A;
    let mut app = app();
    screen_with(&mut app, radial(["Sword", "Bow", "Staff"], 0));
    let wheel = only::<RadialValue>(&mut app);
    for a in [A::Increment, A::Decrement, A::Focus, A::Click] {
        assert!(node(&app, wheel).supports_action(a), "{a:?}");
    }
    act(&mut app, wheel, A::Decrement, None);
    assert_eq!(app.world().get::<RadialValue>(wheel), Some(&RadialValue(2)));
    act(&mut app, wheel, A::Increment, None);
    assert_eq!(app.world().get::<RadialValue>(wheel), Some(&RadialValue(0)));
    act(&mut app, wheel, A::Increment, None);
    assert_eq!(app.world().get::<RadialValue>(wheel), Some(&RadialValue(1)));
    assert_eq!(seen(&app).indices, [2, 0, 1], "one report a step");
}

#[test]
fn a_splitters_node_takes_steps_and_a_value() {
    use accesskit::Action as A;
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let split = app
        .world_mut()
        .spawn((
            split(
                SplitAxis::Horizontal,
                (panel(), children![label("A")]),
                (panel(), children![label("B")]),
            ),
            SplitFraction(0.5),
            SplitMin::new(80.0, 80.0),
            ChildOf(root),
        ))
        .id();
    settle(&mut app);
    let divider = {
        let mut stack = vec![split];
        let mut found = None;
        while let Some(entity) = stack.pop() {
            if let Some(kids) = app.world().get::<Children>(entity) {
                for &kid in &**kids {
                    if app.world().get::<Role>(kid) == Some(&Role::SplitDivider) {
                        found = Some(kid);
                    }
                    stack.push(kid);
                }
            }
        }
        found.expect("a divider")
    };
    settle(&mut app);
    // The node is a splitter with a value, and it declares the actions a screen reader needs to
    // move it: before this it declared none, so a reader could read the value and not change it.
    let node = node(&app, divider);
    assert_eq!(node.role(), accesskit::Role::Splitter);
    for action in [A::Focus, A::Increment, A::Decrement, A::SetValue] {
        assert!(node.supports_action(action), "it declares {action:?}");
    }
    assert!(!node.supports_action(A::Click), "a divider is not clicked");
    // A step moves the fraction; the split's own ValueChange<f32> carries it.
    let before = app.world().resource::<Seen>().numbers.len();
    act(&mut app, divider, A::Increment, None);
    let numbers = app.world().resource::<Seen>().numbers.clone();
    assert_eq!(numbers.len(), before + 1, "one step, one change");
    assert!(
        numbers[before] > 0.5,
        "Increment widens the first pane: {}",
        numbers[before]
    );
    // A value lands on it, held to the minimums.
    act(
        &mut app,
        divider,
        A::SetValue,
        Some(accesskit::ActionData::NumericValue(0.25)),
    );
    let numbers = app.world().resource::<Seen>().numbers.clone();
    assert!(
        (numbers[numbers.len() - 1] - 0.25).abs() < 0.01,
        "SetValue puts the divider there: {:?}",
        numbers.last()
    );
}

#[test]
fn a_header_cell_is_a_column_header_a_screen_reader_can_sort_with() {
    use accesskit::Action as A;
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let table = app
        .world_mut()
        .spawn((
            table(
                vec![
                    Column::new("Name").width(9.0),
                    Column::new("Qty").width(6.0),
                ],
                vec![
                    vec!["Meadow".to_string(), "12".to_string()],
                    vec!["Harbour".to_string(), "3".to_string()],
                ],
            ),
            ChildOf(root),
        ))
        .id();
    settle(&mut app);
    let header = {
        let mut stack = vec![table];
        let mut found = None;
        while let Some(entity) = stack.pop() {
            if let Some(kids) = app.world().get::<Children>(entity) {
                for &kid in &**kids {
                    if app.world().get::<Role>(kid) == Some(&Role::TableHeaderCell) {
                        found = Some(kid);
                    }
                    stack.push(kid);
                }
            }
        }
        found.expect("a header cell")
    };
    settle(&mut app);
    // It is a column header with its own words, and it takes a Click: before this it was a
    // container in the tree and a screen reader could not sort at all.
    let header_node = node(&app, header);
    assert_eq!(header_node.role(), accesskit::Role::ColumnHeader);
    assert_eq!(header_node.label(), Some("Qty"));
    assert!(header_node.supports_action(A::Click), "it takes a Click");
    // Pressing it sorts by that column, and the node says so.
    act(&mut app, header, A::Click, None);
    assert_eq!(
        app.world().get::<TableSort>(table).map(|sort| sort.column),
        Some(Some(1)),
        "the click sorted by Qty"
    );
    assert_eq!(
        node(&app, header).sort_direction(),
        Some(accesskit::SortDirection::Ascending),
        "and the node reads ascending"
    );
    // A click again turns it round.
    act(&mut app, header, A::Click, None);
    assert_eq!(
        node(&app, header).sort_direction(),
        Some(accesskit::SortDirection::Descending),
        "and the second reads descending"
    );
}

#[test]
fn a_list_is_a_list_whose_rows_say_their_place() {
    let mut app = app();
    let root = app.world_mut().spawn(screen()).id();
    let list = app
        .world_mut()
        .spawn((
            list(
                3.0,
                Children::spawn(SpawnIter(
                    ["Meadow", "Harbour", "Mill", "Cellar"]
                        .into_iter()
                        .map(button),
                )),
            ),
            ChildOf(root),
        ))
        .id();
    settle(&mut app);
    // The viewport is an `Ak::List` that knows how many rows it holds...
    let view = {
        let mut stack = vec![list];
        let mut found = None;
        while let Some(entity) = stack.pop() {
            if app.world().get::<ListView>(entity).is_some() {
                found = Some(entity);
            }
            if let Some(kids) = app.world().get::<Children>(entity) {
                stack.extend(kids.iter());
            }
        }
        found.expect("the list's viewport")
    };
    settle(&mut app);
    let node_ = node(&app, view);
    assert_eq!(node_.role(), accesskit::Role::List);
    assert_eq!(node_.size_of_set(), Some(4), "four rows, spawned or not");
    // ...and each row is a list item with its place among them.
    let rows: Vec<Entity> = {
        // In tree order: a breadth-first walk visits each child list front to back.
        let mut queue = std::collections::VecDeque::from([list]);
        let mut found = Vec::new();
        while let Some(entity) = queue.pop_front() {
            if app.world().get::<Role>(entity) == Some(&Role::Button) {
                found.push(entity);
            }
            if let Some(kids) = app.world().get::<Children>(entity) {
                queue.extend(kids.iter());
            }
        }
        found
    };
    assert_eq!(rows.len(), 4, "four row buttons");
    for (index, row) in rows.iter().enumerate() {
        let n = node(&app, *row);
        assert_eq!(
            n.role(),
            accesskit::Role::ListItem,
            "row {index} is a list item"
        );
        assert_eq!(
            n.position_in_set(),
            Some(index),
            "row {index} says where it is"
        );
        assert_eq!(n.size_of_set(), Some(4));
    }
}
