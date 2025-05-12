use bevy::app::App;
use godot::prelude::*;
use std::sync::{mpsc::{channel, Sender}, Mutex};

use crate::{
    plugins::core::{GodotPhysicsFrame, GodotVisualFrame}, prelude::*, GodotPlugin
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

        {
            let (sender, receiver) = channel();
            let mut collision_watcher = CollisionWatcher::new_alloc();
            collision_watcher.bind_mut().notification_channel = Some(sender);
            collision_watcher.set_name("CollisionWatcher");
            self.base_mut().add_child(&collision_watcher);

            app.insert_non_send_resource(CollisionEventReader(receiver));
        }


        self.app = Some(app);
    }

    fn process(&mut self, _delta: f64) {
        use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        if let Some(app) = self.app.as_mut() {
            app.insert_resource(GodotVisualFrame);

            if let Err(e) = catch_unwind(AssertUnwindSafe(|| app.update())) {
                self.app = None;

                eprintln!("bevy app update panicked");
                resume_unwind(e);
            }

            app.world_mut().remove_resource::<GodotVisualFrame>();
        }
    }

    fn physics_process(&mut self, _delta: f64) {
        use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }

        if let Some(app) = self.app.as_mut() {
            app.insert_resource(GodotPhysicsFrame);

            if let Err(e) = catch_unwind(AssertUnwindSafe(|| app.update())) {
                self.app = None;

                eprintln!("bevy app update panicked");
                resume_unwind(e);
            }

            app.world_mut().remove_resource::<GodotPhysicsFrame>();
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
struct CollisionWatcher {
    base: Base<Node>,
    notification_channel: Option<Sender<CollisionEvent>>,
}

#[godot_api]
impl INode for CollisionWatcher {
    fn init(base: Base<Node>) -> Self {
        Self { base, notification_channel: None }
    }
}

impl CollisionWatcher {
    fn collision_event(
        &self,
        target: Gd<Node>,
        origin: Gd<Node>,
        event_type: CollisionEventType,
    ) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(CollisionEvent {
                event_type,
                origin: origin.instance_id(),
                target: target.instance_id(),
            });
        }
    }
}
