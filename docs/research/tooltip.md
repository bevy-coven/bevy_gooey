# The tooltip: beside its widget, for every input

Researched 2026-10-30 (iteration 2). home-still's distill answered `500: gpu busy` again (`4867 MB free < 5000 MB required to load bge-m3`, beside a 14.6 GB vLLM engine), so semantic search was unavailable; the corpus stem *Supporting Novice to Expert Transitions in User Interfaces* (`10.1145_2659796`) was read whole and is a 40 kB partial conversion that never mentions tooltips. This note rests on the three web sources below and the vendored Bevy 0.20 source.

**Iteration 7 (issue #3, the lifecycle).** distill was healthy and idle (`hs status`: distill `idle (Cuda)`, two scribes converting, 18070 documents embedded). A `distill_search` for stale tooltip text returned tutorial-system papers and no tooltip source, so the craft and shipping sources below are web pages read in full on 2026-10-30.

## Sources

1. Microsoft, *Xbox Accessibility Guideline 112: UI navigation* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112 (craft).
2. Lars Doucet, *The Steam Controller Configurator's Untapped Power*, Game Developer, 2017-01-25 — https://www.gamedeveloper.com/design/the-steam-controller-configurator-s-untapped-power (shipping game: the gamepad port of *Defender's Quest: Valley of the Forgotten DX*).
3. W3C WAI, *Understanding SC 1.4.13: Content on Hover or Focus* (WCAG 2.1, Level AA) — https://www.w3.org/WAI/WCAG21/Understanding/content-on-hover-or-focus.html (technical: the contract for transient content).
4. Bevy 0.20 source: `bevy_ui_widgets-0.20.0/src/popover.rs` (`Popover`, `PopoverPlacement`, `position_popover`), added to every app by `UiWidgetsPlugins` (technical: what the engine already does).
5. Jakob Nielsen's group: Alita Kendrick, *Tooltip Guidelines*, Nielsen Norman Group, 2019-01-27 — https://www.nngroup.com/articles/tooltip-guidelines/ (craft: what a tooltip is for, and for what it must not do).
6. zSSkitzz and Everynone, *Update Widget Tooltip?*, Epic Developer Community Forums (Unreal Engine UMG), 2020-07-31 — https://forums.unrealengine.com/t/update-widget-tooltip/468949 (shipping practice: a game developer's inventory tooltip that outlives the state it describes).
7. Bevy 0.20 source: `bevy_ecs-0.20.1/src/lifecycle.rs` (`ComponentHooks::on_insert`, `on_discard`, `on_remove`) and `system/commands/mod.rs` (`EntityCommands::try_despawn`) (technical: when a hook runs, and when it does not).

## Craft principle

XAG 112: "All aspects of the game, including menu UI's and gameplay, are fully navigable by multiple inputs (mouse, eye gaze, voice, keyboard, and controller)."

NN/g: "A **tooltip** is a brief, informative message that appears when a user interacts with an element in a graphical user interface (GUI)… They provide descriptions or explanations for their paired element. Thus, tooltips are highly contextual and specific." A tooltip that outlives the state of its element is not that: a "Locked: reach level 5" hint that still opens on a button the player has unlocked tells the player the wrong thing about the control they are looking at, and it is the kind of "information pollution" NN/g warns against ("waste the time of any users unlucky enough to activate that tooltip").

A tooltip shown only on pointer hover is an aspect of the menu a pad or keyboard player can never reach. And a tooltip that opens over the next control in the list hides the very thing the player is about to move to.

## How shipping games do it

Doucet, on porting *Defender's Quest* to the pad, lists the risk first: "Tooltips -- will players using a controller be at a disadvantage?" His answer is that the fixed menu cursor "stick[s] to the enemy waves, so that you can properly inspect the tooltips", and that the redesigned gamepad screen "is fully navigable with simple up/down/left/right menu commands (as well as the mouse), and provides all the same tooltip information as the conventional layout." On a pad, landing on a widget is what opens its tooltip; the mouse keeps working beside it ("I also like to stick tooltips on everything as an extra bit of polish and aid to understanding").

The same failure is in every engine's first inventory screen. An Unreal developer: "if I make the tooltip to show the description of an item and the item gets used/deleted, the tooltip stays the same on the screen until your cursor moves away from the inventory" (Epic forum, *Update Widget Tooltip?*). The accepted answer was to hold a reference to the tooltip and "*remove it from parent*, and then set it to null": the owner of the widget, not the player, has to tidy it. A crate that spawns the box for the game should tidy it too.

## Model adopted

- **T1. Focus opens a tooltip, as hover does.** A widget's tooltip shows while the pointer is over it (`Hovered`, which includes the tooltip itself, a child) **or** while it has focus and focus is visible (`InputFocus` and `InputFocusVisible`, Bevy's own "the player is on a keyboard or pad" signal). WCAG 1.4.13's trigger is the same pair: "Where receiving and then removing pointer hover or keyboard focus triggers additional content".
- **T2. It never covers the next control.** WCAG 1.4.13 asks for a way to dismiss transient content "unless the additional content … does not obscure or replace other content". The tooltip opens beside its widget, across the flow of its container: to the right in a column (a menu), below in a row (a bar). Bevy's `Popover` places it, trying the preferred side, then the opposite side, then the two along the flow, and keeps the first that the window does not clip, so a widget at the window's right edge opens its tooltip to the left.
- **T3. It stays while its trigger stays.** WCAG 1.4.13 *Persistent*: "The additional content remains visible until the hover or focus trigger is removed". No timeout.
- **T4. A tooltip is a view of its component.** The box exists exactly while its widget has a `Tooltip`, and shows what `Tooltip.0` says now. Adding spawns one box; removing despawns it; inserting a `Tooltip` over another, or editing `Tooltip.0` in place (a reflection-driven inspector does), changes the box's text; removing and adding again leaves one box, never two. Bevy's own docs set the mechanism: `on_insert` "runs when this component is added (with `.insert`) or replaced" but "won't run if the component is already present and is only mutated, such as in a system via a query", and `on_remove` runs "when this component is removed from an entity. Despawning an entity counts as removing all of its components." So the hooks make and remove the box, and a `Changed<Tooltip>` system keeps its text, the one path that sees every edit.
- **T5. Hooks queue a check, not an action.** Both hooks queue the same command: "the box exists iff the widget has a `Tooltip` now", evaluated when the command runs, not when the hook fired. Two hooks fired in one flush (a remove then an insert) cannot stack two boxes or leave a doomed one, and a despawned widget (whose children Bevy despawns itself) is a no-op.

## Parameters

- The tooltip's outer edge sits `gap + padding + border` (27 px in both presets) from its widget's outer edge: one spacing step (`GooeySkin::gap`) outside the panel the widget usually sits in. A design default; see Deviations.
- The tooltip keeps `gap` from the window's edge (`Popover::window_margin`). A design default.
- Side order: across the flow first (column → Right, Left; row → Below, Above), then along it. A design default resting on T2.
- The tooltip is centred on its widget across the main axis (`PopoverAlign::Center`): the eye reads it as attached to that widget, not to its neighbour. A design default.

## Validation targets

- With focus on a widget that has a `Tooltip` and focus visible, its tooltip is displayed; moving focus away hides it (before: never displayed without a pointer; `/tmp/g2/before/options-focus-1280x720.png`).
- In the gallery's column menu the tooltip's rectangle intersects no other button and its edge is `gap + padding + border` px from its widget, one `gap` outside the panel (before: flush against the widget's bottom edge, covering the next one).
- A widget at the window's right edge opens its tooltip to its left, inside the window.
- In a row the tooltip opens below its widget.
- Tooltip text against its fill: ≥ 4.5:1 (it is `text` on `solid_surface()`, the opaque panel fill, already checked by `problems()`).
- At `text_scale` 1.5 in a centred menu the tooltip still opens beside the widget (wrapped), not over the next one.
- No measurable frame-time cost: one pass over the tooltips per frame, writing only on a change.
- A `Tooltip` removed from a widget with visible focus: no box opens (before the fix the orphaned box opened with the old text: issue #3's test fails on its first assertion).
- `remove::<Tooltip>()` then `insert(Tooltip(..))` leaves one `TooltipBox`, with the new text (before: two).
- `insert(Tooltip(..))` over an existing one and an in-place edit of `Tooltip.0` both change the text of the box that is already there, the same entity, so an open tooltip does not blink (before: ignored).
- A removed `Tooltip` on a hovered widget closes at once; no frame cost beyond one `Changed<Tooltip>` filter.

## What great looks like

On a pad, the player lands on "Options" and the line "Sound, controls and display" is beside it at once, aligned with the button's centre, one spacing step away, never over "Quit" below. The focus ring and the tooltip are read together. At the window's edge it flips without the player noticing. A game writes `Tooltip("…".into())` on a widget and nothing else: no observers, no input-mode code.

A game unlocks "Options" and writes `commands.entity(button).remove::<Tooltip>()` or `insert(Tooltip("Open"))`, and what the player sees follows on the next frame: the hint is gone or reads the new line, in place, with no second box under it and no leftover from the old one.

## Deviations

- WCAG 1.4.13 *Hoverable* ("the pointer can be moved over the additional content without the additional content disappearing"): the tooltip is a child of its widget, so hovering it keeps it open, but the pointer crosses the `gap` on the way and the tooltip closes there. The tooltip holds only text, never a control. A design default.
- WCAG 1.4.13 *Dismissible*: no dismiss key. T2 makes the tooltip obscure nothing, the exception the criterion names.
- Changed after the first captures: the gap was planned as `gap` (10 px) from the widget. At 10 px the tooltip straddled the gallery's panel border (panel edge at x 752, tooltip from x 743), which read as a box cut through another box's frame; it now opens one `gap` outside the panel (tooltip edge at x 763). Measured from the widget, that is `gap + padding + border`. `Popover` measures from inside the widget's border, so `apply_skin` passes `gap + padding + 2 × border`.
- Changed after the first captures: the tooltip's fill was `surface_raised`, a button's resting fill, so the open tooltip read as a fourth button. It is now drawn as a small panel: `solid_surface()` with a `surface_raised` border of `border` px, `padding` × `padding / 2` inside, and a drop shadow in `background` at 0.7 alpha (y offset `2 × border`, blur `padding`). A design default.
- Iteration 7, a design default: the text is updated in place rather than by despawning and respawning the box on every insert (the issue's fix direction). A respawned box starts `Display::None` and is restyled a frame later, so a tooltip that is open while its text changes would blink; the in-place update keeps the entity. Removal still despawns. The hooks are `on_insert` and `on_remove`, not the issue's `on_replace`: in Bevy 0.20 the hook that runs "when this component is about to be dropped, such as being replaced (with `.insert`) or removed" is `on_discard`, and it runs on a replace, where the box must stay.
- Changed after the first captures: at `text_scale` 1.5 the one-line tooltip (≈ 520 px) fitted neither side of the centred menu, so `Popover` fell back to Below and covered Quit (`/tmp/g2/after/dark-large-options-1280x720.png`, first round). The box is now at most 24 text sizes wide (about 40 characters, a comfortable line) and, in a column, no wider than the room the window leaves on the widget's roomier side; under 8 text sizes of room it keeps the 24-size measure and falls back. A design default.

## Pending index

- distill was unavailable (`gpu busy`). Search the corpus for "tooltip", "contextual help gamepad", "on-demand information display game HUD", and cite *Supporting Novice to Expert Transitions* when its full text is converted (the stored `10.1145_2659796` markdown is 40 kB, cut off before its body).
- Index the NN/g *Tooltip Guidelines* and the Epic forum thread (`forums.unrealengine.com/t/update-widget-tooltip/468949`) into home-still: no `distill_search` hit for "tooltip" in the game-UI corpus.
- Index XAG 112, WCAG 1.4.13 and Doucet's article into home-still.
