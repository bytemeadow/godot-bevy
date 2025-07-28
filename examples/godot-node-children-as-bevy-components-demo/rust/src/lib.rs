use godot::builtin::Vector2;
use bevy::ecs::system::Query;
use bevy::math::Vec2;
use bevy::prelude::{App, Component, Res, Time, Update};
use bevy::transform::components::Transform;
use godot::global::godot_print;
use godot_bevy::plugins::GodotDefaultPlugins;
use godot_bevy::prelude::godot_prelude::gdextension;
use godot_bevy::prelude::godot_prelude::ExtensionLibrary;
use godot_bevy::prelude::{bevy_app, ComponentAsGodotNode};
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

    // Add the transform syncing plugin since we're using Transform components
    app.add_plugins(GodotDefaultPlugins);

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
    app.add_systems(Update, orbit_system);
}

// Components are data that can be attached to entities.
// This one will store the starting position of a Node2D.
#[derive(Debug, Default, Clone, Component, ComponentAsGodotNode)]
struct InitialPosition {
    initialized: bool,
    pos: Vec2,
}

// This component tracks the angle at which the Node2D is orbiting its starting position.
#[derive(Debug, Clone, Component, ComponentAsGodotNode)]
#[godot_node(base = Node2D, class_name = Orbiter)]
struct Orbiter {
    #[godot_export(export_type = Vector2, transform_with = vector2_to_vec2)]
    amplitude: Vec2,
    angle: f32,
}

fn vector2_to_vec2(v: Vector2) -> Vec2 {
    Vec2::new(v.x, v.y)
}

impl Default for Orbiter {
    fn default() -> Self {
        Self {
            amplitude: Vec2::new(1.0, 1.0),
            angle: 0.0,
        }
    }
}

// This system orbits entities created above
fn orbit_system(
    // The `query` parameter is a Bevy `Query` that matches all `Transform` components.
    // `Transform` is a Godot-Bevy-provided component that matches all Node2Ds in the scene.
    // (https://docs.rs/godot-bevy/latest/godot_bevy/plugins/core/transforms/struct.Transform.html)
    mut query: Query<(&mut Transform, &mut InitialPosition, &mut Orbiter)>,

    // This is equivalent to Godot's `_process` `delta: float` parameter.
    process_delta: Res<Time>,
) {
    // For single matches, you can use `single_mut()` instead:
    // `if let Ok(mut transform) = transform.single_mut() {`
    for (mut transform, mut initial_position, mut orbiter) in query.iter_mut() {
        if !initial_position.initialized {
            initial_position.initialized = true;
            initial_position.pos = Vec2::new(transform.translation.x, transform.translation.y);
        }
        let position2d = initial_position.pos + Vec2::from_angle(orbiter.angle) * 100.0;
        transform.translation.x = position2d.x * orbiter.amplitude.x;
        transform.translation.y = position2d.y * orbiter.amplitude.y;
        orbiter.angle += process_delta.as_ref().delta_secs();
        orbiter.angle %= 2.0 * PI;
    }
}
