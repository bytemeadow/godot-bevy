/*
 * Integration tests for godot-bevy
 * All tests are async and wait for real Godot frame progression
 */

use godot::init::{ExtensionLibrary, gdextension};

mod framework;
mod real_frame_tests;
mod scene_tree_tests;
mod transform_sync_tests;

// Re-export for tests
pub use framework::*;

#[gdextension(entry_symbol = godot_bevy_itest)]
unsafe impl ExtensionLibrary for framework::IntegrationTests {}
