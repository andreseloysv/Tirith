extends CanvasLayer
## Parchment panels of the Rookery: one room's conversation (threaded by `re`, with the
## brain summary and participants) and the "Flows" history. Own CanvasLayer, so it works
## in any scene; styled like the citadel's detail panel.

const Data: GDScript = preload("res://messenger/messenger_data.gd")

signal flows_requested

var _panel: PanelContainer
var _title: Label
var _subtitle: Label
var _body: RichTextLabel
var _log_button: Button
var _current_kind: String = ""
var _current_room_id: String = ""


func _ready() -> void:
	layer = 20
	_panel = PanelContainer.new()
	_panel.name = "MessengerPanel"
	_panel.add_theme_stylebox_override("panel", _parchment_style())
	_panel.set_anchors_preset(Control.PRESET_CENTER_LEFT)
	_panel.position = Vector2(16, -330)
	_panel.size = Vector2(500, 660)
	_panel.custom_minimum_size = Vector2(500, 0)
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
	_subtitle.add_theme_font_size_override("font_size", 14)
	_subtitle.add_theme_color_override("font_color", Color(0.45, 0.30, 0.14))
	_subtitle.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	vbox.add_child(_subtitle)
	vbox.add_child(HSeparator.new())
	_body = RichTextLabel.new()
	_body.bbcode_enabled = true
	_body.scroll_active = true
	_body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_body.add_theme_font_size_override("normal_font_size", 15)
	_body.add_theme_font_size_override("bold_font_size", 15)
	_body.add_theme_font_size_override("italics_font_size", 15)
	_body.add_theme_color_override("default_color", Color(0.18, 0.11, 0.04))
	vbox.add_child(_body)
	var close: Button = Button.new()
	close.text = "Close"
	close.pressed.connect(hide_panel)
	vbox.add_child(close)
	_log_button = Button.new()
	_log_button.name = "FlowsButton"
	_log_button.text = "Messenger flows (F)"
	_log_button.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	_log_button.position = Vector2(16, -92)
	_log_button.pressed.connect(func() -> void: flows_requested.emit())
	add_child(_log_button)


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


func hide_panel() -> void:
	_panel.visible = false
	_current_kind = ""
	_current_room_id = ""


func is_open() -> bool:
	return _panel != null and _panel.visible


func current_kind() -> String:
	return _current_kind


func current_room_id() -> String:
	return _current_room_id


func set_log_button_visible(on: bool) -> void:
	_log_button.visible = on


static func esc(text: String) -> String:
	return text.replace("[", "[lb]")


func show_room(room: Dictionary) -> void:
	_current_kind = "room"
	_current_room_id = str(room.get("id", ""))
	_title.text = str(room.get("title", room.get("id", "Room")))
	var project: String = str(room.get("project", ""))
	_subtitle.text = "%s  -  %d messages  -  %d in the last hour%s" % [
		str(room.get("url", "")), int(room.get("message_count", 0)),
		int(room.get("activity_per_hour", 0)),
		("  -  project " + project) if project != "" else ""]
	var body: String = ""
	var summary: String = str(room.get("summary", ""))
	if summary != "":
		var src: String = str(room.get("summary_source", ""))
		body += "[b]Right now[/b]%s\n%s\n\n" % [
			"  [color=#7a5a1e](Muse)[/color]" if src == "muse" else "", esc(summary)]
	var needs: Array = room.get("needs_owner", []) as Array
	for item: Variant in needs:
		var nd: Dictionary = item as Dictionary
		body += "[color=#b4470f][b]Needs you[/b][/color]  #%d %s: %s\n" % [
			int(nd.get("n", 0)), esc(str(nd.get("from", ""))), esc(str(nd.get("reason", "")))]
	if not needs.is_empty():
		body += "\n"
	body += "[b]Present and speaking[/b]\n"
	for p: Variant in room.get("participants", []) as Array:
		var pd: Dictionary = p as Dictionary
		var kind: String = str(pd.get("kind", "external"))
		var label: String = {"owner": "you", "tirith-agent": "Tirith hero", "external": "visitor"}.get(kind, kind)
		var hero: String = ""
		if pd.get("hero_name") != null and str(pd.get("hero_name")) != "":
			hero = " (" + str(pd.get("hero_name")) + ")"
		var color: String = Data.speaker_color(str(pd.get("name", ""))).to_html(false)
		body += "[color=#%s]o[/color] [b]%s[/b]%s  [i]%s[/i]  %d msgs%s\n" % [
			color, esc(str(pd.get("name", ""))), esc(hero), label, int(pd.get("messages", 0)),
			"  [color=#2f7d3a]here[/color]" if bool(pd.get("present", false)) else ""]
	body += "\n[b]Conversation[/b]  [color=#7a5a1e](indented = reply)[/color]\n"
	for entry: Variant in Data.threaded(room):
		var md: Dictionary = (entry as Dictionary)["msg"] as Dictionary
		var depth: int = int((entry as Dictionary)["depth"])
		var who: String = str(md.get("from", ""))
		var color2: String = Data.speaker_color(who).to_html(false)
		var re_list: Array = md.get("re", []) as Array
		var re_text: String = ""
		if not re_list.is_empty():
			var parts: PackedStringArray = PackedStringArray()
			for r: Variant in re_list:
				parts.append("#%d" % int(r))
			re_text = "  [color=#7a5a1e]re %s[/color]" % ", ".join(parts)
		var seal: String = ""
		var signed: String = str(md.get("signed", "none"))
		if signed == "self":
			seal += "  [color=#a8801c](signed)[/color]"
		elif signed == "platform":
			seal += "  [color=#7f8a96](platform)[/color]"
		var agreed: Array = md.get("agreed", []) as Array
		if not agreed.is_empty():
			var names: PackedStringArray = PackedStringArray()
			for a: Variant in agreed:
				names.append(str(a))
			seal += "  [color=#a3261e][b](sealed: %s)[/b][/color]" % esc(", ".join(names))
		var indent: String = "    ".repeat(depth)
		body += "%s[color=#%s][b]%s[/b][/color] [color=#7a5a1e]#%d %s[/color]%s%s\n%s%s\n\n" % [
			indent, color2, esc(who), int(md.get("n", 0)), Data.short_time(md.get("at")), re_text, seal,
			indent, esc(Data.clip(str(md.get("text", "")), 360))]
	_body.text = body
	_panel.visible = true
	_body.scroll_to_line(_body.get_line_count())


func show_flows(flows: Array) -> void:
	_current_kind = "flows"
	_current_room_id = ""
	_title.text = "Messenger flows"
	_subtitle.text = "Who passed what to whom - newest first (%d)" % flows.size()
	var body: String = ""
	if flows.is_empty():
		body = "[i]Nothing has been passed yet.[/i]"
	var i: int = flows.size() - 1
	while i >= 0:
		var f: Dictionary = flows[i] as Dictionary
		var color: String = Data.flow_color(f).to_html(false)
		body += "[color=#%s][b]%s[/b][/color]  [color=#7a5a1e]%s[/color]\n%s  ->  %s\n%s\n\n" % [
			color, str(f.get("kind", "")).to_upper(), Data.short_time(f.get("at")),
			esc(str(f.get("from_name", ""))), esc(str(f.get("to_name", ""))),
			esc(Data.clip(str(f.get("text", "")), 240))]
		i -= 1
	_body.text = body
	_panel.visible = true
	_body.scroll_to_line(0)
