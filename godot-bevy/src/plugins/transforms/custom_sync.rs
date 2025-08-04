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
            #[tracing::instrument]
            #[$crate::prelude::main_thread_system]
            pub fn [<post_update_godot_transforms_ $name:lower>](
                change_tick: bevy::ecs::system::SystemChangeTick,
                entities: bevy::prelude::Query<
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
                use godot::classes::{Engine, Node2D, Node3D, Object, SceneTree};
                use godot::global::godot_print;
                use godot::prelude::{Array, Dictionary, Gd, ToGodot};

                // Try to get the BevyAppSingleton autoload for bulk optimization
                let engine = Engine::singleton();
                if let Some(scene_tree) = engine
                    .get_main_loop()
                    .and_then(|main_loop| main_loop.try_cast::<SceneTree>().ok())
                {
                    if let Some(root) = scene_tree.get_root() {
                        if let Some(bevy_app) = root.get_node_or_null("BevyAppSingleton") {
                            // Check if this BevyApp has the raw array methods (prefer these over bulk Dictionary methods)
                            if bevy_app.has_method("bulk_update_transforms_3d") {
                                // Use bulk optimization path
                                [<post_update_godot_transforms_ $name:lower _bulk>](
                                    change_tick,
                                    entities,
                                    bevy_app.upcast::<Object>(),
                                );
                                return;
                            }
                        }
                    }
                }

                // Fallback to individual FFI calls
                [<post_update_godot_transforms_ $name:lower _individual>](change_tick, entities);
            }

            fn [<post_update_godot_transforms_ $name:lower _bulk>](
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
                mut batch_singleton: godot::prelude::Gd<godot::classes::Object>,
            ) {
                use $crate::plugins::transforms::{IntoGodotTransform, IntoGodotTransform2D};
                use bevy::ecs::change_detection::DetectChanges;
                use godot::global::godot_print;
                use godot::prelude::ToGodot;

                let _span = tracing::info_span!("bulk_data_preparation_optimized", system = stringify!($name)).entered();

                // Pre-allocate vectors with estimated capacity to avoid reallocations
                let entity_count = entities.iter().count();
                let mut instance_ids_3d = Vec::with_capacity(entity_count);
                let mut positions_3d = Vec::with_capacity(entity_count);
                let mut rotations_3d = Vec::with_capacity(entity_count);
                let mut scales_3d = Vec::with_capacity(entity_count);

                let mut instance_ids_2d = Vec::with_capacity(entity_count);
                let mut positions_2d = Vec::with_capacity(entity_count);
                let mut rotations_2d = Vec::with_capacity(entity_count);
                let mut scales_2d = Vec::with_capacity(entity_count);

                // Collect raw transform data (no FFI allocations)
                let _collect_span = tracing::info_span!("collect_raw_arrays", system = stringify!($name)).entered();
                for (transform_ref, reference, metadata, (node2d, node3d)) in entities.iter_mut() {
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

                    let instance_id = reference.instance_id();

                    if node2d.is_some() {
                        let transform_2d = transform_ref.to_godot_transform_2d();
                        instance_ids_2d.push(instance_id.to_i64());
                        positions_2d.push(godot::prelude::Vector2::new(transform_2d.origin.x, transform_2d.origin.y));
                        rotations_2d.push(transform_2d.rotation());
                        scales_2d.push(godot::prelude::Vector2::new(transform_2d.scale().x, transform_2d.scale().y));
                    } else if node3d.is_some() {
                        // Use Bevy transform components directly (avoid complex conversions)
                        instance_ids_3d.push(instance_id.to_i64());
                        positions_3d.push(godot::prelude::Vector3::new(
                            transform_ref.translation.x,
                            transform_ref.translation.y,
                            transform_ref.translation.z
                        ));

                        // Convert Bevy rotation (quaternion) to Euler angles
                        let (x, y, z) = transform_ref.rotation.to_euler(bevy::math::EulerRot::XYZ);
                        rotations_3d.push(godot::prelude::Vector3::new(x, y, z));

                        scales_3d.push(godot::prelude::Vector3::new(
                            transform_ref.scale.x,
                            transform_ref.scale.y,
                            transform_ref.scale.z
                        ));
                    }
                }
                drop(_collect_span);

                // Convert to Godot packed arrays (much more efficient than Dictionary arrays)
                let _convert_span = tracing::info_span!("convert_to_packed_arrays", system = stringify!($name)).entered();
                let has_3d_updates = !instance_ids_3d.is_empty();
                let has_2d_updates = !instance_ids_2d.is_empty();
                drop(_convert_span);

                // End data preparation phase
                drop(_span);

                // Make raw array FFI calls if we have updates
                let total_updates = instance_ids_3d.len() + instance_ids_2d.len();
                if total_updates > 0 {
                    static mut BATCH_LOG_COUNTER: u32 = 0;
                    unsafe {
                        BATCH_LOG_COUNTER += 1;
                    }

                    let _ffi_calls_span = tracing::info_span!("raw_array_ffi_calls", total_entities = total_updates, system = stringify!($name)).entered();

                    if has_3d_updates {
                        let _span = tracing::info_span!("raw_ffi_call_3d", entities = instance_ids_3d.len(), system = stringify!($name)).entered();

                        // Convert to packed arrays
                        let instance_ids_packed = godot::prelude::PackedInt64Array::from(instance_ids_3d.as_slice());
                        let positions_packed = godot::prelude::PackedVector3Array::from(positions_3d.as_slice());
                        let rotations_packed = godot::prelude::PackedVector3Array::from(rotations_3d.as_slice());
                        let scales_packed = godot::prelude::PackedVector3Array::from(scales_3d.as_slice());

                        batch_singleton.call("bulk_update_transforms_3d", &[
                            instance_ids_packed.to_variant(),
                            positions_packed.to_variant(),
                            rotations_packed.to_variant(),
                            scales_packed.to_variant()
                        ]);
                    }
                    if has_2d_updates {
                        let _span = tracing::info_span!("raw_ffi_call_2d", entities = instance_ids_2d.len(), system = stringify!($name)).entered();

                        // Convert to packed arrays
                        let instance_ids_packed = godot::prelude::PackedInt64Array::from(instance_ids_2d.as_slice());
                        let positions_packed = godot::prelude::PackedVector2Array::from(positions_2d.as_slice());
                        let rotations_packed = godot::prelude::PackedFloat32Array::from(rotations_2d.as_slice());
                        let scales_packed = godot::prelude::PackedVector2Array::from(scales_2d.as_slice());

                        batch_singleton.call("bulk_update_transforms_2d", &[
                            instance_ids_packed.to_variant(),
                            positions_packed.to_variant(),
                            rotations_packed.to_variant(),
                            scales_packed.to_variant()
                        ]);
                    }
                }
            }

            fn [<post_update_godot_transforms_ $name:lower _individual>](
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

                // Original individual FFI approach
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
                        let _span = tracing::info_span!("individual_ffi_call_2d", system = stringify!($name)).entered();
                        let mut obj = reference.get::<Node2D>();
                        obj.set_transform(transform_ref.to_godot_transform_2d());
                    } else if node3d.is_some() {
                        let _span = tracing::info_span!("individual_ffi_call_3d", system = stringify!($name)).entered();
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
            #[tracing::instrument]
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
    fn with_sync_mode(self, mode: crate::plugins::transforms::TransformSyncMode) -> Self;
}

impl GodotTransformSyncPluginExt for crate::plugins::transforms::GodotTransformSyncPlugin {
    fn without_auto_sync(mut self) -> Self {
        self.auto_sync = false;
        self
    }

    fn with_sync_mode(mut self, mode: crate::plugins::transforms::TransformSyncMode) -> Self {
        self.sync_mode = mode;
        self
    }
}

// Re-export the macro at the crate level
pub use add_transform_sync_systems;
