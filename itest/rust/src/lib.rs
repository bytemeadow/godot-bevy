/*
 * Integration tests for godot-bevy
 * All tests are async and wait for real Godot frame progression
 */

use godot::init::{ExtensionLibrary, gdextension};

// Declare the test runner class
godot_bevy_test::declare_test_runner!();

// Test modules
mod benchmarks;
mod real_frame_tests;
mod scene_tree_tests;
mod transform_sync_tests;

#[gdextension(entry_symbol = godot_bevy_itest)]
unsafe impl ExtensionLibrary for IntegrationTests {}
