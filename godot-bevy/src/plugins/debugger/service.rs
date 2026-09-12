use std::{
    collections::{BTreeMap, HashSet},
    time::Instant,
};

use bevy_ecs::{prelude::*, reflect::AppTypeRegistry, world::EntityRef};
use godot::{
    classes::{Engine, Node, SceneTree},
    obj::{Gd, Singleton},
};

use super::{
    DebuggerConfig, DebuggerTransport, Wire,
    edit::{Edit, InspectionError, PathSegment, mutate_component, read_component},
    value::{Kind, ReadOnlyReason, Scalar, Value, Writable},
    wire,
};
use crate::{
    interop::GodotNodeHandle,
    plugins::scene_tree::{GodotChildOf, NodeEntityIndex},
};

const METHODS: [&str; 7] = [
    "rpc.discover",
    "godot.subscribe",
    "godot.unsubscribe",
    "godot.query",
    "godot.get_components",
    "godot.mutate_leaf",
    "godot.resolve_node",
];

#[derive(Debug)]
pub(super) struct RpcError {
    code: i64,
    message: &'static str,
}

impl RpcError {
    pub(super) fn invalid(message: &'static str) -> Self {
        Self {
            code: -32600,
            message,
        }
    }
    fn params(message: &'static str) -> Self {
        Self {
            code: -32602,
            message,
        }
    }
    pub(super) fn to_wire(&self) -> Wire {
        Wire::object([
            ("code", Wire::Integer(self.code)),
            ("message", Wire::String(self.message.into())),
            (
                "data",
                Wire::object([("reason", Wire::String(self.message.into()))]),
            ),
        ])
    }
}

impl From<InspectionError> for RpcError {
    fn from(error: InspectionError) -> Self {
        Self {
            code: -32000,
            message: error.0,
        }
    }
}

#[derive(Clone)]
struct Summary {
    entity: Entity,
    name: String,
    parent: Option<Entity>,
    has_node: bool,
    components: Vec<String>,
    unsupported_components: BTreeMap<String, Wire>,
}

impl Summary {
    fn to_wire(&self) -> Wire {
        Wire::object([
            ("entity", reference(self.entity)),
            ("name", Wire::String(self.name.clone())),
            ("parent", self.parent.map(reference).unwrap_or(Wire::Null)),
            ("has_node", Wire::Bool(self.has_node)),
            (
                "components",
                Wire::Array(self.components.iter().cloned().map(Wire::String).collect()),
            ),
            (
                "unsupported_components",
                Wire::Object(self.unsupported_components.clone()),
            ),
        ])
    }
}

pub(super) struct Subscription {
    interval_s: f64,
    sent_at: Instant,
    initial: bool,
    summaries: BTreeMap<Entity, Summary>,
}

#[derive(Resource, Default)]
pub(super) struct SummaryChanges {
    active: bool,
    dirty: HashSet<Entity>,
}

pub(super) fn track_summary_changes(
    query: Query<Entity, Or<(Changed<Name>, Changed<GodotChildOf>)>>,
    mut removed_names: RemovedComponents<Name>,
    mut removed_parents: RemovedComponents<GodotChildOf>,
    config: Res<DebuggerConfig>,
    mut changes: ResMut<SummaryChanges>,
) {
    if !config.enabled {
        *changes = SummaryChanges::default();
        return;
    }
    if changes.active {
        changes.dirty.extend(query.iter());
        changes.dirty.extend(removed_names.read());
        changes.dirty.extend(removed_parents.read());
    }
}

fn summary(world: &World, entity: Entity) -> Option<Summary> {
    let entity = world.get_entity(entity).ok()?;
    let registry = world
        .get_resource::<AppTypeRegistry>()
        .map(|registry| registry.read());
    let mut components = Vec::new();
    let mut unsupported_components = BTreeMap::new();
    for id in entity.archetype().components() {
        let Some(info) = world.components().get_info(*id) else {
            continue;
        };
        let registration = info.type_id().and_then(|id| registry.as_ref()?.get(id));
        let name = registration
            .map(|registration| registration.type_info().type_path().to_string())
            .unwrap_or_else(|| info.name().to_string());
        if registration.is_none() && name != std::any::type_name::<GodotNodeHandle>() {
            unsupported_components.insert(
                name.clone(),
                Wire::String("component not registered".into()),
            );
        }
        components.push(name);
    }
    components.sort();
    Some(Summary {
        entity: entity.id(),
        name: entity
            .get::<Name>()
            .map(|name| name.as_str().into())
            .unwrap_or_default(),
        parent: entity.get::<GodotChildOf>().map(GodotChildOf::get),
        has_node: entity.contains::<GodotNodeHandle>(),
        components,
        unsupported_components,
    })
}

fn summaries(world: &mut World) -> BTreeMap<Entity, Summary> {
    let entities = world
        .query::<EntityRef>()
        .iter(world)
        .map(|entity| entity.id())
        .collect::<Vec<_>>();
    entities
        .into_iter()
        .filter_map(|entity| Some((entity, summary(world, entity)?)))
        .collect()
}

pub(super) fn publish(world: &mut World, subscription: &mut Option<Subscription>) {
    if !world.resource::<DebuggerConfig>().enabled {
        *subscription = None;
        *world.resource_mut::<SummaryChanges>() = SummaryChanges::default();
        return;
    }
    let Some(subscription) = subscription else {
        return;
    };
    if subscription.initial {
        subscription.initial = false;
        subscription.sent_at = Instant::now();
        notify(
            world,
            Wire::object([
                ("snapshot", Wire::Bool(true)),
                (
                    "added",
                    Wire::Array(
                        subscription
                            .summaries
                            .values()
                            .map(Summary::to_wire)
                            .collect(),
                    ),
                ),
                ("removed", Wire::Array(Vec::new())),
                ("updated", Wire::Array(Vec::new())),
            ]),
        );
        return;
    }
    if subscription.sent_at.elapsed().as_secs_f64() < subscription.interval_s {
        return;
    }
    let live = world
        .query::<EntityRef>()
        .iter(world)
        .map(|entity| entity.id())
        .collect::<HashSet<_>>();
    let known = subscription
        .summaries
        .keys()
        .copied()
        .collect::<HashSet<_>>();
    let mut dirty = std::mem::take(&mut world.resource_mut::<SummaryChanges>().dirty);
    dirty.extend(live.symmetric_difference(&known).copied());
    let mut dirty = dirty.into_iter().collect::<Vec<_>>();
    dirty.sort();
    let (mut added, mut removed, mut updated) = (Vec::new(), Vec::new(), Vec::new());
    for entity in dirty {
        match summary(world, entity) {
            Some(current) => {
                match subscription.summaries.get(&entity) {
                    None => added.push(current.to_wire()),
                    Some(previous)
                        if previous.name != current.name || previous.parent != current.parent =>
                    {
                        updated.push(current.to_wire())
                    }
                    Some(_) => {}
                }
                subscription.summaries.insert(entity, current);
            }
            None => {
                if subscription.summaries.remove(&entity).is_some() {
                    removed.push(reference(entity));
                }
            }
        }
    }
    subscription.sent_at = Instant::now();
    if !added.is_empty() || !removed.is_empty() || !updated.is_empty() {
        notify(
            world,
            Wire::object([
                ("snapshot", Wire::Bool(false)),
                ("added", Wire::Array(added)),
                ("removed", Wire::Array(removed)),
                ("updated", Wire::Array(updated)),
            ]),
        );
    }
}

fn notify(world: &World, params: Wire) {
    world.resource::<DebuggerTransport>().send(Wire::object([
        ("jsonrpc", Wire::String("2.0".into())),
        ("method", Wire::String("godot.summary".into())),
        ("params", params),
    ]));
}

pub(super) fn request_method(request: &Wire) -> Result<&str, RpcError> {
    if request.get("jsonrpc").and_then(Wire::as_str) != Some("2.0")
        || request.get("result").is_some()
        || request.get("error").is_some()
    {
        return Err(RpcError::invalid("expected a JSON-RPC 2.0 request"));
    }
    request
        .get("method")
        .and_then(Wire::as_str)
        .ok_or_else(|| RpcError::invalid("method must be a string"))
}

pub(super) fn dispatch(
    world: &mut World,
    subscription: &mut Option<Subscription>,
    request: &Wire,
) -> Result<Wire, RpcError> {
    let method = request_method(request)?;
    let empty = Wire::object([]);
    let params = match request.get("params") {
        None | Some(Wire::Null) => &empty,
        Some(params @ Wire::Object(_)) => params,
        _ => return Err(RpcError::params("params must be a Dictionary")),
    };
    if !world.resource::<DebuggerConfig>().enabled && method != "godot.unsubscribe" {
        return Err(InspectionError("debugger disabled").into());
    }
    match method {
        "rpc.discover" => Ok(discover()),
        "godot.subscribe" => {
            let interval_s = match params.get("interval_s") {
                Some(Wire::Float(value)) => *value,
                Some(Wire::Integer(value)) => *value as f64,
                _ => {
                    return Err(RpcError::params(
                        "interval_s must be a finite nonnegative number",
                    ));
                }
            };
            if !interval_s.is_finite() || interval_s < 0.0 {
                return Err(RpcError::params(
                    "interval_s must be a finite nonnegative number",
                ));
            }
            *subscription = Some(Subscription {
                interval_s,
                sent_at: Instant::now(),
                initial: true,
                summaries: summaries(world),
            });
            let mut changes = world.resource_mut::<SummaryChanges>();
            changes.active = true;
            changes.dirty.clear();
            Ok(Wire::object([("subscribed", Wire::Bool(true))]))
        }
        "godot.unsubscribe" => {
            *subscription = None;
            *world.resource_mut::<SummaryChanges>() = SummaryChanges::default();
            Ok(Wire::object([("subscribed", Wire::Bool(false))]))
        }
        "godot.query" => query(world, params),
        "godot.get_components" => get_components(world, params),
        "godot.mutate_leaf" => mutate(world, params),
        "godot.resolve_node" => resolve(world, params),
        _ => Err(RpcError {
            code: -32601,
            message: "method not found",
        }),
    }
}

fn string<'a>(params: &'a Wire, key: &str) -> Result<&'a str, RpcError> {
    params
        .get(key)
        .and_then(Wire::as_str)
        .ok_or_else(|| RpcError::params("missing or invalid string parameter"))
}

fn optional_string<'a>(params: &'a Wire, key: &str) -> Result<Option<&'a str>, RpcError> {
    params
        .get(key)
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| RpcError::params("invalid string parameter"))
        })
        .transpose()
}

fn index(params: &Wire, key: &str) -> Result<usize, RpcError> {
    match params.get(key) {
        Some(Wire::Integer(value)) => {
            usize::try_from(*value).map_err(|_| RpcError::params("index must be nonnegative"))
        }
        _ => Err(RpcError::params("missing or invalid integer parameter")),
    }
}

fn target(params: &Wire) -> Result<Entity, RpcError> {
    let value = params
        .get("entity")
        .ok_or_else(|| RpcError::params("entity reference required"))?;
    let bits = string(value, "bits")?
        .parse::<u64>()
        .map_err(|_| RpcError::params("invalid entity bits"))?;
    let entity =
        Entity::try_from_bits(bits).ok_or_else(|| RpcError::params("invalid entity bits"))?;
    if index(value, "generation")? != entity.generation().to_bits() as usize {
        return Err(InspectionError("stale entity").into());
    }
    Ok(entity)
}

fn reference(entity: Entity) -> Wire {
    wire::entity(entity.to_bits(), entity.generation().to_bits())
}

fn query(world: &mut World, params: &Wire) -> Result<Wire, RpcError> {
    let page = index(params, "page")?;
    let page_size = index(params, "page_size")?;
    if page_size == 0 || page_size > 256 {
        return Err(RpcError::params("page_size must be between 1 and 256"));
    }
    let offset = page
        .checked_mul(page_size)
        .ok_or_else(|| RpcError::params("page offset overflow"))?;
    let name = optional_string(params, "name_contains")?;
    let component = optional_string(params, "component")?;
    let path = optional_string(params, "node_path")?;
    let matches = summaries(world)
        .into_values()
        .filter(|summary| {
            name.is_none_or(|name| summary.name.contains(name))
                && component.is_none_or(|component| {
                    summary.components.iter().any(|value| value == component)
                })
                && path.is_none_or(|path| {
                    world
                        .get::<GodotNodeHandle>(summary.entity)
                        .and_then(|handle| {
                            Gd::<Node>::try_from_instance_id(handle.instance_id()).ok()
                        })
                        .is_some_and(|node| {
                            node.is_inside_tree() && node.get_path().to_string() == path
                        })
                })
        })
        .collect::<Vec<_>>();
    Ok(Wire::object([
        ("total", Wire::Integer(matches.len() as i64)),
        ("page", Wire::Integer(page as i64)),
        ("page_size", Wire::Integer(page_size as i64)),
        (
            "entities",
            Wire::Array(
                matches
                    .iter()
                    .skip(offset)
                    .take(page_size)
                    .map(Summary::to_wire)
                    .collect(),
            ),
        ),
    ]))
}

fn get_components(world: &World, params: &Wire) -> Result<Wire, RpcError> {
    let entity = target(params)?;
    let summary = summary(world, entity).ok_or(InspectionError("stale entity"))?;
    let components = match params.get("components") {
        None => summary.components,
        Some(Wire::Array(components)) => components
            .iter()
            .map(|component| {
                component
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| RpcError::params("components must contain type paths"))
            })
            .collect::<Result<_, _>>()?,
        _ => return Err(RpcError::params("components must be an array")),
    };
    let limits = &world.resource::<DebuggerConfig>().value_limits;
    let mut values = BTreeMap::new();
    for component in components {
        let value = if component == std::any::type_name::<GodotNodeHandle>() {
            let handle = world
                .get::<GodotNodeHandle>(entity)
                .ok_or(InspectionError("component absent"))?;
            Value {
                type_path: component.clone(),
                kind: Kind::Node {
                    instance_id: handle.instance_id().to_i64() as u64,
                    valid: Gd::<Node>::try_from_instance_id(handle.instance_id()).is_ok(),
                },
                writable: Writable::No(ReadOnlyReason::Reference),
                range: None,
            }
        } else {
            read_component(world, entity, &component, limits)?
        };
        values.insert(component, Wire::from(&value));
    }
    Ok(Wire::Object(values))
}

fn mutate(world: &mut World, params: &Wire) -> Result<Wire, RpcError> {
    let entity = target(params)?;
    let entity_ref = world
        .get_entity(entity)
        .map_err(|_| InspectionError("stale entity"))?;
    if let Some(handle) = entity_ref.get::<GodotNodeHandle>() {
        let node = Gd::<Node>::try_from_instance_id(handle.instance_id())
            .map_err(|_| InspectionError("pending despawn"))?;
        if pending_deletion(node) {
            return Err(InspectionError("pending despawn").into());
        }
    }
    let component = string(params, "component")?;
    if component == std::any::type_name::<GodotNodeHandle>() {
        return Err(InspectionError(ReadOnlyReason::Reference.as_str()).into());
    }
    let path = params.get("path").and_then(Wire::as_array).ok_or_else(|| {
        RpcError::params("path must be an array of field, index or variant segments")
    })?;
    let path = path
        .iter()
        .map(|segment| {
            let Wire::Object(fields) = segment else {
                return Err(RpcError::params("invalid path segment"));
            };
            if fields.len() != 1 {
                return Err(RpcError::params("invalid path segment"));
            }
            if fields.contains_key("field") {
                Ok(PathSegment::Field(string(segment, "field")?.into()))
            } else if fields.contains_key("index") {
                Ok(PathSegment::Index(index(segment, "index")?))
            } else if fields.contains_key("variant") {
                Ok(PathSegment::Variant(string(segment, "variant")?.into()))
            } else {
                Err(RpcError::params("invalid path segment"))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let edit = match params.get("value") {
        Some(Wire::Bool(value)) => Edit::Scalar(Scalar::Bool(*value)),
        Some(Wire::Integer(value)) => Edit::Scalar(Scalar::Integer(*value)),
        Some(Wire::Float(value)) => Edit::Scalar(Scalar::Float(*value)),
        Some(Wire::String(value)) => Edit::Scalar(Scalar::String(value.clone())),
        Some(value @ Wire::Object(fields)) if fields.len() == 1 => {
            Edit::Variant(string(value, "variant")?.into())
        }
        _ => return Err(RpcError::params("decode failure")),
    };
    let limits = world.resource::<DebuggerConfig>().value_limits;
    Ok(Wire::from(&mutate_component(
        world, entity, component, &path, &edit, &limits,
    )?))
}

fn pending_deletion(node: Gd<Node>) -> bool {
    let mut node = Some(node);
    while let Some(current) = node {
        if current.is_queued_for_deletion() || !current.is_inside_tree() {
            return true;
        }
        node = current.get_parent();
    }
    false
}

fn resolve(world: &World, params: &Wire) -> Result<Wire, RpcError> {
    let scene_path = string(params, "scene_path")?;
    let node_path = string(params, "node_path")?;
    if scene_path.is_empty()
        || node_path.is_empty()
        || node_path.starts_with('/')
        || node_path.contains(':')
        || node_path.split('/').any(|segment| segment == "..")
    {
        return Err(RpcError::params(
            "expected an authored scene and relative node path",
        ));
    }
    let tree = Engine::singleton()
        .get_main_loop()
        .and_then(|node| node.try_cast::<SceneTree>().ok())
        .ok_or(InspectionError("scene tree unavailable"))?;
    let root = tree
        .get_root()
        .ok_or(InspectionError("scene tree unavailable"))?;
    let index = world
        .get_resource::<NodeEntityIndex>()
        .ok_or(InspectionError("node index unavailable"))?;
    let mut stack = vec![root.upcast::<Node>()];
    let mut candidates = Vec::new();
    while let Some(node) = stack.pop() {
        if node.get_scene_file_path() == scene_path
            && let Some(target) = node.try_get_node_as::<Node>(node_path)
            && !pending_deletion(target.clone())
            && let Some(entity) = index.get(target.instance_id())
            && world
                .get::<GodotNodeHandle>(entity)
                .is_some_and(|handle| handle.instance_id() == target.instance_id())
        {
            candidates.push(entity);
        }
        stack.extend(node.get_children().iter_shared());
    }
    candidates.sort();
    candidates.dedup();
    Ok(Wire::object([
        (
            "candidates",
            Wire::Array(candidates.into_iter().map(reference).collect()),
        ),
        ("reason", Wire::Null),
    ]))
}

fn discover() -> Wire {
    Wire::object([
        ("openrpc", Wire::String("1.2.6".into())),
        (
            "info",
            Wire::object([
                (
                    "title",
                    Wire::String("godot-bevy inspection service".into()),
                ),
                ("version", Wire::String(env!("CARGO_PKG_VERSION").into())),
            ]),
        ),
        (
            "methods",
            Wire::Array(
                METHODS
                    .iter()
                    .map(|name| {
                        Wire::object([
                            ("name", Wire::String((*name).into())),
                            ("params", method_params(name)),
                            (
                                "result",
                                Wire::object([
                                    ("name", Wire::String("result".into())),
                                    ("schema", Wire::object([])),
                                ]),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

fn method_params(name: &str) -> Wire {
    let params: &[(&str, &str, bool)] = match name {
        "godot.subscribe" => &[("interval_s", "number", true)],
        "godot.query" => &[
            ("name_contains", "string", false),
            ("component", "string", false),
            ("node_path", "string", false),
            ("page", "integer", true),
            ("page_size", "integer", true),
        ],
        "godot.get_components" => &[("entity", "object", true), ("components", "array", false)],
        "godot.mutate_leaf" => &[
            ("entity", "object", true),
            ("component", "string", true),
            ("path", "array", true),
            ("value", "", true),
        ],
        "godot.resolve_node" => &[
            ("scene_path", "string", true),
            ("node_path", "string", true),
        ],
        _ => &[],
    };
    Wire::Array(
        params
            .iter()
            .map(|(name, kind, required)| {
                Wire::object([
                    ("name", Wire::String((*name).into())),
                    ("required", Wire::Bool(*required)),
                    (
                        "schema",
                        if kind.is_empty() {
                            Wire::object([])
                        } else {
                            Wire::object([("type", Wire::String((*kind).into()))])
                        },
                    ),
                ])
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
