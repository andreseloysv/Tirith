extends SceneTree
## Headless tests for the Ops board (world/ops). Loads the demo fixture,
## builds the board, and checks header counts, pipeline rows, stage pills,
## agent rows, waiting rows, filters, sorting, signals and time formatting.
## Run: Godot --headless --path world --script res://ops/tests/test_ops.gd \
##        -- --state <abs path to fixtures/state.example.json>

const OpsTime = preload("res://ops/scripts/ops_time.gd")

var _failures: Array = []
var _focused: Array = []


func _initialize() -> void:
	_run()


func check(label: String, ok: bool) -> void:
	print(("PASS " if ok else "FAIL ") + label)
	if not ok:
		_failures.append(label)


func _in_group_deep(node: Node, group: String, out: Array) -> void:
	if node.is_in_group(group):
		out.append(node)
	for child: Node in node.get_children():
		_in_group_deep(child, group, out)


func _group_in(board: Node, group: String) -> Array:
	var out: Array = []
	_in_group_deep(board, group, out)
	return out


func _find_meta(nodes: Array, key: String, value: String) -> Node:
	for n: Node in nodes:
		if n.has_meta(key) and str(n.get_meta(key)) == value:
			return n
	return null


func _label_text(node: Node, child_name: String) -> String:
	var c: Node = node.find_child(child_name, true, false) if node != null else null
	if c != null and c is Label:
		return (c as Label).text
	if c != null and c is RichTextLabel:
		return (c as RichTextLabel).text
	return "<missing:%s>" % child_name


func _run() -> void:
	await process_frame
	# --- OpsTime formatting (pure functions) ----------------------------
	check("format_duration 34s", OpsTime.format_duration(34) == "34s")
	check("format_duration 0s", OpsTime.format_duration(0) == "0s")
	check("format_duration negative clamps", OpsTime.format_duration(-5) == "0s")
	check("format_duration 90s", OpsTime.format_duration(90) == "1m 30s")
	check("format_duration 6000 = 1h 40m", OpsTime.format_duration(6000) == "1h 40m")
	check("format_duration 7200 = 2h 00m", OpsTime.format_duration(7200) == "2h 00m")
	check("format_duration 26h = 1d 2h", OpsTime.format_duration(93600) == "1d 2h")
	check("format_live 34s", OpsTime.format_duration_live(34) == "34s")
	check("format_live 6000 ticks seconds",
		OpsTime.format_duration_live(6000) == "1h 40m 00s"
		and OpsTime.format_duration_live(6009) == "1h 40m 09s")
	check("format_ago seconds", OpsTime.format_ago(30) == "30s ago")
	check("format_ago 12 min", OpsTime.format_ago(12 * 60) == "12 min ago")
	check("format_ago 1h 40m", OpsTime.format_ago(6000) == "1h 40m ago")
	check("format_ago 2h exact", OpsTime.format_ago(7200) == "2h ago")
	check("format_ago future is just now", OpsTime.format_ago(-10) == "just now")
	check("parse_iso Z", OpsTime.parse_iso("2026-10-28T19:40:00Z") == 1793216400)
	check("parse_iso null/empty/garbage",
		OpsTime.parse_iso(null) == -1 and OpsTime.parse_iso("") == -1
		and OpsTime.parse_iso("garbage") == -1)
	var t1930: int = OpsTime.parse_iso("2026-10-28T19:30:00Z")
	var anchor: int = OpsTime.parse_iso("2026-10-28T19:40:00Z")
	check("local time with +60 bias",
		OpsTime.format_local_time_with_bias(t1930, 60, anchor) == "20:30")
	check("local time with -300 bias",
		OpsTime.format_local_time_with_bias(t1930, -300, anchor) == "14:30")
	check("elapsed between", OpsTime.elapsed_between(t1930, anchor, -1) == 600)

	# --- build the board from the fixture --------------------------------
	var path: String = TirithState.resolve_state_path()
	check("state path resolves from --state", path != "" and FileAccess.file_exists(path))
	var state: TirithState = TirithState.new()
	check("fixture parses", state.load_from(path))
	var scene: PackedScene = load("res://ops/scenes/ops_board.tscn") as PackedScene
	check("ops board scene loads", scene != null)
	var board: Node = scene.instantiate()
	root.add_child(board)
	await process_frame
	board.set_state(state)
	await process_frame
	await process_frame

	# Header counts: 1 running, ios failed today (within 24 h), 2/1/1 agents, 2 waiting.
	var hc: Dictionary = board.header_counts()
	check("header pipelines_running=1", int(hc.get("pipelines_running", -1)) == 1)
	check("header pipelines_failed_24h=1", int(hc.get("pipelines_failed_24h", -1)) == 1)
	check("header agents working/stalled/failed = 2/1/1",
		int(hc.get("agents_working", -1)) == 2 and int(hc.get("agents_stalled", -1)) == 1
		and int(hc.get("agents_failed", -1)) == 1)
	check("header waiting=2", int(hc.get("waiting", -1)) == 2)
	check("now is anchored at generated_at", int(board.now_unix()) == anchor)
	check("board open by default, pipelines tab first",
		board.is_board_open() and board.current_tab_name() == "pipelines")

	# --- pipelines --------------------------------------------------------
	check("pipeline rows newest first",
		board.pipeline_run_ids() == ["android-release#7", "ios-testflight#3"])
	var rows: Array = _group_in(board, "ops_pipeline_rows")
	check("2 pipeline row nodes", rows.size() == 2)
	var android: Node = _find_meta(rows, "run_id", "android-release#7")
	var ios: Node = _find_meta(rows, "run_id", "ios-testflight#3")
	check("android row exists, running", android != null
		and str(android.get_meta("status")) == "running")
	check("ios row exists, failed", ios != null
		and str(ios.get_meta("status")) == "failed")
	check("android row project chip", str(android.get_meta("project")) == "chromasol")
	# Running run started 19:30, anchor 19:40 -> 10m 00s live, then ticks.
	check("android live duration 10m 00s",
		_label_text(android, "DurationLabel") == "10m 00s")
	check("android started ago", _label_text(android, "StartedLabel").contains("10 min ago"))
	# Finished run 09:00-09:12 -> 12m compact duration.
	check("ios total duration 12m",
		_label_text(ios, "DurationLabel") == "12m 00s"
		or _label_text(ios, "DurationLabel") == "12m")

	# Stage pills of the expanded running run.
	var pills: Array = _group_in(board, "ops_stage_pills")
	var android_pills: Array = []
	for p: Node in pills:
		if str(p.get_meta("run_id")) == "android-release#7":
			android_pills.append(p)
	check("android has 8 stage pills", android_pills.size() == 8)
	var pill_statuses: Array = []
	for p: Node in android_pills:
		pill_statuses.append(str(p.get_meta("status")))
	check("android pill statuses", pill_statuses == ["passed", "passed", "passed",
		"passed", "running", "pending", "pending", "pending"])
	var build_pill: Node = _find_meta(android_pills, "stage_name", "build")
	check("build pill duration 34s", _label_text(build_pill, "StageDuration") == "34s")
	var sign_pill: Node = _find_meta(android_pills, "stage_name", "sign")
	# sign started 19:31:47, anchor 19:40:00 -> 493 s live.
	check("sign pill live duration 8m 13s",
		_label_text(sign_pill, "StageDuration") == "8m 13s")
	var bars: Array = _group_in(board, "ops_duration_bars")
	var android_bar: Node = _find_meta(bars, "run_id", "android-release#7")
	check("duration bar has 8 segments, first 34s", android_bar != null
		and (android_bar.get("segments") as Array).size() == 8
		and int(((android_bar.get("segments") as Array)[0] as Dictionary).get("seconds", -1)) == 34)
	# Default selected stage is the running one; its log shows.
	check("default stage log is the running stage",
		board.selected_stage_log("android-release#7").contains("signReleaseBundle"))
	board.select_stage("android-release#7", "build")
	check("clicking a stage shows its log tail",
		board.selected_stage_log("android-release#7").contains("BUILD SUCCESSFUL in 34s"))
	board.select_stage("android-release#7", "archive")
	check("unknown stage selection is ignored",
		board.selected_stage_name("android-release#7") == "build")

	# Filters.
	board.set_pipeline_filter("relagame", "")
	check("filter by project", board.pipeline_run_ids() == ["ios-testflight#3"])
	board.set_pipeline_filter("", "running")
	check("filter by status", board.pipeline_run_ids() == ["android-release#7"])
	board.set_pipeline_filter("", "")
	check("filters reset", board.pipeline_run_ids().size() == 2)
	# Re-fetch rows after rebuilds (nodes are recreated on filter changes).
	rows = _group_in(board, "ops_pipeline_rows")
	android = _find_meta(rows, "run_id", "android-release#7")

	# Tick: live timers advance by one second without a state refresh.
	board.tick_once()
	await process_frame
	check("tick advances run duration", _label_text(android, "DurationLabel") == "10m 01s")
	check("tick advances now", int(board.now_unix()) == anchor + 1)

	# --- agents -------------------------------------------------------------
	check("agent rows sorted by elapsed desc by default",
		board.agent_row_ids() == ["w6-perf", "w1-data", "icon-2", "t-collector", "t-world"])
	var agent_rows: Array = _group_in(board, "ops_agent_rows")
	check("5 agent row nodes", agent_rows.size() == 5)
	var ivren: Node = _find_meta(agent_rows, "agent_id", "w6-perf")
	check("ivren row meta", ivren != null and str(ivren.get_meta("status")) == "working"
		and str(ivren.get_meta("project")) == "chromasol")
	check("ivren name and epithet",
		_label_text(ivren, "NameLabel").contains("Ivren of the Seventh Gate"))
	check("ivren rank and class",
		_label_text(ivren, "SubLabel").contains("Captain")
		and _label_text(ivren, "SubLabel").contains("Swiftrider"))
	check("ivren feature shown",
		_label_text(ivren, "SubLabel").contains("Performance and size"))
	check("ivren files changed with +/-",
		_label_text(ivren, "FilesLabel").contains("+412")
		and _label_text(ivren, "FilesLabel").contains("-96"))
	# One tick already happened: 17:40 -> anchor+1 = 2h 00m 01s live.
	check("ivren live elapsed ticks", _label_text(ivren, "ElapsedLabel") == "2h 00m 01s")
	var halvard: Node = _find_meta(agent_rows, "agent_id", "w1-data")
	check("halvard finished duration 1h 40m",
		_label_text(halvard, "ElapsedLabel") == "1h 40m")
	var summary_label: Node = ivren.find_child("SummaryLabel", true, false)
	check("summary one line with full text on hover", summary_label != null
		and (summary_label as Control).tooltip_text.contains("1.39 s per photo"))
	board.connect("focus_agent", func(agent_id: String) -> void: _focused.append(agent_id))
	(ivren as BaseButton).pressed.emit()
	await process_frame
	check("clicking an agent row emits focus_agent", _focused == ["w6-perf"])
	check("clicking an agent row selects and expands it",
		str(board.get("selected_agent_id")) == "w6-perf"
		and _label_text(ivren, "DetailLabel").contains("Measuring the 200-photo scan"))
	board.set_agent_sort("project")
	check("sort by project groups chromasol first",
		board.agent_row_ids().slice(0, 3) == ["w6-perf", "w1-data", "icon-2"])
	board.set_agent_sort("status")
	check("sort by status puts working first",
		str(board.agent_row_ids()[0]) in ["w6-perf", "t-world"])
	board.set_agent_sort("elapsed_desc")

	# --- waiting ------------------------------------------------------------
	check("waiting rows newest first", board.waiting_ids() == ["w-2", "w-1"])
	var wait_rows: Array = _group_in(board, "ops_waiting_rows")
	check("2 waiting row nodes", wait_rows.size() == 2)
	var w2: Node = _find_meta(wait_rows, "item_id", "w-2")
	check("waiting row project and text", w2 != null
		and _label_text(w2, "TextLabel").contains("Install the commit guard"))
	# w-2 since 19:38, now anchor+1s -> 2 min ago (121 s -> 2 min).
	check("waiting age", _label_text(w2, "AgeLabel").contains("2 min ago"))

	# --- tabs, header jump, Tab toggle ---------------------------------------
	board.set_board_open(false)
	check("collapse hides panel but header stays", not board.is_board_open()
		and bool(board.call("is_header_visible")))
	var waiting_btn: Node = board.find_child("HeaderWaiting", true, false)
	(waiting_btn as BaseButton).pressed.emit()
	await process_frame
	check("header waiting click opens waiting tab",
		board.is_board_open() and board.current_tab_name() == "waiting")
	var tab_event := InputEventKey.new()
	tab_event.keycode = KEY_TAB
	tab_event.pressed = true
	board._input(tab_event)
	check("Tab hides the board", not board.is_board_open())
	board._input(tab_event)
	check("Tab shows the board again", board.is_board_open())
	board.show_tab_named("agents")
	check("show_tab_named agents", board.current_tab_name() == "agents")
	board.show_tab_named("bogus")
	check("unknown tab ignored", board.current_tab_name() == "agents")

	# --- messages (flows) from the ops demo fixture ------------------------------------
	var demo: TirithState = TirithState.new()
	var demo_path: String = path.get_base_dir().get_base_dir().path_join("world/ops/tests/demo_state.json")
	check("demo fixture parses", demo.load_from(demo_path))
	board.set_state(demo)
	await process_frame
	check("demo: 4 pipeline runs, newest first",
		board.pipeline_run_ids() == ["android-release#7", "tirith-ci#12", "android-release#6", "ios-testflight#3"])
	check("messages tab exists", board.find_child("Tab_messages", true, false) != null)
	check("9 messages newest first",
		board.message_ids() == ["f-9", "f-8", "f-7", "f-6", "f-5", "f-4", "f-3", "f-2", "f-1"])
	var flow_rows: Array = _group_in(board, "ops_flow_rows")
	check("9 flow row nodes", flow_rows.size() == 9)
	var f3: Node = _find_meta(flow_rows, "flow_id", "f-3")
	check("flow row from and to names",
		_label_text(f3, "FromLabel") == "The Manager"
		and _label_text(f3, "ToLabel") == "Ivren of the Seventh Gate")
	check("flow row text and kind chip",
		_label_text(f3, "TextLabel").contains("Measure scan speed")
		and str(f3.get_meta("kind")) == "brief" and f3.find_child("KindChip", true, false) != null)
	check("ref icon only when ref is set",
		f3.find_child("RefIcon", true, false) != null
		and _find_meta(flow_rows, "flow_id", "f-4").find_child("RefIcon", true, false) == null)
	board.set_message_filter("", "request")
	check("filter by kind", board.message_ids() == ["f-6"])
	board.set_message_filter("agent:w6-perf", "")
	check("filter by agent", board.message_ids() == ["f-3"])
	board.set_message_filter("owner", "")
	check("filter by owner endpoint", board.message_ids() == ["f-9", "f-7", "f-6", "f-4"])
	board.set_message_filter("", "")
	flow_rows = _group_in(board, "ops_flow_rows")
	_focused.clear()
	(_find_meta(flow_rows, "flow_id", "f-3") as BaseButton).pressed.emit()
	(_find_meta(flow_rows, "flow_id", "f-1") as BaseButton).pressed.emit()
	(_find_meta(flow_rows, "flow_id", "f-4") as BaseButton).pressed.emit()
	check("clicking a flow row focuses its agent endpoint only",
		_focused == ["w6-perf", "w1-data"])
	board.show_tab_named("messages")
	check("show messages tab", board.current_tab_name() == "messages")

	# --- empty state does not crash -------------------------------------------
	var empty: TirithState = TirithState.new()
	board.set_state(empty)
	await process_frame
	check("empty state: no rows anywhere",
		board.pipeline_run_ids().is_empty() and board.agent_row_ids().is_empty()
		and board.waiting_ids().is_empty())
	var ehc: Dictionary = board.header_counts()
	check("empty state: header counts zero",
		int(ehc.get("pipelines_running", -1)) == 0 and int(ehc.get("waiting", -1)) == 0)

	print("RESULT: %s (%d failures)" % ["ALL PASS" if _failures.is_empty() else "FAILURES", _failures.size()])
	quit(0 if _failures.is_empty() else 1)
