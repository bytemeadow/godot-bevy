//! 🤖 This file is generated. Changes to it will be lost.
//! To regenerate: uv run python -m godot_bevy_codegen

use crate::interop::{GodotNode, node_markers::*};
use bevy_ecs::system::EntityCommands;

/// Adds appropriate marker components to an entity based on the Godot node type.
/// This function is automatically generated and handles all 282 Godot node types.
///
/// Godot's hierarchy: Node -> {Node3D, CanvasItem -> {Node2D, Control}, Others}
/// We check the major branches: 3D, 2D, Control (UI), and Universal (direct Node children)
pub fn add_comprehensive_node_type_markers(
    entity_commands: &mut EntityCommands,
    node: &mut GodotNode,
) {
    // All nodes inherit from Node, so add this first
    entity_commands.insert(NodeMarker);

    // Check the major hierarchy branches to minimize FFI calls
    if node.try_get::<godot::classes::Node3D>().is_some() {
        entity_commands.insert(Node3DMarker);
        check_3d_node_types_comprehensive(entity_commands, node);
    } else if node.try_get::<godot::classes::Node2D>().is_some() {
        entity_commands.insert(Node2DMarker);
        entity_commands.insert(CanvasItemMarker); // Node2D inherits from CanvasItem
        check_2d_node_types_comprehensive(entity_commands, node);
    } else if node.try_get::<godot::classes::Control>().is_some() {
        entity_commands.insert(ControlMarker);
        entity_commands.insert(CanvasItemMarker); // Control inherits from CanvasItem
        check_control_node_types_comprehensive(entity_commands, node);
    }

    // Check node types that inherit directly from Node
    check_universal_node_types_comprehensive(entity_commands, node);
}

/// Adds node type markers based on a pre-analyzed type string from GDScript.
/// This avoids FFI calls by using type information determined on the GDScript side.
/// This provides significant performance improvements by eliminating multiple
/// GodotNode::try_get calls for each node.
pub fn add_node_type_markers_from_string(entity_commands: &mut EntityCommands, node_type: &str) {
    // Add appropriate markers based on the type string
    entity_commands.insert(NodeMarker);

    match node_type {
        "Node" => {
            // NodeMarker added above for all nodes.
        }
        "AnimationMixer" => {
            entity_commands.insert(AnimationMixerMarker);
        }
        "AudioStreamPlayer" => {
            entity_commands.insert(AudioStreamPlayerMarker);
        }
        "CanvasItem" => {
            entity_commands.insert(CanvasItemMarker);
        }
        "CanvasLayer" => {
            entity_commands.insert(CanvasLayerMarker);
        }
        "EditorFileSystem" => {
            entity_commands.insert(EditorFileSystemMarker);
        }
        "EditorPlugin" => {
            entity_commands.insert(EditorPluginMarker);
        }
        "EditorResourcePreview" => {
            entity_commands.insert(EditorResourcePreviewMarker);
        }
        "HTTPRequest" => {
            entity_commands.insert(HTTPRequestMarker);
        }
        "InstancePlaceholder" => {
            entity_commands.insert(InstancePlaceholderMarker);
        }
        "MissingNode" => {
            entity_commands.insert(MissingNodeMarker);
        }
        "MultiplayerSpawner" => {
            entity_commands.insert(MultiplayerSpawnerMarker);
        }
        "MultiplayerSynchronizer" => {
            entity_commands.insert(MultiplayerSynchronizerMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationAgent2D" => {
            entity_commands.insert(NavigationAgent2DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationAgent3D" => {
            entity_commands.insert(NavigationAgent3DMarker);
        }
        "Node3D" => {
            entity_commands.insert(Node3DMarker);
        }
        "ResourcePreloader" => {
            entity_commands.insert(ResourcePreloaderMarker);
        }
        "ShaderGlobalsOverride" => {
            entity_commands.insert(ShaderGlobalsOverrideMarker);
        }
        "StatusIndicator" => {
            entity_commands.insert(StatusIndicatorMarker);
        }
        "Timer" => {
            entity_commands.insert(TimerMarker);
        }
        "Viewport" => {
            entity_commands.insert(ViewportMarker);
        }
        "WorldEnvironment" => {
            entity_commands.insert(WorldEnvironmentMarker);
        }
        "AnimationPlayer" => {
            entity_commands.insert(AnimationPlayerMarker);
            entity_commands.insert(AnimationMixerMarker);
        }
        "AnimationTree" => {
            entity_commands.insert(AnimationTreeMarker);
            entity_commands.insert(AnimationMixerMarker);
        }
        "AudioListener3D" => {
            entity_commands.insert(AudioListener3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "AudioStreamPlayer3D" => {
            entity_commands.insert(AudioStreamPlayer3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "BoneAttachment3D" => {
            entity_commands.insert(BoneAttachment3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Camera3D" => {
            entity_commands.insert(Camera3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CollisionObject3D" => {
            entity_commands.insert(CollisionObject3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CollisionPolygon3D" => {
            entity_commands.insert(CollisionPolygon3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CollisionShape3D" => {
            entity_commands.insert(CollisionShape3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Control" => {
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "GridMap" => {
            entity_commands.insert(GridMapMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GridMapEditorPlugin" => {
            entity_commands.insert(GridMapEditorPluginMarker);
            entity_commands.insert(EditorPluginMarker);
        }
        "ImporterMeshInstance3D" => {
            entity_commands.insert(ImporterMeshInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Joint3D" => {
            entity_commands.insert(Joint3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "LightmapProbe" => {
            entity_commands.insert(LightmapProbeMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Marker3D" => {
            entity_commands.insert(Marker3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationLink3D" => {
            entity_commands.insert(NavigationLink3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationObstacle3D" => {
            entity_commands.insert(NavigationObstacle3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationRegion3D" => {
            entity_commands.insert(NavigationRegion3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Node2D" => {
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "OpenXRCompositionLayer" => {
            entity_commands.insert(OpenXRCompositionLayerMarker);
            entity_commands.insert(Node3DMarker);
        }
        "OpenXRHand" => {
            entity_commands.insert(OpenXRHandMarker);
            entity_commands.insert(Node3DMarker);
        }
        #[cfg(not(feature = "experimental-wasm"))]
        "OpenXRRenderModel" => {
            entity_commands.insert(OpenXRRenderModelMarker);
            entity_commands.insert(Node3DMarker);
        }
        #[cfg(not(feature = "experimental-wasm"))]
        "OpenXRRenderModelManager" => {
            entity_commands.insert(OpenXRRenderModelManagerMarker);
            entity_commands.insert(Node3DMarker);
        }
        "ParallaxBackground" => {
            entity_commands.insert(ParallaxBackgroundMarker);
            entity_commands.insert(CanvasLayerMarker);
        }
        "Path3D" => {
            entity_commands.insert(Path3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "PathFollow3D" => {
            entity_commands.insert(PathFollow3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "RayCast3D" => {
            entity_commands.insert(RayCast3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "RemoteTransform3D" => {
            entity_commands.insert(RemoteTransform3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "ShapeCast3D" => {
            entity_commands.insert(ShapeCast3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Skeleton3D" => {
            entity_commands.insert(Skeleton3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SkeletonModifier3D" => {
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SpringArm3D" => {
            entity_commands.insert(SpringArm3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SpringBoneCollision3D" => {
            entity_commands.insert(SpringBoneCollision3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SubViewport" => {
            entity_commands.insert(SubViewportMarker);
            entity_commands.insert(ViewportMarker);
        }
        "VehicleWheel3D" => {
            entity_commands.insert(VehicleWheel3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "VisualInstance3D" => {
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Window" => {
            entity_commands.insert(WindowMarker);
            entity_commands.insert(ViewportMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "XRFaceModifier3D" => {
            entity_commands.insert(XRFaceModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "XRNode3D" => {
            entity_commands.insert(XRNode3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "XROrigin3D" => {
            entity_commands.insert(XROrigin3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "AcceptDialog" => {
            entity_commands.insert(AcceptDialogMarker);
            entity_commands.insert(WindowMarker);
            entity_commands.insert(ViewportMarker);
        }
        "AnimatedSprite2D" => {
            entity_commands.insert(AnimatedSprite2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Area3D" => {
            entity_commands.insert(Area3DMarker);
            entity_commands.insert(CollisionObject3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "AudioListener2D" => {
            entity_commands.insert(AudioListener2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "AudioStreamPlayer2D" => {
            entity_commands.insert(AudioStreamPlayer2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "BackBufferCopy" => {
            entity_commands.insert(BackBufferCopyMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "BaseButton" => {
            entity_commands.insert(BaseButtonMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Bone2D" => {
            entity_commands.insert(Bone2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "BoneConstraint3D" => {
            entity_commands.insert(BoneConstraint3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "BoneTwistDisperser3D" => {
            entity_commands.insert(BoneTwistDisperser3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CPUParticles2D" => {
            entity_commands.insert(CPUParticles2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Camera2D" => {
            entity_commands.insert(Camera2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "CanvasGroup" => {
            entity_commands.insert(CanvasGroupMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "CanvasModulate" => {
            entity_commands.insert(CanvasModulateMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "CollisionObject2D" => {
            entity_commands.insert(CollisionObject2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "CollisionPolygon2D" => {
            entity_commands.insert(CollisionPolygon2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "CollisionShape2D" => {
            entity_commands.insert(CollisionShape2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ColorRect" => {
            entity_commands.insert(ColorRectMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ConeTwistJoint3D" => {
            entity_commands.insert(ConeTwistJoint3DMarker);
            entity_commands.insert(Joint3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Container" => {
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Decal" => {
            entity_commands.insert(DecalMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "FogVolume" => {
            entity_commands.insert(FogVolumeMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GPUParticles2D" => {
            entity_commands.insert(GPUParticles2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "GPUParticlesAttractor3D" => {
            entity_commands.insert(GPUParticlesAttractor3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GPUParticlesCollision3D" => {
            entity_commands.insert(GPUParticlesCollision3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Generic6DOFJoint3D" => {
            entity_commands.insert(Generic6DOFJoint3DMarker);
            entity_commands.insert(Joint3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GeometryInstance3D" => {
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphEdit" => {
            entity_commands.insert(GraphEditMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "HingeJoint3D" => {
            entity_commands.insert(HingeJoint3DMarker);
            entity_commands.insert(Joint3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "IKModifier3D" => {
            entity_commands.insert(IKModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "ItemList" => {
            entity_commands.insert(ItemListMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Joint2D" => {
            entity_commands.insert(Joint2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Label" => {
            entity_commands.insert(LabelMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Light2D" => {
            entity_commands.insert(Light2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Light3D" => {
            entity_commands.insert(Light3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "LightOccluder2D" => {
            entity_commands.insert(LightOccluder2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "LightmapGI" => {
            entity_commands.insert(LightmapGIMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "LimitAngularVelocityModifier3D" => {
            entity_commands.insert(LimitAngularVelocityModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Line2D" => {
            entity_commands.insert(Line2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "LineEdit" => {
            entity_commands.insert(LineEditMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "LookAtModifier3D" => {
            entity_commands.insert(LookAtModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Marker2D" => {
            entity_commands.insert(Marker2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "MenuBar" => {
            entity_commands.insert(MenuBarMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "MeshInstance2D" => {
            entity_commands.insert(MeshInstance2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ModifierBoneTarget3D" => {
            entity_commands.insert(ModifierBoneTarget3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "MultiMeshInstance2D" => {
            entity_commands.insert(MultiMeshInstance2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationLink2D" => {
            entity_commands.insert(NavigationLink2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationObstacle2D" => {
            entity_commands.insert(NavigationObstacle2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationRegion2D" => {
            entity_commands.insert(NavigationRegion2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "NinePatchRect" => {
            entity_commands.insert(NinePatchRectMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "OccluderInstance3D" => {
            entity_commands.insert(OccluderInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "OpenXRCompositionLayerCylinder" => {
            entity_commands.insert(OpenXRCompositionLayerCylinderMarker);
            entity_commands.insert(OpenXRCompositionLayerMarker);
            entity_commands.insert(Node3DMarker);
        }
        "OpenXRCompositionLayerEquirect" => {
            entity_commands.insert(OpenXRCompositionLayerEquirectMarker);
            entity_commands.insert(OpenXRCompositionLayerMarker);
            entity_commands.insert(Node3DMarker);
        }
        "OpenXRCompositionLayerQuad" => {
            entity_commands.insert(OpenXRCompositionLayerQuadMarker);
            entity_commands.insert(OpenXRCompositionLayerMarker);
            entity_commands.insert(Node3DMarker);
        }
        "OpenXRVisibilityMask" => {
            entity_commands.insert(OpenXRVisibilityMaskMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Panel" => {
            entity_commands.insert(PanelMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "Parallax2D" => {
            entity_commands.insert(Parallax2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ParallaxLayer" => {
            entity_commands.insert(ParallaxLayerMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Path2D" => {
            entity_commands.insert(Path2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "PathFollow2D" => {
            entity_commands.insert(PathFollow2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "PhysicalBoneSimulator3D" => {
            entity_commands.insert(PhysicalBoneSimulator3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "PhysicsBody3D" => {
            entity_commands.insert(PhysicsBody3DMarker);
            entity_commands.insert(CollisionObject3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "PinJoint3D" => {
            entity_commands.insert(PinJoint3DMarker);
            entity_commands.insert(Joint3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Polygon2D" => {
            entity_commands.insert(Polygon2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Popup" => {
            entity_commands.insert(PopupMarker);
            entity_commands.insert(WindowMarker);
            entity_commands.insert(ViewportMarker);
        }
        "Range" => {
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "RayCast2D" => {
            entity_commands.insert(RayCast2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ReferenceRect" => {
            entity_commands.insert(ReferenceRectMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ReflectionProbe" => {
            entity_commands.insert(ReflectionProbeMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "RemoteTransform2D" => {
            entity_commands.insert(RemoteTransform2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "RetargetModifier3D" => {
            entity_commands.insert(RetargetModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "RichTextLabel" => {
            entity_commands.insert(RichTextLabelMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "RootMotionView" => {
            entity_commands.insert(RootMotionViewMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Separator" => {
            entity_commands.insert(SeparatorMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ShapeCast2D" => {
            entity_commands.insert(ShapeCast2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Skeleton2D" => {
            entity_commands.insert(Skeleton2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "SkeletonIK3D" => {
            entity_commands.insert(SkeletonIK3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SliderJoint3D" => {
            entity_commands.insert(SliderJoint3DMarker);
            entity_commands.insert(Joint3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SpringBoneCollisionCapsule3D" => {
            entity_commands.insert(SpringBoneCollisionCapsule3DMarker);
            entity_commands.insert(SpringBoneCollision3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SpringBoneCollisionPlane3D" => {
            entity_commands.insert(SpringBoneCollisionPlane3DMarker);
            entity_commands.insert(SpringBoneCollision3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SpringBoneCollisionSphere3D" => {
            entity_commands.insert(SpringBoneCollisionSphere3DMarker);
            entity_commands.insert(SpringBoneCollision3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SpringBoneSimulator3D" => {
            entity_commands.insert(SpringBoneSimulator3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Sprite2D" => {
            entity_commands.insert(Sprite2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "TabBar" => {
            entity_commands.insert(TabBarMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "TextEdit" => {
            entity_commands.insert(TextEditMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "TextureRect" => {
            entity_commands.insert(TextureRectMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "TileMap" => {
            entity_commands.insert(TileMapMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "TileMapLayer" => {
            entity_commands.insert(TileMapLayerMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "TouchScreenButton" => {
            entity_commands.insert(TouchScreenButtonMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Tree" => {
            entity_commands.insert(TreeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VideoStreamPlayer" => {
            entity_commands.insert(VideoStreamPlayerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VisibleOnScreenNotifier2D" => {
            entity_commands.insert(VisibleOnScreenNotifier2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VisibleOnScreenNotifier3D" => {
            entity_commands.insert(VisibleOnScreenNotifier3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "VoxelGI" => {
            entity_commands.insert(VoxelGIMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "XRAnchor3D" => {
            entity_commands.insert(XRAnchor3DMarker);
            entity_commands.insert(XRNode3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "XRBodyModifier3D" => {
            entity_commands.insert(XRBodyModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "XRCamera3D" => {
            entity_commands.insert(XRCamera3DMarker);
            entity_commands.insert(Camera3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "XRController3D" => {
            entity_commands.insert(XRController3DMarker);
            entity_commands.insert(XRNode3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "XRHandModifier3D" => {
            entity_commands.insert(XRHandModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "AimModifier3D" => {
            entity_commands.insert(AimModifier3DMarker);
            entity_commands.insert(BoneConstraint3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Area2D" => {
            entity_commands.insert(Area2DMarker);
            entity_commands.insert(CollisionObject2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "AspectRatioContainer" => {
            entity_commands.insert(AspectRatioContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "BoxContainer" => {
            entity_commands.insert(BoxContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Button" => {
            entity_commands.insert(ButtonMarker);
            entity_commands.insert(BaseButtonMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "CPUParticles3D" => {
            entity_commands.insert(CPUParticles3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CSGShape3D" => {
            entity_commands.insert(CSGShape3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CenterContainer" => {
            entity_commands.insert(CenterContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ChainIK3D" => {
            entity_commands.insert(ChainIK3DMarker);
            entity_commands.insert(IKModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CharacterBody3D" => {
            entity_commands.insert(CharacterBody3DMarker);
            entity_commands.insert(PhysicsBody3DMarker);
            entity_commands.insert(CollisionObject3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CodeEdit" => {
            entity_commands.insert(CodeEditMarker);
            entity_commands.insert(TextEditMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ConfirmationDialog" => {
            entity_commands.insert(ConfirmationDialogMarker);
            entity_commands.insert(AcceptDialogMarker);
            entity_commands.insert(WindowMarker);
            entity_commands.insert(ViewportMarker);
        }
        "ConvertTransformModifier3D" => {
            entity_commands.insert(ConvertTransformModifier3DMarker);
            entity_commands.insert(BoneConstraint3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CopyTransformModifier3D" => {
            entity_commands.insert(CopyTransformModifier3DMarker);
            entity_commands.insert(BoneConstraint3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "DampedSpringJoint2D" => {
            entity_commands.insert(DampedSpringJoint2DMarker);
            entity_commands.insert(Joint2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "DirectionalLight2D" => {
            entity_commands.insert(DirectionalLight2DMarker);
            entity_commands.insert(Light2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "DirectionalLight3D" => {
            entity_commands.insert(DirectionalLight3DMarker);
            entity_commands.insert(Light3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "EditorProperty" => {
            entity_commands.insert(EditorPropertyMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "EditorSpinSlider" => {
            entity_commands.insert(EditorSpinSliderMarker);
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "FlowContainer" => {
            entity_commands.insert(FlowContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "FoldableContainer" => {
            entity_commands.insert(FoldableContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "GPUParticles3D" => {
            entity_commands.insert(GPUParticles3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GPUParticlesAttractorBox3D" => {
            entity_commands.insert(GPUParticlesAttractorBox3DMarker);
            entity_commands.insert(GPUParticlesAttractor3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GPUParticlesAttractorSphere3D" => {
            entity_commands.insert(GPUParticlesAttractorSphere3DMarker);
            entity_commands.insert(GPUParticlesAttractor3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GPUParticlesAttractorVectorField3D" => {
            entity_commands.insert(GPUParticlesAttractorVectorField3DMarker);
            entity_commands.insert(GPUParticlesAttractor3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GPUParticlesCollisionBox3D" => {
            entity_commands.insert(GPUParticlesCollisionBox3DMarker);
            entity_commands.insert(GPUParticlesCollision3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GPUParticlesCollisionHeightField3D" => {
            entity_commands.insert(GPUParticlesCollisionHeightField3DMarker);
            entity_commands.insert(GPUParticlesCollision3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GPUParticlesCollisionSDF3D" => {
            entity_commands.insert(GPUParticlesCollisionSDF3DMarker);
            entity_commands.insert(GPUParticlesCollision3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "GPUParticlesCollisionSphere3D" => {
            entity_commands.insert(GPUParticlesCollisionSphere3DMarker);
            entity_commands.insert(GPUParticlesCollision3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphElement" => {
            entity_commands.insert(GraphElementMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "GridContainer" => {
            entity_commands.insert(GridContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "GrooveJoint2D" => {
            entity_commands.insert(GrooveJoint2DMarker);
            entity_commands.insert(Joint2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "HSeparator" => {
            entity_commands.insert(HSeparatorMarker);
            entity_commands.insert(SeparatorMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Label3D" => {
            entity_commands.insert(Label3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "LinkButton" => {
            entity_commands.insert(LinkButtonMarker);
            entity_commands.insert(BaseButtonMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "MarginContainer" => {
            entity_commands.insert(MarginContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "MeshInstance3D" => {
            entity_commands.insert(MeshInstance3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "MultiMeshInstance3D" => {
            entity_commands.insert(MultiMeshInstance3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "OmniLight3D" => {
            entity_commands.insert(OmniLight3DMarker);
            entity_commands.insert(Light3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "PanelContainer" => {
            entity_commands.insert(PanelContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "PhysicalBone3D" => {
            entity_commands.insert(PhysicalBone3DMarker);
            entity_commands.insert(PhysicsBody3DMarker);
            entity_commands.insert(CollisionObject3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "PhysicsBody2D" => {
            entity_commands.insert(PhysicsBody2DMarker);
            entity_commands.insert(CollisionObject2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "PinJoint2D" => {
            entity_commands.insert(PinJoint2DMarker);
            entity_commands.insert(Joint2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "PointLight2D" => {
            entity_commands.insert(PointLight2DMarker);
            entity_commands.insert(Light2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "PopupMenu" => {
            entity_commands.insert(PopupMenuMarker);
            entity_commands.insert(PopupMarker);
            entity_commands.insert(WindowMarker);
            entity_commands.insert(ViewportMarker);
        }
        "PopupPanel" => {
            entity_commands.insert(PopupPanelMarker);
            entity_commands.insert(PopupMarker);
            entity_commands.insert(WindowMarker);
            entity_commands.insert(ViewportMarker);
        }
        "ProgressBar" => {
            entity_commands.insert(ProgressBarMarker);
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "RigidBody3D" => {
            entity_commands.insert(RigidBody3DMarker);
            entity_commands.insert(PhysicsBody3DMarker);
            entity_commands.insert(CollisionObject3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "ScrollBar" => {
            entity_commands.insert(ScrollBarMarker);
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ScrollContainer" => {
            entity_commands.insert(ScrollContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "Slider" => {
            entity_commands.insert(SliderMarker);
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "SpinBox" => {
            entity_commands.insert(SpinBoxMarker);
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "SplitContainer" => {
            entity_commands.insert(SplitContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "SpotLight3D" => {
            entity_commands.insert(SpotLight3DMarker);
            entity_commands.insert(Light3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SpriteBase3D" => {
            entity_commands.insert(SpriteBase3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "StaticBody3D" => {
            entity_commands.insert(StaticBody3DMarker);
            entity_commands.insert(PhysicsBody3DMarker);
            entity_commands.insert(CollisionObject3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SubViewportContainer" => {
            entity_commands.insert(SubViewportContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "TabContainer" => {
            entity_commands.insert(TabContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "TextureButton" => {
            entity_commands.insert(TextureButtonMarker);
            entity_commands.insert(BaseButtonMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "TextureProgressBar" => {
            entity_commands.insert(TextureProgressBarMarker);
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "TwoBoneIK3D" => {
            entity_commands.insert(TwoBoneIK3DMarker);
            entity_commands.insert(IKModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "VSeparator" => {
            entity_commands.insert(VSeparatorMarker);
            entity_commands.insert(SeparatorMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VisibleOnScreenEnabler2D" => {
            entity_commands.insert(VisibleOnScreenEnabler2DMarker);
            entity_commands.insert(VisibleOnScreenNotifier2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VisibleOnScreenEnabler3D" => {
            entity_commands.insert(VisibleOnScreenEnabler3DMarker);
            entity_commands.insert(VisibleOnScreenNotifier3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "AnimatableBody3D" => {
            entity_commands.insert(AnimatableBody3DMarker);
            entity_commands.insert(StaticBody3DMarker);
            entity_commands.insert(PhysicsBody3DMarker);
            entity_commands.insert(CollisionObject3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "AnimatedSprite3D" => {
            entity_commands.insert(AnimatedSprite3DMarker);
            entity_commands.insert(SpriteBase3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CSGCombiner3D" => {
            entity_commands.insert(CSGCombiner3DMarker);
            entity_commands.insert(CSGShape3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CSGPrimitive3D" => {
            entity_commands.insert(CSGPrimitive3DMarker);
            entity_commands.insert(CSGShape3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CharacterBody2D" => {
            entity_commands.insert(CharacterBody2DMarker);
            entity_commands.insert(PhysicsBody2DMarker);
            entity_commands.insert(CollisionObject2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "CheckBox" => {
            entity_commands.insert(CheckBoxMarker);
            entity_commands.insert(ButtonMarker);
            entity_commands.insert(BaseButtonMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "CheckButton" => {
            entity_commands.insert(CheckButtonMarker);
            entity_commands.insert(ButtonMarker);
            entity_commands.insert(BaseButtonMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ColorPickerButton" => {
            entity_commands.insert(ColorPickerButtonMarker);
            entity_commands.insert(ButtonMarker);
            entity_commands.insert(BaseButtonMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "EditorCommandPalette" => {
            entity_commands.insert(EditorCommandPaletteMarker);
            entity_commands.insert(ConfirmationDialogMarker);
            entity_commands.insert(AcceptDialogMarker);
            entity_commands.insert(WindowMarker);
            entity_commands.insert(ViewportMarker);
        }
        "EditorDock" => {
            entity_commands.insert(EditorDockMarker);
            entity_commands.insert(MarginContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "EditorInspector" => {
            entity_commands.insert(EditorInspectorMarker);
            entity_commands.insert(ScrollContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "FileDialog" => {
            entity_commands.insert(FileDialogMarker);
            entity_commands.insert(ConfirmationDialogMarker);
            entity_commands.insert(AcceptDialogMarker);
            entity_commands.insert(WindowMarker);
            entity_commands.insert(ViewportMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphFrame" => {
            entity_commands.insert(GraphFrameMarker);
            entity_commands.insert(GraphElementMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphNode" => {
            entity_commands.insert(GraphNodeMarker);
            entity_commands.insert(GraphElementMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "HBoxContainer" => {
            entity_commands.insert(HBoxContainerMarker);
            entity_commands.insert(BoxContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "HFlowContainer" => {
            entity_commands.insert(HFlowContainerMarker);
            entity_commands.insert(FlowContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "HScrollBar" => {
            entity_commands.insert(HScrollBarMarker);
            entity_commands.insert(ScrollBarMarker);
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "HSlider" => {
            entity_commands.insert(HSliderMarker);
            entity_commands.insert(SliderMarker);
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "HSplitContainer" => {
            entity_commands.insert(HSplitContainerMarker);
            entity_commands.insert(SplitContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "IterateIK3D" => {
            entity_commands.insert(IterateIK3DMarker);
            entity_commands.insert(ChainIK3DMarker);
            entity_commands.insert(IKModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "MenuButton" => {
            entity_commands.insert(MenuButtonMarker);
            entity_commands.insert(ButtonMarker);
            entity_commands.insert(BaseButtonMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "OpenXRBindingModifierEditor" => {
            entity_commands.insert(OpenXRBindingModifierEditorMarker);
            entity_commands.insert(PanelContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "OptionButton" => {
            entity_commands.insert(OptionButtonMarker);
            entity_commands.insert(ButtonMarker);
            entity_commands.insert(BaseButtonMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "RigidBody2D" => {
            entity_commands.insert(RigidBody2DMarker);
            entity_commands.insert(PhysicsBody2DMarker);
            entity_commands.insert(CollisionObject2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ScriptCreateDialog" => {
            entity_commands.insert(ScriptCreateDialogMarker);
            entity_commands.insert(ConfirmationDialogMarker);
            entity_commands.insert(AcceptDialogMarker);
            entity_commands.insert(WindowMarker);
            entity_commands.insert(ViewportMarker);
        }
        "ScriptEditor" => {
            entity_commands.insert(ScriptEditorMarker);
            entity_commands.insert(PanelContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "SoftBody3D" => {
            entity_commands.insert(SoftBody3DMarker);
            entity_commands.insert(MeshInstance3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "SplineIK3D" => {
            entity_commands.insert(SplineIK3DMarker);
            entity_commands.insert(ChainIK3DMarker);
            entity_commands.insert(IKModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "Sprite3D" => {
            entity_commands.insert(Sprite3DMarker);
            entity_commands.insert(SpriteBase3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "StaticBody2D" => {
            entity_commands.insert(StaticBody2DMarker);
            entity_commands.insert(PhysicsBody2DMarker);
            entity_commands.insert(CollisionObject2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VBoxContainer" => {
            entity_commands.insert(VBoxContainerMarker);
            entity_commands.insert(BoxContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VFlowContainer" => {
            entity_commands.insert(VFlowContainerMarker);
            entity_commands.insert(FlowContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VScrollBar" => {
            entity_commands.insert(VScrollBarMarker);
            entity_commands.insert(ScrollBarMarker);
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VSlider" => {
            entity_commands.insert(VSliderMarker);
            entity_commands.insert(SliderMarker);
            entity_commands.insert(RangeMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VSplitContainer" => {
            entity_commands.insert(VSplitContainerMarker);
            entity_commands.insert(SplitContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "VehicleBody3D" => {
            entity_commands.insert(VehicleBody3DMarker);
            entity_commands.insert(RigidBody3DMarker);
            entity_commands.insert(PhysicsBody3DMarker);
            entity_commands.insert(CollisionObject3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "AnimatableBody2D" => {
            entity_commands.insert(AnimatableBody2DMarker);
            entity_commands.insert(StaticBody2DMarker);
            entity_commands.insert(PhysicsBody2DMarker);
            entity_commands.insert(CollisionObject2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "CCDIK3D" => {
            entity_commands.insert(CCDIK3DMarker);
            entity_commands.insert(IterateIK3DMarker);
            entity_commands.insert(ChainIK3DMarker);
            entity_commands.insert(IKModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CSGBox3D" => {
            entity_commands.insert(CSGBox3DMarker);
            entity_commands.insert(CSGPrimitive3DMarker);
            entity_commands.insert(CSGShape3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CSGCylinder3D" => {
            entity_commands.insert(CSGCylinder3DMarker);
            entity_commands.insert(CSGPrimitive3DMarker);
            entity_commands.insert(CSGShape3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CSGMesh3D" => {
            entity_commands.insert(CSGMesh3DMarker);
            entity_commands.insert(CSGPrimitive3DMarker);
            entity_commands.insert(CSGShape3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CSGPolygon3D" => {
            entity_commands.insert(CSGPolygon3DMarker);
            entity_commands.insert(CSGPrimitive3DMarker);
            entity_commands.insert(CSGShape3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CSGSphere3D" => {
            entity_commands.insert(CSGSphere3DMarker);
            entity_commands.insert(CSGPrimitive3DMarker);
            entity_commands.insert(CSGShape3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "CSGTorus3D" => {
            entity_commands.insert(CSGTorus3DMarker);
            entity_commands.insert(CSGPrimitive3DMarker);
            entity_commands.insert(CSGShape3DMarker);
            entity_commands.insert(GeometryInstance3DMarker);
            entity_commands.insert(VisualInstance3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "ColorPicker" => {
            entity_commands.insert(ColorPickerMarker);
            entity_commands.insert(VBoxContainerMarker);
            entity_commands.insert(BoxContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "EditorFileDialog" => {
            entity_commands.insert(EditorFileDialogMarker);
            entity_commands.insert(FileDialogMarker);
            entity_commands.insert(ConfirmationDialogMarker);
            entity_commands.insert(AcceptDialogMarker);
            entity_commands.insert(WindowMarker);
            entity_commands.insert(ViewportMarker);
        }
        "EditorResourcePicker" => {
            entity_commands.insert(EditorResourcePickerMarker);
            entity_commands.insert(HBoxContainerMarker);
            entity_commands.insert(BoxContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "EditorToaster" => {
            entity_commands.insert(EditorToasterMarker);
            entity_commands.insert(HBoxContainerMarker);
            entity_commands.insert(BoxContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "FABRIK3D" => {
            entity_commands.insert(FABRIK3DMarker);
            entity_commands.insert(IterateIK3DMarker);
            entity_commands.insert(ChainIK3DMarker);
            entity_commands.insert(IKModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "FileSystemDock" => {
            entity_commands.insert(FileSystemDockMarker);
            entity_commands.insert(EditorDockMarker);
            entity_commands.insert(MarginContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "JacobianIK3D" => {
            entity_commands.insert(JacobianIK3DMarker);
            entity_commands.insert(IterateIK3DMarker);
            entity_commands.insert(ChainIK3DMarker);
            entity_commands.insert(IKModifier3DMarker);
            entity_commands.insert(SkeletonModifier3DMarker);
            entity_commands.insert(Node3DMarker);
        }
        "OpenXRInteractionProfileEditorBase" => {
            entity_commands.insert(OpenXRInteractionProfileEditorBaseMarker);
            entity_commands.insert(HBoxContainerMarker);
            entity_commands.insert(BoxContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "PhysicalBone2D" => {
            entity_commands.insert(PhysicalBone2DMarker);
            entity_commands.insert(RigidBody2DMarker);
            entity_commands.insert(PhysicsBody2DMarker);
            entity_commands.insert(CollisionObject2DMarker);
            entity_commands.insert(Node2DMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "ScriptEditorBase" => {
            entity_commands.insert(ScriptEditorBaseMarker);
            entity_commands.insert(VBoxContainerMarker);
            entity_commands.insert(BoxContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "EditorScriptPicker" => {
            entity_commands.insert(EditorScriptPickerMarker);
            entity_commands.insert(EditorResourcePickerMarker);
            entity_commands.insert(HBoxContainerMarker);
            entity_commands.insert(BoxContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        "OpenXRInteractionProfileEditor" => {
            entity_commands.insert(OpenXRInteractionProfileEditorMarker);
            entity_commands.insert(OpenXRInteractionProfileEditorBaseMarker);
            entity_commands.insert(HBoxContainerMarker);
            entity_commands.insert(BoxContainerMarker);
            entity_commands.insert(ContainerMarker);
            entity_commands.insert(ControlMarker);
            entity_commands.insert(CanvasItemMarker);
        }
        // Custom user types that extend Godot nodes
        _ => {}
    }
}

pub fn remove_comprehensive_node_type_markers(
    entity_commands: &mut EntityCommands,
    node: &mut GodotNode,
) {
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
}

fn check_3d_node_types_comprehensive(entity_commands: &mut EntityCommands, node: &mut GodotNode) {
    if node.try_get::<godot::classes::AimModifier3D>().is_some() {
        entity_commands.insert(AimModifier3DMarker);
    }
    if node.try_get::<godot::classes::AnimatableBody3D>().is_some() {
        entity_commands.insert(AnimatableBody3DMarker);
    }
    if node.try_get::<godot::classes::AnimatedSprite3D>().is_some() {
        entity_commands.insert(AnimatedSprite3DMarker);
    }
    if node.try_get::<godot::classes::Area3D>().is_some() {
        entity_commands.insert(Area3DMarker);
    }
    if node.try_get::<godot::classes::AudioListener3D>().is_some() {
        entity_commands.insert(AudioListener3DMarker);
    }
    if node
        .try_get::<godot::classes::AudioStreamPlayer3D>()
        .is_some()
    {
        entity_commands.insert(AudioStreamPlayer3DMarker);
    }
    if node.try_get::<godot::classes::BoneAttachment3D>().is_some() {
        entity_commands.insert(BoneAttachment3DMarker);
    }
    if node.try_get::<godot::classes::BoneConstraint3D>().is_some() {
        entity_commands.insert(BoneConstraint3DMarker);
    }
    if node
        .try_get::<godot::classes::BoneTwistDisperser3D>()
        .is_some()
    {
        entity_commands.insert(BoneTwistDisperser3DMarker);
    }
    if node.try_get::<godot::classes::Ccdik3D>().is_some() {
        entity_commands.insert(CCDIK3DMarker);
    }
    if node.try_get::<godot::classes::CpuParticles3D>().is_some() {
        entity_commands.insert(CPUParticles3DMarker);
    }
    if node.try_get::<godot::classes::CsgBox3D>().is_some() {
        entity_commands.insert(CSGBox3DMarker);
    }
    if node.try_get::<godot::classes::CsgCombiner3D>().is_some() {
        entity_commands.insert(CSGCombiner3DMarker);
    }
    if node.try_get::<godot::classes::CsgCylinder3D>().is_some() {
        entity_commands.insert(CSGCylinder3DMarker);
    }
    if node.try_get::<godot::classes::CsgMesh3D>().is_some() {
        entity_commands.insert(CSGMesh3DMarker);
    }
    if node.try_get::<godot::classes::CsgPolygon3D>().is_some() {
        entity_commands.insert(CSGPolygon3DMarker);
    }
    if node.try_get::<godot::classes::CsgPrimitive3D>().is_some() {
        entity_commands.insert(CSGPrimitive3DMarker);
    }
    if node.try_get::<godot::classes::CsgShape3D>().is_some() {
        entity_commands.insert(CSGShape3DMarker);
    }
    if node.try_get::<godot::classes::CsgSphere3D>().is_some() {
        entity_commands.insert(CSGSphere3DMarker);
    }
    if node.try_get::<godot::classes::CsgTorus3D>().is_some() {
        entity_commands.insert(CSGTorus3DMarker);
    }
    if node.try_get::<godot::classes::Camera3D>().is_some() {
        entity_commands.insert(Camera3DMarker);
    }
    if node.try_get::<godot::classes::ChainIk3D>().is_some() {
        entity_commands.insert(ChainIK3DMarker);
    }
    if node.try_get::<godot::classes::CharacterBody3D>().is_some() {
        entity_commands.insert(CharacterBody3DMarker);
    }
    if node
        .try_get::<godot::classes::CollisionObject3D>()
        .is_some()
    {
        entity_commands.insert(CollisionObject3DMarker);
    }
    if node
        .try_get::<godot::classes::CollisionPolygon3D>()
        .is_some()
    {
        entity_commands.insert(CollisionPolygon3DMarker);
    }
    if node.try_get::<godot::classes::CollisionShape3D>().is_some() {
        entity_commands.insert(CollisionShape3DMarker);
    }
    if node.try_get::<godot::classes::ConeTwistJoint3D>().is_some() {
        entity_commands.insert(ConeTwistJoint3DMarker);
    }
    if node
        .try_get::<godot::classes::ConvertTransformModifier3D>()
        .is_some()
    {
        entity_commands.insert(ConvertTransformModifier3DMarker);
    }
    if node
        .try_get::<godot::classes::CopyTransformModifier3D>()
        .is_some()
    {
        entity_commands.insert(CopyTransformModifier3DMarker);
    }
    if node.try_get::<godot::classes::Decal>().is_some() {
        entity_commands.insert(DecalMarker);
    }
    if node
        .try_get::<godot::classes::DirectionalLight3D>()
        .is_some()
    {
        entity_commands.insert(DirectionalLight3DMarker);
    }
    if node.try_get::<godot::classes::Fabrik3D>().is_some() {
        entity_commands.insert(FABRIK3DMarker);
    }
    if node.try_get::<godot::classes::FogVolume>().is_some() {
        entity_commands.insert(FogVolumeMarker);
    }
    if node.try_get::<godot::classes::GpuParticles3D>().is_some() {
        entity_commands.insert(GPUParticles3DMarker);
    }
    if node
        .try_get::<godot::classes::GpuParticlesAttractor3D>()
        .is_some()
    {
        entity_commands.insert(GPUParticlesAttractor3DMarker);
    }
    if node
        .try_get::<godot::classes::GpuParticlesAttractorBox3D>()
        .is_some()
    {
        entity_commands.insert(GPUParticlesAttractorBox3DMarker);
    }
    if node
        .try_get::<godot::classes::GpuParticlesAttractorSphere3D>()
        .is_some()
    {
        entity_commands.insert(GPUParticlesAttractorSphere3DMarker);
    }
    if node
        .try_get::<godot::classes::GpuParticlesAttractorVectorField3D>()
        .is_some()
    {
        entity_commands.insert(GPUParticlesAttractorVectorField3DMarker);
    }
    if node
        .try_get::<godot::classes::GpuParticlesCollision3D>()
        .is_some()
    {
        entity_commands.insert(GPUParticlesCollision3DMarker);
    }
    if node
        .try_get::<godot::classes::GpuParticlesCollisionBox3D>()
        .is_some()
    {
        entity_commands.insert(GPUParticlesCollisionBox3DMarker);
    }
    if node
        .try_get::<godot::classes::GpuParticlesCollisionHeightField3D>()
        .is_some()
    {
        entity_commands.insert(GPUParticlesCollisionHeightField3DMarker);
    }
    if node
        .try_get::<godot::classes::GpuParticlesCollisionSdf3d>()
        .is_some()
    {
        entity_commands.insert(GPUParticlesCollisionSDF3DMarker);
    }
    if node
        .try_get::<godot::classes::GpuParticlesCollisionSphere3D>()
        .is_some()
    {
        entity_commands.insert(GPUParticlesCollisionSphere3DMarker);
    }
    if node
        .try_get::<godot::classes::Generic6DofJoint3D>()
        .is_some()
    {
        entity_commands.insert(Generic6DOFJoint3DMarker);
    }
    if node
        .try_get::<godot::classes::GeometryInstance3D>()
        .is_some()
    {
        entity_commands.insert(GeometryInstance3DMarker);
    }
    if node.try_get::<godot::classes::GridMap>().is_some() {
        entity_commands.insert(GridMapMarker);
    }
    if node.try_get::<godot::classes::HingeJoint3D>().is_some() {
        entity_commands.insert(HingeJoint3DMarker);
    }
    if node.try_get::<godot::classes::IkModifier3D>().is_some() {
        entity_commands.insert(IKModifier3DMarker);
    }
    if node
        .try_get::<godot::classes::ImporterMeshInstance3D>()
        .is_some()
    {
        entity_commands.insert(ImporterMeshInstance3DMarker);
    }
    if node.try_get::<godot::classes::IterateIk3D>().is_some() {
        entity_commands.insert(IterateIK3DMarker);
    }
    if node.try_get::<godot::classes::JacobianIk3D>().is_some() {
        entity_commands.insert(JacobianIK3DMarker);
    }
    if node.try_get::<godot::classes::Joint3D>().is_some() {
        entity_commands.insert(Joint3DMarker);
    }
    if node.try_get::<godot::classes::Label3D>().is_some() {
        entity_commands.insert(Label3DMarker);
    }
    if node.try_get::<godot::classes::Light3D>().is_some() {
        entity_commands.insert(Light3DMarker);
    }
    if node.try_get::<godot::classes::LightmapGi>().is_some() {
        entity_commands.insert(LightmapGIMarker);
    }
    if node.try_get::<godot::classes::LightmapProbe>().is_some() {
        entity_commands.insert(LightmapProbeMarker);
    }
    if node
        .try_get::<godot::classes::LimitAngularVelocityModifier3D>()
        .is_some()
    {
        entity_commands.insert(LimitAngularVelocityModifier3DMarker);
    }
    if node.try_get::<godot::classes::LookAtModifier3D>().is_some() {
        entity_commands.insert(LookAtModifier3DMarker);
    }
    if node.try_get::<godot::classes::Marker3D>().is_some() {
        entity_commands.insert(Marker3DMarker);
    }
    if node.try_get::<godot::classes::MeshInstance3D>().is_some() {
        entity_commands.insert(MeshInstance3DMarker);
    }
    if node
        .try_get::<godot::classes::ModifierBoneTarget3D>()
        .is_some()
    {
        entity_commands.insert(ModifierBoneTarget3DMarker);
    }
    if node
        .try_get::<godot::classes::MultiMeshInstance3D>()
        .is_some()
    {
        entity_commands.insert(MultiMeshInstance3DMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node.try_get::<godot::classes::NavigationLink3D>().is_some() {
        entity_commands.insert(NavigationLink3DMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node
        .try_get::<godot::classes::NavigationObstacle3D>()
        .is_some()
    {
        entity_commands.insert(NavigationObstacle3DMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node
        .try_get::<godot::classes::NavigationRegion3D>()
        .is_some()
    {
        entity_commands.insert(NavigationRegion3DMarker);
    }
    if node
        .try_get::<godot::classes::OccluderInstance3D>()
        .is_some()
    {
        entity_commands.insert(OccluderInstance3DMarker);
    }
    if node.try_get::<godot::classes::OmniLight3D>().is_some() {
        entity_commands.insert(OmniLight3DMarker);
    }
    if node
        .try_get::<godot::classes::OpenXrCompositionLayer>()
        .is_some()
    {
        entity_commands.insert(OpenXRCompositionLayerMarker);
    }
    if node
        .try_get::<godot::classes::OpenXrCompositionLayerCylinder>()
        .is_some()
    {
        entity_commands.insert(OpenXRCompositionLayerCylinderMarker);
    }
    if node
        .try_get::<godot::classes::OpenXrCompositionLayerEquirect>()
        .is_some()
    {
        entity_commands.insert(OpenXRCompositionLayerEquirectMarker);
    }
    if node
        .try_get::<godot::classes::OpenXrCompositionLayerQuad>()
        .is_some()
    {
        entity_commands.insert(OpenXRCompositionLayerQuadMarker);
    }
    if node.try_get::<godot::classes::OpenXrHand>().is_some() {
        entity_commands.insert(OpenXRHandMarker);
    }
    #[cfg(not(feature = "experimental-wasm"))]
    if node
        .try_get::<godot::classes::OpenXrRenderModel>()
        .is_some()
    {
        entity_commands.insert(OpenXRRenderModelMarker);
    }
    #[cfg(not(feature = "experimental-wasm"))]
    if node
        .try_get::<godot::classes::OpenXrRenderModelManager>()
        .is_some()
    {
        entity_commands.insert(OpenXRRenderModelManagerMarker);
    }
    if node
        .try_get::<godot::classes::OpenXrVisibilityMask>()
        .is_some()
    {
        entity_commands.insert(OpenXRVisibilityMaskMarker);
    }
    if node.try_get::<godot::classes::Path3D>().is_some() {
        entity_commands.insert(Path3DMarker);
    }
    if node.try_get::<godot::classes::PathFollow3D>().is_some() {
        entity_commands.insert(PathFollow3DMarker);
    }
    if node.try_get::<godot::classes::PhysicalBone3D>().is_some() {
        entity_commands.insert(PhysicalBone3DMarker);
    }
    if node
        .try_get::<godot::classes::PhysicalBoneSimulator3D>()
        .is_some()
    {
        entity_commands.insert(PhysicalBoneSimulator3DMarker);
    }
    if node.try_get::<godot::classes::PhysicsBody3D>().is_some() {
        entity_commands.insert(PhysicsBody3DMarker);
    }
    if node.try_get::<godot::classes::PinJoint3D>().is_some() {
        entity_commands.insert(PinJoint3DMarker);
    }
    if node.try_get::<godot::classes::RayCast3D>().is_some() {
        entity_commands.insert(RayCast3DMarker);
    }
    if node.try_get::<godot::classes::ReflectionProbe>().is_some() {
        entity_commands.insert(ReflectionProbeMarker);
    }
    if node
        .try_get::<godot::classes::RemoteTransform3D>()
        .is_some()
    {
        entity_commands.insert(RemoteTransform3DMarker);
    }
    if node
        .try_get::<godot::classes::RetargetModifier3D>()
        .is_some()
    {
        entity_commands.insert(RetargetModifier3DMarker);
    }
    if node.try_get::<godot::classes::RigidBody3D>().is_some() {
        entity_commands.insert(RigidBody3DMarker);
    }
    if node.try_get::<godot::classes::RootMotionView>().is_some() {
        entity_commands.insert(RootMotionViewMarker);
    }
    if node.try_get::<godot::classes::ShapeCast3D>().is_some() {
        entity_commands.insert(ShapeCast3DMarker);
    }
    if node.try_get::<godot::classes::Skeleton3D>().is_some() {
        entity_commands.insert(Skeleton3DMarker);
    }
    if node.try_get::<godot::classes::SkeletonIk3d>().is_some() {
        entity_commands.insert(SkeletonIK3DMarker);
    }
    if node
        .try_get::<godot::classes::SkeletonModifier3D>()
        .is_some()
    {
        entity_commands.insert(SkeletonModifier3DMarker);
    }
    if node.try_get::<godot::classes::SliderJoint3D>().is_some() {
        entity_commands.insert(SliderJoint3DMarker);
    }
    if node.try_get::<godot::classes::SoftBody3D>().is_some() {
        entity_commands.insert(SoftBody3DMarker);
    }
    if node.try_get::<godot::classes::SplineIk3D>().is_some() {
        entity_commands.insert(SplineIK3DMarker);
    }
    if node.try_get::<godot::classes::SpotLight3D>().is_some() {
        entity_commands.insert(SpotLight3DMarker);
    }
    if node.try_get::<godot::classes::SpringArm3D>().is_some() {
        entity_commands.insert(SpringArm3DMarker);
    }
    if node
        .try_get::<godot::classes::SpringBoneCollision3D>()
        .is_some()
    {
        entity_commands.insert(SpringBoneCollision3DMarker);
    }
    if node
        .try_get::<godot::classes::SpringBoneCollisionCapsule3D>()
        .is_some()
    {
        entity_commands.insert(SpringBoneCollisionCapsule3DMarker);
    }
    if node
        .try_get::<godot::classes::SpringBoneCollisionPlane3D>()
        .is_some()
    {
        entity_commands.insert(SpringBoneCollisionPlane3DMarker);
    }
    if node
        .try_get::<godot::classes::SpringBoneCollisionSphere3D>()
        .is_some()
    {
        entity_commands.insert(SpringBoneCollisionSphere3DMarker);
    }
    if node
        .try_get::<godot::classes::SpringBoneSimulator3D>()
        .is_some()
    {
        entity_commands.insert(SpringBoneSimulator3DMarker);
    }
    if node.try_get::<godot::classes::Sprite3D>().is_some() {
        entity_commands.insert(Sprite3DMarker);
    }
    if node.try_get::<godot::classes::SpriteBase3D>().is_some() {
        entity_commands.insert(SpriteBase3DMarker);
    }
    if node.try_get::<godot::classes::StaticBody3D>().is_some() {
        entity_commands.insert(StaticBody3DMarker);
    }
    if node.try_get::<godot::classes::TwoBoneIk3D>().is_some() {
        entity_commands.insert(TwoBoneIK3DMarker);
    }
    if node.try_get::<godot::classes::VehicleBody3D>().is_some() {
        entity_commands.insert(VehicleBody3DMarker);
    }
    if node.try_get::<godot::classes::VehicleWheel3D>().is_some() {
        entity_commands.insert(VehicleWheel3DMarker);
    }
    if node
        .try_get::<godot::classes::VisibleOnScreenEnabler3D>()
        .is_some()
    {
        entity_commands.insert(VisibleOnScreenEnabler3DMarker);
    }
    if node
        .try_get::<godot::classes::VisibleOnScreenNotifier3D>()
        .is_some()
    {
        entity_commands.insert(VisibleOnScreenNotifier3DMarker);
    }
    if node.try_get::<godot::classes::VisualInstance3D>().is_some() {
        entity_commands.insert(VisualInstance3DMarker);
    }
    if node.try_get::<godot::classes::VoxelGi>().is_some() {
        entity_commands.insert(VoxelGIMarker);
    }
    if node.try_get::<godot::classes::XrAnchor3D>().is_some() {
        entity_commands.insert(XRAnchor3DMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node.try_get::<godot::classes::XrBodyModifier3D>().is_some() {
        entity_commands.insert(XRBodyModifier3DMarker);
    }
    if node.try_get::<godot::classes::XrCamera3D>().is_some() {
        entity_commands.insert(XRCamera3DMarker);
    }
    if node.try_get::<godot::classes::XrController3D>().is_some() {
        entity_commands.insert(XRController3DMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node.try_get::<godot::classes::XrFaceModifier3D>().is_some() {
        entity_commands.insert(XRFaceModifier3DMarker);
    }
    if node.try_get::<godot::classes::XrHandModifier3D>().is_some() {
        entity_commands.insert(XRHandModifier3DMarker);
    }
    if node.try_get::<godot::classes::XrNode3D>().is_some() {
        entity_commands.insert(XRNode3DMarker);
    }
    if node.try_get::<godot::classes::XrOrigin3D>().is_some() {
        entity_commands.insert(XROrigin3DMarker);
    }
}

fn remove_3d_node_types_comprehensive(entity_commands: &mut EntityCommands, _node: &mut GodotNode) {
    entity_commands
        .remove::<AimModifier3DMarker>()
        .remove::<AnimatableBody3DMarker>()
        .remove::<AnimatedSprite3DMarker>()
        .remove::<Area3DMarker>()
        .remove::<AudioListener3DMarker>()
        .remove::<AudioStreamPlayer3DMarker>()
        .remove::<BoneAttachment3DMarker>()
        .remove::<BoneConstraint3DMarker>()
        .remove::<BoneTwistDisperser3DMarker>()
        .remove::<CCDIK3DMarker>()
        .remove::<CPUParticles3DMarker>()
        .remove::<CSGBox3DMarker>()
        .remove::<CSGCombiner3DMarker>()
        .remove::<CSGCylinder3DMarker>()
        .remove::<CSGMesh3DMarker>()
        .remove::<CSGPolygon3DMarker>()
        .remove::<CSGPrimitive3DMarker>()
        .remove::<CSGShape3DMarker>()
        .remove::<CSGSphere3DMarker>()
        .remove::<CSGTorus3DMarker>()
        .remove::<Camera3DMarker>()
        .remove::<ChainIK3DMarker>()
        .remove::<CharacterBody3DMarker>()
        .remove::<CollisionObject3DMarker>()
        .remove::<CollisionPolygon3DMarker>()
        .remove::<CollisionShape3DMarker>()
        .remove::<ConeTwistJoint3DMarker>()
        .remove::<ConvertTransformModifier3DMarker>()
        .remove::<CopyTransformModifier3DMarker>()
        .remove::<DecalMarker>()
        .remove::<DirectionalLight3DMarker>()
        .remove::<FABRIK3DMarker>()
        .remove::<FogVolumeMarker>()
        .remove::<GPUParticles3DMarker>()
        .remove::<GPUParticlesAttractor3DMarker>()
        .remove::<GPUParticlesAttractorBox3DMarker>()
        .remove::<GPUParticlesAttractorSphere3DMarker>()
        .remove::<GPUParticlesAttractorVectorField3DMarker>()
        .remove::<GPUParticlesCollision3DMarker>()
        .remove::<GPUParticlesCollisionBox3DMarker>()
        .remove::<GPUParticlesCollisionHeightField3DMarker>()
        .remove::<GPUParticlesCollisionSDF3DMarker>()
        .remove::<GPUParticlesCollisionSphere3DMarker>()
        .remove::<Generic6DOFJoint3DMarker>()
        .remove::<GeometryInstance3DMarker>()
        .remove::<GridMapMarker>()
        .remove::<HingeJoint3DMarker>()
        .remove::<IKModifier3DMarker>()
        .remove::<ImporterMeshInstance3DMarker>()
        .remove::<IterateIK3DMarker>()
        .remove::<JacobianIK3DMarker>()
        .remove::<Joint3DMarker>()
        .remove::<Label3DMarker>()
        .remove::<Light3DMarker>()
        .remove::<LightmapGIMarker>()
        .remove::<LightmapProbeMarker>()
        .remove::<LimitAngularVelocityModifier3DMarker>()
        .remove::<LookAtModifier3DMarker>()
        .remove::<Marker3DMarker>()
        .remove::<MeshInstance3DMarker>()
        .remove::<ModifierBoneTarget3DMarker>()
        .remove::<MultiMeshInstance3DMarker>()
        .remove::<OccluderInstance3DMarker>()
        .remove::<OmniLight3DMarker>()
        .remove::<OpenXRCompositionLayerMarker>()
        .remove::<OpenXRCompositionLayerCylinderMarker>()
        .remove::<OpenXRCompositionLayerEquirectMarker>()
        .remove::<OpenXRCompositionLayerQuadMarker>()
        .remove::<OpenXRHandMarker>()
        .remove::<OpenXRVisibilityMaskMarker>()
        .remove::<Path3DMarker>()
        .remove::<PathFollow3DMarker>()
        .remove::<PhysicalBone3DMarker>()
        .remove::<PhysicalBoneSimulator3DMarker>()
        .remove::<PhysicsBody3DMarker>()
        .remove::<PinJoint3DMarker>()
        .remove::<RayCast3DMarker>()
        .remove::<ReflectionProbeMarker>()
        .remove::<RemoteTransform3DMarker>()
        .remove::<RetargetModifier3DMarker>()
        .remove::<RigidBody3DMarker>()
        .remove::<RootMotionViewMarker>()
        .remove::<ShapeCast3DMarker>()
        .remove::<Skeleton3DMarker>()
        .remove::<SkeletonIK3DMarker>()
        .remove::<SkeletonModifier3DMarker>()
        .remove::<SliderJoint3DMarker>()
        .remove::<SoftBody3DMarker>()
        .remove::<SplineIK3DMarker>()
        .remove::<SpotLight3DMarker>()
        .remove::<SpringArm3DMarker>()
        .remove::<SpringBoneCollision3DMarker>()
        .remove::<SpringBoneCollisionCapsule3DMarker>()
        .remove::<SpringBoneCollisionPlane3DMarker>()
        .remove::<SpringBoneCollisionSphere3DMarker>()
        .remove::<SpringBoneSimulator3DMarker>()
        .remove::<Sprite3DMarker>()
        .remove::<SpriteBase3DMarker>()
        .remove::<StaticBody3DMarker>()
        .remove::<TwoBoneIK3DMarker>()
        .remove::<VehicleBody3DMarker>()
        .remove::<VehicleWheel3DMarker>()
        .remove::<VisibleOnScreenEnabler3DMarker>()
        .remove::<VisibleOnScreenNotifier3DMarker>()
        .remove::<VisualInstance3DMarker>()
        .remove::<VoxelGIMarker>()
        .remove::<XRAnchor3DMarker>()
        .remove::<XRCamera3DMarker>()
        .remove::<XRController3DMarker>()
        .remove::<XRHandModifier3DMarker>()
        .remove::<XRNode3DMarker>()
        .remove::<XROrigin3DMarker>();

    #[cfg(feature = "experimental-godot-api")]
    entity_commands
        .remove::<NavigationLink3DMarker>()
        .remove::<NavigationObstacle3DMarker>()
        .remove::<NavigationRegion3DMarker>()
        .remove::<XRBodyModifier3DMarker>()
        .remove::<XRFaceModifier3DMarker>();

    #[cfg(not(feature = "experimental-wasm"))]
    entity_commands
        .remove::<OpenXRRenderModelMarker>()
        .remove::<OpenXRRenderModelManagerMarker>();
}

fn check_2d_node_types_comprehensive(entity_commands: &mut EntityCommands, node: &mut GodotNode) {
    if node.try_get::<godot::classes::AnimatableBody2D>().is_some() {
        entity_commands.insert(AnimatableBody2DMarker);
    }
    if node.try_get::<godot::classes::AnimatedSprite2D>().is_some() {
        entity_commands.insert(AnimatedSprite2DMarker);
    }
    if node.try_get::<godot::classes::Area2D>().is_some() {
        entity_commands.insert(Area2DMarker);
    }
    if node.try_get::<godot::classes::AudioListener2D>().is_some() {
        entity_commands.insert(AudioListener2DMarker);
    }
    if node
        .try_get::<godot::classes::AudioStreamPlayer2D>()
        .is_some()
    {
        entity_commands.insert(AudioStreamPlayer2DMarker);
    }
    if node.try_get::<godot::classes::BackBufferCopy>().is_some() {
        entity_commands.insert(BackBufferCopyMarker);
    }
    if node.try_get::<godot::classes::Bone2D>().is_some() {
        entity_commands.insert(Bone2DMarker);
    }
    if node.try_get::<godot::classes::CpuParticles2D>().is_some() {
        entity_commands.insert(CPUParticles2DMarker);
    }
    if node.try_get::<godot::classes::Camera2D>().is_some() {
        entity_commands.insert(Camera2DMarker);
    }
    if node.try_get::<godot::classes::CanvasGroup>().is_some() {
        entity_commands.insert(CanvasGroupMarker);
    }
    if node.try_get::<godot::classes::CanvasModulate>().is_some() {
        entity_commands.insert(CanvasModulateMarker);
    }
    if node.try_get::<godot::classes::CharacterBody2D>().is_some() {
        entity_commands.insert(CharacterBody2DMarker);
    }
    if node
        .try_get::<godot::classes::CollisionObject2D>()
        .is_some()
    {
        entity_commands.insert(CollisionObject2DMarker);
    }
    if node
        .try_get::<godot::classes::CollisionPolygon2D>()
        .is_some()
    {
        entity_commands.insert(CollisionPolygon2DMarker);
    }
    if node.try_get::<godot::classes::CollisionShape2D>().is_some() {
        entity_commands.insert(CollisionShape2DMarker);
    }
    if node
        .try_get::<godot::classes::DampedSpringJoint2D>()
        .is_some()
    {
        entity_commands.insert(DampedSpringJoint2DMarker);
    }
    if node
        .try_get::<godot::classes::DirectionalLight2D>()
        .is_some()
    {
        entity_commands.insert(DirectionalLight2DMarker);
    }
    if node.try_get::<godot::classes::GpuParticles2D>().is_some() {
        entity_commands.insert(GPUParticles2DMarker);
    }
    if node.try_get::<godot::classes::GrooveJoint2D>().is_some() {
        entity_commands.insert(GrooveJoint2DMarker);
    }
    if node.try_get::<godot::classes::Joint2D>().is_some() {
        entity_commands.insert(Joint2DMarker);
    }
    if node.try_get::<godot::classes::Light2D>().is_some() {
        entity_commands.insert(Light2DMarker);
    }
    if node.try_get::<godot::classes::LightOccluder2D>().is_some() {
        entity_commands.insert(LightOccluder2DMarker);
    }
    if node.try_get::<godot::classes::Line2D>().is_some() {
        entity_commands.insert(Line2DMarker);
    }
    if node.try_get::<godot::classes::Marker2D>().is_some() {
        entity_commands.insert(Marker2DMarker);
    }
    if node.try_get::<godot::classes::MeshInstance2D>().is_some() {
        entity_commands.insert(MeshInstance2DMarker);
    }
    if node
        .try_get::<godot::classes::MultiMeshInstance2D>()
        .is_some()
    {
        entity_commands.insert(MultiMeshInstance2DMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node.try_get::<godot::classes::NavigationLink2D>().is_some() {
        entity_commands.insert(NavigationLink2DMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node
        .try_get::<godot::classes::NavigationObstacle2D>()
        .is_some()
    {
        entity_commands.insert(NavigationObstacle2DMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node
        .try_get::<godot::classes::NavigationRegion2D>()
        .is_some()
    {
        entity_commands.insert(NavigationRegion2DMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node.try_get::<godot::classes::Parallax2D>().is_some() {
        entity_commands.insert(Parallax2DMarker);
    }
    if node.try_get::<godot::classes::ParallaxLayer>().is_some() {
        entity_commands.insert(ParallaxLayerMarker);
    }
    if node.try_get::<godot::classes::Path2D>().is_some() {
        entity_commands.insert(Path2DMarker);
    }
    if node.try_get::<godot::classes::PathFollow2D>().is_some() {
        entity_commands.insert(PathFollow2DMarker);
    }
    if node.try_get::<godot::classes::PhysicalBone2D>().is_some() {
        entity_commands.insert(PhysicalBone2DMarker);
    }
    if node.try_get::<godot::classes::PhysicsBody2D>().is_some() {
        entity_commands.insert(PhysicsBody2DMarker);
    }
    if node.try_get::<godot::classes::PinJoint2D>().is_some() {
        entity_commands.insert(PinJoint2DMarker);
    }
    if node.try_get::<godot::classes::PointLight2D>().is_some() {
        entity_commands.insert(PointLight2DMarker);
    }
    if node.try_get::<godot::classes::Polygon2D>().is_some() {
        entity_commands.insert(Polygon2DMarker);
    }
    if node.try_get::<godot::classes::RayCast2D>().is_some() {
        entity_commands.insert(RayCast2DMarker);
    }
    if node
        .try_get::<godot::classes::RemoteTransform2D>()
        .is_some()
    {
        entity_commands.insert(RemoteTransform2DMarker);
    }
    if node.try_get::<godot::classes::RigidBody2D>().is_some() {
        entity_commands.insert(RigidBody2DMarker);
    }
    if node.try_get::<godot::classes::ShapeCast2D>().is_some() {
        entity_commands.insert(ShapeCast2DMarker);
    }
    if node.try_get::<godot::classes::Skeleton2D>().is_some() {
        entity_commands.insert(Skeleton2DMarker);
    }
    if node.try_get::<godot::classes::Sprite2D>().is_some() {
        entity_commands.insert(Sprite2DMarker);
    }
    if node.try_get::<godot::classes::StaticBody2D>().is_some() {
        entity_commands.insert(StaticBody2DMarker);
    }
    if node.try_get::<godot::classes::TileMap>().is_some() {
        entity_commands.insert(TileMapMarker);
    }
    if node.try_get::<godot::classes::TileMapLayer>().is_some() {
        entity_commands.insert(TileMapLayerMarker);
    }
    if node
        .try_get::<godot::classes::TouchScreenButton>()
        .is_some()
    {
        entity_commands.insert(TouchScreenButtonMarker);
    }
    if node
        .try_get::<godot::classes::VisibleOnScreenEnabler2D>()
        .is_some()
    {
        entity_commands.insert(VisibleOnScreenEnabler2DMarker);
    }
    if node
        .try_get::<godot::classes::VisibleOnScreenNotifier2D>()
        .is_some()
    {
        entity_commands.insert(VisibleOnScreenNotifier2DMarker);
    }
}

fn remove_2d_node_types_comprehensive(entity_commands: &mut EntityCommands, _node: &mut GodotNode) {
    entity_commands
        .remove::<AnimatableBody2DMarker>()
        .remove::<AnimatedSprite2DMarker>()
        .remove::<Area2DMarker>()
        .remove::<AudioListener2DMarker>()
        .remove::<AudioStreamPlayer2DMarker>()
        .remove::<BackBufferCopyMarker>()
        .remove::<Bone2DMarker>()
        .remove::<CPUParticles2DMarker>()
        .remove::<Camera2DMarker>()
        .remove::<CanvasGroupMarker>()
        .remove::<CanvasModulateMarker>()
        .remove::<CharacterBody2DMarker>()
        .remove::<CollisionObject2DMarker>()
        .remove::<CollisionPolygon2DMarker>()
        .remove::<CollisionShape2DMarker>()
        .remove::<DampedSpringJoint2DMarker>()
        .remove::<DirectionalLight2DMarker>()
        .remove::<GPUParticles2DMarker>()
        .remove::<GrooveJoint2DMarker>()
        .remove::<Joint2DMarker>()
        .remove::<Light2DMarker>()
        .remove::<LightOccluder2DMarker>()
        .remove::<Line2DMarker>()
        .remove::<Marker2DMarker>()
        .remove::<MeshInstance2DMarker>()
        .remove::<MultiMeshInstance2DMarker>()
        .remove::<ParallaxLayerMarker>()
        .remove::<Path2DMarker>()
        .remove::<PathFollow2DMarker>()
        .remove::<PhysicalBone2DMarker>()
        .remove::<PhysicsBody2DMarker>()
        .remove::<PinJoint2DMarker>()
        .remove::<PointLight2DMarker>()
        .remove::<Polygon2DMarker>()
        .remove::<RayCast2DMarker>()
        .remove::<RemoteTransform2DMarker>()
        .remove::<RigidBody2DMarker>()
        .remove::<ShapeCast2DMarker>()
        .remove::<Skeleton2DMarker>()
        .remove::<Sprite2DMarker>()
        .remove::<StaticBody2DMarker>()
        .remove::<TileMapMarker>()
        .remove::<TileMapLayerMarker>()
        .remove::<TouchScreenButtonMarker>()
        .remove::<VisibleOnScreenEnabler2DMarker>()
        .remove::<VisibleOnScreenNotifier2DMarker>();

    #[cfg(feature = "experimental-godot-api")]
    entity_commands
        .remove::<NavigationLink2DMarker>()
        .remove::<NavigationObstacle2DMarker>()
        .remove::<NavigationRegion2DMarker>()
        .remove::<Parallax2DMarker>();
}

fn check_control_node_types_comprehensive(
    entity_commands: &mut EntityCommands,
    node: &mut GodotNode,
) {
    if node
        .try_get::<godot::classes::AspectRatioContainer>()
        .is_some()
    {
        entity_commands.insert(AspectRatioContainerMarker);
    }
    if node.try_get::<godot::classes::BaseButton>().is_some() {
        entity_commands.insert(BaseButtonMarker);
    }
    if node.try_get::<godot::classes::BoxContainer>().is_some() {
        entity_commands.insert(BoxContainerMarker);
    }
    if node.try_get::<godot::classes::Button>().is_some() {
        entity_commands.insert(ButtonMarker);
    }
    if node.try_get::<godot::classes::CenterContainer>().is_some() {
        entity_commands.insert(CenterContainerMarker);
    }
    if node.try_get::<godot::classes::CheckBox>().is_some() {
        entity_commands.insert(CheckBoxMarker);
    }
    if node.try_get::<godot::classes::CheckButton>().is_some() {
        entity_commands.insert(CheckButtonMarker);
    }
    if node.try_get::<godot::classes::CodeEdit>().is_some() {
        entity_commands.insert(CodeEditMarker);
    }
    if node.try_get::<godot::classes::ColorPicker>().is_some() {
        entity_commands.insert(ColorPickerMarker);
    }
    if node
        .try_get::<godot::classes::ColorPickerButton>()
        .is_some()
    {
        entity_commands.insert(ColorPickerButtonMarker);
    }
    if node.try_get::<godot::classes::ColorRect>().is_some() {
        entity_commands.insert(ColorRectMarker);
    }
    if node.try_get::<godot::classes::Container>().is_some() {
        entity_commands.insert(ContainerMarker);
    }
    if node.try_get::<godot::classes::EditorDock>().is_some() {
        entity_commands.insert(EditorDockMarker);
    }
    if node.try_get::<godot::classes::EditorInspector>().is_some() {
        entity_commands.insert(EditorInspectorMarker);
    }
    if node.try_get::<godot::classes::EditorProperty>().is_some() {
        entity_commands.insert(EditorPropertyMarker);
    }
    if node
        .try_get::<godot::classes::EditorResourcePicker>()
        .is_some()
    {
        entity_commands.insert(EditorResourcePickerMarker);
    }
    if node
        .try_get::<godot::classes::EditorScriptPicker>()
        .is_some()
    {
        entity_commands.insert(EditorScriptPickerMarker);
    }
    if node.try_get::<godot::classes::EditorSpinSlider>().is_some() {
        entity_commands.insert(EditorSpinSliderMarker);
    }
    if node.try_get::<godot::classes::EditorToaster>().is_some() {
        entity_commands.insert(EditorToasterMarker);
    }
    if node.try_get::<godot::classes::FileSystemDock>().is_some() {
        entity_commands.insert(FileSystemDockMarker);
    }
    if node.try_get::<godot::classes::FlowContainer>().is_some() {
        entity_commands.insert(FlowContainerMarker);
    }
    if node
        .try_get::<godot::classes::FoldableContainer>()
        .is_some()
    {
        entity_commands.insert(FoldableContainerMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node.try_get::<godot::classes::GraphEdit>().is_some() {
        entity_commands.insert(GraphEditMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node.try_get::<godot::classes::GraphElement>().is_some() {
        entity_commands.insert(GraphElementMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node.try_get::<godot::classes::GraphFrame>().is_some() {
        entity_commands.insert(GraphFrameMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node.try_get::<godot::classes::GraphNode>().is_some() {
        entity_commands.insert(GraphNodeMarker);
    }
    if node.try_get::<godot::classes::GridContainer>().is_some() {
        entity_commands.insert(GridContainerMarker);
    }
    if node.try_get::<godot::classes::HBoxContainer>().is_some() {
        entity_commands.insert(HBoxContainerMarker);
    }
    if node.try_get::<godot::classes::HFlowContainer>().is_some() {
        entity_commands.insert(HFlowContainerMarker);
    }
    if node.try_get::<godot::classes::HScrollBar>().is_some() {
        entity_commands.insert(HScrollBarMarker);
    }
    if node.try_get::<godot::classes::HSeparator>().is_some() {
        entity_commands.insert(HSeparatorMarker);
    }
    if node.try_get::<godot::classes::HSlider>().is_some() {
        entity_commands.insert(HSliderMarker);
    }
    if node.try_get::<godot::classes::HSplitContainer>().is_some() {
        entity_commands.insert(HSplitContainerMarker);
    }
    if node.try_get::<godot::classes::ItemList>().is_some() {
        entity_commands.insert(ItemListMarker);
    }
    if node.try_get::<godot::classes::Label>().is_some() {
        entity_commands.insert(LabelMarker);
    }
    if node.try_get::<godot::classes::LineEdit>().is_some() {
        entity_commands.insert(LineEditMarker);
    }
    if node.try_get::<godot::classes::LinkButton>().is_some() {
        entity_commands.insert(LinkButtonMarker);
    }
    if node.try_get::<godot::classes::MarginContainer>().is_some() {
        entity_commands.insert(MarginContainerMarker);
    }
    if node.try_get::<godot::classes::MenuBar>().is_some() {
        entity_commands.insert(MenuBarMarker);
    }
    if node.try_get::<godot::classes::MenuButton>().is_some() {
        entity_commands.insert(MenuButtonMarker);
    }
    if node.try_get::<godot::classes::NinePatchRect>().is_some() {
        entity_commands.insert(NinePatchRectMarker);
    }
    if node
        .try_get::<godot::classes::OpenXrBindingModifierEditor>()
        .is_some()
    {
        entity_commands.insert(OpenXRBindingModifierEditorMarker);
    }
    if node
        .try_get::<godot::classes::OpenXrInteractionProfileEditor>()
        .is_some()
    {
        entity_commands.insert(OpenXRInteractionProfileEditorMarker);
    }
    if node
        .try_get::<godot::classes::OpenXrInteractionProfileEditorBase>()
        .is_some()
    {
        entity_commands.insert(OpenXRInteractionProfileEditorBaseMarker);
    }
    if node.try_get::<godot::classes::OptionButton>().is_some() {
        entity_commands.insert(OptionButtonMarker);
    }
    if node.try_get::<godot::classes::Panel>().is_some() {
        entity_commands.insert(PanelMarker);
    }
    if node.try_get::<godot::classes::PanelContainer>().is_some() {
        entity_commands.insert(PanelContainerMarker);
    }
    if node.try_get::<godot::classes::ProgressBar>().is_some() {
        entity_commands.insert(ProgressBarMarker);
    }
    if node.try_get::<godot::classes::Range>().is_some() {
        entity_commands.insert(RangeMarker);
    }
    if node.try_get::<godot::classes::ReferenceRect>().is_some() {
        entity_commands.insert(ReferenceRectMarker);
    }
    if node.try_get::<godot::classes::RichTextLabel>().is_some() {
        entity_commands.insert(RichTextLabelMarker);
    }
    if node.try_get::<godot::classes::ScriptEditor>().is_some() {
        entity_commands.insert(ScriptEditorMarker);
    }
    if node.try_get::<godot::classes::ScriptEditorBase>().is_some() {
        entity_commands.insert(ScriptEditorBaseMarker);
    }
    if node.try_get::<godot::classes::ScrollBar>().is_some() {
        entity_commands.insert(ScrollBarMarker);
    }
    if node.try_get::<godot::classes::ScrollContainer>().is_some() {
        entity_commands.insert(ScrollContainerMarker);
    }
    if node.try_get::<godot::classes::Separator>().is_some() {
        entity_commands.insert(SeparatorMarker);
    }
    if node.try_get::<godot::classes::Slider>().is_some() {
        entity_commands.insert(SliderMarker);
    }
    if node.try_get::<godot::classes::SpinBox>().is_some() {
        entity_commands.insert(SpinBoxMarker);
    }
    if node.try_get::<godot::classes::SplitContainer>().is_some() {
        entity_commands.insert(SplitContainerMarker);
    }
    if node
        .try_get::<godot::classes::SubViewportContainer>()
        .is_some()
    {
        entity_commands.insert(SubViewportContainerMarker);
    }
    if node.try_get::<godot::classes::TabBar>().is_some() {
        entity_commands.insert(TabBarMarker);
    }
    if node.try_get::<godot::classes::TabContainer>().is_some() {
        entity_commands.insert(TabContainerMarker);
    }
    if node.try_get::<godot::classes::TextEdit>().is_some() {
        entity_commands.insert(TextEditMarker);
    }
    if node.try_get::<godot::classes::TextureButton>().is_some() {
        entity_commands.insert(TextureButtonMarker);
    }
    if node
        .try_get::<godot::classes::TextureProgressBar>()
        .is_some()
    {
        entity_commands.insert(TextureProgressBarMarker);
    }
    if node.try_get::<godot::classes::TextureRect>().is_some() {
        entity_commands.insert(TextureRectMarker);
    }
    if node.try_get::<godot::classes::Tree>().is_some() {
        entity_commands.insert(TreeMarker);
    }
    if node.try_get::<godot::classes::VBoxContainer>().is_some() {
        entity_commands.insert(VBoxContainerMarker);
    }
    if node.try_get::<godot::classes::VFlowContainer>().is_some() {
        entity_commands.insert(VFlowContainerMarker);
    }
    if node.try_get::<godot::classes::VScrollBar>().is_some() {
        entity_commands.insert(VScrollBarMarker);
    }
    if node.try_get::<godot::classes::VSeparator>().is_some() {
        entity_commands.insert(VSeparatorMarker);
    }
    if node.try_get::<godot::classes::VSlider>().is_some() {
        entity_commands.insert(VSliderMarker);
    }
    if node.try_get::<godot::classes::VSplitContainer>().is_some() {
        entity_commands.insert(VSplitContainerMarker);
    }
    if node
        .try_get::<godot::classes::VideoStreamPlayer>()
        .is_some()
    {
        entity_commands.insert(VideoStreamPlayerMarker);
    }
}

fn remove_control_node_types_comprehensive(
    entity_commands: &mut EntityCommands,
    _node: &mut GodotNode,
) {
    entity_commands
        .remove::<AspectRatioContainerMarker>()
        .remove::<BaseButtonMarker>()
        .remove::<BoxContainerMarker>()
        .remove::<ButtonMarker>()
        .remove::<CenterContainerMarker>()
        .remove::<CheckBoxMarker>()
        .remove::<CheckButtonMarker>()
        .remove::<CodeEditMarker>()
        .remove::<ColorPickerMarker>()
        .remove::<ColorPickerButtonMarker>()
        .remove::<ColorRectMarker>()
        .remove::<ContainerMarker>()
        .remove::<EditorDockMarker>()
        .remove::<EditorInspectorMarker>()
        .remove::<EditorPropertyMarker>()
        .remove::<EditorResourcePickerMarker>()
        .remove::<EditorScriptPickerMarker>()
        .remove::<EditorSpinSliderMarker>()
        .remove::<EditorToasterMarker>()
        .remove::<FileSystemDockMarker>()
        .remove::<FlowContainerMarker>()
        .remove::<FoldableContainerMarker>()
        .remove::<GridContainerMarker>()
        .remove::<HBoxContainerMarker>()
        .remove::<HFlowContainerMarker>()
        .remove::<HScrollBarMarker>()
        .remove::<HSeparatorMarker>()
        .remove::<HSliderMarker>()
        .remove::<HSplitContainerMarker>()
        .remove::<ItemListMarker>()
        .remove::<LabelMarker>()
        .remove::<LineEditMarker>()
        .remove::<LinkButtonMarker>()
        .remove::<MarginContainerMarker>()
        .remove::<MenuBarMarker>()
        .remove::<MenuButtonMarker>()
        .remove::<NinePatchRectMarker>()
        .remove::<OpenXRBindingModifierEditorMarker>()
        .remove::<OpenXRInteractionProfileEditorMarker>()
        .remove::<OpenXRInteractionProfileEditorBaseMarker>()
        .remove::<OptionButtonMarker>()
        .remove::<PanelMarker>()
        .remove::<PanelContainerMarker>()
        .remove::<ProgressBarMarker>()
        .remove::<RangeMarker>()
        .remove::<ReferenceRectMarker>()
        .remove::<RichTextLabelMarker>()
        .remove::<ScriptEditorMarker>()
        .remove::<ScriptEditorBaseMarker>()
        .remove::<ScrollBarMarker>()
        .remove::<ScrollContainerMarker>()
        .remove::<SeparatorMarker>()
        .remove::<SliderMarker>()
        .remove::<SpinBoxMarker>()
        .remove::<SplitContainerMarker>()
        .remove::<SubViewportContainerMarker>()
        .remove::<TabBarMarker>()
        .remove::<TabContainerMarker>()
        .remove::<TextEditMarker>()
        .remove::<TextureButtonMarker>()
        .remove::<TextureProgressBarMarker>()
        .remove::<TextureRectMarker>()
        .remove::<TreeMarker>()
        .remove::<VBoxContainerMarker>()
        .remove::<VFlowContainerMarker>()
        .remove::<VScrollBarMarker>()
        .remove::<VSeparatorMarker>()
        .remove::<VSliderMarker>()
        .remove::<VSplitContainerMarker>()
        .remove::<VideoStreamPlayerMarker>();

    #[cfg(feature = "experimental-godot-api")]
    entity_commands
        .remove::<GraphEditMarker>()
        .remove::<GraphElementMarker>()
        .remove::<GraphFrameMarker>()
        .remove::<GraphNodeMarker>();
}

fn check_universal_node_types_comprehensive(
    entity_commands: &mut EntityCommands,
    node: &mut GodotNode,
) {
    if node.try_get::<godot::classes::AnimationMixer>().is_some() {
        entity_commands.insert(AnimationMixerMarker);
    }
    if node
        .try_get::<godot::classes::AudioStreamPlayer>()
        .is_some()
    {
        entity_commands.insert(AudioStreamPlayerMarker);
    }
    if node.try_get::<godot::classes::CanvasItem>().is_some() {
        entity_commands.insert(CanvasItemMarker);
    }
    if node.try_get::<godot::classes::CanvasLayer>().is_some() {
        entity_commands.insert(CanvasLayerMarker);
    }
    if node.try_get::<godot::classes::EditorFileSystem>().is_some() {
        entity_commands.insert(EditorFileSystemMarker);
    }
    if node.try_get::<godot::classes::EditorPlugin>().is_some() {
        entity_commands.insert(EditorPluginMarker);
    }
    if node
        .try_get::<godot::classes::EditorResourcePreview>()
        .is_some()
    {
        entity_commands.insert(EditorResourcePreviewMarker);
    }
    if node.try_get::<godot::classes::HttpRequest>().is_some() {
        entity_commands.insert(HTTPRequestMarker);
    }
    if node
        .try_get::<godot::classes::InstancePlaceholder>()
        .is_some()
    {
        entity_commands.insert(InstancePlaceholderMarker);
    }
    if node.try_get::<godot::classes::MissingNode>().is_some() {
        entity_commands.insert(MissingNodeMarker);
    }
    if node
        .try_get::<godot::classes::MultiplayerSpawner>()
        .is_some()
    {
        entity_commands.insert(MultiplayerSpawnerMarker);
    }
    if node
        .try_get::<godot::classes::MultiplayerSynchronizer>()
        .is_some()
    {
        entity_commands.insert(MultiplayerSynchronizerMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node
        .try_get::<godot::classes::NavigationAgent2D>()
        .is_some()
    {
        entity_commands.insert(NavigationAgent2DMarker);
    }
    #[cfg(feature = "experimental-godot-api")]
    if node
        .try_get::<godot::classes::NavigationAgent3D>()
        .is_some()
    {
        entity_commands.insert(NavigationAgent3DMarker);
    }
    if node.try_get::<godot::classes::Node3D>().is_some() {
        entity_commands.insert(Node3DMarker);
    }
    if node
        .try_get::<godot::classes::ResourcePreloader>()
        .is_some()
    {
        entity_commands.insert(ResourcePreloaderMarker);
    }
    if node
        .try_get::<godot::classes::ShaderGlobalsOverride>()
        .is_some()
    {
        entity_commands.insert(ShaderGlobalsOverrideMarker);
    }
    if node.try_get::<godot::classes::StatusIndicator>().is_some() {
        entity_commands.insert(StatusIndicatorMarker);
    }
    if node.try_get::<godot::classes::Timer>().is_some() {
        entity_commands.insert(TimerMarker);
    }
    if node.try_get::<godot::classes::Viewport>().is_some() {
        entity_commands.insert(ViewportMarker);
    }
    if node.try_get::<godot::classes::WorldEnvironment>().is_some() {
        entity_commands.insert(WorldEnvironmentMarker);
    }
}
fn remove_universal_node_types_comprehensive(
    entity_commands: &mut EntityCommands,
    _node: &mut GodotNode,
) {
    entity_commands
        .remove::<AnimationMixerMarker>()
        .remove::<AudioStreamPlayerMarker>()
        .remove::<CanvasItemMarker>()
        .remove::<CanvasLayerMarker>()
        .remove::<EditorFileSystemMarker>()
        .remove::<EditorPluginMarker>()
        .remove::<EditorResourcePreviewMarker>()
        .remove::<HTTPRequestMarker>()
        .remove::<InstancePlaceholderMarker>()
        .remove::<MissingNodeMarker>()
        .remove::<MultiplayerSpawnerMarker>()
        .remove::<MultiplayerSynchronizerMarker>()
        .remove::<Node3DMarker>()
        .remove::<ResourcePreloaderMarker>()
        .remove::<ShaderGlobalsOverrideMarker>()
        .remove::<StatusIndicatorMarker>()
        .remove::<TimerMarker>()
        .remove::<ViewportMarker>()
        .remove::<WorldEnvironmentMarker>();

    #[cfg(feature = "experimental-godot-api")]
    entity_commands
        .remove::<NavigationAgent2DMarker>()
        .remove::<NavigationAgent3DMarker>();
}
