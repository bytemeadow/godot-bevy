//! Easy plugins for showing Godot UI panels for Bevy world inspection.
//!
//! **Pros:** no manual code required, native Godot UI integration
//!
//! **Cons:** limited configurability compared to egui version
//!
//! When you want something more custom, you can use these plugins as a starting point.

use std::{marker::PhantomData, time::Instant};

use bevy::{
    app::{App, Plugin, Update},
    asset::Asset,
    ecs::{
        query::QueryFilter,
        schedule::BoxedCondition,
        system::IntoSystem,
    },
    log::info,
    prelude::{Resource, Condition},
};

use crate::bridge::GodotNodeHandle;

pub mod ui;
pub mod world;
pub mod resources;
pub mod assets;
pub mod entities;
pub mod utils;

// Re-export the main plugins
pub use world::WorldInspectorPlugin;
pub use resources::ResourceInspectorPlugin;
pub use assets::AssetInspectorPlugin;
pub use entities::FilterQueryInspectorPlugin;

// Re-export utilities
pub use utils::{guess_entity_name, pretty_type_name};

const DEFAULT_WINDOW_SIZE: (i32, i32) = (500, 600);

// State structs
#[derive(bevy::prelude::Resource)]
pub(crate) struct WorldInspectorState {
    pub last_update: Instant,
    pub last_ui_update: Instant,
    pub is_initialized: bool,
    pub window_created: bool,
    pub window_handle: Option<GodotNodeHandle>,
}

impl Default for WorldInspectorState {
    fn default() -> Self {
        Self {
            last_update: Instant::now(),
            last_ui_update: Instant::now(),
            is_initialized: false,
            window_created: false,
            window_handle: None,
        }
    }
}

#[derive(bevy::prelude::Resource)]
pub(crate) struct ResourceInspectorState<T> {
    pub last_update: Instant,
    pub window_created: bool,
    pub window_handle: Option<GodotNodeHandle>,
    pub marker: PhantomData<T>,
}

impl<T> Default for ResourceInspectorState<T> {
    fn default() -> Self {
        Self {
            last_update: Instant::now(),
            window_created: false,
            window_handle: None,
            marker: PhantomData,
        }
    }
}

#[derive(bevy::prelude::Resource)]
pub(crate) struct AssetInspectorState<A> {
    pub last_update: Instant,
    pub window_created: bool,
    pub window_handle: Option<GodotNodeHandle>,
    pub marker: PhantomData<A>,
}

impl<A> Default for AssetInspectorState<A> {
    fn default() -> Self {
        Self {
            last_update: Instant::now(),
            window_created: false,
            window_handle: None,
            marker: PhantomData,
        }
    }
}

#[derive(bevy::prelude::Resource)]
pub(crate) struct FilterQueryInspectorState<F> {
    pub last_update: Instant,
    pub window_created: bool,
    pub window_handle: Option<GodotNodeHandle>,
    pub marker: PhantomData<F>,
}

impl<F> Default for FilterQueryInspectorState<F> {
    fn default() -> Self {
        Self {
            last_update: Instant::now(),
            window_created: false,
            window_handle: None,
            marker: PhantomData,
        }
    }
} 