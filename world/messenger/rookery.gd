extends Node3D
## The Rookery: the citadel's messenger tower and plaza.
##
## One perch per agentsmessenger.com room, with a banner, a lantern, a cord of message tags
## (wax seal = agreement, small seal = signed, faint arcs = replies) and a roosting bird.
## When a message arrives a messenger bird flies from the speaker (its hero if it is a
## Tirith agent, otherwise a visiting envoy on the plaza) to the room's perch, and a short
## speech scroll pops above the speaker. The same messengers carry the Steward's information
## flows (briefs, reports, requests, answers...) along an arc between any two endpoints.
##
## Self-contained: needs only the files in res://messenger/. Public API, see README.md:
##   apply_state(raw)                    feed the parsed state.json (rooms + flows)
##   locator: Callable(endpoint) -> Variant   global Vector3 of "agent:<lane>", "owner", ...
##   send_flow(from_pos, to_pos, kind, text)  one messenger between two world positions
##   play_message(room_id, msg)          animate one room message
##   try_pick(collider) -> bool          call from the main scene's click picking
##   show_room(id) / show_flows()        open the parchment panels

const M: GDScript = preload("res://scripts/mats.gd")
const Data: GDScript = preload("res://messenger/messenger_data.gd")
const PanelsScript: GDScript = preload("res://messenger/messenger_panels.gd")

const MAX_PERCHES: int = 8
const MAX_TAGS: int = 8
const MAX_ENVOYS: int = 12
const MAX_FLIGHTS: int = 40
const TOWER_Z: float = -5.0
const TOWER_R: float = 2.5
const PARCHMENT: Color = Color(0.96, 0.91, 0.78)

## Maps an endpoint ("agent:<lane>", "owner", "steward", "pipeline:<run>") to a global
## Vector3, or returns null when unknown. Set by the main scene; `room:<id>` is handled here.
var locator: Callable = Callable()
## Where "steward" resolves when the locator gives nothing (default: the Rookery's own top).
var steward_position: Variant = null
## Flights per message/flow are skipped for the very first state (the existing history).
var animate_initial: bool = false
var bird_scale: float = 2.4
## Perches and envoys are drawn larger than life so they read from the citadel's camera.
var perch_scale: float = 1.7
var envoy_scale: float = 1.6

var panels: CanvasLayer
var _rooms: Array = []
var _room_index: Dictionary = {}           # room id -> perch node
var _envoys: Dictionary = {}               # speaker name -> envoy node
var _flows_seen: Dictionary = {}
var _flows: Array = []
var _first_apply: bool = true
var _flights: Array = []
var _pending_tags: Dictionary = {}         # "room#n" -> true while its bird is still flying
var _time: float = 0.0
var _banners: Array = []
var _roost: Dictionary = {}                # room id -> roost bird
var _lanterns: Dictionary = {}             # room id -> MeshInstance3D
var _dot_mat_cache: Dictionary = {}


func _ready() -> void:
	name = "Rookery"
	_build_plaza()
	_build_tower()
	panels = PanelsScript.new()
	panels.name = "MessengerPanels"
	add_child(panels)
	panels.flows_requested.connect(show_flows)
	set_process_unhandled_key_input(true)


# ----------------------------------------------------------------- building

## Billboard text (kept local so the Rookery does not depend on the world's helper names).
static func _label(text: String, pos: Vector3, color: Color = Color.WHITE, font_size: int = 48,
		pixel_size: float = 0.008, outline: int = 10) -> Label3D:
	var l: Label3D = Label3D.new()
	l.text = text
	l.position = pos
	l.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	l.shaded = false
	l.font_size = font_size
	l.pixel_size = pixel_size
	l.modulate = color
	l.outline_size = outline
	l.outline_modulate = Color(0.05, 0.05, 0.08, 0.85)
	l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	l.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	return l


func _build_plaza() -> void:
	add_child(M.mesh_cyl(10.6, 0.30, M.stone_dark(), Vector3(0, 0.15, 0), 56))
	add_child(M.mesh_cyl(9.9, 0.40, M.road(), Vector3(0, 0.18, 0), 56))
	add_child(M.mesh_torus(9.6, 10.05, M.stone(), Vector3(0, 0.40, 0)))
	# A compass star inlaid in the cobbles, pointing the way to the tower.
	var inlay: MeshInstance3D = M.star(3.0, M.gold(), Vector3(0, 0.42, 3.0))
	inlay.rotation_degrees = Vector3(-90, 0, 0)
	inlay.material_override = M.emissive(Color(0.95, 0.74, 0.25), 0.6)
	inlay.scale = Vector3(1, 1, 0.12)
	add_child(inlay)
	# Lamp posts on the rim.
	for i: int in 6:
		var a: float = PI * (0.18 + 0.64 * float(i) / 5.0)
		var p: Vector3 = Vector3(cos(a) * 9.3, 0, sin(a) * 9.3)
		add_child(M.mesh_cyl(0.10, 1.9, M.wood_dark(), p + Vector3(0, 1.3, 0), 8))
		var bulb: MeshInstance3D = M.mesh_sphere(0.22, M.emissive(Color(1.0, 0.78, 0.4), 2.2), p + Vector3(0, 2.4, 0))
		add_child(bulb)
	var sign_post: Node3D = Node3D.new()
	sign_post.position = Vector3(-4.8, 0, 8.2)
	add_child(sign_post)
	sign_post.add_child(M.mesh_cyl(0.09, 1.7, M.wood_dark(), Vector3(0, 0.85, 0), 8))
	sign_post.add_child(M.mesh_box(Vector3(2.1, 0.55, 0.10), M.wood(), Vector3(0, 1.75, 0)))
	sign_post.add_child(_label("The Rookery", Vector3(0, 1.75, 0.08), Color(1.0, 0.92, 0.72), 40, 0.0075, 8))
	sign_post.rotation.y = 0.35


func _build_tower() -> void:
	var t: Node3D = Node3D.new()
	t.name = "Tower"
	t.position = Vector3(0, 0.36, TOWER_Z)
	add_child(t)
	t.add_child(M.mesh_frustum(3.3, 3.8, 1.3, M.stone_dark(), Vector3(0, 0.65, 0), 40))
	t.add_child(M.mesh_frustum(TOWER_R, 3.2, 12.5, M.stone(), Vector3(0, 7.5, 0), 40))
	for y: float in [4.2, 8.8, 13.0]:
		t.add_child(M.mesh_torus(TOWER_R - 0.05, TOWER_R + 0.35, M.stone_dark(), Vector3(0, y, 0)))
	# Door and windows on the plaza side (+z).
	t.add_child(M.mesh_box(Vector3(1.3, 1.9, 0.5), M.solid(Color(0.16, 0.10, 0.07)), Vector3(0, 1.5, 3.0)))
	var arch: MeshInstance3D = M.mesh_cyl(0.65, 0.5, M.solid(Color(0.16, 0.10, 0.07)), Vector3(0, 2.45, 3.0), 20)
	arch.rotation_degrees = Vector3(90, 0, 0)
	t.add_child(arch)
	# Dovecote crown: timber drum, tiled roof, golden star.
	t.add_child(M.mesh_frustum(3.0, 2.7, 1.5, M.wood_dark(), Vector3(0, 14.1, 0), 32))
	for i: int in 8:
		var a: float = TAU * float(i) / 8.0
		var win: MeshInstance3D = M.mesh_box(Vector3(0.5, 0.7, 0.3), M.solid(Color(0.10, 0.07, 0.05)),
			Vector3(sin(a) * 2.8, 14.2, cos(a) * 2.8))
		win.rotation.y = a
		t.add_child(win)
	t.add_child(M.mesh_cone(3.7, 4.0, M.roof(Color(0.28, 0.58, 0.64)), Vector3(0, 17.0, 0), 32))
	var star: MeshInstance3D = M.star(0.9, M.emissive(Color(0.95, 0.74, 0.25), 1.6), Vector3(0, 19.7, 0))
	t.add_child(star)
	star.set_meta("spin", true)
	_banners.append(star)


## Where a speaker standing on the plaza gets placed: a front arc of figures.
func _envoy_spot(index: int) -> Vector3:
	var slots: int = MAX_ENVOYS
	var a: float = deg_to_rad(-78.0 + 156.0 * (float(index % slots) + 0.5) / float(slots))
	var r: float = 6.6 if index % 2 == 0 else 4.6
	return Vector3(sin(a) * r, 0.38, 1.5 + cos(a) * r * 0.8)


func _make_envoy(speaker: String, kind: String) -> Node3D:
	var root: Node3D = Node3D.new()
	root.name = "Envoy_" + speaker
	var cloak_color: Color = Data.speaker_color(speaker)
	if kind == "owner":
		cloak_color = Color(0.16, 0.32, 0.85)
	var cloak: StandardMaterial3D = M.solid(cloak_color, 0.85)
	root.add_child(M.mesh_cone(0.46, 1.25, cloak, Vector3(0, 0.62, 0), 16))
	root.add_child(M.mesh_sphere(0.19, M.skin(), Vector3(0, 1.38, 0)))
	var hood: MeshInstance3D = M.mesh_cone(0.27, 0.46, cloak, Vector3(0, 1.58, -0.02), 16)
	root.add_child(hood)
	root.add_child(M.mesh_box(Vector3(0.32, 0.26, 0.12), M.wood_dark(), Vector3(0.34, 0.78, 0.18)))
	var trim: Material = M.gold() if kind == "owner" else M.silver()
	root.add_child(M.mesh_torus(0.34, 0.42, trim, Vector3(0, 0.12, 0)))
	var staff: MeshInstance3D = M.mesh_cyl(0.03, 1.8, M.wood_dark(), Vector3(-0.42, 0.9, 0.05), 6)
	root.add_child(staff)
	root.add_child(M.mesh_sphere(0.08, M.emissive(Color(1.0, 0.86, 0.5), 2.0), Vector3(-0.42, 1.84, 0.05)))
	if kind == "owner":
		root.add_child(M.star(0.17, M.gold(), Vector3(0, 0.95, 0.4)))
	var tag: String = speaker if kind != "owner" else speaker + " (you)"
	root.add_child(_label(tag, Vector3(0, 2.2, 0), Color(1, 0.96, 0.85), 34, 0.0065, 8))
	root.set_meta("kind", kind)
	return root


func _ensure_envoy(speaker: String, kind: String) -> Node3D:
	if _envoys.has(speaker):
		return _envoys[speaker] as Node3D
	if _envoys.size() >= MAX_ENVOYS:
		# Plaza full: the oldest envoy without a recent message makes room.
		var oldest: String = str(_envoys.keys()[0])
		(_envoys[oldest] as Node3D).queue_free()
		_envoys.erase(oldest)
	var envoy: Node3D = _make_envoy(speaker, kind)
	var slot: int = _envoys.size()
	envoy.position = _envoy_spot(slot)
	envoy.scale = Vector3.ONE * envoy_scale
	envoy.rotation.y = atan2(0.0 - envoy.position.x, (TOWER_Z + 2.0) - envoy.position.z)
	add_child(envoy)
	_envoys[speaker] = envoy
	return envoy


func _perch_angle(index: int) -> float:
	var side: float = -1.0 if index % 2 == 0 else 1.0
	return side * deg_to_rad(34.0 + 6.0 * float((index / 2) % 2))


func _perch_height(index: int, count: int) -> float:
	var step: float = minf(1.9, 7.6 / float(maxi(count - 1, 1)))
	return 4.9 + float(index) * step


func _build_perch(room: Dictionary, index: int, count: int) -> Node3D:
	var perch: Node3D = Node3D.new()
	perch.name = "Perch_" + str(room["id"])
	var a: float = _perch_angle(index)
	perch.position = Vector3(sin(a) * TOWER_R, _perch_height(index, count) + 0.36, TOWER_Z + cos(a) * TOWER_R)
	perch.rotation.y = a
	perch.scale = Vector3.ONE * perch_scale
	add_child(perch)
	var color: Color = Data.room_color(room)
	# Dovecote alcove built into the wall, beam, rod and strut.
	perch.add_child(M.mesh_box(Vector3(1.7, 1.25, 0.9), M.wood(), Vector3(0, 0.95, 0.2)))
	perch.add_child(M.mesh_box(Vector3(0.9, 0.7, 0.1), M.solid(Color(0.10, 0.07, 0.05)), Vector3(0, 0.95, 0.68)))
	var gable: MeshInstance3D = M.mesh_cone(1.35, 0.8, M.roof(Color(0.28, 0.58, 0.64)), Vector3(0, 1.95, 0.2), 4)
	gable.rotation.y = PI / 4.0
	perch.add_child(gable)
	perch.add_child(M.mesh_box(Vector3(0.34, 0.24, 2.2), M.wood(), Vector3(0, 0.05, 1.1)))
	var strut: MeshInstance3D = M.mesh_box(Vector3(0.18, 0.18, 1.7), M.wood_dark(), Vector3(0, -0.55, 0.85))
	strut.rotation_degrees = Vector3(-30, 0, 0)
	perch.add_child(strut)
	var rod: MeshInstance3D = M.mesh_cyl(0.06, 1.5, M.wood_dark(), Vector3(0, 0.34, 2.15), 8)
	rod.rotation_degrees = Vector3(0, 0, 90)
	perch.add_child(rod)
	# Mast, banner and lantern.
	perch.add_child(M.mesh_cyl(0.05, 2.0, M.wood_dark(), Vector3(1.05, 1.0, 1.75), 8))
	var banner: Node3D = M.banner(0.85, 1.5, color)
	banner.position = Vector3(1.0, 1.95, 1.75)
	perch.add_child(banner)
	_banners.append(banner)
	var lantern: MeshInstance3D = M.mesh_sphere(0.16, M.emissive(Color(1.0, 0.8, 0.45), 0.3), Vector3(-0.95, 1.0, 1.95))
	perch.add_child(lantern)
	perch.add_child(M.mesh_cyl(0.015, 0.6, M.wood_dark(), Vector3(-0.95, 1.4, 1.95), 6))
	_lanterns[str(room["id"])] = lantern
	# Title and counters.
	perch.add_child(_label(str(room["title"]), Vector3(0, 2.6, 1.6), Color(1.0, 0.92, 0.7), 44, 0.0072, 10))
	var count_label: Label3D = _label("", Vector3(0, 2.28, 1.6), Color(0.85, 0.9, 1.0), 30, 0.0062, 8)
	count_label.name = "Count"
	perch.add_child(count_label)
	var cord_root: Node3D = Node3D.new()
	cord_root.name = "Tags"
	cord_root.position = Vector3(0, -0.42, 2.0)
	perch.add_child(cord_root)
	var need: Node3D = Node3D.new()
	need.name = "NeedsYou"
	need.position = Vector3(1.05, 3.0, 1.75)
	perch.add_child(need)
	M.make_clickable(perch, Vector3(3.4, 4.0, 2.6), Vector3(0, 0.6, 1.2), "room",
		{"id": str(room["id"])})
	var roost: Node3D = _make_bird(Color(0.94, 0.92, 0.86), Color(0.98, 0.98, 0.96), false)
	roost.position = Vector3(-0.25, 0.46, 2.15)
	roost.rotation.y = PI / 2.0 + 0.25
	roost.set_meta("folded", true)
	perch.add_child(roost)
	_roost[str(room["id"])] = roost
	return perch


# ----------------------------------------------------------------- birds

func _make_bird(body_color: Color, wing_color: Color, carries_scroll: bool,
		scroll_color: Color = Color(0.97, 0.9, 0.7)) -> Node3D:
	var bird: Node3D = Node3D.new()
	bird.name = "Bird"
	var body: MeshInstance3D = M.mesh_sphere(0.15, M.solid(body_color, 0.8), Vector3.ZERO)
	body.scale = Vector3(0.85, 0.8, 1.5)
	bird.add_child(body)
	bird.add_child(M.mesh_sphere(0.095, M.solid(body_color, 0.8), Vector3(0, 0.07, -0.22)))
	var beak: MeshInstance3D = M.mesh_cone(0.035, 0.12, M.solid(Color(0.95, 0.65, 0.2)), Vector3(0, 0.06, -0.33), 8)
	beak.rotation_degrees = Vector3(-90, 0, 0)
	bird.add_child(beak)
	bird.add_child(M.mesh_box(Vector3(0.14, 0.02, 0.24), M.solid(wing_color, 0.8), Vector3(0, 0.0, 0.32)))
	for side: float in [-1.0, 1.0]:
		var pivot: Node3D = Node3D.new()
		pivot.name = "WingL" if side < 0.0 else "WingR"
		pivot.position = Vector3(side * 0.08, 0.03, -0.02)
		var wing_mat: StandardMaterial3D = M.solid(wing_color, 0.8)
		wing_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
		pivot.add_child(M.mesh_box(Vector3(0.42, 0.015, 0.22), wing_mat, Vector3(side * 0.21, 0, 0)))
		bird.add_child(pivot)
	if carries_scroll:
		var scroll: Node3D = Node3D.new()
		scroll.name = "Scroll"
		scroll.position = Vector3(0, -0.17, -0.02)
		var roll: MeshInstance3D = M.mesh_cyl(0.035, 0.22, M.solid(scroll_color, 0.7), Vector3.ZERO, 10)
		roll.rotation_degrees = Vector3(0, 0, 90)
		scroll.add_child(roll)
		var band: MeshInstance3D = M.mesh_cyl(0.042, 0.05, M.emissive(scroll_color.darkened(0.25), 1.2), Vector3.ZERO, 10)
		band.rotation_degrees = Vector3(0, 0, 90)
		scroll.add_child(band)
		bird.add_child(scroll)
	bird.scale = Vector3.ONE * bird_scale
	return bird


func _flap(bird: Node3D, speed: float, amount: float) -> void:
	var l: Node3D = bird.get_node_or_null("WingL") as Node3D
	var r: Node3D = bird.get_node_or_null("WingR") as Node3D
	if l == null or r == null:
		return
	var a: float = sin(_time * speed + float(bird.get_instance_id() % 7)) * amount
	l.rotation.z = -a
	r.rotation.z = a


func _fold(bird: Node3D) -> void:
	var l: Node3D = bird.get_node_or_null("WingL") as Node3D
	var r: Node3D = bird.get_node_or_null("WingR") as Node3D
	if l != null and r != null:
		l.rotation.z = 1.25
		r.rotation.z = -1.25


# ----------------------------------------------------------------- endpoints

func tower_top_position() -> Vector3:
	return to_global(Vector3(0, 19.0, TOWER_Z))


func perch_position(room_id: String) -> Variant:
	if not _room_index.has(room_id):
		return null
	var perch: Node3D = _room_index[room_id] as Node3D
	return perch.to_global(Vector3(0.0, 0.8, 2.15))


func _fallback_spot(endpoint: String) -> Vector3:
	var h: int = 0
	for i: int in endpoint.length():
		h = (h * 31 + endpoint.unicode_at(i)) % 9973
	var a: float = deg_to_rad(-70.0 + float(h % 140))
	return to_global(Vector3(sin(a) * 7.5, 2.4, 2.0 + cos(a) * 6.0))


## Global position of a flow/message endpoint. Never null: unknown things get a stable
## spot on the plaza so the messenger still shows where information went.
func endpoint_position(endpoint: String) -> Vector3:
	if endpoint.begins_with("room:"):
		var p: Variant = perch_position(endpoint.trim_prefix("room:"))
		if p != null:
			return p as Vector3
	if locator.is_valid():
		var got: Variant = locator.call(endpoint)
		if typeof(got) == TYPE_VECTOR3:
			return got as Vector3
	if endpoint == "steward":
		return steward_position as Vector3 if typeof(steward_position) == TYPE_VECTOR3 else tower_top_position()
	if endpoint == "owner":
		return to_global(Vector3(0.0, 2.6, 8.4))
	return _fallback_spot(endpoint)


## Where a room speaker stands: its hero when it is a Tirith agent the world can place,
## otherwise an envoy on the plaza (created on first use).
func speaker_position(room: Dictionary, speaker: String) -> Vector3:
	var part: Dictionary = Data.participant_of(room, speaker)
	var kind: String = str(part.get("kind", "external"))
	var hero_id: Variant = part.get("hero_id")
	if kind == "tirith-agent" and hero_id != null and locator.is_valid():
		var got: Variant = locator.call("agent:" + str(hero_id))
		if typeof(got) == TYPE_VECTOR3:
			return (got as Vector3) + Vector3(0, 2.4, 0)
	var envoy: Node3D = _ensure_envoy(speaker, kind)
	return envoy.to_global(Vector3(0, 2.1, 0))


# ----------------------------------------------------------------- state

func apply_state(raw: Dictionary) -> void:
	var fresh_rooms: Array = Data.rooms_of(raw)
	var fresh_flows: Array = Data.flows_of(raw)
	var old_rooms: Array = _rooms
	var play: bool = animate_initial or not _first_apply
	var new_msgs: Array = Data.new_messages(old_rooms, fresh_rooms)
	var grown_seals: Array = _grown_seals(old_rooms, fresh_rooms)
	if play:
		for item: Variant in new_msgs:
			_pending_tags["%s#%d" % [str(((item as Dictionary)["room"] as Dictionary)["id"]),
				int(((item as Dictionary)["msg"] as Dictionary).get("n", 0))]] = true
	apply_rooms(fresh_rooms)
	if play:
		var delay: float = 0.0
		for item: Variant in new_msgs.slice(0, 6):
			var d: Dictionary = item as Dictionary
			get_tree().create_timer(delay).timeout.connect(
				play_message.bind(str((d["room"] as Dictionary)["id"]), d["msg"] as Dictionary))
			delay += 0.7
		for seal: Variant in grown_seals.slice(0, 4):
			stamp_seal(str((seal as Dictionary)["room"]), int((seal as Dictionary)["n"]))
	var to_send: Array = Data.new_flows(_flows_seen, fresh_flows)
	for f: Variant in fresh_flows:
		_flows_seen[str((f as Dictionary)["id"])] = true
	_flows = fresh_flows
	if play:
		var d2: float = 0.0
		for f: Variant in to_send.slice(maxi(0, to_send.size() - 8)):
			get_tree().create_timer(d2).timeout.connect(play_flow.bind(f as Dictionary))
			d2 += 0.9
	_first_apply = false
	if panels != null and panels.is_open() and panels.current_kind() == "room":
		show_room(panels.current_room_id())
	elif panels != null and panels.is_open() and panels.current_kind() == "flows":
		show_flows()


func _grown_seals(old_rooms: Array, fresh_rooms: Array) -> Array:
	var before: Dictionary = {}
	for room: Variant in old_rooms:
		for m: Variant in (room as Dictionary)["messages"] as Array:
			before["%s#%d" % [str((room as Dictionary)["id"]), int((m as Dictionary).get("n", 0))]] = \
				((m as Dictionary).get("agreed", []) as Array).size()
	var out: Array = []
	for room: Variant in fresh_rooms:
		for m: Variant in (room as Dictionary)["messages"] as Array:
			var key: String = "%s#%d" % [str((room as Dictionary)["id"]), int((m as Dictionary).get("n", 0))]
			if before.has(key) and ((m as Dictionary).get("agreed", []) as Array).size() > int(before[key]):
				out.append({"room": str((room as Dictionary)["id"]), "n": int((m as Dictionary).get("n", 0))})
	return out


func apply_rooms(rooms: Array) -> void:
	_rooms = rooms.slice(0, MAX_PERCHES)
	var wanted: Dictionary = {}
	for i: int in _rooms.size():
		var room: Dictionary = _rooms[i] as Dictionary
		wanted[str(room["id"])] = true
		if not _room_index.has(str(room["id"])):
			_room_index[str(room["id"])] = _build_perch(room, i, _rooms.size())
		_refresh_perch(room)
	for room_id: Variant in _room_index.keys():
		if not wanted.has(room_id):
			(_room_index[room_id] as Node3D).queue_free()
			_room_index.erase(room_id)
			_roost.erase(room_id)
			_lanterns.erase(room_id)
	# Speakers of the current rooms keep their envoy; make sure plaza figures exist for them.
	for room2: Variant in _rooms:
		for p: Variant in (room2 as Dictionary)["participants"] as Array:
			var pd: Dictionary = p as Dictionary
			var linked: bool = str(pd.get("kind", "")) == "tirith-agent" and pd.get("hero_id") != null \
				and locator.is_valid() and typeof(locator.call("agent:" + str(pd.get("hero_id")))) == TYPE_VECTOR3
			if not linked and (int(pd.get("messages", 0)) > 0 or bool(pd.get("present", false))):
				_ensure_envoy(str(pd.get("name", "")), str(pd.get("kind", "external")))


func _refresh_perch(room: Dictionary) -> void:
	var perch: Node3D = _room_index[str(room["id"])] as Node3D
	var count_label: Label3D = perch.get_node("Count") as Label3D
	count_label.text = "%d messages  -  %d/h" % [int(room.get("message_count", 0)), int(room.get("activity_per_hour", 0))]
	var lantern: MeshInstance3D = _lanterns[str(room["id"])] as MeshInstance3D
	var lit: bool = int(room.get("activity_per_hour", 0)) > 0
	lantern.material_override = M.emissive(Color(1.0, 0.78, 0.4), 3.0 if lit else 0.25)
	var need: Node3D = perch.get_node("NeedsYou") as Node3D
	for c: Node in need.get_children():
		c.queue_free()
	if not (room["needs_owner"] as Array).is_empty():
		var flag: MeshInstance3D = M.mesh_sphere(0.2, M.emissive(Color(1.0, 0.55, 0.12), 3.5))
		need.add_child(flag)
		need.add_child(_label("!", Vector3(0, 0, 0.0), Color(0.2, 0.08, 0.0), 60, 0.009, 0))
		need.add_child(_label("needs you", Vector3(0, 0.38, 0), Color(1.0, 0.7, 0.3), 30, 0.006, 8))
	_rebuild_tags(room)


func _rebuild_tags(room: Dictionary) -> void:
	var perch: Node3D = _room_index[str(room["id"])] as Node3D
	var tags: Node3D = perch.get_node("Tags") as Node3D
	for c: Node in tags.get_children():
		c.queue_free()
	var shown: Array = []
	for m: Variant in room["messages"] as Array:
		if not _pending_tags.has("%s#%d" % [str(room["id"]), int((m as Dictionary).get("n", 0))]):
			shown.append(m)
	shown = shown.slice(maxi(0, shown.size() - MAX_TAGS))
	var spacing: float = 0.40
	var x0: float = -spacing * float(MAX_TAGS - 1) / 2.0
	var x_of: Dictionary = {}
	tags.add_child(M.mesh_box(Vector3(spacing * float(MAX_TAGS) + 0.1, 0.025, 0.025), M.solid(Color(0.30, 0.20, 0.10)),
		Vector3(0, 0.0, 0)))
	var offset: int = MAX_TAGS - shown.size()
	for i: int in shown.size():
		var md: Dictionary = shown[i] as Dictionary
		var x: float = x0 + spacing * float(i + offset)
		x_of[int(md.get("n", 0))] = x
		var tag: Node3D = Node3D.new()
		tag.position = Vector3(x, -0.30, 0)
		tags.add_child(tag)
		tag.add_child(M.mesh_box(Vector3(0.012, 0.28, 0.012), M.solid(Color(0.30, 0.20, 0.10)), Vector3(0, 0.18, 0)))
		tag.add_child(M.mesh_box(Vector3(0.30, 0.40, 0.03), M.solid(PARCHMENT, 0.9), Vector3.ZERO))
		var spk: Color = Data.speaker_color(str(md.get("from", "")))
		tag.add_child(M.mesh_box(Vector3(0.30, 0.09, 0.036), M.solid(spk, 0.8), Vector3(0, 0.155, 0)))
		for ln: int in 3:
			tag.add_child(M.mesh_box(Vector3(0.20 - 0.04 * float(ln == 2), 0.016, 0.034),
				M.solid(Color(0.40, 0.30, 0.20)), Vector3(-0.02 * float(ln == 2), 0.07 - 0.07 * float(ln), 0)))
		var signed: String = str(md.get("signed", "none"))
		if signed == "self":
			var gold_seal: MeshInstance3D = M.mesh_cyl(0.055, 0.04, M.gold(), Vector3(-0.08, -0.15, 0.02), 14)
			gold_seal.rotation_degrees = Vector3(90, 0, 0)
			tag.add_child(gold_seal)
		elif signed == "platform":
			var silver_seal: MeshInstance3D = M.mesh_cyl(0.035, 0.04, M.silver(), Vector3(-0.09, -0.16, 0.02), 12)
			silver_seal.rotation_degrees = Vector3(90, 0, 0)
			tag.add_child(silver_seal)
		if not (md.get("agreed", []) as Array).is_empty():
			var wax: MeshInstance3D = M.mesh_cyl(0.085, 0.05, M.emissive(Color(0.78, 0.10, 0.10), 0.5), Vector3(0.07, -0.13, 0.03), 16)
			wax.rotation_degrees = Vector3(90, 0, 0)
			tag.add_child(wax)
			tag.add_child(M.mesh_sphere(0.03, M.solid(Color(0.55, 0.05, 0.05), 0.5), Vector3(0.07, -0.13, 0.065)))
	# Reply arcs between tags, drawn as thin faint ribbons over the cord.
	for md2: Variant in shown:
		var m2: Dictionary = md2 as Dictionary
		for r: Variant in m2.get("re", []) as Array:
			if x_of.has(int(r)) and x_of.has(int(m2.get("n", 0))):
				var xa: float = float(x_of[int(m2["n"])])
				var xb: float = float(x_of[int(r)])
				tags.add_child(_arc_ribbon(xa, xb, Data.speaker_color(str(m2.get("from", "")))))


func _arc_ribbon(xa: float, xb: float, color: Color) -> MeshInstance3D:
	var span: float = absf(xa - xb)
	var height: float = clampf(0.10 + span * 0.22, 0.14, 0.62)
	var mesh: ImmediateMesh = ImmediateMesh.new()
	var mat: StandardMaterial3D = M.transparent(color.lightened(0.15), 0.60)
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mesh.surface_begin(Mesh.PRIMITIVE_TRIANGLE_STRIP, mat)
	var steps: int = 20
	for i: int in steps + 1:
		var t: float = float(i) / float(steps)
		var x: float = lerpf(xa, xb, t)
		var y: float = 0.40 + sin(t * PI) * height
		var dy: float = cos(t * PI) * PI * height
		var dx: float = xb - xa
		var nrm: Vector2 = Vector2(-dy, dx).normalized() * 0.014
		mesh.surface_add_vertex(Vector3(x + nrm.x, y + nrm.y, 0.03))
		mesh.surface_add_vertex(Vector3(x - nrm.x, y - nrm.y, 0.03))
	mesh.surface_end()
	var mi: MeshInstance3D = MeshInstance3D.new()
	mi.mesh = mesh
	return mi


# ----------------------------------------------------------------- animations

## One room message: speech scroll above the speaker, a messenger bird to the perch,
## a reply arc to the author it answers.
func play_message(room_id: String, msg: Dictionary) -> void:
	var room: Dictionary = {}
	for r: Variant in _rooms:
		if str((r as Dictionary)["id"]) == room_id:
			room = r as Dictionary
	if room.is_empty() or not _room_index.has(room_id):
		return
	var speaker: String = str(msg.get("from", ""))
	var origin: Vector3 = speaker_position(room, speaker)
	var color: Color = Data.speaker_color(speaker)
	_pop_scroll(origin + Vector3(0, 0.9, 0), speaker, str(msg.get("text", "")), color, 5.0)
	var target: Vector3 = perch_position(room_id) as Vector3
	# Reply: a faint arc to whoever is answered.
	for r2: Variant in msg.get("re", []) as Array:
		var answered: Dictionary = Data.message_by_number(room, int(r2))
		if not answered.is_empty() and str(answered.get("from", "")) != speaker:
			_arc_dots(origin, speaker_position(room, str(answered.get("from", ""))), color, 7.0, 0.35)
			break
	var key: String = "%s#%d" % [room_id, int(msg.get("n", 0))]
	var bird: Node3D = _make_bird(Color(0.95, 0.93, 0.88), color.lightened(0.35), true, color.lightened(0.5))
	_launch(bird, origin, target, 0.55, -1.0, func() -> void:
		_pending_tags.erase(key)
		if _room_index.has(room_id):
			for rr: Variant in _rooms:
				if str((rr as Dictionary)["id"]) == room_id:
					_rebuild_tags(rr as Dictionary)
			_pulse_perch(room_id, color)
		_land_and_leave(bird, room_id))


func _pulse_perch(room_id: String, color: Color) -> void:
	var lantern: MeshInstance3D = _lanterns.get(room_id) as MeshInstance3D
	if lantern != null:
		lantern.material_override = M.emissive(color.lightened(0.4), 6.0)
		var tw: Tween = create_tween()
		tw.tween_property(lantern, "scale", Vector3.ONE * 1.8, 0.15)
		tw.tween_property(lantern, "scale", Vector3.ONE, 0.5)
		tw.tween_callback(func() -> void:
			if is_instance_valid(lantern):
				lantern.material_override = M.emissive(Color(1.0, 0.78, 0.4), 3.0))
	var roost: Node3D = _roost.get(room_id) as Node3D
	if roost != null:
		var tw2: Tween = create_tween()
		tw2.tween_property(roost, "scale", Vector3.ONE * bird_scale * 1.25, 0.12)
		tw2.tween_property(roost, "scale", Vector3.ONE * bird_scale, 0.3)
	_ring_pulse(perch_position(room_id) as Vector3, color)


func _land_and_leave(bird: Node3D, room_id: String) -> void:
	# Perch the bird on the rod for a moment, then send it off over the plaza.
	var perch: Node3D = _room_index.get(room_id) as Node3D
	if perch == null or not is_instance_valid(bird):
		return
	var spot: Vector3 = perch.to_global(Vector3(0.3, 0.46, 2.15))
	bird.global_position = spot
	bird.set_meta("landed", true)
	_fold(bird)
	var away: Vector3 = to_global(Vector3(randf_range(-14.0, 14.0), 12.0 + randf() * 5.0, 22.0))
	get_tree().create_timer(2.6).timeout.connect(func() -> void:
		if is_instance_valid(bird):
			bird.remove_meta("landed")
			_launch(bird, bird.global_position, away, 0.0, 3.4, func() -> void:
				if is_instance_valid(bird):
					bird.queue_free()))


## Start a flight along a quadratic arc between two global points.
func _launch(bird: Node3D, from_pos: Vector3, to_pos: Vector3, delay: float, duration: float,
		on_arrive: Callable, extra: Dictionary = {}) -> void:
	if _flights.size() >= MAX_FLIGHTS:
		bird.queue_free()
		on_arrive.call()
		return
	if bird.get_parent() == null:
		add_child(bird)
	bird.visible = delay <= 0.0
	bird.global_position = from_pos
	var dist: float = from_pos.distance_to(to_pos)
	var dur: float = duration if duration > 0.0 else clampf(dist / 7.5, 1.4, 4.2)
	var mid: Vector3 = (from_pos + to_pos) * 0.5 + Vector3(0, clampf(dist * 0.22, 1.2, 7.0), 0)
	_flights.append({"node": bird, "p0": from_pos, "p1": mid, "p2": to_pos, "t": -delay, "dur": dur,
		"done": on_arrive, "extra": extra})


func _bez(p0: Vector3, p1: Vector3, p2: Vector3, t: float) -> Vector3:
	var a: Vector3 = p0.lerp(p1, t)
	var b: Vector3 = p1.lerp(p2, t)
	return a.lerp(b, t)


func _pop_scroll(pos: Vector3, speaker: String, text: String, color: Color, life: float) -> void:
	var body: String = Data.clip(text, 110)
	var holder: Node3D = Node3D.new()
	holder.name = "SpeechScroll"
	add_child(holder)
	holder.global_position = pos
	var lines: int = int(ceil(float(body.length()) / 30.0)) + 1
	var h: float = 0.27 * float(lines) + 0.26
	var back_mat: StandardMaterial3D = M.transparent(Color(0.36, 0.22, 0.10), 1.0)
	back_mat.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	back_mat.no_depth_test = true
	back_mat.render_priority = 0
	var paper_mat: StandardMaterial3D = M.transparent(PARCHMENT, 1.0)
	paper_mat.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	paper_mat.no_depth_test = true
	paper_mat.render_priority = 1
	var back: MeshInstance3D = M.mesh_quad(Vector2(2.24, h + 0.10), back_mat)
	var paper: MeshInstance3D = M.mesh_quad(Vector2(2.14, h), paper_mat)
	holder.add_child(back)
	holder.add_child(paper)
	var title: Label3D = _label(speaker, Vector3(0, h / 2.0 - 0.20, 0), color.darkened(0.25), 32, 0.0062, 0)
	title.outline_size = 0
	title.no_depth_test = true
	title.render_priority = 3
	holder.add_child(title)
	var label: Label3D = _label(body, Vector3(0, -0.12, 0), Color(0.18, 0.11, 0.04), 30, 0.0058, 0)
	label.outline_size = 0
	label.width = 350.0
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.no_depth_test = true
	label.render_priority = 3
	label.position = Vector3(0, -0.14, 0)
	holder.add_child(label)
	# A small pointer below the scroll towards the speaker.
	var tip_mat: StandardMaterial3D = M.transparent(PARCHMENT, 1.0)
	tip_mat.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	tip_mat.no_depth_test = true
	tip_mat.render_priority = 1
	var tip: MeshInstance3D = M.mesh_quad(Vector2(0.22, 0.22), tip_mat, Vector3(0, -h / 2.0 - 0.02, 0))
	holder.add_child(tip)
	holder.scale = Vector3.ONE * 0.05
	holder.position.y += h * 0.9
	var tw: Tween = create_tween()
	tw.tween_property(holder, "scale", Vector3.ONE * 1.9, 0.28).set_trans(Tween.TRANS_BACK).set_ease(Tween.EASE_OUT)
	tw.tween_interval(life)
	tw.tween_property(holder, "scale", Vector3.ONE * 0.01, 0.25)
	tw.tween_callback(holder.queue_free)


func _dot_material(color: Color) -> StandardMaterial3D:
	var key: String = color.to_html()
	if not _dot_mat_cache.has(key):
		var m: StandardMaterial3D = M.transparent(color, 0.8)
		m.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
		m.albedo_texture = M.soft_dot_texture()
		_dot_mat_cache[key] = m
	return _dot_mat_cache[key] as StandardMaterial3D


## A faint dotted arc between two global points that fades away.
func _arc_dots(from_pos: Vector3, to_pos: Vector3, color: Color, life: float, size: float) -> void:
	var holder: Node3D = Node3D.new()
	holder.name = "ArcDots"
	add_child(holder)
	var dist: float = from_pos.distance_to(to_pos)
	var mid: Vector3 = (from_pos + to_pos) * 0.5 + Vector3(0, clampf(dist * 0.25, 0.8, 5.0), 0)
	var count: int = clampi(int(dist * 2.2), 10, 40)
	var mat: StandardMaterial3D = _dot_material(color).duplicate() as StandardMaterial3D
	for i: int in count + 1:
		var dot: MeshInstance3D = M.mesh_quad(Vector2(size, size), mat)
		holder.add_child(dot)
		dot.global_position = _bez(from_pos, mid, to_pos, float(i) / float(count))
	var tw: Tween = create_tween()
	tw.tween_interval(life * 0.6)
	tw.tween_property(mat, "albedo_color:a", 0.0, life * 0.4)
	tw.tween_callback(holder.queue_free)


func _ring_pulse(pos: Vector3, color: Color) -> void:
	var mat: StandardMaterial3D = M.transparent(color.lightened(0.3), 0.8)
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	mat.albedo_texture = M.soft_dot_texture()
	var q: MeshInstance3D = M.mesh_quad(Vector2(0.6, 0.6), mat)
	add_child(q)
	q.global_position = pos
	var tw: Tween = create_tween().set_parallel(true)
	tw.tween_property(q, "scale", Vector3.ONE * 4.0, 0.7)
	tw.tween_property(mat, "albedo_color:a", 0.0, 0.7)
	tw.chain().tween_callback(q.queue_free)


## A wax seal stamped onto a message tag (an agreement arrived).
func stamp_seal(room_id: String, n: int) -> void:
	if not _room_index.has(room_id):
		return
	var perch: Node3D = _room_index[room_id] as Node3D
	var pos: Vector3 = perch.to_global(Vector3(0.0, -0.3, 2.25))
	var seal: MeshInstance3D = M.mesh_cyl(0.20, 0.07, M.emissive(Color(0.82, 0.10, 0.10), 1.5), Vector3.ZERO, 20)
	seal.rotation = perch.global_transform.basis.get_euler() + Vector3(PI / 2.0, 0, 0)
	add_child(seal)
	seal.global_position = pos + Vector3(0, 1.0, 0)
	seal.scale = Vector3.ONE * 2.0
	var tw: Tween = create_tween()
	tw.tween_property(seal, "global_position", pos, 0.35).set_trans(Tween.TRANS_BOUNCE).set_ease(Tween.EASE_OUT)
	tw.parallel().tween_property(seal, "scale", Vector3.ONE * 0.5, 0.35)
	tw.tween_interval(0.8)
	tw.tween_callback(seal.queue_free)
	_ring_pulse(pos, Color(0.9, 0.2, 0.15))


# ----------------------------------------------------------------- flows

func play_flow(flow: Dictionary) -> void:
	send_flow(endpoint_position(str(flow.get("from", "steward"))),
		endpoint_position(str(flow.get("to", "steward"))),
		str(flow.get("kind", "message")), str(flow.get("text", "")), -1.0, Data.flow_color(flow))


## One messenger carrying a small scroll between two global positions. The colour follows
## the flow kind (brief gold, message blue, report green/red, request amber, alert red,
## answer white) and the text shows briefly above it. Returns the messenger node.
func send_flow(from_pos: Vector3, to_pos: Vector3, kind: String, text: String = "",
		duration: float = -1.0, color_override: Variant = null) -> Node3D:
	var color: Color = color_override as Color if typeof(color_override) == TYPE_COLOR \
		else (Data.FLOW_COLORS.get(kind, Data.FLOW_COLORS["message"]) as Color)
	var bird: Node3D = _make_bird(Color(0.96, 0.95, 0.92), color, true, color)
	bird.scale = Vector3.ONE * bird_scale * 0.95
	add_child(bird)
	var label: Label3D = _label(Data.clip(text, 80), Vector3(0, 0.6, 0), color.lightened(0.35), 34, 0.0075, 14)
	label.width = 420.0
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.no_depth_test = true
	label.render_priority = 4
	label.scale = Vector3.ONE / bird_scale
	bird.add_child(label)
	_arc_dots(from_pos, to_pos, color, 3.2, 0.2)
	_launch(bird, from_pos, to_pos, 0.0, duration, func() -> void:
		_ring_pulse(to_pos, color)
		if is_instance_valid(label):
			var tw: Tween = create_tween()
			tw.tween_interval(1.4)
			tw.tween_callback(func() -> void:
				if is_instance_valid(bird):
					bird.queue_free()))
	return bird


# ----------------------------------------------------------------- process / input

func _process(delta: float) -> void:
	_time += delta
	for b: Variant in _banners:
		if not is_instance_valid(b):
			continue
		var n: Node3D = b as Node3D
		if n.has_meta("spin"):
			n.rotation.y += delta * 0.8
		else:
			n.rotation.z = sin(_time * 1.6 + float(n.get_instance_id() % 11)) * 0.07
	for envoy: Variant in _envoys.values():
		var e: Node3D = envoy as Node3D
		e.position.y = 0.38 + absf(sin(_time * 1.3 + float(e.get_instance_id() % 13))) * 0.05
	for roost: Variant in _roost.values():
		var rb: Node3D = roost as Node3D
		if is_instance_valid(rb):
			_fold(rb)
	var finished: Array = []
	for f: Variant in _flights:
		var fd: Dictionary = f as Dictionary
		var bird: Node3D = fd["node"] as Node3D
		if not is_instance_valid(bird):
			finished.append(f)
			continue
		fd["t"] = float(fd["t"]) + delta
		if float(fd["t"]) < 0.0:
			continue
		bird.visible = true
		var u: float = clampf(float(fd["t"]) / float(fd["dur"]), 0.0, 1.0)
		var eased: float = u * u * (3.0 - 2.0 * u) * 0.6 + u * 0.4
		var pos: Vector3 = _bez(fd["p0"] as Vector3, fd["p1"] as Vector3, fd["p2"] as Vector3, eased)
		var ahead: Vector3 = _bez(fd["p0"] as Vector3, fd["p1"] as Vector3, fd["p2"] as Vector3, minf(eased + 0.03, 1.0))
		bird.global_position = pos
		if ahead.distance_to(pos) > 0.001:
			bird.look_at(ahead, Vector3.UP)
		_flap(bird, 26.0, 0.8)
		if u >= 1.0:
			finished.append(f)
			(fd["done"] as Callable).call()
	for f2: Variant in finished:
		_flights.erase(f2)
	for child: Node in get_children():
		if child.name == "Bird" and child.has_meta("landed"):
			_fold(child as Node3D)


func _unhandled_key_input(event: InputEvent) -> void:
	var key: InputEventKey = event as InputEventKey
	if key != null and key.pressed and not key.echo:
		if key.keycode == KEY_F:
			if panels.is_open() and panels.current_kind() == "flows":
				panels.hide_panel()
			else:
				show_flows()
		elif key.keycode == KEY_ESCAPE:
			panels.hide_panel()


## Open a room's parchment panel (conversation, summary, participants).
func show_room(room_id: String) -> void:
	for r: Variant in _rooms:
		if str((r as Dictionary)["id"]) == room_id:
			panels.show_room(r as Dictionary)
			return


func show_flows() -> void:
	panels.show_flows(_flows)


## Returns true (and opens the room panel) when `collider` is one of the Rookery's perches.
func try_pick(collider: Object) -> bool:
	if collider == null or not (collider is Node) or not (collider as Node).has_meta("kind"):
		return false
	var body: Node = collider as Node
	if str(body.get_meta("kind")) != "room" or not is_ancestor_of(body):
		return false
	show_room(str((body.get_meta("payload") as Dictionary).get("id", "")))
	return true


func flights_active() -> int:
	return _flights.size()


func room_ids() -> Array:
	return _room_index.keys()


func envoy_names() -> Array:
	return _envoys.keys()
