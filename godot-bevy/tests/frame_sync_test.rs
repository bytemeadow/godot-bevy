//! Test suite for verifying the frame synchronization helper works correctly
//!
//! This is a simple test to ensure our test framework properly synchronizes
//! Godot frame processing with Bevy updates.

use godot::prelude::*;
use godot_bevy_testability::*;

/// Simple node that tracks when its callbacks are invoked
#[derive(GodotClass)]
#[class(base=Node)]
struct FrameTestNode {
    base: Base<Node>,
    enter_tree_called: bool,
    ready_called: bool,
    process_called_count: u32,
}

#[godot_api]
impl INode for FrameTestNode {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            enter_tree_called: false,
            ready_called: false,
            process_called_count: 0,
        }
    }

    fn enter_tree(&mut self) {
        godot_print!("[FrameTestNode] _enter_tree() called");
        self.enter_tree_called = true;
    }

    fn ready(&mut self) {
        godot_print!("[FrameTestNode] _ready() called");
        self.ready_called = true;
        // Enable processing
        self.base_mut().set_process(true);
    }

    fn process(&mut self, _delta: f64) {
        self.process_called_count += 1;
        godot_print!(
            "[FrameTestNode] _process() called (count: {})",
            self.process_called_count
        );
    }
}

#[godot_api]
impl FrameTestNode {
    #[func]
    fn was_enter_tree_called(&self) -> bool {
        self.enter_tree_called
    }

    #[func]
    fn was_ready_called(&self) -> bool {
        self.ready_called
    }

    #[func]
    fn get_process_count(&self) -> u32 {
        self.process_called_count
    }
}

/// Test that wait_for_next_tick properly processes Godot callbacks
pub fn test_frame_sync_basic(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    godot_print!("\n=== Testing Frame Synchronization ===");

    let mut env = ctx.setup_full_integration();

    // Create a test node
    let mut test_node = FrameTestNode::new_alloc();
    test_node.set_name("TestNode");

    // Add it to the scene
    godot_print!("Adding test node to scene...");
    env.add_node_to_scene(test_node.clone());

    // The node should not have its callbacks called yet
    assert!(
        !test_node.bind().was_enter_tree_called(),
        "enter_tree should not be called yet"
    );

    // Wait for a frame - this should trigger _enter_tree and _ready
    godot_print!("Waiting for first frame...");
    env.wait_for_next_tick(ctx);

    // Check that callbacks were invoked
    godot_print!("Checking if callbacks were invoked...");
    {
        let binding = test_node.bind();
        if !binding.was_enter_tree_called() {
            return Err(TestError::assertion(
                "_enter_tree() was not called after wait_for_next_tick",
            ));
        }
        if !binding.was_ready_called() {
            return Err(TestError::assertion(
                "_ready() was not called after wait_for_next_tick",
            ));
        }
    } // binding is dropped here

    godot_print!("✓ Callbacks were properly invoked");

    // Wait for another frame to see if _process is called
    godot_print!("Waiting for second frame...");
    env.wait_for_next_tick(ctx);

    let process_count = test_node.bind().get_process_count();
    godot_print!("Process was called {} times", process_count);

    if process_count == 0 {
        return Err(TestError::assertion(
            "_process() was not called after enabling processing",
        ));
    }

    godot_print!("✓ Frame synchronization is working correctly!");

    // Cleanup
    test_node.queue_free();

    Ok(())
}

/// Test that multiple frames advance correctly
pub fn test_frame_sync_multiple(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    godot_print!("\n=== Testing Multiple Frame Advances ===");

    let mut env = ctx.setup_full_integration();

    // Create a test node that counts process calls
    let mut test_node = FrameTestNode::new_alloc();
    test_node.set_name("CounterNode");
    env.add_node_to_scene(test_node.clone());

    // Process first frame to initialize
    env.wait_for_next_tick(ctx);

    // Process multiple frames and verify counter increases
    for i in 1..=3 {
        godot_print!("Processing frame {}...", i);
        env.wait_for_next_tick(ctx);

        let count = test_node.bind().get_process_count();
        godot_print!("Process count after frame {}: {}", i, count);

        if count != i {
            return Err(TestError::assertion(format!(
                "Expected process count {}, got {}",
                i, count
            )));
        }
    }

    godot_print!("✓ Multiple frames processed correctly!");

    // Cleanup
    test_node.queue_free();

    Ok(())
}

bevy_godot_test_main! {
    test_frame_sync_basic,
    test_frame_sync_multiple,
}
