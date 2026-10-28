extends Node3D
## Stand-alone demo of the Rookery: a little ground, mock heroes, an owner beacon and a
## pipeline cart (so flows have endpoints), the Rookery fed from a state file, and a
## script that plays new messages and flows as they would arrive from the collector.
##
## Run:   Godot --path world res://messenger/demo.tscn -- --state ../fixtures/state.rooms.example.json
## Stills: add  --capture <dir>   (writes the PNGs and quits)

const M: GDScript = preload("res://scripts/mats.gd")
const RookeryScript: GDScript = preload("res://messenger/rookery.gd")
const Data: GDScript = preload("res://messenger/messenger_data.gd")

var rookery: Node3D
var camera: Camera3D
var raw: Dictionary = {}
var heroes: Dictionary = {}
var owner_beacon: Node3D
var cart: Node3D
var _yaw: float = 0.0
var _pitch: float = 0.33
var _dist: float = 26.0
var _target: Vector3 = Vector3(0, 6.0, -1.0)
var _drag: bool = false
var _moved: float = 0.0


func _ready() -> void:
	_environment()
	rookery = RookeryScript.new()
	rookery.name = "Rookery"
	add_child(rookery)
	_mock_citadel_endpoints()
	rookery.locator = _locate
	raw = _load_state()
	var args: PackedStringArray = OS.get_cmdline_user_args()
	var capture_dir: String = ""
	for i: int in args.size():
		if args[i] == "--capture" and i + 1 < args.size():
			capture_dir = args[i + 1]
	_update_camera()
	if capture_dir != "":
		_capture(capture_dir)
	else:
		_live_script()


func _load_state() -> Dictionary:
	var path: String = ProjectSettings.globalize_path("res://").path_join("../fixtures/state.rooms.example.json")
	var args: PackedStringArray = OS.get_cmdline_user_args()
	for i: int in args.size():
		if args[i] == "--state" and i + 1 < args.size():
			path = args[i + 1]
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed as Dictionary if typeof(parsed) == TYPE_DICTIONARY else {}


func _environment() -> void:
	var env: Environment = Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color(0.62, 0.76, 0.90)
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.78, 0.82, 0.92)
	env.ambient_light_energy = 0.55
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	var we: WorldEnvironment = WorldEnvironment.new()
	we.environment = env
	add_child(we)
	var sun: DirectionalLight3D = DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-34, 38, 0)
	sun.light_energy = 1.35
	sun.light_color = Color(1.0, 0.95, 0.85)
	sun.shadow_enabled = true
	add_child(sun)
	add_child(M.mesh_cyl(120.0, 0.4, M.grass_ground(), Vector3(0, -0.2, 0), 48))
	camera = Camera3D.new()
	camera.fov = 52.0
	add_child(camera)
	camera.current = true


func _mock_citadel_endpoints() -> void:
	# Stand-ins for what the main scene provides: hero figures, the owner's beacon, a cart.
	var spots: Dictionary = {"w6-perf": Vector3(-16, 0, 6), "w1-data": Vector3(-14.5, 0, 2.5),
		"icon-2": Vector3(-17.5, 0, 2.0), "t-world": Vector3(-14.0, 0, 9.0), "t-collector": Vector3(-18.0, 0, 9.5)}
	var colors: Dictionary = {"w6-perf": Color(0.98, 0.82, 0.18), "w1-data": Color(0.95, 0.52, 0.16),
		"icon-2": Color(0.55, 0.85, 0.95), "t-world": Color(0.95, 0.52, 0.16), "t-collector": Color(0.48, 0.25, 0.78)}
	var agents: Array = raw.get("agents", []) as Array
	for id: Variant in spots.keys():
		var hero: Node3D = Node3D.new()
		hero.position = spots[id] as Vector3
		hero.add_child(M.mesh_cone(0.5, 1.4, M.solid(colors[id] as Color), Vector3(0, 0.7, 0), 16))
		hero.add_child(M.mesh_sphere(0.22, M.skin(), Vector3(0, 1.6, 0)))
		hero.add_child(M.mesh_box(Vector3(1.4, 0.8, 0.8), M.wood(), Vector3(0, 0.4, 0.9)))
		add_child(hero)
		heroes[str(id)] = hero
	var cobbles: MeshInstance3D = M.mesh_cyl(7.5, 0.3, M.road(), Vector3(-16, 0.05, 6), 32)
	add_child(cobbles)
	owner_beacon = Node3D.new()
	owner_beacon.position = Vector3(15, 0, 8)
	owner_beacon.add_child(M.mesh_cyl(0.7, 6.0, M.stone(), Vector3(0, 3.0, 0), 16))
	owner_beacon.add_child(M.mesh_sphere(0.6, M.emissive(Color(1.0, 0.6, 0.2), 3.0), Vector3(0, 6.4, 0)))
	add_child(owner_beacon)
	cart = Node3D.new()
	cart.position = Vector3(16, 0, 18)
	cart.add_child(M.mesh_box(Vector3(2.0, 0.8, 1.2), M.wood(), Vector3(0, 0.8, 0)))
	cart.add_child(M.mesh_cyl(0.5, 0.2, M.wood_dark(), Vector3(-0.8, 0.5, 0.7), 12))
	cart.add_child(M.mesh_cyl(0.5, 0.2, M.wood_dark(), Vector3(0.8, 0.5, 0.7), 12))
	add_child(cart)
	var _unused: Array = agents


func _locate(endpoint: String) -> Variant:
	if endpoint.begins_with("agent:"):
		var id: String = endpoint.trim_prefix("agent:")
		if heroes.has(id):
			return (heroes[id] as Node3D).global_position + Vector3(0, 2.2, 0)
	elif endpoint == "owner":
		return owner_beacon.global_position + Vector3(0, 6.6, 0)
	elif endpoint == "steward":
		return Vector3(-4.0, 17.0, -14.0)
	elif endpoint.begins_with("pipeline:"):
		return cart.global_position + Vector3(0, 2.0, 0)
	return null


func _state_upto(rooms_keep: Dictionary, flows_keep: int) -> Dictionary:
	var copy: Dictionary = raw.duplicate(true)
	for room: Variant in copy.get("rooms", []) as Array:
		var rd: Dictionary = room as Dictionary
		var keep: int = int(rooms_keep.get(str(rd["id"]), 999))
		rd["messages"] = (rd["messages"] as Array).slice(0, keep)
	copy["flows"] = (copy.get("flows", []) as Array).slice(0, flows_keep)
	return copy


func _live_script() -> void:
	rookery.apply_state(_state_upto({"meta-hackathon-ideas": 3, "chromasol-war-room": 2}, 3))
	await get_tree().create_timer(2.5).timeout
	var steps: Array = [[5, 4, 5], [6, 5, 7], [7, 6, 9], [8, 7, 11]]
	for s: Variant in steps:
		var a: Array = s as Array
		rookery.apply_state(_state_upto({"meta-hackathon-ideas": a[0], "chromasol-war-room": a[1]}, a[2]))
		await get_tree().create_timer(6.0).timeout
	rookery.apply_state(raw)


func _update_camera() -> void:
	var off: Vector3 = Vector3(sin(_yaw) * cos(_pitch), sin(_pitch), cos(_yaw) * cos(_pitch)) * _dist
	camera.global_position = _target + off
	camera.look_at(_target, Vector3.UP)


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		var mb: InputEventMouseButton = event
		if mb.button_index == MOUSE_BUTTON_WHEEL_UP and mb.pressed:
			_dist = clampf(_dist * 0.9, 6.0, 90.0)
		elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN and mb.pressed:
			_dist = clampf(_dist * 1.1, 6.0, 90.0)
		elif mb.button_index == MOUSE_BUTTON_LEFT:
			if mb.pressed:
				_drag = true
				_moved = 0.0
			else:
				_drag = false
				if _moved < 6.0:
					_pick(mb.position)
		_update_camera()
	elif event is InputEventMouseMotion and _drag:
		var mm: InputEventMouseMotion = event
		_moved += mm.relative.length()
		_yaw -= mm.relative.x * 0.005
		_pitch = clampf(_pitch + mm.relative.y * 0.004, 0.05, 1.3)
		_update_camera()


func _pick(pos: Vector2) -> void:
	var from: Vector3 = camera.project_ray_origin(pos)
	var to: Vector3 = from + camera.project_ray_normal(pos) * 400.0
	var query: PhysicsRayQueryParameters3D = PhysicsRayQueryParameters3D.create(from, to, 2)
	var hit: Dictionary = get_world_3d().direct_space_state.intersect_ray(query)
	if not hit.is_empty():
		rookery.try_pick(hit.get("collider") as Object)


# ------------------------------------------------------------------ stills

func _shot(path: String, wait: float = 0.6) -> void:
	await get_tree().create_timer(wait).timeout
	await RenderingServer.frame_post_draw
	var img: Image = get_viewport().get_texture().get_image()
	print("shot ", path, " -> ", "ok" if img.save_png(path) == OK else "ERR", " ", img.get_size())


func _cam(target: Vector3, yaw: float, pitch: float, dist: float) -> void:
	_target = target
	_yaw = yaw
	_pitch = pitch
	_dist = dist
	_update_camera()


func _capture(dir: String) -> void:
	DirAccess.make_dir_recursive_absolute(dir)
	rookery.apply_state(_state_upto({"meta-hackathon-ideas": 7, "chromasol-war-room": 6}, 0))
	await get_tree().create_timer(1.0).timeout
	_cam(Vector3(-1, 8.5, -1), 0.15, 0.26, 36.0)
	await _shot(dir.path_join("01-rookery.png"), 0.8)
	# Perches close up: tags, seals, reply arcs, banners.
	_cam(Vector3(-0.5, 6.4, -3.4), 0.0, 0.10, 15.5)
	await _shot(dir.path_join("02-perches.png"), 0.6)
	# A message arriving: scroll above the speaker, bird in flight.
	rookery.apply_state(raw)
	await get_tree().create_timer(0.9).timeout
	_cam(Vector3(0, 7.5, 0), 0.0, 0.22, 27.0)
	await _shot(dir.path_join("03-message-flight.png"), 0.45)
	# Flows: several kinds in the air at once.
	for i: int in [0, 3, 5, 7, 8, 9]:
		rookery.play_flow((raw["flows"] as Array)[i] as Dictionary)
		await get_tree().create_timer(0.45).timeout
	_cam(Vector3(-4, 9.0, 4), 0.6, 0.36, 40.0)
	await _shot(dir.path_join("04-flows.png"), 0.3)
	await get_tree().create_timer(5.0).timeout
	_cam(Vector3(0, 6.0, -2), 0.0, 0.2, 30.0)
	rookery.show_room("meta-hackathon-ideas")
	await _shot(dir.path_join("05-room-panel.png"), 0.4)
	rookery.show_flows()
	await _shot(dir.path_join("06-flows-panel.png"), 0.4)
	print("CAPTURE DONE")
	get_tree().quit()
