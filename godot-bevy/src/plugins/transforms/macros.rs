/// Macro for generating 2D transform synchronization systems with compile-time queries.
///
/// This macro generates systems that sync transforms between Bevy and Godot for 2D entities
/// matching specific component queries. Much simpler than the combined 2D/3D macro.
///
/// # Usage
///
/// ```rust
/// use godot_bevy::plugins::transforms::add_transform_sync_systems_2d;
/// use bevy::ecs::query::With;
/// use bevy::ecs::component::Component;
/// use bevy::prelude::*;
///
/// #[derive(Component)]
/// struct Boid;
///
/// let mut app = App::new();
/// add_transform_sync_systems_2d! {
///     app,
///     Boid = With<Boid>
/// }
/// ```
#[macro_export]
macro_rules! add_transform_sync_systems_2d {
    ($app:expr, $($name:ident = $query:ty),+ $(,)?) => {
        $(
            $crate::add_transform_sync_systems_2d!(@generate_systems $app, $name, $query, $query);
        )+
    };

    ($app:expr, $($name:ident = bevy_to_godot: $bevy_to_godot_query:ty, godot_to_bevy: $godot_to_bevy_query:ty),+ $(,)?) => {
        $(
            $crate::add_transform_sync_systems_2d!(@generate_systems $app, $name, $bevy_to_godot_query, $godot_to_bevy_query);
        )+
    };

    ($app:expr, $($name:ident = bevy_to_godot: $bevy_to_godot_query:ty),+ $(,)?) => {
        $(
            $crate::add_transform_sync_systems_2d!(@generate_post_system $app, $name, $bevy_to_godot_query);
        )+
    };

    ($app:expr, $($name:ident = godot_to_bevy: $godot_to_bevy_query:ty),+ $(,)?) => {
        $(
            $crate::add_transform_sync_systems_2d!(@generate_pre_system $app, $name, $godot_to_bevy_query);
        )+
    };

    (@generate_systems $app:expr, $name:ident, $bevy_to_godot_query:ty, $godot_to_bevy_query:ty) => {
        $crate::add_transform_sync_systems_2d!(@generate_post_system $app, $name, $bevy_to_godot_query);
        $crate::add_transform_sync_systems_2d!(@generate_pre_system $app, $name, $godot_to_bevy_query);
    };

    (@generate_post_system $app:expr, $name:ident, $bevy_to_godot_query:ty) => {
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
                        $bevy_to_godot_query,
                    ),
                >,
            ) {
                // Early return if transform syncing is disabled
                if config.sync_mode == $crate::plugins::core::TransformSyncMode::Disabled {
                    return;
                }

                use godot::builtin::Transform2D as GodotTransform2D;
                use godot::classes::Node2D;

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

            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_2d_ $name:lower>]);
        }
    };

    (@generate_pre_system $app:expr, $name:ident, $godot_to_bevy_query:ty) => {
        $crate::paste::paste! {
            #[$crate::prelude::main_thread_system]
            pub fn [<pre_update_godot_transforms_2d_ $name:lower>](
                config: bevy::prelude::Res<$crate::plugins::core::GodotCustomTransformSyncConfig>,
                mut entities: bevy::prelude::Query<
                    (&mut $crate::plugins::transforms::Transform2D, &mut $crate::interop::GodotNodeHandle),
                    $godot_to_bevy_query
                >,
            ) {
                // Early return if not using two-way sync
                if config.sync_mode != $crate::plugins::core::TransformSyncMode::TwoWay {
                    return;
                }

                use bevy::ecs::change_detection::DetectChanges;
                use godot::builtin::Transform2D as GodotTransform2D;
                use godot::classes::Node2D;

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

            $app.add_systems(bevy::app::PreUpdate, [<pre_update_godot_transforms_2d_ $name:lower>]);
        }
    };
}

/// Macro for generating 3D transform synchronization systems with compile-time queries.
///
/// This macro generates systems that sync transforms between Bevy and Godot for 3D entities
/// matching specific component queries. Much simpler than the combined 2D/3D macro.
///
/// # Usage
///
/// ```rust
/// use godot_bevy::plugins::transforms::add_transform_sync_systems_3d;
/// use bevy::ecs::query::With;
/// use bevy::ecs::component::Component;
/// use bevy::prelude::*;
///
/// #[derive(Component)]
/// struct Player;
///
/// let mut app = App::new();
/// add_transform_sync_systems_3d! {
///     app,
///     Player = With<Player>
/// }
/// ```
#[macro_export]
macro_rules! add_transform_sync_systems_3d {
    ($app:expr, $($name:ident = $query:ty),+ $(,)?) => {
        $(
            $crate::add_transform_sync_systems_3d!(@generate_systems $app, $name, $query, $query);
        )+
    };

    ($app:expr, $($name:ident = bevy_to_godot: $bevy_to_godot_query:ty, godot_to_bevy: $godot_to_bevy_query:ty),+ $(,)?) => {
        $(
            $crate::add_transform_sync_systems_3d!(@generate_systems $app, $name, $bevy_to_godot_query, $godot_to_bevy_query);
        )+
    };

    ($app:expr, $($name:ident = bevy_to_godot: $bevy_to_godot_query:ty),+ $(,)?) => {
        $(
            $crate::add_transform_sync_systems_3d!(@generate_post_system $app, $name, $bevy_to_godot_query);
        )+
    };

    ($app:expr, $($name:ident = godot_to_bevy: $godot_to_bevy_query:ty),+ $(,)?) => {
        $(
            $crate::add_transform_sync_systems_3d!(@generate_pre_system $app, $name, $godot_to_bevy_query);
        )+
    };

    (@generate_systems $app:expr, $name:ident, $bevy_to_godot_query:ty, $godot_to_bevy_query:ty) => {
        $crate::add_transform_sync_systems_3d!(@generate_post_system $app, $name, $bevy_to_godot_query);
        $crate::add_transform_sync_systems_3d!(@generate_pre_system $app, $name, $godot_to_bevy_query);
    };

    (@generate_post_system $app:expr, $name:ident, $bevy_to_godot_query:ty) => {
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
                        $bevy_to_godot_query,
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

            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_3d_ $name:lower>]);
        }
    };

    (@generate_pre_system $app:expr, $name:ident, $godot_to_bevy_query:ty) => {
        $crate::paste::paste! {
            #[$crate::prelude::main_thread_system]
            pub fn [<pre_update_godot_transforms_3d_ $name:lower>](
                config: bevy::prelude::Res<$crate::plugins::core::GodotCustomTransformSyncConfig>,
                mut entities: bevy::prelude::Query<
                    (&mut $crate::plugins::transforms::Transform3D, &mut $crate::interop::GodotNodeHandle),
                    $godot_to_bevy_query
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

            $app.add_systems(bevy::app::PreUpdate, [<pre_update_godot_transforms_3d_ $name:lower>]);
        }
    };
}

// Re-export the new macros at the crate level
pub use add_transform_sync_systems_2d;
pub use add_transform_sync_systems_3d;