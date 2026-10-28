extends RefCounted
## Parsing of the messenger sections of state.json: `rooms` (docs/CONTRACT-rooms.md) and
## `flows` (docs/CONTRACT-flows.md). Pure data, no scene code, so the Rookery and the
## headless tests share one reading of the file. Every accessor fills defaults, so an old
## state file without these keys simply yields empty arrays.

const FLOW_KINDS: Array = ["brief", "message", "report", "request", "answer", "decision", "merge", "alert"]

const FLOW_COLORS: Dictionary = {
	"brief": Color(0.97, 0.76, 0.26),
	"message": Color(0.36, 0.62, 1.0),
	"report": Color(0.38, 0.85, 0.48),
	"request": Color(1.0, 0.64, 0.22),
	"answer": Color(0.98, 0.98, 0.96),
	"decision": Color(0.68, 0.46, 0.92),
	"merge": Color(0.30, 0.80, 0.78),
	"alert": Color(1.0, 0.30, 0.26),
}

const SPEAKER_PALETTE: Array = [
	Color(0.80, 0.34, 0.30), Color(0.30, 0.55, 0.78), Color(0.38, 0.66, 0.40),
	Color(0.72, 0.52, 0.22), Color(0.58, 0.40, 0.72), Color(0.28, 0.65, 0.66),
	Color(0.78, 0.45, 0.62), Color(0.55, 0.60, 0.30),
]


static func rooms_of(raw: Dictionary) -> Array:
	var out: Array = []
	var rooms: Variant = raw.get("rooms", [])
	if typeof(rooms) != TYPE_ARRAY:
		return out
	for item: Variant in rooms as Array:
		if typeof(item) == TYPE_DICTIONARY and str((item as Dictionary).get("id", "")) != "":
			out.append(normalize_room(item as Dictionary))
	return out


static func normalize_room(room: Dictionary) -> Dictionary:
	var out: Dictionary = room.duplicate(true)
	out["id"] = str(room.get("id", ""))
	out["title"] = str(room.get("title", out["id"]))
	out["url"] = str(room.get("url", ""))
	out["summary"] = str(room.get("summary", "")) if room.get("summary") != null else ""
	out["project"] = str(room.get("project", "")) if room.get("project") != null else ""
	out["activity_per_hour"] = int(room.get("activity_per_hour", 0))
	out["message_count"] = int(room.get("message_count", 0))
	if typeof(room.get("messages")) != TYPE_ARRAY:
		out["messages"] = []
	if typeof(room.get("participants")) != TYPE_ARRAY:
		out["participants"] = []
	if typeof(room.get("needs_owner")) != TYPE_ARRAY:
		out["needs_owner"] = []
	if typeof(room.get("reply_edges")) != TYPE_ARRAY:
		out["reply_edges"] = []
	return out


static func flows_of(raw: Dictionary) -> Array:
	var out: Array = []
	var flows: Variant = raw.get("flows", [])
	if typeof(flows) != TYPE_ARRAY:
		return out
	for item: Variant in flows as Array:
		if typeof(item) != TYPE_DICTIONARY:
			continue
		var f: Dictionary = (item as Dictionary).duplicate(true)
		if str(f.get("id", "")) == "" or str(f.get("from", "")) == "" or str(f.get("to", "")) == "":
			continue
		if not FLOW_KINDS.has(str(f.get("kind", ""))):
			f["kind"] = "message"
		f["text"] = str(f.get("text", ""))
		f["from_name"] = str(f.get("from_name", f["from"]))
		f["to_name"] = str(f.get("to_name", f["to"]))
		out.append(f)
	return out


## Colour of a flow: reports turn red when the text says it failed.
static func flow_color(flow: Dictionary) -> Color:
	var kind: String = str(flow.get("kind", "message"))
	if kind == "report":
		var text: String = str(flow.get("text", "")).to_lower()
		if text.begins_with("failed") or text.contains(" failed"):
			return FLOW_COLORS["alert"] as Color
	return FLOW_COLORS.get(kind, FLOW_COLORS["message"]) as Color


static func speaker_color(speaker: String) -> Color:
	var h: int = 0
	for i: int in speaker.length():
		h = (h * 31 + speaker.unicode_at(i)) % 100003
	return SPEAKER_PALETTE[h % SPEAKER_PALETTE.size()] as Color


static func room_color(room: Dictionary) -> Color:
	return speaker_color(str(room.get("id", "")))


static func message_by_number(room: Dictionary, n: int) -> Dictionary:
	for m: Variant in room.get("messages", []) as Array:
		if int((m as Dictionary).get("n", -1)) == n:
			return m as Dictionary
	return {}


static func participant_of(room: Dictionary, speaker: String) -> Dictionary:
	for p: Variant in room.get("participants", []) as Array:
		if str((p as Dictionary).get("name", "")) == speaker:
			return p as Dictionary
	return {"name": speaker, "kind": "external", "hero_id": null, "hero_name": null}


## Messages in thread order: [{"msg": Dictionary, "depth": int}]. A message sits under the
## first message it answers (`re`) that is present; unanswered ones are roots.
static func threaded(room: Dictionary) -> Array:
	var messages: Array = room.get("messages", []) as Array
	var by_n: Dictionary = {}
	for m: Variant in messages:
		by_n[int((m as Dictionary).get("n", 0))] = m
	var children: Dictionary = {}
	var roots: Array = []
	for m: Variant in messages:
		var md: Dictionary = m as Dictionary
		var parent: int = -1
		for r: Variant in md.get("re", []) as Array:
			if by_n.has(int(r)) and int(r) != int(md.get("n", 0)):
				parent = int(r)
				break
		if parent < 0:
			roots.append(md)
		else:
			if not children.has(parent):
				children[parent] = []
			(children[parent] as Array).append(md)
	var out: Array = []
	for root: Variant in roots:
		_walk(root as Dictionary, 0, children, out)
	return out


static func _walk(msg: Dictionary, depth: int, children: Dictionary, out: Array) -> void:
	out.append({"msg": msg, "depth": depth})
	var n: int = int(msg.get("n", 0))
	if children.has(n):
		for c: Variant in children[n] as Array:
			_walk(c as Dictionary, mini(depth + 1, 4), children, out)


## Messages in `fresh` that were not in `old` (by room id + number), oldest first.
## `old` and `fresh` are arrays from rooms_of(). Rooms missing from `old` count as all-new.
static func new_messages(old: Array, fresh: Array) -> Array:
	var seen: Dictionary = {}
	for room: Variant in old:
		for m: Variant in (room as Dictionary).get("messages", []) as Array:
			seen["%s#%d" % [str((room as Dictionary)["id"]), int((m as Dictionary).get("n", 0))]] = true
	var out: Array = []
	for room: Variant in fresh:
		for m: Variant in (room as Dictionary).get("messages", []) as Array:
			var key: String = "%s#%d" % [str((room as Dictionary)["id"]), int((m as Dictionary).get("n", 0))]
			if not seen.has(key):
				out.append({"room": room, "msg": m})
	return out


static func new_flows(seen_ids: Dictionary, flows: Array) -> Array:
	var out: Array = []
	for f: Variant in flows:
		if not seen_ids.has(str((f as Dictionary)["id"])):
			out.append(f)
	return out


## "2026-10-28T19:36:02.000Z" -> "19:36"
static func short_time(iso: Variant) -> String:
	var s: String = str(iso) if iso != null else ""
	var t: int = s.find("T")
	if t < 0 or s.length() < t + 6:
		return "--:--"
	return s.substr(t + 1, 5)


static func clip(text: String, limit: int) -> String:
	var one: String = " ".join(text.split("\n", false)).strip_edges()
	if one.length() <= limit:
		return one
	return one.substr(0, limit - 1).strip_edges() + "..."
