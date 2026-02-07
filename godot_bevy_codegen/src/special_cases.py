import re
import unittest
from typing import List, Dict


def get_type_cfg_attribute(
    node_type: str,
) -> str:
    """Get the cfg attribute for a type if it needs version or feature gating."""
    cfg = []
    # Check for WASM-excluded types first
    if node_type in SpecialCases.wasm_excluded_types:
        cfg.append('not(feature = "experimental-wasm")\n')
    if node_type in SpecialCases.experimental_classes:
        cfg.append('feature = "experimental-godot-api"\n')
    cfg_start = "#[cfg("
    cfg_end = ")]\n"
    if cfg:
        return cfg_start + ", ".join(cfg) + cfg_end
    else:
        return ""


def categorize_types_by_hierarchy(
    node_types: List[str], parent_map: Dict[str, str]
) -> Dict[str, List[str]]:
    """Categorize node types by their inheritance hierarchy"""

    def is_descendant_of(ancestor_node_type: str, ancestor: str) -> bool:
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


class SpecialCases:
    # These classes are considered experimental by Godot
    # and require the "experimental-godot-api" feature flag in godot-rust.
    experimental_classes = [
        "AudioSample",
        "AudioSamplePlayback",
        "Compositor",
        "CompositorEffect",
        "GraphEdit",
        "GraphElement",
        "GraphFrame",
        "GraphNode",
        "NavigationAgent2D",
        "NavigationAgent3D",
        "NavigationLink2D",
        "NavigationLink3D",
        "NavigationMesh",
        "NavigationMeshSourceGeometryData2D",
        "NavigationMeshSourceGeometryData3D",
        "NavigationObstacle2D",
        "NavigationObstacle3D",
        "NavigationPathQueryParameters2D",
        "NavigationPathQueryParameters3D",
        "NavigationPathQueryResult2D",
        "NavigationPathQueryResult3D",
        "NavigationPolygon",
        "NavigationRegion2D",
        "NavigationRegion3D",
        "NavigationServer2D",
        "NavigationServer3D",
        "Parallax2D",
        "SkeletonModification2D",
        "SkeletonModification2DCCDIK",
        "SkeletonModification2DFABRIK",
        "SkeletonModification2DJiggle",
        "SkeletonModification2DLookAt",
        "SkeletonModification2DPhysicalBones",
        "SkeletonModification2DStackHolder",
        "SkeletonModification2DTwoBoneIK",
        "SkeletonModificationStack2D",
        "StreamPeerGZIP",
        "XRBodyModifier3D",
        "XRBodyTracker",
        "XRFaceModifier3D",
        "XRFaceTracker",
    ]

    # Types that are excluded when building for web/WASM
    # These types don't exist in the web extension API
    wasm_excluded_types = {
        "OpenXRRenderModel",
        "OpenXRRenderModelManager",
    }

    @staticmethod
    def fix_godot_class_name_for_rust(name: str) -> str:
        """
        Replace every group of 2+ uppercase letters with capitalized first, lowercase middle, capitalized last.
        Only match 2+ uppercase letters, NOT followed by another capital plus lowercase.

        For example:
            OpenXRCompositionLayerQuad → OpenXrCompositionLayerQuad (XR → Xr)
            CSGBox3D → CsgBox3D (CSG → Csg)
            VoxelGI → VoxelGi (GI → Gi)
            LODGroup → LodGroup
        """
        if name == "GPUParticlesCollisionSDF3D":
            return "GpuParticlesCollisionSdf3d"
        if name == "SkeletonIK3D":
            return "SkeletonIk3d"

        def repl(match):
            group = match.group(0)
            if len(group) <= 1:
                return group
            return group[0] + group[1:].lower()

        # Lookahead: do not match if followed by an uppercase then lowercase letter (next word starting)
        return re.sub(r"[A-Z]{2,}(?=[A-Z][a-z]|$|\d)", repl, name)


class Tests(unittest.TestCase):
    def test_fix_godot_class_name_for_rust(self):
        self.assertEqual(
            "CsgBox3D",
            SpecialCases.fix_godot_class_name_for_rust("CSGBox3D"),
        )
        self.assertEqual(
            "VoxelGi",
            SpecialCases.fix_godot_class_name_for_rust("VoxelGI"),
        )
        self.assertEqual(
            "XrAnchor3D",
            SpecialCases.fix_godot_class_name_for_rust("XRAnchor3D"),
        )
        self.assertEqual(
            "OpenXrCompositionLayerQuad",
            SpecialCases.fix_godot_class_name_for_rust("OpenXRCompositionLayerQuad"),
        )
        self.assertEqual(
            "LodGroup",
            SpecialCases.fix_godot_class_name_for_rust("LODGroup"),
        )
        self.assertEqual(
            "HttpServerXyz",
            SpecialCases.fix_godot_class_name_for_rust("HTTPServerXYZ"),
        )


if __name__ == "__main__":
    unittest.main()
