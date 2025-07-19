/// Macro for generating 2D transform synchronization systems with compile-time queries.
///
/// This macro generates systems that sync transforms between Bevy and Godot for 2D entities
/// matching specific component queries. It uses the new bevy Transform approach with
/// TransformSyncMetadata for proper change detection.
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
/// struct Player;
/// #[derive(Component)]
/// struct UIElement;
/// #[derive(Component)]
/// struct PhysicsActor;
///
/// let mut app = App::new();
/// // Mixed directional sync in a single call
/// add_transform_sync_systems_2d! {
///     app,
///     UIElements = bevy_to_godot: With<UIElement>,    // ECS → Godot only
///     PhysicsResults = godot_to_bevy: With<PhysicsActor>, // Godot → ECS only
///     Player = With<Player>,                          // Bidirectional
/// }
/// ```
#[macro_export]
macro_rules! add_transform_sync_systems_2d {
    // Main entry point - handles mixed directional sync
    ($app:expr, $($tokens:tt)*) => {
        $crate::add_transform_sync_systems_2d!(@parse_all $app, $($tokens)*);
    };

    // Parse all items recursively
    (@parse_all $app:expr, $name:ident = bevy_to_godot: $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems_2d!(@generate_post_system $app, $name, $query);
        $crate::add_transform_sync_systems_2d!(@parse_all $app, $($rest)*);
    };

    (@parse_all $app:expr, $name:ident = godot_to_bevy: $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems_2d!(@generate_pre_system $app, $name, $query);
        $crate::add_transform_sync_systems_2d!(@parse_all $app, $($rest)*);
    };

    (@parse_all $app:expr, $name:ident = $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems_2d!(@generate_systems $app, $name, $query, $query);
        $crate::add_transform_sync_systems_2d!(@parse_all $app, $($rest)*);
    };

    // Handle last item (without trailing comma)
    (@parse_all $app:expr, $name:ident = bevy_to_godot: $query:ty) => {
        $crate::add_transform_sync_systems_2d!(@generate_post_system $app, $name, $query);
    };

    (@parse_all $app:expr, $name:ident = godot_to_bevy: $query:ty) => {
        $crate::add_transform_sync_systems_2d!(@generate_pre_system $app, $name, $query);
    };

    (@parse_all $app:expr, $name:ident = $query:ty) => {
        $crate::add_transform_sync_systems_2d!(@generate_systems $app, $name, $query, $query);
    };

    // Handle empty case
    (@parse_all $app:expr,) => {};
    (@parse_all $app:expr) => {};

    (@generate_systems $app:expr, $name:ident, $bevy_to_godot_query:ty, $godot_to_bevy_query:ty) => {
        $crate::add_transform_sync_systems_2d!(@generate_post_system $app, $name, $bevy_to_godot_query);
        $crate::add_transform_sync_systems_2d!(@generate_pre_system $app, $name, $godot_to_bevy_query);
    };

    (@generate_post_system $app:expr, $name:ident, $bevy_to_godot_query:ty) => {
        $crate::paste::paste! {
            #[$crate::prelude::main_thread_system]
            pub fn [<post_update_godot_transforms_2d_ $name:lower>](
                change_tick: bevy::ecs::system::SystemChangeTick,
                mut entities: bevy::prelude::Query<
                    (
                        bevy::ecs::change_detection::Ref<bevy::prelude::Transform>,
                        &mut $crate::interop::GodotNodeHandle,
                        &$crate::plugins::transforms::TransformSyncMetadata,
                        &$crate::interop::node_markers::Node2DMarker,
                    ),
                    (
                        bevy::ecs::query::Changed<bevy::prelude::Transform>,
                        $bevy_to_godot_query,
                    ),
                >,
            ) {
                use $crate::plugins::transforms::IntoGodotTransform2D;
                use bevy::ecs::change_detection::DetectChanges;
                use godot::classes::Node2D;

                for (transform_ref, mut reference, metadata, _) in entities.iter_mut() {
                    // Check if we have sync information for this entity
                    if let Some(sync_tick) = metadata.last_sync_tick {
                        if !transform_ref
                            .last_changed()
                            .is_newer_than(sync_tick, change_tick.this_run())
                        {
                            // This change was from our Godot sync, skip it
                            continue;
                        }
                    }

                    let mut obj = reference.get::<Node2D>();
                    obj.set_transform(transform_ref.to_godot_transform_2d());
                }
            }

            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_2d_ $name:lower>]);
        }
    };

    (@generate_pre_system $app:expr, $name:ident, $godot_to_bevy_query:ty) => {
        $crate::paste::paste! {
            #[$crate::prelude::main_thread_system]
            pub fn [<pre_update_godot_transforms_2d_ $name:lower>](
                mut entities: bevy::prelude::Query<
                    (
                        &mut bevy::prelude::Transform,
                        &mut $crate::interop::GodotNodeHandle,
                        &mut $crate::plugins::transforms::TransformSyncMetadata,
                        &$crate::interop::node_markers::Node2DMarker,
                    ),
                    $godot_to_bevy_query
                >,
            ) {
                use $crate::plugins::transforms::IntoBevyTransform;
                use bevy::ecs::change_detection::DetectChanges;
                use godot::classes::Node2D;

                for (mut bevy_transform, mut reference, mut metadata, _) in entities.iter_mut() {
                    let new_bevy_transform = reference
                        .get::<Node2D>()
                        .get_transform()
                        .to_bevy_transform();

                    // Only write if actually different - avoids triggering change detection
                    if *bevy_transform != new_bevy_transform {
                        *bevy_transform = new_bevy_transform;

                        // Store the last changed tick for this entity, this helps us in the post_ operations
                        // to disambiguate our change (syncing from Godot to Bevy above) versus changes that
                        // *user* systems do this frame. It's only the latter that we may need to copy back to
                        // Godot
                        metadata.last_sync_tick = Some(bevy_transform.last_changed());
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
/// matching specific component queries. It uses the new bevy Transform approach with
/// TransformSyncMetadata for proper change detection.
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
/// #[derive(Component)]
/// struct VisualEffect;
/// #[derive(Component)]
/// struct PhysicsActor;
///
/// let mut app = App::new();
/// // Mixed directional sync in a single call
/// add_transform_sync_systems_3d! {
///     app,
///     VisualEffects = bevy_to_godot: With<VisualEffect>,  // ECS → Godot only
///     PhysicsResults = godot_to_bevy: With<PhysicsActor>, // Godot → ECS only
///     Player = With<Player>,                              // Bidirectional
/// }
/// ```
#[macro_export]
macro_rules! add_transform_sync_systems_3d {
    // Main entry point - handles mixed directional sync
    ($app:expr, $($tokens:tt)*) => {
        $crate::add_transform_sync_systems_3d!(@parse_all $app, $($tokens)*);
    };

    // Parse all items recursively
    (@parse_all $app:expr, $name:ident = bevy_to_godot: $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems_3d!(@generate_post_system $app, $name, $query);
        $crate::add_transform_sync_systems_3d!(@parse_all $app, $($rest)*);
    };

    (@parse_all $app:expr, $name:ident = godot_to_bevy: $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems_3d!(@generate_pre_system $app, $name, $query);
        $crate::add_transform_sync_systems_3d!(@parse_all $app, $($rest)*);
    };

    (@parse_all $app:expr, $name:ident = $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems_3d!(@generate_systems $app, $name, $query, $query);
        $crate::add_transform_sync_systems_3d!(@parse_all $app, $($rest)*);
    };

    // Handle last item (without trailing comma)
    (@parse_all $app:expr, $name:ident = bevy_to_godot: $query:ty) => {
        $crate::add_transform_sync_systems_3d!(@generate_post_system $app, $name, $query);
    };

    (@parse_all $app:expr, $name:ident = godot_to_bevy: $query:ty) => {
        $crate::add_transform_sync_systems_3d!(@generate_pre_system $app, $name, $query);
    };

    (@parse_all $app:expr, $name:ident = $query:ty) => {
        $crate::add_transform_sync_systems_3d!(@generate_systems $app, $name, $query, $query);
    };

    // Handle empty case
    (@parse_all $app:expr,) => {};
    (@parse_all $app:expr) => {};

    (@generate_systems $app:expr, $name:ident, $bevy_to_godot_query:ty, $godot_to_bevy_query:ty) => {
        $crate::add_transform_sync_systems_3d!(@generate_post_system $app, $name, $bevy_to_godot_query);
        $crate::add_transform_sync_systems_3d!(@generate_pre_system $app, $name, $godot_to_bevy_query);
    };

    (@generate_post_system $app:expr, $name:ident, $bevy_to_godot_query:ty) => {
        $crate::paste::paste! {
            #[$crate::prelude::main_thread_system]
            pub fn [<post_update_godot_transforms_3d_ $name:lower>](
                change_tick: bevy::ecs::system::SystemChangeTick,
                mut entities: bevy::prelude::Query<
                    (
                        bevy::ecs::change_detection::Ref<bevy::prelude::Transform>,
                        &mut $crate::interop::GodotNodeHandle,
                        &$crate::plugins::transforms::TransformSyncMetadata,
                        &$crate::interop::node_markers::Node3DMarker,
                    ),
                    (
                        bevy::ecs::query::Changed<bevy::prelude::Transform>,
                        $bevy_to_godot_query,
                    ),
                >,
            ) {
                use $crate::plugins::transforms::IntoGodotTransform;
                use bevy::ecs::change_detection::DetectChanges;
                use godot::classes::Node3D;

                for (transform_ref, mut reference, metadata, _) in entities.iter_mut() {
                    // Check if we have sync information for this entity
                    if let Some(sync_tick) = metadata.last_sync_tick {
                        if !transform_ref
                            .last_changed()
                            .is_newer_than(sync_tick, change_tick.this_run())
                        {
                            // This change was from our Godot sync, skip it
                            continue;
                        }
                    }

                    let mut obj = reference.get::<Node3D>();
                    obj.set_transform(transform_ref.to_godot_transform());
                }
            }

            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_3d_ $name:lower>]);
        }
    };

    (@generate_pre_system $app:expr, $name:ident, $godot_to_bevy_query:ty) => {
        $crate::paste::paste! {
            #[$crate::prelude::main_thread_system]
            pub fn [<pre_update_godot_transforms_3d_ $name:lower>](
                mut entities: bevy::prelude::Query<
                    (
                        &mut bevy::prelude::Transform,
                        &mut $crate::interop::GodotNodeHandle,
                        &mut $crate::plugins::transforms::TransformSyncMetadata,
                        &$crate::interop::node_markers::Node3DMarker,
                    ),
                    $godot_to_bevy_query
                >,
            ) {
                use $crate::plugins::transforms::IntoBevyTransform;
                use bevy::ecs::change_detection::DetectChanges;
                use godot::classes::Node3D;

                for (mut bevy_transform, mut reference, mut metadata, _) in entities.iter_mut() {
                    let new_bevy_transform = reference
                        .get::<Node3D>()
                        .get_transform()
                        .to_bevy_transform();

                    // Only write if actually different - avoids triggering change detection
                    if *bevy_transform != new_bevy_transform {
                        *bevy_transform = new_bevy_transform;

                        // Store the last changed tick for this entity, this helps us in the post_ operations
                        // to disambiguate our change (syncing from Godot to Bevy above) versus changes that
                        // *user* systems do this frame. It's only the latter that we may need to copy back to
                        // Godot
                        metadata.last_sync_tick = Some(bevy_transform.last_changed());
                    }
                }
            }

            $app.add_systems(bevy::app::PreUpdate, [<pre_update_godot_transforms_3d_ $name:lower>]);
        }
    };

}

/// Helper trait to easily disable auto sync and configure custom systems
pub trait GodotTransformSyncPluginExt {
    /// Disable automatic transform syncing - you must provide your own sync systems
    fn without_auto_sync(self) -> Self;

    /// Configure the sync mode while keeping auto sync enabled
    fn with_sync_mode(self, mode: crate::plugins::core::TransformSyncMode) -> Self;
}

impl GodotTransformSyncPluginExt for crate::plugins::transforms::GodotTransformSyncPlugin {
    fn without_auto_sync(mut self) -> Self {
        self.auto_sync = false;
        self
    }

    fn with_sync_mode(mut self, mode: crate::plugins::core::TransformSyncMode) -> Self {
        self.sync_mode = mode;
        self
    }
}

// Re-export the new macros at the crate level
pub use add_transform_sync_systems_2d;
pub use add_transform_sync_systems_3d;
