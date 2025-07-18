use bevy::ecs::query::Changed;
use bevy::ecs::system::Query;
use bevy::log::info;
use bevy::prelude::{App, Update};
use bevy::transform::components::Transform;
use godot::global::godot_print;
use godot_bevy::prelude::godot_prelude::ExtensionLibrary;
use godot_bevy::prelude::godot_prelude::gdextension;
use godot_bevy::prelude::{
    GodotCorePlugins, GodotTransformSyncPlugin, TransformSyncMode, bevy_app,
};

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
    });

    app.add_systems(Update, print_positions);
}

fn print_positions(mut query: Query<&mut Transform, Changed<Transform>>) {
    // TODO print once per second or something so we're not spamming, or perhaps only on change?
    // for mut transform in query.iter_mut() {
    //     transform.translation.x += 1.;
    // }
}
