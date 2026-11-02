# The radial menu

Researched 2026-11-02 (iteration 45). Section 4 of the loop brief names the radial menu among "a missing widget every game needs", and the log's backlog has carried an "inventory grid" for twenty iterations without a widget that a gamepad player reaches in one move. A weapon wheel, an emote wheel, a build menu and a companion-command wheel are one widget: items on a ring, chosen by the direction of a stick, the pointer or the D-pad, confirmed by a release, a click or South. This crate has every part (focus, `ValueChange`, the skin, the pad) except the ring.

## Sources

1. The *International Journal of Human-Computer Studies* (2011) study of radial menu selection with finger, stylus and mouse (`10.1016/j.ijhcs.2011.11.002` in home-still; the index returned it without a title), and Kshitij Modi, *Game UI Design: How Radial Menus Enhance Player Experience*, 300Mind, 2026, https://300mind.studio/blog/radial-menus-in-game-design/ (the practice guide) — craft.
2. *The Sims* pie menu, quoted from the Sims chapter of *Procedural Storytelling in Game Design* (home-still): "In The Sims, the pie menu provides this" (an "explosion of available choices") — shipping game. Gameface UI's `RadialMenu` (Coherent Labs' UI middleware for game engines) is the reference for the API a game expects: https://gameface-ui.coherent-labs.com/components/complex/radial-menu/ — shipping-game UI middleware.
3. Jason Gregory, *Game Engine Architecture*, 3rd ed., section 9.5.2 "Dead Zone" (home-still), and Gameface's `selectByVector` — technical (analog input and its dead zone).

## Craft principle

The 2011 study states why a ring suits a stick: "In radial selections, the ratio of amplitude to width is a trigonometric function of the number of items, n, in the radial menu (since the width of the pie segment increases as the user moves further from the centre)" and "the Fitts' Law prediction for radial selections is essentially a logarithmic function of the number of radial menu items". A wedge is a wide target, and every item is the same distance away.

The practice guide gives the three rules this widget follows. "Every item in a radial menu sits at an equal distance from the center", so selection is by direction ("players choose an option by moving in its direction rather than scrolling through a list"). "For most gameplay scenarios, limiting the menu to 6–10 options provides a good balance" ("Beyond that, segments become too thin to select accurately, especially on a gamepad"). And on confirmation: "a radial menu can feel sluggish if players must first hover over an option and then press a separate confirmation button. … release-to-select interactions create a faster and more fluid experience by completing the selection in a single motion."

## How shipping games do it

The Sims builds its whole interaction on the ring: the pie menu shows every verb a Sim can do to a thing, around the thing, and is chosen by direction. Gameface UI's component shows the contract a game expects: "It supports mouse and controller input, visual indicators, and customizable slice spacing, making it ideal for in-game weapon wheels", with `selectByIndex`, `onChange` ("Callback triggered when a new item becomes selected") and `selectByVector`: "Selects a menu item based on normalized vector coordinates (range -1 to 1), useful for controller or joystick input. **Includes an internal deadzone for stable selection.**" A weapon wheel also names the highlighted item in its middle ("RadialMenu.Content … the circular space at its center … icons, labels, player stats, or contextual info"), so the player reads the choice where the thumb is, not on the ring.

## Model adopted

- **R1. `radial(items, selected)`** is a disc with the items on a ring, item 0 at the top, clockwise. The root is Bevy's `Button` and a tab stop (`AutoDirectionalNavigation`), like `choice`, so Enter, Space, South and a click arrive as `Activate`. Its state is `RadialValue(usize)` (the highlighted item) and `RadialCount`.
- **R2. One event, `ValueChange<usize>`.** Moving the highlight sends one with `is_final: false`; confirming (`Activate`) sends one with `is_final: true` for the highlighted item, even when it did not move. A game applies on the final one; a game that previews as the player turns uses the first. Release-to-select is the game triggering `Activate` on the radial when its hold button is let go.
- **R3. Direction, never distance.** A stick pushed past `RADIAL_STICK_PUSH` (either stick of any pad) highlights the item whose wedge it points into (`model::radial_pick`); inside that dead zone it changes nothing and the highlight stays. The pointer is the same: the item a pointer over the disc points at, with a dead circle in the middle (`RADIAL_DEAD_ZONE`) that holds the highlight. The whole wedge is the target, so the ring's items are not 44 px buttons to hit but a quarter, an eighth of the disc.
- **R4. Keys and D-pad step.** Right and Down (D-pad too) go to the next item, Left and Up to the previous, wrapping; Home and End go to the ends. A wheel of 8 has no eight arrows, and APG's radio group wraps on all four. The arrows move no focus (the radial keeps them); Tab leaves.
- **R5. The centre says the choice** in the heading size, so the player reads it under the thumb. The ring's items are short labels.
- **R6. Only the lit item is accent.** The highlighted item takes the accent fill with inverted text (the selected tab's look), the ring carries the focus ring on its border while focused.

## Parameters

`RADIAL_STICK_PUSH` = 0.5 (a deliberate push, the same half as `STICK_PUSH`), `RADIAL_DEAD_ZONE` = 0.25 of the disc's radius for a pointer, disc diameter 26 text sizes, the ring at 40% of the diameter, so a wheel follows `text_scale` and the player's interface size. Design defaults; the guide's "6–10 options" is the stated bound, `tests/radial.rs` lays out 8 labelled items without overlap and 10 without any item leaving the disc.

## Validation targets

Item `i` of `n` is picked by a stick pointing at angle `i·360°/n` from up, at every `n` from 1 to 12, and boundaries go to the clockwise neighbour; a push inside the dead zone changes nothing. The pointer over the wedge highlights it and a click confirms exactly once with `is_final: true`; the pointer in the centre changes nothing. Arrows, D-pad, Home and End step and wrap and move no focus; a disabled radial does nothing. The selected item alone is lit and the centre reads its label. The disc is square at every text size from 100% to 200% and no item leaves it; no two items of eight overlap. Thirty idle frames write nothing.

## Deviations

A stick wheel in many games uses the right stick; this one takes either, whichever is pushed, because a menu has no camera to protect (design default). No wedge fills: bevy_ui has no sector shape, so the ring is items on a disc, the highlighted one accent (Gameface clips CSS shapes into wedges; a wedge mesh would be a render dependency this crate does not have). No nested rings or per-item weights ("allocating them more angular space"): a game with more than a ring's worth uses a second radial or a `select`. Recorded in the log.

## What great looks like

A round dark disc in the middle of the screen, eight pills on a ring, the one the stick points to filled with the accent and its name large in the middle, the focus ring around the disc; at 200% text the same wheel, larger, nothing touching. Turning the stick moves the highlight with no flicker at the centre.

## Pending index

The three sources are in home-still except the 300Mind article and Gameface's page (fetched today, web): `paper_download` cannot take them.
