//! Auto-sync bundle system for automatic bundle registration
//!
//! `#[derive(BevyBundle)]` registers a bundle-creation function keyed by the
//! type's Godot class. When a node enters the scene tree, the matching creators
//! for the node's class (and its ancestors) are run.

use bevy_app::App;
use bevy_ecs::{entity::Entity, system::Commands};
use godot::meta::ClassId;
use std::collections::HashMap;
use std::sync::OnceLock;
use tracing::trace;

use crate::interop::{GodotAccess, GodotNodeHandle};

/// Function type for creating bundles from Godot nodes
pub type BundleCreatorFn = fn(&mut Commands, Entity, &mut GodotAccess, GodotNodeHandle) -> bool;

/// Registry entry for auto-sync bundles using the inventory crate
pub struct AutoSyncBundleRegistry {
    /// The bundle struct's name — human-readable, used in trace logs.
    pub godot_class_name: &'static str,
    /// The type's real registered Godot class id (handles `#[class(rename)]`).
    pub godot_class_id_fn: fn() -> ClassId,
    /// Function to create and add the bundle to an entity
    pub create_bundle_fn: BundleCreatorFn,
}

// Collect all auto-sync bundle registrations
crate::inventory::collect!(AutoSyncBundleRegistry);

// Registered creators keyed by Godot class name, built once at plugin build.
static BUNDLE_REGISTRY: OnceLock<HashMap<String, Vec<&'static AutoSyncBundleRegistry>>> =
    OnceLock::new();

/// Initialize the global bundle registry, keyed by Godot class name.
pub fn register_all_autosync_bundles(_app: &mut App) {
    BUNDLE_REGISTRY.get_or_init(|| {
        let mut map: HashMap<String, Vec<&'static AutoSyncBundleRegistry>> = HashMap::new();
        for entry in crate::inventory::iter::<AutoSyncBundleRegistry> {
            map.entry((entry.godot_class_id_fn)().to_string())
                .or_default()
                .push(entry);
        }

        tracing::debug!(
            "Registered {} AutoSyncBundle entries across {} Godot classes",
            map.values().map(Vec::len).sum::<usize>(),
            map.len()
        );
        map
    });
}

/// Add registered bundles for a node, matched by its Godot class hierarchy.
///
/// `class_hierarchy` is the node's class followed by its ancestors (as produced
/// by the scene-tree plugin). A registered type matches iff its class name is in
/// that chain — i.e. the node is-a that type. The matched `create_bundle_fn`
/// still does a `try_get` to retrieve the typed node it builds the bundle from.
pub fn try_add_bundles_for_node(
    commands: &mut Commands,
    entity: Entity,
    godot: &mut GodotAccess,
    node_handle: GodotNodeHandle,
    class_hierarchy: &[String],
) {
    let Some(registry) = BUNDLE_REGISTRY.get() else {
        return;
    };
    for class_name in class_hierarchy {
        let Some(entries) = registry.get(class_name) else {
            continue;
        };
        for entry in entries {
            if (entry.create_bundle_fn)(commands, entity, godot, node_handle) {
                trace!(
                    "Added bundle for {} to entity {:?}",
                    entry.godot_class_name, entity
                );
            }
        }
    }
}
