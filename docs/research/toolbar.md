# Toolbar, icon buttons and toggle groups

Researched 2026-11-02 (iteration 38, #24 G-20). home-still's index has nothing on toolbar patterns (the 2026-11-02 corpus re-scan found no game-UI paper), so the sources below were fetched today.

## Sources

1. W3C, *WAI-ARIA Authoring Practices, Toolbar Pattern* (and *Managing Focus Within Components Using a Roving tabindex*), https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/ — craft.
2. Xbox Accessibility Guideline 103 (*Visual distinction*) and 112 (*UI navigation*), https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/103 — shipping games.
3. Bevy 0.20 `bevy_ui::ComputedNode::content_box`, `bevy_ui_widgets::Button`/`Activate`, and this crate's `menu.md`, `tabs.md`, `icons.md` — technical.

## Craft principle

APG, Toolbar: "Implement focus management so the keyboard tab sequence includes one stop for the toolbar and arrow keys move focus among the controls in the toolbar." And for an overflowing bar: "the aria-haspopup attribute of the button that opens the overflow popup should be set to \"menu\"", i.e. what does not fit is reached through one button that opens a menu.

## How shipping games do it

Xbox 103: "Text labels can help take the guesswork out of what an icon or symbol is intended to indicate, and instead states the intent in words." A toolbar of bare glyphs is therefore paired with a name the player can reach by hover and by focus: here a `Tooltip` (with its shortcut, #12). XAG 112: "The UI is fully navigable by keyboard and controller digital input alone": every control is reachable with the arrows and the D-pad.

## Model adopted

- **T1. `toolbar(items)`** is a full-width row (`Role::Toolbar`, a menu bar's look: the solid surface, one quiet line under it). Its children are its items, in order. It adds nothing between them but a half gap.
- **T2. `icon_button(IconSource, Tooltip)`** is a `button()` whose only child is an `icon()`: a target square, the icon tinted by the button's state, the tooltip's words as its name. The tooltip is required: an icon with no words is the case XAG 103 names.
- **T3. `toggle_group(options, selected)`** is a row of `GroupOption`s (an icon with its tooltip, or a label) with radio behaviour: exactly one is lit (`Checked`, the accent fill like a selected tab). The widget's state is `ToggleGroupValue`; activating another option sends one `ValueChange<usize>` on the group and moves the lit one; activating the lit one sends nothing; the game sets the value and the group follows with no event (`update_toggle_groups`).
- **T4. `toolbar_separator()`** is a vertical line (`Role::ToolbarSeparator`), the menu separator's colour. The menu's own `separator()` stays horizontal.
- **T5. Overflow**: `fit_bars` measures each item's laid-out width (cached while shown, so a hidden item's width is known), and when the items do not fit the bar's content width it hides the **last** items (`Display::None`, the marker `Overflowed`) until they and a "More" button fit. "More" is a `menu_title` spawned on first need; its menu lists what is hidden as menu rows named by the item's tooltip (or text) that forward `Activate` to the real item; a hidden toggle group lists its options as radio rows. `model::bar_fit` is the engine-free rule. Focus on an item that hides moves to "More".
- **T6. Keys**: Enter, Space, South and a click activate (Bevy's `Button`); the arrows and the D-pad move focus between items by the crate's spatial navigation; Tab visits each control.

## Parameters

|part|value|
|---|---|
|widgets|`toolbar`, `icon_button`, `toggle_group`, `toolbar_separator`; `GroupOption::{text, icon}`|
|state|`ToggleGroupValue(usize)`, `ToggleGroupCount(usize)`; marker `Toolbar`, `Overflowed`|
|target|`skin.target()` square at least (44 px, 24 px compact)|
|gaps|half a `skin.gap` between items; the separator a line wide, margins half a gap|

## Validation targets

- Activating the second option of a toggle group emits `ValueChange(1)` and unchecks the first (the issue's acceptance); the lit option's fill is the accent and its icon `accent_text` (4.5:1).
- Activating the lit option sends nothing; a game's value moves the lit one with no event; a value past the end is pulled back.
- An icon button is at least `skin.target()` square and carries its tooltip; its icon follows disabled and pressed.
- A toolbar 300 px wide of ten buttons hides the last ones, shows "More", and "More" lists exactly the hidden ones; widening shows them again and hides "More"; activating a row of the menu activates the real item.
- Thirty idle frames write nothing.

## What great looks like

A tool's top bar like a shipping editor's: a row of equal square glyph buttons with a hairline between groups, one mode lit in the accent, a tooltip with the chord on hover or focus, and at a narrow width a quiet "More" instead of a cut-off row.

## Deviations

- **No roving tabindex.** APG makes the toolbar one tab stop. This crate reaches everything with the arrows and the D-pad already, and a pad player has no Tab; every item is a tab stop as a menu bar's titles are. Design default (owner may override).
- The issue's `separator()` already names the menu's horizontal line, so the toolbar's is `toolbar_separator()`. Assumed decision (owner may override).
- The More button's label is the word "More", not an ellipsis: Bevy's default font draws some punctuation as a box (the host supplies the font), and the word is the accessible name.
- A toggle group is radio only; a toolbar toggle that can be off is a `toggle`'s job. Design default.

## Pending index

The APG toolbar page and the Xbox guidelines are not in home-still.
