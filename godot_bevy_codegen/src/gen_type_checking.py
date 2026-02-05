import textwrap
import unittest
from pathlib import Path
from typing import List, Dict

from godot_bevy_codegen.src.gdextension_api import (
    ExtensionApi,
    VersionHeader,
    GodotClass,
)
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
            // Add appropriate markers based on the type string
            {textwrap.indent(_generate_string_match_marker_insertion(api), "            ").strip()}
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


def _count_parents(node_type: str, parent_map: Dict[str, str]) -> int:
    """Count the number of parents for a given node type"""
    count = 0
    parent = parent_map.get(node_type, None)
    while parent is not None:
        count += 1
        parent = parent_map.get(parent, None)
    return count


def _generate_string_match_marker_insertion(
    api: ExtensionApi,
) -> str:
    """Generate match arms for the string-based marker function"""
    node_types = api.classes_descended_from("Node")
    parent_map = api.parent_map()

    # Sort node types by parent count (fewer parents first), then alphabetically
    sorted_node_types = sorted(
        node_types, key=lambda nt: (_count_parents(nt, parent_map), nt)
    )

    lines = [
        "entity_commands.insert(NodeMarker);",
        "",
        "match node_type {",
    ]

    for node_type in sorted_node_types:
        lines.append(f'    "{node_type}" => {{')
        if node_type == "Node":
            lines.append(f"        // NodeMarker added above for all nodes.")
        else:
            lines.append(f"        entity_commands.insert({node_type}Marker);")
            parent = parent_map.get(node_type, None)
            while parent is not None:
                if parent == "Node":
                    break
                lines.append(f"        entity_commands.insert({parent}Marker);")
                parent = parent_map.get(parent, None)
        lines.append("    },")

    lines.append("    // Custom user types that extend Godot nodes")
    lines.append("    _ => {}")
    lines.append("}")

    return "\n".join(lines)


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


class Tests(unittest.TestCase):
    def test_generate_string_match_arms(self):
        api = ExtensionApi(
            header=VersionHeader(4, 6, 0, "", "", "", None),
            classes=[
                GodotClass(
                    "Object", "core", True, True, None, None, None, None, "", ""
                ),
                GodotClass(
                    "Node", "core", True, True, "Object", None, None, None, "", ""
                ),
                GodotClass(
                    "Child1", "core", True, True, "Node", None, None, None, "", ""
                ),
                GodotClass(
                    "Child2", "core", True, True, "Child1", None, None, None, "", ""
                ),
            ],
        )
        self.assertEqual(
            textwrap.dedent(
                """\
                entity_commands.insert(NodeMarker);

                match node_type {
                    "Node" => {
                        // NodeMarker added above for all nodes.
                    },
                    "Child1" => {
                        entity_commands.insert(Child1Marker);
                    },
                    "Child2" => {
                        entity_commands.insert(Child2Marker);
                        entity_commands.insert(Child1Marker);
                    },
                    // Custom user types that extend Godot nodes
                    _ => {}
                }"""
            ),
            _generate_string_match_marker_insertion(api),
        )


if __name__ == "__main__":
    unittest.main()
