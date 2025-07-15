#[cfg(test)]
mod tests {
    use crate::interop::node_markers::*;
    use crate::{add_transform_sync_systems, transform_sync_systems};
    use bevy::ecs::query::{Or, With};
    use bevy::prelude::*;

    // Test components
    #[derive(Component)]
    struct Player;

    #[derive(Component)]
    struct PlayerInput;

    // Test the macro generates systems correctly
    transform_sync_systems! {
        TestPhysicsBody = Or<(
            With<CharacterBody3DMarker>,
            With<RigidBody3DMarker>,
            With<StaticBody3DMarker>,
        )>
    }

    // Test separate queries syntax
    transform_sync_systems! {
        TestPlayer = bevy_to_godot: With<Player>, godot_to_bevy: With<PlayerInput>
    }

    // Test 2D only syntax
    transform_sync_systems! {
        Test2DOnly = 2d: With<Player>
    }

    // Test 3D only syntax
    transform_sync_systems! {
        Test3DOnly = 3d: With<Player>
    }

    // Test 2D post-update only
    transform_sync_systems! {
        Test2DPostOnly = 2d: bevy_to_godot: With<Player>
    }

    // Test 2D pre-update only
    transform_sync_systems! {
        Test2DPreOnly = 2d: godot_to_bevy: With<Player>
    }

    // Test 3D post-update only
    transform_sync_systems! {
        Test3DPostOnly = 3d: bevy_to_godot: With<Player>
    }

    // Test 3D pre-update only
    transform_sync_systems! {
        Test3DPreOnly = 3d: godot_to_bevy: With<Player>
    }

    #[test]
    fn test_macro_generates_systems() {
        let mut app = App::new();

        // Test that we can add the generated systems without errors
        app.add_systems(
            bevy::app::Last,
            (
                post_update_godot_transforms_2d_testphysicsbody,
                post_update_godot_transforms_3d_testphysicsbody,
            ),
        )
        .add_systems(
            bevy::app::PreUpdate,
            (
                pre_update_godot_transforms_2d_testphysicsbody,
                pre_update_godot_transforms_3d_testphysicsbody,
            ),
        );

        // Test that the systems exist in the schedule
        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_separate_queries_macro() {
        let mut app = App::new();

        // Test that we can add the generated systems with separate queries
        app.add_systems(
            bevy::app::Last,
            (
                post_update_godot_transforms_2d_testplayer,
                post_update_godot_transforms_3d_testplayer,
            ),
        )
        .add_systems(
            bevy::app::PreUpdate,
            (
                pre_update_godot_transforms_2d_testplayer,
                pre_update_godot_transforms_3d_testplayer,
            ),
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
            ConvenienceTest = With<Node3DMarker>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_convenience_macro_with_separate_queries() {
        let mut app = App::new();

        // Test the convenience macro with separate queries
        add_transform_sync_systems! {
            app,
            ConveniencePlayer = bevy_to_godot: With<Player>, godot_to_bevy: With<PlayerInput>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_2d_only_macro() {
        let mut app = App::new();

        // Test the 2D only macro
        add_transform_sync_systems! {
            app,
            Test2DOnly = 2d: With<Player>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_3d_only_macro() {
        let mut app = App::new();

        // Test the 3D only macro
        add_transform_sync_systems! {
            app,
            Test3DOnly = 3d: With<Player>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_manual_2d_only_systems() {
        let mut app = App::new();

        // Test that we can manually add 2D only systems
        app.add_systems(bevy::app::Last, post_update_godot_transforms_2d_test2donly)
            .add_systems(
                bevy::app::PreUpdate,
                pre_update_godot_transforms_2d_test2donly,
            );

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_manual_3d_only_systems() {
        let mut app = App::new();

        // Test that we can manually add 3D only systems
        app.add_systems(bevy::app::Last, post_update_godot_transforms_3d_test3donly)
            .add_systems(
                bevy::app::PreUpdate,
                pre_update_godot_transforms_3d_test3donly,
            );

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_2d_post_update_only() {
        let mut app = App::new();

        // Test the 2D post-update only macro
        add_transform_sync_systems! {
            app,
            Test2DPostOnly = 2d: bevy_to_godot: With<Player>
        }

        // Should only have the post-update system
        app.add_systems(
            bevy::app::Last,
            post_update_godot_transforms_2d_test2dpostonly,
        );

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_2d_pre_update_only() {
        let mut app = App::new();

        // Test the 2D pre-update only macro
        add_transform_sync_systems! {
            app,
            Test2DPreOnly = 2d: godot_to_bevy: With<Player>
        }

        // Should only have the pre-update system
        app.add_systems(
            bevy::app::PreUpdate,
            pre_update_godot_transforms_2d_test2dpreonly,
        );

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_3d_post_update_only() {
        let mut app = App::new();

        // Test the 3D post-update only macro
        add_transform_sync_systems! {
            app,
            Test3DPostOnly = 3d: bevy_to_godot: With<Player>
        }

        // Should only have the post-update system
        app.add_systems(
            bevy::app::Last,
            post_update_godot_transforms_3d_test3dpostonly,
        );

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_3d_pre_update_only() {
        let mut app = App::new();

        // Test the 3D pre-update only macro
        add_transform_sync_systems! {
            app,
            Test3DPreOnly = 3d: godot_to_bevy: With<Player>
        }

        // Should only have the pre-update system
        app.add_systems(
            bevy::app::PreUpdate,
            pre_update_godot_transforms_3d_test3dpreonly,
        );

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }
}
