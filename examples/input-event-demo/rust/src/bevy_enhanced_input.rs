use bevy::prelude::*;
use bevy_enhanced_input::{EnhancedInputPlugin, prelude::*};
use godot::global::godot_print;
pub struct BevyEnhancedInputPlugin;

impl Plugin for BevyEnhancedInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EnhancedInputPlugin)
            .add_input_context::<Player>()
            .add_observer(bind)
            .add_observer(apply_move)
            .add_observer(interact)
            .add_systems(Startup, init_input);
    }
}

fn init_input(mut commands: Commands) {
    commands.spawn(Actions::<Player>::default());
}

fn bind(trigger: Trigger<Bind<Player>>, mut players: Query<&mut Actions<Player>>) {
    let mut actions = players.get_mut(trigger.target()).unwrap();
    actions
        .bind::<Move>()
        .to((Cardinal::wasd_keys(), Axial::left_stick()))
        .with_modifiers(DeadZone::default());
    actions
        .bind::<Interact>()
        .to((KeyCode::KeyE, GamepadButton::South));
}

fn apply_move(trigger: Trigger<Fired<Move>>) {
    godot_print!("[BEVY ENHANCED INPUT] move: {}", trigger.value);
}

fn interact(_trigger: Trigger<Fired<Interact>>) {
    godot_print!("[BEVY ENHANCED INPUT] interact");
}

#[derive(InputContext)]
struct Player;

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
struct Move;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct Interact;
