# Reflect inspector

Researched 2026-11-02 (iteration 39, #18 G-14). home-still's index has nothing on property inspectors, so the sources below were fetched today.

## Sources

1. W3C WAI, *Forms Tutorial, Labeling Controls*, https://www.w3.org/WAI/tutorials/forms/labels/ — craft.
2. Unity Scripting API, `RangeAttribute` (a number's range and its widget chosen from an attribute on the field), https://docs.unity3d.com/ScriptReference/RangeAttribute.html — a shipping editor.
3. `bevy_reflect` 0.20, the `Reflect` derive's `#[reflect(@...)]` custom attributes, `NamedField::docs` and `TypeInfo`, https://docs.rs/bevy_reflect/0.20.0/bevy_reflect/derive.Reflect.html, plus this crate's `numbers.md`, `colors.md`, `sections.md` — technical.

## Craft principle

WAI, Labeling Controls: "Labels need to describe the purpose of the form control", and "most users prefer or even need visible labels to understand the form": a row an inspector builds from a field is always labelled by the field's name, never a bare control, and the field's doc comment is the longer words a tooltip offers, not a replacement for the label.

## How shipping games do it

Unity chooses the widget from the field's type and lets an attribute refine it: "When this attribute is used, the float or int will be shown as a slider in the Inspector instead of the default number field", written on the field itself (`[Range(0.2f, 0.8f)]`), so the type that owns the data owns its editor hints and no per-type panel is written. Bevy's reflection has the same hook: "`#[reflect(@...)]` can be used to register custom attributes to the field's `TypeInfo`. It accepts any expression after the `@` symbol that resolves to a value which implements `Reflect`", and "Any number of custom attributes may be registered, however … the type of each attribute must be unique".

## Model adopted

- **I1. `inspector(InspectTarget)`** is a column (`Role::Inspector`) that builds its rows the frame it is added, from the reflected value of its target: `InspectTarget::component::<T>(entity)` or `InspectTarget::resource::<T>()`. The type must be registered (`app.register_type::<T>()`, or Bevy's auto-registration) with `#[reflect(Component)]` / `#[reflect(Resource)]`; one that is not shows a read-only line saying so.
- **I2. One widget per kind of field**, chosen from the value's reflection: `bool` → `toggle`; an enum whose variants are all unit → `choice` over the variant names; `f32`, `f64` and the integers → `number_field`; `Vec2`, `Vec3`, `Vec4` → `vec_field`; `Srgba` and `Color` → `color_field`; `String` → `text_input`; a struct or tuple struct → a `section` of its fields (open, with a `SectionId` from the type path and field path, so it keeps its state); anything else (an `Option`, a `Vec`, a map, an enum with data) → a read-only line `name: {value:?}` that follows the value. A field is labelled with its name, humanised (`max_speed` → "Max speed").
- **I3. Hints from the type, as Unity's attributes are**: a field's doc comment is its tooltip (needs the `reflect` feature, which turns on Bevy's `reflect_documentation`); `#[reflect(@RangeInclusive::<f32>::new(0.0, 10.0))]` sets the number's range (`f32`, `f64`, `i32` ranges) and `#[reflect(@InspectStep(0.5))]` its step. With no range the field is unbounded, an integer's step is 1 and a float's 0.1.
- **I4. Edits write back through `ReflectMut`** at the field's reflect path and send one `InspectorChange { entity, path }` on the inspector (`EntityEvent`, `path` is Bevy's reflect path, `"nested.speed"`). The component's change tick moves, so a game's `Changed<T>` sees it. A value that is already equal is not written.
- **I5. The rows follow the data**: each frame the inspector reads the target and moves a row's state (`NumberValue`, `Checked`, `ChoiceValue`, `ColorValue`, `VecValue`, `TextInputValue`) only when it differs from the field, with no event, and never for the text field the player is typing in. The shape of the rows is fixed when the inspector is added: a field that gains or loses entries (a `Vec`) is read-only text.
- **I6. Not in this slice**: removing, adding or reordering components, enum variants with data, and rebuilding when the target's type changes: despawn the inspector and spawn another.

## Parameters

|part|value|
|---|---|
|feature|`reflect` (`bevy/reflect_documentation`); the module and everything it exports are behind it|
|state|`Inspector { target }`; per row `InspectRow { inspector, path }`|
|event|`InspectorChange { entity, path }`|
|hints|doc comments (tooltip), `RangeInclusive<f32 / f64 / i32>`, `InspectStep(f32)`|
|ids|`SectionId("inspect:<type path>.<field path>")`|

## Validation targets

- The issue's acceptance: a struct with a bool, a unit enum, an `f32` with a range, a `Vec3` and a nested struct yields exactly a toggle, a choice, a number field, a vec field and a section (holding the nested struct's own rows), and editing the `f32` changes the component.
- Each edit kind writes its field and sends one `InspectorChange` with the right path: a toggle, a choice, a number, one vec component, a colour, a typed string; a nested field's path is `"nested.speed"`.
- A number is held to its range and an integer field stays an integer; a doc comment is the row's tooltip.
- A game writing the component moves every row with no `InspectorChange`; a field the player is typing in is left alone.
- An unsupported field and an unregistered type are read-only lines; a resource target works like a component target.
- Thirty idle frames write nothing, component or widget.

## What great looks like

An editor's details panel: the component's fields stacked as labelled rows of the same controls the game's own settings use, nested structs folded under quiet headers, a drag on a number rolling the value with the object moving at once, and no panel written per type.

## Deviations

- Unity draws a ranged number as a slider; the issue asks for `number_field` with the range, and a drag on the field is the same gesture (design default, as the issue states).
- The issue names `#[reflect(@RangeInclusive)]`; `RangeInclusive<f32>` is the one the examples use, so `f64` and `i32` are accepted too (design default).
- A struct's `section` is open at first, and its state is kept by `SectionId` across a rebuilt inspector (design default, `sections.md`).

## Pending index

The Unity and `bevy_reflect` pages and the WAI tutorial are web pages not in home-still.
