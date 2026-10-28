# Ops board (`world/ops/`)

A GitLab-style 2D overlay for the citadel: pipelines with stage graphs and live durations, agents
with live elapsed timers, the owner's waiting list, and the message flows between the Steward,
the agents and the owner. Self-contained: it needs only `world/scripts/tirith_state.gd` and does
not depend on `main.gd`. It only reads state; it opens no ports and writes no files.

## Integrate (two lines in the main scene script)

```gdscript
var ops: Node = preload("res://ops/scenes/ops_board.tscn").instantiate()
add_child(ops)                              # a CanvasLayer (layer 50)
ops.focus_agent.connect(_on_focus_agent)    # func _on_focus_agent(agent_id: String) -> void
```

Every time the state is (re)loaded (the world already polls the file mtime):

```gdscript
ops.set_state(state)   # a TirithState; timers are anchored at state.raw.generated_at
```

* **Tab** toggles the dock (the header bar with counters stays). The board consumes Tab, so
  do not bind it elsewhere. Drag the dock's left edge to resize it.
* `focus_agent(agent_id)` is emitted when an agent row is clicked, or a Messages row whose
  endpoint is an agent (receiver first). Fly the camera to that hero.
* Other API: `show_tab_named("pipelines"|"agents"|"waiting"|"messages")`, `set_board_open(bool)`,
  `set_header_visible(bool)`, `set_dock_width(px)`, `set_pipeline_filter(project, status)`,
  `set_message_filter(endpoint, kind)`, `set_agent_sort("elapsed_desc"|"status"|"project")`.
* Timers tick every second from the state's `generated_at` (live clock, local time zone).
  Headless runs do not tick by themselves; call `tick_once()`.
* `flows` (docs/CONTRACT-flows.md) is read from `state.raw["flows"]`; missing means empty.

## Files

* `scenes/ops_board.tscn`, `scripts/ops_board.gd` the board; `ops_time.gd` time helpers;
  `ops_theme.gd` palette, fonts (system fonts only), theme; `ops_icon.gd` vector icons;
  `ops_frame.gd`, `ops_card_button.gd`, `ops_duration_bar.gd` small widgets.
* `tests/test_ops.gd` headless test, `tests/demo_state.json` fixture with extra runs and flows,
  `tests/capture.gd` screenshot script, `screenshots/` its output.

## Run

```sh
G=/Applications/Godot.app/Contents/MacOS/Godot
$G --headless --path world --script res://ops/tests/test_ops.gd -- --state "$PWD/fixtures/state.example.json"
$G --path world --script res://ops/tests/capture.gd -- --state "$PWD/world/ops/tests/demo_state.json"
```
