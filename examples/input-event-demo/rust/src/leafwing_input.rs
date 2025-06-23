use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;
use godot_bevy::prelude::godot_prelude::godot_print;
use leafwing_input_manager::prelude::*;

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect, Component)]
pub enum PlayerAction {
    Jump,
    Shoot,
    MoveLeft,
    MoveRight,
    Sprint,
    GamepadAction,
}

pub struct LeafwingInputTestPlugin;

impl Plugin for LeafwingInputTestPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<PlayerAction>::default())
            .add_systems(Startup, spawn_player)
            .add_systems(Update, test_leafwing_input);
    }
}

#[derive(Component)]
pub struct Player;

fn spawn_player(mut commands: Commands) {
    godot_print!("🎮 Spawning player with leafwing-input-manager!");

    // Create input map using Bevy's KeyCode and GamepadButton (which now works via our bridge!)
    let mut input_map = InputMap::new([
        (PlayerAction::Jump, bevy::input::keyboard::KeyCode::Space),
        (PlayerAction::Shoot, bevy::input::keyboard::KeyCode::KeyF),
        (PlayerAction::MoveLeft, bevy::input::keyboard::KeyCode::KeyA),
        (PlayerAction::MoveRight, bevy::input::keyboard::KeyCode::KeyD),
        (PlayerAction::Sprint, bevy::input::keyboard::KeyCode::ShiftLeft),
    ]);

    // Add gamepad button mapping 
    input_map.insert(PlayerAction::GamepadAction, GamepadButton::South);
    
    // Add mouse button mappings
    input_map.insert(PlayerAction::Jump, bevy::input::mouse::MouseButton::Left);
    input_map.insert(PlayerAction::Shoot, bevy::input::mouse::MouseButton::Right);

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

    // Debug: Check the state of all actions to see what's happening
    static mut LAST_DEBUG_TIME: f32 = 0.0;
    let current_time = unsafe { LAST_DEBUG_TIME + 0.1 };
    unsafe { LAST_DEBUG_TIME = current_time; }
    
    // Print debug info every ~100 frames to avoid spam
    if (current_time as u32) % 100 == 0 {
        let active_actions: Vec<_> = [
            PlayerAction::Jump,
            PlayerAction::Shoot,
            PlayerAction::MoveLeft,
            PlayerAction::MoveRight,
            PlayerAction::Sprint,
            PlayerAction::GamepadAction,
        ].iter().filter(|action| action_state.pressed(action)).collect();
        
        if !active_actions.is_empty() {
            godot_print!("🔍 LEAFWING DEBUG: Active actions: {:?}", active_actions);
        }
    }
}