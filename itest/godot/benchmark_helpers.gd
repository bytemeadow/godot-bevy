extends Node
class_name BenchmarkHelpers

# Benchmark helper methods for internal/exploratory performance testing.
# These methods are used by the Rust internal benchmarks to compare different
# approaches for scene tree analysis.
#
# This file lives in itest/ rather than the addon because these benchmarks
# are for research purposes, not production code.
#
# Uses OptimizedSceneTreeWatcher's _analyze_node_type and _compute_collision_mask
# methods to avoid duplicating the type detection logic.

# Reference to OptimizedSceneTreeWatcher for reusing its analysis methods
var _watcher: Node = null

func _ready():
	name = "BenchmarkHelpers"
	_find_watcher()

func _find_watcher():
	# Find the OptimizedSceneTreeWatcher to reuse its analysis methods
	# It should be a sibling node (both are children of BevyAppSingleton)
	_watcher = get_node_or_null("../OptimizedSceneTreeWatcher")
	if not _watcher:
		_watcher = get_node_or_null("/root/BevyAppSingleton/OptimizedSceneTreeWatcher")
	if not _watcher:
		# Try searching from parent
		var parent = get_parent()
		if parent:
			_watcher = parent.get_node_or_null("OptimizedSceneTreeWatcher")
	if not _watcher:
		push_warning("[BenchmarkHelpers] OptimizedSceneTreeWatcher not found - benchmarks may not work correctly")

# =============================================================================
# Benchmark Methods
# =============================================================================

func benchmark_analyze_nodes_full(nodes: Array) -> Dictionary:
	"""
	Benchmark method: Analyze nodes with ALL metadata (current approach).
	Returns instance_ids, node_types, node_names, parent_ids, collision_masks.
	"""
	if not _watcher:
		_find_watcher()
	if not _watcher:
		return {}

	var instance_ids = PackedInt64Array()
	var node_types = PackedStringArray()
	var node_names = PackedStringArray()
	var parent_ids = PackedInt64Array()
	var collision_masks = PackedInt64Array()

	for node in nodes:
		if not is_instance_valid(node):
			continue
		instance_ids.append(node.get_instance_id())
		node_types.append(_watcher._analyze_node_type(node))
		node_names.append(node.name)
		var parent = node.get_parent()
		parent_ids.append(parent.get_instance_id() if parent else 0)
		collision_masks.append(_watcher._compute_collision_mask(node))

	return {
		"instance_ids": instance_ids,
		"node_types": node_types,
		"node_names": node_names,
		"parent_ids": parent_ids,
		"collision_masks": collision_masks
	}

func benchmark_analyze_nodes_type_only(nodes: Array) -> Dictionary:
	"""
	Benchmark method: Analyze nodes with ONLY node_type (hybrid approach).
	Rust would get the rest (name, parent, collision) via FFI.
	"""
	if not _watcher:
		_find_watcher()
	if not _watcher:
		return {}

	var instance_ids = PackedInt64Array()
	var node_types = PackedStringArray()

	for node in nodes:
		if not is_instance_valid(node):
			continue
		instance_ids.append(node.get_instance_id())
		node_types.append(_watcher._analyze_node_type(node))

	return {
		"instance_ids": instance_ids,
		"node_types": node_types
	}

func benchmark_analyze_nodes_full_with_groups(nodes: Array) -> Dictionary:
	"""
	Benchmark method: Analyze nodes with ALL metadata INCLUDING groups.
	Tests if adding groups to bulk analysis is worthwhile.
	"""
	if not _watcher:
		_find_watcher()
	if not _watcher:
		return {}

	var instance_ids = PackedInt64Array()
	var node_types = PackedStringArray()
	var node_names = PackedStringArray()
	var parent_ids = PackedInt64Array()
	var collision_masks = PackedInt64Array()
	var groups = []  # Array of PackedStringArrays

	for node in nodes:
		if not is_instance_valid(node):
			continue
		instance_ids.append(node.get_instance_id())
		node_types.append(_watcher._analyze_node_type(node))
		node_names.append(node.name)
		var parent = node.get_parent()
		parent_ids.append(parent.get_instance_id() if parent else 0)
		collision_masks.append(_watcher._compute_collision_mask(node))

		# Collect groups for this node
		var node_groups = PackedStringArray()
		for group in node.get_groups():
			node_groups.append(group)
		groups.append(node_groups)

	return {
		"instance_ids": instance_ids,
		"node_types": node_types,
		"node_names": node_names,
		"parent_ids": parent_ids,
		"collision_masks": collision_masks,
		"groups": groups
	}
