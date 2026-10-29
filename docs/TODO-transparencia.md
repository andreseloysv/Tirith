# TODO: transparencia total de agentes

Objetivo: ver qué hace cada agente (Claude, Muse Code, Codex, Copilot…), quién creó a quién, y cada
pipeline y hook de git en todos los proyectos, sin depender de que los agentes reporten por voluntad
propia. El CLI `tirith` es el estándar; los hooks lo enriquecen; los "errores de compilación" lo hacen
obligatorio.

## 1. Árbol de delegación (quién creó a quién)

- [ ] `lane run` pasa `TIRITH_LANE=<id>` a sus procesos hijos (hoy solo pasa `TIRITH_SESSION`).
- [ ] `register_lane` lee `TIRITH_LANE` y guarda `parent` automáticamente (o `steward` si no hay).
- [ ] Guardar también el id de la petición original del usuario en cada lane.
- [ ] Añadir `parent` a `state.schema.json`, `docs/CONTRACT.md` y a los fixtures.
- [ ] Mostrar el árbol (organigrama) en la vista Agents de la web.
- [ ] Tests: lane anidado a varios niveles hereda el padre correcto.

## 2. `lane run` interactivo (seguir chateando)

- [ ] Flag `tirith lane run --interactive -- claude`: la terminal queda conectada al agente (no se
      redirige stdout al log).
- [ ] Registrar igual: lane, pid, parent, sesión, cambios de git inicio→fin, resultado al salir.
- [ ] Actividad durante el chat vía hooks (recomendado); opcional: copia de pantalla con `script`.
- [ ] Modo normal (log) se mantiene para agente → sub-agente en segundo plano.

## 3. Hooks de agentes → un solo comando `tirith hook`

- [ ] Comando `tirith hook --agent claude|muse|...` que lee el JSON del hook y escribe un evento
      estándar en `events.jsonl`, etiquetado con `TIRITH_LANE`.
- [ ] Verificar nombres de eventos y formato de payload de Muse Code en
      https://dev.meta.ai/docs/muse-code/extending antes de implementar su adaptador.
- [ ] Claude Code: `.claude/settings.json` con `PreToolUse`, `PostToolUse`, `SubagentStop`, `Stop`,
      `UserPromptSubmit`, `SessionStart`.
- [ ] Muse Code: `.muse/hooks.json` (los hooks de proyecto solo corren tras confiar en la carpeta).
- [ ] Capturar los sub-agentes internos (p. ej. la herramienta Task de Claude), que `lane run` no ve.
- [ ] Mostrar los eventos de hooks por lane en el panel del agente.

## 4. Inscripción automática de agentes (cómo empiezan a usar Tirith)

- [ ] `AGENTS.md` único con las reglas de Tirith (lo leen Codex, Copilot y, según una guía de
      terceros, Muse); `CLAUDE.md` apunta a él.
- [ ] Versión a nivel de usuario (`~/.claude/CLAUDE.md` y equivalentes) para que aplique a todos los
      proyectos.
- [ ] Hook `SessionStart`: inicia una sesión de Tirith y añade al contexto "Estás bajo Tirith. Reglas: …".
- [ ] Confirmar si Muse Code tiene hook de inicio de sesión y qué archivo de instrucciones lee.
- [ ] Reglas mínimas del texto:
  - [ ] Lanzar sub-agentes solo con `tirith lane run -- <modo no interactivo del agente>`.
  - [ ] Ejecutar etapas solo con `tirith run --pipeline <id> --stage <nombre>`.
  - [ ] Todo lane lleva brief (el "por qué" y criterios de aceptación) y termina con `--report`.
  - [ ] Si está bloqueado: `tirith wait add`, no adivinar.
  - [ ] Registrar decisiones con `tirith note`.

## 5. Agentes soportados con `lane run` (modo no interactivo)

- [ ] Claude Code: `claude -p "<tarea>"`.
- [ ] Codex CLI: `codex exec "<tarea>"`.
- [ ] GitHub Copilot CLI: `copilot -p "<tarea>"`.
- [ ] Muse Code: `muse exec "<tarea>"` (confirmar con `muse exec --help`).
- [ ] Definir permisos por adelantado para cada uno (flags / allowlists), porque no pueden preguntar.
- [ ] Agentes en la nube (Copilot coding agent, etc.): leer su actividad desde GitHub (PRs, Actions).

## 6. Pipelines con autodescubrimiento (`tirith.toml`)

- [ ] Especificar el manifiesto `tirith.toml` por proyecto: `[[pipelines]]` con etapas
      build / test / deploy y su comando, y `[[hooks]]` (pre-commit, etc.).
- [ ] El collector lee `tirith.toml` y declara los pipelines solo (sin `tirith pipeline declare`).
- [ ] Documentar el formato en `docs/` junto a los demás contratos.
- [ ] Hooks de git propios (lint, tests…) llaman a `tirith run --stage pre-commit -- <check>`.
- [ ] Registrar cada ejecución de hook de git (permitido / bloqueado), incluida la guarda de commits.
- [ ] (Opcional) Leer CI remoto (GitHub Actions) como etapas.

## 7. Cumplimiento obligatorio ("errores de compilación" para agentes)

- [ ] Comando `tirith check` con todas las reglas duras:
  - [ ] existe `tirith.toml`;
  - [ ] build / test / deploy declarados;
  - [ ] hooks de git pasan por `tirith run`;
  - [ ] guarda de commits intacta.
- [ ] Mensajes de error pensados para agentes: archivo:línea, regla violada y cómo arreglarlo.
- [ ] Puertas donde se ejecuta:
  - [ ] Hooks de agente (Claude `PreToolUse` con exit 2, hooks de Muse que bloquean): bloquear
        comandos crudos, sub-agentes fuera de `lane run`, `--no-verify`.
  - [ ] Git `pre-commit` / `pre-push` con `tirith check` (reusar el wrapper de git que ya rechaza
        `--no-verify`).
  - [ ] `tirith lane run` se niega a arrancar si `tirith check` falla (como ya hace con la guarda).
  - [ ] Check obligatorio en CI + protección de rama (la única puerta que ningún agente local evita).
- [ ] El collector marca como "esperando por ti": proyecto con Makefile/`package.json`/CI sin
      `tirith.toml`, hooks que no pasan por `tirith run`, y cada violación con el agente responsable.

## 8. Más visibilidad por agente (después)

- [ ] Guardar el diff real, archivos tocados y comandos ejecutados por lane (hoy solo `shortstat`).
- [ ] Coste y tiempo por lane: tokens, llamadas al modelo, duración; totales por sesión.
- [ ] Registro de decisiones ("por qué") visible en la vista Flow junto a las aristas.

## Orden sugerido

1 (árbol) → 2 (interactivo) → 3 (hooks) → 4 (inscripción) → 7 (`tirith check`) → 6 (`tirith.toml`)
→ 5 (probar con cada agente) → 8.
