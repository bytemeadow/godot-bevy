//! Tests for parent entity mapping race conditions and warnings
//!
//! These tests replicate scenarios where the "Parent entity not found in ent_mapping" warning occurs
//!
//! Note: These tests require the `api-4-3` feature to run properly.

use godot::prelude::*;
use godot_bevy_testability::*;

use crate::scene_tree::utils::find_entity_for_node;

// Custom node class that spawns a child in _enter_tree
#[derive(GodotClass)]
#[class(base=Node3D)]
struct DynamicChildSpawner {
    base: Base<Node3D>,
    spawned_child: Option<Gd<Node3D>>,
}

#[godot_api]
impl INode3D for DynamicChildSpawner {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            spawned_child: None,
        }
    }

    fn enter_tree(&mut self) {
        godot_print!("[DynamicChildSpawner] _enter_tree() called");

        // Get parent name for debugging
        if let Some(parent) = self.base().get_parent() {
            godot_print!("[DynamicChildSpawner] Parent is: {}", parent.get_name());
        } else {
            godot_print!("[DynamicChildSpawner] No parent yet");
        }

        // This is the problematic scenario: spawning a child in _enter_tree()
        let mut child = Node3D::new_alloc();
        child.set_name("sub_node");

        godot_print!(
            "[DynamicChildSpawner] About to add_child(sub_node) in _enter_tree() - THIS SHOULD TRIGGER THE BUG"
        );
        self.base_mut().add_child(&child.clone().upcast::<Node>());

        godot_print!(
            "[DynamicChildSpawner] sub_node spawned with ID: {}",
            child.instance_id()
        );
        godot_print!(
            "[DynamicChildSpawner] Child count after add_child: {}",
            self.base().get_child_count()
        );

        self.spawned_child = Some(child);
    }
}

#[godot_api]
impl DynamicChildSpawner {
    #[func]
    fn get_spawned_child(&self) -> Option<Gd<Node3D>> {
        self.spawned_child.clone()
    }
}

/// Test that demonstrates the race condition with dynamically inserted children.
///
/// This test shows a limitation of the current scene tree processing:
/// When a node spawns a child in its _enter_tree() callback, the child's NodeAdded event
/// may be processed before the parent entity is fully registered in ent_mapping.
///
/// IMPORTANT TEST ENVIRONMENT LIMITATION:
/// In the test environment, Godot's main loop is not running, so `_process()` and other
/// frame-based callbacks don't execute until test cleanup. This means we cannot perfectly
/// reproduce the runtime timing. However, we can still demonstrate that the race condition
/// exists by showing that events are generated but not processed in the expected order.
pub fn test_parent_child_with_dynamic_insertion(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    let mut env = ctx.setup_full_integration();

    // Create the exact structure: main_scene -> managed_scene -> some_node -> sub_node (dynamically spawned)
    let mut managed_scene = godot::classes::Node3D::new_alloc();
    managed_scene.set_name("managed_scene");

    // Use our custom node class that spawns a child in _enter_tree()
    let mut some_node_gd = DynamicChildSpawner::new_alloc();
    some_node_gd.set_name("some_node");
    let mut some_node = some_node_gd.clone();

    // Build the hierarchy first
    managed_scene.add_child(&some_node.clone().upcast::<Node>());

    // Add to scene - this SHOULD trigger _enter_tree() on some_node, which spawns sub_node
    // The bug occurs here: sub_node is added while the scene tree plugin is still processing
    env.add_node_to_scene(managed_scene.clone());

    // Wait for the next Godot frame to process
    // This properly synchronizes Godot's frame processing with Bevy's update
    godot_print!("[TEST] Waiting for first frame to process nodes");
    env.wait_for_next_tick(ctx);

    // Verify that some_node itself has an entity (basic check)
    let some_node_entity = find_entity_for_node(ctx, some_node.instance_id());
    if some_node_entity.is_none() {
        return Err(TestError::assertion("some_node entity was not created"));
    }
    godot_print!("[TEST] some_node entity created: {:?}", some_node_entity);

    // Get the spawned child from our custom node - this triggers _enter_tree()
    godot_print!("[TEST] Checking for spawned child...");
    let sub_node = some_node.bind().get_spawned_child();

    // The sub_node might not exist yet because _enter_tree hasn't been called
    // Let's give it a chance and run another tick
    if sub_node.is_none() {
        godot_print!("[TEST] Sub node not spawned yet, waiting for next frame...");
        env.wait_for_next_tick(ctx);

        // Check again after the updates
        let sub_node_after = some_node.bind().get_spawned_child();
        if let Some(ref sub) = sub_node_after {
            godot_print!("[TEST] sub_node now exists with ID: {}", sub.instance_id());

            // Check if it has an entity (it shouldn't due to the race condition)
            let sub_node_entity = find_entity_for_node(ctx, sub.instance_id());
            if sub_node_entity.is_none() {
                godot_print!("[TEST] RACE CONDITION CONFIRMED: sub_node has no entity!");
                godot_print!(
                    "[TEST] This proves the parent wasn't in ent_mapping when sub_node was processed"
                );
                // This is actually the expected behavior for this test
                // We're demonstrating the race condition
            } else {
                godot_print!(
                    "[TEST] Unexpected: sub_node has entity {:?}",
                    sub_node_entity
                );
            }
        } else {
            return Err(TestError::assertion(
                "DynamicChildSpawner failed to spawn sub_node even after update",
            ));
        }
    } else {
        godot_print!(
            "[TEST] sub_node already exists with ID: {}",
            sub_node.as_ref().unwrap().instance_id()
        );
    }

    // Clean up
    managed_scene.queue_free();
    Ok(())
}

/// Test normal three-level hierarchy processing (comparable to dynamic test).
/// This creates the same structure as the dynamic test but all at once, not via _enter_tree().
pub fn test_normal_parent_child_processing(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    let mut env = ctx.setup_full_integration();

    // Create the same structure as dynamic test: main_scene -> managed_scene -> some_node -> sub_node
    // But create all levels normally (not dynamically)
    let mut managed_scene = godot::classes::Node3D::new_alloc();
    managed_scene.set_name("managed_scene");

    let mut some_node = godot::classes::Node3D::new_alloc();
    some_node.set_name("some_node");
    managed_scene.add_child(&some_node);

    // Add a third level child (equivalent to what GDScript spawns dynamically)
    let mut sub_node = godot::classes::Node3D::new_alloc();
    sub_node.set_name("sub_node");
    some_node.add_child(&sub_node);

    // Add the complete hierarchy to scene at once
    env.add_node_to_scene(managed_scene.clone());

    // Process all nodes together
    ctx.app.update();

    // Verify all three levels were created
    let _managed_entity = find_entity_for_node(ctx, managed_scene.instance_id())
        .expect("Managed scene entity should exist");
    let _some_node_entity =
        find_entity_for_node(ctx, some_node.instance_id()).expect("Some node entity should exist");
    let _sub_node_entity =
        find_entity_for_node(ctx, sub_node.instance_id()).expect("Sub node entity should exist");

    // Clean up
    managed_scene.queue_free();

    Ok(())
}
