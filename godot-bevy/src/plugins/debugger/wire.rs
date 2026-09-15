use std::collections::BTreeMap;

use godot::builtin::{GString, VarArray, VarDictionary as Dictionary, Variant, VariantType};
use godot::meta::ToGodot;

use super::value::{Field, Kind, Scalar, Value, Writable};

/// Owned transport data. Sinks may move it to another thread without moving Godot objects.
#[derive(Clone, Debug, PartialEq)]
pub enum Wire {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Wire>),
    Object(BTreeMap<String, Wire>),
}

impl Wire {
    pub fn object<const N: usize>(fields: [(&str, Wire); N]) -> Self {
        Self::Object(
            fields
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
        )
    }

    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(fields) => fields.get(key),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn to_variant(&self) -> Variant {
        match self {
            Self::Null => Variant::nil(),
            Self::Bool(value) => value.to_variant(),
            Self::Integer(value) => value.to_variant(),
            Self::Float(value) => value.to_variant(),
            Self::String(value) => value.as_str().to_variant(),
            Self::Array(items) => {
                let mut array = VarArray::new();
                for item in items {
                    array.push(&item.to_variant());
                }
                array.to_variant()
            }
            Self::Object(fields) => {
                let mut dict = Dictionary::new();
                for (name, value) in fields {
                    dict.set(name.as_str(), &value.to_variant());
                }
                dict.to_variant()
            }
        }
    }

    pub(crate) fn from_variant(
        value: &Variant,
        depth: usize,
        remaining: &mut usize,
    ) -> Result<Self, &'static str> {
        if depth > 32 || *remaining == 0 {
            return Err("request exceeds decoding budget");
        }
        *remaining -= 1;
        Ok(match value.get_type() {
            VariantType::NIL => Self::Null,
            VariantType::BOOL => Self::Bool(value.to()),
            VariantType::INT => Self::Integer(value.to()),
            VariantType::FLOAT => Self::Float(value.to()),
            VariantType::STRING => Self::String(value.to::<GString>().to_string()),
            VariantType::ARRAY => Self::Array(
                value
                    .try_to::<VarArray>()
                    .map_err(|_| "typed arrays are not supported in requests")?
                    .iter_shared()
                    .map(|item| Self::from_variant(&item, depth + 1, remaining))
                    .collect::<Result<_, _>>()?,
            ),
            VariantType::DICTIONARY => {
                let mut fields = BTreeMap::new();
                let dictionary = value
                    .try_to::<Dictionary>()
                    .map_err(|_| "typed dictionaries are not supported in requests")?;
                for (key, value) in dictionary.iter_shared() {
                    if key.get_type() != VariantType::STRING {
                        return Err("dictionary keys must be strings");
                    }
                    fields.insert(
                        key.to::<GString>().to_string(),
                        Self::from_variant(&value, depth + 1, remaining)?,
                    );
                }
                Self::Object(fields)
            }
            _ => return Err("unsupported Variant in request"),
        })
    }
}

pub(crate) fn entity(bits: u64, generation: u32) -> Wire {
    Wire::object([
        ("bits", Wire::String(bits.to_string())),
        ("generation", Wire::Integer(i64::from(generation))),
    ])
}

impl From<&Value> for Wire {
    fn from(value: &Value) -> Self {
        let writable = match &value.writable {
            Writable::Yes => Self::object([("allowed", Self::Bool(true))]),
            Writable::No(reason) => Self::object([
                ("allowed", Self::Bool(false)),
                ("reason", Self::String(reason.as_str().into())),
            ]),
        };
        let mut fields = BTreeMap::from([
            ("type_path".into(), Self::String(value.type_path.clone())),
            ("writable".into(), writable),
        ]);
        if let Some(range) = value.range {
            fields.insert(
                "range".into(),
                Self::object([
                    ("min", Self::Float(range.min)),
                    ("max", Self::Float(range.max)),
                ]),
            );
        }
        let mut set = |key: &str, value| {
            fields.insert(key.into(), value);
        };
        let kind = match &value.kind {
            Kind::Scalar(scalar) => {
                let (kind, value) = match scalar {
                    Scalar::Bool(value) => ("bool", Self::Bool(*value)),
                    Scalar::Integer(value) => ("integer", Self::Integer(*value)),
                    Scalar::Decimal(value) => ("integer", Self::String(value.clone())),
                    Scalar::Float(value) => ("float", Self::Float(*value)),
                    Scalar::String(value) => ("string", Self::String(value.clone())),
                    Scalar::Char(value) => ("char", Self::String(value.to_string())),
                };
                set("value", value);
                kind
            }
            Kind::Struct(children) => {
                set("fields", named_fields(children));
                "struct"
            }
            Kind::TupleStruct(children) | Kind::Tuple(children) => {
                set("fields", values(children));
                if matches!(value.kind, Kind::TupleStruct(_)) {
                    "tuple_struct"
                } else {
                    "tuple"
                }
            }
            Kind::List { items, truncated } | Kind::Array { items, truncated } => {
                set("items", values(items));
                set("truncated", Self::Integer(*truncated as i64));
                if matches!(value.kind, Kind::List { .. }) {
                    "list"
                } else {
                    "array"
                }
            }
            Kind::Map { entries, truncated } => {
                set(
                    "entries",
                    Self::Array(
                        entries
                            .iter()
                            .map(|(key, value)| {
                                Self::object([
                                    ("key", Self::from(key)),
                                    ("value", Self::from(value)),
                                ])
                            })
                            .collect(),
                    ),
                );
                set("truncated", Self::Integer(*truncated as i64));
                "map"
            }
            Kind::Set { members, truncated } => {
                set("members", values(members));
                set("truncated", Self::Integer(*truncated as i64));
                "set"
            }
            Kind::Enum {
                variant,
                fields,
                unit_variants,
            } => {
                set("variant", Self::String(variant.clone()));
                set("fields", named_fields(fields));
                set(
                    "unit_variants",
                    Self::Array(unit_variants.iter().cloned().map(Self::String).collect()),
                );
                "enum"
            }
            Kind::Entity { bits, generation } => {
                set("entity", entity(*bits, *generation));
                "entity"
            }
            Kind::Node { instance_id, valid } => {
                set("instance_id", Self::String(instance_id.to_string()));
                set("valid", Self::Bool(*valid));
                "node"
            }
            Kind::Asset { id } => {
                set("id", Self::String(id.clone()));
                "asset"
            }
            Kind::Opaque(debug) => {
                set("debug", Self::String(debug.clone()));
                "opaque"
            }
            Kind::Unsupported(reason) => {
                set("reason", Self::String(reason.clone()));
                "unsupported"
            }
            Kind::DepthLimit => {
                set("reason", Self::String("maximum depth reached".into()));
                "depth_limit"
            }
        };
        fields.insert("kind".into(), Self::String(kind.into()));
        Self::Object(fields)
    }
}

fn named_fields(fields: &[Field]) -> Wire {
    Wire::Array(
        fields
            .iter()
            .map(|field| {
                Wire::object([
                    ("name", Wire::String(field.name.clone())),
                    ("value", Wire::from(&field.value)),
                ])
            })
            .collect(),
    )
}

fn values(values: &[Value]) -> Wire {
    Wire::Array(values.iter().map(Wire::from).collect())
}
