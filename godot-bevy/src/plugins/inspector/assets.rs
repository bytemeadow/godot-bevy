use std::{marker::PhantomData, time::Instant};

use bevy::{
    app::{App, Plugin, Update},
    asset::Asset,
    ecs::{
        schedule::BoxedCondition,
        system::IntoSystem,
        world::World,
    },
    log::info,
    prelude::Condition,
};

use super::{
    ui::create_asset_inspector_window,
    utils::pretty_type_name,
    AssetInspectorState
};

/// Plugin displaying a Godot window for all assets of type `A`.
///
/// ```rust
/// use bevy::prelude::*;
/// use godot_bevy::prelude::*;
/// use godot_bevy::plugins::inspector::AssetInspectorPlugin;
/// use godot_bevy::plugins::assets::GodotResource;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(GodotPlugin)
///         .add_plugins(AssetInspectorPlugin::<GodotResource>::new())
///         .run();
/// }
/// ```
pub struct AssetInspectorPlugin<A> {
    condition: Option<BoxedCondition>,
    marker: PhantomData<fn() -> A>,
}

impl<A> Default for AssetInspectorPlugin<A> {
    fn default() -> Self {
        Self {
            condition: None,
            marker: PhantomData,
        }
    }
}

impl<A> AssetInspectorPlugin<A> {
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

impl<A: Asset + std::fmt::Debug> Plugin for AssetInspectorPlugin<A> {
    fn build(&self, app: &mut bevy::app::App) {
        if !app.is_plugin_added::<crate::plugins::core::GodotCorePlugin>() {
            panic!("AssetInspectorPlugin requires GodotCorePlugin to be added first");
        }

        app.init_resource::<AssetInspectorState<A>>();

        // For now, just add systems unconditionally
        app.add_systems(Update, (asset_inspector_system::<A>, asset_inspector_ui_update_system::<A>));
    }
}

// Asset Inspector implementation  
fn asset_inspector_system<A: Asset + std::fmt::Debug>(world: &mut World) {
    let mut inspector_state = world.resource_mut::<AssetInspectorState<A>>();
    
    if !inspector_state.window_created {
        if let Some(handle) = create_asset_inspector_window::<A>() {
            inspector_state.window_created = true;
            inspector_state.window_handle = Some(handle);
            info!("🔍 Asset Inspector for {} created", pretty_type_name::<A>());
        }
    }
}

fn asset_inspector_ui_update_system<A: Asset + std::fmt::Debug>(world: &mut World) {
    let mut inspector_state = world.resource_mut::<AssetInspectorState<A>>();
    
    let now = Instant::now();
    if now.duration_since(inspector_state.last_update).as_millis() < 1000 {
        return;
    }
    inspector_state.last_update = now;
    
    if let Some(window_handle) = &mut inspector_state.window_handle {
        if let Some(window_node) = window_handle.try_get::<godot::classes::Node>() {
            update_asset_inspector_ui::<A>(world, &window_node);
        }
    }
}

fn update_asset_inspector_ui<A: Asset + std::fmt::Debug>(world: &World, _window_node: &godot::prelude::Gd<godot::classes::Node>) {
    // Simple asset display - could be expanded
    if let Some(assets) = world.get_resource::<bevy::asset::Assets<A>>() {
        info!("🔍 Assets {}: {} loaded", pretty_type_name::<A>(), assets.len());
    }
} 