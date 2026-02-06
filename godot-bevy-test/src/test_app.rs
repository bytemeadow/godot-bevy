//! Bevy-style test app for frame-by-frame testing
//!
//! This provides a Bevy-like API for tests while ensuring real Godot integration:
//! - app.world() / app.world_mut() for ECS access (just like Bevy)
//! - app.update().await for frame stepping (async because we wait for Godot)
//! - Automatic cleanup on drop
//! - Relies on library's automatic watcher setup

use bevy::prelude::*;
use godot::obj::{Gd, NewAlloc};

use crate::{TestContext, await_frame};

/// A test app that provides Bevy-style API while running in Godot runtime
///
/// Example:
/// ```ignore
/// let mut app = TestApp::new(ctx, |app| {
///     app.add_plugins(GodotTransformSyncPlugin);
/// }).await;
///
/// let entity = app.with_world_mut(|world| {
///     world.spawn((Transform::default(),)).id()
/// });
///
/// app.update().await;
///
/// let translation_x = app.with_world(|world| {
///     world.get::<Transform>(entity).unwrap().translation.x
/// });
/// assert_eq!(translation_x, 0.0);
/// ```
pub struct TestApp {
    ctx: TestContext,
    bevy_app: Option<Gd<godot_bevy::BevyApp>>,
}

impl TestApp {
    /// Create a new test app with custom setup
    ///
    /// The setup function is called during BevyApp initialization.
    /// GodotCorePlugins is automatically added, providing scene tree integration.
    /// The library handles all watcher creation automatically.
    pub async fn new<F>(ctx: &TestContext, setup: F) -> Self
    where
        F: FnOnce(&mut App) + Send + 'static,
    {
        use std::sync::Mutex;

        await_frame().await; // Wait for any previous test cleanup

        let mut bevy_app = godot_bevy::BevyApp::new_alloc();

        let setup_mutex = Mutex::new(Some(setup));

        // Configure the app - the library will handle all watcher setup and core plugins
        bevy_app
            .bind_mut()
            .set_instance_init_func(Box::new(move |app: &mut App| {
                // User setup - take from Mutex to call FnOnce
                if let Some(setup_fn) = setup_mutex.lock().unwrap().take() {
                    setup_fn(app);
                }
            }));

        // Add to scene tree (triggers ready() which initializes the app)
        ctx.scene_tree.clone().add_child(&bevy_app);

        // Wait for ready() to complete
        await_frame().await;

        Self {
            ctx: ctx.clone(),
            bevy_app: Some(bevy_app),
        }
    }

    /// Step one frame
    ///
    /// This waits for Godot to advance one frame, during which Godot will call
    /// BevyApp::process(), which internally calls app.update().
    pub async fn update(&self) {
        await_frame().await;
    }

    /// Get immutable access to the Bevy World
    ///
    /// Use this to query component state, just like in Bevy tests.
    /// Note: This uses a closure to avoid lifetime issues with the Gd borrow.
    pub fn with_world<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&World) -> R,
    {
        let binding = self.bevy_app.as_ref().unwrap().bind();
        let app = binding.get_app().expect("App should be initialized");
        f(app.world())
    }

    /// Get mutable access to the Bevy World
    ///
    /// Use this to spawn entities, modify components, etc.
    pub fn with_world_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut World) -> R,
    {
        let mut binding = self.bevy_app.as_mut().unwrap().bind_mut();
        let app = binding.get_app_mut().expect("App should be initialized");
        f(app.world_mut())
    }

    /// Convenience: Get a component from a single entity
    pub fn get_single<C>(&mut self) -> C
    where
        C: Component + Clone,
    {
        self.with_world_mut(|world| {
            let mut query = world.query::<&C>();
            query
                .iter(world)
                .next()
                .expect("Should have entity")
                .clone()
        })
    }

    /// Convenience: Get entity ID with a specific component
    pub fn single_entity_with<C: Component>(&mut self) -> Entity {
        self.with_world_mut(|world| {
            let mut query = world.query::<(Entity, &C)>();
            query.iter(world).next().expect("Should have entity").0
        })
    }

    /// Look up the Bevy entity for a Godot node by instance ID
    pub fn entity_for_node(&self, instance_id: godot::obj::InstanceId) -> Option<Entity> {
        self.with_world(|world| {
            world
                .resource::<godot_bevy::prelude::NodeEntityIndex>()
                .get(instance_id)
        })
    }

    /// Check whether a Bevy entity exists for a Godot node
    pub fn has_entity_for_node(&self, instance_id: godot::obj::InstanceId) -> bool {
        self.with_world(|world| {
            world
                .resource::<godot_bevy::prelude::NodeEntityIndex>()
                .contains(instance_id)
        })
    }

    /// Add a new Godot node to the scene tree and return it with its entity
    ///
    /// Creates a node, sets its name, adds it to the scene tree, waits one
    /// frame for entity creation, and returns both the node and its entity.
    pub async fn add_node<T>(&mut self, name: &str) -> (Gd<T>, Entity)
    where
        T: godot::obj::Inherits<godot::classes::Node> + NewAlloc + godot::obj::GodotClass,
    {
        let node = T::new_alloc();
        node.clone().upcast::<godot::classes::Node>().set_name(name);
        self.ctx
            .scene_tree
            .clone()
            .add_child(&node.clone().upcast::<godot::classes::Node>());
        self.update().await;
        let entity = self
            .entity_for_node(node.instance_id())
            .unwrap_or_else(|| panic!("Entity should exist for node '{name}' after update"));
        (node, entity)
    }

    /// Get the test context
    pub fn ctx(&self) -> &TestContext {
        &self.ctx
    }

    /// Manually cleanup the TestApp
    ///
    /// This should be called BEFORE calling queue_free() on any Godot nodes
    /// that have entities in the ECS. This prevents transform sync systems
    /// from trying to access freed nodes.
    pub fn cleanup(&mut self) {
        if let Some(mut app) = self.bevy_app.take() {
            app.queue_free();
        }
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Cleanup if not already done
        self.cleanup();
    }
}
