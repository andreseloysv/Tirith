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
- [x] Muse protocol probe 1: read muse serve help
- [x] Muse protocol probe 2: export the MSP schema
- [x] Muse protocol probe 3: generate the MSP JSON schema
- [x] Manager de guardia: atender el chat
- [ ] Revisar el plan (2026-10-29) (en revisión)
- [x] Manager de guardia: atender el chat
- [ ] Verificar: Verify Commit lane/manager-permission-questions-nev, me (en curso: verify-commit-lane-manager-permi-2)
- [ ] Manager de guardia: atender el chat (en curso: manager-60ade9f8)
- [x] Revisar el plan (2026-10-30) (hecho: f37c457)
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Revisar el plan (2026-10-30) (hecho: 627881c)
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Revisar el plan (2026-10-30) (hecho: 54e1e3d)
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Verificar: Verify Commit lane/manager-permission-questions-nev, me (hecho: e921649)
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Revisar el plan (2026-10-30) (hecho: 860eb45)
- [x] Manager de guardia: atender el chat
- [x] Revisar el plan (2026-10-30) (hecho: fb8048e)
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Muse chat real check: one tiny session

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
- [x] Instalar los hooks en los ajustes de usuario de cada agente.
- [x] Protegerlos para que un agente no pueda quitárselos (se reparan solos).
- [x] Enlazar cada lane con la petición original del usuario.
- [ ] Verificar el payload real de Muse Code y ajustar su adaptador (en revisión)

## 4. `lane run --interactive` (seguir chateando)

- [x] `tirith lane run --interactive -- <agente>`: la terminal queda conectada al agente.
- [x] Registrar igual: lane, pid, parent, sesión, cambios de git inicio→fin, resultado al salir.
- [x] Actividad durante el chat vía los hooks de la sección 3.
- [x] Capturar Ctrl+C para cerrar el lane bien.
- [x] Probarlo con un agente real en una terminal real.

## 5. Agentes soportados (modo no interactivo, salida JSON)

- [x] Actividad en vivo leída de los logs JSON de los lanes (herramientas, comandos, archivos).
- [x] Tokens y coste por lane.
- [x] Permisos estándar por agente (flags / perfiles), porque no pueden preguntar.
- [x] `tirith doctor --agent <alias>`: comprueba ejecutable, `PATH` del wrapper y que saltarse los hooks falla.
- [x] Confirmar el formato JSON de Muse Code con una ejecución real (capturada el 2026-10-29; adaptador en main, lane museformat2). (hecho: cc5b438)
- [x] Codex y Copilot instalados (dueño).
- [x] Codex y Copilot probados con `lane run` (presets por alias, salida JSON, permisos). (hecho: d7cda11)
- [x] Agentes en la nube y CI remoto desde GitHub (PRs y Actions).
- [x] Token de GitHub disponible para el collector.

## 6. Pipelines con autodescubrimiento (`tirith.toml`)

- [x] Formato `tirith.toml`: `[[pipelines]]` con etapas y `[[hooks]]`.
- [x] El collector declara los pipelines solo.
- [x] Formato documentado en `docs/CONTRACT-manifest.md`.
- [x] `tirith run --stage <nombre>` resuelve proyecto, pipeline y comando desde el manifiesto.
- [x] Pipelines declarados que nunca corrieron aparecen como pendientes.
- [x] Hooks de git propios llaman a `tirith run --stage pre-commit -- <check>`.
- [x] Registrar cada ejecución de hook de git (permitido / bloqueado), incluida la guarda.
- [x] (Opcional) Leer CI remoto (GitHub Actions) como etapas.

## 7. Reglas de código ("errores de compilación" para agentes)

- [x] `tirith check`: manifiesto, build / test / deploy, hooks válidos, guarda intacta.
- [x] Mensajes `archivo:línea: regla: problema. Fix: …`.
- [x] `PreToolUse` bloquea la herramienta interna de sub-agentes y los comandos de agente sin `lane run`.
- [x] Shims en `~/.local/bin` por alias: en modo no interactivo no arrancan sin el token de `lane run`.
- [x] `PreToolUse` bloquea etapas crudas (o las reescribe a `tirith run`).
- [x] `lane run` exige brief y se niega a arrancar si `tirith check` falla.
- [x] Git `pre-commit` / `pre-push` con `tirith check` (nueva versión de la guarda).
- [x] Reinstalar la guarda nueva en Tirith.
- [x] Reinstalar la guarda nueva en ChromaSol.
- [x] Workflow de CI con tests y `tirith check --ci`.
- [x] Hacer obligatorio el CI con protección de rama en GitHub (dueño: https://github.com/andreseloysv/Tirith/settings/branches).
- [x] El collector registra cada violación bloqueada (`state.violations`) y la pone en "esperando por ti".
- [x] Violaciones visibles en la web, con el agente responsable.
- [x] Activar las puertas en la máquina del dueño (hooks de usuario + shims).
- [x] `tirith.toml` en ChromaSol y RelaGame.
- [x] ChromaSol: unir su hook `pre-commit` de rutas protegidas con la guarda de palabras (dueño); hasta entonces `lane run` no arranca allí.
- [x] Tirith no se bloquea solo: duplicados por elemento (no por paso) y recupera un bloqueo de git abandonado en el checkout principal. (hecho: b8f1ea3)
  - [x] Revisar: Tirith no se bloquea solo: duplicados por elemento (no por paso) y recupera un bloqueo de git abandonado en el checkout principal. (hecho: 511c1d0)
- [ ] Todo comando tiene timeout por código (collector, etapas y comandos de los agentes): Tirith lo aprende del historial, los agentes pueden subirlo o bajarlo dentro de límites con motivo, y al vencer se detiene el proceso con sus hijos y se limpian sus bloqueos. (en revisión)
- [ ] Un elemento se marca hecho en cuanto su trabajo está en main y verificado, aunque la lane se haya fusionado sola; lo ya fusionado hoy se marca con su commit. (en revisión)
- [x] Llevar a main el trabajo de la lane reglas-de-c-digo-errores-de-comp: aplicar ~/.tirith/diffs/reglas-de-c-digo-errores-de-comp.patch sobre main y reportar solo lo que está en el diff (2 tests nuevos, no 3; no nombrar index.lock como archivo cambiado). (hecho: 511c1d0)
- [x] Integrar: lane/reglas-de-c-digo-errores-de-comp (duplicados por elemento y bloqueo de git abandonado). (hecho: 511c1d0)
- [ ] Los agentes no piden permisos: dentro de su copia pueden ejecutar cualquier comando y solo los hooks de Tirith deciden (reglas actuales + límite: escribir solo en su copia y su temporal, nunca leer credenciales, sin servidores ni puertos, sin matar procesos ajenos). (en revisión)
- [ ] Cada bloqueo de un hook enseña la alternativa exacta y el agente sigue; varios bloqueos seguidos van al manager, nunca al dueño. (en revisión)
- [ ] Tests rápidos en la lane (solo lo relacionado con el cambio) y suite completa en la cola de fusión; el paralelismo se ajusta a CPU y disco. (en revisión)
- [ ] El tiempo límite de un agente mide inactividad (sin salida ni progreso durante N minutos), no el tiempo total; una lane que sigue trabajando nunca se corta por durar mucho. (en revisión)
- [x] Los tests limpian sus carpetas temporales, incluidos los hooks bloqueados que crean, para no llenar el disco. (check: `cd collector && uv run -q pytest -q tests -k cleanup`) (hecho: 46f4696)
- [x] La cola de merge arranca sola desde el collector: ningún lane terminado espera en la cola sin un proceso de merge. (check: `cd collector && uv run -q pytest -q tests -k mergeq`) (hecho: fbb405e)
- [x] La cola de merge usa la rama real de cada lane, la de su worktree. (check: `cd collector && uv run -q pytest -q tests -k mergeq`) (hecho: 9fea4fd)
- [ ] Cada fallo de la cola de merge vuelve con su motivo al lane o al manager. (check: `cd collector && uv run -q pytest -q tests -k mergeq`)
- [x] i-3a1bd406 (hecho: e95c9f2)
- [ ] Tirith guarda una copia del brief de cada lane en su carpeta, así un lane cortado se puede retomar tras un reinicio. (check: `cd collector && uv run -q pytest -q tests -k brief`)
- [x] Ninguna petición de permiso del manager llega al dueño: el código la permite o la bloquea según las reglas. (check: `cd collector && uv run -q pytest -q tests -k msession`)
  - [x] Revisar: Ninguna petición de permiso del manager llega al dueño: el código la permite o la bloquea según las reglas. (check: `cd collector && uv run -q pytest -q tests -k msession`) — falló: the merge did not go through: the review was interrupted; run it again
- [x] Cada hora un lane de prueba recorre todo el camino en un repo de prueba, del plan al push, y la salud se pone roja si no llega. (check: `cd collector && uv run -q pytest -q tests -k canary`) (hecho: b69b549)
- [x] La suite de tests de Python termina en menos de 4 minutos. (check: `bash scripts/check-test-time.sh`) (hecho: 31bc010)
- [x] Ningún test de Python falla al azar: los tests inestables se arreglan. (check: `bash scripts/check-flaky.sh`) (hecho: 3945182)
- [x] Las etapas que bloquean merges ya no compilan ni prueban el port a Rust parado. (check: `! grep -qE 'tirith-core|tirith-cli|--workspace' tirith.toml`) (hecho: 5d8c719)
- [x] Llevar a main el trabajo de lane/u-keepalive (un lane no termina mientras su etapa de tests corre): hoy marcado hecho con ec74885, que es un commit del plan, y main no tiene el test. (check: `git merge-base --is-ancestor e5acf09 main`) (hecho: e5acf09)
- [x] i-3af30eb5 (hecho: 8e3a1a9)
- [ ] El control de escrituras Bash del manager deja leer con git (branch --list, worktree list, stash list, tag -l) y escribir briefs en /tmp, y bloquea git -c x=y commit, merge y push. (check: `cd collector && uv run -q pytest -q tests/test_msession.py -k manager_bash_reads_and_git_options`)
- [ ] La regla workspace-write no bloquea por error: resuelve variables de shell como $S/x hacia el temporal del lane, permite 2>/dev/null y no toma como destino de escritura los argumentos de grep tras una tubería. (check: `cd collector && uv run -q pytest -q tests -k workspace_write_false_positives`)
- [ ] Los 11 tests de test_autoreview vuelven a pasar: el Spawner del test falla con 'review' is not in list porque el collector ahora arranca 'canary run'. (check: `cd collector && uv run -q pytest -q tests/test_autoreview.py`)
- [ ] Los 4 tests rojos de test_brain (brain_calls), test_rooms y test_collector vuelven a pasar sin debilitarlos. (check: `cd collector && uv run -q pytest -q tests/test_brain.py tests/test_rooms.py tests/test_collector.py`)
- [ ] La regla roadmap-edit no bloquea comandos que solo leen el plan: un grep del plan en un comando que también usa sed -n o python -c open() de otros ficheros pasa. (check: `cd collector && uv run -q pytest -q tests -k roadmap_edit_read_only`)
- [x] tirith.toml no nombra el port a Rust (tirith-core, tirith-cli) en ninguna etapa que bloquea merges: mover el pipeline rust-port a un archivo aparte. (check: `! grep -qE 'tirith-core|tirith-cli|--workspace' tirith.toml`) (hecho: d2f1927)
- [x] Un lane no termina mientras su etapa de tests sigue corriendo: Tirith espera el resultado y se lo pasa al agente. (check: `cd collector && uv run -q pytest -q tests -k keepalive`) (hecho: ec74885)

## 8. Más visibilidad por agente

- [x] Objetivo, estado activo, archivos cambiados (git en vivo), herramientas y tests por agente.
- [x] Vista Flow: cada agente se expande con toda su información.
- [x] Logs legibles: más nuevos arriba, colores por tipo, JSON convertido en información clara.
- [x] Detección de tests sin falsos positivos.
- [x] Guardar el diff real por lane.
- [x] Totales de coste y tiempo por sesión.
- [x] Registro de decisiones ("por qué") visible en la vista Flow.

## 9. App de escritorio (Tauri) con chat

- [x] App Tauri 2 que carga la web actual (`web/`) sin reescribirla; Mac, Windows y Linux.
- [x] Comunicación web ↔ app por IPC de Tauri (sin puertos).
- [x] Vista de chat: cada conversación es un lane (`lane run --interactive` por debajo).
- [x] Agentes por stdin/stdout: modo stream-json del agente `anthropic` (`chat_args`).
- [ ] Chat con `muse serve` (en curso: u-musechat)
- [x] Preguntas de permiso del agente en la UI.
- [x] La app arranca el collector si no hay uno (con uv; Python aún no va empaquetado).
- [x] Abrir la app en modo dev y comprobar que se ve y se actualiza.
- [x] Windows: shims propios (`.cmd`).
- [x] Windows: versión propia del wrapper de git y de la guarda (`safe-git`).
- [x] Confirmar qué agentes existen en cada plataforma. (hecho: fea63d1)
- [x] Empaquetado por plataforma (configuración y scripts de build).
- [x] Firmar la app de macOS con el Developer ID del dueño.
- [x] Notarizar la app de macOS (`scripts/notarize-macos.sh`, perfil `tirith-notary`; aceptada por Apple el 2026-10-29).
- [ ] Muse protocol probe: read muse serve help and schema
- [x] Chat de la app: adjuntar archivos con el clip y arrastrando funciona de verdad en la app de escritorio, y sin scroll horizontal. (hecho: bd92d3b)
  - [x] Revisar: Chat de la app: adjuntar archivos con el clip y arrastrando funciona de verdad en la app de escritorio, y sin scroll horizontal. — falló: the merge did not go through: merging lane/app-de-escritorio-tauri-con-chat-7 failed conflicts? : Auto-merging app/README.md CONF
- [x] Los mensajes del chat nunca se cortan ni se salen por la derecha: texto, rutas y código largos se parten o tienen su propio scroll dentro del bloque. (hecho: a71d1cb)
- [ ] Probar a mano en la app Tauri real (npm run tauri dev): clip, arrastrar y sin scroll horizontal; no hubo sesión gráfica
- [x] Integrar en main la rama lane/chatoverflow (mensajes del chat sin cortes). (hecho: a71d1cb)
- [x] Llevar a main el cambio de la lane app-de-escritorio-tauri-con-chat-7 (clip, arrastrar, sin scroll horizontal; tests ya verdes): reaplicar a mano /Users/andreseloyasanchezva/.tirith/diffs/app-de-escritorio-tauri-con-chat-7.patch sobre main, sin rediseñar, y resolver el conflicto en app/README.md conservando lo de main. (hecho: bd92d3b)
  - [x] Revisar: Llevar a main el cambio de la lane app-de-escritorio-tauri-con-chat-7 (clip, arrastrar, sin scroll horizontal; tests ya verdes): reaplicar a mano /Users/andreseloyasanchezva/.tirith/diffs/app-de-escritorio-tauri-con-chat-7.patch sobre main, sin rediseñar, y resolver el conflicto en app/README.md conservando lo de main. — falló: the tests failed: …tauri-con-chat-8/collector/.venv/lib/python3.12/site-packages/_pytest/pathlib.py:123: PytestWarning
- [ ] La app de escritorio se recompila y se reinicia sola cuando cambia su código Rust en main (firmada), sin que el dueño tenga que hacer nada.
- [x] Integrar en main el arreglo de adjuntos (clip y arrastrar) de la lane app-de-escritorio-tauri-con-chat-8, hoy sin commit. (hecho: bd92d3b)
- [x] Arrastrar archivos al chat funciona de verdad en la app de escritorio, y la miniatura adjunta se ve bien (✕ pequeño dentro de la esquina, nombre y tamaño). (hecho: 931214a)
- [x] La app de macOS funciona sola en un Mac limpio: incluye el collector y la web, y en el primer arranque instala lo que falte (uv) con un botón. (check: `bash scripts/check-standalone-app.sh`) (hecho: 79cb84d)
- [x] Llevar a main lane/standalone (bf1c2de): el elemento de integrar se marcó hecho con 09cf8bd, un commit del plan, y main no tiene scripts/check-standalone-app.sh ni standalone.rs. (check: `git cat-file -e main:scripts/check-standalone-app.sh && git cat-file -e main:app/src-tauri/src/standalone.rs`) (hecho: 1316f44)
- [x] Integrar en main la rama lane/standalone (bf1c2de, app sola en un Mac limpio) con bash scripts/check-standalone-app.sh en verde. (check: `git cat-file -e main:scripts/check-standalone-app.sh && git cat-file -e main:app/src-tauri/src/standalone.rs`) (hecho: 09cf8bd)

## 10. Plan visible en el Overview

- [x] El collector lee este archivo (casillas por sección) y calcula el progreso de cada paso.
- [x] `tirith lane run --step <n>`: cada lane dice en qué paso trabaja.
- [x] Tarjeta «Plan» en el Overview: pasos, progreso, estado y agentes trabajando en cada uno.
- [x] Fecha límite opcional con cuenta atrás.
- [x] Cabecera de un paso del Plan con varios agentes: avatares apilados (+N) y un solo botón, sin tapar el título ni la barra de progreso. (hecho: ce69997)
- [x] Reaplicar sobre main solo el cambio de cabecera de paso de lane/stepheader (4 archivos: git diff --binary 876bf81 cb7e35b | git apply en un worktree nuevo desde main), commit y test web (check: `git cat-file -e main:web/js/planagents.test.mjs && git show main:web/css/style.css | grep -q pavas`) (hecho: 9b1b249)
- [x] Quitar la vista Flow del menú y su código (el dueño la considera inútil). (hecho: 360eded)
- [ ] El test web/js/planagents.test.mjs no reescribe la captura versionada web/screenshots/plan-many-agents.png al pasar: la escribe en un temporal, y solo la actualiza con una orden aparte. (check: `! grep -q "screenshots" web/js/planagents.test.mjs`)

## 11. Lanzar un sub-agente desde el Plan

- [x] `tirith launch --project <id> --step <n>`: worktree, brief generado de las tareas abiertas, lane en segundo plano.
- [x] Enlaces `tirith://` con un manejador local (sin servidores) y token para que ninguna otra web lance agentes.
- [x] Botón «Terminar con un agente» en cada paso no terminado del Plan.
- [x] Registrar el manejador `tirith://` en macOS.
- [x] Probar un lanzamiento real desde el navegador con el enlace `tirith://` (probado por un agente el 2026-10-29: el enlace llega al inbox y queda en el log del handler).
- [x] Al terminar un lane lanzado así: botón «Revisar y unir» (tests + merge).
- [x] Buzón del dueño: «Termínalo» y «¿Qué pasa?» en cada paso; el aviso llega al steward.
- [x] Respuestas bajo cada paso (steward, o Muse Spark automático si el steward no responde en 2 min).
- [x] El steward vigila el buzón desde su sesión y actúa al instante.
- [x] Markdown en todo el texto de los agentes (respuestas, objetivos, mensajes).

## 12. «Tu turno»: llamadas a la acción claras

- [x] Cada aviso trae opciones de un clic y una caja para escribir; la respuesta llega al steward por el buzón.
- [x] El steward revisa cada informe de lane y resuelve lo que puede; al dueño solo llega lo que necesita su mano.
- [x] Lanes fallidos: «Reintentar» / «Ver el log» / «Descartar».
- [x] Despachador en el collector: «Termínalo» lanza el agente (o responde por qué no); «¿Qué pasa?» lo responde Muse Spark al momento.
- [x] Hooks del steward: le inyectan las peticiones abiertas y le impiden terminar el turno sin responderlas.
- [x] Escalado: petición sin respuesta en 10 min → notificación de macOS y opciones para el dueño.
- [x] Comandos para el dueño como acciones con botón «Run» (confirmación, salida y resultado en la UI).
- [x] Regla en código: una respuesta con un comando de terminal en el texto se rechaza; hay que usar `--run`.
- [x] Todo build y test pasa por `tirith run` (bloqueo en código) y el Overview muestra también los pipelines recién terminados.
- [x] Cada lane terminado se revisa por código: tests como pipeline visible, merge automático si pasan, hook que impide al steward terminar con revisiones pendientes, violación a los 30 min.
- [x] Todo aviso al dueño tiene al menos una opción o acción (regla en código); los avisos automáticos (guarda, checks) traen su botón.
- [x] Botón «Stop» para parar un lane desde la UI.
- [x] Vista Pipelines al estilo GitLab: cada ejecución en una fila con sus etapas como círculos, tooltip por etapa y quién la lanzó.
- [x] Guarda compatible con hooks existentes (encadenar el `pre-commit` de ChromaSol).
- [x] Un resultado de acción del dueño reabre la petición para el manager (aviso al instante y respuesta obligatoria).
- [x] Renombrar «steward» a «manager» en toda la UI, la CLI y la documentación.
- [x] Settings: modelo y esfuerzo del manager y de los agentes que crea (por defecto, por proyecto y por paso).

## 13. Configuración del proyecto visible y editable

- [x] Tarjeta «Setup» por proyecto: etapas (build / test / deploy) con su último resultado, hooks, reglas y `tirith check`.
- [x] Reglas propias en `tirith.toml` (`[[rules]]`), aplicadas por las puertas y validadas por `tirith check`.
- [x] Crear etapas y reglas pidiéndolo (chat o formulario «+ Etapa» / «+ Regla»); la UI se actualiza sola.
- [x] Aviso con un clic cuando las reglas no están activas en la máquina.
- [x] Setup explica por qué una regla no está activa (agente sin `settings`, si puede ejecutarse sin reglas y cómo arreglarlo).
- [x] Settings (y todas las vistas) con el mismo margen interior que la cabecera: el texto de introducción y las tarjetas no pegados al borde izquierdo. (hecho: 610281d)

## 14. Manager de guardia, chat y piloto automático

- [x] Manager que vive dentro de Tirith (`managerd`): arranca de guardia, y sus fallos van a la cola del manager.
- [x] Los avisos de «Tu turno» llegan al chat como mensajes del manager; solo se muestra lo que el manager se niega a hacer.
- [x] Un mensaje del chat nunca se pierde; el indicador de escritura dice la verdad; fotos en el chat del manager.
- [x] Piloto automático con cuentas verdaderas (agentes trabajando / en cola); pausa los pasos que no convergen.
- [x] Añadir un proyecto desde la app con selector de carpeta.
- [x] Editar, borrar o añadir elementos del Plan con un botón; el plan se vuelve a comprobar después.
- [x] `state.json` pequeño y rápido (detalle del agente bajo demanda); `tirith` se encuentra fuera de una terminal.
- [x] La app de escritorio muestra la web actual sin recompilar y arranca el collector aun abierta desde Finder.
- [x] Variables de entorno por proyecto, editables en Settings.
- [x] Los lanes no llenan el disco: `target` de cargo compartido, worktrees ya unidos borrados, guarda de espacio libre.
- [x] Referencias a archivos clicables en el chat; la caja del chat crece con el texto.
- [x] Cerrar la app detiene todo el proceso del collector; un segundo SIGTERM durante el cierre se ignora; un archivo de bloqueo decide si ya hay un collector.
- [x] Los nombres de lanes en el chat del manager son enlaces que abren el lane.
- [x] Los enlaces web del chat se abren en el navegador predeterminado desde la app de escritorio.
- [x] Las opciones largas de respuesta se ajustan en varias líneas; cualquier archivo se puede arrastrar al chat o adjuntar.
- [x] Comprobar en pantalla que el piloto automático y la lista «Agents» coinciden (comprobado con captura el 2026-10-29).
- [x] El turno del manager termina al responder; sus sub-agentes arrancan desacoplados y cada lane tiene su propia carpeta.
- [x] El chat marca «Delivered» lo que la app aceptó (nunca «Not sent»); rutas de archivo clicables; la caja crece con el texto.
- [x] Las imágenes arrastradas a la app llegan al chat.
- [x] El collector no muere por un ciclo fallido, la app lo reinicia si muere y la web avisa si los datos están viejos.
- [x] El piloto automático dice por qué no arranca un paso.
- [x] Las lanes no llenan el disco (build de Rust compartido, carpetas fusionadas borradas, freno con poco espacio).
- [x] Variables de entorno por proyecto en Settings.
- [x] Todas las reglas activas: codex y copilot protegidos de verdad (no solo explicado).
- [x] Tests fiables: sin fallos al azar dentro de las lanes; las revisiones atascadas se vuelven a lanzar.
- [x] Al dueño solo le llega lo que solo él puede hacer, con el motivo (credenciales, pago, cuenta, aparato o decisión).
- [x] Instrucciones al dueño concretas y prerrellenadas, con botón «Open in Terminal»; se cierran solas al comprobarse.
- [ ] Un solo manager con sesión persistente que vive en Tirith (misma memoria con la app abierta o cerrada). (reabierto: the merge did not go through: merging lane/manager-de-guardia-chat-y-piloto-2 failed conflicts? : Auto-merging collector/tests/te)
  - [x] Revisar: Un solo manager con sesión persistente que vive en Tirith (misma memoria con la app abierta o cerrada). — falló: the merge did not go through: merging lane/manager-de-guardia-chat-y-piloto-2 failed conflicts? : error: Your local changes to th
- [x] El chat muestra el trabajo del manager en vivo: texto, herramientas y resultados de cada paso (secretos tapados).
- [ ] El chat del manager como el CLI: texto palabra a palabra, diffs en color, lista de tareas; Esc interrumpe y ↑ recupera.
- [ ] Sub-agentes anidados en la conversación; permisos seguros aprobados por código y el resto como botones Allow/Deny.
- [x] Autorrevisión cada minuto (collector, manager, piloto, disco, revisiones) con arreglo automático y una línea en el chat.
- [x] Lo que dice un agente en su informe se verifica contra su cambio real antes de fusionar.
- [x] El collector vuelve a tardar menos de 3 s por ciclo (hoy ~20 s) y un test lo vigila. (hecho: abdf921)
- [x] Fusionar nunca es una pregunta para el dueño: si los tests y la verificación pasan, Tirith fusiona solo; un duplicado o conflicto va a la cola del manager. Nunca dos agentes en el mismo elemento. (hecho: cd1b9c1)
- [x] Regla en código, sin excepciones: ningún agente arranca sin estar ligado a un elemento abierto del plan (revisiones y revisiones del plan incluidas: Tirith añade su elemento). (hecho: 222849b)
- [x] Regla en código: ningún agente termina sin informar a Tirith del resultado de su elemento (hecho / en parte + nuevos elementos / bloqueado + motivo); Tirith es el único que escribe el plan y marca «hecho» solo cuando la revisión pasa. (hecho: 222849b)
- [ ] Llevar a main el commit 0a71483 de la rama lane/chat-wrap-long-reply-buttons-dra-2 (botones de respuesta largos que se ajustan, adjuntar o arrastrar cualquier archivo al chat): resolver los conflictos con main y pasar los tests
- [x] El collector se reinicia solo cuando cambia su código en main (así ningún cambio fusionado queda sin cargar). (hecho: 1df0b8d)
- [x] El piloto automático no lanza agentes: avisa al manager en el chat de cada paso listo y el manager decide (lanzar con buenas instrucciones, juntar, esperar o descartar). (hecho: 634c8fe)
- [x] Tirith libera disco solo cuando queda poco (carpetas de lanes ya fusionadas y cachés de compilación) y nunca se lo pide al dueño. (hecho: 4ef849a)
- [x] El manager persistente arranca siempre: comparte el elemento fijo del manager con los turnos sueltos (sin bloquearse por «ya en curso») y, si falla, el modo de respaldo responde en menos de 1 minuto. (hecho: b70ce39)
- [ ] Los permisos de comandos seguros (solo lectura, comandos de Tirith) se aprueban por código; al dueño nunca le llega una pregunta de permiso que no sea suya.
- [ ] La autorrevisión no escribe en el chat lo que arregló sola: queda en el punto de salud (al hacer clic); el chat solo muestra lo que necesita al dueño.
- [ ] Llevar a main el trabajo de la lane manager-de-guardia-chat-y-piloto-2: aplicar a mano ~/.tirith/diffs/manager-de-guardia-chat-y-piloto-2.patch sobre main y resolver los conflictos en collector/tirith/agentrun.py y collector/tirith/managerd.py conservando lo que ya hay en main (los tests de la lane pasaban: 1113). (en revisión)
  - [x] Revisar: Llevar a main el trabajo de la lane manager-de-guardia-chat-y-piloto-2: aplicar a mano ~/.tirith/diffs/manager-de-guardia-chat-y-piloto-2.patch sobre main y resolver los conflictos en collector/tirith/agentrun.py y collector/tirith/managerd.py conservando lo que ya hay en main (los tests de la lane pasaban: 1113). — falló: the merge did not go through: the review was interrupted; run it again
- [x] Bucle «sigue hasta terminar» por código: si al acabar falta informe, verificación, tests o rama lista, Tirith reanuda la misma sesión con la lista de lo que falta (hasta 3 veces) y solo después avisa al manager. (hecho: 4e04435)
- [x] Cola única de fusión: los agentes no tocan main; Tirith fusiona de uno en uno (trae main, suite completa, fusiona, empuja) y un conflicto vuelve al mismo agente. (hecho: 46fe181)
- [x] Elementos pequeños con su comprobación: el manager parte cada pedido en elementos de ~30 min con un comando de «hecho»; Tirith rechaza elementos enormes. (hecho: dda7276)
- [x] Integrar en main todo el trabajo terminado y sin fusionar de las lanes (lista del 2026-10-29) y descartar con motivo lo reemplazado. (hecho: a5063a4)
- [ ] README y docs/SUBMISSION.md al día para el jurado del hackathon: qué hace Tirith hoy (autonomía con reglas en código, plan, manager en el chat, app notarizada), cómo usa los modelos de Meta, cómo probarlo y capturas reales. (en revisión)
- [x] Quitar todo el GDScript: la carpeta world/ (ciudadela 3D de Godot), ./run --3d y toda referencia a Godot en el código, la web y los docs. (hecho: f936c0c)
- [x] Integrar en main lane/smallitems (elementos pequeños con comprobación; ya commiteado, cortado por el límite de 1 h antes de fusionar). (hecho: dda7276)
- [x] Integrar en main las lanes terminadas teach, mergeq, fasttests, pilotask, idletimeout y diskauto (resolviendo conflictos). (hecho: b2401d0)
- [x] Landing page pública de Tirith (GitHub Pages): qué es, qué hace, cómo usa los modelos de Meta, capturas reales y cómo probarlo. (hecho: 6f05bca)
- [x] Vídeo de demo de 3 min del Tirith actual (manager en el chat, plan, reglas en código, agentes trabajando) grabado por script, más un GIF corto para el README. (hecho: 679d021)
- [x] Repo preparado para conseguir estrellas: README con GIF y estrella arriba, badges, inicio en una línea, imagen social, CONTRIBUTING y plantillas de issues. (hecho: aa90394)
- [x] Licencia MIT (elegida por el dueño): archivo LICENSE, campo license en los manifiestos y badge en el README. (hecho: b740efc)
- [x] La app reinicia el collector si el estado deja de actualizarse aunque el proceso siga vivo. (check: `cd app/src-tauri && cargo test -q staleness`) (hecho: c4775c9)
- [ ] El recargado automático del collector nunca lo deja bloqueado. (check: `cd collector && uv run -q pytest -q tests -k reload`)
- [x] Tras arrancar o recargar, el collector escribe un estado útil en menos de 30 s. (check: `cd collector && uv run -q pytest -q tests -k first_cycle`) (hecho: bb947e4)
- [ ] Quitar worktrees de lanes sin chocar con el límite 'git': 'git worktree remove' (diskdiet.py, review.py) usa su propia clase de límite 'git-worktree-remove' (minutos, no los ~9 s aprendidos de git), con el test test_worktree_remove_uses_own_timeout_class (check: `cd collector && python3 -m pytest -q tests -k test_worktree_remove_uses_own_timeout_class`)
- [ ] Hacer robusto el test test_context_over_the_limit_compacts_then_restarts_with_a_handoff de test_msession.py, que falla bajo carga porque Rig.until espera 25 s de reloj real (check: `cd collector && uv run pytest tests/test_msession.py -q -k compacts_then_restarts`)
- [ ] La landing page tiene animaciones al hacer scroll (aparición suave de secciones, capturas y tarjetas) respetando «reducir movimiento». (check: `node web/js/landing.test.mjs`) (en revisión)
- [x] Logo de Tirith en 3D renderizado con Blender y renders 3D en la landing page. (check: `test -f site/img/logo-3d.png`) (hecho: 7fabf84)
- [ ] Registrar en la revisión de una lane la causa cuando se interrumpe (señal o código de salida del proceso), con un test que mata el proceso de la revisión y comprueba la causa registrada. (check: `cd collector && uv run pytest -q -k review_interrupted_records_cause`)
- [x] Volver a grabar el vídeo de demo con la interfaz actual (sin Flow, logo 3D) y cada subtítulo sobre la escena que describe. (check: `test -f docs/demo/out/tirith-demo.mp4`) (hecho: bb07913)
- [x] El límite de tamaño de una lane cuenta solo su propio cambio (contra la base con main), no lo que trajo al fusionar main. (check: `cd collector && uv run -q pytest -q tests -k budget`) (hecho: 84c9ea6)
- [ ] Hacer robusto el test test_second_message_reaches_the_same_process_and_waits_for_the_turn_to_end de test_msession.py, que falla bajo carga como el de compacts (espera de reloj real) (check: `cd collector && uv run pytest tests/test_msession.py -q -k second_message_reaches_the_same_process`)
- [ ] Rehacer sobre main el commit 8b4bf7c de lane/manager-de-guardia-chat-y-piloto-3 (una conversación del manager entre turnos headless, 5 archivos) en un worktree nuevo, con test por etapa e informe con el número exacto de tests (check: `git show main:collector/tests/test_managerd.py | grep -q test_headless_turns_share_one_session && git show main:collector/tirith/managerd.py | grep -q 'def adopt_session'`)
- [x] El piloto automático manda como mucho una propuesta abierta por paso. (check: `cd collector && uv run -q pytest -q tests -k autopilot`) (hecho: 62a63b6)
- [x] Los mensajes del dueño pasan delante: el manager atiende cada mensaje del chat antes que las propuestas del piloto. (check: `cd collector && uv run -q pytest -q tests -k managerd`) (hecho: 5754f53)
- [ ] web/js/landing.test.mjs carga playwright-core del propio repo (o se salta con aviso si no está), nunca de node_modules de otro proyecto como propuestavenezuela (check: `node web/js/landing.test.mjs`)
- [ ] tirith plan done acepta un commit que solo toca docs cuando el elemento mismo es de docs (README, docs/…), con un test (check: `cd collector && uv run -q pytest -q tests -k plan_done_docs`)
- [ ] Un worktree de lane nunca se registra como proyecto automático: si git falla o vence el límite al resolver la raíz (_in_main_worktree en projects.py), no se guarda en caché ni se registra la carpeta. (check: `cd collector && uv run -q pytest -q tests -k worktree_never_auto_project`)
- [x] El icono de la app de macOS se genera a partir del logo 3D. (check: `grep -q logo-3d scripts/make_icon.py`) (hecho: 7bcd959)
- [x] El logo 3D es una torre de piedra con base circular, renderizada en Blender con texturas de piedra reales y calidad de portada, con almenas de piedra y sin la esfera flotante (pedido del dueño). (check: `test -f site/img/logo-3d.png`) (hecho: 7ff25cf)
- [x] La captura principal de la landing muestra la app actual con datos de demo limpios: sin avisos, sin reglas inactivas y sin Flow. (check: `node web/js/landing.test.mjs`) (hecho: fadd25d)
- [x] Los renders 3D de la landing (torre con agentes y puerta de reglas) usan la torre de piedra nueva con el mejor render y la mejor luz posibles, más contraste con el fondo y sin cortes en los bordes (pedido del dueño). (check: `node web/js/landing.test.mjs`) (hecho: 7bcd959)
- [x] En el móvil, el texto bajo la captura principal de la landing no se monta sobre la imagen. (check: `node web/js/landing.test.mjs`) (hecho: fadd25d)
- [x] La imagen social (social-preview-3d.png) usa el mismo modelo de torre de piedra que el logo. (check: `grep -q stone_tower site/3d/social.py`) (hecho: 7bcd959)
- [ ] Evidencia de la torre de piedra: capturas de la landing publicada en escritorio y móvil, antes y después, en docs/evidence/logo-torre/. (check: `cd docs/evidence/logo-torre && test -f antes-escritorio.png && test -f antes-movil.png && test -f despues-escritorio.png && test -f despues-movil.png`)
- [x] La landing deja de mostrar render-board.png (pedido del dueño). (check: `! grep -q render-board site/index.html`) (hecho: 7ecbd87)
- [ ] Imágenes de inspiración para los renders, generadas con un modelo de imagen de Google y guardadas fuera del repo (espera: una clave de Google que funcione, del dueño). (check: `test -n "$(ls ~/.cache/tirith-inspiration 2>/dev/null)"`)
- [ ] El icono de la app de macOS (el mismo modelo de torre de piedra) se lee bien a 32 px: se genera una versión pequeña y se comprueba su contraste (pedido del dueño). (check: `test -f app/src-tauri/icons/32x32.png && grep -q stone_tower scripts/make_icon.py`)
- [ ] En el móvil, el diagrama de la landing cabe entero sin desplazarse de lado. (check: `node web/js/landing.test.mjs`)
- [ ] En el móvil, los bloques de código de la landing se leen enteros sin desplazarse de lado. (check: `node web/js/landing.test.mjs`)
- [ ] El texto de estado de la landing dice la verdad de hoy: ya no pone en construcción lo que ya funciona. (check: `! grep -q 'under construction' site/index.html`)
- [ ] El README abre con la torre de piedra y capturas actuales de la app. (check: `grep -q -E 'logo-3d|social-preview-3d' README.md`)
- [ ] El video demo se regraba con el logo de piedra y la app actual. (check: `test site/img/logo-3d.png -ot docs/demo/out/tirith-demo.mp4`)
- [x] El logo de piedra tiene luz de portada: luz cálida lateral, contraluz que recorta la silueta y la ventana iluminando la piedra de alrededor (pedido del dueño). (check: `test -f site/img/logo-3d.png`) (hecho: 7bcd959)

## 15. Collector y CLI en Rust

- [x] Base: crate Rust compartido (workspace con la app), arnés de paridad que corre el collector de Python y el de Rust sobre las mismas carpetas de prueba y compara `state.json`.

## Orden

1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9, y 10 en cuanto se pidió.

Hasta el 4 de noviembre (entrega del hackathon): activar y proteger los hooks (3), el bloqueo de
sub-agentes fuera de `lane run` (7), los logs legibles (8) y el plan en el Overview (10).

## 16. Velocidad y autonomía: de un objetivo grande a resultados sin el dueño

- [ ] `tirith do "<objetivo>"` convierte un objetivo grande en elementos pequeños del plan, cada uno con su comprobación, y los lanza. (check: `cd collector && uv run -q pytest -q tests -k tirith_do`)
- [ ] El planificador no lanza a la vez dos lanes que tocan los mismos archivos. (check: `cd collector && uv run -q pytest -q tests -k conflicts`)
- [ ] La cola de merge prueba varios lanes juntos en una sola pasada de tests y separa al culpable si falla. (check: `cd collector && uv run -q pytest -q tests -k mergetrain`) (en revisión)
- [ ] Cada elemento terminado muestra su evidencia en la app: salida de tests y capturas si cambia la interfaz. (check: `cd collector && uv run -q pytest -q tests -k evidence`)
- [ ] La app muestra cuánto tarda un elemento en llegar a main, cuántos llegan por hora y cuántas veces hizo falta el dueño. (check: `cd collector && uv run -q pytest -q tests -k metrics`)
- [ ] Cada lane recibe en su brief un mapa del código con los archivos relevantes para su elemento. (check: `cd collector && uv run -q pytest -q tests -k codemap`)
- [ ] `tirith plan add` muestra el id corto del elemento y todos los comandos lo aceptan. (check: `cd collector && uv run -q pytest -q tests -k item_id`)
- [ ] El dueño recibe un resumen visual de lo que llegó a main, con un botón de opinión que se convierte en elementos del plan. (check: `cd collector && uv run -q pytest -q tests -k digest`)

## 17. Tirith se mejora usando Tirith

- [x] Tirith se ejecuta desde una copia estable que solo avanza cuando la prueba completa pasa en la versión nueva, y vuelve atrás sola si la salud se pone roja. (check: `cd collector && uv run -q pytest -q tests -k stable`)
- [ ] Tirith registra cada vez que hizo falta una persona o algo esperó, y pone la fricción mayor arriba del plan. (check: `cd collector && uv run -q pytest -q tests -k friction`)
- [ ] La cola de merge tiene prioridad: los arreglos de la propia cola y de sus etapas entran primero. (check: `cd collector && uv run -q pytest -q tests -k merge_priority`)
- [x] Las instrucciones de los lanes dicen quién revisa de verdad: Tirith hace el commit y la cola de merge revisa, nunca «para que lo revise el dueño». (check: `! grep -rq 'for the owner to review' collector/tirith`) (hecho: 39e0c0b)
- [x] Los lanes que lanza Tirith desde la app ven las variables de entorno del dueño (claves como GITHUB_TOKEN o GEMINI_API_KEY de su shell), sin copiarlas a ningún archivo del repo. (check: `cd collector && uv run -q pytest -q tests -k owner_env`) (hecho: 7e9adbc)
- [ ] Los límites de tiempo de los comandos crecen con la carga de la máquina, así git lento con carga alta no pone la salud en rojo. (check: `cd collector && uv run -q pytest -q tests -k load_scaled`)
- [ ] La cola de merge puede cancelar el merge en curso cuando otro lane lo hace innecesario. (check: `cd collector && uv run -q pytest -q tests -k merge_cancel`)
- [ ] Un lane vivo que espera su etapa de tests no cuenta como parado: ni el estado ni el autopilot lo marcan stalled solo porque su log lleva 10 minutos sin cambiar. (check: `cd collector && uv run -q pytest -q tests -k stalled_while_tests_run`)
- [x] La cola de merge no descarta un merge ya probado cuando main solo cambió en el plan: incorpora esos commits y sigue. (check: `cd collector && uv run -q pytest -q tests -k plan_only`) (hecho: b9f21d9)
- [x] Todas las llamadas a git usan el límite de la política de timeouts, sin límites fijos en el código. (check: `cd collector && uv run -q pytest -q tests -k no_fixed_git_timeout`) (hecho: 9786a90)
- [x] El manager se compacta cuantas veces haga falta y solo se reinicia si compactar no basta, como mucho una vez por hora. (check: `cd collector && uv run -q pytest -q tests -k compact`) (hecho: 9dfb0a7)
- [ ] El brain solo se llama cuando cambia lo que tiene que juzgar, nunca para una lista vacía ni en cada ciclo del collector. (check: `cd collector && uv run -q pytest -q tests -k brain_calls`) (en curso: u-braincalls)
- [x] Tren de integración A: todos los lanes terminados del collector, la cola y las reglas, juntos en una rama probada una sola vez. (check: `git merge-base --is-ancestor lane/u-mqplanmoves main`) (hecho: 8e3a1a9)
- [ ] Tren de integración B: todos los lanes terminados de la app, la web y la landing, juntos en una rama probada una sola vez. (check: `git merge-base --is-ancestor lane/u-stonetower main`) (en curso: u-trainB)
- [x] Al devolver un lane, la cola de merge reabre su elemento del plan y comprueba que el agente arrancó. (check: `cd collector && uv run -q pytest -q tests -k send_back`) (hecho: 9a0d35c)
- [x] La ronda de seguimiento de un lane (keep-going) arranca su agente sin que la regla agent-cli la bloquee, y un lane con informe done no termina como failed por ese bloqueo. (check: `cd collector && uv run -q pytest -q tests -k keepgoing_resume_not_blocked`) (hecho: 1abd541)
- [ ] Tirith sube main a GitHub como mucho 5 minutos después de cada cambio del plan. (check: `cd collector && uv run -q pytest -q tests -k push_plan`)
- [x] La cola de merge distingue un git cortado por tiempo de un conflicto real, y reintenta el primero sin devolver el lane. (check: `cd collector && uv run -q pytest -q tests -k merge_timeout`) (hecho: 1589d3d)
- [ ] Un lane ya mezclado a mano sale de la cola y de las revisiones sin intervención. (check: `cd collector && uv run -q pytest -q tests -k already_merged`)
- [x] i-aef081ed (hecho: fde18e0)
- [ ] Dos ejecuciones simultáneas de etapas en worktrees distintos no comparten número de ejecución ni fichero de log. (check: `cd collector && uv run -q pytest -q tests -k concurrent_runs_separate_logs`)
- [ ] Un lane no deja su item marcado como hecho en el plan antes de que su trabajo entre en main. (check: `cd collector && uv run -q pytest -q tests -k item_stays_open_until_merged`)
- [ ] tirith check: _coverage_problems in manifest.py counts pipelines from include files, so a project whose build or test stages exist only in an include passes the check while the merge queue finds no stage to run; count only gating pipelines
- [ ] La cola de merge reintenta el commit de la lane cuando encuentra un index.lock abandonado en su worktree, en vez de marcarla red-suite y devolverla (check: `cd collector && uv run -q pytest -q tests -k commit_workspace_recovers_lock`)
- [ ] Una lane de continuación que reutiliza el worktree de otra entra sola en la cola de merge al terminar con done (check: `cd collector && uv run -q pytest -q tests -k continuation_lane_auto_enqueued`)
- [x] Pasar un solo test tarda menos de 30 segundos también con carga alta: hoy test_code_move_is_retried tardó 8 minutos (pytest con -k, 1 passed in 493 s). (check: `cd collector && uv run -q pytest -q --durations=5 tests/test_mergeq.py -k test_code_move_is_retried`) (hecho: c906004)
- [x] tests/test_mergeq.py::test_code_move_is_retried_and_the_next_entry_still_runs no falla con carga alta: se averigua por qué el lane two queda en waiting (1 failed en ~/.tirith/logs/ci-168-test.log) y se arregla el test si es frágil o la cola si es un error real (check: `cd collector && uv run -q pytest -q tests/test_mergeq.py -k test_code_move_is_retried`)
- [ ] Devolver una lane de continuación que reutiliza el worktree de otra funciona: relaunch_lane usa la rama real de la lane (mergeq.branch_of), no lane/<id>, que hoy falla con invalid reference: lane/u-testfix-port (check: `cd collector && uv run -q pytest -q tests -k send_back_continuation`)
- [ ] La app muestra arriba, sin preguntar, qué entró hoy en main, qué se está haciendo, qué está atascado y quién lo lleva. (check: `cd collector && uv run -q pytest -q tests -k now_bar`)
- [ ] Cada petición del dueño en el chat queda ligada a un elemento del plan o a una respuesta, y la app lista sus peticiones con su estado. (check: `cd collector && uv run -q pytest -q tests -k owner_requests`)
- [ ] Al volver, el dueño ve un resumen de lo que pasó mientras no estaba: lo que entró, lo que falló y lo que sigue. (check: `cd collector && uv run -q pytest -q tests -k while_away`)
- [ ] Un ciclo del collector tarda menos de 5 segundos con carga alta: hoy llegó a 66 s, 49 s de ellos en el despachador. (check: `cd collector && uv run -q pytest -q tests -k cycle_budget`) (en revisión)
- [x] El tamaño de contexto del manager se mide con la última petición, no con la suma de todas las del turno. (check: `cd collector && uv run -q pytest -q tests -k context_tokens`) (hecho: ee2eb5a)
- [ ] Launch the collector and the app from <home>/stable/current, and run 'tirith stable promote' automatically after each merge on main
- [ ] test_code_move_is_retried_and_the_next_entry_still_runs: root cause of lane two ending in waiting under load not found (36 mergeq tests pass at 800 s under heavy load); next failing CI log now prints the rows with the waiting message, read it and fix
- [x] test_plan_done_checks_the_commit y test_the_writer_lock_serializes_changes no fallan con carga alta: en ci#175 git commit del plan salió con error y salida vacía, y solos pasan (check: `cd collector && uv run -q pytest -q tests -k plan_commit_failure_says_why`) (hecho: 48dd464)
- [x] test_canary_run_reaches_the_bare_origin_and_ticks_the_plan no falla con carga alta: en ci#177 el canary acabó en el paso push con wait_s=60 (check: `cd collector && uv run -q pytest -q tests/test_canary.py -k canary_run_reaches`) (hecho: 552afdc)
- [x] Un cambio del plan no falla con carga alta por esperar el lock: LOCK_WAIT_S de 30 s en plangate.py hizo fallar test_the_writer_lock_serializes_changes en ci#177 con «another change to the plan is taking too long» (check: `cd collector && uv run -q pytest -q tests/test_plangate.py -k writer_lock`) (hecho: 8299cd0)
- [x] Antes de rechazar un merge, la cola repite los tests fallados de uno en uno: si pasan, entra y el test queda anotado como inestable. (check: `cd collector && uv run -q pytest -q tests -k rerun_failed`) (hecho: 56448d5)
- [x] Con 3 lanes o más esperando en la cola de merge no arranca ningún lane nuevo: primero se termina lo empezado. (check: `cd collector && uv run -q pytest -q tests -k wip_limit`) (hecho: 56448d5)
- [ ] Deflake timing-sensitive tests that fail intermittently under load (test_idletimeout silent/chatty stage, test_mergeq code-move retry, test_plangate dirty plan)
- [ ] El trabajo de devolver lanes (reabrir el elemento del plan y comprobar que el agente arrancó) entra en main: la rama lane/tirith-se-mejora-usando-tirith-2 choca con main en test_mergeq.py y CONTRACT-mergeq.md, y su elemento quedó marcado como hecho con un commit de otro lane. (check: `cd collector && uv run -q pytest -q tests -k send_back_reopens_the_plan_item`)
- [ ] Un lane sin agente registrado también vuelve a su agente: la cola solo reenvía si lane.agent está en la configuración, y los lanes con agent null llegan al manager sin decir por qué. (check: `cd collector && uv run -q pytest -q tests -k send_back_without_agent`)
- [ ] El piloto automático no trata una copia de merge (_mergeq-<lane>) como proyecto: mergeq-tirith-se-mejora-usando-tirith-5 generó cinco propuestas (pasos 3, 7, 9, 10 y 14) a partir de la copia de docs/TODO.md. (check: `cd collector && uv run -q pytest -q tests -k autopilot_ignores_merge_worktree`)
- [ ] Un elemento solo se marca hecho si el lane tiene commits propios en main: una rama sin commits con trabajo sin guardar no cuenta como mezclada. (check: `cd collector && uv run -q pytest -q tests -k false_tick`) (en revisión)
- [ ] Llevar a main el arreglo de lanes devueltos de lane/tirith-se-mejora-usando-tirith-2 (06391d4): tuvo conflicto en la cola y su elemento se marcó hecho sin haber entrado. (check: `git merge-base --is-ancestor 06391d4 main`) (en revisión)
- [ ] `tirith project add` deja un proyecto listo de una vez: plan creado, tirith.toml mínimo y guarda con un clic del dueño. (check: `cd collector && uv run -q pytest -q tests -k project_ready`)
- [ ] Un merge ya probado tampoco se descarta si main solo cambió en docs/ o site/, que no afectan a los tests. (check: `cd collector && uv run -q pytest -q tests -k merge_paths`)
- [ ] La comprobación de lo que dice un lane no devuelve a un lane de solo verificar: su diff vacío es lo esperado, y los archivos que cita como hallazgos no son cambios suyos. (check: `cd collector && uv run -q pytest -q tests -k claim_check_verify_lane`)
- [ ] Tirith borra por sí mismo las carpetas temporales de tests antiguas, incluidas las de pytest-old-archive, sin esperar al dueño. (check: `cd collector && uv run -q pytest -q tests -k temp_cleanup`)
- [ ] Se sabe qué quitó la protección de los hooks del guard de Tirith a las 18:58:19Z del 30 oct y ya no puede volver a pasar. (check: `cd collector && uv run -q pytest -q tests -k guard_flags_stay`)
- [ ] Los tests de una lane corren en paralelo: hoy 41 tests de test_mergeq.py tardaron 17 minutos en serie porque el modo changed no pasa -n. (check: `cd collector && uv run -q pytest -q tests -k changed_tests_run_in_parallel`) (en curso: u-fastn)
- [ ] La suite de Python tarda menos de 6 minutos con carga alta: hoy tarda de 12 a 16, casi todo esperando y no en CPU. (check: `cd collector && uv run -q pytest -q tests -k suite_budget`) (en curso: u-suitefast)
- [ ] Tren C: las lanes que aceleran la cola (u-mqfast, -11, u-cyclefast, -14) juntas en una rama probada una sola vez, en vez de cuatro fusiones que chocan entre sí. (check: `cd collector && uv run -q pytest -q tests/test_mergeq.py -k 'train or flaky or rerun'`) (en curso: u-trainc)
- [ ] La cola no tira una fusión ya probada porque la copia principal esté sucia: aterriza si los archivos sucios no chocan, o espera sin repetir los tests. (check: `cd collector && uv run -q pytest -q tests/test_mergeq.py -k dirty_checkout`)
- [ ] Ningún test escribe en archivos versionados: web/js/planagents.test.mjs reescribe web/screenshots/plan-many-agents.png y ensució la copia principal. (check: `! grep -q 'plan-many-agents.png' web/js/planagents.test.mjs || grep -q 'UPDATE_SCREENSHOTS' web/js/planagents.test.mjs`)
- [ ] La regla workspace-write no toma las expresiones de sed -i -e como rutas de escritura (hoy bloquea /^=======$/d como si fuera un archivo). (check: `cd collector && uv run -q pytest -q tests -k workspace_write_sed_expression`)
- [ ] Un árbol que ya pasó sus tests no se vuelve a probar: Tirith recuerda el árbol probado, y ni la lane, ni la claim check, ni la cola repiten esa etapa. (check: `cd collector && uv run -q pytest -q tests -k same_tree_is_not_tested_again`)
- [x] La suite de tests de Python vuelve a pasar entera en main tras el tren de integración. (check: `cd collector && uv run -q pytest -q -n 8`) (hecho: b133b4b)

## 18. Salas privadas: hablar con el manager desde el móvil

- [x] Tirith crea una sala cifrada de agentsmessenger.com, entra con su clave y envía y lee mensajes compatibles con am.mjs. (check: `cd collector && uv run -q pytest -q tests -k sealed`) (hecho: c5e27f2)
- [ ] Lo que escribe un dispositivo del dueño en una sala llega al manager por el mismo chat, y su respuesta vuelve cifrada a la sala. (check: `cd collector && uv run -q pytest -q tests -k roombridge`) (en revisión)
- [ ] `tirith room new`, `invite` y `join` con QR de un solo uso, y state.json dice a qué salas estás conectado y quién hay en cada una. (check: `cd collector && uv run -q pytest -q tests -k private_rooms`) (en revisión)
- [ ] Vista Salas en la app: crear una sala, ver su QR, ver a qué salas estás conectado y quién hay, invitar a otro manager y salir. (check: `node web/js/rooms.test.mjs`) (en revisión)
- [ ] La cámara del iPhone escanea el QR y abre el chat cifrado en agentsmessenger.com/chat, sin instalar nada. (check: `curl -fsS https://agentsmessenger.com/chat | grep -q chat.js`)
- [ ] La cola de merge no pasa los tests de Python a un lane que solo cambia site/, ios/ o docs/. (check: `cd collector && uv run -q pytest -q tests -k merge_paths`)
- [ ] El QR de invitación de las salas privadas usa la URL https://agentsmessenger.com/chat#r=…&c=…&m=… del contrato, no otra. (check: `cd collector && uv run -q pytest -q tests -k room_invite_url`)
- [ ] Los agentes de una sala privada se hablan entre sí libremente, como dice docs/CONTRACT-private-rooms.md. (check: `cd collector && uv run -q pytest -q tests -k room_agents_talk`)
- [ ] El QR de las salas usa agentsmessenger.com/chat y los agentes de una sala hablan libres: solo se frenan tras 30 mensajes seguidos sin el dueño, como dice el contrato actualizado. (Corrige lo escrito por los lanes con el contrato anterior.) (check: `cd collector && ! grep -rq 'github.io/Tirith/chat' tirith ../web/js && uv run -q pytest -q tests -k roombridge_brake`) (en curso: u-salas)

## 19. Que los agentes de una sala no se queden callados

- [ ] La vista Salas muestra el estado real de cada agente: escuchando, ocupado o dormido desde hace N minutos, con quién lo puede despertar, para que el dueño vea de un vistazo por qué alguien no contesta en vez de adivinarlo. (check: `node web/js/rooms.test.mjs`)

