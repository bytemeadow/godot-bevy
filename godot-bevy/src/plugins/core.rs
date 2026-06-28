use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;
use bevy_ecs::event::EntityEvent;
use bevy_ecs::lifecycle::Remove;
use bevy_ecs::observer::On;
use bevy_ecs::prelude::{Name, Resource};
use bevy_ecs::system::Query;
use std::any::TypeId;

use crate::interop::{GodotAccess, GodotMainThread, GodotNode, GodotNodeHandle};
use bevy_ecs::system::EntityCommands;
use godot::classes::Node;
use tracing::debug;

/// Function that adds a component to an entity with access to the Godot node
type ComponentInserter = Box<dyn Fn(&mut EntityCommands, &mut GodotNode) + Send + Sync>;

/// Registry for components that should be added to entities spawned from the scene tree
#[derive(Resource, Default)]
pub struct SceneTreeComponentRegistry {
    /// Components to add to every entity spawned from scene tree
    /// Stored as (TypeId, inserter) to avoid duplicates
    components: Vec<(TypeId, ComponentInserter)>,
}

impl SceneTreeComponentRegistry {
    /// Register a component type to be added to all scene tree entities
    pub fn register<C>(&mut self)
    where
        C: Component + Default,
    {
        let type_id = TypeId::of::<C>();

        // Check if already registered
        if self.components.iter().any(|(id, _)| *id == type_id) {
            return;
        }

        let inserter = Box::new(|entity: &mut EntityCommands, _node: &mut GodotNode| {
            entity.insert(C::default());
        });
        self.components.push((type_id, inserter));
    }

    /// Register a component type with custom initialization logic
    pub fn register_with_init<C, F>(&mut self, init_fn: F)
    where
        C: Component,
        F: Fn(&mut EntityCommands, &mut GodotNode) + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<C>();

        // Check if already registered
        if self.components.iter().any(|(id, _)| *id == type_id) {
            return;
        }

        let inserter = Box::new(init_fn);
        self.components.push((type_id, inserter));
    }

    /// Add all registered components to an entity
    pub fn add_to_entity(&self, entity: &mut EntityCommands, node: &mut GodotNode) {
        for (_, inserter) in &self.components {
            inserter(entity, node);
        }
    }
}

/// Extension trait for App to register scene tree components
pub trait AppSceneTreeExt {
    /// Register a component to be added to all scene tree entities with default value
    fn register_scene_tree_component<C>(&mut self) -> &mut Self
    where
        C: Component + Default;

    /// Register a component with custom initialization logic that has access to the Godot node
    fn register_scene_tree_component_with_init<C, F>(&mut self, init_fn: F) -> &mut Self
    where
        C: Component,
        F: Fn(&mut EntityCommands, &mut GodotNode) + Send + Sync + 'static;
}

impl AppSceneTreeExt for App {
    fn register_scene_tree_component<C>(&mut self) -> &mut Self
    where
        C: Component + Default,
    {
        // Get or create the registry
        if !self
            .world()
            .contains_resource::<SceneTreeComponentRegistry>()
        {
            self.world_mut()
                .init_resource::<SceneTreeComponentRegistry>();
        }

        self.world_mut()
            .resource_mut::<SceneTreeComponentRegistry>()
            .register::<C>();

        self
    }

    fn register_scene_tree_component_with_init<C, F>(&mut self, init_fn: F) -> &mut Self
    where
        C: Component,
        F: Fn(&mut EntityCommands, &mut GodotNode) + Send + Sync + 'static,
    {
        // Get or create the registry
        if !self
            .world()
            .contains_resource::<SceneTreeComponentRegistry>()
        {
            self.world_mut()
                .init_resource::<SceneTreeComponentRegistry>();
        }

        self.world_mut()
            .resource_mut::<SceneTreeComponentRegistry>()
            .register_with_init::<C, F>(init_fn);

        self
    }
}

/// Minimal core plugin with only essential Godot-Bevy integration.
/// This includes scene tree management, basic Bevy setup, and core resources.
#[derive(Default)]
pub struct GodotBaseCorePlugin;

impl Plugin for GodotBaseCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_time::TimePlugin)
            .add_plugins(bevy_app::TaskPoolPlugin::default())
            .add_plugins(bevy_diagnostic::FrameCountPlugin)
            .add_plugins(bevy_diagnostic::DiagnosticsPlugin)
            .init_non_send::<GodotMainThread>()
            .init_resource::<SceneTreeComponentRegistry>()
            .add_observer(on_godot_node_handle_removed);

        // Keeps RunFixedMainLoop's Before/After anchor sets live for ecosystem plugins
        // (e.g. leafwing). TimePlugin stays so Time<Real>/Virtual still advance in _process.
        crate::plugins::fixed_schedule::host_fixed_main_loop(app);
    }
}

pub trait FindEntityByNameExt<T> {
    fn find_entity_by_name(self, name: &str) -> Option<T>;
}

impl<'a, T: 'a, U> FindEntityByNameExt<T> for U
where
    U: Iterator<Item = (&'a Name, T)>,
{
    fn find_entity_by_name(mut self, name: &str) -> Option<T> {
        self.find_map(|(ent_name, t)| (ent_name.as_str() == name).then_some(t))
    }
}

/// Observer that automatically frees Godot nodes when GodotNodeHandle components are removed
fn on_godot_node_handle_removed(
    trigger: On<Remove, GodotNodeHandle>,
    query: Query<&GodotNodeHandle>,
    mut godot: GodotAccess,
) {
    if let Ok(handle) = query.get(trigger.event_target())
        && let Some(mut node) = godot.try_get::<Node>(*handle)
    {
        debug!(
            "Freeing Godot node with instance_id {:?}",
            handle.instance_id()
        );
        node.queue_free();
    }
}
