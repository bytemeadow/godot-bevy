#![allow(unused)] // scaffolding: never-called fns, glob imports

use bevy::prelude::*;
use godot_bevy::prelude::*;

#[derive(Event, Clone)]
struct UiSignal; // satisfies the GodotSignalsPlugin<T> Event bound

// Each fn below mirrors one plugins.md funnel block. The `#[bevy_app] fn build_app`
// header stays literal in the .md; only the anchored body is `{{#include}}`d.
// `#[rustfmt::skip]` keeps the shown formatting byte-faithful to the rendered page.

#[rustfmt::skip]
fn adding_features(app: &mut App) {
// ANCHOR: adding_features
    app.add_plugins(GodotTransformSyncPlugin::default())
        .add_plugins(GodotAudioPlugin)
        .add_plugins(BevyInputBridgePlugin);
// ANCHOR_END: adding_features
}

#[rustfmt::skip]
fn everything(app: &mut App) {
// ANCHOR: everything
    app.add_plugins(GodotDefaultPlugins);
// ANCHOR_END: everything
}

#[rustfmt::skip]
fn pure_ecs(app: &mut App) {
// ANCHOR: pure_ecs
    app.add_plugins(GodotTransformSyncPlugin::default())
        .add_plugins(GodotAudioPlugin)
        .add_plugins(BevyInputBridgePlugin);
// ANCHOR_END: pure_ecs
}

#[rustfmt::skip]
fn physics_platformer(app: &mut App) {
// ANCHOR: physics_platformer
    app.add_plugins(GodotTransformSyncPlugin {
            sync_mode: TransformSyncMode::Disabled,  // Use Godot physics
            ..Default::default()
        })
        .add_plugins(GodotCollisionsPlugin)
        .add_plugins(GodotSignalsPlugin::<UiSignal>::default())
        .add_plugins(GodotAudioPlugin);
// ANCHOR_END: physics_platformer
}

#[rustfmt::skip]
fn ui_heavy(app: &mut App) {
// ANCHOR: ui_heavy
    app.add_plugins(GodotSignalsPlugin::<UiSignal>::default())
        .add_plugins(BevyInputBridgePlugin)
        .add_plugins(GodotAudioPlugin);
// ANCHOR_END: ui_heavy
}

// No `#[bevy_app]` wrapper in the .md for this one, so the body sits at column 0.
#[rustfmt::skip]
fn transform_sync_modes(app: &mut App) {
// ANCHOR: transform_sync_modes
// Default: One-way sync (Bevy → Godot)
app.add_plugins(GodotTransformSyncPlugin::default());

// Two-way sync (Bevy ↔ Godot)
app.add_plugins(GodotTransformSyncPlugin {
    sync_mode: TransformSyncMode::TwoWay,
    ..Default::default()
});

// Disabled (use Godot physics directly)
app.add_plugins(GodotTransformSyncPlugin {
    sync_mode: TransformSyncMode::Disabled,
    ..Default::default()
});
// ANCHOR_END: transform_sync_modes
}
