//! The colour field: a [`crate::text_input`] row whose text is a hex colour, with a swatch that
//! opens a picker (a saturation and value area and strips for hue, brightness and alpha) in a modal
//! popover. The text row does the typing; this keeps the colour, applies what is typed, turns the
//! picker's pointer, keys and sliders into one [`ValueChange<Srgba>`] and keeps every part of the
//! row and its picker showing the same colour.

use bevy::color::Srgba;
use bevy::ecs::relationship::RelatedSpawner;
use bevy::ecs::spawn::SpawnWith;
use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::FocusedInput;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::events::{
    PointerCancel, PointerDrag, PointerDragEnd, PointerPress, PointerRelease,
};
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{ComputedUiRenderTargetInfo, InteractionDisabled, UiGlobalTransform};
use bevy::ui_widgets::{
    Activate, Slider, SliderOrientation, SliderRange, SliderStep, SliderThumb, SliderValue,
    TrackClick, ValueChange, observe, slider_self_update,
};

use crate::model::{HexError, area_values, hex_text, hsv_to_rgb, parse_hex, rgb_to_hsv};
use crate::number::{MIXED_HINT, MixedValue};
use crate::popover::{Placement, PopoverPanel};
use crate::skin::Role;
use crate::text::{TextEdited, TextHint, TextInputOf, TextInputValue};
use crate::widgets::{ONE_LINE, popover_modal};

/// The colour a key press moves the area's saturation or value by (Shift: ten times).
const AREA_KEY_STEP: f32 = 0.01;

/// The colour a D-pad press moves the area's saturation by, a twentieth as a slider's step is.
const AREA_PAD_STEP: f32 = 0.05;

/// One step of a strip's keys and D-pad: a hundredth of its range.
const STRIP_STEPS: f32 = 100.0;

/// What a [`crate::color_field`] holds: its colour, sRGB with alpha. It is the widget's state: read
/// it to know the colour, set it to change the field from code (the swatch, the text and an open
/// picker show it in the same frame and no [`ValueChange`] is sent for a change the game made
/// itself). The player's own changes are one `ValueChange<Srgba>` on the row each: `is_final` is
/// false while a drag is still rolling the colour and true when it ends, on Enter, when focus
/// leaves the text after an edit, and for a key or D-pad step.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn on_tint(tint: Query<&ColorValue, Changed<ColorValue>>) {
///     for tint in &tint {
///         info!("tint is {}", hex_text(tint.0.to_f32_array()));
///     }
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct ColorValue(pub Srgba);

impl Default for ColorValue {
    fn default() -> Self {
        Self(Srgba::WHITE)
    }
}

/// Text the player typed that a [`crate::color_field`] would not take, sent on the row with the
/// reason ([`HexError`] reads as words). The field shows the colour again.
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq)]
pub struct ColorRefused {
    /// The colour field's row.
    #[event_target]
    pub entity: Entity,
    /// What the player typed.
    pub text: String,
    /// Why it was refused.
    pub reason: HexError,
}

/// The open picker of a [`crate::color_field`]: the popover [`crate::color_field`]'s swatch opens. It
/// keeps the hue, saturation and value the player is on, which the row's [`ColorValue`] cannot (a
/// grey has no hue and a black no saturation, so dragging to black and back would forget where the
/// player was). A game never builds one; it may read which row a picker belongs to.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ColorPicker {
    field: Entity,
    hue: f32,
    saturation: f32,
    value: f32,
}

impl ColorPicker {
    /// The colour field's row this picker edits.
    pub fn field(&self) -> Entity {
        self.field
    }

    /// The hue (degrees), saturation and value (each 0 to 1) the picker is on.
    pub fn hsv(&self) -> [f32; 3] {
        [self.hue, self.saturation, self.value]
    }
}

/// On a colour field's swatch button: its row. Part of what [`crate::color_field`] spawns, a game
/// never builds one.
#[derive(Component, Clone, Copy, Debug)]
pub struct ColorSwatch(pub(crate) Entity);

/// One half of a swatch: the colour opaque (`alpha` false) or at its own alpha (`alpha` true), so a
/// translucent colour shows as one. Part of what [`crate::color_field`] spawns.
#[derive(Component, Clone, Copy, Debug)]
pub struct SwatchHalf {
    pub(crate) row: Entity,
    pub(crate) alpha: bool,
}

/// The picker's saturation and value area, a focusable plane: the pointer sets both, the arrow
/// keys step them, and the D-pad's Left and Right step the saturation. Part of the picker.
#[derive(Component, Clone, Copy, Debug)]
pub struct ColorArea {
    pub(crate) picker: Entity,
}

/// The ring on the area at the colour's saturation and value. Part of the picker.
#[derive(Component, Clone, Copy, Debug)]
pub struct AreaMarker {
    pub(crate) picker: Entity,
}

/// Which channel one of the picker's strips sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Channel {
    Hue,
    Value,
    Alpha,
}

/// A picker's slider: hue, brightness or alpha. Part of the picker.
#[derive(Component, Clone, Copy, Debug)]
pub struct ColorStrip {
    pub(crate) picker: Entity,
    pub(crate) channel: Channel,
}

/// The gradient a strip's rail shows. Part of the picker.
#[derive(Component, Clone, Copy, Debug)]
pub struct ColorRail {
    pub(crate) picker: Entity,
    pub(crate) channel: Channel,
}

/// On the area from the press that began a drag until it ends: one final change closes it.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct AreaGrab;

/// What an edit of the picker sets.
#[derive(Clone, Copy, Debug)]
enum Edit {
    /// Saturation and value together, from the area.
    Area([f32; 2]),
    /// The hue, in degrees.
    Hue(f32),
    /// The value (brightness).
    Value(f32),
    /// The alpha.
    Alpha(f32),
    /// Nothing: the colour as it stands, to end a drag.
    Settle,
}

/// An edit from one of the picker's parts, sent to the picker's panel, which makes it one
/// [`ValueChange<Srgba>`] on the row.
#[derive(EntityEvent, Clone, Copy, Debug)]
struct Pick {
    #[event_target]
    entity: Entity,
    edit: Edit,
    is_final: bool,
}

/// A request to step the area's saturation by `steps` of a pad press, sent to the area.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub(crate) struct AreaStepBy {
    #[event_target]
    pub(crate) entity: Entity,
    pub(crate) steps: f32,
}

/// The ramp a strip's rail or the area's veil draws: `stops` in order, at even places, left to right
/// or top to bottom, interpolated in sRGB (the hue's six primaries then blend as a hue ramp does).
fn ramp(angle: f32, stops: &[Color]) -> Gradient {
    let last = (stops.len().max(2) - 1) as f32;
    LinearGradient {
        color_space: InterpolationColorSpace::Srgba,
        angle,
        stops: stops
            .iter()
            .enumerate()
            .map(|(i, &c)| ColorStop::new(c, Val::Percent(100.0 * i as f32 / last)))
            .collect(),
    }
    .into()
}

/// The six primaries and back to red: the hue strip, which never changes.
fn hue_ramp() -> BackgroundGradient {
    let at = |h: f32| Color::srgb_from_array(hsv_to_rgb(h, 1.0, 1.0));
    BackgroundGradient(vec![ramp(
        LinearGradient::TO_RIGHT,
        &[
            at(0.0),
            at(60.0),
            at(120.0),
            at(180.0),
            at(240.0),
            at(300.0),
            at(360.0),
        ],
    )])
}

/// The strip a channel draws for the colour the picker is on: black up to the colour at full
/// value, or the colour clear up to the colour opaque. The hue strip is [`hue_ramp`].
fn strip_ramp(channel: Channel, hsv: [f32; 3], rgb: [f32; 3]) -> BackgroundGradient {
    let to = |c: [f32; 3], alpha: f32| Color::srgba(c[0], c[1], c[2], alpha);
    match channel {
        Channel::Hue => hue_ramp(),
        Channel::Value => BackgroundGradient(vec![ramp(
            LinearGradient::TO_RIGHT,
            &[Color::BLACK, to(hsv_to_rgb(hsv[0], hsv[1], 1.0), 1.0)],
        )]),
        Channel::Alpha => BackgroundGradient(vec![ramp(
            LinearGradient::TO_RIGHT,
            &[to(rgb, 0.0), to(rgb, 1.0)],
        )]),
    }
}

/// The area's veil: white clear to the right over the hue's own colour, then clear black below, so
/// saturation grows to the right and value upwards. It never changes; the hue is the node's
/// background.
fn area_veil() -> BackgroundGradient {
    BackgroundGradient(vec![
        ramp(
            LinearGradient::TO_RIGHT,
            &[
                Color::srgba(1.0, 1.0, 1.0, 1.0),
                Color::srgba(1.0, 1.0, 1.0, 0.0),
            ],
        ),
        ramp(
            LinearGradient::TO_BOTTOM,
            &[
                Color::srgba(0.0, 0.0, 0.0, 0.0),
                Color::srgba(0.0, 0.0, 0.0, 1.0),
            ],
        ),
    ])
}

/// What a colour row's text is for a colour: its hex, or nothing while the colour is mixed (the
/// hint shows [`MIXED_HINT`]).
fn text_of(value: Srgba, mixed: bool) -> String {
    if mixed {
        String::new()
    } else {
        hex_text(value.to_f32_array())
    }
}

/// Keeps the state: a [`ValueChange<Srgba>`] on a colour row becomes its [`ColorValue`], and the
/// colour is one colour again. The row's own observer.
pub(crate) fn color_self_update(
    change: On<ValueChange<Srgba>>,
    mut rows: Query<&mut ColorValue>,
    mut commands: Commands,
) {
    if let Ok(mut held) = rows.get_mut(change.source) {
        held.set_if_neq(ColorValue(change.value));
        commands.entity(change.source).remove::<MixedValue>();
    }
}

/// A colour row's text ends an edit (Enter, or focus leaving after a change): a hex colour is the
/// new value; anything else is refused with its reason and the field shows the colour again. The
/// text is written back as the field shows colours (`#f80` is `#FF8800`).
pub(crate) fn color_on_text(
    change: On<ValueChange<String>>,
    rows: Query<(
        &ColorValue,
        Has<MixedValue>,
        &TextInputValue,
        Has<InteractionDisabled>,
    )>,
    mut commands: Commands,
) {
    if !change.is_final {
        return;
    }
    let row = change.source;
    let Ok((value, mixed, shown, disabled)) = rows.get(row) else {
        return;
    };
    if disabled || (mixed && change.value.trim().is_empty()) {
        return;
    }
    match parse_hex(&change.value) {
        Ok([r, g, b, a]) => {
            let typed = Srgba::new(r, g, b, a);
            if mixed || typed != value.0 {
                commands.trigger(ValueChange {
                    source: row,
                    value: typed,
                    is_final: true,
                });
            }
            // Queued, so it lands after the text row's own observer has taken the typed text.
            let text = text_of(typed, false);
            if shown.0 != text {
                commands.entity(row).insert(TextInputValue(text));
            }
        }
        Err(reason) => {
            let text = text_of(value.0, mixed);
            if shown.0 != text {
                commands.entity(row).insert(TextInputValue(text));
            }
            commands.trigger(ColorRefused {
                entity: row,
                text: change.value.clone(),
                reason,
            });
        }
    }
}

/// Escape on a colour row's field puts the colour's text back and ends the edit with no change (the
/// blur it causes sends nothing); the press goes on, so inside a dialog it is still Back.
pub(crate) fn color_on_key_input(
    input: On<FocusedInput<KeyboardInput>>,
    fields: Query<&TextInputOf>,
    mut rows: Query<(&ColorValue, Has<MixedValue>, &mut TextInputValue)>,
    mut commands: Commands,
) {
    if input.input.state != ButtonState::Pressed || input.input.key_code != KeyCode::Escape {
        return;
    }
    let Ok(&TextInputOf(row)) = fields.get(input.focused_entity) else {
        return;
    };
    if let Ok((value, mixed, mut shown)) = rows.get_mut(row) {
        let text = text_of(value.0, mixed);
        if shown.0 != text {
            shown.0 = text;
        }
        commands.entity(row).remove::<TextEdited>();
    }
}

/// The swatch is pressed: it opens the picker below it, or closes the one it opened. A disabled row
/// opens nothing.
pub(crate) fn swatch_on_activate(
    activate: On<Activate>,
    swatches: Query<&ColorSwatch>,
    rows: Query<&ColorValue, Without<InteractionDisabled>>,
    open: Query<(Entity, &PopoverPanel), With<ColorPicker>>,
    mut commands: Commands,
) {
    let Ok(&ColorSwatch(row)) = swatches.get(activate.entity) else {
        return;
    };
    if let Some((popover, _)) = open.iter().find(|(_, p)| p.anchor == activate.entity) {
        commands.entity(popover).despawn();
        return;
    }
    let Ok(value) = rows.get(row) else {
        return;
    };
    commands.spawn((
        popover_modal(activate.entity, Placement::Below),
        color_picker(row, value.0),
    ));
}

/// What an open picker adds to its modal popover: the state, the observer that turns its parts'
/// edits into one change on the row, and the area and the three strips.
fn color_picker(row: Entity, value: Srgba) -> impl Bundle {
    let [h, s, v] = rgb_to_hsv([value.red, value.green, value.blue], [0.0, 1.0, 1.0]);
    (
        ColorPicker {
            field: row,
            hue: h,
            saturation: s,
            value: v,
        },
        observe(picker_on_pick),
        Children::spawn(SpawnWith(move |panel: &mut RelatedSpawner<ChildOf>| {
            let picker = panel.target_entity();
            let rgb = [value.red, value.green, value.blue];
            panel.spawn((
                Node::default(),
                Role::ColorArea,
                ColorArea { picker },
                BackgroundColor(Color::srgb_from_array(hsv_to_rgb(h, 1.0, 1.0))),
                area_veil(),
                Hovered::default(),
                TabIndex(0),
                AutoDirectionalNavigation::default(),
                (
                    observe(area_on_press),
                    observe(area_on_drag),
                    observe(area_on_release),
                    observe(area_on_drag_end),
                    observe(area_on_cancel),
                    observe(area_on_key),
                    observe(area_on_step),
                ),
                children![(
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(s * 100.0),
                        top: Val::Percent((1.0 - v) * 100.0),
                        ..default()
                    },
                    UiTransform::from_translation(Val2::percent(-50.0, -50.0)),
                    Role::ColorAreaMarker,
                    AreaMarker { picker },
                    Pickable::IGNORE,
                    children![(
                        Node::default(),
                        Role::ColorAreaMarkerInner,
                        Pickable::IGNORE
                    )],
                )],
            ));
            panel.spawn(strip(picker, Channel::Hue, "Hue", h, 360.0, hue_ramp()));
            panel.spawn(strip(
                picker,
                Channel::Value,
                "Brightness",
                v,
                1.0,
                strip_ramp(Channel::Value, [h, s, v], rgb),
            ));
            panel.spawn(strip(
                picker,
                Channel::Alpha,
                "Alpha",
                value.alpha,
                1.0,
                strip_ramp(Channel::Alpha, [h, s, v], rgb),
            ));
        })),
    )
}

/// One of the picker's strips: a [`crate::slider`] without its readout and fill, whose rail is the
/// gradient of the channel it sets.
fn strip(
    picker: Entity,
    channel: Channel,
    label: &str,
    value: f32,
    max: f32,
    gradient: BackgroundGradient,
) -> impl Bundle {
    let fraction = (value / max).clamp(0.0, 1.0);
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Slider {
            track_click: TrackClick::Snap,
            orientation: SliderOrientation::Horizontal,
        },
        SliderValue(value),
        SliderRange::new(0.0, max),
        SliderStep(max / STRIP_STEPS),
        Role::Slider,
        ColorStrip { picker, channel },
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        observe(slider_self_update),
        observe(strip_on_change),
        children![
            (
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Role::SliderHeader,
                children![(Text::new(label), ONE_LINE, Role::ControlText)],
            ),
            (
                Node {
                    align_items: AlignItems::Center,
                    ..default()
                },
                Role::SliderTrack,
                children![
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        Role::ColorRail,
                        ColorRail { picker, channel },
                        gradient,
                    ),
                    (
                        Node {
                            flex_grow: fraction,
                            ..default()
                        },
                        Role::SliderGap,
                    ),
                    (Node::default(), Role::SliderThumb, SliderThumb),
                ],
            ),
        ],
    )
}

/// A strip's slider moved: the picker takes the channel.
fn strip_on_change(
    change: On<ValueChange<f32>>,
    strips: Query<&ColorStrip>,
    mut commands: Commands,
) {
    let Ok(strip) = strips.get(change.source) else {
        return;
    };
    let edit = match strip.channel {
        Channel::Hue => Edit::Hue(change.value),
        Channel::Value => Edit::Value(change.value),
        Channel::Alpha => Edit::Alpha(change.value),
    };
    commands.trigger(Pick {
        entity: strip.picker,
        edit,
        is_final: change.is_final,
    });
}

/// A part of the picker edited the colour: the picker keeps the hue, saturation and value, and the
/// row gets one [`ValueChange<Srgba>`] (a change that leaves the colour where it is says nothing,
/// unless it ends a drag or the colour was mixed).
fn picker_on_pick(
    pick: On<Pick>,
    mut pickers: Query<&mut ColorPicker>,
    rows: Query<(&ColorValue, Has<MixedValue>), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok(mut picker) = pickers.get_mut(pick.entity) else {
        return;
    };
    let Ok((value, mixed)) = rows.get(picker.field) else {
        return;
    };
    let mut alpha = value.0.alpha;
    let mut next = *picker;
    match pick.edit {
        Edit::Area([saturation, v]) => {
            next.saturation = saturation.clamp(0.0, 1.0);
            next.value = v.clamp(0.0, 1.0);
        }
        Edit::Hue(hue) => next.hue = hue.clamp(0.0, 360.0),
        Edit::Value(v) => next.value = v.clamp(0.0, 1.0),
        Edit::Alpha(a) => alpha = a.clamp(0.0, 1.0),
        Edit::Settle => {}
    }
    picker.set_if_neq(next);
    let [r, g, b] = hsv_to_rgb(next.hue, next.saturation, next.value);
    let to = Srgba::new(r, g, b, alpha);
    if to != value.0 || pick.is_final || mixed {
        commands.trigger(ValueChange {
            source: picker.field,
            value: to,
            is_final: pick.is_final,
        });
    }
}

/// What the area reads to turn a pointer position into a saturation and value.
type AreaGeometry = (
    &'static ColorArea,
    &'static ComputedNode,
    &'static UiGlobalTransform,
    &'static ComputedUiRenderTargetInfo,
);

/// The saturation and value under `position` (the pointer's, in the window's logical px), held to
/// the area's inside: the frame is not part of the plane, so the marker is under the pointer to the
/// pixel at both edges.
fn values_under(
    node: &ComputedNode,
    transform: &UiGlobalTransform,
    target: &ComputedUiRenderTargetInfo,
    ui_scale: f32,
    position: Vec2,
) -> Option<[f32; 2]> {
    let normalized =
        node.normalize_point(*transform, position * target.scale_factor() / ui_scale)?;
    let size = node.size();
    let border = node.border();
    let (origin, inside) = (border.min_inset, size - border.min_inset - border.max_inset);
    let offset = (normalized + Vec2::splat(0.5)) * size;
    Some(area_values(
        [offset.x, offset.y],
        [origin.x, origin.y],
        [inside.x, inside.y],
    ))
}

/// A press on the area sets the saturation and value under the pointer and begins a drag.
fn area_on_press(
    mut press: On<PointerPress>,
    areas: Query<AreaGeometry, Without<InteractionDisabled>>,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    let Ok((area, node, transform, target)) = areas.get(press.entity) else {
        return;
    };
    press.propagate(false);
    if let Some(at) = values_under(node, transform, target, ui_scale.0, press.pointer.position) {
        commands.entity(press.entity).insert(AreaGrab);
        commands.trigger(Pick {
            entity: area.picker,
            edit: Edit::Area(at),
            is_final: false,
        });
    }
}

/// A drag that began on the area follows the pointer, held to the area's edge.
fn area_on_drag(
    mut drag: On<PointerDrag>,
    areas: Query<AreaGeometry, With<AreaGrab>>,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    let Ok((area, node, transform, target)) = areas.get(drag.entity) else {
        return;
    };
    drag.propagate(false);
    if let Some(at) = values_under(node, transform, target, ui_scale.0, drag.pointer.position) {
        commands.trigger(Pick {
            entity: area.picker,
            edit: Edit::Area(at),
            is_final: false,
        });
    }
}

/// Lets go of the area: one final change, the colour as it stands, and the drag is over.
fn finish(
    area: Entity,
    grabs: &Query<&ColorArea, With<AreaGrab>>,
    commands: &mut Commands,
) -> bool {
    let Ok(grabbed) = grabs.get(area) else {
        return false;
    };
    commands.entity(area).remove::<AreaGrab>();
    commands.trigger(Pick {
        entity: grabbed.picker,
        edit: Edit::Settle,
        is_final: true,
    });
    true
}

fn area_on_release(
    mut release: On<PointerRelease>,
    grabs: Query<&ColorArea, With<AreaGrab>>,
    mut commands: Commands,
) {
    if finish(release.entity, &grabs, &mut commands) {
        release.propagate(false);
    }
}

fn area_on_drag_end(
    mut end: On<PointerDragEnd>,
    grabs: Query<&ColorArea, With<AreaGrab>>,
    mut commands: Commands,
) {
    if finish(end.entity, &grabs, &mut commands) {
        end.propagate(false);
    }
}

fn area_on_cancel(
    mut cancel: On<PointerCancel>,
    grabs: Query<&ColorArea, With<AreaGrab>>,
    mut commands: Commands,
) {
    if finish(cancel.entity, &grabs, &mut commands) {
        cancel.propagate(false);
    }
}

/// The arrows on the focused area step the saturation (Left, Right) and the value (Up, Down) by a
/// hundredth, Shift by a tenth, a held key repeating; each press is one final change. Tab leaves.
fn area_on_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    keys: Res<ButtonInput<KeyCode>>,
    areas: Query<&ColorArea, Without<InteractionDisabled>>,
    pickers: Query<&ColorPicker>,
    mut commands: Commands,
) {
    if input.input.state != ButtonState::Pressed {
        return;
    }
    let Ok(area) = areas.get(input.focused_entity) else {
        return;
    };
    let Ok(picker) = pickers.get(area.picker) else {
        return;
    };
    let (dx, dy) = match input.input.key_code {
        KeyCode::ArrowLeft => (-1.0, 0.0),
        KeyCode::ArrowRight => (1.0, 0.0),
        KeyCode::ArrowUp => (0.0, 1.0),
        KeyCode::ArrowDown => (0.0, -1.0),
        _ => return,
    };
    input.propagate(false);
    let by = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        AREA_KEY_STEP * 10.0
    } else {
        AREA_KEY_STEP
    };
    commands.trigger(Pick {
        entity: area.picker,
        edit: Edit::Area([picker.saturation + dx * by, picker.value + dy * by]),
        is_final: true,
    });
}

/// The D-pad's Left and Right on the area step the saturation, as they step a slider.
fn area_on_step(
    step: On<AreaStepBy>,
    areas: Query<&ColorArea, Without<InteractionDisabled>>,
    pickers: Query<&ColorPicker>,
    mut commands: Commands,
) {
    let Ok(area) = areas.get(step.entity) else {
        return;
    };
    let Ok(picker) = pickers.get(area.picker) else {
        return;
    };
    commands.trigger(Pick {
        entity: area.picker,
        edit: Edit::Area([picker.saturation + step.steps * AREA_PAD_STEP, picker.value]),
        is_final: true,
    });
}

/// What [`update_colors`] reads and writes on a colour row.
type Rows = (
    Entity,
    Ref<'static, ColorValue>,
    Has<MixedValue>,
    Option<Ref<'static, MixedValue>>,
    &'static mut TextInputValue,
    &'static mut TextHint,
);

/// A strip's slider and area marker the colour moved.
type Parts<'w, 's> = (
    Query<'w, 's, (&'static ColorArea, &'static mut BackgroundColor), Without<SwatchHalf>>,
    Query<'w, 's, (&'static AreaMarker, &'static mut Node)>,
    Query<'w, 's, (Entity, &'static ColorStrip, &'static SliderValue)>,
    Query<'w, 's, (&'static ColorRail, &'static mut BackgroundGradient)>,
);

/// Brings every part of a colour field up to its colour: a [`ColorValue`] or [`MixedValue`] that
/// changed writes the hex into the row's text (or nothing and the [`MIXED_HINT`], while mixed) and
/// the colour into the swatch; an open picker moves its area's marker and hue, its three strips and
/// their gradients to the colour, keeping the hue, saturation and value it was on where the colour
/// does not say them (a grey). Writes only what differs, so a colour nobody moved costs nothing.
/// Runs in [`crate::GooeySystems::Style`], before [`crate::update_text_inputs`].
pub fn update_colors(
    mut rows: Query<Rows, With<ColorValue>>,
    mut unmixed: RemovedComponents<MixedValue>,
    mut pickers: Query<(Entity, &mut ColorPicker)>,
    values: Query<Ref<ColorValue>>,
    mut halves: Query<(&SwatchHalf, &mut BackgroundColor), Without<ColorArea>>,
    (mut areas, mut markers, strips, mut rails): Parts,
    mut commands: Commands,
) {
    let unmixed: Vec<Entity> = unmixed.read().collect();
    let mut moved: Vec<(Entity, Srgba, bool)> = Vec::new();
    for (entity, value, mixed, mark, mut text, mut hint) in &mut rows {
        if !(value.is_changed() || mark.is_some_and(|m| m.is_added()) || unmixed.contains(&entity))
        {
            continue;
        }
        let shown = text_of(value.0, mixed);
        if text.0 != shown {
            text.0 = shown;
        }
        let words = if mixed { MIXED_HINT } else { "" };
        if hint.0 != words {
            hint.0 = words.to_string();
        }
        moved.push((entity, value.0, mixed));
    }
    if !moved.is_empty() {
        for (half, mut background) in &mut halves {
            let Some(&(_, colour, mixed)) = moved.iter().find(|(row, ..)| *row == half.row) else {
                continue;
            };
            let wanted = if mixed {
                Color::NONE
            } else if half.alpha {
                Color::Srgba(colour)
            } else {
                Color::Srgba(colour.with_alpha(1.0))
            };
            background.set_if_neq(BackgroundColor(wanted));
        }
    }
    // An open picker follows the colour. A colour the game or a typed hex set moves the hue,
    // saturation and value it is on; one the picker itself made is already what they say.
    let mut shown: Vec<(Entity, [f32; 3], Srgba)> = Vec::new();
    for (entity, mut picker) in &mut pickers {
        let Ok(value) = values.get(picker.field) else {
            continue;
        };
        if !(picker.is_added() || picker.is_changed() || value.is_changed()) {
            continue;
        }
        let rgb = [value.0.red, value.0.green, value.0.blue];
        let [h, s, v] = picker.hsv();
        let made = hsv_to_rgb(h, s, v);
        if (value.is_changed() || picker.is_added())
            && made.iter().zip(rgb).any(|(a, b)| (a - b).abs() > 1e-4)
        {
            let [h, s, v] = rgb_to_hsv(rgb, [h, s, v]);
            picker.set_if_neq(ColorPicker {
                hue: h,
                saturation: s,
                value: v,
                ..*picker
            });
        }
        shown.push((entity, picker.hsv(), value.0));
    }
    if shown.is_empty() {
        return;
    }
    let of = |picker: Entity| shown.iter().find(|(e, ..)| *e == picker);
    for (area, mut background) in &mut areas {
        if let Some(&(_, [h, ..], _)) = of(area.picker) {
            let [r, g, b] = hsv_to_rgb(h, 1.0, 1.0);
            background.set_if_neq(BackgroundColor(Color::srgb(r, g, b)));
        }
    }
    for (marker, mut node) in &mut markers {
        if let Some(&(_, [_, s, v], _)) = of(marker.picker) {
            let (left, top) = (Val::Percent(s * 100.0), Val::Percent((1.0 - v) * 100.0));
            if node.left != left {
                node.left = left;
            }
            if node.top != top {
                node.top = top;
            }
        }
    }
    for (entity, strip, value) in &strips {
        if let Some(&(_, [h, _, v], colour)) = of(strip.picker) {
            let wanted = match strip.channel {
                Channel::Hue => h,
                Channel::Value => v,
                Channel::Alpha => colour.alpha,
            };
            // `SliderValue` is immutable: a new one is inserted.
            if value.0 != wanted {
                commands.entity(entity).insert(SliderValue(wanted));
            }
        }
    }
    for (rail, mut gradient) in &mut rails {
        if let Some(&(_, hsv, colour)) = of(rail.picker)
            && rail.channel != Channel::Hue
        {
            let wanted = strip_ramp(rail.channel, hsv, [colour.red, colour.green, colour.blue]);
            gradient.set_if_neq(wanted);
        }
    }
}
