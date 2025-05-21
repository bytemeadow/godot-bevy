use bevy::app::{App, Plugin};

pub struct GameplayPlugin;

pub mod player;
pub mod mob;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin);
    }
}
