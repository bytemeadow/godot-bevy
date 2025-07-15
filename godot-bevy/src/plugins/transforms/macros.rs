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
/// // Generate systems for physics bodies only
/// transform_sync_systems! {
///     PhysicsBody3D = Or<(
///         With<CharacterBody3DMarker>,
///         With<RigidBody3DMarker>,
///         With<StaticBody3DMarker>,
///     )>
/// }
///
/// // Generate systems for custom 2D and 3D filters
/// transform_sync_systems! {
///     Custom2D = With<MyCustom2DMarker>,
///     Custom3D = Or<(With<PhysicsMarker>, With<SpecialMarker>)>
/// }
/// ```
///
/// The macro generates four systems:
/// - `pre_update_godot_transforms_2d_{name}` - Reads transforms from Godot to Bevy (2D)
/// - `post_update_godot_transforms_2d_{name}` - Writes transforms from Bevy to Godot (2D)
/// - `pre_update_godot_transforms_3d_{name}` - Reads transforms from Godot to Bevy (3D)
/// - `post_update_godot_transforms_3d_{name}` - Writes transforms from Bevy to Godot (3D)
///
/// You can then add these systems to your Bevy App:
///
/// ```rust
/// app.add_systems(PreUpdate, pre_update_godot_transforms_3d_physics_body)
///    .add_systems(Last, post_update_godot_transforms_3d_physics_body);
/// ```
#[macro_export]
macro_rules! transform_sync_systems {
    // Handle 3D only case
    ($name:ident = $query:ty) => {
        $crate::transform_sync_systems! {
            $name: 3d = $query
        }
    };

    // Handle explicit 2D/3D specification
    ($($name:ident: $dim:tt = $query:ty),+ $(,)?) => {
        $(
            $crate::transform_sync_systems!(@generate_systems $name, $dim, $query);
        )+
    };

    // Internal macro for generating the actual systems
    (@generate_systems $name:ident, 2d, $query:ty) => {
        paste::paste! {
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
                        $query,
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
                    $query
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

    (@generate_systems $name:ident, 3d, $query:ty) => {
        paste::paste! {
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
                        $query,
                    ),
                >,
            ) {
                // Early return if transform syncing is disabled
                if config.sync_mode == $crate::plugins::core::TransformSyncMode::Disabled {
                    return;
                }

                use godot::classes::Node3D;
                use $crate::plugins::transforms::conversions::IntoGodotTransform;

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
                    $query
                >,
            ) {
                // Early return if not using two-way sync
                if config.sync_mode != $crate::plugins::core::TransformSyncMode::TwoWay {
                    return;
                }

                use bevy::ecs::change_detection::DetectChanges;
                use godot::classes::Node3D;
                use $crate::plugins::transforms::conversions::IntoGodotTransform;

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
///     PhysicsBody3D = Or<(
///         With<CharacterBody3DMarker>,
///         With<RigidBody3DMarker>,
///         With<StaticBody3DMarker>,
///     )>
/// }
/// ```
#[macro_export]
macro_rules! add_transform_sync_systems {
    ($app:expr, $($name:ident $(: $dim:tt)? = $query:ty),+ $(,)?) => {
        // Generate the systems
        $crate::transform_sync_systems! {
            $($name $(: $dim)? = $query),+
        }

        // Add them to the app
        $(
            $crate::add_transform_sync_systems!(@add_to_app $app, $name, $($dim)?);
        )+
    };

    (@add_to_app $app:expr, $name:ident,) => {
        $crate::add_transform_sync_systems!(@add_to_app $app, $name, 3d);
    };

    (@add_to_app $app:expr, $name:ident, 2d) => {
        paste::paste! {
            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_2d_ $name:lower>])
                .add_systems(bevy::app::PreUpdate, [<pre_update_godot_transforms_2d_ $name:lower>]);
        }
    };

    (@add_to_app $app:expr, $name:ident, 3d) => {
        paste::paste! {
            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_3d_ $name:lower>])
                .add_systems(bevy::app::PreUpdate, [<pre_update_godot_transforms_3d_ $name:lower>]);
        }
    };
}

// Re-export the macros at the crate level
pub use add_transform_sync_systems;
pub use transform_sync_systems;
