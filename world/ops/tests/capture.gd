extends SceneTree
## Renders the board over a grey backdrop and saves screenshots (needs a real window).
## Run: Godot --path world --script res://ops/tests/capture.gd -- --state <demo_state.json>

const OUT: String = "res://ops/screenshots/"


func _initialize() -> void:
	_run()


func _shot(file: String) -> void:
	await process_frame
	await process_frame
	await create_timer(0.35).timeout
	var img: Image = root.get_texture().get_image()
	img.save_png(ProjectSettings.globalize_path(OUT + file))
	print("saved ", file, " ", img.get_size())


func _run() -> void:
	root.size = Vector2i(1600, 900)
	var bg_layer: CanvasLayer = CanvasLayer.new()
	bg_layer.layer = 0
	var bg: ColorRect = ColorRect.new()
	bg.color = Color(0.42, 0.44, 0.47)
	bg.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	bg_layer.add_child(bg)
	root.add_child(bg_layer)
	var state: TirithState = TirithState.new()
	state.load_from(TirithState.resolve_state_path())
	var board: Node = (load("res://ops/scenes/ops_board.tscn") as PackedScene).instantiate()
	root.add_child(board)
	await process_frame
	board.set_state(state)
	await _shot("01-pipelines.png")
	board.select_stage("android-release#7", "build")
	await _shot("02-pipelines-stage-log.png")
	board.show_tab_named("agents")
	board.set("selected_agent_id", "")
	await _shot("03-agents.png")
	board._on_agent_pressed("w6-perf")
	await _shot("04-agents-expanded.png")
	board.show_tab_named("waiting")
	await _shot("05-waiting.png")
	board.show_tab_named("messages")
	await _shot("06-messages.png")
	board.set_board_open(false)
	await _shot("07-collapsed.png")
	quit(0)
