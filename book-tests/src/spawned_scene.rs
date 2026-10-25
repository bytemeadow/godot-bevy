use bevy::prelude::*;
use godot::classes::Area2D;
use godot_bevy::prelude::*;

pub fn bullet_plugin(app: &mut App) {
    app.add_systems(Startup, spawn_bullet);
}

fn spawn_bullet(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn(
        GodotScene::from_handle(assets.load("scenes/bullet.tscn")).with_setup::<Area2D, _>(
            |root| {
                root.set_collision_mask(4);
            },
        ),
    );
}
