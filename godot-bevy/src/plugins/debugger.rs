//! Bevy Entity Debugger Plugin
//!
//! This plugin integrates with Godot's EditorDebuggerPlugin system to provide
//! real-time inspection of Bevy entities and components in the Godot editor.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::{Name, Resource, World};
use bevy_ecs::world::EntityRef;
use bevy_time::Time;
use godot::classes::EngineDebugger;
use godot::prelude::*;

use crate::interop::GodotNodeHandle;

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

/// Exclusive system that collects and sends entity data
/// Uses exclusive world access to get component information from archetypes
fn debugger_exclusive_system(world: &mut World) {
    // Get config and check if enabled
    let config = world.get_resource::<DebuggerConfig>();
    let enabled = config.map(|c| c.enabled).unwrap_or(false);
    let update_interval = config.map(|c| c.update_interval).unwrap_or(0.5);

    if !enabled {
        return;
    }

    // Check timer
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

    // Check if debugger is active (this is a Godot call, should be on main thread)
    // In practice, the Update schedule runs on main thread for godot-bevy
    if !EngineDebugger::singleton().is_active() {
        return;
    }

    // Collect entity data with component names
    let mut entities = VarArray::new();

    let mut query = world.query::<EntityRef>();
    for entity_ref in query.iter(world) {
        // Get name if present
        let name = entity_ref
            .get::<Name>()
            .map(|n| n.as_str().to_string())
            .unwrap_or_default();

        // Check for GodotNodeHandle
        let has_godot_node = entity_ref.get::<GodotNodeHandle>().is_some();

        // Get component names from archetype
        let mut components = VarArray::new();
        let archetype = entity_ref.archetype();
        for component_id in archetype.components() {
            if let Some(component_info) = world.components().get_info(*component_id) {
                let component_name = component_info.name().to_string();
                components.push(&Variant::from(GString::from(&component_name)));
            }
        }

        // Create array: [entity_bits, name, has_godot_node, components]
        let mut entry = VarArray::new();
        entry.push(&Variant::from(entity_ref.id().to_bits() as i64));
        entry.push(&Variant::from(GString::from(&name)));
        entry.push(&Variant::from(has_godot_node));
        entry.push(&components.to_variant());

        entities.push(&entry.to_variant());
    }

    // Send to debugger
    let mut debugger = EngineDebugger::singleton();
    let message: GString = "bevy:entities".into();
    debugger.send_message(&message, &entities);
}
