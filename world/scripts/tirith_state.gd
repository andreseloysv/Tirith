class_name TirithState
extends RefCounted
## Parsing and normalising of state.json (docs/CONTRACT.md v1). No scene code here:
## everything returns plain Dictionaries/Arrays with defaults filled in, so the world
## and the headless tests can share exactly the same reading of the file.

const SUPPORTED_VERSION: int = 1

## Class accent colours and display names, from docs/LORE.md.
const CLASS_INFO: Dictionary = {
	"steward": {"label": "Manager", "color": Color(0.16, 0.32, 0.85)},
	"stonewright": {"label": "Stonewright", "color": Color(0.95, 0.52, 0.16)},
	"loremaster": {"label": "Loremaster", "color": Color(0.48, 0.25, 0.78)},
	"warden": {"label": "Warden", "color": Color(0.12, 0.62, 0.62)},
	"swiftrider": {"label": "Swiftrider", "color": Color(0.98, 0.82, 0.18)},
	"herald": {"label": "Herald", "color": Color(0.85, 0.18, 0.25)},
	"glasswright": {"label": "Glasswright", "color": Color(0.55, 0.85, 0.95)},
	"archivist": {"label": "Archivist", "color": Color(0.35, 0.62, 0.30)},
}

const RANK_LABELS: Dictionary = {
	"recruit": "Recruit", "sentinel": "Sentinel", "sergeant": "Sergeant",
	"captain": "Captain", "marshal": "Marshal",
}

const STATUS_COLORS: Dictionary = {
	"working": Color(0.35, 0.85, 0.45), "done": Color(0.45, 0.75, 1.0),
	"failed": Color(1.0, 0.35, 0.30), "stalled": Color(0.75, 0.75, 0.75),
	"running": Color(1.0, 0.75, 0.25), "passed": Color(0.35, 0.85, 0.45),
	"pending": Color(0.65, 0.65, 0.65), "skipped": Color(0.50, 0.50, 0.55),
}

var raw: Dictionary = {}
var load_error: String = ""


## Where state.json lives: `--state <file>` (user args) wins, then $TIRITH_STATE,
## then ~/.tirith/state.json per the contract.
static func resolve_state_path() -> String:
	var args: PackedStringArray = OS.get_cmdline_user_args()
	for i: int in args.size():
		if args[i] == "--state" and i + 1 < args.size():
			return args[i + 1]
		if args[i].begins_with("--state="):
			return args[i].trim_prefix("--state=")
	var env_path: String = OS.get_environment("TIRITH_STATE")
	if env_path != "":
		return env_path
	return OS.get_environment("HOME").path_join(".tirith").path_join("state.json")


static func file_mtime(path: String) -> int:
	if not FileAccess.file_exists(path):
		return -1
	return int(FileAccess.get_modified_time(path))


## Load and parse. On failure `load_error` explains it and `raw` stays empty,
## so the world can still show the citadel with a notice instead of crashing.
func load_from(path: String) -> bool:
	load_error = ""
	raw = {}
	if not FileAccess.file_exists(path):
		load_error = "No state file at %s" % path
		return false
	var text: String = FileAccess.get_file_as_string(path)
	var parsed: Variant = JSON.parse_string(text)
	if typeof(parsed) != TYPE_DICTIONARY:
		load_error = "state.json is not a JSON object"
		return false
	raw = parsed
	if int(raw.get("version", 0)) != SUPPORTED_VERSION:
		load_error = "Unsupported state version %s" % str(raw.get("version"))
		return false
	return true


func projects() -> Array:
	return raw.get("projects", []) as Array


func agents() -> Array:
	return raw.get("agents", []) as Array


func pipelines() -> Array:
	return raw.get("pipelines", []) as Array


func waiting() -> Array:
	return raw.get("waiting_on_you", []) as Array


func steward() -> Dictionary:
	return raw.get("manager", raw.get("steward", {})) as Dictionary


func brain() -> Dictionary:
	return raw.get("brain", {}) as Dictionary


func project_by_id(project_id: String) -> Dictionary:
	for p: Variant in projects():
		if (p as Dictionary).get("id", "") == project_id:
			return p
	return {}


func agents_of_project(project_id: String) -> Array:
	var out: Array = []
	for a: Variant in agents():
		if (a as Dictionary).get("project", "") == project_id:
			out.append(a)
	return out


## HUD counters, straight from the contract fields.
func counts() -> Dictionary:
	var c: Dictionary = {"working": 0, "done": 0, "failed": 0, "stalled": 0,
		"pipelines_running": 0, "waiting": waiting().size()}
	for a: Variant in agents():
		var status: String = str((a as Dictionary).get("status", "working"))
		if c.has(status):
			c[status] = int(c[status]) + 1
	for p: Variant in pipelines():
		if str((p as Dictionary).get("status", "")) == "running":
			c["pipelines_running"] = int(c["pipelines_running"]) + 1
	return c


static func class_label(class_id: String) -> String:
	var info: Dictionary = CLASS_INFO.get(class_id, {})
	return str(info.get("label", class_id.capitalize()))


static func class_color(class_id: String) -> Color:
	var info: Dictionary = CLASS_INFO.get(class_id, {})
	return info.get("color", Color(0.8, 0.8, 0.8)) as Color


static func rank_label(rank_id: String) -> String:
	return str(RANK_LABELS.get(rank_id, rank_id.capitalize()))


static func status_color(status: String) -> Color:
	return STATUS_COLORS.get(status, Color.WHITE) as Color


static func full_name(agent: Dictionary) -> String:
	var name_part: String = str(agent.get("name", "?"))
	var epithet: String = str(agent.get("epithet", ""))
	if epithet != "":
		name_part += " " + epithet
	return "%s, %s %s" % [name_part, rank_label(str(agent.get("rank", ""))),
		class_label(str(agent.get("class", "")))]


static func format_elapsed(seconds: int) -> String:
	if seconds < 60:
		return "%ds" % seconds
	if seconds < 3600:
		return "%dm" % (seconds / 60)
	return "%dh %02dm" % [seconds / 3600, (seconds % 3600) / 60]


static func color_of(project: Dictionary, fallback: Color) -> Color:
	var html: String = str(project.get("color", ""))
	if html.begins_with("#") and html.length() >= 7:
		return Color.html(html)
	return fallback
