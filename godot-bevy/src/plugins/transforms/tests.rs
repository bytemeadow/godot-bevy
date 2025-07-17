#[cfg(test)]
#[allow(dead_code)]
mod test_transforms {
    use crate::interop::node_markers::*;
    use crate::{add_transform_sync_systems_2d, add_transform_sync_systems_3d};
    use bevy::ecs::query::{Or, With};
    use bevy::prelude::*;

    // Test components
    #[derive(Component)]
    pub struct Player;

    #[derive(Component)]
    pub struct PlayerInput;

    #[test]
    fn test_2d_macro_same_query() {
        let mut app = App::new();

        // Test the 2D macro with same query for both directions
        add_transform_sync_systems_2d! {
            app,
            Test2D = With<Player>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_3d_macro_same_query() {
        let mut app = App::new();

        // Test the 3D macro with same query for both directions
        add_transform_sync_systems_3d! {
            app,
            Test3D = With<Player>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_2d_macro_separate_queries() {
        let mut app = App::new();

        // Test the 2D macro with separate queries for each direction
        add_transform_sync_systems_2d! {
            app,
            Test2DPlayerPost = bevy_to_godot: With<Player>
        }

        add_transform_sync_systems_2d! {
            app,
            Test2DPlayerPre = godot_to_bevy: With<PlayerInput>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_3d_macro_separate_queries() {
        let mut app = App::new();

        // Test the 3D macro with separate queries for each direction
        add_transform_sync_systems_3d! {
            app,
            Test3DPlayerPost = bevy_to_godot: With<Player>
        }

        add_transform_sync_systems_3d! {
            app,
            Test3DPlayerPre = godot_to_bevy: With<PlayerInput>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_2d_bevy_to_godot_only() {
        let mut app = App::new();

        // Test 2D bevy_to_godot only
        add_transform_sync_systems_2d! {
            app,
            Test2DPostOnly = bevy_to_godot: With<Player>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_2d_godot_to_bevy_only() {
        let mut app = App::new();

        // Test 2D godot_to_bevy only
        add_transform_sync_systems_2d! {
            app,
            Test2DPreOnly = godot_to_bevy: With<Player>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_3d_bevy_to_godot_only() {
        let mut app = App::new();

        // Test 3D bevy_to_godot only
        add_transform_sync_systems_3d! {
            app,
            Test3DPostOnly = bevy_to_godot: With<Player>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_3d_godot_to_bevy_only() {
        let mut app = App::new();

        // Test 3D godot_to_bevy only
        add_transform_sync_systems_3d! {
            app,
            Test3DPreOnly = godot_to_bevy: With<Player>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_complex_query_2d() {
        let mut app = App::new();

        // Test complex query with 2D
        add_transform_sync_systems_2d! {
            app,
            TestPhysicsBody2D = Or<(
                With<CharacterBody2DMarker>,
                With<RigidBody2DMarker>,
                With<StaticBody2DMarker>,
            )>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_complex_query_3d() {
        let mut app = App::new();

        // Test complex query with 3D
        add_transform_sync_systems_3d! {
            app,
            TestPhysicsBody3D = Or<(
                With<CharacterBody3DMarker>,
                With<RigidBody3DMarker>,
                With<StaticBody3DMarker>,
            )>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_multiple_systems_2d() {
        let mut app = App::new();

        // Test multiple systems in one macro call
        add_transform_sync_systems_2d! {
            app,
            TestPlayer2D = With<Player>,
            TestInput2D = With<PlayerInput>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_multiple_systems_3d() {
        let mut app = App::new();

        // Test multiple systems in one macro call
        add_transform_sync_systems_3d! {
            app,
            TestPlayer3D = With<Player>,
            TestInput3D = With<PlayerInput>
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_mixed_directional_sync_2d() {
        let mut app = App::new();

        // Test mixed directional sync syntax - all directions in one call!
        add_transform_sync_systems_2d! {
            app,
            UIElements = bevy_to_godot: With<Player>,
            PhysicsResults = godot_to_bevy: With<PlayerInput>,
            Interactive = With<Player>,
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }

    #[test]
    fn test_mixed_directional_sync_3d() {
        let mut app = App::new();

        // Test mixed directional sync syntax - all directions in one call!
        add_transform_sync_systems_3d! {
            app,
            VisualEffects = bevy_to_godot: With<Player>,
            PhysicsResults = godot_to_bevy: With<PlayerInput>,
            Interactive = With<Player>,
        }

        assert!(
            app.world()
                .contains_resource::<bevy::ecs::schedule::Schedules>()
        );
    }
}
