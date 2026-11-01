# Menu bar and dropdown menus: a row of titles, a card of commands, a submenu on a key

Researched 2026-11-01 (iteration 27). Issue #9 (G-05, `grimoire-ui`, `consumer:bevy_grimoire`) is the pass's item: a desktop tool's menu bar is where a new user finds a command and learns its chord, and it is the first consumer of `popover_modal` (#8) and `Shortcut` (#12). A game's pause menu, a level editor's File and Edit, a mod browser's sort menu all want the same row of titles, each opening a card of items with checks, radio groups, separators, submenus and right-aligned shortcuts, walkable by key and by pad.

## Sources

1. Li, Bengio and Bailly, *Predicting Human Performance in Vertical Menu Selection Using Deep Learning*, CHI 2018 (arXiv 1803.05073), home-still `10.48550_arXiv.1803.05073` (craft: what a vertical menu costs the player as it grows).
2. XAG 112 on *Sea of Thieves* (Xbox Accessibility Guidelines), home-still, quoted in `docs/research/cancel.md`; and the Game Accessibility Guidelines' reading of *Halo Infinite*, quoted in `docs/research/popover.md` (shipping: backing out of a layer is always available and always one step).
3. W3C WAI-ARIA Authoring Practices, *Menu and Menubar Pattern* (https://www.w3.org/WAI/ARIA/apg/patterns/menubar/) and Bevy 0.20's own `bevy_ui_widgets::menu` (technical: the keyboard contract every menu shares, and what the engine already ships). The scikit-learn API paper (`W1728842521`) for one way to do a thing.

## Craft principle

Li et al. on why a menu is judged by its length: "Hick's Law [11] is also a well-known model that describes the time required for an expert human to make a decision of choosing among a given number of options." Time to choose grows with the number of items on show, so the menu's job is to keep the list short and grouped: a separator between groups, a submenu for the rare, a chord on the item so the expert never opens it at all (`docs/research/shortcut.md`: "Despite this potential, hotkeys are under-used"). A menu that cannot say "Nothing here" for an empty group leaves the player wondering whether it is broken.

## How shipping games do it

XAG 112 on *Sea of Thieves*: "regardless of where the player is in the UI navigation sequence, they're always provided the ability (and the important visual interaction prompt on the bottom-left corner) to go back." A cascade of menus is a stack of places; every one is left with the same key, one level a press, with the opener lit and focused again. On pad and keyboard alike the first press goes up a level and never straight out of the whole cascade. The Halo Infinite reading quoted in `popover.md` is the other half: opening a layer puts focus inside it.

## The keyboard contract (APG)

"When a `menu` opens, or when a `menubar` receives focus, keyboard focus is placed on the first item." "`Enter`: When focus is on a `menuitem` that has a submenu, opens the submenu and places focus on its first item. Otherwise, activates the item and closes the menu." "`Down Arrow`: When focus is on a `menuitem` in a `menubar`, and that `menuitem` has a submenu, opens the submenu." "`Right Arrow`: When focus is in a `menu` and on an item that does not have a submenu ... Closes the submenu and any parent menus. Moves focus to the next item in the `menubar`." "`Left Arrow`: When focus is in a submenu of an item in a `menu`, closes the submenu and returns focus to the parent `menuitem`." "`Escape`: Close the menu that contains focus and return focus to the element or context, e.g., menu button or parent `menuitem`, from which the menu was opened." And two notes this crate takes whole: "Disabled menu items are focusable but cannot be activated." "A separator in a menu is not focusable or interactive."

Bevy 0.20 ships `bevy_ui_widgets::menu` (`MenuPopup`, `MenuItem`, `MenuButton`). Its own docs say what it is not: "menu popups only remain open as long as some child of the popup has focus", "two menus cannot be displayed at the same time unless one is an ancestor of the other", and its focus-loss handler carries "TODO: Change this logic when we support submenus". A menu bar needs a second menu open the moment the first closes, a disabled item that can be focused to read why, and a cascade; so this crate wraps what it already wraps for a popover (Bevy's `Button` for activation, `Popover` for placement) and does not use `MenuPopup` (see Deviations).

## Model adopted

- **M1. Five constructors.** `menu_bar(titles)`, `menu_title(label, items)`, `menu_item(label)`, `check_item(label, checked)`, `radio_item(label, checked)`, `submenu(label, items)` and `separator()`; `items` and `titles` are `children![..]` of the others, so a menu reads as the tree it is. `menu_item` returns a [`MenuItem`], a component (so `.with_shortcut(..)` chains) whose hook builds the row's parts; everything else a free function returning `impl Bundle`.
- **M2. A menu is spawned once and shown by opening.** The panel under a title (and under a submenu item) exists from the start with its items as live entities, so `observe(|_: On<Activate>| ..)` in the items sits where the game wrote it and a check item's `Checked` is the game's to read. Closed, it is `Display::None`, `Visibility::Hidden` and in no scope; open, it is an ordinary modal popover (`PopoverPanel`, `Dialog`, modal `TabGroup`) placed below its title (right of its item for a submenu) by the machinery of #8: outside press, `Cancel`, focus in and focus back, outermost close first. No second placement or focus path.
- **M3. The bar.** F10, or Alt pressed and released with no other key between, puts focus on the first title (the focus it left is kept and Escape from the bar restores it). Left and Right walk the titles and wrap (APG: "optionally wrapping"); Down, Enter, Space and a click open the menu with focus on its first item; a click on an open title closes it; pointer over another title while a menu is open opens that one instead (every desktop menu bar does).
- **M4. In a menu.** Up and Down are the crate's spatial navigation inside the modal scope; Home and End the first and last item; Right opens a submenu item's cascade, and on any other item moves to the next title and opens its menu; Left closes a submenu (focus back on its item) and, in a title's menu, moves to the previous title; Escape or the pad's B closes the level focus is in and gives focus to its opener; Enter, Space, South and a click activate and close the whole cascade (a submenu item opens instead).
- **M5. Items.** A plain item sends Bevy's `Activate` and closes the cascade. A check item flips its own `Checked` and sends `ValueChange<bool>`; a radio item sets its own and clears the radio items in its run (the siblings between two separators), sending `ValueChange<bool>` for the one chosen. Neither is a Bevy radio group: a menu's radios are a run of siblings. A disabled item (`Disabled(reason)`: Bevy's `InteractionDisabled` plus a `Tooltip` of the reason) is focusable, never activates, and the reason shows on hover or visible focus. A submenu with no items shows one muted "Nothing here" row.
- **M6. Look.** The bar is the panel's colour; a title is a button's size with no frame at rest, the hovered fill under the pointer or while its menu is open, and the accent while pressed. The card is a popover's (opaque, outline frame, shadow). An item is a row of a mark slot, the label, the shortcut and a chevron: the slot is 1.1 text sizes wide and exists in every item of a menu that has a check or a radio item in it (so labels line up), and in none of a plain menu (the first capture kept it everywhere and wasted 57 px at the left of every plain menu, 80 px at 200% text), the shortcut is `text_muted` at rest and the label's colour while the row is lit, so contrast is the button's. A row is at least `target()` tall. A separator is one line of `outline` with half a gap above and below. A menu that is taller than the window leaves a gap at the top and bottom and scrolls.
- **M7. One place for each key.** Left, Right, Home and End on a title or an item are the menu observers' (`navigate_by_input` leaves them, as it does a slider's); Down on a title opens its menu and does not move focus. Up and Down in a menu are navigation's. F10 and Alt are `focus_menu_bar`, gated by `not_capturing`.

## Parameters

| value | choice |
| --- | --- |
| row height | `target()` (44 px at 720p) |
| item padding | `pad() * 1.5` across, `pad() * 0.6` down, a button's |
| mark slot | 1.1 text sizes wide, only in a menu with a check or a radio item |
| gap label to shortcut | two gaps and half a text size |
| card min width | 10 text sizes |
| separator | one line, half a gap margin |
| gap from title | half a gap and a frame (the popover's) |
| Alt tap | pressed and released with no other key between |

## Validation targets

- F10, Right, Down, Enter: the second menu's first item activates once (`tests/menu.rs`); a disabled item never activates by Enter, Space, South or a click, and shows its reason.
- Shortcuts are right-aligned in a column: every item's shortcut ends on the card's inner right edge, and labels start on one x.
- Escape closes one level, restores focus to the opener, then (in the bar) to where F10 was pressed; a press outside closes the cascade once.
- A check item flips and reports once; a radio run keeps exactly one on; a separator is never focused.
- An empty submenu says "Nothing here". A long menu stays inside the window and its focused item is scrolled into view.
- At 200% text nothing leaves its parent; contrast of label, shortcut and mark in every preset at rest and lit; whole-pixel frames; nothing written while idle.

## What great looks like

The look of a desktop menu in the skin's own language: a quiet bar, a title that lights under the pointer and stays lit while its card is open, a card that floats a half gap under it with the popover's frame and shadow, rows that light as one full-width bar with the focus ring, a check or a dot in a column of its own, chords in muted text hard against the right edge, thin separators grouping the rest. Keyboard and pad get the same: the ring is on the item, the title stays lit, Escape peels one level.

## Deviations

- **Bevy's `bevy_ui_widgets::menu` is not wrapped** (assumed decision): it closes a menu when focus leaves it, cannot hold two open at once, and has no submenus (quoted above); the popover of #8 is the crate's one close path, and `Button` and `Popover` are what is wrapped.
- `.with_shortcut(..)` on a `menu_item` needs a value that is a bundle and has methods, so `MenuItem` is a component with a build hook, as `Tooltip` is; `Disabled(String)` is likewise a component whose hook adds `InteractionDisabled` and the `Tooltip`.
- Right and Left across the bar open the neighbour's menu *with focus in it* (APG's second option), where APG recommends opening it without moving focus; one rule for every way a menu opens.
- No hover delay or safe triangle for submenus: a submenu opens on Right, Enter, Space or a click. A pointer player's cascade by hover is in the backlog.
- A menu item has no type-ahead (APG: optional).
- Radio items' group is the run between separators, not a named group.

## Pending index

- home-still has no document on menu bars or cascading menus. `distill_search` ("cascading menus submenu hover delay triangle", "menu bar keyboard navigation accessibility", 2026-11-01) returned VR menu and Hick's-law papers only. To index: the APG *Menu and Menubar* page, Apple's HIG *Menus* and Microsoft's *Menus and toolbars* UX guide, and Ahlström's *Modelling and improving selection in cascading pull-down menus using Fitts' law, the steering law and force fields* (CHI 2005).
