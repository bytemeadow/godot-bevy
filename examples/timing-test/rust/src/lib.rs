#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use godot_bevy::prelude::{
    godot_prelude::{gdextension, godot_print, ExtensionLibrary},
    *,
};

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(TimingTestPlugin);
}

struct TimingTestPlugin;

impl Plugin for TimingTestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimingStats>()
            .init_resource::<ProcessCallCounter>()
            .add_systems(Startup, setup_timing_test)
            .add_systems(First, first_schedule_system)
            .add_systems(PreUpdate, pre_update_system)
            .add_systems(Update, update_system)
            .add_systems(FixedUpdate, fixed_update_system)
            .add_systems(PostUpdate, post_update_system)
            .add_systems(Last, last_schedule_system)
            .add_systems(PhysicsUpdate, physics_update_system);
    }
}

#[derive(Resource, Default)]
struct ProcessCallCounter {
    physics_process_calls: u32,
    app_update_calls: u32,
}

#[derive(Resource, Default)]
struct TimingStats {
    update_runs: u32,
    physics_update_runs: u32,
    fixed_update_runs: u32,
    first_schedule_runs: u32,
}

fn setup_timing_test() {
    godot_print!("🚀 Timing Test Started!");
    godot_print!("📊 Watching for timing behavior...");
    godot_print!("⏱️  app.update() runs in process(), PhysicsUpdate runs in physics_process()");
}

fn first_schedule_system(
    mut stats: ResMut<TimingStats>,
    mut counter: ResMut<ProcessCallCounter>,
    time: Res<Time>,
) {
    stats.first_schedule_runs += 1;
    counter.app_update_calls += 1;

    // Log every few runs to show execution frequency
    if stats.first_schedule_runs % 60 == 0 {
        // Every second at 60fps
        godot_print!(
            "🔍 DEBUG: First Schedule #{}: app_update_calls: {}, Time: {:.2}s",
            stats.first_schedule_runs,
            counter.app_update_calls,
            time.elapsed_secs()
        );
    }

    // Original periodic logging
    if stats.first_schedule_runs % 120 == 0 {
        // Every 2 seconds at 60fps
        godot_print!(
            "📺 First Schedule Run #{}: Time: {:.2}s (runs in app.update())",
            stats.first_schedule_runs,
            time.elapsed_secs()
        );
    }
}

fn pre_update_system(time: Res<Time>, counter: Res<ProcessCallCounter>) {
    if time.elapsed_secs() % 3.0 < 0.017 {
        godot_print!(
            "🔄 PreUpdate at {:.2}s (app_update_calls: {})",
            time.elapsed_secs(),
            counter.app_update_calls
        );
    }
}

fn update_system(mut stats: ResMut<TimingStats>, time: Res<Time>) {
    stats.update_runs += 1;

    if time.elapsed_secs() % 4.0 < 0.017 {
        godot_print!(
            "📋 Update running at {:.2}s (part of app.update())",
            time.elapsed_secs()
        );
    }
}

fn fixed_update_system(mut stats: ResMut<TimingStats>, time: Res<Time>) {
    stats.fixed_update_runs += 1;

    // FixedUpdate runs as part of app.update(), maintaining its own timing
    if stats.fixed_update_runs % 128 == 0 {
        // Every ~2 seconds at 64Hz
        godot_print!(
            "🔧 FixedUpdate Run #{}: Time: {:.2}s (Bevy's internal 64Hz timing)",
            stats.fixed_update_runs,
            time.elapsed_secs()
        );
    }
}

fn post_update_system(time: Res<Time>) {
    if time.elapsed_secs() % 5.0 < 0.017 {
        godot_print!(
            "📤 PostUpdate running at {:.2}s (part of app.update())",
            time.elapsed_secs()
        );
    }
}

fn last_schedule_system(stats: Res<TimingStats>, time: Res<Time>) {
    if time.elapsed_secs() % 6.0 < 0.017 {
        godot_print!(
            "🏁 Last Schedule: Update runs: {}, Physics runs: {}, Fixed updates: {}, Time: {:.2}s",
            stats.update_runs,
            stats.physics_update_runs,
            stats.fixed_update_runs,
            time.elapsed_secs()
        );
    }
}

fn physics_update_system(
    mut stats: ResMut<TimingStats>,
    mut counter: ResMut<ProcessCallCounter>,
    time: Res<Time>,
) {
    stats.physics_update_runs += 1;
    counter.physics_process_calls += 1;

    // This runs in physics_process() at Godot's physics framerate
    if stats.physics_update_runs % 60 == 0 {
        // Every second at 60Hz physics
        godot_print!(
            "⚡ PhysicsUpdate #{}: physics_process_calls: {}, Time: {:.2}s",
            stats.physics_update_runs,
            counter.physics_process_calls,
            time.elapsed_secs()
        );
    }
}
