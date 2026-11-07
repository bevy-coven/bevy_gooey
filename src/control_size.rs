//! Control sizes: a control a step smaller or larger than the skin's own, for a dense inspector or a
//! dialog's main action.

use bevy::prelude::*;

/// How tall a control stands: a button, an icon button, a toggle, a choice, a text field, a number,
/// vec or colour field, or a select. Put it on the control's root, beside what `button()` and its
/// kin spawn. The heights are AppKit's push-button heights as Rune measures them
/// (`NSControl.ControlSize` Regular, Large and ExtraLarge), in logical px; Mini and Small (16 and
/// 20) are below WCAG 2.2 SC 2.5.8's 24 px target and are not offered.
///
/// A size is a least height: the control is never smaller than the skin's own target
/// ([`crate::GooeySkin::target`]), so under a comfortable skin (a 44 px target) every size is 44 px,
/// and only a compact skin (the Desk's or the tool's, at 24) shows three heights. The text size
/// follows the skin, not the control. A control with no `ControlSize` keeps the height it had before.
/// For a text, number, vec or colour field the framed field is the size, and its row adds its label's
/// air round it. A `text_area` takes the size as its least height; its lines start at its top.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // An inspector's small controls and a dialog's large main action.
/// commands.spawn((toggle("Snap", true), ControlSize::Small));
/// commands.spawn((button("Create project"), ControlSize::Large));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ControlSize {
    /// 24 logical px: a dense inspector's row.
    Small,
    /// 28 logical px: a desktop control.
    #[default]
    Regular,
    /// 36 logical px: a dialog's main action.
    Large,
}

impl ControlSize {
    /// Every size, smallest first.
    pub const ALL: [ControlSize; 3] =
        [ControlSize::Small, ControlSize::Regular, ControlSize::Large];

    /// The size's height, logical px, before the skin's target is applied.
    pub const fn px(self) -> f32 {
        match self {
            ControlSize::Small => CONTROL_SMALL_PX,
            ControlSize::Regular => CONTROL_REGULAR_PX,
            ControlSize::Large => CONTROL_LARGE_PX,
        }
    }

    /// The height a control of this size stands at under a skin whose target is `target` logical
    /// px: the larger of the two.
    pub fn height(self, target: f32) -> f32 {
        self.px().max(target)
    }
}

/// [`ControlSize::Small`]'s height, logical px: AppKit's regular push button.
pub const CONTROL_SMALL_PX: f32 = 24.0;
/// [`ControlSize::Regular`]'s height, logical px: AppKit's large push button.
pub const CONTROL_REGULAR_PX: f32 = 28.0;
/// [`ControlSize::Large`]'s height, logical px: AppKit's extra-large push button.
pub const CONTROL_LARGE_PX: f32 = 36.0;
