# Icons: a small image tinted by the widget it stands in

Researched 2026-11-01 (iteration 37, #25 G-21). home-still's index has nothing on icon practice (`distill_search` for icon recognition returned privacy and VR-menu papers only), so the sources below were fetched today.

## Sources

1. Aurora Harley, *Icon Usability*, Nielsen Norman Group, 2014, https://www.nngroup.com/articles/icon-usability/ — craft.
2. Game Accessibility Guidelines, *Ensure no essential information is conveyed by a fixed colour alone*, https://gameaccessibilityguidelines.com/full-list/ and Xbox Accessibility Guideline 103, https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/103 (Grounded, For Honor) — shipping games.
3. *Item Icons*, 7 Days to Die modding wiki, https://7d2dmodding.wiki.gg/wiki/Item_Icons (a shipped game's tint rule); Bevy 0.20 `bevy_ui::widget::ImageNode` (`color`, `texture_atlas`) and W3C WCAG 2.2 SC 1.4.11 — technical.

## Craft principle

NN/g: "icons must first and foremost communicate meaning" and "a text label must be present alongside an icon to clarify its meaning in that particular context." An icon is a part of a widget, not a widget that stands alone: this crate draws it beside the label a `button` or menu row already has, and never makes the icon the only carrier of a command's name.

## How shipping games do it

Xbox guideline 103: "icons, patterns or shapes in addition to colour give the player multiple ways to receive the information", with Grounded using "color, as well as shape, iconography, and text labels", and For Honor's team icons "surrounded by both a black outline and a white outline". 7 Days to Die: "The game can recolour an icon on the fly with a tint — you don't need a separate coloured copy of the icon", and "The tint is a multiply: every pixel of the icon is multiplied by the tint colour", so "tintable base icons are near-white". The host supplies one white-on-transparent atlas and the skin colours it.

## Model adopted

- **I1. `icon(IconSource) -> impl Bundle`** is a square `Node` with `Role::Icon` and an `Icon(IconSource)` component. `IconSource` is `Image(Handle<Image>)` or `Atlas { image, layout, index }` (Bevy's `TextureAtlas`). The crate ships no art: the host supplies it.
- **I2. The tint is the role.** `IconTone { Text (default), Muted, Accent, Danger }` is a component; `apply_skin` writes `ImageNode::color` from it: `Text` is the text colour and follows the widget it stands in as the button's text does (muted when that widget is disabled, the accent's text while it is pressed); `Muted` is `text_muted`; `Accent` the accent; `Danger` the skin's `danger`. A tone is a skin colour, never a number in the host.
- **I3. The size is a step of the `type_scale`** (`IconSize(usize)`, default 2: the body's line, 20 px dark and 14 px in the tool look), multiplied by `text_scale`, so an icon is as tall as the text beside it at any text size and the 200% promise holds.
- **I4. A missing image draws an empty box of the right size and reports once.** An image whose load failed is a `warn!` once and the marker `IconMissing`; the box keeps its size so a toolbar never reflows. A handle still loading is not missing.
- **I5. Changing `Icon` changes the picture in the same frame** (`update_icons`, Style, before `apply_skin`); thirty idle frames write nothing.

## Parameters

|part|value|
|---|---|
|widget|`icon(IconSource) -> impl Bundle`, components `Icon`, `IconTone`, `IconSize`, marker `IconMissing`|
|size|`GooeySkin::type_size(IconSize.0)` square, never shrunk|
|tones|`Text`, `Muted`, `Accent`, `Danger`|
|input|none: an icon takes no focus and `Pickable::IGNORE`, so a click lands on the button it is in|

## Validation targets

- An icon with `IconTone::Danger` inside a button has `ImageNode::color == skin.danger` (the issue's acceptance); the same in every look and after a skin change.
- A default icon in a button is the button's text colour at rest, muted when disabled, the accent's text pressed.
- An atlas icon draws the atlas index it was given; changing `Icon` changes it the same frame.
- An icon is `type_size(2)` square at 1x and at 200% text; a missing image is the same size and one warning.
- A click on an icon activates its button; Tab never stops on an icon.
- `danger`, `accent` and `text_muted` keep the contrast `problems()` already holds (3:1 non-text, WCAG 1.4.11): no new colour.
- Frame time within 0.5 ms: no system does work without an icon.

## What great looks like

A toolbar row where the glyphs and their labels share one baseline and one weight, an icon that dims with a disabled button and lights with a pressed one, red only where the host said danger.

## Deviations

- `IconTone::Text` follows the widget's state; the others are fixed. A design default.
- Warning and success tones wait for named tones (G-29, #33): the skin has no such colours yet.

## Pending index

None of the three sources is in home-still; the NN/g, Xbox, 7D2D and WCAG pages should be added to `docs/research/corpus.md`'s pending list.
