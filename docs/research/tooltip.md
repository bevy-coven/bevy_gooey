# The tooltip: beside its widget, for every input

Researched 2026-10-30 (iteration 2). home-still's distill answered `500: gpu busy` again (`4867 MB free < 5000 MB required to load bge-m3`, beside a 14.6 GB vLLM engine), so semantic search was unavailable; the corpus stem *Supporting Novice to Expert Transitions in User Interfaces* (`10.1145_2659796`) was read whole and is a 40 kB partial conversion that never mentions tooltips. This note rests on the three web sources below and the vendored Bevy 0.20 source.

## Sources

1. Microsoft, *Xbox Accessibility Guideline 112: UI navigation* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112 (craft).
2. Lars Doucet, *The Steam Controller Configurator's Untapped Power*, Game Developer, 2017-01-25 — https://www.gamedeveloper.com/design/the-steam-controller-configurator-s-untapped-power (shipping game: the gamepad port of *Defender's Quest: Valley of the Forgotten DX*).
3. W3C WAI, *Understanding SC 1.4.13: Content on Hover or Focus* (WCAG 2.1, Level AA) — https://www.w3.org/WAI/WCAG21/Understanding/content-on-hover-or-focus.html (technical: the contract for transient content).
4. Bevy 0.20 source: `bevy_ui_widgets-0.20.0/src/popover.rs` (`Popover`, `PopoverPlacement`, `position_popover`), added to every app by `UiWidgetsPlugins` (technical: what the engine already does).

## Craft principle

XAG 112: "All aspects of the game, including menu UI's and gameplay, are fully navigable by multiple inputs (mouse, eye gaze, voice, keyboard, and controller)."

A tooltip shown only on pointer hover is an aspect of the menu a pad or keyboard player can never reach. And a tooltip that opens over the next control in the list hides the very thing the player is about to move to.

## How shipping games do it

Doucet, on porting *Defender's Quest* to the pad, lists the risk first: "Tooltips -- will players using a controller be at a disadvantage?" His answer is that the fixed menu cursor "stick[s] to the enemy waves, so that you can properly inspect the tooltips", and that the redesigned gamepad screen "is fully navigable with simple up/down/left/right menu commands (as well as the mouse), and provides all the same tooltip information as the conventional layout." On a pad, landing on a widget is what opens its tooltip; the mouse keeps working beside it ("I also like to stick tooltips on everything as an extra bit of polish and aid to understanding").

## Model adopted

- **T1. Focus opens a tooltip, as hover does.** A widget's tooltip shows while the pointer is over it (`Hovered`, which includes the tooltip itself, a child) **or** while it has focus and focus is visible (`InputFocus` and `InputFocusVisible`, Bevy's own "the player is on a keyboard or pad" signal). WCAG 1.4.13's trigger is the same pair: "Where receiving and then removing pointer hover or keyboard focus triggers additional content".
- **T2. It never covers the next control.** WCAG 1.4.13 asks for a way to dismiss transient content "unless the additional content … does not obscure or replace other content". The tooltip opens beside its widget, across the flow of its container: to the right in a column (a menu), below in a row (a bar). Bevy's `Popover` places it, trying the preferred side, then the opposite side, then the two along the flow, and keeps the first that the window does not clip, so a widget at the window's right edge opens its tooltip to the left.
- **T3. It stays while its trigger stays.** WCAG 1.4.13 *Persistent*: "The additional content remains visible until the hover or focus trigger is removed". No timeout.

## Parameters

- The gap between widget and tooltip is `GooeySkin::gap` (10 px in both presets), the same step a menu puts between its buttons, so a tooltip sits one spacing step away like a sibling. A design default.
- The tooltip keeps `gap` from the window's edge (`Popover::window_margin`). A design default.
- Side order: across the flow first (column → Right, Left; row → Below, Above), then along it. A design default resting on T2.
- The tooltip is centred on its widget across the main axis (`PopoverAlign::Center`): the eye reads it as attached to that widget, not to its neighbour. A design default.

## Validation targets

- With focus on a widget that has a `Tooltip` and focus visible, its tooltip is displayed; moving focus away hides it (before: never displayed without a pointer; `/tmp/g2/before/options-focus-1280x720.png`).
- In the gallery's column menu the tooltip's rectangle intersects no other button and is exactly `gap` px from its widget (before: flush against the widget's bottom edge, covering the next one).
- A widget at the window's right edge opens its tooltip to its left, inside the window.
- In a row the tooltip opens below its widget.
- Tooltip text against its fill: ≥ 4.5:1 (it is `text` on `surface_raised`, already checked by `problems()`).
- No measurable frame-time cost: one pass over the tooltips per frame, writing only on a change.

## What great looks like

On a pad, the player lands on "Options" and the line "Sound, controls and display" is beside it at once, aligned with the button's centre, one spacing step away, never over "Quit" below. The focus ring and the tooltip are read together. At the window's edge it flips without the player noticing. A game writes `Tooltip("…".into())` on a widget and nothing else: no observers, no input-mode code.

## Deviations

- WCAG 1.4.13 *Hoverable* ("the pointer can be moved over the additional content without the additional content disappearing"): the tooltip is a child of its widget, so hovering it keeps it open, but the pointer crosses the `gap` on the way and the tooltip closes there. The tooltip holds only text, never a control. A design default.
- WCAG 1.4.13 *Dismissible*: no dismiss key. T2 makes the tooltip obscure nothing, the exception the criterion names.

## Pending index

- distill was unavailable (`gpu busy`). Search the corpus for "tooltip", "contextual help gamepad", "on-demand information display game HUD", and cite *Supporting Novice to Expert Transitions* when its full text is converted (the stored `10.1145_2659796` markdown is 40 kB, cut off before its body).
- Index XAG 112, WCAG 1.4.13 and Doucet's article into home-still.
