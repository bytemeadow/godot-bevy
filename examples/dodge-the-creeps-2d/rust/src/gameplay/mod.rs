use bevy::app::{App, Plugin};

pub struct GameplayPlugin;

pub mod countdown;
pub mod mob;
pub mod player;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin)
            .add_plugins(mob::MobPlugin)
            .add_plugins(countdown::CountdownPlugin);
    }
}
