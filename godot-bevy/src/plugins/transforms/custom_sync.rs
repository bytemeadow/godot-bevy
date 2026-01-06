/// Macro for generating transform synchronization systems with compile-time queries.
///
/// This macro generates systems that sync transforms between Bevy and Godot for entities
/// matching specific component queries. It automatically handles both 2D and 3D nodes
/// using runtime type detection, similar to the default sync systems.
///
/// # Usage
///
/// ```ignore
/// use godot_bevy::add_transform_sync_systems;
/// use bevy_ecs::query::With;
/// use bevy_ecs::component::Component;
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
            #[cfg(debug_assertions)]
            #[tracing::instrument]
            pub fn [<post_update_godot_transforms_ $name:lower>](
                change_tick: $crate::bevy_ecs::system::SystemChangeTick,
                entities: $crate::bevy_ecs::system::Query<
                    (
                        $crate::bevy_ecs::change_detection::Ref<$crate::bevy_transform::components::Transform>,
                        &$crate::interop::GodotNodeHandle,
                        &$crate::plugins::transforms::TransformSyncMetadata,
                        $crate::bevy_ecs::query::AnyOf<(&$crate::interop::node_markers::Node2DMarker, &$crate::interop::node_markers::Node3DMarker)>,
                    ),
                    (
                        $crate::bevy_ecs::query::Changed<$crate::bevy_transform::components::Transform>,
                        $bevy_to_godot_query,
                    ),
                >,
                mut godot: $crate::interop::GodotAccess,
                bulk_ops_cache: $crate::bevy_ecs::system::NonSendMut<$crate::interop::BulkOperationsCache>,
            ) {
                if let Some(bulk_ops_node) = bulk_ops_cache.get() {
                    [<post_update_godot_transforms_ $name:lower _bulk>](
                        change_tick,
                        entities,
                        bulk_ops_node,
                    );
                    return;
                }
                [<post_update_godot_transforms_ $name:lower _individual>](change_tick, entities, &mut godot);
            }

            #[cfg(not(debug_assertions))]
            #[tracing::instrument]
            pub fn [<post_update_godot_transforms_ $name:lower>](
                change_tick: $crate::bevy_ecs::system::SystemChangeTick,
                entities: $crate::bevy_ecs::system::Query<
                    (
                        $crate::bevy_ecs::change_detection::Ref<$crate::bevy_transform::components::Transform>,
                        &$crate::interop::GodotNodeHandle,
                        &$crate::plugins::transforms::TransformSyncMetadata,
                        $crate::bevy_ecs::query::AnyOf<(&$crate::interop::node_markers::Node2DMarker, &$crate::interop::node_markers::Node3DMarker)>,
                    ),
                    (
                        $crate::bevy_ecs::query::Changed<$crate::bevy_transform::components::Transform>,
                        $bevy_to_godot_query,
                    ),
                >,
                mut godot: $crate::interop::GodotAccess,
            ) {
                [<post_update_godot_transforms_ $name:lower _individual>](change_tick, entities, &mut godot);
            }

            #[cfg(debug_assertions)]
            fn [<post_update_godot_transforms_ $name:lower _bulk>](
                change_tick: $crate::bevy_ecs::system::SystemChangeTick,
                mut entities: $crate::bevy_ecs::system::Query<
                    (
                        $crate::bevy_ecs::change_detection::Ref<$crate::bevy_transform::components::Transform>,
                        &$crate::interop::GodotNodeHandle,
                        &$crate::plugins::transforms::TransformSyncMetadata,
                        $crate::bevy_ecs::query::AnyOf<(&$crate::interop::node_markers::Node2DMarker, &$crate::interop::node_markers::Node3DMarker)>,
                    ),
                    (
                        $crate::bevy_ecs::query::Changed<$crate::bevy_transform::components::Transform>,
                        $bevy_to_godot_query,
                    ),
                >,
                mut batch_singleton: godot::prelude::Gd<godot::classes::Object>,
            ) {
                use $crate::plugins::transforms::{IntoGodotTransform, IntoGodotTransform2D};
                use $crate::bevy_ecs::change_detection::DetectChanges;
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

                        rotations_3d.push(godot::prelude::Vector4 {
                            x: transform_ref.rotation.x,
                            y: transform_ref.rotation.y,
                            z: transform_ref.rotation.z,
                            w: transform_ref.rotation.w,
                        });

                        scales_3d.push(godot::prelude::Vector3::new(
                            transform_ref.scale.x,
                            transform_ref.scale.y,
                            transform_ref.scale.z
                        ));
                    }
                }
                drop(_collect_span);

                // Convert to Godot packed arrays (much more efficient than VarDictionary arrays)
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
                        let rotations_packed = godot::prelude::PackedVector4Array::from(rotations_3d.as_slice());
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
                change_tick: $crate::bevy_ecs::system::SystemChangeTick,
                mut entities: $crate::bevy_ecs::system::Query<
                    (
                        $crate::bevy_ecs::change_detection::Ref<$crate::bevy_transform::components::Transform>,
                        &$crate::interop::GodotNodeHandle,
                        &$crate::plugins::transforms::TransformSyncMetadata,
                        $crate::bevy_ecs::query::AnyOf<(&$crate::interop::node_markers::Node2DMarker, &$crate::interop::node_markers::Node3DMarker)>,
                    ),
                    (
                        $crate::bevy_ecs::query::Changed<$crate::bevy_transform::components::Transform>,
                        $bevy_to_godot_query,
                    ),
                >,
                godot: &mut $crate::interop::GodotAccess,
            ) {
                use $crate::plugins::transforms::{IntoGodotTransform, IntoGodotTransform2D};
                use $crate::bevy_ecs::change_detection::DetectChanges;
                use godot::classes::{Node2D, Node3D};

                // Original individual FFI approach
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

                    // Handle both 2D and 3D nodes in a single system
                    if node2d.is_some() {
                        let _span = tracing::info_span!("individual_ffi_call_2d", system = stringify!($name)).entered();
                        let mut obj = godot.get::<Node2D>(*reference);
                        obj.set_transform(transform_ref.to_godot_transform_2d());
                    } else if node3d.is_some() {
                        let _span = tracing::info_span!("individual_ffi_call_3d", system = stringify!($name)).entered();
                        let mut obj = godot.get::<Node3D>(*reference);
                        obj.set_transform(transform_ref.to_godot_transform());
                    }
                }
            }

            $app.add_systems($crate::bevy_app::Last, [<post_update_godot_transforms_ $name:lower>]);
        }
    };

    (@generate_pre_system $app:expr, $name:ident, $godot_to_bevy_query:ty) => {
        $crate::paste::paste! {
            #[cfg(debug_assertions)]
            #[tracing::instrument]
            pub fn [<pre_update_godot_transforms_ $name:lower>](
                entities: $crate::bevy_ecs::system::Query<
                    (
                        $crate::bevy_ecs::entity::Entity,
                        &mut $crate::bevy_transform::components::Transform,
                        &$crate::interop::GodotNodeHandle,
                        &mut $crate::plugins::transforms::TransformSyncMetadata,
                        $crate::bevy_ecs::query::AnyOf<(&$crate::interop::node_markers::Node2DMarker, &$crate::interop::node_markers::Node3DMarker)>,
                    ),
                    $godot_to_bevy_query
                >,
                mut godot: $crate::interop::GodotAccess,
                bulk_ops_cache: $crate::bevy_ecs::system::NonSendMut<$crate::interop::BulkOperationsCache>,
            ) {
                if let Some(bulk_ops_node) = bulk_ops_cache.get() {
                    [<pre_update_godot_transforms_ $name:lower _bulk>](
                        entities,
                        bulk_ops_node,
                    );
                    return;
                }
                [<pre_update_godot_transforms_ $name:lower _individual>](entities, &mut godot);
            }

            #[cfg(not(debug_assertions))]
            #[tracing::instrument]
            pub fn [<pre_update_godot_transforms_ $name:lower>](
                entities: $crate::bevy_ecs::system::Query<
                    (
                        $crate::bevy_ecs::entity::Entity,
                        &mut $crate::bevy_transform::components::Transform,
                        &$crate::interop::GodotNodeHandle,
                        &mut $crate::plugins::transforms::TransformSyncMetadata,
                        $crate::bevy_ecs::query::AnyOf<(&$crate::interop::node_markers::Node2DMarker, &$crate::interop::node_markers::Node3DMarker)>,
                    ),
                    $godot_to_bevy_query
                >,
                mut godot: $crate::interop::GodotAccess,
            ) {
                [<pre_update_godot_transforms_ $name:lower _individual>](entities, &mut godot);
            }

            #[cfg(debug_assertions)]
            fn [<pre_update_godot_transforms_ $name:lower _bulk>](
                mut entities: $crate::bevy_ecs::system::Query<
                    (
                        $crate::bevy_ecs::entity::Entity,
                        &mut $crate::bevy_transform::components::Transform,
                        &$crate::interop::GodotNodeHandle,
                        &mut $crate::plugins::transforms::TransformSyncMetadata,
                        $crate::bevy_ecs::query::AnyOf<(&$crate::interop::node_markers::Node2DMarker, &$crate::interop::node_markers::Node3DMarker)>,
                    ),
                    $godot_to_bevy_query
                >,
                mut batch_singleton: godot::prelude::Gd<godot::classes::Object>,
            ) {
                use $crate::bevy_ecs::change_detection::DetectChanges;
                use godot::prelude::ToGodot;

                let _span = tracing::info_span!("bulk_read_preparation", system = stringify!($name)).entered();

                // Collect entity info for 3D and 2D nodes separately
                let mut entities_3d: Vec<($crate::bevy_ecs::entity::Entity, i64)> = Vec::new();
                let mut entities_2d: Vec<($crate::bevy_ecs::entity::Entity, i64)> = Vec::new();

                for (entity, _, reference, _, (node2d, node3d)) in entities.iter() {
                    let instance_id = reference.instance_id().to_i64();
                    if node2d.is_some() {
                        entities_2d.push((entity, instance_id));
                    } else if node3d.is_some() {
                        entities_3d.push((entity, instance_id));
                    }
                }

                drop(_span);

                // Process 3D entities
                if !entities_3d.is_empty() {
                    let _span = tracing::info_span!("bulk_read_3d", count = entities_3d.len(), system = stringify!($name)).entered();

                    let instance_ids: Vec<i64> = entities_3d.iter().map(|(_, id)| *id).collect();
                    let ids_packed = godot::prelude::PackedInt64Array::from(instance_ids.as_slice());

                    let result = batch_singleton
                        .call("bulk_get_transforms_3d", &[ids_packed.to_variant()])
                        .to::<godot::builtin::VarDictionary>();

                    if let (Some(positions), Some(rotations), Some(scales)) = (
                        result
                            .get("positions")
                            .map(|v| v.to::<godot::builtin::PackedVector3Array>()),
                        result
                            .get("rotations")
                            .map(|v| v.to::<godot::builtin::PackedVector4Array>()),
                        result
                            .get("scales")
                            .map(|v| v.to::<godot::builtin::PackedVector3Array>()),
                    ) {
                        for (i, (entity, _)) in entities_3d.iter().enumerate() {
                            if let Ok((_, mut bevy_transform, _, mut metadata, _)) = entities.get_mut(*entity) {
                                if let (Some(pos), Some(rot), Some(scale)) =
                                    (positions.get(i), rotations.get(i), scales.get(i))
                                {
                                    let new_bevy_transform = $crate::bevy_transform::components::Transform {
                                        translation: $crate::bevy_math::Vec3::new(pos.x, pos.y, pos.z),
                                        rotation: $crate::bevy_math::Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w),
                                        scale: $crate::bevy_math::Vec3::new(scale.x, scale.y, scale.z),
                                    };

                                    if *bevy_transform != new_bevy_transform {
                                        *bevy_transform = new_bevy_transform;
                                        metadata.last_sync_tick = Some(bevy_transform.last_changed());
                                    }
                                }
                            }
                        }
                    }
                }

                // Process 2D entities
                if !entities_2d.is_empty() {
                    let _span = tracing::info_span!("bulk_read_2d", count = entities_2d.len(), system = stringify!($name)).entered();

                    let instance_ids: Vec<i64> = entities_2d.iter().map(|(_, id)| *id).collect();
                    let ids_packed = godot::prelude::PackedInt64Array::from(instance_ids.as_slice());

                    let result = batch_singleton
                        .call("bulk_get_transforms_2d", &[ids_packed.to_variant()])
                        .to::<godot::builtin::VarDictionary>();

                    if let (Some(positions), Some(rotations), Some(scales)) = (
                        result
                            .get("positions")
                            .map(|v| v.to::<godot::builtin::PackedVector2Array>()),
                        result
                            .get("rotations")
                            .map(|v| v.to::<godot::builtin::PackedFloat32Array>()),
                        result
                            .get("scales")
                            .map(|v| v.to::<godot::builtin::PackedVector2Array>()),
                    ) {
                        for (i, (entity, _)) in entities_2d.iter().enumerate() {
                            if let Ok((_, mut bevy_transform, _, mut metadata, _)) = entities.get_mut(*entity) {
                                if let (Some(pos), Some(rot), Some(scale)) =
                                    (positions.get(i), rotations.get(i), scales.get(i))
                                {
                                    let new_bevy_transform = $crate::bevy_transform::components::Transform {
                                        translation: $crate::bevy_math::Vec3::new(pos.x, pos.y, 0.0),
                                        rotation: $crate::bevy_math::Quat::from_rotation_z(rot),
                                        scale: $crate::bevy_math::Vec3::new(scale.x, scale.y, 1.0),
                                    };

                                    if *bevy_transform != new_bevy_transform {
                                        *bevy_transform = new_bevy_transform;
                                        metadata.last_sync_tick = Some(bevy_transform.last_changed());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            fn [<pre_update_godot_transforms_ $name:lower _individual>](
                mut entities: $crate::bevy_ecs::system::Query<
                    (
                        $crate::bevy_ecs::entity::Entity,
                        &mut $crate::bevy_transform::components::Transform,
                        &$crate::interop::GodotNodeHandle,
                        &mut $crate::plugins::transforms::TransformSyncMetadata,
                        $crate::bevy_ecs::query::AnyOf<(&$crate::interop::node_markers::Node2DMarker, &$crate::interop::node_markers::Node3DMarker)>,
                    ),
                    $godot_to_bevy_query
                >,
                godot: &mut $crate::interop::GodotAccess,
            ) {
                use $crate::plugins::transforms::IntoBevyTransform;
                use $crate::bevy_ecs::change_detection::DetectChanges;
                use godot::classes::{Node2D, Node3D};

                for (_, mut bevy_transform, reference, mut metadata, (node2d, node3d)) in entities.iter_mut() {
                    let new_bevy_transform = if node2d.is_some() {
                        godot
                            .get::<Node2D>(*reference)
                            .get_transform()
                            .to_bevy_transform()
                    } else if node3d.is_some() {
                        godot
                            .get::<Node3D>(*reference)
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

            $app.add_systems($crate::bevy_app::PreUpdate, [<pre_update_godot_transforms_ $name:lower>]);
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
