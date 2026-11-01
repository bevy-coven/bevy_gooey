# Player options the crate owns

`text_size`, `interface_size` and `pad_style` are the options-screen rows for the three settings this crate itself reads. Each is one line, needs no observer and no state of your own, and the whole screen answers the frame the player steps it:

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn display_page(mut commands: Commands) {
    commands.spawn((
        panel(),
        children![
            heading("Display"),
            text_size("Text size"),
            interface_size("Interface size"),
            pad_style("Controller"),
        ],
    ));
}
```

| row | the player picks | and it sets | the steps |
|---|---|---|---|
| `text_size(label)` | how large every word is | `GooeySkin::text_scale` | 100%, 125%, 150%, 200% (`TEXT_SIZE_STEPS`) |
| `interface_size(label)` | how large the whole interface is, on top of the window's share | `UiScaleSettings::scale` | 75%, 100%, 125%, 150% (`INTERFACE_SIZE_STEPS`) |
| `pad_style(label)` | whose button names a pad shows | `BindingSettings::pad_style` | Xbox, PlayStation, Switch (`PAD_STYLES`) |

Text size is the one the Xbox Accessibility Guideline 101 and WCAG 2.2 (1.4.4) ask for: text that resizes to 200% "without loss of content or functionality", and the rule that it reaches everything the player reads, the dialog, the toast and the legend as well as the menu. `GooeySkin::text_scale` is read by every part, so the row only has to write it. Interface size enlarges the panels and the targets too, and goes below 100% for the player who wants the interface to cover less of the game; EVE Online's UI scaling tops out at 150%, as this one does.

## What a row is

A row is a [`choice`](controls.md) (Bevy's `Button`, one tab stop, Left and Right and the D-pad step it, a click cycles it) with one more component, so it takes `Tooltip`, `InteractionDisabled` and your own `ValueChange<usize>` observer like any other. The setting is not kept twice: the resource above is the one place it lives, and the row is a view and an editor of it.

- **The player's step writes one field.** Stepping Text size to 150% sets `text_scale` and nothing else, so a skin of your own, the scale's `reference_short_side` and the binding prompts survive. A step is one write and one `ValueChange<usize>` (the index into the step table), and a press at the end of the row changes nothing and says nothing.
- **The setting moves the row, with no event.** The row shows the step nearest the resource the frame it is spawned and whenever the resource changes, so a loaded save shows on it, Shift+T in your own code shows on it, and another row for the same setting follows. Nothing is sent for a write the game made, so your observer hears only the player. A value between two steps (a game that set 1.1) shows the nearest step, "100%", and is left alone until the player steps.
- **Saving is the resources.** Under the `serde` feature `GooeySkin`, `UiScaleSettings` and `BindingSettings` are data: save them when the player leaves the screen (or on each `ValueChange<usize>` you observe), load them before the first frame, and the rows show them. Save `text_scale` and not the whole skin if the skin is yours and not the player's.
- **The label is yours.** Every widget takes its string from the host; the options are percentages and the makers' names, so there is nothing else to localise.
- **Plugins.** The rows' systems are `GooeyStylePlugin`'s. `interface_size` also needs `GooeyScalePlugin` for the setting itself: without it the row steps and nothing is drawn differently, and nothing panics.

Other steps, or a setting of your own, are a `choice` and eight lines: the steps in a table, an observer that indexes it, and a write when you load. The three rows exist because the crate owns these settings and so can keep them true without you.

## Every step holds

WCAG asks that a resize offering steps between 100% and 200% have "no loss of content or functionality at these incremental steps", so the steps are sizes the crate lays every widget out at: `tests/large_text.rs` builds a panel of every widget (a heading, a body line, buttons, a meter, a toggle, a choice, a slider, rebinding rows, a text input, tabs and a list) and the dialog, a toast and the legend at each, and finds nothing outside its box; and every 5% from 100% to 200% it finds every control's text on one line. A game's own screen must still fit its window at the size it offers: a panel that is 520 px tall at 100% is 780 px at 150% in a 720p window, and the interface does not scroll for you (put the long part in a [`list`](lists.md), and keep the legend's corner clear: [prompts](prompts.md)).

`cargo run --example options` is the whole screen, three rows and the legend, with the capture knobs the gallery has (`GOOEY_KEYS=right,right,right` is three steps on Text size).

![The options screen: the ring on Text size steps 100%, 125%, 150% and 200% and the panel and the legend grow with it, back to 100%; Interface size steps to 125% and 150% and back; a pad button turns the legend to the pad's, and Controller steps to PlayStation (the legend reads Cross Select, Circle Back), to Switch and back to Xbox](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/options.gif)

*The three rows in `examples/options`: the whole screen answers each step the frame it is made.*
