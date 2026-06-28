/// Macro for registering transform synchronization for a subset of entities.
///
/// Custom sync is just auto sync with a query filter: this macro registers the
/// same shared sync systems the [`GodotTransformSyncPlugin`] uses, restricted to
/// entities matching your filter. Both 2D and 3D nodes are handled, and the
/// change-detection guard and physics-interpolation reset behave identically to
/// auto sync.
///
/// [`GodotTransformSyncPlugin`]: crate::plugins::transforms::GodotTransformSyncPlugin
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
        $crate::add_transform_sync_systems!(@generate_post_system $app, $query);
        $crate::add_transform_sync_systems!(@parse_all $app, $($rest)*);
    };

    (@parse_all $app:expr, $name:ident = godot_to_bevy: $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems!(@generate_pre_system $app, $query);
        $crate::add_transform_sync_systems!(@parse_all $app, $($rest)*);
    };

    (@parse_all $app:expr, $name:ident = $query:ty, $($rest:tt)*) => {
        $crate::add_transform_sync_systems!(@generate_systems $app, $query, $query);
        $crate::add_transform_sync_systems!(@parse_all $app, $($rest)*);
    };

    // Handle last item (without trailing comma)
    (@parse_all $app:expr, $name:ident = bevy_to_godot: $query:ty) => {
        $crate::add_transform_sync_systems!(@generate_post_system $app, $query);
    };

    (@parse_all $app:expr, $name:ident = godot_to_bevy: $query:ty) => {
        $crate::add_transform_sync_systems!(@generate_pre_system $app, $query);
    };

    (@parse_all $app:expr, $name:ident = $query:ty) => {
        $crate::add_transform_sync_systems!(@generate_systems $app, $query, $query);
    };

    // Handle empty case
    (@parse_all $app:expr,) => {};
    (@parse_all $app:expr) => {};

    (@generate_systems $app:expr, $bevy_to_godot_query:ty, $godot_to_bevy_query:ty) => {
        $crate::add_transform_sync_systems!(@generate_post_system $app, $bevy_to_godot_query);
        $crate::add_transform_sync_systems!(@generate_pre_system $app, $godot_to_bevy_query);
    };

    // Bevy → Godot write, restricted to the filter. Runs in `FixedLast` (physics
    // rate) to match auto sync and Godot's physics-interpolation cadence.
    (@generate_post_system $app:expr, $bevy_to_godot_query:ty) => {
        $app.add_systems(
            $crate::bevy_app::FixedLast,
            $crate::plugins::transforms::sync_systems::post_update_godot_transforms::<$bevy_to_godot_query>,
        );
    };

    // Godot → Bevy read, restricted to the filter. Runs in `PreUpdate` (step 1,
    // suffix, idle frames) and once per physics step in `FixedFirst` for steps
    // 2..N, matching auto sync's per-step cadence so a Godot author between steps
    // isn't clobbered. No twoway gate here (direction is opt-in via the filter),
    // but the `not_first_fixed_step` dedup keeps 1-step frames perf-neutral.
    (@generate_pre_system $app:expr, $godot_to_bevy_query:ty) => {
        $app.add_systems(
            $crate::bevy_app::PreUpdate,
            $crate::plugins::transforms::sync_systems::pre_update_godot_transforms::<$godot_to_bevy_query>,
        );
        $app.add_systems(
            $crate::bevy_app::FixedFirst,
            $crate::plugins::transforms::sync_systems::pre_update_godot_transforms::<$godot_to_bevy_query>.run_if($crate::plugins::fixed_schedule::not_first_fixed_step),
        );
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
