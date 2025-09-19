//! Scene tree integration tests
//!
//! Tests for the godot-bevy scene tree plugin, verifying that:
//! - Godot nodes create corresponding Bevy entities
//! - Entity-node relationships are maintained
//! - Component synchronization works correctly
//! - Different node types are handled properly

use godot_bevy_testability::*;

mod scene_tree {
    pub mod entity_lifecycle;
    pub mod markers_and_groups;
    pub mod multiple_nodes;
    pub mod name_synchronization;
    pub mod utils;
}

// Import test functions
use scene_tree::entity_lifecycle::{
    test_entity_despawn_frees_node, test_node_creates_entity, test_node_deletion_removes_entity,
};
use scene_tree::markers_and_groups::{
    test_node_groups_component, test_node_type_markers, test_protected_entity_deletion,
};
use scene_tree::multiple_nodes::{
    test_different_node_types_create_entities, test_multiple_nodes_create_multiple_entities,
};
use scene_tree::name_synchronization::{
    test_initial_node_names_sync_to_entities, test_node_name_changes_update_entity,
};

bevy_godot_test_main! {
    // Basic entity lifecycle tests
    test_node_creates_entity,
    test_node_deletion_removes_entity,
    test_entity_despawn_frees_node,

    // Multiple nodes and different types
    test_multiple_nodes_create_multiple_entities,
    test_different_node_types_create_entities,

    // Name synchronization
    test_node_name_changes_update_entity,
    test_initial_node_names_sync_to_entities,

    // Node type markers and groups
    test_node_type_markers,
    test_node_groups_component,
    test_protected_entity_deletion,
}
