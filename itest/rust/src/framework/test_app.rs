/*
 * Bevy-style test app for frame-by-frame testing
 *
 * This provides a Bevy-like API for tests while ensuring real Godot integration:
 * - app.world() / app.world_mut() for ECS access (just like Bevy)
 * - app.update().await for frame stepping (async because we wait for Godot)
 * - Automatic cleanup on drop
 * - Sets up required watchers (scene tree, collision, input)
 */

use bevy::prelude::*;
use godot::obj::{Gd, NewAlloc};
use std::sync::mpsc::channel;

use super::{TestContext, await_frame};

/// A test app that provides Bevy-style API while running in Godot runtime
///
/// Example:
/// ```rust
/// let mut app = TestApp::new(ctx, |app| {
///     app.add_plugins(GodotTransformSyncPlugin);
/// }).await;
///
/// let entity = app.world_mut().spawn((Transform::default(),)).id();
///
/// app.update().await;
///
/// assert_eq!(app.world().get::<Transform>(entity).unwrap().translation.x, 0.0);
/// ```
pub struct TestApp {
    ctx: TestContext,
    bevy_app: Option<Gd<godot_bevy::BevyApp>>,
}

impl TestApp {
    /// Create a new test app with custom setup
    ///
    /// The setup function is called during BevyApp initialization.
    /// GodotBaseCorePlugin is automatically added.
    /// Watchers (scene tree, collision, input) are automatically set up.
    pub async fn new<F>(ctx: &TestContext, setup: F) -> Self
    where
        F: FnOnce(&mut App) + Send + 'static,
    {
        use std::sync::Mutex;

        await_frame().await; // Wait for any previous test cleanup

        let mut bevy_app = godot_bevy::BevyApp::new_alloc();

        // Set up watchers as children BEFORE app initialization
        // (so scene tree plugin can find them in PreStartup)
        let event_readers = Self::setup_watchers_early(&mut bevy_app);

        // Wrap both event readers and user setup in Mutex
        let readers_mutex = Mutex::new(Some(event_readers));
        let setup_mutex = Mutex::new(Some(setup));

        // Configure the app before initialization
        bevy_app
            .bind_mut()
            .set_instance_init_func(Box::new(move |app: &mut App| {
                // Insert event readers FIRST (before plugins look for them)
                if let Some(readers) = readers_mutex.lock().unwrap().take() {
                    Self::insert_event_readers_direct(app, readers);
                }

                // Add core plugins (schedules + scene tree with flexible watcher search)
                app.add_plugins(godot_bevy::plugins::GodotCorePlugins);

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

    /// Set up watchers as children (before app initialization)
    /// Returns the event reader receivers to be inserted after app exists
    /// Note: Only sets up scene tree and collision watchers (input events are not Send-safe)
    fn setup_watchers_early(
        bevy_app: &mut Gd<godot_bevy::BevyApp>,
    ) -> (
        std::sync::mpsc::Receiver<godot_bevy::plugins::scene_tree::SceneTreeEvent>,
        std::sync::mpsc::Receiver<godot_bevy::plugins::collisions::CollisionEvent>,
    ) {
        use godot::classes::{GDScript, Node, ResourceLoader};
        use godot_bevy::watchers::{
            collision_watcher::CollisionWatcher, scene_tree_watcher::SceneTreeWatcher,
        };

        // Create all watchers and channels
        let (st_sender, st_receiver) = channel();
        let mut scene_tree_watcher = SceneTreeWatcher::new_alloc();
        scene_tree_watcher.bind_mut().notification_channel = Some(st_sender);
        scene_tree_watcher.set_name("SceneTreeWatcher");

        let (col_sender, col_receiver) = channel();
        let mut collision_watcher = CollisionWatcher::new_alloc();
        collision_watcher.bind_mut().notification_channel = Some(col_sender);
        collision_watcher.set_name("CollisionWatcher");

        // Add watchers as children (BEFORE app init)
        let mut base = bevy_app.clone().upcast::<godot::classes::Node>();
        base.add_child(&scene_tree_watcher);
        base.add_child(&collision_watcher);

        // Create OptimizedSceneTreeWatcher (GDScript) for signal routing
        // This connects to scene tree signals and forwards events to the Rust watcher
        match ResourceLoader::singleton()
            .load("res://addons/godot-bevy/optimized_scene_tree_watcher.gd")
        {
            Some(script) => {
                let mut script = script.cast::<GDScript>();
                let instance = script.instantiate(&[]);
                if let Ok(mut optimized_watcher) = instance.try_to::<godot::obj::Gd<Node>>() {
                    optimized_watcher.set_name("OptimizedSceneTreeWatcher");
                    // Add as child - it will auto-detect the SceneTreeWatcher sibling in _ready()
                    base.add_child(&optimized_watcher);
                }
            }
            None => {
                eprintln!("[TestApp] Failed to load optimized_scene_tree_watcher.gd");
            }
        }

        (st_receiver, col_receiver)
    }

    /// Insert event readers directly into an App (during init)
    fn insert_event_readers_direct(
        app: &mut App,
        readers: (
            std::sync::mpsc::Receiver<godot_bevy::plugins::scene_tree::SceneTreeEvent>,
            std::sync::mpsc::Receiver<godot_bevy::plugins::collisions::CollisionEvent>,
        ),
    ) {
        use godot_bevy::plugins::{
            collisions::CollisionEventReader, scene_tree::SceneTreeEventReader,
        };

        let (st_receiver, col_receiver) = readers;

        app.insert_non_send_resource(SceneTreeEventReader(st_receiver));
        app.insert_non_send_resource(CollisionEventReader(col_receiver));
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
    pub fn get_single<C: Component>(&mut self) -> C
    where
        C: Clone,
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
