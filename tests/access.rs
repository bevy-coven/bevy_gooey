//! The accessibility tree: what a screen reader is told of every widget, in the same headless window.
#![cfg(feature = "testing")]

use accesskit::{Role as Ak, Toggled};
use bevy::a11y::{AccessibilityNode, AccessibilityRequested};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::InteractionDisabled;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::{click, find_by_label};
use bevy_gooey::*;

/// How many accessibility nodes changed in the last frame.
#[derive(Resource, Default)]
struct NodesChanged(usize);

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
