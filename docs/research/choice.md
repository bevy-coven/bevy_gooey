# The choice: one option out of a few, stepped with Left and Right

Researched 2026-10-31 (iteration 11), built and measured the same day (iteration 12). home-still's corpus has nothing on settings controls (`hs distill search`, four queries on steppers, option selectors, radio buttons against drop-downs and gamepad settings rows returned papers on exercise machines, VR menus and slot machines; `catalog_recent` limit 200 held no game-UI document; iteration 12 ran two more, "option cycling widget gamepad settings menu" and "stepper left right arrows select one of few values interface", and `catalog_recent` again: nothing), so the three kinds of source below are web copies. Every quotation below was fetched again in iteration 12 and matches; the gap is under `## Pending index`.

## Sources

1. Nielsen Norman Group, *Design Guidelines for Input Steppers* (2018) — https://www.nngroup.com/articles/input-steppers/ (craft: how a stepper is built so it can be hit, read and kept inside its range).
2. Kaley, A., Nielsen Norman Group, *Listboxes vs. Dropdown Lists* (2020) — https://www.nngroup.com/articles/listbox-dropdown/, and Nielsen, J., *Checkboxes vs. Radio Buttons* (2004) — https://www.nngroup.com/articles/checkboxes-vs-radio-buttons/ (craft: when one visible value is the right trade against all options visible).
3. Microsoft, *Xbox Accessibility Guideline 107: Input* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/107 (shipping games: Minecraft's and Gears 5's settings screens).
4. W3C WAI, *ARIA Authoring Practices Guide: Spinbutton Pattern* — https://www.w3.org/WAI/ARIA/apg/patterns/spinbutton/ (technical: the keyboard contract of a control that steps through a set of values), and the Bevy 0.20.0 source, `bevy_ui_widgets-0.20.0/src/radio.rs` (technical: why Bevy's `RadioGroup` is not the base).

## Craft principle

NN/g on input steppers: "Use large buttons for both desktop and mobile. Target areas should be big enough to support the input modality." And: "Clarify the step value and the stepper range … if the input value has a maximum or minimum limit, be sure to indicate it." And: "Expedia used a grayed-out plus segment to signal when that the value had reached its limits."

NN/g on which control shows one value: for a dropdown list, "Because dropdown lists hide the other available options, they work well to downplay alternatives and underemphasize the ability to make changes. (This is advantageous in cases where the default will satisfy most users and where the alternative options may be dangerous or confusing to nonexpert users.)" Its table: "Seeing the selected item is more important than viewing the other available options" → "Use a dropdown list."

## How shipping games do it

XAG 107, on Minecraft's settings: "While configuring settings in Minecraft, only single key presses are required to adjust settings—even slider inputs." And its rule for every game: "A UI should be navigable by using single, non-simultaneous key presses." On Gears 5's controller options the guideline shows the row as a label and its current value, one option at a time: "The player is focused on the 'Button Tap Challenges' option. The current value is 'press & hold'" and, one press later, "'quick taps'". The press is a *tap on a direction*, never a held button: "There's no instance where multiple buttons should be pressed simultaneously or held down to complete actions."

Also XAG 107: "The *down click* or *down press* … on an item doesn't automatically activate that item. Activation occurs on the *up event*", so a click on a chevron steps when the button is released, and a press that slides off cancels.

## Model adopted

APG, spinbutton: "Spinbuttons often have three components, including a text field that displays the current value, an increase button, and a decrease button. The text field is usually the only focusable component because the increase and decrease functions are keyboard accessible via arrow keys." And: "`Home`: If the spinbutton has a minimum value, sets the value to its minimum. `End`: … maximum."

- **C1. One tab stop, the row.** The chevrons are pointer targets only; Tab, arrows and the D-pad land on the row, which carries the focus ring like a toggle's and a slider's. With it focused, **Left and Right step back and forward** (keys through an observer on the focused entity, the D-pad through `gamepad_adjust`, held to repeat), **Home and End** jump to the first and the last option, and **Up and Down move focus on**, as on a slider (`navigate_by_input` leaves Left and Right to it). NN/g: "up and down, left and right should match the button layout on the screen."
- **C2. The ends are ends.** A step past the first or the last option does nothing and reports nothing; the chevron that cannot step dims (`text_muted`), like Expedia's grayed-out segment.
- **C3. Targets a thumb can hit.** Each chevron sits in a target at least `min_target` wide and as high as a toggle's row, and a click on it, released over it, steps once. A press on a dimmed chevron does nothing and is not read as a click on the row.
- **C4. Activate cycles.** Enter, Space, the pad's South button and a click on the label or the value move to the next option and wrap from the last to the first, so every option is reachable by one repeated single press (XAG 107), however the player's input is limited. Left and Right never wrap: a player who steps past the end must see that it is the end.
- **C5. The row does not move.** The value's cell is as wide as the widest option (every option is laid out in the same grid cell and only the selected one is shown), so stepping never shifts the chevrons under the player's thumb.
- **C6. A drawn chevron, not a glyph.** The host supplies the font, and a font's `‹` may be missing; the chevron is a node, two borders of the skin's line weight on a rotated square, so it is one weight with every other line (`docs/research/lines.md`) and scales with the text size.
- **C7. The widget keeps its state, the game hears changes.** `ChoiceValue(usize)` on the entity is the selected index; `ValueChange<usize>` (`is_final: true`) is sent from it whenever a press, key, pad or the game's `Activate` changes it, like a toggle's and a slider's. `UiFeedback::Activate` is written for each change and `Focus` when it gains focus.

## Parameters

No new field in `GooeySkin`; every size derives from the existing ones.

|part|size|
|---|---|
|row|border `border`, padding `padding × 1.5` left, the right padding what leaves the forward chevron's ink where a toggle's switch ends (`padding × 1.5 − (arrow − ink) / 2`, 2.5 px at 720p), none above and below, `radius`, `gap` between the label and the stepper; as high as a toggle (`switch + 1.2 × padding + 2 × border` = 52.8 px at 720p)|
|arrow target|`min_target` wide (44 px), `switch + 1.2 × padding` high (46.8 px, at least `min_target`)|
|chevron|a square of 0.6 text sizes (12 px; the note first said 0.5, see Deviations) with two borders of `line`, turned an eighth of a turn, about 8.5 × 17 px of ink (10 × 16 px as drawn, the stroke's width included), centred in its target|
|value|body text; the cell is the widest option wide, plus half a `gap` each side|
|colours|chevron `text`; under the pointer `accent`; at an end or disabled `text_muted`; the value `text`|

## Validation targets

- Row 53 px high at 1280x720 (equal to a toggle's), 106 px at 2560x1440; label at the same x as a toggle's label; the right chevron's ink ends where a toggle's track ends (within 2 px).
- Chevron targets at least 44 x 44 px at 1280x720.
- The row's width and the chevrons' x do not change while stepping through every option (0 px).
- Chevron ink centred in its target within 1 px; stroke one `line` wide (3 px, 6 px at 2560x1440).
- Chevron on the panel at least 3:1 (non-text); a dimmed chevron plainly dimmer than a live one.
- Left and Right step by one and stop at the ends with no event; Home and End jump; Enter, Space and South cycle and wrap; a click on the right chevron steps once and a click on a dimmed one does nothing; Up and Down move focus; a disabled choice ignores all of it; each change sends exactly one `ValueChange<usize>` and one `UiFeedback::Activate`.
- Frame time within 0.5 ms of the gallery before this change.

Measured in iteration 12, 1280x720, dark (parchment alike): row 53 px high, the same as a toggle (the ring, 3 px, spans y 463 to 515); the labels of the Subtitles, Quality and Volume rows start at x 517; the forward chevron's ink ends at x 762 and the switch's track at x 762 (0 px); chevron ink 10 × 16 px, centred in its arrow within 1 px (`tests/choice.rs`); arrows 44 × 46.8 px; a live chevron 14.22:1 on the panel (parchment 10.20:1), a spent one 6.64:1 (4.84:1), the value 14.22:1 (10.20:1); the row, the arrows and the value cell do not move through every option (0 px, a 17-character option beside two short ones). The sources of the numbers are in the iteration 12 entry of `docs/loop-log.md`.

## What great looks like

A settings row that reads at a glance and works without a manual: the label on the left, the current value on the right between two crisp chevrons that brighten under the pointer and dim at an end, on the same rhythm and the same left edge as the switch above it and the slider below it. With a pad in hand, Left and Right step it and Down goes on to the next row; with a mouse, the chevrons are big and the value never jumps sideways. It looks like the rest of the family: one line weight, one focus ring.

## Deviations

- NN/g (Nielsen 2004): "If possible, use radio buttons rather than drop-down menus. Radio buttons have lower cognitive load because they make all options permanently visible", for 5 options or fewer. A stepper shows one option, as a dropdown does. Not adopted for a game's settings row: one tab stop and one Left/Right gesture is what a pad needs, the options of a graphics or difficulty setting are ordered and the player rarely compares them side by side, and the same note allows it: "Limited space might sometimes force you to violate this guideline". A segmented, all-visible form is a separate item in the backlog.
- NN/g on steppers: "Arrow left/right buttons are a possibility for horizontal steppers, but are rarely recommended because they have less of a direct connotation with the concept of making something larger or smaller." That is about numbers; a choice is an ordered list of names (Low, Medium, High), where previous and next are the meaning, and a number is the slider's. Chevrons it is.
- NN/g: "Allow long press or click on the buttons for faster continuous increment or decrement." Held on the D-pad (the existing repeat), not on the pointer's chevrons: a choice has a few options and a click on the value cycles. A design default.
- APG would adjust the value with Up and Down; Up and Down move focus off the row (assumed decision of iteration 10, kept so a pad never has to leave a column of settings by another route).
- Wrapping on Activate and not on Left and Right is a design default (a cycling button is the common idiom for a setting that offers a few options; a stepper that wraps hides where the list ends).
- Bevy's `RadioGroup` is not the base: it wraps around on every arrow, takes Up and Down as well as Left and Right (so the vertical focus move of `navigate_by_input` would act twice), keeps no state (the app must set `Checked` on each button) and has no one-value layout.
- Built in iteration 12, differing from the first draft of the parameters: the chevron is 0.6 text sizes, not 0.5 (a 7 px wide stroke pair read light beside 14 px text at 3x zoom; 10 × 16 px as drawn is taller than the text's capitals), and the value cell has half a `gap` of air each side, not a whole one (the chevrons stood 30 px from the value and read as two buttons beside a word). Design defaults, `CHEVRON_TEXT_SIZES` in `src/skin.rs` and the `ChoiceCell` arm.
- The row's root is Bevy's headless `Button` carrying `ChoiceValue` and `ChoiceCount`, so Enter, Space, the pad's South button and a click on the row are Bevy's own `Activate` (one handler, nothing acts twice) and this crate turns that into "next, wrapping". A game that observes `Activate` on a choice hears the cycles but not the steps; the docs say to observe `ValueChange<usize>`, which every change sends once. A design decision (the alternative, a bespoke press handler, would duplicate `bevy_ui_widgets`).
- `ChoiceCount` is a public component so that `update_choices` can be a public system like `update_sliders`; the options of a choice are fixed for its life. A design default.
- A press on an arrow stops at the arrow (the row is never pressed; Bevy's `AcquireFocus` still goes up from the press's original target, so the row takes focus and the ring hides); the step happens on release over the arrow (XAG 107's up event, which `PointerClick` is). An arrow that cannot step does nothing and says nothing.
- A held Left or Right key repeats at the operating system's rate, as a slider's does (Bevy's slider does not filter `repeat` either); the D-pad repeats through `gamepad_adjust` and stops at the end. A design default.
- The accessibility node stays Bevy's `Button`'s; a spin-button role with a value text ("Quality, Medium") is a backlog item with the slider's and the toasts'.

Assumed decisions (owner may override), all above: the chevron and cell sizes, the `Button` root, public `ChoiceCount`, the arrows' release-activated step, key repeat.

## Pending index

- Nothing relevant in home-still (two more queries in iteration 12, and `catalog_recent` limit 200: no game-UI document). Index NN/g's *Design Guidelines for Input Steppers*, *Listboxes vs. Dropdown Lists* and *Checkboxes vs. Radio Buttons*, XAG 107 and the APG spinbutton pattern. Search later for: "gamepad settings menu design", "option cycling widget". The Game Accessibility Guidelines and *The Gamer's Brain* remain pending from `docs/research/gooey.md`.
