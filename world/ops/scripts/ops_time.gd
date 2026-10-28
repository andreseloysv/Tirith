extends RefCounted
## Pure time helpers for the Ops board: ISO parsing, durations, "x ago", local clock.
## Everything is static and has no scene dependency so it can be unit tested.

const MONTHS: PackedStringArray = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug",
	"Sep", "Oct", "Nov", "Dec"]

static var _iso_re: RegEx = null


## ISO-8601 (UTC "Z" or +hh:mm offsets, optional fraction) to unix seconds; -1 when unknown.
static func parse_iso(value: Variant) -> int:
	if typeof(value) != TYPE_STRING:
		return -1
	if _iso_re == null:
		_iso_re = RegEx.new()
		_iso_re.compile("^(\\d{4})-(\\d{2})-(\\d{2})[T ](\\d{2}):(\\d{2}):(\\d{2})(?:\\.\\d+)?(Z|[+-]\\d{2}:?\\d{2})?$")
	var m: RegExMatch = _iso_re.search((value as String).strip_edges())
	if m == null:
		return -1
	var dt: Dictionary = {"year": int(m.get_string(1)), "month": int(m.get_string(2)),
		"day": int(m.get_string(3)), "hour": int(m.get_string(4)),
		"minute": int(m.get_string(5)), "second": int(m.get_string(6))}
	var unix: int = Time.get_unix_time_from_datetime_dict(dt)
	var zone: String = m.get_string(7)
	if zone != "" and zone != "Z":
		var sign: int = -1 if zone.begins_with("-") else 1
		var digits: String = zone.substr(1).replace(":", "")
		unix -= sign * (int(digits.substr(0, 2)) * 3600 + int(digits.substr(2, 2)) * 60)
	return unix


## Compact duration for finished things: 34s, 1m 30s, 1h 40m, 1d 2h.
@warning_ignore("integer_division")
static func format_duration(seconds: int) -> String:
	var s: int = maxi(seconds, 0)
	if s < 60:
		return "%ds" % s
	if s < 3600:
		return "%dm %02ds" % [s / 60, s % 60]
	if s < 86400:
		return "%dh %02dm" % [s / 3600, (s % 3600) / 60]
	return "%dd %dh" % [s / 86400, (s % 86400) / 3600]


## Same, but keeps the seconds ticking for hour-long things: 1h 40m 09s.
@warning_ignore("integer_division")
static func format_duration_live(seconds: int) -> String:
	var s: int = maxi(seconds, 0)
	if s >= 3600 and s < 86400:
		return "%dh %02dm %02ds" % [s / 3600, (s % 3600) / 60, s % 60]
	return format_duration(s)


## Age: 30s ago, 12 min ago, 1h 40m ago, 2h ago, 3d ago.
@warning_ignore("integer_division")
static func format_ago(seconds: int) -> String:
	if seconds < 0:
		return "just now"
	if seconds < 60:
		return "%ds ago" % seconds
	if seconds < 3600:
		return "%d min ago" % (seconds / 60)
	if seconds < 86400:
		var m: int = (seconds % 3600) / 60
		if m == 0:
			return "%dh ago" % (seconds / 3600)
		return "%dh %dm ago" % [seconds / 3600, m]
	return "%dd ago" % (seconds / 86400)


## Seconds a thing has been going: finished ones use their end, live ones use `now`.
static func elapsed_between(start: int, now: int, end: int) -> int:
	if start < 0:
		return 0
	return maxi((end if end >= 0 else now) - start, 0)


## Local wall clock for a UTC unix time. `bias_minutes` is the owner's UTC offset.
## Times on the same local day as `anchor` show "19:30", others "Oct 27 09:00".
static func format_local_time_with_bias(unix: int, bias_minutes: int, anchor: int) -> String:
	if unix < 0:
		return "--:--"
	var d: Dictionary = Time.get_datetime_dict_from_unix_time(unix + bias_minutes * 60)
	var a: Dictionary = Time.get_datetime_dict_from_unix_time(anchor + bias_minutes * 60)
	var clock: String = "%02d:%02d" % [int(d["hour"]), int(d["minute"])]
	if d["year"] == a["year"] and d["month"] == a["month"] and d["day"] == a["day"]:
		return clock
	return "%s %d %s" % [MONTHS[int(d["month"]) - 1], int(d["day"]), clock]


static func local_bias_minutes() -> int:
	return int(Time.get_time_zone_from_system().get("bias", 0))


static func format_local_time(unix: int, anchor: int) -> String:
	return format_local_time_with_bias(unix, local_bias_minutes(), anchor)
