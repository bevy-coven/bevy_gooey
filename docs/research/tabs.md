# Tabs: pages of a settings screen, and a bumper that turns them

Researched 2026-10-31 (iteration 14). home-still's corpus has nothing on tabs or on gamepad menus (`hs distill search`, four queries on tab navigation with bumpers, tabbed settings menus, roving tabindex and tab design guidance returned only a lighting thesis, a hotkey-rehearsal paper, web-accessibility LLM benchmarks and unrelated books; `catalog_recent` limit 200 held only biomedical, SIGGRAPH and rendering-course embeds), so the three kinds of source below are web copies, Bevy's source and the owner's editor. Every quotation was fetched on 2026-10-31 and matches; the gap is under `## Pending index`.

## Sources

1. Nielsen Norman Group, Sunwall, E., *Tabs, Used Right* (2024, from Nielsen's 2007 article) — https://www.nngroup.com/articles/tabs-used-right/ (craft: what a tab strip must show, and how a game's few tabs should look and sit).
2. Microsoft, *Xbox Accessibility Guideline 112: UI navigation*, *114: Consistent UI components* and *106: Narration* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112, `/114`, `/106` (shipping games and the console's conventions: RB and LB turn tabs, in Forza Horizon 4 and Minecraft).
3. W3C, *Tabs Pattern* (WAI-ARIA Authoring Practices Guide) — https://www.w3.org/WAI/ARIA/apg/patterns/tabs/, and the Bevy 0.20.0 source, `bevy_ui_widgets-0.20.0/src/tabs.rs` (technical: what a tab list does with the keys and what Bevy leaves to us).
4. The owner's `bevy_emerge` editor, `crates/bevy_emerge_mapper/src/chrome.rs`, whose comments name "the tab strip" among the pieces it draws by hand with an unnamed palette (the same literal "byte-identical, twice").

## Craft principle

NN/g on how a tab shows it is selected: "Use at least two selection indicators to enhance the visual salience of the selected tab. Multiple indicators are critical to distinguish the selected tab when there are only two tabs, as there are fewer unselected tabs to compare against." And the other half, for the ones that are not: "Unselected tabs should be visible to remind users of the additional options. Unselected tabs that are too faded into the background may not be noticeable, so users may never discover their content." A strip where the selected tab differs from the rest by a hair ("CNN: the selected navigation tab was bolded, but the effect was so subtle that it was nearly indistinguishable") fails the first; a strip of muted labels ("MongoDB: ... poor color contrast with the white background and could be mistaken for unselectable features") fails the second.

On what belongs in a tab control: "Tab labels should usually be 1-2 words", "The fewer tabs, the better", "Tabs Should All Look and Work the Same: ... clicking on any of the tabs should change its panel, and they should use the same unselected and selected styling", "Always one tab selected by default" for in-page tabs, "Strictly speaking, each in-page tab should have the same layout but with different data", "Websites and simple apps should avoid stacking tab lists", and on the panel: "Another way to convey the connection between the tab label and the tab panel is to use proximity." On access: "Tabs should have high-contrast focus highlighting."

## How shipping games do it

Xbox, on what every menu keeps the same from screen to screen (XAG 112): "Interaction mechanisms are consistent throughout (like “A” to select, “B” for back, “LT” and “RT” for moving through pages, or “RB” and “LB” for moving through tabs). If interaction methods were to change from screen to screen, this would be very disorienting and confusing for a player." XAG 114 names a shipped game: "In Forza Horizon 4, the use of the “LB” and “RB” controller buttons to move left and right (respectively) between menu tabs is consistent across various areas of the UI." XAG 112 names another where tabs hold a lot: "In Minecraft, players can locate specific crafting or inventory items by using the four filtered tabs." XAG 106 allows that the bumpers are a convention and not a law ("maybe not every game uses right bumper (RB) and left bumper (LB) to move between submenus in the \"Options\" menu"), which is why a pad player is told by a button prompt, and what a screen reader must say of a tab: "Worlds, Tab, 1 of 3, selected". The list this crate already has pages with the triggers (`LT`, `RT`); the bumpers are the tabs'.

XAG 112 on the end of a row of tabs: "After navigating to the last item in the UI/menu structure, the player should be taken back to the first item in the UI/menu structure and vice versa." It applies to "linear menu structures where focus can be moved EITHER up or down, OR left or right", as a strip of tabs is, and not to "a menu with focusable items arranged in a 4x4 tile", so a strip of tabs loops and a grid does not. It adds: "We encourage developers to have an option to enable and disable menu looping for all menus."

## Technical

APG, tabs, keyboard interaction: "When focus moves into the tab list, places focus on the active `tab` element"; "`Left Arrow`: moves focus to the previous tab. If focus is on the first tab, moves focus to the last tab. Optionally, activates the newly focused tab"; "`Tab`: When the tab list contains the focus, moves focus to the next element in the page tab sequence outside the tablist, which is the tabpanel unless the first element containing meaningful content inside the tabpanel is focusable"; and the activation note: "It is recommended that tabs activate automatically when they receive focus as long as their associated tab panels are displayed without noticeable latency." A game's pages are on screen the same frame.

Bevy 0.20.0 gives the headless half (`bevy_ui_widgets::{TabList, Tab, SelectedTab}`): a roving `TabIndex` (one tab per list is a Tab-key stop), Left/Right (or Up/Down) with wrap, Home and End, Enter and Space, a click, `TabActivation::Automatic` (moving focus selects), and one event, `ValueChange<Option<Entity>>` on the list, which "does not update that state unless [`tablist_self_update`] is attached". It gives no panel: "Headless tab-strip behavior and policy" stops at the strip, and no bumper. Both are this crate's.

## Model adopted

- **T1. One call, pages as children.** `tabs(["Audio", "Display", "Controls"], 0, children![tab_page(children![..]), ..])` returns a column (`Role::Tabs`) of a strip (`Role::TabList`, Bevy's `TabList` with `TabActivation::Automatic`; one `Role::Tab` per label, Bevy's `Tab`) and the pages (`Role::TabPages`, one `Role::TabPage` per `tab_page`). The i-th label opens the i-th page. `tab_page` is a column like `menu()`, so a page's widgets are spaced like a menu's.
- **T2. State is the widget's: `TabsValue(usize)`.** The selected index, on the root: read it, or set it from code and the screen follows in the same frame, with no `ValueChange` for a change the game made (as `ChoiceValue`). Every change a player makes is one `ValueChange<usize>` on the root, so a game never touches Bevy's `Option<Entity>` and never needs an entity id. `TabsCount(usize)` is the number of labels; a value past the end is pulled back to the last.
- **T3. One page shows, all of them take their room.** The pages stand in one grid cell, so the strip's page is as wide and as high as the largest page and nothing moves when a tab is chosen (NN/g: stacking tabs "destroys spatial memory"; here the same holds for a panel that jumps). The pages not selected are `Visibility::Hidden`: they keep their room, are not drawn or picked, and none of their widgets is a Tab-key stop or a neighbour for the arrows (Bevy's navigation and this crate's skip what is not visible).
- **T4. Two selection indicators, and the rest still readable.** The selected tab is filled `accent` with `accent_text` on it; the others are `surface_raised` with `text`, a button's own colours, so every tab is full-contrast text on a visible fill (NN/g's second rule) and the selected one differs by fill luminance (at least 3:1 against the others, the pair `problems()` already holds for the meter) and by its text colour: two cues, neither only hue. Hover and the focus ring are a button's.
- **T5. The strip is a button row.** `min_target` high at least, a button's padding, border and radius, `gap` apart, each tab growing equally into the width (`flex_grow`), so the targets are as big as the room allows; the strip is `gap` above the pages (proximity).
- **T6. Keys and the pad, all single presses.** With focus on a tab: Left and Right (Bevy's observer: focus moves, the tab is selected) and the D-pad's Left and Right (`step_tabs`) step and wrap; Home and End and Enter and Space are Bevy's; Up and Down leave the strip (`navigate_by_input` leaves Left and Right to a tab, as to a slider and a choice). Anywhere in the tab widget's scope, **LB and RB** (the pad's `LeftTrigger` and `RightTrigger`, Bevy's names for the bumpers) and **Ctrl+PageUp and Ctrl+PageDown** step to the previous and next tab, wrapping; PageUp and PageDown alone stay the list's. Which widget: the tabs the focus is in, else the only visible tabs in the focus's scope (an open dialog is its own scope), else nothing.
- **T7. Focus never rests where it cannot be seen.** A bumper or a code change that hides the page focus was in lands focus on the first widget of the new page (the pad player's hand stays in the content), or on the selected tab when the page has none; focus on a tab goes to the newly selected tab. Arrows and D-pad Up and Down that reach the strip from a page land on the **selected** tab, not the nearest (APG: "places focus on the active tab element"). Never for a focus the pointer set.
- **T8. Feedback is a choice's.** One `UiFeedbackKind::Activate` per change, `Focus` for focus onto a tab; the crate plays nothing.
- **T9. Nothing runs at rest.** `update_tabs` is `Changed<TabsValue>`; `land_tab_focus` is an ancestor walk of one entity; `step_tabs` returns at once without a press.

## Parameters

No new field in `GooeySkin`; every size derives from the existing ones.

|part|value|
|---|---|
|root|a column, `row_gap = gap`|
|strip|a row, `column_gap = gap`|
|tab|`min_height = min_target`, a button's padding, border (`snap_line_width(border)`) and radius; `flex_grow 1`; fill `accent` selected, `hovered()` under the pointer, else `surface_raised`; text `accent_text` selected, `text` else, `text_muted` disabled|
|pages|one grid cell; every page `grid_row`/`grid_column` start(1); `row_gap = gap`|
|bumpers|`LeftTrigger` back, `RightTrigger` forward (the triggers `LeftTrigger2`, `RightTrigger2` stay `page_lists`')|
|step|`model::tab_step(selected, count, delta)`: wraps, clamps `count == 0` to 0|

## Validation targets

- 3 tabs at 1280x720: each tab at least 44 px high (88 at 2560x1440); tabs and strip stay inside the root; the first page shows, the other two are not drawn.
- The selected tab's fill is `accent`, the others `surface_raised`, and the two differ by at least 3:1 relative luminance in both presets; text on both is at least 4.5:1 in both presets; focus ring 3 px on all four sides of the focused tab.
- Choosing each tab in turn (click, arrows, bumpers): the strip's and the root's rectangles change by 0 px; one `ValueChange<usize>` each, one `UiFeedback::Activate` each.
- Left from the first tab goes to the last and Right from the last to the first, focus following; the same by the D-pad; Home and End.
- LB/RB with focus on a slider in page 0: page 1 shows, focus is on page 1's first widget with the ring visible, page 0's slider is not a neighbour for any arrow and Tab never lands on it; wraps at both ends; with focus on the selected tab, focus stays on a tab (the new one); with focus outside every tab widget and none or two visible tab widgets: nothing.
- Ctrl+PageDown and Ctrl+PageUp do what the bumpers do; PageDown without Ctrl does not turn a tab, and Ctrl+PageDown does not page a list.
- Up from a widget in the page lands on the selected tab even when another tab is nearer; Down from the tab lands in the page.
- `TabsValue` set from code: the page shows the next update, no event, focus rescued if it was in the old page; a value past the end shows the last page.
- A click on a button in the selected page reaches it (the hidden pages do not take the pointer).
- Thirty idle frames write nothing the skin restyles; frame time of the gallery within 0.5 ms of before.
- Every border on whole device pixels at 1x to 2x (the crate's line rule).

## What great looks like

A settings panel: a title, then a row of three equal tabs, **Audio** lit amber with dark text, **Display** and **Controls** in the button grey with light text, ten pixels above the page they open. Choose Display with RB: the amber moves one tab right and the rows below change, and nothing else on the screen moves, not the panel's edge, not the strip, not a row of the page you left. The focus ring, the same blue, sits 3 px round the tab under the pad. A pad player turns three pages with RB, RB, LB without once pressing Down, and a keyboard player gets there with Left and Right and drops into the page with Down or Tab. It looks like the buttons beside it because it is made of them.

## Deviations

- **Automatic activation** (APG's recommendation for pages with no latency) over Bevy's default `Manual`; focus on a tab by Left and Right selects it. A game that wants manual sets `TabList::activation` on the strip.
- **The selected tab is lit by fill, not by an underline.** NN/g lists lines, fill, font weight, size and icon as indicators; this crate has one bold-less font and draws lit things in `accent` everywhere (a switch that is on, a meter's fill), so a selected tab is "on". The underline is not drawn: an extra line under a button row would be a second frame beside the focus ring. A design default; the validation targets measure the two cues instead.
- **All pages take the room of the largest** (not Bevy's, not NN/g's): a panel that resizes with the tab breaks spatial memory and moves the pad player's target. The price is blank room under a short page. A design default.
- **The bumpers act from anywhere in the tab widget's scope**, not only from the strip: XAG 112 asks the mapping be the same on every screen, and the player's hand is on the content. A design default, with the fall-back to the only visible tabs so a Back button beside the tabs does not stop them.
- **Ctrl+PageUp and Ctrl+PageDown**: the browser and terminal convention; APG's tabs pattern lists no key for it, and Ctrl+Tab would also cycle Bevy's focus. A design default.
- **The strip wraps** at both ends (APG's Left Arrow; Bevy's behaviour), where a choice stops: a choice is a value with ends, a tab strip a one-direction loop (XAG 112).
- **No vertical strip, no disabled or closable tab, no scrolling strip, no icons.** NN/g: "The fewer tabs, the better", and a strip that overflows "becomes a carousel" whose hidden tabs "become less discoverable"; the strip does not wrap or scroll, so a game keeps to a handful of one- or two-word labels. A game that needs more takes a `choice` or a `list`.
- **No accessibility position ("Worlds, Tab, 1 of 3, selected", XAG 106)**: Bevy's `TabList` and `Tab` carry their roles, and the selected state is Bevy's `Selected`; the "n of m" narration is a backlog item with the slider's, the choice's and the list's.
- Assumed decisions (owner may override), all above: the `tabs(labels, selected, children![tab_page(..)])` shape, `tab_page` as the page's wrapper, fill as the indicator, pages of one size, LB and RB with the scope fall-back, Ctrl+PageUp and Ctrl+PageDown, wrapping.

## Pending index

- Nothing relevant in home-still (four queries on 2026-10-31 and `catalog_recent` limit 200). Index NN/g's *Tabs, Used Right*, the three Xbox Accessibility Guidelines and W3C's *Tabs Pattern*. Search later for: "gamepad menu tabs bumpers", "pause menu structure", "settings menu categories". No studio write-up of a game's own tab strip was found; the shipping-game evidence is Microsoft's guidelines naming Forza Horizon 4 and Minecraft. The Game Accessibility Guidelines and *The Gamer's Brain* remain pending from `docs/research/gooey.md`.
