//! State roles: the skin owns the interaction states a host composes its own controls from
//! (hover, pressed, held, held ground, unchecked, card, preview edge), each answering from the
//! nearest field when the skin does not set it, and the pairs the crate can know are checked.

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

const FILL_ROLES: [Role; 6] = [
    Role::Hovered,
    Role::Pressed,
    Role::Held,
    Role::HeldGround,
    Role::Unchecked,
    Role::Card,
];

fn srgb(color: Color) -> Srgba {
    Srgba::from(color)
}

/// A grey of CIE L\* `target`, the lightness `STATE_STEP` measures.
fn grey_at(target: f32) -> Color {
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if cie_lightness([mid, mid, mid]) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Color::srgb(lo, lo, lo)
}

fn rgb(color: Color) -> [f32; 3] {
    let c = srgb(color);
    [c.red, c.green, c.blue]
}

#[test]
fn a_skin_that_sets_none_of_them_answers_every_role_in_every_preset() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        assert_eq!(skin.pressed, None);
        assert_eq!(skin.hover, None);
        // The defaults: the nearest field, so nothing is transparent or left to the host.
        assert_eq!(skin.pressed(), skin.solid_surface(), "{}", preset.name());
        assert_eq!(skin.card(), skin.solid_surface());
        assert_eq!(skin.unchecked(), skin.text_muted);
        assert_eq!(skin.preview_edge(), skin.outline);
        assert_eq!(skin.hovered(), skin.surface_raised.mix(&skin.accent, 0.35));
        assert!(skin.problems().is_empty(), "{}", preset.name());
        assert!(
            skin.held() != skin.text && skin.held_ground() != skin.solid_surface(),
            "held steps back"
        );
    }
    assert!(GooeySkin::high_contrast().high_contrast_problems().is_empty());
}

#[test]
fn a_node_with_a_state_role_is_drawn_with_that_skin_colour() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app_with(skin.clone());
        let nodes = FILL_ROLES.map(|role| app.world_mut().spawn((role, Node::default())).id());
        let edge = app
            .world_mut()
            .spawn((
                Role::PreviewEdge,
                Node {
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
            ))
            .id();
        settle(&mut app);
        let want = [
            skin.hovered(),
            skin.pressed(),
            skin.held(),
            skin.held_ground(),
            skin.unchecked(),
            skin.card(),
        ];
        for ((node, want), role) in nodes.into_iter().zip(want).zip(FILL_ROLES) {
            assert_eq!(fill(&app, node), Some(want), "{role:?} in {}", preset.name());
            assert_ne!(want, Color::NONE);
        }
        assert_eq!(fill(&app, edge), Some(Color::NONE));
        assert_eq!(
            app.world().get::<BorderColor>(edge).map(|b| b.top),
            Some(skin.preview_edge())
        );
    }
}

#[test]
fn pressed_at_the_lightness_of_hover_is_a_pair_a_reader_cannot_tell_apart() {
    let mut skin = GooeySkin::dark();
    let hovered = skin.hovered();
    // The same lightness, a different hue and still readable under the text: the only fault is
    // that pressed looks the same as hover.
    skin.pressed = Some(grey_at(cie_lightness(rgb(hovered))));
    let problems = skin.problems();
    assert!(
        problems.iter().any(|p| matches!(
            p,
            SkinProblem::Indistinguishable { pair: "pressed and hovered", needed, .. }
                if *needed == STATE_STEP
        )),
        "{problems:?}"
    );
    // Nothing else is wrong with that skin: the check is the step and not a contrast.
    assert_eq!(problems.len(), 1, "{problems:?}");

    // Just under the step is reported, a step of the bar is not.
    let l = cie_lightness(rgb(hovered));
    let lightness_pair = |delta: f32| GooeySkin {
        pressed: Some(grey_at(l - delta)),
        ..GooeySkin::dark()
    };
    assert!(
        lightness_pair(STATE_STEP - 1.0)
            .problems()
            .iter()
            .any(|p| matches!(p, SkinProblem::Indistinguishable { .. }))
    );
    assert!(
        !lightness_pair(STATE_STEP + 1.0)
            .problems()
            .iter()
            .any(|p| matches!(p, SkinProblem::Indistinguishable { .. }))
    );
    assert!(
        SkinProblem::Indistinguishable {
            pair: "pressed and hovered",
            delta_l: 0.0,
            needed: STATE_STEP
        }
        .to_string()
        .contains("pressed and hovered")
    );
}

#[test]
fn a_pressed_or_card_ground_the_text_cannot_be_read_on_is_reported() {
    let mut skin = GooeySkin::dark();
    skin.pressed = Some(skin.text);
    skin.card = Some(skin.text);
    let pairs: Vec<_> = skin
        .problems()
        .into_iter()
        .filter_map(|p| match p {
            SkinProblem::LowContrast { pair, .. } => Some(pair),
            _ => None,
        })
        .collect();
    assert!(pairs.contains(&"text on pressed"), "{pairs:?}");
    assert!(pairs.contains(&"text on card"), "{pairs:?}");

    let mut marks = GooeySkin::dark();
    marks.unchecked = Some(marks.solid_surface());
    marks.preview_edge = Some(marks.solid_surface());
    let pairs: Vec<_> = marks
        .problems()
        .into_iter()
        .filter_map(|p| match p {
            SkinProblem::LowContrast { pair, .. } => Some(pair),
            _ => None,
        })
        .collect();
    assert!(pairs.contains(&"unchecked on surface"), "{pairs:?}");
    assert!(pairs.contains(&"preview_edge on surface"), "{pairs:?}");
}

#[test]
fn the_hover_a_skin_sets_is_the_one_a_hovered_button_draws_and_is_checked() {
    let mut skin = GooeySkin::dark();
    let hover = Color::srgb(0.30, 0.45, 0.20);
    skin.hover = Some(hover);
    assert_eq!(skin.hovered(), hover);

    let mut app = app_with(skin.clone());
    let button = app.world_mut().spawn(button("Play")).id();
    let host = app.world_mut().spawn((Role::Hovered, Node::default())).id();
    let card = app.world_mut().spawn(panel()).id();
    app.world_mut().entity_mut(card).add_child(button);
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);
    // A real pointer over it, so the picking plugin's own hover is what the button reads.
    let node = app.world().get::<ComputedNode>(button).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(button)
        .expect("placed")
        .translation
        * node.inverse_scale_factor();
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("a window");
    let location = Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    };
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(fill(&app, button), Some(hover), "the crate's own hover");
    assert_eq!(fill(&app, host), Some(hover), "the host's hover");

    // The pair "text on hovered button" now measures the colour the skin set.
    let mut dim = GooeySkin::dark();
    dim.hover = Some(dim.text);
    assert!(dim.problems().iter().any(|p| matches!(
        p,
        SkinProblem::LowContrast { pair: "text on hovered button", .. }
    )));
}

#[test]
fn held_and_held_ground_come_off_one_fade() {
    let skin = GooeySkin::dark();
    assert_eq!(skin.held_fade, 0.6);
    assert_eq!(skin.held(), skin.text.mix(&skin.background, 0.6));
    assert_eq!(
        skin.held_ground(),
        skin.solid_surface().mix(&skin.background, 0.6)
    );
    let none = GooeySkin {
        held_fade: 0.0,
        ..GooeySkin::dark()
    };
    assert_eq!((none.held(), none.held_ground()), (none.text, none.solid_surface()));
    let gone = GooeySkin {
        held_fade: 1.0,
        ..GooeySkin::dark()
    };
    assert_eq!(
        (gone.held(), gone.held_ground()),
        (gone.background, gone.background)
    );
    // One number moves both, in step: more fade is nearer the background for each.
    let l = |c: Color| cie_lightness(rgb(c));
    let (a, b) = (
        GooeySkin {
            held_fade: 0.3,
            ..GooeySkin::dark()
        },
        GooeySkin {
            held_fade: 0.8,
            ..GooeySkin::dark()
        },
    );
    assert!(l(a.held()) > l(b.held()));
    assert!(l(a.held_ground()) > l(b.held_ground()));
    // A skin that sets one answers for it; the other still follows the fade.
    let set = GooeySkin {
        held: Some(Color::srgb(0.9, 0.1, 0.1)),
        ..GooeySkin::dark()
    };
    assert_eq!(set.held(), Color::srgb(0.9, 0.1, 0.1));
    assert_eq!(set.held_ground(), GooeySkin::dark().held_ground());
}

#[test]
fn a_fade_outside_zero_to_one_is_a_bad_size_and_never_extrapolates_a_colour() {
    for bad in [-0.1, 1.5, f32::NAN, f32::INFINITY] {
        let skin = GooeySkin {
            held_fade: bad,
            ..GooeySkin::dark()
        };
        assert!(
            skin.problems()
                .contains(&SkinProblem::BadSize { field: "held_fade" }),
            "{bad}"
        );
        for c in [skin.held(), skin.held_ground()] {
            let c = srgb(c);
            assert!(
                [c.red, c.green, c.blue]
                    .into_iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(&v)),
                "{bad}: {c:?}"
            );
        }
    }
}

#[test]
fn a_state_role_follows_a_changed_skin_a_swapped_role_and_a_scope() {
    let mut app = app_with(GooeySkin::dark());
    let row = app.world_mut().spawn((Role::Hovered, Node::default())).id();
    let scoped = app
        .world_mut()
        .spawn((
            Role::Pressed,
            Node::default(),
            SkinScope(GooeySkin::parchment()),
        ))
        .id();
    settle(&mut app);
    assert_eq!(fill(&app, row), Some(GooeySkin::dark().hovered()));
    assert_eq!(fill(&app, scoped), Some(GooeySkin::parchment().pressed()));

    // The pointer goes down: the host swaps the role and the fill follows the next frame.
    app.world_mut().entity_mut(row).insert(Role::Pressed);
    app.update();
    assert_eq!(fill(&app, row), Some(GooeySkin::dark().pressed()));

    // A skin the game changes restyles it; the scoped node keeps its own skin.
    let sunk = Color::srgb(0.02, 0.03, 0.05);
    app.insert_resource(GooeySkin {
        pressed: Some(sunk),
        ..GooeySkin::dark()
    });
    app.update();
    assert_eq!(fill(&app, row), Some(sunk));
    assert_eq!(fill(&app, scoped), Some(GooeySkin::parchment().pressed()));

    // A preview edge swapped for a fill leaves no stale fill: the edge role clears it.
    app.world_mut().entity_mut(row).insert(Role::PreviewEdge);
    app.update();
    assert_eq!(fill(&app, row), Some(Color::NONE));
}

#[test]
fn a_theme_step_carries_the_state_colours_with_the_rest_of_the_palette() {
    let custom = GooeySkin {
        pressed: Some(Color::srgb(0.02, 0.02, 0.02)),
        card: Some(Color::srgb(0.3, 0.3, 0.4)),
        held_fade: 0.25,
        ..GooeySkin::dark()
    };
    let stepped = custom.with_colours_of(&GooeySkin::parchment());
    // A preset's palette is every colour field: a host's override is replaced by the look's own
    // (derived) answer, the way its other colours are; the fade is a size and stays.
    assert_eq!(stepped.pressed, None);
    assert_eq!(stepped.card, None);
    assert_eq!(stepped.held_fade, 0.25);
    assert_eq!(stepped.pressed(), stepped.solid_surface());
    // The preset row still names each preset's own look, state colours included.
    for preset in SkinPreset::ALL {
        assert_eq!(SkinPreset::of(&preset.skin()), preset);
    }
    // A tuned pressed fill counts toward which look a skin is nearest.
    let tuned = GooeySkin {
        pressed: Some(Color::srgb(0.9, 0.9, 0.9)),
        ..GooeySkin::dark()
    };
    assert_eq!(SkinPreset::of(&tuned), SkinPreset::Dark);
}

#[cfg(feature = "serde")]
#[test]
fn a_skin_file_with_state_colours_reads_back_and_one_without_has_none()
-> Result<(), Box<dyn std::error::Error>> {
    let skin = GooeySkin {
        pressed: Some(Color::srgb(0.02, 0.03, 0.05)),
        held_fade: 0.4,
        preview_edge: Some(Color::srgb(0.6, 0.6, 0.7)),
        ..GooeySkin::dark()
    };
    let back = ron::from_str::<GooeySkin>(&ron::to_string(&skin)?)?;
    assert_eq!(back, skin);
    let file = ron::from_str::<GooeySkin>("(text_scale: 1.5)")?;
    assert_eq!(file.pressed, None);
    assert_eq!(file.held_fade, 0.6);
    Ok(())
}

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
    .insert_resource(skin);
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

fn fill(app: &App, entity: Entity) -> Option<Color> {
    app.world().get::<BackgroundColor>(entity).map(|b| b.0)
}
