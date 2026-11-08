# Inspector

An editor shows any component or resource as editable rows without a panel for each type. With the `reflect` feature, `inspector(InspectTarget)` builds the rows from the type's own reflection.

```toml
bevy_gooey = { version = "0.23", features = ["reflect"] }
```

```rust
use std::ops::RangeInclusive;

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
struct Turret {
    /// Tracks targets.
    online: bool,
    mode: Mode,
    /// Shots a second.
    #[reflect(@RangeInclusive::<f32>::new(0.0, 20.0))]
    rate: f32,
    aim: Vec3,
    tint: Color,
    barrel: Barrel,
}

fn setup(app: &mut App) {
    app.register_type::<Turret>(); // and Mode, Barrel
}

fn editor(mut commands: Commands, turret: Entity) {
    commands.spawn((
        inspector(InspectTarget::component::<Turret>(turret)),
        observe(|change: On<InspectorChange>| info!("edited {}", change.path)),
    ));
}
```

![The inspector example: a Turret panel with an Online switch, a Mode choice, a Rate number field, an Aim vector, a Tint colour field and a Barrel section; edits made with the keys show in a readout beside it as the game sees them, and a field's doc comment is its tooltip](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/inspector.gif)

- **The target** is `InspectTarget::component::<T>(entity)` or `InspectTarget::resource::<T>()`. The type is registered for reflection with `#[reflect(Component)]` (or `#[reflect(Resource)]`); one that is not shows a line saying so instead of a panel.
- **One widget per field**: `bool` is a `toggle`; an enum whose variants all carry nothing is a `choice`; `f32`, `f64` and the integers are a `number_field` (an integer stays an integer: a typed 5.7 is 6); `Vec2`, `Vec3` and `Vec4` a `vec_field`; `Srgba` and `Color` a `color_field`; `String` a `text_input`; a struct or tuple struct a `section` of its fields (open at first, and its state kept across a rebuilt inspector by a `SectionId` of the type and field path). Anything else (an `Option`, a `Vec`, a map, an enum with data) is a line `Name: {value:?}` that follows the value. A field is labelled with its name (`max_speed` is "Max speed").
- **Hints come from the type**, as Unity's attributes do: a field's doc comment is its tooltip; `#[reflect(@RangeInclusive::<f32>::new(0.0, 10.0))]` (also `f64` and `i32` ranges) is its range; `#[reflect(@InspectStep(0.5))]` its step. With none an integer steps by 1 and a float by 0.1, or by a hundredth of its range.
- **Edits write back through `ReflectMut`** at the field's reflect path and send **one `InspectorChange { entity, path }`** on the inspector, after the write. The component's change tick moves, so a `Changed<Turret>` in your game sees it; a value that is already equal is not written and says nothing.
- **The rows follow the data.** A value the game writes moves the row in the same frame with no `InspectorChange`; a text field the player is typing in is left alone until the focus leaves it.
- **A multi-selection reads mixed.** `InspectMixed(vec!["yaw".into(), "position.x".into()])` on the inspector's root names the reflect paths whose values differ across the selection your game inspects: a number or colour row reads mixed (`MixedValue`, the `--` hint in its field), a vector row's listed components do (`VecMixed`; its own path marks every component). The player's edit of a mixed row takes the marker off it and sends the one `InspectorChange`; your game applies the edit to every member and then takes the path out of the list. Changing the list, or removing `InspectMixed`, puts each row it names back to the component's own value or to mixed; a list that has not changed writes nothing. A path the target does not have is ignored, and toggles, choices and text rows have no mixed state here and keep the component's value.
- **The shape is fixed when the inspector is added.** A field that gains or loses entries (a `Vec`) is a read-only line; to inspect another thing, despawn the inspector and spawn a new one. Adding and removing components and enum variants that carry data are not part of this widget.

`examples/inspector.rs` shows it (`cargo run --example inspector --features reflect`); `GOOEY_KEYS="tab;enter;down;right;right;down;up;up;up"` flips Online, steps Mode and Rate.

Research: [`docs/research/inspector.md`](../research/inspector.md).
