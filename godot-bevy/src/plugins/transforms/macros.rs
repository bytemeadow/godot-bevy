/// Macro for generating custom transform synchronization systems with compile-time queries.
///
/// This macro generates systems that sync transforms between Bevy and Godot for entities
/// matching specific component queries. The queries are compiled at compile-time for
/// maximum performance.
///
/// # Usage
///
/// ```rust
/// use godot_bevy::plugins::transforms::transform_sync_systems;
/// use godot_bevy::interop::node_markers::*;
/// use bevy::ecs::query::{Or, With};
///
/// // Generate systems for physics bodies only (both 2D and 3D)
/// transform_sync_systems! {
///     PhysicsBody = Or<(
///         With<CharacterBody3DMarker>,
///         With<RigidBody3DMarker>,
///         With<StaticBody3DMarker>,
///     )>
/// }
///
/// // Generate systems for 2D only
/// transform_sync_systems! {
///     Boid2D = 2d: With<Boid>
/// }
///
/// // Generate systems for 3D only
/// transform_sync_systems! {
///     Boid3D = 3d: With<Boid>
/// }
///
/// // Generate systems with different queries for each direction
/// transform_sync_systems! {
///     Player = bevy_to_godot: With<Player>, godot_to_bevy: With<PlayerInput>
/// }
///
/// // Generate systems with different queries for each direction, 2D only
/// transform_sync_systems! {
///     Player2D = 2d: bevy_to_godot: With<Player>, godot_to_bevy: With<PlayerInput>, 3d: none
/// }
/// ```
///
/// The macro generates up to four systems:
/// - `pre_update_godot_transforms_2d_{name}` - Reads 2D transforms from Godot to Bevy
/// - `pre_update_godot_transforms_3d_{name}` - Reads 3D transforms from Godot to Bevy
/// - `post_update_godot_transforms_2d_{name}` - Writes 2D transforms from Bevy to Godot
/// - `post_update_godot_transforms_3d_{name}` - Writes 3D transforms from Bevy to Godot
///
/// You can then add these systems to your Bevy App:
///
/// ```rust
/// app.add_systems(PreUpdate, (pre_update_godot_transforms_2d_physics_body, pre_update_godot_transforms_3d_physics_body))
///    .add_systems(Last, (post_update_godot_transforms_2d_physics_body, post_update_godot_transforms_3d_physics_body));
/// ```
#[macro_export]
macro_rules! transform_sync_systems {
    // Handle same query for both directions (both 2D and 3D)
    ($($name:ident = $query:ty),+ $(,)?) => {
        $(
            $crate::transform_sync_systems!(@generate_systems $name, $query, $query, $query, $query);
        )+
    };

    // Handle different queries for each direction (both 2D and 3D)
    ($($name:ident = bevy_to_godot: $bevy_to_godot_query:ty, godot_to_bevy: $godot_to_bevy_query:ty),+ $(,)?) => {
        $(
            $crate::transform_sync_systems!(@generate_systems $name, $bevy_to_godot_query, $godot_to_bevy_query, $bevy_to_godot_query, $godot_to_bevy_query);
        )+
    };

    // Handle 2D and 3D specific queries with same query for both directions
    ($($name:ident = 2d: $query_2d:ty, 3d: $query_3d:ty),+ $(,)?) => {
        $(
            $crate::transform_sync_systems!(@generate_systems $name, $query_2d, $query_2d, $query_3d, $query_3d);
        )+
    };

    // Handle 2D only
    ($($name:ident = 2d: $query_2d:ty),+ $(,)?) => {
        $(
            $crate::transform_sync_systems!(@generate_systems $name, $query_2d, $query_2d, none, none);
        )+
    };

    // Handle 3D only
    ($($name:ident = 3d: $query_3d:ty),+ $(,)?) => {
        $(
            $crate::transform_sync_systems!(@generate_systems $name, none, none, $query_3d, $query_3d);
        )+
    };

    // Handle 2D and 3D specific queries with different queries for each direction
    ($($name:ident = 2d: bevy_to_godot: $bevy_to_godot_2d:ty, godot_to_bevy: $godot_to_bevy_2d:ty, 3d: bevy_to_godot: $bevy_to_godot_3d:ty, godot_to_bevy: $godot_to_bevy_3d:ty),+ $(,)?) => {
        $(
            $crate::transform_sync_systems!(@generate_systems $name, $bevy_to_godot_2d, $godot_to_bevy_2d, $bevy_to_godot_3d, $godot_to_bevy_3d);
        )+
    };

    // Handle 2D specific with different queries, 3D None
    ($($name:ident = 2d: bevy_to_godot: $bevy_to_godot_2d:ty, godot_to_bevy: $godot_to_bevy_2d:ty, 3d: None),+ $(,)?) => {
        $(
            $crate::transform_sync_systems!(@generate_systems $name, $bevy_to_godot_2d, $godot_to_bevy_2d, None, None);
        )+
    };

    // Handle 3D specific with different queries, 2D None
    ($($name:ident = 2d: None, 3d: bevy_to_godot: $bevy_to_godot_3d:ty, godot_to_bevy: $godot_to_bevy_3d:ty),+ $(,)?) => {
        $(
            $crate::transform_sync_systems!(@generate_systems $name, None, None, $bevy_to_godot_3d, $godot_to_bevy_3d);
        )+
    };

    // Internal macro for generating the actual systems
    (@generate_systems $name:ident, $bevy_to_godot_2d:tt, $godot_to_bevy_2d:tt, $bevy_to_godot_3d:tt, $godot_to_bevy_3d:tt) => {
        $crate::transform_sync_systems!(@generate_2d_systems $name, $bevy_to_godot_2d, $godot_to_bevy_2d);
        $crate::transform_sync_systems!(@generate_3d_systems $name, $bevy_to_godot_3d, $godot_to_bevy_3d);
    };

    // Generate 2D systems if query is not None
    (@generate_2d_systems $name:ident, none, none) => {
        // Skip generation for None queries
    };

    (@generate_2d_systems $name:ident, $bevy_to_godot_2d:ty, $godot_to_bevy_2d:ty) => {
        $crate::paste::paste! {
            #[$crate::prelude::main_thread_system]
            pub fn [<post_update_godot_transforms_2d_ $name:lower>](
                config: bevy::prelude::Res<$crate::plugins::core::GodotCustomTransformSyncConfig>,
                mut entities: bevy::prelude::Query<
                    (&$crate::plugins::transforms::Transform2D, &mut $crate::interop::GodotNodeHandle),
                    (
                        bevy::ecs::query::Or<(
                            bevy::ecs::query::Added<$crate::plugins::transforms::Transform2D>,
                            bevy::ecs::query::Changed<$crate::plugins::transforms::Transform2D>
                        )>,
                        $bevy_to_godot_2d,
                    ),
                >,
            ) {
                // Early return if transform syncing is disabled
                if config.sync_mode == $crate::plugins::core::TransformSyncMode::Disabled {
                    return;
                }

                use godot::builtin::Transform2D as GodotTransform2D;
                use godot::classes::Node2D;
                use $crate::plugins::transforms::conversions::IntoGodotTransform2D;

                for (transform, mut reference) in entities.iter_mut() {
                    if let Some(mut obj) = reference.try_get::<Node2D>() {
                        let mut obj_transform = GodotTransform2D::IDENTITY.translated(obj.get_position());
                        obj_transform = obj_transform.rotated(obj.get_rotation());
                        obj_transform = obj_transform.scaled(obj.get_scale());

                        if obj_transform != *transform.as_godot() {
                            obj.set_transform(*transform.as_godot());
                        }
                    }
                }
            }

            #[$crate::prelude::main_thread_system]
            pub fn [<pre_update_godot_transforms_2d_ $name:lower>](
                config: bevy::prelude::Res<$crate::plugins::core::GodotCustomTransformSyncConfig>,
                mut entities: bevy::prelude::Query<
                    (&mut $crate::plugins::transforms::Transform2D, &mut $crate::interop::GodotNodeHandle),
                    $godot_to_bevy_2d
                >,
            ) {
                // Early return if not using two-way sync
                if config.sync_mode != $crate::plugins::core::TransformSyncMode::TwoWay {
                    return;
                }

                use bevy::ecs::change_detection::DetectChanges;
                use godot::builtin::Transform2D as GodotTransform2D;
                use godot::classes::Node2D;
                use $crate::plugins::transforms::conversions::IntoGodotTransform2D;

                for (mut transform, mut reference) in entities.iter_mut() {
                    // Skip entities that were changed recently (e.g., by PhysicsUpdate systems)
                    if transform.is_changed() {
                        continue;
                    }

                    if let Some(obj) = reference.try_get::<Node2D>() {
                        let mut obj_transform = GodotTransform2D::IDENTITY.translated(obj.get_position());
                        obj_transform = obj_transform.rotated(obj.get_rotation());
                        obj_transform = obj_transform.scaled(obj.get_scale());

                        if obj_transform != *transform.as_godot() {
                            *transform.as_godot_mut() = obj_transform;
                        }
                    }
                }
            }
        }
    };

    // Generate 3D systems if query is not None
    (@generate_3d_systems $name:ident, none, none) => {
        // Skip generation for None queries
    };

    (@generate_3d_systems $name:ident, $bevy_to_godot_3d:ty, $godot_to_bevy_3d:ty) => {
        $crate::paste::paste! {
            #[$crate::prelude::main_thread_system]
            pub fn [<post_update_godot_transforms_3d_ $name:lower>](
                config: bevy::prelude::Res<$crate::plugins::core::GodotCustomTransformSyncConfig>,
                mut entities: bevy::prelude::Query<
                    (&$crate::plugins::transforms::Transform3D, &mut $crate::interop::GodotNodeHandle),
                    (
                        bevy::ecs::query::Or<(
                            bevy::ecs::query::Added<$crate::plugins::transforms::Transform3D>,
                            bevy::ecs::query::Changed<$crate::plugins::transforms::Transform3D>
                        )>,
                        $bevy_to_godot_3d,
                    ),
                >,
            ) {
                // Early return if transform syncing is disabled
                if config.sync_mode == $crate::plugins::core::TransformSyncMode::Disabled {
                    return;
                }

                use godot::classes::Node3D;

                for (transform, mut reference) in entities.iter_mut() {
                    if let Some(mut obj) = reference.try_get::<Node3D>() {
                        if obj.get_transform() != *transform.as_godot() {
                            obj.set_transform(*transform.as_godot());
                        }
                    }
                }
            }

            #[$crate::prelude::main_thread_system]
            pub fn [<pre_update_godot_transforms_3d_ $name:lower>](
                config: bevy::prelude::Res<$crate::plugins::core::GodotCustomTransformSyncConfig>,
                mut entities: bevy::prelude::Query<
                    (&mut $crate::plugins::transforms::Transform3D, &mut $crate::interop::GodotNodeHandle),
                    $godot_to_bevy_3d
                >,
            ) {
                // Early return if not using two-way sync
                if config.sync_mode != $crate::plugins::core::TransformSyncMode::TwoWay {
                    return;
                }

                use bevy::ecs::change_detection::DetectChanges;
                use godot::classes::Node3D;

                for (mut transform, mut reference) in entities.iter_mut() {
                    // Skip entities that were changed recently (e.g., by PhysicsUpdate systems)
                    if transform.is_changed() {
                        continue;
                    }

                    if let Some(godot_node) = reference.try_get::<Node3D>() {
                        let godot_transform = godot_node.get_transform();
                        if *transform.as_godot() != godot_transform {
                            *transform.as_godot_mut() = godot_transform;
                        }
                    }
                }
            }
        }
    };
}

/// Helper macro to generate transform sync systems and add them to an App
///
/// This macro both generates the systems and adds them to the provided App
/// with the correct scheduling.
///
/// # Usage
///
/// ```rust
/// use godot_bevy::plugins::transforms::add_transform_sync_systems;
///
/// let mut app = App::new();
/// add_transform_sync_systems! {
///     app,
///     PhysicsBody = Or<(
///         With<CharacterBody3DMarker>,
///         With<RigidBody3DMarker>,
///         With<StaticBody3DMarker>,
///     )>
/// }
///
/// // Or with separate queries for each direction
/// add_transform_sync_systems! {
///     app,
///     Player = bevy_to_godot: With<Player>, godot_to_bevy: With<PlayerInput>
/// }
///
/// // Or with specific dimensions
/// add_transform_sync_systems! {
///     app,
///     Boid2D = 2d: With<Boid>
/// }
/// ```
#[macro_export]
macro_rules! add_transform_sync_systems {
    // Handle same query for both directions (both 2D and 3D)
    ($app:expr, $($name:ident = $query:ty),+ $(,)?) => {
        // Generate the systems
        $crate::transform_sync_systems! {
            $($name = $query),+
        }

        // Add them to the app
        $(
            $crate::add_transform_sync_systems!(@add_to_app $app, $name, both, both);
        )+
    };

    // Handle separate query syntax (both 2D and 3D)
    ($app:expr, $($name:ident = bevy_to_godot: $bevy_to_godot_query:ty, godot_to_bevy: $godot_to_bevy_query:ty),+ $(,)?) => {
        // Generate the systems with separate queries
        $crate::transform_sync_systems! {
            $($name = bevy_to_godot: $bevy_to_godot_query, godot_to_bevy: $godot_to_bevy_query),+
        }

        // Add them to the app
        $(
            $crate::add_transform_sync_systems!(@add_to_app $app, $name, both, both);
        )+
    };

    // Handle 2D and 3D specific queries with same query for both directions
    ($app:expr, $($name:ident = 2d: $query_2d:ty, 3d: $query_3d:ty),+ $(,)?) => {
        // Generate the systems
        $crate::transform_sync_systems! {
            $($name = 2d: $query_2d, 3d: $query_3d),+
        }

        // Add them to the app
        $(
            $crate::add_transform_sync_systems!(@add_to_app $app, $name, $query_2d, $query_3d);
        )+
    };

    // Handle 2D only
    ($app:expr, $($name:ident = 2d: $query_2d:ty),+ $(,)?) => {
        // Generate the systems
        $crate::transform_sync_systems! {
            $($name = 2d: $query_2d),+
        }

        // Add them to the app
        $(
            $crate::add_transform_sync_systems!(@add_to_app $app, $name, $query_2d, none);
        )+
    };

    // Handle 3D only
    ($app:expr, $($name:ident = 3d: $query_3d:ty),+ $(,)?) => {
        // Generate the systems
        $crate::transform_sync_systems! {
            $($name = 3d: $query_3d),+
        }

        // Add them to the app
        $(
            $crate::add_transform_sync_systems!(@add_to_app $app, $name, none, $query_3d);
        )+
    };

    // Handle 2D and 3D specific queries with different queries for each direction
    ($app:expr, $($name:ident = 2d: bevy_to_godot: $bevy_to_godot_2d:ty, godot_to_bevy: $godot_to_bevy_2d:ty, 3d: bevy_to_godot: $bevy_to_godot_3d:ty, godot_to_bevy: $godot_to_bevy_3d:ty),+ $(,)?) => {
        // Generate the systems with separate queries
        $crate::transform_sync_systems! {
            $($name = 2d: bevy_to_godot: $bevy_to_godot_2d, godot_to_bevy: $godot_to_bevy_2d, 3d: bevy_to_godot: $bevy_to_godot_3d, godot_to_bevy: $godot_to_bevy_3d),+
        }

        // Add them to the app
        $(
            $crate::add_transform_sync_systems!(@add_to_app $app, $name, enabled, enabled);
        )+
    };

    // Handle 2D specific with different queries, 3D None
    ($app:expr, $($name:ident = 2d: bevy_to_godot: $bevy_to_godot_2d:ty, godot_to_bevy: $godot_to_bevy_2d:ty, 3d: none),+ $(,)?) => {
        // Generate the systems
        $crate::transform_sync_systems! {
            $($name = 2d: bevy_to_godot: $bevy_to_godot_2d, godot_to_bevy: $godot_to_bevy_2d, 3d: none),+
        }

        // Add them to the app
        $(
            $crate::add_transform_sync_systems!(@add_to_app $app, $name, enabled, none);
        )+
    };

    // Handle 3D specific with different queries, 2D None
    ($app:expr, $($name:ident = 2d: none, 3d: bevy_to_godot: $bevy_to_godot_3d:ty, godot_to_bevy: $godot_to_bevy_3d:ty),+ $(,)?) => {
        // Generate the systems
        $crate::transform_sync_systems! {
            $($name = 2d: none, 3d: bevy_to_godot: $bevy_to_godot_3d, godot_to_bevy: $godot_to_bevy_3d),+
        }

        // Add them to the app
        $(
            $crate::add_transform_sync_systems!(@add_to_app $app, $name, none, enabled);
        )+
    };

    // Internal helper to add systems to app, conditionally based on enabled dimensions
    (@add_to_app $app:expr, $name:ident, both, both) => {
        $crate::paste::paste! {
            $app.add_systems(bevy::app::Last, (
                    [<post_update_godot_transforms_2d_ $name:lower>],
                    [<post_update_godot_transforms_3d_ $name:lower>],
                ))
                .add_systems(bevy::app::PreUpdate, (
                    [<pre_update_godot_transforms_2d_ $name:lower>],
                    [<pre_update_godot_transforms_3d_ $name:lower>],
                ));
        }
    };

    (@add_to_app $app:expr, $name:ident, enabled, enabled) => {
        $crate::paste::paste! {
            $app.add_systems(bevy::app::Last, (
                    [<post_update_godot_transforms_2d_ $name:lower>],
                    [<post_update_godot_transforms_3d_ $name:lower>],
                ))
                .add_systems(bevy::app::PreUpdate, (
                    [<pre_update_godot_transforms_2d_ $name:lower>],
                    [<pre_update_godot_transforms_3d_ $name:lower>],
                ));
        }
    };

    (@add_to_app $app:expr, $name:ident, enabled, none) => {
        $crate::paste::paste! {
            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_2d_ $name:lower>])
                .add_systems(bevy::app::PreUpdate, [<pre_update_godot_transforms_2d_ $name:lower>]);
        }
    };

    (@add_to_app $app:expr, $name:ident, none, enabled) => {
        $crate::paste::paste! {
            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_3d_ $name:lower>])
                .add_systems(bevy::app::PreUpdate, [<pre_update_godot_transforms_3d_ $name:lower>]);
        }
    };

    (@add_to_app $app:expr, $name:ident, $query_2d:ty, none) => {
        $crate::paste::paste! {
            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_2d_ $name:lower>])
                .add_systems(bevy::app::PreUpdate, [<pre_update_godot_transforms_2d_ $name:lower>]);
        }
    };

    (@add_to_app $app:expr, $name:ident, none, $query_3d:ty) => {
        $crate::paste::paste! {
            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_3d_ $name:lower>])
                .add_systems(bevy::app::PreUpdate, [<pre_update_godot_transforms_3d_ $name:lower>]);
        }
    };

    (@add_to_app $app:expr, $name:ident, $query_2d:ty, $query_3d:ty) => {
        $crate::paste::paste! {
            $app.add_systems(bevy::app::Last, (
                    [<post_update_godot_transforms_2d_ $name:lower>],
                    [<post_update_godot_transforms_3d_ $name:lower>],
                ))
                .add_systems(bevy::app::PreUpdate, (
                    [<pre_update_godot_transforms_2d_ $name:lower>],
                    [<pre_update_godot_transforms_3d_ $name:lower>],
                ));
        }
    };
}

// Re-export the macros at the crate level
pub use add_transform_sync_systems;
pub use transform_sync_systems;
