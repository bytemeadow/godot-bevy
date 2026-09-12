use bevy::ecs::system::SystemState;
use bevy::prelude::{
    App, FixedUpdate, IntoScheduleConfigs, Query, Resource, Transform, With, World,
};
use godot::classes::{MeshInstance2D, RenderingServer};
use godot::prelude::*;
use godot_bevy::prelude::{GodotAccess, GodotNodeHandle, MeshInstance2DMarker};
use godot_bevy_test::capture::{self, CaptureAdapter, CaptureManifest, Phase};

#[derive(Default, Resource)]
pub(super) struct CaptureFrame(pub u32);

pub(super) fn install(app: &mut App) {
    app.init_resource::<CaptureFrame>();
    app.add_systems(
        FixedUpdate,
        super::update_quad_y_position.run_if(|| capture::phase() == Phase::Running),
    );
    capture::install(
        app,
        CaptureAdapter {
            name: "two-way-sync-demo",
            extension: |_, _, _| Ok(()),
            scenarios: &["orbit-split", "orbit-split-hidden"],
            is_settled,
            reset,
            before_frame,
        },
    );
}

fn is_settled(world: &mut World, _: &CaptureManifest) -> Result<bool, String> {
    let mut state = SystemState::<(
        Query<&GodotNodeHandle, (With<MeshInstance2DMarker>, With<Transform>)>,
        GodotAccess,
    )>::new(world);
    let (query, mut godot) = state.get_mut(world).map_err(|error| error.to_string())?;
    let Ok(handle) = query.single() else {
        return Ok(false);
    };
    let Some(quad) = godot.try_get::<MeshInstance2D>(*handle) else {
        return Ok(false);
    };
    Ok(quad.get_name() == "Quad"
        && quad.get_mesh().is_some()
        && quad.has_method("set_capture_frame"))
}

fn reset(world: &mut World, manifest: &CaptureManifest) -> Result<(), String> {
    world.insert_resource(CaptureFrame(0));
    RenderingServer::singleton().set_default_clear_color(Color::BLACK);
    let mut state = SystemState::<(
        Query<(&mut Transform, &GodotNodeHandle), With<MeshInstance2DMarker>>,
        GodotAccess,
    )>::new(world);
    let (mut query, mut godot) = state.get_mut(world).map_err(|error| error.to_string())?;
    let (mut transform, handle) = query.single_mut().map_err(|error| error.to_string())?;
    let mut quad = godot
        .try_get::<MeshInstance2D>(*handle)
        .ok_or("missing Quad")?;
    transform.translation.x = 0.0;
    transform.translation.y = 100.0;
    quad.set_position(Vector2::new(0.0, 100.0));
    quad.set_modulate(Color::WHITE);
    quad.set_visible(manifest.scenario != "orbit-split-hidden");
    Ok(())
}

fn before_frame(world: &mut World, _: &CaptureManifest, frame: u32) -> Result<(), String> {
    world.resource_mut::<CaptureFrame>().0 = frame;
    let mut state = SystemState::<(
        Query<&GodotNodeHandle, With<MeshInstance2DMarker>>,
        GodotAccess,
    )>::new(world);
    let (query, mut godot) = state.get_mut(world).map_err(|error| error.to_string())?;
    let handle = query.single().map_err(|error| error.to_string())?;
    let mut quad = godot
        .try_get::<MeshInstance2D>(*handle)
        .ok_or("missing Quad")?;
    quad.call("set_capture_frame", &[frame.to_variant()]);
    if frame == 1 {
        quad.set_visible(true);
    }
    Ok(())
}
