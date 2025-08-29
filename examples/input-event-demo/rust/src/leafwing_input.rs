use bevy::prelude::*;
use godot_bevy::prelude::godot_prelude::godot_print;
use leafwing_input_manager::prelude::*;
use leafwing_input_manager::user_input::MouseMove;

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum PlayerAction {
    #[actionlike(DualAxis)]
    Move,
    #[actionlike(DualAxis)]
    MouseLook,
    Jump,
    Shoot,
}

pub struct LeafwingInputTestPlugin;

impl Plugin for LeafwingInputTestPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<PlayerAction>::default())
            .add_systems(Startup, spawn_player)
            .add_systems(Update, update_player_input);
    }
}

#[derive(Component)]
pub struct Player;

fn spawn_player(mut commands: Commands) {
    godot_print!("🎮 Spawning Player!");

    let input_map = InputMap::default()
        .with_dual_axis(PlayerAction::Move, GamepadStick::LEFT) // Gamepad
        .with_dual_axis(PlayerAction::Move, VirtualDPad::wasd())    // Keyboard
        .with_dual_axis(PlayerAction::Move, VirtualDPad::arrow_keys())  // Keyboard

        .with_dual_axis(PlayerAction::MouseLook, MouseMove::default()) // Mouse movement

        .with(PlayerAction::Jump, GamepadButton::South) // Gamepad
        .with(PlayerAction::Jump, KeyCode::Space)   // Keyboard
        .with(PlayerAction::Jump, MouseButton::Left)    // Mouse
    ;

    commands.spawn(input_map).insert(Player);
}

fn update_player_input(query: Query<&ActionState<PlayerAction>, With<Player>>) {
    let Ok(action_state) = query.single() else {
        godot_print!("❌ LEAFWING: No Player found!");
        return;
    };

    if action_state.just_pressed(&PlayerAction::Jump) {
        godot_print!("🚀 LEAFWING: Player jumped!");
    }

    if action_state.just_pressed(&PlayerAction::Shoot) {
        godot_print!("💥 LEAFWING: Player shot!");
    }

    let move_value: Vec2 = action_state.clamped_axis_pair(&PlayerAction::Move);
    if move_value.length() > 0.1 {
        godot_print!(
            "🏃 LEAFWING: Move value: ({:.2}, {:.2})",
            move_value.x,
            move_value.y
        );
    }

    let mouse_look: Vec2 = action_state.clamped_axis_pair(&PlayerAction::MouseLook);
    if mouse_look.length() > 1.0 {
        godot_print!(
            "🖱️ LEAFWING: Mouse look: ({:.2}, {:.2})",
            mouse_look.x,
            mouse_look.y
        );
    }

    // Debug info for troubleshooting specific issues
    static mut LAST_DEBUG_TIME: f32 = 0.0;
    let current_time = unsafe { LAST_DEBUG_TIME + 0.016 };
    unsafe {
        LAST_DEBUG_TIME = current_time;
    }

    // Print debug info every ~2 seconds to help diagnose issues
    if (current_time as u32).is_multiple_of(120) {
        godot_print!("🔍 LEAFWING DEBUG: Checking input states...");

        // Check if any input is being detected
        if action_state.pressed(&PlayerAction::Jump) {
            godot_print!("  - Jump is currently pressed");
        }

        let axis_data = action_state.axis_pair(&PlayerAction::Move);
        if axis_data.xy().length() > 0.01 {
            godot_print!(
                "  - Raw move axis: ({:.2}, {:.2})",
                axis_data.x,
                axis_data.y
            );
        }

        // Test specific input types
        godot_print!(
            "  - Testing keyboard (WASD/arrows), mouse (left click + movement), gamepad (left stick + A button)"
        );

        let mouse_axis = action_state.axis_pair(&PlayerAction::MouseLook);
        if mouse_axis.xy().length() > 0.1 {
            godot_print!(
                "  - Raw mouse movement: ({:.2}, {:.2})",
                mouse_axis.x,
                mouse_axis.y
            );
        }
    }
}
