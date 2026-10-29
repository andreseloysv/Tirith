# TODO: transparencia total de agentes

Objetivo: ver qué hace cada agente (CLI de Anthropic, Muse Code, Codex, Copilot…), quién creó a quién, y
cada pipeline y hook de git en todos los proyectos, sin depender de que los agentes reporten por
voluntad propia.

**Principio: ninguna regla vive en un archivo de texto.** Cada regla es código que bloquea o que
registra solo. Los mensajes de bloqueo son la documentación: dicen al agente qué regla violó y qué
comando usar en su lugar, como un error de compilación. Lo que no se puede forzar con código (anotar
decisiones, pedir ayuda) se captura automáticamente en vez de pedírselo al agente.

**Sub-agentes solo con `tirith lane run`.** Es la única forma de que el dueño vea todo. El sub-agente
recibe el contexto en el brief (por qué, criterios de aceptación, decisiones tomadas, archivos,
errores), igual que un sub-agente interno, y `lane run` devuelve su respuesta final al terminar.

**Agentes por alias.** El código y la documentación nombran a los agentes por alias (`anthropic`,
`muse`…); el ejecutable real y la ruta de sus ajustes viven en `~/.tirith/config.toml`
(`[agents.<alias>]`), fuera del repo. Así la guarda de commits no se toca.

Este archivo es la fuente del panel «Plan» del Overview: Tirith lee las casillas de cada sección.

## 0. Guarda de commits compatible

- [x] Alias de agentes en el código; nombres reales en `config.toml` (`[agents.<alias>]`).
- [x] Guarda de Tirith intacta e inmutable con las palabras del dueño.

## 1. `lane run` devuelve la respuesta final

- [x] Al terminar imprime un resumen (lane, estado, cambios de git, duración) y la respuesta final.
- [x] La respuesta sale del último evento JSON del agente; si no hay JSON, del final del log.
- [x] Se guarda como informe del lane (`report`) y se ve en la web.
- [x] Lane sin respuesta o con error → "esperando por ti" automático.
- [x] Tests de extracción para cada formato y para texto plano.

## 2. Árbol de delegación (quién creó a quién)

- [x] `lane run` pasa `TIRITH_LANE=<id>` a sus procesos hijos.
- [x] `register_lane` guarda `parent` automáticamente (o `steward`).
- [x] `parent` en `state.schema.json`, `docs/CONTRACT.md` y fixtures.
- [x] Vista Tree (organigrama) en Agents.
- [x] Tests de lanes anidados.

## 3. Hooks de agentes → `tirith hook`

- [x] `tirith hook --agent <alias>` escribe un evento estándar en `events.jsonl` con `TIRITH_LANE`.
- [x] Rápido y a prueba de fallos (siempre sale con 0; punto de extensión para bloquear).
- [x] Adaptador `anthropic`: `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`,
      `SubagentStop`, `Stop`.
- [x] Adaptador `muse` (lector genérico; sus docs no detallan el payload).
- [x] `tirith hook install|uninstall` (fusiona, idempotente).
- [x] `UserPromptSubmit` guarda la petición original del usuario.
- [x] Eventos por lane en el panel del agente.
- [ ] Instalar los hooks en los ajustes de usuario de cada agente.
- [ ] Protegerlos para que un agente no pueda quitárselos.
- [ ] Enlazar cada lane con la petición original del usuario.
- [ ] Verificar el payload real de Muse Code y ajustar su adaptador.

## 4. `lane run --interactive` (seguir chateando)

- [ ] `tirith lane run --interactive -- <agente>`: la terminal queda conectada al agente.
- [ ] Registrar igual: lane, pid, parent, sesión, cambios de git inicio→fin, resultado al salir.
- [ ] Actividad durante el chat vía los hooks de la sección 3.
- [ ] Capturar Ctrl+C para cerrar el lane bien.

## 5. Agentes soportados (modo no interactivo, salida JSON)

- [x] Actividad en vivo leída de los logs JSON de los lanes (herramientas, comandos, archivos).
- [ ] Tokens y coste por lane.
- [ ] Permisos estándar por agente (flags / perfiles), porque no pueden preguntar.
- [ ] Verificar dentro del sandbox de cada agente que el wrapper de git está en `PATH`.
- [ ] Confirmar el formato JSON de Muse Code con una ejecución real.
- [ ] Codex y Copilot probados con `lane run`.
- [ ] Agentes en la nube (Copilot coding agent, etc.): actividad desde GitHub (PRs, Actions).

## 6. Pipelines con autodescubrimiento (`tirith.toml`)

- [x] Formato `tirith.toml`: `[[pipelines]]` con etapas y `[[hooks]]`.
- [x] El collector declara los pipelines solo.
- [x] Formato documentado en `docs/CONTRACT-manifest.md`.
- [x] `tirith run --stage <nombre>` resuelve proyecto, pipeline y comando desde el manifiesto.
- [x] Pipelines declarados que nunca corrieron aparecen como pendientes.
- [ ] Hooks de git propios llaman a `tirith run --stage pre-commit -- <check>`.
- [ ] Registrar cada ejecución de hook de git (permitido / bloqueado), incluida la guarda.
- [ ] (Opcional) Leer CI remoto (GitHub Actions) como etapas.

## 7. Reglas de código ("errores de compilación" para agentes)

- [x] `tirith check`: manifiesto, build / test / deploy, hooks válidos, guarda intacta.
- [x] Mensajes `archivo:línea: regla: problema. Fix: …`.
- [ ] `PreToolUse` bloquea la herramienta interna de sub-agentes y los comandos de agente sin `lane run`.
- [ ] Shims en `~/.local/bin` por alias: en modo no interactivo no arrancan sin el token de `lane run`.
- [ ] `PreToolUse` bloquea etapas crudas (o las reescribe a `tirith run`).
- [ ] `lane run` exige brief y se niega a arrancar si `tirith check` falla.
- [ ] Git `pre-commit` / `pre-push` con `tirith check`.
- [ ] Check obligatorio en CI + protección de rama (en repos privados requiere GitHub Pro).
- [ ] El collector marca como violación todo lo que se cuele, con el agente responsable.

## 8. Más visibilidad por agente

- [x] Objetivo, estado activo, archivos cambiados (git en vivo), herramientas y tests por agente.
- [x] Vista Flow: cada agente se expande con toda su información.
- [ ] Logs legibles: más nuevos arriba, colores por tipo, JSON convertido en información clara.
- [ ] Detección de tests sin falsos positivos.
- [ ] Guardar el diff real por lane.
- [ ] Totales de coste y tiempo por sesión.
- [ ] Registro de decisiones ("por qué") visible en la vista Flow.

## 9. App de escritorio (Tauri) con chat

- [ ] App Tauri 2 que carga la web actual (`web/`) sin reescribirla; Mac, Windows y Linux.
- [ ] Comunicación web ↔ app por IPC de Tauri (sin puertos).
- [ ] Vista de chat: cada conversación es un lane (`lane run --interactive` por debajo).
- [ ] Agentes por stdin/stdout (modo stream-json del CLI `anthropic`, `muse serve`).
- [ ] Preguntas de permiso del agente en la UI.
- [ ] Collector dentro de la app (Python con uv como sidecar).
- [ ] Windows: versión propia del wrapper de git y de la guarda.
- [ ] Confirmar qué agentes existen en cada plataforma.
- [ ] Móvil: para luego.
- [ ] Firma y empaquetado por plataforma.

## 10. Plan visible en el Overview

- [ ] El collector lee este archivo (casillas por sección) y calcula el progreso de cada paso.
- [ ] `tirith lane run --step <n>`: cada lane dice en qué paso trabaja.
- [ ] Tarjeta «Plan» en el Overview: pasos, progreso, estado y agentes trabajando en cada uno.
- [ ] Fecha límite opcional con cuenta atrás.

## Orden

1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9, y 10 en cuanto se pidió.

Hasta el 4 de noviembre (entrega del hackathon): activar y proteger los hooks (3), el bloqueo de
sub-agentes fuera de `lane run` (7), los logs legibles (8) y el plan en el Overview (10).
