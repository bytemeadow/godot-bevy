extends Node
class_name OptimizedBulkOperations

# Optimized Bulk Operations
# Batches collision-signal connections into a single Rust-to-Godot call,
# avoiding one FFI call per node when wiring up many colliders.


func _ready():
	name = "OptimizedBulkOperations"


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
