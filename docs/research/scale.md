# The interface keeps its share of the screen: scale with the window

Researched 2026-10-30 (iteration 4). home-still's distill was healthy and idle (`system_status`), so the craft and API sources are corpus stems; the text-size numbers, the shipping game and the engine practice are web pages, each quoted from a fetched copy.

## Sources

1. Cairns, Barlet, Haynes and Power, *Future design of accessibility in games: A design vocabulary*, IJHCS 2019 — home-still `10.1016_j.ijhcs.2019.06.010` (craft).
2. Microsoft, *Xbox Accessibility Guideline 101: Text display* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/101 (craft: minimum text sizes per resolution, and how to measure them).
3. Game Accessibility Guidelines, *Use an easily readable default font size* — https://gameaccessibilityguidelines.com/use-an-easily-readable-default-font-size/ (craft).
4. Wube Software (kovarex), *Friday Facts #277: The scaling problem and solution*, 2019 — https://factorio.com/blog/post/fff-277 (shipping game: Factorio's GUI rewrite for 0.17).
5. Epic Games, *DPI Scaling in Unreal Engine* — https://dev.epicgames.com/documentation/en-us/unreal-engine/dpi-scaling-in-unreal-engine (technical: how the engine most shipped console games use scales a UI by default).
6. Buitinck et al., *API design for machine learning software: experiences from the scikit-learn project*, 2013 — home-still `W1728842521` (technical: API defaults).
7. Bevy 0.20 source: `bevy_ui-0.20.0/src/lib.rs` (`UiScale`: "A multiplier to fixed-sized ui values"), `update.rs` (a UI target's scale factor is the camera's `target_scaling_factor() * ui_scale.0`), `ui_node.rs` (`ComputedUiRenderTargetInfo::logical_size` divides by that factor) and `bevy_ui_widgets-0.20.0/src/popover.rs` (places a popover in that same logical space) (technical: what the engine does).

## Craft principle

Cairns et al. name the problem among the access options a game offers: device options are "distinct from, but can interact with, presentation options (e.g. resolution of screen could make text too small)", and they are "about replacing or modifying the device itself, for example using a large screen, versus a mobile screen". The screen a game runs on is not the designer's; the text must survive it.

XAG 101 puts numbers on it, and the numbers are proportional to the screen: "**Console:** Font size should equal or exceed: 26 px at 1080p, 52 px at 4K" and "**PC/VR:** Font size should equal or exceed: 18 px at 1080p, 36 px at 4K", measured as body height, "the sum of the number of pixels in the descender space, the x-height space, and the ascender space". Doubling the resolution doubles the minimum. It names the situation: "There are also situational circumstances that can make text hard to read, such as playing on a screen that's far away". And: "Developers are encouraged to consider individual player scenarios, for example the typical distance from couch to television, and provide players the option to scale larger or smaller at the player's discretion".

The Game Accessibility Guidelines give the 10-foot figure: "Amazon TV have 10-foot-UI guidelines that include text size recommendations, of 28px minimum when viewed on a 1080p screen … use 28px as a minimum rather than a target".

A fixed pixel size meets those rules at one resolution only. Measured on this crate's gallery (`/tmp/g4/findings.md`): the tooltip line's body height is 21 px at 1280x720 and still 21 px at 2560x1440, where XAG asks 24 px (PC) and 34.7 px (console); the menu panel is 43 % of the window's height at 720p and 22 % at 1440p.

## How shipping games do it

Factorio rewrote its GUI so that it scales as one picture: "One of the many goals of the GUI rewrite was to make sure that the GUI looks correct in all possible UI scale values. By correct, we mainly mean, that the proportions of everything stay the same, so it is just smaller, but not deformed in a weird way." Its trap is rounding: "Imagine that you have a 1 pixel wide gap between two 20 pixel wide buttons and then you want it to show in 120%. The buttons are enlarged to 24 pixels, but the gap either stays 1 pixel or becomes 2, but the proportions of the layout changes." Its answer, for pixel-art tiles: "1 module is 4 pixels in standard scale (100%) … we limited the possible scale values to be multiples of 25% (from 75% to 200%)."

Unreal, the engine behind much of the console catalogue, does it for every project unless told otherwise: "UMG supports automatic scaling for resolution-independent UI. There are default DPI scaling settings applied to every project." The rule most projects use: "Shortest Side: Evaluates the scale curve based on the shortest side of the viewport (Most Common Setting)." On top of the curve sits a project-wide "Application Scale".

## Model adopted

- **S1. The whole interface scales with the window, as one picture.** `UiScale` is set to the window's shorter side divided by a reference size, so every widget, gap, border, radius, text and tooltip keeps its share of the screen, as Factorio's GUI keeps its proportions. One multiplier for everything, not a per-widget size: Bevy multiplies every `Val::Px` by `UiScale` and lays text out at the final size, so nothing in `apply_skin` changes and the host's own UI nodes scale with the crate's.
- **S2. The shorter side decides**, as Unreal's most common rule: a 16:9 window scales by its height, an ultrawide by its height, a portrait window by its width, so the interface never outgrows the narrow dimension.
- **S3. On by default.** Unreal applies its scaling "to every project"; scikit-learn: "Whenever an operation requires a user-defined parameter, an appropriate default value is defined by the library. The default value should cause the operation to be performed in a sensible way". A game built on `GooeyPlugins` is legible on a 1440p monitor and a 4K television without a line of code. It is its own plugin, `GooeyScalePlugin`, so a host that drives `UiScale` itself leaves it out.
- **S4. The player's size on top.** `UiScaleSettings::scale` multiplies the window's factor, as Unreal's Application Scale does and as XAG asks ("provide players the option to scale larger or smaller"). It is the knob the host used to turn on `UiScale` directly. Text alone still grows with `GooeySkin::text_scale`.
- **S5. A window with no size changes nothing.** A minimised window reports 0 px; the scale it would give is 0, which no layout can use, so `UiScale` keeps its last value. A reference or player scale at or below zero is invalid; it leaves `UiScale` as it is and warns once.

## Parameters

- `UiScaleSettings::reference_short_side`: **720** logical px. The skin's sizes are the sizes a 1280x720 window shows, and every earlier pass set and measured them there (44 px targets, 21 px body text). At 720 the default text meets XAG's console minimum at every resolution: body height 21 px × (1080 / 720) = 31.5 px at 1080p (≥ 26, and ≥ the GAG's 28), 42 px at 1440p (≥ 34.7), 63 px at 4K (≥ 52). A design default; see Deviations.
- `UiScaleSettings::scale`: **1.0**.
- `UiScale = shorter side / reference_short_side × scale`. 1280x720 → 1.0, 1280x800 (Steam Deck) → 1.11, 1920x1080 → 1.5, 2560x1440 → 2.0, 3840x2160 → 3.0, 720x1280 → 1.0.

## Validation targets

- 1280x720: `UiScale` 1.0 and every capture identical to the baseline.
- 2560x1440: `UiScale` 2.0; the tooltip line's body height ≥ 34.7 px (XAG console at 1440p; expected 42); the panel's share of the window height equal to 1280x720's within 1 %.
- 1920x1080: body height ≥ 28 px (GAG); expected 31–32.
- 1280x800 (1.11, a scale that is not a whole number): every button's border even within 1 px on all four sides, no text clipped, the focus ring whole.
- At 2x the tooltip opens beside Options, clear of Play and Quit and inside the window: the crate's own placement (`TooltipBox::room`, `Popover`) works in the scaled space.
- A window resized from 1280x720 to 1920x1080 is drawn at 1.5 the next frame; a portrait 720x1280 window at 1.0; `scale` 1.25 at 1280x720 gives 1.25; a 0x0 window keeps the last value.
- Frame time at 1280x720 within 0.5 ms of the baseline: one system reading one window and writing `UiScale` only when it changes.

## What great looks like

The same composition at every resolution: the menu holds the same share of a 720p laptop, a 1080p television and a 1440p monitor, so a screenshot from one is a scaled copy of the other. Text is drawn at its final size, crisp at 2x, never a stretched bitmap. Borders, the focus ring and the meter's frame thicken with everything else, so a 3 px ring at 720p is a 6 px ring a player can find from the couch at 1440p. At a scale that is not a whole number (the Steam Deck's 1.11) nothing wobbles: borders stay even and text sits centred in its button.

## Deviations

- **Reference 720, where Unreal's default curve is 1.0 at 1080.** The skin's sizes were designed and checked at 1280x720, and at 720 they meet XAG's console minimum at every size; at 1080 they would meet only the PC one (21 px body at 1080p, < 26). A design default resting on XAG 101; a game that designs at 1080 sets `reference_short_side: 1080.0`.
- **Continuous scale, where Factorio snaps to 25 % steps.** Factorio snaps so its 4 px pixel-art modules land on whole pixels. This crate draws no bitmap art: boxes are rounded rectangles and text is laid out at its final size. Snapping to quarters would also not give whole pixels here (a 3 px border is 3.75 px at 125 %). Measured after the change: at 1280x800 (1.11x) every border and the meter's frame, gap and fill are even (3 / 3 / 14 / 3 / 3 px); at 1920x1080 (1.5x) the meter's gap is 5 px above the fill and 4 px below (frame 4, gap 5, fill 19, gap 4, frame 4; ideal 4.5 / 4.5 / 18 / 4.5 / 4.5), because Bevy's layout rounds each border and padding to physical pixels from the node's sub-pixel position. This is Factorio's "the gap either stays 1 pixel or becomes 2" problem, and it is not new: a window scale factor of 1.5 (a laptop at 150 %) gave the same before this change, since Bevy multiplies the window's factor and `UiScale` into one. Its fix belongs to line weights, not to the scale, and is next in the backlog: snap each line to whole device pixels, as CSS does (CSS Values 4, https://www.w3.org/TR/css-values-4/, "snap a length as a border width": "If len is greater than 1 device pixel, round it down to the nearest integer number of device pixels").
- **No lower bound.** A window smaller than 720 px on its short side draws a smaller interface, in proportion, as Unreal's curve does. XAG's minimums are proportional to the resolution, so the proportional size still meets them.
- **`UiScale` is no longer the host's.** The crate doc said "`UiScale` is the host's"; `GooeyScalePlugin` now sets it. Assumed decision: on by default in `GooeyPlugins` (Unreal's default, and a game that never tests at 1440p ships legible text), over opt-in (the default would keep shipping 21 px text to 4K televisions). A host that wants its own `UiScale` leaves the plugin out; a player's size goes in `UiScaleSettings::scale`.

## Pending index

- Index XAG 101, the Game Accessibility Guidelines' font-size page, Factorio FFF #277 and Unreal's DPI scaling page into home-still.
- Not in the corpus: a console studio's write-up on 10-foot text sizes (searched "minimum text size for television viewing distance in video games user interface": no game source). A GDC talk on resolution-independent HUDs would join Factorio as the shipping-game source.
