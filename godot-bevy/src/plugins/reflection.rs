//! Reflection plugin for godot-bevy.
//!
//! This plugin sets up Bevy's type registry and registers all godot-bevy types
//! for reflection. This enables runtime inspection of components and resources.

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_reflect::TypeRegistry;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::plugins::collisions::Collisions;
use crate::plugins::scene_tree::{NodeEntityIndex, SceneTreeConfig};

/// Resource providing access to Bevy's type registry.
///
/// This is the central registry for all reflected types in the application.
/// Components and resources that derive `Reflect` can be registered here
/// to enable runtime inspection.
///
/// # Example
///
/// ```rust,ignore
/// use bevy_reflect::Reflect;
/// use godot_bevy::prelude::*;
///
/// #[derive(Component, Reflect, Default)]
/// #[reflect(Component)]
/// struct MyComponent {
///     value: f32,
/// }
///
/// fn setup(app: &mut App) {
///     app.register_type::<MyComponent>();
/// }
/// ```
#[derive(Resource, Clone)]
pub struct AppTypeRegistry {
    internal: Arc<RwLock<TypeRegistry>>,
}

impl Default for AppTypeRegistry {
    fn default() -> Self {
        Self {
            internal: Arc::new(RwLock::new(TypeRegistry::default())),
        }
    }
}

impl AppTypeRegistry {
    /// Create a new type registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get read access to the type registry.
    pub fn read(&self) -> RwLockReadGuard<'_, TypeRegistry> {
        self.internal.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Get write access to the type registry.
    pub fn write(&self) -> RwLockWriteGuard<'_, TypeRegistry> {
        self.internal.write().unwrap_or_else(|e| e.into_inner())
    }
}

/// Plugin that sets up reflection infrastructure for godot-bevy.
///
/// This plugin:
/// - Initializes the `AppTypeRegistry` resource
/// - Registers core godot-bevy types that have `#[derive(Reflect)]`
/// - Enables runtime inspection of components and resources
///
/// # Registering Your Own Types
///
/// To make your components inspectable, derive `Reflect` and register them:
///
/// ```rust,ignore
/// use bevy_reflect::Reflect;
/// use godot_bevy::prelude::*;
///
/// #[derive(Component, Reflect, Default)]
/// #[reflect(Component)]
/// struct Health {
///     current: f32,
///     max: f32,
/// }
///
/// // In your app setup:
/// app.register_type::<Health>();
/// ```
///
/// # Usage
///
/// This plugin is automatically included in `GodotCorePlugins`. If you're
/// building a custom plugin set, add it manually:
///
/// ```rust,ignore
/// app.add_plugins(GodotReflectionPlugin);
/// ```
#[derive(Default)]
pub struct GodotReflectionPlugin;

impl Plugin for GodotReflectionPlugin {
    fn build(&self, app: &mut App) {
        // Initialize the type registry resource
        app.init_resource::<AppTypeRegistry>();

        // Register godot-bevy types that have #[derive(Reflect)]
        // Users should register their own types via app.register_type::<T>()

        // Scene tree types (have Reflect derived)
        app.register_type::<NodeEntityIndex>();
        app.register_type::<SceneTreeConfig>();

        // Collision types (has Reflect derived)
        app.register_type::<Collisions>();
    }
}

/// Extension trait for accessing the type registry.
pub trait AppReflectionExt {
    /// Get a reference to the app's type registry.
    fn type_registry(&self) -> &AppTypeRegistry;

    /// Get a mutable reference to the app's type registry.
    fn type_registry_mut(&mut self) -> &mut AppTypeRegistry;
}

impl AppReflectionExt for App {
    fn type_registry(&self) -> &AppTypeRegistry {
        self.world().resource::<AppTypeRegistry>()
    }

    fn type_registry_mut(&mut self) -> &mut AppTypeRegistry {
        self.world_mut()
            .resource_mut::<AppTypeRegistry>()
            .into_inner()
    }
}
