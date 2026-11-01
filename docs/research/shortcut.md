# Shortcuts: one chord type with a platform-correct label, a conflict check, a chord row and a hint on a tooltip

Researched 2026-11-01 (iteration 26). Issue #12 (G-08, `grimoire-ui`, `consumer:bevy_grimoire`) is the pass's item by the owner's rule: the lowest open request whose `Depends on` is all closed (#9 G-05 waits for this one). Menus (G-05), a command palette (G-07), a toolbar (G-20), a shortcut overlay (G-26) and a keymap editor all need the same four things: a value that names a key chord, the words that chord wears on a mac and on a PC, a way to find two commands on one chord, and a place to show it. Today `Binding::Key` is one `KeyCode` (`docs/research/binding.md` B-"No chords"), `Tooltip` is a bare `String`, and a game that wants "Ctrl+S" next to "Save" spells it by hand per platform.

## Sources

1. Malacria, Bailly, Harrison, Cockburn and Gutwin, *Promoting Hotkey use through rehearsal with ExposeHK*, CHI 2013, home-still `10.1145_2470654.2470735` (craft: why hotkeys go unused and what a shown shortcut buys), with Cockburn et al., *Supporting Novice to Expert Transitions in User Interfaces*, ACM Computing Surveys 2014 (`10.1145_2659796`, already in the corpus).
2. Microsoft, *Xbox Accessibility Guideline 107: Input* (home-still, quoted in `docs/research/binding.md`), on the remapping screens of *Grounded* and *Gears 5* (shipping: a remap that accepts what the player presses and is "represented correctly across any hints").
3. Apple, *Human Interface Guidelines: Keyboards* (https://developers.apple.com/design/human-interface-guidelines/inputs/keyboards/) and Microsoft, *Keyboard: Win32 apps UX guide* (https://learn.microsoft.com/en-us/windows/win32/uxguide/inter-keyboard) (technical: the order and the spelling of a chord on each platform). The scikit-learn API paper (corpus stem `W1728842521`) for the API: one small value type, constructors that cannot build a wrong value.

## Craft principle

Malacria et al.: "Despite this potential, hotkeys are under-used: several studies have demonstrated that few users employ any form of shortcut interface." And on why: "It shows that switching to traditional shortcuts involves a temporary 'performance dip' that discourages their use", because learning "involves moving the cursor to the point where a simple click would complete the selection – but then pausing, awaiting the shortcut feedback, and committing it to memory before proceeding". Cockburn et al. state the prize: "studies have consistently shown that keyboard shortcuts offer a higher performance ceiling than does mouse selection". A shortcut that is not shown where the click is never gets learned, so the chord has to be a first-class value every widget can print in the same words (R5 in `gooey.md`: "The novice's action rehearses the expert's").

## How shipping games do it

XAG 107: "Players should be given the option to remap all of the controls within the game itself ... This should ideally allow the player to assign an action to all potential game inputs, as opposed to simply swapping button assignments." Its *Grounded* example: "all keyboard inputs and mouse clicks can be remapped to other buttons or mouse clicks of the player's choice", and the label of the new mapping "is represented correctly across any hints". A tool or a game with modifier chords (a sprint-while-aiming layer, an editor's keymap) therefore needs the remap row to accept "Ctrl+Shift+K" as one input, and every hint to say the same words the row shows (`Binding::label` is already that single source).

## Model adopted

- **S1. `Shortcut { modifiers: Modifiers, key: KeyCode }`**, `Copy + Eq + Hash`, serde under the `serde` feature, reflected. `Modifiers { ctrl, shift, alt, meta }` are the *physical* modifiers (either side of the keyboard counts), never "primary": a saved keymap says what was pressed. `Modifiers::PRIMARY` and `Shortcut::primary(key)` build the platform's primary key (Meta/Command on a mac, Ctrl elsewhere) so a game spells "Save" once for both. Apple: "Prefer the Command key as the main modifier key in a custom keyboard shortcut."
- **S2. A modifier is never a shortcut's key.** `Shortcut::new(modifiers, key)` returns `None` for a modifier key, and a chord row waits through a modifier press (the issue's "Modifier-only presses are not shortcuts"). The fields stay public for data loaded from disk; `Shortcut::is_valid()` says whether one built by hand is sound.
- **S3. `Shortcut::label()` is the platform's spelling, `label_on(Platform)` is testable.** Apple HIG: "If you use more than one modifier key in a custom shortcut, always list them in this order: Control, Option, Shift, Command" (symbols ⌃⌥⇧⌘, no separator: "⌘⇧P"). Microsoft: "Present modifier keys in the following order: Windows logo, Application, Ctrl, Alt, Shift", joined by "+": "Ctrl+Shift+P", with "Super" as the Windows key's name in this crate's `key_name`. The key's own name is `model::key_name`, except a mac's glyphs for Enter, Backspace, Delete, Escape, Tab and the arrows. `Platform::current()` is the compile target (`cfg!(target_os = "macos")`); a web build on a mac cannot know, and says "Ctrl" (documented).
- **S4. `model::shortcut_conflicts(&[(&str, S)]) -> Vec<(String, String)>`**, generic over any `S: PartialEq` so `model` stays engine-free: every pair of commands, in list order, whose shortcuts are equal. Three commands on one chord are three pairs. Ctrl+S and Ctrl+Shift+S are not a conflict; the check does not guess a game's contexts (a command that is live only in one mode is the game's to filter before calling).
- **S5. `Shortcut::just_pressed(&ButtonInput<KeyCode>)`**: the key went down this frame with exactly these modifiers held (either side). It is the one place the chord rule lives, so a menu bar, a palette and a game's own handlers match chords the same way and Ctrl+Shift+S never also fires Ctrl+S.
- **S6. `Binding::Chord(Shortcut)`**: a `binding` row whose value is a `Chord` is a chord row (as a `Pad` value makes a pad row). It waits through bare modifier presses, binds the first non-modifier key with the modifiers held at that frame (none held: a chord with no modifiers, "F5"), exits on Escape, on a pad button and on a mouse press (a mouse button is not a shortcut), and sends one `ValueChange<Binding>`. A plain `Key` row keeps its rules: a modifier alone still binds (a game's "hold Shift to sprint" is a key row).
- **S7. `Tooltip { text, shortcut }`**, built `Tooltip::new("Save").with_shortcut(Shortcut::primary(KeyCode::KeyS))`. The box shows the line and the shortcut on a keycap chip (`Role::PromptKey`, the prompts' keycap) to its right, so the same chord reads the same in a hint and a tooltip. The chip exists exactly while the shortcut does; editing the `Tooltip` in place updates both. This replaces the tuple struct `Tooltip(String)` (a clean cutover: every caller moves to `Tooltip::new`).

## Parameters

| value | choice |
| --- | --- |
| mac order, glue | ⌃ ⌥ ⇧ ⌘, none |
| other order, glue | Super, Ctrl, Alt, Shift, "+" |
| chip | the prompts' keycap: `PromptKey` border, fill and text colour, a gap after the line |
| conflict result | every pair in list order, none for a single command |

## Validation targets

- Ctrl+Shift+P labels "Ctrl+Shift+P" and, on a mac, Meta+Shift+P labels "⇧⌘P" and Ctrl+Meta+F "⌃⌘F"; Ctrl+Alt+Shift+Super+X in the Microsoft order; Meta alone is not a shortcut; `Shortcut::new` refuses `ShiftLeft`.
- Two commands bound to Ctrl+S are reported as one conflict; Ctrl+S and Ctrl+Shift+S are not; three on one chord are three pairs; a command is never paired with itself.
- A chord row bound to Ctrl+S captures Ctrl+Shift+K (pressed Ctrl, then Shift, then K over three frames, and all in one frame), sending one `ValueChange<Binding::Chord>`; Ctrl alone binds nothing and keeps the capture; Escape exits and is never bound; a mouse click or a pad button exits; a key row still binds Shift alone.
- `Shortcut::just_pressed` is true on the frame of Ctrl+S, false for Ctrl+Shift+S, false for S alone, and either Ctrl counts.
- A tooltip with a shortcut shows the chip beside the line, at the keycap's contrast in every preset, inside the window at 200% text; removing the shortcut removes the chip; nothing is written while idle.

## What great looks like

A hint that reads like a shipping tool's: "Save" and a small raised "Ctrl+S" cap, the same cap the legend draws, so one visual language says "this is an input". In the demo the player presses Enter on a chord row, holds Ctrl, Shift and taps K, and the cap changes from "Ctrl+P" to "Ctrl+Shift+K"; a second row bound to the same chord shows a visible conflict line the moment it is bound, and clears when it is rebound.

## Deviations

- The issue's `model::shortcut_conflicts(&[(&str, Shortcut)])` is generic over the shortcut type (assumed decision): `model` holds no engine type, and `Shortcut` carries Bevy's `KeyCode`. The call in the issue compiles unchanged.
- The issue's "Cmd and Ctrl map to the platform's primary key" is `Modifiers::PRIMARY` and `Shortcut::primary` (assumed decision), not a match rule that treats a held Ctrl as a held Cmd: a mac player's Ctrl+S is not Cmd+S, and a keymap editor must be able to bind both.
- `Tooltip(String)` becomes `Tooltip { text, shortcut }` with `Tooltip::new` (assumed decision): a shortcut needs a place, a second component cannot give `.with_shortcut` on `Tooltip`, and appending the label to the string would lose the chip. A breaking change, so the next release is a minor bump.
- A chord row exits on a mouse press, where a key row binds it (assumed decision): a mouse button cannot be part of a `Shortcut`.

## Pending index

- Index Apple's *Human Interface Guidelines: Keyboards* and Microsoft's *Keyboard (Win32 UX guide)* pages; neither is a paper, so `paper_search` has nothing to download. The two quotes above are from the pages as read on 2026-11-01.
- Search later for: "keymap conflict detection context-dependent keybindings", "key chord capture modifier-only press".
