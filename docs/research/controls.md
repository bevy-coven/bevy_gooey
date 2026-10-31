# Settings controls: a toggle and a slider

Researched 2026-10-30 (iteration 10). No issue is open; this is the loop's own pick (the missing widgets every options screen needs, ranked above the look items in the log's backlog). home-still's distill answered eight searches ("game settings menu slider toggle controller navigation usability", "toggle switch versus checkbox interface design", "slider touch target size thumb accessibility", "game menu design accessibility options volume slider", "direct manipulation … immediate feedback", "slider control continuous value adjustment dragging", "the player adjusts settings in a menu with the D-pad"); none returned a document on settings controls, so every quote below is a web copy fetched this pass and the gap is under `## Pending index`.

## Sources

1. Kendrick, A. (2018). *Toggle-Switch Guidelines.* Nielsen Norman Group — https://www.nngroup.com/articles/toggle-switch-guidelines/ (craft: when a switch, how it must read).
2. Laubheimer, P. (2017). *Input Controls for Parameters: Balancing Exploration and Precision with Sliders, Knobs, and Matrices.* Nielsen Norman Group — https://www.nngroup.com/articles/sliders-knobs/ (craft: what a slider is good for, why it is hard to hit).
3. Microsoft, *Xbox Accessibility Guideline 107: Input* — https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/107 (shipping games: Minecraft's settings sliders, Gears 5's controller options, Grounded).
4. W3C WAI, *WAI-ARIA Authoring Practices Guide*, the *Switch Pattern* — https://www.w3.org/WAI/ARIA/apg/patterns/switch/ — and the *Slider Pattern* — https://www.w3.org/WAI/ARIA/apg/patterns/slider/ (technical: the keyboard contract of each).
5. Bevy 0.20.0 source: `bevy_ui_widgets` `checkbox.rs` (`Checkbox`, `ToggleChecked`, `checkbox_self_update`), `slider.rs` (`Slider`, `SliderThumb`, `SliderValue`, `SliderRange`, `SliderStep`, `TrackClick`, `SetSliderValue`, `slider_self_update`, `slider_on_key_input`) and `lib.rs` (`ValueChange`) (technical: what is headless and what the skin must draw).
6. Buitinck, L. et al. (2013). *API design for machine learning software: experiences from the scikit-learn project* (corpus stem `W1728842521`) (technical: the API's defaults).

## Craft principle

Kendrick, on what a switch promises: "Toggle switches should take immediate effect and should not require the user to click Save or Submit to apply the new state." A game's options are exactly that case (Fullscreen, Subtitles, Invert Y apply the moment they flip), so the widget is a switch, and its `ValueChange<bool>` is the moment the game applies the setting. On how it must look: "when a user changes the state of a toggle, the switch should change position — as it would in the real world", and "always use a high-contrast color to signal state change". The state is the knob's place first and its colour second, so it survives a colour-blind player and a washed-out display.

Laubheimer, on what a slider is for: "Sliders are effective when users can scrub through the range of the control and see the effects in real time"; on its weakness: "Most sliders have a narrow width, so users often have a hard time to hit a specific spot accurately" (the Accot–Zhai steering law: the time to steer through a tunnel depends on its length and width). A volume or brightness slider is a scrub, not a typed number, so the whole row is the tunnel, not a 24 px bar, and every change is announced as it happens (`is_final: false` while dragging) so the game can play the sound at the new level.

## How shipping games do it

Microsoft's XAG 107 uses a shipped game for the rule a settings slider must keep: "While configuring settings in Minecraft, only single key presses are required to adjust settings—even slider inputs. In contrast, for some games, a player must hold down a button (such as holding down “A”) while adjusting the slider left or right so that the slider doesn't lose focus. In this example, the player can simply tap “A” to select once, and then use either the thumb stick, or single digital inputs from the D-pad, to adjust the slider left and right. There's no instance where multiple buttons should be pressed simultaneously or held down to complete actions."

The same page shows the shape of a shipped options row: Gears 5's controller options, "The player is focused on the "invert y-axis" option. The current value is "off"." A label on the left, its state on the right, the whole row focused; Grounded's keyboard settings and The Long Dark's "accessible interactions" are the same row. So a row is a label and its control, the row is the focus target, and the pad needs no mode to change a value: a slider in focus takes Left and Right.

## Model adopted

- **C1. The widget is the entity.** `toggle(label, on)` and `slider(label, value, range)` each return one bundle whose root is Bevy's own widget entity (`Checkbox`, `Slider`), the way `button()` is Bevy's `Button`: `ValueChange` is triggered on it, `observe(..)` and `Tooltip` work on it, `InteractionDisabled` on it disables it, and `SliderValue` and `Checked` are read from it. Buitinck et al.: "Sensible defaults. Whenever an operation requires a user-defined parameter, an appropriate default value is defined by the library." Both bundles carry Bevy's self-update observer (`checkbox_self_update`, `slider_self_update`), so the widget moves by itself and the game observes `ValueChange` only to apply the setting.
- **C2. A toggle is a row: label, then a switch** (a framed track with a knob), the whole row the hit target (at least `min_target` high) and the focus ring (the button's, in the row's border). Off: the knob at the start, `text_muted` on a `surface_raised` track; on: the knob at the end, `accent_text` on an `accent` track. The track is framed in `outline` like the meter, one line wide with one line of gap. APG: "it is critical the label on a switch does not change when its state changes": the label is static text, and Space (and Enter) change the state, which is Bevy's checkbox and needs nothing here. South on the pad does the same (`gamepad_select`).
- **C3. A slider is a row: label and value on one line, the track under them.** The track is a meter-high framed rail ended by a thumb as tall as 1.5 rails; the fill runs from the start to the thumb's centre. The value reads as a percentage when the range is 0 to 1 (a volume, a brightness) and as the plain number otherwise, trimmed to two decimals (`model::slider_text`). A click anywhere on the row snaps the thumb to that x and a drag goes on from there (`TrackClick::Snap`), so the tunnel is the whole row (Laubheimer).
- **C4. Left and Right adjust a focused slider, Up and Down still move.** Keys: Bevy's `slider_on_key_input` steps on Left and Right, Home and End jump (APG). The pad: the D-pad's Left and Right step a focused, enabled slider through Bevy's `SetSliderValue`, and a held D-pad repeats after a short delay (`model::repeat_steps`), so a full sweep is not twenty presses (XAG 107: "single digital inputs from the D-pad, to adjust the slider"). `navigate_by_input` does not move focus sideways off a slider, or a Left press would adjust the value and also leave the control. The step is a twentieth of the range (5 % of a volume), replaceable by inserting a `SliderStep`.
- **C5. Colour pairs the skin already checks.** The panel's `text` pair covers labels, `outline` on `surface` the frame, `accent` on `surface_raised` the on state, `accent_text` on `accent` the on knob; the one new pair is the off knob, `text_muted` on `surface_raised`, at the 3:1 non-text minimum in `GooeySkin::problems()`. The slider's thumb is `text` with a keyline in the panel's solid surface, so one of its two colours always contrasts with the fill, the track and the panel behind it, in both presets.

## Parameters

- Toggle track: 1.5 text sizes high (30 px at the default 20 px, times the player's text scale), twice as wide; knob = track − 4 lines (the meter's rule); row padding as a button's.
- Slider rail: `meter_height` high; thumb: 1 text size wide and 1.5 rails tall; value readout in the body size.
- Slider step: a twentieth of the range. D-pad repeat: first repeat after 0.4 s, then 12 steps a second. Both are design defaults: the OS keyboard repeat Bevy forwards for keys is 0.5 s then about 30 per second, and a pad has none; a 0 to 1 slider then sweeps in about 2 s.
- No `GooeySkin` field is added: sizes derive from `text_size`, `meter_height`, `border`, `padding` and `text_scale`.

## Validation targets

- Toggle at 1280x720: the row at least 44 px high; label text at least 4.5:1 (10.26:1 dark, 8.19:1 parchment on its surface); off knob on track and on track on off track at least 3:1; frame, gap and knob on whole device pixels at 1x, 1.5x and 2x; state read from the knob's place alone (a greyscale capture still separates on from off).
- A click on the track, the knob or the label flips it once and gives it focus; Space and Enter flip it; South flips it; a disabled toggle does nothing and reads muted.
- Slider: the thumb's centre within 1 px of `value` fraction × the thumb's travel at 720p, 1080p and 1440p; a click at x leaves the thumb's centre within 3 px of x (the border's drift, see Deviations); a drag moves the value by `distance / (width − thumb) × span`.
- A focused slider: Right adds one step, Left subtracts one, Home and End jump, Up and Down move focus to the neighbours and never change the value; the D-pad does the same and a held D-pad repeats (first repeat 0.4 s, then 12 per second); a disabled slider is skipped by all of it.
- Every change is a `ValueChange<f32>` with `is_final: false` while dragging and `true` at the end.
- Frame time: no per-frame system touches a slider or toggle that did not change (the readout and thumb update on `Changed<SliderValue>`).
- `GooeySkin::problems()` is empty for both presets with the new pair, and its tests that break one colour still report exactly one.

## What great looks like

A settings column that reads like a console's: label left, state right, a switch whose knob visibly slides to the side and fills with the accent, a slider whose thumb is easy to grab at couch distance and whose number changes under it, one ring around whichever row has focus, the same ring as a button's. On the pad, Down through the rows and Left and Right on the volume, with no A-hold. Two skins, one family.

## Deviations

- Bevy's slider maps a click and a drag onto the width of the slider node, and assumes the thumb travels the whole node. The row has the focus ring as its border, so the thumb's travel is the node's width minus two borders (6 px at the default line): a click lands within 3 px of the pointer and a full drag drifts by at most 6 px. Padding the row sideways would multiply that, so the row has no horizontal padding; the rail is inset by half a thumb so its ends are the thumb's centres at 0 and 1. A design default, measured in the tests.
- APG's slider also takes Up and Down as increase and decrease and Page Up and Page Down. Up and Down move focus here, because in a column of settings they must (Minecraft's and Gears 5's rows do the same), and Page Up and Down are not added (optional in APG, and `bevy_ui_widgets` has none).
- NN/g suggests an "On"/"Off" descriptor beside a switch when colour alone is not enough; the knob's place carries the state here, and a state word is left to the game's own label ("Subtitles"), as in Gears 5's row. A design default.
- Not adopted: a checkbox (a square with a tick) as a second widget; a game's options apply immediately, which NN/g gives to the switch. `Checkbox` is Bevy's component and a game that wants the tick can put it on its own node.
- Not adopted this pass: the thumb and track reacting to hover, a "reset to default" mark (NN/g's tick for the default value), vertical sliders, a narration label (accesskit labels) for the new nodes.

## Pending index

- Index into home-still: NN/g's two articles above, Microsoft's XAG 106 and 107, the APG switch and slider patterns; a paper or talk on settings-screen design in shipped games (Hades, Celeste, Doom Eternal, Gears 5 options menus) and on game-menu gamepad navigation with hold-to-repeat; `paper_search` found nothing on either this pass.
- `catalog_recent` (limit 200) on 2026-10-30 21:0x held only embed events for biomedical journals, the *Advances in Real-Time Rendering* course notes, the PCG book and software-engineering books: no game-UI, HUD, menu, navigation or accessibility document that `corpus.md` lacks, so it is unchanged. `hs status`: both scribes and distill healthy, distill idle on CUDA; searches answered in 4 to 12 s.
