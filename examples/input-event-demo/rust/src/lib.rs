#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use godot_bevy::prelude::{
    BevyInputBridgePlugin,
    godot_prelude::{ExtensionLibrary, gdextension},
    *,
};

mod bevy_input;
mod godot_input;
mod leafwing_input;

use bevy_input::BevyInputTestPlugin;
use godot_input::GodotInputPlugin;
use leafwing_input::LeafwingInputTestPlugin;

// This example demonstrates godot-bevy's input event system with three separate plugins:
//
// 1. GodotInputPlugin - Raw Godot input events (ActionInput, KeyboardInput, etc.)
// 2. BevyInputTestPlugin - Bevy's standard input resources via the bridge
// 3. LeafwingInputTestPlugin - leafwing-input-manager integration
//
// Key behavior:
// - Keys mapped in Godot's Input Map (like arrow keys → "ui_down", "move_down")
//   generate ActionInput events only (no duplicate raw keyboard events)
// - Unmapped keys (like random letters) generate KeyboardInput events only
// - This prevents duplicate events and follows Godot's intended input flow
//
// You can easily disable individual plugins by commenting them out in build_app()

#[bevy_app]
fn build_app(app: &mut App) {
    // Enable/disable plugins as needed for testing:

    // Add the input plugin this example needs
    // BevyInputBridgePlugin automatically includes GodotInputEventPlugin
    app.add_plugins(BevyInputBridgePlugin);
    // Add gamepad input plugin as its used for testing
    app.add_plugins(GilrsPlugin);

    // Plugin 1: Raw Godot Input Events - shows direct Godot input
    app.add_plugins(GodotInputPlugin);

    // Plugin 2: Bevy Input Bridge Test - shows if bridge is working
    app.add_plugins(BevyInputTestPlugin);

    // Plugin 3: Leafwing Input Manager - shows if leafwing integration works
    app.add_plugins(LeafwingInputTestPlugin);
}
