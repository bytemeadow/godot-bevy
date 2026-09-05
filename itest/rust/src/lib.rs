use godot::init::{ExtensionLibrary, gdextension};

#[cfg(all(feature = "coverage-flush", not(coverage)))]
compile_error!("coverage-flush requires cfg(coverage)");

#[cfg(feature = "coverage-flush")]
mod coverage_flush;

godot_bevy_test::declare_test_runner!();

mod asset_reader_tests;
mod attachable_component_tests;
mod autosync_match_tests;
mod benchmarks;
mod collision_tests;
mod event_bridge_tests;
#[cfg(feature = "harness-probes")]
mod harness_probe_tests;
mod input_ecosystem_tests;
mod input_tests;
#[cfg(feature = "autosync-tests")]
mod macro_redesign_tests;
mod pause_tests;
mod real_frame_tests;
mod scene_tree_tests;
mod scene_tree_watcher_init_tests;
mod shutdown_tests;
mod signal_tests;
mod time_scale_tests;
mod transform_sync_tests;

#[gdextension(entry_symbol = godot_bevy_itest)]
unsafe impl ExtensionLibrary for IntegrationTests {
    #[cfg(feature = "profile-tracy")]
    fn on_stage_init(stage: godot::init::InitStage) {
        if stage == godot::init::InitStage::Scene {
            godot_bevy_test::profiling::install_profile_subscriber();
        }
    }

    #[cfg(feature = "coverage-flush")]
    fn on_stage_deinit(stage: godot::init::InitStage) {
        coverage_flush::dump(stage);
    }
}
