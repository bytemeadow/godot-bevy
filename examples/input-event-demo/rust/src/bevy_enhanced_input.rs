use bevy::prelude::*;
use bevy_enhanced_input::{EnhancedInputPlugin, prelude::*};
use godot::global::godot_print;

pub struct BevyEnhancedInputPlugin;

impl Plugin for BevyEnhancedInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EnhancedInputPlugin)
            .add_input_context::<Player>()
            .add_observer(apply_move)
            .add_observer(interact)
            .add_systems(Startup, init_input);
    }
}

fn init_input(mut commands: Commands) {
    commands.spawn((
        Player,
        actions!(Player[
            (
                Action::<Move>::new(),
                DeadZone::default(),
                Bindings::spawn((
                    Cardinal::wasd_keys(),
                    Axial::left_stick(),
                )),
            ),
            (
                Action::<Interact>::new(),
                bindings![KeyCode::KeyE, GamepadButton::South],
            ),
        ]),
    ));
}

fn apply_move(trigger: On<Fire<Move>>) {
    godot_print!("[BEVY ENHANCED INPUT] move: {}", trigger.value);
}

fn interact(_trigger: On<Fire<Interact>>) {
    godot_print!("[BEVY ENHANCED INPUT] interact");
}

// Input context marker component
#[derive(Component)]
struct Player;

// Action definitions
#[derive(InputAction)]
#[action_output(Vec2)]
struct Move;

#[derive(InputAction)]
#[action_output(bool)]
struct Interact;
