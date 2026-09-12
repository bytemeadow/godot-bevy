use bevy_asset::ReflectHandle;
use bevy_ecs::entity::Entity;
use bevy_reflect::{
    PartialReflect, Reflect, ReflectFromReflect, ReflectRef, TypeInfo, TypeRegistry,
    attributes::CustomAttributes, enums::VariantInfo,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Value {
    pub type_path: String,
    pub kind: Kind,
    pub writable: Writable,
    pub range: Option<InspectorRange>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Scalar(Scalar),
    Struct(Vec<Field>),
    TupleStruct(Vec<Value>),
    Tuple(Vec<Value>),
    List {
        items: Vec<Value>,
        truncated: usize,
    },
    Array {
        items: Vec<Value>,
        truncated: usize,
    },
    Map {
        entries: Vec<(Value, Value)>,
        truncated: usize,
    },
    Set {
        members: Vec<Value>,
        truncated: usize,
    },
    Enum {
        variant: String,
        fields: Vec<Field>,
        unit_variants: Vec<String>,
    },
    Entity {
        bits: u64,
        generation: u32,
    },
    Node {
        instance_id: u64,
        valid: bool,
    },
    Asset {
        id: String,
    },
    Opaque(String),
    Unsupported(String),
    DepthLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Bool(bool),
    Integer(i64),
    Float(f64),
    Decimal(String),
    String(String),
    Char(char),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Writable {
    Yes,
    No(ReadOnlyReason),
}

impl Writable {
    pub(crate) fn restrict(&self, reason: ReadOnlyReason) -> Self {
        match self {
            Self::Yes => Self::No(reason),
            Self::No(_) => self.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadOnlyReason {
    ComponentNotRegistered,
    NoReflectComponent,
    ComponentImmutable,
    OpaqueAncestor,
    KindNotEditable,
    Reference,
    Relationship,
    DepthLimit,
    InspectorReadOnly,
}

impl ReadOnlyReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ComponentNotRegistered => "component not registered",
            Self::NoReflectComponent => "no ReflectComponent",
            Self::ComponentImmutable => "component immutable",
            Self::OpaqueAncestor => "field reached through an opaque or unsupported ancestor",
            Self::KindNotEditable => "kind not editable in this round",
            Self::Reference => "reference is read-only",
            Self::Relationship => "relationship is read-only",
            Self::DepthLimit => "maximum depth reached",
            Self::InspectorReadOnly => "field is marked InspectorReadOnly",
        }
    }
}

/// Field attribute that shows a value in the inspector but rejects edits to it or its children.
#[derive(Clone, Copy, Debug, Reflect)]
pub struct InspectorReadOnly;

/// Field attribute that enforces inclusive bounds on scalar edits in the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct InspectorRange {
    pub min: f64,
    pub max: f64,
}

impl InspectorRange {
    pub const fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }
}

pub(crate) fn field_attributes(
    value: &dyn PartialReflect,
    name: Option<&str>,
    index: usize,
) -> Option<&'static CustomAttributes> {
    match value.get_represented_type_info()? {
        TypeInfo::Struct(info) => Some(info.field(name?)?.custom_attributes()),
        TypeInfo::TupleStruct(info) => Some(info.field_at(index)?.custom_attributes()),
        TypeInfo::Tuple(info) => Some(info.field_at(index)?.custom_attributes()),
        TypeInfo::Enum(info) => {
            let ReflectRef::Enum(value) = value.reflect_ref() else {
                return None;
            };
            match info.variant(value.variant_name())? {
                VariantInfo::Struct(info) => Some(
                    match name {
                        Some(name) => info.field(name)?,
                        None => info.field_at(index)?,
                    }
                    .custom_attributes(),
                ),
                VariantInfo::Tuple(info) => Some(info.field_at(index)?.custom_attributes()),
                VariantInfo::Unit(_) => None,
            }
        }
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ValueLimits {
    pub max_depth: usize,
    pub max_elements: usize,
}

impl Default for ValueLimits {
    fn default() -> Self {
        Self {
            max_depth: 16,
            max_elements: 128,
        }
    }
}

impl Value {
    pub(crate) fn unsupported(type_path: String, reason: ReadOnlyReason) -> Self {
        Self {
            type_path,
            kind: Kind::Unsupported(reason.as_str().into()),
            writable: Writable::No(reason),
            range: None,
        }
    }
}

/// Inspects a reflected value, inheriting the supplied writability and enforcing traversal limits.
pub fn inspect(
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    writable: Writable,
    limits: &ValueLimits,
) -> Value {
    inspect_at(value, registry, writable, limits, 0)
}

fn inspect_at(
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    writable: Writable,
    limits: &ValueLimits,
    depth: usize,
) -> Value {
    let type_path = value.reflect_type_path().to_string();
    let make = |kind, writable| Value {
        type_path: type_path.clone(),
        kind,
        writable,
        range: None,
    };
    if let Some(entity) = value.try_downcast_ref::<Entity>() {
        return make(
            Kind::Entity {
                bits: entity.to_bits(),
                generation: entity.generation().to_bits(),
            },
            writable.restrict(ReadOnlyReason::Reference),
        );
    }
    if let Some(reflected) = value.try_as_reflect()
        && let Some(adapter) = registry.get_type_data::<ReflectHandle>(reflected.as_any().type_id())
        && let Some(handle) = adapter.downcast_handle_untyped(reflected.as_any())
    {
        return make(
            Kind::Asset {
                id: format!("{:?}", handle.id()),
            },
            writable.restrict(ReadOnlyReason::Reference),
        );
    }
    if let Some(scalar) = scalar(value) {
        let writable = if value.try_downcast_ref::<&str>().is_some() {
            writable.restrict(ReadOnlyReason::KindNotEditable)
        } else {
            writable
        };
        return make(Kind::Scalar(scalar), writable);
    }
    if depth >= limits.max_depth {
        return make(
            Kind::DepthLimit,
            writable.restrict(ReadOnlyReason::DepthLimit),
        );
    }
    let child = |value: &dyn PartialReflect| {
        inspect_at(value, registry, writable.clone(), limits, depth + 1)
    };
    let readonly_child = |value: &dyn PartialReflect| {
        inspect_at(
            value,
            registry,
            writable.restrict(ReadOnlyReason::KindNotEditable),
            limits,
            depth + 1,
        )
    };
    let field = |child: &dyn PartialReflect, name: Option<&str>, index: usize| {
        let attributes = field_attributes(value, name, index);
        let writable =
            if attributes.is_some_and(|attributes| attributes.contains::<InspectorReadOnly>()) {
                writable.restrict(ReadOnlyReason::InspectorReadOnly)
            } else {
                writable.clone()
            };
        let mut result = inspect_at(child, registry, writable, limits, depth + 1);
        result.range = attributes
            .and_then(|attributes| attributes.get::<InspectorRange>())
            .copied();
        result
    };
    let kind = match value.reflect_ref() {
        ReflectRef::Struct(value) => Kind::Struct(
            (0..value.field_len())
                .filter_map(|i| {
                    Some(Field {
                        name: value.name_at(i)?.into(),
                        value: field(value.field_at(i)?, value.name_at(i), i),
                    })
                })
                .collect(),
        ),
        ReflectRef::TupleStruct(value) => Kind::TupleStruct(
            value
                .iter_fields()
                .enumerate()
                .map(|(i, value)| field(value, None, i))
                .collect(),
        ),
        ReflectRef::Tuple(value) => Kind::Tuple(
            value
                .iter_fields()
                .enumerate()
                .map(|(i, value)| field(value, None, i))
                .collect(),
        ),
        ReflectRef::List(value) => Kind::List {
            items: value.iter().take(limits.max_elements).map(child).collect(),
            truncated: value.len().saturating_sub(limits.max_elements),
        },
        ReflectRef::Array(value) => Kind::Array {
            items: value.iter().take(limits.max_elements).map(child).collect(),
            truncated: value.len().saturating_sub(limits.max_elements),
        },
        ReflectRef::Map(value) => Kind::Map {
            entries: value
                .iter()
                .take(limits.max_elements)
                .map(|(key, value)| (readonly_child(key), readonly_child(value)))
                .collect(),
            truncated: value.len().saturating_sub(limits.max_elements),
        },
        ReflectRef::Set(value) => Kind::Set {
            members: value
                .iter()
                .take(limits.max_elements)
                .map(readonly_child)
                .collect(),
            truncated: value.len().saturating_sub(limits.max_elements),
        },
        ReflectRef::Enum(value) => {
            let unit_variants: Vec<String> = match value.get_represented_type_info() {
                Some(TypeInfo::Enum(info)) => info
                    .iter()
                    .filter(|variant| matches!(variant, VariantInfo::Unit(_)))
                    .map(|variant| variant.name().to_string())
                    .collect(),
                _ => Vec::new(),
            };
            let can_switch = !unit_variants.is_empty()
                && value.try_as_reflect().is_some_and(|value| {
                    registry
                        .get_type_data::<ReflectFromReflect>(value.as_any().type_id())
                        .is_some()
                });
            return make(
                Kind::Enum {
                    variant: value.variant_name().into(),
                    fields: (0..value.field_len())
                        .filter_map(|i| {
                            Some(Field {
                                name: value
                                    .name_at(i)
                                    .map(str::to_string)
                                    .unwrap_or_else(|| i.to_string()),
                                value: field(value.field_at(i)?, value.name_at(i), i),
                            })
                        })
                        .collect(),
                    unit_variants,
                },
                if can_switch {
                    writable
                } else {
                    writable.restrict(ReadOnlyReason::KindNotEditable)
                },
            );
        }
        ReflectRef::Opaque(_) => {
            return make(
                Kind::Opaque(format!("{value:?}")),
                writable.restrict(ReadOnlyReason::OpaqueAncestor),
            );
        }
    };
    make(kind, writable.restrict(ReadOnlyReason::KindNotEditable))
}

fn scalar(value: &dyn PartialReflect) -> Option<Scalar> {
    macro_rules! integer {
        ($($ty:ty),*) => { $(
            if let Some(value) = value.try_downcast_ref::<$ty>() {
                return Some(match i64::try_from(*value) {
                    Ok(value) => Scalar::Integer(value),
                    Err(_) => Scalar::Decimal(value.to_string()),
                });
            }
        )* };
    }
    macro_rules! small_integer {
        ($($ty:ty),*) => { $(
            if let Some(value) = value.try_downcast_ref::<$ty>() {
                return Some(Scalar::Integer(i64::from(*value)));
            }
        )* };
    }
    if let Some(value) = value.try_downcast_ref::<i64>() {
        return Some(Scalar::Integer(*value));
    }
    small_integer!(i8, i16, i32, u8, u16, u32);
    integer!(i128, isize, u64, u128, usize);
    if let Some(value) = value.try_downcast_ref::<bool>() {
        return Some(Scalar::Bool(*value));
    }
    if let Some(value) = value.try_downcast_ref::<f32>() {
        return Some(Scalar::Float(f64::from(*value)));
    }
    if let Some(value) = value.try_downcast_ref::<f64>() {
        return Some(Scalar::Float(*value));
    }
    if let Some(value) = value.try_downcast_ref::<String>() {
        return Some(Scalar::String(value.clone()));
    }
    if let Some(value) = value.try_downcast_ref::<&str>() {
        return Some(Scalar::String((*value).into()));
    }
    if let Some(value) = value.try_downcast_ref::<char>() {
        return Some(Scalar::Char(*value));
    }
    None
}

#[cfg(test)]
#[path = "value_tests.rs"]
mod tests;
