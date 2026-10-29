class_name CitadelUI
extends CanvasLayer
## HUD (counts, brain status, help) and the parchment detail panel.
## Pure presentation: it formats Dictionaries from TirithState, never reads files.

var _hud_text: RichTextLabel
var _notice: Label
var _panel: PanelContainer
var _title: Label
var _subtitle: Label
var _body: RichTextLabel


func _ready() -> void:
	_build_hud()
	_build_panel()


func _parchment_style() -> StyleBoxFlat:
	var s: StyleBoxFlat = StyleBoxFlat.new()
	s.bg_color = Color(0.96, 0.91, 0.78, 0.97)
	s.border_color = Color(0.45, 0.30, 0.14)
	s.set_border_width_all(3)
	s.set_corner_radius_all(10)
	s.content_margin_left = 18
	s.content_margin_right = 18
	s.content_margin_top = 14
	s.content_margin_bottom = 14
	return s


func _hud_style() -> StyleBoxFlat:
	var s: StyleBoxFlat = StyleBoxFlat.new()
	s.bg_color = Color(0.07, 0.09, 0.14, 0.78)
	s.border_color = Color(0.85, 0.68, 0.25, 0.9)
	s.set_border_width_all(2)
	s.set_corner_radius_all(8)
	s.content_margin_left = 14
	s.content_margin_right = 14
	s.content_margin_top = 10
	s.content_margin_bottom = 10
	return s


func _build_hud() -> void:
	var hud: PanelContainer = PanelContainer.new()
	hud.name = "HUD"
	hud.add_theme_stylebox_override("panel", _hud_style())
	hud.set_anchors_preset(Control.PRESET_TOP_LEFT)
	hud.position = Vector2(14, 12)
	add_child(hud)
	var vbox: VBoxContainer = VBoxContainer.new()
	hud.add_child(vbox)
	var title: Label = Label.new()
	title.text = "T I R I T H"
	title.add_theme_font_size_override("font_size", 24)
	title.add_theme_color_override("font_color", Color(1.0, 0.85, 0.4))
	vbox.add_child(title)
	_hud_text = RichTextLabel.new()
	_hud_text.bbcode_enabled = true
	_hud_text.fit_content = true
	_hud_text.custom_minimum_size = Vector2(270, 0)
	_hud_text.add_theme_font_size_override("normal_font_size", 15)
	_hud_text.add_theme_color_override("default_color", Color(0.92, 0.93, 0.95))
	vbox.add_child(_hud_text)
	_notice = Label.new()
	_notice.add_theme_font_size_override("font_size", 12)
	_notice.add_theme_color_override("font_color", Color(1.0, 0.75, 0.55))
	_notice.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_notice.custom_minimum_size = Vector2(270, 0)
	vbox.add_child(_notice)
	# Controls help, bottom-left.
	_help = Label.new()
	var help: Label = _help
	help.text = HELP_2D5
	help.add_theme_font_size_override("font_size", 13)
	help.add_theme_color_override("font_color", Color(1, 1, 1, 0.85))
	help.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.8))
	help.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	help.position = Vector2(14, -30)
	add_child(help)


const HELP_2D5: String = "2.5D  -  Drag or two-finger scroll: pan  -  Pinch, Cmd+scroll, wheel, +/-: zoom  -  Q/E: rotate  -  WASD: move  -  V: free 3D  -  T: tour  -  Click: inspect  -  Dbl-click: centre"
const HELP_3D: String = "3D  -  Drag: orbit  -  Right-drag or two-finger scroll: pan  -  Pinch, wheel, +/-: zoom  -  Q/E: turn  -  V: back to 2.5D  -  T: tour  -  Click: inspect"
var _help: Label
var _tour_text: String = ""
var _is_2d5: bool = true


func set_view_label(is_2d5: bool) -> void:
	_is_2d5 = is_2d5
	_refresh_help()


func set_tour_label(on: bool) -> void:
	_tour_text = "  -  DEMO TOUR (T to stop)" if on else ""
	_refresh_help()


func _refresh_help() -> void:
	if _help != null:
		_help.text = (HELP_2D5 if _is_2d5 else HELP_3D) + _tour_text


func _build_panel() -> void:
	_panel = PanelContainer.new()
	_panel.name = "DetailPanel"
	_panel.add_theme_stylebox_override("panel", _parchment_style())
	_panel.set_anchors_preset(Control.PRESET_CENTER_RIGHT)
	_panel.custom_minimum_size = Vector2(470, 0)
	_panel.position = Vector2(-486, -330)
	_panel.size = Vector2(470, 660)
	_panel.visible = false
	add_child(_panel)
	var vbox: VBoxContainer = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 8)
	_panel.add_child(vbox)
	_title = Label.new()
	_title.add_theme_font_size_override("font_size", 23)
	_title.add_theme_color_override("font_color", Color(0.25, 0.15, 0.05))
	_title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	vbox.add_child(_title)
	_subtitle = Label.new()
	_subtitle.add_theme_font_size_override("font_size", 15)
	_subtitle.add_theme_color_override("font_color", Color(0.45, 0.30, 0.14))
	_subtitle.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	vbox.add_child(_subtitle)
	vbox.add_child(HSeparator.new())
	_body = RichTextLabel.new()
	_body.bbcode_enabled = true
	_body.fit_content = false
	_body.scroll_active = true
	_body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_body.add_theme_font_size_override("normal_font_size", 15)
	_body.add_theme_font_size_override("bold_font_size", 15)
	_body.add_theme_font_size_override("mono_font_size", 13)
	_body.add_theme_color_override("default_color", Color(0.16, 0.11, 0.06))
	vbox.add_child(_body)
	var close: Button = Button.new()
	close.text = "Close  (Esc)"
	close.pressed.connect(hide_panel)
	vbox.add_child(close)


func hide_panel() -> void:
	_panel.visible = false


func is_panel_open() -> bool:
	return _panel.visible


static func esc(text: String) -> String:
	return text.replace("[", "[lb]")


func set_hud(state: TirithState, state_path: String) -> void:
	var c: Dictionary = state.counts()
	var brain: Dictionary = state.brain()
	var brain_line: String = "Brain: off"
	if bool(brain.get("enabled", false)):
		brain_line = "Brain: %s" % str(brain.get("model", "muse-spark"))
		if brain.get("last_error") != null:
			brain_line += " (error)"
	var waiting_color: String = "#ff9a4d" if int(c["waiting"]) > 0 else "#b8bec9"
	_hud_text.text = ("[color=#7ddb8a]Working %d[/color]   [color=#8fc2ff]Done %d[/color]   " +
		"[color=#ff7a6b]Failed %d[/color]   [color=#c9c9c9]Stalled %d[/color]\n" +
		"[color=#ffc857]Pipelines running %d[/color]   [color=%s]Waiting for you %d[/color]\n" +
		"[color=#aeb6c6]%s[/color]") % [int(c["working"]), int(c["done"]), int(c["failed"]),
		int(c["stalled"]), int(c["pipelines_running"]), waiting_color, int(c["waiting"]), brain_line]
	_notice.text = state.load_error if state.load_error != "" else "Reading " + state_path.get_file()


func _log_block(lines: Array) -> String:
	if lines.is_empty():
		return "[i]No log lines yet.[/i]"
	var out: String = "[code]"
	for line: Variant in lines:
		out += esc(str(line)) + "\n"
	return out + "[/code]"


func _status_word(status: String) -> String:
	var hex: String = TirithState.status_color(status).to_html(false)
	return "[color=#%s][b]%s[/b][/color]" % [hex, status.to_upper()]


func show_detail(kind: String, payload: Dictionary, state: TirithState) -> void:
	match kind:
		"hero": _show_hero(payload, state)
		"building": _show_building(payload, state)
		"district": _show_district(payload, state)
		"caravan": _show_caravan(payload, state)
		"beacon": _show_beacon(payload, state)
		"board": _show_board(payload, state)
		_:
			_title.text = kind.capitalize()
			_subtitle.text = ""
			_body.text = esc(JSON.stringify(payload, "  "))
	_panel.visible = true
	_panel.modulate.a = 0.0
	var tw: Tween = create_tween()
	tw.tween_property(_panel, "modulate:a", 1.0, 0.22)


func _show_hero(agent: Dictionary, state: TirithState) -> void:
	if str(agent.get("id", "")) == "__steward":
		_show_board(state.steward(), state)
		return
	_title.text = TirithState.full_name(agent)
	var project: Dictionary = state.project_by_id(str(agent.get("project", "")))
	_subtitle.text = "%s - %s" % [str(project.get("name", agent.get("project", ""))),
		str(agent.get("feature", ""))]
	var body: String = "Status: %s    Elapsed: %s    Renown: %d\n" % [
		_status_word(str(agent.get("status", ""))),
		TirithState.format_elapsed(int(agent.get("elapsed_s", 0))),
		int(agent.get("renown", 0))]
	body += "Started: %s" % str(agent.get("started_at", "?"))
	if agent.get("ended_at") != null:
		body += "    Ended: %s" % str(agent.get("ended_at"))
	body += "\nFiles changed: %d  (+%d / -%d)\n\n" % [int(agent.get("files_changed", 0)),
		int(agent.get("insertions", 0)), int(agent.get("deletions", 0))]
	body += "[b]What they are doing[/b]\n%s\n\n" % esc(str(agent.get("summary", "")))
	if agent.get("report") != null and str(agent.get("report")) != "":
		body += "[b]Final report[/b]\n%s\n\n" % esc(str(agent.get("report")))
	body += "[b]Log tail[/b]\n" + _log_block(agent.get("log_tail", []) as Array)
	_body.text = body


func _show_building(nd: Dictionary, state: TirithState) -> void:
	_title.text = str(nd.get("label", "?"))
	var owner: Dictionary = {}
	var edges: Array = []
	for p: Variant in state.projects():
		var arch: Dictionary = (p as Dictionary).get("architecture", {}) as Dictionary
		for n: Variant in (arch.get("nodes", []) as Array):
			if str((n as Dictionary).get("id", "")) == str(nd.get("id", "")):
				owner = p
				edges = arch.get("edges", []) as Array
	_subtitle.text = "%s - %s" % [str(owner.get("name", "?")), str(nd.get("kind", "")).capitalize()]
	var body: String = "Kind: [b]%s[/b]    Files: [b]%d[/b]\n\n[b]Connections[/b]\n" % [
		str(nd.get("kind", "")), int(nd.get("files", 0))]
	var any_edge: bool = false
	for e: Variant in edges:
		var edge: Dictionary = e
		if str(edge.get("from", "")) == str(nd.get("id", "")):
			body += "-> %s  (%s)\n" % [esc(str(edge.get("to", ""))), esc(str(edge.get("label", "")))]
			any_edge = true
		if str(edge.get("to", "")) == str(nd.get("id", "")):
			body += "<- %s  (%s)\n" % [esc(str(edge.get("from", ""))), esc(str(edge.get("label", "")))]
			any_edge = true
	if not any_edge:
		body += "[i]No roads touch this building.[/i]"
	_body.text = body


func _show_district(project: Dictionary, state: TirithState) -> void:
	_title.text = str(project.get("name", "?"))
	var arch: Dictionary = project.get("architecture", {}) as Dictionary
	var nodes: Array = arch.get("nodes", []) as Array
	_subtitle.text = "District - %d buildings" % nodes.size()
	var body: String = ""
	if nodes.is_empty():
		body += "[i]Not mapped yet: no architecture has been drawn for this project.[/i]\n\n"
	else:
		body += "Architecture source: [b]%s[/b]  (drawn %s)\n\n[b]Buildings[/b]\n" % [
			str(arch.get("source", "?")), str(arch.get("generated_at", "?"))]
		for n: Variant in nodes:
			var nd: Dictionary = n
			body += "- %s (%s, %d files)\n" % [esc(str(nd.get("label", ""))),
				str(nd.get("kind", "")), int(nd.get("files", 0))]
		body += "\n"
	body += "[b]Heroes here[/b]\n"
	var heroes: Array = state.agents_of_project(str(project.get("id", "")))
	if heroes.is_empty():
		body += "[i]Nobody is working here right now.[/i]\n"
	for a: Variant in heroes:
		var agent: Dictionary = a
		body += "- %s - %s - %s\n" % [esc(TirithState.full_name(agent)),
			_status_word(str(agent.get("status", ""))), esc(str(agent.get("feature", "")))]
	body += "\n[b]Caravans[/b]\n"
	var any_run: bool = false
	for p: Variant in state.pipelines():
		var pipe: Dictionary = p
		if str(pipe.get("project", "")) == str(project.get("id", "")):
			body += "- %s - %s\n" % [esc(str(pipe.get("run", ""))), _status_word(str(pipe.get("status", "")))]
			any_run = true
	if not any_run:
		body += "[i]No pipeline runs recorded.[/i]"
	_body.text = body


func _show_caravan(pipeline: Dictionary, state: TirithState) -> void:
	_title.text = str(pipeline.get("run", "?"))
	var project: Dictionary = state.project_by_id(str(pipeline.get("project", "")))
	_subtitle.text = "%s - %s" % [str(project.get("name", pipeline.get("project", ""))),
		str(pipeline.get("pipeline", ""))]
	var focus: Dictionary = pipeline.get("_stage", {}) as Dictionary
	var body: String = "Run status: %s\nStarted: %s\n\n[b]Stations[/b]\n" % [
		_status_word(str(pipeline.get("status", ""))), str(pipeline.get("started_at", "?"))]
	for s: Variant in (pipeline.get("stages", []) as Array):
		var stage: Dictionary = s
		var dur: String = ""
		if stage.get("duration_s") != null:
			dur = "  (%s)" % TirithState.format_elapsed(int(stage.get("duration_s")))
		var marker: String = "  <-- cart" if str(stage.get("status", "")) == "running" else ""
		body += "- %s  %s%s%s\n" % [esc(str(stage.get("name", ""))),
			_status_word(str(stage.get("status", ""))), dur, marker]
		if str(stage.get("status", "")) == "running" and focus.is_empty():
			focus = stage
	if not focus.is_empty():
		body += "\n[b]Stage log: %s[/b]\n" % esc(str(focus.get("name", "")))
		body += _log_block(focus.get("log_tail", []) as Array)
	_body.text = body


func _show_beacon(payload: Dictionary, state: TirithState) -> void:
	_title.text = "Waiting for you"
	_subtitle.text = "The beacon is lit while this list is not empty"
	var items: Array = payload.get("waiting", state.waiting()) as Array
	if items.is_empty():
		_body.text = "[i]Nothing is waiting for you. The beacon is dark.[/i]"
		return
	var body: String = ""
	for w: Variant in items:
		var item: Dictionary = w
		var project: Dictionary = state.project_by_id(str(item.get("project", "")))
		body += "[b]%s[/b]  (since %s)\n%s\n" % [esc(str(project.get("name", item.get("project", "")))),
			str(item.get("since", "?")), esc(str(item.get("text", "")))]
		if item.get("link") != null and str(item.get("link")) != "":
			body += "%s\n" % esc(str(item.get("link")))
		body += "\n"
	_body.text = body


func _show_board(steward: Dictionary, _state: TirithState) -> void:
	_title.text = "%s, %s" % [str(steward.get("name", "The Manager")), str(steward.get("title", ""))]
	_subtitle.text = "On the tower balcony"
	var body: String = "[b]Right now[/b]\n%s\n(since %s)\n\n[b]Recent notes[/b]\n" % [
		esc(str(steward.get("activity", ""))), str(steward.get("since", "?"))]
	var notes: Array = steward.get("notes", []) as Array
	if notes.is_empty():
		body += "[i]No notes pinned yet.[/i]"
	for n: Variant in notes:
		var note: Dictionary = n
		body += "- [%s] %s: %s\n" % [str(note.get("at", "?")).substr(11, 5),
			esc(str(note.get("project", ""))), esc(str(note.get("text", "")))]
	_body.text = body


