extends RefCounted
## Palette, fonts, StyleBox helpers and the Theme of the Ops board.
## Look: dark warm stone, thin gold lines, serif headings, parchment stage pills.

const BG: Color = Color(0.050, 0.046, 0.043, 0.97)
const CARD: Color = Color(0.085, 0.078, 0.070, 1.0)
const CARD_HOVER: Color = Color(0.125, 0.112, 0.092, 1.0)
const GOLD: Color = Color(0.80, 0.65, 0.24)
const GOLD_DIM: Color = Color(0.80, 0.65, 0.24, 0.30)
const TEXT: Color = Color(0.93, 0.89, 0.78)
const MUTED: Color = Color(0.62, 0.58, 0.50)
const DIM: Color = Color(0.42, 0.40, 0.36)
const PARCHMENT: Color = Color(0.92, 0.86, 0.70)
const INK: Color = Color(0.13, 0.10, 0.06)

const BLUE: Color = Color(0.31, 0.63, 1.0)
const GREEN: Color = Color(0.32, 0.78, 0.40)
const RED: Color = Color(0.93, 0.34, 0.31)
const AMBER: Color = Color(0.96, 0.73, 0.26)
const GREY: Color = Color(0.60, 0.60, 0.62)

const SERIF: PackedStringArray = ["Georgia", "Palatino", "Cambria", "Times New Roman", "Noto Serif", "DejaVu Serif"]
const SANS: PackedStringArray = ["Helvetica Neue", "Inter", "Segoe UI", "Roboto", "Arial", "DejaVu Sans"]
const MONO: PackedStringArray = ["Menlo", "SF Mono", "Consolas", "DejaVu Sans Mono", "Courier New"]

static var _fonts: Dictionary = {}


static func status_color(status: String) -> Color:
	match status:
		"running", "working":
			return BLUE
		"passed", "done":
			return GREEN
		"failed":
			return RED
		"stalled":
			return AMBER
		"canceled", "cancelled":
			return GREY
		"skipped":
			return Color(0.50, 0.50, 0.55)
		_:
			return Color(0.52, 0.50, 0.46)


static func status_label(status: String) -> String:
	return status.to_upper()


static func font(kind: String, bold: bool = false) -> Font:
	var key: String = "%s:%s" % [kind, bold]
	if _fonts.has(key):
		return _fonts[key] as Font
	var f: SystemFont = SystemFont.new()
	match kind:
		"serif":
			f.font_names = SERIF
		"mono":
			f.font_names = MONO
		_:
			f.font_names = SANS
	f.font_weight = 700 if bold else 400
	f.antialiasing = TextServer.FONT_ANTIALIASING_LCD
	f.subpixel_positioning = TextServer.SUBPIXEL_POSITIONING_AUTO
	_fonts[key] = f
	return f


static func box(bg: Color, border: Color = Color(0, 0, 0, 0), border_w: int = 0,
		radius: int = 6, margin: Vector4 = Vector4(0, 0, 0, 0)) -> StyleBoxFlat:
	var sb: StyleBoxFlat = StyleBoxFlat.new()
	sb.bg_color = bg
	sb.border_color = border
	sb.set_border_width_all(border_w)
	sb.set_corner_radius_all(radius)
	sb.content_margin_left = margin.x
	sb.content_margin_top = margin.y
	sb.content_margin_right = margin.z
	sb.content_margin_bottom = margin.w
	sb.anti_aliasing = true
	return sb


static func label(text: String, size: int = 13, color: Color = TEXT, kind: String = "sans",
		bold: bool = false) -> Label:
	var l: Label = Label.new()
	l.text = text
	l.add_theme_font_override("font", font(kind, bold))
	l.add_theme_font_size_override("font_size", size)
	l.add_theme_color_override("font_color", color)
	l.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return l


## Let clicks fall through to the owning button (except nodes tagged "keep_mouse").
static func ignore_mouse(node: Node) -> void:
	if node is Control and not node.has_meta("keep_mouse"):
		(node as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
	for c: Node in node.get_children():
		ignore_mouse(c)


static func make_theme() -> Theme:
	var t: Theme = Theme.new()
	t.default_font = font("sans")
	t.default_font_size = 13
	t.set_color("font_color", "Label", TEXT)
	# Buttons (toolbar buttons, option buttons).
	for cls: String in ["Button", "OptionButton"]:
		t.set_stylebox("normal", cls, box(Color(1, 1, 1, 0.05), GOLD_DIM, 1, 6, Vector4(10, 4, 10, 4)))
		t.set_stylebox("hover", cls, box(Color(0.80, 0.65, 0.24, 0.16), GOLD, 1, 6, Vector4(10, 4, 10, 4)))
		t.set_stylebox("pressed", cls, box(Color(0.80, 0.65, 0.24, 0.26), GOLD, 1, 6, Vector4(10, 4, 10, 4)))
		t.set_stylebox("focus", cls, StyleBoxEmpty.new())
		t.set_color("font_color", cls, TEXT)
		t.set_color("font_hover_color", cls, Color(1, 0.95, 0.8))
		t.set_color("font_pressed_color", cls, GOLD)
		t.set_color("font_focus_color", cls, TEXT)
	t.set_font_size("font_size", "Button", 12)
	t.set_font_size("font_size", "OptionButton", 12)
	# Popup of the option buttons.
	t.set_stylebox("panel", "PopupMenu", box(Color(0.07, 0.065, 0.058, 1), GOLD, 1, 6, Vector4(4, 4, 4, 4)))
	t.set_stylebox("hover", "PopupMenu", box(Color(0.80, 0.65, 0.24, 0.22), Color(0, 0, 0, 0), 0, 4))
	t.set_color("font_color", "PopupMenu", TEXT)
	t.set_color("font_hover_color", "PopupMenu", Color(1, 0.95, 0.8))
	t.set_font_size("font_size", "PopupMenu", 12)
	# Thin scroll bars.
	for cls: String in ["VScrollBar", "HScrollBar"]:
		t.set_stylebox("scroll", cls, box(Color(1, 1, 1, 0.03), Color(0, 0, 0, 0), 0, 3, Vector4(3, 3, 3, 3)))
		t.set_stylebox("grabber", cls, box(Color(0.80, 0.65, 0.24, 0.45), Color(0, 0, 0, 0), 0, 3))
		t.set_stylebox("grabber_highlight", cls, box(Color(0.80, 0.65, 0.24, 0.8), Color(0, 0, 0, 0), 0, 3))
		t.set_stylebox("grabber_pressed", cls, box(GOLD, Color(0, 0, 0, 0), 0, 3))
	# Tooltips.
	t.set_stylebox("panel", "TooltipPanel", box(Color(0.06, 0.055, 0.05, 0.98), GOLD, 1, 6, Vector4(10, 7, 10, 7)))
	t.set_color("font_color", "TooltipLabel", TEXT)
	t.set_font_size("font_size", "TooltipLabel", 12)
	return t
