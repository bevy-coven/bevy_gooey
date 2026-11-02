# Named tones: a skin carries a host's own semantic colours

Researched 2026-11-02 (iteration 41). Issue #33 (G-29, `grimoire-ui`, `consumer:bevy_grimoire`). A tool has more semantic colours than the skin's fixed fields: a suggestion ink, a "dim but legible" key, a map grid line, a problem banner's fill, a veil, a recessed ground. Today a host either merges them into `text_muted` / `accent` (and changes the look) or keeps a second palette beside the skin (and "the UI's colours are the skin" is false). A game has the same need: a rarity colour, a faction colour, a damage number.

## Sources

1. W3C, *Understanding Success Criterion 1.4.11: Non-text Contrast* (WCAG 2.2), https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html (craft: which pairs must be checked, and why a graphic you add is held to the same minimum as the ones the toolkit ships).
2. Visual Studio Code, *Themes*, https://code.visualstudio.com/docs/configure/themes (shipping: a shipped editor whose theme is a named-colour table the user extends) and the Design Tokens Community Group, *Design Tokens Format Module 2025.10*, https://www.designtokens.org/tr/drafts/format/ (the industry shape of a skin file: named tokens grouped under a semantic name, each with a value).
3. This crate's own `GooeySkin::problems` (technical: the pair list and `contrast_ratio`), the scikit-learn API paper (`W1728842521`, home-still: one way to do a thing, defaults that work).

## Craft principle

WCAG 1.4.11: "The intent of this success criterion is to ensure that user interface components (i.e., controls) and meaningful graphics are distinguishable by people with moderately low vision." A colour a host adds to its UI is a "meaningful graphic" the moment it carries meaning; the crate cannot check what it cannot see, so the host declares the pairs it relies on and the skin holds them to the same arithmetic as its own.

## How shipping games do it

VS Code: "You can customize your active color theme with the `workbench.colorCustomizations` ... user settings. To set the colors of VS Code UI elements such as list & trees ..., diff editor, Activity Bar, notifications, scroll bar, split view, buttons, and more, use `workbench.colorCustomizations`": a theme is a flat table of names to colours, so a theme file names every colour the UI draws, and an extension's own colour is one more row. The Design Tokens format: a token is a name and a `$value`, grouped (`"semantic": { "success": {..}, "warning": {..} }`). A skin file here is serde data already; a `tones` table is the same shape.

## Model adopted

- **N1. `GooeySkin::tones: BTreeMap<String, Color>`** (serde `default`, empty in every preset, reflected). A `BTreeMap` so a saved skin file lists them in a stable order.
- **N2. `GooeySkin::tone(&self, name) -> Option<Color>`.** Case-sensitive, never translated; an unknown name is `None`, never a default colour (a wrong colour drawn silently is worse than a missing fill).
- **N3. `Role::Tone(&'static str)`**: a node drawn with the tone's colour as its fill, restyled by `apply_skin` in the frame the skin changes. A name the skin does not hold draws no fill (and the node keeps what it had). `Role` stays `Copy`.
- **N4. `GooeySkin::tone_problems(&[ToneCheck])`** where a host declares `(label, ink, ground, needed)`: `ink` and `ground` name a tone or one of the skin's own colours (`text`, `surface`, `accent`, ...); one `SkinProblem::LowContrast { pair: label, .. }` for each pair under its ratio, one `SkinProblem::UnknownTone { name }` for a name that is neither. The label is the host's (a `&'static str`) because `SkinProblem::LowContrast::pair` is, and a host's own message reads better named by it ("suggest on field") than by two keys.
- **N5. Tones are the host's, so a theme step leaves them.** `with_colours_of` (the theme row) copies the preset's built-in colours and leaves `tones` as the host set them; a preset has none to give.
- **N6. Not checked by `problems()`.** The issue: "A tone is not checked on its own". `high_contrast_problems()` does not see them either; a host passes `ENHANCED_CONTRAST` as `needed` for its own high-contrast pairs.

## Parameters

No new constant. A tone's alpha is ignored by `tone_problems` like every other pair (`rgb()`); a translucent tone is checked over nothing, so a host that needs it checked declares the opaque ground it sits on as the ground.

## Validation targets

A skin file with `tones: {"suggest": Srgba(..)}` reads back through `tone("suggest")` and `tone("absent")` is `None`; `tone_problems` reports `LowContrast` for a pair under its ratio and nothing for a pair over it; a node with `Role::Tone("suggest")` is restyled in the frame the skin resource changes; every preset has no tones and `problems()` is unchanged; frame time unchanged (one extra match arm).

## Deviations

- **The issue's `tone_problems(&[(ink, ground, needed)])` takes a label too.** Design default: `SkinProblem::LowContrast::pair` is a `&'static str` (every existing assertion matches it as a literal), so a run-time "ink on ground" string cannot be made without leaking; the host names the pair. Recorded as an assumed decision on the issue.
- **`SkinProblem::UnknownTone`** is new: a declared pair that names a tone the skin lacks is a bug in the host's check, and silently passing it would make the check vacuous.

## What great looks like

A host's colour table is one `tones` map in the skin file, the editor's 14 colours move out of its own module, and a test that says `skin.tone_problems(&[..]).is_empty()` fails loudly when someone dims a ground.

## Pending index

The Design Tokens Format Module and VS Code's theme page are not in home-still (web sources, quoted above).
