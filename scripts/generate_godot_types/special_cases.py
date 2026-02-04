import re
import unittest


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

    # Known classes that don't exist in the current Godot version or aren't available
    # Used for filtering both node types and signal generation
    excluded_classes = {
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
