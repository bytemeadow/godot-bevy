use bevy::{
    ecs::{
        entity::Entity,
        name::Name,
        world::{EntityRef, World},
    },
};

/// Guesses an appropriate entity name like `Light (6)` or falls back to `Entity (8)`
pub fn guess_entity_name(world: &World, entity: Entity) -> String {
    match world.get_entity(entity) {
        Ok(entity_ref) => {
            if let Some(name) = entity_ref.get::<Name>() {
                return format!("{} ({})", name.as_str(), entity.index());
            }

            guess_entity_name_from_components(entity, entity_ref)
        }
        Err(_) => format!("Entity {} (inexistent)", entity.index()),
    }
}

fn guess_entity_name_from_components(entity: Entity, entity_ref: EntityRef) -> String {
    let component_count = entity_ref.archetype().components().count();
    
    // Simple heuristics based on component count
    match component_count {
        0 => format!("Empty Entity ({})", entity.index()),
        1..=2 => format!("Simple Entity ({})", entity.index()),
        3..=5 => format!("Basic Entity ({})", entity.index()),
        6..=10 => format!("Complex Entity ({})", entity.index()),
        _ => format!("Rich Entity ({})", entity.index()),
    }
}

/// Returns a pretty, human-readable type name
pub fn pretty_type_name<T>() -> String {
    let type_name = std::any::type_name::<T>();
    type_name
        .split("::")
        .last()
        .unwrap_or(type_name)
        .replace("<", "_")
        .replace(">", "_")
        .replace(",", "_")
}

/// Get simplified component names for an entity
pub fn get_entity_component_names(world: &World, entity: Entity) -> Vec<String> {
    let mut component_names = Vec::new();
    
    if let Ok(entity_ref) = world.get_entity(entity) {
        let archetype = entity_ref.archetype();
        for component_id in archetype.components() {
            if let Some(component_info) = world.components().get_info(component_id) {
                let type_name = component_info.name();
                // Simplify the type name for display
                let simple_name = type_name
                    .split("::")
                    .last()
                    .unwrap_or(type_name)
                    .replace("Component", "");
                component_names.push(simple_name);
            }
        }
    }
    
    component_names
}

/// Count the number of resource archetypes in the world
pub fn count_resources(world: &World) -> usize {
    let mut resource_count = 0;
    for archetype in world.archetypes().iter() {
        if archetype.entities().is_empty() && archetype.components().count() > 0 {
            resource_count += 1;
        }
    }
    resource_count
} 