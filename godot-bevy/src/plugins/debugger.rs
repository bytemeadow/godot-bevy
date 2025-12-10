//! Bevy Entity Debugger Plugin
//!
//! This plugin integrates with Godot's EditorDebuggerPlugin system to provide
//! real-time inspection of Bevy entities and components in the Godot editor.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Local, Name, Query, Res, Resource};
use bevy_reflect::Reflect;
use bevy_time::Time;
use godot::classes::EngineDebugger;
use godot::prelude::*;

use crate::interop::GodotNodeHandle;
use crate::prelude::main_thread_system;

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

/// Plugin that enables Bevy entity inspection in Godot's debugger
#[derive(Default)]
pub struct GodotDebuggerPlugin;

impl Plugin for GodotDebuggerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebuggerConfig>()
            .add_systems(Update, debugger_update_system);
    }
}

/// System that periodically sends entity data to the editor
#[main_thread_system]
fn debugger_update_system(
    config: Res<DebuggerConfig>,
    time: Res<Time>,
    mut elapsed: Local<f32>,
    entities_query: Query<(Entity, Option<&Name>, Option<&GodotNodeHandle>)>,
) {
    // Check if debugger is active
    if !EngineDebugger::singleton().is_active() {
        return;
    }

    if !config.enabled {
        return;
    }

    // Update timer
    *elapsed += time.delta_secs();

    if *elapsed < config.update_interval {
        return;
    }
    *elapsed = 0.0;

    // Collect entity data
    let entity_data = collect_entities(&entities_query);

    // Send to debugger
    let mut debugger = EngineDebugger::singleton();
    let message: GString = "bevy:entities".into();
    debugger.send_message(&message, &entity_data);
}

/// Collect all entities and their basic info
fn collect_entities(
    entities_query: &Query<(Entity, Option<&Name>, Option<&GodotNodeHandle>)>,
) -> VarArray {
    let mut entities = VarArray::new();

    for (entity, name, godot_handle) in entities_query.iter() {
        let entity_bits = entity.to_bits() as i64;

        // Try to get entity name
        let name_str = name.map(|n| n.as_str().to_string()).unwrap_or_default();

        // Check if entity has a GodotNodeHandle
        let has_godot_node = godot_handle.is_some();

        // Create array: [entity_bits, name, has_godot_node]
        let mut entry = VarArray::new();
        entry.push(&Variant::from(entity_bits));
        entry.push(&Variant::from(GString::from(&name_str)));
        entry.push(&Variant::from(has_godot_node));

        entities.push(&entry.to_variant());
    }

    entities
}

/// Format a reflected value as a string
#[allow(dead_code)]
fn format_reflect_value(value: &dyn Reflect) -> String {
    // Use debug formatting which leverages the Reflect trait's debug impl
    format!("{value:?}")
}
