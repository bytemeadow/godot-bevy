use bevy_ecs::{
    component::ComponentInfo,
    entity::Entity,
    reflect::{AppTypeRegistry, ReflectComponent},
    world::World,
};
use bevy_reflect::{
    PartialReflect, Reflect, ReflectFromPtr, ReflectFromReflect, ReflectMut, ReflectRef, TypeInfo,
    TypeRegistry,
    enums::{DynamicEnum, DynamicVariant, VariantInfo},
};

use super::value::{
    InspectorRange, InspectorReadOnly, Kind, ReadOnlyReason, Scalar, Value, ValueLimits, Writable,
    field_attributes, inspect,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathSegment {
    Field(String),
    Index(usize),
    /// Checks the active variant before traversing its payload.
    Variant(String),
}

#[derive(Clone, Debug)]
pub enum Edit {
    Scalar(Scalar),
    Variant(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct InspectionError(pub &'static str);

fn component_info<'w>(
    world: &'w World,
    entity: Entity,
    type_path: &str,
) -> Result<&'w ComponentInfo, InspectionError> {
    let entity = world
        .get_entity(entity)
        .map_err(|_| InspectionError("stale entity"))?;
    let registry = world
        .get_resource::<AppTypeRegistry>()
        .map(|registry| registry.read());
    entity
        .archetype()
        .components()
        .iter()
        .filter_map(|id| world.components().get_info(*id))
        .find(|info| {
            info.name().to_string() == type_path
                || info
                    .type_id()
                    .and_then(|id| registry.as_ref()?.get(id))
                    .is_some_and(|registration| registration.type_info().type_path() == type_path)
        })
        .ok_or(InspectionError("component absent"))
}

fn permission(info: &ComponentInfo, registry: &TypeRegistry) -> Writable {
    let Some(registration) = info.type_id().and_then(|id| registry.get(id)) else {
        return Writable::No(ReadOnlyReason::ComponentNotRegistered);
    };
    if !info.mutable() {
        return Writable::No(ReadOnlyReason::ComponentImmutable);
    }
    if info.relationship_accessor().is_some() {
        return Writable::No(ReadOnlyReason::Relationship);
    }
    if registration.data::<ReflectComponent>().is_none() {
        return Writable::No(ReadOnlyReason::NoReflectComponent);
    }
    Writable::Yes
}

/// Reads a component with registration, field permissions and traversal limits applied.
pub fn read_component(
    world: &World,
    entity: Entity,
    type_path: &str,
    limits: &ValueLimits,
) -> Result<Value, InspectionError> {
    let info = component_info(world, entity, type_path)?;
    let Some(registry) = world.get_resource::<AppTypeRegistry>() else {
        return Ok(Value::unsupported(
            type_path.into(),
            ReadOnlyReason::ComponentNotRegistered,
        ));
    };
    let registry = registry.read();
    let Some(registration) = info.type_id().and_then(|id| registry.get(id)) else {
        return Ok(Value::unsupported(
            type_path.into(),
            ReadOnlyReason::ComponentNotRegistered,
        ));
    };
    let Some(from_ptr) = registration.data::<ReflectFromPtr>() else {
        return Ok(Value::unsupported(
            type_path.into(),
            ReadOnlyReason::OpaqueAncestor,
        ));
    };
    let entity = world.entity(entity);
    let ptr = entity
        .get_by_id(info.id())
        .map_err(|_| InspectionError("component absent"))?;
    // SAFETY: registration and pointer come from the same component TypeId; the world is borrowed immutably.
    let value = unsafe { from_ptr.as_reflect(ptr) };
    Ok(inspect(
        value.as_partial_reflect(),
        &registry,
        permission(info, &registry),
        limits,
    ))
}

/// Validates a leaf edit before mutable access and applies it with ECS change detection.
pub fn mutate_component(
    world: &mut World,
    entity: Entity,
    type_path: &str,
    path: &[PathSegment],
    edit: &Edit,
    limits: &ValueLimits,
) -> Result<Value, InspectionError> {
    if path.len() > limits.max_depth {
        return Err(InspectionError("maximum depth reached"));
    }
    let info = component_info(world, entity, type_path)?;
    let registry = world
        .get_resource::<AppTypeRegistry>()
        .cloned()
        .ok_or(InspectionError("component not registered"))?;
    let registry = registry.read();
    if let Writable::No(reason) = permission(info, &registry) {
        return Err(InspectionError(reason.as_str()));
    }
    let adapter = info
        .type_id()
        .and_then(|id| registry.get_type_data::<ReflectComponent>(id))
        .cloned()
        .ok_or(InspectionError("no ReflectComponent"))?;
    let reflected = adapter
        .reflect(world.entity(entity))
        .ok_or(InspectionError("component absent"))?;
    let (leaf, range) = leaf(reflected.as_partial_reflect(), path, &registry)?;
    let value = inspect(leaf, &registry, Writable::Yes, limits);
    if let Writable::No(reason) = value.writable {
        return Err(InspectionError(reason.as_str()));
    }
    let decoded = decode(leaf, edit, &registry)?;
    let mut accepted = inspect(
        decoded.as_partial_reflect(),
        &registry,
        Writable::Yes,
        limits,
    );
    if let Some(range) = range {
        if !within_range(&accepted.kind, range) {
            return Err(InspectionError("value outside InspectorRange"));
        }
        accepted.range = Some(range);
    }
    if decoded.as_any().type_id()
        != leaf
            .try_as_reflect()
            .ok_or(InspectionError("decode failure"))?
            .as_any()
            .type_id()
    {
        return Err(InspectionError("decode failure"));
    }
    let mut reflected = adapter
        .reflect_mut(world.entity_mut(entity))
        .ok_or(InspectionError("component absent"))?;
    let leaf = leaf_mut(reflected.as_partial_reflect_mut(), path)
        .expect("path validated under exclusive world access");
    leaf.try_as_reflect_mut()
        .expect("concrete leaf validated before mutable access")
        .set(decoded)
        .expect("decoded leaf has the target TypeId");
    Ok(accepted)
}

fn within_range(kind: &Kind, range: InspectorRange) -> bool {
    if !range.min.is_finite() || !range.max.is_finite() || range.min > range.max {
        return false;
    }
    let signed = |number: i128| {
        let boundary = 2.0_f64.powi(127);
        (range.min <= -boundary || (range.min < boundary && number >= range.min.ceil() as i128))
            && (range.max >= boundary
                || (range.max >= -boundary && number <= range.max.floor() as i128))
    };
    match kind {
        Kind::Scalar(Scalar::Float(number)) => *number >= range.min && *number <= range.max,
        Kind::Scalar(Scalar::Integer(number)) => signed(i128::from(*number)),
        Kind::Scalar(Scalar::Decimal(number)) => {
            if let Ok(number) = number.parse::<i128>() {
                return signed(number);
            }
            let Ok(number) = number.parse::<u128>() else {
                return false;
            };
            let boundary = 2.0_f64.powi(128);
            (range.min <= 0.0 || (range.min < boundary && number >= range.min.ceil() as u128))
                && (range.max >= boundary
                    || (range.max >= 0.0 && number <= range.max.floor() as u128))
        }
        _ => false,
    }
}

fn leaf<'a>(
    value: &'a dyn PartialReflect,
    path: &[PathSegment],
    registry: &TypeRegistry,
) -> Result<(&'a dyn PartialReflect, Option<InspectorRange>), InspectionError> {
    if path.is_empty() {
        return Ok((value, None));
    }
    if matches!(
        inspect(
            value,
            registry,
            Writable::Yes,
            &ValueLimits {
                max_depth: 0,
                max_elements: 0
            }
        )
        .kind,
        Kind::Entity { .. } | Kind::Asset { .. }
    ) {
        return Err(InspectionError(ReadOnlyReason::Reference.as_str()));
    }
    let parent = value;
    let (first, rest) = path.split_first().unwrap();
    let next = match (value.reflect_ref(), first) {
        (ReflectRef::Struct(value), PathSegment::Field(name)) => value.field(name),
        (ReflectRef::TupleStruct(value), PathSegment::Index(index)) => value.field(*index),
        (ReflectRef::Tuple(value), PathSegment::Index(index)) => value.field(*index),
        (ReflectRef::List(value), PathSegment::Index(index)) => value.get(*index),
        (ReflectRef::Array(value), PathSegment::Index(index)) => value.get(*index),
        (ReflectRef::Enum(value), PathSegment::Variant(name)) if value.variant_name() == name => {
            let Some((field, rest)) = rest.split_first() else {
                return Err(InspectionError("path shape changed"));
            };
            let child = match field {
                PathSegment::Field(name) => value.field(name),
                PathSegment::Index(index) => value.field_at(*index),
                _ => None,
            }
            .ok_or(InspectionError("path shape changed"))?;
            return descend(parent, child, field, rest, registry);
        }
        (ReflectRef::Opaque(_), _) => {
            return Err(InspectionError(ReadOnlyReason::OpaqueAncestor.as_str()));
        }
        (ReflectRef::Map(_) | ReflectRef::Set(_), _) => {
            return Err(InspectionError(ReadOnlyReason::KindNotEditable.as_str()));
        }
        _ => None,
    }
    .ok_or(InspectionError("path shape changed"))?;
    descend(parent, next, first, rest, registry)
}

fn descend<'a>(
    parent: &dyn PartialReflect,
    child: &'a dyn PartialReflect,
    field: &PathSegment,
    rest: &[PathSegment],
    registry: &TypeRegistry,
) -> Result<(&'a dyn PartialReflect, Option<InspectorRange>), InspectionError> {
    let attributes = match field {
        PathSegment::Field(name) => field_attributes(parent, Some(name), 0),
        PathSegment::Index(index) => field_attributes(parent, None, *index),
        PathSegment::Variant(_) => None,
    };
    if attributes.is_some_and(|attributes| attributes.contains::<InspectorReadOnly>()) {
        return Err(InspectionError(ReadOnlyReason::InspectorReadOnly.as_str()));
    }
    let (leaf, range) = leaf(child, rest, registry)?;
    Ok((
        leaf,
        range.or_else(|| {
            attributes
                .and_then(|attributes| attributes.get::<InspectorRange>())
                .copied()
        }),
    ))
}

fn leaf_mut<'a>(
    value: &'a mut dyn PartialReflect,
    path: &[PathSegment],
) -> Option<&'a mut dyn PartialReflect> {
    let Some((first, rest)) = path.split_first() else {
        return Some(value);
    };
    let child = match (value.reflect_mut(), first) {
        (ReflectMut::Struct(value), PathSegment::Field(name)) => value.field_mut(name),
        (ReflectMut::TupleStruct(value), PathSegment::Index(index)) => value.field_mut(*index),
        (ReflectMut::Tuple(value), PathSegment::Index(index)) => value.field_mut(*index),
        (ReflectMut::List(value), PathSegment::Index(index)) => value.get_mut(*index),
        (ReflectMut::Array(value), PathSegment::Index(index)) => value.get_mut(*index),
        (ReflectMut::Enum(value), PathSegment::Variant(_)) => {
            let (field, rest) = rest.split_first()?;
            let child = match field {
                PathSegment::Field(name) => value.field_mut(name),
                PathSegment::Index(index) => value.field_at_mut(*index),
                _ => None,
            }?;
            return leaf_mut(child, rest);
        }
        _ => None,
    }?;
    leaf_mut(child, rest)
}

fn decode(
    value: &dyn PartialReflect,
    edit: &Edit,
    registry: &TypeRegistry,
) -> Result<Box<dyn Reflect>, InspectionError> {
    let fail = || InspectionError("decode failure");
    let concrete = value.try_as_reflect().ok_or_else(fail)?;
    if let Edit::Variant(name) = edit {
        let Some(TypeInfo::Enum(info)) = value.get_represented_type_info() else {
            return Err(fail());
        };
        let Some(VariantInfo::Unit(_)) = info.variant(name) else {
            return Err(InspectionError(
                "payload-bearing variant construction is not editable",
            ));
        };
        let mut variant = DynamicEnum::new(name, DynamicVariant::Unit);
        variant.set_represented_type(value.get_represented_type_info());
        return registry
            .get_type_data::<ReflectFromReflect>(concrete.as_any().type_id())
            .and_then(|from| from.from_reflect(&variant))
            .ok_or_else(fail);
    }
    let Edit::Scalar(input) = edit else {
        unreachable!()
    };
    macro_rules! integer {
        ($($ty:ty),*) => { $(
            if concrete.is::<$ty>() {
                let text = match input {
                    Scalar::Integer(value) => value.to_string(),
                    Scalar::Float(value) if value.is_finite() && value.fract() == 0.0 => {
                        if *value == 0.0 { "0".into() } else { format!("{value:.0}") }
                    }
                    Scalar::Decimal(value) | Scalar::String(value) => value.clone(),
                    _ => return Err(fail()),
                };
                return text.parse::<$ty>().map(|value| Box::new(value) as Box<dyn Reflect>).map_err(|_| fail());
            }
        )* };
    }
    integer!(
        i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
    );
    if concrete.is::<f32>() || concrete.is::<f64>() {
        let number = match input {
            Scalar::Float(number) => *number,
            Scalar::Integer(number) => *number as f64,
            _ => return Err(fail()),
        };
        if !number.is_finite() {
            return Err(fail());
        }
        if concrete.is::<f32>() {
            let number = number as f32;
            return if number.is_finite() {
                Ok(Box::new(number))
            } else {
                Err(fail())
            };
        }
        return Ok(Box::new(number));
    }
    match input {
        Scalar::Bool(value) if concrete.is::<bool>() => Ok(Box::new(*value)),
        Scalar::String(value) if concrete.is::<String>() => Ok(Box::new(value.clone())),
        Scalar::Char(value) if concrete.is::<char>() => Ok(Box::new(*value)),
        Scalar::String(value) if concrete.is::<char>() => {
            let mut chars = value.chars();
            let value = chars.next().ok_or_else(fail)?;
            if chars.next().is_some() {
                return Err(fail());
            }
            Ok(Box::new(value))
        }
        _ => Err(fail()),
    }
}
