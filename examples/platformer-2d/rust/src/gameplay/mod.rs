use bevy::app::{App, Plugin};

pub mod gem;
pub mod player;

pub struct GameplayPlugin;
impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(gem::GemPlugin);
    }
}
