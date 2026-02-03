#!/usr/bin/env python3
"""
Fully automatic Godot type generation for godot-bevy.

This script:
1. Runs `godot --dump-extension-api-with-docs` to generate extension_api.json
2. Parses all Node-derived types from the API
3. Generates comprehensive node marker components
4. Generates complete type checking functions
5. Updates the scene tree plugin to use generated code

Usage: python scripts/generate_godot_types.py
"""

import json
import subprocess
import sys
import textwrap
from collections import defaultdict
from pathlib import Path
from typing import Any


def run_cargo_fmt(file_path, project_root):
    """Run cargo fmt on a specific file to format the generated Rust code"""
    try:
        # Run cargo fmt on the specific file
        result = subprocess.run(
            ["cargo", "fmt", "--", str(file_path)],
            cwd=project_root,
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode == 0:
            print(f"  ✓ Formatted {file_path.name}")
        else:
            print(f"  ⚠ cargo fmt warning for {file_path.name}: {result.stderr}")

    except FileNotFoundError:
        print(f"  ⚠ cargo fmt not found - skipping formatting for {file_path.name}")
    except subprocess.TimeoutExpired:
        print(f"  ⚠ cargo fmt timed out for {file_path.name}")
    except Exception as e:
        print(f"  ⚠ Could not format {file_path.name}: {e}")


def run_godot_dump_api(api_file):
    """Run godot --dump-extension-api-with-docs to generate extension_api.json"""
    print("🚀 Generating extension_api.json from Godot...")

    try:
        # Try different common Godot executable names
        godot_commands = ["godot", "godot4", "/usr/local/bin/godot"]

        for cmd in godot_commands:
            try:
                result = subprocess.run(
                    [
                        cmd,
                        "--headless",
                        "--dump-extension-api-with-docs",
                        str(api_file),
                    ],
                    capture_output=True,
                    text=True,
                    timeout=30,
                )

                if result.returncode == 0 and api_file.exists():
                    print(f"✅ Successfully generated extension_api.json using '{cmd}'")
                    return

            except (subprocess.TimeoutExpired, FileNotFoundError):
                continue

        # If all commands failed, give helpful error
        raise RuntimeError(
            "Could not run Godot to generate extension_api.json.\n"
            "Please ensure Godot 4 is installed and available in PATH.\n"
            "You can also manually run: godot --dump-extension-api-with-docs extension_api.json"
        )

    except Exception as e:
        print(f"❌ Error generating extension_api.json: {e}")
        sys.exit(1)


def _generate_initial_tree_analysis():
    """Generate method for analyzing the initial scene tree with type info"""
    return textwrap.dedent('''
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
        ''')


def fix_godot_class_name_for_rust(class_name):
    """Fix Godot class names to match the actual Rust bindings"""
    # Map class names from extension API to actual Rust struct names
    name_fixes = {
        "CPUParticles2D": "CpuParticles2D",
        "CPUParticles3D": "CpuParticles3D",
        "GPUParticles2D": "GpuParticles2D",
        "GPUParticles3D": "GpuParticles3D",
        "GPUParticlesAttractor3D": "GpuParticlesAttractor3D",
        "GPUParticlesAttractorBox3D": "GpuParticlesAttractorBox3D",
        "GPUParticlesAttractorSphere3D": "GpuParticlesAttractorSphere3D",
        "GPUParticlesAttractorVectorField3D": "GpuParticlesAttractorVectorField3D",
        "GPUParticlesCollision3D": "GpuParticlesCollision3D",
        "GPUParticlesCollisionBox3D": "GpuParticlesCollisionBox3D",
        "GPUParticlesCollisionHeightField3D": "GpuParticlesCollisionHeightField3D",
        "GPUParticlesCollisionSDF3D": "GpuParticlesCollisionSdf3d",
        "GPUParticlesCollisionSphere3D": "GpuParticlesCollisionSphere3D",
        "HTTPRequest": "HttpRequest",
        "SkeletonIK3D": "SkeletonIk3d",
        "Generic6DOFJoint3D": "Generic6DofJoint3D",
        "OpenXRRenderModel": "OpenXrRenderModel",
        "OpenXRRenderModelManager": "OpenXrRenderModelManager",
    }

    return name_fixes.get(class_name, class_name)


def signal_name_to_const(signal_name):
    """Convert a signal name to UPPER_SNAKE_CASE constant name"""
    import re

    # Handle empty or invalid names
    if not signal_name:
        return "SIGNAL"

    # Insert underscores before uppercase letters (for camelCase/PascalCase)
    result = re.sub("([a-z0-9])([A-Z])", r"\1_\2", signal_name)

    # Replace non-alphanumeric characters with underscores
    result = re.sub(r"[^a-zA-Z0-9_]", "_", result)

    # Convert to uppercase
    result = result.upper()

    # Collapse multiple underscores
    result = re.sub(r"_+", "_", result)

    # Strip leading/trailing underscores
    result = result.strip("_")

    # Ensure it doesn't start with a digit (prepend underscore if needed)
    if result and result[0].isdigit():
        result = "_" + result

    # Fallback if empty after processing
    if not result:
        result = "SIGNAL"

    return result


def _generate_gdscript_type_analysis(categories):
    """Generate the GDScript node type analysis function"""
    # Node3D hierarchy (most common in 3D games)
    lines = [
        "\t# Check Node3D hierarchy first (most common in 3D games)",
        "\tif node is Node3D:",
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
            lines.append(f'\t\tif node is {node_type}: return "{node_type}"')

    # Add remaining 3D types
    for node_type in sorted(categories["3d"]):
        if node_type not in common_3d:
            lines.append(f'\t\tif node is {node_type}: return "{node_type}"')

    lines.append('\t\treturn "Node3D"')
    lines.append("")

    # Node2D hierarchy (common in 2D games)
    lines.append("\t# Check Node2D hierarchy (common in 2D games)")
    lines.append("\telif node is Node2D:")

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
            lines.append(f'\t\tif node is {node_type}: return "{node_type}"')

    # Add remaining 2D types
    for node_type in sorted(categories["2d"]):
        if node_type not in common_2d:
            lines.append(f'\t\tif node is {node_type}: return "{node_type}"')

    lines.append('\t\treturn "Node2D"')
    lines.append("")

    # Control hierarchy (UI elements)
    lines.append("\t# Check Control hierarchy (UI elements)")
    lines.append("\telif node is Control:")

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
            lines.append(f'\t\tif node is {node_type}: return "{node_type}"')

    # Add remaining Control types
    for node_type in sorted(categories["control"]):
        if node_type not in common_control:
            lines.append(f'\t\tif node is {node_type}: return "{node_type}"')

    lines.append('\t\treturn "Control"')
    lines.append("")

    # Universal types (direct Node children)
    lines.append("\t# Check other common node types that inherit directly from Node")
    common_universal = [
        "AnimationPlayer",
        "Timer",
        "AudioStreamPlayer",
        "HTTPRequest",
        "CanvasLayer",
    ]

    for node_type in common_universal:
        if node_type in categories["universal"]:
            lines.append(f'\telif node is {node_type}: return "{node_type}"')

    # Add remaining universal types
    for node_type in sorted(categories["universal"]):
        if node_type not in common_universal:
            lines.append(f'\telif node is {node_type}: return "{node_type}"')

    return "\n".join(lines)


def bbcode_to_markdown(text):
    """Convert Godot BBCode format to Rustdoc-compatible Markdown"""
    import re
    from textwrap import dedent

    # Basic inline formatting
    text = text.replace("[b]", "**").replace("[/b]", "**")
    text = text.replace("[i]", "*").replace("[/i]", "*")
    text = text.replace("[code]", "`").replace("[/code]", "`")

    # [member something] -> `something`
    text = re.sub(r"\[member\s+([^]]+)]", r"`\1`", text)

    # [param something] -> `something`
    text = re.sub(r"\[param\s+([^]]+)]", r"`\1`", text)

    # [constant something] -> `something`
    text = re.sub(r"\[constant\s+([^]]+)]", r"`\1`", text)

    # [method something] -> `something()`
    text = re.sub(r"\[method\s+([^]]+)]", r"`\1()`", text)

    # [signal something] -> `something`
    text = re.sub(r"\[signal\s+([^]]+)]", r"`\1`", text)

    # [enum something] -> `something`
    text = re.sub(r"\[enum\s+([^]]+)]", r"`\1`", text)

    # [url=...]...[/url] -> [link text](url)
    text = re.sub(r"\[url=([^]]+)]([^\[]+)\[/url]", r"[\2](\1)", text)

    # [codeblock]...[/codeblock] -> ```text\n...\n```
    def codeblock_repl(m):
        code = m.group(1).strip()
        # Dedent the code block
        code = dedent(code)
        return f"\n```text\n{code}\n```\n"

    text = re.sub(r"\[codeblock](.*?)\[/codeblock]", codeblock_repl, text, flags=re.S)

    # [codeblocks] (with language specified)
    def codeblocks_repl(m):
        code = m.group(1).strip()
        code = dedent(code)
        return f"\n```gdscript\n{code}\n```\n"

    text = re.sub(
        r"\[codeblocks](.*?)\[/codeblocks]", codeblocks_repl, text, flags=re.S
    )

    # Remove any remaining BBCode-style tags that we didn't handle
    text = re.sub(r"\[/?[a-zA-Z0-9_]+]", "", text)

    return text


def sanitize_doc_comment(text):
    """Sanitize text to be safe for Rustdoc /// comments"""
    # The main concern is preventing */ or */ sequences that could escape the comment
    # Also handle other problematic sequences

    # Replace tabs with 4 spaces for consistent formatting
    text = text.replace("\t", "    ")

    # Replace */ with *\/ to prevent closing block comments
    text = text.replace("*/", r"*\/")

    # Replace leading /// with \/\/\/ to prevent nested doc comments
    text = text.replace("///", r"\/\/\/")

    # Ensure we don't have unclosed backticks that would break markdown
    # Count backticks and add one if odd
    backtick_count = text.count("`")
    if backtick_count % 2 != 0:
        text += "`"

    return text


def categorize_types_by_hierarchy(node_types, parent_map):
    """Categorize node types by their inheritance hierarchy"""

    def is_descendant_of(ancestor_node_type, ancestor):
        current = ancestor_node_type
        while current in parent_map:
            current = parent_map[current]
            if current == ancestor:
                return True
        return False

    categories = {"3d": [], "2d": [], "control": [], "universal": []}

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


def get_type_cfg_attribute(
    wasm_excluded_types: set[str],
    version_gated_types: dict[str, list[str]],
    node_type: str,
) -> str:
    """Get the cfg attribute for a type if it needs version or feature gating."""
    # Check for WASM-excluded types first
    if node_type in wasm_excluded_types:
        return '#[cfg(not(feature = "experimental-wasm"))]\n'
    # Check for version-gated types
    for version, types in version_gated_types.items():
        if node_type in types:
            return f'#[cfg(feature = "api-{version}")]\n'
    return ""


def generate_signal_names(
    classes_by_name: dict[Any, Any],
    excluded_classes: set[str | Any],
    wasm_excluded_types: set[str],
    version_gated_types: dict[str, list[str]],
    signal_names_file: Path,
    project_root: Path,
):
    """Generate the signal_names.rs file with signal constants"""
    print("📡 Generating signal names...")

    content = textwrap.dedent("""\
        #![allow(dead_code)]
        //! 🤖 This file is automatically generated by scripts/generate_godot_types.py
        //! To regenerate: python scripts/generate_godot_types.py
        //!
        //! Signal name constants for Godot classes.
        //! These provide convenient, discoverable signal names for connecting to Godot signals.
        //!
        //! Example usage:
        //! ```ignore
        //! use godot_bevy::interop::signal_names::ButtonSignals;
        //! // Connect to the "pressed" signal
        //! button.connect(ButtonSignals::PRESSED.into(), callable);
        //! ```
        
        """)

    # Collect all classes with signals, sorted by name, skipping excluded classes
    classes_with_signals = []
    for class_name in sorted(classes_by_name.keys()):
        # Skip classes that we know are not available / problematic
        if class_name in excluded_classes:
            continue

        class_info = classes_by_name[class_name]
        signals = class_info.get("signals", [])
        if signals:
            classes_with_signals.append((class_name, class_info, signals))

    signal_count = 0

    # Generate a dedicated *Signals struct and impl block for each class
    for class_name, class_info, signals in classes_with_signals:
        rust_class_name = fix_godot_class_name_for_rust(class_name)
        signals_struct_name = f"{rust_class_name}Signals"

        # Optional: cfg-gate the whole struct/impl if the class is version-gated
        cfg_attr = get_type_cfg_attribute(
            wasm_excluded_types, version_gated_types, class_name
        )
        if cfg_attr:
            content += cfg_attr

        # Struct declaration
        content += f"/// Signal constants for `{rust_class_name}`\n"
        content += f"pub struct {signals_struct_name};\n\n"

        # Impl block
        if cfg_attr:
            content += cfg_attr
        content += f"impl {signals_struct_name} {{\n"

        # Generate constants for each signal
        for signal in signals:
            signal_name = signal["name"]
            description = signal.get("description", "").strip()
            const_name = signal_name_to_const(signal_name)

            # Add doc comment with description if available
            if description:
                # Convert BBCode to Markdown
                description = bbcode_to_markdown(description)
                # Sanitize to prevent escaping doc comments
                description = sanitize_doc_comment(description)

                # Format description for Rust doc comments
                description_lines = description.replace("\r\n", "\n").split("\n")
                for line in description_lines:
                    # Strip trailing whitespace but preserve empty lines
                    line = line.rstrip()
                    content += f"    /// {line}\n"
            else:
                # Fallback: just mention the signal name
                content += f"    /// Signal `{signal_name}`\n"

            # Constant definition
            content += (
                f'    pub const {const_name}: &\'static str = "{signal_name}";\n\n'
            )
            signal_count += 1

        # Close impl block
        content += "}\n\n"

    # Write the file
    with open(signal_names_file, "w") as f:
        f.write(content)

    print(
        f"✅ Generated {signal_count} signal constants across {len(classes_with_signals)} classes"
    )
    run_cargo_fmt(signal_names_file, project_root)


def _generate_hierarchy_function_comprehensive(
    wasm_excluded_types: set[str],
    version_gated_types: dict[str, list[str]],
    name,
    types,
):
    """Generate a hierarchy-specific type checking function"""
    content = textwrap.dedent(f"""\
        fn check_{name}_node_types_comprehensive(
            entity_commands: &mut EntityCommands,
            node: &mut GodotNode,
        ) {{
        """)

    for node_type in sorted(types):
        rust_class_name = fix_godot_class_name_for_rust(node_type)
        cfg_attr = get_type_cfg_attribute(
            wasm_excluded_types, version_gated_types, node_type
        )
        if cfg_attr:
            content += textwrap.dedent(f"""\
                        {cfg_attr.strip()}
                    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
                        entity_commands.insert({node_type}Marker);
                    }}
                """)
        else:
            content += textwrap.dedent(f"""\
                    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
                        entity_commands.insert({node_type}Marker);
                    }}
                """)

    content += "}\n\n"

    content += textwrap.dedent(f"""\
        fn remove_{name}_node_types_comprehensive(
            entity_commands: &mut EntityCommands,
            _node: &mut GodotNode,
        ) {{
            entity_commands
        """)

    # Separate regular and version-gated types
    regular_types = []
    gated_types = {}

    for node_type in sorted(types):
        cfg_attr = get_type_cfg_attribute(
            wasm_excluded_types, version_gated_types, node_type
        )
        if cfg_attr:
            version = cfg_attr.strip()
            if version not in gated_types:
                gated_types[version] = []
            gated_types[version].append(node_type)
        else:
            regular_types.append(node_type)

    # Generate regular removes in a chain
    for node_type in regular_types:
        content += f"        .remove::<{node_type}Marker>()\n"

    # Close the chain with semicolon
    content += ";\n"

    # Generate version-gated removes separately
    for version, types_list in gated_types.items():
        content += f"\n    {version}\n"
        content += "    entity_commands\n"
        for node_type in types_list:
            content += f"        .remove::<{node_type}Marker>()\n"
        content += ";\n"

    content += "}\n\n"
    return content


def filter_valid_godot_classes(excluded_classes: set[str | Any], node_types):
    """Filter out Godot classes that don't exist or aren't available"""
    # Use the shared excluded_classes set defined in __init__
    return [t for t in node_types if t not in excluded_classes]


def _generate_string_match_arms(
    wasm_excluded_types: set[str], version_gated_types: dict[str, list[str]], categories
):
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
        cfg_attr = get_type_cfg_attribute(
            wasm_excluded_types, version_gated_types, node_type
        )
        if cfg_attr:
            match_arms.append(f"""        {cfg_attr.strip()}
    "{node_type}" => {{
        entity_commands.insert(Node3DMarker);
        entity_commands.insert({marker_name});
    }}""")
        else:
            match_arms.append(f"""        "{node_type}" => {{
        entity_commands.insert(Node3DMarker);
        entity_commands.insert({marker_name});
    }}""")

    # Generate Node2D types (skip base Node2D since it's already handled)
    for node_type in categories["2d"]:
        if node_type == "Node2D":
            continue  # Skip base type
        marker_name = f"{node_type}Marker"
        cfg_attr = get_type_cfg_attribute(
            wasm_excluded_types, version_gated_types, node_type
        )
        if cfg_attr:
            match_arms.append(f"""        {cfg_attr.strip()}
    "{node_type}" => {{
        entity_commands.insert(Node2DMarker);
        entity_commands.insert(CanvasItemMarker);
        entity_commands.insert({marker_name});
    }}""")
        else:
            match_arms.append(f"""        "{node_type}" => {{
        entity_commands.insert(Node2DMarker);
        entity_commands.insert(CanvasItemMarker);
        entity_commands.insert({marker_name});
    }}""")

    # Generate Control types (skip base Control since it's already handled)
    for node_type in categories["control"]:
        if node_type == "Control":
            continue  # Skip base type
        marker_name = f"{node_type}Marker"
        cfg_attr = get_type_cfg_attribute(
            wasm_excluded_types, version_gated_types, node_type
        )
        if cfg_attr:
            match_arms.append(f"""        {cfg_attr.strip()}
    "{node_type}" => {{
        entity_commands.insert(ControlMarker);
        entity_commands.insert(CanvasItemMarker);
        entity_commands.insert({marker_name});
    }}""")
        else:
            match_arms.append(f"""        "{node_type}" => {{
        entity_commands.insert(ControlMarker);
        entity_commands.insert(CanvasItemMarker);
        entity_commands.insert({marker_name});
    }}""")

    # Generate universal (direct Node) types (skip base Node, Node3D, and CanvasItem since already handled)
    for node_type in categories["universal"]:
        if node_type in ["Node", "CanvasItem", "Node3D"]:
            continue  # Skip base types
        marker_name = f"{node_type}Marker"
        cfg_attr = get_type_cfg_attribute(
            wasm_excluded_types, version_gated_types, node_type
        )
        if cfg_attr:
            match_arms.append(f"""        {cfg_attr.strip()}
    "{node_type}" => {{
        entity_commands.insert({marker_name});
    }}""")
        else:
            match_arms.append(f"""        "{node_type}" => {{
        entity_commands.insert({marker_name});
    }}""")

    return "\n".join(match_arms)


def _generate_universal_function_comprehensive(
    wasm_excluded_types: set[str], version_gated_types: dict[str, list[str]], types
):
    """Generate the universal types checking function"""
    content = textwrap.dedent("""\
        fn check_universal_node_types_comprehensive(
            entity_commands: &mut EntityCommands,
            node: &mut GodotNode,
        ) {
        """)

    for node_type in sorted(types):
        rust_class_name = fix_godot_class_name_for_rust(node_type)
        cfg_attr = get_type_cfg_attribute(
            wasm_excluded_types, version_gated_types, node_type
        )
        if cfg_attr:
            content += textwrap.dedent(f"""\
                {cfg_attr.strip()}
                    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
                        entity_commands.insert({node_type}Marker);
                    }}
                """)
        else:
            content += textwrap.dedent(f"""\
                    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
                        entity_commands.insert({node_type}Marker);
                    }}
                """)
    content += "}\n"

    content += textwrap.dedent("""\
        fn remove_universal_node_types_comprehensive(
            entity_commands: &mut EntityCommands,
            _node: &mut GodotNode,
        ) {
            entity_commands
        """)
    # Separate regular and version-gated types
    regular_types = []
    gated_types = {}

    for node_type in sorted(types):
        cfg_attr = get_type_cfg_attribute(
            wasm_excluded_types, version_gated_types, node_type
        )
        if cfg_attr:
            version = cfg_attr.strip()
            if version not in gated_types:
                gated_types[version] = []
            gated_types[version].append(node_type)
        else:
            regular_types.append(node_type)

    # Generate regular removes in a chain
    for node_type in regular_types:
        content += f"        .remove::<{node_type}Marker>()\n"

    # Close the chain with semicolon
    content += ";\n"

    # Generate version-gated removes separately
    for version, types_list in gated_types.items():
        content += f"\n    {version}\n"
        content += "    entity_commands\n"
        for node_type in types_list:
            content += f"        .remove::<{node_type}Marker>()\n"
        content += ";\n"

    content += "}\n"
    return content


def generate_type_checking_code(
    excluded_classes: set[str],
    wasm_excluded_types: set[str],
    version_gated_types: dict[str, list[str]],
    type_checking_file: Path,
    project_root: Path,
    node_types,
    parent_map,
):
    """Generate the complete type checking implementation"""
    print("🔍 Generating type checking code...")

    # Filter out invalid Godot classes first to avoid unnecessary work
    valid_types = filter_valid_godot_classes(excluded_classes, node_types)

    # Categorize only the valid types
    categories = categorize_types_by_hierarchy(valid_types, parent_map)

    content = textwrap.dedent(f"""\
        // 🤖 This file is automatically generated by scripts/generate_godot_types.py
        // To regenerate: python scripts/generate_godot_types.py
        
        use bevy_ecs::system::EntityCommands;
        use crate::interop::{{GodotNode, node_markers::*}};
        
        /// Adds appropriate marker components to an entity based on the Godot node type.
        /// This function is automatically generated and handles all {len(valid_types)} Godot node types.
        ///
        /// Godot's hierarchy: Node -> {{Node3D, CanvasItem -> {{Node2D, Control}}, Others}}
        /// We check the major branches: 3D, 2D, Control (UI), and Universal (direct Node children)
        pub fn add_comprehensive_node_type_markers(
            entity_commands: &mut EntityCommands,
            node: &mut GodotNode,
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
        /// GodotNode::try_get calls for each node.
        pub fn add_node_type_markers_from_string(
            entity_commands: &mut EntityCommands,
            node_type: &str,
        ) {{
            // All nodes inherit from Node
            entity_commands.insert(NodeMarker);
        
            // Add appropriate markers based on the type string
            match node_type {{
        {_generate_string_match_arms(wasm_excluded_types, version_gated_types, categories)}
                // For any unrecognized type, we already have NodeMarker
                // This handles custom user types that extend Godot nodes
                _ => {{}}
            }}
        }}
        
        pub fn remove_comprehensive_node_type_markers(
            entity_commands: &mut EntityCommands,
            node: &mut GodotNode,
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
        
        """)

    # Generate specific checking functions
    content += _generate_hierarchy_function_comprehensive(
        wasm_excluded_types, version_gated_types, "3d", categories["3d"]
    )
    content += _generate_hierarchy_function_comprehensive(
        wasm_excluded_types, version_gated_types, "2d", categories["2d"]
    )
    content += _generate_hierarchy_function_comprehensive(
        wasm_excluded_types,
        version_gated_types,
        "control",
        categories["control"],
    )
    content += _generate_universal_function_comprehensive(
        wasm_excluded_types, version_gated_types, categories["universal"]
    )

    with open(type_checking_file, "w") as f:
        f.write(content)

    print(f"✅ Generated type checking for {len(valid_types)} types")
    run_cargo_fmt(type_checking_file, project_root)


def generate_gdscript_watcher(
    excluded_classes: set[str], gdscript_watcher_file: Path, node_types, parent_map
):
    """Generate the optimized GDScript scene tree watcher with all node types"""
    print("📜 Generating GDScript optimized scene tree watcher...")

    # Filter and categorize types
    valid_types = filter_valid_godot_classes(excluded_classes, node_types)
    categories = categorize_types_by_hierarchy(valid_types, parent_map)

    content = textwrap.dedent(f'''\
        extends Node
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
        
        {_generate_gdscript_type_analysis(categories)}
        
            # Default fallback
            return "Node"
        
        {_generate_initial_tree_analysis()}
        ''')

    with open(gdscript_watcher_file, "w") as f:
        f.write(content)

    print(f"✅ Generated GDScript watcher with {len(valid_types)} node types")


def verify_plugin_integration(plugin_file: Path):
    """Verify that the plugin is set up to use the generated code"""
    print("🔍 Verifying plugin integration...")

    with open(plugin_file, "r") as f:
        content = f.read()

    if "add_comprehensive_node_type_markers" in content:
        print("✅ Plugin is correctly integrated with generated code")
    else:
        print("⚠️  Plugin integration needed:")
        print(
            "   1. Add: use super::node_type_checking_generated::add_comprehensive_node_type_markers;"
        )
        print(
            "   2. Replace add_node_type_markers calls with add_comprehensive_node_type_markers"
        )
        print("   3. This is a one-time setup - future script runs won't need this")


class GodotTypeGenerator:
    def __init__(self):
        self.project_root = Path(__file__).parent.parent
        self.api_file = self.project_root / "extension_api.json"
        self.node_markers_file = (
            self.project_root / "godot-bevy" / "src" / "interop" / "node_markers.rs"
        )
        self.signal_names_file = (
            self.project_root / "godot-bevy" / "src" / "interop" / "signal_names.rs"
        )
        self.type_checking_file = (
            self.project_root
            / "godot-bevy"
            / "src"
            / "plugins"
            / "scene_tree"
            / "node_type_checking_generated.rs"
        )
        self.plugin_file = (
            self.project_root
            / "godot-bevy"
            / "src"
            / "plugins"
            / "scene_tree"
            / "plugin.rs"
        )
        self.gdscript_watcher_file = (
            self.project_root
            / "addons"
            / "godot-bevy"
            / "optimized_scene_tree_watcher.gd"
        )
        self.signal_names_file = (
            self.project_root / "godot-bevy" / "src" / "interop" / "signal_names.rs"
        )

        # Store all classes by name for signal generation
        self.classes_by_name = {}

        # Known classes that don't exist in current Godot version or aren't available
        # Used for filtering both node types and signal generation
        self.excluded_classes = {
            # CSG classes (require special module)
            "CSGBox3D",
            "CSGCombiner3D",
            "CSGCylinder3D",
            "CSGMesh3D",
            "CSGPolygon3D",
            "CSGPrimitive3D",
            "CSGShape3D",
            "CSGSphere3D",
            "CSGTorus3D",
            # Editor classes
            "GridMapEditorPlugin",
            "ScriptCreateDialog",
            "FileSystemDock",
            "OpenXRBindingModifierEditor",
            "OpenXRInteractionProfileEditor",
            "OpenXRInteractionProfileEditorBase",
            # XR classes that might not be available
            "XRAnchor3D",
            "XRBodyModifier3D",
            "XRCamera3D",
            "XRController3D",
            "XRFaceModifier3D",
            "XRHandModifier3D",
            "XRNode3D",
            "XROrigin3D",
            # OpenXR classes
            "OpenXRCompositionLayer",
            "OpenXRCompositionLayerCylinder",
            "OpenXRCompositionLayerEquirect",
            "OpenXRCompositionLayerQuad",
            "OpenXRHand",
            "OpenXRVisibilityMask",
            # Classes that might not be available in all builds
            "VoxelGI",
            "LightmapGI",
            "FogVolume",
            "WorldEnvironment",
            # Navigation classes (might be module-specific)
            "NavigationAgent2D",
            "NavigationAgent3D",
            "NavigationLink2D",
            "NavigationLink3D",
            "NavigationObstacle2D",
            "NavigationObstacle3D",
            "NavigationRegion2D",
            "NavigationRegion3D",
            # Other problematic classes
            "StatusIndicator",
            # Graph classes (not available in all Godot builds)
            "GraphEdit",
            "GraphElement",
            "GraphFrame",
            "GraphNode",
            # Parallax2D is in extension API but not in current Rust bindings
            "Parallax2D",
        }

        # Types that require specific Godot API versions
        # Based on Godot release notes and documentation
        self.version_gated_types = {
            "4-3": [  # Types added in Godot 4.3+
                "TileMapLayer",  # Replaces old TileMap layers system
                "AnimationMixer",  # Base class for animation (introduced 4.2, enhanced 4.3)
                "AudioStreamInteractive",  # Interactive music support
                "AudioStreamPlaylist",  # Playlist support
                "AudioStreamSynchronized",  # Synchronized audio streams
            ],
            "4-4": [  # Types added in Godot 4.4+
                "LookAtModifier3D",  # New 3D animation modifier
                "RetargetModifier3D",  # Animation retargeting
                "SpringBoneSimulator3D",  # Physics-based animation
                "SpringBoneCollision3D",  # Spring bone collision base
                "SpringBoneCollisionCapsule3D",  # Capsule collision for spring bones
                "SpringBoneCollisionPlane3D",  # Plane collision for spring bones
                "SpringBoneCollisionSphere3D",  # Sphere collision for spring bones
            ],
            # Note: Godot 4.5 didn't add significant new node types
        }

        # Types that are excluded when building for web/WASM
        # These types don't exist in the web extension API
        self.wasm_excluded_types = {
            "OpenXRRenderModel",
            "OpenXRRenderModelManager",
        }

    def load_and_parse_extension_api(self, api_file: Path):
        """Load and parse the extension API to extract node types"""
        print("📖 Parsing extension API...")

        if not api_file.exists():
            raise FileNotFoundError(f"extension_api.json not found at {api_file}")

        with open(api_file) as f:
            api = json.load(f)

        # Store all classes by name for signal generation
        self.classes_by_name = {
            class_info["name"]: class_info for class_info in api["classes"]
        }

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

        filtered_types = sorted(
            [
                t
                for t in node_types
                if not any(t.startswith(prefix) for prefix in excluded_prefixes)
                and t not in excluded_types
            ]
        )

        print(f"✅ Found {len(filtered_types)} node types")
        return filtered_types, parent_map

    def generate_node_markers(self, node_types):
        """Generate the node_markers.rs file"""
        print("🏷️  Generating node markers...")

        content = textwrap.dedent("""\
            use bevy_ecs::component::Component;
            use bevy_ecs::prelude::ReflectComponent;
            use bevy_reflect::Reflect;
            
            /// Marker components for Godot node types.
            /// These enable type-safe ECS queries like: Query<&GodotNodeHandle, With<Sprite2DMarker>>
            ///
            /// 🤖 This file is automatically generated by scripts/generate_godot_types.py
            /// To regenerate: python scripts/generate_godot_types.py
            
            // Base node type marker
            #[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
            #[reflect(Component)]
            pub struct NodeMarker;
            
            """)

        # Generate all markers
        for node_type in node_types:
            cfg_attr = get_type_cfg_attribute(
                self.wasm_excluded_types, self.version_gated_types, node_type
            )
            if cfg_attr:
                content += cfg_attr
            content += f"#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]\n"
            content += f"#[reflect(Component)]\n"
            content += f"pub struct {node_type}Marker;\n\n"

        with open(self.node_markers_file, "w") as f:
            f.write(content)

        print(f"✅ Generated {len(node_types)} node markers")
        run_cargo_fmt(self.node_markers_file, self.project_root)

    def run(self):
        """Run the complete generation pipeline"""
        print("🎯 Starting Godot type generation pipeline...")

        try:
            # Step 1: Generate extension API
            run_godot_dump_api(self.api_file)

            # Step 2: Parse API and extract types
            node_types, parent_map = self.load_and_parse_extension_api(self.api_file)

            # Step 3: Generate node markers
            self.generate_node_markers(node_types)

            # Step 4: Generate type checking code
            generate_type_checking_code(
                self.excluded_classes,
                self.wasm_excluded_types,
                self.version_gated_types,
                self.type_checking_file,
                self.project_root,
                node_types,
                parent_map,
            )

            # Step 5: Generate optimized GDScript watcher
            generate_gdscript_watcher(
                self.excluded_classes,
                self.gdscript_watcher_file,
                node_types,
                parent_map,
            )

            # Step 6: Generate signal names
            generate_signal_names(
                self.classes_by_name,
                self.excluded_classes,
                self.wasm_excluded_types,
                self.version_gated_types,
                self.signal_names_file,
                self.project_root,
            )

            # Step 7: Verify plugin integration
            verify_plugin_integration(self.plugin_file)

            print(textwrap.dedent(f"""
                🎉 Generation complete!
                
                Generated:
                  • {len(node_types)} node marker components
                  • Complete type checking functions
                  • Optimized GDScript scene tree watcher
                  • Signal name constants for all Godot classes
                
                Files generated:
                  • {self.node_markers_file.relative_to(self.project_root)}
                  • {self.type_checking_file.relative_to(self.project_root)}
                  • {self.gdscript_watcher_file.relative_to(self.project_root)}
                  • {self.signal_names_file.relative_to(self.project_root)}
                
                Next steps:
                  • Run 'cargo check' to verify the build
                  • Commit the generated files
                """))

        except Exception as e:
            print(f"❌ Generation failed: {e}")
            sys.exit(1)


def main():
    generator = GodotTypeGenerator()
    generator.run()


if __name__ == "__main__":
    main()
