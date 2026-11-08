//! The machine plate: the family API standard, checked against `GooeyPlugins`.
//!
//! Headless: Bevy's default plugins with no window backend, as the other tests here run. Names come
//! from `System::name()`, which is the full type path only with Bevy's `debug` feature; the
//! dev-dependency on `bevy` turns it on, and the test asserts it saw systems of the crate so a build
//! without it fails instead of passing vacuously.

use bevy::app::App;
use bevy::ecs::reflect::{AppTypeRegistry, ReflectComponent, ReflectResource};
use bevy::ecs::schedule::{Schedules, SystemSet};
use bevy::prelude::*;
use bevy::reflect::{GetTypeRegistration, TypeRegistry, prelude::ReflectDefault};
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::winit::WinitPlugin;
use bevy_gooey::{GooeyPlugins, GooeySystems};

const CRATE: &str = "bevy_gooey::";

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
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
    ));
    app.update();
    app
}

/// Every system of the crate must be in one of the crate's root-set variants.
#[test]
fn every_system_is_in_the_root_set() {
    let app = app();
    let roots = [
        GooeySystems::Input,
        GooeySystems::Navigate,
        GooeySystems::Style,
        GooeySystems::Settle,
        GooeySystems::Layout,
    ]
    .map(|variant| variant.intern());
    let schedules = app.world().resource::<Schedules>();
    let mut seen = 0;
    let mut outside = Vec::new();
    for (label, schedule) in schedules.iter() {
        let Ok(systems) = schedule.systems() else {
            continue;
        };
        for (key, system) in systems {
            let name = system.name().to_string();
            if !name.starts_with(CRATE) {
                continue;
            }
            seen += 1;
            let in_root = roots.iter().any(|root| {
                schedule
                    .graph()
                    .systems_in_set(*root)
                    .is_ok_and(|set| set.contains(&key))
            });
            if !in_root {
                outside.push(format!("{name} in {label:?}"));
            }
        }
    }
    assert!(
        seen > 0,
        "no `{CRATE}` systems seen: is Bevy's `debug` feature on?"
    );
    assert!(
        outside.is_empty(),
        "systems outside GooeySystems: {outside:#?}"
    );
}

fn assert_settings<T: GetTypeRegistration>(registry: &TypeRegistry) {
    let name = std::any::type_name::<T>();
    let registration = registry
        .get(std::any::TypeId::of::<T>())
        .unwrap_or_else(|| panic!("{name} is not registered by GooeyPlugins"));
    assert!(
        registration.data::<ReflectResource>().is_some(),
        "{name} has no ReflectResource"
    );
    assert!(
        registration.data::<ReflectDefault>().is_some(),
        "{name} has no ReflectDefault"
    );
    #[cfg(feature = "serde")]
    {
        assert!(
            registration
                .data::<bevy::reflect::prelude::ReflectSerialize>()
                .is_some(),
            "{name} has no ReflectSerialize"
        );
        assert!(
            registration
                .data::<bevy::reflect::prelude::ReflectDeserialize>()
                .is_some(),
            "{name} has no ReflectDeserialize"
        );
    }
}

fn assert_component<T: GetTypeRegistration>(registry: &TypeRegistry) {
    let name = std::any::type_name::<T>();
    let registration = registry
        .get(std::any::TypeId::of::<T>())
        .unwrap_or_else(|| panic!("{name} is not registered by GooeyPlugins"));
    assert!(
        registration.data::<ReflectComponent>().is_some(),
        "{name} has no ReflectComponent"
    );
}

/// Every settings resource is reflected, defaulted, serializable and registered.
#[test]
fn settings_are_reflected() {
    let app = app();
    let registry = app.world().resource::<AppTypeRegistry>().read();
    assert_settings::<bevy_gooey::BindingSettings>(&registry);
    assert_settings::<bevy_gooey::DragSettings>(&registry);
    assert_settings::<bevy_gooey::GooeySkin>(&registry);
    assert_settings::<bevy_gooey::HoldSettings>(&registry);
    assert_settings::<bevy_gooey::MotionSettings>(&registry);
    assert_settings::<bevy_gooey::PointerSettings>(&registry);
    assert_settings::<bevy_gooey::SplitPeekSettings>(&registry);
    assert_settings::<bevy_gooey::ToastSettings>(&registry);
    assert_settings::<bevy_gooey::TooltipSettings>(&registry);
    assert_settings::<bevy_gooey::UiScaleSettings>(&registry);
}

/// Every public component is reflected and registered. `DragPayload<T>` and `DropTarget<T>` are
/// generic over the game's payload type and are registered per payload by the game.
#[test]
fn components_are_reflected() {
    let app = app();
    let registry = app.world().resource::<AppTypeRegistry>().read();
    assert_component::<bevy_gooey::AnchorPinned>(&registry);
    assert_component::<bevy_gooey::AreaMarker>(&registry);
    assert_component::<bevy_gooey::Arriving>(&registry);
    assert_component::<bevy_gooey::BindingValue>(&registry);
    assert_component::<bevy_gooey::ButtonVariant>(&registry);
    assert_component::<bevy_gooey::Capturing>(&registry);
    assert_component::<bevy_gooey::CardCursor>(&registry);
    assert_component::<bevy_gooey::CardGrid>(&registry);
    assert_component::<bevy_gooey::CardRef>(&registry);
    assert_component::<bevy_gooey::CardSelection>(&registry);
    assert_component::<bevy_gooey::ChoiceCount>(&registry);
    assert_component::<bevy_gooey::ChoiceValue>(&registry);
    assert_component::<bevy_gooey::ChordText>(&registry);
    assert_component::<bevy_gooey::ClickCount>(&registry);
    assert_component::<bevy_gooey::ColorArea>(&registry);
    assert_component::<bevy_gooey::ColorPicker>(&registry);
    assert_component::<bevy_gooey::ColorRail>(&registry);
    assert_component::<bevy_gooey::ColorStrip>(&registry);
    assert_component::<bevy_gooey::ColorSwatch>(&registry);
    assert_component::<bevy_gooey::ColorTrack>(&registry);
    assert_component::<bevy_gooey::ColorValue>(&registry);
    assert_component::<bevy_gooey::ComboField>(&registry);
    assert_component::<bevy_gooey::ComboListOpen>(&registry);
    assert_component::<bevy_gooey::ComboNote>(&registry);
    assert_component::<bevy_gooey::ComboRow>(&registry);
    assert_component::<bevy_gooey::ComboValue>(&registry);
    assert_component::<bevy_gooey::Combobox>(&registry);
    assert_component::<bevy_gooey::CommandPalette>(&registry);
    assert_component::<bevy_gooey::Confirm>(&registry);
    assert_component::<bevy_gooey::ConfirmButton>(&registry);
    assert_component::<bevy_gooey::ContextMenu>(&registry);
    assert_component::<bevy_gooey::ContextTarget>(&registry);
    assert_component::<bevy_gooey::ControlSize>(&registry);
    assert_component::<bevy_gooey::Cooldown>(&registry);
    assert_component::<bevy_gooey::DefaultButton>(&registry);
    assert_component::<bevy_gooey::Dialog>(&registry);
    assert_component::<bevy_gooey::Disabled>(&registry);
    assert_component::<bevy_gooey::Dock>(&registry);
    assert_component::<bevy_gooey::DockPanel>(&registry);
    assert_component::<bevy_gooey::DragLabel>(&registry);
    assert_component::<bevy_gooey::Dragging>(&registry);
    assert_component::<bevy_gooey::DropCandidate>(&registry);
    assert_component::<bevy_gooey::DropRefusing>(&registry);
    assert_component::<bevy_gooey::FieldProblem>(&registry);
    assert_component::<bevy_gooey::FocusRegion>(&registry);
    assert_component::<bevy_gooey::GroupHold>(&registry);
    assert_component::<bevy_gooey::GroupReveal>(&registry);
    assert_component::<bevy_gooey::HoldProgress>(&registry);
    assert_component::<bevy_gooey::HoldToConfirm>(&registry);
    assert_component::<bevy_gooey::HoverGroup>(&registry);
    assert_component::<bevy_gooey::Icon>(&registry);
    assert_component::<bevy_gooey::IconMissing>(&registry);
    assert_component::<bevy_gooey::IconSize>(&registry);
    assert_component::<bevy_gooey::IconTone>(&registry);
    assert_component::<bevy_gooey::Indicating>(&registry);
    assert_component::<bevy_gooey::IndicatorPuck>(&registry);
    #[cfg(feature = "reflect")]
    {
        assert_component::<bevy_gooey::InspectMixed>(&registry);
        assert_component::<bevy_gooey::InspectRow>(&registry);
        assert_component::<bevy_gooey::Inspector>(&registry);
    }
    assert_component::<bevy_gooey::InstantMotion>(&registry);
    assert_component::<bevy_gooey::LayoutTravel>(&registry);
    assert_component::<bevy_gooey::Leave>(&registry);
    assert_component::<bevy_gooey::Leaving>(&registry);
    assert_component::<bevy_gooey::ListView>(&registry);
    assert_component::<bevy_gooey::Loading>(&registry);
    assert_component::<bevy_gooey::LoadingHold>(&registry);
    assert_component::<bevy_gooey::LoadingSpinner>(&registry);
    assert_component::<bevy_gooey::MenuBar>(&registry);
    assert_component::<bevy_gooey::MenuItem>(&registry);
    assert_component::<bevy_gooey::MenuMarked>(&registry);
    assert_component::<bevy_gooey::MenuOpen>(&registry);
    assert_component::<bevy_gooey::MenuPanel>(&registry);
    assert_component::<bevy_gooey::MenuTitle>(&registry);
    assert_component::<bevy_gooey::Meter>(&registry);
    assert_component::<bevy_gooey::MixedValue>(&registry);
    assert_component::<bevy_gooey::MotionTarget>(&registry);
    assert_component::<bevy_gooey::MovesSmoothly>(&registry);
    assert_component::<bevy_gooey::NotificationCenter>(&registry);
    assert_component::<bevy_gooey::NotificationRow>(&registry);
    assert_component::<bevy_gooey::NumberInput>(&registry);
    assert_component::<bevy_gooey::NumberRange>(&registry);
    assert_component::<bevy_gooey::NumberStep>(&registry);
    assert_component::<bevy_gooey::NumberValue>(&registry);
    assert_component::<bevy_gooey::Opacity>(&registry);
    assert_component::<bevy_gooey::OptionRow>(&registry);
    assert_component::<bevy_gooey::Overflowed>(&registry);
    assert_component::<bevy_gooey::PaletteCount>(&registry);
    assert_component::<bevy_gooey::PaletteInput>(&registry);
    assert_component::<bevy_gooey::PaletteNote>(&registry);
    assert_component::<bevy_gooey::PaletteRow>(&registry);
    assert_component::<bevy_gooey::Pop>(&registry);
    assert_component::<bevy_gooey::PopoverPanel>(&registry);
    assert_component::<bevy_gooey::PopoverPoint>(&registry);
    assert_component::<bevy_gooey::PopoverSlide>(&registry);
    assert_component::<bevy_gooey::Popped>(&registry);
    assert_component::<bevy_gooey::PressScale>(&registry);
    assert_component::<bevy_gooey::PressScaled>(&registry);
    assert_component::<bevy_gooey::PrimaryButton>(&registry);
    assert_component::<bevy_gooey::Progress>(&registry);
    assert_component::<bevy_gooey::ProgressClamped>(&registry);
    assert_component::<bevy_gooey::ProgressText>(&registry);
    assert_component::<bevy_gooey::Prompt>(&registry);
    assert_component::<bevy_gooey::PromptsAnchor>(&registry);
    assert_component::<bevy_gooey::RadialCount>(&registry);
    assert_component::<bevy_gooey::RadialSlot>(&registry);
    assert_component::<bevy_gooey::RadialValue>(&registry);
    assert_component::<bevy_gooey::RevealTrack>(&registry);
    assert_component::<bevy_gooey::Role>(&registry);
    assert_component::<bevy_gooey::ScrollEdgeFade>(&registry);
    assert_component::<bevy_gooey::ScrollFade>(&registry);
    assert_component::<bevy_gooey::ScrollFadeParts>(&registry);
    assert_component::<bevy_gooey::ScrollGlide>(&registry);
    assert_component::<bevy_gooey::SectionGlide>(&registry);
    assert_component::<bevy_gooey::SectionHeader>(&registry);
    assert_component::<bevy_gooey::SectionId>(&registry);
    assert_component::<bevy_gooey::SectionOpen>(&registry);
    assert_component::<bevy_gooey::Select>(&registry);
    assert_component::<bevy_gooey::SelectMode>(&registry);
    assert_component::<bevy_gooey::SelectionIndicator>(&registry);
    assert_component::<bevy_gooey::Shortcut>(&registry);
    assert_component::<bevy_gooey::ShortcutBadge>(&registry);
    assert_component::<bevy_gooey::ShortcutLegendCard>(&registry);
    assert_component::<bevy_gooey::SkinScope>(&registry);
    assert_component::<bevy_gooey::SlotCursor>(&registry);
    assert_component::<bevy_gooey::SlotGrid>(&registry);
    assert_component::<bevy_gooey::SlotItems>(&registry);
    assert_component::<bevy_gooey::SlotRef>(&registry);
    assert_component::<bevy_gooey::SlotRules>(&registry);
    assert_component::<bevy_gooey::Spinner>(&registry);
    assert_component::<bevy_gooey::Split>(&registry);
    assert_component::<bevy_gooey::SplitDivider>(&registry);
    assert_component::<bevy_gooey::SplitFraction>(&registry);
    assert_component::<bevy_gooey::SplitGlide>(&registry);
    assert_component::<bevy_gooey::SplitMin>(&registry);
    assert_component::<bevy_gooey::SplitPeek>(&registry);
    assert_component::<bevy_gooey::SplitSaved>(&registry);
    assert_component::<bevy_gooey::StatusBar>(&registry);
    assert_component::<bevy_gooey::StatusItem>(&registry);
    assert_component::<bevy_gooey::StatusKind>(&registry);
    assert_component::<bevy_gooey::StatusPriority>(&registry);
    assert_component::<bevy_gooey::Stay>(&registry);
    assert_component::<bevy_gooey::SwatchHalf>(&registry);
    assert_component::<bevy_gooey::TabPageOf>(&registry);
    assert_component::<bevy_gooey::Table>(&registry);
    assert_component::<bevy_gooey::TableCursor>(&registry);
    assert_component::<bevy_gooey::TableRow>(&registry);
    assert_component::<bevy_gooey::TableRows>(&registry);
    assert_component::<bevy_gooey::TableSelection>(&registry);
    assert_component::<bevy_gooey::TableSort>(&registry);
    assert_component::<bevy_gooey::TableWidths>(&registry);
    assert_component::<bevy_gooey::TabsCount>(&registry);
    assert_component::<bevy_gooey::TabsValue>(&registry);
    assert_component::<bevy_gooey::TextEdited>(&registry);
    assert_component::<bevy_gooey::TextEditing>(&registry);
    assert_component::<bevy_gooey::TextHint>(&registry);
    assert_component::<bevy_gooey::TextInputOf>(&registry);
    assert_component::<bevy_gooey::TextInputValue>(&registry);
    assert_component::<bevy_gooey::TextLimit>(&registry);
    assert_component::<bevy_gooey::Toast>(&registry);
    assert_component::<bevy_gooey::ToastButton>(&registry);
    assert_component::<bevy_gooey::ToastStack>(&registry);
    assert_component::<bevy_gooey::ToggleGroupCount>(&registry);
    assert_component::<bevy_gooey::ToggleGroupValue>(&registry);
    assert_component::<bevy_gooey::Toolbar>(&registry);
    assert_component::<bevy_gooey::Tooltip>(&registry);
    assert_component::<bevy_gooey::TooltipBox>(&registry);
    assert_component::<bevy_gooey::TooltipDwell>(&registry);
    assert_component::<bevy_gooey::TravelFrom>(&registry);
    assert_component::<bevy_gooey::TravelSpring>(&registry);
    assert_component::<bevy_gooey::TypeSelectGroup>(&registry);
    assert_component::<bevy_gooey::VecMixed>(&registry);
    assert_component::<bevy_gooey::VecPart>(&registry);
    assert_component::<bevy_gooey::VecValue>(&registry);
    assert_component::<bevy_gooey::VerticalTab>(&registry);
    assert_component::<bevy_gooey::WorldAnchor>(&registry);
    assert_component::<bevy_gooey::WorldAnchors>(&registry);
}
