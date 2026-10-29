window.TIRITH_STATE = {
  "version": 1,
  "generated_at": "2026-10-28T19:40:00Z",
  "brain": {
    "enabled": true,
    "model": "muse-spark",
    "last_error": null
  },
  "steward": {
    "name": "Aldric of the Tower",
    "title": "The Steward",
    "activity": "Reviewing Ivren's performance report before merging",
    "since": "2026-10-28T19:31:00Z",
    "notes": [
      {
        "at": "2026-10-28T19:05:00Z",
        "project": "chromasol",
        "text": "Merged Halvard's Room database lane into integration"
      },
      {
        "at": "2026-10-28T19:20:00Z",
        "project": "chromasol",
        "text": "Launched Ivren on performance; owner's phone assigned to her"
      },
      {
        "at": "2026-10-28T19:31:00Z",
        "project": "tirith",
        "text": "Contract v1 fixed; two lanes can start in parallel"
      }
    ]
  },
  "waiting_on_you": [
    {
      "id": "w-1",
      "project": "chromasol",
      "text": "Tick the AI asset declaration on the store listing and press Save",
      "since": "2026-10-28T18:50:00Z",
      "link": null
    },
    {
      "id": "w-2",
      "project": "tirith",
      "text": "Install the commit guard in the Tirith repository",
      "since": "2026-10-28T19:38:00Z",
      "link": null
    }
  ],
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
      }
    },
    {
      "id": "tirith",
      "name": "Tirith",
      "color": "#c9a227",
      "architecture": {
        "nodes": [
          {
            "id": "cli",
            "label": "tirith CLI",
            "kind": "input",
            "files": 3
          },
          {
            "id": "collector",
            "label": "Collector",
            "kind": "service",
            "files": 5
          },
          {
            "id": "brain",
            "label": "Muse brain",
            "kind": "core",
            "files": 2
          },
          {
            "id": "state",
            "label": "state.json",
            "kind": "data",
            "files": 1
          },
          {
            "id": "world",
            "label": "3D citadel",
            "kind": "ui",
            "files": 20
          }
        ],
        "edges": [
          {
            "from": "cli",
            "to": "collector",
            "label": "events"
          },
          {
            "from": "collector",
            "to": "brain",
            "label": "log tails"
          },
          {
            "from": "brain",
            "to": "state",
            "label": "summaries"
          },
          {
            "from": "collector",
            "to": "state",
            "label": "every 2 s"
          },
          {
            "from": "state",
            "to": "world",
            "label": "poll"
          }
        ],
        "generated_at": "2026-10-28T19:35:00Z",
        "source": "manual"
      }
    },
    {
      "id": "relagame",
      "name": "RelaGame",
      "color": "#d9772b",
      "architecture": {
        "nodes": [],
        "edges": [],
        "generated_at": null,
        "source": "none"
      }
    }
  ],
  "agents": [
    {
      "id": "w6-perf",
      "parent": "steward",
      "project": "chromasol",
      "name": "Ivren",
      "epithet": "of the Seventh Gate",
      "class": "swiftrider",
      "rank": "captain",
      "renown": 760,
      "feature": "Performance and size on the owner's phone",
      "status": "working",
      "started_at": "2026-10-28T17:40:00Z",
      "ended_at": null,
      "elapsed_s": 7200,
      "summary": "Measuring the 200-photo scan on the phone: 1.39 s per photo after reusing the pose task.",
      "log_tail": [
        "scan 200: persons 5.32 s/photo, none 0.70 s/photo",
        "A/B ratio median 0.70"
      ],
      "files_changed": 7,
      "insertions": 412,
      "deletions": 96,
      "report": null,
      "log_entries": [
        {
          "ts": "2026-10-28T19:39:56Z",
          "kind": "say",
          "text": "Ratio is stable at 0.70 across three runs, so the pool reuse is what made the difference.",
          "detail": "",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T19:39:51Z",
          "kind": "output",
          "text": "scan 200: persons 5.32 s/photo, none 0.70 s/photo\nA/B ratio median 0.70\nruns: 3\nwarm cache: yes\nGPU delegate: off\nmemory peak: 412 MB",
          "detail": "threads: 4\nbatch: 8\nmodel: detect-small\nreport: build/perf/scan-200.json",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T19:39:50Z",
          "kind": "command",
          "text": "adb shell am instrument -w app.perf/.ScanBenchmark",
          "detail": "",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T19:39:38Z",
          "kind": "test",
          "text": "uv run pytest -q",
          "detail": "passed",
          "fields": {
            "passed": 101,
            "failed": 0
          },
          "path": null
        },
        {
          "ts": "2026-10-28T19:39:20Z",
          "kind": "system",
          "text": "12.4k tokens, 38 s",
          "detail": "{\n  \"input_tokens\": 11800,\n  \"output_tokens\": 600,\n  \"duration_ms\": 38000\n}",
          "fields": {
            "input_tokens": 11800,
            "output_tokens": 600,
            "duration_ms": 38000
          },
          "path": null
        },
        {
          "ts": "2026-10-28T19:39:19Z",
          "kind": "say",
          "text": "",
          "detail": "{\n  \"scan_id\": \"scan-200\",\n  \"device\": \"pixel-7\",\n  \"photos\": 200,\n  \"median_ratio\": 0.7,\n  \"finished_at\": \"2026-10-28T19:39:20Z\",\n  \"cached\": true,\n  \"stages\": [\n    {\n      \"name\": \"detect\"\n    },\n    {\n      \"name\": \"match\"\n    },\n    {\n      \"name\": \"rank\"\n    }\n  ],\n  \"notes\": {\n    \"warm\": true,\n    \"threads\": 4\n  }\n}",
          "fields": {
            "scan_id": "scan-200",
            "device": "pixel-7",
            "photos": 200,
            "median_ratio": 0.7,
            "finished_at": "2026-10-28T19:39:20Z",
            "cached": true,
            "stages": [
              {
                "name": "detect"
              },
              {
                "name": "match"
              },
              {
                "name": "rank"
              }
            ],
            "notes": {
              "warm": true,
              "threads": 4
            }
          },
          "path": null
        },
        {
          "ts": "2026-10-28T19:39:02Z",
          "kind": "edit",
          "text": "app/src/main/java/scan/PersonPool.kt",
          "detail": "+12 \u22123",
          "fields": null,
          "path": "/Users/owner/DEVELOP/Chromasol/app/src/main/java/scan/PersonPool.kt"
        },
        {
          "ts": "2026-10-28T19:38:45Z",
          "kind": "read",
          "text": "Grep ModelPool in app/src",
          "detail": "",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T19:38:30Z",
          "kind": "error",
          "text": "FileNotFoundException: build/perf/baseline.json",
          "detail": "Traceback (most recent call last):\n  File \"tools/perf.py\", line 41, in compare\n    base = json.load(open(path))\nFileNotFoundError: build/perf/baseline.json",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T19:38:24Z",
          "kind": "command",
          "text": "python tools/perf.py compare build/perf/baseline.json build/perf/scan-200.json",
          "detail": "",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T19:38:00Z",
          "kind": "spawn",
          "text": "Profile the detector on the phone",
          "detail": "reviewer",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T19:37:30Z",
          "kind": "edit",
          "text": "app/src/main/java/scan/ScanViewModel.kt",
          "detail": "+4 \u22121",
          "fields": null,
          "path": "/Users/owner/DEVELOP/Chromasol/app/src/main/java/scan/ScanViewModel.kt"
        },
        {
          "ts": "2026-10-28T19:37:10Z",
          "kind": "edit",
          "text": "app/src/main/java/scan/Detector.kt",
          "detail": "created",
          "fields": null,
          "path": "/Users/owner/DEVELOP/Chromasol/app/src/main/java/scan/Detector.kt"
        },
        {
          "ts": "2026-10-28T19:37:00Z",
          "kind": "system",
          "text": "System: init",
          "detail": "{\n  \"type\": \"system\",\n  \"subtype\": \"init\"\n}",
          "fields": {
            "type": "system",
            "subtype": "init"
          },
          "path": null
        },
        {
          "ts": "2026-10-28T19:36:59Z",
          "kind": "system",
          "text": "System: rate_limit_event",
          "detail": "{\n  \"type\": \"rate_limit_event\",\n  \"status\": \"ok\"\n}",
          "fields": {
            "type": "rate_limit_event",
            "status": "ok"
          },
          "path": null
        },
        {
          "ts": "2026-10-28T19:36:55Z",
          "kind": "say",
          "text": "Reusing the person detector between photos should cut the per-photo cost. Starting there.",
          "detail": "",
          "fields": null,
          "path": null
        }
      ]
    },
    {
      "id": "w1-data",
      "parent": "w6-perf",
      "project": "chromasol",
      "name": "Halvard",
      "epithet": "Stonehand",
      "class": "stonewright",
      "rank": "sergeant",
      "renown": 340,
      "feature": "Room database with the iOS migrations",
      "status": "done",
      "started_at": "2026-10-28T13:30:00Z",
      "ended_at": "2026-10-28T15:10:00Z",
      "elapsed_s": 6000,
      "summary": "Finished: Room v1 to v4 migrations, curated names and the strings check, all green.",
      "log_tail": [
        "Finished 20 tests",
        "strings OK"
      ],
      "files_changed": 31,
      "insertions": 2210,
      "deletions": 140,
      "report": "Room database with the same schema and v1-v4 migrations as iOS; 19 new device tests.",
      "log_entries": [
        {
          "ts": "2026-10-28T18:40:00Z",
          "kind": "done",
          "text": "Room v1 to v4 migrations, curated names and the strings check, all green.",
          "detail": "Migrations are covered by 20 instrumented tests.\nThe strings check found no missing translations.",
          "fields": {
            "usage": "31.2k tokens, 4 min"
          },
          "path": null
        },
        {
          "ts": "2026-10-28T18:39:30Z",
          "kind": "test",
          "text": "./gradlew testDebugUnitTest",
          "detail": "passed",
          "fields": {
            "passed": 20,
            "failed": 0
          },
          "path": null
        },
        {
          "ts": "2026-10-28T18:38:20Z",
          "kind": "edit",
          "text": "app/src/main/java/db/Migrations.kt",
          "detail": "+58 \u22122",
          "fields": null,
          "path": "/Users/owner/DEVELOP/Chromasol/app/src/main/java/db/Migrations.kt"
        },
        {
          "ts": "2026-10-28T18:37:20Z",
          "kind": "read",
          "text": "Read app/schemas/4.json",
          "detail": "",
          "fields": null,
          "path": "/Users/owner/DEVELOP/Chromasol/app/schemas/4.json"
        },
        {
          "ts": "2026-10-28T18:36:40Z",
          "kind": "say",
          "text": "Migration 3 to 4 adds the curated name column.",
          "detail": "",
          "fields": null,
          "path": null
        }
      ]
    },
    {
      "id": "icon-2",
      "parent": "w6-perf",
      "project": "chromasol",
      "name": "Sefa",
      "epithet": "Prismhand",
      "class": "glasswright",
      "rank": "sentinel",
      "renown": 120,
      "feature": "Crystal C icon in Blender",
      "status": "failed",
      "started_at": "2026-10-28T16:00:00Z",
      "ended_at": "2026-10-28T16:50:00Z",
      "elapsed_s": 3000,
      "summary": "The render matches the outline but the owner preferred the generated image.",
      "log_tail": [
        "IoU 0.9271",
        "SSIM 0.7953"
      ],
      "files_changed": 4,
      "insertions": 600,
      "deletions": 0,
      "report": "Silhouette matches; inner contour still differs from the target.",
      "log_entries": [
        {
          "ts": "2026-10-28T17:40:00Z",
          "kind": "error",
          "text": "Error: the owner preferred the generated image",
          "detail": "",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T17:39:00Z",
          "kind": "output",
          "text": "IoU 0.9271\nSSIM 0.7953",
          "detail": "",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T17:38:40Z",
          "kind": "command",
          "text": "blender -b icon.blend -P render_compare.py",
          "detail": "",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T17:36:40Z",
          "kind": "edit",
          "text": "icons/crystal_c.blend",
          "detail": "edited",
          "fields": null,
          "path": "/Users/owner/DEVELOP/Chromasol/icons/crystal_c.blend"
        }
      ]
    },
    {
      "id": "t-world",
      "parent": "steward",
      "project": "tirith",
      "name": "Marrow",
      "epithet": "Quickstep",
      "class": "stonewright",
      "rank": "recruit",
      "renown": 0,
      "feature": "The 3D citadel",
      "status": "working",
      "started_at": "2026-10-28T19:40:00Z",
      "ended_at": null,
      "elapsed_s": 60,
      "summary": "Laying out the hill, the rings and the districts.",
      "log_tail": [],
      "files_changed": 0,
      "insertions": 0,
      "deletions": 0,
      "report": null,
      "log_entries": [
        {
          "ts": "2026-10-28T19:39:30Z",
          "kind": "say",
          "text": "Laying out the hill, the rings and the districts.",
          "detail": "",
          "fields": null,
          "path": null
        }
      ]
    },
    {
      "id": "t-collector",
      "parent": "t-world",
      "project": "tirith",
      "name": "Odile",
      "epithet": "Inkwell",
      "class": "loremaster",
      "rank": "recruit",
      "renown": 0,
      "feature": "Collector, CLI and the brain",
      "status": "stalled",
      "started_at": "2026-10-28T19:10:00Z",
      "ended_at": null,
      "elapsed_s": 1800,
      "summary": "Waiting: no log output for 12 minutes.",
      "log_tail": [
        "retrying model turn in 2000ms"
      ],
      "files_changed": 3,
      "insertions": 150,
      "deletions": 0,
      "report": null,
      "log_entries": [
        {
          "ts": "2026-10-28T19:28:00Z",
          "kind": "error",
          "text": "retrying model turn in 2000ms",
          "detail": "",
          "fields": null,
          "path": null
        },
        {
          "ts": "2026-10-28T19:25:00Z",
          "kind": "command",
          "text": "uv run tirith collect --once",
          "detail": "",
          "fields": null,
          "path": null
        }
      ]
    }
  ],
  "pipelines": [
    {
      "run": "android-release#7",
      "project": "chromasol",
      "pipeline": "android-release",
      "status": "running",
      "started_at": "2026-10-28T19:30:00Z",
      "ended_at": null,
      "stages": [
        {
          "name": "build",
          "status": "passed",
          "started_at": "2026-10-28T19:30:00Z",
          "ended_at": "2026-10-28T19:30:34Z",
          "duration_s": 34,
          "log_tail": [
            "BUILD SUCCESSFUL in 34s"
          ]
        },
        {
          "name": "tests",
          "status": "passed",
          "started_at": "2026-10-28T19:30:34Z",
          "ended_at": "2026-10-28T19:31:20Z",
          "duration_s": 46,
          "log_tail": [
            "tests 82 failures 0"
          ]
        },
        {
          "name": "lint",
          "status": "passed",
          "started_at": "2026-10-28T19:31:20Z",
          "ended_at": "2026-10-28T19:31:45Z",
          "duration_s": 25,
          "log_tail": [
            "no network: OK"
          ]
        },
        {
          "name": "16kb",
          "status": "passed",
          "started_at": "2026-10-28T19:31:45Z",
          "ended_at": "2026-10-28T19:31:47Z",
          "duration_s": 2,
          "log_tail": [
            "16 KB: OK"
          ]
        },
        {
          "name": "sign",
          "status": "running",
          "started_at": "2026-10-28T19:31:47Z",
          "ended_at": null,
          "duration_s": null,
          "log_tail": [
            "signReleaseBundle"
          ]
        },
        {
          "name": "device",
          "status": "pending",
          "started_at": null,
          "ended_at": null,
          "duration_s": null,
          "log_tail": []
        },
        {
          "name": "store",
          "status": "pending",
          "started_at": null,
          "ended_at": null,
          "duration_s": null,
          "log_tail": []
        },
        {
          "name": "review",
          "status": "pending",
          "started_at": null,
          "ended_at": null,
          "duration_s": null,
          "log_tail": []
        }
      ]
    },
    {
      "run": "ios-testflight#3",
      "project": "relagame",
      "pipeline": "ios-testflight",
      "status": "failed",
      "started_at": "2026-10-28T09:00:00Z",
      "ended_at": "2026-10-28T09:12:00Z",
      "stages": [
        {
          "name": "export",
          "status": "passed",
          "started_at": "2026-10-28T09:00:00Z",
          "ended_at": "2026-10-28T09:04:00Z",
          "duration_s": 240,
          "log_tail": []
        },
        {
          "name": "archive",
          "status": "failed",
          "started_at": "2026-10-28T09:04:00Z",
          "ended_at": "2026-10-28T09:12:00Z",
          "duration_s": 480,
          "log_tail": [
            "error: No signing certificate"
          ]
        },
        {
          "name": "upload",
          "status": "skipped",
          "started_at": null,
          "ended_at": null,
          "duration_s": null,
          "log_tail": []
        }
      ]
    }
  ]
};
