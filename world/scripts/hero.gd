class_name Hero
extends Node3D
## One agent as a citadel hero. Procedural placeholder built from primitives,
## distinct per class (silhouette, accent colour, gear) and rank (trim, stars,
## cape, aura). If `res://assets/characters/<class>.glb` exists it is loaded
## instead and its AnimationPlayer clips are mapped by name:
## idle, work, celebrate, fail, walk (substring match, case-insensitive).

const GLB_DIR: String = "res://assets/characters/"
## Heroic scale: heroes stand about 2.6 m against 2.2-4.2 m buildings,
## so a hero's head reaches the eaves like in a hero game.
const HERO_SCALE: float = 1.9

var agent: Dictionary = {}
var status: String = "working"
var walking: bool = false
var _t: float = 0.0
var _phase: float = 0.0
var _celebrate_left: float = 0.0
var _slump: float = 0.0
var _greyed: bool = false

var _body: Node3D
var _arm_l: Node3D
var _arm_r: Node3D
var _leg_l: Node3D
var _leg_r: Node3D
var _head: Node3D
var _orb_mat: StandardMaterial3D
var _orb: MeshInstance3D
var _plate: Label3D
var _plate_root: Node3D
var _plate_epithet: Label3D
var _plate_rank: Label3D
var _bar_fg: MeshInstance3D
var _bar_mat: StandardMaterial3D
var _ring_mat: StandardMaterial3D
var _sparks: GPUParticles3D
var _sparkles: GPUParticles3D
var _smoke: GPUParticles3D
var _scaffold: Node3D
var _bubble: Label3D
var _fail_light: OmniLight3D
var _mats: Array = []  # [StandardMaterial3D, original albedo] for the stalled grey-out
var _anim: AnimationPlayer = null
var _anim_map: Dictionary = {}
var _glb_root: Node3D = null  # set when an owner GLB replaces the placeholder
var _glb_base: Vector3 = Vector3.ZERO
var plate_lift: float = 0.0  # raises the nameplate so neighbours do not overlap


func setup(p_agent: Dictionary) -> void:
	agent = p_agent
	status = str(agent.get("status", "working"))
	_phase = float(abs(str(agent.get("id", "x")).hash()) % 100) / 10.0
	add_to_group("heroes")
	var class_id: String = str(agent.get("class", "stonewright"))
	if not _try_load_glb(class_id):
		_build_procedural(class_id)
	else:
		_build_glb_rank(class_id)
	_build_bench(class_id)
	_build_ring(class_id)
	_build_plate()
	_apply_status_visuals(true)
	# Applied last so GLB fitting above stays in unscaled local space.
	scale = Vector3.ONE * HERO_SCALE
	Mats.make_clickable(self, Vector3(1.4, 2.6, 1.8), Vector3(0, 1.2, 0.3),
		"hero", agent)


## Owner-supplied model wins over the placeholder when the file is present.
func _try_load_glb(class_id: String) -> bool:
	var path: String = GLB_DIR + class_id + ".glb"
	if not ResourceLoader.exists(path):
		return false
	var scene: PackedScene = load(path) as PackedScene
	if scene == null:
		return false
	var inst: Node = scene.instantiate()
	# Wrap in a holder: fitting and whole-body animation then never touch the
	# imported scene's own root transform (some GLBs carry one).
	var holder: Node3D = Node3D.new()
	holder.name = "GLBHolder"
	add_child(holder)
	holder.add_child(inst)
	_glb_root = holder
	_fit_glb(inst)
	_anim = _find_anim_player(inst)
	if _anim != null:
		for clip: String in _anim.get_animation_list():
			var low: String = clip.to_lower()
			for key: String in ["idle", "work", "celebrate", "fail", "walk"]:
				if low.contains(key) and not _anim_map.has(key):
					_anim_map[key] = clip
	_play_anim("idle")
	return true


## Normalise an owner GLB: feet on the ground, about 1.85 m tall, centred.
func _fit_glb(inst: Node) -> void:
	var aabb: AABB = AABB()
	var first: bool = true
	var stack: Array = [inst]
	var inv: Transform3D = global_transform.affine_inverse()
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		for child: Node in n.get_children():
			stack.append(child)
		if n is MeshInstance3D:
			var mi: MeshInstance3D = n
			var local: AABB = (inv * mi.global_transform) * mi.get_aabb()
			aabb = local if first else aabb.merge(local)
			first = false
			# Unique materials per hero so the stalled grey-out is per-agent.
			if mi.mesh != null:
				for si: int in range(mi.mesh.get_surface_count()):
					var src: Material = mi.get_surface_override_material(si)
					if src == null:
						src = mi.mesh.surface_get_material(si)
					if src is StandardMaterial3D:
						var dup: StandardMaterial3D = (src as StandardMaterial3D).duplicate()
						mi.set_surface_override_material(si, dup)
						_track(dup)
	if first or aabb.size.y <= 0.01:
		return
	var target_h: float = 1.85
	var factor: float = target_h / aabb.size.y
	_glb_root.scale = Vector3.ONE * factor
	_glb_root.position = Vector3(-aabb.get_center().x * factor,
		-aabb.position.y * factor, -aabb.get_center().z * factor)
	_glb_base = _glb_root.position


func _find_anim_player(node: Node) -> AnimationPlayer:
	if node is AnimationPlayer:
		return node
	for child: Node in node.get_children():
		var found: AnimationPlayer = _find_anim_player(child)
		if found != null:
			return found
	return null


func _play_anim(key: String) -> void:
	if _anim == null:
		return
	var clip: String = str(_anim_map.get(key, _anim_map.get("idle", "")))
	if clip != "" and _anim.current_animation != clip:
		_anim.play(clip)


func _track(mat: StandardMaterial3D) -> StandardMaterial3D:
	_mats.append([mat, mat.albedo_color])
	return mat


func _class_mat(class_id: String, light: float = 1.0) -> StandardMaterial3D:
	var c: Color = TirithState.class_color(class_id)
	if light > 1.0:
		c = c.lightened(minf(light - 1.0, 0.6))
	elif light < 1.0:
		c = c.darkened(1.0 - light)
	return _track(Mats.solid(c, 0.8))


func _build_procedural(class_id: String) -> void:
	_body = Node3D.new()
	_body.name = "Body"
	_body.position.y = 0.62  # pivot at the hips so a slump reads well
	add_child(_body)
	var broad: float = 1.25 if class_id == "stonewright" else (0.85 if class_id in ["swiftrider", "loremaster"] else 1.0)
	var tall: float = 1.12 if class_id == "steward" else 1.0
	var tunic: StandardMaterial3D = _class_mat(class_id, 0.85)
	var cloth: StandardMaterial3D = _track(Mats.solid(Color(0.88, 0.85, 0.78), 0.9))
	# Torso, belt, skirt of the tunic.
	_body.add_child(Mats.mesh_box(Vector3(0.60 * broad, 0.68 * tall, 0.36), tunic, Vector3(0, 0.38, 0)))
	_body.add_child(Mats.mesh_box(Vector3(0.62 * broad, 0.10, 0.38), _track(Mats.wood_dark()), Vector3(0, 0.08, 0)))
	_body.add_child(Mats.mesh_box(Vector3(0.54 * broad, 0.20, 0.34), tunic, Vector3(0, -0.04, 0)))
	if class_id == "herald":  # white tabard front
		_body.add_child(Mats.mesh_box(Vector3(0.34, 0.62, 0.03), cloth, Vector3(0, 0.34, 0.20)))
	if class_id == "stonewright":  # leather apron
		_body.add_child(Mats.mesh_box(Vector3(0.44, 0.58, 0.03), _track(Mats.wood()), Vector3(0, 0.30, 0.20)))
	# Head with a simple face direction cue (nose) and class headgear.
	_head = Node3D.new()
	_head.position.y = 0.86 * tall
	_body.add_child(_head)
	_head.add_child(Mats.mesh_sphere(0.21, _track(Mats.skin()), Vector3(0, 0.10, 0)))
	_head.add_child(Mats.mesh_box(Vector3(0.07, 0.07, 0.08), _track(Mats.skin()), Vector3(0, 0.06, 0.21)))
	_build_headgear(class_id)
	# Arms on shoulder pivots, legs on hip pivots.
	_arm_l = _make_arm(-0.38 * broad, class_id)
	_arm_r = _make_arm(0.38 * broad, class_id)
	_body.add_child(_arm_l)
	_body.add_child(_arm_r)
	_leg_l = _make_leg(-0.16)
	_leg_r = _make_leg(0.16)
	add_child(_leg_l)
	add_child(_leg_r)
	_build_gear(class_id)
	_build_rank(class_id)
	if class_id == "archivist":
		_body.rotation.x = 0.14


func _make_arm(side_x: float, class_id: String) -> Node3D:
	var pivot: Node3D = Node3D.new()
	pivot.position = Vector3(side_x, 0.66, 0)
	pivot.add_child(Mats.mesh_box(Vector3(0.16, 0.52, 0.16), _class_mat(class_id, 0.95), Vector3(0, -0.24, 0)))
	pivot.add_child(Mats.mesh_sphere(0.09, _track(Mats.skin()), Vector3(0, -0.52, 0)))
	return pivot


func _make_leg(side_x: float) -> Node3D:
	var pivot: Node3D = Node3D.new()
	pivot.position = Vector3(side_x, 0.62, 0)
	pivot.add_child(Mats.mesh_box(Vector3(0.18, 0.54, 0.20),
		_track(Mats.solid(Color(0.30, 0.28, 0.32), 0.9)), Vector3(0, -0.28, 0)))
	pivot.add_child(Mats.mesh_box(Vector3(0.19, 0.10, 0.30), _track(Mats.wood_dark()), Vector3(0, -0.57, 0.04)))
	return pivot


func _build_headgear(class_id: String) -> void:
	var mat: StandardMaterial3D = _class_mat(class_id, 1.05)
	match class_id:
		"steward":
			_head.add_child(Mats.mesh_box(Vector3(0.46, 0.34, 0.06), mat, Vector3(0, 0.05, -0.20)))  # high collar
			_head.add_child(Mats.mesh_cyl(0.20, 0.08, _track(Mats.gold()), Vector3(0, 0.30, 0)))
		"stonewright":
			_head.add_child(Mats.mesh_box(Vector3(0.40, 0.10, 0.40), _track(Mats.wood()), Vector3(0, 0.27, 0)))
		"loremaster":
			_head.add_child(Mats.mesh_cone(0.26, 0.42, mat, Vector3(0, 0.38, 0)))
			for sx: float in [-0.10, 0.10]:  # round spectacles
				_head.add_child(Mats.mesh_cyl(0.055, 0.02, _track(Mats.silver()), Vector3(sx, 0.12, 0.20)))
		"warden":
			_head.add_child(Mats.mesh_box(Vector3(0.40, 0.26, 0.40), _track(Mats.silver()), Vector3(0, 0.22, 0)))
			_head.add_child(Mats.mesh_box(Vector3(0.06, 0.20, 0.34), mat, Vector3(0, 0.38, 0)))  # crest
		"swiftrider":
			_head.add_child(Mats.mesh_cone(0.25, 0.34, mat, Vector3(0, 0.30, -0.03)))  # hood
			_head.add_child(Mats.mesh_box(Vector3(0.30, 0.08, 0.05), _track(Mats.silver()), Vector3(0, 0.24, 0.18)))  # goggles up
		"herald":
			_head.add_child(Mats.mesh_cyl(0.22, 0.10, mat, Vector3(0, 0.29, 0)))
		"glasswright":
			_head.add_child(Mats.mesh_cyl(0.07, 0.03, _track(Mats.gold()), Vector3(0.11, 0.12, 0.20)))  # monocle
		"archivist":
			_head.add_child(Mats.mesh_cone(0.27, 0.36, mat, Vector3(0, 0.30, -0.02)))


func _build_gear(class_id: String) -> void:
	match class_id:
		"steward":
			var staff: Node3D = Node3D.new()
			staff.position = Vector3(0, -0.52, 0.05)
			staff.add_child(Mats.mesh_cyl(0.03, 1.7, _track(Mats.wood_dark()), Vector3(0, 0.55, 0)))
			staff.add_child(Mats.star(0.17, _track(Mats.emissive(Color(1.0, 0.82, 0.3), 2.2)), Vector3(0, 1.5, 0)))
			_arm_r.add_child(staff)
			_body.add_child(Mats.mesh_box(Vector3(0.30, 0.07, 0.04), _track(Mats.gold()), Vector3(0, 0.60, 0.20)))
		"stonewright":
			var hammer: Node3D = Node3D.new()
			hammer.position = Vector3(0, -0.55, 0.05)
			hammer.add_child(Mats.mesh_cyl(0.03, 0.5, _track(Mats.wood()), Vector3(0, 0.1, 0)))
			hammer.add_child(Mats.mesh_box(Vector3(0.26, 0.13, 0.13), _track(Mats.silver()), Vector3(0, 0.36, 0)))
			_arm_r.add_child(hammer)
			_body.add_child(Mats.mesh_sphere(0.07, _track(Mats.emissive(Color(1.0, 0.7, 0.3), 1.6)), Vector3(-0.30, 0.02, 0.12)))
		"loremaster":
			_arm_l.add_child(Mats.mesh_box(Vector3(0.30, 0.40, 0.09), _class_mat(class_id, 0.7), Vector3(0, -0.55, 0.08)))
			_body.add_child(Mats.mesh_box(Vector3(0.30, 0.36, 0.16), _track(Mats.wood()), Vector3(0, 0.32, -0.27)))
			_body.add_child(Mats.mesh_cyl(0.05, 0.34, _track(Mats.solid(Color(0.95, 0.92, 0.82))), Vector3(0.16, 0.55, -0.27)))
		"warden":
			var shield: Node3D = Node3D.new()
			shield.position = Vector3(-0.10, -0.45, 0.06)
			shield.add_child(Mats.mesh_box(Vector3(0.46, 0.66, 0.06), _track(Mats.silver())))
			shield.add_child(Mats.star(0.15, _class_mat(class_id, 1.1), Vector3(0, 0.05, 0.05)))
			_arm_l.add_child(shield)
			var spear: Node3D = Node3D.new()
			spear.position = Vector3(0, -0.52, 0.05)
			spear.add_child(Mats.mesh_cyl(0.025, 1.8, _track(Mats.wood_dark()), Vector3(0, 0.6, 0)))
			spear.add_child(Mats.mesh_cone(0.06, 0.28, _track(Mats.silver()), Vector3(0, 1.62, 0)))
			_arm_r.add_child(spear)
		"swiftrider":
			for arm: Node3D in [_arm_l, _arm_r]:
				arm.add_child(Mats.mesh_box(Vector3(0.05, 0.34, 0.10), _track(Mats.silver()), Vector3(0, -0.68, 0.05)))
			_body.add_child(Mats.mesh_box(Vector3(0.14, 0.10, 0.55), _class_mat(class_id, 1.1), Vector3(0.1, 0.62, -0.25)))
		"herald":
			var horn: MeshInstance3D = Mats.mesh_cone(0.13, 0.62, _track(Mats.silver()), Vector3(0, -0.62, 0.18))
			horn.rotation.x = PI / 2.0
			_arm_r.add_child(horn)
			var pole: Node3D = Node3D.new()
			pole.position = Vector3(0, 0.45, -0.26)
			pole.add_child(Mats.mesh_cyl(0.025, 1.5, _track(Mats.wood_dark()), Vector3(0, 0.6, 0)))
			var pennant: Node3D = Mats.banner(0.20, 0.55, Color(0.95, 0.95, 0.95))
			pennant.position.y = 1.32
			pole.add_child(pennant)
			_body.add_child(pole)
		"glasswright":
			var shard: MeshInstance3D = Mats.mesh_cone(0.11, 0.42,
				_track(Mats.emissive(Color(0.65, 0.9, 1.0), 1.8)), Vector3(0.45, 1.25, 0.2))
			shard.name = "FloatingShard"
			add_child(shard)
			for i: int in 3:
				var vial_color: Color = Color.from_hsv(0.05 + 0.28 * float(i), 0.8, 1.0)
				_body.add_child(Mats.mesh_sphere(0.05, _track(Mats.emissive(vial_color, 1.2)),
					Vector3(-0.14 + 0.14 * float(i), 0.08, 0.21)))
		"archivist":
			_arm_l.add_child(Mats.mesh_box(Vector3(0.34, 0.26, 0.10), _track(Mats.wood()), Vector3(0, -0.5, 0.08)))
			_arm_r.add_child(Mats.mesh_sphere(0.08, _track(Mats.emissive(Color(1.0, 0.75, 0.35), 1.8)), Vector3(0, -0.60, 0.06)))
			for i: int in 3:
				_body.add_child(Mats.mesh_box(Vector3(0.05, 0.14, 0.03), _track(Mats.gold()),
					Vector3(0.24, 0.02 - 0.05 * float(i), 0.16)))


func _build_rank(class_id: String) -> void:
	var rank: String = str(agent.get("rank", "recruit"))
	var order: Array = ["recruit", "sentinel", "sergeant", "captain", "marshal"]
	var lvl: int = order.find(rank)
	if lvl >= 1:  # silver (or gold for marshal) shoulder trim
		var trim: StandardMaterial3D = _track(Mats.gold()) if lvl >= 4 else _track(Mats.silver())
		for sx: float in [-0.36, 0.36]:
			_body.add_child(Mats.mesh_box(Vector3(0.22, 0.09, 0.24), trim, Vector3(sx, 0.70, 0)))
	if lvl >= 2:
		_body.add_child(Mats.star(0.09, _track(Mats.gold()), Vector3(0, 0.52, 0.20)))
	if lvl >= 3:
		_body.add_child(Mats.star(0.09, _track(Mats.gold()), Vector3(-0.16, 0.52, 0.20)))
		_body.add_child(Mats.star(0.09, _track(Mats.gold()), Vector3(0.16, 0.52, 0.20)))
	if lvl >= 3:  # cape, longer for marshal
		var cape_len: float = 1.25 if lvl >= 4 else 0.75
		var cape_mat: StandardMaterial3D = _class_mat(class_id, 0.8)
		cape_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
		_body.add_child(Mats.mesh_box(Vector3(0.56, cape_len, 0.04), cape_mat,
			Vector3(0, 0.68 - cape_len / 2.0, -0.22)))
	if lvl >= 4:
		_body.add_child(Mats.star(0.13, _track(Mats.emissive(Color(1.0, 0.85, 0.35), 2.5)), Vector3(0, 0.52, 0.21)))
		var aura: MeshInstance3D = Mats.mesh_sphere(0.95, Mats.transparent(
			TirithState.class_color(class_id), 0.10), Vector3(0, 0.45, 0))
		aura.name = "Aura"
		add_child(aura)


## Rank cues for an owner GLB: chest stars and a class-colour cape,
## placed on the fitted 1.85 m frame (the hero root scale does the rest).
func _build_glb_rank(class_id: String) -> void:
	var order: Array = ["recruit", "sentinel", "sergeant", "captain", "marshal"]
	var lvl: int = order.find(str(agent.get("rank", "recruit")))
	if lvl >= 2:
		add_child(Mats.star(0.11, Mats.gold(), Vector3(0, 1.32, 0.30)))
	if lvl >= 3:
		add_child(Mats.star(0.11, Mats.gold(), Vector3(-0.20, 1.32, 0.28)))
		add_child(Mats.star(0.11, Mats.gold(), Vector3(0.20, 1.32, 0.28)))
		var cape_len: float = 1.35 if lvl >= 4 else 0.85
		var cape_mat: StandardMaterial3D = Mats.solid(
			TirithState.class_color(class_id).darkened(0.18), 0.9)
		cape_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
		add_child(Mats.mesh_box(Vector3(0.66, cape_len, 0.04), cape_mat,
			Vector3(0, 1.62 - cape_len / 2.0, -0.27)))
	if lvl >= 4:
		var aura: MeshInstance3D = Mats.mesh_sphere(1.0, Mats.transparent(
			TirithState.class_color(class_id), 0.10), Vector3(0, 0.95, 0))
		aura.name = "Aura"
		add_child(aura)


## The glowing selection ring in the class colour, hero-game style.
func _build_ring(class_id: String) -> void:
	var color: Color = TirithState.class_color(class_id)
	var disc_mat: StandardMaterial3D = Mats.transparent(color, 0.22)
	var disc: MeshInstance3D = Mats.mesh_cyl(1.02, 0.03, disc_mat, Vector3(0, 0.04, 0.15), 36)
	disc.name = "HeroDisc"
	add_child(disc)
	_ring_mat = Mats.emissive(color.lightened(0.1), 2.0)
	var ring: MeshInstance3D = Mats.mesh_torus(0.94, 1.10, _ring_mat, Vector3(0, 0.07, 0.15))
	ring.name = "HeroRing"
	add_child(ring)
	# Status particles: bench sparks, done sparkles, failed smoke.
	_sparkles = _make_particles("DoneSparkles", Color(0.65, 0.95, 1.0), 16, 1.6,
		0.05, 0.55, Vector3(0, 0.35, 0), Vector3(0, 0.9, 0.1), 2.6)
	add_child(_sparkles)
	_smoke = _make_particles("FailSmoke", Color(0.45, 0.16, 0.13, 0.5), 12, 2.4,
		0.15, 0.4, Vector3(0, 0.22, 0), Vector3(0, 1.9, 0), 1.2)
	add_child(_smoke)


func _make_particles(p_name: String, color: Color, amount: int, lifetime: float,
		size: float, velocity: float, gravity: Vector3, pos: Vector3,
		energy: float) -> GPUParticles3D:
	var proc: ParticleProcessMaterial = ParticleProcessMaterial.new()
	proc.direction = Vector3(0, 1, 0)
	proc.spread = 32.0
	proc.gravity = gravity
	proc.initial_velocity_min = velocity * 0.6
	proc.initial_velocity_max = velocity * 1.25
	proc.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
	proc.emission_sphere_radius = 0.30
	proc.scale_min = 0.7
	proc.scale_max = 1.3
	var curve: Curve = Curve.new()
	curve.add_point(Vector2(0.0, 0.45))
	curve.add_point(Vector2(0.35, 1.0))
	curve.add_point(Vector2(1.0, 1.5 if p_name == "FailSmoke" else 0.25))
	var curve_tex: CurveTexture = CurveTexture.new()
	curve_tex.curve = curve
	proc.scale_curve = curve_tex
	var mesh: SphereMesh = SphereMesh.new()
	mesh.radius = size
	mesh.height = size * 2.0
	mesh.radial_segments = 8
	mesh.rings = 6
	var mat: StandardMaterial3D
	if color.a < 0.9:
		mat = Mats.transparent(color, color.a)
	else:
		mat = Mats.emissive(color, energy)
	mesh.material = mat
	var p: GPUParticles3D = GPUParticles3D.new()
	p.name = p_name
	p.amount = amount
	p.lifetime = lifetime
	p.randomness = 0.6
	p.local_coords = false
	p.process_material = proc
	p.draw_pass_1 = mesh
	p.position = pos
	p.visibility_aabb = AABB(Vector3(-4, -1, -4), Vector3(8, 8, 8))
	p.emitting = false
	return p


func _build_bench(class_id: String) -> void:
	var bench: Node3D = Node3D.new()
	bench.name = "Workbench"
	bench.position = Vector3(0, 0, 0.78)
	add_child(bench)
	bench.add_child(Mats.mesh_box(Vector3(1.25, 0.09, 0.58), Mats.wood(), Vector3(0, 0.74, 0)))
	for sx: float in [-0.54, 0.54]:
		for sz: float in [-0.20, 0.20]:
			bench.add_child(Mats.mesh_box(Vector3(0.09, 0.72, 0.09), Mats.wood_dark(), Vector3(sx, 0.36, sz)))
	# Something on the bench in the class colour, plus papers and a lantern.
	bench.add_child(Mats.mesh_box(Vector3(0.34, 0.16, 0.26),
		Mats.solid(TirithState.class_color(class_id), 0.7), Vector3(-0.28, 0.87, 0)))
	bench.add_child(Mats.mesh_box(Vector3(0.42, 0.03, 0.30),
		Mats.solid(Color(0.96, 0.93, 0.84)), Vector3(0.26, 0.80, 0.02)))
	bench.add_child(Mats.mesh_sphere(0.06, Mats.emissive(Color(1.0, 0.72, 0.32), 1.6), Vector3(0.52, 0.86, -0.16)))
	bench.add_child(Mats.mesh_box(Vector3(0.30, 0.22, 0.30), Mats.wood(), Vector3(-0.72, 0.11, 0.1)))
	bench.add_child(Mats.mesh_cyl(0.20, 0.42, Mats.wood_dark(), Vector3(0.78, 0.21, 0.16), 10))
	# Gold sparks fly from the bench while this hero is working.
	_sparks = _make_particles("WorkSparks", Color(1.0, 0.78, 0.30), 14, 0.7,
		0.035, 1.1, Vector3(0, -2.2, 0), Vector3(-0.2, 0.95, 0), 3.0)
	bench.add_child(_sparks)
	# A little scaffold beside the bench, up only while work is under way.
	_scaffold = Node3D.new()
	_scaffold.name = "Scaffold"
	_scaffold.position = Vector3(1.05, 0, -0.35)
	bench.add_child(_scaffold)
	for sx: float in [-0.42, 0.42]:
		for sz: float in [-0.30, 0.30]:
			_scaffold.add_child(Mats.mesh_cyl(0.045, 1.9, Mats.wood_dark(), Vector3(sx, 0.95, sz), 6))
	_scaffold.add_child(Mats.mesh_box(Vector3(1.05, 0.07, 0.75), Mats.wood(), Vector3(0, 1.55, 0)))
	_scaffold.add_child(Mats.mesh_box(Vector3(1.05, 0.07, 0.75), Mats.wood(), Vector3(0, 0.85, 0)))
	var brace: MeshInstance3D = Mats.mesh_box(Vector3(0.06, 1.5, 0.06), Mats.wood(), Vector3(0, 0.95, 0.32))
	brace.rotation.z = 0.6
	_scaffold.add_child(brace)
	_scaffold.add_child(Mats.mesh_box(Vector3(0.30, 0.24, 0.30), Mats.silver(), Vector3(-0.2, 1.70, 0)))
	_scaffold.add_child(Mats.mesh_cyl(0.11, 0.22, Mats.wood_dark(), Vector3(0.28, 1.42, 0.2), 8))


## Hero-game nameplate: a dark panel that always faces the camera and
## scales with distance (see _update_plate). Name, epithet in gold,
## rank stars and class, and a status bar in the status colour.
func _build_plate() -> void:
	var class_id: String = str(agent.get("class", "stonewright"))
	var class_color: Color = TirithState.class_color(class_id)
	_plate_root = Node3D.new()
	_plate_root.name = "Nameplate"
	_plate_root.position = Vector3(0, 2.42 + fmod(_phase, 1.0) * 0.16, 0)
	add_child(_plate_root)
	var border_mat: StandardMaterial3D = Mats.transparent(class_color, 0.92)
	border_mat.render_priority = 18
	_plate_root.add_child(Mats.mesh_quad(Vector2(2.60, 1.16), border_mat, Vector3(0, 0, -0.012)))
	var panel_mat: StandardMaterial3D = Mats.transparent(Color(0.05, 0.07, 0.12), 0.80)
	panel_mat.render_priority = 19
	panel_mat.no_depth_test = true
	border_mat.no_depth_test = true
	_plate_root.add_child(Mats.mesh_quad(Vector2(2.52, 1.08), panel_mat, Vector3(0, 0, 0)))
	# Class icon: a diamond tile with the class initial.
	var icon: MeshInstance3D = Mats.mesh_quad(Vector2(0.20, 0.20),
		Mats.emissive(class_color, 0.7), Vector3(-1.12, 0.33, 0.015))
	icon.rotation.z = PI / 4.0
	_plate_root.add_child(icon)
	var icon_letter: Label3D = _plate_label(TirithState.class_label(class_id).substr(0, 1),
		Vector3(-1.12, 0.33, 0.03), Color.WHITE, 44, 0.0032)
	_plate_root.add_child(icon_letter)
	_plate = _plate_label("", Vector3(0.08, 0.33, 0.02), Color.WHITE, 54, 0.0042)
	_plate.name = "PlateName"
	_plate_root.add_child(_plate)
	_plate_epithet = _plate_label("", Vector3(0, 0.06, 0.02), Color(1.0, 0.85, 0.45), 38, 0.0031)
	_plate_root.add_child(_plate_epithet)
	_plate_rank = _plate_label("", Vector3(0, -0.20, 0.02),
		class_color.lightened(0.35), 36, 0.0029)
	_plate_root.add_child(_plate_rank)
	# Status bar: dark trough, coloured fill, status orb at its right end.
	_plate_root.add_child(Mats.mesh_quad(Vector2(2.24, 0.14),
		Mats.transparent(Color(0.02, 0.02, 0.03), 0.9), Vector3(0, -0.42, 0.012)))
	_bar_mat = Mats.emissive(Color.WHITE, 1.6)
	_bar_fg = Mats.mesh_quad(Vector2(2.16, 0.09), _bar_mat, Vector3(0, -0.42, 0.02))
	_bar_fg.name = "StatusBar"
	_plate_root.add_child(_bar_fg)
	_orb_mat = Mats.emissive(Color.WHITE, 2.2)
	_orb = Mats.mesh_sphere(0.085, _orb_mat, Vector3(1.14, 0.33, 0.02))
	_orb.name = "StatusOrb"
	_plate_root.add_child(_orb)
	# Rank stars above the panel (sergeant 1, captain 2, marshal 3).
	var order: Array = ["recruit", "sentinel", "sergeant", "captain", "marshal"]
	var lvl: int = order.find(str(agent.get("rank", "recruit")))
	var star_count: int = maxi(lvl - 1, 0)
	for i: int in range(star_count):
		_plate_root.add_child(Mats.star(0.10,
			Mats.emissive(Color(1.0, 0.82, 0.30), 1.4),
			Vector3((float(i) - float(star_count - 1) / 2.0) * 0.26, 0.70, 0)))
	_bubble = Mats.label3d(". . .", Vector3(0.75, 2.15, 0), Color(0.88, 0.88, 0.88), 60, 0.006)
	_bubble.visible = false
	add_child(_bubble)
	_fail_light = OmniLight3D.new()
	_fail_light.light_color = Color(1.0, 0.25, 0.2)
	_fail_light.light_energy = 0.0
	_fail_light.omni_range = 4.2
	_fail_light.position = Vector3(0, 1.4, 0.4)
	add_child(_fail_light)
	_refresh_plate()


func _plate_label(text: String, pos: Vector3, color: Color,
		font_size: int, pixel_size: float) -> Label3D:
	var l: Label3D = Mats.label3d(text, pos, color, font_size, pixel_size, 8)
	l.billboard = BaseMaterial3D.BILLBOARD_DISABLED
	l.no_depth_test = true
	l.render_priority = 20
	return l


func _refresh_plate() -> void:
	_plate.text = str(agent.get("name", "?"))
	var epithet: String = str(agent.get("epithet", ""))
	_plate_epithet.text = epithet
	_plate_epithet.visible = epithet != ""
	var status_word: String = {"working": "WORKING", "done": "DONE",
		"failed": "FAILED", "stalled": "STALLED"}.get(status, status.to_upper())
	_plate_rank.text = "%s %s - %s - %s" % [
		TirithState.rank_label(str(agent.get("rank", ""))),
		TirithState.class_label(str(agent.get("class", ""))),
		status_word,
		TirithState.format_elapsed(int(agent.get("elapsed_s", 0)))]
	var sc: Color = TirithState.status_color(status)
	if _bar_mat != null:
		_bar_mat.albedo_color = sc
		_bar_mat.emission = sc
	_update_bar()


func _bar_fill() -> float:
	match status:
		"done":
			return 1.0
		"failed":
			return 0.30
		"stalled":
			return 0.50
		_:
			return 0.75 + sin(_t * 2.2 + _phase) * 0.08


func _update_bar() -> void:
	if _bar_fg == null:
		return
	var fill: float = clampf(_bar_fill(), 0.02, 1.0)
	_bar_fg.scale.x = fill
	_bar_fg.position.x = -1.08 + 1.08 * fill


## Billboard the whole plate and keep it legible from the overview:
## its world size grows with camera distance, within sane limits.
func _update_plate() -> void:
	if _plate_root == null or not _plate_root.visible:
		return
	var cam: Camera3D = get_viewport().get_camera_3d()
	if cam == null:
		return
	_plate_root.global_rotation = cam.global_rotation
	var world_scale: float
	var compact: bool
	if cam.projection == Camera3D.PROJECTION_ORTHOGONAL:
		# Orthographic: keep a roughly constant on-screen size.
		world_scale = clampf(cam.size * 0.05, 0.6, 3.2)
		compact = cam.size > 80.0
	else:
		var dist: float = cam.global_position.distance_to(global_position)
		world_scale = clampf(dist * 0.042, 0.80, 2.30)
		compact = dist > 70.0
	_plate_root.scale = Vector3.ONE * (world_scale / HERO_SCALE)
	_plate_root.position.y = 2.42 + plate_lift + fmod(_phase, 1.0) * 0.16
	_plate_epithet.visible = not compact and str(agent.get("epithet", "")) != ""
	_plate_rank.visible = not compact
	if status == "working":
		_update_bar()
	if _ring_mat != null:
		_ring_mat.emission_energy_multiplier = 2.0 + (sin(_t * 2.6 + _phase) * 0.55 if status == "working" else 0.0)


## Called on first build and whenever state.json changes this agent.
func update_agent(p_agent: Dictionary) -> void:
	var old_status: String = status
	agent = p_agent
	status = str(agent.get("status", "working"))
	for child: Node in get_children():
		if child is StaticBody3D:
			child.set_meta("payload", agent)
	_apply_status_visuals(false)
	if status == "done" and old_status != "done":
		_celebrate_left = 2.6
	_refresh_plate()


func _apply_status_visuals(instant: bool) -> void:
	if _orb_mat != null:
		_orb_mat.albedo_color = TirithState.status_color(status)
		_orb_mat.emission = TirithState.status_color(status)
	if _bubble != null:
		_bubble.visible = status == "stalled"
	if _fail_light != null:
		_fail_light.light_energy = 1.6 if status == "failed" else 0.0
	var want_grey: bool = status == "stalled"
	if want_grey != _greyed:
		_greyed = want_grey
		for entry: Variant in _mats:
			var mat: StandardMaterial3D = entry[0]
			var orig: Color = entry[1]
			mat.albedo_color = orig.lerp(Color(0.55, 0.55, 0.57), 0.78) if _greyed else orig
	if instant and status == "failed":
		_slump = 1.0
	if instant and status == "done":
		_celebrate_left = 2.4
	if _sparks != null:
		_sparks.emitting = status == "working"
	if _sparkles != null:
		_sparkles.emitting = status == "done"
	if _smoke != null:
		_smoke.emitting = status == "failed"
	if _scaffold != null:
		_scaffold.visible = status == "working"
	_update_bar()
	if _anim != null:
		_play_anim(str({"working": "work", "done": "celebrate", "failed": "fail",
			"stalled": "idle"}.get(status, "idle")))


## Walk in from the spawn point; used when a new hero appears in state.json.
func walk_to(target: Vector3, final_yaw: float, duration: float = 1.6) -> void:
	walking = true
	_play_anim("walk")
	var dir: Vector3 = target - global_position
	dir.y = 0.0
	if dir.length() > 0.05:
		rotation.y = atan2(dir.x, dir.z)
	var tween: Tween = create_tween()
	tween.tween_property(self, "global_position", target, duration).set_trans(Tween.TRANS_SINE)
	tween.parallel().tween_property(self, "rotation:y", final_yaw, 0.4).set_delay(maxf(duration - 0.4, 0.0))
	tween.tween_callback(func() -> void:
		walking = false
		_apply_status_visuals(false))


func _process(delta: float) -> void:
	_t += delta
	_update_plate()
	var shard: Node = get_node_or_null("FloatingShard")
	if shard != null:
		(shard as Node3D).position.y = 1.25 + sin(_t * 1.7 + _phase) * 0.08
		(shard as Node3D).rotation.y = _t * 0.9
	if _orb != null and status == "working":
		var s: float = 1.0 + sin(_t * 3.0 + _phase) * 0.18
		_orb.scale = Vector3.ONE * s
	if _anim != null:
		return
	if _body == null:
		_animate_glb(delta)
		return
	if _celebrate_left > 0.0:
		_celebrate_left -= delta
	# Slump eases in/out so a status change never pops.
	var slump_target: float = 1.0 if status == "failed" else 0.0
	_slump = lerpf(_slump, slump_target, minf(delta * 3.0, 1.0))
	var archivist_lean: float = 0.14 if str(agent.get("class", "")) == "archivist" else 0.0
	_body.rotation.x = archivist_lean + _slump * 0.42
	_body.position.y = 0.62 - _slump * 0.10
	var bob: float = 0.0
	if walking:
		var swing: float = sin(_t * 9.0 + _phase) * 0.55
		_leg_l.rotation.x = swing
		_leg_r.rotation.x = -swing
		_arm_l.rotation.x = -swing * 0.7
		_arm_r.rotation.x = swing * 0.7
		bob = absf(sin(_t * 9.0 + _phase)) * 0.05
	elif _celebrate_left > 0.0:
		_arm_l.rotation.x = -2.6
		_arm_r.rotation.x = -2.6
		_arm_l.rotation.z = 0.5
		_arm_r.rotation.z = -0.5
		bob = absf(sin(_t * 6.0)) * 0.14
		_head.rotation.x = -0.15
	elif status == "working":
		_arm_l.rotation.z = 0.0
		_arm_r.rotation.z = 0.0
		_arm_r.rotation.x = -0.85 + sin(_t * 3.4 + _phase) * 0.55
		_arm_l.rotation.x = -0.55 + sin(_t * 1.7 + _phase) * 0.10
		_leg_l.rotation.x = 0.0
		_leg_r.rotation.x = 0.0
		_head.rotation.x = 0.22
		bob = sin(_t * 3.4 + _phase) * 0.015
	elif status == "failed":
		_arm_l.rotation.x = 0.25
		_arm_r.rotation.x = 0.25
		_head.rotation.x = 0.45
	elif status == "stalled":
		_arm_l.rotation.x = sin(_t * 0.8 + _phase) * 0.06
		_arm_r.rotation.x = sin(_t * 0.8 + _phase + 1.0) * 0.06
		_head.rotation.x = 0.12
		if _bubble != null:
			_bubble.scale = Vector3.ONE * (1.0 + sin(_t * 2.0) * 0.12)
	else:  # done / resting: slow breathing
		_arm_l.rotation.x = sin(_t * 1.1 + _phase) * 0.07
		_arm_r.rotation.x = sin(_t * 1.1 + _phase + 0.4) * 0.07
		_head.rotation.x = 0.0
		bob = sin(_t * 1.1 + _phase) * 0.012
	_body.position.y += bob


## Whole-body motion for a static GLB (no rig): work rocks forward, done
## bounces once then breathes, failed slumps, walk bobs and rolls slightly.
func _animate_glb(delta: float) -> void:
	if _glb_root == null:
		return
	if _celebrate_left > 0.0:
		_celebrate_left -= delta
	var slump_target: float = 1.0 if status == "failed" else 0.0
	_slump = lerpf(_slump, slump_target, minf(delta * 3.0, 1.0))
	var bob: float = 0.0
	var rock: float = _slump * 0.30
	var roll: float = 0.0
	if walking:
		bob = absf(sin(_t * 9.0 + _phase)) * 0.06
		roll = sin(_t * 9.0 + _phase) * 0.05
	elif _celebrate_left > 0.0:
		bob = absf(sin(_t * 6.0)) * 0.20
		roll = sin(_t * 6.0) * 0.05
	elif status == "working":
		# Tool swing: a slow lift then a sharp strike down onto the bench.
		var swing: float = sin(_t * 3.4 + _phase)
		var strike: float = pow(maxf(swing, 0.0), 3.0)
		rock += 0.08 + swing * 0.05 + strike * 0.14
		bob = swing * 0.018 - strike * 0.03
	elif status == "stalled":
		roll = sin(_t * 0.8 + _phase) * 0.03
	else:
		bob = sin(_t * 1.1 + _phase) * 0.015
	_glb_root.position = _glb_base + Vector3(0, bob - _slump * 0.08, 0)
	_glb_root.rotation = Vector3(rock, 0, roll)





