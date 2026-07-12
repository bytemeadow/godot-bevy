//! 🤖 This file is generated. Changes to it will be lost.
//! To regenerate: uv run python -m godot_bevy_codegen

use crate::interop::{GodotNode, node_markers::*};
use bevy_ecs::system::EntityCommands;

/// Adds node type markers based on a pre-analyzed type string from GDScript.
/// This avoids FFI calls by using type information determined on the GDScript side.
/// This provides significant performance improvements by eliminating multiple
/// GodotNode::try_get calls for each node.
pub fn add_node_type_markers_from_string(ec: &mut EntityCommands, node_type: &str) -> bool {
    // Add appropriate markers based on the type string
    match node_type {
        "Node" => {
            ec.insert(NodeMarker);
            true
        }
        "AnimationMixer" => {
            ec.insert((AnimationMixerMarker, NodeMarker));
            true
        }
        "AudioStreamPlayer" => {
            ec.insert((AudioStreamPlayerMarker, NodeMarker));
            true
        }
        "CanvasItem" => {
            ec.insert((CanvasItemMarker, NodeMarker));
            true
        }
        "CanvasLayer" => {
            ec.insert((CanvasLayerMarker, NodeMarker));
            true
        }
        "EditorFileSystem" => {
            ec.insert((EditorFileSystemMarker, NodeMarker));
            true
        }
        "EditorPlugin" => {
            ec.insert((EditorPluginMarker, NodeMarker));
            true
        }
        "EditorResourcePreview" => {
            ec.insert((EditorResourcePreviewMarker, NodeMarker));
            true
        }
        "HTTPRequest" => {
            ec.insert((HTTPRequestMarker, NodeMarker));
            true
        }
        "InstancePlaceholder" => {
            ec.insert((InstancePlaceholderMarker, NodeMarker));
            true
        }
        "MissingNode" => {
            ec.insert((MissingNodeMarker, NodeMarker));
            true
        }
        "MultiplayerSpawner" => {
            ec.insert((MultiplayerSpawnerMarker, NodeMarker));
            true
        }
        "MultiplayerSynchronizer" => {
            ec.insert((MultiplayerSynchronizerMarker, NodeMarker));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationAgent2D" => {
            ec.insert((NavigationAgent2DMarker, NodeMarker));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationAgent3D" => {
            ec.insert((NavigationAgent3DMarker, NodeMarker));
            true
        }
        "Node3D" => {
            ec.insert((Node3DMarker, NodeMarker));
            true
        }
        "ResourcePreloader" => {
            ec.insert((ResourcePreloaderMarker, NodeMarker));
            true
        }
        "ShaderGlobalsOverride" => {
            ec.insert((ShaderGlobalsOverrideMarker, NodeMarker));
            true
        }
        "StatusIndicator" => {
            ec.insert((StatusIndicatorMarker, NodeMarker));
            true
        }
        "Timer" => {
            ec.insert((TimerMarker, NodeMarker));
            true
        }
        "Viewport" => {
            ec.insert((ViewportMarker, NodeMarker));
            true
        }
        "WorldEnvironment" => {
            ec.insert((WorldEnvironmentMarker, NodeMarker));
            true
        }
        "AnimationPlayer" => {
            ec.insert((AnimationPlayerMarker, AnimationMixerMarker, NodeMarker));
            true
        }
        "AnimationTree" => {
            ec.insert((AnimationTreeMarker, AnimationMixerMarker, NodeMarker));
            true
        }
        "AudioListener3D" => {
            ec.insert((AudioListener3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "AudioStreamPlayer3D" => {
            ec.insert((AudioStreamPlayer3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "BoneAttachment3D" => {
            ec.insert((BoneAttachment3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "Camera3D" => {
            ec.insert((Camera3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "CollisionObject3D" => {
            ec.insert((CollisionObject3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "CollisionPolygon3D" => {
            ec.insert((CollisionPolygon3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "CollisionShape3D" => {
            ec.insert((CollisionShape3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "Control" => {
            ec.insert((ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "GridMap" => {
            ec.insert((GridMapMarker, Node3DMarker, NodeMarker));
            true
        }
        "GridMapEditorPlugin" => {
            ec.insert((GridMapEditorPluginMarker, EditorPluginMarker, NodeMarker));
            true
        }
        "ImporterMeshInstance3D" => {
            ec.insert((ImporterMeshInstance3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "Joint3D" => {
            ec.insert((Joint3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "LightmapProbe" => {
            ec.insert((LightmapProbeMarker, Node3DMarker, NodeMarker));
            true
        }
        "Marker3D" => {
            ec.insert((Marker3DMarker, Node3DMarker, NodeMarker));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationLink3D" => {
            ec.insert((NavigationLink3DMarker, Node3DMarker, NodeMarker));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationObstacle3D" => {
            ec.insert((NavigationObstacle3DMarker, Node3DMarker, NodeMarker));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationRegion3D" => {
            ec.insert((NavigationRegion3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "Node2D" => {
            ec.insert((Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "OpenXRCompositionLayer" => {
            ec.insert((OpenXRCompositionLayerMarker, Node3DMarker, NodeMarker));
            true
        }
        "OpenXRHand" => {
            ec.insert((OpenXRHandMarker, Node3DMarker, NodeMarker));
            true
        }
        "ParallaxBackground" => {
            ec.insert((ParallaxBackgroundMarker, CanvasLayerMarker, NodeMarker));
            true
        }
        "Path3D" => {
            ec.insert((Path3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "PathFollow3D" => {
            ec.insert((PathFollow3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "RayCast3D" => {
            ec.insert((RayCast3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "RemoteTransform3D" => {
            ec.insert((RemoteTransform3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "ShapeCast3D" => {
            ec.insert((ShapeCast3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "Skeleton3D" => {
            ec.insert((Skeleton3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "SkeletonModifier3D" => {
            ec.insert((SkeletonModifier3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "SpringArm3D" => {
            ec.insert((SpringArm3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "SpringBoneCollision3D" => {
            ec.insert((SpringBoneCollision3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "SubViewport" => {
            ec.insert((SubViewportMarker, ViewportMarker, NodeMarker));
            true
        }
        "VehicleWheel3D" => {
            ec.insert((VehicleWheel3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "VisualInstance3D" => {
            ec.insert((VisualInstance3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "Window" => {
            ec.insert((WindowMarker, ViewportMarker, NodeMarker));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "XRFaceModifier3D" => {
            ec.insert((XRFaceModifier3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "XRNode3D" => {
            ec.insert((XRNode3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "XROrigin3D" => {
            ec.insert((XROrigin3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "AcceptDialog" => {
            ec.insert((AcceptDialogMarker, WindowMarker, ViewportMarker, NodeMarker));
            true
        }
        "AnimatedSprite2D" => {
            ec.insert((
                AnimatedSprite2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "Area3D" => {
            ec.insert((
                Area3DMarker,
                CollisionObject3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "AudioListener2D" => {
            ec.insert((
                AudioListener2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "AudioStreamPlayer2D" => {
            ec.insert((
                AudioStreamPlayer2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "BackBufferCopy" => {
            ec.insert((
                BackBufferCopyMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "BaseButton" => {
            ec.insert((
                BaseButtonMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "Bone2D" => {
            ec.insert((Bone2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "CPUParticles2D" => {
            ec.insert((
                CPUParticles2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "Camera2D" => {
            ec.insert((Camera2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "CanvasGroup" => {
            ec.insert((
                CanvasGroupMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "CanvasModulate" => {
            ec.insert((
                CanvasModulateMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "CollisionObject2D" => {
            ec.insert((
                CollisionObject2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "CollisionPolygon2D" => {
            ec.insert((
                CollisionPolygon2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "CollisionShape2D" => {
            ec.insert((
                CollisionShape2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "ColorRect" => {
            ec.insert((ColorRectMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "ConeTwistJoint3D" => {
            ec.insert((
                ConeTwistJoint3DMarker,
                Joint3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "Container" => {
            ec.insert((ContainerMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "Decal" => {
            ec.insert((
                DecalMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "FogVolume" => {
            ec.insert((
                FogVolumeMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticles2D" => {
            ec.insert((
                GPUParticles2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticlesAttractor3D" => {
            ec.insert((
                GPUParticlesAttractor3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticlesCollision3D" => {
            ec.insert((
                GPUParticlesCollision3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "Generic6DOFJoint3D" => {
            ec.insert((
                Generic6DOFJoint3DMarker,
                Joint3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "GeometryInstance3D" => {
            ec.insert((
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphEdit" => {
            ec.insert((GraphEditMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "HingeJoint3D" => {
            ec.insert((HingeJoint3DMarker, Joint3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "ItemList" => {
            ec.insert((ItemListMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "Joint2D" => {
            ec.insert((Joint2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "Label" => {
            ec.insert((LabelMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "Light2D" => {
            ec.insert((Light2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "Light3D" => {
            ec.insert((
                Light3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "LightOccluder2D" => {
            ec.insert((
                LightOccluder2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "LightmapGI" => {
            ec.insert((
                LightmapGIMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "Line2D" => {
            ec.insert((Line2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "LineEdit" => {
            ec.insert((LineEditMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "LookAtModifier3D" => {
            ec.insert((
                LookAtModifier3DMarker,
                SkeletonModifier3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "Marker2D" => {
            ec.insert((Marker2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "MenuBar" => {
            ec.insert((MenuBarMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "MeshInstance2D" => {
            ec.insert((
                MeshInstance2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "MultiMeshInstance2D" => {
            ec.insert((
                MultiMeshInstance2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationLink2D" => {
            ec.insert((
                NavigationLink2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationObstacle2D" => {
            ec.insert((
                NavigationObstacle2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "NavigationRegion2D" => {
            ec.insert((
                NavigationRegion2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "NinePatchRect" => {
            ec.insert((
                NinePatchRectMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "OccluderInstance3D" => {
            ec.insert((
                OccluderInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "OpenXRCompositionLayerCylinder" => {
            ec.insert((
                OpenXRCompositionLayerCylinderMarker,
                OpenXRCompositionLayerMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "OpenXRCompositionLayerEquirect" => {
            ec.insert((
                OpenXRCompositionLayerEquirectMarker,
                OpenXRCompositionLayerMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "OpenXRCompositionLayerQuad" => {
            ec.insert((
                OpenXRCompositionLayerQuadMarker,
                OpenXRCompositionLayerMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "OpenXRVisibilityMask" => {
            ec.insert((
                OpenXRVisibilityMaskMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "Panel" => {
            ec.insert((PanelMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "Parallax2D" => {
            ec.insert((Parallax2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "ParallaxLayer" => {
            ec.insert((
                ParallaxLayerMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "Path2D" => {
            ec.insert((Path2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "PathFollow2D" => {
            ec.insert((
                PathFollow2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "PhysicalBoneSimulator3D" => {
            ec.insert((
                PhysicalBoneSimulator3DMarker,
                SkeletonModifier3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "PhysicsBody3D" => {
            ec.insert((
                PhysicsBody3DMarker,
                CollisionObject3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "PinJoint3D" => {
            ec.insert((PinJoint3DMarker, Joint3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "Polygon2D" => {
            ec.insert((Polygon2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "Popup" => {
            ec.insert((PopupMarker, WindowMarker, ViewportMarker, NodeMarker));
            true
        }
        "Range" => {
            ec.insert((RangeMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "RayCast2D" => {
            ec.insert((RayCast2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "ReferenceRect" => {
            ec.insert((
                ReferenceRectMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "ReflectionProbe" => {
            ec.insert((
                ReflectionProbeMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "RemoteTransform2D" => {
            ec.insert((
                RemoteTransform2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "RetargetModifier3D" => {
            ec.insert((
                RetargetModifier3DMarker,
                SkeletonModifier3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "RichTextLabel" => {
            ec.insert((
                RichTextLabelMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "RootMotionView" => {
            ec.insert((
                RootMotionViewMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "Separator" => {
            ec.insert((SeparatorMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "ShapeCast2D" => {
            ec.insert((
                ShapeCast2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "Skeleton2D" => {
            ec.insert((Skeleton2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "SkeletonIK3D" => {
            ec.insert((
                SkeletonIK3DMarker,
                SkeletonModifier3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "SliderJoint3D" => {
            ec.insert((SliderJoint3DMarker, Joint3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "SpringBoneCollisionCapsule3D" => {
            ec.insert((
                SpringBoneCollisionCapsule3DMarker,
                SpringBoneCollision3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "SpringBoneCollisionPlane3D" => {
            ec.insert((
                SpringBoneCollisionPlane3DMarker,
                SpringBoneCollision3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "SpringBoneCollisionSphere3D" => {
            ec.insert((
                SpringBoneCollisionSphere3DMarker,
                SpringBoneCollision3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "SpringBoneSimulator3D" => {
            ec.insert((
                SpringBoneSimulator3DMarker,
                SkeletonModifier3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "Sprite2D" => {
            ec.insert((Sprite2DMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "TabBar" => {
            ec.insert((TabBarMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "TextEdit" => {
            ec.insert((TextEditMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "TextureRect" => {
            ec.insert((
                TextureRectMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "TileMap" => {
            ec.insert((TileMapMarker, Node2DMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "TileMapLayer" => {
            ec.insert((
                TileMapLayerMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "TouchScreenButton" => {
            ec.insert((
                TouchScreenButtonMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "Tree" => {
            ec.insert((TreeMarker, ControlMarker, CanvasItemMarker, NodeMarker));
            true
        }
        "VideoStreamPlayer" => {
            ec.insert((
                VideoStreamPlayerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VisibleOnScreenNotifier2D" => {
            ec.insert((
                VisibleOnScreenNotifier2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VisibleOnScreenNotifier3D" => {
            ec.insert((
                VisibleOnScreenNotifier3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "VoxelGI" => {
            ec.insert((
                VoxelGIMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "XRAnchor3D" => {
            ec.insert((XRAnchor3DMarker, XRNode3DMarker, Node3DMarker, NodeMarker));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "XRBodyModifier3D" => {
            ec.insert((
                XRBodyModifier3DMarker,
                SkeletonModifier3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "XRCamera3D" => {
            ec.insert((XRCamera3DMarker, Camera3DMarker, Node3DMarker, NodeMarker));
            true
        }
        "XRController3D" => {
            ec.insert((
                XRController3DMarker,
                XRNode3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "XRHandModifier3D" => {
            ec.insert((
                XRHandModifier3DMarker,
                SkeletonModifier3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "Area2D" => {
            ec.insert((
                Area2DMarker,
                CollisionObject2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "AspectRatioContainer" => {
            ec.insert((
                AspectRatioContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "BoxContainer" => {
            ec.insert((
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "Button" => {
            ec.insert((
                ButtonMarker,
                BaseButtonMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "CPUParticles3D" => {
            ec.insert((
                CPUParticles3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CSGShape3D" => {
            ec.insert((
                CSGShape3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CenterContainer" => {
            ec.insert((
                CenterContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "CharacterBody3D" => {
            ec.insert((
                CharacterBody3DMarker,
                PhysicsBody3DMarker,
                CollisionObject3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CodeEdit" => {
            ec.insert((
                CodeEditMarker,
                TextEditMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "ConfirmationDialog" => {
            ec.insert((
                ConfirmationDialogMarker,
                AcceptDialogMarker,
                WindowMarker,
                ViewportMarker,
                NodeMarker,
            ));
            true
        }
        "DampedSpringJoint2D" => {
            ec.insert((
                DampedSpringJoint2DMarker,
                Joint2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "DirectionalLight2D" => {
            ec.insert((
                DirectionalLight2DMarker,
                Light2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "DirectionalLight3D" => {
            ec.insert((
                DirectionalLight3DMarker,
                Light3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "EditorProperty" => {
            ec.insert((
                EditorPropertyMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "EditorSpinSlider" => {
            ec.insert((
                EditorSpinSliderMarker,
                RangeMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "FlowContainer" => {
            ec.insert((
                FlowContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticles3D" => {
            ec.insert((
                GPUParticles3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticlesAttractorBox3D" => {
            ec.insert((
                GPUParticlesAttractorBox3DMarker,
                GPUParticlesAttractor3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticlesAttractorSphere3D" => {
            ec.insert((
                GPUParticlesAttractorSphere3DMarker,
                GPUParticlesAttractor3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticlesAttractorVectorField3D" => {
            ec.insert((
                GPUParticlesAttractorVectorField3DMarker,
                GPUParticlesAttractor3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticlesCollisionBox3D" => {
            ec.insert((
                GPUParticlesCollisionBox3DMarker,
                GPUParticlesCollision3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticlesCollisionHeightField3D" => {
            ec.insert((
                GPUParticlesCollisionHeightField3DMarker,
                GPUParticlesCollision3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticlesCollisionSDF3D" => {
            ec.insert((
                GPUParticlesCollisionSDF3DMarker,
                GPUParticlesCollision3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "GPUParticlesCollisionSphere3D" => {
            ec.insert((
                GPUParticlesCollisionSphere3DMarker,
                GPUParticlesCollision3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphElement" => {
            ec.insert((
                GraphElementMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "GridContainer" => {
            ec.insert((
                GridContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "GrooveJoint2D" => {
            ec.insert((
                GrooveJoint2DMarker,
                Joint2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "HSeparator" => {
            ec.insert((
                HSeparatorMarker,
                SeparatorMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "Label3D" => {
            ec.insert((
                Label3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "LinkButton" => {
            ec.insert((
                LinkButtonMarker,
                BaseButtonMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "MarginContainer" => {
            ec.insert((
                MarginContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "MeshInstance3D" => {
            ec.insert((
                MeshInstance3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "MultiMeshInstance3D" => {
            ec.insert((
                MultiMeshInstance3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "OmniLight3D" => {
            ec.insert((
                OmniLight3DMarker,
                Light3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "PanelContainer" => {
            ec.insert((
                PanelContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "PhysicalBone3D" => {
            ec.insert((
                PhysicalBone3DMarker,
                PhysicsBody3DMarker,
                CollisionObject3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "PhysicsBody2D" => {
            ec.insert((
                PhysicsBody2DMarker,
                CollisionObject2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "PinJoint2D" => {
            ec.insert((
                PinJoint2DMarker,
                Joint2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "PointLight2D" => {
            ec.insert((
                PointLight2DMarker,
                Light2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "PopupMenu" => {
            ec.insert((
                PopupMenuMarker,
                PopupMarker,
                WindowMarker,
                ViewportMarker,
                NodeMarker,
            ));
            true
        }
        "PopupPanel" => {
            ec.insert((
                PopupPanelMarker,
                PopupMarker,
                WindowMarker,
                ViewportMarker,
                NodeMarker,
            ));
            true
        }
        "ProgressBar" => {
            ec.insert((
                ProgressBarMarker,
                RangeMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "RigidBody3D" => {
            ec.insert((
                RigidBody3DMarker,
                PhysicsBody3DMarker,
                CollisionObject3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "ScrollBar" => {
            ec.insert((
                ScrollBarMarker,
                RangeMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "ScrollContainer" => {
            ec.insert((
                ScrollContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "Slider" => {
            ec.insert((
                SliderMarker,
                RangeMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "SpinBox" => {
            ec.insert((
                SpinBoxMarker,
                RangeMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "SplitContainer" => {
            ec.insert((
                SplitContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "SpotLight3D" => {
            ec.insert((
                SpotLight3DMarker,
                Light3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "SpriteBase3D" => {
            ec.insert((
                SpriteBase3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "StaticBody3D" => {
            ec.insert((
                StaticBody3DMarker,
                PhysicsBody3DMarker,
                CollisionObject3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "SubViewportContainer" => {
            ec.insert((
                SubViewportContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "TabContainer" => {
            ec.insert((
                TabContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "TextureButton" => {
            ec.insert((
                TextureButtonMarker,
                BaseButtonMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "TextureProgressBar" => {
            ec.insert((
                TextureProgressBarMarker,
                RangeMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VSeparator" => {
            ec.insert((
                VSeparatorMarker,
                SeparatorMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VisibleOnScreenEnabler2D" => {
            ec.insert((
                VisibleOnScreenEnabler2DMarker,
                VisibleOnScreenNotifier2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VisibleOnScreenEnabler3D" => {
            ec.insert((
                VisibleOnScreenEnabler3DMarker,
                VisibleOnScreenNotifier3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "AnimatableBody3D" => {
            ec.insert((
                AnimatableBody3DMarker,
                StaticBody3DMarker,
                PhysicsBody3DMarker,
                CollisionObject3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "AnimatedSprite3D" => {
            ec.insert((
                AnimatedSprite3DMarker,
                SpriteBase3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CSGCombiner3D" => {
            ec.insert((
                CSGCombiner3DMarker,
                CSGShape3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CSGPrimitive3D" => {
            ec.insert((
                CSGPrimitive3DMarker,
                CSGShape3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CharacterBody2D" => {
            ec.insert((
                CharacterBody2DMarker,
                PhysicsBody2DMarker,
                CollisionObject2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "CheckBox" => {
            ec.insert((
                CheckBoxMarker,
                ButtonMarker,
                BaseButtonMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "CheckButton" => {
            ec.insert((
                CheckButtonMarker,
                ButtonMarker,
                BaseButtonMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "ColorPickerButton" => {
            ec.insert((
                ColorPickerButtonMarker,
                ButtonMarker,
                BaseButtonMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "EditorCommandPalette" => {
            ec.insert((
                EditorCommandPaletteMarker,
                ConfirmationDialogMarker,
                AcceptDialogMarker,
                WindowMarker,
                ViewportMarker,
                NodeMarker,
            ));
            true
        }
        "EditorFileDialog" => {
            ec.insert((
                EditorFileDialogMarker,
                ConfirmationDialogMarker,
                AcceptDialogMarker,
                WindowMarker,
                ViewportMarker,
                NodeMarker,
            ));
            true
        }
        "EditorInspector" => {
            ec.insert((
                EditorInspectorMarker,
                ScrollContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "FileDialog" => {
            ec.insert((
                FileDialogMarker,
                ConfirmationDialogMarker,
                AcceptDialogMarker,
                WindowMarker,
                ViewportMarker,
                NodeMarker,
            ));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphFrame" => {
            ec.insert((
                GraphFrameMarker,
                GraphElementMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        #[cfg(feature = "experimental-godot-api")]
        "GraphNode" => {
            ec.insert((
                GraphNodeMarker,
                GraphElementMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "HBoxContainer" => {
            ec.insert((
                HBoxContainerMarker,
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "HFlowContainer" => {
            ec.insert((
                HFlowContainerMarker,
                FlowContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "HScrollBar" => {
            ec.insert((
                HScrollBarMarker,
                ScrollBarMarker,
                RangeMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "HSlider" => {
            ec.insert((
                HSliderMarker,
                SliderMarker,
                RangeMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "HSplitContainer" => {
            ec.insert((
                HSplitContainerMarker,
                SplitContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "MenuButton" => {
            ec.insert((
                MenuButtonMarker,
                ButtonMarker,
                BaseButtonMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "OpenXRBindingModifierEditor" => {
            ec.insert((
                OpenXRBindingModifierEditorMarker,
                PanelContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "OptionButton" => {
            ec.insert((
                OptionButtonMarker,
                ButtonMarker,
                BaseButtonMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "RigidBody2D" => {
            ec.insert((
                RigidBody2DMarker,
                PhysicsBody2DMarker,
                CollisionObject2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "ScriptCreateDialog" => {
            ec.insert((
                ScriptCreateDialogMarker,
                ConfirmationDialogMarker,
                AcceptDialogMarker,
                WindowMarker,
                ViewportMarker,
                NodeMarker,
            ));
            true
        }
        "ScriptEditor" => {
            ec.insert((
                ScriptEditorMarker,
                PanelContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "SoftBody3D" => {
            ec.insert((
                SoftBody3DMarker,
                MeshInstance3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "Sprite3D" => {
            ec.insert((
                Sprite3DMarker,
                SpriteBase3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "StaticBody2D" => {
            ec.insert((
                StaticBody2DMarker,
                PhysicsBody2DMarker,
                CollisionObject2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VBoxContainer" => {
            ec.insert((
                VBoxContainerMarker,
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VFlowContainer" => {
            ec.insert((
                VFlowContainerMarker,
                FlowContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VScrollBar" => {
            ec.insert((
                VScrollBarMarker,
                ScrollBarMarker,
                RangeMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VSlider" => {
            ec.insert((
                VSliderMarker,
                SliderMarker,
                RangeMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VSplitContainer" => {
            ec.insert((
                VSplitContainerMarker,
                SplitContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "VehicleBody3D" => {
            ec.insert((
                VehicleBody3DMarker,
                RigidBody3DMarker,
                PhysicsBody3DMarker,
                CollisionObject3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "AnimatableBody2D" => {
            ec.insert((
                AnimatableBody2DMarker,
                StaticBody2DMarker,
                PhysicsBody2DMarker,
                CollisionObject2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "CSGBox3D" => {
            ec.insert((
                CSGBox3DMarker,
                CSGPrimitive3DMarker,
                CSGShape3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CSGCylinder3D" => {
            ec.insert((
                CSGCylinder3DMarker,
                CSGPrimitive3DMarker,
                CSGShape3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CSGMesh3D" => {
            ec.insert((
                CSGMesh3DMarker,
                CSGPrimitive3DMarker,
                CSGShape3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CSGPolygon3D" => {
            ec.insert((
                CSGPolygon3DMarker,
                CSGPrimitive3DMarker,
                CSGShape3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CSGSphere3D" => {
            ec.insert((
                CSGSphere3DMarker,
                CSGPrimitive3DMarker,
                CSGShape3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "CSGTorus3D" => {
            ec.insert((
                CSGTorus3DMarker,
                CSGPrimitive3DMarker,
                CSGShape3DMarker,
                GeometryInstance3DMarker,
                VisualInstance3DMarker,
                Node3DMarker,
                NodeMarker,
            ));
            true
        }
        "ColorPicker" => {
            ec.insert((
                ColorPickerMarker,
                VBoxContainerMarker,
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "EditorResourcePicker" => {
            ec.insert((
                EditorResourcePickerMarker,
                HBoxContainerMarker,
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "EditorToaster" => {
            ec.insert((
                EditorToasterMarker,
                HBoxContainerMarker,
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "FileSystemDock" => {
            ec.insert((
                FileSystemDockMarker,
                VBoxContainerMarker,
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "OpenXRInteractionProfileEditorBase" => {
            ec.insert((
                OpenXRInteractionProfileEditorBaseMarker,
                HBoxContainerMarker,
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "PhysicalBone2D" => {
            ec.insert((
                PhysicalBone2DMarker,
                RigidBody2DMarker,
                PhysicsBody2DMarker,
                CollisionObject2DMarker,
                Node2DMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "ScriptEditorBase" => {
            ec.insert((
                ScriptEditorBaseMarker,
                VBoxContainerMarker,
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "EditorScriptPicker" => {
            ec.insert((
                EditorScriptPickerMarker,
                EditorResourcePickerMarker,
                HBoxContainerMarker,
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        "OpenXRInteractionProfileEditor" => {
            ec.insert((
                OpenXRInteractionProfileEditorMarker,
                OpenXRInteractionProfileEditorBaseMarker,
                HBoxContainerMarker,
                BoxContainerMarker,
                ContainerMarker,
                ControlMarker,
                CanvasItemMarker,
                NodeMarker,
            ));
            true
        }
        // Custom user types that extend Godot nodes
        _ => false,
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
    ec.remove::<LookAtModifier3DMarker>();
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
    ec.remove::<GraphFrameMarker>();
    #[cfg(feature = "experimental-godot-api")]
    ec.remove::<GraphNodeMarker>();
    ec.remove::<HBoxContainerMarker>();
    ec.remove::<HFlowContainerMarker>();
    ec.remove::<HScrollBarMarker>();
    ec.remove::<HSliderMarker>();
    ec.remove::<HSplitContainerMarker>();
    ec.remove::<MenuButtonMarker>();
    ec.remove::<OpenXRBindingModifierEditorMarker>();
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
    ec.remove::<EditorToasterMarker>();
    ec.remove::<FileSystemDockMarker>();
    ec.remove::<OpenXRInteractionProfileEditorBaseMarker>();
    ec.remove::<PhysicalBone2DMarker>();
    ec.remove::<ScriptEditorBaseMarker>();
    ec.remove::<EditorScriptPickerMarker>();
    ec.remove::<OpenXRInteractionProfileEditorMarker>();
}
