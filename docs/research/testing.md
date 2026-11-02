# Headless test helpers (feature `testing`): drive a gooey UI by what the player reads

Researched 2026-11-02 (iteration 41). Issue #31 (G-27, `grimoire-ui`, `consumer:bevy_grimoire`). A host's UI tests today reach for entity ids and component queries; every crate test here builds its own pointer-and-key plumbing. The same helpers serve any game that tests its menus.

## Sources

1. Testing Library, *Guiding Principles*, https://testing-library.com/docs/guiding-principles/ (craft: test the way the software is used).
2. Playwright, *Locators* (`getByRole`, `getByText`: strict mode, a locator matching two elements is an error naming them) and Unity's UI Test Framework / `Selenium`-style "find by visible text" (shipping practice: tests find a control by the words on it). [INFERENCE: these pages were not fetched this pass; the strictness rule is documented behaviour of both and is what the issue's "two widgets with one label: errors naming both" asks for.]
3. Bevy 0.20 `bevy_picking` (`PointerInput`, `PointerId::Mouse`, `Location`) and `bevy_input_focus` (`InputFocus`), read from the vendored source; the crate's own `tests/pointer.rs` and `tests/palette.rs`, which already drive real `PointerInput`, `KeyboardInput` and `ButtonInput` in a headless window (technical).

## Craft principle

Testing Library: "The more your tests resemble the way your software is used, the more confidence they can give you." A player finds "Options" by its words, clicks it where it is drawn and sees where focus went. A test that does the same cannot pass while the button is covered, unfocusable, off-screen or mislabelled, which an entity-id test can.

## How shipping games do it

Tools that test shipped UIs (Playwright, Selenium, Unity's UI Test Framework) all share one shape: a **locator by visible text or role**, **strict** when it matches two (an error that names both rather than a silent first-match), and **actions that go through the real input path** (a click is a press and a release at the element's centre). The helpers here are those four verbs over a Bevy `App`.

## Model adopted

- **T1. `bevy_gooey::testing`** behind the cargo feature `testing` (off by default: no test code in a game's build). Free functions over `&mut App`, no plugin, no resource.
- **T2. `find_by_label(app, label) -> Result<Entity, FindError>`.** The label is the widget's own text: the `Text` of the widget or of a child, the match exact, trimmed. `FindError::{NotFound { label }, Ambiguous { label, entities }}`; `Display` names them (with each one's `Name` or `Role`). A widget is a match only if it is visible (`InheritedVisibility`), so a hidden page's "Back" is not one.
- **T3. `click(app, entity)`** is a primary press and release (`PointerInput`) at the centre of the entity's `ComputedNode` (physical px over the scale factor), with the pointer moved there first, then updates the app until the events settle (two frames). `press(app, key)` is a `KeyboardInput` press and release (with `ButtonInput` kept as a real keyboard's); `type_text(app, text)` is one press-and-release per character with its `text`; `focused(app) -> Option<Entity>` is `InputFocus`, and `focused_label(app) -> Option<String>` is its words. Each action runs enough `app.update()`s for the crate's observers and its next `Style` pass to land (2).
- **T4. Documented in the guide** as the way to test a gooey UI, with the issue's acceptance as the example: open the gallery's widgets, click "Options" by label, read the focused widget's label.

## Parameters

Frames per action: 2 (one for the input to be read in `PreUpdate`, one for the `Style` set). Design default.

## Validation targets

A test in this repository clicks a button by its label and reads the focused widget's label; two widgets with one label give `Ambiguous` naming both; no widget gives `NotFound`; a hidden widget is not found; `type_text` types into a text field and the row's value is the text; `press` activates a focused button.

## Deviations

- **The issue's `focused` returns what?** Unspecified. Decided: `focused` returns the entity (`Option<Entity>`), `focused_label` its words, so the issue's acceptance reads `focused_label(&mut app)`. Recorded as an assumed decision on the issue.
- **The path is `bevy_gooey::testing`, not `gooey::testing`**: the crate's name.

## What great looks like

A host test reads like the manual: `click(&mut app, find_by_label(&mut app, "Options")?); assert_eq!(focused_label(&mut app).as_deref(), Some("Text size"));`.

## Pending index

Playwright's and Unity's pages (web sources, not in home-still).
