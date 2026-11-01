# Context menus: a menu that belongs to a thing, opened where the player points or where focus is

Researched 2026-11-01 (iteration 28). Issue #10 (G-06, `grimoire-ui`, `consumer:bevy_grimoire`) is the pass's item: right-click on a piece in the world or a row in a list is the fastest door to a command. A game's inventory slot ("Use / Drop / Inspect"), a level editor's selected object, a mod list's row all want one menu per thing, opened at the pointer, and reachable without a mouse.

## Sources

1. Malacria, Bailly, Cockburn and others, *Promoting Hotkey use through rehearsal with ExposeHK*, CHI 2013, home-still `10.1145_2470654.2470735` (craft: a menu is also how a player learns the chord, so the chord stays on every item, and the menu must still be usable "using the pointer only").
2. XAG 112 on *Sea of Thieves* (Xbox Accessibility Guidelines), quoted in `docs/research/cancel.md` and `docs/research/menu.md` (shipping: backing out of a layer is always one step, with focus given back).
3. Wikipedia, *Context menu* (https://en.wikipedia.org/wiki/Context_menu), summarising Microsoft's and Apple's guidance, and the W3C WAI-ARIA APG *Menu Button* and *Menu* patterns already quoted in `docs/research/menu.md` (technical: the keys that open one, and where a keyboard-opened menu appears).

## Craft principle

ExposeHK: "users can still browse the menus and select the items using the pointer only as with a regular menubar if preferred." A menu teaches by being shown; the player who never learned the chord opens the menu and reads it. A context menu is the same menu, narrower: only what applies to the thing under the pointer, so Hick's Law (`docs/research/menu.md`) works for it rather than against it. The menu's items carry their chords (`menu_item(..).with_shortcut(..)`), unchanged.

## How shipping games do it

XAG 112 on *Sea of Thieves*: "regardless of where the player is in the UI navigation sequence, they're always provided the ability ... to go back." A context menu is a layer opened over what the player was doing: Escape (or B) closes it, focus goes back to where it was, and the thing the player pointed at is still the thing in hand. Strategy and RPG inventories (right-click a slot for Use, Split, Drop) and editors (right-click a node) open the menu with its top-left corner at the cursor, or flipped to stay on screen when the cursor is near an edge.

## Technical: how it opens

Wikipedia, *Context menu*: "Application key or Shift+F10 opens a context menu for the region that has focus." And: "If the context menu is being triggered by keyboard, such as by using Shift + F10, the context menu appears near the focused widget instead of the position of the pointer, to save recognition efforts." So three openings, one menu: a secondary click opens it at the pointer; Shift+F10 and the Menu key open it at the widget that has focus. Windows opens on the release of the button, macOS on the press; this crate follows `PointerClick` (press and release on the same thing), so a drag that starts on the thing and ends elsewhere opens nothing.

Bevy 0.20 supplies the placement (`Popover` with candidate positions, which the crate's `popover` and `menu` already use: `Placement::AtPointer`, `Placement::Below`), the picking (a secondary `PointerClick` bubbles from the picked entity up `ChildOf`, for UI nodes and for sprites and meshes carrying `Pickable` alike) and the menu rows (`menu_item`, `check_item`, `radio_item`, `submenu`, `separator`, `Disabled`: iteration 27).

## Model adopted

- **C1. A context menu is a menu panel of its own and a thing names it.** `context_menu(items)` spawns the same panel a menu title owns (`menu_panel`: absolute, `Display::None`, scrolling, in no scope) as a root, its `items` live entities, so `observe(|_: On<Activate>| ..)` sits where the game wrote it. `ContextMenu(menu)` on any entity (a UI node, a sprite, a mesh, a row) says "this is my menu". One menu may be named by many things (every row of a list shares one), which is why the menu is not a child of its thing: a UI panel cannot hang under a sprite and be laid out, and a menu shared by many cannot hang under all of them.
- **C2. Three ways to open it, one rule.** A secondary `PointerClick` that reaches an entity with `ContextMenu` (the nearest one up the hierarchy, so a picked child opens its parent's menu, and the event goes no further) opens it at the pointer (`Placement::AtPointer`, which flips at the window's edge); Shift+F10 and the Menu key open the menu of the focused widget, or of its nearest ancestor that has one, below the focused widget (`Placement::Below`). Opening is the popover's: modal `TabGroup` and `Dialog`, focus goes to the first item, Escape, B or a press outside closes it and focus goes back, the press that opened it is never outside.
- **C3. What it was opened for.** While open, the panel carries `ContextTarget(entity)`: the entity that has the `ContextMenu` (not the widget a keyboard opened it beside). A game reads it in an item's `Activate` observer (walk `ChildOf`, `ContextTarget::of`) and in an `On<Add, ContextTarget>` observer, which runs before the menu is first drawn, to enable, disable or relabel items for this thing.
- **C4. A second right-click moves it.** The press of the second click is outside the panel, so the first closes; the click then opens the menu again at the new point; opening an already open menu closes and reopens it (one menu at a time, a context menu opening another closes the first). A right-click on the open menu does nothing.
- **C5. F10 stays the bar's.** F10 with Shift held is not `focus_menu_bar`'s: Shift+F10 is the context menu's only.
- **C6. No new look.** Everything drawn is a menu panel's.

## Parameters

| value | choice |
| --- | --- |
| trigger, pointer | secondary `PointerClick` |
| trigger, keyboard | Shift+F10 (exactly Shift held) and `KeyCode::ContextMenu` |
| placement | `AtPointer`; `Below` the focused widget for a key |
| open focus | first item, focus shown (the popover's rule) |
| one at a time | yes |

## Validation targets

- A right-click on a picked world sprite opens its menu with the first corner at the pointer; Shift+F10 on a focused button opens that button's menu below it (`tests/context_menu.rs`).
- A second right-click elsewhere moves the menu; a right-click on a different thing opens that thing's; a right-click on the menu itself does nothing.
- Near the right and bottom edge the menu stays inside the window.
- Enter chooses an item once and closes the menu; Escape closes it and gives focus back; a press outside closes it once and sends one `PopoverClosed`.
- `ContextTarget` is the thing the menu was opened for, while open only; one menu shared by two things reports the right one each time.
- Shift+F10 never focuses a menu bar; Shift+F10 with no menu above the focus does nothing; a menu the game despawned opens nothing and panics nothing.
- Nothing is written while the menu is closed and the app idles.

## What great looks like

The look of the menu bar's cards (iteration 27): an opaque card with the popover's frame and shadow, rows lighting as full-width bars with the ring, chords hard against the right edge, with its top-left corner a few pixels from the cursor as the platform's own does, flipping up and left when the cursor is near the bottom-right.

## Deviations

- The issue proposes `ContextMenu(Vec<MenuEntry>)`, a menu as data on the entity (assumed decision, owner may override). Menus here are live entities (iteration 27, decision 1: a game's `observe` sits on the item), so a data list would need a second way to react to a choice. `ContextMenu(Entity)` names the live menu instead, which also lets one menu serve every row of a list.
- The menu is opened on the click, which is the release (Windows); macOS's press-to-open is not offered.
- Menus do not open on a long press (touch); a game that wants it can send the same `PointerClick` from its own gesture. Not built.

## Pending index

- home-still has no document on context menus. `distill_search` ("context menu right-click keyboard accessibility", "pie menu marking menu context menu selection time", 2026-11-01) returned the ExposeHK paper and VR marking-menu papers only. To index: Microsoft's *Context Menus* UX guideline (the learn.microsoft.com URL of the Win32 UX guide returned 404 on 2026-11-01) and Apple's HIG *Contextual menus*.
