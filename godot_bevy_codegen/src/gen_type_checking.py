import textwrap
from pathlib import Path
from typing import List, Dict

from godot_bevy_codegen.src.gdextension_api import ExtensionApi
from godot_bevy_codegen.src.special_cases import (
    SpecialCases,
    get_type_cfg_attribute,
    categorize_types_by_hierarchy,
)
from godot_bevy_codegen.src.util import (
    indent_log,
    run_cargo_fmt,
)


def generate_type_checking_code(
    type_checking_file: Path,
    project_root: Path,
    api: ExtensionApi,
) -> None:
    """Generate the complete type checking implementation"""
    indent_log("🔍 Generating type checking code...")

    node_types = api.classes_descended_from("Node")
    categories = categorize_types_by_hierarchy(node_types, api.parent_map())

    content = textwrap.dedent(
        f"""\
        //! 🤖 This file is generated. Changes to it will be lost.
        //! To regenerate: uv run python -m godot_bevy_codegen
        
        use bevy_ecs::system::EntityCommands;
        use crate::interop::{{GodotNode, node_markers::*}};
        
        /// Adds appropriate marker components to an entity based on the Godot node type.
        /// This function is automatically generated and handles all {len(node_types)} Godot node types.
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
        {_generate_string_match_arms(categories)}
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
        
        """
    )

    # Generate specific checking functions
    content += _generate_hierarchy_function_comprehensive("3d", categories["3d"])
    content += _generate_hierarchy_function_comprehensive("2d", categories["2d"])
    content += _generate_hierarchy_function_comprehensive(
        "control",
        categories["control"],
    )
    content += _generate_universal_function_comprehensive(categories["universal"])

    type_checking_file.parent.mkdir(parents=True, exist_ok=True)
    with open(type_checking_file, "w") as f:
        f.write(content)

    indent_log(f"✅ Generated type checking for {len(node_types)} types")
    run_cargo_fmt(type_checking_file, project_root)


def _generate_hierarchy_function_comprehensive(
    name: str,
    types: List[str],
) -> str:
    """Generate a hierarchy-specific type checking function"""
    content = textwrap.dedent(
        f"""\
        fn check_{name}_node_types_comprehensive(
            entity_commands: &mut EntityCommands,
            node: &mut GodotNode,
        ) {{
        """
    )

    for node_type in sorted(types):
        rust_class_name = SpecialCases.fix_godot_class_name_for_rust(node_type)
        cfg_attr = get_type_cfg_attribute(node_type)
        if cfg_attr:
            content += textwrap.dedent(
                f"""\
                        {cfg_attr.strip()}
                    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
                        entity_commands.insert({node_type}Marker);
                    }}
                """
            )
        else:
            content += textwrap.dedent(
                f"""\
                    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
                        entity_commands.insert({node_type}Marker);
                    }}
                """
            )

    content += "}\n\n"

    content += textwrap.dedent(
        f"""\
        fn remove_{name}_node_types_comprehensive(
            entity_commands: &mut EntityCommands,
            _node: &mut GodotNode,
        ) {{
            entity_commands
        """
    )

    # Separate regular and version-gated types
    regular_types = []
    gated_types = {}

    for node_type in sorted(types):
        cfg_attr = get_type_cfg_attribute(node_type)
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


def _generate_string_match_arms(
    categories: Dict[str, List[str]],
) -> str:
    """Generate match arms for the string-based marker function"""
    match_arms = []

    # Add base types first
    base_types = [
        '        "Node3D" => {',
        "            entity_commands.insert(Node3DMarker);",
        "        }",
        '        "Node2D" => {',
        "            entity_commands.insert(Node2DMarker);",
        "            entity_commands.insert(CanvasItemMarker);",
        "        }",
        '        "Control" => {',
        "            entity_commands.insert(ControlMarker);",
        "            entity_commands.insert(CanvasItemMarker);",
        "        }",
        '        "CanvasItem" => {',
        "            entity_commands.insert(CanvasItemMarker);",
        "        }",
        '        "Node" => {',
        "            // NodeMarker already added above",
        "        }",
    ]
    match_arms.extend(base_types)

    # Generate Node3D types (skip base Node3D since it's already handled)
    for node_type in categories["3d"]:
        if node_type == "Node3D":
            continue  # Skip base type
        marker_name = f"{node_type}Marker"
        cfg_attr = get_type_cfg_attribute(node_type)
        if cfg_attr:
            match_arms.append(
                f"""        {cfg_attr.strip()}
    "{node_type}" => {{
        entity_commands.insert(Node3DMarker);
        entity_commands.insert({marker_name});
    }}"""
            )
        else:
            match_arms.append(
                f"""        "{node_type}" => {{
        entity_commands.insert(Node3DMarker);
        entity_commands.insert({marker_name});
    }}"""
            )

    # Generate Node2D types (skip base Node2D since it's already handled)
    for node_type in categories["2d"]:
        if node_type == "Node2D":
            continue  # Skip base type
        marker_name = f"{node_type}Marker"
        cfg_attr = get_type_cfg_attribute(node_type)
        if cfg_attr:
            match_arms.append(
                f"""        {cfg_attr.strip()}
    "{node_type}" => {{
        entity_commands.insert(Node2DMarker);
        entity_commands.insert(CanvasItemMarker);
        entity_commands.insert({marker_name});
    }}"""
            )
        else:
            match_arms.append(
                f"""        "{node_type}" => {{
        entity_commands.insert(Node2DMarker);
        entity_commands.insert(CanvasItemMarker);
        entity_commands.insert({marker_name});
    }}"""
            )

    # Generate Control types (skip base Control since it's already handled)
    for node_type in categories["control"]:
        if node_type == "Control":
            continue  # Skip base type
        marker_name = f"{node_type}Marker"
        cfg_attr = get_type_cfg_attribute(node_type)
        if cfg_attr:
            match_arms.append(
                f"""        {cfg_attr.strip()}
    "{node_type}" => {{
        entity_commands.insert(ControlMarker);
        entity_commands.insert(CanvasItemMarker);
        entity_commands.insert({marker_name});
    }}"""
            )
        else:
            match_arms.append(
                f"""        "{node_type}" => {{
        entity_commands.insert(ControlMarker);
        entity_commands.insert(CanvasItemMarker);
        entity_commands.insert({marker_name});
    }}"""
            )

    # Generate universal (direct Node) types (skip base Node, Node3D, and CanvasItem since already handled)
    for node_type in categories["universal"]:
        if node_type in ["Node", "CanvasItem", "Node3D"]:
            continue  # Skip base types
        marker_name = f"{node_type}Marker"
        cfg_attr = get_type_cfg_attribute(node_type)
        if cfg_attr:
            match_arms.append(
                f"""        {cfg_attr.strip()}
    "{node_type}" => {{
        entity_commands.insert({marker_name});
    }}"""
            )
        else:
            match_arms.append(
                f"""        "{node_type}" => {{
        entity_commands.insert({marker_name});
    }}"""
            )

    return "\n".join(match_arms)


def _generate_universal_function_comprehensive(
    types: List[str],
) -> str:
    """Generate the universal types checking function"""
    content = textwrap.dedent(
        """\
        fn check_universal_node_types_comprehensive(
            entity_commands: &mut EntityCommands,
            node: &mut GodotNode,
        ) {
        """
    )

    for node_type in sorted(types):
        rust_class_name = SpecialCases.fix_godot_class_name_for_rust(node_type)
        cfg_attr = get_type_cfg_attribute(node_type)
        if cfg_attr:
            content += textwrap.dedent(
                f"""\
                {cfg_attr.strip()}
                    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
                        entity_commands.insert({node_type}Marker);
                    }}
                """
            )
        else:
            content += textwrap.dedent(
                f"""\
                    if node.try_get::<godot::classes::{rust_class_name}>().is_some() {{
                        entity_commands.insert({node_type}Marker);
                    }}
                """
            )
    content += "}\n"

    content += textwrap.dedent(
        """\
        fn remove_universal_node_types_comprehensive(
            entity_commands: &mut EntityCommands,
            _node: &mut GodotNode,
        ) {
            entity_commands
        """
    )
    # Separate regular and version-gated types
    regular_types = []
    gated_types = {}

    for node_type in sorted(types):
        cfg_attr = get_type_cfg_attribute(node_type)
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
