use bevy::ecs::system::Query;
use bevy::prelude::{
    App, Commands, Component, Entity, IntoScheduleConfigs, Res, Time, Update, Without,
};
use bevy::transform::components::Transform;
use godot::builtin::Vector2;
use godot::classes::Sprite2D;
use godot::global::godot_print;
use godot_bevy::prelude::{
    GodotAccess, GodotNodeHandle, GodotTransformSyncPlugin, Sprite2DMarker, bevy_app,
};
use std::f32::consts::PI;

#[bevy_app]
fn build_app(app: &mut App) {
    godot_print!("Hello from Godot-Bevy!");

    // Transform components require the opt-in sync plugin.
    app.add_plugins(GodotTransformSyncPlugin::default());

    // Godot-Bevy synchronizes the Bevy 'Update' schedule parameter with the
    // Godot `_process` update cycle. The `FixedUpdate` schedule is driven from
    // Godot's `_physics_process` update cycle (its fixed physics clock).
    app.add_systems(Update, (orbit_setup, orbit_system).chain());
}

#[derive(Debug, Component)]
struct InitialPosition {
    pos: Vector2,
}

#[derive(Debug, Component)]
struct Orbiter {
    angle: f32,
}

#[derive(Debug, Component)]
struct NodeInitialized;

fn orbit_setup(
    mut commands: Commands,
    uninitialized: Query<(Entity, &GodotNodeHandle, &Sprite2DMarker), Without<NodeInitialized>>,
    mut godot: GodotAccess,
) {
    for (entity, node_handle, _) in uninitialized.iter() {
        let sprite_node = godot.get::<Sprite2D>(*node_handle);
        godot_print!(
            "Initializing node: {:?}",
            sprite_node.get_name().to_string()
        );
        commands
            .entity(entity)
            .insert(InitialPosition {
                pos: sprite_node.get_transform().origin,
            })
            .insert(Orbiter { angle: 0.0 })
            .insert(NodeInitialized);
    }
}

fn orbit_system(
    mut transform: Query<(&mut Transform, &InitialPosition, &mut Orbiter)>,

    // This is equivalent to Godot's `_process` `delta: float` parameter.
    process_delta: Res<Time>,
) {
    for (mut transform, initial_position, mut orbiter) in transform.iter_mut() {
        let position2d = initial_position.pos + Vector2::from_angle(orbiter.angle) * 100.0;
        transform.translation.x = position2d.x;
        transform.translation.y = position2d.y;
        orbiter.angle += process_delta.as_ref().delta_secs();
        orbiter.angle %= 2.0 * PI;
    }
}
