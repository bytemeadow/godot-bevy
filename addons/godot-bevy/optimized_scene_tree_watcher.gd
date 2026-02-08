class_name OptimizedSceneTreeWatcher
extends Node

# 🤖 This file is generated. Changes to it will be lost.
# To regenerate: uv run python -m godot_bevy_codegen

## Optimized Scene Tree Watcher
##
## This GDScript class serves as a high-performance bridge between Godot's scene tree
## and the Bevy ECS (Entity Component System) in the godot-bevy integration.
##
## Key responsibilities:
## - Intercepts scene tree events (node added, removed, renamed) via Godot signals
## - Pre-analyzes node metadata (type, name, parent, collision signals, groups) on the
##   GDScript side to minimize expensive FFI (Foreign Function Interface) calls
## - Forwards optimized event data to the Rust SceneTreeWatcher for Bevy entity creation
## - Provides initial scene tree analysis for bulk entity spawning during startup
## - Supports multiple deployment strategies (production autoload, test framework)

# Reference to the Rust SceneTreeWatcher
var rust_watcher: Node = null


func _ready():
    name = "OptimizedSceneTreeWatcher"

    # Auto-detect the Rust SceneTreeWatcher using multiple strategies:
    # 1. Try production path: /root/BevyAppSingleton (autoload singleton)
    # 2. Try as sibling: get_parent().get_node("SceneTreeWatcher") (test framework)
    # 3. Use set_rust_watcher() if watcher is set externally

    # Strategy 1: Production - BevyApp autoload singleton
    var bevy_app: Node = get_node_or_null("/root/BevyAppSingleton")
    if bevy_app:
        rust_watcher = bevy_app.get_node_or_null("SceneTreeWatcher")

    # Strategy 2: Test environment - sibling node
    if not rust_watcher and get_parent():
        rust_watcher = get_parent().get_node_or_null("SceneTreeWatcher")

    # If still not found, it may be set later via set_rust_watcher()
    if not rust_watcher:
        push_warning("[OptimizedSceneTreeWatcher] SceneTreeWatcher not found. Will wait for set_rust_watcher() call.")

    # Connect to scene tree signals - these will forward to Rust with type info
    # Use immediate connections for add/remove to get events as early as possible
    get_tree().node_added.connect(_on_node_added)
    get_tree().node_removed.connect(_on_node_removed)
    get_tree().node_renamed.connect(_on_node_renamed, CONNECT_DEFERRED)


func set_rust_watcher(watcher: Node):
    """Called from Rust to set the SceneTreeWatcher reference (optional)"""
    rust_watcher = watcher


func _on_node_added(node: Node):
    """Handle node added events with type optimization"""
    if not rust_watcher:
        return

    # Check if node is still valid
    if not is_instance_valid(node):
        return

    # Check if node is marked to be excluded from scene tree watcher
    if node.has_meta("_bevy_exclude"):
        return

    # Analyze node type on GDScript side - this is much faster than FFI
    var node_type: String = node.get_class()
    var node_name: StringName = node.name
    var parent: Node = node.get_parent()
    var parent_id: int = parent.get_instance_id() if parent else 0
    var collision_mask: int = _compute_collision_mask(node)

    # Collect groups for this node
    var node_groups: PackedStringArray = PackedStringArray()
    for group: StringName in node.get_groups():
        node_groups.append(group)

    # Forward to Rust watcher with pre-analyzed metadata
    # Try newest API first (with groups), then fall back to older APIs
    if rust_watcher.has_method("scene_tree_event_typed_metadata_groups"):
        rust_watcher.scene_tree_event_typed_metadata_groups(
            node,
            "NodeAdded",
            node_type,
            node_name,
            parent_id,
            collision_mask,
            node_groups
        )
    elif rust_watcher.has_method("scene_tree_event_typed_metadata"):
        rust_watcher.scene_tree_event_typed_metadata(
            node,
            "NodeAdded",
            node_type,
            node_name,
            parent_id,
            collision_mask
        )
    elif rust_watcher.has_method("scene_tree_event_typed"):
        rust_watcher.scene_tree_event_typed(node, "NodeAdded", node_type)
    else:
        # Fallback to regular method if typed method not available
        rust_watcher.scene_tree_event(node, "NodeAdded")

func _on_node_removed(node: Node):
    """Handle node removed events - no type analysis needed for removal"""
    if not rust_watcher:
        return

    # This is called immediately (not deferred) so the node should still be valid
    # We need to send this event so Rust can clean up the corresponding Bevy entity
    rust_watcher.scene_tree_event(node, "NodeRemoved")

func _on_node_renamed(node: Node):
    """Handle node renamed events - no type analysis needed for renaming"""
    if not rust_watcher:
        return

    # Check if node is still valid
    if not is_instance_valid(node):
        return

    var node_name: StringName = node.name
    if rust_watcher.has_method("scene_tree_event_named"):
        rust_watcher.scene_tree_event_named(node, "NodeRenamed", node_name)
    else:
        rust_watcher.scene_tree_event(node, "NodeRenamed")

func _compute_collision_mask(node: Node) -> int:
    var mask: int = 0
    if node.has_signal("body_entered"):
        mask |= 1
    if node.has_signal("body_exited"):
        mask |= 2
    if node.has_signal("area_entered"):
        mask |= 4
    if node.has_signal("area_exited"):
        mask |= 8
    return mask


func analyze_initial_tree() -> Dictionary:
    """
    Analyze the entire initial scene tree and return node information with types.
    Returns a Dictionary with PackedArrays for maximum performance:
    {
        "instance_ids": PackedInt64Array,
        "node_types": PackedStringArray,
        "node_names": PackedStringArray,
        "parent_ids": PackedInt64Array,
        "collision_masks": PackedInt64Array,
        "groups": Array[PackedStringArray]  # Added in v2 - may not be present in older addons
    }
    Used for optimized initial scene tree setup.
    """
    var instance_ids: PackedInt64Array = PackedInt64Array()
    var node_types: PackedStringArray = PackedStringArray()
    var node_names: PackedStringArray = PackedStringArray()
    var parent_ids: PackedInt64Array = PackedInt64Array()
    var collision_masks: PackedInt64Array = PackedInt64Array()
    var groups: Array = []  # Array of PackedStringArrays
    var root: Window = get_tree().get_root()
    if root:
        _analyze_node_recursive(root, instance_ids, node_types, node_names, parent_ids, collision_masks, groups)

    return {
        "instance_ids": instance_ids,
        "node_types": node_types,
        "node_names": node_names,
        "parent_ids": parent_ids,
        "collision_masks": collision_masks,
        "groups": groups
    }


func _analyze_node_recursive(
    node: Node,
    instance_ids: PackedInt64Array,
    node_types: PackedStringArray,
    node_names: PackedStringArray,
    parent_ids: PackedInt64Array,
    collision_masks: PackedInt64Array,
    groups: Array
):
    """Recursively analyze nodes and collect type information into PackedArrays"""
    # Check if node is still valid before processing
    if not is_instance_valid(node):
        return

    # Check if node is marked to be excluded from scene tree watcher
    if node.has_meta("_bevy_exclude"):
        return

    # Add this node's information with pre-analyzed type
    var instance_id: int = node.get_instance_id()
    var node_type: String = node.get_class()
    var node_name: StringName = node.name
    var parent: Node = node.get_parent()
    var parent_id: int = parent.get_instance_id() if parent else 0
    var collision_mask: int = _compute_collision_mask(node)

    # Collect groups for this node
    var node_groups: PackedStringArray = PackedStringArray()
    for group: StringName in node.get_groups():
        node_groups.append(group)

    # Only append if we have valid data
    if instance_id != 0 and node_type != "":
        instance_ids.append(instance_id)
        node_types.append(node_type)
        node_names.append(node_name)
        parent_ids.append(parent_id)
        collision_masks.append(collision_mask)
        groups.append(node_groups)

    # Recursively process children
    for child: Node in node.get_children():
        _analyze_node_recursive(child, instance_ids, node_types, node_names, parent_ids, collision_masks, groups)
