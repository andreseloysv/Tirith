extends Control
## Small vector icons: status glyphs (spinner, check, cross...), the class badge, chevrons.
## Drawn in code so the board has no image assets and stays crisp at any scale.

var kind: String = "pending"
var col: Color = Color.WHITE
var glyph: String = ""


func setup(icon_kind: String, color: Color, px: int = 16, letter: String = "") -> Control:
	kind = icon_kind
	col = color
	glyph = letter
	custom_minimum_size = Vector2(px, px)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_process(kind == "running")
	queue_redraw()
	return self


func _process(_delta: float) -> void:
	queue_redraw()


func _draw() -> void:
	var c: Vector2 = size / 2.0
	var r: float = minf(size.x, size.y) / 2.0 - 1.5
	var w: float = maxf(1.6, r * 0.18)
	var faint: Color = Color(col.r, col.g, col.b, 0.28)
	match kind:
		"running":
			draw_arc(c, r, 0, TAU, 40, faint, w, true)
			var a: float = fmod(float(Time.get_ticks_msec()) / 1000.0 * TAU * 1.3, TAU)
			draw_arc(c, r, a, a + TAU * 0.3, 16, col, w + 0.4, true)
		"passed":
			draw_circle(c, r, faint)
			draw_arc(c, r, 0, TAU, 40, col, w, true)
			draw_polyline(PackedVector2Array([c + Vector2(-r * 0.42, 0), c + Vector2(-r * 0.1, r * 0.34),
				c + Vector2(r * 0.46, -r * 0.34)]), col, w, true)
		"failed":
			draw_circle(c, r, faint)
			draw_arc(c, r, 0, TAU, 40, col, w, true)
			var k: float = r * 0.36
			draw_line(c + Vector2(-k, -k), c + Vector2(k, k), col, w, true)
			draw_line(c + Vector2(-k, k), c + Vector2(k, -k), col, w, true)
		"canceled":
			draw_arc(c, r, 0, TAU, 40, col, w, true)
			var d: float = r * 0.5
			draw_line(c + Vector2(-d, d), c + Vector2(d, -d), col, w, true)
		"skipped":
			draw_arc(c, r, 0, TAU, 40, faint, w, true)
			draw_colored_polygon(PackedVector2Array([c + Vector2(-r * 0.25, -r * 0.4),
				c + Vector2(r * 0.4, 0), c + Vector2(-r * 0.25, r * 0.4)]), col)
		"stalled":
			draw_circle(c, r, faint)
			draw_arc(c, r, 0, TAU, 40, col, w, true)
			draw_line(c + Vector2(-r * 0.22, -r * 0.38), c + Vector2(-r * 0.22, r * 0.38), col, w)
			draw_line(c + Vector2(r * 0.22, -r * 0.38), c + Vector2(r * 0.22, r * 0.38), col, w)
		"pending":
			draw_arc(c, r, 0, TAU, 40, Color(col.r, col.g, col.b, 0.55), w, true)
			draw_circle(c, r * 0.2, Color(col.r, col.g, col.b, 0.55))
		"class":
			var pts: PackedVector2Array = PackedVector2Array()
			for i: int in 6:
				var ang: float = TAU * float(i) / 6.0 - PI / 2.0
				pts.append(c + Vector2(cos(ang), sin(ang)) * (r + 0.5))
			draw_colored_polygon(pts, Color(col.r * 0.45, col.g * 0.45, col.b * 0.45, 1.0))
			pts.append(pts[0])
			draw_polyline(pts, col, 1.6, true)
			var f: Font = get_theme_default_font()
			var fs: int = int(size.y * 0.5)
			draw_string(f, Vector2(0, c.y + fs * 0.36), glyph, HORIZONTAL_ALIGNMENT_CENTER, size.x, fs,
				Color(1, 0.96, 0.86))
		"mark":
			var pts2: PackedVector2Array = PackedVector2Array([c + Vector2(0, -r), c + Vector2(r * 0.8, 0),
				c + Vector2(0, r), c + Vector2(-r * 0.8, 0)])
			draw_colored_polygon(pts2, Color(col.r, col.g, col.b, 0.22))
			pts2.append(pts2[0])
			draw_polyline(pts2, col, 1.6, true)
			draw_circle(c, r * 0.22, col)
		"gem":
			var pts3: PackedVector2Array = PackedVector2Array([c + Vector2(0, -r), c + Vector2(r * 0.75, 0),
				c + Vector2(0, r), c + Vector2(-r * 0.75, 0)])
			draw_colored_polygon(pts3, col)
		"arrow":
			draw_line(Vector2(1, c.y), Vector2(size.x - 2, c.y), col, 1.8, true)
			draw_polyline(PackedVector2Array([Vector2(size.x - 2 - r * 0.5, c.y - r * 0.45),
				Vector2(size.x - 2, c.y), Vector2(size.x - 2 - r * 0.5, c.y + r * 0.45)]), col, 1.8, true)
		"link":
			draw_arc(c + Vector2(-r * 0.3, r * 0.3), r * 0.5, 0, TAU, 20, col, 1.8, true)
			draw_arc(c + Vector2(r * 0.3, -r * 0.3), r * 0.5, 0, TAU, 20, col, 1.8, true)
		"owner":
			draw_circle(c + Vector2(0, -r * 0.3), r * 0.36, col)
			draw_arc(c + Vector2(0, r * 0.95), r * 0.75, PI * 1.12, PI * 1.88, 16, col, 2.4, true)
		"guard":
			var sp: PackedVector2Array = PackedVector2Array([c + Vector2(-r * 0.7, -r * 0.8),
				c + Vector2(r * 0.7, -r * 0.8), c + Vector2(r * 0.7, r * 0.1), c + Vector2(0, r),
				c + Vector2(-r * 0.7, r * 0.1)])
			draw_colored_polygon(sp, Color(col.r, col.g, col.b, 0.3))
			sp.append(sp[0])
			draw_polyline(sp, col, 1.6, true)
		"room":
			draw_rect(Rect2(c - Vector2(r * 0.7, r * 0.7), Vector2(r * 1.4, r * 1.4)), Color(col.r, col.g, col.b, 0.3))
			draw_rect(Rect2(c - Vector2(r * 0.7, r * 0.7), Vector2(r * 1.4, r * 1.4)), col, false, 1.6)
		"pipeline":
			for i: int in 3:
				draw_circle(Vector2(size.x * (0.2 + 0.3 * i), c.y), r * 0.2, col)
		"chevron_down":
			draw_polyline(PackedVector2Array([c + Vector2(-r * 0.5, -r * 0.2), c + Vector2(0, r * 0.3),
				c + Vector2(r * 0.5, -r * 0.2)]), col, 1.8, true)
		"chevron_left":
			draw_polyline(PackedVector2Array([c + Vector2(r * 0.2, -r * 0.5), c + Vector2(-r * 0.3, 0),
				c + Vector2(r * 0.2, r * 0.5)]), col, 1.8, true)
		"chevron_right":
			draw_polyline(PackedVector2Array([c + Vector2(-r * 0.2, -r * 0.5), c + Vector2(r * 0.3, 0),
				c + Vector2(-r * 0.2, r * 0.5)]), col, 1.8, true)
		"connector":
			draw_line(Vector2(0, c.y), Vector2(size.x, c.y), col, 2.0, true)
		_:
			draw_circle(c, r, col)
