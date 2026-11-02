# Radial items with pictures

Researched 2026-11-02 (iteration 49). `radial(items, selected)` (iteration 45) takes text only, and a weapon wheel, an emote wheel and a build menu are pictures: a game with a pistol, a rifle and a grenade has an icon for each, and the player picks by the shape under the thumb. `slot_grid` already draws a `Slot`'s picture through `IconSource`; the wheel is the one pointing widget without it. `docs/research/radial.md` names "icons, labels" as what a wheel's items hold (Gameface's `RadialMenu`) and left them out of slice 1; this is the slice.

## Sources

1. Kshitij Modi, *Game UI Design: How Radial Menus Enhance Player Experience*, 300Mind, 2026, https://300mind.studio/blog/radial-menus-in-game-design/ — craft.
2. Coherent Labs, Gameface UI `RadialMenu`, https://gameface-ui.coherent-labs.com/components/complex/radial-menu/ — shipping-game UI middleware for in-game weapon wheels.
3. W3C, *Understanding SC 1.4.11 Non-text Contrast* (WCAG 2.2), https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html — technical. In home-still, Lamberti et al. style menu studies are indexed (`10.1109_tvcg.2024.3420236`, marking menus in VR, items chosen by icon), but none states the icon-and-label rule below.

## Craft principle

The practice guide: "Using distinctive icons alongside concise labels improves readability, particularly for new players." An icon alone is learned by repetition, so a wheel that draws only pictures must still say the name somewhere the player is already looking; a wheel that draws only words is slow to scan at a glance in a fight.

## How shipping games do it

Gameface's radial menu takes "any content that should remain static and upright" in its slices ("icons, labels, or contextual info"), and its centre shows the selected item ("Displaying the Selected Item in the Center"): "In most cases, you only need to place an image, icon, or any content inside the RadialMenu." A weapon wheel draws a picture for each weapon on the ring and the name of the lit one in the middle.

## Model adopted

- **I1. `RadialItem { label, icon: Option<IconSource> }`** is one item (`RadialItem::new(label)`, `.icon(source)`), and `radial` takes `impl IntoIterator<Item: Into<RadialItem>>`, so `radial(["Pistol", "Rifle"], 0)` and a `Vec<String>` keep compiling; `From<&str>` and `From<String>` make a text item. No second constructor: one widget.
- **I2. An item with a picture draws the picture, not its label.** The label is the item's name: it is read in the middle of the disc when the item is lit (as for a text item), and it is the accessible name. The pairing the craft source asks for ("icons alongside labels") is the ring's picture plus the centre's words.
- **I3. The picture is `Role::RadialIcon`**, a square of `RADIAL_ICON_TEXT_SIZES` (3) text sizes in the wheel's own scale (so it shrinks with the wheel in a short window), tinted like a button's icon: `skin.text`, `accent_text` on the lit item, `text_muted` disabled. The item is a button's size and look with the icon's room as its least size.
- **I4. An item is data once.** The label lives in a private component on the item, so the centre never reads a drawn `Text` (an icon item has none) and the label and the picture cannot disagree.

## Parameters

`RADIAL_ICON_TEXT_SIZES` = 3 (48 px at the default text size: the item is a square about 76 px across, eight of them on a 26-size disc do not touch). Design default.

## Validation targets

An item with a picture has an image node of `RadialIcon` and no `Text`; a text item is unchanged. The centre reads the lit item's label for either kind and follows a stick, key or the game. The lit item's picture is `accent_text`, the others `text`, and a disabled wheel's `text_muted` (3:1 against the disc, SC 1.4.11, in every preset). With eight picture items no two overlap and none leaves the disc at every `TEXT_SIZE_STEPS` step; a mixed wheel works. The wheel's accessible value is the lit label. Thirty idle frames write nothing.

## Deviations

No picture-and-label in the ring at once: the ring has little room and the centre already says the name (design default, as Gameface's own weapon wheel). No per-item tint: a coloured picture is the game's image, tinted white-on-transparent like a slot's.

## What great looks like

A round dark disc with eight weapon silhouettes on the ring, the lit one on an accent pill with the picture inverted, the weapon's name large in the middle under the thumb, the focus ring on the disc.

## Pending index

The Gameface page and the 300Mind article are web sources (fetched 2026-11-02); `paper_download` cannot take them. The WCAG page likewise.
