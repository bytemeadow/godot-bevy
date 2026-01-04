use crate::plugins::core::PrePhysicsUpdate;
use crate::plugins::{
    collisions::CollisionMessageReader,
    core::{PhysicsDelta, PhysicsUpdate},
    input::InputEventReader,
    scene_tree::SceneTreeMessageReader,
};
use crate::watchers::collision_watcher::CollisionWatcher;
use crate::watchers::input_watcher::GodotInputWatcher;
use crate::watchers::scene_tree_watcher::SceneTreeWatcher;
use bevy_app::{App, PluginsState};
use bevy_ecs::message::Messages;
use godot::prelude::*;
use std::sync::OnceLock;
use std::sync::mpsc::channel;

// Stores the client's entrypoint (the function they decorated with the `#[bevy_app]` macro) at runtime
pub static BEVY_INIT_FUNC: OnceLock<Box<dyn Fn(&mut App) + Send + Sync>> = OnceLock::new();

// Configuration for BevyApp, set by the #[bevy_app] macro attributes
pub static BEVY_APP_CONFIG: OnceLock<BevyAppConfig> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub struct BevyAppConfig {
    pub scene_tree_auto_despawn_children: bool,
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct BevyApp {
    base: Base<Node>,
    app: Option<App>,
    // Optional per-instance init function (for tests)
    // If set, this takes precedence over the global BEVY_INIT_FUNC
    #[allow(clippy::type_complexity)]
    instance_init_func: Option<Box<dyn Fn(&mut App) + Send + Sync>>,
}

impl BevyApp {
    pub fn get_app(&self) -> Option<&App> {
        self.app.as_ref()
    }

    pub fn get_app_mut(&mut self) -> Option<&mut App> {
        self.app.as_mut()
    }

    /// Set a per-instance init function (for tests)
    /// This allows each BevyApp instance to have its own configuration
    pub fn set_instance_init_func(&mut self, func: Box<dyn Fn(&mut App) + Send + Sync>) {
        self.instance_init_func = Some(func);
    }

    fn register_scene_tree_watcher(&mut self, app: &mut App) {
        // Check if SceneTreeWatcher already exists (e.g., created by test framework)
        // If so, don't create a new one or replace the event reader
        if self.base().has_node("SceneTreeWatcher") {
            return;
        }

        let (sender, receiver) = channel();
        let mut scene_tree_watcher = SceneTreeWatcher::new_alloc();
        scene_tree_watcher.bind_mut().notification_channel = Some(sender);
        scene_tree_watcher.set_name("SceneTreeWatcher");
        self.base_mut().add_child(&scene_tree_watcher);
        app.insert_resource(SceneTreeMessageReader::new(receiver));
    }

    fn register_input_event_watcher(&mut self, app: &mut App) {
        let (sender, receiver) = channel();
        let mut input_event_watcher = GodotInputWatcher::new_alloc();
        input_event_watcher.bind_mut().notification_channel = Some(sender);
        input_event_watcher.set_name("InputEventWatcher");
        self.base_mut().add_child(&input_event_watcher);
        app.insert_non_send_resource(InputEventReader(receiver));
    }

    fn register_collision_watcher(&mut self, app: &mut App) {
        // Check if CollisionWatcher already exists (e.g., created by test framework)
        if self.base().has_node("CollisionWatcher") {
            return;
        }

        let (sender, receiver) = channel();
        let mut collision_watcher = CollisionWatcher::new_alloc();
        collision_watcher.bind_mut().notification_channel = Some(sender);
        collision_watcher.set_name("CollisionWatcher");
        self.base_mut().add_child(&collision_watcher);
        app.insert_resource(CollisionMessageReader::new(receiver));
    }

    fn register_optimized_scene_tree_watcher(&mut self) {
        // Check if the optimized watcher file exists before trying to load it
        // This prevents error logs when the file is not present (e.g., in examples)
        let path = "res://addons/godot-bevy/optimized_scene_tree_watcher.gd";

        // Use FileAccess to check if file actually exists (ResourceLoader.exists() may cache)
        if godot::classes::FileAccess::file_exists(&godot::builtin::GString::from(path)) {
            let mut resource_loader = godot::classes::ResourceLoader::singleton();

            // Try to load and instantiate the OptimizedSceneTreeWatcher GDScript class
            if let Some(resource) = resource_loader.load(path)
                && let Ok(mut script) = resource.try_cast::<godot::classes::GDScript>()
                && let Ok(instance) = script.try_instantiate(&[])
                && let Ok(mut node) = instance.try_to::<godot::obj::Gd<godot::classes::Node>>()
            {
                node.set_name("OptimizedSceneTreeWatcher");
                self.base_mut().add_child(&node);
                tracing::info!("Successfully registered OptimizedSceneTreeWatcher");
            } else {
                tracing::warn!(
                    "Failed to instantiate OptimizedSceneTreeWatcher - using fallback method"
                );
            }
        } else {
            tracing::debug!("OptimizedSceneTreeWatcher not available - using fallback method");
        }
    }

    #[cfg(debug_assertions)]
    fn register_optimized_bulk_operations(&mut self) {
        // Check if OptimizedBulkOperations already exists (e.g., loaded from tscn)
        if self.base().has_node("OptimizedBulkOperations") {
            return;
        }

        // Check if the bulk operations file exists before trying to load it
        let path = "res://addons/godot-bevy/optimized_bulk_operations.gd";

        // Use FileAccess to check if file actually exists
        if godot::classes::FileAccess::file_exists(&godot::builtin::GString::from(path)) {
            let mut resource_loader = godot::classes::ResourceLoader::singleton();

            // Try to load and instantiate the OptimizedBulkOperations GDScript class
            if let Some(resource) = resource_loader.load(path)
                && let Ok(mut script) = resource.try_cast::<godot::classes::GDScript>()
                && let Ok(instance) = script.try_instantiate(&[])
                && let Ok(mut node) = instance.try_to::<godot::obj::Gd<godot::classes::Node>>()
            {
                node.set_name("OptimizedBulkOperations");
                self.base_mut().add_child(&node);
                tracing::info!("Successfully registered OptimizedBulkOperations");
            } else {
                tracing::warn!(
                    "Failed to instantiate OptimizedBulkOperations - bulk operations unavailable"
                );
            }
        } else {
            tracing::debug!("OptimizedBulkOperations not available");
        }
    }
}

#[godot_api]
impl INode for BevyApp {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            app: Default::default(),
            instance_init_func: None,
        }
    }

    fn ready(&mut self) {
        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        // Register bulk operations helper (used by transform sync, input systems, and benchmarks)
        // Only registered in debug builds where bulk FFI is faster than individual calls
        // This is done before the init check so benchmarks can use it without a full Bevy app
        #[cfg(debug_assertions)]
        self.register_optimized_bulk_operations();

        // Register the optimized scene tree watcher early (before init check)
        // This allows benchmarks and tools to use the watcher even without a full Bevy app
        self.register_optimized_scene_tree_watcher();

        // If no init function is provided, don't initialize the Bevy app.
        // This allows the node to exist purely for GDScript utility methods (e.g., bulk transforms)
        // while tests create their own BevyApp instances with set_instance_init_func().
        let has_init = self.instance_init_func.is_some() || BEVY_INIT_FUNC.get().is_some();
        if !has_init {
            return;
        }

        let mut app = App::new();

        // Configure GodotCorePlugins based on #[bevy_app] attribute configuration
        let config = BEVY_APP_CONFIG.get().copied().unwrap_or(BevyAppConfig {
            scene_tree_auto_despawn_children: true,
        });

        // Manually add core plugins with configuration
        app.add_plugins(crate::plugins::core::GodotBaseCorePlugin)
            .add_plugins(crate::plugins::scene_tree::GodotSceneTreePlugin {
                auto_despawn_children: config.scene_tree_auto_despawn_children,
            });

        // Call the init function - use instance function if set, otherwise global
        if let Some(ref instance_func) = self.instance_init_func {
            instance_func(&mut app);
        } else if let Some(app_builder_func) = BEVY_INIT_FUNC.get() {
            app_builder_func(&mut app);
        }

        // Create watchers BEFORE app.finish() so PreStartup systems can find them
        // Check which plugins were added by looking for their resources/events

        // Scene tree plugin check - look for Messages<SceneTreeMessage>
        use crate::plugins::scene_tree::SceneTreeMessage;
        if app
            .world()
            .contains_resource::<Messages<SceneTreeMessage>>()
        {
            self.register_scene_tree_watcher(&mut app);
            self.register_optimized_scene_tree_watcher();
        }

        // Collision plugin check - similar approach
        use crate::plugins::collisions::CollisionMessage;
        if app
            .world()
            .contains_resource::<Messages<CollisionMessage>>()
        {
            self.register_collision_watcher(&mut app);
        }

        // Input event plugin check - check for KeyboardInput as a marker
        use crate::plugins::input::KeyboardInput;
        if app.world().contains_resource::<Messages<KeyboardInput>>() {
            self.register_input_event_watcher(&mut app);
        }

        // Finalize plugins - PreStartup systems will now find the watchers
        if app.plugins_state() != PluginsState::Cleaned {
            while app.plugins_state() == PluginsState::Adding {
                #[cfg(not(target_arch = "wasm32"))]
                bevy_tasks::tick_global_task_pools_on_main_thread();
            }

            app.finish();
            app.cleanup();
        }

        app.init_resource::<PhysicsDelta>();
        self.app = Some(app);
    }

    fn process(&mut self, _delta: f64) {
        use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        if let Some(app) = self.app.as_mut()
            && let Err(e) = catch_unwind(AssertUnwindSafe(|| {
                // Run the full Bevy update cycle - much simpler!
                app.update();

                // Mark frame end for profiling
                crate::profiling::frame_mark();
            }))
        {
            self.app = None;

            eprintln!("bevy app update panicked");
            resume_unwind(e);
        }
    }

    fn physics_process(&mut self, delta: f32) {
        use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        if let Some(app) = self.app.as_mut()
            && let Err(e) = catch_unwind(AssertUnwindSafe(|| {
                // Update physics delta resource with Godot's delta
                app.world_mut().resource_mut::<PhysicsDelta>().delta_seconds = delta;

                // Run only our physics-specific schedule
                app.world_mut().run_schedule(PrePhysicsUpdate);
                app.world_mut().run_schedule(PhysicsUpdate);

                // Mark physics frame end for profiling
                crate::profiling::secondary_frame_mark("physics");
            }))
        {
            self.app = None;

            eprintln!("bevy app physics update panicked");
            resume_unwind(e);
        }
    }
}
