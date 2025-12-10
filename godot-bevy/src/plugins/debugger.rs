//! Bevy Entity Debugger Plugin
//!
//! This plugin integrates with Godot's EditorDebuggerPlugin system to provide
//! real-time inspection of Bevy entities and components in the Godot editor.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Name, Resource, World};
use bevy_ecs::world::EntityRef;
use bevy_reflect::{PartialReflect, ReflectFromPtr, ReflectRef};
use bevy_time::Time;
use godot::classes::EngineDebugger;
use godot::meta::ToGodot;
use godot::prelude::{VarDictionary as Dictionary, *};

use crate::interop::GodotNodeHandle;
use crate::plugins::reflection::AppTypeRegistry;

/// Configuration for the debugger plugin
#[derive(Resource)]
pub struct DebuggerConfig {
    /// Whether the debugger is enabled
    pub enabled: bool,
    /// How often to send entity updates (in seconds)
    pub update_interval: f32,
}

impl Default for DebuggerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            update_interval: 0.5, // Update twice per second
        }
    }
}

/// Timer resource for the debugger
#[derive(Resource, Default)]
struct DebuggerTimer {
    elapsed: f32,
}

/// Plugin that enables Bevy entity inspection in Godot's debugger
#[derive(Default)]
pub struct GodotDebuggerPlugin;

impl Plugin for GodotDebuggerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebuggerConfig>()
            .init_resource::<DebuggerTimer>()
            .add_systems(Update, debugger_exclusive_system);
    }
}

fn debugger_exclusive_system(world: &mut World) {
    let config = world.get_resource::<DebuggerConfig>();
    let enabled = config.map(|c| c.enabled).unwrap_or(false);
    let update_interval = config.map(|c| c.update_interval).unwrap_or(0.5);

    if !enabled {
        return;
    }

    let delta = world
        .get_resource::<Time>()
        .map(|t| t.delta_secs())
        .unwrap_or(0.0);

    let should_send = {
        let mut timer = world.get_resource_mut::<DebuggerTimer>();
        if let Some(ref mut timer) = timer {
            timer.elapsed += delta;
            if timer.elapsed < update_interval {
                false
            } else {
                timer.elapsed = 0.0;
                true
            }
        } else {
            false
        }
    };

    if !should_send {
        return;
    }

    if !EngineDebugger::singleton().is_active() {
        return;
    }

    // Clone registry so we can release the borrow on world
    let type_registry = world.get_resource::<AppTypeRegistry>().cloned();

    let mut entities = VarArray::new();
    let mut query = world.query::<EntityRef>();

    for entity_ref in query.iter(world) {
        let name = entity_ref
            .get::<Name>()
            .map(|n| n.as_str().to_string())
            .unwrap_or_default();

        let has_godot_node = entity_ref.get::<GodotNodeHandle>().is_some();

        let parent_bits: i64 = entity_ref
            .get::<ChildOf>()
            .map(|child_of| child_of.parent().to_bits() as i64)
            .unwrap_or(-1);

        // Build component data with reflection
        let mut components = VarArray::new();
        let archetype = entity_ref.archetype();

        for component_id in archetype.components() {
            let Some(component_info) = world.components().get_info(*component_id) else {
                continue;
            };

            let mut component_dict = Dictionary::new();

            // Try to get pretty type name from registry, fallback to full name
            let (full_name, short_name) = if let Some(ref registry) = type_registry {
                let registry = registry.read();
                if let Some(type_id) = component_info.type_id() {
                    if let Some(registration) = registry.get(type_id) {
                        let type_info = registration.type_info();
                        let table = type_info.type_path_table();
                        (table.path().to_string(), table.short_path().to_string())
                    } else {
                        let name = component_info.name().to_string();
                        (name.clone(), name)
                    }
                } else {
                    let name = component_info.name().to_string();
                    (name.clone(), name)
                }
            } else {
                let name = component_info.name().to_string();
                (name.clone(), name)
            };

            // Skip hierarchy components - already shown visually in the tree
            if short_name == "ChildOf"
                || short_name == "Children"
                || full_name.contains("::ChildOf")
                || full_name.contains("::Children")
            {
                continue;
            }

            component_dict.set("name", GString::from(&full_name));
            component_dict.set("short_name", GString::from(&short_name));

            // Try to get reflected value
            if let Some(ref registry) = type_registry {
                let registry = registry.read();
                if let Some(type_id) = component_info.type_id()
                    && let Some(registration) = registry.get(type_id)
                    && let Some(reflect_from_ptr) = registration.data::<ReflectFromPtr>()
                    && let Ok(ptr) = entity_ref.get_by_id(*component_id)
                {
                    // SAFETY: ptr is valid and type matches registration
                    let reflected = unsafe { reflect_from_ptr.as_reflect(ptr) };
                    let value_dict = reflect_to_dict(reflected);
                    component_dict.set("value", value_dict);
                }
            }

            components.push(&component_dict.to_variant());
        }

        let mut entry = VarArray::new();
        entry.push(&Variant::from(entity_ref.id().to_bits() as i64));
        entry.push(&Variant::from(GString::from(&name)));
        entry.push(&Variant::from(has_godot_node));
        entry.push(&Variant::from(parent_bits));
        entry.push(&components.to_variant());

        entities.push(&entry.to_variant());
    }

    let mut debugger = EngineDebugger::singleton();
    let message: GString = "bevy:entities".into();
    debugger.send_message(&message, &entities);
}

/// Convert a reflected value to a Godot Dictionary
fn reflect_to_dict(value: &dyn PartialReflect) -> Dictionary {
    let mut dict = Dictionary::new();

    match value.reflect_ref() {
        ReflectRef::Struct(s) => {
            dict.set("type", "struct");
            let mut fields = Dictionary::new();
            for i in 0..s.field_len() {
                if let Some(field_name) = s.name_at(i)
                    && let Some(field_value) = s.field_at(i)
                {
                    fields.set(field_name, reflect_value_to_variant(field_value));
                }
            }
            dict.set("fields", fields);
        }
        ReflectRef::TupleStruct(ts) => {
            dict.set("type", "tuple_struct");
            let mut fields = VarArray::new();
            for i in 0..ts.field_len() {
                if let Some(field_value) = ts.field(i) {
                    fields.push(&reflect_value_to_variant(field_value));
                }
            }
            dict.set("fields", fields);
        }
        ReflectRef::Tuple(t) => {
            dict.set("type", "tuple");
            let mut fields = VarArray::new();
            for i in 0..t.field_len() {
                if let Some(field_value) = t.field(i) {
                    fields.push(&reflect_value_to_variant(field_value));
                }
            }
            dict.set("fields", fields);
        }
        ReflectRef::List(l) => {
            dict.set("type", "list");
            let mut items = VarArray::new();
            for i in 0..l.len() {
                if let Some(item) = l.get(i) {
                    items.push(&reflect_value_to_variant(item));
                }
            }
            dict.set("items", items);
        }
        ReflectRef::Map(m) => {
            dict.set("type", "map");
            dict.set("len", m.len() as i64);
        }
        ReflectRef::Set(s) => {
            dict.set("type", "set");
            dict.set("len", s.len() as i64);
        }
        ReflectRef::Enum(e) => {
            dict.set("type", "enum");
            dict.set("variant", e.variant_name());
            let mut fields = Dictionary::new();
            for i in 0..e.field_len() {
                let field_name = e
                    .name_at(i)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| i.to_string());
                if let Some(field_value) = e.field_at(i) {
                    fields.set(field_name, reflect_value_to_variant(field_value));
                }
            }
            if !fields.is_empty() {
                dict.set("fields", fields);
            }
        }
        ReflectRef::Array(a) => {
            dict.set("type", "array");
            let mut items = VarArray::new();
            for i in 0..a.len() {
                if let Some(item) = a.get(i) {
                    items.push(&reflect_value_to_variant(item));
                }
            }
            dict.set("items", items);
        }
        ReflectRef::Opaque(_) => {
            dict.set("type", "opaque");
            // Try to get debug representation
            if let Some(debug_str) = value.try_as_reflect().map(|r| format!("{r:?}")) {
                dict.set("debug", GString::from(&debug_str));
            }
        }
    }

    dict
}

/// Convert a reflected value to a simple Godot Variant for display
fn reflect_value_to_variant(value: &dyn PartialReflect) -> Variant {
    // Try common primitive types first
    if let Some(v) = value.try_downcast_ref::<f32>() {
        return Variant::from(*v);
    }
    if let Some(v) = value.try_downcast_ref::<f64>() {
        return Variant::from(*v);
    }
    if let Some(v) = value.try_downcast_ref::<i32>() {
        return Variant::from(*v);
    }
    if let Some(v) = value.try_downcast_ref::<i64>() {
        return Variant::from(*v);
    }
    if let Some(v) = value.try_downcast_ref::<u32>() {
        return Variant::from(*v as i64);
    }
    if let Some(v) = value.try_downcast_ref::<u64>() {
        return Variant::from(*v as i64);
    }
    if let Some(v) = value.try_downcast_ref::<bool>() {
        return Variant::from(*v);
    }
    if let Some(v) = value.try_downcast_ref::<String>() {
        return Variant::from(GString::from(v));
    }
    if let Some(v) = value.try_downcast_ref::<&str>() {
        return Variant::from(GString::from(*v));
    }

    // For complex types, recurse
    reflect_to_dict(value).to_variant()
}
