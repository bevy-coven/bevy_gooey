# Collapsible sections

Researched 2026-11-02 (iteration 39, #19 G-15). home-still's index has nothing on disclosure widgets (the corpus re-scans of iterations 37 and 38 found no game-UI paper), so the sources below were fetched today.

## Sources

1. W3C, *WAI-ARIA Authoring Practices, Disclosure Pattern* and *Accordion Pattern*, https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/ and https://www.w3.org/WAI/ARIA/apg/patterns/accordion/ — craft.
2. Dear ImGui, `CollapsingHeader`, `SetNextItemOpen` and the ID-keyed `ImGuiStorage` (the open state of a tree node is kept under the node's ID), https://github.com/ocornut/imgui/issues/2958 and the *Dear ImGui FAQ*; Unity's inspector foldouts keep their state the same way — shipping tools.
3. Bevy 0.20 `bevy_ui::Display::None` (the node and its children take no part in layout) and `Visibility::Hidden` / `InheritedVisibility` (Bevy's Tab navigation and picking skip what is not visible), read from this crate's own `tabs.md` and `AGENTS.md` ("A page that is not shown is hidden, not removed, and focus never rests in it") — technical.

## Craft principle

APG, Accordion: "An accordion is a vertically stacked set of interactive headings that each contain a title, content snippet, or thumbnail representing a section of content. The headings function as controls that enable users to reveal or hide their associated sections of content. Accordions are commonly used to reduce the need to scroll when presenting multiple sections of content on a single page." Keys: "`Enter` or `Space`: When focus is on the accordion header for a collapsed panel, expands the associated panel … for an expanded panel, collapses the panel", and "`Tab`: … all focusable elements in the accordion are included in the page `Tab` sequence." (a collapsed panel's widgets are not visible and so are not in it).

APG, Disclosure: "When the controlled content is hidden, the button is often styled as a typical push button with a right-pointing arrow or triangle to hint that activating the button will display additional content. When the content is visible, the arrow or triangle typically points down."

## How shipping games do it

Dear ImGui keeps the open state out of the caller's code, under the header's ID: "You can use GetStateStorage()->GetInt(ImGuiID key, int default_val = 0) with the ID of the tree node or correspondingly the SetInt() function", and `SetNextItemOpen(true)` (with `ImGuiCond_Once`) is the caller's *starting* choice that the stored state wins over after the first frame. An inspector that is rebuilt (a different entity selected and the first one selected again) therefore comes back with the player's sections open and closed as they left them. Unity's inspector does the same for a component's foldout.

## Model adopted

- **S1. `section(title, open, children)`** is a column (`Role::Section`) of a header, the body and a note. The header is a full-width `Role::Button` (a target high, left-aligned, the button's fill, hover, ring and pressed look: one visual language) holding a chevron (the tree's: right when closed, down when open) and the title. The body (`Role::SectionBody`) holds the game's `children` bundle and is indented one padding, so what is inside reads as inside.
- **S2. State is `SectionOpen(bool)`** on the section's root: read it, set it from code and the section follows with no event. A player's Enter, Space, South or click sends one `ValueChange<bool>` on the section (APG's `aria-expanded`) and flips it. It is the widget's state, like `ChoiceValue`.
- **S3. A closed body is not laid out and holds no focus**: `Display::None` and `Visibility::Hidden` together (`Display::None` alone leaves `InheritedVisibility` alone, so Tab would still stop in it; `Hidden` alone leaves a hole). Focus that is inside a body that closes goes to the header.
- **S4. `SectionId(String)`** on the section keeps the state across a despawn and respawn: `SectionStates` (a resource, serde with the `serde` feature, so a game can save it with the player's settings) holds each id's last state. A section spawned with an id the resource knows starts as the resource says (the game's `open` is only the first choice, ImGui's `ImGuiCond_Once`); a section with no id keeps nothing.
- **S5. A section with no children shows "Empty"** (`Role::SectionNote`, muted) in the body's place when open, so an open section is never an unexplained blank; it follows children added later.
- **S6. Keys**: Enter, Space, South and a click (Bevy's `Button`); the arrows and the D-pad move focus through the header as through any button; a nested section is the same widget inside a body.

## Parameters

|part|value|
|---|---|
|header|`Role::Button`, `skin.target()` high at least, padding as a button, a half gap between chevron and title|
|body|indented `skin.pad()` on the left, `skin.gap` between rows|
|section|half a gap between header and body|
|chevron|the tree's `TreeClosed`/`TreeOpen`, in the header's ink (inverted while pressed)|
|state|`SectionOpen(bool)`, `SectionId(String)`, resource `SectionStates`|

## Validation targets

- The issue's acceptance: toggling hides the children from layout (a closed body is 0 px high and its children have no size) and from focus (Tab and the arrows never land in it); the state survives despawn and respawn with the same id.
- Enter, Space, South and a click each toggle once and send one `ValueChange<bool>`; a game's `SectionOpen` write moves the section with no event.
- Focus inside a closing body moves to the header.
- A section with no children shows "Empty", and loses it when a child is added.
- A header is at least `skin.target()` high in every look; its words and chevron are 4.5:1 on the header's fill at rest, hovered and pressed; the ring is whole.
- At 200% text nothing leaves its parent (`tests/large_text.rs`). Thirty idle frames write nothing.

## What great looks like

An inspector panel: a stack of quiet raised bars, each with a small chevron and a title, content indented under an open one with the same gap as everywhere else, a closed one only its bar; the focused bar's ring complete; a nested section reads as nested without a second visual language.

## Deviations

- APG's accordion puts each header in a heading and has no state store; the store is ImGui's practice and this issue's acceptance (design default).
- The header is a button's look rather than a bare row: a design default, so the focus ring, hover and press are the ones every other control has.

## Pending index

APG disclosure/accordion and the Dear ImGui FAQ are web pages not in home-still.
