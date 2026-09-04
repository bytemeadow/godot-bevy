#[cfg(test)]
mod tests {
    // Regression guard for the `godot_to_bevy:`/bidirectional arm: its `.run_if(...)`
    // resolves via `IntoScheduleConfigs`, so the macro must pull the trait into scope
    // itself. We import only what a minimal external caller needs -- deliberately NOT
    // `IntoScheduleConfigs` -- so a missing in-macro import fails this compile.
    use super::GodotTransformSyncPluginExt;
    use crate::bevy_app::App;
    use crate::bevy_ecs::prelude::{Component, With};
    use crate::plugins::transforms::{GodotTransformSyncPlugin, TransformSyncMode};

    #[derive(Component)]
    struct PhysicsActor;

    #[test]
    fn godot_to_bevy_arm_resolves_run_if_without_trait_import() {
        let mut app = App::new();
        crate::add_transform_sync_systems! {
            app,
            PhysicsResults = godot_to_bevy: With<PhysicsActor>,
        }
    }

    #[test]
    fn plugin_extension_preserves_unmodified_configuration() {
        let disabled = GodotTransformSyncPlugin::default().without_auto_sync();
        assert!(!disabled.auto_sync);
        assert_eq!(disabled.sync_mode, TransformSyncMode::OneWay);

        let two_way = GodotTransformSyncPlugin::default().with_sync_mode(TransformSyncMode::TwoWay);
        assert!(two_way.auto_sync);
        assert_eq!(two_way.sync_mode, TransformSyncMode::TwoWay);
    }
}
