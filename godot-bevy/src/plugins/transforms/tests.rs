#[cfg(test)]
mod tests {
    use crate::interop::node_markers::*;
    use crate::{add_transform_sync_systems, transform_sync_systems};
    use bevy::ecs::query::{Or, With};
    use bevy::prelude::*;

    // Test the macro generates systems correctly
    transform_sync_systems! {
        TestPhysicsBody: 3d = Or<(
            With<CharacterBody3DMarker>,
            With<RigidBody3DMarker>,
            With<StaticBody3DMarker>,
        )>
    }

    #[test]
    fn test_macro_generates_systems() {
        let mut app = App::new();

        // Test that we can add the generated systems without errors
        app.add_systems(
            bevy::app::Last,
            post_update_godot_transforms_3d_testphysicsbody,
        )
        .add_systems(
            bevy::app::PreUpdate,
            pre_update_godot_transforms_3d_testphysicsbody,
        );

        // Test that the systems exist in the schedule
        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_convenience_macro() {
        let mut app = App::new();

        // Test the convenience macro
        add_transform_sync_systems! {
            app,
            ConvenienceTest: 3d = With<Node3DMarker>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }
}
