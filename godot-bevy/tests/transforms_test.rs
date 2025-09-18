//! Transform synchronization tests
//!
//! Run with: `cargo test --features api-4-3 --test transforms_test`

mod transforms;

use godot_bevy_testability::bevy_godot_test_main;

// Import all test functions
use transforms::basic_sync::{
    test_bevy_to_godot_sync, test_godot_node_creates_entity_with_transform,
};
use transforms::hierarchy_sync::test_hierarchy_transform_sync;
use transforms::scene_tree_integration::{
    test_bevy_to_godot_sync_with_scene_tree, test_scene_tree_node_creates_entity_with_transform,
};
use transforms::two_way_sync::test_two_way_sync_with_scene_tree;

bevy_godot_test_main! {
    test_godot_node_creates_entity_with_transform,
    test_bevy_to_godot_sync,
    test_hierarchy_transform_sync,
    test_scene_tree_node_creates_entity_with_transform,
    test_bevy_to_godot_sync_with_scene_tree,
    test_two_way_sync_with_scene_tree,
}
