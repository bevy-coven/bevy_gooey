# Accessibility tree: every widget a node with a role, a name, a value and its state

Researched 2026-11-02 (iteration 42). Issue #32 (G-28, `grimoire-ui`, `consumer:bevy_grimoire`): "Screen-reader users need every widget in the accessibility tree with its role, label and state." Acceptance: a toggle's node reports role CheckBox, its label and its checked state, which update on toggle.

## Sources

1. **Craft:** Microsoft, *Xbox Accessibility Guideline 112: UI navigation*, https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112 (read 2026-11-02), and W3C WAI, *WAI-ARIA Authoring Practices: Providing Accessible Names and Descriptions*, https://www.w3.org/WAI/ARIA/apg/practices/names-and-descriptions/.
2. **Shipping game:** Naughty Dog, *The Last of Us Part II: Accessibility Features Detailed*, https://www.naughtydog.com/blog/the_last_of_us_part_ii_accessibility_features_detailed (Text-to-Speech over every menu), and Xbox's own platform narration ("Let Games Read to Me") as XAG 112 describes it.
3. **Technical:** AccessKit's data schema (`accesskit` 0.24: a tree of nodes, each with an id, a role and optional attributes; the toolkit pushes the tree, the platform adapter holds it), and Bevy 0.20's `bevy_a11y` / `bevy_ui::accessibility` / `bevy_ui_widgets` / `bevy_winit::accessibility`, read from the vendored source.

## Craft principle

XAG 112: "players who use assistive technologies like screen narration or non-traditional inputs ... can be hampered or blocked from gameplay if focus doesn't move in a clear, consistent way. The movement of focus should always be predictable." and, for the order the narration reads in: "the 'title' of each settings page is always at the top of the page and read first in the screen narration reading order".

APG, *Providing Accessible Names and Descriptions*: "Both the WAI-ARIA specification and WCAG require all focusable, interactive elements to have an accessible name." and "Rule 2: Prefer Visible Text ... using the visible text for the accessible name simplifies maintenance, prevents bugs, and reduces language translation requirements." Also: elements named from content (button, checkbox, menuitem, tab, tooltip, treeitem, row, cell, heading ...) are "named by a string calculated from their descendant content"; a control with a separate label (a slider, a text field) has the label as its name and presents its value in addition.

## How shipping games do it

Naughty Dog: "our menus and other on-screen text can be voiced using the Text-To-Speech option." and, per the studio's own account of coverage, every piece of text on screen is voiced, with status on request. The shape is the one a game UI kit can give for free: the words the player reads are the words that are spoken, so nothing is authored twice, and a menu is voiced in the order it is laid out. Xbox's platform narration reads the same way: role, name, state, in focus order.

## Model adopted

- **A1. The names are the words on screen** (APG rule 2). A node's name comes from the `Text` the widget already draws, never from a second string a game must write: a button's from its text, a toggle's, slider's, choice's, binding's and text field's from the row's label (`Role::ControlText`), an icon button's from its `Tooltip`, a dialog's from its heading, a tab's from its tab text.
- **A2. Bevy's nodes stay.** `bevy_ui_widgets` already requires an `AccessibilityNode` with the right role for `Button`, `Checkbox` (CheckBox, with `Checked` kept as `toggled`), `Slider` (numeric value, range, step), `TabList`, `Tab` (with `Selected`), `TextInput` and menu items, and `bevy_ui` keeps `InteractionDisabled` as `disabled`. The crate never re-sets a role Bevy owns; it fills what Bevy leaves empty (the name of a checkbox, a slider, a text field; the value text of a slider, a choice, a binding) and adds a node where Bevy has none (headings, labels, meters, progress, tables, trees, toasts, splitters, containers).
- **A3. One exhaustive table.** `access::spec(Role)` is an exhaustive `match` over `Role`, so a new `Role` variant fails to compile until someone says what an assistive technology sees: `Hidden` (decorative: no node, and a node Bevy added is hidden), `Group` (a container: keeps the tree nested, so the reading order is layout order and not archetype order), or `Node { role, name, value }`.
- **A4. Decorative nodes are hidden.** An icon is `ImageNode`, which Bevy turns into an `Image` node; here it carries `hidden` (the icon beside a label adds nothing, and an icon-only button is named by its tooltip).
- **A5. State follows the widget, written only on a difference.** `hidden` for what is not shown (`InheritedVisibility`, a `Display::None` self), `disabled` from `InteractionDisabled`, `selected` from Bevy's `Selected`, `expanded` for a section, a menu title, a tree row and a select, `numeric value` for a meter, a progress bar and a splitter, and the value text for the rest. The node is read through `Deref` and written only when it differs, so a settled UI marks nothing changed.
- **A6. Nothing runs unless a screen reader does.** `bevy_a11y::AccessibilityRequested` is false until an assistive technology connects; the system returns at once while it is, so a game with no screen reader pays one resource read a frame. Bevy sends every node on every update while it is true, so nothing is lost by building the names when it flips.
- **A7. Focus has a node.** `bevy_winit` skips the whole tree update when the focused entity has no node, so a focus stop gets a node even when its role is a part (a divider, a swatch, a colour area).

## Parameters

None numeric. Roles: toggle CheckBox (Bevy's), slider Slider (Bevy's), choice SpinButton (a stepper: its arrows and Left/Right change the value), binding Button with the key as its value, meter Meter, progress ProgressIndicator, dialog Dialog (modal), toast Status or Alert, heading Heading, text Label, table Table/Row/Cell, tree Tree/TreeItem, splitter Splitter. Design defaults, chosen to match the ARIA role each widget implements in APG.

## Validation targets

- A toggle reports CheckBox, its label as name and `toggled` true or false, and the node changes the frame after the toggle (the acceptance).
- Every widget the gallery draws: every focus stop has a node with a role and a non-empty name; no node holds an `Unknown` role.
- A hidden page, a closed menu and an icon are hidden; a disabled control is `disabled`; a selected tab is `selected`.
- With no screen reader requested the system writes nothing (a settled tree is never marked changed, with or without one).
- Frame time: no change within 0.5 ms with no screen reader.

## Deviations

- The choice is SpinButton, not ARIA's `combobox`: a combobox promises a popup the choice does not have. Design default; revisit when AccessKit actions are handled.
- A tree row is a `TreeItem` with no `level` or `expanded`, and a table has no row or column counts: the rows are a pool of a dozen entities rebound while scrolling, so a row's place is the table's, not the entity's. Listed in the backlog.
- **Actions are not handled** (an assistive technology's Click, Increment, SetValue and Focus requests arrive as `ActionRequest` messages that Bevy does not act on): the tree is complete for reading and for following focus; acting from the screen reader is the next slice and is listed in the backlog.

## What great looks like

Nothing on screen changes: the work is the tree. Great is a screen-reader user hearing "Master volume, slider, 70 percent", "Fullscreen, check box, checked", "Quit game, dialog" the frame focus lands, in the order the screen reads, with nothing read twice and no unnamed control. The evidence is a dump of the tree for the gallery (role, name, value, flags per node) next to the screenshot, and the tests.

## Pending index

- Xbox Accessibility Guidelines 101 and 112 and the APG names note are web pages, not in home-still; `paper_search` has no game-UI accessibility paper newer than *Future design of accessibility in games*. Pending: the Game Accessibility Guidelines.
