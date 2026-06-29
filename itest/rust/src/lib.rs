/*
 * Integration tests for godot-bevy
 * All tests are async and wait for real Godot frame progression
 */

use godot::init::{ExtensionLibrary, gdextension};

// Declare the test runner class
godot_bevy_test::declare_test_runner!();

// Test modules
mod autosync_match_tests;
mod benchmarks;
mod collision_tests;
mod input_ecosystem_tests;
mod input_tests;
#[cfg(feature = "autosync-tests")]
mod macro_redesign_tests;
mod real_frame_tests;
mod scene_tree_tests;
mod scene_tree_watcher_init_tests;
mod signal_tests;
mod transform_sync_tests;

#[gdextension(entry_symbol = godot_bevy_itest)]
unsafe impl ExtensionLibrary for IntegrationTests {}
