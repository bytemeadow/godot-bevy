#![allow(clippy::type_complexity)]

use bevy::app::Update;
use bevy::ecs::query::{Changed, With};
use bevy::ecs::system::Query;
use bevy::prelude::App;
use bevy::utils::default;
use bevy::{math::ops::cos, transform::components::Transform};
use godot::classes::Engine;
use godot_bevy::prelude::MeshInstance2DMarker;
use godot_bevy::prelude::godot_prelude::ExtensionLibrary;
use godot_bevy::prelude::godot_prelude::gdextension;
use godot_bevy::prelude::{GodotTransformSyncPlugin, TransformSyncMode, bevy_app};

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
        ..default()
    });

    app.add_systems(Update, update_quad_y_position);
}

fn update_quad_y_position(
    mut query: Query<&mut Transform, (With<MeshInstance2DMarker>, Changed<Transform>)>,
) {
    for mut transform in query.iter_mut() {
        // Notice that we only change the y coordinate here. The x coordinate is
        // also updated every frame, except it's done in GDScript, see quad.gd
        transform.translation.y = cos(Engine::singleton().get_frames_drawn() as f32 / 50.) * 100.;
    }
}
