# Status bar

Researched 2026-11-02 (iteration 38, #23 G-19). home-still's index has nothing on status bars, so the sources below were fetched today.

## Sources

1. Microsoft, *Visual Studio Code UX Guidelines: Status Bar*, https://code.visualstudio.com/api/ux-guidelines/status-bar — craft and shipping product.
2. Xbox Accessibility Guideline 103 (*Visual distinction*: "not by colour alone"), https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/103 — shipping games.
3. Bevy 0.20 `bevy_ui::ComputedNode`, and this crate's `toolbar.md`, `progress.md` and `tooltip.md` — technical.

## Craft principle

VS Code: "Items that relate to the entire workspace (status, problems/warnings, sync) go on the left and items that are secondary or contextual (language, spacing, feedback) go on the right." And: "Limit the number of items added", "Use short text labels", "Use icons only when necessary". The API's `priority`: "The priority of this item. Higher value means the item should be shown more to the left."

## How shipping games do it

Xbox 103 asks for state by shape and words as well as colour. A status item's kind is therefore a mark of its own (a bar at its start, like a field's message) and never only a colour of its text; its words stay the text colour at 4.5:1.

## Model adopted

- **S1. `status_bar(left, right)`** is a full-width row (`Role::StatusBar`, the toolbar's look) of two groups: the left grows and holds workspace state, the right is pushed to the end and holds contextual state. Each group is the items passed, in order.
- **S2. `status_item(text)`** is a Bevy `Button` (hover and focus lit, Enter, Space, South and a click send `Activate`), small text, `Role::StatusItem` a target high. It takes a `Tooltip` like any widget and a `StatusKind { Info, Warning, Error }` component (`Info` default) which draws a mark at its start in `accent` (warning) or `danger` (error); the words never change colour.
- **S3. `StatusPriority(u8)`** on an item (default 128). When the bar is narrower than its items, `fit_bars` hides the lowest priority of the **right** group first, then the left's, ties the later item first (`model::bar_fit`); hidden items keep their width so they return when the room does. A bar never wraps.
- **S4. A `status_progress(text, fraction)`** is an item that hosts a slim `Progress` (the meter's own track, without its words line) after its text, so a long task shows where the player always looks.

## Parameters

|part|value|
|---|---|
|widgets|`status_bar`, `status_item`, `status_progress`; `StatusKind`, `StatusPriority`|
|height|an item is `skin.target()` high, the bar adds a quarter gap of padding|
|mark|`line * 2` wide bar, the full item height, `accent` or `danger`|
|text|body text, one line (`ONE_LINE`), `text` 4.5:1 on the bar|

## Validation targets

- At 800 px wide the lowest-priority item hides first; at 1280 px all show (the issue's acceptance); activating an item triggers `Activate`.
- An item hidden by priority is the right group's lowest, never a left one while a right one remains.
- Widening the window shows hidden items again in the reverse order.
- A warning and an error item each draw a mark in their colour; info none; the words' contrast is the panel's.
- Thirty idle frames write nothing.

## What great looks like

A slim bar along the window's foot: workspace state on the left, the file's state on the right, one amber or red tick where something needs attention, and a thin progress under a running task, never wrapping and never cutting an item in half.

## Deviations

- The issue says each item "has a tooltip"; here a `Tooltip` is the crate's own optional component on an item, not forced, since a one-word item needs none. Design default.
- A warning's mark is the accent, as `FieldProblemKind::Warning` is (iteration 37); a named amber is #33's.
- The issue's "hosts an optional progress" is `status_progress`, a slim bar (no words line) because the item's own text is the words.

## Pending index

The VS Code page and the Xbox guidelines are not in home-still.
