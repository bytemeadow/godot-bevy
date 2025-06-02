use std::{marker::PhantomData, time::Instant};

use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        schedule::BoxedCondition,
        system::IntoSystem,
        world::World,
    },
    log::info,
    prelude::{Resource, Condition},
};

use super::{
    ui::create_resource_inspector_window,
    utils::pretty_type_name,
    ResourceInspectorState
};

/// Plugin displaying a Godot window for a single resource.
/// Remember to insert the resource and register the type if using Reflect.
///
/// ```rust
/// use bevy::prelude::*;
/// use godot_bevy::prelude::*;
/// use godot_bevy::plugins::inspector::ResourceInspectorPlugin;
///
/// #[derive(Resource, Default)]
/// struct Configuration {
///     name: String,
///     value: f32,
/// }
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(GodotPlugin)
///         .init_resource::<Configuration>()
///         .add_plugins(ResourceInspectorPlugin::<Configuration>::new())
///         .run();
/// }
/// ```
pub struct ResourceInspectorPlugin<T> {
    condition: Option<BoxedCondition>,
    marker: PhantomData<fn() -> T>,
}

impl<T> Default for ResourceInspectorPlugin<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
            condition: None,
        }
    }
}

impl<T> ResourceInspectorPlugin<T> {
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

impl<T: Resource + std::fmt::Debug> Plugin for ResourceInspectorPlugin<T> {
    fn build(&self, app: &mut bevy::app::App) {
        if !app.is_plugin_added::<crate::plugins::core::GodotCorePlugin>() {
            panic!("ResourceInspectorPlugin requires GodotCorePlugin to be added first");
        }

        app.init_resource::<ResourceInspectorState<T>>();

        // For now, just add systems unconditionally
        app.add_systems(Update, (resource_inspector_system::<T>, resource_inspector_ui_update_system::<T>));
    }
}

// Resource Inspector implementation
fn resource_inspector_system<T: Resource + std::fmt::Debug>(world: &mut World) {
    let mut inspector_state = world.resource_mut::<ResourceInspectorState<T>>();
    
    if !inspector_state.window_created {
        if let Some(handle) = create_resource_inspector_window::<T>() {
            inspector_state.window_created = true;
            inspector_state.window_handle = Some(handle);
            info!("🔍 Resource Inspector for {} created", pretty_type_name::<T>());
        }
    }
}

fn resource_inspector_ui_update_system<T: Resource + std::fmt::Debug>(world: &mut World) {
    let mut inspector_state = world.resource_mut::<ResourceInspectorState<T>>();
    
    let now = Instant::now();
    if now.duration_since(inspector_state.last_update).as_millis() < 1000 {
        return;
    }
    inspector_state.last_update = now;
    
    if let Some(window_handle) = &mut inspector_state.window_handle {
        if let Some(window_node) = window_handle.try_get::<godot::classes::Node>() {
            update_resource_inspector_ui::<T>(world, &window_node);
        }
    }
}

fn update_resource_inspector_ui<T: Resource + std::fmt::Debug>(world: &World, _window_node: &godot::prelude::Gd<godot::classes::Node>) {
    // Simple resource display - could be expanded
    if let Some(resource) = world.get_resource::<T>() {
        info!("🔍 Resource {}: {:?}", pretty_type_name::<T>(), resource);
    }
} 