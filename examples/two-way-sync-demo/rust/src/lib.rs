#![allow(clippy::type_complexity)]

use bevy::app::Update;
use bevy::ecs::query::{Changed, With};
use bevy::ecs::system::Query;
use bevy::prelude::App;
use bevy::utils::default;
use bevy::{math::ops::cos, transform::components::Transform};
use godot::classes::Engine;
use godot::obj::Singleton;
use godot_bevy::prelude::{GodotBevyLogPlugin, MeshInstance2DMarker};
use godot_bevy::prelude::{GodotTransformSyncPlugin, TransformSyncMode, bevy_app};

#[cfg(feature = "capture")]
mod capture;

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
        ..default()
    })
    .add_plugins(GodotBevyLogPlugin::default());

    #[cfg(feature = "capture")]
    if godot_bevy_test::capture::is_enabled() {
        capture::install(app);
        return;
    }
    app.add_systems(Update, update_quad_y_position);
}

fn update_quad_y_position(
    mut query: Query<&mut Transform, (With<MeshInstance2DMarker>, Changed<Transform>)>,
    #[cfg(feature = "capture")] capture_frame: Option<bevy::prelude::Res<capture::CaptureFrame>>,
) {
    #[cfg(feature = "capture")]
    let frame = capture_frame
        .map(|frame| frame.0 as f32)
        .unwrap_or_else(|| Engine::singleton().get_frames_drawn() as f32);
    #[cfg(not(feature = "capture"))]
    let frame = Engine::singleton().get_frames_drawn() as f32;
    for mut transform in query.iter_mut() {
        transform.translation.y = cos(frame / 50.) * 100.;
    }
}
