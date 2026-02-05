import shutil
import textwrap
from pathlib import Path
from typing import Dict, List

from godot_bevy_codegen.src.file_paths import FilePaths
from godot_bevy_codegen.src.gdextension_api import ExtensionApi
from godot_bevy_codegen.src.special_cases import categorize_types_by_hierarchy
from godot_bevy_codegen.src.util import (
    indent_log,
)


def generate_gdscript_watcher(
    gdscript_watcher_file: Path,
    api: ExtensionApi,
) -> None:
    """Generate the optimized GDScript scene tree watcher with all node types"""
    indent_log("📜 Generating GDScript optimized scene tree watcher...")

    node_types = api.classes_descended_from("Node")

    # Filter and categorize types
    categories = categorize_types_by_hierarchy(node_types, api.parent_map())

    content = textwrap.dedent(
        f'''\
        class_name OptimizedSceneTreeWatcher
        extends Node
        
        # 🤖 This file is generated. Changes to it will be lost.
        # To regenerate: uv run python -m godot_bevy_codegen
        
        # Generated for Godot version: {api.header.version_full_name}
        # If you need support for a different version, swap out `optimized_scene_tree_watcher.gd`
        # with `optimized_scene_tree_watcher_versions/optimized_scene_tree_watcher*_*_*.gd_ignore` of your desired version.
        
        # Optimized Scene Tree Watcher
        # This GDScript class intercepts scene tree events and performs type analysis
        # on the GDScript side to avoid expensive FFI calls from Rust.
        # Handles {len(node_types)} different Godot node types.
        
        # Reference to the Rust SceneTreeWatcher
        var rust_watcher: Node = null
        
        func _ready():
            name = "OptimizedSceneTreeWatcher"
        
            # Auto-detect the Rust SceneTreeWatcher using multiple strategies:
            # 1. Try production path: /root/BevyAppSingleton (autoload singleton)
            # 2. Try as sibling: get_parent().get_node("SceneTreeWatcher") (test framework)
            # 3. Use set_rust_watcher() if watcher is set externally
        
            # Strategy 1: Production - BevyApp autoload singleton
            var bevy_app = get_node_or_null("/root/BevyAppSingleton")
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
            var node_type = _analyze_node_type(node)
            var node_name = node.name
            var parent = node.get_parent()
            var parent_id = parent and parent.get_instance_id() or 0
            var collision_mask = _compute_collision_mask(node)
        
            # Collect groups for this node
            var node_groups = PackedStringArray()
            for group in node.get_groups():
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
        
            var node_name = node.name
            if rust_watcher.has_method("scene_tree_event_named"):
                rust_watcher.scene_tree_event_named(node, "NodeRenamed", node_name)
            else:
                rust_watcher.scene_tree_event(node, "NodeRenamed")
        
        func _compute_collision_mask(node: Node) -> int:
            var mask = 0
            if node.has_signal("body_entered"):
                mask |= 1
            if node.has_signal("body_exited"):
                mask |= 2
            if node.has_signal("area_entered"):
                mask |= 4
            if node.has_signal("area_exited"):
                mask |= 8
            return mask
        
        func _analyze_node_type(node: Node) -> String:
            """
            Analyze node type hierarchy on GDScript side.
            Returns the most specific built-in Godot type name.
            This avoids multiple FFI calls that would be needed on the Rust side.
            Generated from Godot extension API to ensure completeness.
            """
        {textwrap.indent(_generate_gdscript_type_analysis(categories), '        ')}
        
            # Default fallback
            return "Node"
        
        {textwrap.indent(_generate_initial_tree_analysis(), '        ')}'''
    )

    gdscript_watcher_file.parent.mkdir(parents=True, exist_ok=True)
    with open(gdscript_watcher_file, "w") as f:
        f.write(content)

    indent_log(f"✅ Generated GDScript watcher with {len(node_types)} node types")


def _generate_initial_tree_analysis() -> str:
    """Generate method for analyzing the initial scene tree with type info"""
    return textwrap.dedent(
        '''
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
            var instance_ids = PackedInt64Array()
            var node_types = PackedStringArray()
            var node_names = PackedStringArray()
            var parent_ids = PackedInt64Array()
            var collision_masks = PackedInt64Array()
            var groups = []  # Array of PackedStringArrays
            var root = get_tree().get_root()
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
            
        func _analyze_node_recursive(node: Node, instance_ids: PackedInt64Array, node_types: PackedStringArray, node_names: PackedStringArray, parent_ids: PackedInt64Array, collision_masks: PackedInt64Array, groups: Array):
            """Recursively analyze nodes and collect type information into PackedArrays"""
            # Check if node is still valid before processing
            if not is_instance_valid(node):
                return
            
            # Check if node is marked to be excluded from scene tree watcher
            if node.has_meta("_bevy_exclude"):
                return
            
            # Add this node's information with pre-analyzed type
            var instance_id = node.get_instance_id()
            var node_type = _analyze_node_type(node)
            var node_name = node.name
            var parent = node.get_parent()
            var parent_id = parent and parent.get_instance_id() or 0
            var collision_mask = _compute_collision_mask(node)
            
            # Collect groups for this node
            var node_groups = PackedStringArray()
            for group in node.get_groups():
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
            for child in node.get_children():
                _analyze_node_recursive(child, instance_ids, node_types, node_names, parent_ids, collision_masks, groups)
        '''
    )


def _generate_gdscript_type_analysis(categories: Dict[str, List[str]]) -> str:
    """Generate the GDScript node type analysis function"""
    # Node3D hierarchy (most common in 3D games)
    lines = [
        "",
        "    # Check Node3D hierarchy first (most common in 3D games)",
        "    if node is Node3D:",
    ]

    # Add common 3D types first for better performance
    common_3d = [
        "MeshInstance3D",
        "StaticBody3D",
        "RigidBody3D",
        "CharacterBody3D",
        "Area3D",
        "Camera3D",
        "DirectionalLight3D",
        "OmniLight3D",
        "SpotLight3D",
        "CollisionShape3D",
    ]

    for node_type in common_3d:
        if node_type in categories["3d"]:
            lines.append(f'        if node is {node_type}: return "{node_type}"')

    # Add remaining 3D types
    for node_type in sorted(categories["3d"]):
        if node_type not in common_3d:
            lines.append(f'        if node is {node_type}: return "{node_type}"')

    lines.append('        return "Node3D"')
    lines.append("")

    # Node2D hierarchy (common in 2D games)
    lines.append("    # Check Node2D hierarchy (common in 2D games)")
    lines.append("    elif node is Node2D:")

    # Add common 2D types first
    common_2d = [
        "Sprite2D",
        "StaticBody2D",
        "RigidBody2D",
        "CharacterBody2D",
        "Area2D",
        "Camera2D",
        "CollisionShape2D",
        "AnimatedSprite2D",
    ]

    for node_type in common_2d:
        if node_type in categories["2d"]:
            lines.append(f'        if node is {node_type}: return "{node_type}"')

    # Add remaining 2D types
    for node_type in sorted(categories["2d"]):
        if node_type not in common_2d:
            lines.append(f'        if node is {node_type}: return "{node_type}"')

    lines.append('        return "Node2D"')
    lines.append("")

    # Control hierarchy (UI elements)
    lines.append("    # Check Control hierarchy (UI elements)")
    lines.append("    elif node is Control:")

    # Add common UI types first
    common_control = [
        "Button",
        "Label",
        "Panel",
        "VBoxContainer",
        "HBoxContainer",
        "MarginContainer",
        "ColorRect",
        "LineEdit",
        "TextEdit",
        "CheckBox",
    ]

    for node_type in common_control:
        if node_type in categories["control"]:
            lines.append(f'        if node is {node_type}: return "{node_type}"')

    # Add remaining Control types
    for node_type in sorted(categories["control"]):
        if node_type not in common_control:
            lines.append(f'        if node is {node_type}: return "{node_type}"')

    lines.append('        return "Control"')
    lines.append("")

    # Universal types (direct Node children)
    lines.append("    # Check other common node types that inherit directly from Node")
    common_universal = [
        "AnimationPlayer",
        "Timer",
        "AudioStreamPlayer",
        "HTTPRequest",
        "CanvasLayer",
    ]

    for node_type in common_universal:
        if node_type in categories["universal"]:
            lines.append(f'    elif node is {node_type}: return "{node_type}"')

    # Add remaining universal types
    for node_type in sorted(categories["universal"]):
        if node_type not in common_universal:
            lines.append(f'    elif node is {node_type}: return "{node_type}"')

    return "\n".join(lines)


def use_watcher_version(version: str) -> None:
    most_recent_gdscript_watcher_file: Path = FilePaths.gdscript_watcher_file(version)
    shutil.copy(
        most_recent_gdscript_watcher_file, FilePaths.gdscript_watcher_current_file
    )
