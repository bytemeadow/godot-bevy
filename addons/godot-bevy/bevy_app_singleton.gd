extends BevyApp


# Bulk Transform Optimization Methods
# Automatically detected by godot-bevy library for performance optimization

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


# Bulk Transform Read Methods
# Returns transform data for multiple nodes in a single FFI call

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


# Bulk Input Action Checking
# Checks an input event against all actions in a single FFI call
# Returns arrays of matching action data for efficient transfer

func bulk_check_actions(event: InputEvent) -> Dictionary:
	var actions: PackedStringArray = PackedStringArray()
	var pressed: Array = []
	var strengths: PackedFloat32Array = PackedFloat32Array()

	for action in InputMap.get_actions():
		if event.is_action(action):
			actions.append(action)
			pressed.append(event.is_action_pressed(action))
			strengths.append(event.get_action_strength(action))

	return {"actions": actions, "pressed": pressed, "strengths": strengths}
