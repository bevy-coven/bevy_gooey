# Accessibility: what a screen reader is told

Every gooey widget is a node of the accessibility tree with a role, a name, a value and its state, built from the words the widget already draws. There is nothing to call and nothing to write twice: spawn `toggle("Subtitles", true)` and a screen reader says "Subtitles, check box, checked", and says "not checked" the frame the player toggles it.

It is on while a screen reader is connected. Bevy's `AccessibilityPlugin` (in `DefaultPlugins`) keeps `bevy::a11y::AccessibilityRequested`, which an assistive technology sets, and the crate's one system (`update_accessibility`, last in `GooeySystems::Style`) returns at once while it is false: a game nobody reads aloud pays for one resource read a frame. When it turns true, every node is built in that frame. A node is read through `Deref` and written only when it differs, so a settled interface marks nothing changed.

## What each widget says

| Widget | Role | Name | Value and state |
|---|---|---|---|
| `button`, `icon_button` | Button | its text, else its `Tooltip` | disabled |
| `toggle` | CheckBox | its label | checked or not |
| `slider` | Slider | its label | the words ("70%"), the number and its range |
| `choice`, `select`, the option rows | SpinButton | its label | the selected option's words |
| `radial` | SpinButton | the host's `AccessibleLabel` | the highlighted item's name |
| `binding` | Button | its label | the key's name ("Space"), the prompt while it waits |
| `text_input`, `number_field`, `vec_field` | TextInput | the row's label | the text |
| `text_area` | MultilineTextInput | the row's label | the text |
| `color_field` | ColorWell (the swatch) and TextInput | the row's label | the hex |
| `heading`, `label` | Heading, Label | its words | |
| `meter`, `progress` | Meter, ProgressIndicator | none (see below) | the number; a progress bar's line ("3 of 7, about 20 s left") |
| `tabs` | Tab, TabPanel | the tab's text | selected; a page that is not shown is hidden |
| `table`, `tree` | Table or Tree, Row or TreeItem, Cell | a cell's text | a row's selection |
| `split`, a table column's edge | Splitter | "Divider", "Resize column" | the split's fraction |
| `menu_bar`, `menu_title`, `menu_item`, `check_item`, `radio_item`, `submenu` | MenuBar, MenuItem, MenuItemCheckBox, MenuItemRadio, Menu | the item's text | open or not, checked, a closed menu hidden |
| `dialog` | Dialog, modal | its heading | |
| `toast`, `toast_alert` | Status, Alert | its words | |
| `toggle_group` | RadioGroup, RadioButton | the option's text or tooltip | checked |
| `section` | the header is a Button | its title | expanded or collapsed |
| `viewport()` | Pane | none (name it) | |

Hidden from the tree (no node, or `hidden` on one Bevy added): icons (an icon beside a label adds nothing, and an icon-only button is named by its tooltip), a toggle's track and knob, every slider and meter part, a choice's arrows, a keycap's frame, a shortcut badge, a drag ghost, a spinner, a separator. A `Group` container (a panel, a screen, a list) is a generic container that keeps the tree nested, so the reading order is the layout order. A node that is not shown (a page behind a tab, a closed menu, a tooltip not open, a `Display::None` section body) is `hidden`; a control with `InteractionDisabled` is `disabled`.

## Naming something the crate cannot

A meter, a progress bar and a viewport have no label of their own: the words beside them are a separate `Label`. Name the bar with Bevy's own component, which always wins over the crate's words:

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn hud(mut commands: Commands) {
    commands.spawn((meter(75.0, 100.0), AccessibleLabel::new("Health")));
    commands.spawn((viewport(), AccessibleLabel::new("Level view")));
}
```

The crate owns the name and value of its own widgets: a label or value another system writes into a gooey widget's `AccessibilityNode` is replaced; use `AccessibleLabel`. Names it supplies when a control has none ("Divider", "Resize column", "Saturation and brightness", "Command") are English words like "Dismiss" and "More"; `AccessibleLabel` replaces them for a translation.

## See the tree

`GOOEY_A11Y=1` on the gallery stands in for a connected screen reader and prints the tree at the capture frame, nested as the screen reader is told it:

```text
GOOEY_A11Y=1 GOOEY_FRAMES=60 cargo run --release --example gallery
Heading "bevy_gooey"
GenericContainer
  Button "Play"
  Button "Options"
    Tooltip hidden
  Button "Quit"
Label "Health"        (the meter's number is #60)
Tab "Audio" selected / Tab "Display" unselected / Tab "Controls" unselected
TabPanel
  Slider "Volume" "70%" #0.7
  CheckBox "Subtitles" True
TabPanel hidden
  SpinButton "Quality" "Medium" hidden
```

In a test, read the node the way a reader would: `app.world().get::<AccessibilityNode>(entity)` and `.role()`, `.label()`, `.value()`, `.toggled()` (`accesskit` is the crate Bevy's nodes are made of), after setting `AccessibilityRequested` to true. See [testing](testing.md).

## Not yet

An assistive technology's own requests (Click, Increment, SetValue, Focus) reach a game as Bevy's `ActionRequest` messages and nothing in Bevy or this crate acts on them yet: the tree is complete for reading and for following focus (every focus stop has a named node), but acting from the screen reader is the next step. A tree row is a `TreeItem` without its depth (`level`) or its open state yet, and a table's rows and columns carry no counts.

Research: `docs/research/accessibility.md` (Xbox Accessibility Guideline 112, the WAI-ARIA authoring practices on names, Naughty Dog's text-to-speech over every menu).
