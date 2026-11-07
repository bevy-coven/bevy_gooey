# Settings controls

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

## Control sizes

`ControlSize::Small`, `Regular` and `Large` on a control's root make it 24, 28 or 36 logical px tall: a dense inspector's rows, a desktop control, a dialog's main action. For a text, number, vec or colour field the framed field is that tall, and its row adds a little air round it. Put it on `button`, `icon_button`, `toggle`, `choice`, `select`, `text_input`, `number_field`, `vec_field` or `color_field`:

```rust,ignore
commands.spawn((toggle("Snap", true), ControlSize::Small));
commands.spawn((button("Create project"), ControlSize::Large));
```

A size is a least height: a control is never under the skin's `target()`. So under a comfortable skin (a 44 px target, the default) every size is 44 px, and only a compact skin (`GooeySkin::desk()`, `tool()`: a 24 px target) shows the three. The text keeps the skin's size. A control with no `ControlSize` is as it was, and taking one off puts it back. There is no 16 or 20 px size: WCAG 2.2 SC 2.5.8 asks for 24.

