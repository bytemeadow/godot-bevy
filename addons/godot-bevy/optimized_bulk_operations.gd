extends Node
class_name OptimizedBulkOperations

# Optimized Bulk Operations
# This GDScript class provides bulk FFI optimization methods for godot-bevy.
# These methods reduce FFI overhead by batching operations that would otherwise
# require many individual Rust-to-Godot calls.
#
# In debug builds, these bulk methods are faster due to high Rust FFI overhead.
# In release builds, direct FFI calls are faster, so these are primarily used
# for debug build performance.


func _ready():
	name = "OptimizedBulkOperations"


# =============================================================================
# Bulk Transform Write Methods
# =============================================================================

func bulk_update_transforms_3d(
	instance_ids: PackedInt64Array,
	positions: PackedVector3Array,
	rotations: PackedVector4Array,
	scales: PackedVector3Array
) -> void:
	var rotation: Quaternion = Quaternion()
	for i: int in range(instance_ids.size()):
		var node: Node3D = instance_from_id(instance_ids[i]) as Node3D
		node.position = positions[i]
		rotation.x = rotations[i].x
		rotation.y = rotations[i].y
		rotation.z = rotations[i].z
		rotation.w = rotations[i].w
		node.quaternion = rotation
		node.scale = scales[i]


func bulk_update_transforms_2d(
	instance_ids: PackedInt64Array,
	positions: PackedVector2Array,
	rotations: PackedFloat32Array,
	scales: PackedVector2Array
) -> void:
	for i: int in range(instance_ids.size()):
		var node: Node2D = instance_from_id(instance_ids[i]) as Node2D
		node.position = positions[i]
		node.rotation = rotations[i]
		node.scale = scales[i]


# =============================================================================
# Bulk Transform Read Methods
# =============================================================================

func bulk_get_transforms_3d(instance_ids: PackedInt64Array) -> Dictionary:
	var positions: PackedVector3Array = PackedVector3Array()
	var rotations: PackedVector4Array = PackedVector4Array()
	var scales: PackedVector3Array = PackedVector3Array()

	positions.resize(instance_ids.size())
	rotations.resize(instance_ids.size())
	scales.resize(instance_ids.size())

	for i: int in range(instance_ids.size()):
		var node: Node3D = instance_from_id(instance_ids[i]) as Node3D
		positions[i] = node.position
		var q: Quaternion = node.quaternion
		rotations[i] = Vector4(q.x, q.y, q.z, q.w)
		scales[i] = node.scale

	return {"positions": positions, "rotations": rotations, "scales": scales}


func bulk_get_transforms_2d(instance_ids: PackedInt64Array) -> Dictionary:
	var positions: PackedVector2Array = PackedVector2Array()
	var rotations: PackedFloat32Array = PackedFloat32Array()
	var scales: PackedVector2Array = PackedVector2Array()

	positions.resize(instance_ids.size())
	rotations.resize(instance_ids.size())
	scales.resize(instance_ids.size())

	for i: int in range(instance_ids.size()):
		var node: Node2D = instance_from_id(instance_ids[i]) as Node2D
		positions[i] = node.position
		rotations[i] = node.rotation
		scales[i] = node.scale

	return {"positions": positions, "rotations": rotations, "scales": scales}


# =============================================================================
# Bulk Input Action Checking
# =============================================================================

func bulk_check_actions(event: InputEvent) -> Dictionary:
	var actions: PackedStringArray = PackedStringArray()
	var pressed: Array[bool] = []
	var strengths: PackedFloat32Array = PackedFloat32Array()

	for action in InputMap.get_actions():
		if event.is_action(action):
			actions.append(action)
			pressed.append(event.is_action_pressed(action))
			strengths.append(event.get_action_strength(action))

	return {"actions": actions, "pressed": pressed, "strengths": strengths}


# =============================================================================
# Bulk Collision Signal Connections
# =============================================================================

# Collision mask bit flags (must match Rust constants)
const COLLISION_MASK_BODY_ENTERED = 1
const COLLISION_MASK_BODY_EXITED = 2
const COLLISION_MASK_AREA_ENTERED = 4
const COLLISION_MASK_AREA_EXITED = 8

func bulk_connect_collision_signals(
	instance_ids: PackedInt64Array,
	collision_masks: PackedInt64Array,
	collision_watcher: Node
) -> void:
	"""
	Connect collision signals for multiple nodes in a single call.
	Each node connects up to 4 signals based on its collision mask:
	- body_entered (mask bit 0)
	- body_exited (mask bit 1)
	- area_entered (mask bit 2)
	- area_exited (mask bit 3)

	The collision_watcher.collision_event callable expects:
	- colliding_body: Node (passed by the signal)
	- origin_node: Node (bound)
	- event_type: String ("Started" or "Ended", bound)
	"""
	for i: int in range(instance_ids.size()):
		var node: Node = instance_from_id(instance_ids[i]) as Node
		if not node:
			continue

		var mask: int = collision_masks[i]

		if mask & COLLISION_MASK_BODY_ENTERED:
			node.connect(
				"body_entered",
				collision_watcher.collision_event.bind(node, "Started")
			)

		if mask & COLLISION_MASK_BODY_EXITED:
			node.connect(
				"body_exited",
				collision_watcher.collision_event.bind(node, "Ended")
			)

		if mask & COLLISION_MASK_AREA_ENTERED:
			node.connect(
				"area_entered",
				collision_watcher.collision_event.bind(node, "Started")
			)

		if mask & COLLISION_MASK_AREA_EXITED:
			node.connect(
				"area_exited",
				collision_watcher.collision_event.bind(node, "Ended")
			)
