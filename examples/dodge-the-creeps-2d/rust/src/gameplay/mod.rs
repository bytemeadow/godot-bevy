use bevy::app::{App, Plugin};

pub struct GameplayPlugin;

pub mod audio;
pub mod countdown;
pub mod gameover;
pub mod mob;
pub mod player;
pub mod score;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(audio::AudioPlugin)
            .add_plugins(player::PlayerPlugin)
            .add_plugins(mob::MobPlugin)
            .add_plugins(countdown::CountdownPlugin)
            .add_plugins(score::ScorePlugin)
            .add_plugins(gameover::GameoverPlugin);
    }
}
