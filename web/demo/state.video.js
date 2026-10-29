window.TIRITH_STATE = {
 "version": 1,
 "generated_at": "2026-10-29T19:50:58Z",
 "brain": {
  "enabled": true,
  "model": "muse-spark",
  "last_error": null
 },
 "models": {
  "manager": {
   "agent": "anthropic",
   "model": "model-large",
   "effort": "high",
   "settings": {
    "path": "~/.config/agent/settings.json",
    "model": "model-small",
    "effort": null
   },
   "in_sync": false,
   "effort_key": false
  },
  "workers": {
   "agent": "anthropic",
   "model": "model-small",
   "effort": "medium",
   "fallback": [
    "muse"
   ]
  },
  "projects": {
   "chromasol": {
    "model": "model-large"
   }
  },
  "steps": {
   "tirith/3": {
    "effort": "low"
   },
   "chromasol/2": {
    "model": "model-large",
    "effort": "high"
   }
  },
  "choices": [
   "model-large",
   "model-small",
   "model-mini"
  ],
  "efforts": [
   "low",
   "medium",
   "high",
   "xhigh",
   "max"
  ],
  "choices_by_agent": {
   "anthropic": [
    "model-large",
    "model-small"
   ],
   "muse": [
    "model-mini"
   ]
  },
  "agents": [
   "anthropic",
   "muse"
  ],
  "agent_status": {
   "anthropic": {
    "state": "out",
    "since": "2026-10-28T19:10:00Z"
   },
   "muse": {
    "state": "ok",
    "since": null
   }
  },
  "pending": []
 },
 "rules": [
  {
   "id": "subagent-tool",
   "description": "Agents may not start helpers on their own; every helper starts through Tirith so you can see it.",
   "source": "built-in",
   "project": null,
   "count_24h": 1,
   "last_at": "2026-10-28T18:44:00Z",
   "last_agent": "icon-2"
  },
  {
   "id": "agent-cli",
   "description": "Agents may only be started through Tirith, so each one shows up as a lane you can follow.",
   "source": "built-in",
   "project": null,
   "count_24h": 0,
   "last_at": null,
   "last_agent": null
  },
  {
   "id": "no-verify",
   "description": "Nobody may skip the safety checks that run before a commit.",
   "source": "built-in",
   "project": null,
   "count_24h": 1,
   "last_at": "2026-10-28T18:20:00Z",
   "last_agent": "icon-2"
  },
  {
   "id": "hooks-path",
   "description": "Nobody may switch off the commit checks by pointing git at other hooks.",
   "source": "built-in",
   "project": null,
   "count_24h": 0,
   "last_at": null,
   "last_agent": null
  },
  {
   "id": "git-hooks",
   "description": "Agents may not edit or delete the commit checks that guard the project.",
   "source": "built-in",
   "project": null,
   "count_24h": 0,
   "last_at": null,
   "last_agent": null
  },
  {
   "id": "raw-stage",
   "description": "Tests and builds must run through Tirith, so you can see them in Pipelines.",
   "source": "built-in",
   "project": null,
   "count_24h": 4,
   "last_at": "2026-10-28T19:31:00Z",
   "last_agent": "t-world"
  },
  {
   "id": "lane-brief",
   "description": "Every lane starts with a written task, so the agent knows what to do.",
   "source": "built-in",
   "project": null,
   "count_24h": 0,
   "last_at": null,
   "last_agent": null
  },
  {
   "id": "project-check",
   "description": "No new lane starts while the project's setup has problems.",
   "source": "built-in",
   "project": null,
   "count_24h": 0,
   "last_at": null,
   "last_agent": null
  },
  {
   "id": "commit-guard",
   "description": "Commits are checked for banned words and failing tests before they are accepted.",
   "source": "built-in",
   "project": null,
   "count_24h": 0,
   "last_at": null,
   "last_agent": null
  }
 ],
 "autopilot": {
  "enabled": true,
  "max_parallel": 4,
  "started": [],
  "paused_steps": []
 },
 "launch": {
  "token": "demo-launch-token",
  "available": true
 },
 "projects": [
  {
   "id": "chromasol",
   "name": "ChromaSol",
   "color": "#2f7d5b",
   "architecture": {
    "nodes": [
     {
      "id": "photos",
      "label": "Photos",
      "kind": "input",
      "files": 2
     },
     {
      "id": "subject",
      "label": "Person cutout",
      "kind": "service",
      "files": 11
     },
     {
      "id": "engine",
      "label": "Colour engine (C)",
      "kind": "core",
      "files": 14
     },
     {
      "id": "dictionary",
      "label": "Wada dictionary",
      "kind": "data",
      "files": 3
     },
     {
      "id": "database",
      "label": "Room database",
      "kind": "data",
      "files": 6
     },
     {
      "id": "ui",
      "label": "Screens",
      "kind": "ui",
      "files": 31
     }
    ],
    "edges": [
     {
      "from": "photos",
      "to": "subject",
      "label": "bitmap"
     },
     {
      "from": "subject",
      "to": "engine",
      "label": "masks"
     },
     {
      "from": "dictionary",
      "to": "engine",
      "label": "348 combos"
     },
     {
      "from": "engine",
      "to": "database",
      "label": "scans"
     },
     {
      "from": "database",
      "to": "ui",
      "label": "feed"
     }
    ],
    "generated_at": "2026-10-28T19:00:00Z",
    "source": "muse"
   },
   "folder": "/Users/you/DEVELOP/ChromaSol",
   "auto": false,
   "counts": {
    "agents_working": 2,
    "agents_total": 2,
    "pipelines_running": 0,
    "flows_24h": 0
   },
   "roadmap": {
    "file": "/Users/you/DEVELOP/ChromaSol/docs/PLAN.md",
    "deadline": "2026-11-04T01:50:58Z",
    "done": 11,
    "total": 13,
    "steps": [
     {
      "id": "1",
      "title": "Scan a photo and find the colours",
      "status": "done",
      "done": 3,
      "total": 3,
      "items": [
       {
        "text": "Person cutout",
        "done": true,
        "who": "agent"
       },
       {
        "text": "Colour engine",
        "done": true,
        "who": "agent"
       },
       {
        "text": "Result screen",
        "done": true,
        "who": "agent"
       }
      ],
      "agents": []
     },
     {
      "id": "2",
      "title": "Save scans on the phone",
      "status": "done",
      "done": 2,
      "total": 2,
      "items": [
       {
        "text": "Room database",
        "done": true,
        "who": "agent"
       },
       {
        "text": "History screen",
        "done": true,
        "who": "agent"
       }
      ],
      "agents": []
     },
     {
      "id": "3",
      "title": "Store listing and signing",
      "status": "done",
      "done": 3,
      "total": 3,
      "items": [
       {
        "text": "Release signing",
        "done": true,
        "who": "agent"
       },
       {
        "text": "Screenshots",
        "done": true,
        "who": "agent"
       },
       {
        "text": "Privacy text",
        "done": true,
        "who": "agent"
       }
      ],
      "agents": []
     },
     {
      "id": "4",
      "title": "Dark mode",
      "status": "done",
      "done": 3,
      "total": 3,
      "items": [
       {
        "text": "Theme tokens for light and dark",
        "done": true,
        "who": "agent"
       },
       {
        "text": "Follow the system theme",
        "done": true,
        "who": "agent"
       },
       {
        "text": "Toggle in Settings",
        "done": true,
        "who": "agent"
       }
      ],
      "agents": []
     },
     {
      "id": "5",
      "title": "Faster scan",
      "status": "active",
      "done": 0,
      "total": 2,
      "items": [
       {
        "text": "Reuse the pose task between photos",
        "done": false,
        "who": "agent"
       },
       {
        "text": "Benchmark 200 photos on the phone",
        "done": false,
        "who": "agent"
       }
      ],
      "agents": [
       "ivren"
      ]
     }
    ]
   },
   "setup": {
    "manifest": "/Users/owner/DEVELOP/Chromasol/tirith.toml",
    "check": {
     "ok": true,
     "problems": []
    },
    "pipelines": [
     {
      "id": "android-release",
      "name": "Android release",
      "stages": [
       {
        "name": "build",
        "kind": "build",
        "run": "./gradlew assembleRelease",
        "last": {
         "status": "passed",
         "at": "2026-10-28T19:30:34Z",
         "duration_s": 34,
         "run": "android-release#7"
        }
       },
       {
        "name": "tests",
        "kind": "test",
        "run": "./gradlew test",
        "last": {
         "status": "passed",
         "at": "2026-10-28T19:31:20Z",
         "duration_s": 46,
         "run": "android-release#7"
        }
       },
       {
        "name": "sign",
        "kind": null,
        "run": "./scripts/sign.sh",
        "last": {
         "status": "running",
         "at": "2026-10-28T19:31:47Z",
         "duration_s": null,
         "run": "android-release#7"
        }
       },
       {
        "name": "store",
        "kind": "deploy",
        "run": "./scripts/upload.sh",
        "last": null
       }
      ]
     }
    ],
    "hooks": [
     {
      "event": "pre-commit",
      "run": "./gradlew lintRelease",
      "last": {
       "status": "passed",
       "at": "2026-10-28T18:50:00Z",
       "duration_s": 12,
       "run": "git-hooks#9"
      }
     }
    ],
    "rules": [
     {
      "id": "subagent-tool",
      "label": "Sub-agent tool",
      "source": "built-in",
      "enforced_by": [
       "hook"
      ],
      "active": true,
      "description": "Agents may not start helpers on their own; every helper starts through Tirith so you can see it."
     },
     {
      "id": "agent-cli",
      "label": "Agent without lane run",
      "source": "built-in",
      "enforced_by": [
       "hook",
       "shim"
      ],
      "active": true,
      "description": "Agents may only be started through Tirith, so each one shows up as a lane you can follow."
     },
     {
      "id": "no-verify",
      "label": "Skipped git hooks",
      "source": "built-in",
      "enforced_by": [
       "hook"
      ],
      "active": true,
      "description": "Nobody may skip the safety checks that run before a commit."
     },
     {
      "id": "hooks-path",
      "label": "Changed hooks path",
      "source": "built-in",
      "enforced_by": [
       "hook"
      ],
      "active": true,
      "description": "Nobody may switch off the commit checks by pointing git at other hooks."
     },
     {
      "id": "git-hooks",
      "label": "Edited git hooks",
      "source": "built-in",
      "enforced_by": [
       "hook",
       "git"
      ],
      "active": true,
      "description": "Agents may not edit or delete the commit checks that guard the project."
     },
     {
      "id": "raw-stage",
      "label": "Raw pipeline stage",
      "source": "built-in",
      "enforced_by": [
       "hook"
      ],
      "active": true,
      "description": "Tests and builds must run through Tirith, so you can see them in Pipelines."
     },
     {
      "id": "lane-brief",
      "label": "Lane without brief",
      "source": "built-in",
      "enforced_by": [
       "lane run"
      ],
      "active": true,
      "description": "Every lane starts with a written task, so the agent knows what to do."
     },
     {
      "id": "project-check",
      "label": "Project check failing",
      "source": "built-in",
      "enforced_by": [
       "lane run"
      ],
      "active": true,
      "description": "No new lane starts while the project's setup has problems."
     },
     {
      "id": "commit-guard",
      "label": "Commit guard",
      "source": "built-in",
      "enforced_by": [
       "git"
      ],
      "active": true,
      "description": "Commits are checked for banned words and failing tests before they are accepted."
     }
    ],
    "gates_active": true,
    "gates_problems": [],
    "gates_fix": [],
    "gates_summary": "",
    "gates_actions": [],
    "pending": [],
    "protection": [
     {
      "agent": "anthropic",
      "ok": true,
      "checked_at": "2026-10-29T19:49:58Z"
     },
     {
      "agent": "muse",
      "ok": true,
      "checked_at": "2026-10-29T19:49:58Z"
     }
    ]
   }
  }
 ],
 "sessions": [
  {
   "id": "chromasol-today",
   "title": "ChromaSol today",
   "started_at": "2026-10-29T18:50:58Z",
   "ended_at": null,
   "projects": [
    "chromasol"
   ],
   "counts": {
    "agents": 2,
    "pipelines": 1,
    "flows": 0,
    "notes": 0
   }
  }
 ],
 "agents": [
  {
   "id": "wren",
   "parent": "manager-s1",
   "project": "chromasol",
   "name": "Wren",
   "epithet": "of the Quiet Palette",
   "class": "glasswright",
   "rank": "recruit",
   "renown": 20,
   "feature": "Dark mode",
   "status": "done",
   "started_at": "2026-10-29T19:50:52Z",
   "ended_at": "2026-10-29T19:50:54Z",
   "elapsed_s": 2,
   "summary": "Dark theme tokens and the Settings toggle are in; 86 tests pass.",
   "log_tail": [
    "chromasol#8 · test passed · 86 tests · 46s",
    "Bash: tirith plan report --status done --files 4 --tests 4",
    "reported: done"
   ],
   "files_changed": 4,
   "insertions": 118,
   "deletions": 31,
   "report": "Done: dark theme tokens, system follow, Settings toggle. 4 files, 4 tests added.",
   "session": "chromasol-today",
   "goal": "Give ChromaSol a dark theme that follows the system and can be switched in Settings.",
   "active": false,
   "last_activity_at": "2026-10-29T19:50:56Z",
   "files": [],
   "tools": [
    {
     "name": "Bash",
     "count": 4
    },
    {
     "name": "Edit",
     "count": 2
    },
    {
     "name": "Read",
     "count": 2
    }
   ],
   "tests": [
    {
     "at": "2026-10-29T19:50:49Z",
     "command": "tirith run --project chromasol --stage test",
     "result": "passed",
     "failed": 0
    }
   ],
   "events": [],
   "log_entries": [],
   "step": "4",
   "request_id": "in-demo-1",
   "ran": {
    "model": "model-large",
    "effort": "high"
   },
   "turn_requests": [],
   "review": {
    "status": "merged",
    "at": "2026-10-29T19:50:55Z",
    "files": 4,
    "passed": 86,
    "run": "chromasol#8",
    "claims": {
     "ok": true,
     "mismatches": []
    }
   },
   "live": null
  },
  {
   "id": "ivren",
   "parent": "manager-s1",
   "project": "chromasol",
   "name": "Ivren",
   "epithet": "of the Seventh Gate",
   "class": "swiftrider",
   "rank": "recruit",
   "renown": 20,
   "feature": "Faster scan",
   "status": "working",
   "started_at": "2026-10-29T19:50:53Z",
   "ended_at": null,
   "elapsed_s": 5,
   "summary": "Moving the pose landmarker out of the per-photo loop so it is built once.",
   "log_tail": [
    "Grep \"PoseLandmarker\" in app/src",
    "Edit app/src/main/java/scan/PoseRunner.kt",
    "Bash: tirith run --project chromasol --stage test"
   ],
   "files_changed": 2,
   "insertions": 41,
   "deletions": 12,
   "report": null,
   "session": "chromasol-today",
   "goal": "Reuse the pose task between photos so a 200-photo scan is fast on the phone.",
   "active": true,
   "last_activity_at": "2026-10-29T19:50:56Z",
   "files": [],
   "tools": [
    {
     "name": "Bash",
     "count": 4
    },
    {
     "name": "Edit",
     "count": 2
    },
    {
     "name": "Read",
     "count": 2
    }
   ],
   "tests": [],
   "events": [],
   "log_entries": [],
   "step": "5",
   "request_id": "in-demo-1",
   "ran": {
    "model": "model-large",
    "effort": "high"
   },
   "turn_requests": [],
   "live": null
  }
 ],
 "pipelines": [
  {
   "run": "chromasol#7",
   "project": "chromasol",
   "pipeline": "ci",
   "status": "passed",
   "started_at": "2026-10-29T16:38:58Z",
   "ended_at": "2026-10-29T16:39:50Z",
   "session": "chromasol-today",
   "stages": [
    {
     "name": "build",
     "status": "passed",
     "started_at": "2026-10-29T16:38:58Z",
     "ended_at": "2026-10-29T16:39:13Z",
     "duration_s": 15,
     "log_tail": [
      "BUILD SUCCESSFUL in 15s"
     ]
    },
    {
     "name": "test",
     "status": "passed",
     "started_at": "2026-10-29T16:39:13Z",
     "ended_at": "2026-10-29T16:39:50Z",
     "duration_s": 37,
     "log_tail": [
      "tests 82 failures 0"
     ]
    }
   ]
  },
  {
   "run": "chromasol#6",
   "project": "chromasol",
   "pipeline": "ci",
   "status": "failed",
   "started_at": "2026-10-29T14:20:58Z",
   "ended_at": "2026-10-29T14:21:50Z",
   "session": "chromasol-today",
   "stages": [
    {
     "name": "build",
     "status": "passed",
     "started_at": "2026-10-29T14:20:58Z",
     "ended_at": "2026-10-29T14:21:13Z",
     "duration_s": 15,
     "log_tail": [
      "BUILD SUCCESSFUL in 15s"
     ]
    },
    {
     "name": "test",
     "status": "failed",
     "started_at": "2026-10-29T14:21:13Z",
     "ended_at": "2026-10-29T14:21:50Z",
     "duration_s": 37,
     "log_tail": [
      "tests 82 failures 2"
     ]
    }
   ]
  },
  {
   "run": "chromasol#5",
   "project": "chromasol",
   "pipeline": "ci",
   "status": "passed",
   "started_at": "2026-10-29T11:50:58Z",
   "ended_at": "2026-10-29T11:51:50Z",
   "session": "chromasol-today",
   "stages": [
    {
     "name": "build",
     "status": "passed",
     "started_at": "2026-10-29T11:50:58Z",
     "ended_at": "2026-10-29T11:51:13Z",
     "duration_s": 15,
     "log_tail": [
      "BUILD SUCCESSFUL in 15s"
     ]
    },
    {
     "name": "test",
     "status": "passed",
     "started_at": "2026-10-29T11:51:13Z",
     "ended_at": "2026-10-29T11:51:50Z",
     "duration_s": 37,
     "log_tail": [
      "tests 82 failures 0"
     ]
    }
   ]
  },
  {
   "run": "chromasol#8",
   "project": "chromasol",
   "pipeline": "ci",
   "status": "passed",
   "started_at": "2026-10-29T19:49:58Z",
   "ended_at": "2026-10-29T19:50:44Z",
   "session": "chromasol-today",
   "stages": [
    {
     "name": "build",
     "status": "passed",
     "started_at": "2026-10-29T19:49:58Z",
     "ended_at": "2026-10-29T19:50:12Z",
     "duration_s": 14,
     "log_tail": [
      "BUILD SUCCESSFUL in 14s"
     ]
    },
    {
     "name": "test",
     "status": "passed",
     "started_at": "2026-10-29T19:50:12Z",
     "ended_at": "2026-10-29T19:50:44Z",
     "duration_s": 32,
     "log_tail": [
      "tests 86 failures 0"
     ]
    }
   ]
  }
 ],
 "violations": [
  {
   "ts": "2026-10-29T19:50:52Z",
   "rule": "raw-stage",
   "agent": "anthropic",
   "lane": "wren",
   "project": "chromasol",
   "message": "tirith: raw-stage: running the tests raw is not allowed. Do this instead: tirith run --project chromasol --stage test",
   "blocked": "running the tests raw is not allowed",
   "instead": "tirith run --project chromasol --stage test"
  }
 ],
 "rooms": [],
 "flows": [],
 "inbox": [
  {
   "id": "in-demo-1",
   "at": "2026-10-29T19:50:58Z",
   "kind": "chat",
   "project": "chromasol",
   "step": "",
   "text": "Add a dark mode, and make the scan faster. Only ask me if you really need me.",
   "status": "open",
   "manager_pending": true,
   "answers": [],
   "lanes": []
  }
 ],
 "says": [],
 "manager": {
  "name": "Aldric of the Tower",
  "title": "The Manager",
  "activity": "",
  "notes": [],
  "presence": {
   "mode": "on_call",
   "working": true,
   "lane": "manager-s1",
   "session": {
    "lane": "manager-s1",
    "busy": true,
    "turns": 3,
    "context_tokens": 38000,
    "asks": 0
   }
  },
  "turns": [
   {
    "id": "manager-turn-aa11",
    "name": "Manager",
    "status": "done",
    "started_at": "2026-10-29T19:50:59Z",
    "ended_at": "2026-10-29T19:50:55Z",
    "elapsed_s": 0,
    "parent": "manager",
    "turn_requests": [
     "in-demo-1"
    ],
    "lanes": [
     "wren",
     "ivren"
    ],
    "live": null
   }
  ]
 },
 "waiting_on_you": [
  {
   "id": "w-dark-1",
   "project": "chromasol",
   "text": "Dark mode is merged and its tests pass. Ship it in v1.2 this week, or hold it for the 2.0 redesign?",
   "since": "2026-10-29T19:50:54Z",
   "link": null,
   "source": "brain",
   "session": "chromasol-today",
   "kind": "decision",
   "status": "open",
   "command": null,
   "options": [
    {
     "label": "Ship in v1.2",
     "value": "ship"
    },
    {
     "label": "Hold for 2.0",
     "value": "hold"
    }
   ]
  }
 ],
 "answered_waits": [],
 "manager_queue": [],
 "health": {
  "level": "green",
  "at": "2026-10-29T19:50:58Z",
  "checks": [
   {
    "id": "collector",
    "label": "Tirith itself",
    "level": "ok",
    "text": "Tirith is collecting normally."
   },
   {
    "id": "manager",
    "label": "The manager",
    "level": "ok",
    "text": "The manager is answering normally."
   },
   {
    "id": "autopilot",
    "label": "The autopilot",
    "level": "ok",
    "text": "The autopilot is running."
   },
   {
    "id": "disk",
    "label": "Disk",
    "level": "ok",
    "text": "41 GB free."
   }
  ]
 }
};
