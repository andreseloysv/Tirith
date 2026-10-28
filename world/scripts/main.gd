extends Node3D
## Tirith main: environment and light, orbit camera, state.json polling,
## click picking, and the screenshot capture mode used for the demo stills.
## The world itself is built by WorldBuilder; text and panels by CitadelUI.

var state: TirithState
var state_path: String = ""
var builder: WorldBuilder
var ui: CitadelUI
var camera: Camera3D
var env: Environment

# Camera rig: goal values eased towards every frame for smooth motion.
const VIEW_2D5: int = 0
const VIEW_3D: int = 1
const PITCH_2D5: float = 0.6109   # 35 degrees down
const YAW_2D5: float = 0.7854     # 45 degrees around
const ORTHO_DISTANCE: float = 170.0
const ORTHO_MIN: float = 6.0
const ORTHO_MAX: float = 100.0
const ORTHO_DEFAULT: float = 57.0
const ORTHO_PER_DIST: float = 0.85  # ortho size that frames what a 3D distance frames
const DEFAULT_TARGET: Vector3 = Vector3(5, 11, 9)

var view_mode: int = VIEW_2D5
var _target: Vector3 = DEFAULT_TARGET
var _target_goal: Vector3 = _target
var _yaw: float = YAW_2D5
var _yaw_goal: float = _yaw
var _pitch: float = PITCH_2D5
var _pitch_goal: float = _pitch
var _dist: float = 80.0
var _dist_goal: float = _dist
var _size: float = ORTHO_DEFAULT
var _size_goal: float = _size
var _tour_on: bool = false
var _tour_clock: float = 0.0
var _tour_index: int = 0
var _hero_cursor: int = -1

var _last_mtime: int = -2
var _poll_acc: float = 0.0
var _drag_button: int = 0
var _press_pos: Vector2 = Vector2.ZERO
var _press_msec: int = 0
var _drag_moved: float = 0.0


func _ready() -> void:
	_setup_environment()
	builder = WorldBuilder.new()
	builder.name = "World"
	add_child(builder)
	ui = CitadelUI.new()
	ui.name = "UI"
	add_child(ui)
	state_path = TirithState.resolve_state_path()
	state = TirithState.new()
	_reload_state(true)
	_update_camera(true)
	var args: PackedStringArray = OS.get_cmdline_user_args()
	for i: int in range(args.size()):
		if args[i] == "--capture" and i + 1 < args.size():
			_capture_sequence(args[i + 1])


const SUN_EULER_DEG: Vector3 = Vector3(-29.0, 42.0, 0.0)

const SKY_SHADER: String = """
shader_type sky;
uniform vec3 sun_dir = vec3(0.4, 0.5, 0.6);
uniform vec4 zenith : source_color = vec4(0.13, 0.32, 0.68, 1.0);
uniform vec4 horizon : source_color = vec4(0.83, 0.86, 0.94, 1.0);
uniform vec4 warm : source_color = vec4(1.0, 0.72, 0.42, 1.0);

float hash21(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float vnoise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash21(i), hash21(i + vec2(1.0, 0.0)), f.x),
			mix(hash21(i + vec2(0.0, 1.0)), hash21(i + vec2(1.0, 1.0)), f.x), f.y);
}

float fbm(vec2 p) {
	float v = 0.0;
	float amp = 0.5;
	for (int i = 0; i < 5; i++) {
		v += amp * vnoise(p);
		p *= 2.03;
		amp *= 0.5;
	}
	return v;
}

void sky() {
	vec3 dir = EYEDIR;
	float h = clamp(dir.y, -0.2, 1.0);
	vec3 col = mix(horizon.rgb, zenith.rgb, pow(smoothstep(-0.02, 0.55, h), 0.7));
	float sun_d = max(dot(dir, normalize(sun_dir)), 0.0);
	col += warm.rgb * (pow(sun_d, 14.0) * 0.38 + pow(sun_d, 240.0) * 1.6);
	col = mix(col, warm.rgb, (1.0 - smoothstep(0.0, 0.24, h)) * pow(sun_d, 2.0) * 0.5);
	if (dir.y > 0.012) {
		vec2 uv = dir.xz / (dir.y + 0.14) * 1.15 + vec2(TIME * 0.004, TIME * 0.0016);
		float cl = fbm(uv * 1.7);
		float cover = smoothstep(0.50, 0.72, cl) * smoothstep(0.012, 0.10, dir.y);
		vec3 cloud_col = mix(vec3(0.90, 0.93, 0.99), vec3(1.0, 0.80, 0.60), pow(sun_d, 2.0));
		cloud_col *= 0.82 + 0.34 * cl;
		col = mix(col, cloud_col, cover * 0.88);
	}
	col = mix(vec3(0.58, 0.65, 0.70), col, smoothstep(-0.16, 0.012, dir.y));
	COLOR = col;
}
"""


func _setup_environment() -> void:
	var sun_basis: Basis = Basis.from_euler(SUN_EULER_DEG * PI / 180.0)
	var to_sun: Vector3 = (sun_basis * Vector3.BACK).normalized()
	var sky_shader: Shader = Shader.new()
	sky_shader.code = SKY_SHADER
	var sky_mat: ShaderMaterial = ShaderMaterial.new()
	sky_mat.shader = sky_shader
	sky_mat.set_shader_parameter("sun_dir", to_sun)
	var sky: Sky = Sky.new()
	sky.sky_material = sky_mat
	env = Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_sky_contribution = 0.42
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_ACES
	env.tonemap_exposure = 1.0
	env.glow_enabled = true
	env.glow_normalized = true
	env.glow_intensity = 0.42
	env.glow_strength = 1.0
	env.glow_bloom = 0.06
	env.glow_hdr_threshold = 1.05
	env.ssao_enabled = true
	env.ssao_radius = 0.9
	env.ssao_intensity = 1.6
	env.ssao_power = 1.5
	env.ssao_detail = 0.6
	env.ssil_enabled = true
	env.ssil_intensity = 0.9
	env.fog_enabled = true
	env.fog_light_color = Color(0.76, 0.81, 0.90)
	env.fog_light_energy = 0.85
	env.fog_sun_scatter = 0.12
	env.fog_density = 0.0017
	env.fog_sky_affect = 0.22
	env.fog_height = 12.0
	env.fog_height_density = 0.0025
	env.volumetric_fog_enabled = true
	env.volumetric_fog_density = 0.0012
	env.volumetric_fog_albedo = Color(0.90, 0.87, 0.84)
	env.volumetric_fog_length = 190.0
	env.volumetric_fog_sky_affect = 0.15
	env.adjustment_enabled = true
	env.adjustment_saturation = 1.22
	env.adjustment_contrast = 1.08
	var we: WorldEnvironment = WorldEnvironment.new()
	we.environment = env
	add_child(we)
	var sun: DirectionalLight3D = DirectionalLight3D.new()
	sun.name = "Sun"
	sun.light_color = Color(1.0, 0.77, 0.54)
	sun.light_energy = 1.95
	sun.light_angular_distance = 2.0
	sun.shadow_enabled = true
	sun.shadow_blur = 1.4
	sun.shadow_opacity = 0.88
	sun.directional_shadow_max_distance = 320.0
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.rotation_degrees = SUN_EULER_DEG
	add_child(sun)
	var fill: DirectionalLight3D = DirectionalLight3D.new()
	fill.name = "SkyFill"
	fill.light_color = Color(0.55, 0.68, 1.0)
	fill.light_energy = 0.30
	fill.shadow_enabled = false
	fill.rotation_degrees = Vector3(-42.0, 222.0, 0.0)
	add_child(fill)
	camera = Camera3D.new()
	camera.name = "Camera"
	camera.fov = 50.0
	camera.near = 0.1
	camera.far = 700.0
	add_child(camera)
	camera.current = true


func _reload_state(first: bool) -> void:
	var fresh: TirithState = TirithState.new()
	var ok: bool = fresh.load_from(state_path)
	_last_mtime = TirithState.file_mtime(state_path)
	if ok:
		state = fresh
		builder.apply_state(state, first)
	else:
		# Keep showing the last good world; the HUD carries the error.
		state.load_error = fresh.load_error
		if first:
			builder.apply_state(state, true)
	ui.set_hud(state, state_path)


func _process(delta: float) -> void:
	_poll_acc += delta
	if _poll_acc >= 1.0:
		_poll_acc = 0.0
		var mtime: int = TirithState.file_mtime(state_path)
		if mtime != _last_mtime:
			_reload_state(false)
	_keyboard_pan(delta)
	_update_tour(delta)
	_update_camera(false, delta)


# ------------------------------------------------------------------ camera
# Two modes share one rig. In 2.5D (default) the camera is orthographic at a
# fixed isometric-style angle and only pans, zooms (orthographic size) and
# turns in 90 degree steps. In 3D it is the free orbit camera.

func is_2d5() -> bool:
	return view_mode == VIEW_2D5


func _update_camera(snap: bool, delta: float = 0.0) -> void:
	var k: float = 1.0 if snap else 1.0 - exp(-delta * 7.0)
	_target = _target.lerp(_target_goal, k)
	_yaw = lerpf(_yaw, _yaw_goal, k)
	_pitch = lerpf(_pitch, _pitch_goal, k)
	_dist = lerpf(_dist, _dist_goal, k)
	_size = lerpf(_size, _size_goal, k)
	var dist: float = ORTHO_DISTANCE if is_2d5() else _dist
	var offset: Vector3 = Vector3(sin(_yaw) * cos(_pitch), sin(_pitch), cos(_yaw) * cos(_pitch)) * dist
	camera.global_position = _target + offset
	camera.look_at(_target, Vector3.UP)
	if env != null:
		# Fog is measured from the camera, which sits far away in 2.5D.
		env.fog_density = 0.00025 if is_2d5() else 0.0017
		env.fog_height_density = 0.0004 if is_2d5() else 0.0025
		env.volumetric_fog_enabled = not is_2d5()
	if is_2d5():
		if camera.projection != Camera3D.PROJECTION_ORTHOGONAL:
			camera.projection = Camera3D.PROJECTION_ORTHOGONAL
		camera.keep_aspect = Camera3D.KEEP_HEIGHT
		camera.size = _size
	elif camera.projection != Camera3D.PROJECTION_PERSPECTIVE:
		camera.projection = Camera3D.PROJECTION_PERSPECTIVE
		camera.fov = 50.0


## Fly the camera to look at `target`. `dist` is the perspective distance in
## 3D mode; in 2.5D it is converted into the orthographic size and the yaw is
## left alone unless `yaw_2d5` is given.
func set_camera_goal(target: Vector3, yaw: float, pitch: float, dist: float, snap: bool = false) -> void:
	_target_goal = _clamp_target(target)
	if is_2d5():
		_size_goal = clampf(dist * ORTHO_PER_DIST, ORTHO_MIN, ORTHO_MAX)
	else:
		_yaw_goal = yaw
		_pitch_goal = pitch
		_dist_goal = dist
	if snap:
		_update_camera(true)


## Centre on a world position (used by double-click, hotkeys and the Ops board).
## `ortho_size` < 0 keeps the current zoom unless it is further out than `max_size`.
func focus_point(pos: Vector3, ortho_size: float = -1.0, snap: bool = false) -> void:
	_target_goal = _clamp_target(pos + Vector3(0, 1.0, 0))
	if is_2d5():
		if ortho_size > 0.0:
			_size_goal = clampf(ortho_size, ORTHO_MIN, ORTHO_MAX)
	elif ortho_size > 0.0:
		_dist_goal = clampf(ortho_size / ORTHO_PER_DIST, 10.0, 170.0)
	if snap:
		_update_camera(true)


func reset_camera(snap: bool = false) -> void:
	_target_goal = DEFAULT_TARGET
	if is_2d5():
		_yaw_goal = _yaw_goal_for_step(_yaw_goal)
		_size_goal = ORTHO_DEFAULT
		_pitch_goal = PITCH_2D5
	else:
		_yaw_goal = 0.38
		_pitch_goal = 0.60
		_dist_goal = 80.0
	if snap:
		_update_camera(true)


func set_view_mode(mode: int, snap: bool = false) -> void:
	if mode == view_mode:
		return
	view_mode = mode
	if is_2d5():
		_size_goal = clampf(_dist_goal * ORTHO_PER_DIST, ORTHO_MIN, ORTHO_MAX)
		_size = _size_goal
		_pitch_goal = PITCH_2D5
		_yaw_goal = _yaw_goal_for_step(_yaw_goal)
	else:
		_dist_goal = clampf(_size_goal / ORTHO_PER_DIST, 10.0, 170.0)
		_dist = _dist_goal
		_pitch_goal = 0.60
	ui.set_view_label(is_2d5())
	_update_camera(snap)


func toggle_view_mode() -> void:
	set_view_mode(VIEW_3D if is_2d5() else VIEW_2D5)


## The 2.5D yaw is always 45 degrees plus a multiple of 90 degrees.
func _yaw_goal_for_step(yaw: float) -> float:
	return roundf((yaw - YAW_2D5) / (PI / 2.0)) * (PI / 2.0) + YAW_2D5


func rotate_view(steps: int) -> void:
	if is_2d5():
		_yaw_goal = _yaw_goal_for_step(_yaw_goal) + float(steps) * PI / 2.0
	else:
		_yaw_goal += float(steps) * PI / 4.0


func _clamp_target(t: Vector3) -> Vector3:
	return Vector3(clampf(t.x, -85.0, 85.0), clampf(t.y, 0.0, 34.0), clampf(t.z, -85.0, 85.0))


func _zoom_by(factor: float) -> void:
	# factor > 1 zooms in.
	if is_2d5():
		_size_goal = clampf(_size_goal / factor, ORTHO_MIN, ORTHO_MAX)
	else:
		_dist_goal = clampf(_dist_goal / factor, 10.0, 170.0)


## Pan the camera target by a screen-space amount in pixels (content follows
## the pointer: dragging right moves the world right).
func _pan_pixels(px: Vector2) -> void:
	var right: Vector3 = Vector3(cos(_yaw_goal), 0, -sin(_yaw_goal))
	var forward: Vector3 = Vector3(-sin(_yaw_goal), 0, -cos(_yaw_goal))
	var units_per_px: float
	var fwd_gain: float = 1.0
	if is_2d5():
		var vh: float = maxf(get_viewport().get_visible_rect().size.y, 1.0)
		units_per_px = _size_goal / vh
		fwd_gain = 1.0 / maxf(sin(PITCH_2D5), 0.3)  # ground foreshortening
	else:
		units_per_px = _dist_goal * 0.0016
	_target_goal -= right * px.x * units_per_px
	_target_goal += forward * px.y * units_per_px * fwd_gain
	_target_goal = _clamp_target(_target_goal)


func _keyboard_pan(delta: float) -> void:
	if get_viewport().gui_get_focus_owner() != null:
		return
	var dir: Vector2 = Vector2.ZERO
	if Input.is_key_pressed(KEY_A) or Input.is_key_pressed(KEY_LEFT):
		dir.x -= 1.0
	if Input.is_key_pressed(KEY_D) or Input.is_key_pressed(KEY_RIGHT):
		dir.x += 1.0
	if Input.is_key_pressed(KEY_W) or Input.is_key_pressed(KEY_UP):
		dir.y += 1.0
	if Input.is_key_pressed(KEY_S) or Input.is_key_pressed(KEY_DOWN):
		dir.y -= 1.0
	if dir != Vector2.ZERO:
		_tour_on = false
		var right: Vector3 = Vector3(cos(_yaw_goal), 0, -sin(_yaw_goal))
		var forward: Vector3 = Vector3(-sin(_yaw_goal), 0, -cos(_yaw_goal))
		var span: float = _size_goal if is_2d5() else _dist_goal * 0.8
		var speed: float = span * 0.55 * delta
		_target_goal = _clamp_target(_target_goal + (right * dir.x + forward * dir.y).normalized() * speed)
	if Input.is_key_pressed(KEY_EQUAL) or Input.is_key_pressed(KEY_PLUS) or Input.is_key_pressed(KEY_KP_ADD):
		_zoom_by(exp(delta * 1.6))
	if Input.is_key_pressed(KEY_MINUS) or Input.is_key_pressed(KEY_KP_SUBTRACT):
		_zoom_by(exp(-delta * 1.6))


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		var key: InputEventKey = event
		if key.keycode == KEY_ESCAPE:
			ui.hide_panel()
		elif key.keycode == KEY_R:
			_tour_on = false
			reset_camera()
		elif key.keycode == KEY_V:
			toggle_view_mode()
		elif key.keycode == KEY_Q:
			rotate_view(-1)
		elif key.keycode == KEY_E:
			rotate_view(1)
		elif key.keycode == KEY_T:
			_tour_on = not _tour_on
			_tour_clock = 0.0
			_tour_index = 0
			ui.set_tour_label(_tour_on)
		elif key.keycode == KEY_H:
			_tour_on = false
			_cycle_hero()
		elif key.keycode == KEY_B:
			_tour_on = false
			focus_point(Vector3(0, 28, 0), 26.0)
		elif key.keycode >= KEY_1 and key.keycode <= KEY_9:
			_tour_on = false
			_focus_project(int(key.keycode - KEY_1))
		return
	if event is InputEventMagnifyGesture:
		# Trackpad pinch: factor > 1 spreads the fingers (zoom in).
		_zoom_by(pow(maxf((event as InputEventMagnifyGesture).factor, 0.05), 1.2))
		return
	if event is InputEventPanGesture:
		var pg: InputEventPanGesture = event
		if pg.meta_pressed or pg.ctrl_pressed:
			_zoom_by(exp(-pg.delta.y * 0.045))
		else:
			# Two-finger scroll pans; the gesture delta is in scroll units.
			_pan_pixels(-pg.delta * 11.0)
		return
	if event is InputEventMouseButton:
		var mb: InputEventMouseButton = event
		if mb.button_index == MOUSE_BUTTON_WHEEL_UP and mb.pressed:
			_zoom_by(1.12)
		elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN and mb.pressed:
			_zoom_by(1.0 / 1.12)
		elif mb.button_index in [MOUSE_BUTTON_LEFT, MOUSE_BUTTON_RIGHT, MOUSE_BUTTON_MIDDLE]:
			if mb.pressed:
				_drag_button = int(mb.button_index)
				_press_pos = mb.position
				_press_msec = Time.get_ticks_msec()
				_drag_moved = 0.0
				if mb.double_click and mb.button_index == MOUSE_BUTTON_LEFT:
					_center_on_pick(mb.position)
			else:
				if mb.button_index == MOUSE_BUTTON_LEFT and _drag_moved < 6.0 \
						and Time.get_ticks_msec() - _press_msec < 500:
					_pick(mb.position)
				_drag_button = 0
	elif event is InputEventMouseMotion and _drag_button != 0:
		var mm: InputEventMouseMotion = event
		_drag_moved += mm.relative.length()
		if _drag_moved > 6.0:
			_tour_on = false
		if is_2d5() or _drag_button != MOUSE_BUTTON_LEFT:
			_pan_pixels(mm.relative)
		else:
			_yaw_goal -= mm.relative.x * 0.0052
			_pitch_goal = clampf(_pitch_goal + mm.relative.y * 0.0042, 0.10, 1.38)


func _focus_project(index: int) -> void:
	var projects: Array = state.projects()
	if index < 0 or index >= projects.size():
		return
	var pid: String = str((projects[index] as Dictionary).get("id", ""))
	var center: Vector3 = builder.district_center(pid)
	if is_2d5():
		focus_point(center, 22.0)
	else:
		set_camera_goal(center + Vector3(0, 1.5, 0), atan2(center.x, center.z), 0.62, 27.0)


func _cycle_hero() -> void:
	var heroes: Array[Node] = get_tree().get_nodes_in_group("heroes")
	if heroes.is_empty():
		return
	_hero_cursor = (_hero_cursor + 1) % heroes.size()
	focus_point((heroes[_hero_cursor] as Node3D).global_position, 12.0)


## Demo tour (key T): slowly visits the overview, each district, the heroes
## and the beacon, for recording the demo video.
func _update_tour(delta: float) -> void:
	if not _tour_on:
		return
	_tour_clock -= delta
	if _tour_clock > 0.0:
		return
	_tour_clock = 6.0
	var stops: Array = [["all"]]
	for i: int in range(state.projects().size()):
		stops.append(["project", i])
	for h: Node in get_tree().get_nodes_in_group("heroes"):
		stops.append(["hero", h])
	stops.append(["beacon"])
	var stop: Array = stops[_tour_index % stops.size()]
	_tour_index += 1
	match str(stop[0]):
		"all":
			reset_camera()
			rotate_view(1 if _tour_index > stops.size() else 0)
		"project":
			_focus_project(int(stop[1]))
		"hero":
			if is_instance_valid(stop[1]):
				focus_point((stop[1] as Node3D).global_position, 11.0)
		"beacon":
			focus_point(Vector3(0, 28, 0), 26.0)


func _pick_hit(screen_pos: Vector2) -> Dictionary:
	var from: Vector3 = camera.project_ray_origin(screen_pos)
	var to: Vector3 = from + camera.project_ray_normal(screen_pos) * 600.0
	var query: PhysicsRayQueryParameters3D = PhysicsRayQueryParameters3D.create(from, to, 2)
	var space: PhysicsDirectSpaceState3D = get_world_3d().direct_space_state
	return space.intersect_ray(query)


func _center_on_pick(screen_pos: Vector2) -> void:
	var hit: Dictionary = _pick_hit(screen_pos)
	if hit.is_empty():
		return
	var collider: Object = hit.get("collider")
	if collider == null or not (collider as Node).has_meta("kind"):
		return
	var kind: String = str((collider as Node).get_meta("kind"))
	var size_for: float = 11.0 if kind == "hero" else 20.0
	focus_point((collider as Node3D).global_position, minf(_size_goal, size_for) if is_2d5() else -1.0)


func _pick(screen_pos: Vector2) -> void:
	var hit: Dictionary = _pick_hit(screen_pos)
	if hit.is_empty():
		return
	var collider: Object = hit.get("collider")
	if collider == null or not (collider as Node).has_meta("kind"):
		return
	var body: Node = collider
	var kind: String = str(body.get_meta("kind"))
	var payload: Dictionary = body.get_meta("payload") as Dictionary
	# Always show the freshest copy of hero/pipeline data we have.
	if kind == "hero":
		for a: Variant in state.agents():
			if str((a as Dictionary).get("id", "")) == str(payload.get("id", "")):
				payload = a
				break
	elif kind == "caravan":
		for p: Variant in state.pipelines():
			if str((p as Dictionary).get("run", "")) == str(payload.get("run", "")):
				var stage_keep: Variant = payload.get("_stage")
				payload = (p as Dictionary).duplicate()
				if stage_keep != null:
					payload["_stage"] = stage_keep
				break
	ui.show_detail(kind, payload, state)


# ------------------------------------------------------------- capture mode

func _capture_sequence(out_dir: String) -> void:
	await get_tree().create_timer(1.2).timeout
	# 1: the whole citadel in the default 2.5D view.
	reset_camera(true)
	await _save_shot(out_dir.path_join("01-overview-2d5.png"))
	# 2: first district zoomed.
	var projects: Array = state.projects()
	if not projects.is_empty():
		var pid: String = str((projects[0] as Dictionary).get("id", ""))
		focus_point(builder.district_center(pid), 20.0, true)
		await _save_shot(out_dir.path_join("02-district-2d5.png"))
	# 3: a hero close-up with the parchment panel; prefer one using an owner GLB.
	var hero_agent: Dictionary = {}
	for a: Variant in state.agents():
		var glb: String = "res://assets/characters/%s.glb" % str((a as Dictionary).get("class", ""))
		if ResourceLoader.exists(glb):
			hero_agent = a
			break
	if hero_agent.is_empty():
		for a: Variant in state.agents():
			if str((a as Dictionary).get("status", "")) == "working":
				hero_agent = a
				break
	if not hero_agent.is_empty():
		var hero: Hero = builder.hero_node(str(hero_agent.get("id", "")))
		if hero != null:
			focus_point(hero.global_position, 9.0, true)
			ui.show_detail("hero", hero_agent, state)
			await _save_shot(out_dir.path_join("03-hero-2d5.png"))
			ui.hide_panel()
	# 4: the first caravan on its road.
	var carts: Array[Node] = get_tree().get_nodes_in_group("carts")
	if not carts.is_empty():
		focus_point((carts[0] as Node3D).global_position, 22.0, true)
		await _save_shot(out_dir.path_join("04-caravan-2d5.png"))
	# 5: the free 3D mode still works.
	set_view_mode(VIEW_3D, true)
	reset_camera(true)
	await _save_shot(out_dir.path_join("05-overview-3d.png"))
	print("CAPTURE DONE -> ", out_dir)
	get_tree().quit()


func _save_shot(path: String) -> void:
	await get_tree().create_timer(0.65).timeout
	await RenderingServer.frame_post_draw
	var img: Image = get_viewport().get_texture().get_image()
	var err: Error = img.save_png(path)
	print("shot ", path, " -> ", "ok" if err == OK else str(err), " ", img.get_size())
