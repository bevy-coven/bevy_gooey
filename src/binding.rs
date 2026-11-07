//! The binding row: what keeps a [`crate::binding`] row's state and listens for the next press.
//! Its look is [`crate::apply_skin`]'s, like every widget's.

use bevy::input::ButtonState;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input::mouse::MouseButton;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ValueChange};

use crate::drag::DragState;
use crate::feedback::{UiFeedback, UiFeedbackKind};
use crate::model::{ChordGlyphs, PadStyle, is_modifier_key, key_name, mouse_name, pad_name};
use crate::nav::show_focus;
use crate::prompt::InputDevice;
use crate::shortcut::{Modifiers, Platform, Shortcut};
use crate::skin::Role;

/// The `GlobalZIndex` of the catcher a capturing row puts over the interface: above a dialog (100),
/// a toast (150) and a tooltip (200).
const CATCHER_Z: i32 = 1000;

/// A physical input a player can bind an action to: what a [`crate::binding`] row shows, waits for
/// and reports. A key is its *place* on the keyboard (Bevy's [`KeyCode`], so WASD is the same
/// four keys on a QWERTY and an AZERTY keyboard), which is what a game's own input code reads
/// from `ButtonInput<KeyCode>`.
///
/// It is `Eq + Hash`, so a game can find two actions on one input, and with the `serde` feature it
/// is (de)serialisable, so a game saves its bindings as data.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// let jump = Binding::Key(KeyCode::Space);
/// assert_eq!(jump.label(PadStyle::Xbox), "Space");
/// assert_eq!(Binding::Pad(GamepadButton::South).label(PadStyle::PlayStation), "Cross");
/// ```
/// A `Chord` is a keyboard key with modifiers held ("Ctrl+Shift+K"), what a [`crate::binding`]
/// row whose value is a `Chord` captures: pass one as the row's first value and the row waits for
/// a chord, as a `Pad` value makes it wait for a pad button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Binding {
    /// A keyboard key, by its place on the keyboard.
    Key(KeyCode),
    /// A mouse button.
    Mouse(MouseButton),
    /// A gamepad button, by its place on the pad (`South` is the bottom button).
    Pad(GamepadButton),
    /// A key with modifiers held, by their places on the keyboard. A chord with no modifier is
    /// a plain key, which a chord row can bind too ("F5").
    Chord(Shortcut),
}

impl Binding {
    /// The words on the keycap of a [`crate::binding`] row: "W", "Left Shift", "Right Click", "A".
    /// A game shows the same words in its hints and tutorials, so a remapped control is named the
    /// same wherever the player sees it. `style` only matters for a pad button.
    pub fn label(&self, style: PadStyle) -> String {
        match self {
            Self::Key(code) => key_name(&format!("{code:?}")),
            Self::Mouse(button) => mouse_name(&format!("{button:?}")),
            Self::Pad(button) => pad_name(&format!("{button:?}"), style),
            Self::Chord(chord) => chord.label(),
        }
    }

    /// The device this input is pressed on: the keyboard for a key, a chord or a mouse button, a
    /// pad for a pad button. What a [`crate::binding`] row listens to, and which of a
    /// [`crate::Prompt`]'s inputs is the player's now.
    pub fn device(&self) -> InputDevice {
        match self {
            Self::Key(_) | Self::Mouse(_) | Self::Chord(_) => InputDevice::Keyboard,
            Self::Pad(_) => InputDevice::Pad,
        }
    }
}

/// What a [`crate::binding`] row has bound: the widget's state. Read it to know the control, or set
/// it to change the row from code (the keycap follows in the same frame, and no [`ValueChange`] is
/// sent for a change the game made itself): a game that loads the player's bindings spawns its rows
/// with them, and one that refuses a clash puts the old value back.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn apply(rows: Query<&BindingValue, Changed<BindingValue>>) {
///     for bound in &rows {
///         info!("now bound to {:?}", bound.0);
///     }
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct BindingValue(pub Binding);

/// On a [`crate::binding`] row while it waits for the player's next press. While any row has it
/// the crate's own input systems stand still (see [`not_capturing`]) and nothing else can be
/// pressed with the pointer, so a game that reads raw keys of its own gates them on this too:
/// `Query<(), With<Capturing>>` is empty when none waits. Handle Back through [`crate::Cancel`]:
/// it is never sent for the press that exits a capture.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct Capturing;

/// A capture that has just begun: the press that started it, however it arrived, is not heard.
/// Removed by [`bind_by_input`] on its first run.
#[derive(Component)]
pub struct Armed;

/// The transparent node over the whole window while a row waits: the pointer lands on it, so a
/// click binds the mouse button and cannot press the button under it.
#[derive(Component)]
pub struct Catcher(Entity);

/// What [`crate::binding`] says and which keys end it. The crate has no strings of its own, so the
/// prompts are the host's to translate; `{exit}` in a prompt is replaced by the name of the input
/// that exits ([`BindingSettings::cancel_key`] on a keyboard row, [`BindingSettings::cancel_button`]
/// on a pad row).
///
/// ```
/// # use bevy_gooey::*;
/// let settings = BindingSettings {
///     pad_style: PadStyle::PlayStation,
///     ..Default::default()
/// };
/// assert_eq!(settings.pad_prompt(), "Press a button, Options exits");
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct BindingSettings {
    /// What a keyboard row's keycap says while it waits. Default: "Press a key, {exit} exits".
    pub key_prompt: String,
    /// What a pad row's keycap says while it waits. Default: "Press a button, {exit} exits".
    pub pad_prompt: String,
    /// What a chord row's keycap says while it waits. Default: "Press a shortcut, {exit} exits".
    pub chord_prompt: String,
    /// Whose names a pad button gets, "A" or "Cross" or "B". Default: Xbox.
    #[reflect(ignore)]
    pub pad_style: PadStyle,
    /// The key that exits a capture without binding, so it cannot be bound: XAG 112's "Press
    /// \[ESC\] to exit remapping mode". A game that must make Escape bindable (XAG 107 asks that
    /// the Esc key be remappable) moves the exit to another key. Default: Escape.
    pub cancel_key: KeyCode,
    /// The pad button that exits a capture, on any gamepad. Default: Start.
    pub cancel_button: GamepadButton,
    /// How a chord spells its modifiers and named keys: Apple's symbols (`⇧⌘P`) or ASCII words
    /// (`Shift+Cmd+P`). Default: [`ChordGlyphs::Glyphs`]. Set
    /// [`ChordGlyphs::Words`](crate::ChordGlyphs::Words) when the host's font has no symbols —
    /// Bevy's own bundled face maps none of a mac chord's thirteen, so a mac build reading this
    /// is showing boxes. On anything but a mac the two spellings agree.
    #[reflect(ignore)]
    pub chord_glyphs: ChordGlyphs,
    /// The platform every chord on screen is spelled for. `None` (the default) is the one this
    /// build runs on ([`Platform::current`]); a host that documents or previews another platform's
    /// keymap names it.
    #[reflect(ignore)]
    pub platform: Option<Platform>,
}

impl Default for BindingSettings {
    fn default() -> Self {
        Self {
            key_prompt: "Press a key, {exit} exits".to_owned(),
            pad_prompt: "Press a button, {exit} exits".to_owned(),
            chord_prompt: "Press a shortcut, {exit} exits".to_owned(),
            pad_style: PadStyle::default(),
            cancel_key: KeyCode::Escape,
            cancel_button: GamepadButton::Start,
            chord_glyphs: ChordGlyphs::default(),
            platform: None,
        }
    }
}

impl BindingSettings {
    /// The platform every chord on screen is spelled for: [`BindingSettings::platform`], or this
    /// build's.
    pub fn platform(&self) -> Platform {
        self.platform.unwrap_or_else(Platform::current)
    }

    /// The words a chord wears on this host: [`Shortcut::label_with`], in
    /// [`BindingSettings::chord_glyphs`] and [`BindingSettings::platform`]. Every place the crate
    /// draws a chord reads this, so one setting reaches a tooltip, a menu row, a prompt, a badge,
    /// a legend and the palette.
    ///
    /// ```
    /// # use bevy_gooey::*;
    /// let settings = BindingSettings {
    ///     platform: Some(Platform::Mac),
    ///     chord_glyphs: ChordGlyphs::Words,
    ///     ..Default::default()
    /// };
    /// assert_eq!(settings.chord(Shortcut::primary(KeyCode::KeyS)), "Cmd+S");
    /// ```
    pub fn chord(&self, shortcut: Shortcut) -> String {
        shortcut.label_with(self.platform(), self.chord_glyphs)
    }

    /// The words an input wears on this host: [`Binding::label`] for a key, a mouse button or a
    /// pad button (its own [`BindingSettings::pad_style`]), [`BindingSettings::chord`] for a
    /// chord.
    pub fn input_label(&self, binding: Binding) -> String {
        match binding {
            Binding::Chord(chord) => self.chord(chord),
            other => other.label(self.pad_style),
        }
    }

    /// What a keyboard row says while it waits, with `{exit}` filled in.
    pub fn key_prompt(&self) -> String {
        let exit = self.input_label(Binding::Key(self.cancel_key));
        self.key_prompt.replace("{exit}", &exit)
    }

    /// What a pad row says while it waits, with `{exit}` filled in.
    pub fn pad_prompt(&self) -> String {
        let exit = self.input_label(Binding::Pad(self.cancel_button));
        self.pad_prompt.replace("{exit}", &exit)
    }

    /// What a chord row says while it waits, with `{exit}` filled in.
    pub fn chord_prompt(&self) -> String {
        let exit = self.input_label(Binding::Key(self.cancel_key));
        self.chord_prompt.replace("{exit}", &exit)
    }

    fn prompt(&self, binding: &Binding) -> String {
        match binding {
            Binding::Chord(_) => self.chord_prompt(),
            other => match other.device() {
                InputDevice::Keyboard => self.key_prompt(),
                InputDevice::Pad => self.pad_prompt(),
            },
        }
    }
}

/// Run condition: no [`crate::binding`] row waits for a press and no payload is carried from the
/// keyboard or a pad ([`crate::DragSettings`]). The crate's systems that read keys and pad buttons
/// ([`crate::navigate_by_input`], [`crate::gamepad_select`], [`crate::gamepad_adjust`],
/// [`crate::cancel_by_input`], [`crate::page_lists`], [`crate::step_tabs`]) run under it, so the
/// arrow or button that is the new binding, or that steers a carried payload, never also moves
/// focus. A host that registers those systems itself adds it to them.
pub fn not_capturing(waiting: Query<(), With<Capturing>>, drag: Option<Res<DragState>>) -> bool {
    waiting.is_empty() && !drag.is_some_and(|drag| drag.carried_by_key())
}

/// Run condition: no [`crate::binding`] row waits for a press. [`not_capturing`] without the
/// carried payload, for the systems that are what carries it.
pub(crate) fn no_row_waits(waiting: Query<(), With<Capturing>>) -> bool {
    waiting.is_empty()
}

/// Keeps the state: a [`ValueChange<Binding>`] on a row becomes its [`BindingValue`]. The row's own
/// observer, like [`crate::choice`]'s.
pub(crate) fn binding_self_update(
    change: On<ValueChange<Binding>>,
    mut rows: Query<&mut BindingValue>,
) {
    if let Ok(mut value) = rows.get_mut(change.source) {
        value.set_if_neq(BindingValue(change.value));
    }
}

/// Enter, Space, the pad's South button and a click (Bevy's `Button` makes each an [`Activate`])
/// start a capture: the row is armed, and listens from the next frame. An activation while it
/// waits is the press being bound, not a second start.
pub(crate) fn binding_on_activate(
    event: On<Activate>,
    rows: Query<Has<Capturing>, (With<BindingValue>, Without<InteractionDisabled>)>,
    mut out: MessageWriter<UiFeedback>,
    mut commands: Commands,
) {
    let Ok(waiting) = rows.get(event.entity) else {
        return;
    };
    if waiting {
        return;
    }
    commands.entity(event.entity).insert((Capturing, Armed));
    out.write(UiFeedback {
        entity: event.entity,
        kind: UiFeedbackKind::Activate,
    });
}

/// While a row waits, every key pressed on it stops here, so Tab is not Tab navigation, which Bevy
/// observes on the window at the far end of this event's path, and nothing above the row sees it.
/// The key itself is read in [`bind_by_input`].
pub(crate) fn binding_on_key_input(
    mut input: On<FocusedInput<KeyboardInput>>,
    waiting: Query<(), With<Capturing>>,
) {
    if input.input.state == ButtonState::Pressed && waiting.contains(input.focused_entity) {
        input.propagate(false);
    }
}

/// What a waiting row heard this frame.
enum Heard {
    Nothing,
    /// The player left the capture: the exit key or button, or the other device.
    Exit,
    Bind(Binding),
}

/// A row that waits for a key or a mouse button: the exit key leaves, any other key and any mouse
/// button binds (the primary one too: the catcher keeps it from also pressing something), and a
/// pad button means the player picked up the wrong device, so it leaves.
fn heard_by_keyboard_row(
    settings: &BindingSettings,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    gamepads: &Query<&Gamepad>,
) -> Heard {
    if keys.just_pressed(settings.cancel_key) {
        return Heard::Exit;
    }
    if let Some(&key) = keys.get_just_pressed().min() {
        return Heard::Bind(Binding::Key(key));
    }
    if let Some(&button) = mouse.get_just_pressed().next() {
        return Heard::Bind(Binding::Mouse(button));
    }
    if gamepads
        .iter()
        .any(|pad| pad.get_just_pressed().next().is_some())
    {
        return Heard::Exit;
    }
    Heard::Nothing
}

/// A row that waits for a chord: the exit key leaves, the first key that is not a modifier binds
/// with the modifiers held at that moment (a modifier pressed alone is not a shortcut, so the row
/// keeps waiting through it), and a mouse button or a pad button means the input is not a
/// shortcut, so it leaves.
fn heard_by_chord_row(
    settings: &BindingSettings,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    gamepads: &Query<&Gamepad>,
) -> Heard {
    if keys.just_pressed(settings.cancel_key) {
        return Heard::Exit;
    }
    let chord = keys
        .get_just_pressed()
        .filter(|key| !is_modifier_key(&format!("{key:?}")))
        .min();
    if let Some(&key) = chord {
        return Heard::Bind(Binding::Chord(Shortcut::new(Modifiers::held(keys), key)));
    }
    if mouse.get_just_pressed().next().is_some()
        || gamepads
            .iter()
            .any(|pad| pad.get_just_pressed().next().is_some())
    {
        return Heard::Exit;
    }
    Heard::Nothing
}

/// A row that waits for a pad button: the exit button leaves, any other binds, and a key or a
/// mouse press leaves (the player is at the keyboard, not the pad).
fn heard_by_pad_row(
    settings: &BindingSettings,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    gamepads: &Query<&Gamepad>,
) -> Heard {
    if gamepads
        .iter()
        .any(|pad| pad.just_pressed(settings.cancel_button))
    {
        return Heard::Exit;
    }
    if let Some(button) = gamepads
        .iter()
        .flat_map(|pad| pad.get_just_pressed().copied())
        .min()
    {
        return Heard::Bind(Binding::Pad(button));
    }
    if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() {
        return Heard::Exit;
    }
    Heard::Nothing
}

/// Listens for the press a waiting [`crate::binding`] row is waiting for. Runs last in
/// [`crate::GooeySystems::Navigate`], after every system that reads the same keys and was held
/// back (they run under [`not_capturing`] and so did not see this press).
///
/// A row armed this frame only becomes listening: the press that started it is over. After that
/// the first press ends the capture. A bind sends one [`ValueChange<Binding>`] (none when it is
/// the input the row already holds), and either way the row is focused again, with the ring shown
/// for a key or a pad button, because a pointer press on the catcher cleared focus. A row whose
/// focus was taken away by the game is left without a binding.
#[allow(clippy::too_many_arguments)]
pub fn bind_by_input(
    settings: Res<BindingSettings>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    gamepads: Query<&Gamepad>,
    mut focus: ResMut<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    rows: Query<(Entity, &BindingValue, Has<Armed>), With<Capturing>>,
    mut out: MessageWriter<UiFeedback>,
    mut commands: Commands,
) {
    for (row, held, armed) in &rows {
        if armed {
            commands.entity(row).remove::<Armed>();
            continue;
        }
        let heard = match held.0 {
            Binding::Chord(_) => heard_by_chord_row(&settings, &keys, &mouse, &gamepads),
            other => match other.device() {
                InputDevice::Keyboard => heard_by_keyboard_row(&settings, &keys, &mouse, &gamepads),
                InputDevice::Pad => heard_by_pad_row(&settings, &keys, &mouse, &gamepads),
            },
        };
        match heard {
            Heard::Nothing => {
                // Focus taken elsewhere by the game, with nothing pressed: the capture is over.
                if focus.get() != Some(row) {
                    commands.entity(row).remove::<Capturing>();
                }
                continue;
            }
            Heard::Exit => {}
            Heard::Bind(binding) => {
                if binding != held.0 {
                    commands.trigger(ValueChange {
                        source: row,
                        value: binding,
                        is_final: true,
                    });
                    out.write(UiFeedback {
                        entity: row,
                        kind: UiFeedbackKind::Activate,
                    });
                }
            }
        }
        commands.entity(row).remove::<Capturing>();
        // A press on the pointer's catcher cleared focus, and the row takes it back. Only that:
        // focus a key or a button moved is not put back, or a leak would not show.
        if mouse.get_just_pressed().next().is_some() && focus.get() != Some(row) {
            focus.set(row, FocusCause::Navigated);
        }
        // A key or a pad button shows the ring; a click leaves a mouse player's focus unseen.
        let by_keys = keys.get_just_pressed().next().is_some()
            || gamepads
                .iter()
                .any(|pad| pad.get_just_pressed().next().is_some());
        if by_keys {
            show_focus(&mut shown);
        }
    }
}

/// Puts the catcher over the window while a row waits and takes it away when none does (or the
/// row is gone). A node above every widget takes the pointer, so a click binds the mouse button
/// (XAG 112: "Any mouse input will be applied as the new control") and cannot press anything. At
/// rest the two queries are empty.
pub fn catch_pointer(
    waiting: Query<Entity, With<Capturing>>,
    catchers: Query<(Entity, &Catcher)>,
    mut commands: Commands,
) {
    for (catcher, Catcher(row)) in &catchers {
        if !waiting.contains(*row) {
            commands.entity(catcher).despawn();
        }
    }
    for row in &waiting {
        if !catchers.iter().any(|(_, Catcher(held))| *held == row) {
            commands.spawn((
                Name::new("Binding catcher"),
                Catcher(row),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                GlobalZIndex(CATCHER_Z),
                Pickable::default(),
            ));
        }
    }
}

/// A row whose keycap needs writing: its value changed or a capture began.
type Written = Or<(Changed<BindingValue>, Added<Capturing>)>;

/// Writes each changed row's keycap: the bound input's name when it rests, the prompt while it
/// waits (the other is hidden, which keeps its room, so the keycap is as wide as the wider and
/// waiting moves nothing). Runs in [`crate::GooeySystems::Style`], before [`crate::apply_skin`],
/// the same frame a value changes, a capture begins or ends, or [`BindingSettings`] does.
pub fn update_bindings(
    settings: Res<BindingSettings>,
    rows: Query<(Entity, &BindingValue, Has<Capturing>)>,
    changed: Query<Entity, (With<BindingValue>, Written)>,
    mut ended: RemovedComponents<Capturing>,
    children: Query<&Children>,
    mut parts: Query<(&Role, &mut Text, &mut Visibility)>,
) {
    let mut dirty: Vec<Entity> = if settings.is_changed() {
        rows.iter().map(|(row, ..)| row).collect()
    } else {
        changed.iter().collect()
    };
    dirty.extend(ended.read());
    dirty.sort_unstable();
    dirty.dedup();
    for row in dirty {
        let Ok((_, value, waiting)) = rows.get(row) else {
            continue;
        };
        let name = settings.input_label(value.0);
        let prompt = settings.prompt(&value.0);
        for part in children.iter_descendants(row) {
            let Ok((role, mut text, mut visibility)) = parts.get_mut(part) else {
                continue;
            };
            let (words, shown) = match role {
                Role::BindingText => (&name, !waiting),
                Role::BindingPrompt => (&prompt, waiting),
                _ => continue,
            };
            if text.0 != *words {
                text.0.clone_from(words);
            }
            visibility.set_if_neq(if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
    }
}
