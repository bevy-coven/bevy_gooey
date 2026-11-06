# Chord glyphs: how a host whose face cannot draw ⌘ ⌃ ⌥ ↩ ⎋ ⇥ says its chords

Researched 2026-11-06 (iteration 64). Issue #45 (`bug`, `consumer:bevy_grimoire`) is the pass's
item: on macOS every chord the crate draws wears Apple's modifier glyphs (`⌘ ⇧ ⌥ ⌃`) and its named-key
glyphs (`↩ ⌫ ⌦ ⎋ ⇥`), and a host whose font has none of them draws a tofu box instead. The owner
pointed at the Undo tooltip's keycap and wrote *"Why is there a '?' here?"* — the cap draws `?Z`
where its accessible text reads `⌘Z`.

**Measured, and worse than the issue says.** The thirteen codepoints `model::shortcut_label` can emit
with `mac == true` are `⇧ ⌫ ⌦ ← ↑ ↓ → ⌘ ⌃ ⌥ ↩ ⎋ ⇥`. Bevy 0.20's own bundled face,
`bevy_text-0.20.0/src/FiraMono-subset.ttf` (the `default_font` feature's source, so the crate's
default `GooeyFonts`), maps **95 codepoints and not one of the thirteen**. So the defect is not a
property of one host's font choice: with the crate's defaults, every mac chord in a tooltip, a menu,
a badge or a legend is tofu. The host's `FiraMono-Regular.ttf` (1,350 codepoints) is the generous case:
it has the arrows, `⇧`, `⌫` and `⌦`, and misses seven.

## Sources

1. **Craft:** Malacria, Bailly, Harrison, Cockburn and Gutwin, *Promoting Hotkey use through rehearsal
   with ExposeHK*, CHI 2013 (home-still `10.1145_2470654.2470735`), on why a chord has to be shown
   where the click is; and Lightbown, *Designing the User Experience of Game Development Tools*, 2015
   (home-still `lightbown_designing_ux_of_game_dev_tools_2015`), on legibility in exactly the kind of
   tool that filed this issue.
2. **Shipping game / shipped product:** Microsoft, *Xbox Accessibility Guideline 107: Input*
   (https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/107) — a
   remapped input must be "represented correctly across any hints, tips, tutorials, or controller map
   schemes"; and Apple, *Human Interface Guidelines: Keyboards*
   (https://developers.apple.com/design/human-interface-guidelines/inputs/keyboards/), which is where
   the glyphs come from: "the modifier keys are Control, Option, Shift, and Command", shown in that
   order as `⌃⌥⇧⌘`.
3. **Technical:** Bevy 0.20's text stack — `bevy_text::Font` (raw bytes, `Blob<u8>`),
   `TextFont`/`FontSource`, and `parley`/`fontique` 0.11, whose `Charmap::map` can answer "does this
   face have this codepoint" and which Bevy exposes only inside `text_system`, with no query a game
   system can make. Plus the cmap measurement above, taken with `fonttools`.

## Craft principle

ExposeHK: "Hotkeys are powerful and efficient but are rarely used, even by expert users", and its
remedy is to show the chord *in the menu, beside the command*, so novices rehearse it. A chord that is
drawn cannot be learned; a chord drawn as a box is worse than one not drawn at all, because it costs
the room and teaches the player that the interface is broken.

Lightbown, on the audience that filed this issue — people building games in a tool all day: "Although
our brain works hard to compensate for the limitations of our eyes, there are some things that it
simply cannot do. To ensure that the user is able to see the visual language that we have designed, we
must also consider the ability of our eyes to see contrast." The same argument holds for a glyph: the
eye cannot recover a shape that was never drawn. The interface's own vocabulary must survive the face
the host shipped.

## How shipping games do it

XAG 107: "When a player remaps a control within the game, the labelling of the new mapping is
represented correctly across any hints, tips, tutorials, or controller map schemes." "Correctly" is
about the *player reading it*, not about matching a platform's typography: a box is not correct.

Apple's HIG is the source of the notation and is explicit that the symbols are the modifiers:
"the modifier keys are Control, Option, Shift, and Command", written in that order, as `⌃⌥⇧⌘`, main key
last. Apple applies this to *macOS menus*, where the system font is guaranteed to carry the glyphs.
A game's or a tool's own interface has no such guarantee — the face is the host's choice, and a
monospace coding face (the normal case for a dev tool) routinely ships Latin only.

So the practice splits along the line this issue draws: **the notation stays Apple's, and the spelling
becomes the host's choice.** Two spellings, one meaning:

| chord | Glyphs (Apple's) | Words |
| --- | --- | --- |
| Shift-Command-P | `⇧⌘P` | `Shift+Cmd+P` |
| Command-S | `⌘S` | `Cmd+S` |
| Option-Return | `⌥↩` | `Opt+Enter` |
| Control-Escape | `⌃⎋` | `Ctrl+Esc` |

`Words` is ASCII for every key the crate can name, so it resolves in any face on any platform — which
is the property that makes it a fix rather than a second guess.

## Model adopted

- **S1. `model::ChordGlyphs { Glyphs, Words }`**, engine-free, `Default = Glyphs`, with `ALL` and
  `name()` ("Glyphs", "Words") for a settings row, exactly as `PadStyle` is (`model::PadStyle` is the
  precedent for "whose names this host's inputs get, because the crate cannot detect it").
- **S2. `model::shortcut_label(code, modifiers, mac, glyphs)`** takes the spelling. `Words` on a mac
  spells the modifiers in Apple's order with words — `Ctrl`, `Opt`, `Shift`, `Cmd` — joined by `+`,
  and names the key with the crate's one key vocabulary, `key_name`, so a cap says `Esc`, `Enter`,
  `Up Arrow` in Words and in the prompts alike. On any other platform `Words` and `Glyphs` are the
  same string: Microsoft's notation is already words (`Ctrl+Shift+P`), so the setting cannot make a
  Windows or Linux label worse.
- **S3. The setting lives in `BindingSettings`**, beside `pad_style`, and for the same reason: it is
  a fact about the host's assets, not about its look, and it is what every chord-drawing system
  already reads. `BindingSettings::{chord, binding_label}` are the two methods that spell a chord or
  a binding as *this host* spells them.
- **S4. `BindingSettings::platform: Option<Platform>`** — `None` is this build's platform. It exists
  because a host may document or preview another platform's keymap, and because it is the only way
  this loop can demonstrate a mac-only defect on a Linux capture host. One field, `platform()` reads
  it, the drawing layer never calls `Platform::current()` itself.
- **S5. Every place the crate draws a chord reads those two methods**: `update_prompts`,
  `update_bindings`, the tooltip's keycap, a menu row's chord, a shortcut badge, the legend, the
  command palette's hint. A chord text also carries `ChordText(Shortcut)`, and one system
  (`update_chord_texts`, gated on `BindingSettings` changing) rewrites every one of them, so a setting
  changed at run time reaches a menu that was already built.
- **S6. `Shortcut::label()`, `label_on(platform)` and `Binding::label(PadStyle)` keep their meanings
  unchanged**: a value type, readable with no app, spelling the chord as the platform spells it. A
  host that shows a chord outside the crate's widgets calls `label_with(platform, glyphs)` when it
  needs the host's spelling. The arithmetic is one function (`model::shortcut_label`).

## Parameters

| value | choice | why |
| --- | --- | --- |
| default | `Glyphs` | today's look and Apple's notation, byte-identical; a host with a symbol-capable face changes nothing |
| mac modifier words | `Ctrl`, `Opt`, `Shift`, `Cmd` | Apple's order, the shortest unambiguous ASCII words |
| glue in Words | `+` | what every non-mac label already uses; one rule per platform family |
| key name in Words | `model::key_name` | the one key vocabulary (a keycap, a prompt and a hint never name a key two ways) |
| `Platform::Other` | `Words` == `Glyphs` | Microsoft's notation is already words |

## Validation targets

- `shortcut_label("KeyP", [ctrl, shift], true, Words)` is `"Ctrl+Shift+P"`; with meta,
  `"Shift+Cmd+P"`; with alt and `Enter`, `"Opt+Enter"`; with ctrl and `Escape`, `"Ctrl+Esc"`.
- `Glyphs` still gives `"⇧⌘P"` and `"⌘S"` (the notation is not lost), and on `Platform::Other` the two
  spellings are equal for every key the crate names.
- `Words` is **pure ASCII** for every key `KeyCode` has and every one of the 16 modifier
  combinations, on both platforms: a host with no symbol coverage can always read its own interface.
- A `binding` row, a `prompt`, a menu row, a tooltip's keycap, a badge and the legend all change with
  the setting, in the same frame, without being respawned.
- Nothing is written while idle (the rewriting system is gated on the resource moving).

## What great looks like

A tooltip on a mac reads "Undo" and a small cap saying `Cmd+Z` in the same face and the same size as
the words beside it — no box, no missing shape, nothing the player has to decode. Flip the theme or
the text size and the cap follows; flip `chord_glyphs` and every chord on screen changes at once,
because one function spells them all. On a host whose face does carry the symbols, `⇧⌘P` still reads
as Apple writes it, so a mac-native face loses nothing.

## Deviations

- **The default stays `Glyphs`**, although the crate's own default font (Bevy's 95-codepoint FiraMono
  subset) draws none of the thirteen. An assumed decision, and the conservative one: the default must
  not change the look of every existing host, `Glyphs` is Apple's notation and the issue's own first
  choice of shape for the fix, and only the host knows which face it shipped. The measured fact is
  recorded here and in `ChordGlyphs`'s rustdoc so the next host reads it before choosing.
- **The setting is `BindingSettings`, not `GooeySkin`** (the issue offered either). `GooeySkin` is
  what a project *saves* as a look and what the theme row edits; glyph coverage is a property of the
  font asset the host loaded, and `pad_style` already sits in `BindingSettings` for exactly that
  reason.
- **No automatic detection.** The issue's second option (degrade when the glyph is absent) needs the
  face's cmap; `fontique` can answer it but only inside Bevy's `text_system`, and `bevy_text` exposes
  no query to a game system. Parsing cmap by hand would put a font parser in the crate for one
  setting. The lever is the issue's preferred fix and it costs a host one line.
- **`BindingSettings::platform` is added**, beyond the issue's ask, for the demo (see S4).

## Pending index

- Nothing to index: all three source kinds were read directly (the corpus's ExposeHK stem and
  Lightbown stem, XAG 107 and Apple's HIG from the web, and the vendored Bevy 0.20 / fontique 0.11
  sources). `hs distill` answered normally this pass; the scribe's local GPU backend was yielded, so
  no new PDF was converted.
