use bevy::prelude::*;
use godot::classes::Area2D;
use godot_bevy::prelude::*;

#[derive(Component)]
pub struct Bullet {
    pub collision_mask: u32,
}

pub fn bullet_plugin(app: &mut App) {
    app.add_observer(configure_bullet)
        .add_systems(Startup, spawn_bullet);
}

fn spawn_bullet(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        Bullet { collision_mask: 4 },
        GodotScene::from_handle(assets.load("scenes/bullet.tscn")),
    ));
}

fn configure_bullet(
    event: On<Add<GodotNodeHandle>>,
    bullets: Query<(&Bullet, &GodotNodeHandle)>,
    mut godot: GodotAccess,
) {
    if let Ok((bullet, handle)) = bullets.get(event.entity) {
        let mut root = godot.get::<Area2D>(*handle);
        root.set_collision_mask(bullet.collision_mask);
    }
}
