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

/// Wait for one frame boundary. With `test-frame-signal` this resolves on
/// `bevy_frame_complete` (returning the frame's physics step count); without it,
/// it falls back to a plain process-frame wait and reports 0 steps.
async fn wait_frame_boundary(app: &Gd<godot_bevy::BevyApp>) -> i64 {
    #[cfg(feature = "test-frame-signal")]
    {
        crate::await_bevy_frame(app).await
    }
    #[cfg(not(feature = "test-frame-signal"))]
    {
        let _ = app;
        await_frame().await;
        0
    }
}

impl TestApp {
    /// Create a new test app by initializing the BevyAppSingleton autoload.
    ///
    /// Returns a fully-settled app: two frames have elapsed so that the initial
    /// scene tree population is complete (messages written, swapped, and read).
    ///
    /// The setup function is called during BevyApp initialization.
    /// GodotCorePlugins is automatically added, providing scene tree integration.
    pub async fn new<F>(ctx: &TestContext, setup: F) -> Self
    where
        F: FnOnce(&mut App) + Send + 'static,
    {
        use std::sync::Mutex;

        await_frame().await; // Wait for any previous test cleanup

        let scene_tree = ctx.scene_tree.get_tree();
        let root = scene_tree.get_root().expect("Root should exist");
        let mut bevy_app = root
            .try_get_node_as::<godot_bevy::BevyApp>("BevyAppSingleton")
            .expect("BevyAppSingleton autoload not found. Enable the godot-bevy plugin in Project Settings > Plugins, or add BevyAppSingleton as an autoload.");

        let setup_mutex = Mutex::new(Some(setup));

        bevy_app
            .bind_mut()
            .set_instance_init_func(Box::new(move |app: &mut App| {
                if let Some(setup_fn) = setup_mutex.lock().unwrap().take() {
                    setup_fn(app);
                }
            }));

        bevy_app.bind_mut().initialize();

        #[cfg(feature = "test-frame-signal")]
        {
            crate::await_bevy_frame(&bevy_app).await;
            crate::await_bevy_frame(&bevy_app).await;
        }
        #[cfg(not(feature = "test-frame-signal"))]
        {
            await_frame().await;
            await_frame().await;
        }

        Self {
            ctx: ctx.clone(),
            bevy_app: Some(bevy_app),
        }
    }

    /// Advance one Godot frame. With the `test-frame-signal` feature, resolves on
    /// `bevy_frame_complete` (after the suffix + clear_trackers), so any Update
    /// system writes are visible when this returns. Returns the physics step count.
    pub async fn update(&self) -> i64 {
        wait_frame_boundary(self.bevy_app.as_ref().unwrap()).await
    }

    /// Advance multiple Godot frames.
    ///
    /// Convenience for calling `update()` N times.
    pub async fn updates(&self, count: u32) {
        for _ in 0..count {
            self.update().await;
        }
    }

    /// Advance one full frame, guaranteeing a physics tick ran. Under `--fixed-fps`
    /// this is equivalent to `update()`. Without the `test-frame-signal` feature it
    /// falls back to waiting for a physics_frame signal before the frame boundary.
    pub async fn physics_update(&self) {
        #[cfg(not(feature = "test-frame-signal"))]
        crate::await_physics_frame().await;
        wait_frame_boundary(self.bevy_app.as_ref().unwrap()).await;
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

    /// Add an already-built Godot node (with its children set up) to the scene tree
    /// and return it with its entity.
    ///
    /// Like [`add_node`](Self::add_node) but takes a node the caller constructed, so
    /// shapes/children are present before the scene-tree connect runs -- needed to test
    /// spawn-into-overlap, where the overlap must exist at the first flush.
    pub async fn add_prebuilt_node<T>(&mut self, node: Gd<T>, name: &str) -> (Gd<T>, Entity)
    where
        T: godot::obj::Inherits<godot::classes::Node> + godot::obj::GodotClass,
    {
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
                    "Entity not yet created for prebuilt node '{}' after {} frame(s), waiting another frame",
                    name,
                    i + 1,
                );
            }
        }

        panic!("Entity should exist for prebuilt node '{name}' after 3 frames");
    }

    /// Get the test context
    pub fn ctx(&self) -> &TestContext {
        &self.ctx
    }

    /// Clean up the TestApp, resetting the autoload's BevyApp for the next test.
    ///
    /// This should be called BEFORE calling queue_free() on any Godot nodes
    /// that have entities in the ECS. This prevents transform sync systems
    /// from trying to access freed nodes.
    pub async fn cleanup(&mut self) {
        if let Some(mut app) = self.bevy_app.take() {
            app.bind_mut().teardown();
            // process() still runs and emits the signal even with app=None, so this
            // settles the frame boundary regardless of feature path.
            #[cfg(feature = "test-frame-signal")]
            crate::await_bevy_frame(&app).await;
            #[cfg(not(feature = "test-frame-signal"))]
            await_frame().await;
        }
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        if let Some(mut app) = self.bevy_app.take() {
            app.bind_mut().teardown();
        }
        // Guarantee scene-tree isolation between tests. A test that panics before
        // its own free (failed assert), or simply forgets one, would otherwise leak
        // its nodes into the next test's scene scan and create spurious entities --
        // coupling tests through the shared singleton. Drop runs even on panic, so
        // free every leftover test node here. Explicitly-freed nodes are already out
        // of the tree, so this is a no-op for them (no double-free).
        for child in self.ctx.scene_tree.get_children().iter_shared() {
            child.free();
        }
    }
}
