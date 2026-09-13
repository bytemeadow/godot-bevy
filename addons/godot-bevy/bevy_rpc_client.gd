@tool
extends RefCounted

signal sessions_changed
signal active_session_changed(session_id)
signal summary_received(session_id, params)
signal interval_changed
signal connection_error(reason)

var sessions: Dictionary = {}
var active_session_id := -1
var update_interval := 0.5
var config_ready := false
var _next_id := 0
var _start_order := 0
var _pending: Dictionary = {}
var _last_frames: Array = []

const RETRY_INTERVAL := 0.5
const CONFIG_TIMEOUT := 20.0

func setup_session(session_id: int, session) -> void:
	var started = _started.bind(session_id)
	var stopped = _stopped.bind(session_id)
	sessions[session_id] = {"session": session, "active": false, "order": 0,
		"started": started, "stopped": stopped, "interval": 0.5,
		"config_ready": false, "config_id": -1, "elapsed": 0.0, "next_retry": RETRY_INTERVAL, "error": ""}
	session.started.connect(started)
	session.stopped.connect(stopped)
	if session.is_active():
		_started(session_id)

func _started(session_id: int) -> void:
	_cancel_pending(session_id, "session restarted")
	var record: Dictionary = sessions[session_id]
	record.config_ready = false
	record.config_id = -1
	record.elapsed = 0.0
	record.next_retry = RETRY_INTERVAL
	record.error = ""
	_start_order += 1
	sessions[session_id].active = true
	sessions[session_id].order = _start_order
	sessions_changed.emit()
	select_session(session_id)

func _stopped(session_id: int) -> void:
	sessions[session_id].active = false
	sessions[session_id].config_ready = false
	sessions[session_id].config_id = -1
	_cancel_pending(session_id, "session stopped")
	if active_session_id == session_id:
		var newest := -1
		for id in sessions:
			if sessions[id].active and (newest == -1 or sessions[id].order > sessions[newest].order):
				newest = id
		select_session(newest)
	sessions_changed.emit()

func is_session_active(session_id: int) -> bool:
	return sessions.has(session_id) and sessions[session_id].active

func select_session(session_id: int) -> void:
	if session_id != -1 and not is_session_active(session_id):
		return
	if active_session_id == session_id:
		return
	active_session_id = session_id
	config_ready = sessions[session_id].config_ready if session_id != -1 else false
	update_interval = sessions[session_id].interval if session_id != -1 else 0.5
	active_session_changed.emit(session_id)
	if session_id != -1 and not sessions[session_id].error.is_empty():
		connection_error.emit(sessions[session_id].error)

func advance(delta: float) -> void:
	for session_id in sessions:
		var record: Dictionary = sessions[session_id]
		if not record.active or record.config_ready or not record.error.is_empty():
			continue
		record.elapsed += delta
		if record.elapsed >= CONFIG_TIMEOUT:
			_config_error(session_id, "Bevy debugger did not answer within 20 s")
		elif record.elapsed >= record.next_retry:
			_request_config(session_id)

func _request_config(session_id: int) -> void:
	var record: Dictionary = sessions[session_id]
	if record.config_ready or not record.error.is_empty():
		return
	record.next_retry = record.elapsed + RETRY_INTERVAL
	if record.config_id != -1:
		_send(session_id, _pending[record.config_id].frame)
		return
	record.config_id = request("godot.debugger_config", {}, func(frame):
		record.config_id = -1
		if not is_session_active(session_id):
			return
		if frame.has("error"):
			_config_error(session_id, frame.error.message)
			return
		var interval = frame.result.get("update_interval", 0.5)
		if not (interval is float or interval is int) or not is_finite(float(interval)) or float(interval) < 0.0:
			_config_error(session_id, "Invalid DebuggerConfig.update_interval")
			return
		record.interval = float(interval)
		record.config_ready = true
		if active_session_id == session_id:
			config_ready = true
			update_interval = float(interval)
			interval_changed.emit()
	, session_id)

func _config_error(session_id: int, reason: String) -> void:
	var record: Dictionary = sessions[session_id]
	cancel_request(record.config_id)
	record.config_id = -1
	record.error = reason
	if active_session_id == session_id:
		connection_error.emit(reason)

# Recreate containers so typed arrays/dictionaries cannot reach wire.rs.
static func wire_value(value):
	match typeof(value):
		TYPE_NIL, TYPE_BOOL, TYPE_INT, TYPE_FLOAT, TYPE_STRING:
			return value
		TYPE_STRING_NAME, TYPE_NODE_PATH:
			return String(value)
		TYPE_DICTIONARY:
			var result: Dictionary = {}
			for key in value:
				result[str(key)] = wire_value(value[key])
			return result
		TYPE_ARRAY, TYPE_PACKED_BYTE_ARRAY, TYPE_PACKED_INT32_ARRAY, TYPE_PACKED_INT64_ARRAY, TYPE_PACKED_FLOAT32_ARRAY, TYPE_PACKED_FLOAT64_ARRAY, TYPE_PACKED_STRING_ARRAY:
			var result: Array = []
			for item in value:
				result.append(wire_value(item))
			return result
	push_error("Unsupported debugger request Variant: %s" % type_string(typeof(value)))
	return null

func request(method: String, params: Dictionary, callback: Callable = Callable(), session_id: int = -1) -> int:
	if session_id == -1:
		session_id = active_session_id
	_next_id += 1
	var id := _next_id
	if not is_session_active(session_id):
		if callback.is_valid():
			callback.call_deferred({"id": id, "error": {"message": "no running session"}})
		return id
	var frame = wire_value({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
	if callback.is_valid():
		_pending[id] = {"session": session_id, "callback": callback, "frame": frame}
	_send(session_id, frame)
	return id

func _send(session_id: int, frame: Dictionary) -> void:
	sessions[session_id].session.send_message("bevy:rpc", [frame])

func cancel_request(id: int) -> void:
	_pending.erase(id)

func _capture(message: String, data: Array, session_id: int) -> bool:
	if message != "bevy:rpc":
		return false
	_last_frames.append({"session": session_id, "message": message, "data": data.duplicate(true)})
	if _last_frames.size() > 5:
		_last_frames.pop_front()
	if not is_session_active(session_id) or data.size() != 1 or not data[0] is Dictionary:
		return true
	var frame: Dictionary = data[0]
	if frame.get("jsonrpc") != "2.0":
		return true
	if frame.has("id"):
		var id = frame.id
		if _pending.has(id) and _pending[id].session == session_id:
			var callback: Callable = _pending[id].callback
			_pending.erase(id)
			if callback.is_valid():
				callback.call(frame)
	elif frame.get("method") == "godot.ready" and frame.get("params") is Dictionary:
		_request_config(session_id)
	elif frame.get("method") == "godot.summary" and frame.get("params") is Dictionary:
		summary_received.emit(session_id, frame.params)
	return true

func _cancel_pending(session_id: int, reason: String) -> void:
	for id in _pending.keys():
		if _pending[id].session == session_id:
			var callback: Callable = _pending[id].callback
			_pending.erase(id)
			if callback.is_valid():
				callback.call({"id": id, "error": {"message": reason}})

func diagnostics() -> Dictionary:
	var pending: Dictionary = {}
	for id in _pending:
		pending[id] = {"session": _pending[id].session, "frame": _pending[id].frame.duplicate(true)}
	return {"active_session_id": active_session_id, "config_ready": config_ready,
		"pending": pending, "last_frames": _last_frames.duplicate(true)}

func shutdown() -> void:
	for id in sessions:
		if is_session_active(id):
			request("godot.unsubscribe", {}, Callable(), id)
		sessions[id].active = false
		_cancel_pending(id, "debugger detached")
		var record: Dictionary = sessions[id]
		record.session.started.disconnect(record.started)
		record.session.stopped.disconnect(record.stopped)
	sessions.clear()
	active_session_id = -1
	config_ready = false
	active_session_changed.emit(-1)
	sessions_changed.emit()
