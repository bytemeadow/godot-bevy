extends CanvasLayer


func _on_show_score_toggled(toggled_on: bool) -> void:
	BevyAppSingleton.send_event("show_score_changed", toggled_on)
