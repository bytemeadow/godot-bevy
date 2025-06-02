use std::{marker::PhantomData, time::Instant};

use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        query::QueryFilter,
        schedule::BoxedCondition,
        system::IntoSystem,
        world::World,
    },
    log::info,
    prelude::Condition,
};

use super::{
    ui::create_filter_query_inspector_window,
    utils::pretty_type_name,
    FilterQueryInspectorState
};

/// Plugin displaying a Godot window for all entities matching the filter `F`.
///
/// ```rust
/// use bevy::prelude::*;
/// use godot_bevy::prelude::*;
/// use godot_bevy::plugins::inspector::FilterQueryInspectorPlugin;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(GodotPlugin)
///         .add_plugins(FilterQueryInspectorPlugin::<With<Transform>>::new())
///         .run();
/// }
/// ```
pub struct FilterQueryInspectorPlugin<F> {
    condition: Option<BoxedCondition>,
    marker: PhantomData<fn() -> F>,
}

impl<F> Default for FilterQueryInspectorPlugin<F> {
    fn default() -> Self {
        Self {
            condition: None,
            marker: PhantomData,
        }
    }
}

impl<F> FilterQueryInspectorPlugin<F> {
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

impl<F: 'static + QueryFilter + Send + Sync> Plugin for FilterQueryInspectorPlugin<F> {
    fn build(&self, app: &mut bevy::app::App) {
        if !app.is_plugin_added::<crate::plugins::core::GodotCorePlugin>() {
            panic!("FilterQueryInspectorPlugin requires GodotCorePlugin to be added first");
        }

        app.init_resource::<FilterQueryInspectorState<F>>();

        // For now, just add systems unconditionally
        app.add_systems(Update, (filter_query_inspector_system::<F>, filter_query_inspector_ui_update_system::<F>));
    }
}

// Filter Query Inspector implementation
fn filter_query_inspector_system<F: 'static + QueryFilter + Send + Sync>(world: &mut World) {
    let mut inspector_state = world.resource_mut::<FilterQueryInspectorState<F>>();
    
    if !inspector_state.window_created {
        if let Some(handle) = create_filter_query_inspector_window::<F>() {
            inspector_state.window_created = true;
            inspector_state.window_handle = Some(handle);
            info!("🔍 Filter Query Inspector for {} created", pretty_type_name::<F>());
        }
    }
}

fn filter_query_inspector_ui_update_system<F: 'static + QueryFilter + Send + Sync>(world: &mut World) {
    let mut inspector_state = world.resource_mut::<FilterQueryInspectorState<F>>();
    
    let now = Instant::now();
    if now.duration_since(inspector_state.last_update).as_millis() < 1000 {
        return;
    }
    inspector_state.last_update = now;
    
    if let Some(window_handle) = &mut inspector_state.window_handle {
        if let Some(window_node) = window_handle.try_get::<godot::classes::Node>() {
            update_filter_query_inspector_ui::<F>(world, &window_node);
        }
    }
}

fn update_filter_query_inspector_ui<F: QueryFilter>(world: &World, _window_node: &godot::prelude::Gd<godot::classes::Node>) {
    // Simple query display - could be expanded
    let entity_count = world.iter_entities().count();
    info!("🔍 Filter Query {}: {} total entities", pretty_type_name::<F>(), entity_count);
} 