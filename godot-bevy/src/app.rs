use crate::plugins::{
    collisions::CollisionMessageReader, input::InputEventReader, scene_tree::SceneTreeMessageReader,
};
use crate::watchers::collision_watcher::CollisionWatcher;
use crate::watchers::input_watcher::GodotInputWatcher;
use crate::watchers::scene_tree_watcher::SceneTreeWatcher;
use bevy_app::{App, PluginsState};
use bevy_ecs::message::Messages;
use crossbeam_channel::unbounded;
use godot::prelude::*;
use std::sync::OnceLock;

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
    // True after the startup schedules have run (lifetime flag, set once).
    started: bool,
    // True from the first physics callback of a frame until the end of process().
    // Guards the prefix from running twice in frames with >= 1 physics steps.
    prefix_done_this_frame: bool,
    // Physics steps run in the current render frame; reported via bevy_frame_complete.
    #[cfg(feature = "test-frame-signal")]
    physics_steps_this_frame: u32,
}

impl BevyApp {
    pub fn get_app(&self) -> Option<&App> {
        self.app.as_ref()
    }

    /// In production the split-Main driver owns the update loop; calling
    /// `app.update()` directly is valid for testing but must not be mixed
    /// with the production driver in the same frame.
    pub fn get_app_mut(&mut self) -> Option<&mut App> {
        self.app.as_mut()
    }

    /// Set a per-instance init function (for tests)
    /// This allows each BevyApp instance to have its own configuration
    pub fn set_instance_init_func(&mut self, func: Box<dyn Fn(&mut App) + Send + Sync>) {
        self.instance_init_func = Some(func);
    }

    /// Tear down the Bevy app and remove all watchers.
    pub fn teardown(&mut self) {
        self.app = None;
        for name in &[
            "SceneTreeWatcher",
            "OptimizedSceneTreeWatcher",
            "CollisionWatcher",
            "InputEventWatcher",
        ] {
            if let Some(mut child) = self.base().try_get_node_as::<godot::classes::Node>(*name) {
                self.base_mut().remove_child(&child);
                child.queue_free();
            }
        }
    }

    /// Initialize the Bevy app on an already-in-tree node.
    /// No-ops if neither `set_instance_init_func()` nor `#[bevy_app]` has been set.
    pub fn initialize(&mut self) {
        let has_init = self.instance_init_func.is_some() || BEVY_INIT_FUNC.get().is_some();
        if !has_init {
            return;
        }
        self.teardown();
        self.do_initialize();
    }

    fn do_initialize(&mut self) {
        // Reset per-app state so that re-initialization (e.g. the itest harness
        // calling teardown -> do_initialize) runs startup fresh.
        self.started = false;
        self.prefix_done_this_frame = false;

        let mut app = App::new();

        let config = BEVY_APP_CONFIG.get().copied().unwrap_or(BevyAppConfig {
            scene_tree_auto_despawn_children: true,
        });

        app.add_plugins(crate::plugins::core::GodotBaseCorePlugin)
            .add_plugins(crate::plugins::scene_tree::GodotSceneTreePlugin {
                auto_despawn_children: config.scene_tree_auto_despawn_children,
            });

        if let Some(ref instance_func) = self.instance_init_func {
            instance_func(&mut app);
        } else if let Some(app_builder_func) = BEVY_INIT_FUNC.get() {
            app_builder_func(&mut app);
        }

        use crate::plugins::scene_tree::SceneTreeMessage;
        if app
            .world()
            .contains_resource::<Messages<SceneTreeMessage>>()
        {
            self.register_scene_tree_watcher(&mut app);
            self.register_optimized_scene_tree_watcher();
        }

        use crate::plugins::collisions::CollisionStarted;
        if app
            .world()
            .contains_resource::<Messages<CollisionStarted>>()
        {
            self.register_collision_watcher(&mut app);
        }

        use crate::plugins::input::GodotKeyboardInput;
        if app
            .world()
            .contains_resource::<Messages<GodotKeyboardInput>>()
        {
            self.register_input_event_watcher(&mut app);
        }

        #[cfg(debug_assertions)]
        self.cache_bulk_operations(&mut app);

        if app.plugins_state() != PluginsState::Cleaned {
            while app.plugins_state() == PluginsState::Adding {
                #[cfg(not(target_arch = "wasm32"))]
                bevy_tasks::tick_global_task_pools_on_main_thread();
            }

            app.finish();
            app.cleanup();
        }

        // godot-bevy drives Main directly (prefix + N fixed steps + suffix); a
        // secondary SubApp would never be extracted or updated. Fail loud at build
        // time rather than silently skip.
        assert!(
            app.sub_apps().sub_apps.is_empty(),
            "godot-bevy drives Main itself; a secondary SubApp would never be updated"
        );

        self.app = Some(app);
    }

    fn register_scene_tree_watcher(&mut self, app: &mut App) {
        // Check if SceneTreeWatcher already exists (e.g., created by test framework)
        // If so, don't create a new one or replace the event reader
        if self.base().has_node("SceneTreeWatcher") {
            return;
        }

        let (sender, receiver) = unbounded();
        let mut scene_tree_watcher = SceneTreeWatcher::new_alloc();
        scene_tree_watcher.bind_mut().notification_channel = Some(sender);
        scene_tree_watcher.set_name("SceneTreeWatcher");
        self.base_mut().add_child(&scene_tree_watcher);
        app.insert_resource(SceneTreeMessageReader::new(receiver));
    }

    fn register_input_event_watcher(&mut self, app: &mut App) {
        let (sender, receiver) = unbounded();
        let mut input_event_watcher = GodotInputWatcher::new_alloc();
        input_event_watcher.bind_mut().notification_channel = Some(sender);
        input_event_watcher.set_name("InputEventWatcher");
        self.base_mut().add_child(&input_event_watcher);
        app.insert_non_send(InputEventReader(receiver));
    }

    fn register_collision_watcher(&mut self, app: &mut App) {
        // Check if CollisionWatcher already exists (e.g., created by test framework)
        if self.base().has_node("CollisionWatcher") {
            return;
        }

        let (sender, receiver) = unbounded();
        let mut collision_watcher = CollisionWatcher::new_alloc();
        collision_watcher.bind_mut().notification_channel = Some(sender);
        collision_watcher.set_name("CollisionWatcher");
        self.base_mut().add_child(&collision_watcher);
        app.insert_resource(CollisionMessageReader::new(receiver));
    }

    fn register_optimized_scene_tree_watcher(&mut self) {
        if self.base().has_node("OptimizedSceneTreeWatcher") {
            return;
        }

        // Check if the optimized watcher file exists before trying to load it
        // This prevents error logs when the file is not present (e.g., in examples)
        let path = "res://addons/godot-bevy/optimized_scene_tree_watcher.gd";

        // Use FileAccess to check if file actually exists (ResourceLoader.exists() may cache)
        if godot::classes::FileAccess::file_exists(path) {
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

    /// Register the OptimizedBulkOperations GDScript node as a child.
    /// This is called early (before app creation) so benchmarks can use it.
    #[cfg(debug_assertions)]
    fn register_optimized_bulk_operations(&mut self) {
        // Check if OptimizedBulkOperations already exists (e.g., loaded from tscn)
        if self.base().has_node("OptimizedBulkOperations") {
            return;
        }

        // Check if the bulk operations file exists before trying to load it
        let path = "res://addons/godot-bevy/optimized_bulk_operations.gd";

        // Use FileAccess to check if file actually exists
        if godot::classes::FileAccess::file_exists(path) {
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

    /// Cache the OptimizedBulkOperations node reference in the Bevy app.
    /// This avoids repeated scene tree lookups every frame.
    #[cfg(debug_assertions)]
    fn cache_bulk_operations(&self, app: &mut App) {
        use crate::interop::BulkOperationsCache;

        if let Some(node) = self
            .base()
            .get_node_or_null("OptimizedBulkOperations")
            .map(|n| n.upcast::<godot::classes::Object>())
        {
            app.insert_non_send(BulkOperationsCache::new(node));
            tracing::debug!("Cached OptimizedBulkOperations node reference");
        } else {
            // Initialize empty cache so systems don't need to check for resource existence
            app.init_non_send::<BulkOperationsCache>();
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
            started: false,
            prefix_done_this_frame: false,
            #[cfg(feature = "test-frame-signal")]
            physics_steps_this_frame: 0,
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

        let has_init = self.instance_init_func.is_some() || BEVY_INIT_FUNC.get().is_some();
        if !has_init {
            return;
        }

        self.do_initialize();
    }

    fn process(&mut self, _delta: f64) {
        use crate::plugins::fixed_schedule::{run_main_suffix, run_preamble};
        use std::panic::{AssertUnwindSafe, catch_unwind};

        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        let need_startup = !self.started;
        let need_prefix = !self.prefix_done_this_frame;

        // Run the frame's suffix (and startup/prefix fallback). Capture any panic
        // so the end-of-frame signal still fires before we propagate it.
        let result = self.app.as_mut().map(|app| {
            catch_unwind(AssertUnwindSafe(|| {
                let world = app.world_mut();
                run_preamble(world, need_startup, need_prefix);
                run_main_suffix(world);
                world.clear_trackers();
                crate::profiling::frame_mark();
            }))
        });

        self.started = true;
        self.prefix_done_this_frame = false;

        // Emit unconditionally: after suffix+clear, before resume_unwind, and even
        // when app == None. A panicking/torn-down frame still resumes its awaiter,
        // which fails cleanly rather than hanging the suite.
        #[cfg(feature = "test-frame-signal")]
        {
            let steps = self.physics_steps_this_frame as i64;
            self.physics_steps_this_frame = 0;
            self.signals().bevy_frame_complete().emit(steps);
        }

        if let Some(Err(e)) = result {
            self.app = None;
            eprintln!("bevy app update panicked");
            std::panic::resume_unwind(e);
        }
    }

    fn physics_process(&mut self, delta: f32) {
        use crate::plugins::fixed_schedule::{run_godot_fixed_main, run_preamble};
        use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        #[cfg(feature = "test-frame-signal")]
        {
            self.physics_steps_this_frame += 1;
        }

        let need_startup = !self.started;
        let need_prefix = !self.prefix_done_this_frame;

        // Godot guarantees _process fires every render frame (main.cpp:4935), so the
        // prefix set here will always be followed by _process running the suffix.
        if let Some(app) = self.app.as_mut()
            && let Err(e) = catch_unwind(AssertUnwindSafe(|| {
                let world = app.world_mut();
                run_preamble(world, need_startup, need_prefix);
                run_godot_fixed_main(world, std::time::Duration::from_secs_f64(delta as f64));
                crate::profiling::secondary_frame_mark("physics");
            }))
        {
            self.app = None;
            eprintln!("bevy app physics update panicked");
            resume_unwind(e);
        }
        self.started = true;
        self.prefix_done_this_frame = true;
    }
}

#[cfg(feature = "test-frame-signal")]
#[godot_api]
impl BevyApp {
    /// Emitted at the end of every render frame, after the Bevy suffix + clear_trackers.
    /// Carries the number of physics steps that ran this frame. Test harness only.
    #[signal]
    fn bevy_frame_complete(physics_steps: i64);
}
