class_name OptimizedSceneTreeWatcher
extends Node

## This GDScript class serves as a high-performance bridge between Godot's scene tree
## and the Bevy ECS (Entity Component System) in the godot-bevy integration.
## It pre-analyzes node metadata (type, name, parent, collision signals, groups) on the
##   GDScript side to minimize expensive FFI (Foreign Function Interface) calls

var rust_watcher: Node = null


func _ready():
    name = "OptimizedSceneTreeWatcher"

    var bevy_app: Node = get_node_or_null("/root/BevyAppSingleton")
    if bevy_app:
        rust_watcher = bevy_app.get_node_or_null("SceneTreeWatcher")

    if not rust_watcher and get_parent():
        rust_watcher = get_parent().get_node_or_null("SceneTreeWatcher")

    if not rust_watcher:
        push_warning("[OptimizedSceneTreeWatcher] SceneTreeWatcher not found. Will wait for set_rust_watcher() call.")

    # Use immediate connections for add/remove to get events as early as possible
    get_tree().node_added.connect(_on_node_added)
    get_tree().node_removed.connect(_on_node_removed)
    get_tree().node_renamed.connect(_on_node_renamed, CONNECT_DEFERRED)


func set_rust_watcher(watcher: Node):
    rust_watcher = watcher


func _is_excluded_from_mirror(node: Node) -> bool:
    # True if this node or any ancestor carries the _bevy_exclude meta. Exclusion is
    # subtree-wide, matching the initial walk's recursion-halt.
    var current: Node = node
    while current:
        if current.has_meta("_bevy_exclude"):
            return true
        current = current.get_parent()
    return false


func _on_node_added(node: Node):
    if not rust_watcher:
        return

    if not is_instance_valid(node):
        return

    if _is_excluded_from_mirror(node):
        return

    # Analyze node type on GDScript side - this is much faster than FFI
    var node_type: String = node.get_class()
    var node_name: StringName = node.name
    var parent: Node = node.get_parent()
    var parent_id: int = parent.get_instance_id() if parent else 0
    var collision_mask: int = _compute_collision_mask(node)

    var node_groups: PackedStringArray = PackedStringArray()
    for group: StringName in node.get_groups():
        node_groups.append(group)

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
        rust_watcher.scene_tree_event(node, "NodeAdded")

func _on_node_removed(node: Node):
    if not rust_watcher:
        return

    # This is called immediately (not deferred) so the node should still be valid
    rust_watcher.scene_tree_event(node, "NodeRemoved")

func _on_node_renamed(node: Node):
    if not rust_watcher:
        return

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
    if not is_instance_valid(node):
        return

    if node.has_meta("_bevy_exclude"):
        return

    var instance_id: int = node.get_instance_id()
    var node_type: String = node.get_class()
    var node_name: StringName = node.name
    var parent: Node = node.get_parent()
    var parent_id: int = parent.get_instance_id() if parent else 0
    var collision_mask: int = _compute_collision_mask(node)

    var node_groups: PackedStringArray = PackedStringArray()
    for group: StringName in node.get_groups():
        node_groups.append(group)

    if instance_id != 0 and node_type != "":
        instance_ids.append(instance_id)
        node_types.append(node_type)
        node_names.append(node_name)
        parent_ids.append(parent_id)
        collision_masks.append(collision_mask)
        groups.append(node_groups)

    for child: Node in node.get_children():
        _analyze_node_recursive(child, instance_ids, node_types, node_names, parent_ids, collision_masks, groups)
