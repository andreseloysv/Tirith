class_name Mats
extends RefCounted
## Shared materials and small procedural-mesh helpers. Everything in the citadel
## is built from these, so the whole world needs no imported assets.

static func solid(color: Color, roughness: float = 0.85, metallic: float = 0.0) -> StandardMaterial3D:
	var m: StandardMaterial3D = StandardMaterial3D.new()
	m.albedo_color = color
	m.roughness = roughness
	m.metallic = metallic
	return m


static func emissive(color: Color, energy: float = 2.0) -> StandardMaterial3D:
	var m: StandardMaterial3D = StandardMaterial3D.new()
	m.albedo_color = color
	m.emission_enabled = true
	m.emission = color
	m.emission_energy_multiplier = energy
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED if energy > 3.0 else BaseMaterial3D.SHADING_MODE_PER_PIXEL
	return m


static func transparent(color: Color, alpha: float) -> StandardMaterial3D:
	var m: StandardMaterial3D = StandardMaterial3D.new()
	var c: Color = color
	c.a = alpha
	m.albedo_color = c
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	return m


# ------------------------------------------------- procedural textures
# Original textures painted in code at startup (brick, cobble, grass, rock,
# planks, roof tiles) plus a normal map derived from the same painting, so
# sun at a low angle rakes across the mortar and cobble gaps. Textures are
# shared; materials are still fresh instances (heroes tint their own).

static var _tex_cache: Dictionary = {}
static var _nrm_cache: Dictionary = {}


static func _frac(v: float) -> float:
	return v - floorf(v)


static func _hash2(a: int, b: int) -> float:
	return _frac(sin(float(a) * 127.1 + float(b) * 311.7) * 43758.5453)


static func _noise(seed_val: int, frequency: float) -> FastNoiseLite:
	var n: FastNoiseLite = FastNoiseLite.new()
	n.seed = seed_val
	n.frequency = frequency
	n.fractal_octaves = 3
	return n


static func _paint(name: String) -> Image:
	var size: int = 256
	var img: Image = Image.create(size, size, false, Image.FORMAT_RGB8)
	var coarse: FastNoiseLite = _noise(7101, 0.012)
	var fine: FastNoiseLite = _noise(90210, 0.09)
	for y: int in range(size):
		for x: int in range(size):
			var c: float = coarse.get_noise_2d(float(x), float(y)) * 0.5 + 0.5
			var f: float = fine.get_noise_2d(float(x), float(y)) * 0.5 + 0.5
			var col: Color = Color.WHITE
			match name:
				"brick":
					var bh: int = 32
					var bw: int = 64
					var row: int = y / bh
					var off: int = bw / 2 if row % 2 == 1 else 0
					var col_id: int = (x + off) / bw
					var mortar: bool = (y % bh) < 3 or ((x + off) % bw) < 3
					var tone: float = 0.92 + _hash2(col_id * 7 + row * 131, row * 3 - col_id) * 0.10
					var streak: float = clampf(coarse.get_noise_2d(float(x) * 3.0, float(y) * 0.25) * 0.5 + 0.5, 0.0, 1.0)
					col = Color(0.96, 0.93, 0.85) * (tone * (0.94 + f * 0.09) * (0.95 + streak * 0.07))
					if mortar:
						col = Color(0.76, 0.71, 0.61) * (0.88 + f * 0.16)
				"cobble":
					var cell: int = 32
					var cx: int = x / cell
					var cy: int = y / cell
					var px: float = float(x % cell) / float(cell) - 0.5
					var py: float = float(y % cell) / float(cell) - 0.5
					var d: float = sqrt(px * px + py * py)
					var tone: float = 0.84 + _hash2(cx * 17 + cy * 57, cy * 9 - cx) * 0.20
					col = Color(0.73, 0.69, 0.61) * (tone * (1.02 - d * 0.34) * (0.93 + f * 0.11))
					if d > 0.47:
						col = Color(0.55, 0.51, 0.44) * (0.85 + f * 0.2)
				"grass":
					var patch: float = coarse.get_noise_2d(float(x) * 0.6, float(y) * 0.6) * 0.5 + 0.5
					col = Color(0.23, 0.48, 0.20).lerp(Color(0.45, 0.66, 0.26), patch)
					col = col.lerp(Color(0.62, 0.72, 0.30), clampf((f - 0.62) * 2.2, 0.0, 0.55))
					if _hash2(x, y) > 0.985:
						col = Color(0.55, 0.75, 0.35)
				"rock":
					var warp: float = coarse.get_noise_2d(float(x) * 0.5, float(y) * 0.05) * 14.0
					var band: float = _frac((float(y) + warp) / 26.0)
					var tone: float = 0.72 + band * 0.22 + (f - 0.5) * 0.22
					col = Color(0.58, 0.54, 0.48) * tone
					if band < 0.10:
						col *= 0.68
				"planks":
					var ph: int = 36
					var row: int = y / ph
					var grain: float = sin(float(y) * 1.7 + fine.get_noise_2d(float(x) * 0.2, float(y)) * 6.0) * 0.05
					var tone: float = 0.82 + _hash2(row * 31, 7) * 0.22 + grain + (c - 0.5) * 0.12
					col = Color(0.62, 0.44, 0.27) * tone
					if (y % ph) < 3 or (x + row * 53) % 128 < 3:
						col *= 0.55
				"tiles":
					var th: int = 28
					var tw: int = 28
					var row: int = y / th
					var arc: float = absf(float(x % tw) / float(tw) - 0.5) * 2.0
					var tone: float = (0.80 + _hash2(x / tw + row * 41, row) * 0.24) * (1.05 - arc * 0.28)
					col = Color(0.85, 0.85, 0.87) * tone * (0.92 + f * 0.12)
					if (y % th) < 4:
						col *= 0.62
			img.set_pixel(x, y, col)
	return img


static var _img_cache: Dictionary = {}


static func _image(name: String) -> Image:
	if not _img_cache.has(name):
		_img_cache[name] = _paint(name)
	return _img_cache[name] as Image


static func texture(name: String) -> Texture2D:
	if not _tex_cache.has(name):
		_tex_cache[name] = ImageTexture.create_from_image(_image(name))
	return _tex_cache[name] as Texture2D


## Tangent-space normal map from the painted image's luminance (Sobel),
## wrapped so the texture tiles seamlessly.
static func normal_texture(name: String) -> Texture2D:
	if not _nrm_cache.has(name):
		var src: Image = _image(name)
		var size: int = src.get_width()
		var img: Image = Image.create(size, size, false, Image.FORMAT_RGB8)
		var strength: float = 2.4
		for y: int in range(size):
			for x: int in range(size):
				var xl: Color = src.get_pixel((x - 1 + size) % size, y)
				var xr: Color = src.get_pixel((x + 1) % size, y)
				var yu: Color = src.get_pixel(x, (y - 1 + size) % size)
				var yd: Color = src.get_pixel(x, (y + 1) % size)
				var dx: float = (xl.get_luminance() - xr.get_luminance()) * strength
				var dy: float = (yd.get_luminance() - yu.get_luminance()) * strength
				var n: Vector3 = Vector3(dx, dy, 1.0).normalized()
				img.set_pixel(x, y, Color(n.x * 0.5 + 0.5, n.y * 0.5 + 0.5, n.z * 0.5 + 0.5))
		_nrm_cache[name] = ImageTexture.create_from_image(img)
	return _nrm_cache[name] as Texture2D


## A soft round dot for particle billboards (sparks, sparkles, smoke).
static func soft_dot_texture() -> Texture2D:
	if not _tex_cache.has("soft_dot"):
		var size: int = 64
		var img: Image = Image.create(size, size, false, Image.FORMAT_RGBA8)
		for y: int in range(size):
			for x: int in range(size):
				var d: float = Vector2(float(x), float(y)).distance_to(
					Vector2(size * 0.5, size * 0.5)) / (size * 0.5)
				var a: float = clampf(1.0 - d, 0.0, 1.0)
				img.set_pixel(x, y, Color(1, 1, 1, a * a))
		_tex_cache["soft_dot"] = ImageTexture.create_from_image(img)
	return _tex_cache["soft_dot"] as Texture2D


static func textured(tex_name: String, tint: Color, roughness: float = 0.92,
		uv_scale: float = 1.0, triplanar: bool = false,
		with_normal: bool = true) -> StandardMaterial3D:
	var m: StandardMaterial3D = StandardMaterial3D.new()
	m.albedo_color = tint
	m.albedo_texture = texture(tex_name)
	m.texture_repeat = true
	m.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC
	m.roughness = roughness
	m.uv1_scale = Vector3(uv_scale, uv_scale, uv_scale)
	if triplanar:
		m.uv1_triplanar = true
		m.uv1_world_triplanar = true
		m.uv1_triplanar_sharpness = 0.6
	if with_normal and tex_name != "grass":
		m.normal_enabled = true
		m.normal_texture = normal_texture(tex_name)
		m.normal_scale = 0.45
	return m


# Palette (see visual-plan/PRD.md).
static func stone() -> StandardMaterial3D:
	return textured("brick", Color(1.02, 0.98, 0.90), 0.92, 0.42, true)
static func stone_dark() -> StandardMaterial3D:
	return textured("brick", Color(0.66, 0.61, 0.53), 0.96, 0.42, true)
static func rock() -> StandardMaterial3D:
	return textured("rock", Color(0.92, 0.88, 0.82), 1.0, 0.20, true)
static func silver() -> StandardMaterial3D: return solid(Color(0.72, 0.77, 0.83), 0.32, 0.55)
static func gold() -> StandardMaterial3D: return solid(Color(0.95, 0.74, 0.25), 0.24, 0.85)
static func grass() -> StandardMaterial3D:
	return textured("grass", Color(1.0, 1.0, 1.0), 1.0, 16.0, false, false)
static func grass_ground() -> StandardMaterial3D:
	return textured("grass", Color(0.82, 0.88, 0.78), 1.0, 72.0, false, false)
static func road() -> StandardMaterial3D:
	return textured("cobble", Color(0.96, 0.92, 0.84), 1.0, 0.75, true)
static func wood() -> StandardMaterial3D:
	return textured("planks", Color(1.0, 1.0, 1.0), 0.9, 1.4)
static func wood_dark() -> StandardMaterial3D:
	return textured("planks", Color(0.58, 0.50, 0.44), 0.9, 1.4)
static func roof(color: Color) -> StandardMaterial3D:
	return textured("tiles", color, 0.78, 2.2)
static func skin() -> StandardMaterial3D: return solid(Color(0.92, 0.72, 0.55), 0.7)


static func mesh_box(size: Vector3, mat: Material, pos: Vector3 = Vector3.ZERO) -> MeshInstance3D:
	var mi: MeshInstance3D = MeshInstance3D.new()
	var b: BoxMesh = BoxMesh.new()
	b.size = size
	mi.mesh = b
	mi.material_override = mat
	mi.position = pos
	return mi


static func mesh_cyl(radius: float, height: float, mat: Material,
		pos: Vector3 = Vector3.ZERO, sides: int = 24) -> MeshInstance3D:
	var mi: MeshInstance3D = MeshInstance3D.new()
	var c: CylinderMesh = CylinderMesh.new()
	c.top_radius = radius
	c.bottom_radius = radius
	c.height = height
	c.radial_segments = sides
	mi.mesh = c
	mi.material_override = mat
	mi.position = pos
	return mi


static func mesh_cone(radius: float, height: float, mat: Material,
		pos: Vector3 = Vector3.ZERO, sides: int = 24) -> MeshInstance3D:
	var mi: MeshInstance3D = MeshInstance3D.new()
	var c: CylinderMesh = CylinderMesh.new()
	c.top_radius = 0.01
	c.bottom_radius = radius
	c.height = height
	c.radial_segments = sides
	mi.mesh = c
	mi.material_override = mat
	mi.position = pos
	return mi


static func mesh_frustum(top_radius: float, bottom_radius: float, height: float,
		mat: Material, pos: Vector3 = Vector3.ZERO, sides: int = 48) -> MeshInstance3D:
	var mi: MeshInstance3D = MeshInstance3D.new()
	var c: CylinderMesh = CylinderMesh.new()
	c.top_radius = top_radius
	c.bottom_radius = bottom_radius
	c.height = height
	c.radial_segments = sides
	mi.mesh = c
	mi.material_override = mat
	mi.position = pos
	return mi


static func mesh_torus(inner_radius: float, outer_radius: float, mat: Material,
		pos: Vector3 = Vector3.ZERO) -> MeshInstance3D:
	var mi: MeshInstance3D = MeshInstance3D.new()
	var t: TorusMesh = TorusMesh.new()
	t.inner_radius = inner_radius
	t.outer_radius = outer_radius
	t.rings = 40
	t.ring_segments = 10
	mi.mesh = t
	mi.material_override = mat
	mi.position = pos
	return mi


static func mesh_quad(size: Vector2, mat: Material,
		pos: Vector3 = Vector3.ZERO) -> MeshInstance3D:
	var mi: MeshInstance3D = MeshInstance3D.new()
	var q: QuadMesh = QuadMesh.new()
	q.size = size
	mi.mesh = q
	mi.material_override = mat
	mi.position = pos
	return mi


static func mesh_sphere(radius: float, mat: Material,
		pos: Vector3 = Vector3.ZERO) -> MeshInstance3D:
	var mi: MeshInstance3D = MeshInstance3D.new()
	var s: SphereMesh = SphereMesh.new()
	s.radius = radius
	s.height = radius * 2.0
	s.radial_segments = 20
	s.rings = 12
	mi.mesh = s
	mi.material_override = mat
	mi.position = pos
	return mi


## A flat seven-pointed star prism — the citadel emblem (docs/LORE.md).
static func star_mesh(radius: float, inner_ratio: float = 0.48, depth: float = 0.18) -> ArrayMesh:
	var points: Array[Vector2] = []
	for i: int in 14:
		var ang: float = -PI / 2.0 + float(i) * PI / 7.0
		var r: float = radius if i % 2 == 0 else radius * inner_ratio
		points.append(Vector2(cos(ang), sin(ang)) * r)
	var verts: PackedVector3Array = PackedVector3Array()
	var half: float = depth / 2.0
	# Front and back fans around the centre, then the side walls.
	for i: int in 14:
		var a: Vector2 = points[i]
		var b: Vector2 = points[(i + 1) % 14]
		verts.append_array([Vector3(0, 0, half), Vector3(b.x, b.y, half), Vector3(a.x, a.y, half)])
		verts.append_array([Vector3(0, 0, -half), Vector3(a.x, a.y, -half), Vector3(b.x, b.y, -half)])
		verts.append_array([
			Vector3(a.x, a.y, -half), Vector3(a.x, a.y, half), Vector3(b.x, b.y, half),
			Vector3(a.x, a.y, -half), Vector3(b.x, b.y, half), Vector3(b.x, b.y, -half)])
	var normals: PackedVector3Array = PackedVector3Array()
	for i: int in range(0, verts.size(), 3):
		var n: Vector3 = (verts[i + 1] - verts[i]).cross(verts[i + 2] - verts[i]).normalized()
		normals.append_array([n, n, n])
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	var mesh: ArrayMesh = ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


static func star(radius: float, mat: Material, pos: Vector3 = Vector3.ZERO) -> MeshInstance3D:
	var mi: MeshInstance3D = MeshInstance3D.new()
	mi.mesh = star_mesh(radius)
	mi.material_override = mat
	mi.position = pos
	return mi


## A hanging banner: sways gently by rotating around its top edge (wind).
## The pivot is at the pole, the cloth hangs below it.
static func banner(width: float, height: float, color: Color) -> Node3D:
	var pivot: Node3D = Node3D.new()
	pivot.name = "Banner"
	var cloth_mat: StandardMaterial3D = solid(color, 0.9)
	cloth_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	var cloth: MeshInstance3D = mesh_box(Vector3(width, height, 0.06),
		cloth_mat, Vector3(0, -height / 2.0, 0))
	pivot.add_child(cloth)
	var trim: MeshInstance3D = mesh_box(Vector3(width, 0.10, 0.08),
		gold(), Vector3(0, -height + 0.05, 0))
	pivot.add_child(trim)
	if width <= 2.2:  # citadel emblem on flags and pennants, not on the scroll
		pivot.add_child(star(minf(width, height) * 0.24, gold(),
			Vector3(0, -height * 0.38, 0.06)))
	pivot.set_meta("is_banner", true)
	return pivot


## Billboard text used for nameplates and signs.
static func label3d(text: String, pos: Vector3, color: Color = Color.WHITE,
		font_size: int = 48, pixel_size: float = 0.008, outline: int = 10) -> Label3D:
	var l: Label3D = Label3D.new()
	l.text = text
	l.position = pos
	l.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	l.no_depth_test = false
	l.shaded = false
	l.font_size = font_size
	l.pixel_size = pixel_size
	l.modulate = color
	l.outline_size = outline
	l.outline_modulate = Color(0.05, 0.05, 0.08, 0.85)
	l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	l.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	return l


## Wrap any node so mouse picking can find it: adds a StaticBody3D with a box
## shape and stores the payload the UI panel needs in metadata.
static func make_clickable(parent: Node3D, size: Vector3, center: Vector3,
		kind: String, payload: Dictionary) -> StaticBody3D:
	var body: StaticBody3D = StaticBody3D.new()
	body.input_ray_pickable = true
	body.collision_layer = 2
	body.collision_mask = 0
	var shape: CollisionShape3D = CollisionShape3D.new()
	var box: BoxShape3D = BoxShape3D.new()
	box.size = size
	shape.shape = box
	shape.position = center
	body.add_child(shape)
	body.set_meta("kind", kind)
	body.set_meta("payload", payload)
	body.add_to_group("clickable")
	parent.add_child(body)
	return body

