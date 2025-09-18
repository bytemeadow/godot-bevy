#!/usr/bin/env python3
"""
Fully automatic Godot type generation for godot-bevy.

This script:
1. Runs `godot --dump-extension-api` to generate extension_api.json
2. Parses all Node-derived types from the API
3. Generates comprehensive node marker components
4. Generates complete type checking functions
5. Updates the scene tree plugin to use generated code

Usage: python scripts/generate_godot_types.py
"""

import json
import os
import subprocess
import sys
from pathlib import Path
from collections import defaultdict

class GodotTypeGenerator:
    def __init__(self):
        self.project_root = Path(__file__).parent.parent
        self.api_file = self.project_root / "extension_api.json"
        self.node_markers_file = self.project_root / "godot-bevy" / "src" / "interop" / "node_markers.rs"
        self.type_checking_file = self.project_root / "godot-bevy" / "src" / "plugins" / "scene_tree" / "node_type_checking_generated.rs"
        self.plugin_file = self.project_root / "godot-bevy" / "src" / "plugins" / "scene_tree" / "plugin.rs"
        self.gdscript_watcher_file = self.project_root / "addons" / "godot-bevy" / "optimized_scene_tree_watcher.gd"

        # Types that require specific Godot API versions
        # Based on Godot release notes and documentation
        self.version_gated_types = {
            "4-4": [  # Types added in Godot 4.4+
                "LookAtModifier3D",
                "RetargetModifier3D",
                "SpringBoneSimulator3D",
                "SpringBoneCollision3D",
                "SpringBoneCollisionCapsule3D",
                "SpringBoneCollisionPlane3D",
                "SpringBoneCollisionSphere3D",
            ],
            # Add more versions as needed
        }

    def run_godot_dump_api(self):
        """Run godot --dump-extension-api to generate extension_api.json"""
        print("🚀 Generating extension_api.json from Godot...")

        try:
            # Try different common Godot executable names
            godot_commands = ["godot", "godot4", "/usr/local/bin/godot"]

            for cmd in godot_commands:
                try:
                    result = subprocess.run([
                        cmd, "--headless", "--dump-extension-api", str(self.api_file)
                    ], capture_output=True, text=True, timeout=30)

                    if result.returncode == 0 and self.api_file.exists():
                        print(f"✅ Successfully generated extension_api.json using '{cmd}'")
                        return

                except (subprocess.TimeoutExpired, FileNotFoundError):
                    continue

            # If all commands failed, give helpful error
            raise RuntimeError(
                "Could not run Godot to generate extension_api.json.\n"
                "Please ensure Godot 4 is installed and available in PATH.\n"
                "You can also manually run: godot --dump-extension-api extension_api.json"
            )

        except Exception as e:
            print(f"❌ Error generating extension_api.json: {e}")
            sys.exit(1)

    def load_and_parse_extension_api(self):
        """Load and parse the extension API to extract node types"""
        print("📖 Parsing extension API...")

        if not self.api_file.exists():
            raise FileNotFoundError(f"extension_api.json not found at {self.api_file}")

        with open(self.api_file) as f:
            api = json.load(f)

        # Build inheritance relationships
        inheritance_map = defaultdict(list)
        parent_map = {}

        for class_info in api["classes"]:
            name = class_info["name"]
            if "inherits" in class_info:
                parent = class_info["inherits"]
                inheritance_map[parent].append(name)
                parent_map[name] = parent

        # Collect all Node-derived types
        node_types = set()

        def collect_descendants(class_name):
            node_types.add(class_name)
            for child in inheritance_map.get(class_name, []):
                collect_descendants(child)

        collect_descendants("Node")

        # Filter out base Node class and editor-only classes
        excluded_prefixes = ["Editor", "ScriptEditor", "VisualShader"]
        excluded_types = {"Node", "MissingNode", "ImporterMeshInstance3D"}

        filtered_types = sorted([
            t for t in node_types
            if not any(t.startswith(prefix) for prefix in excluded_prefixes)
            and t not in excluded_types
        ])

        print(f"✅ Found {len(filtered_types)} node types")
        return filtered_types, parent_map

    def get_type_cfg_attribute(self, node_type):
        """Get the cfg attribute for a type if it needs version gating."""
        for version, types in self.version_gated_types.items():
            if node_type in types:
                return f'#[cfg(feature = "api-{version}")]\n'
        return ""

    def generate_node_markers(self, node_types):
        """Generate the node_markers.rs file"""
        print("🏷️  Generating node markers...")

        content = '''use bevy::ecs::component::Component;

/// Marker components for Godot node types.
/// These enable type-safe ECS queries like: Query<&GodotNodeHandle, With<Sprite2DMarker>>
///
/// 🤖 This file is automatically generated by scripts/generate_godot_types.py
/// To regenerate: python scripts/generate_godot_types.py

// Base node type marker
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeMarker;

'''

        # Generate all markers
        for node_type in node_types:
            cfg_attr = self.get_type_cfg_attribute(node_type)
            if cfg_attr:
                content += cfg_attr
            content += f"#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]\n"
            content += f"pub struct {node_type}Marker;\n\n"

        with open(self.node_markers_file, "w") as f:
            f.write(content)

        print(f"✅ Generated {len(node_types)} node markers")

    def categorize_types_by_hierarchy(self, node_types, parent_map):
        """Categorize node types by their inheritance hierarchy"""

        def is_descendant_of(node_type, ancestor):
            current = node_type
            while current in parent_map:
                current = parent_map[current]
                if current == ancestor:
                    return True
            return False

        categories = {
            "3d": [],
            "2d": [],
            "control": [],
            "universal": []
        }

        for node_type in node_types:
            if is_descendant_of(node_type, "Node3D"):
                categories["3d"].append(node_type)
            elif is_descendant_of(node_type, "Node2D"):
                categories["2d"].append(node_type)
            elif is_descendant_of(node_type, "Control"):
                categories["control"].append(node_type)
            elif parent_map.get(node_type) == "Node":
                categories["universal"].append(node_type)

        return categories

    def generate_type_checking_code(self, node_types, parent_map):
        """Generate the complete type checking implementation"""
        print("🔍 Generating type checking code...")

        # Filter out invalid Godot classes first to avoid unnecessary work
        valid_types = self.filter_valid_godot_classes(node_types)

        # Categorize only the valid types
        categories = self.categorize_types_by_hierarchy(valid_types, parent_map)

        content = f'''// 🤖 This file is automatically generated by scripts/generate_godot_types.py
// To regenerate: python scripts/generate_godot_types.py

use bevy::ecs::system::EntityCommands;
use crate::interop::{{GodotNodeHandle, node_markers::*}};

/// Adds appropriate marker components to an entity based on the Godot node type.
/// This function is automatically generated and handles all {len(valid_types)} Godot node types.
///
/// Godot's hierarchy: Node -> {{Node3D, CanvasItem -> {{Node2D, Control}}, Others}}
/// We check the major branches: 3D, 2D, Control (UI), and Universal (direct Node children)
pub fn add_comprehensive_node_type_markers(
    entity_commands: &mut EntityCommands,
    node: &mut GodotNodeHandle,
) {{
    // All nodes inherit from Node, so add this first
    entity_commands.insert(NodeMarker);

    // Check the major hierarchy branches to minimize FFI calls
    if node.try_get::<godot::classes::Node3D>().is_some() {{
        entity_commands.insert(Node3DMarker);
        check_3d_node_types_comprehensive(entity_commands, node);
    }} else if node.try_get::<godot::classes::Node2D>().is_some() {{
        entity_commands.insert(Node2DMarker);
        entity_commands.insert(CanvasItemMarker); // Node2D inherits from CanvasItem
        check_2d_node_types_comprehensive(entity_commands, node);
    }} else if node.try_get::<godot::classes::Control>().is_some() {{
        entity_commands.insert(ControlMarker);
        entity_commands.insert(CanvasItemMarker); // Control inherits from CanvasItem
        check_control_node_types_comprehensive(entity_commands, node);
    }}

    // Check node types that inherit directly from Node
    check_universal_node_types_comprehensive(entity_commands, node);
}}

/// Adds node type markers based on a pre-analyzed type string from GDScript.
/// This avoids FFI calls by using type information determined on the GDScript side.
/// This provides significant performance improvements by eliminating multiple
/// GodotNodeHandle::try_get calls for each node.
pub fn add_node_type_markers_from_string(
    entity_commands: &mut EntityCommands,
    node_type: &str,
) {{
    // All nodes inherit from Node
    entity_commands.insert(NodeMarker);
    
    // Add appropriate markers based on the type string
    match node_type {{
{self._generate_string_match_arms(categories)}
        // For any unrecognized type, we already have NodeMarker
        // This handles custom user types that extend Godot nodes
        _ => {{}}
    }}
}}

pub fn remove_comprehensive_node_type_markers(
    entity_commands: &mut EntityCommands,
    node: &mut GodotNodeHandle,
) {{
    // All nodes inherit from Node, so remove this first
    entity_commands.remove::<NodeMarker>();

    entity_commands.remove::<Node3DMarker>();
    remove_3d_node_types_comprehensive(entity_commands, node);

    entity_commands.remove::<Node2DMarker>();
    entity_commands.remove::<CanvasItemMarker>(); // Node2D inherits from CanvasItem
    remove_2d_node_types_comprehensive(entity_commands, node);

    entity_commands.remove::<ControlMarker>();
    remove_control_node_types_comprehensive(entity_commands, node);

    remove_universal_node_types_comprehensive(entity_commands, node);
}}

'''

        # Generate specific checking functions
        content += self._generate_hierarchy_function_comprehensive("3d", categories["3d"])
        content += self._generate_hierarchy_function_comprehensive("2d", categories["2d"])
        content += self._generate_hierarchy_function_comprehensive("control", categories["control"])
        content += self._generate_universal_function_comprehensive(categories["universal"])

        with open(self.type_checking_file, "w") as f:
            f.write(content)

        print(f"✅ Generated type checking for {len(valid_types)} types")

    def filter_valid_godot_classes(self, node_types):
        """Filter out Godot classes that don't exist or aren't available"""
        # Known classes that don't exist in current Godot version or aren't available
        excluded_classes = {
            # CSG classes (require special module)
            'CSGBox3D', 'CSGCombiner3D', 'CSGCylinder3D', 'CSGMesh3D', 'CSGPolygon3D',
            'CSGPrimitive3D', 'CSGShape3D', 'CSGSphere3D', 'CSGTorus3D',
            # Editor classes
            'GridMapEditorPlugin', 'ScriptCreateDialog', 'FileSystemDock',
            'OpenXRBindingModifierEditor', 'OpenXRInteractionProfileEditor',
            'OpenXRInteractionProfileEditorBase',
            # XR classes that might not be available
            'XRAnchor3D', 'XRBodyModifier3D', 'XRCamera3D', 'XRController3D',
            'XRFaceModifier3D', 'XRHandModifier3D', 'XRNode3D', 'XROrigin3D',
            # OpenXR classes
            'OpenXRCompositionLayer', 'OpenXRCompositionLayerCylinder',
            'OpenXRCompositionLayerEquirect', 'OpenXRCompositionLayerQuad',
            'OpenXRHand', 'OpenXRVisibilityMask',
            # Classes that might not be available in all builds
            'VoxelGI', 'LightmapGI', 'FogVolume', 'WorldEnvironment',
            # Navigation classes (might be module-specific)
            'NavigationAgent2D', 'NavigationAgent3D', 'NavigationLink2D',
            'NavigationLink3D', 'NavigationObstacle2D', 'NavigationObstacle3D',
            'NavigationRegion2D', 'NavigationRegion3D',
            # Other problematic classes
            'StatusIndicator',
            # Graph classes (not available in all Godot builds)
            'GraphEdit', 'GraphElement', 'GraphFrame', 'GraphNode',
            # Parallax2D is in extension API but not in current Rust bindings
            'Parallax2D',
        }

        return [t for t in node_types if t not in excluded_classes]

    def fix_godot_class_name_for_rust(self, class_name):
        """Fix Godot class names to match the actual Rust bindings"""
        # Map class names from extension API to actual Rust struct names
        name_fixes = {
            'CPUParticles2D': 'CpuParticles2D',
            'CPUParticles3D': 'CpuParticles3D',
            'GPUParticles2D': 'GpuParticles2D',
            'GPUParticles3D': 'GpuParticles3D',
            'GPUParticlesAttractor3D': 'GpuParticlesAttractor3D',
            'GPUParticlesAttractorBox3D': 'GpuParticlesAttractorBox3D',
            'GPUParticlesAttractorSphere3D': 'GpuParticlesAttractorSphere3D',
            'GPUParticlesAttractorVectorField3D': 'GpuParticlesAttractorVectorField3D',
            'GPUParticlesCollision3D': 'GpuParticlesCollision3D',
            'GPUParticlesCollisionBox3D': 'GpuParticlesCollisionBox3D',
            'GPUParticlesCollisionHeightField3D': 'GpuParticlesCollisionHeightField3D',
            'GPUParticlesCollisionSDF3D': 'GpuParticlesCollisionSdf3d',
            'GPUParticlesCollisionSphere3D': 'GpuParticlesCollisionSphere3D',
            'HTTPRequest': 'HttpRequest',
            'SkeletonIK3D': 'SkeletonIk3d',
            'Generic6DOFJoint3D': 'Generic6DofJoint3D',
        }

        return name_fixes.get(class_name, class_name)

    def _generate_string_match_arms(self, categories):
        """Generate match arms for the string-based marker function"""
        match_arms = []
        
        # Add base types first
        base_types = [
            '        "Node3D" => {\n            entity_commands.insert(Node3DMarker);\n        }',
            '        "Node2D" => {\n            entity_commands.insert(Node2DMarker);\n            entity_commands.insert(CanvasItemMarker);\n        }',
            '        "Control" => {\n            entity_commands.insert(ControlMarker);\n            entity_commands.insert(CanvasItemMarker);\n        }',
            '        "CanvasItem" => {\n            entity_commands.insert(CanvasItemMarker);\n        }',
            '        "Node" => {\n            // NodeMarker already added above\n        }',
        ]
        match_arms.extend(base_types)
        
        # Generate Node3D types (skip base Node3D since it's already handled)
        for node_type in categories["3d"]:
            if node_type == "Node3D":
                continue  # Skip base type
            marker_name = f"{node_type}Marker"
            cfg_attr = self.get_type_cfg_attribute(node_type)
            if cfg_attr:
                match_arms.append(f'''        {cfg_attr.strip()}
        "{node_type}" => {{
            entity_commands.insert(Node3DMarker);
            entity_commands.insert({marker_name});
        }}''')
            else:
                match_arms.append(f'''        "{node_type}" => {{
            entity_commands.insert(Node3DMarker);
            entity_commands.insert({marker_name});
        }}''')
        
        # Generate Node2D types (skip base Node2D since it's already handled)
        for node_type in categories["2d"]:
            if node_type == "Node2D":
                continue  # Skip base type
            marker_name = f"{node_type}Marker"
            cfg_attr = self.get_type_cfg_attribute(node_type)
            if cfg_attr:
                match_arms.append(f'''        {cfg_attr.strip()}
        "{node_type}" => {{
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
            entity_commands.insert({marker_name});
        }}''')
            else:
                match_arms.append(f'''        "{node_type}" => {{
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
            entity_commands.insert({marker_name});
        }}''')

        # Generate Control types (skip base Control since it's already handled)
        for node_type in categories["control"]:
            if node_type == "Control":
                continue  # Skip base type
            marker_name = f"{node_type}Marker"
            cfg_attr = self.get_type_cfg_attribute(node_type)
            if cfg_attr:
                match_arms.append(f'''        {cfg_attr.strip()}
        "{node_type}" => {{
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
            entity_commands.insert({marker_name});
        }}''')
            else:
                match_arms.append(f'''        "{node_type}" => {{
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
            entity_commands.insert({marker_name});
        }}''')

        # Generate universal (direct Node) types (skip base Node, Node3D, and CanvasItem since already handled)
        for node_type in categories["universal"]:
            if node_type in ["Node", "CanvasItem", "Node3D"]:
                continue  # Skip base types
            marker_name = f"{node_type}Marker"
            cfg_attr = self.get_type_cfg_attribute(node_type)
            if cfg_attr:
                match_arms.append(f'''        {cfg_attr.strip()}
        "{node_type}" => {{
            entity_commands.insert({marker_name});
        }}''')
            else:
                match_arms.append(f'''        "{node_type}" => {{
            entity_commands.insert({marker_name});
        }}''')
        
        return '\n'.join(match_arms)

    def generate_gdscript_watcher(self, node_types, parent_map):
        """Generate the optimized GDScript scene tree watcher with all node types"""
        print("📜 Generating GDScript optimized scene tree watcher...")
        
        # Filter and categorize types
        valid_types = self.filter_valid_godot_classes(node_types)
        categories = self.categorize_types_by_hierarchy(valid_types, parent_map)
        
        content = f'''extends Node
class_name OptimizedSceneTreeWatcher

# 🤖 This file is automatically generated by scripts/generate_godot_types.py
# To regenerate: python scripts/generate_godot_types.py

# Optimized Scene Tree Watcher
# This GDScript class intercepts scene tree events and performs type analysis
# on the GDScript side to avoid expensive FFI calls from Rust.
# Handles {len(valid_types)} different Godot node types.

# Reference to the Rust SceneTreeWatcher
var rust_watcher: Node = null

func _ready():
	name = "OptimizedSceneTreeWatcher"
	
	# Auto-detect the Rust SceneTreeWatcher
	var bevy_app = get_node("/root/BevyAppSingleton")
	if bevy_app:
		rust_watcher = bevy_app.get_node("SceneTreeWatcher")
	
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
	
	# Analyze node type on GDScript side - this is much faster than FFI
	var node_type = _analyze_node_type(node)
	
	# Forward to Rust watcher with pre-analyzed type - this uses the MPSC sender
	if rust_watcher.has_method("scene_tree_event_typed"):
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
		
	rust_watcher.scene_tree_event(node, "NodeRenamed")

func _analyze_node_type(node: Node) -> String:
	"""
	Analyze node type hierarchy on GDScript side.
	Returns the most specific built-in Godot type name.
	This avoids multiple FFI calls that would be needed on the Rust side.
	Generated from Godot extension API to ensure completeness.
	"""
	
{self._generate_gdscript_type_analysis(categories)}
	
	# Default fallback
	return "Node"

{self._generate_initial_tree_analysis()}
'''
        
        with open(self.gdscript_watcher_file, "w") as f:
            f.write(content)
        
        print(f"✅ Generated GDScript watcher with {len(valid_types)} node types")

    def _generate_gdscript_type_analysis(self, categories):
        """Generate the GDScript node type analysis function"""
        lines = []
        
        # Node3D hierarchy (most common in 3D games)
        lines.append("\t# Check Node3D hierarchy first (most common in 3D games)")
        lines.append("\tif node is Node3D:")
        
        # Add common 3D types first for better performance
        common_3d = ["MeshInstance3D", "StaticBody3D", "RigidBody3D", "CharacterBody3D", "Area3D", 
                     "Camera3D", "DirectionalLight3D", "OmniLight3D", "SpotLight3D", "CollisionShape3D"]
        
        for node_type in common_3d:
            if node_type in categories["3d"]:
                lines.append(f"\t\tif node is {node_type}: return \"{node_type}\"")
        
        # Add remaining 3D types
        for node_type in sorted(categories["3d"]):
            if node_type not in common_3d:
                lines.append(f"\t\tif node is {node_type}: return \"{node_type}\"")
        
        lines.append("\t\treturn \"Node3D\"")
        lines.append("")
        
        # Node2D hierarchy (common in 2D games)
        lines.append("\t# Check Node2D hierarchy (common in 2D games)")
        lines.append("\telif node is Node2D:")
        
        # Add common 2D types first
        common_2d = ["Sprite2D", "StaticBody2D", "RigidBody2D", "CharacterBody2D", "Area2D", 
                     "Camera2D", "CollisionShape2D", "AnimatedSprite2D"]
        
        for node_type in common_2d:
            if node_type in categories["2d"]:
                lines.append(f"\t\tif node is {node_type}: return \"{node_type}\"")
        
        # Add remaining 2D types
        for node_type in sorted(categories["2d"]):
            if node_type not in common_2d:
                lines.append(f"\t\tif node is {node_type}: return \"{node_type}\"")
        
        lines.append("\t\treturn \"Node2D\"")
        lines.append("")
        
        # Control hierarchy (UI elements)
        lines.append("\t# Check Control hierarchy (UI elements)")
        lines.append("\telif node is Control:")
        
        # Add common UI types first
        common_control = ["Button", "Label", "Panel", "VBoxContainer", "HBoxContainer", 
                         "MarginContainer", "ColorRect", "LineEdit", "TextEdit", "CheckBox"]
        
        for node_type in common_control:
            if node_type in categories["control"]:
                lines.append(f"\t\tif node is {node_type}: return \"{node_type}\"")
        
        # Add remaining Control types
        for node_type in sorted(categories["control"]):
            if node_type not in common_control:
                lines.append(f"\t\tif node is {node_type}: return \"{node_type}\"")
        
        lines.append("\t\treturn \"Control\"")
        lines.append("")
        
        # Universal types (direct Node children)
        lines.append("\t# Check other common node types that inherit directly from Node")
        common_universal = ["AnimationPlayer", "Timer", "AudioStreamPlayer", "HTTPRequest", "CanvasLayer"]
        
        for node_type in common_universal:
            if node_type in categories["universal"]:
                lines.append(f"\telif node is {node_type}: return \"{node_type}\"")
        
        # Add remaining universal types  
        for node_type in sorted(categories["universal"]):
            if node_type not in common_universal:
                lines.append(f"\telif node is {node_type}: return \"{node_type}\"")
        
        return '\n'.join(lines)

    def _generate_initial_tree_analysis(self):
        """Generate method for analyzing the initial scene tree with type info"""
        return '''func analyze_initial_tree() -> Dictionary:
	"""
	Analyze the entire initial scene tree and return node information with types.
	Returns a Dictionary with PackedArrays for maximum performance:
	{
		"instance_ids": PackedInt64Array,
		"node_types": PackedStringArray
	}
	Used for optimized initial scene tree setup.
	"""
	var instance_ids = PackedInt64Array()
	var node_types = PackedStringArray()
	var root = get_tree().get_root()
	if root:
		_analyze_node_recursive(root, instance_ids, node_types)
	
	return {
		"instance_ids": instance_ids,
		"node_types": node_types
	}

func _analyze_node_recursive(node: Node, instance_ids: PackedInt64Array, node_types: PackedStringArray):
	"""Recursively analyze nodes and collect type information into PackedArrays"""
	# Check if node is still valid before processing
	if not is_instance_valid(node):
		return
	
	# Add this node's information with pre-analyzed type
	var instance_id = node.get_instance_id()
	var node_type = _analyze_node_type(node)
	
	# Only append if we have valid data
	if instance_id != 0 and node_type != "":
		instance_ids.append(instance_id)
		node_types.append(node_type)
	
	# Recursively process children
	for child in node.get_children():
		_analyze_node_recursive(child, instance_ids, node_types)'''

    def _generate_hierarchy_function_comprehensive(self, name, types):
        """Generate a hierarchy-specific type checking function"""
        content = f'''fn check_{name}_node_types_comprehensive(
    entity_commands: &mut EntityCommands,
    node: &mut GodotNodeHandle,
) {{
'''

        for node_type in sorted(types):
            rust_class_name = self.fix_godot_class_name_for_rust(node_type)
            cfg_attr = self.get_type_cfg_attribute(node_type)
            if cfg_attr:
                content += f'''    {cfg_attr.strip()}
    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
        entity_commands.insert({node_type}Marker);
    }}
'''
            else:
                content += f'''    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
        entity_commands.insert({node_type}Marker);
    }}
'''

        content += "}\n\n"

        content += f'''fn remove_{name}_node_types_comprehensive(
    entity_commands: &mut EntityCommands,
    _node: &mut GodotNodeHandle,
) {{
    entity_commands
'''

        # Separate regular and version-gated types
        regular_types = []
        gated_types = {}

        for node_type in sorted(types):
            cfg_attr = self.get_type_cfg_attribute(node_type)
            if cfg_attr:
                version = cfg_attr.strip()
                if version not in gated_types:
                    gated_types[version] = []
                gated_types[version].append(node_type)
            else:
                regular_types.append(node_type)

        # Generate regular removes in a chain
        for node_type in regular_types:
            content += f'''        .remove::<{node_type}Marker>()
'''

        # Close the chain with semicolon
        content += ";\n"

        # Generate version-gated removes separately
        for version, types_list in gated_types.items():
            content += f"\n    {version}\n"
            content += "    entity_commands\n"
            for node_type in types_list:
                content += f'''        .remove::<{node_type}Marker>()
'''
            content += ";\n"

        content += "}\n\n"
        return content

    def _generate_universal_function_comprehensive(self, types):
        """Generate the universal types checking function"""
        content = '''fn check_universal_node_types_comprehensive(
    entity_commands: &mut EntityCommands,
    node: &mut GodotNodeHandle,
) {
'''

        for node_type in sorted(types):
            rust_class_name = self.fix_godot_class_name_for_rust(node_type)
            cfg_attr = self.get_type_cfg_attribute(node_type)
            if cfg_attr:
                content += f'''    {cfg_attr.strip()}
    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
        entity_commands.insert({node_type}Marker);
    }}
'''
            else:
                content += f'''    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
        entity_commands.insert({node_type}Marker);
    }}
'''

        content += "}\n"

        content += '''fn remove_universal_node_types_comprehensive(
    entity_commands: &mut EntityCommands,
    _node: &mut GodotNodeHandle,
) {
    entity_commands
'''

        # Separate regular and version-gated types
        regular_types = []
        gated_types = {}

        for node_type in sorted(types):
            cfg_attr = self.get_type_cfg_attribute(node_type)
            if cfg_attr:
                version = cfg_attr.strip()
                if version not in gated_types:
                    gated_types[version] = []
                gated_types[version].append(node_type)
            else:
                regular_types.append(node_type)

        # Generate regular removes in a chain
        for node_type in regular_types:
            content += f'''        .remove::<{node_type}Marker>()
'''

        # Close the chain with semicolon
        content += ";\n"

        # Generate version-gated removes separately
        for version, types_list in gated_types.items():
            content += f"\n    {version}\n"
            content += "    entity_commands\n"
            for node_type in types_list:
                content += f'''        .remove::<{node_type}Marker>()
'''
            content += ";\n"

        content += "}\n"
        return content

    def verify_plugin_integration(self):
        """Verify that the plugin is set up to use the generated code"""
        print("🔍 Verifying plugin integration...")

        with open(self.plugin_file, "r") as f:
            content = f.read()

        if "add_comprehensive_node_type_markers" in content:
            print("✅ Plugin is correctly integrated with generated code")
        else:
            print("⚠️  Plugin integration needed:")
            print("   1. Add: use super::node_type_checking_generated::add_comprehensive_node_type_markers;")
            print("   2. Replace add_node_type_markers calls with add_comprehensive_node_type_markers")
            print("   3. This is a one-time setup - future script runs won't need this")

    def run(self):
        """Run the complete generation pipeline"""
        print("🎯 Starting Godot type generation pipeline...")

        try:
            # Step 1: Generate extension API
            self.run_godot_dump_api()

            # Step 2: Parse API and extract types
            node_types, parent_map = self.load_and_parse_extension_api()

            # Step 3: Generate node markers
            self.generate_node_markers(node_types)

            # Step 4: Generate type checking code
            self.generate_type_checking_code(node_types, parent_map)

            # Step 5: Generate optimized GDScript watcher
            self.generate_gdscript_watcher(node_types, parent_map)

            # Step 6: Verify plugin integration
            self.verify_plugin_integration()

            print(f"""
🎉 Generation complete!

Generated:
  • {len(node_types)} node marker components
  • Complete type checking functions
  • Optimized GDScript scene tree watcher

Files generated:
  • {self.node_markers_file.relative_to(self.project_root)}
  • {self.type_checking_file.relative_to(self.project_root)}
  • {self.gdscript_watcher_file.relative_to(self.project_root)}

Next steps:
  • Run 'cargo check' to verify the build
  • Commit the generated files
""")

        except Exception as e:
            print(f"❌ Generation failed: {e}")
            sys.exit(1)

def main():
    generator = GodotTypeGenerator()
    generator.run()

if __name__ == "__main__":
    main()
