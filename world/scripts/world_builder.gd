class_name WorldBuilder
extends Node3D
## Builds the citadel from state.json: static hill/tower once, then districts,
## heroes and caravans diffed on every state change so updates animate instead
## of popping. All geometry is procedural (see Mats); no imported assets.

const T1_Y: float = 2.6   # terrace tops
const T2_Y: float = 5.2
const T3_Y: float = 7.8
const DISTRICT_R: float = 18.0
const CARAVAN_R: float = 32.5
const BALCONY_Y: float = 29.6

var _t: float = 0.0
var _banners: Array = []
var _motes: Array = []       # {node, from, to, speed, off} light flowing on roads
var _clouds: Array = []
var _star_top: MeshInstance3D
var _beacon_mat: StandardMaterial3D
var _beacon_light: OmniLight3D
var _beacon_on: bool = false
var _scroll_label: Label3D
var _beacon_body: StaticBody3D
var _board_body: StaticBody3D
var _steward_bubble: Label3D
var _steward_name: Label3D

var _district_nodes: Dictionary = {}   # project id -> Node3D
var _district_sigs: Dictionary = {}    # project id -> architecture signature
var _district_centers: Dictionary = {} # project id -> Vector3
var _heroes: Dictionary = {}           # agent id -> Hero
var _caravans: Dictionary = {}         # run -> Dictionary of parts


func _ready() -> void:
	_build_static()


func district_center(project_id: String) -> Vector3:
	return _district_centers.get(project_id, Vector3(0, T2_Y, DISTRICT_R)) as Vector3


func hero_node(agent_id: String) -> Hero:
	return _heroes.get(agent_id) as Hero


# ---------------------------------------------------------------- static world

func _build_static() -> void:
	# Ground and horizon: a deep-green plain that fog fades into the hills.
	add_child(Mats.mesh_cyl(260.0, 0.4, Mats.grass_ground(), Vector3(0, -0.48, 0), 64))
	var apron: StandardMaterial3D = Mats.textured("grass", Color(0.95, 0.98, 0.9), 1.0, 30.0, false, false)
	add_child(Mats.mesh_cyl(56.0, 0.5, apron, Vector3(0, -0.25, 0), 72))
	# Ring terraces as a hill: sloped rock skirt, stone retaining band,
	# grass top, cobbled plaza only on the tower terrace.
	_terrace(27.0, T2_Y, 0.0)
	_terrace(11.0, T3_Y, 9.5)
	_ring_wall(25.6, T2_Y, 28)
	_ring_wall(10.2, T3_Y, 14)
	# South road out of the gate to the caravan ring.
	add_child(Mats.mesh_box(Vector3(4.2, 0.08, 9.0), Mats.road(), Vector3(0, 0.04, 33.0)))
	_ring_road(DISTRICT_R, T2_Y)
	_build_tower()
	_build_caravan_road()
	_decorate_terraces()
	_scatter_nature()
	_scatter_detail()


func _ring_road(radius: float, base_y: float) -> void:
	var segments: int = 48
	var step: float = TAU / float(segments)
	var seg_len: float = 2.0 * radius * sin(step / 2.0) + 0.3
	for i: int in range(segments):
		var ang: float = step * float(i)
		var seg: MeshInstance3D = Mats.mesh_box(Vector3(seg_len, 0.07, 3.2),
			Mats.road(), Vector3(sin(ang) * radius, base_y + 0.16, cos(ang) * radius))
		seg.rotation.y = ang
		add_child(seg)


## Filler life on the rings: cottages on the lower terrace, lamps and
## greenery above, so the citadel feels lived in between the districts.
func _decorate_terraces() -> void:
	# Lamps on the board rim, between the districts; nothing that costs space.
	for i: int in range(12):
		var ang: float = 0.26 + float(i) * TAU / 12.0
		var lamp: Node3D = Node3D.new()
		lamp.position = Vector3(sin(ang) * 24.9, T2_Y + 0.1, cos(ang) * 24.9)
		lamp.add_child(Mats.mesh_cyl(0.06, 2.0, Mats.wood_dark(), Vector3(0, 1.0, 0), 8))
		lamp.add_child(Mats.mesh_sphere(0.15, Mats.emissive(Color(1.0, 0.72, 0.32), 1.5), Vector3(0, 2.1, 0)))
		add_child(lamp)


func _terrace(radius: float, top_y: float, plaza_radius: float) -> void:
	# Battered rock skirt so the rings read as one hill, not stacked drums.
	add_child(Mats.mesh_frustum(radius - 0.8, radius + 3.4, top_y + 0.3,
		Mats.rock(), Vector3(0, (top_y - 0.3) / 2.0, 0), 72))
	# Stone retaining band and a grass lip just under the wall walk.
	add_child(Mats.mesh_cyl(radius + 0.15, 1.15, Mats.stone_dark(), Vector3(0, top_y - 0.55, 0), 72))
	add_child(Mats.mesh_cyl(radius - 0.3, 0.30, Mats.grass(), Vector3(0, top_y - 0.05, 0), 72))
	if plaza_radius > 0.0:
		add_child(Mats.mesh_cyl(plaza_radius, 0.34, Mats.road(), Vector3(0, top_y - 0.03, 0), 48))
		var medallion: MeshInstance3D = Mats.star(2.6,
			Mats.emissive(Color(0.95, 0.74, 0.28), 0.55), Vector3(0, top_y + 0.16, 9.5))
		medallion.rotation.x = -PI / 2.0
		add_child(medallion)


func _ring_wall(radius: float, base_y: float, segments: int) -> void:
	var step: float = TAU / float(segments)
	var seg_len: float = 2.0 * radius * sin(step / 2.0) + 0.25
	for i: int in range(segments):
		var ang: float = step * float(i)
		if absf(wrapf(ang, -PI, PI)) < step * 0.9:
			continue  # south gate gap (+Z is angle 0 in this placement)
		var pos: Vector3 = Vector3(sin(ang), 0, cos(ang)) * radius
		var wall: MeshInstance3D = Mats.mesh_box(Vector3(seg_len, 1.15, 0.55),
			Mats.stone(), pos + Vector3(0, base_y + 0.57, 0))
		wall.rotation.y = ang
		add_child(wall)
		var cap: MeshInstance3D = Mats.mesh_box(Vector3(seg_len, 0.18, 0.75),
			Mats.stone_dark(), pos + Vector3(0, base_y + 1.22, 0))
		cap.rotation.y = ang
		add_child(cap)
		for mo: float in [-seg_len * 0.26, seg_len * 0.26]:  # merlons
			var out: Vector3 = Vector3(sin(ang), 0, cos(ang))
			var side: Vector3 = Vector3(cos(ang), 0, -sin(ang))
			var merlon: MeshInstance3D = Mats.mesh_box(Vector3(0.55, 0.42, 0.6),
				Mats.stone(), pos + side * mo + Vector3(0, base_y + 1.5, 0))
			merlon.rotation.y = ang
			add_child(merlon)
		if i % 6 == 0:
			var turret: Node3D = Node3D.new()
			turret.position = pos + Vector3(0, base_y, 0)
			turret.add_child(Mats.mesh_cyl(0.95, 2.9, Mats.stone(), Vector3(0, 1.45, 0), 12))
			turret.add_child(Mats.mesh_cyl(1.12, 0.28, Mats.stone_dark(), Vector3(0, 2.95, 0), 12))
			turret.add_child(Mats.mesh_cone(1.25, 1.5,
				Mats.roof(Color(0.42, 0.52, 0.66)), Vector3(0, 3.8, 0), 12))
			turret.add_child(Mats.mesh_sphere(0.13, Mats.gold(), Vector3(0, 4.65, 0)))
			turret.add_child(Mats.mesh_box(Vector3(0.3, 0.5, 0.08),
				Mats.emissive(Color(1.0, 0.74, 0.36), 1.6),
				Vector3(sin(ang) * 0.93, 1.9, cos(ang) * 0.93)))
			add_child(turret)


func _build_tower() -> void:
	var tower: Node3D = Node3D.new()
	tower.name = "StewardTower"
	tower.scale = Vector3.ONE * 0.74
	tower.position.y = T3_Y
	add_child(tower)
	var y: float = 0.0
	tower.add_child(Mats.mesh_cyl(7.0, 1.2, Mats.stone_dark(), Vector3(0, 0.6, 0), 32))
	y = 1.2
	tower.add_child(Mats.mesh_cyl(5.6, 4.6, Mats.stone(), Vector3(0, y + 2.3, 0), 32))
	y += 4.6
	tower.add_child(Mats.mesh_cyl(6.0, 0.7, Mats.silver(), Vector3(0, y + 0.35, 0), 32))
	y += 0.7
	tower.add_child(Mats.mesh_cyl(4.3, 7.5, Mats.stone(), Vector3(0, y + 3.75, 0), 32))
	y += 7.5
	tower.add_child(Mats.mesh_cyl(4.7, 0.7, Mats.silver(), Vector3(0, y + 0.35, 0), 32))
	y += 0.7
	tower.add_child(Mats.mesh_cyl(3.2, 6.5, Mats.stone(), Vector3(0, y + 3.25, 0), 24))
	y += 6.5
	# Balcony (y here is BALCONY_Y - T3_Y = 21.8).
	tower.add_child(Mats.mesh_cyl(4.6, 0.55, Mats.stone(), Vector3(0, y + 0.27, 0), 24))
	for i: int in range(10):
		var ang: float = TAU * float(i) / 10.0
		tower.add_child(Mats.mesh_box(Vector3(0.28, 0.8, 0.28), Mats.stone_dark(),
			Vector3(sin(ang) * 4.3, y + 0.9, cos(ang) * 4.3)))
	# Corner spires and the central spire with the star.
	for i: int in range(4):
		var ang: float = PI / 4.0 + TAU * float(i) / 4.0
		var sp: Vector3 = Vector3(sin(ang) * 3.4, y, cos(ang) * 3.4)
		tower.add_child(Mats.mesh_cyl(0.55, 3.4, Mats.stone(), sp + Vector3(0, 1.7, 0), 10))
		tower.add_child(Mats.mesh_cone(0.8, 2.0, Mats.silver(), sp + Vector3(0, 4.4, 0), 10))
		tower.add_child(Mats.mesh_sphere(0.11, Mats.gold(), sp + Vector3(0, 5.5, 0)))
	tower.add_child(Mats.mesh_cyl(1.5, 4.4, Mats.stone(), Vector3(0, y + 2.2, 0), 16))
	tower.add_child(Mats.mesh_cone(2.0, 3.0, Mats.silver(), Vector3(0, y + 5.9, 0), 16))
	_star_top = Mats.star(2.1, Mats.emissive(Color(1.0, 0.80, 0.28), 2.6), Vector3(0, y + 8.6, 0))
	_star_top.name = "StarEmblem"
	tower.add_child(_star_top)
	var star_light: OmniLight3D = OmniLight3D.new()
	star_light.light_color = Color(1.0, 0.85, 0.45)
	star_light.light_energy = 1.2
	star_light.omni_range = 26.0
	star_light.position = Vector3(0, y + 8.6, 0)
	tower.add_child(star_light)
	# Warm windows down the shaft.
	var win_mat: StandardMaterial3D = Mats.emissive(Color(1.0, 0.72, 0.32), 1.7)
	for wy: float in [3.2, 8.5, 12.5, 16.5, 19.5]:
		for k: int in range(3):
			var wang: float = 0.5 + float(k) * 2.1 + wy * 0.13
			var wr: float = 5.65 if wy < 6.0 else (4.35 if wy < 14.0 else 3.25)
			var win: MeshInstance3D = Mats.mesh_box(Vector3(0.55, 0.9, 0.12), win_mat,
				Vector3(sin(wang) * wr, wy, cos(wang) * wr))
			win.rotation.y = wang
			tower.add_child(win)
	# Door on the south side.
	tower.add_child(Mats.mesh_box(Vector3(1.7, 2.8, 0.3), Mats.wood_dark(), Vector3(0, 1.4, 5.55)))
	tower.add_child(Mats.mesh_box(Vector3(2.3, 0.5, 0.5), Mats.stone_dark(), Vector3(0, 3.0, 5.55)))
	# Tower banners (steward blue and gold).
	for sx: float in [-2.6, 2.6]:
		var b: Node3D = Mats.banner(1.1, 3.2, Color(0.16, 0.32, 0.85))
		b.position = Vector3(sx, 13.5, 4.15)
		tower.add_child(b)
		_banners.append(b)
	# The beacon: a brazier on the balcony front, lit while someone waits.
	var beacon: Node3D = Node3D.new()
	beacon.name = "Beacon"
	beacon.position = Vector3(0, y + 0.55, 3.6)
	tower.add_child(beacon)
	beacon.add_child(Mats.mesh_cyl(0.32, 1.0, Mats.silver(), Vector3(0, 0.5, 0), 10))
	beacon.add_child(Mats.mesh_cone(0.75, 0.5, Mats.stone_dark(), Vector3(0, 1.2, 0), 12))
	_beacon_mat = Mats.emissive(Color(1.0, 0.55, 0.15), 0.4)
	beacon.add_child(Mats.mesh_sphere(0.52, _beacon_mat, Vector3(0, 1.45, 0)))
	_beacon_light = OmniLight3D.new()
	_beacon_light.light_color = Color(1.0, 0.6, 0.2)
	_beacon_light.light_energy = 0.0
	_beacon_light.omni_range = 30.0
	_beacon_light.position = Vector3(0, 1.6, 0)
	beacon.add_child(_beacon_light)
	_beacon_body = Mats.make_clickable(beacon, Vector3(2.6, 3.2, 2.6), Vector3(0, 1.2, 0), "beacon", {})
	# "Waiting for you" scroll hanging under the balcony.
	var scroll: Node3D = Mats.banner(3.4, 1.15, Color(0.96, 0.90, 0.76))
	scroll.position = Vector3(0, y - 0.4, 3.45)
	tower.add_child(scroll)
	_banners.append(scroll)
	_scroll_label = Mats.label3d("", Vector3(0, y - 0.95, 3.9), Color(0.45, 0.2, 0.05), 56, 0.011)
	tower.add_child(_scroll_label)
	# The Steward on the balcony, with his speech bubble.
	_steward_name = Mats.label3d("The Steward", Vector3(-2.4, y + 3.1, 3.4), Color(0.65, 0.78, 1.0), 44, 0.007)
	tower.add_child(_steward_name)
	_steward_bubble = Mats.label3d("", Vector3(-2.4, y + 4.4, 3.4), Color.WHITE, 40, 0.0075)
	_steward_bubble.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_steward_bubble.width = 620.0
	tower.add_child(_steward_bubble)
	var steward_hero: Hero = Hero.new()
	steward_hero.name = "StewardFigure"
	steward_hero.position = Vector3(-2.4, y + 0.55, 3.4)
	steward_hero.rotation.y = 0.0
	tower.add_child(steward_hero)
	steward_hero.setup({"id": "__steward", "name": "Aldric", "epithet": "of the Tower",
		"class": "steward", "rank": "marshal", "status": "working", "elapsed_s": 0,
		"project": "", "feature": "", "summary": "", "log_tail": [], "report": null})
	steward_hero.remove_from_group("heroes")
	var bench: Node = steward_hero.get_node_or_null("Workbench")
	if bench != null:
		bench.queue_free()
	for extra: String in ["Nameplate", "StatusOrb"]:
		var n: Node = steward_hero.get_node_or_null(extra)
		if n != null:
			(n as Node3D).visible = false
	# Notice board at the tower foot.
	var board: Node3D = Node3D.new()
	board.name = "NoticeBoard"
	board.position = Vector3(4.2, T3_Y, 8.6)
	board.rotation.y = 0.5
	add_child(board)
	for sx: float in [-1.1, 1.1]:
		board.add_child(Mats.mesh_box(Vector3(0.16, 2.2, 0.16), Mats.wood_dark(), Vector3(sx, 1.1, 0)))
	board.add_child(Mats.mesh_box(Vector3(2.7, 1.6, 0.12), Mats.wood(), Vector3(0, 1.65, 0)))
	board.add_child(Mats.mesh_box(Vector3(2.4, 1.3, 0.05), Mats.solid(Color(0.95, 0.90, 0.78)), Vector3(0, 1.65, 0.09)))
	board.add_child(Mats.mesh_box(Vector3(3.0, 0.14, 0.7), Mats.stone_dark(), Vector3(0, 2.6, 0)))
	board.add_child(Mats.label3d("Steward's board", Vector3(0, 3.1, 0), Color(1.0, 0.92, 0.7), 44, 0.007))
	for i: int in range(3):
		board.add_child(Mats.mesh_box(Vector3(0.6, 0.42, 0.03), Mats.solid(Color.WHITE),
			Vector3(-0.7 + 0.7 * float(i), 1.8 - 0.25 * float(i % 2), 0.14)))
	_board_body = Mats.make_clickable(board, Vector3(3.0, 2.8, 1.0), Vector3(0, 1.5, 0), "board", {})


func _build_caravan_road() -> void:
	var segments: int = 56
	var step: float = TAU / float(segments)
	var seg_len: float = 2.0 * CARAVAN_R * sin(step / 2.0) + 0.3
	for i: int in range(segments):
		var ang: float = step * float(i)
		var pos: Vector3 = Vector3(sin(ang), 0, cos(ang)) * CARAVAN_R
		var seg: MeshInstance3D = Mats.mesh_box(Vector3(seg_len, 0.10, 4.4),
			Mats.road(), pos + Vector3(0, 0.05, 0))
		seg.rotation.y = ang
		add_child(seg)
		if i % 7 == 0:  # road lantern
			var lamp: Node3D = Node3D.new()
			lamp.position = Vector3(sin(ang), 0, cos(ang)) * (CARAVAN_R + 2.6)
			lamp.add_child(Mats.mesh_cyl(0.07, 2.2, Mats.wood_dark(), Vector3(0, 1.1, 0), 8))
			lamp.add_child(Mats.mesh_sphere(0.17, Mats.emissive(Color(1.0, 0.72, 0.32), 1.5), Vector3(0, 2.3, 0)))
			add_child(lamp)


## One MultiMesh of small ground detail (grass tufts, flower specks, rocks)
## so the terrain has grain at district distance without hundreds of nodes.
func _scatter_detail() -> void:
	var rng: RandomNumberGenerator = RandomNumberGenerator.new()
	rng.seed = 424243
	var bands: Array = [
		[40.0, 54.0, 0.05],
		[24.6, 25.4, T2_Y + 0.12], [11.6, 12.6, T2_Y + 0.12],
	]
	var tuft_mesh: CylinderMesh = CylinderMesh.new()
	tuft_mesh.top_radius = 0.02
	tuft_mesh.bottom_radius = 0.20
	tuft_mesh.height = 0.5
	tuft_mesh.radial_segments = 6
	_add_multimesh(tuft_mesh, Mats.solid(Color.WHITE, 1.0), bands, 520, rng,
		[Color(0.20, 0.46, 0.18), Color(0.32, 0.58, 0.22), Color(0.48, 0.68, 0.26)],
		0.55, 1.05, 0.16)
	var flower_mesh: SphereMesh = SphereMesh.new()
	flower_mesh.radius = 0.09
	flower_mesh.height = 0.13
	flower_mesh.radial_segments = 8
	flower_mesh.rings = 5
	_add_multimesh(flower_mesh, Mats.solid(Color.WHITE, 0.7), bands, 210, rng,
		[Color(0.95, 0.35, 0.30), Color(1.0, 0.80, 0.30), Color(0.75, 0.55, 1.0),
			Color(1.0, 0.95, 0.90), Color(1.0, 0.55, 0.25)], 0.8, 1.6, 0.14)
	var rock_mesh: SphereMesh = SphereMesh.new()
	rock_mesh.radius = 0.5
	rock_mesh.height = 0.62
	rock_mesh.radial_segments = 7
	rock_mesh.rings = 4
	_add_multimesh(rock_mesh, Mats.solid(Color.WHITE, 1.0), bands, 64, rng,
		[Color(0.52, 0.52, 0.55), Color(0.62, 0.59, 0.54), Color(0.44, 0.45, 0.49)],
		0.5, 1.9, 0.10)


func _add_multimesh(mesh: Mesh, mat: Material, bands: Array, count: int,
		rng: RandomNumberGenerator, colors: Array, smin: float, smax: float,
		y_off: float) -> void:
	var mm: MultiMesh = MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.mesh = mesh
	mm.instance_count = count
	for i: int in range(count):
		var band: Array = bands[rng.randi_range(0, bands.size() - 1)]
		var ang: float = rng.randf() * TAU
		var r: float = rng.randf_range(float(band[0]), float(band[1]))
		var pos: Vector3 = Vector3(sin(ang) * r, float(band[2]) + y_off, cos(ang) * r)
		var basis: Basis = Basis.from_euler(Vector3(0, rng.randf() * TAU, 0))
		basis = basis.scaled(Vector3.ONE * rng.randf_range(smin, smax))
		mm.set_instance_transform(i, Transform3D(basis, pos))
		mm.set_instance_color(i, colors[rng.randi_range(0, colors.size() - 1)])
	if mat is StandardMaterial3D:
		(mat as StandardMaterial3D).vertex_color_use_as_albedo = true
	var mmi: MultiMeshInstance3D = MultiMeshInstance3D.new()
	mmi.multimesh = mm
	mmi.material_override = mat
	add_child(mmi)


func _scatter_nature() -> void:
	var rng: RandomNumberGenerator = RandomNumberGenerator.new()
	rng.seed = 20261028
	var leaf: StandardMaterial3D = Mats.solid(Color(0.16, 0.38, 0.17))
	var leaf2: StandardMaterial3D = Mats.solid(Color(0.27, 0.52, 0.22))
	var leaf_warm: StandardMaterial3D = Mats.solid(Color(0.60, 0.38, 0.15))
	for i: int in range(34):
		var ang: float = rng.randf() * TAU
		var r: float = rng.randf_range(44.0, 62.0)
		if absf(wrapf(ang, -PI, PI)) < 0.10:
			continue  # keep the south road clear
		var tree: Node3D = Node3D.new()
		tree.position = Vector3(sin(ang) * r, 0, cos(ang) * r)
		var h: float = rng.randf_range(1.6, 2.6)
		tree.add_child(Mats.mesh_cyl(0.16, h, Mats.wood_dark(), Vector3(0, h / 2.0, 0), 8))
		if i % 2 == 0:
			tree.add_child(Mats.mesh_cone(1.35, 3.0, leaf, Vector3(0, h + 1.4, 0), 9))
			tree.add_child(Mats.mesh_cone(0.95, 2.1, leaf2, Vector3(0, h + 2.6, 0), 9))
		else:
			var canopy: StandardMaterial3D = leaf_warm if i % 7 == 3 else leaf2
			tree.add_child(Mats.mesh_sphere(1.1, canopy, Vector3(0, h + 0.95, 0)))
			tree.add_child(Mats.mesh_sphere(0.75, leaf, Vector3(0.6, h + 0.55, 0.3)))
			tree.add_child(Mats.mesh_sphere(0.6, canopy, Vector3(-0.55, h + 0.7, -0.25)))
		add_child(tree)
	for i: int in range(10):  # rocks
		var ang: float = rng.randf() * TAU
		var r: float = rng.randf_range(44.0, 60.0)
		var rock: MeshInstance3D = Mats.mesh_sphere(rng.randf_range(0.5, 1.1),
			Mats.solid(Color(0.55, 0.55, 0.58)), Vector3(sin(ang) * r, 0.2, cos(ang) * r))
		rock.scale.y = 0.6
		add_child(rock)
	# Layered horizon: dark forested foothills, then blue peaks with snow.
	for i: int in range(12):
		var fang: float = rng.randf() * TAU
		var fr: float = rng.randf_range(105.0, 140.0)
		var fh: float = rng.randf_range(8.0, 17.0)
		add_child(Mats.mesh_cone(rng.randf_range(20.0, 34.0), fh,
			Mats.solid(Color(0.24, 0.42, 0.33)),
			Vector3(sin(fang) * fr, fh / 2.0 - 1.0, cos(fang) * fr), 10))
	for i: int in range(10):
		var hang: float = rng.randf() * TAU
		var hr: float = rng.randf_range(150.0, 215.0)
		var hh: float = rng.randf_range(22.0, 44.0)
		var hrad: float = rng.randf_range(28.0, 52.0)
		var peak_pos: Vector3 = Vector3(sin(hang) * hr, hh / 2.0 - 1.0, cos(hang) * hr)
		add_child(Mats.mesh_cone(hrad, hh,
			Mats.textured("rock", Color(0.62, 0.68, 0.80), 1.0, 0.06, true), peak_pos, 9))
		if hh > 30.0:
			add_child(Mats.mesh_cone(hrad * 0.30, hh * 0.30,
				Mats.solid(Color(0.94, 0.96, 1.0)),
				peak_pos + Vector3(0, hh * 0.36, 0), 9))
	for i: int in range(6):  # drifting clouds
		var cloud: Node3D = Node3D.new()
		cloud.position = Vector3(rng.randf_range(-90.0, 90.0), rng.randf_range(44.0, 56.0), rng.randf_range(-90.0, 90.0))
		var cm: StandardMaterial3D = Mats.solid(Color(1.02, 0.97, 0.92))
		cm.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		for k: int in range(4):
			var puff: MeshInstance3D = Mats.mesh_sphere(rng.randf_range(1.6, 2.8), cm,
				Vector3(float(k) * 1.9 - 3.0, rng.randf_range(-0.4, 0.4), rng.randf_range(-0.8, 0.8)))
			puff.scale.y = 0.55
			cloud.add_child(puff)
		add_child(cloud)
		_clouds.append(cloud)


# ------------------------------------------------------------- dynamic state

func apply_state(state: TirithState, first: bool) -> void:
	_apply_districts(state, first)
	_apply_strips(state)
	_apply_heroes(state, first)
	_apply_caravans(state, first)
	# Steward, beacon and board.
	var steward: Dictionary = state.steward()
	if _steward_bubble != null:
		_steward_bubble.text = str(steward.get("activity", ""))
	if _steward_name != null and str(steward.get("name", "")) != "":
		_steward_name.text = "%s\n%s" % [str(steward.get("name")), str(steward.get("title", "The Steward"))]
	if _board_body != null:
		_board_body.set_meta("payload", steward)
	var waiting: Array = state.waiting()
	_beacon_on = waiting.size() > 0
	if _beacon_body != null:
		_beacon_body.set_meta("payload", {"waiting": waiting})
	if _scroll_label != null:
		_scroll_label.text = "Waiting for you (%d)" % waiting.size() if _beacon_on else ""
		if not _beacon_on and _beacon_light != null:
			_beacon_light.light_energy = 0.0
			_beacon_mat.emission_energy_multiplier = 0.4


func _project_angle(index: int, count: int) -> float:
	# Spread districts around the ring, first one south-west so the default
	# camera (from the south) sees all of them across the frame.
	return 0.85 + TAU * float(index) / float(maxi(count, 1))


func _apply_districts(state: TirithState, first: bool) -> void:
	var seen: Dictionary = {}
	var projects: Array = state.projects()
	for idx: int in range(projects.size()):
		var project: Dictionary = projects[idx]
		var pid: String = str(project.get("id", "p%d" % idx))
		seen[pid] = true
		var ang: float = _project_angle(idx, projects.size())
		var center: Vector3 = Vector3(sin(ang) * DISTRICT_R, T2_Y, cos(ang) * DISTRICT_R)
		_district_centers[pid] = center
		var sig: String = JSON.stringify(project.get("architecture", {})) + str(project.get("color", ""))
		if _district_nodes.has(pid) and str(_district_sigs.get(pid, "")) == sig:
			# Move smoothly if the ring layout shifted (project added/removed).
			var existing: Node3D = _district_nodes[pid]
			if existing.position.distance_to(center) > 0.1:
				var tw: Tween = create_tween()
				tw.tween_property(existing, "position", center, 0.8).set_trans(Tween.TRANS_SINE)
			continue
		if _district_nodes.has(pid):
			_remove_district(pid)
		_district_sigs[pid] = sig
		var node: Node3D = _build_district(project, center, ang)
		_district_nodes[pid] = node
		if not first:
			node.scale = Vector3.ONE * 0.05
			var tw2: Tween = create_tween()
			tw2.tween_property(node, "scale", Vector3.ONE, 0.7).set_trans(Tween.TRANS_BACK).set_ease(Tween.EASE_OUT)
	for pid: String in _district_nodes.keys().duplicate():
		if not seen.has(pid):
			_remove_district(pid)
			_district_nodes.erase(pid)
			_district_sigs.erase(pid)
			_district_centers.erase(pid)


func _remove_district(pid: String) -> void:
	var node: Node3D = _district_nodes.get(pid)
	if node == null:
		return
	# Its road motes die with it.
	_motes = _motes.filter(func(m: Dictionary) -> bool: return str(m.get("project", "")) != pid)
	_banners = _banners.filter(func(b: Node3D) -> bool: return is_instance_valid(b) and b.get_parent() != node)
	node.queue_free()


func _build_district(project: Dictionary, center: Vector3, ang: float) -> Node3D:
	var root: Node3D = Node3D.new()
	root.name = "District_" + str(project.get("id", "?"))
	root.position = center
	root.add_to_group("districts")
	add_child(root)
	var color: Color = TirithState.color_of(project, Color(0.5, 0.5, 0.9))
	# Cobbled plaza with a rim and star medallion in the project colour.
	root.add_child(Mats.mesh_cyl(6.4, 0.22, Mats.road(), Vector3(0, 0.11, 0), 36))
	root.add_child(Mats.mesh_cyl(6.55, 0.10, Mats.solid(color.darkened(0.15)), Vector3(0, 0.05, 0), 36))
	var medallion: MeshInstance3D = Mats.star(1.15,
		Mats.emissive(color.lightened(0.1), 0.7), Vector3(0, 0.24, 0))
	medallion.rotation.x = -PI / 2.0
	root.add_child(medallion)
	# Rim lanterns so the plaza reads as a place, even between buildings.
	for li: int in range(4):
		var lang: float = ang + 0.8 + float(li) * TAU / 4.0
		var lamp: Node3D = Node3D.new()
		lamp.position = Vector3(sin(lang) * 5.9, 0.22, cos(lang) * 5.9)
		lamp.add_child(Mats.mesh_cyl(0.06, 1.7, Mats.wood_dark(), Vector3(0, 0.85, 0), 8))
		lamp.add_child(Mats.mesh_sphere(0.14, Mats.emissive(Color(1.0, 0.74, 0.36), 1.9), Vector3(0, 1.82, 0)))
		lamp.add_child(Mats.mesh_cone(0.24, 0.22, Mats.stone_dark(), Vector3(0, 2.02, 0), 8))
		root.add_child(lamp)
	_build_market_stall(root, color, ang + 2.35)
	# Banner pole on the outer edge, name above it.
	var pole_pos: Vector3 = Vector3(sin(ang + PI), 0, cos(ang + PI)) * 5.6
	var pole: Node3D = Node3D.new()
	pole.position = pole_pos
	root.add_child(pole)
	pole.add_child(Mats.mesh_cyl(0.09, 5.4, Mats.wood_dark(), Vector3(0, 2.7, 0), 8))
	pole.add_child(Mats.mesh_sphere(0.16, Mats.gold(), Vector3(0, 5.5, 0)))
	var banner: Node3D = Mats.banner(1.5, 2.6, color)
	banner.position = Vector3(0, 5.3, 0.12)
	pole.add_child(banner)
	_banners.append(banner)
	var name_label: Label3D = Mats.label3d(str(project.get("name", "?")), pole_pos + Vector3(0, 9.6, 0),
		color.lightened(0.35), 60, 0.008)
	name_label.add_to_group("district_labels")
	root.add_child(name_label)
	Mats.make_clickable(root, Vector3(2.0, 6.0, 2.0), pole_pos + Vector3(0, 3.0, 0),
		"district", project)
	# Architecture buildings and roads, or an empty plot.
	var arch: Dictionary = project.get("architecture", {}) as Dictionary
	var nodes: Array = arch.get("nodes", []) as Array
	if nodes.is_empty():
		_build_empty_plot(root, project)
		return root
	var positions: Dictionary = {}
	var ring_r: float = clampf(1.6 + float(nodes.size()) * 0.42, 2.2, 3.9)
	for i: int in range(nodes.size()):
		var nd: Dictionary = nodes[i]
		var bang: float = TAU * float(i) / float(nodes.size()) + 0.4
		var bp: Vector3 = Vector3(sin(bang) * ring_r, 0.22, cos(bang) * ring_r) if nodes.size() > 1 else Vector3(0, 0.22, 0)
		positions[str(nd.get("id", ""))] = bp
		var building: Node3D = _build_building(nd, color)
		building.position = bp
		building.rotation.y = bang + PI  # face the plaza centre
		root.add_child(building)
	for e: Variant in (arch.get("edges", []) as Array):
		var edge: Dictionary = e
		var a: Variant = positions.get(str(edge.get("from", "")))
		var b: Variant = positions.get(str(edge.get("to", "")))
		if a == null or b == null:
			continue
		_build_road(root, a as Vector3, b as Vector3, color, str(project.get("id", "")))
	return root


## A striped market stall with goods, so every district feels inhabited.
func _build_market_stall(root: Node3D, color: Color, ang: float) -> void:
	var stall: Node3D = Node3D.new()
	stall.name = "MarketStall"
	stall.position = Vector3(sin(ang) * 5.1, 0.22, cos(ang) * 5.1)
	stall.rotation.y = ang + PI
	root.add_child(stall)
	for sx: float in [-0.85, 0.85]:
		for sz: float in [-0.5, 0.5]:
			stall.add_child(Mats.mesh_cyl(0.05, 1.9, Mats.wood_dark(), Vector3(sx, 0.95, sz), 6))
	var canopy_a: MeshInstance3D = Mats.mesh_box(Vector3(1.0, 0.06, 1.5),
		Mats.solid(color, 0.85), Vector3(-0.48, 2.02, 0))
	canopy_a.rotation.z = 0.16
	stall.add_child(canopy_a)
	var canopy_b: MeshInstance3D = Mats.mesh_box(Vector3(1.0, 0.06, 1.5),
		Mats.solid(Color(0.96, 0.93, 0.86), 0.85), Vector3(0.48, 2.02, 0))
	canopy_b.rotation.z = -0.16
	stall.add_child(canopy_b)
	stall.add_child(Mats.mesh_box(Vector3(1.7, 0.08, 1.0), Mats.wood(), Vector3(0, 0.92, 0)))
	stall.add_child(Mats.mesh_box(Vector3(0.5, 0.3, 0.5), Mats.wood(), Vector3(-0.45, 1.12, 0)))
	stall.add_child(Mats.mesh_sphere(0.16, Mats.solid(Color(0.9, 0.35, 0.2)), Vector3(-0.45, 1.34, 0)))
	stall.add_child(Mats.mesh_sphere(0.14, Mats.solid(Color(1.0, 0.75, 0.25)), Vector3(-0.25, 1.32, 0.15)))
	stall.add_child(Mats.mesh_box(Vector3(0.55, 0.35, 0.4),
		Mats.solid(color.lightened(0.2), 0.8), Vector3(0.5, 1.14, -0.1)))
	stall.add_child(Mats.mesh_cyl(0.24, 0.6, Mats.wood_dark(), Vector3(1.15, 0.3, 0.3), 10))
	stall.add_child(Mats.mesh_box(Vector3(0.6, 0.6, 0.6), Mats.wood(), Vector3(-1.2, 0.3, -0.2)))
	stall.add_child(Mats.mesh_box(Vector3(0.45, 0.45, 0.45), Mats.wood(), Vector3(-1.15, 0.83, -0.15)))


func _build_empty_plot(root: Node3D, project: Dictionary) -> void:
	for i: int in range(8):
		var ang: float = TAU * float(i) / 8.0
		root.add_child(Mats.mesh_box(Vector3(0.14, 0.8, 0.14), Mats.wood_dark(),
			Vector3(sin(ang) * 3.6, 0.6, cos(ang) * 3.6)))
	var sign: Node3D = Node3D.new()
	sign.position = Vector3(0, 0.22, 1.8)
	root.add_child(sign)
	sign.add_child(Mats.mesh_box(Vector3(0.14, 1.5, 0.14), Mats.wood_dark(), Vector3(0, 0.75, 0)))
	sign.add_child(Mats.mesh_box(Vector3(2.2, 0.8, 0.09), Mats.wood(), Vector3(0, 1.45, 0)))
	sign.add_child(Mats.label3d("Not mapped yet", Vector3(0, 1.5, 0.25), Color(1.0, 0.93, 0.75), 48, 0.008))
	Mats.make_clickable(sign, Vector3(2.4, 2.0, 0.8), Vector3(0, 1.1, 0), "district", project)


## One architecture node as a small building; silhouette per `kind`, roof and
## banner in the project colour, size hinting at the file count.
func _build_building(nd: Dictionary, color: Color) -> Node3D:
	var b: Node3D = Node3D.new()
	b.name = "Building_" + str(nd.get("id", "?"))
	b.add_to_group("buildings")
	var kind: String = str(nd.get("kind", "service"))
	var files: int = int(nd.get("files", 1))
	var s: float = clampf(0.78 + float(files) * 0.012, 0.78, 1.12)
	b.scale = Vector3.ONE * s
	var roof: StandardMaterial3D = Mats.roof(color.darkened(0.08))
	var glow_win: StandardMaterial3D = Mats.emissive(Color(1.0, 0.75, 0.38), 1.9)
	b.add_child(Mats.mesh_box(Vector3(2.1, 0.28, 2.1), Mats.stone_dark(), Vector3(0, 0.14, 0)))
	var top_y: float = 2.2
	match kind:
		"input":  # a gate: two pillars, a lintel, the star and a lantern
			for sx: float in [-0.75, 0.75]:
				b.add_child(Mats.mesh_box(Vector3(0.42, 2.1, 0.5), Mats.stone(), Vector3(sx, 1.3, 0)))
				b.add_child(Mats.mesh_box(Vector3(0.56, 0.22, 0.62), Mats.stone_dark(), Vector3(sx, 2.42, 0)))
				b.add_child(Mats.mesh_cone(0.40, 0.62, roof, Vector3(sx, 2.82, 0), 8))
				b.add_child(Mats.mesh_box(Vector3(0.30, 0.45, 0.08), glow_win, Vector3(sx, 1.55, 0.27)))
			b.add_child(Mats.mesh_box(Vector3(2.1, 0.4, 0.55), Mats.stone(), Vector3(0, 2.3, 0)))
			b.add_child(Mats.star(0.30, Mats.gold(), Vector3(0, 2.3, 0.32)))
			b.add_child(Mats.mesh_sphere(0.11, Mats.emissive(Color(1.0, 0.74, 0.36), 2.0), Vector3(0, 1.85, 0)))
			b.add_child(Mats.mesh_box(Vector3(1.6, 0.14, 0.9), Mats.stone_dark(), Vector3(0, 0.33, 0.35)))
			top_y = 3.3
		"core":  # a tower with buttresses and a glowing crystal
			b.add_child(Mats.mesh_cyl(0.85, 3.0, Mats.stone(), Vector3(0, 1.75, 0), 14))
			for fi: int in range(4):
				var fang: float = PI / 4.0 + TAU * float(fi) / 4.0
				var fin: MeshInstance3D = Mats.mesh_box(Vector3(0.20, 2.4, 0.42),
					Mats.stone_dark(), Vector3(sin(fang) * 0.86, 1.45, cos(fang) * 0.86))
				fin.rotation.y = fang
				b.add_child(fin)
			b.add_child(Mats.mesh_cyl(1.02, 0.3, roof, Vector3(0, 3.35, 0), 14))
			var crystal: MeshInstance3D = Mats.mesh_cone(0.46, 1.25,
				Mats.emissive(color.lightened(0.2), 3.2), Vector3(0, 4.2, 0), 6)
			b.add_child(crystal)
			var shard: MeshInstance3D = Mats.mesh_cone(0.20, 0.55,
				Mats.emissive(color.lightened(0.35), 2.4), Vector3(0.55, 3.85, 0.3), 6)
			shard.rotation.z = -0.5
			b.add_child(shard)
			b.add_child(Mats.mesh_box(Vector3(0.3, 0.5, 0.08), glow_win, Vector3(0, 2.2, 0.84)))
			b.add_child(Mats.mesh_box(Vector3(0.3, 0.5, 0.08), glow_win, Vector3(0, 1.2, 0.84)))
			top_y = 5.1
		"data":  # a vault with a silver dome, gold band and a heavy door
			b.add_child(Mats.mesh_box(Vector3(1.9, 1.3, 1.9), Mats.stone(), Vector3(0, 0.93, 0)))
			b.add_child(Mats.mesh_cyl(1.12, 0.18, Mats.gold(), Vector3(0, 1.62, 0), 20))
			var dome: MeshInstance3D = Mats.mesh_sphere(1.05, Mats.silver(), Vector3(0, 1.68, 0))
			dome.scale.y = 0.72
			b.add_child(dome)
			b.add_child(Mats.mesh_sphere(0.13, Mats.gold(), Vector3(0, 2.52, 0)))
			for sx: float in [-0.72, 0.72]:
				b.add_child(Mats.mesh_cyl(0.11, 1.25, Mats.stone_dark(), Vector3(sx, 0.9, 1.0), 8))
			b.add_child(Mats.mesh_box(Vector3(1.7, 0.18, 0.5), Mats.stone_dark(), Vector3(0, 1.58, 1.0)))
			b.add_child(Mats.mesh_box(Vector3(0.68, 1.05, 0.10), Mats.wood_dark(), Vector3(0, 0.82, 0.97)))
			var handle: MeshInstance3D = Mats.mesh_torus(0.07, 0.13, Mats.gold(), Vector3(0.18, 0.85, 1.05))
			handle.rotation.x = PI / 2.0
			b.add_child(handle)
			b.add_child(Mats.mesh_box(Vector3(1.3, 0.14, 0.7), Mats.stone_dark(), Vector3(0, 0.33, 1.15)))
			top_y = 3.0
		"ui":  # a library hall: columns, gabled roof, open-book sign, banner
			b.add_child(Mats.mesh_box(Vector3(2.7, 1.7, 1.9), Mats.stone(), Vector3(0, 1.13, 0)))
			b.add_child(Mats.mesh_box(Vector3(3.0, 0.26, 2.2), Mats.stone_dark(), Vector3(0, 2.10, 0)))
			var gable: MeshInstance3D = Mats.mesh_cone(2.15, 0.95, roof, Vector3(0, 2.70, 0), 4)
			gable.rotation.y = PI / 4.0
			gable.scale = Vector3(1.05, 1.0, 0.78)
			b.add_child(gable)
			for sx: float in [-1.15, -0.38, 0.38, 1.15]:
				b.add_child(Mats.mesh_cyl(0.13, 1.7, Mats.stone_dark(), Vector3(sx, 1.13, 1.08), 8))
				b.add_child(Mats.mesh_box(Vector3(0.30, 0.14, 0.30), Mats.stone(), Vector3(sx, 2.02, 1.08)))
			b.add_child(Mats.mesh_box(Vector3(0.55, 0.7, 0.08), glow_win, Vector3(-0.85, 1.25, 0.97)))
			b.add_child(Mats.mesh_box(Vector3(0.55, 0.7, 0.08), glow_win, Vector3(0.85, 1.25, 0.97)))
			var book_l: MeshInstance3D = Mats.mesh_box(Vector3(0.42, 0.06, 0.34),
				Mats.solid(Color(0.97, 0.94, 0.86)), Vector3(-0.20, 2.42, 1.14))
			book_l.rotation.z = 0.28
			b.add_child(book_l)
			var book_r: MeshInstance3D = Mats.mesh_box(Vector3(0.42, 0.06, 0.34),
				Mats.solid(Color(0.97, 0.94, 0.86)), Vector3(0.20, 2.42, 1.14))
			book_r.rotation.z = -0.28
			b.add_child(book_r)
			var hb: Node3D = Mats.banner(0.55, 1.0, color)
			hb.position = Vector3(0, 2.0, 1.12)
			b.add_child(hb)
			_banners.append(hb)
			top_y = 3.3
		_:  # service and anything unknown: a forge with chimney and anvil
			b.add_child(Mats.mesh_box(Vector3(1.85, 1.45, 1.85), Mats.stone(), Vector3(0, 1.0, 0)))
			var pyramid: MeshInstance3D = Mats.mesh_cone(1.55, 0.95, roof, Vector3(0, 2.2, 0), 4)
			pyramid.rotation.y = PI / 4.0
			b.add_child(pyramid)
			b.add_child(Mats.mesh_box(Vector3(0.30, 1.0, 0.30), Mats.stone_dark(), Vector3(0.55, 2.55, -0.4)))
			b.add_child(Mats.mesh_box(Vector3(0.42, 0.14, 0.42), Mats.stone_dark(), Vector3(0.55, 3.08, -0.4)))
			var smoke_mat: StandardMaterial3D = Mats.transparent(Color(0.82, 0.82, 0.86), 0.38)
			b.add_child(Mats.mesh_sphere(0.17, smoke_mat, Vector3(0.55, 3.35, -0.4)))
			b.add_child(Mats.mesh_sphere(0.24, smoke_mat, Vector3(0.68, 3.65, -0.45)))
			b.add_child(Mats.mesh_sphere(0.32, smoke_mat, Vector3(0.88, 4.0, -0.5)))
			b.add_child(Mats.mesh_box(Vector3(0.6, 1.0, 0.08), Mats.wood_dark(), Vector3(-0.42, 0.8, 0.94)))
			b.add_child(Mats.mesh_box(Vector3(0.5, 0.5, 0.08),
				Mats.emissive(Color(1.0, 0.52, 0.16), 2.4), Vector3(0.48, 1.05, 0.94)))
			var awning: MeshInstance3D = Mats.mesh_box(Vector3(1.5, 0.06, 0.7),
				Mats.solid(color.darkened(0.1), 0.85), Vector3(0, 1.62, 1.18))
			awning.rotation.x = -0.3
			b.add_child(awning)
			b.add_child(Mats.mesh_box(Vector3(0.22, 0.4, 0.22), Mats.wood_dark(), Vector3(0.75, 0.48, 1.25)))
			b.add_child(Mats.mesh_box(Vector3(0.62, 0.16, 0.24), Mats.silver(), Vector3(0.75, 0.75, 1.25)))
			top_y = 3.2
	b.add_child(Mats.label3d(str(nd.get("label", "?")), Vector3(0, top_y + 0.55, 0),
		Color(1.0, 0.97, 0.9), 38, 0.0052, 12))
	Mats.make_clickable(b, Vector3(2.6, top_y + 0.6, 2.6), Vector3(0, top_y * 0.5, 0),
		"building", nd)
	return b


func _build_road(root: Node3D, a: Vector3, b: Vector3, color: Color, pid: String) -> void:
	var mid: Vector3 = (a + b) / 2.0
	var length: float = a.distance_to(b)
	var road_seg: MeshInstance3D = Mats.mesh_box(Vector3(0.55, 0.07, length), Mats.road(),
		Vector3(mid.x, 0.26, mid.z))
	road_seg.rotation.y = atan2(b.x - a.x, b.z - a.z)
	root.add_child(road_seg)
	# Light motes flowing from a to b (the edge direction).
	var mote_mat: StandardMaterial3D = Mats.emissive(color.lightened(0.55), 2.2)
	for i: int in range(3):
		var mote: MeshInstance3D = Mats.mesh_sphere(0.09, mote_mat)
		root.add_child(mote)
		_motes.append({"node": mote, "from": Vector3(a.x, 0.55, a.z),
			"to": Vector3(b.x, 0.55, b.z), "speed": 0.16 + 0.03 * float(i),
			"off": float(i) / 3.0, "project": pid})


func _hero_spot(state: TirithState, agent: Dictionary) -> Array:
	# Returns [global position, yaw]: heroes stand on the plaza rim facing in.
	var pid: String = str(agent.get("project", ""))
	var center: Vector3 = district_center(pid)
	var siblings: Array = state.agents_of_project(pid)
	var idx: int = 0
	for i: int in range(siblings.size()):
		if str((siblings[i] as Dictionary).get("id", "")) == str(agent.get("id", "")):
			idx = i
			break
	var base_ang: float = atan2(center.x, center.z)
	var spread: float = 1.25
	var ang: float = base_ang + (float(idx) - float(siblings.size() - 1) / 2.0) * spread
	var spot_r: float = 4.5 - 0.35 * float(idx % 2)
	var pos: Vector3 = center + Vector3(sin(ang) * spot_r, 0.22, cos(ang) * spot_r)
	# Face outward, bench in front: heroes read from the usual outside view.
	var yaw: float = atan2(pos.x - center.x, pos.z - center.z)
	return [pos, yaw]


func _apply_heroes(state: TirithState, first: bool) -> void:
	var seen: Dictionary = {}
	for a: Variant in state.agents():
		var agent: Dictionary = a
		var aid: String = str(agent.get("id", ""))
		if aid == "" or not _district_centers.has(str(agent.get("project", ""))):
			continue
		seen[aid] = true
		var spot: Array = _hero_spot(state, agent)
		var pos: Vector3 = spot[0]
		var yaw: float = float(spot[1])
		if _heroes.has(aid):
			var hero: Hero = _heroes[aid]
			hero.update_agent(agent)
			if not hero.walking and hero.global_position.distance_to(pos) > 0.6:
				hero.walk_to(pos, yaw, 1.2)
			continue
		var new_hero: Hero = Hero.new()
		new_hero.name = "Hero_" + aid
		add_child(new_hero)
		new_hero.setup(agent)
		new_hero.plate_lift = float(state.agents_of_project(str(agent.get("project", ""))).find(agent) % 3) * 1.7
		_heroes[aid] = new_hero
		if first:
			new_hero.global_position = pos
			new_hero.rotation.y = yaw
		else:
			# Walk in from the district gate side.
			var center: Vector3 = district_center(str(agent.get("project", "")))
			var gate: Vector3 = center + (pos - center).normalized() * 8.5 + Vector3(0, 0.22, 0)
			gate.y = pos.y
			new_hero.global_position = gate
			new_hero.walk_to(pos, yaw, 1.8)
	for aid: String in _heroes.keys().duplicate():
		if not seen.has(aid):
			var hero: Hero = _heroes[aid]
			_heroes.erase(aid)
			var tw: Tween = create_tween()
			tw.tween_property(hero, "scale", Vector3.ONE * 0.05, 0.5)
			tw.tween_callback(hero.queue_free)


func _caravan_arc(index_angle: float, stages: int) -> Array:
	# [start angle, end angle]: the station strip sits on the outer ring right
	# next to its project district, centred on the district angle.
	var span: float = clampf(0.085 * float(maxi(stages - 1, 1)), 0.2, 1.0)
	var start: float = index_angle - span / 2.0
	return [start, start + span]


func _stage_angle(arc: Array, i: int, stages: int) -> float:
	if stages <= 1:
		return float(arc[0])
	return lerpf(float(arc[0]), float(arc[1]), float(i) / float(stages - 1))


func _ring_pos(ang: float, radius: float, y: float) -> Vector3:
	return Vector3(sin(ang) * radius, y, cos(ang) * radius)


func _current_stage_index(pipeline: Dictionary) -> int:
	var stages: Array = pipeline.get("stages", []) as Array
	var last_done: int = 0
	for i: int in range(stages.size()):
		var st: String = str((stages[i] as Dictionary).get("status", ""))
		if st == "running" or st == "failed":
			return i
		if st == "passed":
			last_done = i
	return last_done


func _apply_caravans(state: TirithState, _first: bool) -> void:
	var seen: Dictionary = {}
	var pipelines: Array = state.pipelines()
	for idx: int in range(pipelines.size()):
		var pipeline: Dictionary = pipelines[idx]
		var run: String = str(pipeline.get("run", "run%d" % idx))
		seen[run] = true
		var stages: Array = pipeline.get("stages", []) as Array
		var arc: Array = _caravan_arc(_pipeline_angle(state, pipeline, idx), stages.size())
		if _caravans.has(run):
			var parts: Dictionary = _caravans[run]
			if int(parts.get("stage_count", -1)) != stages.size():
				(parts["root"] as Node).queue_free()
				_caravans.erase(run)
			else:
				_update_caravan(parts, pipeline, arc)
				continue
		_caravans[run] = _build_caravan(state, pipeline, arc)
	for run: String in _caravans.keys().duplicate():
		if not seen.has(run):
			((_caravans[run] as Dictionary)["root"] as Node).queue_free()
			_caravans.erase(run)


func _build_caravan(state: TirithState, pipeline: Dictionary, arc: Array) -> Dictionary:
	var root: Node3D = Node3D.new()
	root.name = "Caravan_" + str(pipeline.get("run", "?")).replace("#", "_")
	root.add_to_group("caravans")
	add_child(root)
	var project: Dictionary = state.project_by_id(str(pipeline.get("project", "")))
	var color: Color = TirithState.color_of(project, Color(0.6, 0.6, 0.6))
	var stages: Array = pipeline.get("stages", []) as Array
	var station_parts: Array = []
	for i: int in range(stages.size()):
		var stage: Dictionary = stages[i]
		var ang: float = _stage_angle(arc, i, stages.size())
		var st_root: Node3D = Node3D.new()
		st_root.position = _ring_pos(ang, CARAVAN_R + 3.0, 0)
		st_root.add_to_group("stations")
		root.add_child(st_root)
		st_root.add_child(Mats.mesh_cyl(0.09, 2.4, Mats.wood_dark(), Vector3(0, 1.2, 0), 8))
		var mat: StandardMaterial3D = Mats.emissive(TirithState.status_color(str(stage.get("status", "pending"))).darkened(0.2), 1.15)
		st_root.add_child(Mats.mesh_sphere(0.38, mat, Vector3(0, 2.5, 0)))
		st_root.add_child(Mats.mesh_cyl(0.85, 0.07, mat, Vector3(0, 0.05, 0), 20))
		st_root.add_child(Mats.mesh_cone(0.45, 0.4, Mats.stone_dark(), Vector3(0, 3.05, 0), 10))
		var st_label: Label3D = Mats.label3d(str(stage.get("name", "?")), Vector3(0, 3.75, 0),
			Color(1.0, 0.97, 0.9), 46, 0.008, 14)
		st_label.add_to_group("district_labels")
		st_root.add_child(st_label)
		var payload: Dictionary = pipeline.duplicate()
		payload["_stage"] = stage
		Mats.make_clickable(st_root, Vector3(1.4, 3.4, 1.4), Vector3(0, 1.7, 0), "caravan", payload)
		station_parts.append({"mat": mat,
			"target": TirithState.status_color(str(stage.get("status", "pending"))).darkened(0.2)})
	var cart: Node3D = _build_cart(color)
	root.add_child(cart)
	var start_label: Label3D = Mats.label3d(str(pipeline.get("run", "")),
		_ring_pos(float(arc[0]), CARAVAN_R + 3.0, 4.6), color.lightened(0.4), 48, 0.008)
	root.add_child(start_label)
	var parts: Dictionary = {"root": root, "cart": cart, "stations": station_parts,
		"stage_count": stages.size(), "angle": 0.0, "target_angle": 0.0}
	Mats.make_clickable(cart, Vector3(2.4, 2.2, 3.4), Vector3(0, 1.0, 0), "caravan", pipeline)
	_update_caravan(parts, pipeline, arc, true)
	return parts


func _build_cart(color: Color) -> Node3D:
	var cart: Node3D = Node3D.new()
	cart.name = "Cart"
	cart.add_to_group("carts")
	cart.add_child(Mats.mesh_box(Vector3(1.5, 0.55, 2.5), Mats.wood(), Vector3(0, 0.85, 0)))
	cart.add_child(Mats.mesh_box(Vector3(1.6, 0.18, 2.6), Mats.wood_dark(), Vector3(0, 0.58, 0)))
	for sx: float in [-0.78, 0.78]:
		for sz: float in [-0.85, 0.85]:
			var wheel: MeshInstance3D = Mats.mesh_cyl(0.42, 0.12, Mats.wood_dark(), Vector3(sx, 0.42, sz), 14)
			wheel.rotation.z = PI / 2.0
			cart.add_child(wheel)
	cart.add_child(Mats.mesh_box(Vector3(1.0, 0.7, 1.0), Mats.solid(color, 0.8), Vector3(0, 1.5, -0.5)))
	cart.add_child(Mats.mesh_box(Vector3(0.7, 0.5, 0.7), Mats.solid(Color(0.9, 0.85, 0.72)), Vector3(0.15, 1.4, 0.65)))
	var canopy: MeshInstance3D = Mats.mesh_box(Vector3(1.55, 0.09, 1.6), Mats.solid(color.lightened(0.15)), Vector3(0, 2.15, -0.4))
	cart.add_child(canopy)
	for sx: float in [-0.7, 0.7]:
		for sz: float in [-1.1, 0.3]:
			cart.add_child(Mats.mesh_cyl(0.045, 1.05, Mats.wood_dark(), Vector3(sx, 1.65, sz), 6))
	cart.add_child(Mats.mesh_sphere(0.11, Mats.emissive(Color(1.0, 0.72, 0.32), 1.8), Vector3(0, 1.15, 1.35)))
	var banner: Node3D = Mats.banner(0.4, 0.8, color)
	banner.position = Vector3(0.6, 2.1, 0.3)
	cart.add_child(banner)
	_banners.append(banner)
	return cart


func _update_caravan(parts: Dictionary, pipeline: Dictionary, arc: Array, snap: bool = false) -> void:
	var stages: Array = pipeline.get("stages", []) as Array
	var station_parts: Array = parts["stations"]
	for i: int in range(mini(stages.size(), station_parts.size())):
		(station_parts[i] as Dictionary)["target"] = TirithState.status_color(
			str((stages[i] as Dictionary).get("status", "pending"))).darkened(0.2)
	var cur: int = _current_stage_index(pipeline)
	parts["target_angle"] = _stage_angle(arc, cur, stages.size())
	if snap:
		parts["angle"] = parts["target_angle"]
		_place_cart(parts)
	# Refresh click payloads with the newest pipeline data.
	var cart: Node3D = parts["cart"]
	for child: Node in cart.get_children():
		if child is StaticBody3D:
			child.set_meta("payload", pipeline)


func _place_cart(parts: Dictionary) -> void:
	var cart: Node3D = parts["cart"]
	var ang: float = float(parts["angle"])
	cart.position = _ring_pos(ang, CARAVAN_R, 0.1)
	cart.rotation.y = ang + PI / 2.0  # tangent to the ring, nose along +angle


func _process(delta: float) -> void:
	_t += delta
	var cam: Camera3D = get_viewport().get_camera_3d()
	if cam != null:
		for sv: Variant in _strips.values():
			var sn: Node3D = (sv as Dictionary)["node"]
			if is_instance_valid(sn):
				sn.global_rotation = cam.global_rotation
				sn.scale = Vector3.ONE * (clampf(cam.size * 0.04, 0.9, 3.0) if cam.projection == Camera3D.PROJECTION_ORTHOGONAL else 1.4)
		var ortho: bool = cam.projection == Camera3D.PROJECTION_ORTHOGONAL
		for dl: Node in get_tree().get_nodes_in_group("district_labels"):
			(dl as Node3D).scale = Vector3.ONE * (clampf(cam.size * 0.034, 0.6, 2.4) if ortho else 1.0)
		for cl: Variant in _clouds:
			(cl as Node3D).visible = cam.projection != Camera3D.PROJECTION_ORTHOGONAL
	for b: Variant in _banners:
		if is_instance_valid(b):
			var banner: Node3D = b
			banner.rotation.z = sin(_t * 1.4 + float(banner.get_instance_id() % 17)) * 0.07
			banner.rotation.x = sin(_t * 1.1 + float(banner.get_instance_id() % 11)) * 0.05
	if _star_top != null:
		_star_top.rotation.y = _t * 0.25
	if _beacon_on and _beacon_light != null:
		var pulse: float = 0.72 + 0.28 * sin(_t * 2.6)
		_beacon_light.light_energy = 3.2 * pulse
		_beacon_mat.emission_energy_multiplier = 3.4 * pulse
	for m: Variant in _motes:
		var mote: Dictionary = m
		var node: Node3D = mote["node"]
		if not is_instance_valid(node):
			continue
		var f: float = wrapf(_t * float(mote["speed"]) + float(mote["off"]), 0.0, 1.0)
		node.position = (mote["from"] as Vector3).lerp(mote["to"] as Vector3, f)
		node.position.y += sin(f * PI) * 0.25
	for c: Variant in _clouds:
		var cloud: Node3D = c
		cloud.position.x += delta * 0.7
		if cloud.position.x > 120.0:
			cloud.position.x = -120.0
	for parts: Variant in _caravans.values():
		var p: Dictionary = parts
		var diff: float = wrapf(float(p["target_angle"]) - float(p["angle"]), -PI, PI)
		if absf(diff) > 0.0005:
			p["angle"] = float(p["angle"]) + diff * minf(delta * 1.6, 1.0)
			_place_cart(p)
		for sp: Variant in (p["stations"] as Array):
			var mat: StandardMaterial3D = (sp as Dictionary)["mat"]
			var target: Color = (sp as Dictionary)["target"]
			mat.albedo_color = mat.albedo_color.lerp(target, minf(delta * 5.0, 1.0))
			mat.emission = mat.albedo_color








# ---------------------------------------------------- district pipeline strips

var _strips: Dictionary = {}  # project id -> {"node": Node3D, "sig": String}


## A compact pipeline strip floating under each district banner: one row per
## pipeline run of that project, one tile per stage in its status colour.
func _apply_strips(state: TirithState) -> void:
	for pid: Variant in _district_nodes.keys():
		var runs: Array = []
		for p: Variant in state.pipelines():
			if str((p as Dictionary).get("project", "")) == str(pid):
				runs.append(p)
		var sig: String = ""
		for r: Variant in runs:
			sig += str((r as Dictionary).get("run", "")) + ":"
			for s: Variant in ((r as Dictionary).get("stages", []) as Array):
				sig += str((s as Dictionary).get("status", "-")).left(1) + ","
		var cur: Dictionary = _strips.get(pid, {})
		var district: Node3D = _district_nodes[pid]
		if not cur.is_empty() and cur["sig"] == sig and is_instance_valid(cur["node"]) \
				and (cur["node"] as Node).get_parent() == district:
			continue
		if not cur.is_empty() and is_instance_valid(cur["node"]):
			(cur["node"] as Node).queue_free()
		if runs.is_empty():
			_strips.erase(pid)
			continue
		var strip: Node3D = Node3D.new()
		strip.name = "PipelineStrip"
		var ang: float = atan2(district.position.x, district.position.z) + PI
		strip.position = Vector3(sin(ang), 0, cos(ang)) * 5.6 + Vector3(0, 8.4, 0)
		district.add_child(strip)
		var row_y: float = 0.0
		for r: Variant in runs:
			var stages: Array = (r as Dictionary).get("stages", []) as Array
			var w: float = float(stages.size()) * 0.52 + 0.3
			strip.add_child(Mats.mesh_quad(Vector2(w + 0.12, 0.62), Mats.transparent(Color(0.86, 0.68, 0.30), 0.95),
				Vector3(0, row_y, -0.02)))
			strip.add_child(Mats.mesh_quad(Vector2(w, 0.52), Mats.transparent(Color(0.05, 0.06, 0.1), 0.88),
				Vector3(0, row_y, -0.01)))
			for i: int in range(stages.size()):
				var st: String = str((stages[i] as Dictionary).get("status", "pending"))
				var tile: MeshInstance3D = Mats.mesh_quad(Vector2(0.4, 0.34),
					Mats.emissive(TirithState.status_color(st), 0.9 if st == "running" else 0.4),
					Vector3((float(i) - float(stages.size() - 1) / 2.0) * 0.52, row_y, 0.0))
				strip.add_child(tile)
			row_y += 0.72
		_strips[pid] = {"node": strip, "sig": sig}


## Ring angle for a pipeline: its project's district angle, nudged sideways
## when one project has several runs so their stations do not overlap.
func _pipeline_angle(state: TirithState, pipeline: Dictionary, fallback_idx: int) -> float:
	var pid: String = str(pipeline.get("project", ""))
	var projects: Array = state.projects()
	var pidx: int = -1
	for i: int in range(projects.size()):
		if str((projects[i] as Dictionary).get("id", "")) == pid:
			pidx = i
	if pidx < 0:
		return 0.85 + 0.5 * float(fallback_idx)
	var earlier: int = 0
	for k: int in range(fallback_idx):
		if str((state.pipelines()[k] as Dictionary).get("project", "")) == pid:
			earlier += 1
	return _project_angle(pidx, projects.size()) + float(earlier) * 0.45
