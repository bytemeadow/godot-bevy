use bevy::app::App;
use godot::prelude::*;
use std::sync::{Mutex, mpsc::channel};

use crate::watchers::input_watcher::GodotInputWatcher;
use crate::watchers::scene_tree_watcher::SceneTreeWatcher;
use crate::watchers::signal_watcher::GodotSignalWatcher;
use crate::{
    GodotPlugin,
    plugins::core::{GodotSignalReader, InputEventReader, PhysicsUpdate},
    prelude::*,
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

    fn register_signal_watcher(&mut self, app: &mut App) {
        let (sender, receiver) = channel();
        let mut signal_watcher = GodotSignalWatcher::new_alloc();
        signal_watcher.bind_mut().notification_channel = Some(sender);
        signal_watcher.set_name("SignalWatcher");
        self.base_mut().add_child(&signal_watcher);
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
        self.register_signal_watcher(&mut app);
        self.register_input_event_watcher(&mut app);
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

    fn physics_process(&mut self, _delta: f64) {
        use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        if let Some(app) = self.app.as_mut() {
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| {
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
