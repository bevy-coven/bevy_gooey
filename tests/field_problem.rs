//! Field validation messages: what is wrong with a text row, said under it in words, with the
//! field's frame and a bar in the problem's colour. Laid out for real in a headless window.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::model::contrast_ratio;
use bevy_gooey::*;

/// A headless app with a real window size, so UI layout runs.
fn app(skin: GooeySkin) -> App {
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
    .insert_resource(skin);
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// Three rows in a panel: a name, a count (a number field) and a tint (a colour field), and a Back
/// button under them.
struct Form {
    name: Entity,
    count: Entity,
    tint: Entity,
    back: Entity,
}

fn form(app: &mut App) -> Form {
    let world = app.world_mut();
    let name = world.spawn(text_input("Name", "")).id();
    let count = world
        .spawn(number_field("Count", 3.0, 1.0, 0.0..=10.0))
        .id();
    let tint = world
        .spawn(color_field("Tint", Srgba::rgb(1.0, 0.5, 0.0)))
        .id();
    let back = world.spawn(button("Back")).id();
    let card = world
        .spawn(panel())
        .add_children(&[name, count, tint, back])
        .id();
    world.spawn(screen()).add_child(card);
    settle(app);
    Form {
        name,
        count,
        tint,
        back,
    }
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            for child in children.iter() {
                found.push(child);
                stack.push(child);
            }
        }
    }
    found
}

fn parts(app: &App, root: Entity, role: Role) -> Vec<Entity> {
    descendants(app.world(), root)
        .into_iter()
        .filter(|&e| app.world().get::<Role>(e) == Some(&role))
        .collect()
}

/// The one part of `root` with `role`, or none.
fn part(app: &App, root: Entity, role: Role) -> Option<Entity> {
    let found = parts(app, root, role);
    assert!(found.len() <= 1, "{} {role:?} under one row", found.len());
    found.first().copied()
}

/// Where `entity` is drawn, in logical px (the window's scale factor is 1).
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

fn words(app: &App, row: Entity) -> Option<String> {
    let message = part(app, row, Role::FieldMessage)?;
    descendants(app.world(), message)
        .into_iter()
        .find_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

fn edge(app: &App, row: Entity) -> Color {
    let field = part(app, row, Role::TextField).expect("a field");
    app.world().get::<BorderColor>(field).expect("a frame").top
}

#[test]
fn inserting_a_problem_draws_its_words_under_the_row_and_removing_it_removes_them_the_same_frame() {
    let mut app = app(GooeySkin::dark());
    let form = form(&mut app);
    for row in [form.name, form.count, form.tint] {
        assert_eq!(words(&app, row), None);
        app.world_mut()
            .entity_mut(row)
            .insert(FieldProblem::error("Use 3 to 20 letters"));
        app.update();
        assert_eq!(words(&app, row).as_deref(), Some("Use 3 to 20 letters"));
        // The same flush as the removal: no frame passes with the words still up.
        app.world_mut().entity_mut(row).remove::<FieldProblem>();
        assert_eq!(words(&app, row), None, "before the next frame");
        app.update();
        assert_eq!(words(&app, row), None);
    }
}

#[test]
fn the_message_is_wholly_under_the_row_and_the_next_row_moves_down_by_its_height_and_back() {
    let mut app = app(GooeySkin::dark());
    let form = form(&mut app);
    let row = rect(&app, form.name);
    let below = rect(&app, form.count);
    app.world_mut()
        .entity_mut(form.name)
        .insert(FieldProblem::error("A name is needed"));
    settle(&mut app);
    let message = rect(
        &app,
        part(&app, form.name, Role::FieldMessage).expect("a message"),
    );
    let row_now = rect(&app, form.name);
    let below_now = rect(&app, form.count);
    assert!(
        (row_now.size() - row.size()).length() < 1.5,
        "the row keeps its size {row:?} {row_now:?}"
    );
    assert!(
        message.min.y >= row_now.max.y - 0.5,
        "{message:?} under {row_now:?}"
    );
    assert!(message.max.y <= 720.0 && message.min.x >= row_now.min.x - 0.5);
    assert!(message.max.x <= row_now.max.x + 0.5);
    // Nothing below sits on it, and the rows below moved by the room the message takes.
    assert!(
        below_now.min.y >= message.max.y - 0.5,
        "{below_now:?} under {message:?}"
    );
    let moved = (below_now.min.y - row_now.min.y) - (below.min.y - row.min.y);
    assert!(moved > message.height() * 0.9, "{moved} {message:?}");
    app.world_mut()
        .entity_mut(form.name)
        .remove::<FieldProblem>();
    settle(&mut app);
    let back = rect(&app, form.count).min.y - rect(&app, form.name).min.y;
    assert!((back - (below.min.y - row.min.y)).abs() < 0.5);
}

#[test]
fn an_error_frames_the_field_and_both_kinds_colour_the_bar_and_the_ring_wins_while_it_has_focus() {
    let skin = GooeySkin::dark();
    let mut app = app(skin.clone());
    let form = form(&mut app);
    let quiet = edge(&app, form.name);
    app.world_mut()
        .entity_mut(form.name)
        .insert(FieldProblem::error("No"));
    settle(&mut app);
    assert_eq!(edge(&app, form.name), skin.danger);
    let bar = part(&app, form.name, Role::FieldMark).expect("a bar");
    assert_eq!(
        app.world().get::<BackgroundColor>(bar).expect("a fill").0,
        skin.danger
    );
    // The same row, a warning now: the message is edited in place, one message still.
    let message = part(&app, form.name, Role::FieldMessage).expect("a message");
    app.world_mut()
        .entity_mut(form.name)
        .insert(FieldProblem::warning("Short"));
    settle(&mut app);
    assert_eq!(part(&app, form.name, Role::FieldMessage), Some(message));
    assert_eq!(words(&app, form.name).as_deref(), Some("Short"));
    // A warning does not block: the bar and the words say it and the frame stays the field's own
    // (the accent is the focus ring's blue in the Tool look and the danger's red in parchment).
    assert_eq!(edge(&app, form.name), quiet);
    assert_eq!(
        app.world().get::<BackgroundColor>(bar).expect("a fill").0,
        skin.accent
    );
    // The ring is the ring while the player is in the field.
    let field = part(&app, form.name, Role::TextField).expect("a field");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(field, FocusCause::Navigated);
    settle(&mut app);
    assert_eq!(edge(&app, form.name), skin.focus_ring);
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.world_mut()
        .entity_mut(form.name)
        .remove::<FieldProblem>();
    settle(&mut app);
    assert_eq!(edge(&app, form.name), quiet);
}

#[test]
fn the_words_read_at_text_contrast_and_the_bar_and_frame_at_non_text_contrast_in_every_look() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let panel = rgb(skin.solid_surface());
        let mut app = app(skin.clone());
        let form = form(&mut app);
        for kind in [FieldProblemKind::Error, FieldProblemKind::Warning] {
            app.world_mut()
                .entity_mut(form.name)
                .insert(FieldProblem::new(kind, "Wrong"));
            settle(&mut app);
            let message = part(&app, form.name, Role::FieldMessage).expect("a message");
            let text = descendants(app.world(), message)
                .into_iter()
                .find(|e| app.world().get::<Text>(*e).is_some())
                .expect("words");
            let ink = app.world().get::<TextColor>(text).expect("an ink").0;
            let words = contrast_ratio(rgb(ink), panel);
            assert!(words >= 4.5, "{} {kind:?}: words {words}", preset.name());
            let bar = part(&app, form.name, Role::FieldMark).expect("a bar");
            let fill = app.world().get::<BackgroundColor>(bar).expect("a fill").0;
            let bar_contrast = contrast_ratio(rgb(fill), panel);
            assert!(
                bar_contrast >= 3.0,
                "{} {kind:?}: bar {bar_contrast}",
                preset.name()
            );
            let frame = contrast_ratio(rgb(edge(&app, form.name)), panel);
            assert!(frame >= 3.0, "{} {kind:?}: frame {frame}", preset.name());
            if kind == FieldProblemKind::Error {
                assert_eq!(edge(&app, form.name), skin.danger, "{}", preset.name());
            }
        }
    }
}

#[test]
fn a_long_message_wraps_inside_the_row_at_200_percent_text_and_clears_the_row_below() {
    let skin = GooeySkin {
        text_scale: 2.0,
        ..GooeySkin::dark()
    };
    let mut app = app(skin);
    let form = form(&mut app);
    app.world_mut().entity_mut(form.name).insert(FieldProblem::error(
        "Names are three to twenty letters, digits or dashes, and a name another player has is taken",
    ));
    settle(&mut app);
    let row = rect(&app, form.name);
    let message = rect(
        &app,
        part(&app, form.name, Role::FieldMessage).expect("a message"),
    );
    assert!(message.max.x <= row.max.x + 0.5, "{message:?} in {row:?}");
    // Two lines at least: taller than one line of the text.
    assert!(message.height() > 2.0 * 14.0 * 2.0 * 1.0, "{message:?}");
    assert!(rect(&app, form.count).min.y >= message.max.y - 0.5);
    assert!(rect(&app, form.back).max.y <= 720.0 + 0.5 || message.max.y < 720.0);
}

#[test]
fn a_message_takes_no_input_and_no_focus() {
    let mut app = app(GooeySkin::dark());
    let form = form(&mut app);
    app.world_mut()
        .entity_mut(form.name)
        .insert(FieldProblem::error("No"));
    settle(&mut app);
    for e in descendants(
        app.world(),
        part(&app, form.name, Role::FieldMessage).expect("a message"),
    ) {
        assert_eq!(
            app.world().get::<Pickable>(e),
            Some(&Pickable::IGNORE),
            "{e}"
        );
        assert!(
            app.world()
                .get::<bevy::input_focus::tab_navigation::TabIndex>(e)
                .is_none()
        );
    }
}

#[test]
fn thirty_idle_frames_write_nothing_with_a_problem_up() {
    let mut app = app(GooeySkin::dark());
    let form = form(&mut app);
    app.world_mut()
        .entity_mut(form.tint)
        .insert(FieldProblem::warning("Very dark"));
    settle(&mut app);
    let mut writes = 0;
    for _ in 0..30 {
        app.update();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, Or<(
            Changed<Node>,
            Changed<BackgroundColor>,
            Changed<BorderColor>,
            Changed<Text>,
            Changed<FieldProblem>,
        )>>();
        writes += query.iter(world).count();
    }
    assert_eq!(writes, 0);
}
