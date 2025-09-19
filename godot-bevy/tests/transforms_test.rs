//! Transform synchronization tests
//!
//! Run with: `cargo test --features api-4-3 --test transforms_test`

mod transforms;

use godot_bevy_testability::bevy_godot_test_main;

// Import test modules
use transforms::hierarchy::*;
use transforms::sync_modes::*;
use transforms::transform_initialization::*;

bevy_godot_test_main! {
    // Core initialization tests
    godot_transform_initializes_bevy_entity,
    node_at_origin_has_identity_transform,
    y_axis_rotation_syncs_correctly,
    compound_rotations_sync_correctly,
    extreme_position_values_handled_correctly,
    extreme_scale_values_handled_correctly,

    // Sync mode behavior tests
    one_way_mode_syncs_bevy_to_godot_only,
    two_way_mode_syncs_bidirectionally,
    disabled_mode_prevents_all_sync,
    sync_mode_can_change_at_runtime,

    // Hierarchy tests
    child_entity_has_correct_local_transform,
    parent_movement_updates_child_world_position,
    parent_rotation_affects_child_transform,
    parent_scale_affects_child_transform,
    deep_hierarchy_syncs_correctly,
}
