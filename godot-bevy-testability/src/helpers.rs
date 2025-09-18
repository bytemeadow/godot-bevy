//! Test helper utilities for godot-bevy integration testing
//!
//! This module provides utilities to set up test environments that closely mimic
//! the real godot-bevy runtime, including scene tree watchers and proper event flow.

use crate::BevyGodotTestContext;
use godot::prelude::*;
use godot_bevy::plugins::collisions::CollisionEventReader;
use godot_bevy::plugins::scene_tree::{SceneTreeEvent, SceneTreeEventReader};
use godot_bevy::watchers::collision_watcher::CollisionWatcher;
use godot_bevy::watchers::scene_tree_watcher::SceneTreeWatcher;
use std::sync::mpsc::{Sender, channel};

/// Try to create a BevyApp instance if the class is registered
fn try_create_bevy_app() -> Option<Gd<godot_bevy::app::BevyApp>> {
    // For now, don't try to create BevyApp in tests as it's complex to set up properly
    // The class registration works but the BevyApp expects specific initialization
    None
}

/// Sets up a minimal BevyApp-like environment with watchers in the scene tree
/// This allows testing scene tree integration without requiring the full BevyApp node
pub fn setup_test_environment_with_watchers(ctx: &mut BevyGodotTestContext) -> TestEnvironment {
    // Get the scene tree
    let scene_tree = unsafe {
        let obj_ptr = ctx.scene_tree_ptr as godot::sys::GDExtensionObjectPtr;
        godot::prelude::Gd::<godot::classes::SceneTree>::from_sys_init_opt(|ptr| {
            *(ptr as *mut godot::sys::GDExtensionObjectPtr) = obj_ptr;
        })
        .expect("Failed to get scene tree")
    };

    // Try to create the actual BevyApp node now that we have class registration
    // If it fails, fall back to a regular Node
    let mut bevy_app_singleton = match try_create_bevy_app() {
        Some(app) => app.upcast::<Node>(),
        None => {
            // Fall back to regular Node if BevyApp class isn't available
            let mut node = Node::new_alloc();
            node.set_name("BevyAppSingleton");
            node
        }
    };

    // Add to root
    let mut root = scene_tree.get_root().unwrap();
    root.add_child(&bevy_app_singleton.clone().upcast::<Node>());

    // Set up SceneTreeWatcher
    let (scene_tree_sender, scene_tree_receiver) = channel();
    let mut scene_tree_watcher = SceneTreeWatcher::new_alloc();
    scene_tree_watcher.bind_mut().notification_channel = Some(scene_tree_sender.clone());
    scene_tree_watcher.set_name("SceneTreeWatcher");
    bevy_app_singleton.add_child(&scene_tree_watcher.clone().upcast::<Node>());

    // Register the receiver in the app
    ctx.app
        .insert_non_send_resource(SceneTreeEventReader(scene_tree_receiver));

    // Set up CollisionWatcher
    let (collision_sender, collision_receiver) = channel();
    let mut collision_watcher = CollisionWatcher::new_alloc();
    collision_watcher.bind_mut().notification_channel = Some(collision_sender);
    collision_watcher.set_name("CollisionWatcher");
    bevy_app_singleton.add_child(&collision_watcher.clone().upcast::<Node>());

    // Register the collision receiver
    ctx.app
        .insert_non_send_resource(CollisionEventReader(collision_receiver));

    // Note: The GodotSceneTreePlugin will automatically connect the scene tree signals
    // when it runs its connect_scene_tree system in PreStartup, since we've placed the
    // SceneTreeWatcher at the expected path: /root/BevyAppSingleton/SceneTreeWatcher

    TestEnvironment {
        scene_tree,
        bevy_app_singleton,
        scene_tree_watcher,
        collision_watcher,
        scene_tree_event_sender: scene_tree_sender,
    }
}

/// Holds references to the test environment components
pub struct TestEnvironment {
    pub scene_tree: Gd<godot::classes::SceneTree>,
    pub bevy_app_singleton: Gd<Node>,
    pub scene_tree_watcher: Gd<SceneTreeWatcher>,
    pub collision_watcher: Gd<CollisionWatcher>,
    pub scene_tree_event_sender: Sender<SceneTreeEvent>,
}

impl TestEnvironment {
    /// Add a node to the scene tree (as a child of root)
    pub fn add_node_to_scene<T: Inherits<Node>>(&mut self, node: Gd<T>) -> Gd<T> {
        let mut root = self.scene_tree.get_root().unwrap();
        root.add_child(&node.clone().upcast());
        node
    }

    /// Manually send a scene tree event (useful for testing specific scenarios)
    pub fn send_scene_tree_event(&self, event: SceneTreeEvent) {
        let _ = self.scene_tree_event_sender.send(event);
    }
}

/// Extension trait to simplify setup in tests
pub trait BevyGodotTestContextExt {
    /// Initialize the test environment with full scene tree integration
    fn setup_full_integration(&mut self) -> TestEnvironment;
}

impl BevyGodotTestContextExt for BevyGodotTestContext {
    fn setup_full_integration(&mut self) -> TestEnvironment {
        // Initialize base resources
        self.initialize_godot_bevy_resources();

        // IMPORTANT: Set up the environment with watchers BEFORE adding plugins
        // This ensures the watchers exist at the expected paths when PreStartup systems run
        let env = setup_test_environment_with_watchers(self);

        // Add required plugins AFTER watchers are in place
        self.app
            .add_plugins(godot_bevy::plugins::core::GodotBaseCorePlugin);
        self.app
            .add_plugins(godot_bevy::plugins::scene_tree::GodotSceneTreePlugin::default());

        env
    }
}
