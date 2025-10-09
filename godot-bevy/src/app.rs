use crate::plugins::core::PrePhysicsUpdate;
use crate::watchers::collision_watcher::CollisionWatcher;
use crate::watchers::input_watcher::GodotInputWatcher;
use crate::watchers::scene_tree_watcher::SceneTreeWatcher;
use crate::{
    plugins::{
        collisions::CollisionEventReader,
        core::{PhysicsDelta, PhysicsUpdate},
        input::InputEventReader,
        scene_tree::SceneTreeEventReader,
        signals::{GodotSignalReader, GodotSignalSender},
    },
};
use bevy::app::App;
use godot::prelude::*;
use std::sync::OnceLock;
use std::sync::mpsc::channel;

// Stores the client's entrypoint (the function they decorated with the `#[bevy_app]` macro) at runtime
pub static BEVY_INIT_FUNC: OnceLock<Box<dyn Fn(&mut App) + Send + Sync>> = OnceLock::new();

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
        let (sender, receiver) = channel();
        let mut scene_tree_watcher = SceneTreeWatcher::new_alloc();
        scene_tree_watcher.bind_mut().notification_channel = Some(sender);
        scene_tree_watcher.set_name("SceneTreeWatcher");
        self.base_mut().add_child(&scene_tree_watcher);
        app.insert_non_send_resource(SceneTreeEventReader(receiver));
    }

    fn register_signal_system(&mut self, app: &mut App) {
        let (sender, receiver) = channel();
        // Create channel for Godot signals and insert as resources
        // Signals are connected directly using closures in the signals module
        app.insert_non_send_resource(GodotSignalSender(sender));
        app.insert_non_send_resource(GodotSignalReader(receiver));
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
        let (sender, receiver) = channel();
        let mut collision_watcher = CollisionWatcher::new_alloc();
        collision_watcher.bind_mut().notification_channel = Some(sender);
        collision_watcher.set_name("CollisionWatcher");
        self.base_mut().add_child(&collision_watcher);
        app.insert_non_send_resource(CollisionEventReader(receiver));
    }

    fn register_optimized_scene_tree_watcher(&mut self) {
        // Try to load the OptimizedSceneTreeWatcher GDScript class
        let mut resource_loader = godot::classes::ResourceLoader::singleton();
        if let Some(resource) =
            resource_loader.load("res://addons/godot-bevy/optimized_scene_tree_watcher.gd")
            && let Ok(mut script) = resource.try_cast::<godot::classes::GDScript>()
            && let Ok(instance) = script.try_instantiate(&[])
            && let Ok(mut node) = instance.try_to::<godot::obj::Gd<godot::classes::Node>>()
        {
            node.set_name("OptimizedSceneTreeWatcher");
            self.base_mut().add_child(&node);
            tracing::info!("Successfully registered OptimizedSceneTreeWatcher");
            return;
        }

        // If loading fails, log and continue without optimization
        tracing::info!("OptimizedSceneTreeWatcher not available - will use fallback method");
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

        let mut app = App::new();
        // NOTE: GodotPlugin is NO LONGER added automatically!
        // Users must add it (or specific plugins) in their #[bevy_app] function.
        // This gives full control over which plugins are loaded.

        // Call the init function - use instance function if set, otherwise global
        if let Some(ref instance_func) = self.instance_init_func {
            instance_func(&mut app);
        } else {
            let app_builder_func = BEVY_INIT_FUNC.get().unwrap();
            app_builder_func(&mut app);
        }

        // Finalize plugins before any further operations
        if app.plugins_state() != bevy::app::PluginsState::Cleaned {
            while app.plugins_state() == bevy::app::PluginsState::Adding {
                #[cfg(not(target_arch = "wasm32"))]
                bevy::tasks::tick_global_task_pools_on_main_thread();
            }

            app.finish();
            app.cleanup();
        }

        // Only register watchers if the corresponding plugins were added
        // Check if SceneTreeEventReader resource exists (added by scene tree plugin)
        if app.world().contains_non_send::<SceneTreeEventReader>() {
            self.register_scene_tree_watcher(&mut app);
            self.register_optimized_scene_tree_watcher();
        }

        // Check if GodotSignalReader resource was added (by signal plugin)
        if app.world().contains_non_send::<GodotSignalReader>() {
            self.register_signal_system(&mut app);
        }

        // Check if InputEventReader resource was added (by input plugin)
        if app.world().contains_non_send::<InputEventReader>() {
            self.register_input_event_watcher(&mut app);
        }

        // Check if CollisionEventReader resource was added (by collision plugin)
        if app.world().contains_non_send::<CollisionEventReader>() {
            self.register_collision_watcher(&mut app);
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

                #[cfg(feature = "trace_tracy")]
                // Indicate that rendering of a continuous frame has ended.
                tracing_tracy::client::Client::running()
                    .expect("client must be running")
                    .frame_mark();
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

                #[cfg(feature = "trace_tracy")]
                // Indicate that a physics frame has ended.
                tracing_tracy::client::Client::running()
                    .expect("client must be running")
                    .secondary_frame_mark(tracing_tracy::client::frame_name!("physics"));
            }))
        {
            self.app = None;

            eprintln!("bevy app physics update panicked");
            resume_unwind(e);
        }
    }
}
