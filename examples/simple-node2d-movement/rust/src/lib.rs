use bevy::ecs::system::Query;
use bevy::prelude::{
    App, Commands, Component, Entity, IntoScheduleConfigs, Res, Time, Update, Without,
};
use godot::builtin::Vector2;
use godot::classes::Sprite2D;
use godot::global::godot_print;
use godot_bevy::prelude::godot_prelude::gdextension;
use godot_bevy::prelude::godot_prelude::ExtensionLibrary;
use godot_bevy::prelude::{
    bevy_app, main_thread_system, GodotNodeHandle, Sprite2DMarker, Transform2D,
};
use std::f32::consts::PI;

// The build_app function runs at your game's startup.
//
// Entry point for the Godot-Bevy plugin. For more about the `#[bevy_app]` macro, see:
// (https://docs.rs/godot-bevy-macros/0.6.1/godot_bevy_macros/attr.bevy_app.html)
//
// The #[bevy_app] macro is a wrapper around the Godot-Rust #[gdextension] macro:
// (https://godot-rust.github.io/docs/gdext/master/godot/prelude/trait.ExtensionLibrary.html)
//
// Read more about the Bevy `App` parameter here:
// (https://bevy.org/learn/quick-start/getting-started/apps/)
#[bevy_app]
fn build_app(app: &mut App) {
    // Print to the Godot console:
    // (https://docs.rs/godot-core/0.3.1/godot_core/macro.godot_print.html)
    godot_print!("Hello from Godot-Bevy!");

    // A system is a normal Rust function.
    //
    // This line runs the `orbit_setup` and then the
    // `orbit_system` functions every Godot render frame.
    //
    // Read more about Bevy's Entities, Components, and Systems here:
    // (https://bevy.org/learn/quick-start/getting-started/ecs/).
    //
    // Godot-Bevy synchronizes the Bevy 'Update' schedule parameter with the
    // Godot `_process` update cycle. There is also a `PhysicsUpdate` schedule
    // parameter that is synchronized with the Godot `_physics_process` update cycle.
    //
    // Read more about other schedules provided by Godot-Bevy here:
    // (https://bytemeadow.github.io/godot-bevy/scene-tree/timing.html).
    app.add_systems(Update, (orbit_setup, orbit_system).chain());
}

// Components are data that can be attached to entities.
// This one will store the starting position of a Node2D.
#[derive(Debug, Component)]
struct InitialPosition {
    pos: Vector2,
}

// This component tracks the angle at which the Node2D is orbiting its starting position.
#[derive(Debug, Component)]
struct Orbiter {
    angle: f32,
}

// This component is used as a marker to keep track of which nodes have been initialized.
#[derive(Debug, Component)]
struct NodeInitialized;

// This system initializes Sprite2Ds with the required components to allow the orbit_system to manipulate them.
#[main_thread_system]
fn orbit_setup(
    // Bevy Commands allow us to modify the state of the world, such as adding components to entities.
    mut commands: Commands,

    // Gather all Godot nodes without the `NodeInitialized` component.
    // Also, include the Bevy entity identifier so we can add components to it.
    mut uninitialized: Query<
        (Entity, &mut GodotNodeHandle, &Sprite2DMarker),
        Without<NodeInitialized>,
    >,
) {
    for (entity, mut node_handle, _) in uninitialized.iter_mut() {
        let sprite_node = node_handle.get::<Sprite2D>();
        // The GodotNodeHandle allows us to call Godot methods such as `get_name()`.
        godot_print!(
            "Initializing node: {:?}",
            sprite_node.get_name().to_string()
        );
        // Attach new components to the entity.
        commands
            .entity(entity)
            .insert(InitialPosition {
                pos: sprite_node.get_transform().origin,
            })
            .insert(Orbiter { angle: 0.0 })
            .insert(NodeInitialized);
    }
}

// This system moves all Node2Ds to the right, such as Sprite2Ds.
fn orbit_system(
    // The `transform` parameter is a Bevy `Query` that matches all `Transform2D` components.
    // `Transform2D` is a Godot-Bevy-provided component that matches all Node2Ds in the scene.
    // (https://docs.rs/godot-bevy/latest/godot_bevy/plugins/core/transforms/struct.Transform2D.html)
    mut transform: Query<(&mut Transform2D, &InitialPosition, &mut Orbiter)>,

    // This is equivalent to Godot's `_process` `delta: float` parameter.
    process_delta: Res<Time>,
) {
    // For single matches, you can use `single_mut()` instead:
    // `if let Ok(mut transform) = transform.single_mut() {`
    for (mut transform, initial_position, mut orbiter) in transform.iter_mut() {
        transform.as_godot_mut().origin =
            initial_position.pos + Vector2::from_angle(orbiter.angle) * 100.0;
        orbiter.angle += process_delta.as_ref().delta_secs();
        orbiter.angle %= 2.0 * PI;
    }
}
