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
fn the_presets_pass_their_own_checks() {
    assert_eq!(GooeySkin::dark().problems(), vec![]);
    assert_eq!(GooeySkin::parchment().problems(), vec![]);
    assert_eq!(GooeySkin::high_contrast().problems(), vec![]);
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
    // The fraction and its clamp, not the motion toward it (`tests/meter_motion.rs`).
    app.insert_resource(MotionSettings::off());
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
fn a_meter_the_panel_or_its_frame_would_hide_is_reported() {
    let skin = GooeySkin::default();
    let problems = GooeySkin {
        accent: skin.surface_raised,
        accent_text: Color::WHITE,
        outline: skin.solid_surface(),
        border: 6.0,
        ..default()
    }
    .problems();
    for pair in ["accent on surface_raised", "outline on surface"] {
        assert!(
            problems
                .iter()
                .any(|p| matches!(p, SkinProblem::LowContrast { pair: q, .. } if *q == pair)),
            "{pair} not reported: {problems:?}"
        );
    }
    // 24 px of meter, all of it frame and gap: no fill would show.
    assert!(problems.contains(&SkinProblem::MeterTooThin {
        px: skin.meter_height,
        needed: 24.0
    }));
}

#[test]
fn a_tooltip_starts_hidden() {
    let mut app = app();
    let help = app
        .world_mut()
        .spawn((button("Help"), Tooltip::new("Opens the manual")))
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

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

#[test]
fn high_contrast_holds_every_pair_to_seven_to_one_and_the_other_presets_do_not() {
    let high = GooeySkin::high_contrast();
    assert_eq!(high.problems(), vec![]);
    assert_eq!(high.high_contrast_problems(), vec![]);
    // The bar is XAG 102's, and it can fail: the other two presets meet the minimums and not it.
    for (name, skin) in [
        ("dark", GooeySkin::dark()),
        ("parchment", GooeySkin::parchment()),
    ] {
        assert_eq!(skin.problems(), vec![], "{name} holds the minimums");
        let weak = skin.high_contrast_problems();
        assert!(!weak.is_empty(), "{name} is not a high-contrast skin");
        for problem in &weak {
            assert!(
                matches!(problem, SkinProblem::LowContrast { ratio, needed, .. }
                    if *needed == ENHANCED_CONTRAST && *ratio < ENHANCED_CONTRAST),
                "{name}: {problem}"
            );
        }
    }
    // An edge a skin draws is checked too, and the quiet default is not.
    let faint = GooeySkin {
        surface_edge: Some(high.surface),
        ..GooeySkin::high_contrast()
    };
    assert!(faint.problems().iter().any(|p| matches!(
        p,
        SkinProblem::LowContrast {
            pair: "surface_edge on surface",
            ..
        }
    )));
}

/// The opaque fill a text is drawn on: its own, or the nearest ancestor's, or the window's.
fn solid_behind(app: &App, entity: Entity, skin: &GooeySkin) -> [f32; 3] {
    let mut at = Some(entity);
    while let Some(entity) = at {
        if let Some(fill) = app.world().get::<BackgroundColor>(entity) {
            let fill = Srgba::from(fill.0);
            if fill.alpha >= 0.999 {
                return [fill.red, fill.green, fill.blue];
            }
        }
        at = app.world().get::<ChildOf>(entity).map(|p| p.parent());
    }
    rgb(skin.background)
}

#[test]
fn every_text_a_high_contrast_screen_draws_reads_at_seven_to_one_in_every_state() {
    let skin = GooeySkin::high_contrast();
    let mut app = app();
    app.insert_resource(skin.clone());
    let rest = app.world_mut().spawn(button("Rest")).id();
    let hover = app.world_mut().spawn(button("Hover")).id();
    let press = app.world_mut().spawn(button("Press")).id();
    let off = app.world_mut().spawn(button("Off")).id();
    let widgets = [
        app.world_mut().spawn(toggle("Subtitles", true)).id(),
        app.world_mut().spawn(slider("Volume", 0.7, 0.0..=1.0)).id(),
        app.world_mut()
            .spawn(choice("Quality", ["Low", "High"], 1))
            .id(),
        app.world_mut()
            .spawn(binding("Jump", Binding::Key(KeyCode::Space)))
            .id(),
    ];
    let legend = app
        .world_mut()
        .spawn((prompts(), children![prompt("Select", Prompt::SELECT)]))
        .id();
    let title = app.world_mut().spawn(heading("Options")).id();
    let mut inside = vec![title, rest, hover, press, off, legend];
    inside.extend(widgets);
    let card = app.world_mut().spawn(panel()).add_children(&inside).id();
    app.world_mut().spawn(screen()).add_child(card);
    app.update();
    app.world_mut()
        .entity_mut(hover)
        .insert(bevy::picking::hover::Hovered(true));
    app.world_mut().entity_mut(press).insert(Pressed);
    app.world_mut()
        .entity_mut(off)
        .insert(bevy::ui::InteractionDisabled);
    app.update();

    // What is drawn: a binding's prompt and a choice's other options keep their room and are hidden.
    let mut texts = app
        .world_mut()
        .query_filtered::<(Entity, &Text, &TextColor, &Visibility), ()>();
    let found: Vec<(Entity, String, Color)> = texts
        .iter(app.world())
        .filter(|(.., visibility)| **visibility != Visibility::Hidden)
        .map(|(e, t, c, _)| (e, t.0.clone(), c.0))
        .collect();
    assert!(found.len() >= 12, "the screen draws text: {found:?}");
    for (entity, words, color) in found {
        let ratio = contrast_ratio(rgb(color), solid_behind(&app, entity, &skin));
        assert!(
            ratio >= ENHANCED_CONTRAST,
            "{words:?} is drawn at {ratio:.2}:1, high contrast needs {ENHANCED_CONTRAST}:1"
        );
    }
}

#[test]
fn a_tooltip_and_a_panel_have_a_white_edge_in_high_contrast_and_the_quiet_one_otherwise() {
    for (skin, edge) in [
        (GooeySkin::high_contrast(), Color::WHITE),
        (GooeySkin::dark(), GooeySkin::dark().surface_raised),
        (
            GooeySkin::parchment(),
            GooeySkin::parchment().surface_raised,
        ),
    ] {
        let mut app = app();
        app.insert_resource(skin);
        let card = app.world_mut().spawn(panel()).id();
        let help = app
            .world_mut()
            .spawn((button("Help"), Tooltip::new("Opens the manual")))
            .id();
        let legend = app.world_mut().spawn(prompts()).id();
        app.update();
        let tip = child_with_role(&mut app, help, Role::Tooltip);
        for entity in [card, tip, legend] {
            assert_eq!(
                app.world().get::<BorderColor>(entity),
                Some(&BorderColor::all(edge)),
                "{entity}'s edge"
            );
        }
    }
}
