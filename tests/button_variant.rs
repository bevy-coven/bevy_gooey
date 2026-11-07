//! Button variants: a button says what it does. Every variant has three different fills that its ink
//! reads on in every preset (and at 7:1 in the high-contrast one), the running widgets draw them,
//! and a loading button keeps its width and takes no click.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedNode, InteractionDisabled, Pressed};
use bevy::ui_widgets::Activate;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::click;
use bevy_gooey::*;

fn app_with(skin: GooeySkin) -> App {
    let mut app = App::new();
    // Assumed owner exception (#76): colours ease on the real clock, so this app reads them landed.
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
    .insert_resource(skin);
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

fn fill(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

fn ink(app: &App, button: Entity) -> Color {
    let label = app.world().get::<Children>(button).expect("a label")[0];
    app.world().get::<TextColor>(label).expect("ink").0
}

fn width(app: &App, entity: Entity) -> f32 {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    node.size().x * node.inverse_scale_factor()
}

/// A screen holding a panel with one button of every variant, in order.
fn row_of_variants(app: &mut App) -> Vec<Entity> {
    let buttons: Vec<Entity> = ButtonVariant::ALL
        .into_iter()
        .map(|variant| app.world_mut().spawn((button("Label"), variant)).id())
        .collect();
    let card = app.world_mut().spawn(panel()).add_children(&buttons).id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(app);
    buttons
}

#[test]
fn every_variants_three_fills_differ_and_its_ink_reads_on_each_in_every_preset() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        assert_eq!(
            skin.problems(),
            vec![],
            "{preset:?}: the skin with its variants passes its own checks"
        );
        for variant in ButtonVariant::ALL {
            let [rest, hover, pressed] =
                ButtonPhase::ALL.map(|phase| skin.variant_fill(variant, phase));
            assert!(
                rest != hover && hover != pressed && rest != pressed,
                "{preset:?} {variant:?}: rest, hover and pressed fills differ: {rest:?} {hover:?} {pressed:?}"
            );
        }
    }
    assert_eq!(
        GooeySkin::high_contrast().high_contrast_problems(),
        vec![],
        "the high-contrast skin's variants read at 7:1"
    );
}

#[test]
fn a_skin_whose_danger_has_no_lift_is_refused_naming_the_variant() {
    let mut flat = GooeySkin::dark();
    flat.danger = flat.text;
    assert!(
        flat.problems().iter().any(|p| matches!(
            p,
            SkinProblem::Indistinguishable {
                pair: "danger at rest and hovered",
                ..
            }
        )),
        "{:?}",
        flat.problems()
    );
}

#[test]
fn a_button_draws_its_variants_fill_and_ink_at_rest_and_held_and_changing_it_restyles() {
    let skin = GooeySkin::dark();
    let mut app = app_with(skin.clone());
    let buttons = row_of_variants(&mut app);
    for (variant, entity) in ButtonVariant::ALL.into_iter().zip(&buttons) {
        // A neutral button is the plain one: its fill is what it always was.
        let want = skin.variant_fill(variant, ButtonPhase::Rest);
        if variant.is_filled() || variant == ButtonVariant::Neutral {
            assert_eq!(fill(&app, *entity), want, "{variant:?} at rest");
        } else {
            assert_eq!(
                fill(&app, *entity).alpha(),
                0.0,
                "{variant:?} has no fill at rest"
            );
        }
        if variant != ButtonVariant::Submit && variant != ButtonVariant::Neutral {
            assert_eq!(
                ink(&app, *entity),
                skin.variant_ink(variant, ButtonPhase::Rest),
                "{variant:?} ink at rest"
            );
        }
    }
    let danger = buttons[3];
    app.world_mut().entity_mut(danger).insert(Pressed);
    settle(&mut app);
    assert_eq!(
        fill(&app, danger),
        skin.variant_fill(ButtonVariant::Danger, ButtonPhase::Pressed)
    );
    assert_eq!(
        ink(&app, danger),
        skin.variant_ink(ButtonVariant::Danger, ButtonPhase::Pressed)
    );
    app.world_mut().entity_mut(danger).remove::<Pressed>();
    settle(&mut app);
    app.world_mut()
        .entity_mut(danger)
        .insert(ButtonVariant::Outline);
    settle(&mut app);
    assert_eq!(
        fill(&app, danger).alpha(),
        0.0,
        "taking another variant restyles it"
    );
    let frame = app.world().get::<BorderColor>(danger).expect("a border");
    assert_eq!(frame.top, skin.outline, "an outline button draws its frame");
    let ghost = buttons[6];
    assert_eq!(
        app.world()
            .get::<BorderColor>(ghost)
            .expect("a border")
            .top
            .alpha(),
        0.0,
        "a ghost draws none"
    );
}

#[test]
fn a_submit_variant_and_a_primary_button_are_the_same_look() {
    let skin = GooeySkin::dark();
    let mut app = app_with(skin);
    let submit = app
        .world_mut()
        .spawn((button("Go"), ButtonVariant::Submit))
        .id();
    let primary = app.world_mut().spawn(primary_button("Go")).id();
    let card = app
        .world_mut()
        .spawn(panel())
        .add_children(&[submit, primary])
        .id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);
    assert_eq!(fill(&app, submit), fill(&app, primary));
    assert_eq!(ink(&app, submit), ink(&app, primary));
}

#[test]
fn a_loading_button_keeps_its_width_hides_its_label_and_a_click_sends_no_activate() {
    #[derive(Resource, Default)]
    struct Activations(u32);
    let mut app = app_with(GooeySkin::dark());
    app.init_resource::<Activations>();
    let save = app
        .world_mut()
        .spawn((button("Save changes"), ButtonVariant::Submit))
        .observe(|_: On<Activate>, mut n: ResMut<Activations>| n.0 += 1)
        .id();
    let card = app.world_mut().spawn(panel()).add_child(save).id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);
    let before = width(&app, save);
    let skin = GooeySkin::dark();
    let looks: Vec<Color> = ButtonPhase::ALL
        .map(|p| skin.variant_fill(ButtonVariant::Submit, p))
        .into();

    click(&mut app, save);
    settle(&mut app);
    assert_eq!(
        app.world().resource::<Activations>().0,
        1,
        "a click presses it when it is idle"
    );

    app.world_mut().entity_mut(save).insert(Loading(true));
    settle(&mut app);
    assert!(
        (width(&app, save) - before).abs() <= 0.5,
        "width kept: {} against {before}",
        width(&app, save)
    );
    let label = app.world().get::<Children>(save).expect("kids")[0];
    assert_eq!(
        app.world().get::<Visibility>(label),
        Some(&Visibility::Hidden),
        "the label gives way"
    );
    let spinners = app
        .world_mut()
        .query_filtered::<Entity, With<LoadingSpinner>>()
        .iter(app.world())
        .count();
    assert_eq!(spinners, 1, "a spinner stands over the label");
    assert!(
        looks.contains(&fill(&app, save)),
        "it still looks like a Submit button, not a disabled one: {:?}",
        fill(&app, save)
    );
    assert_ne!(fill(&app, save), skin.surface);
    click(&mut app, save);
    settle(&mut app);
    assert_eq!(
        app.world().resource::<Activations>().0,
        1,
        "no Activate while loading"
    );

    app.world_mut().entity_mut(save).insert(Loading(false));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(label),
        Some(&Visibility::Inherited)
    );
    click(&mut app, save);
    settle(&mut app);
    assert_eq!(
        app.world().resource::<Activations>().0,
        2,
        "and it presses again when it is done"
    );
}

#[test]
fn a_quiet_button_held_is_seen_to_change_on_the_panel_it_stands_on() {
    use bevy_gooey::model::{cie_lightness, composite};
    let rgb = |c: Color| {
        let c = Srgba::from(c);
        [c.red, c.green, c.blue]
    };
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let ground = rgb(skin.solid_surface());
        let seen = |variant, phase| {
            let fill = skin.variant_fill(variant, phase);
            cie_lightness(composite(rgb(fill), fill.alpha(), ground))
        };
        for variant in [
            ButtonVariant::Outline,
            ButtonVariant::Ghost,
            ButtonVariant::GhostMuted,
            ButtonVariant::GhostFaint,
        ] {
            let step =
                (seen(variant, ButtonPhase::Rest) - seen(variant, ButtonPhase::Pressed)).abs();
            assert!(
                step >= 4.0,
                "{preset:?} {variant:?}: held looks {step} L* from rest, not different enough"
            );
        }
    }
}

#[test]
fn a_host_disabled_button_stays_disabled_after_it_loads_and_a_removed_loading_releases() {
    let mut app = app_with(GooeySkin::dark());
    let held = app
        .world_mut()
        .spawn((button("Save"), ButtonVariant::Submit, InteractionDisabled))
        .id();
    let plain = app.world_mut().spawn((button("Other"), Loading(true))).id();
    let card = app
        .world_mut()
        .spawn(panel())
        .add_children(&[held, plain])
        .id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);
    let skin = GooeySkin::dark();

    app.world_mut().entity_mut(held).insert(Loading(true));
    settle(&mut app);
    assert_ne!(
        fill(&app, held),
        skin.surface,
        "drawn as itself while it loads"
    );
    app.world_mut().entity_mut(held).insert(Loading(false));
    settle(&mut app);
    assert!(
        app.world().get::<InteractionDisabled>(held).is_some(),
        "the host's disabling outlives the load"
    );
    assert_eq!(
        fill(&app, held),
        skin.surface,
        "and it looks disabled again at once"
    );

    // A button spawned loading, then stripped of the component, is released.
    assert!(app.world().get::<LoadingHold>(plain).is_some());
    app.world_mut().entity_mut(plain).remove::<Loading>();
    settle(&mut app);
    assert!(app.world().get::<LoadingHold>(plain).is_none());
    assert!(app.world().get::<InteractionDisabled>(plain).is_none());
}
