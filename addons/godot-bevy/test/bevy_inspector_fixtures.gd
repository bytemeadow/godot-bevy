extends RefCounted

# Hand-authored from service.rs / wire.rs; no live capture was run for this round.
static func entity_ref(bits: String) -> Dictionary:
	return {"bits": bits, "generation": 1}

static func row(bits: String, name: String, components: Array, parent = null, node: bool = false) -> Dictionary:
	return {"entity": entity_ref(bits), "name": name, "parent": parent, "has_node": node,
		"components": components, "unsupported_components": {}}

static func snapshot() -> Dictionary:
	return {"jsonrpc": "2.0", "method": "godot.summary", "params": {
		"snapshot": true, "added": [
			row("4294967297", "MainMenu", ["godot_bevy::interop::GodotNodeHandle"], null, true),
			row("4294967298", "Player", ["game::Speed"], entity_ref("4294967297")),
			row("4294967299", "", ["bevy_ecs::observer::Observer"]),
			row("4294967300", "", ["bevy_ecs::resource::IsResource", "bevy_time::time::Time<()>"]),
			row("4294967301", "", []),
			row("4294967302", "", ["game::Unregistered"]),
			row("4294967303", "", ["bevy_ecs::name::Name"]),
		], "removed": [], "updated": []}}

static func delta() -> Dictionary:
	return {"jsonrpc": "2.0", "method": "godot.summary", "params": {
		"snapshot": false,
		"added": [row("4294967304", "Child", ["game::Speed"], entity_ref("4294967298"))],
		"removed": [entity_ref("4294967302")],
		"updated": [row("4294967298", "Player renamed", ["game::Speed"], entity_ref("4294967297"))]}}

static func scalar(kind: String, value, allowed: bool = true) -> Dictionary:
	return {"type_path": {"integer": "i32", "float": "f32", "bool": "bool", "char": "char", "string": "alloc::string::String"}.get(kind, "game::Value"),
		"kind": kind, "value": value,
		"writable": {"allowed": allowed, "reason": "InspectorReadOnly"} if not allowed else {"allowed": true}}

static func aggregate(kind: String, extra: Dictionary) -> Dictionary:
	var value = {"type_path": "game::Value", "kind": kind,
		"writable": {"allowed": false, "reason": "kind not editable"}}
	value.merge(extra)
	return value

static func values() -> Dictionary:
	var leaf = scalar("float", 4.0)
	var readonly = scalar("float", 4.0, false)
	return {
		"integer": scalar("integer", 7), "float": leaf, "bool": scalar("bool", true),
		"string": scalar("string", "hello"), "char": scalar("char", "λ"),
		"struct": aggregate("struct", {"fields": [{"name": "speed", "value": leaf}]}),
		"tuple_struct": aggregate("tuple_struct", {"fields": [leaf]}),
		"tuple": aggregate("tuple", {"fields": [leaf]}),
		"list": aggregate("list", {"items": [leaf], "truncated": 2}),
		"array": aggregate("array", {"items": [leaf], "truncated": 0}),
		"map": aggregate("map", {"entries": [{"key": scalar("string", "key", false), "value": readonly}], "truncated": 0}),
		"set": aggregate("set", {"members": [readonly], "truncated": 0}),
		"enum": {"type_path": "game::Mode", "kind": "enum", "variant": "Run", "fields": [],
			"unit_variants": ["Run", "Idle"], "writable": {"allowed": true}},
		"entity": aggregate("entity", {"entity": entity_ref("4294967298")}),
		"node": aggregate("node", {"instance_id": "31255954925", "valid": true}),
		"asset": aggregate("asset", {"id": "AssetId(7)"}),
		"opaque": aggregate("opaque", {"debug": "Opaque(7)"}),
		"unsupported": aggregate("unsupported", {"reason": "component not registered"}),
		"depth_limit": aggregate("depth_limit", {"reason": "maximum depth reached"}),
	}
