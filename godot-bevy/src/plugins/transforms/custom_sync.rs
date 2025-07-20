/// Macro for generating transform synchronization systems with compile-time queries.
///
/// This macro generates systems that sync transforms between Bevy and Godot for entities
/// matching specific component queries. It automatically handles both 2D and 3D nodes
/// using runtime type detection, similar to the default sync systems.
///
/// # Usage
///
/// ```rust
/// use godot_bevy::add_transform_sync_systems;
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
/// add_transform_sync_systems! {
///     app,
///     UIElements = bevy_to_godot: With<UIElement>,    // ECS → Godot only
///     PhysicsResults = godot_to_bevy: With<PhysicsActor>, // Godot → ECS only
///     Player = With<Player>,                          // Bidirectional
/// }
/// ```
#[macro_export]
macro_rules! add_transform_sync_systems {
    // Main entry point - handles mixed directional sync
    ($app:expr, $($tokens:tt)*) => {
        $crate::add_transform_sync_systems!(@parse_all $app, $($tokens)*);
    };

    // Parse all items recursively
    (@parse_all $app:expr, $name:ident = bevy_to_godot: $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems!(@generate_post_system $app, $name, $query);
        $crate::add_transform_sync_systems!(@parse_all $app, $($rest)*);
    };

    (@parse_all $app:expr, $name:ident = godot_to_bevy: $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems!(@generate_pre_system $app, $name, $query);
        $crate::add_transform_sync_systems!(@parse_all $app, $($rest)*);
    };

    (@parse_all $app:expr, $name:ident = $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems!(@generate_systems $app, $name, $query, $query);
        $crate::add_transform_sync_systems!(@parse_all $app, $($rest)*);
    };

    // Handle last item (without trailing comma)
    (@parse_all $app:expr, $name:ident = bevy_to_godot: $query:ty) => {
        $crate::add_transform_sync_systems!(@generate_post_system $app, $name, $query);
    };

    (@parse_all $app:expr, $name:ident = godot_to_bevy: $query:ty) => {
        $crate::add_transform_sync_systems!(@generate_pre_system $app, $name, $query);
    };

    (@parse_all $app:expr, $name:ident = $query:ty) => {
        $crate::add_transform_sync_systems!(@generate_systems $app, $name, $query, $query);
    };

    // Handle empty case
    (@parse_all $app:expr,) => {};
    (@parse_all $app:expr) => {};

    (@generate_systems $app:expr, $name:ident, $bevy_to_godot_query:ty, $godot_to_bevy_query:ty) => {
        $crate::add_transform_sync_systems!(@generate_post_system $app, $name, $bevy_to_godot_query);
        $crate::add_transform_sync_systems!(@generate_pre_system $app, $name, $godot_to_bevy_query);
    };

    (@generate_post_system $app:expr, $name:ident, $bevy_to_godot_query:ty) => {
        $crate::paste::paste! {
            #[$crate::prelude::main_thread_system]
            pub fn [<post_update_godot_transforms_ $name:lower>](
                change_tick: bevy::ecs::system::SystemChangeTick,
                mut entities: bevy::prelude::Query<
                    (
                        bevy::ecs::change_detection::Ref<bevy::prelude::Transform>,
                        &mut $crate::interop::GodotNodeHandle,
                        &$crate::plugins::transforms::TransformSyncMetadata,
                        bevy::ecs::query::AnyOf<(&$crate::interop::node_markers::Node2DMarker, &$crate::interop::node_markers::Node3DMarker)>,
                    ),
                    (
                        bevy::ecs::query::Changed<bevy::prelude::Transform>,
                        $bevy_to_godot_query,
                    ),
                >,
            ) {
                use $crate::plugins::transforms::{IntoGodotTransform, IntoGodotTransform2D};
                use bevy::ecs::change_detection::DetectChanges;
                use godot::classes::{Node2D, Node3D};

                for (transform_ref, mut reference, metadata, (node2d, node3d)) in entities.iter_mut() {
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

                    // Handle both 2D and 3D nodes in a single system
                    if node2d.is_some() {
                        let mut obj = reference.get::<Node2D>();
                        obj.set_transform(transform_ref.to_godot_transform_2d());
                    } else if node3d.is_some() {
                        let mut obj = reference.get::<Node3D>();
                        obj.set_transform(transform_ref.to_godot_transform());
                    }
                }
            }

            $app.add_systems(bevy::app::Last, [<post_update_godot_transforms_ $name:lower>]);
        }
    };

    (@generate_pre_system $app:expr, $name:ident, $godot_to_bevy_query:ty) => {
        $crate::paste::paste! {
            #[$crate::prelude::main_thread_system]
            pub fn [<pre_update_godot_transforms_ $name:lower>](
                mut entities: bevy::prelude::Query<
                    (
                        &mut bevy::prelude::Transform,
                        &mut $crate::interop::GodotNodeHandle,
                        &mut $crate::plugins::transforms::TransformSyncMetadata,
                        bevy::ecs::query::AnyOf<(&$crate::interop::node_markers::Node2DMarker, &$crate::interop::node_markers::Node3DMarker)>,
                    ),
                    $godot_to_bevy_query
                >,
            ) {
                use $crate::plugins::transforms::IntoBevyTransform;
                use bevy::ecs::change_detection::DetectChanges;
                use godot::classes::{Node2D, Node3D};

                for (mut bevy_transform, mut reference, mut metadata, (node2d, node3d)) in entities.iter_mut() {
                    let new_bevy_transform = if node2d.is_some() {
                        reference
                            .get::<Node2D>()
                            .get_transform()
                            .to_bevy_transform()
                    } else if node3d.is_some() {
                        reference
                            .get::<Node3D>()
                            .get_transform()
                            .to_bevy_transform()
                    } else {
                        panic!("Expected AnyOf to match either a Node2D or a Node3D, is there a bug in bevy?");
                    };

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

            $app.add_systems(bevy::app::PreUpdate, [<pre_update_godot_transforms_ $name:lower>]);
        }
    };

}

/// Helper trait to easily disable auto sync and configure custom systems
pub trait GodotTransformSyncPluginExt {
    /// Disable automatic transform syncing - you must provide your own sync systems via `add_transform_sync_systems` macro
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

// Re-export the macro at the crate level
pub use add_transform_sync_systems;
