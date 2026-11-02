# Testing a gooey UI

A player finds "Options" by its words, clicks it where it is drawn and sees where focus went. `bevy_gooey::testing` (cargo feature `testing`, off by default so a game ships none of it) does the same in a headless `App`, through the real input path (`PointerInput`, `MouseButtonInput`, `KeyboardInput`), so a test cannot pass while a button is covered, hidden, unfocusable or mislabelled: the things a test that triggers an entity directly never notices.

```toml
[dev-dependencies]
bevy_gooey = { version = "0.26", features = ["testing"] }
```

```rust
use bevy::prelude::*;
use bevy_gooey::testing::{click, find_by_label, focused_label, press, type_text};

#[test]
fn options_takes_focus_and_the_name_field_takes_text() {
    let mut app = my_headless_app(); // DefaultPlugins without winit and a renderer, plus GooeyPlugins
    spawn_main_menu(&mut app);
    app.update(); // laid out

    let options = find_by_label(&mut app, "Options").unwrap();
    click(&mut app, options);
    assert_eq!(focused_label(&app).as_deref(), Some("Options"));

    press(&mut app, KeyCode::Enter); // activates the focused button
    click(&mut app, find_by_label(&mut app, "Name").unwrap());
    type_text(&mut app, "Ada");
}
```

`my_headless_app` is the one every test in this crate builds: a `WindowPlugin` with a fixed resolution, a `RenderPlugin` with `backends: None`, `WinitPlugin` disabled and `GooeyPlugins` (see `tests/testing.rs`).

## A UI drawn into an image needs no window

A tool or a game that shows its interface as a texture (an in-world screen, an editor viewport) has a camera whose `RenderTarget` is an `Image`, and a test app may have no `PrimaryWindow` at all (`WindowPlugin { primary_window: None, .. }`). `click` goes to the render target of the camera that draws the entity, as Bevy's picking matches pointers to cameras: the entity's `ComputedUiTargetCamera` (a `UiTargetCamera` on its root, or the default UI camera), then that camera's `RenderTarget`, a window's or an image's, divided by that target's scale factor (`ImageRenderTarget::scale_factor`, which should be 1.0). Two cameras drawing different images each get the clicks of what they draw. A camera that renders to an image is a default UI camera only if it carries `IsDefaultUiCamera` (Bevy picks cameras that render to the primary window): mark it, or name it with `UiTargetCamera` on the roots.

```rust
# use bevy::prelude::*;
# use bevy::camera::{ImageRenderTarget, RenderTarget};
# use bevy::ui::IsDefaultUiCamera;
# use bevy_gooey::*;
# fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
let image = images.add(Image::new_target_texture(
    800,
    600,
    bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb,
    None,
));
commands.spawn((Camera2d, RenderTarget::Image(ImageRenderTarget::from(image)), IsDefaultUiCamera));
# }
```

Bevy gives a clicked widget focus only while a primary window exists, so with none `click` also gives the widget under the pointer focus itself (the same bubbling `AcquireFocus`, the focus ring hidden as a click hides it): `focused(&app)` is the button. `press` and `type_text` need no window either for what the crate reads (`navigate_by_input`, the cancel and tab systems read `ButtonInput`); Bevy's own key handling of a focused widget (Enter on a button, a character in a field) is dispatched only while a primary window exists, so a test of those wants one. A widget drawn by no camera, or by a camera that renders to a window the app does not have, is a panic that says so.

## The helpers

- `find_by_label(&mut app, "Options") -> Result<Entity, FindError>`: the one visible widget whose words are that label. The words are a `Text` node; the widget is its nearest ancestor that takes focus (a button, a toggle, a slider), or the node itself when nothing above it does (a heading). A hidden page, a folded section and an option a `choice` is not showing are not visible, so a "Back" on another tab is not a match. **Two widgets with one label are an error that names both** (`FindError::Ambiguous`, each entity with its `Name` or `Role`) and none is `FindError::NotFound`: a silent first match would pass for the wrong widget.
- `click(&mut app, entity)`: moves the pointer to the centre of the entity's laid-out box and presses and releases the primary button, at any window or image scale and `UiScale`. A node covered by another that takes the pointer (a dialog's scrim) is not clicked, as for a player. It panics if the entity has not been laid out: update the app after spawning the interface.
- `press(&mut app, KeyCode::Enter)`: a key pressed and let go as a keyboard does it. `type_text(&mut app, "Ada")`: one press and release per character, each carrying the text it makes, so a text field takes it.
- `focused(&app) -> Option<Entity>` is `InputFocus`; `focused_label(&app) -> Option<String>` is its words (`label_of(&app, entity)` for any entity: its own `Text`, else the first below it, and a text field reads as its row).

Each action runs two `update`s after it, so the input is read, its observers run and the `Style` pass lands. A test that needs more calls `app.update()`.

## What it does not do

- **Drive a real window.** The app is headless; nothing is drawn. Pixels are checked with the gallery's capture mode, not here.
- **Find a widget by something other than its words.** An icon button has no `Text`: give it a `Name` and query for it, or find its tooltip's text.
- **Hold a key while another is pressed.** `press` is a tap; for a chord (`Ctrl+S`) write `KeyboardInput` messages as `tests/badges.rs` does.
