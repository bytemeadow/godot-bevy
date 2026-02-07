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
        
        /// Adds node type markers based on a pre-analyzed type string from GDScript.
        /// This avoids FFI calls by using type information determined on the GDScript side.
        /// This provides significant performance improvements by eliminating multiple
        /// GodotNode::try_get calls for each node.
        pub fn add_node_type_markers_from_string(
            ec: &mut EntityCommands,
            node_type: &str,
        ) {{
            // Add appropriate markers based on the type string
            {textwrap.indent(_generate_string_match_marker_insertion(api), "            ").strip()}
        }}
        
        pub fn remove_comprehensive_node_type_markers(ec: &mut EntityCommands) {{
            // All nodes inherit from Node, so remove this first
            {textwrap.indent(_generate_node_marker_removal(api), "            ").strip()}
        }}
        
        """
    )

    type_checking_file.parent.mkdir(parents=True, exist_ok=True)
    with open(type_checking_file, "w") as f:
        f.write(content)

    indent_log(f"✅ Generated type checking for {len(node_types)} types")


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
        "ec.insert(NodeMarker);",
        "",
        "match node_type {",
    ]

    for node_type in sorted_node_types:
        cfg_attr = get_type_cfg_attribute(node_type)
        if cfg_attr:
            lines.append(f"    {cfg_attr}")

        lines.append(f'    "{node_type}" => {{')
        if node_type == "Node":
            lines.append(f"        // NodeMarker added above for all nodes.")
        else:
            lines.append(f"        ec.insert({node_type}Marker);")
            parent = parent_map.get(node_type, None)
            while parent is not None:
                if parent == "Node":
                    break
                lines.append(f"        ec.insert({parent}Marker);")
                parent = parent_map.get(parent, None)
        lines.append("    },")

    lines.append("    // Custom user types that extend Godot nodes")
    lines.append("    _ => {}")
    lines.append("}")

    return "\n".join(lines)


def _generate_node_marker_removal(api: ExtensionApi):
    """Generate marker removal code for all node types"""
    node_types = api.classes_descended_from("Node")
    parent_map = api.parent_map()

    # Sort node types by parent count (fewer parents first), then alphabetically
    sorted_node_types = sorted(
        node_types, key=lambda nt: (_count_parents(nt, parent_map), nt)
    )

    lines = []
    for node_type in sorted_node_types:
        cfg_attr = get_type_cfg_attribute(node_type)
        if cfg_attr:
            lines.append(f"{cfg_attr}")
        lines.append(f"ec.remove::<{node_type}Marker>();")
    return "\n".join(lines)


class Tests(unittest.TestCase):
    def test_generate_string_match_marker_insertion(self):
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
                ec.insert(NodeMarker);

                match node_type {
                    "Node" => {
                        // NodeMarker added above for all nodes.
                    },
                    "Child1" => {
                        ec.insert(Child1Marker);
                    },
                    "Child2" => {
                        ec.insert(Child2Marker);
                        ec.insert(Child1Marker);
                    },
                    // Custom user types that extend Godot nodes
                    _ => {}
                }"""
            ),
            _generate_string_match_marker_insertion(api),
        )


if __name__ == "__main__":
    unittest.main()
