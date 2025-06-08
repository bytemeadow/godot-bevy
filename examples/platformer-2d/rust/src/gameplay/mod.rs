use bevy::app::{App, Plugin};
use bevy::prelude::*;
use bevy::state::condition::in_state;
use bevy::state::state::NextState;
use godot::classes::Input;

use crate::level_manager::{CurrentLevel, LevelLoadedEvent};
use crate::scene_management::SceneOperationEvent;
use crate::GameState;

pub mod audio;
pub mod door;
pub mod gem;
pub mod hud;
pub mod player;

use gem::GemsCollected;
use hud::{HudHandles, HudUpdateEvent};

/// System sets for gameplay operations with better parallelization
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameplaySystemSet {
    /// Input detection systems (can run in parallel)
    InputDetection,
    /// State management systems (may have dependencies)
    StateManagement,
}

/// Events for decoupling gameplay systems
#[derive(Event, Debug)]
pub struct ResetLevelEvent;

#[derive(Event, Debug)]
pub struct ReturnToMainMenuEvent;

pub struct GameplayPlugin;
impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(audio::AudioPlugin);
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(gem::GemPlugin);
        app.add_plugins(hud::HudPlugin);
        app.add_plugins(door::DoorPlugin);

        // Add our new events
        app.add_event::<ResetLevelEvent>()
            .add_event::<ReturnToMainMenuEvent>();

        app.add_systems(
            Update,
            (
                // Input detection systems can run in parallel
                (detect_reset_level_input, detect_return_to_menu_input)
                    .in_set(GameplaySystemSet::InputDetection),
                // State management systems run after input detection
                (handle_reset_level_events, handle_return_to_menu_events)
                    .in_set(GameplaySystemSet::StateManagement),
            )
                .chain()
                .run_if(in_state(GameState::InGame)),
        );
    }
}

/// System that detects reset level input and fires events
///
/// Runs in InputDetection set and can execute in parallel with other input systems.
/// Only reads input and writes events, enabling better parallelization.
fn detect_reset_level_input(mut reset_events: EventWriter<ResetLevelEvent>) {
    let input = Input::singleton();

    if input.is_action_just_pressed("reset_level") {
        info!("Reset level input detected");
        reset_events.write(ResetLevelEvent);
    }
}

/// System that detects return to menu input and fires events
///
/// Runs in InputDetection set and can execute in parallel with other input systems.
/// Only reads input and writes events, enabling better parallelization.
fn detect_return_to_menu_input(mut menu_events: EventWriter<ReturnToMainMenuEvent>) {
    let input = Input::singleton();

    if input.is_action_just_pressed("return_to_main_menu") {
        info!("Return to main menu input detected");
        menu_events.write(ReturnToMainMenuEvent);
    }
}

/// System that handles reset level events
///
/// Runs in StateManagement set after input detection. Handles all the state
/// changes and scene management needed for level reset.
fn handle_reset_level_events(
    mut reset_events: EventReader<ResetLevelEvent>,
    mut gems_collected: ResMut<GemsCollected>,
    mut scene_events: EventWriter<SceneOperationEvent>,
    mut hud_handles: ResMut<HudHandles>,
    current_level: Res<CurrentLevel>,
    mut level_loaded_events: EventWriter<LevelLoadedEvent>,
    mut hud_update_events: EventWriter<HudUpdateEvent>,
) {
    for _event in reset_events.read() {
        info!("Processing level reset");

        // Reset gems collected
        gems_collected.0 = 0;

        // Clear HUD handles since they'll be invalid after scene reload
        hud_handles.clear();

        // Send HUD update with reset gem count
        hud_update_events.write(HudUpdateEvent::GemsChanged(0));

        // Request scene reload through centralized scene management
        scene_events.write(SceneOperationEvent::reload());

        // Emit level loaded event with current level ID
        if let Some(level_id) = current_level.level_id {
            level_loaded_events.write(LevelLoadedEvent { level_id });
        }
    }
}

/// System that handles return to main menu events
///
/// Runs in StateManagement set after input detection. Handles all the state
/// changes and scene transitions needed to return to the main menu.
fn handle_return_to_menu_events(
    mut menu_events: EventReader<ReturnToMainMenuEvent>,
    mut gems_collected: ResMut<GemsCollected>,
    mut scene_events: EventWriter<SceneOperationEvent>,
    mut hud_handles: ResMut<HudHandles>,
    mut current_level: ResMut<CurrentLevel>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for _event in menu_events.read() {
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
        scene_events.write(SceneOperationEvent::change_to_file(
            "res://scenes/levels/main_menu.tscn",
        ));
    }
}
