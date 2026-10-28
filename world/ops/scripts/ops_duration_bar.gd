extends Control
## "Time per stage" bar: one segment per stage, width proportional to its seconds.
## `segments` is an Array of {name, seconds, status}.

const OpsTheme = preload("res://ops/scripts/ops_theme.gd")

var segments: Array = []:
	set(value):
		segments = value
		queue_redraw()


func _init() -> void:
	custom_minimum_size = Vector2(0, 8)
	mouse_filter = Control.MOUSE_FILTER_PASS


func _draw() -> void:
	var track: StyleBoxFlat = OpsTheme.box(Color(1, 1, 1, 0.06), Color(0, 0, 0, 0), 0, 4)
	draw_style_box(track, Rect2(Vector2.ZERO, size))
	var total: float = 0.0
	for s: Variant in segments:
		total += float((s as Dictionary).get("seconds", 0))
	if total <= 0.0:
		return
	var gap: float = 2.0
	var usable: float = size.x - gap * float(segments.size() - 1)
	var x: float = 0.0
	for s: Variant in segments:
		var seg: Dictionary = s
		var sec: float = float(seg.get("seconds", 0))
		if sec <= 0.0:
			continue
		var w: float = maxf(usable * sec / total, 3.0)
		var col: Color = OpsTheme.status_color(str(seg.get("status", "pending")))
		draw_style_box(OpsTheme.box(col, Color(0, 0, 0, 0), 0, 3), Rect2(x, 0, w, size.y))
		x += w + gap
