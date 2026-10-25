use bevy::prelude::*;
use godot::classes::Area2D;
use godot_bevy::prelude::*;

#[derive(Component)]
pub struct Bullet;

pub fn bullet_plugin(app: &mut App) {
    app.add_observer(configure_bullet)
        .add_systems(Startup, spawn_bullet);
}

fn spawn_bullet(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        Bullet,
        GodotScene::from_handle(assets.load("scenes/bullet.tscn")),
    ));
}

fn configure_bullet(
    event: On<Add<GodotNodeHandle>>,
    bullets: Query<&GodotNodeHandle, With<Bullet>>,
    mut godot: GodotAccess,
) {
    if let Ok(handle) = bullets.get(event.entity) {
        let mut bullet = godot.get::<Area2D>(*handle);
        bullet.set_collision_mask(4);
    }
}
