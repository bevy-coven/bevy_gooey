# `docs/guide/` — the detail behind the README

The README is the page most people read, so it carries the pitch, the quick start and one bullet per
thing you can do. Everything that used to sit in it and needs more than a sentence lives here, one page
per subject. The README links each bullet to its page.

- [The widgets](widgets.md) — every widget, a meter, a tooltip, a dialog, and how a HUD lets the pointer
  through to the game under it.
- [Navigation](navigation.md) — what moves focus, what shows the ring, and what an open dialog holds.
- [Settings controls](controls.md) — `toggle`, `choice` and `slider`: one bundle each, Bevy's own widget
  at the root, and `ValueChange` once per change.
- [Rebinding controls](bindings.md) — `binding(label, value)`: a row that waits for the player's next key,
  mouse button or pad button, what it hears and holds back while it waits, and `BindingSettings`.
- [Prompts](prompts.md) — `prompts()` and `prompt(label, Prompt::BACK)`: the Select and Back hints in the
  bottom-left corner, the keyboard's or the pad's by `InputDevice`, and what they do not draw.
- [Tabs](tabs.md) — `tabs(labels, selected, children![tab_page(..)])`: pages of widgets under a strip, LB
  and RB, and focus that never rests on a page that is not shown.
- [Lists](lists.md) — `list(rows, children![..])`: the row count, the rail, the follow, PageUp and
  PageDown, and what a list cannot do yet.
- [Toasts](toasts.md) — one spawn, the reading-time maths, `ToastSettings`, and the cap on the stack.
- [Skins](skins.md) — `GooeySkin`, its fields, the two presets, and every rule `problems()` checks.
- [Screen size](scale.md) — `GooeyScalePlugin`, `UiScaleSettings`, and whole-device-pixel lines.
- [The pointer](pointer.md) — which parts of the interface take the pointer and which let it through.
- [Feedback](feedback.md) — `UiFeedback`, the message a game turns into sound.
- [What a host touches](host-api.md) — the settings resources, the components, `UiFeedback`, every
  system and the engine-free maths, in one table.
- [What this crate does not own](scope.md) — the boundary with the game, `bevy_speech_bubbles` and the
  host's audio, strings and fonts.

The design rules and the published sources behind them are in
[`docs/research/`](../research/gooey.md), which is a reading list, not a manual.
