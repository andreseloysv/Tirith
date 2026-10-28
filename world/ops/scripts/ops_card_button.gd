extends Button
## A flat Button that hosts a layout (its "content" Control). Its minimum size follows the
## content, so rows and stage pills can hold labels and icons and still be real buttons.
## (Button ignores a scripted `_get_minimum_size`, so this drives `custom_minimum_size`.)

var _content: Control = null
var _floor: Vector2 = Vector2.ZERO


func set_content(content: Control) -> void:
	_floor = custom_minimum_size
	_content = content
	add_child(content)
	content.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	content.minimum_size_changed.connect(_sync_min_size)
	_sync_min_size()


func _sync_min_size() -> void:
	if _content == null:
		return
	var m: Vector2 = _content.get_combined_minimum_size()
	custom_minimum_size = Vector2(maxf(m.x, _floor.x), maxf(m.y, _floor.y))
