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
      },
      "roadmap": {
        "file": "/Users/me/DEVELOP/Chromasol/docs/TODO.md",
        "deadline": null,
        "done": 3,
        "total": 7,
        "steps": [
          {
            "id": "1",
            "title": "Datos y migraci\u00f3n",
            "status": "done",
            "done": 2,
            "total": 2,
            "items": [
              {
                "text": "Room database",
                "done": true
              },
              {
                "text": "Importar partidas antiguas",
                "done": true
              }
            ],
            "agents": []
          },
          {
            "id": "2",
            "title": "Rendimiento",
            "status": "active",
            "done": 1,
            "total": 3,
            "items": [
              {
                "text": "Arranque en fr\u00edo bajo 2 s",
                "done": true
              },
              {
                "text": "Perfilar la pantalla de lienzo",
                "done": false
              },
              {
                "text": "Reducir memoria en tel\u00e9fonos de gama baja",
                "done": false
              }
            ],
            "agents": [
              "w6-perf"
            ]
          },
          {
            "id": "3",
            "title": "Ficha de la tienda",
            "status": "todo",
            "done": 0,
            "total": 2,
            "items": [
              {
                "text": "Declaraci\u00f3n de recursos de IA",
                "done": false
              },
              {
                "text": "Capturas y descripci\u00f3n",
                "done": false
              }
            ],
            "agents": []
          }
        ]
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
      },
      "roadmap": {
        "file": "/Users/me/DEVELOP/Tirith/docs/TODO.md",
        "deadline": "2026-11-04T21:00:00-05:00",
        "done": 13,
        "total": 26,
        "steps": [
          {
            "id": "0",
            "title": "Guarda de commits compatible",
            "status": "done",
            "done": 2,
            "total": 2,
            "items": [
              {
                "text": "Alias de agentes en el c\u00f3digo; nombres reales en `config.toml`.",
                "done": true
              },
              {
                "text": "Guarda de Tirith intacta e inmutable.",
                "done": true
              }
            ],
            "agents": []
          },
          {
            "id": "1",
            "title": "`lane run` devuelve la respuesta final",
            "status": "done",
            "done": 3,
            "total": 3,
            "items": [
              {
                "text": "Al terminar imprime un resumen y la respuesta final.",
                "done": true
              },
              {
                "text": "Se guarda como informe del lane y se ve en la web.",
                "done": true
              },
              {
                "text": "Lane sin respuesta o con error \u2192 \u00abesperando por ti\u00bb.",
                "done": true
              }
            ],
            "agents": []
          },
          {
            "id": "2",
            "title": "\u00c1rbol de delegaci\u00f3n",
            "status": "done",
            "done": 3,
            "total": 3,
            "items": [
              {
                "text": "`lane run` pasa `TIRITH_LANE` a sus hijos.",
                "done": true
              },
              {
                "text": "`register_lane` guarda `parent` autom\u00e1ticamente.",
                "done": true
              },
              {
                "text": "Vista Tree en Agents.",
                "done": true
              }
            ],
            "agents": []
          },
          {
            "id": "3",
            "title": "Hooks de agentes \u2192 `tirith hook`",
            "status": "active",
            "done": 3,
            "total": 6,
            "items": [
              {
                "text": "`tirith hook --agent <alias>` escribe un evento est\u00e1ndar.",
                "done": true
              },
              {
                "text": "Adaptador `anthropic`.",
                "done": true
              },
              {
                "text": "`tirith hook install|uninstall`.",
                "done": true
              },
              {
                "text": "Instalar los hooks en los ajustes de cada agente.",
                "done": false
              },
              {
                "text": "Protegerlos para que un agente no pueda quit\u00e1rselos.",
                "done": false
              },
              {
                "text": "Verificar el payload real de Muse Code.",
                "done": false
              }
            ],
            "agents": [
              "t-world"
            ]
          },
          {
            "id": "4",
            "title": "`lane run --interactive`",
            "status": "todo",
            "done": 0,
            "total": 2,
            "items": [
              {
                "text": "La terminal queda conectada al agente.",
                "done": false
              },
              {
                "text": "Capturar Ctrl+C para cerrar el lane bien.",
                "done": false
              }
            ],
            "agents": []
          },
          {
            "id": "7",
            "title": "Reglas de c\u00f3digo",
            "status": "active",
            "done": 1,
            "total": 3,
            "items": [
              {
                "text": "`tirith check`: manifiesto, build / test / deploy.",
                "done": true
              },
              {
                "text": "`PreToolUse` bloquea sub-agentes fuera de `lane run`.",
                "done": false
              },
              {
                "text": "Git `pre-commit` / `pre-push` con `tirith check`.",
                "done": false
              }
            ],
            "agents": []
          },
          {
            "id": "8",
            "title": "M\u00e1s visibilidad por agente",
            "status": "active",
            "done": 1,
            "total": 3,
            "items": [
              {
                "text": "Objetivo, estado, archivos cambiados y tests por agente.",
                "done": true
              },
              {
                "text": "Logs legibles, m\u00e1s nuevos arriba.",
                "done": false
              },
              {
                "text": "Guardar el diff real por lane.",
                "done": false
              }
            ],
            "agents": []
          },
          {
            "id": "10",
            "title": "Plan visible en el Overview",
            "status": "todo",
            "done": 0,
            "total": 4,
            "items": [
              {
                "text": "El collector lee este archivo.",
                "done": false
              },
              {
                "text": "`tirith lane run --step <n>`.",
                "done": false
              },
              {
                "text": "Tarjeta \u00abPlan\u00bb en el Overview.",
                "done": false
              },
              {
                "text": "Fecha l\u00edmite con cuenta atr\u00e1s.",
                "done": false
              }
            ],
            "agents": []
          }
        ]
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
      },
      "roadmap": null
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
      "step": "2"
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
      "step": null
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
      "step": null
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
      "step": "3"
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
      "step": null
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
