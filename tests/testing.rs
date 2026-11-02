//! The headless test helpers (`bevy_gooey::testing`): a gooey UI driven by what the player reads.
#![cfg(feature = "testing")]

use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::render_resource::TextureFormat;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{IsDefaultUiCamera, UiTargetCamera};
use bevy::window::ExitCondition;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::{
    FindError, click, find_by_label, focused, focused_label, label_of, press, type_text,
};
use bevy_gooey::*;

/// How many times each button was activated, by its label.
#[derive(Resource, Default)]
struct Activated(Vec<String>);

fn app_scaled(scale: f32) -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1280, 720).with_scale_factor_override(scale),
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
    .init_resource::<Activated>()
    .add_observer(record);
    app.world_mut().spawn(Camera2d);
    app
}

/// Notes a button's activation by its label.
fn record(
    activate: On<Activate>,
    texts: Query<&Text>,
    kids: Query<&Children>,
    mut seen: ResMut<Activated>,
) {
    let words = kids
        .get(activate.entity)
        .ok()
        .and_then(|c| c.iter().find_map(|k| texts.get(k).ok()))
        .map(|t| t.0.clone())
        .unwrap_or_default();
    seen.0.push(words);
}

fn app() -> App {
    app_scaled(1.0)
}

/// A main menu as a game builds it: a heading, three buttons, a toggle and a name field.
fn menu_screen(app: &mut App) {
    app.world_mut().spawn((
        screen(),
        children![(
            panel(),
            children![
                heading("Gallery"),
                button("Play"),
                button("Options"),
                button("Quit"),
                toggle("Music", true),
                text_input("Name", ""),
            ],
        )],
    ));
    for _ in 0..4 {
        app.update();
    }
}

#[test]
fn the_acceptance_clicking_options_by_label_and_reading_the_focused_label() {
    let mut app = app();
    menu_screen(&mut app);
    let options = find_by_label(&mut app, "Options").expect("one Options button");
    assert_eq!(focused(&app).filter(|e| *e == options), None);
    click(&mut app, options);
    assert_eq!(focused(&app), Some(options));
    assert_eq!(focused_label(&app).as_deref(), Some("Options"));
    assert_eq!(app.world().resource::<Activated>().0, ["Options"]);
}

#[test]
fn the_click_lands_where_the_widget_is_drawn_at_any_scale() {
    for scale in [1.0, 1.5, 2.0] {
        let mut app = app_scaled(scale);
        menu_screen(&mut app);
        for label in ["Play", "Quit"] {
            let button = find_by_label(&mut app, label).expect("a button");
            click(&mut app, button);
            assert_eq!(focused_label(&app).as_deref(), Some(label), "scale {scale}");
        }
        assert_eq!(app.world().resource::<Activated>().0, ["Play", "Quit"]);
    }
}

#[test]
fn a_button_covered_by_another_node_is_not_clicked() {
    let mut app = app();
    menu_screen(&mut app);
    let play = find_by_label(&mut app, "Play").expect("a button");
    // A full-window node that takes the pointer, as an open dialog's scrim would.
    app.world_mut().spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Pickable::default(),
        GlobalZIndex(500),
    ));
    app.update();
    click(&mut app, play);
    assert!(
        app.world().resource::<Activated>().0.is_empty(),
        "the player could not press it, so the test cannot either"
    );
}

#[test]
fn two_widgets_with_one_label_are_an_error_naming_both() {
    let mut app = app();
    app.world_mut().spawn((
        screen(),
        children![(
            panel(),
            children![
                (button("Back"), Name::new("first Back")),
                (button("Back"), Name::new("second Back")),
                button("Next"),
            ],
        )],
    ));
    for _ in 0..4 {
        app.update();
    }
    let error = find_by_label(&mut app, "Back").expect_err("two Backs");
    let FindError::Ambiguous { label, matches } = &error else {
        panic!("expected Ambiguous, got {error:?}");
    };
    assert_eq!(label, "Back");
    let mut names: Vec<&str> = matches.iter().map(|(_, what)| what.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["first Back", "second Back"]);
    let message = error.to_string();
    assert!(
        message.contains("first Back") && message.contains("second Back"),
        "{message}"
    );
    assert!(find_by_label(&mut app, "Next").is_ok());
}

#[test]
fn a_label_nobody_reads_is_not_found_and_neither_is_a_hidden_one() {
    let mut app = app();
    menu_screen(&mut app);
    app.world_mut().spawn((
        screen(),
        Visibility::Hidden,
        children![(panel(), children![button("Secret page")])],
    ));
    app.world_mut().spawn((
        screen(),
        children![(
            Node {
                display: Display::None,
                ..default()
            },
            Role::Panel,
            children![button("Folded away")],
        )],
    ));
    for _ in 0..4 {
        app.update();
    }
    for label in ["Nowhere", "Secret page", "Folded away"] {
        assert_eq!(
            find_by_label(&mut app, label),
            Err(FindError::NotFound {
                label: label.to_owned()
            })
        );
    }
}

#[test]
fn a_heading_is_found_as_itself_and_a_button_as_the_button_not_its_text() {
    let mut app = app();
    menu_screen(&mut app);
    let heading = find_by_label(&mut app, "Gallery").expect("a heading");
    assert!(app.world().get::<Text>(heading).is_some());
    let play = find_by_label(&mut app, "Play").expect("a button");
    assert!(
        app.world().get::<Text>(play).is_none(),
        "the button, not its text"
    );
    assert_eq!(label_of(&app, play).as_deref(), Some("Play"));
}

#[test]
fn enter_activates_the_focused_button_and_space_flips_a_toggle() {
    let mut app = app();
    menu_screen(&mut app);
    let play = find_by_label(&mut app, "Play").expect("a button");
    click(&mut app, play);
    app.world_mut().resource_mut::<Activated>().0.clear();
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.world().resource::<Activated>().0, ["Play"]);

    let music = find_by_label(&mut app, "Music").expect("a toggle");
    click(&mut app, music);
    assert!(
        app.world().get::<Checked>(music).is_none(),
        "the click flipped it off"
    );
    press(&mut app, KeyCode::Space);
    assert!(
        app.world().get::<Checked>(music).is_some(),
        "space flipped it on"
    );
}

#[test]
fn text_typed_goes_into_the_field_the_label_focused() {
    let mut app = app();
    menu_screen(&mut app);
    let name = find_by_label(&mut app, "Name").expect("the row's label");
    click(&mut app, name);
    // Focus is on the field, which reads as its row.
    assert_eq!(focused_label(&app).as_deref(), Some("Name"));
    assert_ne!(focused(&app), Some(name));
    type_text(&mut app, "Ada Lovelace");
    let row = app
        .world_mut()
        .query::<(Entity, &TextInputValue)>()
        .iter(app.world())
        .map(|(_, v)| v.0.clone())
        .next()
        .expect("one text row");
    assert_eq!(row, "Ada Lovelace");
}

#[test]
fn nothing_focused_has_no_label() {
    let mut app = app();
    menu_screen(&mut app);
    assert_eq!(focused_label(&app), None);
}

// ---- An interface drawn into an image, in an app with no window -----------------------------

/// A headless app with no `PrimaryWindow`, as a tool that shows its interface as a texture has.
fn windowless_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
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
    .init_resource::<Activated>()
    .add_observer(record);
    app
}

/// An image a camera can render to, `width` by `height` pixels.
fn target_image(app: &mut App, width: u32, height: u32) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            width,
            height,
            TextureFormat::Bgra8UnormSrgb,
            None,
        ))
}

/// A camera that draws the interface into `image`, with the target's own scale factor.
fn camera_into(app: &mut App, image: Handle<Image>, scale_factor: f32) -> Entity {
    app.world_mut()
        .spawn((
            Camera2d,
            RenderTarget::Image(ImageRenderTarget {
                handle: image,
                scale_factor,
            }),
        ))
        .id()
}

/// The one camera `screen()` roots are drawn by when they name none: Bevy picks a camera that
/// renders to the primary window, so one that renders to an image is marked.
fn default_camera_into(app: &mut App, image: Handle<Image>, scale_factor: f32) -> Entity {
    let camera = camera_into(app, image, scale_factor);
    app.world_mut().entity_mut(camera).insert(IsDefaultUiCamera);
    camera
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

#[test]
fn the_acceptance_a_ui_drawn_into_an_image_is_clicked_with_no_window() {
    let mut app = windowless_app();
    let image = target_image(&mut app, 800, 600);
    default_camera_into(&mut app, image, 1.0);
    app.world_mut()
        .spawn((screen(), children![(panel(), children![button("Play")])]));
    settle(&mut app);
    let play = find_by_label(&mut app, "Play").expect("one Play button");

    click(&mut app, play);

    assert_eq!(app.world().resource::<Activated>().0, ["Play"]);
    assert_eq!(focused(&app), Some(play));
}

#[test]
fn a_node_that_covers_the_button_in_an_image_takes_the_click() {
    let mut app = windowless_app();
    let image = target_image(&mut app, 800, 600);
    default_camera_into(&mut app, image, 1.0);
    app.world_mut()
        .spawn((screen(), children![(panel(), children![button("Play")])]));
    settle(&mut app);
    let play = find_by_label(&mut app, "Play").expect("one Play button");
    // A later root drawn over everything, with the default `Pickable`, as a modal veil is.
    app.world_mut().spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        GlobalZIndex(10),
    ));
    settle(&mut app);

    click(&mut app, play);

    assert!(app.world().resource::<Activated>().0.is_empty());
    assert_ne!(focused(&app), Some(play));
}

#[test]
fn each_widget_is_clicked_on_the_camera_that_draws_it() {
    let mut app = windowless_app();
    let first = target_image(&mut app, 640, 480);
    let second = target_image(&mut app, 400, 300);
    let first_camera = camera_into(&mut app, first, 1.0);
    let second_camera = camera_into(&mut app, second, 1.0);
    for (camera, name) in [(first_camera, "Left"), (second_camera, "Right")] {
        app.world_mut().spawn((
            screen(),
            UiTargetCamera(camera),
            children![(panel(), children![button(name)])],
        ));
    }
    settle(&mut app);

    for name in ["Left", "Right"] {
        let entity = find_by_label(&mut app, name).expect("a button");
        click(&mut app, entity);
        assert_eq!(focused_label(&app).as_deref(), Some(name));
    }

    assert_eq!(app.world().resource::<Activated>().0, ["Left", "Right"]);
}

#[test]
fn an_images_own_scale_factor_is_the_clicks() {
    let mut app = windowless_app();
    let image = target_image(&mut app, 1600, 1200);
    default_camera_into(&mut app, image, 2.0);
    app.world_mut()
        .spawn((screen(), children![(panel(), children![button("Play")])]));
    settle(&mut app);
    let play = find_by_label(&mut app, "Play").expect("one Play button");

    click(&mut app, play);

    assert_eq!(app.world().resource::<Activated>().0, ["Play"]);
}

#[test]
fn keys_need_no_window_either() {
    let mut app = windowless_app();
    let image = target_image(&mut app, 800, 600);
    default_camera_into(&mut app, image, 1.0);
    app.world_mut().spawn((
        screen(),
        children![(panel(), children![button("Play"), button("Quit")])],
    ));
    settle(&mut app);
    let play = find_by_label(&mut app, "Play").expect("one Play button");
    click(&mut app, play);

    press(&mut app, KeyCode::ArrowDown);

    assert_eq!(focused_label(&app).as_deref(), Some("Quit"));
}

#[test]
#[should_panic(expected = "drawn by no camera")]
fn a_widget_no_camera_draws_says_so() {
    let mut app = windowless_app();
    let lone = app.world_mut().spawn(button("Play")).id();
    settle(&mut app);
    // Laid out by hand: nothing draws it, so nothing can say where it is.
    app.world_mut()
        .entity_mut(lone)
        .insert((ComputedNode::default(), UiGlobalTransform::default()));

    click(&mut app, lone);
}
