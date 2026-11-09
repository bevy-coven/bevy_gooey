//! Art frames: a skin's panels and buttons drawn from nine-slice images (`docs/research/frames.md`).
//!
//! [`GooeyFrames`] holds the images (assets are the host's, so they live in their own resource, as
//! [`crate::GooeyFonts`] do). [`update_frames`] runs right after [`crate::apply_skin`]: it moves the
//! fill the skin wrote into the tint of an art layer under the widget's children, so every state
//! colour tints the art.

use bevy::prelude::*;
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};

use crate::skin::Role;

/// One nine-slice image: the art and the size of its corners.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// The art. Draw it in light neutrals: the skin's colours multiply it, so a dark panel fill
    /// darkens it and a hovered button fill lights it.
    pub image: Handle<Image>,
    /// The size of each corner and edge band in the image's own pixels.
    pub slice: f32,
}

impl Frame {
    /// A frame with `slice` pixel corners.
    pub fn new(image: Handle<Image>, slice: f32) -> Self {
        Self { image, slice }
    }

    fn mode(&self) -> NodeImageMode {
        NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::all(self.slice),
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale: 1.0,
        })
    }
}

/// The art a skin's widgets are drawn from. Nothing is set by default, and a widget with no frame
/// draws its flat fill as before. Set it before the widgets appear, or at any time: a frame set
/// later is applied to the widgets that restyle (a state change, a skin change) and a frame taken
/// away gives every fill back. The struct is `#[non_exhaustive]`: start from the resource (or
/// `GooeyFrames::default()`) and assign the fields you have art for, so a role added later is not a
/// breaking change.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn setup(mut frames: ResMut<GooeyFrames>, assets: Res<AssetServer>) {
/// frames.panel = Some(Frame::new(assets.load("ui/panel.png"), 12.0));
/// frames.button = Some(Frame::new(assets.load("ui/button.png"), 8.0));
/// # }
/// ```
#[derive(Resource, Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct GooeyFrames {
    /// The frame of every [`crate::panel`] (a dialog's too).
    pub panel: Option<Frame>,
    /// The frame of every [`crate::button`] that has a fill at rest (a ghost button stays flat).
    pub button: Option<Frame>,
    /// The frame of every [`crate::meter`] and [`crate::progress`] track: the art is drawn under the
    /// fill, which keeps the skin's colour (a meter's frame is its track, so its outline goes).
    pub meter: Option<Frame>,
    /// The frame of every slider's rail, switch's track and [`crate::hold_button`]'s track: the art
    /// is drawn under the fill, the knob and the thumb, which keep the skin's colours (a track's
    /// frame is its edge, so its outline goes). A switch's track is tinted by the accent while on.
    pub track: Option<Frame>,
    /// The frame of every text field ([`crate::text_input`], a number or colour field's text). The
    /// art is a layer of the field's cell, behind the field: Bevy measures the field itself as a
    /// leaf, which a child would collapse. The field's border (outline and focus ring) stays over
    /// the art.
    pub field: Option<Frame>,
    /// The frame of what floats over the screen: a tooltip, a popover, a menu's panel, the command
    /// palette and a toast's body. Their outline goes, as a panel's does.
    pub popup: Option<Frame>,
}

impl GooeyFrames {
    fn of(&self, role: Role) -> Option<&Frame> {
        match role {
            Role::Panel => self.panel.as_ref(),
            Role::Button => self.button.as_ref(),
            Role::Meter => self.meter.as_ref(),
            Role::SliderRail | Role::ToggleTrack | Role::HoldTrack => self.track.as_ref(),
            Role::TextField => self.field.as_ref(),
            Role::Tooltip | Role::Popover | Role::MenuPanel | Role::Palette | Role::ToastBody => {
                self.popup.as_ref()
            }
            _ => None,
        }
    }

    /// Whether the role's outline is the frame the art replaces (a button's border is also its focus
    /// ring, so it stays).
    fn replaces_outline(role: Role) -> bool {
        matches!(
            role,
            Role::Panel
                | Role::Meter
                | Role::SliderRail
                | Role::ToggleTrack
                | Role::HoldTrack
                | Role::Tooltip
                | Role::Popover
                | Role::MenuPanel
                | Role::Palette
                | Role::ToastBody
        )
    }
}

/// On a widget drawn from a [`Frame`]: the art's layer, a child the crate spawned (Bevy draws an
/// [`ImageNode`] inside its node's padding, so the art goes on a layer that fills the whole node).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Framed(pub Entity);

/// The layer that holds a widget's art: it fills the widget, draws under its other children, takes
/// no pointer and carries the tint.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct FrameLayer;

/// What [`update_frames`] reads of a widget.
type FramedNode = (
    Entity,
    Ref<'static, Role>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    Option<&'static Framed>,
    Has<ImageNode>,
);

/// Draws the widgets of a framed role from their frame: the fill [`crate::apply_skin`] just wrote
/// becomes the tint of an art layer under the widget's children and the flat fill is cleared; the
/// outline of a panel, tooltip, popover, menu or toast is cleared too (the art has its own edge),
/// while a button's border, its focus ring, stays over the art. A node is read only when its fill, its role or the frames changed,
/// and written only on a difference. Runs in [`crate::GooeySystems::Style`], after
/// [`crate::apply_skin`].
pub fn update_frames(
    frames: Res<GooeyFrames>,
    mut commands: Commands,
    mut nodes: Query<FramedNode, Without<FrameLayer>>,
    mut layers: Query<&mut ImageNode, With<FrameLayer>>,
    parents: Query<&ChildOf>,
) {
    let everything = frames.is_changed();
    for (entity, role, mut fill, mut border, framed, host_image) in &mut nodes {
        if !(everything || fill.is_changed() || role.is_changed()) {
            continue;
        }
        let role = *role;
        match (frames.of(role), framed) {
            (Some(frame), framed) => {
                let tint = fill.0;
                // A flat fill (a ghost button's none) stays flat; a widget with an image of the
                // game's own is the game's.
                let flat = Srgba::from(tint).alpha <= 0.0;
                match framed.and_then(|f| layers.get_mut(f.0).ok()) {
                    Some(mut image) => {
                        if image.image != frame.image {
                            image.image = frame.image.clone();
                        }
                        let mode = frame.mode();
                        if image.image_mode != mode {
                            image.image_mode = mode;
                        }
                        if !flat && image.color != tint {
                            image.color = tint;
                        }
                    }
                    None if flat || host_image => continue,
                    None => {
                        // A text field is a leaf to Bevy's layout: its art goes on its cell.
                        let host = match role {
                            Role::TextField => parents.get(entity).map_or(entity, ChildOf::parent),
                            _ => entity,
                        };
                        let layer = commands
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    right: Val::Px(0.0),
                                    top: Val::Px(0.0),
                                    bottom: Val::Px(0.0),
                                    ..default()
                                },
                                FrameLayer,
                                Role::FrameArt,
                                ImageNode::new(frame.image.clone())
                                    .with_color(tint)
                                    .with_mode(frame.mode()),
                                ZIndex(-1),
                                Pickable::IGNORE,
                                ChildOf(host),
                            ))
                            .id();
                        commands.entity(entity).try_insert(Framed(layer));
                    }
                }
                if !flat {
                    fill.set_if_neq(BackgroundColor(Color::NONE));
                    if GooeyFrames::replaces_outline(role) {
                        border.set_if_neq(BorderColor::all(Color::NONE));
                    }
                }
            }
            (None, Some(framed)) => {
                // The frame is gone: the tint is the fill again and the layer goes.
                if let Ok(image) = layers.get(framed.0) {
                    fill.set_if_neq(BackgroundColor(image.color));
                }
                commands.entity(framed.0).try_despawn();
                commands.entity(entity).remove::<Framed>();
            }
            (None, None) => {}
        }
    }
}
