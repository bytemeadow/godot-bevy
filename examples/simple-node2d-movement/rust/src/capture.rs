use super::{InitialPosition, Orbiter, orbit_setup, orbit_system};
use bevy::ecs::system::SystemState;
use bevy::prelude::{App, FixedUpdate, IntoScheduleConfigs, Query, Transform, Update, World};
use godot::classes::{RenderingServer, Sprite2D};
use godot::prelude::*;
use godot_bevy::prelude::{GodotAccess, GodotNodeHandle};
use godot_bevy_test::capture::{self, CaptureAdapter, CaptureManifest, Phase};

pub(super) fn install(app: &mut App) {
    app.add_systems(Update, orbit_setup);
    app.add_systems(
        FixedUpdate,
        orbit_system.run_if(|| capture::phase() == Phase::Running),
    );
    godot_bevy_test::capture::install(
        app,
        CaptureAdapter {
            name: "simple-node2d-movement",
            extension: |_, _, _| Ok(()),
            scenarios: &["orbit", "orbit-empty-region", "orbit-frames-0-1-2"],
            is_settled,
            reset,
            before_frame: |_, _, _| Ok(()),
        },
    );
}

fn is_settled(world: &mut World, _: &CaptureManifest) -> Result<bool, String> {
    let mut state = SystemState::<(Query<(&GodotNodeHandle, &Orbiter)>, GodotAccess)>::new(world);
    let (query, mut godot) = state.get_mut(world).map_err(|error| error.to_string())?;
    let mut count = 0;
    for (handle, _) in query.iter() {
        let sprite = godot.get::<Sprite2D>(*handle);
        if !sprite
            .get_texture()
            .is_some_and(|texture| texture.get_width() > 0 && texture.get_height() > 0)
        {
            return Ok(false);
        }
        count += 1;
    }
    Ok(count == 6)
}

fn reset(world: &mut World, _: &CaptureManifest) -> Result<(), String> {
    RenderingServer::singleton().set_default_clear_color(Color::BLACK);
    let world_step = world
        .resource::<bevy::prelude::Time<bevy::time::Fixed>>()
        .timestep()
        .as_secs_f32();
    let mut state = SystemState::<(
        Query<(
            &InitialPosition,
            &mut Orbiter,
            &mut Transform,
            &GodotNodeHandle,
        )>,
        GodotAccess,
    )>::new(world);
    let (mut query, mut godot) = state.get_mut(world).map_err(|error| error.to_string())?;
    for (initial, mut orbiter, mut transform, handle) in &mut query {
        let position = initial.pos + Vector2::new(100.0, 0.0);
        transform.translation.x = position.x;
        transform.translation.y = position.y;
        orbiter.angle = world_step;
        let mut sprite = godot.get::<Sprite2D>(*handle);
        sprite.set_position(position);
        sprite.set_visible(true);
    }
    Ok(())
}
