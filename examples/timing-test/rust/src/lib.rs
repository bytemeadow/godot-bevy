#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use godot_bevy::prelude::{godot_prelude::godot_print, *};

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
            .add_systems(Last, last_schedule_system);
    }
}

#[derive(Resource, Default)]
struct ProcessCallCounter {
    physics_process_calls: u32,
    prefix_runs: u32,
}

#[derive(Resource, Default)]
struct TimingStats {
    update_runs: u32,
    fixed_update_runs: u32,
    first_schedule_runs: u32,
}

fn setup_timing_test() {
    godot_print!("🚀 Timing Test Started!");
    godot_print!("📊 Watching for timing behavior...");
    godot_print!(
        "⏱️  prefix (First/PreUpdate) + FixedUpdate run in physics_process; suffix (Update/PostUpdate/Last) runs in process"
    );
}

fn first_schedule_system(
    mut stats: ResMut<TimingStats>,
    mut counter: ResMut<ProcessCallCounter>,
    time: Res<Time>,
) {
    stats.first_schedule_runs += 1;
    counter.prefix_runs += 1;

    if stats.first_schedule_runs.is_multiple_of(60) {
        godot_print!(
            "🔍 DEBUG: First Schedule #{}: prefix_runs: {}, Time: {:.2}s",
            stats.first_schedule_runs,
            counter.prefix_runs,
            time.elapsed_secs()
        );
    }

    if stats.first_schedule_runs.is_multiple_of(120) {
        godot_print!(
            "📺 First Schedule Run #{}: Time: {:.2}s (First runs in the physics_process prefix)",
            stats.first_schedule_runs,
            time.elapsed_secs()
        );
    }
}

fn pre_update_system(time: Res<Time>, counter: Res<ProcessCallCounter>) {
    if time.elapsed_secs() % 3.0 < 0.017 {
        godot_print!(
            "🔄 PreUpdate at {:.2}s (prefix_runs: {})",
            time.elapsed_secs(),
            counter.prefix_runs
        );
    }
}

fn update_system(mut stats: ResMut<TimingStats>, time: Res<Time>) {
    stats.update_runs += 1;

    if time.elapsed_secs() % 4.0 < 0.017 {
        godot_print!(
            "📋 Update running at {:.2}s (Update runs in the process suffix)",
            time.elapsed_secs()
        );
    }
}

fn fixed_update_system(
    mut stats: ResMut<TimingStats>,
    mut counter: ResMut<ProcessCallCounter>,
    time: Res<Time>,
) {
    stats.fixed_update_runs += 1;
    counter.physics_process_calls += 1;

    // FixedUpdate now ticks on Godot's authoritative physics clock (default 60 Hz).
    // Res<Time>.delta_secs() is Godot's physics delta — one clock, not two.
    if stats.fixed_update_runs.is_multiple_of(60) {
        godot_print!(
            "⚡ FixedUpdate #{}: physics_process_calls: {}, Godot delta: {:.4}s",
            stats.fixed_update_runs,
            counter.physics_process_calls,
            time.delta_secs(),
        );
    }
}

fn post_update_system(time: Res<Time>) {
    if time.elapsed_secs() % 5.0 < 0.017 {
        godot_print!(
            "📤 PostUpdate running at {:.2}s (PostUpdate runs in the process suffix)",
            time.elapsed_secs()
        );
    }
}

fn last_schedule_system(stats: Res<TimingStats>, time: Res<Time>) {
    if time.elapsed_secs() % 6.0 < 0.017 {
        godot_print!(
            "🏁 Last Schedule: Update runs: {}, Fixed updates: {}, Time: {:.2}s",
            stats.update_runs,
            stats.fixed_update_runs,
            time.elapsed_secs()
        );
    }
}
