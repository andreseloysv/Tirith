extends CanvasLayer
## The Tirith Ops board: a GitLab-style 2D overlay over the 3D citadel.
##
## A always-visible header bar (counters) plus a dock on the right with three tabs:
## Pipelines (runs, stage graph, live durations, log tails), Agents (roster with live
## elapsed timers) and Waiting (the owner's to-do list). Toggle the dock with Tab.
##
## Feed it with `set_state(TirithState)`; it ticks its own timers every second between
## refreshes. It only reads state, never writes, and does not depend on the main scene.
## Emits `focus_agent(id)` when an agent row is clicked, so the world can fly the camera.

signal focus_agent(agent_id: String)
signal board_toggled(is_open: bool)

const OpsTime = preload("res://ops/scripts/ops_time.gd")
const OpsTheme = preload("res://ops/scripts/ops_theme.gd")
const OpsIcon = preload("res://ops/scripts/ops_icon.gd")
const OpsFrame = preload("res://ops/scripts/ops_frame.gd")
const OpsCardButton = preload("res://ops/scripts/ops_card_button.gd")
const OpsDurationBar = preload("res://ops/scripts/ops_duration_bar.gd")

const MARGIN: float = 12.0
const HEADER_H: float = 50.0
const MIN_WIDTH: int = 460
const MAX_WIDTH: int = 1100
const TAB_NAMES: Array = ["pipelines", "agents", "waiting", "messages"]
const TAB_TITLES: Dictionary = {"pipelines": "Pipelines", "agents": "Agents", "waiting": "Waiting", "messages": "Messages"}
const DAY: int = 86400
const FLOW_KIND_COLORS: Dictionary = {
	"brief": Color(0.31, 0.63, 1.0), "message": Color(0.78, 0.74, 0.64),
	"report": Color(0.32, 0.78, 0.40), "request": Color(0.80, 0.65, 0.24),
	"answer": Color(0.25, 0.75, 0.72), "decision": Color(0.70, 0.45, 0.92),
	"merge": Color(0.95, 0.55, 0.20), "alert": Color(0.93, 0.34, 0.31),
}

## Width of the dock in pixels (drag its left edge to resize).
var dock_width: int = 780
## Agent whose row is expanded ("" for none).
var selected_agent_id: String = ""

var _state: TirithState = TirithState.new()
var _open: bool = true
var _tab: String = "pipelines"
var _base_now: int = 0
var _set_ms: int = 0
var _manual_offset: int = 0
var _wall_clock: bool = true

var _filter_project: String = ""
var _filter_status: String = ""
var _agent_sort: String = "elapsed_desc"
var _expanded: Dictionary = {}
var _selected_stage: Dictionary = {}
var _run_ui: Dictionary = {}
var _agent_ui: Dictionary = {}
var _pl_updaters: Array = []
var _ag_updaters: Array = []
var _wt_updaters: Array = []
var _ms_updaters: Array = []
var _flow_filter_endpoint: String = ""
var _flow_filter_kind: String = ""
var _chip_ui: Dictionary = {}

var _root: Control
var _header: PanelContainer
var _dock: OpsFrame
var _resizer: Control
var _tab_buttons: Dictionary = {}
var _tab_pages: Dictionary = {}
var _pl_project: OptionButton
var _pl_status: OptionButton
var _pl_count: Label
var _pl_scroll: ScrollContainer
var _pl_list: VBoxContainer
var _ag_scroll: ScrollContainer
var _ag_list: VBoxContainer
var _ag_sort_buttons: Dictionary = {}
var _wt_scroll: ScrollContainer
var _wt_list: VBoxContainer
var _ms_endpoint: OptionButton
var _ms_kind: OptionButton
var _ms_count: Label
var _ms_scroll: ScrollContainer
var _ms_list: VBoxContainer
var _toggle_icon: Control
var _timer: Timer


func _ready() -> void:
	layer = 50
	_wall_clock = DisplayServer.get_name() != "headless"
	_base_now = int(Time.get_unix_time_from_system())
	_set_ms = Time.get_ticks_msec()
	_root = Control.new()
	_root.name = "Root"
	_root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_root.theme = OpsTheme.make_theme()
	add_child(_root)
	_build_header()
	_build_dock()
	_resizer = Control.new()
	_resizer.name = "Resizer"
	_resizer.mouse_default_cursor_shape = Control.CURSOR_HSIZE
	_resizer.gui_input.connect(_on_resizer_input)
	_root.add_child(_resizer)
	get_viewport().size_changed.connect(_layout)
	_timer = Timer.new()
	_timer.wait_time = 1.0
	_timer.timeout.connect(_run_updaters)
	add_child(_timer)
	if _wall_clock:
		_timer.start()
	_show_tab()
	_refresh_all()
	_layout()


# --- public API -----------------------------------------------------------------------


## Give the board a freshly parsed state. Timers are anchored at its `generated_at`.
func set_state(state: TirithState) -> void:
	_state = state
	var generated: int = OpsTime.parse_iso(_state.raw.get("generated_at", null))
	_base_now = generated if generated >= 0 else int(Time.get_unix_time_from_system())
	_set_ms = Time.get_ticks_msec()
	_manual_offset = 0
	if is_node_ready():
		_refresh_all()


func now_unix() -> int:
	var live: int = (Time.get_ticks_msec() - _set_ms) / 1000 if _wall_clock else 0
	return _base_now + _manual_offset + live


## Advance the clock one second by hand (tests, or a host that drives its own time).
func tick_once() -> void:
	_manual_offset += 1
	_run_updaters()


func is_board_open() -> bool:
	return _open


func set_board_open(value: bool) -> void:
	_open = value
	_layout()
	board_toggled.emit(_open)


func toggle_board() -> void:
	set_board_open(not _open)


func is_header_visible() -> bool:
	return _header != null and _header.visible


func set_header_visible(value: bool) -> void:
	_header.visible = value
	_layout()


func current_tab_name() -> String:
	return _tab


## Switch tab by name ("pipelines", "agents", "waiting"); unknown names are ignored.
func show_tab_named(tab_name: String) -> void:
	if not TAB_NAMES.has(tab_name):
		return
	_tab = tab_name
	_show_tab()


func set_dock_width(width: int) -> void:
	dock_width = clampi(width, MIN_WIDTH, MAX_WIDTH)
	_layout()


func header_counts() -> Dictionary:
	var now: int = now_unix()
	var c: Dictionary = {"pipelines_running": 0, "pipelines_failed_24h": 0, "agents_working": 0,
		"agents_stalled": 0, "agents_failed": 0, "waiting": _state.waiting().size()}
	for p: Variant in _state.pipelines():
		var run: Dictionary = p
		var status: String = str(run.get("status", ""))
		if status == "running":
			c["pipelines_running"] = int(c["pipelines_running"]) + 1
		elif status == "failed":
			var when: int = OpsTime.parse_iso(run.get("ended_at", null))
			if when < 0:
				when = OpsTime.parse_iso(run.get("started_at", null))
			if when >= 0 and now - when <= DAY:
				c["pipelines_failed_24h"] = int(c["pipelines_failed_24h"]) + 1
	for a: Variant in _state.agents():
		match str((a as Dictionary).get("status", "")):
			"working":
				c["agents_working"] = int(c["agents_working"]) + 1
			"stalled":
				c["agents_stalled"] = int(c["agents_stalled"]) + 1
			"failed":
				c["agents_failed"] = int(c["agents_failed"]) + 1
	return c


func set_pipeline_filter(project_id: String, status: String) -> void:
	_filter_project = project_id
	_filter_status = status
	_sync_filter_widgets()
	_rebuild_pipelines()


func pipeline_run_ids() -> Array:
	var ids: Array = []
	for run: Dictionary in _visible_runs():
		ids.append(str(run.get("run", "")))
	return ids


func select_stage(run_id: String, stage_name: String) -> void:
	var run: Dictionary = _run_by_id(run_id)
	for s: Variant in run.get("stages", []) as Array:
		if str((s as Dictionary).get("name", "")) == stage_name:
			_selected_stage[run_id] = stage_name
			_apply_stage_selection(run_id)
			return


func selected_stage_name(run_id: String) -> String:
	if _selected_stage.has(run_id):
		return str(_selected_stage[run_id])
	return str(_default_stage(_run_by_id(run_id)).get("name", ""))


func selected_stage_log(run_id: String) -> String:
	var name_sel: String = selected_stage_name(run_id)
	for s: Variant in _run_by_id(run_id).get("stages", []) as Array:
		if str((s as Dictionary).get("name", "")) == name_sel:
			return _log_text(s as Dictionary)
	return ""


func set_agent_sort(mode: String) -> void:
	if not ["elapsed_desc", "status", "project"].has(mode):
		return
	_agent_sort = mode
	for key: String in _ag_sort_buttons:
		(_ag_sort_buttons[key] as Button).set_pressed_no_signal(key == mode)
	_rebuild_agents()


func agent_row_ids() -> Array:
	var ids: Array = []
	for a: Dictionary in _sorted_agents():
		ids.append(str(a.get("id", "")))
	return ids


func waiting_ids() -> Array:
	var ids: Array = []
	for w: Dictionary in _sorted_waiting():
		ids.append(str(w.get("id", "")))
	return ids


func _input(event: InputEvent) -> void:
	var key: InputEventKey = event as InputEventKey
	if key != null and key.pressed and not key.echo and key.keycode == KEY_TAB \
			and not key.ctrl_pressed and not key.alt_pressed and not key.meta_pressed:
		toggle_board()
		if get_viewport() != null:
			get_viewport().set_input_as_handled()


# --- layout ---------------------------------------------------------------------------


func _layout() -> void:
	if _root == null:
		return
	var vp: Vector2 = get_viewport().get_visible_rect().size
	var full_w: float = minf(float(dock_width), vp.x - MARGIN * 2.0)
	var header_min: float = _header.get_combined_minimum_size().x
	var w: float = full_w if _open else header_min
	_header.position = Vector2(vp.x - w - MARGIN, MARGIN)
	_header.size = Vector2(w, HEADER_H)
	# Chip captions only fit when the header is wide enough.
	var compact: bool = _open and full_w < 760.0
	for chip_name: String in _chip_ui:
		((_chip_ui[chip_name] as Dictionary)["word"] as Control).visible = not compact
	_dock.visible = _open
	var top: float = MARGIN + HEADER_H + 6.0
	_dock.position = Vector2(vp.x - full_w - MARGIN, top)
	_dock.size = Vector2(full_w, maxf(vp.y - top - MARGIN, 120.0))
	_resizer.visible = _open
	_resizer.position = Vector2(_dock.position.x - 4.0, top)
	_resizer.size = Vector2(8.0, _dock.size.y)
	_toggle_icon.set("kind", "chevron_right" if _open else "chevron_left")
	_toggle_icon.queue_redraw()


func _on_resizer_input(event: InputEvent) -> void:
	var motion: InputEventMouseMotion = event as InputEventMouseMotion
	if motion != null and (motion.button_mask & MOUSE_BUTTON_MASK_LEFT) != 0:
		var vp_w: float = get_viewport().get_visible_rect().size.x
		set_dock_width(int(vp_w - MARGIN - get_viewport().get_mouse_position().x))


# --- header ---------------------------------------------------------------------------


func _build_header() -> void:
	_header = PanelContainer.new()
	_header.name = "Header"
	_header.add_theme_stylebox_override("panel",
		OpsTheme.box(OpsTheme.BG, OpsTheme.GOLD, 2, 10, Vector4(14, 6, 10, 6)))
	_root.add_child(_header)
	var row: HBoxContainer = HBoxContainer.new()
	row.add_theme_constant_override("separation", 4)
	_header.add_child(row)
	var mark: Control = OpsIcon.new().setup("mark", OpsTheme.GOLD, 24)
	row.add_child(mark)
	var title: Label = OpsTheme.label("TIRITH", 16, OpsTheme.GOLD, "serif", true)
	title.add_theme_constant_override("outline_size", 0)
	row.add_child(title)
	row.add_child(OpsTheme.label("OPS", 11, OpsTheme.MUTED, "sans", true))
	var spacer: Control = Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	spacer.custom_minimum_size = Vector2(8, 0)
	row.add_child(spacer)
	row.add_child(_make_chip("HeaderRunning", "running", OpsTheme.BLUE, "running", "Pipelines running"))
	row.add_child(_make_chip("HeaderFailed24h", "failed", OpsTheme.RED, "failed 24h", "Pipelines failed in the last 24 hours"))
	row.add_child(_vsep())
	row.add_child(_make_chip("HeaderWorking", "running", OpsTheme.BLUE, "working", "Agents working"))
	row.add_child(_make_chip("HeaderStalled", "stalled", OpsTheme.AMBER, "stalled", "Agents stalled (log silent for over 10 minutes)"))
	row.add_child(_make_chip("HeaderAgentsFailed", "failed", OpsTheme.RED, "failed", "Agents failed"))
	row.add_child(_vsep())
	row.add_child(_make_chip("HeaderWaiting", "gem", OpsTheme.GOLD, "waiting", "Items waiting for you"))
	var toggle: OpsCardButton = OpsCardButton.new()
	toggle.name = "HeaderToggle"
	toggle.tooltip_text = "Show or hide the board (Tab)"
	toggle.flat = true
	toggle.focus_mode = Control.FOCUS_NONE
	_toggle_icon = OpsIcon.new().setup("chevron_right", OpsTheme.GOLD, 20)
	var tm: MarginContainer = _margin(_toggle_icon, 6, 4, 6, 4)
	toggle.set_content(tm)
	toggle.pressed.connect(toggle_board)
	row.add_child(toggle)


func _vsep() -> Control:
	var s: ColorRect = ColorRect.new()
	s.color = OpsTheme.GOLD_DIM
	s.custom_minimum_size = Vector2(1, 24)
	s.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	s.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return s


func _make_chip(chip_name: String, icon_kind: String, color: Color, word: String, tip: String) -> Control:
	var chip: OpsCardButton = OpsCardButton.new()
	chip.name = chip_name
	chip.tooltip_text = tip
	chip.focus_mode = Control.FOCUS_NONE
	chip.add_theme_stylebox_override("normal", StyleBoxEmpty.new())
	chip.add_theme_stylebox_override("focus", StyleBoxEmpty.new())
	chip.add_theme_stylebox_override("pressed", OpsTheme.box(Color(1, 1, 1, 0.10), Color(0, 0, 0, 0), 0, 6))
	chip.add_theme_stylebox_override("hover", OpsTheme.box(Color(1, 1, 1, 0.07), Color(0, 0, 0, 0), 0, 6))
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 5)
	h.alignment = BoxContainer.ALIGNMENT_CENTER
	h.add_child(OpsIcon.new().setup(icon_kind, color, 16))
	var count: Label = OpsTheme.label("0", 15, OpsTheme.TEXT, "mono", true)
	h.add_child(count)
	var word_label: Label = OpsTheme.label(word, 11, OpsTheme.MUTED)
	h.add_child(word_label)
	chip.set_content(_margin(h, 7, 5, 7, 5))
	OpsTheme.ignore_mouse(chip.get_child(0))
	chip.pressed.connect(_on_chip_pressed.bind(chip_name))
	_chip_ui[chip_name] = {"count": count, "word": word_label, "color": color}
	return chip


func _on_chip_pressed(chip_name: String) -> void:
	if not _open:
		set_board_open(true)
	match chip_name:
		"HeaderRunning":
			set_pipeline_filter("", "running")
			show_tab_named("pipelines")
		"HeaderFailed24h":
			set_pipeline_filter("", "failed")
			show_tab_named("pipelines")
		"HeaderWaiting":
			show_tab_named("waiting")
		_:
			show_tab_named("agents")


func _update_header() -> void:
	var c: Dictionary = header_counts()
	var map: Dictionary = {"HeaderRunning": "pipelines_running", "HeaderFailed24h": "pipelines_failed_24h",
		"HeaderWorking": "agents_working", "HeaderStalled": "agents_stalled",
		"HeaderAgentsFailed": "agents_failed", "HeaderWaiting": "waiting"}
	for chip_name: String in map:
		var ui: Dictionary = _chip_ui[chip_name]
		var n: int = int(c[map[chip_name]])
		var lbl: Label = ui["count"]
		lbl.text = str(n)
		lbl.add_theme_color_override("font_color", (ui["color"] as Color) if n > 0 else OpsTheme.DIM)
	_tab_buttons["pipelines"].text = "Pipelines  %d" % _state.pipelines().size()
	_tab_buttons["agents"].text = "Agents  %d" % _state.agents().size()
	_tab_buttons["waiting"].text = "Waiting  %d" % _state.waiting().size()
	_tab_buttons["messages"].text = "Messages  %d" % _flows().size()


# --- dock and tabs --------------------------------------------------------------------


func _build_dock() -> void:
	_dock = OpsFrame.new()
	_dock.name = "Dock"
	_dock.add_theme_stylebox_override("panel",
		OpsTheme.box(OpsTheme.BG, OpsTheme.GOLD, 2, 10, Vector4(16, 14, 16, 14)))
	_root.add_child(_dock)
	var col: VBoxContainer = VBoxContainer.new()
	col.add_theme_constant_override("separation", 10)
	_dock.add_child(col)
	var tabs: HBoxContainer = HBoxContainer.new()
	tabs.add_theme_constant_override("separation", 4)
	col.add_child(tabs)
	var group: ButtonGroup = ButtonGroup.new()
	for tab_name: String in TAB_NAMES:
		var b: Button = Button.new()
		b.name = "Tab_" + tab_name
		b.toggle_mode = true
		b.button_group = group
		b.focus_mode = Control.FOCUS_NONE
		b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		b.add_theme_font_override("font", OpsTheme.font("serif", true))
		b.add_theme_font_size_override("font_size", 15)
		b.add_theme_stylebox_override("normal", OpsTheme.box(Color(1, 1, 1, 0.03), OpsTheme.GOLD_DIM, 1, 3, Vector4(6, 8, 6, 8)))
		b.add_theme_stylebox_override("hover", OpsTheme.box(Color(0.8, 0.65, 0.24, 0.10), OpsTheme.GOLD, 1, 3, Vector4(12, 8, 12, 8)))
		b.add_theme_stylebox_override("pressed", OpsTheme.box(Color(0.8, 0.65, 0.24, 0.20), OpsTheme.GOLD, 1, 3, Vector4(12, 8, 12, 8)))
		b.add_theme_color_override("font_pressed_color", Color(1.0, 0.92, 0.68))
		b.pressed.connect(show_tab_named.bind(tab_name))
		tabs.add_child(b)
		_tab_buttons[tab_name] = b
	var stack: Control = Control.new()
	stack.size_flags_vertical = Control.SIZE_EXPAND_FILL
	stack.clip_contents = true
	col.add_child(stack)
	_tab_pages["pipelines"] = _build_pipelines_page()
	_tab_pages["agents"] = _build_agents_page()
	_tab_pages["waiting"] = _build_waiting_page()
	_tab_pages["messages"] = _build_messages_page()
	for tab_name: String in TAB_NAMES:
		var page: Control = _tab_pages[tab_name]
		page.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		stack.add_child(page)


func _show_tab() -> void:
	for tab_name: String in TAB_NAMES:
		(_tab_pages[tab_name] as Control).visible = tab_name == _tab
		(_tab_buttons[tab_name] as Button).set_pressed_no_signal(tab_name == _tab)


func _scroll_page() -> Array:
	var scroll: ScrollContainer = ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	var list: VBoxContainer = VBoxContainer.new()
	list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	list.add_theme_constant_override("separation", 0)
	scroll.add_child(list)
	return [scroll, list]


func _build_pipelines_page() -> Control:
	var page: VBoxContainer = VBoxContainer.new()
	page.add_theme_constant_override("separation", 8)
	var bar: HBoxContainer = HBoxContainer.new()
	bar.add_theme_constant_override("separation", 6)
	page.add_child(bar)
	_pl_project = OptionButton.new()
	_pl_project.name = "ProjectFilter"
	_pl_project.focus_mode = Control.FOCUS_NONE
	_pl_project.item_selected.connect(func(idx: int) -> void:
		set_pipeline_filter(str(_pl_project.get_item_metadata(idx)), _filter_status))
	bar.add_child(_pl_project)
	_pl_status = OptionButton.new()
	_pl_status.name = "StatusFilter"
	_pl_status.focus_mode = Control.FOCUS_NONE
	for s: String in ["", "running", "passed", "failed", "canceled"]:
		_pl_status.add_item("All statuses" if s == "" else s.capitalize())
		_pl_status.set_item_metadata(_pl_status.item_count - 1, s)
	_pl_status.item_selected.connect(func(idx: int) -> void:
		set_pipeline_filter(_filter_project, str(_pl_status.get_item_metadata(idx))))
	bar.add_child(_pl_status)
	var sp: Control = Control.new()
	sp.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	bar.add_child(sp)
	_pl_count = OpsTheme.label("", 11, OpsTheme.MUTED)
	bar.add_child(_pl_count)
	var sl: Array = _scroll_page()
	_pl_scroll = sl[0]
	_pl_list = sl[1]
	page.add_child(_pl_scroll)
	return page


func _build_agents_page() -> Control:
	var page: VBoxContainer = VBoxContainer.new()
	page.add_theme_constant_override("separation", 8)
	var bar: HBoxContainer = HBoxContainer.new()
	bar.add_theme_constant_override("separation", 6)
	page.add_child(bar)
	bar.add_child(OpsTheme.label("Sort by", 11, OpsTheme.MUTED))
	var group: ButtonGroup = ButtonGroup.new()
	for mode: String in ["elapsed_desc", "status", "project"]:
		var b: Button = Button.new()
		b.name = "Sort_" + mode
		b.text = {"elapsed_desc": "Elapsed", "status": "Status", "project": "Project"}[mode]
		b.toggle_mode = true
		b.button_group = group
		b.focus_mode = Control.FOCUS_NONE
		b.button_pressed = mode == _agent_sort
		b.pressed.connect(set_agent_sort.bind(mode))
		bar.add_child(b)
		_ag_sort_buttons[mode] = b
	var sp: Control = Control.new()
	sp.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	bar.add_child(sp)
	bar.add_child(OpsTheme.label("click a hero to find them in the citadel", 11, OpsTheme.DIM))
	var sl: Array = _scroll_page()
	_ag_scroll = sl[0]
	_ag_list = sl[1]
	page.add_child(_ag_scroll)
	return page


func _build_waiting_page() -> Control:
	var page: VBoxContainer = VBoxContainer.new()
	page.add_theme_constant_override("separation", 8)
	page.add_child(OpsTheme.label("Things only you can do, newest first", 11, OpsTheme.MUTED))
	var sl: Array = _scroll_page()
	_wt_scroll = sl[0]
	_wt_list = sl[1]
	page.add_child(_wt_scroll)
	return page


# --- refresh --------------------------------------------------------------------------


func _refresh_all() -> void:
	_sync_filter_widgets()
	_rebuild_pipelines()
	_rebuild_agents()
	_rebuild_waiting()
	_rebuild_messages()
	_update_header()
	_layout()


func _run_updaters() -> void:
	for fn: Variant in _pl_updaters + _ag_updaters + _wt_updaters + _ms_updaters:
		(fn as Callable).call()
	_update_header()


func _clear(container: Node) -> void:
	for c: Node in container.get_children():
		container.remove_child(c)
		c.queue_free()


func _restore_scroll(scroll: ScrollContainer, value: int) -> void:
	if value <= 0 or not is_inside_tree():
		return
	await get_tree().process_frame
	if is_instance_valid(scroll):
		scroll.scroll_vertical = value


func _sync_filter_widgets() -> void:
	_pl_project.clear()
	_pl_project.add_item("All projects")
	_pl_project.set_item_metadata(0, "")
	var sel: int = 0
	for p: Variant in _state.projects():
		var pd: Dictionary = p
		_pl_project.add_item(str(pd.get("name", pd.get("id", "?"))))
		_pl_project.set_item_metadata(_pl_project.item_count - 1, str(pd.get("id", "")))
		if str(pd.get("id", "")) == _filter_project:
			sel = _pl_project.item_count - 1
	_pl_project.select(sel)
	for i: int in _pl_status.item_count:
		if str(_pl_status.get_item_metadata(i)) == _filter_status:
			_pl_status.select(i)


# --- shared builders ------------------------------------------------------------------


func _margin(child: Control, l: int, t: int, r: int, b: int) -> MarginContainer:
	var m: MarginContainer = MarginContainer.new()
	m.add_theme_constant_override("margin_left", l)
	m.add_theme_constant_override("margin_top", t)
	m.add_theme_constant_override("margin_right", r)
	m.add_theme_constant_override("margin_bottom", b)
	m.add_child(child)
	return m


func _expand(c: Control) -> Control:
	c.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	return c


func _project_info(project_id: String) -> Dictionary:
	var p: Dictionary = _state.project_by_id(project_id)
	return {"name": str(p.get("name", project_id)), "color": TirithState.color_of(p, OpsTheme.GREY)}


func _project_chip(project_id: String) -> Control:
	var info: Dictionary = _project_info(project_id)
	var col: Color = info["color"]
	var chip: PanelContainer = PanelContainer.new()
	chip.add_theme_stylebox_override("panel",
		OpsTheme.box(Color(col.r, col.g, col.b, 0.16), Color(col.r, col.g, col.b, 0.7), 1, 9, Vector4(7, 1, 8, 1)))
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 5)
	var dot: Control = OpsIcon.new().setup("gem", col, 8)
	dot.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	h.add_child(dot)
	h.add_child(OpsTheme.label(str(info["name"]), 11, OpsTheme.TEXT))
	chip.add_child(h)
	chip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return chip


func _status_chip(status: String) -> Control:
	var col: Color = OpsTheme.status_color(status)
	var chip: PanelContainer = PanelContainer.new()
	chip.add_theme_stylebox_override("panel",
		OpsTheme.box(Color(col.r * 0.62, col.g * 0.62, col.b * 0.62, 1.0), col, 1, 12, Vector4(8, 3, 11, 3)))
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 5)
	h.add_child(OpsIcon.new().setup(_icon_kind(status), Color(1, 1, 1, 0.95), 13))
	var l: Label = OpsTheme.label(OpsTheme.status_label(status), 11, Color(1, 1, 1), "sans", true)
	l.name = "StatusText"
	h.add_child(l)
	chip.add_child(h)
	chip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return chip


func _timer_pill(label_name: String, color: Color, min_w: int) -> Control:
	var pill: PanelContainer = PanelContainer.new()
	pill.add_theme_stylebox_override("panel",
		OpsTheme.box(Color(1, 1, 1, 0.08), Color(1, 1, 1, 0.10), 1, 14, Vector4(12, 3, 12, 3)))
	var l: Label = OpsTheme.label("", 15, color, "mono", true)
	l.name = label_name
	l.custom_minimum_size = Vector2(min_w, 0)
	l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	pill.add_child(l)
	pill.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return pill


func _icon_kind(status: String) -> String:
	match status:
		"running", "working":
			return "running"
		"passed", "done":
			return "passed"
		"failed":
			return "failed"
		"stalled":
			return "stalled"
		"canceled", "cancelled":
			return "canceled"
		"skipped":
			return "skipped"
		_:
			return "pending"


func _row_style(hover: bool, selected: bool = false) -> StyleBoxFlat:
	var sb: StyleBoxFlat = OpsTheme.box(OpsTheme.CARD_HOVER if (hover or selected) else Color(0, 0, 0, 0),
		Color(0, 0, 0, 0), 0, 0)
	sb.border_color = OpsTheme.GOLD if selected else OpsTheme.GOLD_DIM
	sb.border_width_bottom = 1
	if selected:
		sb.border_width_left = 3
	return sb


func _style_row_button(b: Button, selected: bool = false) -> void:
	b.focus_mode = Control.FOCUS_NONE
	b.add_theme_stylebox_override("normal", _row_style(false, selected))
	b.add_theme_stylebox_override("hover", _row_style(true, selected))
	b.add_theme_stylebox_override("pressed", _row_style(true, selected))
	b.add_theme_stylebox_override("focus", StyleBoxEmpty.new())


func _empty_note(text: String) -> Control:
	var l: Label = OpsTheme.label(text, 13, OpsTheme.MUTED, "serif")
	l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return _margin(l, 12, 40, 12, 12)


func _elide(l: Label) -> Label:
	l.clip_text = true
	l.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	l.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	return l


# --- pipelines ------------------------------------------------------------------------


func _run_by_id(run_id: String) -> Dictionary:
	for p: Variant in _state.pipelines():
		if str((p as Dictionary).get("run", "")) == run_id:
			return p
	return {}


func _visible_runs() -> Array:
	var out: Array = []
	for p: Variant in _state.pipelines():
		var run: Dictionary = p
		if _filter_project != "" and str(run.get("project", "")) != _filter_project:
			continue
		if _filter_status != "" and str(run.get("status", "")) != _filter_status:
			continue
		out.append(run)
	out.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return OpsTime.parse_iso(a.get("started_at", null)) > OpsTime.parse_iso(b.get("started_at", null)))
	return out


func _default_stage(run: Dictionary) -> Dictionary:
	var stages: Array = run.get("stages", []) as Array
	for want: String in ["running", "failed"]:
		for s: Variant in stages:
			if str((s as Dictionary).get("status", "")) == want:
				return s
	var last: Dictionary = {}
	for s: Variant in stages:
		var st: String = str((s as Dictionary).get("status", ""))
		if st != "pending" and st != "skipped":
			last = s
	if last.is_empty() and not stages.is_empty():
		last = stages[0]
	return last


func _log_text(stage: Dictionary) -> String:
	var lines: PackedStringArray = PackedStringArray()
	for l: Variant in stage.get("log_tail", []) as Array:
		lines.append(str(l))
	return "\n".join(lines)


func _stage_seconds(stage: Dictionary) -> int:
	var status: String = str(stage.get("status", "pending"))
	var started: int = OpsTime.parse_iso(stage.get("started_at", null))
	var ended: int = OpsTime.parse_iso(stage.get("ended_at", null))
	if status == "running":
		return OpsTime.elapsed_between(started, now_unix(), -1) if started >= 0 else 0
	if stage.get("duration_s", null) != null:
		return int(stage.get("duration_s", 0))
	if started >= 0 and ended >= 0:
		return ended - started
	return 0


func _stage_duration_text(stage: Dictionary) -> String:
	var status: String = str(stage.get("status", "pending"))
	if status == "pending" or status == "skipped":
		return "--"
	if status == "running":
		return OpsTime.format_duration_live(_stage_seconds(stage))
	if stage.get("duration_s", null) == null and OpsTime.parse_iso(stage.get("started_at", null)) < 0:
		return "--"
	return OpsTime.format_duration(_stage_seconds(stage))


func _run_seconds(run: Dictionary) -> int:
	var started: int = OpsTime.parse_iso(run.get("started_at", null))
	var ended: int = OpsTime.parse_iso(run.get("ended_at", null))
	if str(run.get("status", "")) != "running" and ended < 0:
		ended = started
	return OpsTime.elapsed_between(started, now_unix(), ended)


func _rebuild_pipelines() -> void:
	var keep: int = _pl_scroll.scroll_vertical
	_pl_updaters.clear()
	_run_ui.clear()
	_clear(_pl_list)
	var runs: Array = _visible_runs()
	_pl_count.text = "%d of %d runs" % [runs.size(), _state.pipelines().size()]
	if runs.is_empty():
		_pl_list.add_child(_empty_note("No pipeline runs match.\nStart one with `tirith run`."))
	for run: Dictionary in runs:
		_pl_list.add_child(_build_run_card(run))
	_restore_scroll(_pl_scroll, keep)


func _build_run_card(run: Dictionary) -> Control:
	var run_id: String = str(run.get("run", ""))
	var status: String = str(run.get("status", "running"))
	var stages: Array = run.get("stages", []) as Array
	var card: VBoxContainer = VBoxContainer.new()
	card.name = "RunCard"
	card.add_theme_constant_override("separation", 0)
	card.add_to_group("ops_pipeline_rows")
	card.set_meta("run_id", run_id)
	card.set_meta("status", status)
	card.set_meta("project", str(run.get("project", "")))
	var ui: Dictionary = {"pills": {}}
	_run_ui[run_id] = ui

	# Header row (click to expand).
	var head: OpsCardButton = OpsCardButton.new()
	head.name = "RunHeader"
	_style_row_button(head)
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 10)
	h.add_child(OpsIcon.new().setup(_icon_kind(status), OpsTheme.status_color(status), 26))
	var info: VBoxContainer = VBoxContainer.new()
	info.add_theme_constant_override("separation", 3)
	info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var l1: HBoxContainer = HBoxContainer.new()
	l1.add_theme_constant_override("separation", 6)
	var name_label: Label = OpsTheme.label(str(run.get("pipeline", run_id)), 15, OpsTheme.TEXT, "serif", true)
	name_label.name = "NameLabel"
	l1.add_child(name_label)
	var number: String = run_id.get_slice("#", 1) if run_id.contains("#") else run_id
	var run_label: Label = OpsTheme.label("#" + number, 12, OpsTheme.MUTED, "mono")
	run_label.name = "RunLabel"
	l1.add_child(run_label)
	info.add_child(l1)
	var l2: HBoxContainer = HBoxContainer.new()
	l2.add_theme_constant_override("separation", 8)
	l2.add_child(_project_chip(str(run.get("project", ""))))
	var started_label: Label = OpsTheme.label("", 11, OpsTheme.MUTED)
	started_label.name = "StartedLabel"
	l2.add_child(_elide(started_label))
	info.add_child(l2)
	h.add_child(info)
	var mini: HBoxContainer = HBoxContainer.new()
	mini.add_theme_constant_override("separation", 3)
	mini.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	for s: Variant in stages:
		var st: String = str((s as Dictionary).get("status", "pending"))
		mini.add_child(OpsIcon.new().setup(_icon_kind(st), OpsTheme.status_color(st), 11))
	h.add_child(mini)
	h.add_child(_status_chip(status))
	h.add_child(_timer_pill("DurationLabel", OpsTheme.TEXT if status == "running" else OpsTheme.MUTED, 92))
	var chevron: Control = OpsIcon.new().setup("chevron_right", OpsTheme.GOLD, 18)
	h.add_child(chevron)
	ui["chevron"] = chevron
	head.set_content(_margin(h, 10, 9, 8, 9))
	OpsTheme.ignore_mouse(head.get_child(0))
	card.add_child(head)

	# Body: stage graph, time bar, log tail.
	var body: VBoxContainer = VBoxContainer.new()
	body.name = "RunBody"
	body.add_theme_constant_override("separation", 8)
	ui["body"] = body
	var graph_scroll: ScrollContainer = ScrollContainer.new()
	graph_scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	graph_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	graph_scroll.custom_minimum_size = Vector2(0, 62)
	var graph: HBoxContainer = HBoxContainer.new()
	graph.add_theme_constant_override("separation", 0)
	graph_scroll.add_child(graph)
	var prev_status: String = ""
	var running_pill: Control = null
	for s: Variant in stages:
		var stage: Dictionary = s
		if prev_status != "":
			var lc: Color = OpsTheme.GOLD if prev_status in ["passed", "running", "failed"] else OpsTheme.DIM
			var conn: Control = OpsIcon.new().setup("connector", lc, 18)
			conn.custom_minimum_size = Vector2(18, 0)
			graph.add_child(conn)
		var pill: Control = _build_pill(run_id, stage)
		graph.add_child(pill)
		(ui["pills"] as Dictionary)[str(stage.get("name", ""))] = pill
		if str(stage.get("status", "")) == "running":
			running_pill = pill
		prev_status = str(stage.get("status", "pending"))
	body.add_child(graph_scroll)

	var bar_title: HBoxContainer = HBoxContainer.new()
	bar_title.add_child(OpsTheme.label("TIME PER STAGE", 10, OpsTheme.MUTED, "sans", true))
	body.add_child(bar_title)
	var bar: OpsDurationBar = OpsDurationBar.new()
	bar.name = "DurationBar"
	bar.add_to_group("ops_duration_bars")
	bar.set_meta("run_id", run_id)
	body.add_child(bar)
	var legend: Label = OpsTheme.label("", 11, OpsTheme.MUTED, "mono")
	legend.name = "Legend"
	legend.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(legend)

	var log_panel: PanelContainer = PanelContainer.new()
	log_panel.add_theme_stylebox_override("panel",
		OpsTheme.box(Color(0.028, 0.026, 0.024), OpsTheme.GOLD_DIM, 1, 6, Vector4(10, 7, 10, 8)))
	var lv: VBoxContainer = VBoxContainer.new()
	lv.add_theme_constant_override("separation", 4)
	var log_title: Label = OpsTheme.label("", 11, OpsTheme.GOLD, "mono", true)
	log_title.name = "LogTitle"
	lv.add_child(log_title)
	var log_label: Label = OpsTheme.label("", 11, Color(0.80, 0.84, 0.78), "mono")
	log_label.name = "LogLabel"
	log_label.autowrap_mode = TextServer.AUTOWRAP_ARBITRARY
	lv.add_child(log_label)
	log_panel.add_child(lv)
	body.add_child(log_panel)
	ui["log_title"] = log_title
	ui["log_label"] = log_label
	card.add_child(_margin(body, 16, 2, 10, 14))

	var is_expanded: bool = bool(_expanded.get(run_id, status == "running"))
	body.visible = is_expanded
	(chevron as Control).set("kind", "chevron_down" if is_expanded else "chevron_right")
	head.pressed.connect(_toggle_run.bind(run_id))
	_apply_stage_selection(run_id)

	# Live updaters: run duration, "started" text, running stage durations, the bar.
	var update_run: Callable = func() -> void:
		var now: int = now_unix()
		var started: int = OpsTime.parse_iso(run.get("started_at", null))
		var secs: int = _run_seconds(run)
		var dur_label: Label = head.find_child("DurationLabel", true, false) as Label
		if dur_label != null:
			var live: bool = status == "running" and OpsTime.parse_iso(run.get("ended_at", null)) < 0
			dur_label.text = OpsTime.format_duration_live(secs) if live else OpsTime.format_duration(secs)
		if started_label != null:
			started_label.text = "%s  |  %s" % [OpsTime.format_local_time(started, now),
				OpsTime.format_ago(now - started)] if started >= 0 else "not started"
		var segs: Array = []
		var texts: PackedStringArray = PackedStringArray()
		for s2: Variant in stages:
			var sd: Dictionary = s2
			var sec: int = _stage_seconds(sd)
			segs.append({"name": str(sd.get("name", "")), "seconds": sec, "status": str(sd.get("status", "pending"))})
			if sec > 0:
				texts.append("%s %s" % [sd.get("name", ""), OpsTime.format_duration(sec)])
			var pill_node: Control = (ui["pills"] as Dictionary).get(str(sd.get("name", "")), null) as Control
			if pill_node != null:
				var dl: Label = pill_node.find_child("StageDuration", true, false) as Label
				if dl != null:
					dl.text = _stage_duration_text(sd)
		bar.segments = segs
		bar.tooltip_text = "  ".join(texts)
		legend.text = "  |  ".join(texts) if not texts.is_empty() else "no stage has finished yet"
	_pl_updaters.append(update_run)
	update_run.call()
	if running_pill != null and is_expanded:
		_reveal.call_deferred(graph_scroll, running_pill)
	card.add_child(_hairline())
	return card


func _hairline() -> Control:
	var l: ColorRect = ColorRect.new()
	l.color = OpsTheme.GOLD_DIM
	l.custom_minimum_size = Vector2(0, 1)
	l.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return l


func _reveal(scroll: ScrollContainer, node: Control) -> void:
	if is_instance_valid(scroll) and is_instance_valid(node) and scroll.is_inside_tree():
		await get_tree().process_frame
		if is_instance_valid(scroll) and is_instance_valid(node):
			scroll.ensure_control_visible(node)


func _toggle_run(run_id: String) -> void:
	var ui: Dictionary = _run_ui.get(run_id, {})
	if ui.is_empty():
		return
	var body: Control = ui["body"]
	body.visible = not body.visible
	_expanded[run_id] = body.visible
	(ui["chevron"] as Control).set("kind", "chevron_down" if body.visible else "chevron_right")
	(ui["chevron"] as Control).queue_redraw()


func _build_pill(run_id: String, stage: Dictionary) -> Control:
	var status: String = str(stage.get("status", "pending"))
	var dark: bool = status in ["pending", "skipped"]
	var pill: OpsCardButton = OpsCardButton.new()
	pill.name = "StagePill"
	pill.focus_mode = Control.FOCUS_NONE
	pill.custom_minimum_size = Vector2(108, 48)
	pill.add_to_group("ops_stage_pills")
	pill.set_meta("run_id", run_id)
	pill.set_meta("stage_name", str(stage.get("name", "")))
	pill.set_meta("status", status)
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 8)
	h.add_child(OpsIcon.new().setup(_icon_kind(status), _pill_icon_color(status), 20))
	var v: VBoxContainer = VBoxContainer.new()
	v.add_theme_constant_override("separation", 0)
	v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var ink: Color = OpsTheme.DIM if dark else OpsTheme.INK
	var nl: Label = OpsTheme.label(str(stage.get("name", "")), 13, ink if dark else OpsTheme.INK, "sans", true)
	nl.name = "StageName"
	v.add_child(_elide(nl))
	var dl: Label = OpsTheme.label(_stage_duration_text(stage), 12, OpsTheme.DIM if dark else Color(0.33, 0.26, 0.14), "mono", true)
	dl.name = "StageDuration"
	v.add_child(dl)
	h.add_child(v)
	pill.set_content(_margin(h, 11, 5, 13, 5))
	OpsTheme.ignore_mouse(pill.get_child(0))
	pill.pressed.connect(select_stage.bind(run_id, str(stage.get("name", ""))))
	pill.tooltip_text = "%s: %s" % [stage.get("name", ""), status]
	return pill


func _pill_icon_color(status: String) -> Color:
	match status:
		"passed":
			return Color(0.13, 0.55, 0.24)
		"running":
			return Color(0.10, 0.42, 0.90)
		"failed":
			return Color(0.80, 0.18, 0.16)
		_:
			return OpsTheme.status_color(status)


func _style_pill(pill: Button, status: String, selected: bool) -> void:
	var fill: Color
	var border: Color
	var bw: int = 1
	match status:
		"passed":
			fill = OpsTheme.PARCHMENT
			border = Color(0.55, 0.45, 0.22)
		"running":
			fill = Color(0.80, 0.88, 1.0)
			border = OpsTheme.BLUE
			bw = 2
		"failed":
			fill = Color(1.0, 0.82, 0.78)
			border = OpsTheme.RED
			bw = 2
		"canceled":
			fill = Color(0.75, 0.75, 0.77)
			border = OpsTheme.GREY
		_:
			fill = Color(0.11, 0.10, 0.09)
			border = Color(0.30, 0.28, 0.25)
	if selected:
		border = Color(1.0, 0.82, 0.30)
		bw = 3
	var sb: StyleBoxFlat = OpsTheme.box(fill, border, bw, 24)
	var hv: StyleBoxFlat = OpsTheme.box(fill.lightened(0.12), Color(1.0, 0.82, 0.30), maxi(bw, 2), 24)
	pill.add_theme_stylebox_override("normal", sb)
	pill.add_theme_stylebox_override("hover", hv)
	pill.add_theme_stylebox_override("pressed", hv)
	pill.add_theme_stylebox_override("focus", StyleBoxEmpty.new())


func _apply_stage_selection(run_id: String) -> void:
	var ui: Dictionary = _run_ui.get(run_id, {})
	if ui.is_empty():
		return
	var sel: String = selected_stage_name(run_id)
	var picked: Dictionary = {}
	for s: Variant in _run_by_id(run_id).get("stages", []) as Array:
		var sd: Dictionary = s
		var pill: Button = (ui["pills"] as Dictionary).get(str(sd.get("name", "")), null) as Button
		if pill != null:
			_style_pill(pill, str(sd.get("status", "pending")), str(sd.get("name", "")) == sel)
		if str(sd.get("name", "")) == sel:
			picked = sd
	if picked.is_empty():
		return
	(ui["log_title"] as Label).text = "$ %s  |  %s" % [sel, str(picked.get("status", ""))]
	var text: String = _log_text(picked)
	(ui["log_label"] as Label).text = text if text != "" else "(no output yet)"


# --- agents ---------------------------------------------------------------------------


func _agent_seconds(a: Dictionary) -> int:
	var started: int = OpsTime.parse_iso(a.get("started_at", null))
	if started < 0:
		return int(a.get("elapsed_s", 0))
	return OpsTime.elapsed_between(started, now_unix(), OpsTime.parse_iso(a.get("ended_at", null)))


func _agent_is_live(a: Dictionary) -> bool:
	return OpsTime.parse_iso(a.get("ended_at", null)) < 0 and str(a.get("status", "")) in ["working", "stalled"]


func _sorted_agents() -> Array:
	var out: Array = []
	for a: Variant in _state.agents():
		out.append(a)
	var order: Dictionary = {"working": 0, "stalled": 1, "failed": 2, "done": 3}
	var by_elapsed: Callable = func(a: Dictionary, b: Dictionary) -> bool:
		return _agent_seconds(a) > _agent_seconds(b)
	match _agent_sort:
		"status":
			out.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
				var sa: int = int(order.get(str(a.get("status", "")), 9))
				var sb: int = int(order.get(str(b.get("status", "")), 9))
				return by_elapsed.call(a, b) if sa == sb else sa < sb)
		"project":
			out.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
				var pa: String = str(_project_info(str(a.get("project", "")))["name"]).to_lower()
				var pb: String = str(_project_info(str(b.get("project", "")))["name"]).to_lower()
				return by_elapsed.call(a, b) if pa == pb else pa < pb)
		_:
			out.sort_custom(by_elapsed)
	return out


func _rebuild_agents() -> void:
	var keep: int = _ag_scroll.scroll_vertical
	_ag_updaters.clear()
	_agent_ui.clear()
	_clear(_ag_list)
	var agents: Array = _sorted_agents()
	if agents.is_empty():
		_ag_list.add_child(_empty_note("No agents yet.\nRegister one with `tirith lane start`."))
	for a: Dictionary in agents:
		_ag_list.add_child(_build_agent_row(a))
	_restore_scroll(_ag_scroll, keep)


func _build_agent_row(a: Dictionary) -> Control:
	var agent_id: String = str(a.get("id", ""))
	var status: String = str(a.get("status", "working"))
	var class_id: String = str(a.get("class", ""))
	var class_col: Color = TirithState.class_color(class_id)
	var row: OpsCardButton = OpsCardButton.new()
	row.name = "AgentRow"
	_style_row_button(row, agent_id == selected_agent_id)
	row.add_to_group("ops_agent_rows")
	row.set_meta("agent_id", agent_id)
	row.set_meta("status", status)
	row.set_meta("project", str(a.get("project", "")))
	var col: VBoxContainer = VBoxContainer.new()
	col.add_theme_constant_override("separation", 5)

	var top: HBoxContainer = HBoxContainer.new()
	top.add_theme_constant_override("separation", 12)
	var letter: String = TirithState.class_label(class_id).substr(0, 1).to_upper()
	var badge: Control = OpsIcon.new().setup("class", class_col, 44, letter)
	badge.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	top.add_child(badge)
	var names: VBoxContainer = VBoxContainer.new()
	names.add_theme_constant_override("separation", 2)
	names.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var full: String = str(a.get("name", "?"))
	if str(a.get("epithet", "")) != "":
		full += " " + str(a.get("epithet", ""))
	var name_label: Label = OpsTheme.label(full, 16, OpsTheme.TEXT, "serif", true)
	name_label.name = "NameLabel"
	names.add_child(_elide(name_label))
	var sub: Label = OpsTheme.label("%s %s  |  %s" % [TirithState.rank_label(str(a.get("rank", ""))),
		TirithState.class_label(class_id), str(a.get("feature", ""))], 12, OpsTheme.MUTED)
	sub.name = "SubLabel"
	names.add_child(_elide(sub))
	top.add_child(names)
	top.add_child(_status_chip(status))
	var el_color: Color = OpsTheme.status_color(status) if status in ["failed", "stalled"] else OpsTheme.TEXT
	top.add_child(_timer_pill("ElapsedLabel", el_color if status != "done" else OpsTheme.MUTED, 104))
	col.add_child(top)

	var meta: HBoxContainer = HBoxContainer.new()
	meta.add_theme_constant_override("separation", 8)
	meta.add_child(_project_chip(str(a.get("project", ""))))
	var started_label: Label = OpsTheme.label("", 11, OpsTheme.MUTED)
	started_label.name = "StartedLabel"
	meta.add_child(_elide(started_label))
	var files: RichTextLabel = RichTextLabel.new()
	files.name = "FilesLabel"
	files.bbcode_enabled = true
	files.fit_content = true
	files.scroll_active = false
	files.autowrap_mode = TextServer.AUTOWRAP_OFF
	files.mouse_filter = Control.MOUSE_FILTER_IGNORE
	files.custom_minimum_size = Vector2(150, 0)
	files.add_theme_font_override("normal_font", OpsTheme.font("mono"))
	files.add_theme_font_override("bold_font", OpsTheme.font("mono", true))
	files.add_theme_font_size_override("normal_font_size", 11)
	files.add_theme_color_override("default_color", OpsTheme.MUTED)
	files.text = "%d files  [color=#52c766]+%d[/color] [color=#ec5750]-%d[/color]" % [
		int(a.get("files_changed", 0)), int(a.get("insertions", 0)), int(a.get("deletions", 0))]
	meta.add_child(files)
	col.add_child(_margin(meta, 56, 0, 0, 0))

	var summary: Label = OpsTheme.label(str(a.get("summary", "")), 12, Color(0.78, 0.74, 0.64), "sans")
	summary.name = "SummaryLabel"
	summary.set_meta("keep_mouse", true)
	summary.mouse_filter = Control.MOUSE_FILTER_PASS
	summary.tooltip_text = str(a.get("summary", ""))
	col.add_child(_margin(_elide(summary), 56, 0, 0, 0))

	var detail: Label = OpsTheme.label(_agent_detail_text(a), 11, Color(0.80, 0.84, 0.78), "mono")
	detail.name = "DetailLabel"
	detail.autowrap_mode = TextServer.AUTOWRAP_ARBITRARY
	var detail_box: PanelContainer = PanelContainer.new()
	detail_box.name = "DetailBox"
	detail_box.add_theme_stylebox_override("panel",
		OpsTheme.box(Color(0.028, 0.026, 0.024), OpsTheme.GOLD_DIM, 1, 6, Vector4(10, 7, 10, 8)))
	detail_box.add_child(detail)
	detail_box.visible = agent_id == selected_agent_id
	col.add_child(_margin(detail_box, 56, 2, 0, 2))

	row.set_content(_margin(col, 12, 11, 10, 11))
	OpsTheme.ignore_mouse(row.get_child(0))
	row.pressed.connect(_on_agent_pressed.bind(agent_id))
	_agent_ui[agent_id] = {"row": row, "detail": detail_box}

	var update_row: Callable = func() -> void:
		var now: int = now_unix()
		var started: int = OpsTime.parse_iso(a.get("started_at", null))
		var secs: int = _agent_seconds(a)
		var el: Label = row.find_child("ElapsedLabel", true, false) as Label
		if el != null:
			el.text = OpsTime.format_duration_live(secs) if _agent_is_live(a) else OpsTime.format_duration(secs)
		started_label.text = ("started %s  |  %s" % [OpsTime.format_local_time(started, now),
			OpsTime.format_ago(now - started)]) if started >= 0 else "not started"
	_ag_updaters.append(update_row)
	update_row.call()
	return row


func _agent_detail_text(a: Dictionary) -> String:
	var lines: PackedStringArray = PackedStringArray()
	lines.append(str(a.get("summary", "")))
	var tail: Array = a.get("log_tail", []) as Array
	if not tail.is_empty():
		lines.append("")
		lines.append("log tail")
		for l: Variant in tail:
			lines.append("  " + str(l))
	if a.get("report", null) != null:
		lines.append("")
		lines.append("report")
		lines.append("  " + str(a.get("report", "")))
	return "\n".join(lines)


func _on_agent_pressed(agent_id: String) -> void:
	selected_agent_id = agent_id
	for id: String in _agent_ui:
		var ui: Dictionary = _agent_ui[id]
		(ui["detail"] as Control).visible = id == agent_id
		_style_row_button(ui["row"] as Button, id == agent_id)
	focus_agent.emit(agent_id)


# --- waiting --------------------------------------------------------------------------


func _sorted_waiting() -> Array:
	var out: Array = []
	for w: Variant in _state.waiting():
		out.append(w)
	out.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return OpsTime.parse_iso(a.get("since", null)) > OpsTime.parse_iso(b.get("since", null)))
	return out


func _rebuild_waiting() -> void:
	var keep: int = _wt_scroll.scroll_vertical
	_wt_updaters.clear()
	_clear(_wt_list)
	var items: Array = _sorted_waiting()
	if items.is_empty():
		_wt_list.add_child(_empty_note("Nothing is waiting for you.\nThe citadel runs itself."))
	for w: Dictionary in items:
		_wt_list.add_child(_build_waiting_row(w))
	_restore_scroll(_wt_scroll, keep)


func _build_waiting_row(w: Dictionary) -> Control:
	var panel: PanelContainer = PanelContainer.new()
	panel.name = "WaitingRow"
	panel.add_to_group("ops_waiting_rows")
	panel.set_meta("item_id", str(w.get("id", "")))
	panel.set_meta("project", str(w.get("project", "")))
	var sb: StyleBoxFlat = _row_style(false)
	sb.content_margin_left = 12
	sb.content_margin_right = 10
	sb.content_margin_top = 11
	sb.content_margin_bottom = 11
	panel.add_theme_stylebox_override("panel", sb)
	var h: HBoxContainer = HBoxContainer.new()
	h.add_theme_constant_override("separation", 12)
	var gem: Control = OpsIcon.new().setup("gem", OpsTheme.GOLD, 16)
	gem.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	h.add_child(gem)
	var v: VBoxContainer = VBoxContainer.new()
	v.add_theme_constant_override("separation", 5)
	v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var text: Label = OpsTheme.label(str(w.get("text", "")), 14, OpsTheme.TEXT, "serif")
	text.name = "TextLabel"
	text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	v.add_child(text)
	var meta: HBoxContainer = HBoxContainer.new()
	meta.add_theme_constant_override("separation", 8)
	meta.add_child(_project_chip(str(w.get("project", ""))))
	var age: Label = OpsTheme.label("", 11, OpsTheme.MUTED)
	age.name = "AgeLabel"
	meta.add_child(age)
	if w.get("link", null) != null:
		meta.add_child(OpsTheme.label(str(w.get("link", "")), 11, OpsTheme.BLUE, "mono"))
	v.add_child(meta)
	h.add_child(v)
	panel.add_child(h)
	var update_age: Callable = func() -> void:
		var since: int = OpsTime.parse_iso(w.get("since", null))
		age.text = OpsTime.format_ago(now_unix() - since) if since >= 0 else ""
	_wt_updaters.append(update_age)
	update_age.call()
	return panel


# --- messages (information flows) -----------------------------------------------------


func _flows() -> Array:
	return _state.raw.get("flows", []) as Array


func set_message_filter(endpoint: String, kind: String) -> void:
	_flow_filter_endpoint = endpoint
	_flow_filter_kind = kind
	_sync_flow_filters()
	_rebuild_messages()


## Flow ids shown now, newest first.
func message_ids() -> Array:
	var ids: Array = []
	for f: Dictionary in _visible_flows():
		ids.append(str(f.get("id", "")))
	return ids


func _visible_flows() -> Array:
	var out: Array = []
	for f: Variant in _flows():
		var fd: Dictionary = f
		if _flow_filter_kind != "" and str(fd.get("kind", "")) != _flow_filter_kind:
			continue
		if _flow_filter_endpoint != "" and str(fd.get("from", "")) != _flow_filter_endpoint \
				and str(fd.get("to", "")) != _flow_filter_endpoint:
			continue
		out.append(fd)
	out.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return OpsTime.parse_iso(a.get("at", null)) > OpsTime.parse_iso(b.get("at", null)))
	return out


func _build_messages_page() -> Control:
	var page: VBoxContainer = VBoxContainer.new()
	page.add_theme_constant_override("separation", 8)
	var bar: HBoxContainer = HBoxContainer.new()
	bar.add_theme_constant_override("separation", 6)
	page.add_child(bar)
	_ms_endpoint = OptionButton.new()
	_ms_endpoint.name = "EndpointFilter"
	_ms_endpoint.focus_mode = Control.FOCUS_NONE
	_ms_endpoint.item_selected.connect(func(idx: int) -> void:
		set_message_filter(str(_ms_endpoint.get_item_metadata(idx)), _flow_filter_kind))
	bar.add_child(_ms_endpoint)
	_ms_kind = OptionButton.new()
	_ms_kind.name = "KindFilter"
	_ms_kind.focus_mode = Control.FOCUS_NONE
	for k: String in [""] + FLOW_KIND_COLORS.keys():
		_ms_kind.add_item("All kinds" if k == "" else k.capitalize())
		_ms_kind.set_item_metadata(_ms_kind.item_count - 1, k)
	_ms_kind.item_selected.connect(func(idx: int) -> void:
		set_message_filter(_flow_filter_endpoint, str(_ms_kind.get_item_metadata(idx))))
	bar.add_child(_ms_kind)
	var sp: Control = Control.new()
	sp.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	bar.add_child(sp)
	_ms_count = OpsTheme.label("", 11, OpsTheme.MUTED)
	bar.add_child(_ms_count)
	var sl: Array = _scroll_page()
	_ms_scroll = sl[0]
	_ms_list = sl[1]
	page.add_child(_ms_scroll)
	return page


func _sync_flow_filters() -> void:
	_ms_endpoint.clear()
	_ms_endpoint.add_item("Everyone")
	_ms_endpoint.set_item_metadata(0, "")
	var sel: int = 0
	var entries: Array = [["steward", "The Steward"], ["owner", "You"]]
	for a: Variant in _state.agents():
		var ad: Dictionary = a
		entries.append(["agent:" + str(ad.get("id", "")), str(ad.get("name", ad.get("id", "?")))])
	for e: Array in entries:
		_ms_endpoint.add_item(str(e[1]))
		_ms_endpoint.set_item_metadata(_ms_endpoint.item_count - 1, str(e[0]))
		if str(e[0]) == _flow_filter_endpoint:
			sel = _ms_endpoint.item_count - 1
	_ms_endpoint.select(sel)
	for i: int in _ms_kind.item_count:
		if str(_ms_kind.get_item_metadata(i)) == _flow_filter_kind:
			_ms_kind.select(i)


func _rebuild_messages() -> void:
	var keep: int = _ms_scroll.scroll_vertical
	_ms_updaters.clear()
	_sync_flow_filters()
	_clear(_ms_list)
	var flows: Array = _visible_flows()
	_ms_count.text = "%d of %d messages" % [flows.size(), _flows().size()]
	if flows.is_empty():
		_ms_list.add_child(_empty_note("No messages yet.\nBriefs, reports and requests appear here as they happen."))
	for f: Dictionary in flows:
		_ms_list.add_child(_build_flow_row(f))
	_restore_scroll(_ms_scroll, keep)


func _agent_class_of(agent_id: String) -> String:
	for a: Variant in _state.agents():
		if str((a as Dictionary).get("id", "")) == agent_id:
			return str((a as Dictionary).get("class", ""))
	return ""


func _endpoint_icon(endpoint: String) -> Control:
	var kind: String = endpoint.get_slice(":", 0)
	match kind:
		"steward":
			return OpsIcon.new().setup("mark", OpsTheme.GOLD, 18)
		"owner":
			return OpsIcon.new().setup("owner", OpsTheme.TEXT, 18)
		"guard":
			return OpsIcon.new().setup("guard", OpsTheme.AMBER, 18)
		"room":
			return OpsIcon.new().setup("room", OpsTheme.GREEN, 18)
		"pipeline":
			return OpsIcon.new().setup("pipeline", OpsTheme.BLUE, 18)
		"agent":
			var cid: String = _agent_class_of(endpoint.get_slice(":", 1))
			return OpsIcon.new().setup("class", TirithState.class_color(cid), 20,
				TirithState.class_label(cid).substr(0, 1).to_upper())
	return OpsIcon.new().setup("pending", OpsTheme.GREY, 18)


func _build_flow_row(f: Dictionary) -> Control:
	var flow_id: String = str(f.get("id", ""))
	var kind: String = str(f.get("kind", "message"))
	var kcol: Color = FLOW_KIND_COLORS.get(kind, OpsTheme.GREY)
	var from_ep: String = str(f.get("from", ""))
	var to_ep: String = str(f.get("to", ""))
	var row: OpsCardButton = OpsCardButton.new()
	row.name = "FlowRow"
	_style_row_button(row)
	row.add_to_group("ops_flow_rows")
	row.set_meta("flow_id", flow_id)
	row.set_meta("kind", kind)
	row.set_meta("from", from_ep)
	row.set_meta("to", to_ep)
	var col: VBoxContainer = VBoxContainer.new()
	col.add_theme_constant_override("separation", 4)
	var top: HBoxContainer = HBoxContainer.new()
	top.add_theme_constant_override("separation", 7)
	var time_label: Label = OpsTheme.label("", 12, OpsTheme.MUTED, "mono")
	time_label.name = "TimeLabel"
	time_label.custom_minimum_size = Vector2(46, 0)
	top.add_child(time_label)
	top.add_child(_endpoint_icon(from_ep))
	var from_label: Label = OpsTheme.label(str(f.get("from_name", from_ep)), 14, OpsTheme.TEXT, "serif", true)
	from_label.name = "FromLabel"
	top.add_child(from_label)
	top.add_child(OpsIcon.new().setup("arrow", OpsTheme.GOLD, 16))
	top.add_child(_endpoint_icon(to_ep))
	var to_label: Label = OpsTheme.label(str(f.get("to_name", to_ep)), 14, OpsTheme.TEXT, "serif", true)
	to_label.name = "ToLabel"
	top.add_child(_elide(to_label))
	var chip: PanelContainer = PanelContainer.new()
	chip.name = "KindChip"
	chip.add_theme_stylebox_override("panel",
		OpsTheme.box(Color(kcol.r * 0.5, kcol.g * 0.5, kcol.b * 0.5, 1.0), kcol, 1, 10, Vector4(8, 1, 8, 1)))
	chip.add_child(OpsTheme.label(kind.to_upper(), 10, Color(1, 1, 1), "sans", true))
	chip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	top.add_child(chip)
	if f.get("ref", null) != null:
		var link: Control = OpsIcon.new().setup("link", OpsTheme.BLUE, 16)
		link.name = "RefIcon"
		top.add_child(link)
	col.add_child(top)
	var text: Label = OpsTheme.label(str(f.get("text", "")), 13, Color(0.82, 0.78, 0.68))
	text.name = "TextLabel"
	text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	col.add_child(_margin(text, 53, 0, 0, 0))
	row.set_content(_margin(col, 10, 9, 10, 9))
	OpsTheme.ignore_mouse(row.get_child(0))
	row.tooltip_text = ("ref: %s" % str(f.get("ref", ""))) if f.get("ref", null) != null else ""
	row.pressed.connect(_on_flow_pressed.bind(from_ep, to_ep))
	var update_time: Callable = func() -> void:
		var at: int = OpsTime.parse_iso(f.get("at", null))
		time_label.text = OpsTime.format_local_time(at, now_unix()).right(5)
		time_label.tooltip_text = OpsTime.format_ago(now_unix() - at) if at >= 0 else ""
	_ms_updaters.append(update_time)
	update_time.call()
	return row


## Clicking a message focuses the agent at either end (the receiver first).
func _on_flow_pressed(from_ep: String, to_ep: String) -> void:
	for ep: String in [to_ep, from_ep]:
		if ep.begins_with("agent:"):
			focus_agent.emit(ep.get_slice(":", 1))
			return
