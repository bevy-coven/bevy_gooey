# bevy_gooey

Game interfaces made easy for Bevy 0.20: a skin, ready-made game widgets and keyboard and gamepad navigation, on Bevy's own headless widgets.

![The gallery example: a panel with a heading, three buttons and a health meter; Options has the focus ring and its tooltip, "Sound, controls and display", is open beside it](docs/media/gallery.png)

## What you get

- **A skin**, `GooeySkin`: every colour and size as plain data, two presets (`dark`, `parchment`), a player text-scale option, and `problems()` to check it against the WCAG 2.2 contrast and target-size rules. Change it and everything on screen restyles that frame.
- **Widgets**, each a free function returning a bundle: `screen`, `panel`, `menu`, `heading`, `label`, `button`, `meter` (a HUD bar), `Tooltip` and the modal `dialog`.
- **Navigation**: arrows and the D-pad move focus to the nearest widget, Enter, Space and the pad's South button activate, Tab cycles.
- **Feedback**: a `UiFeedback` message for every focus move and activation, for your sound.

## Install

Until the first release on crates.io, take it from git:

```toml
[dependencies]
bevy_gooey = { git = "https://github.com/Ladvien/bevy_gooey" }
```

Features: `serde` (save and load a `GooeySkin` as data).

## Quick start

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GooeyPlugins))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            panel(),
            children![
                heading("Paused"),
                (
                    menu(),
                    children![
                        (button("Resume"), observe(|_: On<Activate>| info!("resume"))),
                        button("Quit"),
                    ]
                ),
            ]
        )],
    ));
}
```

Every widget composes inside `children![]`, and a button acts through an `Activate` observer. `cargo run --example gallery` shows them all.

## Navigation

- Arrow keys and the D-pad move focus to the nearest button in that direction (`navigate_by_input`); a pad moves focus, it never steers a cursor.
- Enter and Space activate the focused button (Bevy's own button handling); the pad's South button does too (`gamepad_select`).
- Tab and Shift+Tab cycle through a `screen`'s buttons.
- An open `dialog` holds focus: focus moves to its first button when it opens, Tab, the arrows and the D-pad cannot leave it, and closing it puts focus back on the button that opened it. Put `AutoFocus` on the dialog's safest choice ("Stay", "Cancel") to start there instead.
- A `Tooltip` opens when its widget has focus from a key or the pad, as it does under the pointer. It opens beside the widget (to the right in a column, below in a row, flipped at the window's edge), one step outside its panel, so it never covers the next button.
- Put Bevy's `AutoFocus` on the button a screen should start on.

## Skins

`GooeySkin::dark()` is the default; `GooeySkin::parchment()` is the light one. Edit the resource live (an options screen, or an editor) and `apply_skin` restyles every widget that frame. With the `serde` feature a skin is RON or any serde format, and a partial file fills the rest from the default, so a project saves only what it changed:

```ron
(accent: Srgba((red: 0.2, green: 0.6, blue: 0.9, alpha: 1.0)), text_scale: 1.25)
```

`GooeySkin::problems()` lists what a skin breaks, and the plugin warns once per problem when the skin changes (it never refuses one):

- text under 4.5:1 contrast against the surface, the raised surface, a hovered button, or the accent (WCAG 2.2, 1.4.3);
- a focus ring under 3:1 against the raised surface (WCAG 2.2, 1.4.11);
- `min_target` under 24 px (WCAG 2.2, 2.5.8; the default is 44 px);
- a size that is not finite, or not positive where it must be.

Fonts are yours: set `GooeyFonts { body, heading }`. The default draws Bevy's built-in font when Bevy's `default_font` feature is on.

## Feedback

Read `UiFeedback` (`entity`, `kind: Focus | Activate`) and play a sound. The crate plays none.

## What this crate does not own

World-space speech balloons and dialogue belong to `bevy_speech_bubbles`; the game's states and pausing, to the game. Audio is the host's: it hears `UiFeedback`. Localization is the host's: every widget takes the string to show. Fonts are the host's: it fills `GooeyFonts`.

## License

MIT OR Apache-2.0.
