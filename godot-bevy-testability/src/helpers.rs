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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};

/// Test-only frame controller that synchronizes Godot frames with Bevy updates
/// This mimics what BevyApp does in production but gives tests control over timing
#[derive(GodotClass)]
#[class(base=Node)]
pub struct TestFrameController {
    base: Base<Node>,
    /// Shared state for coordinating with the test environment
    frame_state: Option<Arc<TestFrameState>>,
    /// Reference to the test context (stored as pointer since we can't store &mut)
    test_ctx_ptr: Option<*mut BevyGodotTestContext>,
}

#[derive(Default)]
pub struct TestFrameState {
    /// Current frame number
    pub frame_count: AtomicU64,
    /// Whether the frame has been processed
    pub frame_processed: AtomicBool,
}

#[godot_api]
impl INode for TestFrameController {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            frame_state: None,
            test_ctx_ptr: None,
        }
    }

    fn process(&mut self, _delta: f64) {
        if let Some(ref state) = self.frame_state {
            // Always increment the frame counter when _process is called
            // This lets us track that Godot is actually processing frames
            let frame_num = state.frame_count.fetch_add(1, Ordering::AcqRel);
            godot_print!(
                "[TestFrameController] _process() called - frame {}",
                frame_num
            );

            // Mark that a frame was processed so wait_for_next_tick knows
            state.frame_processed.store(true, Ordering::Release);
        }
    }

    fn ready(&mut self) {
        godot_print!("[TestFrameController] _ready() called - enabling process");
        // Enable processing so we receive _process callbacks
        self.base_mut().set_process(true);
    }

    fn enter_tree(&mut self) {
        godot_print!("[TestFrameController] _enter_tree() called");
        // Try enabling process here too, in case _ready isn't called
        self.base_mut().set_process(true);
        godot_print!("[TestFrameController] Process enabled in _enter_tree");
    }
}

#[godot_api]
impl TestFrameController {
    #[func]
    fn set_frame_state(&mut self, state_ptr: u64) {
        // Reconstruct the Arc from the pointer
        unsafe {
            let arc = Arc::from_raw(state_ptr as *const TestFrameState);
            self.frame_state = Some(arc.clone());
            // Don't drop the Arc, we're just borrowing it
            std::mem::forget(arc);
        }
    }

    #[func]
    fn set_test_context(&mut self, ctx_ptr: u64) {
        self.test_ctx_ptr = Some(ctx_ptr as *mut BevyGodotTestContext);
    }
}

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

    // Set up TestFrameController for frame synchronization
    let frame_state = Arc::new(TestFrameState::default());
    let mut frame_controller = TestFrameController::new_alloc();
    frame_controller.set_name("TestFrameController");

    // Pass the shared state and context pointer to the controller
    let state_ptr = Arc::into_raw(frame_state.clone()) as u64;
    let ctx_ptr = ctx as *mut BevyGodotTestContext as u64;
    frame_controller.bind_mut().set_frame_state(state_ptr);
    frame_controller.bind_mut().set_test_context(ctx_ptr);

    root.add_child(&frame_controller.clone().upcast::<Node>());

    // Note: The GodotSceneTreePlugin will automatically connect the scene tree signals
    // when it runs its connect_scene_tree system in PreStartup, since we've placed the
    // SceneTreeWatcher at the expected path: /root/BevyAppSingleton/SceneTreeWatcher

    TestEnvironment {
        scene_tree,
        bevy_app_singleton,
        scene_tree_watcher,
        collision_watcher,
        scene_tree_event_sender: scene_tree_sender,
        frame_controller,
        frame_state,
    }
}

/// Holds references to the test environment components
pub struct TestEnvironment {
    pub scene_tree: Gd<godot::classes::SceneTree>,
    pub bevy_app_singleton: Gd<Node>,
    pub scene_tree_watcher: Gd<SceneTreeWatcher>,
    pub collision_watcher: Gd<CollisionWatcher>,
    pub scene_tree_event_sender: Sender<SceneTreeEvent>,
    pub frame_controller: Gd<TestFrameController>,
    pub frame_state: Arc<TestFrameState>,
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

    /// Wait for the next Godot frame to process and automatically update Bevy
    /// This properly synchronizes Godot's frame processing with Bevy's update cycle
    pub fn wait_for_next_tick(&mut self, ctx: &mut BevyGodotTestContext) {
        let starting_frame = self.frame_state.frame_count.load(Ordering::Acquire);
        godot_print!(
            "[wait_for_next_tick] Waiting for Godot to process frame {} -> {}",
            starting_frame,
            starting_frame + 1
        );

        // Reset the processed flag
        self.frame_state
            .frame_processed
            .store(false, Ordering::Release);

        // Wait for Godot to actually process a frame
        // The TestFrameController's _process() will increment frame_count
        let timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();

        while self.frame_state.frame_count.load(Ordering::Acquire) == starting_frame {
            if start.elapsed() > timeout {
                godot_print!("[wait_for_next_tick] WARNING: Timeout waiting for Godot frame!");
                break;
            }
            // Give Godot time to process
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        let current_frame = self.frame_state.frame_count.load(Ordering::Acquire);
        godot_print!(
            "[wait_for_next_tick] Godot processed frame {}",
            current_frame
        );

        // Now update the Bevy app to process any events that Godot generated
        godot_print!("[wait_for_next_tick] Updating Bevy app");
        ctx.app.update();

        godot_print!("[wait_for_next_tick] Frame {} completed", current_frame);
    }

    /// Manually trigger a Godot frame without waiting
    /// Useful for tests that need fine control over timing
    pub fn trigger_frame(&mut self) {
        self.scene_tree.emit_signal("process_frame", &[]);
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
