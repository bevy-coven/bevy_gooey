//! Design tokens: the surface ladder and its measured steps, the three scales, and `Surface(n)` as a
//! game or a tool sees it in a headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// The sRGB grey whose CIE lightness is `l`, by bisection on the crate's own `cie_lightness`.
fn grey_at(l: f32) -> Color {
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..40 {
        let mid = (low + high) / 2.0;
        if cie_lightness([mid; 3]) < l {
            low = mid;
        } else {
            high = mid;
        }
    }
    Color::srgb(low, low, low)
}

fn ladder(lightnesses: &[f32]) -> GooeySkin {
    GooeySkin {
        surfaces: lightnesses.iter().map(|l| grey_at(*l)).collect(),
        ..GooeySkin::dark()
    }
}

fn steps(skin: &GooeySkin) -> Vec<(usize, usize, f32)> {
    skin.problems()
        .into_iter()
        .filter_map(|problem| match problem {
            SkinProblem::ElevationStep {
                lower,
                upper,
                delta_l,
            } => Some((lower, upper, delta_l)),
            _ => None,
        })
        .collect()
}

#[test]
fn lightness_is_cie_l_star() {
    assert_eq!(cie_lightness([0.0; 3]), 0.0);
    assert!((cie_lightness([1.0; 3]) - 100.0).abs() < 0.01);
    // Mid sRGB grey is about L* 53.4, and a near-black sits on the linear segment.
    assert!((cie_lightness([0.5; 3]) - 53.39).abs() < 0.05);
    // sRGB 0.02 is linear 0.00155 (below the 0.04045 knee), so L* is 903.3 times it.
    assert!((cie_lightness([0.02; 3]) - 1.40).abs() < 0.02);
    let ramp: Vec<f32> = (0..=20)
        .map(|i| cie_lightness([i as f32 / 20.0; 3]))
        .collect();
    assert!(ramp.windows(2).all(|pair| pair[0] < pair[1]), "{ramp:?}");
}

#[test]
fn two_surfaces_under_two_l_star_apart_are_reported_and_two_apart_are_not() {
    let close = ladder(&[10.0, 11.6]);
    let found = steps(&close);
    assert_eq!(found.len(), 1, "{:?}", close.problems());
    let (lower, upper, delta) = found[0];
    assert_eq!((lower, upper), (0, 1));
    assert!((delta - 1.6).abs() < 0.05, "delta {delta}");
    assert!(
        close
            .problems()
            .iter()
            .any(|problem| matches!(problem, SkinProblem::ElevationStep { .. }))
    );

    for apart in [2.2, 4.0, 30.0] {
        let fine = ladder(&[10.0, 10.0 + apart]);
        assert!(fine.problems().is_empty(), "{apart}: {:?}", fine.problems());
    }
}

#[test]
fn only_adjacent_steps_are_compared_and_a_step_down_is_a_step() {
    // 0 and 2 are the same grey, 1 is a recessed well between them: each adjacent pair differs.
    assert!(ladder(&[20.0, 12.0, 20.0]).problems().is_empty());
    // The pair that closes up is named by its place, in a longer ladder.
    let found = steps(&ladder(&[5.0, 10.0, 15.0, 16.0, 25.0]));
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].0, found[0].1), (2, 3));
}

#[test]
fn one_surface_is_a_ladder_and_none_is_not() {
    assert!(ladder(&[12.0]).problems().is_empty());
    let none = GooeySkin {
        surfaces: Vec::new(),
        ..GooeySkin::dark()
    };
    assert_eq!(
        none.problems(),
        vec![SkinProblem::BadScale { field: "surfaces" }]
    );
    // Reading a ladder with no step is never a panic: the panel's colour.
    assert_eq!(none.surface_at(3), none.surface);
}

#[test]
fn a_nan_surface_is_reported_not_passed() {
    let skin = GooeySkin {
        surfaces: vec![
            grey_at(10.0),
            Color::srgb(f32::NAN, 0.2, 0.2),
            grey_at(30.0),
        ],
        ..GooeySkin::dark()
    };
    assert_eq!(steps(&skin).len(), 2);
}

#[test]
fn every_preset_has_a_ladder_of_comfortable_steps_and_no_problem() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        assert!(
            skin.problems().is_empty(),
            "{}: {:?}",
            preset.name(),
            skin.problems()
        );
        assert_eq!(skin.surfaces.len(), 5, "{}", preset.name());
        let lightness: Vec<f32> = skin
            .surfaces
            .iter()
            .map(|c| {
                let c = Srgba::from(*c);
                cie_lightness([c.red, c.green, c.blue])
            })
            .collect();
        for pair in lightness.windows(2) {
            assert!(
                (pair[1] - pair[0]).abs() >= 4.0,
                "{}: {lightness:?}",
                preset.name()
            );
        }
    }
    assert!(
        GooeySkin::high_contrast()
            .high_contrast_problems()
            .is_empty()
    );
}

#[test]
fn a_presets_own_sizes_are_on_its_scales() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let on = |scale: &[f32], value: f32| scale.contains(&value);
        assert!(on(&skin.spacing, skin.gap), "{} gap", preset.name());
        assert!(on(&skin.spacing, skin.padding), "{} padding", preset.name());
        assert!(on(&skin.radii, skin.radius), "{} radius", preset.name());
        assert!(
            on(&skin.type_scale, skin.text_size),
            "{} text_size",
            preset.name()
        );
        assert!(
            on(&skin.type_scale, skin.heading_size),
            "{} heading_size",
            preset.name()
        );
    }
}

#[test]
fn a_step_past_the_end_is_the_last_and_type_follows_the_players_text_size() {
    let skin = GooeySkin {
        text_scale: 1.5,
        ..GooeySkin::dark()
    };
    assert_eq!(skin.space(0), 4.0);
    assert_eq!(skin.space(5), 28.0);
    assert_eq!(skin.space(usize::MAX), 28.0);
    assert_eq!(skin.corner(1), 8.0);
    assert_eq!(skin.corner(50), 16.0);
    // Spacing and corners are not text: only the type scale grows with the player's size.
    assert_eq!(skin.type_size(2), 30.0);
    assert_eq!(skin.type_size(9), 48.0);
    assert_eq!(
        GooeySkin::dark().surface_at(0),
        GooeySkin::dark().background
    );
    assert_eq!(
        GooeySkin::dark().surface_at(255),
        GooeySkin::dark().surfaces[4]
    );
}

#[test]
fn a_scale_that_falls_or_is_not_a_number_is_a_bad_scale() {
    let bad = |skin: GooeySkin| skin.problems();
    assert_eq!(
        bad(GooeySkin {
            spacing: [4.0, 8.0, 6.0, 14.0, 20.0, 28.0],
            ..GooeySkin::dark()
        }),
        vec![SkinProblem::BadScale { field: "spacing" }]
    );
    assert_eq!(
        bad(GooeySkin {
            radii: [4.0, f32::NAN, 16.0],
            ..GooeySkin::dark()
        }),
        vec![SkinProblem::BadScale { field: "radii" }]
    );
    assert_eq!(
        bad(GooeySkin {
            type_scale: [0.0, 16.0, 20.0, 26.0, 32.0],
            ..GooeySkin::dark()
        }),
        vec![SkinProblem::BadScale {
            field: "type_scale"
        }]
    );
    // Zero is a fine first step of a spacing or corner scale (a square corner, no gap).
    assert!(
        bad(GooeySkin {
            spacing: [0.0, 4.0, 8.0, 12.0, 16.0, 24.0],
            radii: [0.0, 4.0, 8.0],
            ..GooeySkin::dark()
        })
        .is_empty()
    );
}

#[test]
fn a_theme_step_brings_the_ladder_and_keeps_the_players_scales() {
    let mine = GooeySkin {
        spacing: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        text_scale: 1.5,
        ..GooeySkin::dark()
    };
    let stepped = mine.clone().with_colours_of(&GooeySkin::parchment());
    assert_eq!(stepped.surfaces, GooeySkin::parchment().surfaces);
    assert_eq!(stepped.spacing, mine.spacing);
    assert_eq!(stepped.text_scale, 1.5);
}

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn app_with(skin: GooeySkin) -> App {
    let mut app = App::new();
    // Assumed owner exception (#76): colours ease on the real clock, so this app reads them landed.
    app.insert_resource(MotionSettings::off());
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(WIDTH, HEIGHT)
                        .with_scale_factor_override(1.0),
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
        .expect("a node has a fill")
        .0
}

#[test]
fn a_surface_node_is_filled_with_its_step_and_follows_the_skin_and_a_scope() {
    let mut app = app_with(GooeySkin::dark());
    let levels = [0u8, 1, 2, 3, 4, 9];
    let nodes: Vec<Entity> = levels
        .iter()
        .map(|level| app.world_mut().spawn(surface(*level)).id())
        .collect();
    let scoped = app
        .world_mut()
        .spawn((surface(2), SkinScope(GooeySkin::tool())))
        .id();
    let card = app.world_mut().spawn(panel()).id();
    for node in &nodes {
        app.world_mut().entity_mut(card).add_child(*node);
    }
    app.world_mut().entity_mut(card).add_child(scoped);
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);

    let dark = GooeySkin::dark();
    for (node, level) in nodes.iter().zip(levels) {
        assert_eq!(fill(&app, *node), dark.surface_at(level), "level {level}");
    }
    // A level past the top is the top; the scoped node draws the scope's ladder, not the resource's.
    assert_eq!(fill(&app, nodes[5]), dark.surfaces[4]);
    assert_eq!(fill(&app, scoped), GooeySkin::tool().surfaces[2]);

    // A new skin restyles every unscoped step in the next frame; the scope keeps its own.
    app.insert_resource(GooeySkin::parchment());
    app.update();
    let parchment = GooeySkin::parchment();
    for (node, level) in nodes.iter().zip(levels) {
        assert_eq!(
            fill(&app, *node),
            parchment.surface_at(level),
            "level {level}"
        );
    }
    assert_eq!(fill(&app, scoped), GooeySkin::tool().surfaces[2]);
}

#[test]
fn a_surface_draws_nothing_but_its_fill() {
    let mut app = app_with(GooeySkin::dark());
    let node = app.world_mut().spawn(surface(2)).id();
    app.world_mut().spawn(screen()).add_child(node);
    settle(&mut app);
    let style = app.world().get::<Node>(node).expect("a node");
    assert_eq!(style.padding, UiRect::DEFAULT);
    assert_eq!(style.border, UiRect::DEFAULT);
    assert_eq!(style.border_radius, BorderRadius::DEFAULT);
    assert_eq!(
        *app.world().get::<BorderColor>(node).expect("a border"),
        BorderColor::DEFAULT
    );
}
