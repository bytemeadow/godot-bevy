use crate::GameState;
use crate::level_manager::{CurrentLevel, LevelLoadedMessage};
use crate::scene_management::SceneOperationMessage;
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use bevy::state::condition::in_state;
use bevy::state::state::NextState;
use gem::GemsCollected;
use godot::classes::Input;
use godot_bevy::prelude::GodotAccess;
use hud::{HudHandles, HudUpdateMessage};

pub mod audio;
pub mod door;
pub mod gem;
pub mod hud;
pub mod player;

/// Events for decoupling gameplay systems
#[derive(Event, Debug, Clone)]
pub struct ResetLevelMessage;

#[derive(Event, Debug, Clone)]
pub struct ReturnToMainMenuMessage;

pub struct GameplayPlugin;
impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(audio::AudioPlugin);
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(gem::GemPlugin);
        app.add_plugins(hud::HudPlugin);
        app.add_plugins(door::DoorPlugin);

        // Add observers for reset and return to menu
        app.add_observer(on_reset_level)
            .add_observer(on_return_to_menu);

        app.add_systems(
            Update,
            (detect_reset_level_input, detect_return_to_menu_input)
                .run_if(in_state(GameState::InGame)),
        );
    }
}

/// System that detects reset level input and triggers observer
fn detect_reset_level_input(mut commands: Commands, mut godot: GodotAccess) {
    let input = godot.singleton::<Input>();

    if input.is_action_just_pressed("reset_level") {
        info!("Reset level input detected");
        commands.trigger(ResetLevelMessage);
    }
}

/// System that detects return to menu input and triggers observer
fn detect_return_to_menu_input(mut commands: Commands, mut godot: GodotAccess) {
    let input = godot.singleton::<Input>();

    if input.is_action_just_pressed("return_to_main_menu") {
        info!("Return to main menu input detected");
        commands.trigger(ReturnToMainMenuMessage);
    }
}

/// Observer that handles reset level events
fn on_reset_level(
    _trigger: On<ResetLevelMessage>,
    mut gems_collected: ResMut<GemsCollected>,
    mut scene_events: MessageWriter<SceneOperationMessage>,
    mut hud_handles: ResMut<HudHandles>,
    current_level: Res<CurrentLevel>,
    mut commands: Commands,
) {
    info!("Processing level reset");

    // Reset gems collected
    gems_collected.0 = 0;

    // Clear HUD handles since they'll be invalid after scene reload
    hud_handles.clear();

    // Send HUD update with reset gem count
    commands.trigger(HudUpdateMessage::GemsChanged(0));

    // Request scene reload through centralized scene management
    scene_events.write(SceneOperationMessage::reload());

    // Emit level loaded event with current level ID
    if let Some(level_id) = current_level.level_id {
        commands.trigger(LevelLoadedMessage { level_id });
    }
}

/// Observer that handles return to main menu events
fn on_return_to_menu(
    _trigger: On<ReturnToMainMenuMessage>,
    mut gems_collected: ResMut<GemsCollected>,
    mut scene_events: MessageWriter<SceneOperationMessage>,
    mut hud_handles: ResMut<HudHandles>,
    mut current_level: ResMut<CurrentLevel>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    info!("Processing return to main menu");

    // Reset gems collected
    gems_collected.0 = 0;

    // Clear HUD handles since they'll be invalid after scene changes
    hud_handles.clear();

    // Clear current level state
    current_level.clear();

    // Change to main menu state
    next_state.set(GameState::MainMenu);

    // Request scene change through centralized scene management
    scene_events.write(SceneOperationMessage::change_to_file(
        "res://scenes/levels/main_menu.tscn",
    ));
}
