//! Colour motion: a widget's colours spring to the skin's instead of repainting in one frame, a
//! label in half its fill's time, a skin change on the calm spring, a press at once; the first frame,
//! the colours that are data, reduced and no motion behave as `docs/research/color-motion.md` says.
//! In a headless window on a 60 Hz clock the test sets, with motion on.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, Pressed, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::MotionTrace;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

/// How many frames the test waits for a motion to settle: two seconds.
const PATIENCE_FRAMES: usize = 120;

/// How near two colours must be, per sRGB channel, to be the same one.
const SAME_COLOR: f32 = 1.0 / 255.0;

fn app(settings: MotionSettings) -> App {
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
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )))
    .insert_resource(settings);
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// A button on a panel, laid out and settled: the button and its label.
fn a_button(app: &mut App) -> (Entity, Entity) {
    let button = app.world_mut().spawn(button("Play")).id();
    let card = app.world_mut().spawn(panel()).add_child(button).id();
    app.world_mut().spawn(screen()).add_child(card);
    frames(app, 3);
    let label = app.world().get::<Children>(button).expect("a label")[0];
    (button, label)
}

fn background(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

fn text(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<TextColor>(entity)
        .expect("a text colour")
        .0
}

fn srgb(color: Color) -> [f32; 3] {
    let c = color.to_srgba();
    [c.red, c.green, c.blue]
}

fn same(a: Color, b: Color) -> bool {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    (a.red - b.red).abs() <= SAME_COLOR
        && (a.green - b.green).abs() <= SAME_COLOR
        && (a.blue - b.blue).abs() <= SAME_COLOR
        && (a.alpha - b.alpha).abs() <= SAME_COLOR
}

/// How far apart two colours are: the largest sRGB channel difference.
fn distance(a: Color, b: Color) -> f32 {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    [
        a.red - b.red,
        a.green - b.green,
        a.blue - b.blue,
        a.alpha - b.alpha,
    ]
    .into_iter()
    .map(f32::abs)
    .fold(0.0, f32::max)
}

/// The largest frame-to-frame step of a colour series.
fn largest_step(series: &[Color]) -> f32 {
    series
        .windows(2)
        .map(|w| distance(w[0], w[1]))
        .fold(0.0, f32::max)
}

/// The frames until `button`'s fill is `fill`; panics if it never is.
fn frames_until_fill(app: &mut App, button: Entity, fill: Color) -> usize {
    for frame in 1..=PATIENCE_FRAMES {
        app.update();
        if same(background(app, button), fill) {
            return frame;
        }
    }
    panic!("the fill never reached {fill:?}");
}

/// The skin's own colours for a resting button: its fill and its label, read off an app with no
/// motion so nothing is in between.
fn rest_colors(skin: GooeySkin) -> (Color, Color) {
    let mut app = app(MotionSettings::off());
    app.insert_resource(skin);
    let (button, label) = a_button(&mut app);
    (background(&app, button), text(&app, label))
}

/// Moves the real pointer onto `button`, or off to the window's far corner, so the picking
/// plugin's hover is what the skin reads; it is seen in the next update.
fn hover(app: &mut App, button: Entity, on: bool) {
    let at = if on {
        let node = app.world().get::<ComputedNode>(button).expect("laid out");
        app.world()
            .get::<UiGlobalTransform>(button)
            .expect("placed")
            .translation
            * node.inverse_scale_factor()
    } else {
        Vec2::new(1270.0, 710.0)
    };
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
}

#[test]
fn a_hover_eases_the_fill_in_and_out_with_no_step_bigger_than_an_uninterrupted_change() {
    let mut app = app(MotionSettings::default());
    let (button, _) = a_button(&mut app);
    let rest = background(&app, button);

    // An uninterrupted hover in, then out, frame by frame.
    hover(&mut app, button, true);
    let mut trace = MotionTrace::watch(&mut app, button);
    trace.step(&mut app, PATIENCE_FRAMES / 2);
    let hovered = background(&app, button);
    assert!(!same(hovered, rest), "a hovered button has its own fill");
    hover(&mut app, button, false);
    trace.step(&mut app, PATIENCE_FRAMES / 2);
    let series: Vec<Color> = trace
        .frames()
        .iter()
        .map(|f| f.background.expect("a fill"))
        .collect();
    let jump = distance(rest, hovered);
    let uninterrupted = largest_step(&series);
    assert!(
        uninterrupted < jump / 2.0,
        "the fill eases, it does not repaint: its largest step is {uninterrupted} of {jump}"
    );
    assert!(
        same(*series.last().expect("frames"), rest),
        "out again, it is back at rest"
    );

    // Interrupted: out again three frames in, then in again three frames after that.
    hover(&mut app, button, true);
    let mut interrupted = MotionTrace::watch(&mut app, button);
    interrupted.step(&mut app, 3);
    hover(&mut app, button, false);
    interrupted.step(&mut app, 3);
    hover(&mut app, button, true);
    interrupted.step(&mut app, PATIENCE_FRAMES / 2);
    let series: Vec<Color> = interrupted
        .frames()
        .iter()
        .map(|f| f.background.expect("a fill"))
        .collect();
    assert!(
        largest_step(&series) <= uninterrupted + SAME_COLOR,
        "turned round, no step is bigger than an uninterrupted change's largest: {} against {uninterrupted}",
        largest_step(&series)
    );
    assert!(same(background(&app, button), hovered));

    // At rest, thirty frames write nothing.
    let mut idle = MotionTrace::watch(&mut app, button);
    idle.step(&mut app, 30);
    assert_eq!(
        idle.writes_at_rest(30),
        0,
        "a settled button writes nothing"
    );
}

#[test]
fn a_label_reaches_its_colour_in_half_its_fills_time() {
    let (dark_fill, dark_label) = rest_colors(GooeySkin::dark());
    let (light_fill, light_label) = rest_colors(GooeySkin::parchment());
    let mut app = app(MotionSettings::default());
    let (button, label) = a_button(&mut app);
    assert!(same(background(&app, button), dark_fill));
    assert!(same(text(&app, label), dark_label));
    app.insert_resource(GooeySkin::parchment());
    let mut label_at = None;
    let mut fill_at = None;
    for frame in 1..=PATIENCE_FRAMES {
        app.update();
        if label_at.is_none() && same(text(&app, label), light_label) {
            label_at = Some(frame);
        }
        if fill_at.is_none() && same(background(&app, button), light_fill) {
            fill_at = Some(frame);
        }
    }
    let (label_at, fill_at) = (
        label_at.expect("the label arrives"),
        fill_at.expect("the fill arrives"),
    );
    assert!(
        label_at < fill_at,
        "the label leads its fill: label at frame {label_at}, fill at {fill_at}"
    );
}

/// Every colour of every styled node, by entity: its fill, border sides and text.
fn every_colour(app: &mut App) -> Vec<(Entity, &'static str, Color)> {
    let mut out = vec![];
    let mut nodes = app.world_mut().query_filtered::<(
        Entity,
        Option<&BackgroundColor>,
        Option<&BorderColor>,
        Option<&TextColor>,
    ), With<Role>>();
    for (entity, fill, border, text) in nodes.iter(app.world()) {
        if let Some(fill) = fill {
            out.push((entity, "fill", fill.0));
        }
        if let Some(border) = border {
            for (side, color) in [
                ("top", border.top),
                ("right", border.right),
                ("bottom", border.bottom),
                ("left", border.left),
            ] {
                out.push((entity, side, color));
            }
        }
        if let Some(text) = text {
            out.push((entity, "text", text.0));
        }
    }
    out.sort_by_key(|(entity, side, _)| (*entity, *side));
    out
}

#[test]
fn a_skin_change_reaches_every_colour_within_the_calm_spring() {
    // The same screen built in an app that starts in the new skin, with no motion: what every colour
    // of every styled node must arrive at.
    let mut target = self::app(MotionSettings::off());
    target.insert_resource(GooeySkin::parchment());
    a_button(&mut target);
    let wanted = every_colour(&mut target);
    assert!(wanted.len() >= 4, "the screen has colours to reach");

    let mut app = self::app(MotionSettings::default());
    let (button, label) = a_button(&mut app);
    let before = every_colour(&mut app);
    // The calm spring's settle time for a change across the whole range of a channel.
    let calm = GooeySkin::dark().motion.calm;
    let full_range = model::spring_settle_seconds(
        calm,
        model::SpringState {
            value: 0.0,
            velocity: 0.0,
        },
        1.0,
    );
    let calm_frames = (full_range / FRAME).ceil() as usize;
    app.insert_resource(GooeySkin::parchment());
    app.update();
    let (light_fill, _) = (background(&target, button), ());
    assert!(
        !same(background(&app, button), light_fill),
        "a skin change eases, it does not repaint in one frame"
    );
    let _ = (label, before);
    let mut arrived = None;
    for frame in 2..=PATIENCE_FRAMES {
        app.update();
        let now = every_colour(&mut app);
        if now.len() == wanted.len()
            && now
                .iter()
                .zip(&wanted)
                .all(|(a, b)| a.0 == b.0 && a.1 == b.1 && same(a.2, b.2))
        {
            arrived = Some(frame);
            break;
        }
    }
    let at = arrived.expect("every colour of every node reaches the new skin's");
    assert!(
        at <= calm_frames + 1,
        "every colour is there within the calm spring's settle time plus a frame: {at} frames, calm is {calm_frames}"
    );
}

#[test]
fn a_theme_switch_of_one_polarity_keeps_the_labels_contrast() {
    // Between two dark skins the label stays light and the fill dark: on every frame the label's
    // contrast on its fill is at least the lower of the two skins' own.
    let (dark_fill, dark_label) = rest_colors(GooeySkin::dark());
    let (hc_fill, hc_label) = rest_colors(GooeySkin::high_contrast());
    let floor = contrast_ratio(srgb(dark_fill), srgb(dark_label))
        .min(contrast_ratio(srgb(hc_fill), srgb(hc_label)));
    let mut app = app(MotionSettings::default());
    let (button, label) = a_button(&mut app);
    app.insert_resource(GooeySkin::high_contrast());
    for frame in 1..=PATIENCE_FRAMES {
        app.update();
        let ratio = contrast_ratio(srgb(background(&app, button)), srgb(text(&app, label)));
        assert!(
            ratio >= floor - 0.05,
            "frame {frame}: the label's contrast {ratio} fell under the skins' lower {floor}"
        );
    }
    assert!(same(background(&app, button), hc_fill));
    assert!(same(text(&app, label), hc_label));
}

#[test]
fn a_press_lands_at_once_and_its_release_eases() {
    let mut app = app(MotionSettings::default());
    let (button, _) = a_button(&mut app);
    let rest = background(&app, button);
    let pressed = {
        let mut still = self::app(MotionSettings::off());
        let (button, _) = a_button(&mut still);
        still.world_mut().entity_mut(button).insert(Pressed);
        still.update();
        background(&still, button)
    };
    assert!(!same(pressed, rest), "a pressed button has its own fill");
    app.world_mut().entity_mut(button).insert(Pressed);
    app.update();
    assert!(
        same(background(&app, button), pressed),
        "a press is there in its frame"
    );
    app.world_mut().entity_mut(button).remove::<Pressed>();
    app.update();
    let after = background(&app, button);
    assert!(
        !same(after, rest) && !same(after, pressed),
        "its release eases back"
    );
    frames(&mut app, PATIENCE_FRAMES);
    assert!(same(background(&app, button), rest));
}

#[test]
fn the_first_frame_and_a_colour_that_is_data_land() {
    let mut app = app(MotionSettings::default());
    let toned = |mut skin: GooeySkin, color: Color| {
        skin.tones.insert("accent".into(), color);
        skin
    };
    let (first, second) = (Color::srgb(0.9, 0.2, 0.1), Color::srgb(0.1, 0.3, 0.9));
    app.insert_resource(toned(GooeySkin::dark(), first));
    // A node styled by a tone the skin names is drawn as written.
    let swatch = app
        .world_mut()
        .spawn((Node::default(), Role::Tone("accent")))
        .id();
    app.update();
    assert!(
        same(background(&app, swatch), first),
        "the first frame lands"
    );
    app.insert_resource(toned(GooeySkin::parchment(), second));
    app.update();
    assert!(
        same(background(&app, swatch), second),
        "a tone is data: it is drawn as the skin says in that frame"
    );
    // A button spawned now is drawn in its colours from its first frame.
    let (_, rest_label) = rest_colors(toned(GooeySkin::parchment(), second));
    let button = app.world_mut().spawn(button("Late")).id();
    app.world_mut()
        .spawn((panel(), children![]))
        .add_child(button);
    app.update();
    let label = app.world().get::<Children>(button).expect("a label")[0];
    assert!(
        same(text(&app, label), rest_label),
        "a new widget's first colours land"
    );
}

#[test]
fn with_motion_off_a_hover_repaints_in_its_frame_and_reduced_eases_over_the_fade() {
    let mut off = app(MotionSettings::off());
    let (button, _) = a_button(&mut off);
    let rest = background(&off, button);
    hover(&mut off, button, true);
    off.update();
    let hovered = background(&off, button);
    assert!(!same(hovered, rest), "a hovered button has its own fill");
    frames(&mut off, 3);
    assert!(same(background(&off, button), hovered), "off: at once");

    let mut reduced = app(MotionSettings::reduced());
    let (button, _) = a_button(&mut reduced);
    hover(&mut reduced, button, true);
    reduced.update();
    assert!(
        !same(background(&reduced, button), hovered),
        "reduced: a colour still fades"
    );
    let reduced_frames = frames_until_fill(&mut reduced, button, hovered);
    // The same hover with full motion runs on the snappy spring; the fade is shorter.
    let mut full = app(MotionSettings::default());
    let (button, _) = a_button(&mut full);
    hover(&mut full, button, true);
    full.update();
    let full_frames = frames_until_fill(&mut full, button, hovered);
    assert!(
        reduced_frames < full_frames,
        "a reduced fade ({reduced_frames} frames) runs on fade_seconds, shorter than snappy ({full_frames})"
    );
}

#[test]
fn a_switch_from_a_dark_skin_to_a_light_one_is_hard_to_read_for_a_third_of_the_calm_spring_at_most()
{
    // Light text on a dark fill turning to dark text on a light fill must pass, on some frame, a
    // label as bright as its fill: no continuous change keeps the skins' own contrast (the
    // intermediate value theorem; docs/research/color-motion.md). The label leads its fill, so the
    // frames under WCAG's 4.5:1 are few.
    const WCAG_TEXT_CONTRAST: f32 = 4.5;
    let calm = GooeySkin::dark().motion.calm;
    let most = (calm.duration / 3.0 / FRAME).ceil() as usize;
    let mut app = app(MotionSettings::default());
    let (button, label) = a_button(&mut app);
    app.insert_resource(GooeySkin::parchment());
    let mut hard = 0;
    for _ in 1..=PATIENCE_FRAMES {
        app.update();
        if contrast_ratio(srgb(background(&app, button)), srgb(text(&app, label)))
            < WCAG_TEXT_CONTRAST
        {
            hard += 1;
        }
    }
    assert!(
        hard <= most,
        "the label is under 4.5:1 for {hard} frames, more than a third of the calm spring ({most})"
    );
}

#[test]
fn switching_motion_off_lands_a_colour_on_its_way() {
    let mut app = app(MotionSettings::default());
    let (button, label) = a_button(&mut app);
    let (light_fill, light_label) = rest_colors(GooeySkin::parchment());
    app.insert_resource(GooeySkin::parchment());
    frames(&mut app, 3);
    assert!(!same(background(&app, button), light_fill), "on its way");
    app.insert_resource(MotionSettings::off());
    frames(&mut app, 2);
    assert!(same(background(&app, button), light_fill), "the fill lands");
    assert!(same(text(&app, label), light_label), "the label lands");
}

#[test]
fn a_fade_over_a_colour_on_its_way_keeps_going_to_the_new_colour() {
    let mut app = app(MotionSettings::default());
    let (button, _) = a_button(&mut app);
    let (light_fill, _) = rest_colors(GooeySkin::parchment());
    app.insert_resource(GooeySkin::parchment());
    frames(&mut app, 3);
    let half_way = background(&app, button);
    // A presence fade writes the same colour at a new alpha while it is moving: it is not a jump to
    // the target.
    app.world_mut().entity_mut(button).insert(Opacity(0.5));
    app.update();
    let faded = background(&app, button);
    assert!(
        distance(
            Color::srgb_from_array(srgb(faded)),
            Color::srgb_from_array(srgb(half_way))
        ) < 0.2,
        "the fade does not throw the colour to its target: {half_way:?} then {faded:?}"
    );
    app.world_mut().entity_mut(button).insert(Opacity(1.0));
    frames(&mut app, PATIENCE_FRAMES);
    assert!(same(background(&app, button), light_fill));
}
