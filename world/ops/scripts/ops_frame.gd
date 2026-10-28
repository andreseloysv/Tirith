extends PanelContainer
## Dark stone panel with a thin gold inner line and corner brackets (the art direction of
## docs/inspiration/gitlab_overlay_1.png). Pure decoration drawn over the background.

var gold: Color = Color(0.78, 0.63, 0.22)
var inset: float = 6.0
var bracket: float = 10.0


func _draw() -> void:
	var r: Rect2 = Rect2(Vector2(inset, inset), size - Vector2(inset, inset) * 2.0)
	var line: Color = Color(gold.r, gold.g, gold.b, 0.45)
	draw_rect(r, line, false, 1.0)
	var bright: Color = Color(gold.r, gold.g, gold.b, 0.95)
	var p: Vector2 = r.position
	var q: Vector2 = r.end
	var b: float = bracket
	for corner: Array in [[p, 1.0, 1.0], [Vector2(q.x, p.y), -1.0, 1.0],
			[Vector2(p.x, q.y), 1.0, -1.0], [q, -1.0, -1.0]]:
		var o: Vector2 = corner[0]
		var sx: float = corner[1]
		var sy: float = corner[2]
		draw_polyline(PackedVector2Array([o + Vector2(sx * b, 0), o, o + Vector2(0, sy * b)]),
			bright, 1.5)
