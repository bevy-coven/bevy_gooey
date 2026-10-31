# bevy_gooey

Game interfaces made easy for Bevy 0.20: a skin, ready-made game widgets and keyboard and gamepad navigation, on Bevy's own headless widgets.

![The gallery example: a panel with a heading, three buttons, a health meter, a Subtitles switch, a Quality choice ("Medium" between two chevrons) and a Volume slider; Options has the focus ring and its tooltip, "Sound, controls and display", is open beside it; two toasts, "Game saved" and "Inventory full", stand at the bottom right](docs/media/gallery.png)

## What you get

- **A skin**, `GooeySkin`: every colour and size as plain data, two presets (`dark`, `parchment`), a player text-scale option, and `problems()` to check it against the WCAG 2.2 contrast and target-size rules. Change it and everything on screen restyles that frame.
- **Widgets**, each a free function returning a bundle: `screen`, `panel`, `menu`, `heading`, `label`, `button`, `meter` (a HUD bar), `toggle`, `choice` and `slider` (an options screen's switch, stepper and scrubber), `list` (rows that scroll, following focus), `toast` and `toast_alert` (a line of news at the edge of the screen), `Tooltip` and the modal `dialog`.
- **Navigation**: arrows and the D-pad move focus to the nearest widget, Enter, Space and the pad's South button activate, Tab cycles.
- **Scale**: the whole interface keeps its share of the screen at every resolution, so text legible at 720p is legible on a 1440p monitor and a 4K television.
- **Feedback**: a `UiFeedback` message for every focus move and activation, for your sound.

## Install

From crates.io:

```toml
[dependencies]
bevy_gooey = "0.3"
```

Or from git, for changes not released yet:

```toml
[dependencies]
bevy_gooey = { git = "https://github.com/Ladvien/bevy_gooey" }
```

Features: `serde` (save and load a `GooeySkin` and `UiScaleSettings` as data).

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

- Arrow keys and the D-pad move focus to the nearest button in that direction (`navigate_by_input`); a pad moves focus, it never steers a cursor. The first key or D-pad press shows the focus ring (Bevy's `InputFocusVisible`) and a mouse click hides it again, so a mouse player never sees a stray ring and a pad player always sees theirs. A pad-first game that wants the ring from the first frame inserts `InputFocusVisible(true)`.
- Enter and Space activate the focused button (Bevy's own button handling), flip the focused `toggle` and cycle the focused `choice`; the pad's South button does all three (`gamepad_select`).
- Tab and Shift+Tab cycle through a `screen`'s buttons.
- An open `dialog` holds focus: focus moves to its first button when it opens, Tab, the arrows and the D-pad cannot leave it, and closing it puts focus back on the button that opened it. Put `AutoFocus` on the dialog's safest choice ("Stay", "Cancel") to start there instead.
- A `Tooltip` opens when its widget has focus from a key or the pad, as it does under the pointer. It opens beside the widget (to the right in a column, below in a row, flipped at the window's edge), one step outside its panel, so it never covers the next button.
- A `Tooltip` is a view of its component: `commands.entity(button).remove::<Tooltip>()` takes the hint off (a "Locked: reach level 5" line once the player unlocks it), and inserting a new `Tooltip`, or editing `Tooltip.0`, changes the line the open tooltip shows.
- Put Bevy's `AutoFocus` on the button a screen should start on. With nothing focused (a click on empty space, a button despawned under the player), the first arrow or D-pad press lands there, or on the screen's first button in Tab order, and the next press moves on; with a `dialog` open it lands inside the dialog.
- A `list` scrolls to follow the focus a key, the D-pad or Tab gives its rows (with the next row kept in sight), PageUp and PageDown (the pad's triggers) move a viewport at a time, and Up and Down stay among a list's rows until they run out; see Lists.

## Settings controls

`toggle`, `choice` and `slider` are the controls an options screen is made of. Each is one bundle whose root is Bevy's own headless widget (`Checkbox`, `Button`, `Slider`), so the widget you spawn is the entity that carries its value, takes `observe`, `Tooltip` and `InteractionDisabled`, and says what changed through `ValueChange`:

```rust
fn options(mut commands: Commands) {
    commands.spawn((
        panel(),
        children![
            (toggle("Subtitles", true), observe(|c: On<ValueChange<bool>>| info!("subtitles {}", c.value))),
            (choice("Quality", ["Low", "Medium", "High"], 1), observe(|c: On<ValueChange<usize>>| info!("quality {}", c.value))),
            (slider("Music", 0.7, 0.0..=1.0), observe(|c: On<ValueChange<f32>>| info!("music {}", c.value))),
        ],
    ));
}
```

- A **toggle** is a row: the label on the left, a framed switch on the right whose knob slides to the end and fills with the accent when on. Its state is the knob's place first and its colour second, so it reads for a colour-blind player. It flips by itself (`Checked` on the entity); a click anywhere on the row, Enter, Space and South flip it, and `ValueChange<bool>` is the moment to apply the setting.
- A **choice** is a row: the label on the left, the selected option on the right between two chevrons, for a setting with a few named values (Low, Medium, High). It is one tab stop. With it focused, Left and Right (keys or the D-pad, a held direction repeating as a slider's does) step to the previous and next option and stop at the ends (the chevron that cannot step is dimmed, and a press at an end changes nothing and says nothing), Home and End jump to the ends, and Up and Down still move focus on, so a pad steps a column of settings with no mode to enter. A click on a chevron steps once, on release; Enter, Space, South and a click anywhere else on the row cycle to the next option and wrap round, so one repeated press reaches every option. Every option takes the widest one's room, so the chevrons never move under a thumb. The selected index is `ChoiceValue` on the entity (write one to move it from code; no `ValueChange` comes back for that), and `ValueChange<usize>` fires once per change. Observe that, not `Activate`: only a cycle is an `Activate`.
- A **slider** is a row: the label and the value (a percentage for a range of 0 to 1, the number otherwise) over a rail with a thumb. The whole row is the target: a click puts the thumb under the pointer and a drag goes on from there. With it focused, Left and Right step by a twentieth of the range (insert a `SliderStep` to change it), Home and End jump to the ends, and Up and Down still move focus on. The D-pad does the same, and a held direction repeats after 0.4 s at 12 steps a second (`PAD_REPEAT_DELAY`, `PAD_REPEAT_RATE`), so a pad player never presses A to select the slider nor holds a button to keep it. `ValueChange<f32>` fires as the value moves (`is_final` is false until the drag ends); read the value from `SliderValue`, or write one to move the slider.
- All three are targets at least `min_target` high (a choice's chevrons are `min_target` wide too), show the same focus ring as a button, and grow with `text_scale`.

## Lists

`list(rows, children![..])` is a column of rows that scrolls when they do not all fit: save slots, an inventory, a key-binding screen. The rows are any widgets, in `children![]`:

```rust
fn load_game(mut commands: Commands) {
    commands.spawn((
        panel(),
        children![
            heading("Load game"),
            list(
                4.5,
                children![
                    (button("Meadow, 12:40"), observe(|_: On<Activate>| info!("load meadow"))),
                    button("Harbour, 09:15"),
                    button("Mill, 03:02"),
                    button("Cellar, 01:48"),
                    button("Orchard, 00:20"),
                    button("Autosave"),
                ]
            ),
        ],
    ));
}
```

- `rows` is how many rows tall it stands, a row being a button's height and the `gap` after it: `4.5` shows four rows and half of the fifth. A list that ends exactly on a row looks finished; one that cuts the next row does not (NN/g: "Include signifiers (such as cut-off text) to tell people that there is content below the fold"). The count follows the player's `text_scale`, and `ListView::rows` changes it live.
- The wheel and a trackpad scroll it, and so do a drag on the thumb and a click on the rail (Bevy's own `ScrollArea` and `Scrollbar`). The rail, `2 × border` wide, shows only while the rows do not fit; its lane is always reserved, so rows never change width when others come and go.
- **It follows focus.** When a key, the D-pad or Tab moves focus onto a row, the list scrolls by the least that leaves a row and a half beyond it: the next row whole and the one after it cut, so the list visibly goes on at every step, and it ends flush with the last row. A row taller than the list scrolls to its top, and a jump of more than a viewport (PageDown, a focus set from code) centres the row. A focus a pointer press set is not followed: a press that scrolled the content under the pointer would send its release to another row and lose the click.
- PageUp and PageDown, and the pad's left and right triggers, move focus a viewport up or down the list in one press. Up and Down stay among a list's rows until they run out (rows the list has clipped lie further off than the button under it, so geometry alone would step out past them), and a row the list has clipped away is never the neighbour of a widget outside it.
- To scroll from code, set the `ScrollPosition` on the `ListView` entity (the first child of the list's root). A list builds every row it is given, so it is for tens of rows, not thousands; a list at its end does not scroll the panel behind it; a wheel notch is Bevy's 100 px.
- A list is not announced to screen readers yet: Bevy's `ScrollArea` and `Scrollbar` carry no accessibility node.

## Toasts

A toast is one spawn from any system, with no layout and nothing to set up:

```rust
fn save(mut commands: Commands) {
    commands.spawn(toast("Game saved"));
}

fn pickup_failed(mut commands: Commands) {
    commands.spawn(toast_alert("Inventory full"));
}
```

- The first toast creates the one `ToastStack` (a full-window, pointer-transparent column above a `dialog`'s scrim and below a tooltip) and every toast becomes its child in the frame it is spawned. Despawn the entity to take a toast back sooner.
- A toast stands as long as its message takes to read: `(base_seconds + characters ÷ chars_per_second) × duration_scale`, 2.5 s for "Game saved" at the defaults, then fades out and despawns. It runs on real time, so a paused game's menu still ages it. It fades in as the room it holds opens and fades out as that room closes, so the toasts round it glide to make way and close up and none of them jumps.
- Offer `ToastSettings::duration_scale` in your settings screen: Xbox Accessibility Guideline 116 asks that a player can stretch the time notifications stay up to at least ten times the default (any value above zero works). `fade_seconds: 0` removes the fade and the opening of room for a player who wants no motion. `max_visible` (3) sends the oldest into its fade when a fourth arrives, and `anchor` picks one of six places (bottom centre by default); the newest stands nearest the edge.
- A toast takes no input and no focus: every node of it is `Pickable::IGNORE`, it is no tab stop, and a press, a key or a D-pad direction goes to what is under it. The mark down its left edge is the accent for `toast` and `danger` for `toast_alert`; the words carry the kind too. `GooeySkin::problems()` checks both marks against the toast's surface at 3:1.
- A toast is not announced to screen readers yet: it is visual only.

## Over a game world

A `screen` is layout, not a surface: its empty part is `Pickable::IGNORE`, so a HUD on it lets the pointer through to the sprites, meshes and UI roots under it (a world sprite opts in to picking with `Pickable::default()`), while a `panel`, `button`, `meter` or `dialog` on it still takes the pointer, so a click on one never reaches the world. A click on the empty part still blurs the interface. A menu screen that must swallow the pointer: `commands.spawn(screen()).insert(Pickable::default())`.

## Skins

`GooeySkin::dark()` is the default; `GooeySkin::parchment()` is the light one. Edit the resource live (an options screen, or an editor) and `apply_skin` restyles every widget that frame. With the `serde` feature a skin is RON or any serde format, and a partial file fills the rest from the default, so a project saves only what it changed:

```ron
(accent: Srgba((red: 0.2, green: 0.6, blue: 0.9, alpha: 1.0)), text_scale: 1.25)
```

`GooeySkin::problems()` lists what a skin breaks, and the plugin warns once per problem when the skin changes (it never refuses one):

- text under 4.5:1 contrast against the surface, the raised surface, a hovered button, or the accent (WCAG 2.2, 1.4.3);
- a focus ring under 3:1 against the raised surface (WCAG 2.2, 1.4.11);
- a meter's fill (the accent) under 3:1 against its track, or its frame (`outline`) under 3:1 against the panel, or an off switch's knob (`text_muted`) under 3:1 against its track, or a toast's mark (`accent`, `danger`) under 3:1 against the toast's surface (WCAG 2.2, 1.4.11);
- `min_target` under 24 px (WCAG 2.2, 2.5.8; the default is 44 px);
- a meter too thin to show a fill inside its frame and gap (`meter_height × text_scale` at most 4 × `border`);
- a size that is not finite, or not positive where it must be.

Fonts are yours: set `GooeyFonts { body, heading }`. The default draws Bevy's built-in font when Bevy's `default_font` feature is on.

## Screen size

`GooeyScalePlugin` (in `GooeyPlugins`) sets Bevy's `UiScale` from the window's shorter side, so every widget, gap, border and line of text keeps its share of the screen: the skin's sizes are what a 1280x720 window shows, a 1920x1080 window draws them at 1.5x and a 2560x1440 one at 2x. The default text then stays above the Xbox Accessibility Guidelines' console minimum (26 px at 1080p, 52 px at 4K) on any screen. `UiScale` multiplies every `Val::Px`, so your own UI nodes scale with the crate's.

- `UiScaleSettings { reference_short_side, scale }`: the window size at which the skin's sizes are drawn as they are (720; a game designed at 1080p sets 1080), and the player's interface size on top (1.0). `GooeySkin::text_scale` enlarges text alone; `scale` enlarges everything.
- A minimised window keeps the last scale.
- Every line (a panel's or a button's border, the focus ring, a meter's frame and the gap round its fill) is drawn a whole number of device pixels wide, rounded down as CSS rounds a border, so it is one weight on all four sides at any scale: a 3 px border is 4 px at 1920x1080 (1.5x) and 3 px at 1280x800 (1.11x), and the meter's fill sits centred in its slot. `snap_line_width` is the rule.
- The plugin owns `UiScale`. To drive it yourself, leave the plugin out: `GooeyPlugins.build().disable::<GooeyScalePlugin>()`.

## Feedback

Read `UiFeedback` (`entity`, `kind: Focus | Activate`) and play a sound: a button, a toggle, a choice and a slider each focus, and a button's activation, a toggle's flip and a choice's change of option activate. The crate plays none.

## What this crate does not own

World-space speech balloons and dialogue belong to `bevy_speech_bubbles`; the game's states and pausing, to the game. Audio is the host's: it hears `UiFeedback`. Localization is the host's: every widget takes the string to show. Fonts are the host's: it fills `GooeyFonts`.

## License

MIT OR Apache-2.0.
