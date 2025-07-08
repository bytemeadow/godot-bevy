use bevy::app::App;
use godot::prelude::*;
use std::sync::{Mutex, mpsc::channel};

use crate::watchers::collision_watcher::CollisionWatcher;
use crate::watchers::input_watcher::GodotInputWatcher;
use crate::watchers::scene_tree_watcher::SceneTreeWatcher;
use crate::{
    GodotPlugin,
    plugins::{
        collisions::CollisionEventReader,
        core::{PhysicsDelta, PhysicsUpdate},
        input::InputEventReader,
        scene_tree::SceneTreeEventReader,
        signals::{GodotSignalReader, GodotSignalSender},
    },
};

lazy_static::lazy_static! {
    #[doc(hidden)]
    pub static ref BEVY_INIT_FUNC: Mutex<Option<Box<dyn Fn(&mut App) + Send>>> =
            Mutex::new(None);
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct BevyApp {
    base: Base<Node>,
    app: Option<App>,
}

impl BevyApp {
    pub fn get_app(&self) -> Option<&App> {
        self.app.as_ref()
    }

    pub fn get_app_mut(&mut self) -> Option<&mut App> {
        self.app.as_mut()
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
}

#[godot_api]
impl INode for BevyApp {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            app: Default::default(),
        }
    }

    fn ready(&mut self) {
        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        let mut app = App::new();
        app.add_plugins(GodotPlugin);

        (BEVY_INIT_FUNC.lock().unwrap().as_mut().unwrap())(&mut app);

        self.register_scene_tree_watcher(&mut app);
        self.register_signal_system(&mut app);
        self.register_input_event_watcher(&mut app);
        self.register_collision_watcher(&mut app);
        app.init_resource::<PhysicsDelta>();
        self.app = Some(app);
    }

    fn process(&mut self, _delta: f64) {
        use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        if let Some(app) = self.app.as_mut() {
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| {
                // Run the full Bevy update cycle - much simpler!
                app.update();
            })) {
                self.app = None;

                eprintln!("bevy app update panicked");
                resume_unwind(e);
            }
        }
    }

    fn physics_process(&mut self, delta: f32) {
        use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        if let Some(app) = self.app.as_mut() {
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| {
                // Update physics delta resource with Godot's delta
                app.world_mut().resource_mut::<PhysicsDelta>().delta_seconds = delta;

                // Run only our physics-specific schedule
                app.world_mut().run_schedule(PhysicsUpdate);
            })) {
                self.app = None;

                eprintln!("bevy app physics update panicked");
                resume_unwind(e);
            }
        }
    }
}
