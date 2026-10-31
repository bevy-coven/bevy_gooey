//! A panel over a dialog's scrim is the dialog: it has a measure, room between its title, its body
//! and its choices, an edge in the outline colour, and it stays out of the window's outer 5%. A
//! panel anywhere else is a screen's, unchanged. Laid out for real in a headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// What a restyle writes.
type Written = Or<(
    Changed<Node>,
    Changed<BackgroundColor>,
    Changed<BorderColor>,
)>;

/// What the dialog's restyle writes, counted while it idles.
#[derive(Resource, Default)]
struct Writes(u32);

/// A headless 1280 × 720 window, so UI layout runs; the window's scale factor and `UiScale` are 1.
fn app_with(skin: GooeySkin) -> App {
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
    .init_resource::<Writes>()
    .add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    )
    .insert_resource(skin);
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// Where `entity` is drawn, in logical px.
fn rect(app: &App, entity: Entity) -> Rect {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    let scale = node.inverse_scale_factor();
    Rect::from_center_size(at * scale, node.size() * scale)
}

#[derive(Component)]
struct Title;
#[derive(Component)]
struct Body;
#[derive(Component)]
struct Choices;
#[derive(Component)]
struct Leave;
#[derive(Component)]
struct ScreenPanel;

#[derive(Component)]
struct DialogPanel;
struct Ui {
    panel: Entity,
    title: Entity,
    body: Entity,
    choices: Entity,
    leave: Entity,
}

/// The dialog a game writes: a title, a line of body and two choices in a panel.
fn confirm(app: &mut App, body: &str) -> Ui {
    let world = app.world_mut();
    world.spawn((
        dialog(),
        children![(
            (panel(), DialogPanel),
            children![
                (heading("Leave the game?"), Title),
                (label(body), Body),
                (
                    menu(),
                    Choices,
                    children![(button("Leave"), Leave), button("Stay")]
                ),
            ]
        )],
    ));
    fn tagged<T: Component>(world: &mut World) -> Entity {
        world
            .query_filtered::<Entity, With<T>>()
            .single(world)
            .expect("one tagged widget")
    }
    Ui {
        panel: tagged::<DialogPanel>(world),
        title: tagged::<Title>(world),
        body: tagged::<Body>(world),
        choices: tagged::<Choices>(world),
        leave: tagged::<Leave>(world),
    }
}

const LONG: &str = "Everything since the last autosave is lost: the map you have opened, the items you carry, and the three letters you have not yet read.";

fn near(got: f32, want: f32, what: &str) {
    assert!((got - want).abs() <= 1.5, "{what}: {got}, wanted {want}");
}

#[test]
fn a_short_dialog_is_21_text_sizes_wide_with_its_words_in_order_and_its_choices_as_wide_as_it() {
    let mut app = app_with(GooeySkin::default());
    let ui = confirm(&mut app, "Sure?");
    settle(&mut app);
    let skin = GooeySkin::default();
    let (panel, title, body, choices, leave) = (
        rect(&app, ui.panel),
        rect(&app, ui.title),
        rect(&app, ui.body),
        rect(&app, ui.choices),
        rect(&app, ui.leave),
    );
    let measure = skin.text_size * 21.0;
    near(
        panel.width(),
        measure,
        "a short dialog fills its least measure",
    );
    near((panel.min.x + panel.max.x) / 2.0, 640.0, "centred across");
    near((panel.min.y + panel.max.y) / 2.0, 360.0, "centred down");
    // Padding one and a half screen paddings and one line of frame, on every side.
    let room = skin.padding * 1.5 + skin.border;
    near(title.min.x - panel.min.x, room, "room left of the title");
    near(title.min.y - panel.min.y, room, "room above the title");
    near(panel.max.y - choices.max.y, room, "room under the choices");
    near(
        leave.width(),
        measure - 2.0 * room,
        "a choice fills the panel's inside",
    );
    // Title, body and choices a gap and a half apart, in that order.
    near(body.min.y - title.max.y, skin.gap * 1.5, "title to body");
    near(
        choices.min.y - body.max.y,
        skin.gap * 1.5,
        "body to choices",
    );
}

#[test]
fn a_long_body_wraps_at_36_text_sizes_and_nothing_leaves_the_panel() {
    let mut app = app_with(GooeySkin::default());
    let ui = confirm(&mut app, LONG);
    settle(&mut app);
    let skin = GooeySkin::default();
    let (panel, body, leave) = (
        rect(&app, ui.panel),
        rect(&app, ui.body),
        rect(&app, ui.leave),
    );
    near(panel.width(), skin.text_size * 36.0, "the greatest measure");
    let room = skin.padding * 1.5 + skin.border;
    assert!(
        body.min.x >= panel.min.x + room - 1.5 && body.max.x <= panel.max.x - room + 1.5,
        "the body {body:?} is inside the panel {panel:?}"
    );
    assert!(
        body.height() > skin.text_size * 2.0,
        "{LONG:?} wraps onto more than one line: {} high",
        body.height()
    );
    assert!(
        leave.min.x >= panel.min.x && leave.max.x <= panel.max.x,
        "a choice stays in the panel"
    );
}

#[test]
fn the_measure_grows_with_the_players_text_size_and_a_dialog_keeps_out_of_the_outer_five_percent() {
    let big = GooeySkin {
        text_scale: 1.5,
        ..GooeySkin::default()
    };
    let mut app = app_with(big.clone());
    let ui = confirm(&mut app, "Sure?");
    settle(&mut app);
    near(
        rect(&app, ui.panel).width(),
        big.text_size * big.text_scale * 21.0,
        "21 text sizes of the player's size",
    );

    // At twice the size the greatest measure, 1440 px, is wider than the window less its 5% edges.
    let huge = GooeySkin {
        text_scale: 2.0,
        ..GooeySkin::default()
    };
    let mut app = app_with(huge);
    let ui = confirm(&mut app, LONG);
    settle(&mut app);
    let panel = rect(&app, ui.panel);
    near(panel.min.x, 64.0, "5% of 1280 from the left");
    near(panel.max.x, 1216.0, "5% of 1280 from the right");
    assert!(
        rect(&app, ui.leave).max.x <= panel.max.x,
        "a choice stays in the squeezed panel"
    );
}

#[test]
fn a_dialog_panel_is_framed_in_the_outline_and_a_screens_panel_is_not() {
    for skin in [GooeySkin::dark(), GooeySkin::parchment()] {
        let mut app = app_with(skin.clone());
        let ui = confirm(&mut app, "Sure?");
        let on_screen = app.world_mut().spawn((panel(), ScreenPanel)).id();
        app.world_mut().spawn(screen()).add_child(on_screen);
        settle(&mut app);

        let edge = *app.world().get::<BorderColor>(ui.panel).expect("a border");
        let fill = app
            .world()
            .get::<BackgroundColor>(ui.panel)
            .expect("a fill")
            .0;
        assert_eq!(edge, BorderColor::all(skin.outline), "the dialog's edge");
        assert_eq!(fill, skin.solid_surface(), "the dialog's fill is opaque");

        let node = app.world().get::<Node>(ui.panel).expect("a node");
        assert_eq!(node.padding, UiRect::all(Val::Px(skin.padding * 1.5)));
        assert_eq!(node.row_gap, Val::Px(skin.gap * 1.5));

        // A screen's panel is as it was: one padding and gap, no measure, the raised edge.
        let edge = *app.world().get::<BorderColor>(on_screen).expect("a border");
        assert_eq!(edge, BorderColor::all(skin.surface_raised));
        let fill = app
            .world()
            .get::<BackgroundColor>(on_screen)
            .expect("a fill")
            .0;
        assert_eq!(fill, skin.surface);
        let node = app.world().get::<Node>(on_screen).expect("a node");
        assert_eq!(node.padding, UiRect::all(Val::Px(skin.padding)));
        assert_eq!(node.row_gap, Val::Px(skin.gap));
        assert_eq!((node.min_width, node.max_width), (Val::Auto, Val::Auto));
    }
}

#[test]
fn the_dialog_edge_stands_out_from_its_panel_and_from_the_screen_the_scrim_dims() {
    let rgb = |c: Color| {
        let c = Srgba::from(c);
        [c.red, c.green, c.blue]
    };
    for (name, skin) in [
        ("dark", GooeySkin::dark()),
        ("parchment", GooeySkin::parchment()),
    ] {
        let edge = rgb(skin.outline);
        let scrim = Srgba::from(skin.scrim);
        let dimmed = composite(rgb(skin.scrim), scrim.alpha, rgb(skin.background));
        for (against, what) in [
            (rgb(skin.solid_surface()), "its own panel"),
            (dimmed, "the window the scrim dims"),
        ] {
            let ratio = contrast_ratio(edge, against);
            assert!(
                ratio >= NON_TEXT_CONTRAST,
                "{name}: the dialog's edge is {ratio:.2}:1 against {what}, needs {NON_TEXT_CONTRAST}"
            );
        }
    }
}

#[test]
fn a_dialog_at_rest_writes_nothing() {
    let mut app = app_with(GooeySkin::default());
    confirm(&mut app, LONG);
    settle(&mut app);
    let before = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<Writes>().0,
        before,
        "thirty idle frames restyled nothing"
    );
}
