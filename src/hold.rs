//! `hold_button`: a button that confirms only when it is held, with the setting that turns the hold
//! into a tap.
//!
//! It is not a `bevy_ui_widgets::Button`: that sends `Activate` on a key's down edge and on a
//! pointer's click, so a game that observed `Activate` as for every other button would confirm a
//! delete on a tap. The widget sends no `Activate`; its one event is [`HoldConfirmed`]. Input is
//! read here: Enter and Space and the pad's South while the button has focus, and the primary
//! pointer button while the pointer is over it. Research: `docs/research/hold-button.md`.

use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;

use crate::feedback::{UiFeedback, UiFeedbackKind};
use crate::model::{hold_seconds, hold_step, meter_fraction};
use crate::motion::MotionSettings;
use crate::nav::show_focus;
use crate::skin::Role;

/// Makes a node a hold-to-confirm button: it confirms, with one [`HoldConfirmed`], only after it
/// has been held for `seconds` (times [`HoldSettings::duration_scale`]). Spawn it with
/// [`hold_button`](crate::hold_button); the component is public so a game can change the time.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[require(HoldProgress, HoldState)]
pub struct HoldToConfirm {
    /// How long a full hold takes, seconds, before the player's [`HoldSettings`] scale. Zero, or
    /// not a positive finite number, makes it a tap.
    pub seconds: f32,
}

/// How far a hold has come, `0.0` (nothing) to `1.0` (done): what the bar shows. The crate writes
/// it every frame a hold rises or unwinds; a game may read it, to ramp a sound or a rumble.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct HoldProgress(pub f32);

/// Which inputs hold a [`HoldToConfirm`] now, and whether it has confirmed for this hold. Internal.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct HoldState {
    pointer: bool,
    key: bool,
    pad: bool,
    /// The primary button was let go over the button this frame (a tap's up event).
    lifted: bool,
    /// The hold has confirmed and has not been let go yet: it confirms once.
    spent: bool,
}

impl HoldState {
    fn held(&self) -> bool {
        self.pointer || self.key || self.pad
    }
}

/// Sent on a hold button when it confirms: the hold ran its whole time, or, with
/// [`HoldSettings::tap`], the press was let go on the button. It is the only event a hold button
/// sends: it never sends `Activate`. An assistive technology's Click confirms at once, because a
/// screen reader cannot hold.
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq, Reflect)]
#[reflect(Event, Clone, Debug)]
pub struct HoldConfirmed {
    /// The hold button.
    #[event_target]
    pub entity: Entity,
}

/// How long the player is willing to hold: the one setting every [`HoldToConfirm`] reads (XAG 107:
/// "a single input press instead of an input hold"). A hold takes the game's seconds times
/// `duration_scale`; at 0 ([`HoldSettings::tap`]) it is a tap that confirms when it is let go. A
/// game wires this to an "Accessible interactions" or "Hold to confirm" option.
///
/// ```
/// # use bevy_gooey::HoldSettings;
/// let mut settings = HoldSettings::default();
/// assert_eq!(settings.duration_scale, 1.0);
/// settings = HoldSettings::tap();
/// assert!(settings.is_tap());
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct HoldSettings {
    /// The multiplier on every hold's seconds, held to 0 to 4 when read. Default: 1.
    pub duration_scale: f32,
}

impl Default for HoldSettings {
    fn default() -> Self {
        Self {
            duration_scale: 1.0,
        }
    }
}

impl HoldSettings {
    /// No hold: every hold button confirms when a press is let go on it.
    pub const fn tap() -> Self {
        Self {
            duration_scale: 0.0,
        }
    }

    /// Whether holds are taps (a scale that is zero or negative).
    pub fn is_tap(&self) -> bool {
        self.duration_scale <= 0.0
    }
}

/// A press of the primary button on the hold button starts a pointer hold. It stops here.
pub(crate) fn hold_on_press(
    mut press: On<PointerPress>,
    mut buttons: Query<(&mut HoldState, Has<InteractionDisabled>), With<HoldToConfirm>>,
) {
    press.propagate(false);
    if press.button == PointerButton::Primary
        && let Ok((mut state, disabled)) = buttons.get_mut(press.entity)
        && !disabled
        && !state.pointer
    {
        state.pointer = true;
    }
}

/// The primary button let go over the hold button: the pointer's up event, which is what a tap
/// confirms on.
pub(crate) fn hold_on_release(
    mut release: On<PointerRelease>,
    mut buttons: Query<&mut HoldState, With<HoldToConfirm>>,
) {
    release.propagate(false);
    if release.button == PointerButton::Primary
        && let Ok(mut state) = buttons.get_mut(release.entity)
        && state.pointer
    {
        state.pointer = false;
        state.lifted = true;
    }
}

/// A drag that ends elsewhere, or a cancelled pointer, lets the hold go without confirming a tap.
pub(crate) fn hold_on_drag_end(
    mut end: On<PointerDragEnd>,
    mut buttons: Query<&mut HoldState, With<HoldToConfirm>>,
) {
    end.propagate(false);
    if let Ok(mut state) = buttons.get_mut(end.entity)
        && state.pointer
    {
        state.pointer = false;
    }
}

/// See [`hold_on_drag_end`].
pub(crate) fn hold_on_cancel(
    mut cancel: On<PointerCancel>,
    mut buttons: Query<&mut HoldState, With<HoldToConfirm>>,
) {
    cancel.propagate(false);
    if let Ok(mut state) = buttons.get_mut(cancel.entity)
        && state.pointer
    {
        state.pointer = false;
    }
}

/// Everything [`hold_by_input`] reads of a hold button and writes back.
type HoldItem = (
    Entity,
    &'static HoldToConfirm,
    &'static mut HoldProgress,
    &'static mut HoldState,
    Has<InteractionDisabled>,
    Option<&'static Hovered>,
);

/// Runs every hold: starts and ends the key, pad and pointer holds, rises or unwinds the progress
/// on real time (a paused game's menu still holds), and sends [`HoldConfirmed`]. Runs in
/// [`crate::GooeySystems::Navigate`] after [`crate::gamepad_select`].
///
/// A hold needs a fresh press: Enter still down from the press that opened the screen does not
/// start one. A pointer hold ends, without confirming, when the pointer leaves the button; a key
/// or pad hold when focus leaves it or the button is disabled.
#[allow(clippy::too_many_arguments)]
pub fn hold_by_input(
    time: Option<Res<Time<Real>>>,
    settings: Option<Res<HoldSettings>>,
    motion: Option<Res<MotionSettings>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    gamepads: Query<&Gamepad>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    mut buttons: Query<HoldItem, With<HoldToConfirm>>,
    mut commands: Commands,
) {
    let dt = time.map_or(0.0, |time| time.delta_secs());
    let scale = settings.map_or(1.0, |settings| settings.duration_scale);
    let instant = motion.is_some_and(|motion| motion.is_off());
    let key_down = keys
        .as_ref()
        .is_some_and(|keys| keys.pressed(KeyCode::Enter) || keys.pressed(KeyCode::Space));
    let key_new = keys
        .as_ref()
        .is_some_and(|keys| keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space));
    let pad_down = gamepads.iter().any(|pad| pad.pressed(GamepadButton::South));
    let pad_new = gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::South));
    for (entity, hold, mut progress, mut state, disabled, hovered) in &mut buttons {
        let seconds = hold_seconds(hold.seconds, scale);
        let tap = seconds <= 0.0;
        let focused = focus.get() == Some(entity) && !disabled;
        let mut next = *state;
        let mut clean = next.lifted && !disabled;
        next.lifted = false;
        next.key = focused && key_down && (state.key || key_new);
        next.pad = focused && pad_down && (state.pad || pad_new);
        // Letting go of a key or the pad while the button still has focus is the up event.
        clean |= (state.key && !next.key && focused && !key_down)
            || (state.pad && !next.pad && focused && !pad_down);
        if disabled || (state.pointer && !hovered.is_some_and(|hovered| hovered.get())) {
            next.pointer = false;
        }
        if focused && (key_new || pad_new) && (next.key || next.pad) {
            show_focus(&mut shown);
        }
        let rose = hold_step(progress.0, dt, seconds, next.held(), instant);
        if !next.held() {
            // Every source is up: the next hold may confirm again.
            if tap && clean && !next.spent {
                commands.trigger(HoldConfirmed { entity });
            }
            next.spent = false;
        } else if !tap && !next.spent && rose >= 1.0 {
            commands.trigger(HoldConfirmed { entity });
            next.spent = true;
        }
        if progress.0 != rose {
            progress.0 = rose;
        }
        if *state != next {
            *state = next;
        }
    }
}

/// Sets each hold button's fill to its progress. Runs in [`crate::GooeySystems::Style`].
pub fn update_hold_bars(
    buttons: Query<(&HoldProgress, &Children), Changed<HoldProgress>>,
    parts: Query<(&Role, &Children)>,
    mut nodes: Query<(&Role, &mut Node)>,
) {
    for (progress, children) in &buttons {
        let width = Val::Percent(meter_fraction(progress.0, 1.0) * 100.0);
        for (_, track) in children
            .iter()
            .filter_map(|child| parts.get(child).ok())
            .filter(|(role, _)| **role == Role::HoldTrack)
        {
            for fill in track.iter() {
                if let Ok((Role::HoldFill, mut node)) = nodes.get_mut(fill)
                    && node.width != width
                {
                    node.width = width;
                }
            }
        }
    }
}

/// Writes the button's activate feedback when a hold confirms: the sound of a button, at the act.
pub(crate) fn feedback_on_hold(event: On<HoldConfirmed>, mut out: MessageWriter<UiFeedback>) {
    out.write(UiFeedback {
        entity: event.entity,
        kind: UiFeedbackKind::Activate,
    });
}
