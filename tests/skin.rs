//! What a game sees of the skin: colours on widgets, sizes, restyles, checks, meters, tooltips and
//! feedback, in a headless app.

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::ui::Pressed;
use bevy_gooey::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, GooeyStylePlugin));
    app
}

fn child_with_role(app: &mut App, parent: Entity, role: Role) -> Entity {
    let world = app.world_mut();
    let children: Vec<Entity> = world
        .get::<Children>(parent)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .find(|&c| world.get::<Role>(c) == Some(&role))
        .unwrap_or_else(|| panic!("no {role:?} child"))
}

#[test]
fn a_button_takes_its_rest_pressed_and_focus_colours() {
    let mut app = app();
    let skin = GooeySkin::default();
    let button = app.world_mut().spawn(button("Play")).id();
    app.update();
    let world = app.world();
    assert_eq!(
        world.get::<BackgroundColor>(button),
        Some(&BackgroundColor(skin.surface_raised))
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.min_height),
        Some(Val::Px(44.0))
    );

    app.world_mut().entity_mut(button).insert(Pressed);
    app.update();
    let text = child_with_role(&mut app, button, Role::ButtonText);
    let world = app.world();
    assert_eq!(
        world.get::<BackgroundColor>(button),
        Some(&BackgroundColor(skin.accent))
    );
    assert_eq!(
        world.get::<TextColor>(text),
        Some(&TextColor(skin.accent_text))
    );

    app.insert_resource(InputFocus::from_entity(button));
    app.insert_resource(InputFocusVisible(true));
    app.update();
    assert_eq!(
        app.world().get::<BorderColor>(button),
        Some(&BorderColor::all(skin.focus_ring))
    );
}

#[test]
fn a_new_skin_restyles_what_is_already_on_screen() {
    let mut app = app();
    let label = app.world_mut().spawn(label("x")).id();
    app.update();
    app.insert_resource(GooeySkin::parchment());
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(label),
        Some(&TextColor(GooeySkin::parchment().text))
    );
}

#[test]
fn a_panel_in_a_dialog_hides_what_is_behind_it() {
    let mut app = app();
    let hud = app.world_mut().spawn(panel()).id();
    let dialog = app.world_mut().spawn((dialog(), children![panel()])).id();
    app.update();
    let modal = child_with_role(&mut app, dialog, Role::Panel);
    let skin = GooeySkin::default();
    let world = app.world();
    assert_eq!(
        world.get::<BackgroundColor>(hud),
        Some(&BackgroundColor(skin.surface)),
        "a HUD panel keeps the skin's translucency"
    );
    let fill = world.get::<BackgroundColor>(modal).map(|b| b.0.alpha());
    assert_eq!(fill, Some(1.0), "the menu behind must not show through");
    assert_eq!(
        world.get::<BackgroundColor>(modal),
        Some(&BackgroundColor(skin.solid_surface()))
    );
}

#[test]
fn text_scale_enlarges_every_text() {
    let mut app = app();
    app.insert_resource(GooeySkin {
        text_scale: 1.5,
        ..default()
    });
    let label = app.world_mut().spawn(label("Health")).id();
    let heading = app.world_mut().spawn(heading("Paused")).id();
    app.update();
    let world = app.world();
    assert_eq!(
        world.get::<TextFont>(label).map(|f| f.font_size),
        Some(FontSize::Px(30.0))
    );
    assert_eq!(
        world.get::<TextFont>(heading).map(|f| f.font_size),
        Some(FontSize::Px(48.0))
    );
}

#[test]
fn both_presets_pass_their_own_checks() {
    assert_eq!(GooeySkin::dark().problems(), vec![]);
    assert_eq!(GooeySkin::parchment().problems(), vec![]);
}

#[test]
fn a_skin_that_breaks_the_rules_says_which() {
    let skin = GooeySkin::default();
    let problems = GooeySkin {
        text: skin.surface_raised,
        min_target: 12.0,
        text_scale: 0.0,
        ..default()
    }
    .problems();
    let low = problems
        .iter()
        .find(|p| {
            matches!(
                p,
                SkinProblem::LowContrast {
                    pair: "text on surface_raised",
                    ..
                }
            )
        })
        .expect("text drawn in its own background colour is low contrast");
    assert!(problems.contains(&SkinProblem::TargetTooSmall { px: 12.0 }));
    assert!(problems.contains(&SkinProblem::BadSize {
        field: "text_scale"
    }));
    assert!(
        low.to_string()
            .starts_with("text on surface_raised has contrast "),
        "{low}"
    );
}

#[test]
fn a_meter_fills_by_fraction() {
    let mut app = app();
    let meter = app.world_mut().spawn(meter(25.0, 100.0)).id();
    app.update();
    let fill = child_with_role(&mut app, meter, Role::MeterFill);
    assert_eq!(
        app.world().get::<Node>(fill).map(|n| n.width),
        Some(Val::Percent(25.0))
    );

    if let Some(mut m) = app.world_mut().get_mut::<Meter>(meter) {
        m.value = 150.0;
    }
    app.update();
    assert_eq!(
        app.world().get::<Node>(fill).map(|n| n.width),
        Some(Val::Percent(100.0))
    );
}

#[test]
fn a_tooltip_starts_hidden() {
    let mut app = app();
    let help = app
        .world_mut()
        .spawn((button("Help"), Tooltip("Opens the manual".into())))
        .id();
    app.update();
    let tips: Vec<Entity> = app
        .world_mut()
        .query::<(Entity, &Role, &ChildOf)>()
        .iter(app.world())
        .filter(|(_, role, parent)| **role == Role::Tooltip && parent.parent() == help)
        .map(|(e, ..)| e)
        .collect();
    assert_eq!(tips.len(), 1);
    let tip = tips[0];
    assert_eq!(
        app.world().get::<Node>(tip).map(|n| n.display),
        Some(Display::None)
    );
    let text = child_with_role(&mut app, tip, Role::Label);
    assert_eq!(
        app.world().get::<Text>(text).map(|t| t.0.as_str()),
        Some("Opens the manual")
    );
}

#[test]
fn activating_a_button_reports_feedback() {
    let mut app = app();
    let go = app.world_mut().spawn(button("Go")).id();
    app.world_mut().trigger(Activate { entity: go });
    let plain = app.world_mut().spawn(Node::default()).id();
    app.world_mut().trigger(Activate { entity: plain });
    app.update();
    let messages = app.world().resource::<Messages<UiFeedback>>();
    let read: Vec<UiFeedback> = messages.get_cursor().read(messages).copied().collect();
    assert_eq!(
        read,
        vec![UiFeedback {
            entity: go,
            kind: UiFeedbackKind::Activate
        }]
    );
}
