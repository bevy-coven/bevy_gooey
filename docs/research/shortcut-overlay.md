# Shortcut overlay: hold a modifier and every shortcut in view shows itself

Researched 2026-11-02 (iteration 41). Issue #30 (G-26, `grimoire-ui`, `consumer:bevy_grimoire`; depends on #12, shortcuts, closed). A tool's commands have chords the novice never learns because the chord is in a tooltip they never wait for. A game's hotbar and a build menu have the same problem.

## Sources

1. Malacria, Bailly, Harrison, Cockburn, Gutwin, *Promoting Hotkey use through rehearsal with ExposeHK*, CHI 2013, doi:10.1145/2470654.2470735, home-still `10.1145_2470654.2470735` (craft).
2. Microsoft Office's Alt-key "KeyTips", described in the same paper, and the shortcut hints of Blender and Photoshop (shipping).
3. Bevy 0.20 `ComputedNode`, `InheritedVisibility` and `ButtonInput<KeyCode>`, and this crate's `shortcut.md` (technical).

## Craft principle

ExposeHK: "When activated with a modifier key (e.g., the Control key), EHK displays the hotkeys in the application's toolbar (see Fig.1), menu, or ribbon. Importantly, EHK is compatible with existing toolbar and ribbon designs, and ..." and: "Intermediate users can draw on their spatial knowledge of commands to rapidly find hotkey bindings." The rehearsal works because the badge is drawn **on the control the player already points at**, so the location they know becomes the chord they learn, and because the display is a mode they hold, so nothing is left on screen.

## How shipping games do it

The paper itself describes the product it compares with: "Alt-Key navigation (part of the accessibility interface in Microsoft Office) displays hotkeys associated with successive levels of the interface hierarchy when the 'Alt' key is pressed". Both hold a key to see the keys and show them where the commands are.

## Model adopted

- **S1. `ShortcutBadges { hold: Modifiers, delay_seconds: f32 }`**, a resource (`Default`, serde, reflected; `GooeyNavPlugin` inits it). `hold` is the modifier set that reveals; `Modifiers::default()` (none) turns the overlay off.
- **S2. A widget has a shortcut when it carries a `Shortcut` component** (`Shortcut` becomes a `Component`), **or a `Tooltip` with one** (a game that wrote the chord for the tooltip gets the badge for free: one declaration). The component wins when both exist.
- **S3. While exactly `hold` is held** (`Modifiers::held`, either side counts, nothing else down: Ctrl+Shift is not Ctrl) for `delay_seconds`, one badge per widget that has a shortcut and is visible (`InheritedVisibility`) and on screen (its rect intersects the window): a `Role::ShortcutBadge` child (the keycap look of a tooltip's chord) with the chord's label, `Pickable::IGNORE`, absolute at the widget's top-right corner, `GlobalZIndex` above the interface but under a toast. Releasing (or pressing any other modifier, or a text field gaining focus) removes them the same frame. The delay stops a Ctrl+C from flashing the screen (design default 0.3 s).
- **S4. Badges never cover the focus ring**: the badge sits outside the widget's box at its top-right corner (half out, half in) and is shifted by the ring's border so the ring's corner stays whole. Off-screen widgets show none. [Checked by a test of the rects.]
- **S5. Only the primary chord is shown, in `Shortcut::label()`'s platform words.** A chord that is held down while the overlay is up still fires the widget's command: the overlay is display only and the crate does not run chords.
- **S6. A widget the overlay badges is not otherwise changed**: the badge is a child, spawned and despawned; nothing is written to the widget.

## Parameters

`delay_seconds` 0.3 (design default: ExposeHK's paper reports users activating on purpose; a deliberate hold is longer than a chord's key press, which is under 0.15 s). `hold` default: `Modifiers::PRIMARY`'s counterpart? The default is `Modifiers::ALT`... see Deviations.

## Validation targets

Holding the configured modifier spawns one badge per visible widget with a shortcut; releasing removes them; a hidden widget, an off-screen one and one under `Display::None` get none; badge text equals `Shortcut::label()`; a badge is inside the window and does not intersect the focused widget's ring box; idle (no modifier) frames spawn and write nothing; frame time unchanged with no modifier held (one resource read).

## Deviations

- **Default `hold`.** The issue gives no default. Decision: `Modifiers::ALT` on every platform (Alt is Option on a mac). Options: the platform's primary (Ctrl/Cmd, ExposeHK's own: but Ctrl is held for multi-select and Cmd+Tab, so the overlay would flash on every chord), Alt (free in most tools, Office's KeyTips key), or off by default. Picked Alt for being the least used alone, and because `focus_menu_bar` already treats an Alt tap as the menu key: the overlay needs the hold delay to stay out of its way (a tap is released before 0.3 s). Recorded as an assumed decision.
- **`delay_seconds` is an addition to the issue's `ShortcutBadges { hold }`.** A resource with a field has a `Default`, so the issue's one-field literal needs `..default()`.

## What great looks like

A badge that reads like a keycap on the control, in the skin's own `PromptKey` look, 4.5:1 text and a 3:1 frame, small enough not to hide the control's label (it sits on the corner, not over the words).

## Pending index

Blender's and Photoshop's hint behaviour are not in home-still.
