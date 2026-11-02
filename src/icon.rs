//! Icons: a small image the skin sizes and tints, for the host's own art.

use bevy::asset::LoadState;
use bevy::prelude::*;

/// The picture an [`Icon`] draws: a whole image, or one cell of a texture atlas the host laid out
/// (a sprite sheet of white-on-transparent glyphs is the usual one: the skin tints it).
#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum IconSource {
    /// The whole image.
    Image(Handle<Image>),
    /// One cell of an atlas.
    Atlas {
        /// The atlas's image.
        image: Handle<Image>,
        /// How the image is cut up.
        layout: Handle<TextureAtlasLayout>,
        /// Which cell.
        index: usize,
    },
}

/// What an [`icon`](crate::icon) shows. Change it and the picture changes in the same frame
/// ([`update_icons`]). The size and the tint are the skin's: see [`IconSize`] and [`IconTone`].
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[require(IconTone, IconSize, ImageNode)]
pub struct Icon(pub IconSource);

/// Which colour of the skin tints an icon. The image is multiplied by it, so draw icons white on
/// transparent and let the skin colour them; a tone is a colour of the skin and never a number in
/// the host.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IconTone {
    /// The text colour, following the widget the icon is in as its text does: muted while that
    /// widget is disabled, the accent's text while it is pressed.
    #[default]
    Text,
    /// [`crate::GooeySkin::text_muted`], for an icon that is a hint.
    Muted,
    /// [`crate::GooeySkin::accent`], for an icon that says "on" or "selected".
    Accent,
    /// [`crate::GooeySkin::danger`], for a destructive command.
    Danger,
}

/// An icon's side: a step of [`crate::GooeySkin::type_scale`], so it is as tall as the text of the
/// same step and grows with the player's text size. The default, step 2, is the body's line.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct IconSize(pub usize);

impl Default for IconSize {
    fn default() -> Self {
        Self(2)
    }
}

/// On an [`Icon`] whose image failed to load, or that has none. The icon keeps its size and draws
/// nothing, so a toolbar never reflows; a warning is logged once when the marker is added.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct IconMissing;

/// Puts each changed [`Icon`]'s picture on its [`ImageNode`], and marks an icon whose image is
/// missing ([`IconMissing`], one warning). Runs in [`crate::GooeySystems::Style`] before
/// [`crate::apply_skin`].
pub fn update_icons(
    mut commands: Commands,
    assets: Option<Res<Assets<Image>>>,
    server: Option<Res<AssetServer>>,
    mut icons: Query<(Entity, Ref<Icon>, &mut ImageNode, Has<IconMissing>)>,
) {
    for (entity, icon, mut image, mut missing) in &mut icons {
        if icon.is_changed() {
            let (handle, atlas) = match &icon.0 {
                IconSource::Image(handle) => (handle, None),
                IconSource::Atlas {
                    image,
                    layout,
                    index,
                } => (
                    image,
                    Some(TextureAtlas {
                        layout: layout.clone(),
                        index: *index,
                    }),
                ),
            };
            if image.image != *handle {
                image.image = handle.clone();
            }
            if image.texture_atlas != atlas {
                image.texture_atlas = atlas;
            }
            if missing {
                commands.entity(entity).remove::<IconMissing>();
                missing = false;
            }
        } else if missing {
            continue;
        }
        let source = match &icon.0 {
            IconSource::Image(handle) | IconSource::Atlas { image: handle, .. } => handle,
        };
        if assets
            .as_ref()
            .is_some_and(|assets| assets.contains(source))
        {
            continue;
        }
        // Not there yet is not missing: a handle the server is still loading says `Loading`.
        let gone = server.as_ref().is_none_or(|server| {
            matches!(
                server.load_state(source),
                LoadState::Failed(_) | LoadState::NotLoaded
            )
        });
        if gone && !missing {
            warn!("bevy_gooey: icon {entity}: its image is missing; drawing an empty box");
            commands.entity(entity).insert(IconMissing);
        }
    }
}
