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
    match node_type {
        "Node" => ec.insert(NodeMarker),
        "AnimationMixer" => ec.insert(AnimationMixerMarker),
        "AudioStreamPlayer" => ec.insert(AudioStreamPlayerMarker),
        "CanvasItem" => ec.insert(CanvasItemMarker),
        "CanvasLayer" => ec.insert(CanvasLayerMarker),
        "EditorFileSystem" => ec.insert(EditorFileSystemMarker),
        "EditorPlugin" => ec.insert(EditorPluginMarker),
        "EditorResourcePreview" => ec.insert(EditorResourcePreviewMarker),
        "HTTPRequest" => ec.insert(HTTPRequestMarker),
        "InstancePlaceholder" => ec.insert(InstancePlaceholderMarker),
        "MissingNode" => ec.insert(MissingNodeMarker),
        "MultiplayerSpawner" => ec.insert(MultiplayerSpawnerMarker),
        "MultiplayerSynchronizer" => ec.insert(MultiplayerSynchronizerMarker),
        #[cfg(feature = "experimental-godot-api")]
        "NavigationAgent2D" => ec.insert(NavigationAgent2DMarker),
        #[cfg(feature = "experimental-godot-api")]
        "NavigationAgent3D" => ec.insert(NavigationAgent3DMarker),
        "Node3D" => ec.insert(Node3DMarker),
        "ResourcePreloader" => ec.insert(ResourcePreloaderMarker),
        "ShaderGlobalsOverride" => ec.insert(ShaderGlobalsOverrideMarker),
        "StatusIndicator" => ec.insert(StatusIndicatorMarker),
        "Timer" => ec.insert(TimerMarker),
        "Viewport" => ec.insert(ViewportMarker),
        "WorldEnvironment" => ec.insert(WorldEnvironmentMarker),
        "AnimationPlayer" => ec.insert(AnimationPlayerMarker),
        "AnimationTree" => ec.insert(AnimationTreeMarker),
        "AudioListener3D" => ec.insert(AudioListener3DMarker),
        "AudioStreamPlayer3D" => ec.insert(AudioStreamPlayer3DMarker),
        "BoneAttachment3D" => ec.insert(BoneAttachment3DMarker),
        "Camera3D" => ec.insert(Camera3DMarker),
        "CollisionObject3D" => ec.insert(CollisionObject3DMarker),
        "CollisionPolygon3D" => ec.insert(CollisionPolygon3DMarker),
        "CollisionShape3D" => ec.insert(CollisionShape3DMarker),
        "Control" => ec.insert(ControlMarker),
        "GridMap" => ec.insert(GridMapMarker),
        "GridMapEditorPlugin" => ec.insert(GridMapEditorPluginMarker),
        "ImporterMeshInstance3D" => ec.insert(ImporterMeshInstance3DMarker),
        "Joint3D" => ec.insert(Joint3DMarker),
        "LightmapProbe" => ec.insert(LightmapProbeMarker),
        "Marker3D" => ec.insert(Marker3DMarker),
        #[cfg(feature = "experimental-godot-api")]
        "NavigationLink3D" => ec.insert(NavigationLink3DMarker),
        #[cfg(feature = "experimental-godot-api")]
        "NavigationObstacle3D" => ec.insert(NavigationObstacle3DMarker),
        #[cfg(feature = "experimental-godot-api")]
        "NavigationRegion3D" => ec.insert(NavigationRegion3DMarker),
        "Node2D" => ec.insert(Node2DMarker),
        "OpenXRCompositionLayer" => ec.insert(OpenXRCompositionLayerMarker),
        "OpenXRHand" => ec.insert(OpenXRHandMarker),
        #[cfg(not(feature = "experimental-wasm"))]
        "OpenXRRenderModel" => ec.insert(OpenXRRenderModelMarker),
        #[cfg(not(feature = "experimental-wasm"))]
        "OpenXRRenderModelManager" => ec.insert(OpenXRRenderModelManagerMarker),
        "ParallaxBackground" => ec.insert(ParallaxBackgroundMarker),
        "Path3D" => ec.insert(Path3DMarker),
        "PathFollow3D" => ec.insert(PathFollow3DMarker),
        "RayCast3D" => ec.insert(RayCast3DMarker),
        "RemoteTransform3D" => ec.insert(RemoteTransform3DMarker),
        "ShapeCast3D" => ec.insert(ShapeCast3DMarker),
        "Skeleton3D" => ec.insert(Skeleton3DMarker),
        "SkeletonModifier3D" => ec.insert(SkeletonModifier3DMarker),
        "SpringArm3D" => ec.insert(SpringArm3DMarker),
        "SpringBoneCollision3D" => ec.insert(SpringBoneCollision3DMarker),
        "SubViewport" => ec.insert(SubViewportMarker),
        "VehicleWheel3D" => ec.insert(VehicleWheel3DMarker),
        "VisualInstance3D" => ec.insert(VisualInstance3DMarker),
        "Window" => ec.insert(WindowMarker),
        #[cfg(feature = "experimental-godot-api")]
        "XRFaceModifier3D" => ec.insert(XRFaceModifier3DMarker),
        "XRNode3D" => ec.insert(XRNode3DMarker),
        "XROrigin3D" => ec.insert(XROrigin3DMarker),
        "AcceptDialog" => ec.insert(AcceptDialogMarker),
        "AnimatedSprite2D" => ec.insert(AnimatedSprite2DMarker),
        "Area3D" => ec.insert(Area3DMarker),
        "AudioListener2D" => ec.insert(AudioListener2DMarker),
        "AudioStreamPlayer2D" => ec.insert(AudioStreamPlayer2DMarker),
        "BackBufferCopy" => ec.insert(BackBufferCopyMarker),
        "BaseButton" => ec.insert(BaseButtonMarker),
        "Bone2D" => ec.insert(Bone2DMarker),
        "BoneConstraint3D" => ec.insert(BoneConstraint3DMarker),
        "BoneTwistDisperser3D" => ec.insert(BoneTwistDisperser3DMarker),
        "CPUParticles2D" => ec.insert(CPUParticles2DMarker),
        "Camera2D" => ec.insert(Camera2DMarker),
        "CanvasGroup" => ec.insert(CanvasGroupMarker),
        "CanvasModulate" => ec.insert(CanvasModulateMarker),
        "CollisionObject2D" => ec.insert(CollisionObject2DMarker),
        "CollisionPolygon2D" => ec.insert(CollisionPolygon2DMarker),
        "CollisionShape2D" => ec.insert(CollisionShape2DMarker),
        "ColorRect" => ec.insert(ColorRectMarker),
        "ConeTwistJoint3D" => ec.insert(ConeTwistJoint3DMarker),
        "Container" => ec.insert(ContainerMarker),
        "Decal" => ec.insert(DecalMarker),
        "FogVolume" => ec.insert(FogVolumeMarker),
        "GPUParticles2D" => ec.insert(GPUParticles2DMarker),
        "GPUParticlesAttractor3D" => ec.insert(GPUParticlesAttractor3DMarker),
        "GPUParticlesCollision3D" => ec.insert(GPUParticlesCollision3DMarker),
        "Generic6DOFJoint3D" => ec.insert(Generic6DOFJoint3DMarker),
        "GeometryInstance3D" => ec.insert(GeometryInstance3DMarker),
        #[cfg(feature = "experimental-godot-api")]
        "GraphEdit" => ec.insert(GraphEditMarker),
        "HingeJoint3D" => ec.insert(HingeJoint3DMarker),
        "IKModifier3D" => ec.insert(IKModifier3DMarker),
        "ItemList" => ec.insert(ItemListMarker),
        "Joint2D" => ec.insert(Joint2DMarker),
        "Label" => ec.insert(LabelMarker),
        "Light2D" => ec.insert(Light2DMarker),
        "Light3D" => ec.insert(Light3DMarker),
        "LightOccluder2D" => ec.insert(LightOccluder2DMarker),
        "LightmapGI" => ec.insert(LightmapGIMarker),
        "LimitAngularVelocityModifier3D" => ec.insert(LimitAngularVelocityModifier3DMarker),
        "Line2D" => ec.insert(Line2DMarker),
        "LineEdit" => ec.insert(LineEditMarker),
        "LookAtModifier3D" => ec.insert(LookAtModifier3DMarker),
        "Marker2D" => ec.insert(Marker2DMarker),
        "MenuBar" => ec.insert(MenuBarMarker),
        "MeshInstance2D" => ec.insert(MeshInstance2DMarker),
        "ModifierBoneTarget3D" => ec.insert(ModifierBoneTarget3DMarker),
        "MultiMeshInstance2D" => ec.insert(MultiMeshInstance2DMarker),
        #[cfg(feature = "experimental-godot-api")]
        "NavigationLink2D" => ec.insert(NavigationLink2DMarker),
        #[cfg(feature = "experimental-godot-api")]
        "NavigationObstacle2D" => ec.insert(NavigationObstacle2DMarker),
        #[cfg(feature = "experimental-godot-api")]
        "NavigationRegion2D" => ec.insert(NavigationRegion2DMarker),
        "NinePatchRect" => ec.insert(NinePatchRectMarker),
        "OccluderInstance3D" => ec.insert(OccluderInstance3DMarker),
        "OpenXRCompositionLayerCylinder" => ec.insert(OpenXRCompositionLayerCylinderMarker),
        "OpenXRCompositionLayerEquirect" => ec.insert(OpenXRCompositionLayerEquirectMarker),
        "OpenXRCompositionLayerQuad" => ec.insert(OpenXRCompositionLayerQuadMarker),
        "OpenXRVisibilityMask" => ec.insert(OpenXRVisibilityMaskMarker),
        "Panel" => ec.insert(PanelMarker),
        #[cfg(feature = "experimental-godot-api")]
        "Parallax2D" => ec.insert(Parallax2DMarker),
        "ParallaxLayer" => ec.insert(ParallaxLayerMarker),
        "Path2D" => ec.insert(Path2DMarker),
        "PathFollow2D" => ec.insert(PathFollow2DMarker),
        "PhysicalBoneSimulator3D" => ec.insert(PhysicalBoneSimulator3DMarker),
        "PhysicsBody3D" => ec.insert(PhysicsBody3DMarker),
        "PinJoint3D" => ec.insert(PinJoint3DMarker),
        "Polygon2D" => ec.insert(Polygon2DMarker),
        "Popup" => ec.insert(PopupMarker),
        "Range" => ec.insert(RangeMarker),
        "RayCast2D" => ec.insert(RayCast2DMarker),
        "ReferenceRect" => ec.insert(ReferenceRectMarker),
        "ReflectionProbe" => ec.insert(ReflectionProbeMarker),
        "RemoteTransform2D" => ec.insert(RemoteTransform2DMarker),
        "RetargetModifier3D" => ec.insert(RetargetModifier3DMarker),
        "RichTextLabel" => ec.insert(RichTextLabelMarker),
        "RootMotionView" => ec.insert(RootMotionViewMarker),
        "Separator" => ec.insert(SeparatorMarker),
        "ShapeCast2D" => ec.insert(ShapeCast2DMarker),
        "Skeleton2D" => ec.insert(Skeleton2DMarker),
        "SkeletonIK3D" => ec.insert(SkeletonIK3DMarker),
        "SliderJoint3D" => ec.insert(SliderJoint3DMarker),
        "SpringBoneCollisionCapsule3D" => ec.insert(SpringBoneCollisionCapsule3DMarker),
        "SpringBoneCollisionPlane3D" => ec.insert(SpringBoneCollisionPlane3DMarker),
        "SpringBoneCollisionSphere3D" => ec.insert(SpringBoneCollisionSphere3DMarker),
        "SpringBoneSimulator3D" => ec.insert(SpringBoneSimulator3DMarker),
        "Sprite2D" => ec.insert(Sprite2DMarker),
        "TabBar" => ec.insert(TabBarMarker),
        "TextEdit" => ec.insert(TextEditMarker),
        "TextureRect" => ec.insert(TextureRectMarker),
        "TileMap" => ec.insert(TileMapMarker),
        "TileMapLayer" => ec.insert(TileMapLayerMarker),
        "TouchScreenButton" => ec.insert(TouchScreenButtonMarker),
        "Tree" => ec.insert(TreeMarker),
        "VideoStreamPlayer" => ec.insert(VideoStreamPlayerMarker),
        "VisibleOnScreenNotifier2D" => ec.insert(VisibleOnScreenNotifier2DMarker),
        "VisibleOnScreenNotifier3D" => ec.insert(VisibleOnScreenNotifier3DMarker),
        "VoxelGI" => ec.insert(VoxelGIMarker),
        "XRAnchor3D" => ec.insert(XRAnchor3DMarker),
        #[cfg(feature = "experimental-godot-api")]
        "XRBodyModifier3D" => ec.insert(XRBodyModifier3DMarker),
        "XRCamera3D" => ec.insert(XRCamera3DMarker),
        "XRController3D" => ec.insert(XRController3DMarker),
        "XRHandModifier3D" => ec.insert(XRHandModifier3DMarker),
        "AimModifier3D" => ec.insert(AimModifier3DMarker),
        "Area2D" => ec.insert(Area2DMarker),
        "AspectRatioContainer" => ec.insert(AspectRatioContainerMarker),
        "BoxContainer" => ec.insert(BoxContainerMarker),
        "Button" => ec.insert(ButtonMarker),
        "CPUParticles3D" => ec.insert(CPUParticles3DMarker),
        "CSGShape3D" => ec.insert(CSGShape3DMarker),
        "CenterContainer" => ec.insert(CenterContainerMarker),
        "ChainIK3D" => ec.insert(ChainIK3DMarker),
        "CharacterBody3D" => ec.insert(CharacterBody3DMarker),
        "CodeEdit" => ec.insert(CodeEditMarker),
        "ConfirmationDialog" => ec.insert(ConfirmationDialogMarker),
        "ConvertTransformModifier3D" => ec.insert(ConvertTransformModifier3DMarker),
        "CopyTransformModifier3D" => ec.insert(CopyTransformModifier3DMarker),
        "DampedSpringJoint2D" => ec.insert(DampedSpringJoint2DMarker),
        "DirectionalLight2D" => ec.insert(DirectionalLight2DMarker),
        "DirectionalLight3D" => ec.insert(DirectionalLight3DMarker),
        "EditorProperty" => ec.insert(EditorPropertyMarker),
        "EditorSpinSlider" => ec.insert(EditorSpinSliderMarker),
        "FlowContainer" => ec.insert(FlowContainerMarker),
        "FoldableContainer" => ec.insert(FoldableContainerMarker),
        "GPUParticles3D" => ec.insert(GPUParticles3DMarker),
        "GPUParticlesAttractorBox3D" => ec.insert(GPUParticlesAttractorBox3DMarker),
        "GPUParticlesAttractorSphere3D" => ec.insert(GPUParticlesAttractorSphere3DMarker),
        "GPUParticlesAttractorVectorField3D" => ec.insert(GPUParticlesAttractorVectorField3DMarker),
        "GPUParticlesCollisionBox3D" => ec.insert(GPUParticlesCollisionBox3DMarker),
        "GPUParticlesCollisionHeightField3D" => ec.insert(GPUParticlesCollisionHeightField3DMarker),
        "GPUParticlesCollisionSDF3D" => ec.insert(GPUParticlesCollisionSDF3DMarker),
        "GPUParticlesCollisionSphere3D" => ec.insert(GPUParticlesCollisionSphere3DMarker),
        #[cfg(feature = "experimental-godot-api")]
        "GraphElement" => ec.insert(GraphElementMarker),
        "GridContainer" => ec.insert(GridContainerMarker),
        "GrooveJoint2D" => ec.insert(GrooveJoint2DMarker),
        "HSeparator" => ec.insert(HSeparatorMarker),
        "Label3D" => ec.insert(Label3DMarker),
        "LinkButton" => ec.insert(LinkButtonMarker),
        "MarginContainer" => ec.insert(MarginContainerMarker),
        "MeshInstance3D" => ec.insert(MeshInstance3DMarker),
        "MultiMeshInstance3D" => ec.insert(MultiMeshInstance3DMarker),
        "OmniLight3D" => ec.insert(OmniLight3DMarker),
        "PanelContainer" => ec.insert(PanelContainerMarker),
        "PhysicalBone3D" => ec.insert(PhysicalBone3DMarker),
        "PhysicsBody2D" => ec.insert(PhysicsBody2DMarker),
        "PinJoint2D" => ec.insert(PinJoint2DMarker),
        "PointLight2D" => ec.insert(PointLight2DMarker),
        "PopupMenu" => ec.insert(PopupMenuMarker),
        "PopupPanel" => ec.insert(PopupPanelMarker),
        "ProgressBar" => ec.insert(ProgressBarMarker),
        "RigidBody3D" => ec.insert(RigidBody3DMarker),
        "ScrollBar" => ec.insert(ScrollBarMarker),
        "ScrollContainer" => ec.insert(ScrollContainerMarker),
        "Slider" => ec.insert(SliderMarker),
        "SpinBox" => ec.insert(SpinBoxMarker),
        "SplitContainer" => ec.insert(SplitContainerMarker),
        "SpotLight3D" => ec.insert(SpotLight3DMarker),
        "SpriteBase3D" => ec.insert(SpriteBase3DMarker),
        "StaticBody3D" => ec.insert(StaticBody3DMarker),
        "SubViewportContainer" => ec.insert(SubViewportContainerMarker),
        "TabContainer" => ec.insert(TabContainerMarker),
        "TextureButton" => ec.insert(TextureButtonMarker),
        "TextureProgressBar" => ec.insert(TextureProgressBarMarker),
        "TwoBoneIK3D" => ec.insert(TwoBoneIK3DMarker),
        "VSeparator" => ec.insert(VSeparatorMarker),
        "VisibleOnScreenEnabler2D" => ec.insert(VisibleOnScreenEnabler2DMarker),
        "VisibleOnScreenEnabler3D" => ec.insert(VisibleOnScreenEnabler3DMarker),
        "AnimatableBody3D" => ec.insert(AnimatableBody3DMarker),
        "AnimatedSprite3D" => ec.insert(AnimatedSprite3DMarker),
        "CSGCombiner3D" => ec.insert(CSGCombiner3DMarker),
        "CSGPrimitive3D" => ec.insert(CSGPrimitive3DMarker),
        "CharacterBody2D" => ec.insert(CharacterBody2DMarker),
        "CheckBox" => ec.insert(CheckBoxMarker),
        "CheckButton" => ec.insert(CheckButtonMarker),
        "ColorPickerButton" => ec.insert(ColorPickerButtonMarker),
        "EditorCommandPalette" => ec.insert(EditorCommandPaletteMarker),
        "EditorDock" => ec.insert(EditorDockMarker),
        "EditorInspector" => ec.insert(EditorInspectorMarker),
        "FileDialog" => ec.insert(FileDialogMarker),
        #[cfg(feature = "experimental-godot-api")]
        "GraphFrame" => ec.insert(GraphFrameMarker),
        #[cfg(feature = "experimental-godot-api")]
        "GraphNode" => ec.insert(GraphNodeMarker),
        "HBoxContainer" => ec.insert(HBoxContainerMarker),
        "HFlowContainer" => ec.insert(HFlowContainerMarker),
        "HScrollBar" => ec.insert(HScrollBarMarker),
        "HSlider" => ec.insert(HSliderMarker),
        "HSplitContainer" => ec.insert(HSplitContainerMarker),
        "IterateIK3D" => ec.insert(IterateIK3DMarker),
        "MenuButton" => ec.insert(MenuButtonMarker),
        "OpenXRBindingModifierEditor" => ec.insert(OpenXRBindingModifierEditorMarker),
        "OptionButton" => ec.insert(OptionButtonMarker),
        "RigidBody2D" => ec.insert(RigidBody2DMarker),
        "ScriptCreateDialog" => ec.insert(ScriptCreateDialogMarker),
        "ScriptEditor" => ec.insert(ScriptEditorMarker),
        "SoftBody3D" => ec.insert(SoftBody3DMarker),
        "SplineIK3D" => ec.insert(SplineIK3DMarker),
        "Sprite3D" => ec.insert(Sprite3DMarker),
        "StaticBody2D" => ec.insert(StaticBody2DMarker),
        "VBoxContainer" => ec.insert(VBoxContainerMarker),
        "VFlowContainer" => ec.insert(VFlowContainerMarker),
        "VScrollBar" => ec.insert(VScrollBarMarker),
        "VSlider" => ec.insert(VSliderMarker),
        "VSplitContainer" => ec.insert(VSplitContainerMarker),
        "VehicleBody3D" => ec.insert(VehicleBody3DMarker),
        "AnimatableBody2D" => ec.insert(AnimatableBody2DMarker),
        "CCDIK3D" => ec.insert(CCDIK3DMarker),
        "CSGBox3D" => ec.insert(CSGBox3DMarker),
        "CSGCylinder3D" => ec.insert(CSGCylinder3DMarker),
        "CSGMesh3D" => ec.insert(CSGMesh3DMarker),
        "CSGPolygon3D" => ec.insert(CSGPolygon3DMarker),
        "CSGSphere3D" => ec.insert(CSGSphere3DMarker),
        "CSGTorus3D" => ec.insert(CSGTorus3DMarker),
        "ColorPicker" => ec.insert(ColorPickerMarker),
        "EditorFileDialog" => ec.insert(EditorFileDialogMarker),
        "EditorResourcePicker" => ec.insert(EditorResourcePickerMarker),
        "EditorToaster" => ec.insert(EditorToasterMarker),
        "FABRIK3D" => ec.insert(FABRIK3DMarker),
        "FileSystemDock" => ec.insert(FileSystemDockMarker),
        "JacobianIK3D" => ec.insert(JacobianIK3DMarker),
        "OpenXRInteractionProfileEditorBase" => ec.insert(OpenXRInteractionProfileEditorBaseMarker),
        "PhysicalBone2D" => ec.insert(PhysicalBone2DMarker),
        "ScriptEditorBase" => ec.insert(ScriptEditorBaseMarker),
        "EditorScriptPicker" => ec.insert(EditorScriptPickerMarker),
        "OpenXRInteractionProfileEditor" => ec.insert(OpenXRInteractionProfileEditorMarker),
        // Custom user types that extend Godot nodes
        _ => ec,
    };
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
    ec.remove::<StatusIndicatorMarker>();
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
    ec.remove::<GridMapEditorPluginMarker>();
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
    ec.remove::<OpenXRCompositionLayerMarker>();
    ec.remove::<OpenXRHandMarker>();
    #[cfg(not(feature = "experimental-wasm"))]
    ec.remove::<OpenXRRenderModelMarker>();
    #[cfg(not(feature = "experimental-wasm"))]
    ec.remove::<OpenXRRenderModelManagerMarker>();
    ec.remove::<ParallaxBackgroundMarker>();
    ec.remove::<Path3DMarker>();
    ec.remove::<PathFollow3DMarker>();
    ec.remove::<RayCast3DMarker>();
    ec.remove::<RemoteTransform3DMarker>();
    ec.remove::<ShapeCast3DMarker>();
    ec.remove::<Skeleton3DMarker>();
    ec.remove::<SkeletonModifier3DMarker>();
    ec.remove::<SpringArm3DMarker>();
    ec.remove::<SpringBoneCollision3DMarker>();
    ec.remove::<SubViewportMarker>();
    ec.remove::<VehicleWheel3DMarker>();
    ec.remove::<VisualInstance3DMarker>();
    ec.remove::<WindowMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<XRFaceModifier3DMarker>();
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
    ec.remove::<BoneConstraint3DMarker>();
    ec.remove::<BoneTwistDisperser3DMarker>();
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
    ec.remove::<IKModifier3DMarker>();
    ec.remove::<ItemListMarker>();
    ec.remove::<Joint2DMarker>();
    ec.remove::<LabelMarker>();
    ec.remove::<Light2DMarker>();
    ec.remove::<Light3DMarker>();
    ec.remove::<LightOccluder2DMarker>();
    ec.remove::<LightmapGIMarker>();
    ec.remove::<LimitAngularVelocityModifier3DMarker>();
    ec.remove::<Line2DMarker>();
    ec.remove::<LineEditMarker>();
    ec.remove::<LookAtModifier3DMarker>();
    ec.remove::<Marker2DMarker>();
    ec.remove::<MenuBarMarker>();
    ec.remove::<MeshInstance2DMarker>();
    ec.remove::<ModifierBoneTarget3DMarker>();
    ec.remove::<MultiMeshInstance2DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationLink2DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationObstacle2DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<NavigationRegion2DMarker>();
    ec.remove::<NinePatchRectMarker>();
    ec.remove::<OccluderInstance3DMarker>();
    ec.remove::<OpenXRCompositionLayerCylinderMarker>();
    ec.remove::<OpenXRCompositionLayerEquirectMarker>();
    ec.remove::<OpenXRCompositionLayerQuadMarker>();
    ec.remove::<OpenXRVisibilityMaskMarker>();
    ec.remove::<PanelMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<Parallax2DMarker>();
    ec.remove::<ParallaxLayerMarker>();
    ec.remove::<Path2DMarker>();
    ec.remove::<PathFollow2DMarker>();
    ec.remove::<PhysicalBoneSimulator3DMarker>();
    ec.remove::<PhysicsBody3DMarker>();
    ec.remove::<PinJoint3DMarker>();
    ec.remove::<Polygon2DMarker>();
    ec.remove::<PopupMarker>();
    ec.remove::<RangeMarker>();
    ec.remove::<RayCast2DMarker>();
    ec.remove::<ReferenceRectMarker>();
    ec.remove::<ReflectionProbeMarker>();
    ec.remove::<RemoteTransform2DMarker>();
    ec.remove::<RetargetModifier3DMarker>();
    ec.remove::<RichTextLabelMarker>();
    ec.remove::<RootMotionViewMarker>();
    ec.remove::<SeparatorMarker>();
    ec.remove::<ShapeCast2DMarker>();
    ec.remove::<Skeleton2DMarker>();
    ec.remove::<SkeletonIK3DMarker>();
    ec.remove::<SliderJoint3DMarker>();
    ec.remove::<SpringBoneCollisionCapsule3DMarker>();
    ec.remove::<SpringBoneCollisionPlane3DMarker>();
    ec.remove::<SpringBoneCollisionSphere3DMarker>();
    ec.remove::<SpringBoneSimulator3DMarker>();
    ec.remove::<Sprite2DMarker>();
    ec.remove::<TabBarMarker>();
    ec.remove::<TextEditMarker>();
    ec.remove::<TextureRectMarker>();
    ec.remove::<TileMapMarker>();
    ec.remove::<TileMapLayerMarker>();
    ec.remove::<TouchScreenButtonMarker>();
    ec.remove::<TreeMarker>();
    ec.remove::<VideoStreamPlayerMarker>();
    ec.remove::<VisibleOnScreenNotifier2DMarker>();
    ec.remove::<VisibleOnScreenNotifier3DMarker>();
    ec.remove::<VoxelGIMarker>();
    ec.remove::<XRAnchor3DMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<XRBodyModifier3DMarker>();
    ec.remove::<XRCamera3DMarker>();
    ec.remove::<XRController3DMarker>();
    ec.remove::<XRHandModifier3DMarker>();
    ec.remove::<AimModifier3DMarker>();
    ec.remove::<Area2DMarker>();
    ec.remove::<AspectRatioContainerMarker>();
    ec.remove::<BoxContainerMarker>();
    ec.remove::<ButtonMarker>();
    ec.remove::<CPUParticles3DMarker>();
    ec.remove::<CSGShape3DMarker>();
    ec.remove::<CenterContainerMarker>();
    ec.remove::<ChainIK3DMarker>();
    ec.remove::<CharacterBody3DMarker>();
    ec.remove::<CodeEditMarker>();
    ec.remove::<ConfirmationDialogMarker>();
    ec.remove::<ConvertTransformModifier3DMarker>();
    ec.remove::<CopyTransformModifier3DMarker>();
    ec.remove::<DampedSpringJoint2DMarker>();
    ec.remove::<DirectionalLight2DMarker>();
    ec.remove::<DirectionalLight3DMarker>();
    ec.remove::<EditorPropertyMarker>();
    ec.remove::<EditorSpinSliderMarker>();
    ec.remove::<FlowContainerMarker>();
    ec.remove::<FoldableContainerMarker>();
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
    ec.remove::<TwoBoneIK3DMarker>();
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
    ec.remove::<EditorDockMarker>();
    ec.remove::<EditorInspectorMarker>();
    ec.remove::<FileDialogMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<GraphFrameMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<GraphNodeMarker>();
    ec.remove::<HBoxContainerMarker>();
    ec.remove::<HFlowContainerMarker>();
    ec.remove::<HScrollBarMarker>();
    ec.remove::<HSliderMarker>();
    ec.remove::<HSplitContainerMarker>();
    ec.remove::<IterateIK3DMarker>();
    ec.remove::<MenuButtonMarker>();
    ec.remove::<OpenXRBindingModifierEditorMarker>();
    ec.remove::<OptionButtonMarker>();
    ec.remove::<RigidBody2DMarker>();
    ec.remove::<ScriptCreateDialogMarker>();
    ec.remove::<ScriptEditorMarker>();
    ec.remove::<SoftBody3DMarker>();
    ec.remove::<SplineIK3DMarker>();
    ec.remove::<Sprite3DMarker>();
    ec.remove::<StaticBody2DMarker>();
    ec.remove::<VBoxContainerMarker>();
    ec.remove::<VFlowContainerMarker>();
    ec.remove::<VScrollBarMarker>();
    ec.remove::<VSliderMarker>();
    ec.remove::<VSplitContainerMarker>();
    ec.remove::<VehicleBody3DMarker>();
    ec.remove::<AnimatableBody2DMarker>();
    ec.remove::<CCDIK3DMarker>();
    ec.remove::<CSGBox3DMarker>();
    ec.remove::<CSGCylinder3DMarker>();
    ec.remove::<CSGMesh3DMarker>();
    ec.remove::<CSGPolygon3DMarker>();
    ec.remove::<CSGSphere3DMarker>();
    ec.remove::<CSGTorus3DMarker>();
    ec.remove::<ColorPickerMarker>();
    ec.remove::<EditorFileDialogMarker>();
    ec.remove::<EditorResourcePickerMarker>();
    ec.remove::<EditorToasterMarker>();
    ec.remove::<FABRIK3DMarker>();
    ec.remove::<FileSystemDockMarker>();
    ec.remove::<JacobianIK3DMarker>();
    ec.remove::<OpenXRInteractionProfileEditorBaseMarker>();
    ec.remove::<PhysicalBone2DMarker>();
    ec.remove::<ScriptEditorBaseMarker>();
    ec.remove::<EditorScriptPickerMarker>();
    ec.remove::<OpenXRInteractionProfileEditorMarker>();
}
