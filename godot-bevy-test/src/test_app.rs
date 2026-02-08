//! Bevy-style test app for frame-by-frame testing
//!
//! This provides a Bevy-like API for tests while ensuring real Godot integration:
//! - app.world() / app.world_mut() for ECS access (just like Bevy)
//! - app.update().await for frame stepping (async because we wait for Godot)
//! - Automatic cleanup on drop
//! - Relies on library's automatic watcher setup
//!
//! # Frame pacing
//!
//! The only way to advance time in tests is through `app.update()` (one frame)
//! or `app.updates(n)` (multiple frames). Both wait for real Godot frames, during
//! which BevyApp::process() runs and triggers all Bevy schedules.
//!
//! `TestApp::new()` returns a fully-settled app: the scene tree has been initialized
//! and the initial population of entities is complete. Tests should NOT need any
//! manual `await_frames()` calls.

use bevy::prelude::*;
use godot::obj::{Gd, NewAlloc};

use crate::{TestContext, await_frame};

/// A test app that provides Bevy-style API while running in Godot runtime
///
/// # Example
///
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
///
/// app.cleanup().await;
/// ```
pub struct TestApp {
    ctx: TestContext,
    bevy_app: Option<Gd<godot_bevy::BevyApp>>,
}

impl TestApp {
    /// Create a new test app with custom setup.
    ///
    /// Returns a fully-settled app: the BevyApp is added to the scene tree,
    /// `ready()` has fired, and two frames have elapsed so that the initial
    /// scene tree population is complete (messages written, swapped, and read).
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

        // Run one more frame so initial scene tree entities are created
        await_frame().await;

        Self {
            ctx: ctx.clone(),
            bevy_app: Some(bevy_app),
        }
    }

    /// Advance one Godot frame.
    ///
    /// This waits for Godot to advance one frame, during which Godot will call
    /// BevyApp::process(), which internally calls app.update().
    /// This is the primary way to advance time in tests.
    pub async fn update(&self) {
        await_frame().await;
    }

    /// Advance multiple Godot frames.
    ///
    /// Convenience for calling `update()` N times.
    pub async fn updates(&self, count: u32) {
        for _ in 0..count {
            self.update().await;
        }
    }

    /// Advance time until a physics tick and a render frame have both completed.
    ///
    /// Godot's main loop can run 0 physics ticks in a given render frame if
    /// insufficient time has accumulated. This method waits for the
    /// `physics_frame` signal (guaranteeing a physics tick is about to run),
    /// then waits for `process_frame` (guaranteeing both `_physics_process()`
    /// and `_process()` have completed). Use this when testing systems that
    /// run in `PrePhysicsUpdate` or `PhysicsUpdate`, such as collisions.
    pub async fn physics_update(&self) {
        crate::await_physics_frame().await;
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

    /// Add a new Godot node to the scene tree and return it with its entity.
    ///
    /// Creates a node, sets its name, adds it to the scene tree, and waits
    /// for entity creation. Entity creation may take up to 2 frames, so this
    /// method retries for up to 3 frames before panicking.
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

        for i in 0..3 {
            self.update().await;
            if let Some(entity) = self.entity_for_node(node.instance_id()) {
                return (node, entity);
            }
            if i < 2 {
                godot::global::godot_print!(
                    "Entity not yet created for node '{}' after {} frame(s), waiting another frame",
                    name,
                    i + 1,
                );
            }
        }

        panic!("Entity should exist for node '{name}' after 3 frames");
    }

    /// Get the test context
    pub fn ctx(&self) -> &TestContext {
        &self.ctx
    }

    /// Clean up the TestApp, freeing the BevyApp node.
    ///
    /// This should be called BEFORE calling queue_free() on any Godot nodes
    /// that have entities in the ECS. This prevents transform sync systems
    /// from trying to access freed nodes.
    ///
    /// Waits one frame for Godot to process the queue_free.
    pub async fn cleanup(&mut self) {
        if let Some(mut app) = self.bevy_app.take() {
            app.queue_free();
        }
        await_frame().await;
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Synchronous fallback if cleanup() wasn't called.
        // Prefer calling cleanup().await explicitly.
        if let Some(mut app) = self.bevy_app.take() {
            app.queue_free();
        }
    }
}
