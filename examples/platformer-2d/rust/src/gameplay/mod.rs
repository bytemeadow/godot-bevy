use bevy::app::{App, Plugin};
use bevy::prelude::*;
use bevy::state::condition::in_state;
use bevy::state::state::NextState;
use godot::classes::Input;
use godot_bevy::prelude::SceneTreeRef;

use crate::level_manager::{CurrentLevel, LevelLoadedEvent};
use crate::GameState;

pub mod door;
pub mod gem;
pub mod hud;
pub mod player;

use gem::GemsCollected;
use hud::HudHandles;

pub struct GameplayPlugin;
impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(gem::GemPlugin);
        app.add_plugins(hud::HudPlugin);
        app.add_plugins(door::DoorPlugin);
        app.add_systems(
            Update,
            handle_reset_level.run_if(in_state(GameState::InGame)),
        );
    }
}

/// System that handles reset level input during gameplay
fn handle_reset_level(
    mut gems_collected: ResMut<GemsCollected>,
    mut scene_tree: SceneTreeRef,
    mut hud_handles: ResMut<HudHandles>,
    current_level: Res<CurrentLevel>,
    mut level_loaded_events: EventWriter<LevelLoadedEvent>,
) {
    let input = Input::singleton();

    if input.is_action_just_pressed("reset_level") {
        info!("Reset level input detected - resetting level");

        // Reset gems collected
        gems_collected.0 = 0;

        // Clear HUD handles since they'll be invalid after scene reload
        hud_handles.gems_label = None;
        hud_handles.current_level_label = None;

        // Reload the scene
        scene_tree.get().reload_current_scene();

        // Emit level loaded event with current level ID
        if let Some(level_id) = current_level.level_id {
            level_loaded_events.write(LevelLoadedEvent { level_id });
        }
    }
}
