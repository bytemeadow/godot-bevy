#![allow(clippy::type_complexity)]

use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;
use godot::global::Key;
use godot_bevy::prelude::{
    godot_prelude::{gdextension, godot_print, ExtensionLibrary},
    *,
};
use leafwing_input_manager::prelude::*;

// Import input event types directly to avoid naming conflicts
use godot_bevy::plugins::core::input_event::{
    ActionInput, GamepadAxisInput, GamepadButtonInput, KeyboardInput, MouseButton,
    MouseButtonInput, MouseMotion, TouchInput,
};

// This example demonstrates godot-bevy's input event system.
//
// Key behavior:
// - Keys mapped in Godot's Input Map (like arrow keys → "ui_down", "move_down")
//   generate ActionInput events only (no duplicate raw keyboard events)
// - Unmapped keys (like random letters) generate KeyboardInput events only
// - This prevents duplicate events and follows Godot's intended input flow
//
// Also demonstrates leafwing-input-manager integration via BevyInputBridgePlugin

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect, Component)]
enum PlayerAction {
    Jump,
    Shoot,
    MoveLeft,
    MoveRight,
    Sprint,
    GamepadAction,
}

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(InputEventPlugin)
        // Test leafwing-input-manager integration
        .add_plugins(InputManagerPlugin::<PlayerAction>::default())
        .add_systems(Startup, spawn_player)
        .add_systems(Update, test_leafwing_input);
}

struct InputEventPlugin;

impl Plugin for InputEventPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_keyboard_input,
                handle_mouse_button_input,
                handle_mouse_motion,
                handle_touch_input,
                handle_action_input,
                handle_gamepad_button_input,
                handle_gamepad_axis_input,
                test_bevy_input_resources,
            ),
        );
    }
}

fn handle_keyboard_input(mut keyboard_events: EventReader<KeyboardInput>) {
    for event in keyboard_events.read() {
        let key_name = format!("{:?}", event.keycode);
        let state = if event.pressed { "pressed" } else { "released" };
        let echo_info = if event.echo { " (echo)" } else { "" };

        godot_print!(
            "🎹 Keyboard: {} {} (physical: {:?}){}",
            key_name,
            state,
            event.physical_keycode,
            echo_info
        );

        // Special handling for common keys
        match event.keycode {
            Key::SPACE if event.pressed => {
                godot_print!("🚀 Space bar pressed - Jump!");
            }
            Key::ESCAPE if event.pressed => {
                godot_print!("🚪 Escape pressed - Pause menu!");
            }
            Key::ENTER if event.pressed => {
                godot_print!("✅ Enter pressed - Confirm!");
            }
            _ => {}
        }
    }
}

fn handle_mouse_button_input(mut mouse_button_events: EventReader<MouseButtonInput>) {
    for event in mouse_button_events.read() {
        let button_name = format!("{:?}", event.button);
        let state = if event.pressed { "pressed" } else { "released" };

        godot_print!(
            "🖱️  Mouse: {} {} at ({:.1}, {:.1})",
            button_name,
            state,
            event.position.x,
            event.position.y
        );

        // Special handling for different buttons
        match event.button {
            MouseButton::Left if event.pressed => {
                godot_print!("👆 Left click - Select/Attack!");
            }
            MouseButton::Right if event.pressed => {
                godot_print!("👉 Right click - Context menu!");
            }
            MouseButton::WheelUp => {
                godot_print!("🔼 Scroll up - Zoom in!");
            }
            MouseButton::WheelDown => {
                godot_print!("🔽 Scroll down - Zoom out!");
            }
            _ => {}
        }
    }
}

fn handle_mouse_motion(mut mouse_motion_events: EventReader<MouseMotion>) {
    for event in mouse_motion_events.read() {
        // Only log significant mouse movements to avoid spam
        if event.delta.length() > 5.0 {
            godot_print!(
                "🖱️  Mouse moved: delta({:.1}, {:.1}) position({:.1}, {:.1})",
                event.delta.x,
                event.delta.y,
                event.position.x,
                event.position.y
            );
        }
    }
}

fn handle_touch_input(mut touch_events: EventReader<TouchInput>) {
    for event in touch_events.read() {
        let state = if event.pressed { "touched" } else { "released" };

        godot_print!(
            "👆 Touch: finger {} {} at ({:.1}, {:.1})",
            event.finger_id,
            state,
            event.position.x,
            event.position.y
        );

        if event.pressed {
            godot_print!("📱 Touch started - finger {}", event.finger_id);
        } else {
            godot_print!("📱 Touch ended - finger {}", event.finger_id);
        }
    }
}

fn handle_action_input(mut action_events: EventReader<ActionInput>) {
    for event in action_events.read() {
        let state = if event.pressed { "pressed" } else { "released" };

        godot_print!(
            "🎮 Action: '{}' {} (strength: {:.2})",
            event.action,
            state,
            event.strength
        );

        // Handle common action names
        match event.action.as_str() {
            "ui_accept" if event.pressed => {
                godot_print!("✅ UI Accept action triggered!");
            }
            "ui_cancel" if event.pressed => {
                godot_print!("❌ UI Cancel action triggered!");
            }
            "move_left" | "move_right" | "move_up" | "move_down" => {
                if event.pressed {
                    godot_print!("🏃 Movement action: {}", event.action);
                }
            }
            "jump" => {
                godot_print!("🦘 Jump action: {}", state);
            }
            _ => {}
        }
    }
}

fn test_bevy_input_resources(
    keyboard_input: Res<bevy::input::ButtonInput<bevy::input::keyboard::KeyCode>>,
    mouse_input: Res<bevy::input::ButtonInput<bevy::input::mouse::MouseButton>>,
    gamepad_input: Res<bevy::input::ButtonInput<bevy::input::gamepad::GamepadButton>>,
    gamepad_axes: Res<bevy::input::Axis<bevy::input::gamepad::GamepadAxis>>,
) {
    use bevy::input::gamepad::{GamepadAxis, GamepadButton};
    use bevy::input::keyboard::KeyCode;
    use bevy::input::mouse::MouseButton;

    // Test some common keys using Bevy's standard input resources
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        godot_print!("🧪 BRIDGE TEST: 'T' key detected via Bevy's ButtonInput<KeyCode>!");
    }

    if keyboard_input.just_pressed(KeyCode::KeyB) {
        godot_print!("🧪 BRIDGE TEST: 'B' key detected via Bevy's ButtonInput<KeyCode>!");
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        godot_print!(
            "🧪 BRIDGE TEST: Left mouse button detected via Bevy's ButtonInput<MouseButton>!"
        );
    }

    if keyboard_input.pressed(KeyCode::Space) {
        godot_print!("🧪 BRIDGE TEST: Space held via Bevy input system!");
    }

    // Test gamepad inputs via Bevy's standard resources
    if gamepad_input.just_pressed(GamepadButton::South) {
        godot_print!(
            "🧪 BRIDGE TEST: Gamepad A button detected via Bevy's ButtonInput<GamepadButton>!"
        );
    }

    if gamepad_input.just_pressed(GamepadButton::East) {
        godot_print!(
            "🧪 BRIDGE TEST: Gamepad B button detected via Bevy's ButtonInput<GamepadButton>!"
        );
    }

    // Test gamepad axes
    let left_stick_x = gamepad_axes.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
    let left_stick_y = gamepad_axes.get(GamepadAxis::LeftStickY).unwrap_or(0.0);

    if left_stick_x.abs() > 0.5 || left_stick_y.abs() > 0.5 {
        godot_print!(
            "🧪 BRIDGE TEST: Left stick via Bevy Axis: ({:.2}, {:.2})",
            left_stick_x,
            left_stick_y
        );
    }
}

#[derive(Component)]
struct Player;

fn spawn_player(mut commands: Commands) {
    godot_print!("🎮 Spawning player with leafwing-input-manager!");

    // Create input map using Bevy's KeyCode and GamepadButton (which now works via our bridge!)
    let mut input_map = InputMap::new([
        (PlayerAction::Jump, KeyCode::Space),
        (PlayerAction::Shoot, KeyCode::KeyF),
        (PlayerAction::MoveLeft, KeyCode::KeyA),
        (PlayerAction::MoveRight, KeyCode::KeyD),
        (PlayerAction::Sprint, KeyCode::ShiftLeft),
    ]);

    // Add gamepad button mapping separately
    input_map.insert(PlayerAction::GamepadAction, GamepadButton::South);

    commands.spawn((Player, input_map, ActionState::<PlayerAction>::default()));
}

fn test_leafwing_input(query: Query<&ActionState<PlayerAction>, With<Player>>) {
    let Ok(action_state) = query.single() else {
        return;
    };

    // Test leafwing-input-manager integration
    if action_state.just_pressed(&PlayerAction::Jump) {
        godot_print!("🚀 LEAFWING: Player jumped!");
    }

    if action_state.just_pressed(&PlayerAction::Shoot) {
        godot_print!("💥 LEAFWING: Player shot!");
    }

    if action_state.pressed(&PlayerAction::MoveLeft) {
        godot_print!("⬅️ LEAFWING: Moving left...");
    }

    if action_state.pressed(&PlayerAction::MoveRight) {
        godot_print!("➡️ LEAFWING: Moving right...");
    }

    if action_state.pressed(&PlayerAction::Sprint) {
        godot_print!("🏃 LEAFWING: Sprinting!");
    }

    if action_state.just_pressed(&PlayerAction::GamepadAction) {
        godot_print!("🎮 LEAFWING: Gamepad A button pressed via bridge!");
    }
}

fn handle_gamepad_button_input(mut gamepad_button_events: EventReader<GamepadButtonInput>) {
    for event in gamepad_button_events.read() {
        let state = if event.pressed { "pressed" } else { "released" };

        godot_print!(
            "🎮 Gamepad {}: Button {} {} (pressure: {:.2})",
            event.device,
            event.button_index,
            state,
            event.pressure
        );

        // Handle common buttons
        match event.button_index {
            0 => {
                // A button (South)
                if event.pressed {
                    godot_print!("🔴 A button pressed - Jump/Confirm!");
                }
            }
            1 => {
                // B button (East)
                if event.pressed {
                    godot_print!("🔵 B button pressed - Back/Cancel!");
                }
            }
            2 => {
                // X button (West)
                if event.pressed {
                    godot_print!("🟩 X button pressed - Action!");
                }
            }
            3 => {
                // Y button (North)
                if event.pressed {
                    godot_print!("🟨 Y button pressed - Menu!");
                }
            }
            _ => {}
        }
    }
}

fn handle_gamepad_axis_input(mut gamepad_axis_events: EventReader<GamepadAxisInput>) {
    for event in gamepad_axis_events.read() {
        // Only log significant axis movements to avoid spam
        if event.value.abs() > 0.1 {
            godot_print!(
                "🕹️ Gamepad {}: Axis {} = {:.2}",
                event.device,
                event.axis,
                event.value
            );

            // Handle common axes
            match event.axis {
                0 => godot_print!("⬅️➡️ Left stick X: {:.2}", event.value),
                1 => godot_print!("⬆️⬇️ Left stick Y: {:.2}", event.value),
                2 => godot_print!("⬅️➡️ Right stick X: {:.2}", event.value),
                3 => godot_print!("⬆️⬇️ Right stick Y: {:.2}", event.value),
                4 => godot_print!("🎯 Left trigger: {:.2}", event.value),
                5 => godot_print!("🎯 Right trigger: {:.2}", event.value),
                _ => {}
            }
        }
    }
}
