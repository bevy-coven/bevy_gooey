//! The reflect inspector ([`crate::inspector`]): rows built from a component's or a resource's
//! reflected fields, edits written back through `ReflectMut`, and rows that follow the data. Its
//! look is the widgets' own, so there is nothing for [`crate::apply_skin`] to add but a column's gap.

use std::any::TypeId;
use std::ops::RangeInclusive;

use bevy::color::Srgba;
use bevy::ecs::reflect::{AppTypeRegistry, ReflectComponent};
use bevy::prelude::*;
use bevy::reflect::attributes::CustomAttributes;
use bevy::reflect::enums::{DynamicEnum, DynamicVariant, VariantInfo};
use bevy::reflect::{GetPath, PartialReflect, Reflect, ReflectRef, TypeInfo, TypeRegistry};
use bevy::ui::Checked;
use bevy::ui_widgets::{ValueChange, observe};

use crate::choice::ChoiceValue;
use crate::color::ColorValue;
use crate::number::{MixedValue, NumberValue, VecChange, VecMixed, VecValue};
use crate::section::SectionId;
use crate::skin::Role;
use crate::text::{TextEditing, TextInputValue};
use crate::tooltip::Tooltip;
use crate::widgets::{
    choice, color_field, label, number_field, section, text_input, toggle, vec_field,
};

/// What an [`inspector`](crate::inspector) edits: one reflected component of an entity, or a
/// resource. The type must be registered for reflection (`app.register_type::<T>()`) with
/// `#[reflect(Component)]` or `#[reflect(Resource)]`.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// #[derive(Component, Reflect, Default)]
/// #[reflect(Component)]
/// struct Engine { thrust: f32 }
///
/// #[derive(Resource, Reflect, Default)]
/// #[reflect(Resource)]
/// struct Weather { rain: bool }
///
/// # fn ui(mut commands: Commands, ship: Entity) {
/// commands.spawn(inspector(InspectTarget::component::<Engine>(ship)));
/// commands.spawn(inspector(InspectTarget::resource::<Weather>()));
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectTarget {
    /// A component of an entity.
    Component {
        /// The entity that has it.
        entity: Entity,
        /// The component's type.
        type_id: TypeId,
    },
    /// A resource.
    Resource {
        /// The resource's type.
        type_id: TypeId,
    },
}

impl InspectTarget {
    /// The component `T` of `entity`.
    pub fn component<T: Component + Reflect>(entity: Entity) -> Self {
        Self::Component {
            entity,
            type_id: TypeId::of::<T>(),
        }
    }

    /// The resource `T`.
    pub fn resource<T: Resource + Reflect>() -> Self {
        Self::Resource {
            type_id: TypeId::of::<T>(),
        }
    }

    fn type_id(self) -> TypeId {
        match self {
            Self::Component { type_id, .. } | Self::Resource { type_id } => type_id,
        }
    }
}

/// An [`inspector`](crate::inspector)'s root: what it edits. The rows are built from the target
/// the frame this is added and kept in step with it by [`sync_inspectors`]; to inspect something
/// else, despawn the inspector and spawn another.
#[derive(Component, Clone, Debug)]
pub struct Inspector {
    /// What it edits.
    pub target: InspectTarget,
    rows: Vec<Row>,
    /// The [`InspectMixed`] list [`sync_inspect_mixed`] last put on the rows.
    mixed: Vec<String>,
}

/// A step for a number field built from a reflected number, as a field attribute:
/// `#[reflect(@InspectStep(0.5))]`. Without it an integer steps by 1 and a float by 0.1 (a
/// hundredth of a range, when it has one).
#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
pub struct InspectStep(pub f32);

/// On an [`inspector`](crate::inspector)'s root: the reflect paths whose values **differ across the
/// host's selection** (`"yaw"`, `"position.x"`). A number or colour row whose path is listed reads
/// mixed ([`MixedValue`], the [`MIXED_HINT`](crate::MIXED_HINT) in its field), and a vector row's
/// listed components do ([`VecMixed`]; listing the vector's own path marks every component). An edit of
/// a mixed row takes the marker off it and sends the one [`InspectorChange`] an edit sends: the host
/// applies it to every member and then takes the path out of the list. Changing the list (a path
/// added, taken out, or the whole component removed) puts each row it names back to the component's
/// own value or to mixed; a row the player has edited is left as it is until the list changes
/// for that path. [`sync_inspect_mixed`] does it, only when the list differs from the one it last
/// applied. A path the target does not have is ignored.
///
/// Toggles, choices and text rows have no mixed state in this crate and are left as they are.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands, ship: Entity) {
/// #[derive(Component, Reflect, Default)]
/// #[reflect(Component)]
/// struct Engine { thrust: f32, yaw: f32 }
///
/// // Several ships are selected and their yaw differs.
/// commands.spawn((
///     inspector(InspectTarget::component::<Engine>(ship)),
///     InspectMixed(vec!["yaw".to_owned()]),
/// ));
/// # }
/// ```
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct InspectMixed(pub Vec<String>);

/// A player's edit, sent on the [`inspector`](crate::inspector) after it was written to the
/// component or resource. `path` is Bevy's reflect path of the field from the target (`"speed"`,
/// `"nested.speed"`, `"0"`). A value the game wrote itself sends none.
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq)]
pub struct InspectorChange {
    /// The inspector.
    #[event_target]
    pub entity: Entity,
    /// The field that changed.
    pub path: String,
}

/// On each row an inspector builds (not on a section): which inspector it belongs to and which
/// field it edits.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct InspectRow {
    /// The inspector.
    pub inspector: Entity,
    /// The field's reflect path, as in [`InspectorChange::path`].
    pub path: String,
}

#[derive(Clone, Debug)]
struct Row {
    entity: Entity,
    path: String,
    kind: Kind,
}

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Bool,
    Number,
    Choice(Vec<String>),
    Vec(usize),
    Colour,
    Text,
    ReadOnly(String),
}

/// What the widgets are built from: a description of the target's fields, owned so it outlives the
/// borrow of the world it was read from.
enum Field {
    Bool {
        value: bool,
    },
    Number {
        value: f32,
        step: f32,
        range: RangeInclusive<f32>,
    },
    Choice {
        options: Vec<String>,
        selected: usize,
    },
    Vec {
        values: Vec<f32>,
    },
    Colour {
        value: Srgba,
    },
    Text {
        value: String,
    },
    Group {
        fields: Vec<Named>,
    },
    ReadOnly {
        text: String,
    },
}

struct Named {
    name: String,
    path: String,
    docs: Option<String>,
    field: Field,
}

/// A change a widget asks for.
enum Edit {
    Bool(bool),
    Number(f64),
    Variant(String),
    Vec(Vec<f32>),
    Colour(Srgba),
    Text(String),
}

/// A row's state to move to the field's value.
enum Update {
    Bool(bool),
    Number(f32),
    Choice(usize),
    Vec(Vec<f32>),
    Colour(Srgba),
    Text(String),
    ReadOnly(String),
}

/// A column of the fields of a reflected component or resource, each one the widget for its type:
/// a `bool` is a [`toggle`], a unit enum a [`choice`], a number a [`number_field`], a `Vec2`,
/// `Vec3` or `Vec4` a [`vec_field`], a `Srgba` or `Color` a [`color_field`], a `String` a
/// [`text_input`], a nested struct a [`section`] of its own fields; anything else is a line of
/// text that follows the value. A field's doc comment is its tooltip, `#[reflect(@RangeInclusive::<f32>::new(0.0, 10.0))]`
/// its range and `#[reflect(@InspectStep(0.5))]` its step. An edit is written to the field and
/// sent as one [`InspectorChange`](crate::InspectorChange); a value the game writes moves the row
/// with no event.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// use std::ops::RangeInclusive;
///
/// #[derive(Component, Reflect, Default)]
/// #[reflect(Component)]
/// struct Engine {
///     /// Newtons at full burn.
///     #[reflect(@RangeInclusive::<f32>::new(0.0, 100.0))]
///     thrust: f32,
///     afterburner: bool,
/// }
///
/// # fn ui(mut commands: Commands, ship: Entity) {
/// commands.spawn((
///     inspector(InspectTarget::component::<Engine>(ship)),
///     observe(|change: On<InspectorChange>| info!("{} edited", change.path)),
/// ));
/// # }
/// ```
pub fn inspector(target: InspectTarget) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::Inspector,
        Inspector {
            target,
            rows: Vec::new(),
            mixed: Vec::new(),
        },
    )
}

/// The entity that holds the target: the component's own, or the resource's.
fn locate(world: &World, target: InspectTarget) -> Option<Entity> {
    match target {
        InspectTarget::Component { entity, .. } => Some(entity),
        InspectTarget::Resource { type_id } => {
            let id = world.components().get_id(type_id)?;
            world.resource_entities().get(id)
        }
    }
}

/// The reflected value of the target.
fn read<'w>(
    world: &'w World,
    registry: &TypeRegistry,
    target: InspectTarget,
) -> Option<&'w dyn Reflect> {
    let reflect = registry.get(target.type_id())?.data::<ReflectComponent>()?;
    let entity = world.get_entity(locate(world, target)?).ok()?;
    reflect.reflect(entity)
}

/// The value of the field at `path`; the path of the target itself is empty.
fn field_at<'v>(value: &'v dyn Reflect, path: &str) -> Option<&'v dyn PartialReflect> {
    if path.is_empty() {
        return Some(value.as_partial_reflect());
    }
    value.reflect_path(format!(".{path}").as_str()).ok()
}

/// "max_speed" as "Max speed".
fn humanise(name: &str) -> String {
    let mut words = name.replace('_', " ");
    if let Some(first) = words.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    words
}

/// A field's doc comment as one tooltip line.
fn docs_of(docs: Option<&str>) -> Option<String> {
    let text = docs?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then_some(text)
}

fn number_of(value: &dyn PartialReflect) -> Option<f64> {
    macro_rules! numbers {
        ($($t:ty),*) => {
            $(if let Some(n) = value.try_downcast_ref::<$t>() {
                return Some(*n as f64);
            })*
        };
    }
    numbers!(f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);
    None
}

fn is_integer(value: &dyn PartialReflect) -> bool {
    macro_rules! integers {
        ($($t:ty),*) => { $(value.try_downcast_ref::<$t>().is_some())||* };
    }
    integers!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize)
}

/// Writes `new` to a number field of any numeric type (an integer rounds); whether it changed.
fn set_number(value: &mut dyn PartialReflect, new: f64) -> Option<bool> {
    macro_rules! floats {
        ($($t:ty),*) => {
            $(if let Some(n) = value.try_downcast_mut::<$t>() {
                let new = new as $t;
                let changed = *n != new;
                *n = new;
                return Some(changed);
            })*
        };
    }
    macro_rules! integers {
        ($($t:ty),*) => {
            $(if let Some(n) = value.try_downcast_mut::<$t>() {
                let new = new.round() as $t;
                let changed = *n != new;
                *n = new;
                return Some(changed);
            })*
        };
    }
    floats!(f32, f64);
    integers!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);
    None
}

fn vector_of(value: &dyn PartialReflect) -> Option<Vec<f32>> {
    if let Some(v) = value.try_downcast_ref::<Vec2>() {
        return Some(v.to_array().to_vec());
    }
    if let Some(v) = value.try_downcast_ref::<Vec3>() {
        return Some(v.to_array().to_vec());
    }
    value
        .try_downcast_ref::<Vec4>()
        .map(|v| v.to_array().to_vec())
}

fn colour_of(value: &dyn PartialReflect) -> Option<Srgba> {
    if let Some(c) = value.try_downcast_ref::<Srgba>() {
        return Some(*c);
    }
    value.try_downcast_ref::<Color>().map(|c| Srgba::from(*c))
}

/// The variant names of an enum whose variants all carry nothing.
fn unit_variants(value: &dyn PartialReflect) -> Option<Vec<String>> {
    let ReflectRef::Enum(e) = value.reflect_ref() else {
        return None;
    };
    let TypeInfo::Enum(info) = e.get_represented_type_info()? else {
        return None;
    };
    info.iter()
        .all(|v| matches!(v, VariantInfo::Unit(_)))
        .then(|| info.iter().map(|v| v.name().to_string()).collect())
}

fn range_of(attributes: &CustomAttributes) -> RangeInclusive<f32> {
    if let Some(r) = attributes.get::<RangeInclusive<f32>>() {
        return *r.start()..=*r.end();
    }
    if let Some(r) = attributes.get::<RangeInclusive<f64>>() {
        return *r.start() as f32..=*r.end() as f32;
    }
    if let Some(r) = attributes.get::<RangeInclusive<i32>>() {
        return *r.start() as f32..=*r.end() as f32;
    }
    f32::NEG_INFINITY..=f32::INFINITY
}

/// Describes one field: its widget, from the value's reflected kind.
fn describe(
    value: &dyn PartialReflect,
    attributes: Option<&CustomAttributes>,
    path: &str,
) -> Field {
    if let Some(value) = value.try_downcast_ref::<bool>() {
        return Field::Bool { value: *value };
    }
    if let Some(number) = number_of(value) {
        let range = attributes.map_or(f32::NEG_INFINITY..=f32::INFINITY, range_of);
        let default_step = if is_integer(value) {
            1.0
        } else if range.start().is_finite() && range.end().is_finite() {
            ((range.end() - range.start()) / 100.0).max(f32::EPSILON)
        } else {
            0.1
        };
        let step = attributes
            .and_then(|a| a.get::<InspectStep>())
            .map_or(default_step, |s| s.0);
        return Field::Number {
            value: number as f32,
            step,
            range,
        };
    }
    if let Some(values) = vector_of(value) {
        return Field::Vec { values };
    }
    if let Some(value) = colour_of(value) {
        return Field::Colour { value };
    }
    if let Some(value) = value.try_downcast_ref::<String>() {
        return Field::Text {
            value: value.clone(),
        };
    }
    match value.reflect_ref() {
        ReflectRef::Struct(s) => {
            let info = s.get_represented_struct_info();
            let fields = (0..s.field_len())
                .filter_map(|i| {
                    let name = s.name_at(i)?;
                    let field = info.and_then(|info| info.field_at(i));
                    let path = join(path, name);
                    Some(Named {
                        name: humanise(name),
                        field: describe(
                            s.field_at(i)?,
                            field.map(|f| f.custom_attributes()),
                            &path,
                        ),
                        docs: docs_of(field.and_then(|f| f.docs())),
                        path,
                    })
                })
                .collect();
            Field::Group { fields }
        }
        ReflectRef::TupleStruct(t) => {
            let info = t.get_represented_tuple_struct_info();
            let fields = (0..t.field_len())
                .filter_map(|i| {
                    let field = info.and_then(|info| info.field_at(i));
                    let path = join(path, &i.to_string());
                    Some(Named {
                        name: i.to_string(),
                        field: describe(t.field(i)?, field.map(|f| f.custom_attributes()), &path),
                        docs: docs_of(field.and_then(|f| f.docs())),
                        path,
                    })
                })
                .collect();
            Field::Group { fields }
        }
        ReflectRef::Enum(e) => match unit_variants(value) {
            Some(options) => {
                let selected = e.variant_index();
                Field::Choice { options, selected }
            }
            None => Field::ReadOnly {
                text: format!("{value:?}"),
            },
        },
        _ => Field::ReadOnly {
            text: format!("{value:?}"),
        },
    }
}

fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}.{name}")
    }
}

/// Builds the rows of every inspector that was just added. Runs in
/// [`crate::GooeySystems::Style`], before everything that draws them, so a row stands in the
/// frame its inspector does.
pub fn build_inspectors(
    added: Query<(Entity, &Inspector), Added<Inspector>>,
    mut commands: Commands,
) {
    for (root, _) in &added {
        commands.queue(move |world: &mut World| build(world, root));
    }
}

fn build(world: &mut World, root: Entity) {
    let Some(target) = world.get::<Inspector>(root).map(|i| i.target) else {
        return;
    };
    let registry = world.resource::<AppTypeRegistry>().clone();
    let described = {
        let registry = registry.read();
        read(world, &registry, target).map(|value| {
            let field = describe(value.as_partial_reflect(), None, "");
            (value.reflect_type_path().to_string(), field)
        })
    };
    let mut rows = Vec::new();
    match described {
        Some((type_path, Field::Group { fields })) => {
            for named in fields {
                spawn_named(world, root, root, &type_path, named, &mut rows);
            }
        }
        Some((type_path, field)) => {
            let named = Named {
                name: type_path.rsplit("::").next().unwrap_or("Value").to_string(),
                path: String::new(),
                docs: None,
                field,
            };
            spawn_named(world, root, root, &type_path, named, &mut rows);
        }
        None => {
            world.spawn((
                label("Nothing to inspect: the type is not registered for reflection, or it is not there."),
                ChildOf(root),
            ));
        }
    }
    if let Some(mut inspector) = world.get_mut::<Inspector>(root) {
        inspector.rows = rows;
    }
}

/// Spawns the widget of one field under `parent` and records the row.
fn spawn_named(
    world: &mut World,
    inspector: Entity,
    parent: Entity,
    type_path: &str,
    named: Named,
    rows: &mut Vec<Row>,
) {
    let Named {
        name,
        path,
        docs,
        field,
    } = named;
    let row = InspectRow {
        inspector,
        path: path.clone(),
    };
    let (entity, kind) = match field {
        Field::Bool { value } => (
            world
                .spawn((toggle(name, value), observe(on_bool), row))
                .id(),
            Kind::Bool,
        ),
        Field::Number { value, step, range } => (
            world
                .spawn((
                    number_field(name, value, step, range),
                    observe(on_number),
                    row,
                ))
                .id(),
            Kind::Number,
        ),
        Field::Choice { options, selected } => (
            world
                .spawn((
                    choice(name, options.clone(), selected),
                    observe(on_choice),
                    row,
                ))
                .id(),
            Kind::Choice(options),
        ),
        Field::Vec { values } => {
            let count = values.len();
            let unbounded = f32::NEG_INFINITY..=f32::INFINITY;
            let mut entity = match count {
                2 => world.spawn(vec_field(name, [values[0], values[1]], 0.1, unbounded)),
                3 => world.spawn(vec_field(
                    name,
                    [values[0], values[1], values[2]],
                    0.1,
                    unbounded,
                )),
                _ => world.spawn(vec_field(
                    name,
                    [values[0], values[1], values[2], values[3]],
                    0.1,
                    unbounded,
                )),
            };
            entity.insert((observe(on_vec), row));
            (entity.id(), Kind::Vec(count))
        }
        Field::Colour { value } => (
            world
                .spawn((color_field(name, value), observe(on_colour), row))
                .id(),
            Kind::Colour,
        ),
        Field::Text { value } => (
            world
                .spawn((text_input(name, value), observe(on_text), row))
                .id(),
            Kind::Text,
        ),
        Field::ReadOnly { text } => (
            world.spawn((label(format!("{name}: {text}")), row)).id(),
            Kind::ReadOnly(name),
        ),
        Field::Group { fields } => {
            let id = format!("inspect:{type_path}.{path}");
            let group = world
                .spawn((section(name, true, ()), SectionId::new(id)))
                .id();
            if let Some(docs) = docs {
                world.entity_mut(group).insert(Tooltip::new(docs));
            }
            world.entity_mut(group).insert(ChildOf(parent));
            let body = world
                .get::<Children>(group)
                .and_then(|kids| kids.get(1).copied());
            if let Some(body) = body {
                for named in fields {
                    spawn_named(world, inspector, body, type_path, named, rows);
                }
            }
            return;
        }
    };
    if let Some(docs) = docs {
        world.entity_mut(entity).insert(Tooltip::new(docs));
    }
    world.entity_mut(entity).insert(ChildOf(parent));
    rows.push(Row { entity, path, kind });
}

fn on_bool(change: On<ValueChange<bool>>, rows: Query<&InspectRow>, mut commands: Commands) {
    ask(
        &rows,
        change.source,
        Edit::Bool(change.value),
        &mut commands,
    );
}

fn on_number(change: On<ValueChange<f32>>, rows: Query<&InspectRow>, mut commands: Commands) {
    ask(
        &rows,
        change.source,
        Edit::Number(f64::from(change.value)),
        &mut commands,
    );
}

fn on_choice(
    change: On<ValueChange<usize>>,
    rows: Query<&InspectRow>,
    inspectors: Query<&Inspector>,
    mut commands: Commands,
) {
    let Ok(row) = rows.get(change.source) else {
        return;
    };
    let name = inspectors.get(row.inspector).ok().and_then(|i| {
        i.rows
            .iter()
            .find(|r| r.entity == change.source)
            .and_then(|r| match &r.kind {
                Kind::Choice(options) => options.get(change.value).cloned(),
                _ => None,
            })
    });
    if let Some(name) = name {
        ask(&rows, change.source, Edit::Variant(name), &mut commands);
    }
}

fn on_vec(change: On<VecChange>, rows: Query<&InspectRow>, mut commands: Commands) {
    ask(
        &rows,
        change.entity,
        Edit::Vec(change.values.clone()),
        &mut commands,
    );
}

fn on_colour(change: On<ValueChange<Srgba>>, rows: Query<&InspectRow>, mut commands: Commands) {
    ask(
        &rows,
        change.source,
        Edit::Colour(change.value),
        &mut commands,
    );
}

fn on_text(change: On<ValueChange<String>>, rows: Query<&InspectRow>, mut commands: Commands) {
    ask(
        &rows,
        change.source,
        Edit::Text(change.value.clone()),
        &mut commands,
    );
}

fn ask(rows: &Query<&InspectRow>, row: Entity, edit: Edit, commands: &mut Commands) {
    let Ok(row) = rows.get(row) else {
        return;
    };
    let (inspector, path) = (row.inspector, row.path.clone());
    commands.queue(move |world: &mut World| apply(world, inspector, path, edit));
}

/// Writes an edit to the field and tells the game, unless the field already had that value.
fn apply(world: &mut World, inspector: Entity, path: String, edit: Edit) {
    let Some(target) = world.get::<Inspector>(inspector).map(|i| i.target) else {
        return;
    };
    let registry = world.resource::<AppTypeRegistry>().clone();
    let Some(reflect) = registry
        .read()
        .get(target.type_id())
        .and_then(|r| r.data::<ReflectComponent>())
        .cloned()
    else {
        return;
    };
    let Some(holder) = locate(world, target) else {
        return;
    };
    let Ok(mut entity) = world.get_entity_mut(holder) else {
        return;
    };
    let Some(mut value) = reflect.reflect_mut(&mut entity) else {
        return;
    };
    let changed = {
        let root = value.bypass_change_detection();
        let field = if path.is_empty() {
            Some(root.as_partial_reflect_mut())
        } else {
            root.reflect_path_mut(format!(".{path}").as_str()).ok()
        };
        field.is_some_and(|field| write(field, edit))
    };
    if changed {
        value.set_changed();
        world.trigger(InspectorChange {
            entity: inspector,
            path,
        });
    }
}

/// Puts an edit into a field of the kind it was made for; whether the field changed.
fn write(field: &mut dyn PartialReflect, edit: Edit) -> bool {
    match edit {
        Edit::Bool(new) => field.try_downcast_mut::<bool>().is_some_and(|f| {
            let changed = *f != new;
            *f = new;
            changed
        }),
        Edit::Number(new) => set_number(field, new).unwrap_or(false),
        Edit::Text(new) => field.try_downcast_mut::<String>().is_some_and(|f| {
            let changed = *f != new;
            *f = new;
            changed
        }),
        Edit::Colour(new) => {
            if let Some(f) = field.try_downcast_mut::<Srgba>() {
                let changed = *f != new;
                *f = new;
                changed
            } else if let Some(f) = field.try_downcast_mut::<Color>() {
                let new = Color::Srgba(new);
                let changed = *f != new;
                *f = new;
                changed
            } else {
                false
            }
        }
        Edit::Vec(new) => {
            if let (Some(f), [x, y]) = (
                field.try_downcast_mut::<Vec2>(),
                new[..].try_into().unwrap_or([f32::NAN; 2]),
            ) {
                let new = Vec2::new(x, y);
                let changed = *f != new;
                *f = new;
                changed
            } else if let (Some(f), [x, y, z]) = (
                field.try_downcast_mut::<Vec3>(),
                new[..].try_into().unwrap_or([f32::NAN; 3]),
            ) {
                let new = Vec3::new(x, y, z);
                let changed = *f != new;
                *f = new;
                changed
            } else if let (Some(f), [x, y, z, w]) = (
                field.try_downcast_mut::<Vec4>(),
                new[..].try_into().unwrap_or([f32::NAN; 4]),
            ) {
                let new = Vec4::new(x, y, z, w);
                let changed = *f != new;
                *f = new;
                changed
            } else {
                false
            }
        }
        Edit::Variant(name) => {
            let same =
                matches!(field.reflect_ref(), ReflectRef::Enum(e) if e.variant_name() == name);
            if same {
                return false;
            }
            let variant = DynamicEnum::new(name, DynamicVariant::Unit);
            field.try_apply(&variant).is_ok()
        }
    }
}

/// Keeps every inspector's rows as its target says: each row's state moves to the field's value
/// when it differs, with no event, and read-only lines follow the value. A text field the player is
/// typing in is left alone. Runs in [`crate::GooeySystems::Style`] before the rows are drawn; with
/// nothing changed it writes nothing.
pub fn sync_inspectors(world: &mut World, inspectors: &mut QueryState<(Entity, &Inspector)>) {
    let Some(registry) = world.get_resource::<AppTypeRegistry>().cloned() else {
        return;
    };
    let mut updates: Vec<(Entity, Update)> = Vec::new();
    {
        let registry = registry.read();
        let world_ref: &World = world;
        for (_, inspector) in inspectors.iter(world_ref) {
            let Some(value) = read(world_ref, &registry, inspector.target) else {
                continue;
            };
            for row in &inspector.rows {
                if let Some(update) = differs(world_ref, row, value) {
                    updates.push((row.entity, update));
                }
            }
        }
    }
    for (row, update) in updates {
        let Ok(mut entity) = world.get_entity_mut(row) else {
            continue;
        };
        match update {
            Update::Bool(on) => {
                if on {
                    entity.insert(Checked);
                } else {
                    entity.remove::<Checked>();
                }
            }
            Update::Number(n) => {
                entity.insert(NumberValue(n));
            }
            Update::Choice(i) => {
                entity.insert(ChoiceValue(i));
            }
            Update::Vec(v) => {
                entity.insert(VecValue(v));
            }
            Update::Colour(c) => {
                entity.insert(ColorValue(c));
            }
            Update::Text(t) => {
                entity.insert(TextInputValue(t));
            }
            Update::ReadOnly(text) => {
                if let Some(mut t) = entity.get_mut::<Text>() {
                    t.0 = text;
                }
            }
        }
    }
}

/// What a row shows that the field no longer says, if anything.
fn differs(world: &World, row: &Row, target: &dyn Reflect) -> Option<Update> {
    let field = field_at(target, &row.path)?;
    let entity = world.get_entity(row.entity).ok()?;
    match &row.kind {
        Kind::Bool => {
            let on = *field.try_downcast_ref::<bool>()?;
            (entity.contains::<Checked>() != on).then_some(Update::Bool(on))
        }
        Kind::Number => {
            let n = number_of(field)? as f32;
            (entity.get::<NumberValue>()?.0 != n).then_some(Update::Number(n))
        }
        Kind::Choice(_) => {
            let ReflectRef::Enum(e) = field.reflect_ref() else {
                return None;
            };
            let index = e.variant_index();
            (entity.get::<ChoiceValue>()?.0 != index).then_some(Update::Choice(index))
        }
        Kind::Vec(_) => {
            let values = vector_of(field)?;
            (entity.get::<VecValue>()?.0 != values).then_some(Update::Vec(values))
        }
        Kind::Colour => {
            let c = colour_of(field)?;
            let now = entity.get::<ColorValue>()?.0;
            let far = [
                (c.red, now.red),
                (c.green, now.green),
                (c.blue, now.blue),
                (c.alpha, now.alpha),
            ]
            .iter()
            .any(|(a, b)| (a - b).abs() > 1e-4);
            far.then_some(Update::Colour(c))
        }
        Kind::Text => {
            if entity.contains::<TextEditing>() {
                return None;
            }
            let text = field.try_downcast_ref::<String>()?;
            (&entity.get::<TextInputValue>()?.0 != text).then(|| Update::Text(text.clone()))
        }
        Kind::ReadOnly(name) => {
            let text = format!("{name}: {field:?}");
            (entity.get::<Text>()?.0 != text).then_some(Update::ReadOnly(text))
        }
    }
}

/// The marker a row's state moves to when the [`InspectMixed`] list changes.
enum Mark {
    /// A number or colour row: mixed or not.
    Mixed(bool),
    /// A vector row: which components are mixed.
    Parts(Vec<bool>),
}

/// Whether `path` is named by `list`.
fn listed(list: &[String], path: &str) -> bool {
    list.iter().any(|p| p == path)
}

/// The reflect path of component `index` of the vector at `path`.
fn component_path(path: &str, index: usize) -> String {
    let name = ["x", "y", "z", "w"].get(index).copied().unwrap_or("");
    if path.is_empty() {
        name.to_owned()
    } else {
        format!("{path}.{name}")
    }
}

/// Whether the vector at `path` has component `index` mixed under `list`: the component's own path,
/// or the vector's.
fn component_listed(list: &[String], path: &str, index: usize) -> bool {
    listed(list, path) || listed(list, &component_path(path, index))
}

/// What `row` moves to when the list goes from `was` to `now`, if it moves.
fn mark_of(row: &Row, was: &[String], now: &[String]) -> Option<(Entity, Mark)> {
    match &row.kind {
        Kind::Number | Kind::Colour => {
            let (before, after) = (listed(was, &row.path), listed(now, &row.path));
            (before != after).then(|| (row.entity, Mark::Mixed(after)))
        }
        Kind::Vec(count) => {
            let flags = |list: &[String]| -> Vec<bool> {
                (0..*count)
                    .map(|i| component_listed(list, &row.path, i))
                    .collect()
            };
            let after = flags(now);
            (flags(was) != after).then(|| (row.entity, Mark::Parts(after)))
        }
        Kind::Bool | Kind::Choice(_) | Kind::Text | Kind::ReadOnly(_) => None,
    }
}

/// Puts each row the [`InspectMixed`] list names into its mixed state, and each one it stopped naming
/// back to the component's own value. Runs in [`crate::GooeySystems::Style`] after the rows are built
/// and before they are kept in step; with the list as it was, it writes nothing.
pub fn sync_inspect_mixed(
    world: &mut World,
    inspectors: &mut QueryState<(Entity, &Inspector, Option<&InspectMixed>)>,
) {
    let mut moves: Vec<(Entity, Vec<String>, Vec<(Entity, Mark)>)> = Vec::new();
    for (root, inspector, wanted) in inspectors.iter(world) {
        let now: &[String] = wanted.map_or(&[], |mixed| mixed.0.as_slice());
        if inspector.mixed == now {
            continue;
        }
        let marks = inspector
            .rows
            .iter()
            .filter_map(|row| mark_of(row, &inspector.mixed, now))
            .collect();
        moves.push((root, now.to_vec(), marks));
    }
    for (root, now, marks) in moves {
        for (entity, mark) in marks {
            let Ok(mut row) = world.get_entity_mut(entity) else {
                continue;
            };
            match mark {
                Mark::Mixed(true) => {
                    row.insert(MixedValue);
                }
                Mark::Mixed(false) => {
                    row.remove::<MixedValue>();
                }
                Mark::Parts(flags) if flags.iter().any(|mixed| *mixed) => {
                    row.insert(VecMixed(flags));
                }
                Mark::Parts(_) => {
                    row.remove::<VecMixed>();
                }
            }
        }
        if let Some(mut inspector) = world.get_mut::<Inspector>(root) {
            inspector.mixed = now;
        }
    }
}
