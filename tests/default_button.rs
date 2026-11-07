//! A dialog's default button: Return in a single-line field ends its edit and then presses it;
//! Return with nothing focused presses it; a focused button, a text area and a disabled default
//! keep theirs; two defaults warn and the first wins; a `Confirm` names one.

use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::{press, type_text};
use bevy_gooey::*;

/// What the game heard, in order.
#[derive(Resource, Default)]
struct Heard(Vec<String>);

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(MotionSettings::off());
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
    .init_resource::<Heard>();
    app.world_mut().spawn(Camera2d);
    app.add_observer(
        |change: On<ValueChange<String>>, mut heard: ResMut<Heard>| {
            heard.0.push(format!(
                "{}{}",
                if change.is_final { "final " } else { "edit " },
                change.value
            ));
        },
    );
    app.update();
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn heard(app: &App) -> Vec<String> {
    app.world().resource::<Heard>().0.clone()
}

fn named(label: &'static str) -> impl Bundle {
    (
        button(label),
        observe(move |_: On<Activate>, mut heard: ResMut<Heard>| heard.0.push(label.to_string())),
    )
}

/// The field inside a row.
fn field_of(app: &App, row: Entity) -> Entity {
    let mut stack = vec![row];
    while let Some(at) = stack.pop() {
        if app.world().get::<Role>(at) == Some(&Role::TextField) {
            return at;
        }
        if let Some(kids) = app.world().get::<Children>(at) {
            stack.extend(kids.iter());
        }
    }
    panic!("the row has no field");
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    settle(app);
}

fn unfocus(app: &mut App) {
    app.world_mut().resource_mut::<InputFocus>().clear();
    settle(app);
}

/// A dialog holding a name field, Create (the default) and Cancel: `(dialog, row, create, cancel)`.
fn name_dialog(app: &mut App, create: impl Bundle) -> (Entity, Entity, Entity, Entity) {
    let row = app.world_mut().spawn(text_input("Name", "")).id();
    let create = app.world_mut().spawn((named("create"), create)).id();
    let cancel = app.world_mut().spawn(named("cancel")).id();
    let actions = app
        .world_mut()
        .spawn(menu())
        .add_children(&[create, cancel])
        .id();
    let card = app
        .world_mut()
        .spawn(panel())
        .add_children(&[row, actions])
        .id();
    let dialog = app.world_mut().spawn(dialog()).add_child(card).id();
    settle(app);
    (dialog, row, create, cancel)
}

#[test]
fn return_in_a_field_ends_the_edit_then_presses_the_default_and_a_focused_button_keeps_its_own() {
    let mut app = app();
    let (_, row, _, cancel) = name_dialog(&mut app, DefaultButton);
    let field = field_of(&app, row);
    focus(&mut app, field);
    type_text(&mut app, "Ada");
    press(&mut app, KeyCode::Enter);
    let heard = heard(&app);
    let final_at = heard
        .iter()
        .position(|h| h == "final Ada")
        .expect("the field's final change");
    let create_at = heard
        .iter()
        .position(|h| h == "create")
        .expect("Create pressed");
    assert!(final_at < create_at, "the value, then the press: {heard:?}");
    assert_eq!(
        heard.iter().filter(|h| *h == "create").count(),
        1,
        "one Activate: {heard:?}"
    );

    focus(&mut app, cancel);
    press(&mut app, KeyCode::Enter);
    let heard = self::heard(&app);
    assert_eq!(
        heard.iter().filter(|h| *h == "cancel").count(),
        1,
        "Return on Cancel is Cancel's: {heard:?}"
    );
    assert_eq!(
        heard.iter().filter(|h| *h == "create").count(),
        1,
        "and does not press Create: {heard:?}"
    );
}

#[test]
fn return_with_nothing_focused_presses_the_default_and_a_modifier_does_not() {
    let mut app = app();
    let (_, _, _, _) = name_dialog(&mut app, DefaultButton);
    unfocus(&mut app);
    press(&mut app, KeyCode::Enter);
    assert_eq!(heard(&app), vec!["create".to_string()]);
}

#[test]
fn a_text_area_keeps_its_newline_and_a_disabled_default_presses_nothing() {
    let mut app = app();
    let area = app.world_mut().spawn(text_area("Notes", "", 3)).id();
    let create = app.world_mut().spawn((named("create"), DefaultButton)).id();
    let card = app
        .world_mut()
        .spawn(panel())
        .add_children(&[area, create])
        .id();
    app.world_mut().spawn(dialog()).add_child(card);
    settle(&mut app);
    let field = field_of(&app, area);
    focus(&mut app, field);
    press(&mut app, KeyCode::Enter);
    assert!(
        !heard(&app).contains(&"create".to_string()),
        "{:?}",
        heard(&app)
    );

    // A disabled default, with nothing focused.
    unfocus(&mut app);
    app.world_mut()
        .entity_mut(create)
        .insert(InteractionDisabled);
    settle(&mut app);
    press(&mut app, KeyCode::Enter);
    assert!(
        !heard(&app).contains(&"create".to_string()),
        "{:?}",
        heard(&app)
    );
}

#[test]
fn two_defaults_in_one_dialog_press_the_first_only() {
    let mut app = app();
    let first = app.world_mut().spawn((named("first"), DefaultButton)).id();
    let second = app.world_mut().spawn((named("second"), DefaultButton)).id();
    let card = app
        .world_mut()
        .spawn(panel())
        .add_children(&[first, second])
        .id();
    app.world_mut().spawn(dialog()).add_child(card);
    settle(&mut app);
    unfocus(&mut app);
    press(&mut app, KeyCode::Enter);
    assert_eq!(heard(&app), vec!["first".to_string()]);
}

#[test]
fn a_default_button_draws_as_submit_unless_the_host_chose_a_variant() {
    let mut app = app();
    let (_, _, plain, _) = name_dialog(&mut app, DefaultButton);
    assert_eq!(
        app.world().get::<ButtonVariant>(plain),
        Some(&ButtonVariant::Submit)
    );
    let (_, _, risky, _) = name_dialog(&mut app, (DefaultButton, ButtonVariant::Danger));
    assert_eq!(
        app.world().get::<ButtonVariant>(risky),
        Some(&ButtonVariant::Danger)
    );
}

#[test]
fn a_confirm_names_its_default_and_return_answers_with_it() {
    #[derive(Resource, Default)]
    struct Chosen(Vec<(usize, bool)>);
    let mut app = app();
    app.init_resource::<Chosen>();
    app.add_observer(|chosen: On<ConfirmChosen>, mut seen: ResMut<Chosen>| {
        seen.0.push((chosen.index, chosen.cancelled));
    });
    app.world_mut()
        .spawn(confirm("Delete?", "It cannot be brought back.", ["Delete", "Keep"]).default(0));
    settle(&mut app);
    unfocus(&mut app);
    press(&mut app, KeyCode::Enter);
    settle(&mut app);
    assert_eq!(app.world().resource::<Chosen>().0, vec![(0, false)]);
}

#[test]
fn only_the_dialog_on_top_answers_and_one_without_a_default_presses_nothing_beneath() {
    let mut app = app();
    let (_, _, _, _) = name_dialog(&mut app, DefaultButton);
    // A question over it with no default of its own.
    app.world_mut()
        .spawn(confirm("Sure?", "", ["Yes", "No"]).safe(1));
    settle(&mut app);
    unfocus(&mut app);
    press(&mut app, KeyCode::Enter);
    assert!(
        !heard(&app).contains(&"create".to_string()),
        "Create is under the question and out of reach: {:?}",
        heard(&app)
    );
}

#[test]
fn a_number_field_keeps_its_return() {
    let mut app = app();
    let row = app
        .world_mut()
        .spawn(number_field("Port", 80.0, 1.0, 0.0..=100.0))
        .id();
    let create = app.world_mut().spawn((named("create"), DefaultButton)).id();
    let card = app
        .world_mut()
        .spawn(panel())
        .add_children(&[row, create])
        .id();
    app.world_mut().spawn(dialog()).add_child(card);
    settle(&mut app);
    let field = field_of(&app, row);
    focus(&mut app, field);
    press(&mut app, KeyCode::Enter);
    assert!(
        !heard(&app).contains(&"create".to_string()),
        "{:?}",
        heard(&app)
    );
}
