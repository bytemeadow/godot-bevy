use std::time::Instant;

use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        schedule::BoxedCondition,
        system::IntoSystem,
        world::World,
    },
    log::info,
    prelude::Condition,
};
use godot::{classes::Tree, prelude::*};

use crate::plugins::assets::GodotResource;
use super::{
    ui::create_world_inspector_window, 
    utils::{count_resources, get_entity_component_names, guess_entity_name},
    WorldInspectorState
};

/// Plugin displaying a Godot window with an entity list, resources and assets
///
/// You can use [`WorldInspectorPlugin::run_if`] to control when the window is shown.
///
/// ```rust
/// use bevy::prelude::*;
/// use godot_bevy::prelude::*;
/// use godot_bevy::plugins::inspector::WorldInspectorPlugin;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(GodotPlugin)
///         .add_plugins(WorldInspectorPlugin::new())
///         .run();
/// }
/// ```
#[derive(Default)]
pub struct WorldInspectorPlugin {
    condition: Option<BoxedCondition>,
}

impl WorldInspectorPlugin {
    pub fn new() -> Self {
        Self::default()
    }

    /// Only show the UI if the specified condition is active
    pub fn run_if<M>(mut self, condition: impl Condition<M>) -> Self {
        let condition_system = IntoSystem::into_system(condition);
        self.condition = Some(Box::new(condition_system) as BoxedCondition);
        self
    }
}

impl Plugin for WorldInspectorPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<crate::plugins::core::GodotCorePlugin>() {
            panic!("WorldInspectorPlugin requires GodotCorePlugin to be added first");
        }

        app.init_resource::<WorldInspectorState>();

        // For now, just add systems unconditionally - conditional support can be added later
        app.add_systems(Update, (world_inspector_system, world_inspector_ui_update_system));
    }
}

// World Inspector implementation
fn world_inspector_system(world: &mut World) {
    let mut inspector_state = world.resource_mut::<WorldInspectorState>();
    
    // Create window on first run
    if !inspector_state.window_created {
        if let Some(handle) = create_world_inspector_window() {
            inspector_state.window_created = true;
            inspector_state.window_handle = Some(handle);
            info!("🔍 World Inspector window created successfully");
        } else {
            info!("⚠️ Failed to create World Inspector window");
        }
    }
    
    // Update every 3 seconds to avoid spam
    let now = Instant::now();
    if now.duration_since(inspector_state.last_update).as_secs() < 3 {
        return;
    }
    inspector_state.last_update = now;

    if !inspector_state.is_initialized {
        info!("🔍 Bevy World Inspector initialized");
        inspector_state.is_initialized = true;
    }

    // Log basic world statistics
    log_world_stats(world);
}

fn world_inspector_ui_update_system(world: &mut World) {
    let mut inspector_state = world.resource_mut::<WorldInspectorState>();
    
    // Update UI every 5 seconds to reduce resets (instead of 1 second)
    let now = Instant::now();
    if now.duration_since(inspector_state.last_ui_update).as_millis() < 5000 {
        return;
    }
    inspector_state.last_ui_update = now;
    
    // Only update if window exists
    if let Some(window_handle) = &mut inspector_state.window_handle {
        if let Some(window_node) = window_handle.try_get::<Node>() {
            update_world_inspector_ui(world, &window_node);
        }
    }
}

fn update_world_inspector_ui(world: &mut World, window_node: &Gd<Node>) {
    // Try to get MainContainer directly by index (we know it's child 0)
    if let Some(main_container) = window_node.get_child(0) {
        if main_container.get_name().to_string() == "MainContainer" {
            // Child 1: ScrollContainer (contains InspectorTree)
            if let Some(scroll_container) = main_container.get_child(1) {
                if let Some(inspector_tree) = scroll_container.get_child(0) {
                    if let Ok(mut tree) = inspector_tree.try_cast::<Tree>() {
                        update_world_inspector_tree(world, &mut tree);
                    }
                }
            }
        }
    }
}

fn update_world_inspector_tree(world: &mut World, tree: &mut Gd<Tree>) {
    // Clear the tree
    tree.clear();
    
    // Create invisible root
    let _root = tree.create_item();
    
    // Create the three main sections like the original bevy_inspector_egui
    
    // 1. ENTITIES SECTION
    if let Some(mut entities_root) = tree.create_item() {
        let entity_count = world.iter_entities().count();
        entities_root.set_text(0, &format!("📦 Entities ({})", entity_count));
        
        // Add entity filter info
        if let Some(mut filter_info) = entities_root.create_child() {
            filter_info.set_text(0, "🔍 Filter: [Simplified - showing first 15]");
        }
        
        // Show entities (limit to avoid UI clutter)
        for (_idx, entity) in world.iter_entities().take(15).enumerate() {
            let entity_id = entity.id();
            let entity_name = guess_entity_name(world, entity_id);
            let component_count = entity.archetype().components().count();
            
            if let Some(mut entity_item) = entities_root.create_child() {
                let entity_text = format!("• {} ({} components)", entity_name, component_count);
                entity_item.set_text(0, &entity_text);
                
                // Add component details for this entity (first 5 components)
                let components = get_entity_component_names(world, entity_id);
                for comp_name in components.iter().take(5) {
                    if let Some(mut comp_item) = entity_item.create_child() {
                        comp_item.set_text(0, &format!("└ {}", comp_name));
                    }
                }
                
                if components.len() > 5 {
                    if let Some(mut more_item) = entity_item.create_child() {
                        more_item.set_text(0, &format!("└ ... and {} more components", components.len() - 5));
                    }
                }
            }
        }
        
        if entity_count > 15 {
            if let Some(mut more_entities) = entities_root.create_child() {
                more_entities.set_text(0, &format!("... and {} more entities", entity_count - 15));
            }
        }
    }
    
    // 2. RESOURCES SECTION
    if let Some(mut resources_root) = tree.create_item() {
        let resource_count = count_resources(world);
        resources_root.set_text(0, &format!("🎛️ Resources ({})", resource_count));
        
        // Show actual resource data for known resources (similar to original)
        
        // Time resource (if it exists)
        if let Some(time) = world.get_resource::<bevy::time::Time>() {
            if let Some(mut time_item) = resources_root.create_child() {
                time_item.set_text(0, &format!("└ Time"));
                
                if let Some(mut elapsed_time) = time_item.create_child() {
                    elapsed_time.set_text(0, &format!("   elapsed_secs: {:.3}", time.elapsed_secs()));
                }
                if let Some(mut delta_time) = time_item.create_child() {
                    delta_time.set_text(0, &format!("   delta_secs: {:.6}", time.delta_secs()));
                }
            }
        }
        
        // Assets<GodotResource> resource
        if let Some(godot_assets) = world.get_resource::<bevy::asset::Assets<GodotResource>>() {
            if let Some(mut assets_resource_item) = resources_root.create_child() {
                assets_resource_item.set_text(0, &format!("└ Assets<GodotResource>"));
                
                if let Some(mut assets_count) = assets_resource_item.create_child() {
                    assets_count.set_text(0, &format!("   len: {}", godot_assets.len()));
                }
                if let Some(mut assets_capacity) = assets_resource_item.create_child() {
                    assets_capacity.set_text(0, &format!("   capacity: {}", godot_assets.len())); // Approximation since capacity isn't public
                }
            }
        }
        
        // Generic summary for other resources we can't directly inspect
        if let Some(mut other_resources) = resources_root.create_child() {
            other_resources.set_text(0, &format!("└ {} other resource archetypes", resource_count.saturating_sub(2)));
            
            if let Some(mut note) = other_resources.create_child() {
                note.set_text(0, "   [Resources require reflection for full inspection]");
            }
            if let Some(mut note2) = other_resources.create_child() {
                note2.set_text(0, "   [Consider implementing Debug display for your resources]");
            }
        }
    }
    
    // 3. ASSETS SECTION  
    if let Some(mut assets_root) = tree.create_item() {
        let godot_asset_count = if let Some(godot_assets) = world.get_resource::<bevy::asset::Assets<GodotResource>>() {
            godot_assets.len()
        } else {
            0
        };
        
        assets_root.set_text(0, &format!("🎨 Assets ({})", godot_asset_count));
        
        // Show GodotResource assets
        if let Some(mut godot_assets_item) = assets_root.create_child() {
            godot_assets_item.set_text(0, &format!("└ GodotResource ({})", godot_asset_count));
            
            if godot_asset_count > 0 {
                if let Some(assets) = world.get_resource::<bevy::asset::Assets<GodotResource>>() {
                    for (_idx, (handle_id, _asset)) in assets.iter().take(10).enumerate() {
                        if let Some(mut asset_item) = godot_assets_item.create_child() {
                            asset_item.set_text(0, &format!("   Asset_{:?}", handle_id));
                        }
                    }
                    
                    if assets.len() > 10 {
                        if let Some(mut more_assets) = godot_assets_item.create_child() {
                            more_assets.set_text(0, &format!("   ... and {} more assets", assets.len() - 10));
                        }
                    }
                }
            } else {
                if let Some(mut no_assets) = godot_assets_item.create_child() {
                    no_assets.set_text(0, "   [No GodotResource assets loaded]");
                }
            }
        }
    }
}

fn log_world_stats(world: &World) {
    // Entity statistics
    let entity_count = world.entities().len();
    let archetypes_count = world.archetypes().len();
    
    info!("📦 Entities: {} | Archetypes: {}", entity_count, archetypes_count);

    // Sample a few entities with their names
    let mut entity_samples: Vec<String> = Vec::new();
    for entity in world.iter_entities().take(3) {
        let entity_id = entity.id();
        let name = guess_entity_name(world, entity_id);
        let component_count = entity.archetype().components().count();
        entity_samples.push(format!("{} ({})", name, component_count));
    }
    
    if !entity_samples.is_empty() {
        info!("  Sample entities: {}", entity_samples.join(", "));
    }

    // Resource statistics - simplified count
    let mut resource_archetype_count = 0;
    for archetype in world.archetypes().iter() {
        if archetype.entities().is_empty() && archetype.components().count() > 0 {
            resource_archetype_count += 1;
        }
    }
    
    info!("🎛️ Resource archetypes: {}", resource_archetype_count);

    // Asset statistics
    if let Some(godot_assets) = world.get_resource::<bevy::asset::Assets<GodotResource>>() {
        info!("🎨 Assets: {} GodotResource assets loaded", godot_assets.len());
    }
} 