use crate::container::ParticleRain;
use crate::particle_rain::{NeedsColorization, Particle, ParticleCount, SimulationState, Velocity};
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use godot::classes::{Engine, Node2D, RenderingServer, SceneTree, Sprite2D};
use godot::prelude::*;
use godot_bevy::prelude::{GodotAccess, GodotNodeHandle, GodotResource};
use godot_bevy_test::capture::{self, CaptureAdapter, CaptureManifest, Phase};

const PARTICLE_COUNT: usize = 2000;

#[derive(Resource)]
struct Prepared;

pub(super) fn install(app: &mut App) {
    app.add_systems(FixedFirst, hold_particles_during_warmup);
    capture::install(
        app,
        CaptureAdapter {
            name: "perf-test",
            extension: |_, _, _| Ok(()),
            scenarios: &["rain-2000", "rain-2000-unspawned"],
            is_settled,
            reset,
            before_frame,
        },
    );
}

fn hold_particles_during_warmup(mut state: ResMut<SimulationState>) {
    if capture::phase() == Phase::WarmUp {
        state.is_running = false;
    }
}

fn containers() -> Option<(Gd<Node2D>, Gd<ParticleRain>)> {
    let tree = Engine::singleton()
        .get_main_loop()?
        .try_cast::<SceneTree>()
        .ok()?;
    let scene = tree.get_current_scene()?;
    Some((
        scene.try_get_node_as("GodotParticlesContainer")?,
        scene.try_get_node_as("BevyParticlesContainer")?,
    ))
}

fn is_settled(world: &mut World, manifest: &CaptureManifest) -> Result<bool, String> {
    let Some((mut godot_particles, mut bevy_particles)) = containers() else {
        return Ok(false);
    };
    let unspawned = manifest.scenario == "rain-2000-unspawned";
    let expected = if unspawned { 0 } else { PARTICLE_COUNT };
    if !world.contains_resource::<Prepared>() {
        let scene = world
            .resource::<AssetServer>()
            .load::<GodotResource>("scenes/particle.tscn");
        if !world.resource::<Assets<GodotResource>>().contains(&scene) {
            return Ok(false);
        }
        fastrand::seed(manifest.seed);
        let bounds = Vector2::new(
            manifest.viewport[0] as f32 / 2.0,
            manifest.viewport[1] as f32,
        );
        godot_particles.call(
            "capture_prepare",
            &[
                (manifest.seed as i64).to_variant(),
                (PARTICLE_COUNT as i32).to_variant(),
                bounds.to_variant(),
                unspawned.to_variant(),
            ],
        );
        {
            let mut rain = bevy_particles.bind_mut();
            rain.screen_size = bounds;
            rain.target_particle_count = PARTICLE_COUNT as i32;
            rain.is_running = !unspawned;
        }
        world.insert_resource(Prepared);
        return Ok(false);
    }

    if !godot_particles
        .call("capture_is_settled", &[(expected as i32).to_variant()])
        .try_to::<bool>()
        .map_err(|error| error.to_string())?
        || world.resource::<ParticleCount>().current != expected as i32
    {
        return Ok(false);
    }

    let mut state = SystemState::<(
        Query<(Option<&GodotNodeHandle>, Has<NeedsColorization>), With<Particle>>,
        GodotAccess,
    )>::new(world);
    let (query, mut godot) = state.get_mut(world).map_err(|error| error.to_string())?;
    if query.iter().count() != expected {
        return Ok(false);
    }
    for (handle, needs_colorization) in &query {
        let Some(handle) = handle else {
            return Ok(false);
        };
        if needs_colorization {
            return Ok(false);
        }
        let Some(node) = godot.try_get::<Node2D>(*handle) else {
            return Ok(false);
        };
        if !node
            .try_get_node_as::<Sprite2D>("Sprite")
            .and_then(|sprite| sprite.get_texture())
            .is_some_and(|texture| texture.get_width() > 0 && texture.get_height() > 0)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn seeded_layout(seed: u64, count: usize, width: u32) -> (Vec<Vector2>, Vec<Vector2>) {
    let mut rng = fastrand::Rng::with_seed(seed);
    (0..count)
        .map(|_| {
            let position = Vector2::new(rng.u32(0..width) as f32, -50.0);
            let velocity = Vector2::new(rng.i32(-25..=25) as f32, rng.u32(50..=300) as f32);
            (position, velocity)
        })
        .unzip()
}

fn reset(world: &mut World, manifest: &CaptureManifest) -> Result<(), String> {
    RenderingServer::singleton().set_default_clear_color(Color::BLACK);
    fastrand::seed(manifest.seed);
    let (mut godot_particles, mut bevy_particles) =
        containers().ok_or("missing rain containers")?;
    let width = manifest.viewport[0] / 2;
    godot_particles.set_position(Vector2::ZERO);
    godot_particles.set_visible(true);
    bevy_particles.set_position(Vector2::new(width as f32, 0.0));
    bevy_particles.set_visible(true);
    bevy_particles.bind_mut().target_particle_count = PARTICLE_COUNT as i32;
    world.resource_mut::<ParticleCount>().target = PARTICLE_COUNT as i32;

    let count = if manifest.scenario == "rain-2000-unspawned" {
        0
    } else {
        PARTICLE_COUNT
    };
    let (positions, velocities) = seeded_layout(manifest.seed, count, width);
    godot_particles.call(
        "capture_reset",
        &[
            PackedVector2Array::from(positions.as_slice()).to_variant(),
            PackedVector2Array::from(velocities.as_slice()).to_variant(),
            (PARTICLE_COUNT as i32).to_variant(),
        ],
    );

    let mut state = SystemState::<(
        Query<(Entity, &GodotNodeHandle, &mut Transform, &mut Velocity), With<Particle>>,
        GodotAccess,
    )>::new(world);
    let (mut query, mut godot) = state.get_mut(world).map_err(|error| error.to_string())?;
    let mut particles: Vec<_> = query.iter_mut().collect();
    particles.sort_unstable_by_key(|(entity, _, _, _)| entity.to_bits());
    if particles.len() != count {
        return Err("particle count changed during reset".into());
    }
    let parent = bevy_particles.upcast::<godot::classes::Node>();
    for ((_, handle, mut transform, mut velocity), (position, speed)) in particles
        .into_iter()
        .zip(positions.into_iter().zip(velocities))
    {
        transform.translation = Vec3::new(position.x, position.y, 0.0);
        velocity.0 = speed;
        let mut node = godot.get::<Node2D>(*handle);
        node.reparent(&parent);
        node.set_position(position);
        node.set_visible(true);
        node.get_node_as::<Sprite2D>("Sprite")
            .set_modulate(Color::from_rgba(1.0, 1.0, 1.0, 0.8));
    }
    Ok(())
}

fn before_frame(_: &mut World, manifest: &CaptureManifest, _: u32) -> Result<(), String> {
    let (mut godot_particles, _) = containers().ok_or("missing rain containers")?;
    godot_particles.set(
        "capture_running",
        &(manifest.scenario != "rain-2000-unspawned").to_variant(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::seeded_layout;
    use godot::builtin::Vector2;

    #[test]
    fn seed_11_particle_positions() {
        let (positions, _) = seeded_layout(11, 4, 240);
        assert_eq!(
            positions,
            vec![
                Vector2::new(168.0, -50.0),
                Vector2::new(194.0, -50.0),
                Vector2::new(101.0, -50.0),
                Vector2::new(239.0, -50.0),
            ]
        );
    }
}
