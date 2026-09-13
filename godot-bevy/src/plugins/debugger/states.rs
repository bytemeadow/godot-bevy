use std::{any::TypeId, collections::HashMap};

use bevy_ecs::{prelude::*, reflect::ReflectComponent};
use bevy_reflect::{
    PartialReflect, ReflectFromReflect, ReflectRef, TypeInfo, TypeRegistration, TypeRegistry,
    enums::{DynamicEnum, DynamicVariant, VariantInfo},
};
use bevy_state::reflect::{ReflectFreelyMutableState, ReflectState};

use super::{
    DebuggerConfig, Wire,
    edit::InspectionError,
    service::RpcError,
    value::{ReadOnlyReason, Writable, inspect},
    wire,
};

const REGISTRATION: &str = "Register states with App::register_type_mutable_state::<S>() or App::register_type_state::<S>()";

#[derive(Resource, Default)]
struct Receipts(HashMap<TypeId, Receipt>);

struct Receipt {
    state_entity: Wire,
    next_state_entity: Wire,
    target: String,
    terminal: Option<String>,
}

struct Backing<'a> {
    entity: Option<Entity>,
    registration: &'a TypeRegistration,
    present: bool,
}

fn backing<'a>(
    world: &World,
    registry: &'a TypeRegistry,
    state: TypeId,
    wrapper: &str,
) -> Option<Backing<'a>> {
    let registration = registry.iter().find(|registration| {
        let info = registration.type_info();
        info.type_path_table().module_path() == Some("bevy_state::state::resources")
            && info.type_path_table().ident() == Some(wrapper)
            && info
                .generics()
                .get_named("S")
                .is_some_and(|parameter| parameter.type_id() == state)
    })?;
    let component = world.components().get_id(registration.type_id());
    let entity = component.and_then(|component| world.resource_entities().get(component));
    Some(Backing {
        entity,
        registration,
        present: entity.zip(component).is_some_and(|(entity, component)| {
            world
                .get_entity(entity)
                .is_ok_and(|entity| entity.contains_id(component))
        }),
    })
}

fn identity(backing: &Option<Backing<'_>>) -> Wire {
    backing
        .as_ref()
        .and_then(|backing| backing.entity)
        .map_or(Wire::Null, |entity| {
            wire::entity(entity.to_bits(), entity.generation().to_bits())
        })
}

fn unit_variants(registration: &TypeRegistration) -> Vec<Wire> {
    match registration.type_info() {
        TypeInfo::Enum(info) => info
            .iter()
            .filter_map(|variant| match variant {
                VariantInfo::Unit(info) => Some(Wire::String(info.name().into())),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

fn describe(
    world: &World,
    registry: &TypeRegistry,
    registration: &TypeRegistration,
) -> (Wire, Option<&'static str>) {
    let state = backing(world, registry, registration.type_id(), "State");
    let next = backing(world, registry, registration.type_id(), "NextState");
    let present = state.as_ref().is_some_and(|backing| backing.present);
    let next_present = next.as_ref().is_some_and(|backing| backing.present);
    let queued_reason = match &next {
        None => Some("NextState reflection not registered"),
        Some(next) if next.registration.data::<ReflectComponent>().is_none() => {
            Some("NextState reflection unavailable")
        }
        Some(_) => None,
    };
    let mutable = registration.data::<ReflectFreelyMutableState>().is_some();
    let variants = unit_variants(registration);
    let reason = if !present {
        Some("State resource absent")
    } else if !mutable {
        Some("state is registered read-only")
    } else if let Some(reason) = queued_reason {
        Some(reason)
    } else if !next_present {
        Some("NextState resource absent")
    } else if variants.is_empty() {
        Some("state has no fieldless enum variants")
    } else {
        None
    };
    (
        Wire::object([
            (
                "type_path",
                Wire::String(registration.type_info().type_path().into()),
            ),
            ("state_entity", identity(&state)),
            ("next_state_entity", identity(&next)),
            ("present", Wire::Bool(present)),
            (
                "next_present",
                if next.is_some() {
                    Wire::Bool(next_present)
                } else {
                    Wire::Null
                },
            ),
            ("queued_available", Wire::Bool(queued_reason.is_none())),
            (
                "queued_reason",
                queued_reason.map_or(Wire::Null, |reason| Wire::String(reason.into())),
            ),
            ("mutable", Wire::Bool(mutable)),
            ("can_request", Wire::Bool(reason.is_none())),
            (
                "reason",
                reason.map_or(Wire::Null, |reason| Wire::String(reason.into())),
            ),
            ("unit_variants", Wire::Array(variants)),
        ]),
        reason,
    )
}

pub(super) fn list(world: &World) -> Wire {
    let mut states = Vec::new();
    if let Some(registry) = world.get_resource::<AppTypeRegistry>() {
        let registry = registry.read();
        let mut registrations = registry
            .iter()
            .filter(|registration| registration.data::<ReflectState>().is_some())
            .collect::<Vec<_>>();
        registrations.sort_by_key(|registration| registration.type_info().type_path());
        states = registrations
            .into_iter()
            .map(|registration| describe(world, &registry, registration).0)
            .collect();
    }
    let reason = if states.is_empty() {
        Wire::String(REGISTRATION.into())
    } else {
        Wire::Null
    };
    Wire::object([("states", Wire::Array(states)), ("reason", reason)])
}

pub(super) fn access(
    world: &mut World,
    params: &Wire,
    request_transition: bool,
) -> Result<Wire, RpcError> {
    let type_path = params
        .get("type_path")
        .and_then(Wire::as_str)
        .ok_or_else(|| RpcError::params("state type_path required"))?;
    let registry = world
        .get_resource::<AppTypeRegistry>()
        .ok_or(InspectionError(
            "state not registered with Bevy state reflection",
        ))?
        .clone();
    let registry = registry.read();
    let registration = registry
        .get_with_type_path(type_path)
        .filter(|registration| registration.data::<ReflectState>().is_some())
        .ok_or(InspectionError(
            "state not registered with Bevy state reflection",
        ))?;
    let (mut result, reason) = describe(world, &registry, registration);
    for key in ["state_entity", "next_state_entity"] {
        let expected = params
            .get(key)
            .ok_or_else(|| RpcError::params("state backing identities required"))?;
        if Some(expected) != result.get(key) {
            return Err(InspectionError("stale state backing identity").into());
        }
    }
    if request_transition {
        if let Some(reason) = reason {
            return Err(InspectionError(reason).into());
        }
        let variant = params
            .get("value")
            .and_then(|value| match value {
                Wire::Object(fields) if fields.len() == 1 => fields.get("variant"),
                _ => None,
            })
            .and_then(Wire::as_str)
            .ok_or_else(|| RpcError::params("fieldless state variant required"))?;
        let TypeInfo::Enum(info) = registration.type_info() else {
            return Err(InspectionError("state has no fieldless enum variants").into());
        };
        if !matches!(info.variant(variant), Some(VariantInfo::Unit(_))) {
            return Err(InspectionError("fieldless state variant required").into());
        }
        let current = registration
            .data::<ReflectState>()
            .unwrap()
            .reflect(world)
            .ok_or(InspectionError("State reflection unavailable"))?;
        let already_current = state_label(current.as_partial_reflect()) == variant;
        if !already_current {
            if queued(world, &registry, registration)?.is_some() {
                return Err(InspectionError("a state transition is already queued").into());
            }
            let mut value = DynamicEnum::new(variant, DynamicVariant::Unit);
            value.set_represented_type(Some(registration.type_info()));
            let value = registration
                .data::<ReflectFromReflect>()
                .and_then(|from| from.from_reflect(&value))
                .ok_or(InspectionError("decode failure"))?;
            registration
                .data::<ReflectFreelyMutableState>()
                .unwrap()
                .set_next_state(world, value.as_ref(), &registry);
        }
        world.init_resource::<Receipts>();
        world.resource_mut::<Receipts>().0.insert(
            registration.type_id(),
            Receipt {
                state_entity: result.get("state_entity").unwrap().clone(),
                next_state_entity: result.get("next_state_entity").unwrap().clone(),
                target: variant.into(),
                terminal: already_current.then(|| "Already current".into()),
            },
        );
        if already_current {
            return Err(InspectionError("Already current").into());
        }
    }
    let limits = &world.resource::<DebuggerConfig>().value_limits;
    let value = |value: &dyn PartialReflect| {
        Wire::from(&inspect(
            value,
            &registry,
            Writable::No(ReadOnlyReason::StateValue),
            limits,
        ))
    };
    let current = registration.data::<ReflectState>().unwrap().reflect(world);
    let next = queued(world, &registry, registration)?;
    let current_label = current.map(|current| state_label(current.as_partial_reflect()));
    let queued_label = next.map(state_label);
    let current = current
        .map(|current| value(current.as_partial_reflect()))
        .unwrap_or(Wire::Null);
    let queued = next.map(value).unwrap_or(Wire::Null);
    let receipt = sample_receipt(
        world,
        registration.type_id(),
        &result,
        current_label,
        queued_label,
    );
    if let Wire::Object(fields) = &mut result {
        fields.insert("current".into(), current);
        fields.insert("queued".into(), queued);
        fields.insert("receipt".into(), receipt);
    }
    Ok(result)
}

fn queued<'w>(
    world: &'w World,
    registry: &TypeRegistry,
    registration: &TypeRegistration,
) -> Result<Option<&'w dyn PartialReflect>, InspectionError> {
    let next = backing(world, registry, registration.type_id(), "NextState");
    match next.filter(|backing| {
        backing.present && backing.registration.data::<ReflectComponent>().is_some()
    }) {
        Some(next) => {
            let reflected = next
                .registration
                .data::<ReflectComponent>()
                .and_then(|adapter| adapter.reflect(world.entity(next.entity.unwrap())))
                .ok_or(InspectionError("NextState reflection unavailable"))?;
            let ReflectRef::Enum(next) = reflected.reflect_ref() else {
                return Err(InspectionError("unsupported NextState representation"));
            };
            match next.variant_name() {
                "Unchanged" => Ok(None),
                "Pending" | "PendingIfNeq" => Ok(Some(
                    next.field_at(0)
                        .ok_or(InspectionError("unsupported NextState representation"))?,
                )),
                _ => Err(InspectionError("unsupported NextState representation")),
            }
        }
        None => Ok(None),
    }
}

fn state_label(value: &dyn PartialReflect) -> String {
    match value.reflect_ref() {
        ReflectRef::Enum(value) => value.variant_name().into(),
        _ => value.reflect_type_path().into(),
    }
}

fn sample_receipt(
    world: &mut World,
    state: TypeId,
    descriptor: &Wire,
    current: Option<String>,
    queued: Option<String>,
) -> Wire {
    let Some(mut receipts) = world.get_resource_mut::<Receipts>() else {
        return Wire::Null;
    };
    let Some(receipt) = receipts.0.get_mut(&state) else {
        return Wire::Null;
    };
    if descriptor.get("state_entity") != Some(&receipt.state_entity)
        || descriptor.get("next_state_entity") != Some(&receipt.next_state_entity)
    {
        receipts.0.remove(&state);
        return Wire::Null;
    }
    if receipt.terminal.is_none() {
        if current.as_deref() == Some(receipt.target.as_str()) {
            receipt.terminal = Some(format!("Transition to {} observed", receipt.target));
        } else if descriptor.get("queued_available") == Some(&Wire::Bool(true))
            && descriptor.get("next_present") == Some(&Wire::Bool(true))
            && current.is_some()
        {
            receipt.terminal = match &queued {
                Some(queued) if queued != &receipt.target => {
                    Some(format!("Request replaced by {queued}"))
                }
                None => Some(format!(
                    "No longer pending; transition to {} not observed",
                    receipt.target
                )),
                _ => None,
            };
        }
    }
    Wire::String(receipt.terminal.clone().unwrap_or_else(|| match current {
        Some(current)
            if descriptor.get("queued_available") == Some(&Wire::Bool(true))
                && descriptor.get("next_present") == Some(&Wire::Bool(true)) =>
        {
            format!("Queued: {}; current: {}", receipt.target, current)
        }
        _ => "State or queue unavailable; request outcome unavailable".into(),
    }))
}

#[cfg(test)]
#[path = "states_tests.rs"]
mod tests;
