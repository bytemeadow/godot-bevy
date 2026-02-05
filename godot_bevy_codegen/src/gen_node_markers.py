import textwrap
from pathlib import Path

from godot_bevy_codegen.src.special_cases import get_type_cfg_attribute
from godot_bevy_codegen.src.gdextension_api import ExtensionApi
from godot_bevy_codegen.src.util import indent_log


def generate_node_markers(
    node_markers_file: Path,
    project_root: Path,
    api: ExtensionApi,
) -> None:
    """Generate the node_markers.rs file"""
    indent_log("🏷️  Generating node markers...")

    content = textwrap.dedent(
        """\
        use bevy_ecs::component::Component;
        use bevy_ecs::prelude::ReflectComponent;
        use bevy_reflect::Reflect;
        
        /// Marker components for Godot node types.
        /// These enable type-safe ECS queries like: Query<&GodotNodeHandle, With<Sprite2DMarker>>
        ///
        /// 🤖 This file is generated. Changes to it will be lost.
        /// To regenerate: `python -m godot_bevy_codegen`
        
        """
    )

    # Generate all markers
    node_classes = sorted(api.classes_descended_from("Node"))

    for node_class in node_classes:
        cfg_attr = get_type_cfg_attribute(node_class)
        if cfg_attr:
            content += cfg_attr
        content += f"#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]\n"
        content += f"#[reflect(Component)]\n"
        content += f"pub struct {node_class}Marker;\n\n"

    with open(node_markers_file, "w") as f:
        f.write(content)

    indent_log(f"✅ Generated {len(node_classes)} node markers")
