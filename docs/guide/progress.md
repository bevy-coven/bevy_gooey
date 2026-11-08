# Progress and spinner

Work that takes a while gets honest feedback, picked by how long it takes. Nielsen's response-time limits are the rule: under about a second nothing is needed; from one to ten seconds the player should see the system is working; past ten they "will want to perform other tasks while waiting for the computer to finish, so they should be given feedback indicating when the computer expects to be done".

| the work takes | show |
|---|---|
| under about 1 s | nothing |
| 2 to 10 s, or an unknown amount | `spinner()` |
| 10 s or more | `progress(fraction)` with `ProgressText` |

There is no static variant of either: an indicator that does not move says the system has crashed.

```rust
fn saving(mut commands: Commands) {
    commands.spawn((
        progress(3.0 / 7.0),
        ProgressText {
            step: Some((3, 7, "Measuring meshes".into())),
            eta_seconds: Some(20.0),
        },
    ));
    commands.spawn(spinner());
}

fn advance(mut bars: Query<(&mut Progress, &mut ProgressText)>) {
    for (mut value, mut words) in &mut bars {
        value.0 += 0.001;
        words.eta_seconds = Some(12.0);
    }
}
```

![The forms example: a progress bar with "4 of 7: Packing atlases, about 7 s left" under it and a spinner beside "Syncing with the server"](https://raw.githubusercontent.com/dark-incorporated/bevy_gooey/main/docs/media/forms.gif)

- **`progress(fraction)`** is a track like a meter's (the same frame and fill, so one visual language) with a line of words under it. Change `Progress(f32)` and the fill moves in the same frame. A value outside `0.0..=1.0` is drawn at the nearest end (NaN as empty), logs one warning and puts `ProgressClamped` on the bar until the value is back in range.
- **`ProgressText { step, eta_seconds }`** is drawn "3 of 7: Measuring meshes, about 20 s left". Either part may be `None`; with neither the line is not drawn and takes no room. The time left is rounded the way an estimate deserves (`model::eta_words`): the second under ten seconds, five seconds to a minute, the minute to an hour, then "about 1 h 5 min left". The parts are joined by a comma, not the middle dot of the original request: Bevy's default font draws U+00B7 as a box, and the host supplies the font.
- **The bar keeps its width** as the words change: it is at least 28 body text sizes wide, and the words never decide its width; a longer line wraps inside it.
- **`spinner()`** is a ring in the skin's outline with one quarter in the accent, turning once every 0.9 s on `Time<Real>` (`model::spinner_angle`), so it keeps turning while the game is paused or a frame stalls. It is 1.5 body text sizes square. It writes one `UiTransform` a frame per spinner and nothing else.
- **Both need a label near them.** A spinner alone does not say what is happening; put a `label` beside it, as the example does.
