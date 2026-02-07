//! 🤖 This file is generated. Changes to it will be lost.
//! To regenerate: uv run python -m godot_bevy_codegen

use crate::interop::{GodotNode, node_markers::*};
use bevy_ecs::system::EntityCommands;

/// Adds node type markers based on a pre-analyzed type string from GDScript.
/// This avoids FFI calls by using type information determined on the GDScript side.
/// This provides significant performance improvements by eliminating multiple
/// GodotNode::try_get calls for each node.
pub fn add_node_type_markers_from_string(ec: &mut EntityCommands, node_type: &str) {
    // Add appropriate markers based on the type string
    ec.insert(NodeMarker);

    match node_type {
        "Node" => {
            // NodeMarker added above for all nodes.
        }
        "AnimationMixer" => {
            ec.insert(AnimationMixerMarker);
        }
        "AudioStreamPlayer" => {
            ec.insert(AudioStreamPlayerMarker);
        }
        "CanvasItem" => {
            ec.insert(CanvasItemMarker);
        }
        "CanvasLayer" => {
            ec.insert(CanvasLayerMarker);
        }
        "EditorFileSystem" => {
            ec.insert(EditorFileSystemMarker);
        }
        "EditorPlugin" => {
            ec.insert(EditorPluginMarker);
        }
        "EditorResourcePreview" => {
            ec.insert(EditorResourcePreviewMarker);
        }
        "HTTPRequest" => {
            ec.insert(HTTPRequestMarker);
        }
        "InstancePlaceholder" => {
            ec.insert(InstancePlaceholderMarker);
        }
        "MissingNode" => {
            ec.insert(MissingNodeMarker);
        }
        "MultiplayerSpawner" => {
            ec.insert(MultiplayerSpawnerMarker);
        }
        "MultiplayerSynchronizer" => {
            ec.insert(MultiplayerSynchronizerMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationAgent2D" => {
            ec.insert(NavigationAgent2DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationAgent3D" => {
            ec.insert(NavigationAgent3DMarker);
        }
        "Node3D" => {
            ec.insert(Node3DMarker);
        }
        "ResourcePreloader" => {
            ec.insert(ResourcePreloaderMarker);
        }
        "ShaderGlobalsOverride" => {
            ec.insert(ShaderGlobalsOverrideMarker);
        }
        "SkeletonIK3D" => {
            ec.insert(SkeletonIK3DMarker);
        }
        "Timer" => {
            ec.insert(TimerMarker);
        }
        "Viewport" => {
            ec.insert(ViewportMarker);
        }
        "WorldEnvironment" => {
            ec.insert(WorldEnvironmentMarker);
        }
        "AnimationPlayer" => {
            ec.insert(AnimationPlayerMarker);
            ec.insert(AnimationMixerMarker);
        }
        "AnimationTree" => {
            ec.insert(AnimationTreeMarker);
            ec.insert(AnimationMixerMarker);
        }
        "AudioListener3D" => {
            ec.insert(AudioListener3DMarker);
            ec.insert(Node3DMarker);
        }
        "AudioStreamPlayer3D" => {
            ec.insert(AudioStreamPlayer3DMarker);
            ec.insert(Node3DMarker);
        }
        "BoneAttachment3D" => {
            ec.insert(BoneAttachment3DMarker);
            ec.insert(Node3DMarker);
        }
        "Camera3D" => {
            ec.insert(Camera3DMarker);
            ec.insert(Node3DMarker);
        }
        "CollisionObject3D" => {
            ec.insert(CollisionObject3DMarker);
            ec.insert(Node3DMarker);
        }
        "CollisionPolygon3D" => {
            ec.insert(CollisionPolygon3DMarker);
            ec.insert(Node3DMarker);
        }
        "CollisionShape3D" => {
            ec.insert(CollisionShape3DMarker);
            ec.insert(Node3DMarker);
        }
        "Control" => {
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "GridMap" => {
            ec.insert(GridMapMarker);
            ec.insert(Node3DMarker);
        }
        "ImporterMeshInstance3D" => {
            ec.insert(ImporterMeshInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "Joint3D" => {
            ec.insert(Joint3DMarker);
            ec.insert(Node3DMarker);
        }
        "LightmapProbe" => {
            ec.insert(LightmapProbeMarker);
            ec.insert(Node3DMarker);
        }
        "Marker3D" => {
            ec.insert(Marker3DMarker);
            ec.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationLink3D" => {
            ec.insert(NavigationLink3DMarker);
            ec.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationObstacle3D" => {
            ec.insert(NavigationObstacle3DMarker);
            ec.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationRegion3D" => {
            ec.insert(NavigationRegion3DMarker);
            ec.insert(Node3DMarker);
        }
        "Node2D" => {
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "OccluderInstance3D" => {
            ec.insert(OccluderInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "OpenXRHand" => {
            ec.insert(OpenXRHandMarker);
            ec.insert(Node3DMarker);
        }
        "ParallaxBackground" => {
            ec.insert(ParallaxBackgroundMarker);
            ec.insert(CanvasLayerMarker);
        }
        "Path3D" => {
            ec.insert(Path3DMarker);
            ec.insert(Node3DMarker);
        }
        "PathFollow3D" => {
            ec.insert(PathFollow3DMarker);
            ec.insert(Node3DMarker);
        }
        "RayCast3D" => {
            ec.insert(RayCast3DMarker);
            ec.insert(Node3DMarker);
        }
        "RemoteTransform3D" => {
            ec.insert(RemoteTransform3DMarker);
            ec.insert(Node3DMarker);
        }
        "ShapeCast3D" => {
            ec.insert(ShapeCast3DMarker);
            ec.insert(Node3DMarker);
        }
        "Skeleton3D" => {
            ec.insert(Skeleton3DMarker);
            ec.insert(Node3DMarker);
        }
        "SpringArm3D" => {
            ec.insert(SpringArm3DMarker);
            ec.insert(Node3DMarker);
        }
        "SubViewport" => {
            ec.insert(SubViewportMarker);
            ec.insert(ViewportMarker);
        }
        "VehicleWheel3D" => {
            ec.insert(VehicleWheel3DMarker);
            ec.insert(Node3DMarker);
        }
        "VisualInstance3D" => {
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "Window" => {
            ec.insert(WindowMarker);
            ec.insert(ViewportMarker);
        }
        "XRNode3D" => {
            ec.insert(XRNode3DMarker);
            ec.insert(Node3DMarker);
        }
        "XROrigin3D" => {
            ec.insert(XROrigin3DMarker);
            ec.insert(Node3DMarker);
        }
        "AcceptDialog" => {
            ec.insert(AcceptDialogMarker);
            ec.insert(WindowMarker);
            ec.insert(ViewportMarker);
        }
        "AnimatedSprite2D" => {
            ec.insert(AnimatedSprite2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "Area3D" => {
            ec.insert(Area3DMarker);
            ec.insert(CollisionObject3DMarker);
            ec.insert(Node3DMarker);
        }
        "AudioListener2D" => {
            ec.insert(AudioListener2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "AudioStreamPlayer2D" => {
            ec.insert(AudioStreamPlayer2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "BackBufferCopy" => {
            ec.insert(BackBufferCopyMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "BaseButton" => {
            ec.insert(BaseButtonMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "Bone2D" => {
            ec.insert(Bone2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "CPUParticles2D" => {
            ec.insert(CPUParticles2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "Camera2D" => {
            ec.insert(Camera2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "CanvasGroup" => {
            ec.insert(CanvasGroupMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "CanvasModulate" => {
            ec.insert(CanvasModulateMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "CollisionObject2D" => {
            ec.insert(CollisionObject2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "CollisionPolygon2D" => {
            ec.insert(CollisionPolygon2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "CollisionShape2D" => {
            ec.insert(CollisionShape2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "ColorRect" => {
            ec.insert(ColorRectMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "ConeTwistJoint3D" => {
            ec.insert(ConeTwistJoint3DMarker);
            ec.insert(Joint3DMarker);
            ec.insert(Node3DMarker);
        }
        "Container" => {
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "Decal" => {
            ec.insert(DecalMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "FogVolume" => {
            ec.insert(FogVolumeMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "GPUParticles2D" => {
            ec.insert(GPUParticles2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "GPUParticlesAttractor3D" => {
            ec.insert(GPUParticlesAttractor3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "GPUParticlesCollision3D" => {
            ec.insert(GPUParticlesCollision3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "Generic6DOFJoint3D" => {
            ec.insert(Generic6DOFJoint3DMarker);
            ec.insert(Joint3DMarker);
            ec.insert(Node3DMarker);
        }
        "GeometryInstance3D" => {
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphEdit" => {
            ec.insert(GraphEditMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "HingeJoint3D" => {
            ec.insert(HingeJoint3DMarker);
            ec.insert(Joint3DMarker);
            ec.insert(Node3DMarker);
        }
        "ItemList" => {
            ec.insert(ItemListMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "Joint2D" => {
            ec.insert(Joint2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "Label" => {
            ec.insert(LabelMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "Light2D" => {
            ec.insert(Light2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "Light3D" => {
            ec.insert(Light3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "LightOccluder2D" => {
            ec.insert(LightOccluder2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "LightmapGI" => {
            ec.insert(LightmapGIMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "Line2D" => {
            ec.insert(Line2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "LineEdit" => {
            ec.insert(LineEditMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "Marker2D" => {
            ec.insert(Marker2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "MenuBar" => {
            ec.insert(MenuBarMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "MeshInstance2D" => {
            ec.insert(MeshInstance2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "MultiMeshInstance2D" => {
            ec.insert(MultiMeshInstance2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationLink2D" => {
            ec.insert(NavigationLink2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationObstacle2D" => {
            ec.insert(NavigationObstacle2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationRegion2D" => {
            ec.insert(NavigationRegion2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "NinePatchRect" => {
            ec.insert(NinePatchRectMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "Panel" => {
            ec.insert(PanelMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "ParallaxLayer" => {
            ec.insert(ParallaxLayerMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "Path2D" => {
            ec.insert(Path2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "PathFollow2D" => {
            ec.insert(PathFollow2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "PhysicsBody3D" => {
            ec.insert(PhysicsBody3DMarker);
            ec.insert(CollisionObject3DMarker);
            ec.insert(Node3DMarker);
        }
        "PinJoint3D" => {
            ec.insert(PinJoint3DMarker);
            ec.insert(Joint3DMarker);
            ec.insert(Node3DMarker);
        }
        "Polygon2D" => {
            ec.insert(Polygon2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "Popup" => {
            ec.insert(PopupMarker);
            ec.insert(WindowMarker);
            ec.insert(ViewportMarker);
        }
        "Range" => {
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "RayCast2D" => {
            ec.insert(RayCast2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "ReferenceRect" => {
            ec.insert(ReferenceRectMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "ReflectionProbe" => {
            ec.insert(ReflectionProbeMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "RemoteTransform2D" => {
            ec.insert(RemoteTransform2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "RichTextLabel" => {
            ec.insert(RichTextLabelMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "RootMotionView" => {
            ec.insert(RootMotionViewMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "Separator" => {
            ec.insert(SeparatorMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "ShapeCast2D" => {
            ec.insert(ShapeCast2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "Skeleton2D" => {
            ec.insert(Skeleton2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "SliderJoint3D" => {
            ec.insert(SliderJoint3DMarker);
            ec.insert(Joint3DMarker);
            ec.insert(Node3DMarker);
        }
        "Sprite2D" => {
            ec.insert(Sprite2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "TabBar" => {
            ec.insert(TabBarMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "TextEdit" => {
            ec.insert(TextEditMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "TextureRect" => {
            ec.insert(TextureRectMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "TileMap" => {
            ec.insert(TileMapMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "TouchScreenButton" => {
            ec.insert(TouchScreenButtonMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "Tree" => {
            ec.insert(TreeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "VideoStreamPlayer" => {
            ec.insert(VideoStreamPlayerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "VisibleOnScreenNotifier2D" => {
            ec.insert(VisibleOnScreenNotifier2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "VisibleOnScreenNotifier3D" => {
            ec.insert(VisibleOnScreenNotifier3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "VoxelGI" => {
            ec.insert(VoxelGIMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "XRAnchor3D" => {
            ec.insert(XRAnchor3DMarker);
            ec.insert(XRNode3DMarker);
            ec.insert(Node3DMarker);
        }
        "XRCamera3D" => {
            ec.insert(XRCamera3DMarker);
            ec.insert(Camera3DMarker);
            ec.insert(Node3DMarker);
        }
        "XRController3D" => {
            ec.insert(XRController3DMarker);
            ec.insert(XRNode3DMarker);
            ec.insert(Node3DMarker);
        }
        "Area2D" => {
            ec.insert(Area2DMarker);
            ec.insert(CollisionObject2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "AspectRatioContainer" => {
            ec.insert(AspectRatioContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "BoxContainer" => {
            ec.insert(BoxContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "Button" => {
            ec.insert(ButtonMarker);
            ec.insert(BaseButtonMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "CPUParticles3D" => {
            ec.insert(CPUParticles3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "CSGShape3D" => {
            ec.insert(CSGShape3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "CenterContainer" => {
            ec.insert(CenterContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "CharacterBody3D" => {
            ec.insert(CharacterBody3DMarker);
            ec.insert(PhysicsBody3DMarker);
            ec.insert(CollisionObject3DMarker);
            ec.insert(Node3DMarker);
        }
        "CodeEdit" => {
            ec.insert(CodeEditMarker);
            ec.insert(TextEditMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "ConfirmationDialog" => {
            ec.insert(ConfirmationDialogMarker);
            ec.insert(AcceptDialogMarker);
            ec.insert(WindowMarker);
            ec.insert(ViewportMarker);
        }
        "DampedSpringJoint2D" => {
            ec.insert(DampedSpringJoint2DMarker);
            ec.insert(Joint2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "DirectionalLight2D" => {
            ec.insert(DirectionalLight2DMarker);
            ec.insert(Light2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "DirectionalLight3D" => {
            ec.insert(DirectionalLight3DMarker);
            ec.insert(Light3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "EditorProperty" => {
            ec.insert(EditorPropertyMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "EditorSpinSlider" => {
            ec.insert(EditorSpinSliderMarker);
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "FlowContainer" => {
            ec.insert(FlowContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "GPUParticles3D" => {
            ec.insert(GPUParticles3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "GPUParticlesAttractorBox3D" => {
            ec.insert(GPUParticlesAttractorBox3DMarker);
            ec.insert(GPUParticlesAttractor3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "GPUParticlesAttractorSphere3D" => {
            ec.insert(GPUParticlesAttractorSphere3DMarker);
            ec.insert(GPUParticlesAttractor3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "GPUParticlesAttractorVectorField3D" => {
            ec.insert(GPUParticlesAttractorVectorField3DMarker);
            ec.insert(GPUParticlesAttractor3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "GPUParticlesCollisionBox3D" => {
            ec.insert(GPUParticlesCollisionBox3DMarker);
            ec.insert(GPUParticlesCollision3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "GPUParticlesCollisionHeightField3D" => {
            ec.insert(GPUParticlesCollisionHeightField3DMarker);
            ec.insert(GPUParticlesCollision3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "GPUParticlesCollisionSDF3D" => {
            ec.insert(GPUParticlesCollisionSDF3DMarker);
            ec.insert(GPUParticlesCollision3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "GPUParticlesCollisionSphere3D" => {
            ec.insert(GPUParticlesCollisionSphere3DMarker);
            ec.insert(GPUParticlesCollision3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphElement" => {
            ec.insert(GraphElementMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "GridContainer" => {
            ec.insert(GridContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "GrooveJoint2D" => {
            ec.insert(GrooveJoint2DMarker);
            ec.insert(Joint2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "HSeparator" => {
            ec.insert(HSeparatorMarker);
            ec.insert(SeparatorMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "Label3D" => {
            ec.insert(Label3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "LinkButton" => {
            ec.insert(LinkButtonMarker);
            ec.insert(BaseButtonMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "MarginContainer" => {
            ec.insert(MarginContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "MeshInstance3D" => {
            ec.insert(MeshInstance3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "MultiMeshInstance3D" => {
            ec.insert(MultiMeshInstance3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "OmniLight3D" => {
            ec.insert(OmniLight3DMarker);
            ec.insert(Light3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "PanelContainer" => {
            ec.insert(PanelContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "PhysicalBone3D" => {
            ec.insert(PhysicalBone3DMarker);
            ec.insert(PhysicsBody3DMarker);
            ec.insert(CollisionObject3DMarker);
            ec.insert(Node3DMarker);
        }
        "PhysicsBody2D" => {
            ec.insert(PhysicsBody2DMarker);
            ec.insert(CollisionObject2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "PinJoint2D" => {
            ec.insert(PinJoint2DMarker);
            ec.insert(Joint2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "PointLight2D" => {
            ec.insert(PointLight2DMarker);
            ec.insert(Light2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "PopupMenu" => {
            ec.insert(PopupMenuMarker);
            ec.insert(PopupMarker);
            ec.insert(WindowMarker);
            ec.insert(ViewportMarker);
        }
        "PopupPanel" => {
            ec.insert(PopupPanelMarker);
            ec.insert(PopupMarker);
            ec.insert(WindowMarker);
            ec.insert(ViewportMarker);
        }
        "ProgressBar" => {
            ec.insert(ProgressBarMarker);
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "RigidBody3D" => {
            ec.insert(RigidBody3DMarker);
            ec.insert(PhysicsBody3DMarker);
            ec.insert(CollisionObject3DMarker);
            ec.insert(Node3DMarker);
        }
        "ScrollBar" => {
            ec.insert(ScrollBarMarker);
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "ScrollContainer" => {
            ec.insert(ScrollContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "Slider" => {
            ec.insert(SliderMarker);
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "SpinBox" => {
            ec.insert(SpinBoxMarker);
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "SplitContainer" => {
            ec.insert(SplitContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "SpotLight3D" => {
            ec.insert(SpotLight3DMarker);
            ec.insert(Light3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "SpriteBase3D" => {
            ec.insert(SpriteBase3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "StaticBody3D" => {
            ec.insert(StaticBody3DMarker);
            ec.insert(PhysicsBody3DMarker);
            ec.insert(CollisionObject3DMarker);
            ec.insert(Node3DMarker);
        }
        "SubViewportContainer" => {
            ec.insert(SubViewportContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "TabContainer" => {
            ec.insert(TabContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "TextureButton" => {
            ec.insert(TextureButtonMarker);
            ec.insert(BaseButtonMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "TextureProgressBar" => {
            ec.insert(TextureProgressBarMarker);
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "VSeparator" => {
            ec.insert(VSeparatorMarker);
            ec.insert(SeparatorMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "VisibleOnScreenEnabler2D" => {
            ec.insert(VisibleOnScreenEnabler2DMarker);
            ec.insert(VisibleOnScreenNotifier2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "VisibleOnScreenEnabler3D" => {
            ec.insert(VisibleOnScreenEnabler3DMarker);
            ec.insert(VisibleOnScreenNotifier3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "AnimatableBody3D" => {
            ec.insert(AnimatableBody3DMarker);
            ec.insert(StaticBody3DMarker);
            ec.insert(PhysicsBody3DMarker);
            ec.insert(CollisionObject3DMarker);
            ec.insert(Node3DMarker);
        }
        "AnimatedSprite3D" => {
            ec.insert(AnimatedSprite3DMarker);
            ec.insert(SpriteBase3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "CSGCombiner3D" => {
            ec.insert(CSGCombiner3DMarker);
            ec.insert(CSGShape3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "CSGPrimitive3D" => {
            ec.insert(CSGPrimitive3DMarker);
            ec.insert(CSGShape3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "CharacterBody2D" => {
            ec.insert(CharacterBody2DMarker);
            ec.insert(PhysicsBody2DMarker);
            ec.insert(CollisionObject2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "CheckBox" => {
            ec.insert(CheckBoxMarker);
            ec.insert(ButtonMarker);
            ec.insert(BaseButtonMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "CheckButton" => {
            ec.insert(CheckButtonMarker);
            ec.insert(ButtonMarker);
            ec.insert(BaseButtonMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "ColorPickerButton" => {
            ec.insert(ColorPickerButtonMarker);
            ec.insert(ButtonMarker);
            ec.insert(BaseButtonMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "EditorCommandPalette" => {
            ec.insert(EditorCommandPaletteMarker);
            ec.insert(ConfirmationDialogMarker);
            ec.insert(AcceptDialogMarker);
            ec.insert(WindowMarker);
            ec.insert(ViewportMarker);
        }
        "EditorFileDialog" => {
            ec.insert(EditorFileDialogMarker);
            ec.insert(ConfirmationDialogMarker);
            ec.insert(AcceptDialogMarker);
            ec.insert(WindowMarker);
            ec.insert(ViewportMarker);
        }
        "EditorInspector" => {
            ec.insert(EditorInspectorMarker);
            ec.insert(ScrollContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "FileDialog" => {
            ec.insert(FileDialogMarker);
            ec.insert(ConfirmationDialogMarker);
            ec.insert(AcceptDialogMarker);
            ec.insert(WindowMarker);
            ec.insert(ViewportMarker);
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphNode" => {
            ec.insert(GraphNodeMarker);
            ec.insert(GraphElementMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "HBoxContainer" => {
            ec.insert(HBoxContainerMarker);
            ec.insert(BoxContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "HFlowContainer" => {
            ec.insert(HFlowContainerMarker);
            ec.insert(FlowContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "HScrollBar" => {
            ec.insert(HScrollBarMarker);
            ec.insert(ScrollBarMarker);
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "HSlider" => {
            ec.insert(HSliderMarker);
            ec.insert(SliderMarker);
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "HSplitContainer" => {
            ec.insert(HSplitContainerMarker);
            ec.insert(SplitContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "MenuButton" => {
            ec.insert(MenuButtonMarker);
            ec.insert(ButtonMarker);
            ec.insert(BaseButtonMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "OptionButton" => {
            ec.insert(OptionButtonMarker);
            ec.insert(ButtonMarker);
            ec.insert(BaseButtonMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "RigidBody2D" => {
            ec.insert(RigidBody2DMarker);
            ec.insert(PhysicsBody2DMarker);
            ec.insert(CollisionObject2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "ScriptCreateDialog" => {
            ec.insert(ScriptCreateDialogMarker);
            ec.insert(ConfirmationDialogMarker);
            ec.insert(AcceptDialogMarker);
            ec.insert(WindowMarker);
            ec.insert(ViewportMarker);
        }
        "ScriptEditor" => {
            ec.insert(ScriptEditorMarker);
            ec.insert(PanelContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "SoftBody3D" => {
            ec.insert(SoftBody3DMarker);
            ec.insert(MeshInstance3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "Sprite3D" => {
            ec.insert(Sprite3DMarker);
            ec.insert(SpriteBase3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "StaticBody2D" => {
            ec.insert(StaticBody2DMarker);
            ec.insert(PhysicsBody2DMarker);
            ec.insert(CollisionObject2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "VBoxContainer" => {
            ec.insert(VBoxContainerMarker);
            ec.insert(BoxContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "VFlowContainer" => {
            ec.insert(VFlowContainerMarker);
            ec.insert(FlowContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "VScrollBar" => {
            ec.insert(VScrollBarMarker);
            ec.insert(ScrollBarMarker);
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "VSlider" => {
            ec.insert(VSliderMarker);
            ec.insert(SliderMarker);
            ec.insert(RangeMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "VSplitContainer" => {
            ec.insert(VSplitContainerMarker);
            ec.insert(SplitContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "VehicleBody3D" => {
            ec.insert(VehicleBody3DMarker);
            ec.insert(RigidBody3DMarker);
            ec.insert(PhysicsBody3DMarker);
            ec.insert(CollisionObject3DMarker);
            ec.insert(Node3DMarker);
        }
        "AnimatableBody2D" => {
            ec.insert(AnimatableBody2DMarker);
            ec.insert(StaticBody2DMarker);
            ec.insert(PhysicsBody2DMarker);
            ec.insert(CollisionObject2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "CSGBox3D" => {
            ec.insert(CSGBox3DMarker);
            ec.insert(CSGPrimitive3DMarker);
            ec.insert(CSGShape3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "CSGCylinder3D" => {
            ec.insert(CSGCylinder3DMarker);
            ec.insert(CSGPrimitive3DMarker);
            ec.insert(CSGShape3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "CSGMesh3D" => {
            ec.insert(CSGMesh3DMarker);
            ec.insert(CSGPrimitive3DMarker);
            ec.insert(CSGShape3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "CSGPolygon3D" => {
            ec.insert(CSGPolygon3DMarker);
            ec.insert(CSGPrimitive3DMarker);
            ec.insert(CSGShape3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "CSGSphere3D" => {
            ec.insert(CSGSphere3DMarker);
            ec.insert(CSGPrimitive3DMarker);
            ec.insert(CSGShape3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "CSGTorus3D" => {
            ec.insert(CSGTorus3DMarker);
            ec.insert(CSGPrimitive3DMarker);
            ec.insert(CSGShape3DMarker);
            ec.insert(GeometryInstance3DMarker);
            ec.insert(VisualInstance3DMarker);
            ec.insert(Node3DMarker);
        }
        "ColorPicker" => {
            ec.insert(ColorPickerMarker);
            ec.insert(VBoxContainerMarker);
            ec.insert(BoxContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "EditorResourcePicker" => {
            ec.insert(EditorResourcePickerMarker);
            ec.insert(HBoxContainerMarker);
            ec.insert(BoxContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "FileSystemDock" => {
            ec.insert(FileSystemDockMarker);
            ec.insert(VBoxContainerMarker);
            ec.insert(BoxContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "PhysicalBone2D" => {
            ec.insert(PhysicalBone2DMarker);
            ec.insert(RigidBody2DMarker);
            ec.insert(PhysicsBody2DMarker);
            ec.insert(CollisionObject2DMarker);
            ec.insert(Node2DMarker);
            ec.insert(CanvasItemMarker);
        }
        "ScriptEditorBase" => {
            ec.insert(ScriptEditorBaseMarker);
            ec.insert(VBoxContainerMarker);
            ec.insert(BoxContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        "EditorScriptPicker" => {
            ec.insert(EditorScriptPickerMarker);
            ec.insert(EditorResourcePickerMarker);
            ec.insert(HBoxContainerMarker);
            ec.insert(BoxContainerMarker);
            ec.insert(ContainerMarker);
            ec.insert(ControlMarker);
            ec.insert(CanvasItemMarker);
        }
        // Custom user types that extend Godot nodes
        _ => {}
    }
}

pub fn remove_comprehensive_node_type_markers(ec: &mut EntityCommands) {
    // All nodes inherit from Node, so remove this first
    ec.remove::<NodeMarker>();
    ec.remove::<AnimationMixerMarker>();
    ec.remove::<AudioStreamPlayerMarker>();
    ec.remove::<CanvasItemMarker>();
    ec.remove::<CanvasLayerMarker>();
    ec.remove::<EditorFileSystemMarker>();
    ec.remove::<EditorPluginMarker>();
    ec.remove::<EditorResourcePreviewMarker>();
    ec.remove::<HTTPRequestMarker>();
    ec.remove::<InstancePlaceholderMarker>();
    ec.remove::<MissingNodeMarker>();
    ec.remove::<MultiplayerSpawnerMarker>();
    ec.remove::<MultiplayerSynchronizerMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationAgent2DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationAgent3DMarker>();
    ec.remove::<Node3DMarker>();
    ec.remove::<ResourcePreloaderMarker>();
    ec.remove::<ShaderGlobalsOverrideMarker>();
    ec.remove::<SkeletonIK3DMarker>();
    ec.remove::<TimerMarker>();
    ec.remove::<ViewportMarker>();
    ec.remove::<WorldEnvironmentMarker>();
    ec.remove::<AnimationPlayerMarker>();
    ec.remove::<AnimationTreeMarker>();
    ec.remove::<AudioListener3DMarker>();
    ec.remove::<AudioStreamPlayer3DMarker>();
    ec.remove::<BoneAttachment3DMarker>();
    ec.remove::<Camera3DMarker>();
    ec.remove::<CollisionObject3DMarker>();
    ec.remove::<CollisionPolygon3DMarker>();
    ec.remove::<CollisionShape3DMarker>();
    ec.remove::<ControlMarker>();
    ec.remove::<GridMapMarker>();
    ec.remove::<ImporterMeshInstance3DMarker>();
    ec.remove::<Joint3DMarker>();
    ec.remove::<LightmapProbeMarker>();
    ec.remove::<Marker3DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationLink3DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationObstacle3DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationRegion3DMarker>();
    ec.remove::<Node2DMarker>();
    ec.remove::<OccluderInstance3DMarker>();
    ec.remove::<OpenXRHandMarker>();
    ec.remove::<ParallaxBackgroundMarker>();
    ec.remove::<Path3DMarker>();
    ec.remove::<PathFollow3DMarker>();
    ec.remove::<RayCast3DMarker>();
    ec.remove::<RemoteTransform3DMarker>();
    ec.remove::<ShapeCast3DMarker>();
    ec.remove::<Skeleton3DMarker>();
    ec.remove::<SpringArm3DMarker>();
    ec.remove::<SubViewportMarker>();
    ec.remove::<VehicleWheel3DMarker>();
    ec.remove::<VisualInstance3DMarker>();
    ec.remove::<WindowMarker>();
    ec.remove::<XRNode3DMarker>();
    ec.remove::<XROrigin3DMarker>();
    ec.remove::<AcceptDialogMarker>();
    ec.remove::<AnimatedSprite2DMarker>();
    ec.remove::<Area3DMarker>();
    ec.remove::<AudioListener2DMarker>();
    ec.remove::<AudioStreamPlayer2DMarker>();
    ec.remove::<BackBufferCopyMarker>();
    ec.remove::<BaseButtonMarker>();
    ec.remove::<Bone2DMarker>();
    ec.remove::<CPUParticles2DMarker>();
    ec.remove::<Camera2DMarker>();
    ec.remove::<CanvasGroupMarker>();
    ec.remove::<CanvasModulateMarker>();
    ec.remove::<CollisionObject2DMarker>();
    ec.remove::<CollisionPolygon2DMarker>();
    ec.remove::<CollisionShape2DMarker>();
    ec.remove::<ColorRectMarker>();
    ec.remove::<ConeTwistJoint3DMarker>();
    ec.remove::<ContainerMarker>();
    ec.remove::<DecalMarker>();
    ec.remove::<FogVolumeMarker>();
    ec.remove::<GPUParticles2DMarker>();
    ec.remove::<GPUParticlesAttractor3DMarker>();
    ec.remove::<GPUParticlesCollision3DMarker>();
    ec.remove::<Generic6DOFJoint3DMarker>();
    ec.remove::<GeometryInstance3DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<GraphEditMarker>();
    ec.remove::<HingeJoint3DMarker>();
    ec.remove::<ItemListMarker>();
    ec.remove::<Joint2DMarker>();
    ec.remove::<LabelMarker>();
    ec.remove::<Light2DMarker>();
    ec.remove::<Light3DMarker>();
    ec.remove::<LightOccluder2DMarker>();
    ec.remove::<LightmapGIMarker>();
    ec.remove::<Line2DMarker>();
    ec.remove::<LineEditMarker>();
    ec.remove::<Marker2DMarker>();
    ec.remove::<MenuBarMarker>();
    ec.remove::<MeshInstance2DMarker>();
    ec.remove::<MultiMeshInstance2DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationLink2DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationObstacle2DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationRegion2DMarker>();
    ec.remove::<NinePatchRectMarker>();
    ec.remove::<PanelMarker>();
    ec.remove::<ParallaxLayerMarker>();
    ec.remove::<Path2DMarker>();
    ec.remove::<PathFollow2DMarker>();
    ec.remove::<PhysicsBody3DMarker>();
    ec.remove::<PinJoint3DMarker>();
    ec.remove::<Polygon2DMarker>();
    ec.remove::<PopupMarker>();
    ec.remove::<RangeMarker>();
    ec.remove::<RayCast2DMarker>();
    ec.remove::<ReferenceRectMarker>();
    ec.remove::<ReflectionProbeMarker>();
    ec.remove::<RemoteTransform2DMarker>();
    ec.remove::<RichTextLabelMarker>();
    ec.remove::<RootMotionViewMarker>();
    ec.remove::<SeparatorMarker>();
    ec.remove::<ShapeCast2DMarker>();
    ec.remove::<Skeleton2DMarker>();
    ec.remove::<SliderJoint3DMarker>();
    ec.remove::<Sprite2DMarker>();
    ec.remove::<TabBarMarker>();
    ec.remove::<TextEditMarker>();
    ec.remove::<TextureRectMarker>();
    ec.remove::<TileMapMarker>();
    ec.remove::<TouchScreenButtonMarker>();
    ec.remove::<TreeMarker>();
    ec.remove::<VideoStreamPlayerMarker>();
    ec.remove::<VisibleOnScreenNotifier2DMarker>();
    ec.remove::<VisibleOnScreenNotifier3DMarker>();
    ec.remove::<VoxelGIMarker>();
    ec.remove::<XRAnchor3DMarker>();
    ec.remove::<XRCamera3DMarker>();
    ec.remove::<XRController3DMarker>();
    ec.remove::<Area2DMarker>();
    ec.remove::<AspectRatioContainerMarker>();
    ec.remove::<BoxContainerMarker>();
    ec.remove::<ButtonMarker>();
    ec.remove::<CPUParticles3DMarker>();
    ec.remove::<CSGShape3DMarker>();
    ec.remove::<CenterContainerMarker>();
    ec.remove::<CharacterBody3DMarker>();
    ec.remove::<CodeEditMarker>();
    ec.remove::<ConfirmationDialogMarker>();
    ec.remove::<DampedSpringJoint2DMarker>();
    ec.remove::<DirectionalLight2DMarker>();
    ec.remove::<DirectionalLight3DMarker>();
    ec.remove::<EditorPropertyMarker>();
    ec.remove::<EditorSpinSliderMarker>();
    ec.remove::<FlowContainerMarker>();
    ec.remove::<GPUParticles3DMarker>();
    ec.remove::<GPUParticlesAttractorBox3DMarker>();
    ec.remove::<GPUParticlesAttractorSphere3DMarker>();
    ec.remove::<GPUParticlesAttractorVectorField3DMarker>();
    ec.remove::<GPUParticlesCollisionBox3DMarker>();
    ec.remove::<GPUParticlesCollisionHeightField3DMarker>();
    ec.remove::<GPUParticlesCollisionSDF3DMarker>();
    ec.remove::<GPUParticlesCollisionSphere3DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<GraphElementMarker>();
    ec.remove::<GridContainerMarker>();
    ec.remove::<GrooveJoint2DMarker>();
    ec.remove::<HSeparatorMarker>();
    ec.remove::<Label3DMarker>();
    ec.remove::<LinkButtonMarker>();
    ec.remove::<MarginContainerMarker>();
    ec.remove::<MeshInstance3DMarker>();
    ec.remove::<MultiMeshInstance3DMarker>();
    ec.remove::<OmniLight3DMarker>();
    ec.remove::<PanelContainerMarker>();
    ec.remove::<PhysicalBone3DMarker>();
    ec.remove::<PhysicsBody2DMarker>();
    ec.remove::<PinJoint2DMarker>();
    ec.remove::<PointLight2DMarker>();
    ec.remove::<PopupMenuMarker>();
    ec.remove::<PopupPanelMarker>();
    ec.remove::<ProgressBarMarker>();
    ec.remove::<RigidBody3DMarker>();
    ec.remove::<ScrollBarMarker>();
    ec.remove::<ScrollContainerMarker>();
    ec.remove::<SliderMarker>();
    ec.remove::<SpinBoxMarker>();
    ec.remove::<SplitContainerMarker>();
    ec.remove::<SpotLight3DMarker>();
    ec.remove::<SpriteBase3DMarker>();
    ec.remove::<StaticBody3DMarker>();
    ec.remove::<SubViewportContainerMarker>();
    ec.remove::<TabContainerMarker>();
    ec.remove::<TextureButtonMarker>();
    ec.remove::<TextureProgressBarMarker>();
    ec.remove::<VSeparatorMarker>();
    ec.remove::<VisibleOnScreenEnabler2DMarker>();
    ec.remove::<VisibleOnScreenEnabler3DMarker>();
    ec.remove::<AnimatableBody3DMarker>();
    ec.remove::<AnimatedSprite3DMarker>();
    ec.remove::<CSGCombiner3DMarker>();
    ec.remove::<CSGPrimitive3DMarker>();
    ec.remove::<CharacterBody2DMarker>();
    ec.remove::<CheckBoxMarker>();
    ec.remove::<CheckButtonMarker>();
    ec.remove::<ColorPickerButtonMarker>();
    ec.remove::<EditorCommandPaletteMarker>();
    ec.remove::<EditorFileDialogMarker>();
    ec.remove::<EditorInspectorMarker>();
    ec.remove::<FileDialogMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<GraphNodeMarker>();
    ec.remove::<HBoxContainerMarker>();
    ec.remove::<HFlowContainerMarker>();
    ec.remove::<HScrollBarMarker>();
    ec.remove::<HSliderMarker>();
    ec.remove::<HSplitContainerMarker>();
    ec.remove::<MenuButtonMarker>();
    ec.remove::<OptionButtonMarker>();
    ec.remove::<RigidBody2DMarker>();
    ec.remove::<ScriptCreateDialogMarker>();
    ec.remove::<ScriptEditorMarker>();
    ec.remove::<SoftBody3DMarker>();
    ec.remove::<Sprite3DMarker>();
    ec.remove::<StaticBody2DMarker>();
    ec.remove::<VBoxContainerMarker>();
    ec.remove::<VFlowContainerMarker>();
    ec.remove::<VScrollBarMarker>();
    ec.remove::<VSliderMarker>();
    ec.remove::<VSplitContainerMarker>();
    ec.remove::<VehicleBody3DMarker>();
    ec.remove::<AnimatableBody2DMarker>();
    ec.remove::<CSGBox3DMarker>();
    ec.remove::<CSGCylinder3DMarker>();
    ec.remove::<CSGMesh3DMarker>();
    ec.remove::<CSGPolygon3DMarker>();
    ec.remove::<CSGSphere3DMarker>();
    ec.remove::<CSGTorus3DMarker>();
    ec.remove::<ColorPickerMarker>();
    ec.remove::<EditorResourcePickerMarker>();
    ec.remove::<FileSystemDockMarker>();
    ec.remove::<PhysicalBone2DMarker>();
    ec.remove::<ScriptEditorBaseMarker>();
    ec.remove::<EditorScriptPickerMarker>();
}
