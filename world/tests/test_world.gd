extends SceneTree
## Headless tests: parse the fixture, build the world, check the scene graph,
## then mutate a temp state file and check the world follows.
## Run: Godot --headless --path world --script res://tests/test_world.gd -- --state <fixture>

var _failures: Array = []


func _initialize() -> void:
	_run()


func check(label: String, ok: bool) -> void:
	print(("PASS " if ok else "FAIL ") + label)
	if not ok:
		_failures.append(label)


func _group_count(group: String) -> int:
	return get_nodes_in_group(group).size()


func _run() -> void:
	await process_frame
	# --- data layer -------------------------------------------------------
	var path: String = TirithState.resolve_state_path()
	check("state path resolves from --state", path != "" and FileAccess.file_exists(path))
	var state: TirithState = TirithState.new()
	check("fixture parses", state.load_from(path))
	check("3 projects", state.projects().size() == 3)
	check("5 agents", state.agents().size() == 5)
	check("2 pipelines", state.pipelines().size() == 2)
	check("2 waiting items", state.waiting().size() == 2)
	var c: Dictionary = state.counts()
	check("counts working=2", int(c["working"]) == 2)
	check("counts done=1 failed=1 stalled=1",
		int(c["done"]) == 1 and int(c["failed"]) == 1 and int(c["stalled"]) == 1)
	check("counts pipelines_running=1", int(c["pipelines_running"]) == 1)
	var ivren: Dictionary = state.agents()[0]
	check("full name format",
		TirithState.full_name(ivren) == "Ivren of the Seventh Gate, Captain Swiftrider")
	check("elapsed format", TirithState.format_elapsed(7200) == "2h 00m"
		and TirithState.format_elapsed(90) == "1m" and TirithState.format_elapsed(45) == "45s")
	var missing: TirithState = TirithState.new()
	check("missing file reports error, no crash",
		not missing.load_from("/nonexistent/state.json") and missing.load_error != "")

	# --- scene graph ------------------------------------------------------
	var scene: PackedScene = load("res://scenes/main.tscn") as PackedScene
	check("main scene loads", scene != null)
	var main: Node = scene.instantiate()
	root.add_child(main)
	await process_frame
	await process_frame
	await process_frame
	check("3 districts built", _group_count("districts") == 3)
	check("11 architecture buildings", _group_count("buildings") == 11)
	check("5 heroes built", _group_count("heroes") == 5)
	check("2 caravans, 2 carts", _group_count("caravans") == 2 and _group_count("carts") == 2)
	check("11 stations (8 + 3)", _group_count("stations") == 11)
	check("many clickable things", _group_count("clickable") >= 25)
	var builder: Node = main.get("builder") as Node
	check("beacon lit (waiting not empty)", bool(builder.get("_beacon_on")))
	var tower: Node = builder.find_child("StewardTower", true, false)
	check("steward tower exists", tower != null)
	check("star emblem exists", builder.find_child("StarEmblem", true, false) != null)
	check("notice board exists", builder.find_child("NoticeBoard", true, false) != null)
	var names: Array = []
	for hero: Node in get_nodes_in_group("heroes"):
		names.append(str((hero.get("agent") as Dictionary).get("name", "")))
	check("heroes are the fixture five",
		names.has("Ivren") and names.has("Halvard") and names.has("Sefa")
		and names.has("Marrow") and names.has("Odile"))

	# --- live update follows state.json ------------------------------------
	var raw: Dictionary = (main.get("state") as TirithState).raw.duplicate(true)
	((raw["agents"] as Array)[0] as Dictionary)["status"] = "failed"
	var extra: Dictionary = ((raw["agents"] as Array)[1] as Dictionary).duplicate(true)
	extra["id"] = "w9-extra"
	extra["name"] = "Testrin"
	extra["status"] = "working"
	(raw["agents"] as Array).append(extra)
	raw["waiting_on_you"] = []
	var tmp_path: String = ProjectSettings.globalize_path("user://tirith_test_state.json")
	var f: FileAccess = FileAccess.open(tmp_path, FileAccess.WRITE)
	f.store_string(JSON.stringify(raw))
	f.close()
	main.set("state_path", tmp_path)
	main.call("_reload_state", false)
	await process_frame
	await process_frame
	check("new hero appears after state change", _group_count("heroes") == 6)
	var ivren_node: Node = null
	for hero: Node in get_nodes_in_group("heroes"):
		if str((hero.get("agent") as Dictionary).get("id", "")) == "w6-perf":
			ivren_node = hero
	check("status change reached the hero", ivren_node != null
		and str(ivren_node.get("status")) == "failed")
	check("beacon dark once waiting is empty", not bool(builder.get("_beacon_on")))

	print("RESULT: %s (%d failures)" % ["ALL PASS" if _failures.is_empty() else "FAILURES", _failures.size()])
	quit(0 if _failures.is_empty() else 1)
