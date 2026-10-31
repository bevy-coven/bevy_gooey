# A HUD lets the pointer through to the game under it

Researched 2026-10-30 (iteration 8), for issue #4 (`screen()` blocks every pointer pick under it: a HUD built on it hides the game world from picking). home-still's distill answered four searches ("HUD overlay should not block mouse input to the game world", "game UI input routing: which layer receives a pointer click", "mouse click on the interface accidentally issues a command in the game world", "scikit-learn API design sensible defaults"); the quotes below from the corpus are from those results. The engine and shipped-game sources are web copies fetched this pass.

## Sources

1. Llanos, S. C. & Jørgensen, K. (2011). *Do Players Prefer Integrated User Interfaces? A Qualitative Study of Game UI Design Issues.* DiGRA. doi:10.26503/dl.v2011i1.514 (craft; corpus stem `10.26503_dl.v2011i1.514`).
2. LaMothe, A. *Isometric Game Programming with DirectX 7.0* (Premier Press / Prima Tech, 2001), the mouse-handling chapter (craft; corpus stem *Isometric Game Programming with DirectX 7_0 w-CD …*): what a mouse-driven game does with the pointer.
3. Epic Games, *UMG Rich Text Block Properties* (the `Visibility` table; the same table is on every UMG widget) — https://dev.epicgames.com/documentation/en-us/unreal-engine/umg-rich-text-block-properties?application_version=4.27; GenerationLost, *[UMG] Canvas blocks screen space widget*, Epic Developer Community Forums, 2018-09-27 — https://forums.unrealengine.com/t/umg-canvas-blocks-screen-space-widget/434588; Michael Allar, *UE4 4.5 UMG Workaround: UI loses focus when clicking on no widget*, 2014-10-18 — https://allarsblog.com/2014/10/19/umgmenulosefocus/ (shipping practice: the engine many shipped games' HUDs are built in, and the trade-off a HUD root makes).
4. Godot Engine 4.7 documentation, *Control* (`mouse_filter`, `MOUSE_FILTER_IGNORE`) — https://docs.godotengine.org/en/stable/classes/class_control.html (technical: the same switch in the other engine a game UI kit is compared with).
5. Bevy 0.20.1 source: `bevy_ui/src/picking_backend.rs` (module docs), `bevy_picking/src/lib.rs` (`Pickable`, `Pickable::IGNORE`), `hover.rs` (`build_hover_map`), `window.rs` (`update_window_hits`), `bevy_input_focus/src/pointer_focus.rs` (`click_to_focus`) and `lib.rs` (`AcquireFocus`, `WindowTraversal`, `on_window_acquire_focus_clear`) (technical: what the engine does with a full-window node, and what a press on empty space does to focus).
6. Buitinck, L. et al. (2013). *API design for machine learning software: experiences from the scikit-learn project* (corpus stem `W1728842521`) (technical: the API's defaults).

## Craft principle

Llanos & Jørgensen, on what players of shipped games say about interface layered over the world: "once the players receive more information than they need, superimposed UI elements become annoying". A HUD is superimposed by definition; a HUD that also swallows the pointer is the worst form of it, because the player gets nothing back for the screen it covers.

LaMothe, on what a mouse-driven game does with the pointer: "If it's pointing at an item, pick it up. If it's pointing at a door, open it. Also, when the mouse is over something, provide a visual clue—a highlight, a different cursor, or just something that lets the user know that the mouse is over something that can be interacted with." The game world is where the pointer's meaning lives, so the layer drawn over it must pass the pointer through wherever it draws nothing.

## How shipping games do it

Unreal's UMG, the HUD toolkit of a great many shipped games, draws the line in its `Visibility` setting: "Hit Test Invisible - The widget is visible to the user but only as art. The cursor's hit tests will never see this widget. Self Hit Test Invisible - The widget is the same as Hit Test Invisible but doesn't apply to child widgets." The second is the HUD root's setting: the container is art, its buttons still work.

What a toolkit that forgets this costs, from a developer shipping with it (Epic forums, 2018): "a HUD widget that gets displayed on click of a button. It has a canvas panel and other widgets aligned on the sides. That means, a great part of the screen is basically not occupied by a widget and shows part of the game world, as intended. […] the HUD widget seems to block all mouse input across the screen, even if the mentioned part of the screen isn't occupied by an actual widget and the canvas panel is set to "self hit test invisible", so it shouldn't catch any mouse events."

And the other side of the same trade, from Allar (2014): with a modal widget up, "clicking in the empty/dead space of the viewport causes focus to shift back to the player controller. If you don't want this behavior, the easiest workaround is to set the root CanvasPanel's visibility in your modal widget to Visible instead of Self Hit Test Invisible. This will cause the CanvasPanel to absorb all click events." So a HUD root passes the pointer through, and a modal's root swallows it, by an explicit choice each; Godot's `Control` says the same in one line: "Set mouse_filter to MOUSE_FILTER_IGNORE to tell a Control node to ignore mouse or touch events."

## Model adopted

Bevy 0.20's UI picking backend is the second half of the facts: "Nodes without the [`Pickable`] component will still trigger events and block items below it from being hovered." (`bevy_ui` `picking_backend.rs`). `screen()` is a full-window node with no `Pickable`, so on every frame its empty part is the hovered entity at every pointer position, and a sprite or mesh under a HUD never sees `Over`, `Press` or `Click`.

- **P1. A screen is a layout root, not a surface: `screen()` carries `Pickable::IGNORE`.** "This entity will not block entities beneath it, nor will it emit events. If a backend reports this entity as being hit, the picking plugin will completely ignore it." (`Pickable::IGNORE`). It is UMG's Self Hit Test Invisible and Godot's `MOUSE_FILTER_IGNORE` on the container only: its children are separate nodes and keep their own picking.
- **P2. What draws something still blocks.** `panel()`, `button()`, `meter()`, text and every other widget stay as they are, so a click on a panel never reaches the world, and a tooltip's hover still works. A game's HUD is exactly these widgets on a transparent root.
- **P3. `dialog()`'s scrim keeps blocking.** A modal exists to take the pointer, Allar's "Visible" root. It is not built on `screen()` and is unchanged.
- **P4. A press on the empty part of a HUD still blurs the UI.** With the screen ignored, `bevy_picking` routes the press to the entity below, or to the window when nothing is below (`update_window_hits`: "listed behind all other pointer hits"). `click_to_focus` fires on either and the `AcquireFocus` request "bubbles via `WindowTraversal` until it reaches the window", which clears `InputFocus` and hides the ring, as it did when the press landed on the screen. A game that wants clicks on the world to leave focus alone owns that in its own observer.
- **P5. A screen that must swallow the pointer opts back in on the entity**: `commands.spawn(screen()).insert(Pickable::default())`. A tuple bundle cannot carry both (Bevy panics on a duplicate component), so the docs say `insert`. A design default, tested.

## Parameters

None. No setting is added: a component on one bundle, and the opt-back-in is Bevy's own `Pickable`.

## Validation targets

- Issue #4's test: a full-window pickable node under `screen()` is in the hover map with the pointer at a corner of the window and the HUD root is not.
- Over a `panel()`, a `button()` and a `meter()` on that screen the hover map holds the widget and not the node below: they still block.
- Over the empty part of an open `dialog()`'s scrim the hover map holds the scrim and not the node below.
- A real primary press on the empty part of a HUD is delivered to the node below (one `PointerPress`, none on the HUD root) and clears `InputFocus` and `InputFocusVisible`.
- `commands.spawn(screen()).insert(Pickable::default())` blocks again.
- Gallery, 1280x720 and 2560x1440: every capture is pixel-identical to the baseline (the change draws nothing); `GOOEY_KEYS=click,down` still clears focus and lands on Play with the ring.
- No frame-time cost: a component on one node.

## What great looks like

Invisible: the HUD is there and the game under it still answers the mouse. A sprite under a health meter's empty corner highlights, a click on it selects it, and the same click blurs a focused button. Nothing about the picture changes.

## Deviations

- Issue #4's fix direction is `Pickable::IGNORE` on `screen()`; adopted as written. Its test is the first of the validation targets, extended with the cases that must keep blocking (P2, P3) and the focus effect (P4), which the issue names as a consequence.
- Not adopted: `Pickable::IGNORE` on `menu()`, `label()`, `heading()` or `meter()`. A tooltip on any of them needs the hover (`Tooltip`'s hook adds `Hovered`), and the issue lists them as blocking by intent. A game with a text readout over the world sets `Pickable::IGNORE` on it itself. A design default.
- Not adopted: a `screen_blocking()` variant or a `GooeySkin` field. One path per feature: the default is the common case (a HUD), and the opt-in is one `insert`.

## Pending index

- Index the Epic UMG visibility table, the Epic forum thread, Allar's post and Godot's `Control` page into home-still, and search for a GDC talk on a shipped game's click-through HUD (RTS and city-builder UIs, where the world is the main interaction surface).
- `catalog_recent` (limit 200) on 2026-10-30 19:3x held only `W…` OpenAlex stems (embed events for a bulk import, no titles): no game-UI, HUD, menu, navigation or accessibility document that `corpus.md` lacks, so it is unchanged. `hs status`: both scribes and distill healthy, distill idle on CUDA; searches answered in 4 to 16 s.
