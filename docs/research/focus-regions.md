# Focus regions for tools

Researched 2026-11-02 (iteration 40, #27 G-23). home-still's index has nothing on pane navigation (the corpus re-scans of iterations 37 to 39 found no game-UI paper), so the sources below were fetched today.

## Sources

1. W3C, *Understanding SC 2.1.2 No Keyboard Trap*, https://www.w3.org/WAI/WCAG22/Understanding/no-keyboard-trap.html — craft (accessibility).
2. Microsoft, *Visual Studio Code accessibility*, "Focus Next Part" and "Focus Previous Part" (F6 and Shift+F6 in the default keymap), https://raw.githubusercontent.com/microsoft/vscode-docs/main/docs/configure/accessibility/accessibility.md — shipping tool.
3. This crate's own `nav.rs` (`navigate_by_input`, `FocusScopes`) and Bevy 0.20 `bevy_input_focus::tab_navigation::TabIndex` / `TabGroup` — technical.

## Craft principle

WCAG 2.1.2: "If keyboard focus can be moved to a component of the page using a keyboard interface, then focus can be moved away from that component using only a keyboard interface, and, if it requires more than unmodified arrow or tab keys or other standard exit methods, the user is advised of the method for moving focus away." Its own example is the case a tool's viewport is: "When the editing area receives focus, an instruction appears, letting users know that they can use Alt+F10 / Option+F10 to move focus back out of the area and to the editor toolbar."

So a region that keeps the arrow keys for itself is allowed, if Tab (an "unmodified … tab key") still leaves it and the way out is known. A `Host` region here never takes Tab, so it can never trap.

## How shipping games do it

Visual Studio Code, the tool the editor is closest to: "For quick navigation across the workbench, we recommend using **Focus Next Part** … and **Focus Previous Part** commands." Those are F6 and Shift+F6: they cycle the workbench's parts (activity bar, side bar, editor, panel, status bar) and land on the part's first control, wrapping at the end; "Once focus is in the Status bar via Focus Next Part, you can use arrow navigation to move focus between Status bar entries." The parts are named by the host, are few, and are visited in layout order.

## Model adopted

- **R1. `FocusRegion { arrows: ArrowPolicy }`**, a component on any node (a panel, a viewport, a toolbar). `ArrowPolicy::Navigate` (default): nothing changes, a region only takes part in F6. `ArrowPolicy::Host`: with focus on the region or inside it, the arrow **keys** move no gooey focus (`navigate_by_input` returns), and stay readable in `ButtonInput<KeyCode>` for the host as they always are.
- **R2. F6 and Shift+F6 (exactly, no Ctrl, Alt or Meta)** move focus to the first stop of the next (previous) region, wrapping, in the order the regions stand in the UI tree (sibling order from the root), the order a game spawned them in. A region whose subtree has no visible tab stop is skipped; a region that is itself a tab stop (a viewport with a `TabIndex`) is its own first stop. With focus in no region F6 goes to the first and Shift+F6 to the last. Focus is shown (`InputFocusVisible`).
- **R3. A dialog holds focus over every region.** Regions are those of the scope the player sees (`FocusScopes::active`): an open dialog's own regions only, and outside it nothing is reachable.
- **R4. The D-pad still navigates** out of a `Host` region (a pad has no Tab; the arrows are keys). `not_capturing` gates F6 like every navigation system. Tab is Bevy's and untouched.
- **R5. Enter.** A `Host` region's content is the host's own node (a viewport), which gooey gives no Enter behaviour; a gooey `Button` inside one still activates on Enter (Bevy's), as a widget the host chose to put there.

## Parameters

F6 and Shift+F6 are fixed (VS Code, Windows and browsers use them for panes). No timing.

## Validation targets

With focus in a `Host` region: ArrowUp, ArrowDown, ArrowLeft and ArrowRight move no focus and `ButtonInput::just_pressed` still reads the key. F6 moves focus to the next region's first widget, Shift+F6 back, both wrapping. A `Navigate` region's arrows move focus as before. An open dialog keeps F6 inside itself. A region with nothing focusable is skipped. Nothing written while idle.

## Deviations

Design default: the D-pad leaves a `Host` region (the issue says "arrow keys").

## What great looks like

A tool's viewport that keeps its camera keys while the panels on both sides stay keyboard reachable, with the focus ring on the first control of the region F6 lands on, at once.

## Pending index

All three sources, in home-still (WCAG Understanding documents, VS Code accessibility doc).
