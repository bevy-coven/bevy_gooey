//! Prompts: the hints that say which input does what ("Esc Back"), in the words of the device the
//! player used last. Their look is [`crate::apply_skin`]'s, like every widget's.

use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadAxisChangedEvent, GamepadButton, GamepadButtonStateChangedEvent,
};
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;

use crate::binding::{Binding, BindingSettings};
use crate::model::STICK_PUSH;
use crate::skin::Role;

/// Which family of device the player used last: what [`crate::prompt`] shows the words of, and
/// what a game reads to swap glyphs of its own. [`track_input_device`] keeps it: a key or a mouse
/// button pressed makes it [`InputDevice::Keyboard`], a pad button pressed or a stick pushed
/// [`InputDevice::Pad`]. It starts as `Keyboard`; a game that knows better (a console, a handheld)
/// writes the resource at start-up.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn glyphs(device: Res<InputDevice>) {
///     if device.is_changed() {
///         info!("the player picked up {:?}", *device);
///     }
/// }
/// ```
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Hash, Clone)]
pub enum InputDevice {
    /// The keyboard and the mouse: a key or a mouse button was pressed.
    #[default]
    Keyboard,
    /// A gamepad: a button was pressed or a stick pushed.
    Pad,
}

/// One action's two inputs: what a [`crate::prompt`] shows, the keyboard's when the player last
/// used the keyboard or the mouse and the pad's when they used a pad. It is the prompt's state: set
/// it (a game that rebinds the action does, from the `ValueChange<Binding>` of a [`crate::binding`]
/// row) and the keycap follows in the same frame.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// assert_eq!(Prompt::BACK.keyboard, Binding::Key(KeyCode::Escape));
/// let inventory = Prompt::new(Binding::Key(KeyCode::KeyI), Binding::Pad(GamepadButton::North));
/// assert_eq!(inventory.pad.label(PadStyle::PlayStation), "Triangle");
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct Prompt {
    /// The input shown while the player uses the keyboard or the mouse.
    pub keyboard: Binding,
    /// The input shown while the player uses a pad.
    pub pad: Binding,
}

impl Prompt {
    /// What selects: Enter, and the pad's South button ("A" on an Xbox pad). Space selects too (it
    /// is Bevy's `Button`); the prompt names one.
    pub const SELECT: Self = Self::new(
        Binding::Key(KeyCode::Enter),
        Binding::Pad(GamepadButton::South),
    );

    /// What backs out, and sends a [`crate::Cancel`]: Escape, and the pad's East button ("B" on an
    /// Xbox pad).
    pub const BACK: Self = Self::new(
        Binding::Key(KeyCode::Escape),
        Binding::Pad(GamepadButton::East),
    );

    /// A prompt for an action of the game's own: its keyboard or mouse input, and its pad button.
    pub const fn new(keyboard: Binding, pad: Binding) -> Self {
        Self { keyboard, pad }
    }

    /// The input shown for `device`.
    pub fn binding(&self, device: InputDevice) -> Binding {
        match device {
            InputDevice::Keyboard => self.keyboard,
            InputDevice::Pad => self.pad,
        }
    }
}

/// Follows what the player is using: a key or a mouse button pressed, a pad button pressed or a
/// stick pushed past [`STICK_PUSH`]. Mouse motion alone does not switch, so a bump of the mouse
/// does not swap a pad player's hints; a click does. A pad press in the same frame as a key wins.
/// Runs first in [`crate::GooeySystems::Navigate`]; it reads four message queues, empty in a frame
/// with no input, and writes [`InputDevice`] only when it changes.
pub fn track_input_device(
    mut keys: MessageReader<KeyboardInput>,
    mut clicks: MessageReader<MouseButtonInput>,
    mut buttons: MessageReader<GamepadButtonStateChangedEvent>,
    mut sticks: MessageReader<GamepadAxisChangedEvent>,
    mut device: ResMut<InputDevice>,
) {
    // Every queue is read to its end, so no message is left for the next frame to find.
    let by_keyboard = keys
        .read()
        .filter(|key| key.state == ButtonState::Pressed)
        .count()
        + clicks
            .read()
            .filter(|click| click.state == ButtonState::Pressed)
            .count();
    let by_pad = buttons
        .read()
        .filter(|button| button.state == ButtonState::Pressed)
        .count()
        + sticks
            .read()
            .filter(|stick| stick.value.abs() >= STICK_PUSH)
            .count();
    if by_pad > 0 {
        device.set_if_neq(InputDevice::Pad);
    } else if by_keyboard > 0 {
        device.set_if_neq(InputDevice::Keyboard);
    }
}

/// Where a [`crate::prompts_at`] legend stands in its parent: a corner or the middle of an edge, 5%
/// in from the edges it touches (the safe area of a television, the dialog scrim's own margin). The
/// default is the bottom-left corner, where Xbox's guideline 112 puts "select" and "back"; what
/// that guideline asks is that they stay in one place on every screen, so a game that needs the
/// bottom-left for its own HUD picks another anchor once and spawns every legend, a dialog's too,
/// with it. Set it on a live legend (a component) and the legend moves in the same frame.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     screen(),
///     children![(
///         prompts_at(PromptsAnchor::BottomRight),
///         children![prompt("Select", Prompt::SELECT), prompt("Back", Prompt::BACK)],
///     )],
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Hash, Clone)]
pub enum PromptsAnchor {
    /// The bottom-left corner.
    #[default]
    BottomLeft,
    /// The middle of the bottom edge.
    BottomCenter,
    /// The bottom-right corner.
    BottomRight,
    /// The top-left corner.
    TopLeft,
    /// The middle of the top edge.
    TopCenter,
    /// The top-right corner.
    TopRight,
}

impl PromptsAnchor {
    /// The legend stands on the bottom edge.
    pub const fn at_bottom(self) -> bool {
        matches!(
            self,
            Self::BottomLeft | Self::BottomCenter | Self::BottomRight
        )
    }

    /// The legend stands in the middle of its edge, not in a corner.
    pub const fn is_centred(self) -> bool {
        matches!(self, Self::BottomCenter | Self::TopCenter)
    }

    /// The legend stands at the right of its parent.
    pub const fn at_right(self) -> bool {
        matches!(self, Self::BottomRight | Self::TopRight)
    }

    /// The translation that centres a legend on its `left: 50%`: half of its own width back.
    fn translation(self) -> UiTransform {
        if self.is_centred() {
            UiTransform::from_translation(Val2::percent(-50.0, 0.0))
        } else {
            UiTransform::IDENTITY
        }
    }
}

/// Centres a [`PromptsAnchor::BottomCenter`] or `TopCenter` legend: `apply_skin` writes its `left`
/// at 50% and this writes the constant translation of half its own width back, which is the only
/// way to centre an absolute node that is its content's width (a `left` and a `right` together
/// would stretch it). Runs in [`crate::GooeySystems::Style`], before [`crate::apply_skin`], in the
/// frame a legend or its anchor is added or changed, and writes only a difference.
pub fn place_prompts(
    mut legends: Query<(&PromptsAnchor, &mut UiTransform), Changed<PromptsAnchor>>,
) {
    for (anchor, mut transform) in &mut legends {
        let wanted = anchor.translation();
        if *transform != wanted {
            *transform = wanted;
        }
    }
}

/// Writes each prompt's keycap: the name of the input for the device the player is using, and the
/// keycap's shape ([`Role::PromptKey`] for a key, [`Role::PromptButton`] for a pad button). Runs in
/// [`crate::GooeySystems::Style`], before [`crate::apply_skin`], in the frame a [`Prompt`],
/// [`InputDevice`] or [`BindingSettings`] (whose `pad_style` names the buttons) changes; at rest
/// it reads only those flags.
pub fn update_prompts(
    device: Res<InputDevice>,
    settings: Res<BindingSettings>,
    prompts: Query<(Entity, &Prompt)>,
    changed: Query<Entity, Changed<Prompt>>,
    children: Query<&Children>,
    mut keycaps: Query<(&mut Role, &mut Text)>,
) {
    let dirty: Vec<Entity> = if device.is_changed() || settings.is_changed() {
        prompts.iter().map(|(entity, _)| entity).collect()
    } else {
        changed.iter().collect()
    };
    for entity in dirty {
        let (Ok((_, keys)), Ok(parts)) = (prompts.get(entity), children.get(entity)) else {
            continue;
        };
        let input = keys.binding(*device);
        let shape = match input {
            Binding::Pad(_) => Role::PromptButton,
            Binding::Key(_) | Binding::Mouse(_) | Binding::Chord(_) => Role::PromptKey,
        };
        let name = settings.input_label(input);
        for &part in parts {
            let Ok((mut role, mut text)) = keycaps.get_mut(part) else {
                continue;
            };
            if !matches!(*role, Role::PromptKey | Role::PromptButton) {
                continue;
            }
            role.set_if_neq(shape);
            if text.0 != name {
                text.0.clone_from(&name);
            }
        }
    }
}

/// Centres a keycap's words in the box its `min_width` gives it.
///
/// A keycap is a `Text` node whose `min_width` is a square (see `apply_skin`), and a `Text` node
/// lays its glyphs out from its content box's left edge. A single character is narrower than that
/// square, so without this the words sit hard against the left padding and every pixel of the
/// slack lands on the right: the letter reads left-aligned in a too-wide cap.
///
/// [`Justify::Center`] cannot help. It centres a run inside the width the run was *broken* to, and
/// a cap is [`LineBreak::NoWrap`] so that a name with a space in it ("Left Click", "Num Enter")
/// stays on one line: `NoWrap` asks Bevy for `TextBounds::UNBOUNDED`, which leaves the alignment
/// nothing to centre within. Centring the node's content instead (`JustifyContent::Center`) is a
/// no-op on a node with no children, and letting the text wrap so that alignment has a width
/// breaks a two-word cap onto two lines.
///
/// So the box is made to fit its words: the horizontal padding is widened to whatever leaves the
/// content box exactly as wide as the shaped run, which puts the run in the middle by arithmetic.
/// A cap whose run is already wider than its square keeps the padding `apply_skin` gave it.
///
/// Runs in `PostUpdate`, after Bevy has laid the text out ([`bevy::ui::UiSystems::PostLayout`]):
/// the run's width is not known before then. It writes only a difference, so a settled legend
/// costs a read a frame.
pub fn centre_keycaps(mut caps: Query<(&Role, &mut Node, &ComputedNode, &TextLayoutInfo)>) {
    for (role, mut node, computed, text) in &mut caps {
        if !matches!(*role, Role::PromptKey | Role::PromptButton) {
            continue;
        }
        let run = text.size.x;
        if run <= 0.0 {
            // No font yet (or a glyph the face lacks): nothing has been shaped to centre.
            continue;
        }
        let inner = computed.content_box().size().x;
        // Positive when the content box is wider than the run: half of that, in logical px, is what
        // the left padding must give up so the run's own width is what the content box spans.
        let slack = inner - run;
        if slack <= 0.5 {
            continue;
        }
        let Val::Px(padding) = node.padding.left else {
            continue;
        };
        let Val::Px(vertical) = node.padding.top else {
            continue;
        };
        // `slack` is in physical pixels and a `Val::Px` in logical ones: at a scale factor of 2 and a
        // `UiScale` of 1.2 a slack of 10 px is 4.2 logical px, and half of that is the padding. Adding
        // the physical half widened the cap past its square, which resized the tooltip box that is
        // sized from the cap, which restyled the cap (#79).
        let next = Val::Px(padding + slack * computed.inverse_scale_factor() / 2.0);
        if node.padding.left != next {
            node.padding = UiRect::axes(next, Val::Px(vertical));
        }
    }
}
