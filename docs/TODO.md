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
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [ ] CEO: revisar cómo trabaja el manager
- [ ] Manager de guardia: atender el chat (en curso: manager-1986eb06)
- [x] Revisar el plan (2026-11-05 08:38) (hecho: sin cambios) <!-- tirith:{"id":"i-32bf740b"} -->
- [x] El plan no repite ningún id permanente: Revisar el plan (2026-11-05) sigue dos veces con el id i-2975f07c, así que una recibe un id nuevo (check: `test $(grep -o 'tirith:{"id":"i-[0-9a-f]*"' docs/TODO.md | sort | uniq -d | wc -l) -eq 0`) <!-- tirith:{"id":"i-4d54f8b9","depends_on":[],"writes":[]} -->
  - [x] Revisar: El plan no repite ningún id permanente: Revisar el plan (2026-11-05) sigue dos veces con el id i-2975f07c, así que una recibe un id nuevo (check: `test $(grep -o 'tirith:{"id":"i-[0-9a-f]*"' docs/TODO.md | sort | uniq -d | wc -l) -eq 0`) — falló: the report does not match the change
  - [x] Revisar: El plan no repite ningún id permanente: Revisar el plan (2026-11-05) sigue dos veces con el id i-2975f07c, así que una recibe un id nuevo (check: `test $(grep -o 'tirith:{"id":"i-[0-9a-f]*"' docs/TODO.md | sort | uniq -d | wc -l) -eq 0`)
- [x] Verificar: Verify Drag an image from Finder onto the Tirith chat o (hecho: bafcf90e) <!-- tirith:{"id":"i-9aa639be"} -->
- [x] Do one real Finder image drag onto the Tirith chat and look for 'drag: window drop' in app.log (hecho: bafcf90e)
- [x] Cuando Tirith escribe el plan, repara los ids permanentes repetidos que ya existen: la segunda línea con el mismo id recibe un id nuevo (hoy i-2975f07c sale dos veces y ningún agente puede editar docs/TODO.md) (check: `test $(grep -o 'tirith:{"id":"i-[0-9a-f]*"' docs/TODO.md | sort | uniq -d | wc -l) -eq 0`) (hecho: 13350c57) <!-- tirith:{"id":"i-06ba8f40"} -->
  - [x] Revisar: Cuando Tirith escribe el plan, repara los ids permanentes repetidos que ya existen: la segunda línea con el mismo id recibe un id nuevo (hoy i-2975f07c sale dos veces y ningún agente puede editar docs/TODO.md) (check: `test $(grep -o 'tirith:{"id":"i-[0-9a-f]*"' docs/TODO.md | sort | uniq -d | wc -l) -eq 0`)
- [x] Revisar el plan (2026-11-05 12:31) (hecho: sin cambios) <!-- tirith:{"id":"i-fa9487dc"} -->
- [x] Revisar el plan (2026-11-05 12:27) (hecho: sin cambios) <!-- tirith:{"id":"i-afe49bed"} -->
- [x] Revisar el plan (2026-11-05 12:08) (hecho: sin cambios) <!-- tirith:{"id":"i-6911fd38"} -->
- [x] Revisar el plan (2026-11-05 09:02) (hecho: sin cambios) <!-- tirith:{"id":"i-1a9117c8"} -->
- [x] Revisar el plan (2026-11-05 07:01) (hecho: sin cambios) <!-- tirith:{"id":"i-0d103239"} -->
- [x] Revisar el plan (2026-11-05) (hecho: sin cambios) <!-- tirith:{"id":"i-2975f07c"} -->
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Revisar el plan (2026-11-05) (hecho: sin cambios) <!-- tirith:{"id":"i-19984873"} -->
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
- [x] Revisar el plan (2026-11-04) (hecho: sin cambios) <!-- tirith:{"id":"i-26b48671"} -->
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Revisar el plan (2026-10-31) (hecho: sin cambios)
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
- [x] Manager de guardia: atender el chat
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
- [x] Verificar el payload real de Muse Code y ajustar su adaptador (hecho: cc5b438)

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
- [x] Todo comando tiene timeout por código (collector, etapas y comandos de los agentes): Tirith lo aprende del historial, los agentes pueden subirlo o bajarlo dentro de límites con motivo, y al vencer se detiene el proceso con sus hijos y se limpian sus bloqueos. (hecho: 6e61d1c)
- [x] Un elemento se marca hecho en cuanto su trabajo está en main y verificado, aunque la lane se haya fusionado sola; lo ya fusionado hoy se marca con su commit. (hecho: b5ec4c1)
- [x] Llevar a main el trabajo de la lane reglas-de-c-digo-errores-de-comp: aplicar ~/.tirith/diffs/reglas-de-c-digo-errores-de-comp.patch sobre main y reportar solo lo que está en el diff (2 tests nuevos, no 3; no nombrar index.lock como archivo cambiado). (hecho: 511c1d0)
- [x] Integrar: lane/reglas-de-c-digo-errores-de-comp (duplicados por elemento y bloqueo de git abandonado). (hecho: 511c1d0)
- [x] Los agentes no piden permisos: dentro de su copia pueden ejecutar cualquier comando y solo los hooks de Tirith deciden (reglas actuales + límite: escribir solo en su copia y su temporal, nunca leer credenciales, sin servidores ni puertos, sin matar procesos ajenos). (hecho: d2ede04)
- [x] Cada bloqueo de un hook enseña la alternativa exacta y el agente sigue; varios bloqueos seguidos van al manager, nunca al dueño. (hecho: 3f9d379)
- [x] Tests rápidos en la lane (solo lo relacionado con el cambio) y suite completa en la cola de fusión; el paralelismo se ajusta a CPU y disco. (hecho: 5b1c355)
- [x] El tiempo límite de un agente mide inactividad (sin salida ni progreso durante N minutos), no el tiempo total; una lane que sigue trabajando nunca se corta por durar mucho. (hecho: 10bece7)
- [x] Repasar los 6 elementos grandes del paso 7 marcados (en revisión): lo que ya está en main se marca hecho con su commit y lo que falta se divide con tirith plan add en elementos de unos 30 minutos con su check. (check: `test $(grep -c '(en revisión)' docs/TODO.md) -lt 6`) (hecho: 10bece7) <!-- tirith:{"id":"i-ea7a2601","depends_on":[],"writes":[]} -->
- [x] Un agente puede subir o bajar el timeout de un comando dentro de límites dando un motivo que Tirith registra, porque reglas-8 marcó hecho el elemento grande sin verificar esta parte. (check: `cd collector && uv run -q pytest -q tests -k timeout_override`) (hecho: e979e66) <!-- tirith:{"id":"i-173a70ef","depends_on":[],"writes":[]} -->
  - [x] Revisar: Un agente puede subir o bajar el timeout de un comando dentro de límites dando un motivo que Tirith registra, porque reglas-8 marcó hecho el elemento grande sin verificar esta parte. (check: `cd collector && uv run -q pytest -q tests -k timeout_override`)
- [x] Tirith corta un agente solo tras N minutos sin salida ni progreso, nunca por el tiempo total: una lane que sigue escribiendo salida no se corta por durar mucho. (check: `cd collector && uv run -q pytest -q tests -k idle_timeout`) <!-- tirith:{"id":"i-6f47c8a6","depends_on":[],"writes":[]} -->
  - [x] Revisar: Tirith corta un agente solo tras N minutos sin salida ni progreso, nunca por el tiempo total: una lane que sigue escribiendo salida no se corta por durar mucho. (check: `cd collector && uv run -q pytest -q tests -k idle_timeout`)
- [x] Al vencer el timeout de un comando de una etapa o de un agente, Tirith detiene el proceso con todos sus hijos y limpia sus bloqueos. (check: `cd collector && uv run -q pytest -q tests -k timeout_kills_tree`) (hecho: 0438f8e) <!-- tirith:{"id":"i-6caa45d6","depends_on":[],"writes":[]} -->
  - [x] Revisar: Al vencer el timeout de un comando de una etapa o de un agente, Tirith detiene el proceso con todos sus hijos y limpia sus bloqueos. (check: `cd collector && uv run -q pytest -q tests -k timeout_kills_tree`)
- [x] Los tests limpian sus carpetas temporales, incluidos los hooks bloqueados que crean, para no llenar el disco. (check: `cd collector && uv run -q pytest -q tests -k cleanup`) (hecho: 46f4696)
- [x] La cola de merge arranca sola desde el collector: ningún lane terminado espera en la cola sin un proceso de merge. (check: `cd collector && uv run -q pytest -q tests -k mergeq`) (hecho: fbb405e)
- [x] La cola de merge usa la rama real de cada lane, la de su worktree. (check: `cd collector && uv run -q pytest -q tests -k mergeq`) (hecho: 9fea4fd)
- [x] Cada fallo de la cola de merge vuelve con su motivo al lane o al manager. (check: `cd collector && uv run -q pytest -q tests -k mergeq`) (hecho: 5b35e1b) <!-- tirith:{"id":"i-ca536736"} -->
  - [x] Revisar: Cada fallo de la cola de merge vuelve con su motivo al lane o al manager. (check: `cd collector && uv run -q pytest -q tests -k mergeq`)
- [x] i-3a1bd406 (hecho: e95c9f2)
- [x] Tirith guarda una copia del brief de cada lane en su carpeta, así un lane cortado se puede retomar tras un reinicio. (check: `cd collector && uv run -q pytest -q tests -k brief`) <!-- tirith:{"id":"i-a6a4050c"} -->
  - [x] Revisar: Tirith guarda una copia del brief de cada lane en su carpeta, así un lane cortado se puede retomar tras un reinicio. (check: `cd collector && uv run -q pytest -q tests -k brief`)
- [x] Ninguna petición de permiso del manager llega al dueño: el código la permite o la bloquea según las reglas. (check: `cd collector && uv run -q pytest -q tests -k msession`)
  - [x] Revisar: Ninguna petición de permiso del manager llega al dueño: el código la permite o la bloquea según las reglas. (check: `cd collector && uv run -q pytest -q tests -k msession`) — falló: the merge did not go through: the review was interrupted; run it again
- [x] Cada hora un lane de prueba recorre todo el camino en un repo de prueba, del plan al push, y la salud se pone roja si no llega. (check: `cd collector && uv run -q pytest -q tests -k canary`) (hecho: b69b549)
- [x] La suite de tests de Python termina en menos de 4 minutos. (check: `bash scripts/check-test-time.sh`) (hecho: 31bc010)
- [x] Ningún test de Python falla al azar: los tests inestables se arreglan. (check: `bash scripts/check-flaky.sh`) (hecho: 3945182)
- [x] Las etapas que bloquean merges ya no compilan ni prueban el port a Rust parado. (check: `! grep -qE 'tirith-core|tirith-cli|--workspace' tirith.toml`) (hecho: 5d8c719)
- [x] Llevar a main el trabajo de lane/u-keepalive (un lane no termina mientras su etapa de tests corre): hoy marcado hecho con ec74885, que es un commit del plan, y main no tiene el test. (check: `git merge-base --is-ancestor e5acf09 main`) (hecho: e5acf09)
- [x] i-3af30eb5 (hecho: 8e3a1a9)
- [x] El control de escrituras Bash del manager deja leer con git (branch --list, worktree list, stash list, tag -l) y escribir briefs en /tmp, y bloquea git -c x=y commit, merge y push. (check: `cd collector && uv run -q pytest -q tests/test_msession.py -k manager_bash_reads_and_git_options`) (hecho: ca1aa39)
- [x] La regla workspace-write no bloquea por error: resuelve variables de shell como $S/x hacia el temporal del lane, permite 2>/dev/null y no toma como destino de escritura los argumentos de grep tras una tubería. (check: `cd collector && uv run -q pytest -q tests -k workspace_write_false_positives`) (hecho: ca1aa39)
- [x] Los 11 tests de test_autoreview vuelven a pasar: el Spawner del test falla con 'review' is not in list porque el collector ahora arranca 'canary run'. (check: `cd collector && uv run -q pytest -q tests/test_autoreview.py`) (hecho: 3fc5d50)
- [x] Los 4 tests rojos de test_brain (brain_calls), test_rooms y test_collector vuelven a pasar sin debilitarlos. (check: `cd collector && uv run -q pytest -q tests/test_brain.py tests/test_rooms.py tests/test_collector.py`) (hecho: 11255e9)
- [x] La regla roadmap-edit no bloquea comandos que solo leen el plan: un grep del plan en un comando que también usa sed -n o python -c open() de otros ficheros pasa. (check: `cd collector && uv run -q pytest -q tests -k roadmap_edit_read_only`) (hecho: ca1aa39)
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
- [x] Chat con `muse serve` <!-- tirith:{"id":"i-2e787d8f"} -->
  - [x] Revisar: Chat con `muse serve` — falló: the merge did not go through: the review was interrupted; run it again
- [x] Preguntas de permiso del agente en la UI.
- [x] La app arranca el collector si no hay uno (con uv; Python aún no va empaquetado).
- [x] Abrir la app en modo dev y comprobar que se ve y se actualiza.
- [x] Windows: shims propios (`.cmd`).
- [x] Windows: versión propia del wrapper de git y de la guarda (`safe-git`).
- [x] Confirmar qué agentes existen en cada plataforma. (hecho: fea63d1)
- [x] Empaquetado por plataforma (configuración y scripts de build).
- [x] Firmar la app de macOS con el Developer ID del dueño.
- [x] Notarizar la app de macOS (`scripts/notarize-macos.sh`, perfil `tirith-notary`; aceptada por Apple el 2026-10-29).
- [x] Chat de la app: adjuntar archivos con el clip y arrastrando funciona de verdad en la app de escritorio, y sin scroll horizontal. (hecho: bd92d3b)
  - [x] Revisar: Chat de la app: adjuntar archivos con el clip y arrastrando funciona de verdad en la app de escritorio, y sin scroll horizontal. — falló: the merge did not go through: merging lane/app-de-escritorio-tauri-con-chat-7 failed conflicts? : Auto-merging app/README.md CONF
- [x] Los mensajes del chat nunca se cortan ni se salen por la derecha: texto, rutas y código largos se parten o tienen su propio scroll dentro del bloque. (hecho: a71d1cb)
- [ ] Probar a mano en la app Tauri real (npm run tauri dev): clip, arrastrar y sin scroll horizontal; no hubo sesión gráfica (bloqueado: Needs a graphical session to run 'npm run tauri dev' and check clip, drag and no horizontal scroll by hand; this lane has none, and I made no code changes. The existing static tests chatoverflow, attach, chatdrop, padding are unchanged.) <!-- tirith:{"id":"i-79e7307c"} -->
- [x] Arrastrar una imagen al chat de la app de escritorio la adjunta de verdad (el dueño dice que hoy no funciona): encontrar dónde se pierde (handle_drag en lib.rs → webview → adjunto), arreglarlo y cubrirlo en chatdrop.test.mjs (check: `node web/js/chatdrop.test.mjs`) <!-- tirith:{"id":"i-2a0e66f3","depends_on":[],"writes":[]} -->
  - [x] Revisar: Arrastrar una imagen al chat de la app de escritorio la adjunta de verdad (el dueño dice que hoy no funciona): encontrar dónde se pierde (handle_drag en lib.rs → webview → adjunto), arreglarlo y cubrirlo en chatdrop.test.mjs (check: `node web/js/chatdrop.test.mjs`)
- [x] Probar a mano en la app Tauri real que arrastrar una imagen al chat la adjunta (revisar ~/.tirith/logs/app.log: 'over chat: true') (hecho: bafcf90e)
- [x] Agent card in the app: the role (Manager) is the bold title on top and the agent name (e.g. Quileatar) the smaller line below (check: `cd app && grep -rq 'role' src`) <!-- tirith:{"id":"i-960f1ac7","depends_on":[],"writes":[]} -->
  - [x] Revisar: Agent card in the app: the role (Manager) is the bold title on top and the agent name (e.g. Quileatar) the smaller line below (check: `cd app && grep -rq 'role' src`)
- [x] Fix pre-existing failure in web/js/detail.test.mjs (waitFor agent card old-1 times out) (hecho: e3ea3148) <!-- tirith:{"id":"i-814eeac4"} -->
  - [x] Revisar: Fix pre-existing failure in web/js/detail.test.mjs (waitFor agent card old-1 times out)
- [x] Volver a ejecutar tools/chat_check.py muse con un mensaje mínimo y con --interrupt hasta ver la línea result (la primera vez, 05:16Z, muse no respondió en 240 s y recibió SIGTERM) <!-- tirith:{"id":"i-6e46d7ba","depends_on":[],"writes":[]} -->
- [x] mspchat.py envía tras initialize la notificación que muse serve 1.4.14 exige según su esquema real (hoy responde a id 2 'Not initialized', ver /tmp/chatcheck-muse.log), cubierta en msp_fake_host y test_mspchat (check: `cd collector && uv run -q pytest -q tests/test_mspchat.py`) <!-- tirith:{"id":"i-b398c369","depends_on":[],"writes":[]} -->
- [x] Integrar en main la rama lane/chatoverflow (mensajes del chat sin cortes). (hecho: a71d1cb)
- [x] Llevar a main el cambio de la lane app-de-escritorio-tauri-con-chat-7 (clip, arrastrar, sin scroll horizontal; tests ya verdes): reaplicar a mano /Users/andreseloyasanchezva/.tirith/diffs/app-de-escritorio-tauri-con-chat-7.patch sobre main, sin rediseñar, y resolver el conflicto en app/README.md conservando lo de main. (hecho: bd92d3b)
  - [x] Revisar: Llevar a main el cambio de la lane app-de-escritorio-tauri-con-chat-7 (clip, arrastrar, sin scroll horizontal; tests ya verdes): reaplicar a mano /Users/andreseloyasanchezva/.tirith/diffs/app-de-escritorio-tauri-con-chat-7.patch sobre main, sin rediseñar, y resolver el conflicto en app/README.md conservando lo de main. — falló: the tests failed: …tauri-con-chat-8/collector/.venv/lib/python3.12/site-packages/_pytest/pathlib.py:123: PytestWarning
- [x] La app de escritorio se recompila y se reinicia sola cuando cambia su código Rust en main (firmada), sin que el dueño tenga que hacer nada. (hecho: 60f5650) <!-- tirith:{"id":"i-f1cd1bc7"} -->
  - [x] Revisar: La app de escritorio se recompila y se reinicia sola cuando cambia su código Rust en main (firmada), sin que el dueño tenga que hacer nada.
- [x] Confirmar el chat con muse serve en una sesión real (un mensaje mínimo, ajustes más baratos) y añadir la interrupción de turno y la reanudación si el protocolo las ofrece (hecho: 6584fad) <!-- tirith:{"id":"i-c226d1ce"} -->
- [x] Probar muse serve en una sesión real con un mensaje mínimo (vía tirith lane run) y comprobar con el esquema real los params de turn/interrupt y session/resume <!-- tirith:{"id":"i-51da13a1"} -->
  - [x] Revisar: Probar muse serve en una sesión real con un mensaje mínimo (vía tirith lane run) y comprobar con el esquema real los params de turn/interrupt y session/resume
- [x] Volcar el esquema real de muse serve (muse schema generate-json-schema) a un fichero para comprobar turn/interrupt y session/resume (hecho: a1cdd3c) <!-- tirith:{"id":"i-aa8e3eda","depends_on":[],"writes":[]} -->
- [x] Ejecutar un turno real de muse serve (mensaje mínimo) con un driver stdio vía tirith lane run, ahora que el adaptador soporta turn/interrupt y session/resume <!-- tirith:{"id":"i-1c43c823"} -->
  - [x] Revisar: Ejecutar un turno real de muse serve (mensaje mínimo) con un driver stdio vía tirith lane run, ahora que el adaptador soporta turn/interrupt y session/resume
- [x] Ejecutar de verdad tools/chat_check.py muse (CHAT_CHECK_STEP=9 CHAT_CHECK_ITEM=<item>) con un mensaje mínimo y con --interrupt; en la lane lo bloqueó el permiso del shell
- [x] Recuperar el trabajo de app-13 (parche guardado en ~/.tirith/diffs/app-de-escritorio-tauri-con-chat-13.patch: reanudación en chat_adapter, interrupt y tests contra msp_fake_host) sobre main, reconciliado con la interrupción/reanudación de app-14 (check: `cd collector && uv run pytest -q tests/test_mspchat.py -k 'interrupt_ends_the_running_turn or session_resumes_against_the_fake_host'`) <!-- tirith:{"id":"i-6beaa8d8","depends_on":[],"writes":[]} -->
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
- [x] El test web/js/planagents.test.mjs no reescribe la captura versionada web/screenshots/plan-many-agents.png al pasar: la escribe en un temporal, y solo la actualiza con una orden aparte. (check: `! grep -q "screenshots" web/js/planagents.test.mjs`) (hecho: 4350e44)

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
- [x] Un solo manager con sesión persistente que vive en Tirith (misma memoria con la app abierta o cerrada).
  - [x] Revisar: Un solo manager con sesión persistente que vive en Tirith (misma memoria con la app abierta o cerrada). — falló: the merge did not go through: merging lane/manager-de-guardia-chat-y-piloto-2 failed conflicts? : error: Your local changes to th
  - [x] Revisar: Un solo manager con sesión persistente que vive en Tirith (misma memoria con la app abierta o cerrada).
- [x] El chat muestra el trabajo del manager en vivo: texto, herramientas y resultados de cada paso (secretos tapados).
- [x] El chat del manager como el CLI: texto palabra a palabra, diffs en color, lista de tareas; Esc interrumpe y ↑ recupera.
  - [x] Revisar: El chat del manager como el CLI: texto palabra a palabra, diffs en color, lista de tareas; Esc interrumpe y ↑ recupera.
- [x] Sub-agentes anidados en la conversación; permisos seguros aprobados por código y el resto como botones Allow/Deny.
  - [x] Revisar: Sub-agentes anidados en la conversación; permisos seguros aprobados por código y el resto como botones Allow/Deny.
- [x] Autorrevisión cada minuto (collector, manager, piloto, disco, revisiones) con arreglo automático y una línea en el chat.
- [x] Lo que dice un agente en su informe se verifica contra su cambio real antes de fusionar.
- [x] El collector vuelve a tardar menos de 3 s por ciclo (hoy ~20 s) y un test lo vigila. (hecho: abdf921)
- [x] Fusionar nunca es una pregunta para el dueño: si los tests y la verificación pasan, Tirith fusiona solo; un duplicado o conflicto va a la cola del manager. Nunca dos agentes en el mismo elemento. (hecho: cd1b9c1)
- [x] Regla en código, sin excepciones: ningún agente arranca sin estar ligado a un elemento abierto del plan (revisiones y revisiones del plan incluidas: Tirith añade su elemento). (hecho: 222849b)
- [x] Regla en código: ningún agente termina sin informar a Tirith del resultado de su elemento (hecho / en parte + nuevos elementos / bloqueado + motivo); Tirith es el único que escribe el plan y marca «hecho» solo cuando la revisión pasa. (hecho: 222849b)
- [x] Llevar a main el commit 0a71483 de la rama lane/chat-wrap-long-reply-buttons-dra-2 (botones de respuesta largos que se ajustan, adjuntar o arrastrar cualquier archivo al chat): resolver los conflictos con main y pasar los tests (hecho: 4f3c7b2)
- [x] El collector se reinicia solo cuando cambia su código en main (así ningún cambio fusionado queda sin cargar). (hecho: 1df0b8d)
- [x] El piloto automático no lanza agentes: avisa al manager en el chat de cada paso listo y el manager decide (lanzar con buenas instrucciones, juntar, esperar o descartar). (hecho: 634c8fe)
- [x] Tirith libera disco solo cuando queda poco (carpetas de lanes ya fusionadas y cachés de compilación) y nunca se lo pide al dueño. (hecho: 4ef849a)
- [x] El manager persistente arranca siempre: comparte el elemento fijo del manager con los turnos sueltos (sin bloquearse por «ya en curso») y, si falla, el modo de respaldo responde en menos de 1 minuto. (hecho: b70ce39)
- [x] Los permisos de comandos seguros (solo lectura, comandos de Tirith) se aprueban por código; al dueño nunca le llega una pregunta de permiso que no sea suya.
  - [x] Revisar: Los permisos de comandos seguros (solo lectura, comandos de Tirith) se aprueban por código; al dueño nunca le llega una pregunta de permiso que no sea suya.
- [x] La autorrevisión no escribe en el chat lo que arregló sola: queda en el punto de salud (al hacer clic); el chat solo muestra lo que necesita al dueño. <!-- tirith:{"id":"i-2a7b6720"} -->
  - [x] Revisar: La autorrevisión no escribe en el chat lo que arregló sola: queda en el punto de salud (al hacer clic); el chat solo muestra lo que necesita al dueño. — falló: the report does not match the change
  - [x] Revisar: La autorrevisión no escribe en el chat lo que arregló sola: queda en el punto de salud (al hacer clic); el chat solo muestra lo que necesita al dueño.
- [x] Llevar a main el trabajo de la lane manager-de-guardia-chat-y-piloto-2: aplicar a mano ~/.tirith/diffs/manager-de-guardia-chat-y-piloto-2.patch sobre main y resolver los conflictos en collector/tirith/agentrun.py y collector/tirith/managerd.py conservando lo que ya hay en main (los tests de la lane pasaban: 1113). (hecho: 329f2c4)
  - [x] Revisar: Llevar a main el trabajo de la lane manager-de-guardia-chat-y-piloto-2: aplicar a mano ~/.tirith/diffs/manager-de-guardia-chat-y-piloto-2.patch sobre main y resolver los conflictos en collector/tirith/agentrun.py y collector/tirith/managerd.py conservando lo que ya hay en main (los tests de la lane pasaban: 1113). — falló: the merge did not go through: the review was interrupted; run it again
- [x] Bucle «sigue hasta terminar» por código: si al acabar falta informe, verificación, tests o rama lista, Tirith reanuda la misma sesión con la lista de lo que falta (hasta 3 veces) y solo después avisa al manager. (hecho: 4e04435)
- [x] Cola única de fusión: los agentes no tocan main; Tirith fusiona de uno en uno (trae main, suite completa, fusiona, empuja) y un conflicto vuelve al mismo agente. (hecho: 46fe181)
- [x] Elementos pequeños con su comprobación: el manager parte cada pedido en elementos de ~30 min con un comando de «hecho»; Tirith rechaza elementos enormes. (hecho: dda7276)
- [x] Integrar en main todo el trabajo terminado y sin fusionar de las lanes (lista del 2026-10-29) y descartar con motivo lo reemplazado. (hecho: a5063a4)
- [x] README y docs/SUBMISSION.md al día para el jurado del hackathon: qué hace Tirith hoy (autonomía con reglas en código, plan, manager en el chat, app notarizada), cómo usa los modelos de Meta, cómo probarlo y capturas reales. (hecho: 849dc41)
- [x] Quitar todo el GDScript: la carpeta world/ (ciudadela 3D de Godot), ./run --3d y toda referencia a Godot en el código, la web y los docs. (hecho: f936c0c)
- [x] Integrar en main lane/smallitems (elementos pequeños con comprobación; ya commiteado, cortado por el límite de 1 h antes de fusionar). (hecho: dda7276)
- [x] Integrar en main las lanes terminadas teach, mergeq, fasttests, pilotask, idletimeout y diskauto (resolviendo conflictos). (hecho: b2401d0)
- [x] Landing page pública de Tirith (GitHub Pages): qué es, qué hace, cómo usa los modelos de Meta, capturas reales y cómo probarlo. (hecho: 6f05bca)
- [x] Vídeo de demo de 3 min del Tirith actual (manager en el chat, plan, reglas en código, agentes trabajando) grabado por script, más un GIF corto para el README. (hecho: 679d021)
- [x] Repo preparado para conseguir estrellas: README con GIF y estrella arriba, badges, inicio en una línea, imagen social, CONTRIBUTING y plantillas de issues. (hecho: aa90394)
- [x] Licencia MIT (elegida por el dueño): archivo LICENSE, campo license en los manifiestos y badge en el README. (hecho: b740efc)
- [x] La app reinicia el collector si el estado deja de actualizarse aunque el proceso siga vivo. (check: `cd app/src-tauri && cargo test -q staleness`) (hecho: c4775c9)
- [x] El recargado automático del collector nunca lo deja bloqueado. (check: `cd collector && uv run -q pytest -q tests -k reload`) <!-- tirith:{"id":"i-08c61413"} -->
  - [x] Revisar: El recargado automático del collector nunca lo deja bloqueado. (check: `cd collector && uv run -q pytest -q tests -k reload`)
- [x] Tras arrancar o recargar, el collector escribe un estado útil en menos de 30 s. (check: `cd collector && uv run -q pytest -q tests -k first_cycle`) (hecho: bb947e4)
- [x] Quitar worktrees de lanes sin chocar con el límite 'git': 'git worktree remove' (diskdiet.py, review.py) usa su propia clase de límite 'git-worktree-remove' (minutos, no los ~9 s aprendidos de git), con el test test_worktree_remove_uses_own_timeout_class (check: `cd collector && python3 -m pytest -q tests -k test_worktree_remove_uses_own_timeout_class`) (hecho: b2e07de)
- [x] Hacer robusto el test test_context_over_the_limit_compacts_then_restarts_with_a_handoff de test_msession.py, que falla bajo carga porque Rig.until espera 25 s de reloj real (check: `cd collector && uv run pytest tests/test_msession.py -q -k compacts_then_restarts`)
  - [x] Revisar: Hacer robusto el test test_context_over_the_limit_compacts_then_restarts_with_a_handoff de test_msession.py, que falla bajo carga porque Rig.until espera 25 s de reloj real (check: `cd collector && uv run pytest tests/test_msession.py -q -k compacts_then_restarts`)
- [x] La landing page tiene animaciones al hacer scroll (aparición suave de secciones, capturas y tarjetas) respetando «reducir movimiento». (check: `node web/js/landing.test.mjs`) (hecho: 40ed8f2)
- [x] Logo de Tirith en 3D renderizado con Blender y renders 3D en la landing page. (check: `test -f site/img/logo-3d.png`) (hecho: 7fabf84)
- [x] Registrar en la revisión de una lane la causa cuando se interrumpe (señal o código de salida del proceso), con un test que mata el proceso de la revisión y comprueba la causa registrada. (check: `cd collector && uv run pytest -q -k review_interrupted_records_cause`)
  - [x] Revisar: Registrar en la revisión de una lane la causa cuando se interrumpe (señal o código de salida del proceso), con un test que mata el proceso de la revisión y comprueba la causa registrada. (check: `cd collector && uv run pytest -q -k review_interrupted_records_cause`)
- [x] Volver a grabar el vídeo de demo con la interfaz actual (sin Flow, logo 3D) y cada subtítulo sobre la escena que describe. (check: `test -f docs/demo/out/tirith-demo.mp4`) (hecho: bb07913)
- [x] El límite de tamaño de una lane cuenta solo su propio cambio (contra la base con main), no lo que trajo al fusionar main. (check: `cd collector && uv run -q pytest -q tests -k budget`) (hecho: 84c9ea6)
- [x] Hacer robusto el test test_second_message_reaches_the_same_process_and_waits_for_the_turn_to_end de test_msession.py, que falla bajo carga como el de compacts (espera de reloj real) (check: `cd collector && uv run pytest tests/test_msession.py -q -k second_message_reaches_the_same_process`) (hecho: 46b1c75)
- [x] Rehacer sobre main el commit 8b4bf7c de lane/manager-de-guardia-chat-y-piloto-3 (una conversación del manager entre turnos headless, 5 archivos) en un worktree nuevo, con test por etapa e informe con el número exacto de tests (check: `git show main:collector/tests/test_managerd.py | grep -q test_headless_turns_share_one_session && git show main:collector/tirith/managerd.py | grep -q 'def adopt_session'`) (hecho: 329f2c4)
- [x] El piloto automático manda como mucho una propuesta abierta por paso. (check: `cd collector && uv run -q pytest -q tests -k autopilot`) (hecho: 62a63b6)
- [x] Los mensajes del dueño pasan delante: el manager atiende cada mensaje del chat antes que las propuestas del piloto. (check: `cd collector && uv run -q pytest -q tests -k managerd`) (hecho: 5754f53)
- [x] web/js/landing.test.mjs carga playwright-core del propio repo (o se salta con aviso si no está), nunca de node_modules de otro proyecto como propuestavenezuela (check: `node web/js/landing.test.mjs`) (hecho: 40ed8f2)
- [x] tirith plan done acepta un commit que solo toca docs cuando el elemento mismo es de docs (README, docs/…), con un test (check: `cd collector && uv run -q pytest -q tests -k plan_done_docs`) <!-- tirith:{"id":"i-e570bcdf"} -->
  - [x] Revisar: tirith plan done acepta un commit que solo toca docs cuando el elemento mismo es de docs (README, docs/…), con un test (check: `cd collector && uv run -q pytest -q tests -k plan_done_docs`)
- [x] Un worktree de lane nunca se registra como proyecto automático: si git falla o vence el límite al resolver la raíz (_in_main_worktree en projects.py), no se guarda en caché ni se registra la carpeta. (check: `cd collector && uv run -q pytest -q tests -k worktree_never_auto_project`)
- [x] El icono de la app de macOS se genera a partir del logo 3D. (check: `grep -q logo-3d scripts/make_icon.py`) (hecho: 7bcd959)
- [x] El logo 3D es una torre de piedra con base circular, renderizada en Blender con texturas de piedra reales y calidad de portada, con almenas de piedra y sin la esfera flotante (pedido del dueño). (check: `test -f site/img/logo-3d.png`) (hecho: 7ff25cf)
- [x] La captura principal de la landing muestra la app actual con datos de demo limpios: sin avisos, sin reglas inactivas y sin Flow. (check: `node web/js/landing.test.mjs`) (hecho: fadd25d)
- [x] Los renders 3D de la landing (torre con agentes y puerta de reglas) usan la torre de piedra nueva con el mejor render y la mejor luz posibles, más contraste con el fondo y sin cortes en los bordes (pedido del dueño). (check: `node web/js/landing.test.mjs`) (hecho: 7bcd959)
- [x] En el móvil, el texto bajo la captura principal de la landing no se monta sobre la imagen. (check: `node web/js/landing.test.mjs`) (hecho: fadd25d)
- [x] La imagen social (social-preview-3d.png) usa el mismo modelo de torre de piedra que el logo. (check: `grep -q stone_tower site/3d/social.py`) (hecho: 7bcd959)
- [x] Evidencia de la torre de piedra: capturas de la landing publicada en escritorio y móvil, antes y después, en docs/evidence/logo-torre/. (check: `cd docs/evidence/logo-torre && test -f antes-escritorio.png && test -f antes-movil.png && test -f despues-escritorio.png && test -f despues-movil.png`) (hecho: 40ed8f2)
- [x] La landing deja de mostrar render-board.png (pedido del dueño). (check: `! grep -q render-board site/index.html`) (hecho: 7ecbd87)
- [ ] Imágenes de inspiración para los renders, generadas con un modelo de imagen de Google y guardadas fuera del repo (espera: una clave de Google que funcione, del dueño). (check: `test -n "$(ls ~/.cache/tirith-inspiration 2>/dev/null)"`)
- [x] Fix 3 pre-existing failing process/timing tests (test_cta lane_stop_kills_a_child_that_ignores_sigterm, test_idletimeout child_cpu_is_progress, test_timeouts kills_tree_including_children_that_left_the_group) (hecho: d8874b37)
- [x] Confirm 3 failing process-kill tests (test_cta sigterm, test_idletimeout cpu_is_progress, test_timeouts left_the_group) fail on main too (hecho: 0bcd5b94) <!-- tirith:{"id":"i-d0f63b29"} -->
  - [x] Revisar: Confirm 3 failing process-kill tests (test_cta sigterm, test_idletimeout cpu_is_progress, test_timeouts left_the_group) fail on main too
- [x] El icono de la app de macOS (el mismo modelo de torre de piedra) se lee bien a 32 px: se genera una versión pequeña y se comprueba su contraste (pedido del dueño). (check: `test -f app/src-tauri/icons/32x32.png && grep -q stone_tower scripts/make_icon.py`) (hecho: 849dc41)
- [x] En el móvil, el diagrama de la landing cabe entero sin desplazarse de lado. (check: `node web/js/landing.test.mjs`) (hecho: 40ed8f2)
- [x] En el móvil, los bloques de código de la landing se leen enteros sin desplazarse de lado. (check: `node web/js/landing.test.mjs`) (hecho: 40ed8f2)
- [x] El texto de estado de la landing dice la verdad de hoy: ya no pone en construcción lo que ya funciona. (check: `! grep -q 'under construction' site/index.html`) (hecho: 40ed8f2)
- [x] El README abre con la torre de piedra y capturas actuales de la app. (check: `grep -q -E 'logo-3d|social-preview-3d' README.md`) (hecho: 849dc41)
- [x] El video demo se regraba con el logo de piedra y la app actual. (check: `test site/img/logo-3d.png -ot docs/demo/out/tirith-demo.mp4`) (hecho: 849dc41)
- [x] La reanudación de un lane manager no sale con el error --session-id can only be used with --continue or --resume (visto en manager-418a3be2, marcado failed tras acabar su turno). (check: `cd collector && uv run -q pytest -q tests -k resume_drops_session_id`) (hecho: 5a29510)
- [x] El logo de piedra tiene luz de portada: luz cálida lateral, contraluz que recorta la silueta y la ventana iluminando la piedra de alrededor (pedido del dueño). (check: `test -f site/img/logo-3d.png`) (hecho: 7bcd959)

## 15. Collector y CLI en Rust

- [x] Base: crate Rust compartido (workspace con la app), arnés de paridad que corre el collector de Python y el de Rust sobre las mismas carpetas de prueba y compara `state.json`.

## Orden

1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9, y 10 en cuanto se pidió.

Hasta el 4 de noviembre (entrega del hackathon): activar y proteger los hooks (3), el bloqueo de
sub-agentes fuera de `lane run` (7), los logs legibles (8) y el plan en el Overview (10).

## 16. Velocidad y autonomía: de un objetivo grande a resultados sin el dueño

- [x] `tirith do "<objetivo>"` convierte un objetivo grande en elementos pequeños del plan, cada uno con su comprobación, y los lanza. (check: `cd collector && uv run -q pytest -q tests -k tirith_do`) <!-- tirith:{"id":"i-771fc4f5"} -->
  - [x] Revisar: `tirith do "<objetivo>"` convierte un objetivo grande en elementos pequeños del plan, cada uno con su comprobación, y los lanza. (check: `cd collector && uv run -q pytest -q tests -k tirith_do`) — falló: the report does not match the change
  - [x] Revisar: `tirith do "<objetivo>"` convierte un objetivo grande en elementos pequeños del plan, cada uno con su comprobación, y los lanza. (check: `cd collector && uv run -q pytest -q tests -k tirith_do`)
- [x] El planificador no lanza a la vez dos lanes que tocan los mismos archivos. (check: `cd collector && uv run -q pytest -q tests -k conflicts`) (hecho: e8db252) <!-- tirith:{"id":"i-e6f95c7d"} -->
  - [x] Revisar: El planificador no lanza a la vez dos lanes que tocan los mismos archivos. (check: `cd collector && uv run -q pytest -q tests -k conflicts`)
- [x] La cola de merge prueba varios lanes juntos en una sola pasada de tests y separa al culpable si falla. (check: `cd collector && uv run -q pytest -q tests -k mergetrain`) (hecho: 69fee04)
- [x] Cada elemento terminado muestra su evidencia en la app: salida de tests y capturas si cambia la interfaz. (check: `cd collector && uv run -q pytest -q tests -k evidence`) (hecho: c3e1b9d) <!-- tirith:{"id":"i-a284892e"} -->
  - [x] Revisar: Cada elemento terminado muestra su evidencia en la app: salida de tests y capturas si cambia la interfaz. (check: `cd collector && uv run -q pytest -q tests -k evidence`)
- [x] La app muestra cuánto tarda un elemento en llegar a main, cuántos llegan por hora y cuántas veces hizo falta el dueño. (check: `cd collector && uv run -q pytest -q tests -k metrics`) <!-- tirith:{"id":"i-100695a9"} -->
  - [x] Revisar: La app muestra cuánto tarda un elemento en llegar a main, cuántos llegan por hora y cuántas veces hizo falta el dueño. (check: `cd collector && uv run -q pytest -q tests -k metrics`)
- [x] Cada lane recibe en su brief un mapa del código con los archivos relevantes para su elemento. (check: `cd collector && uv run -q pytest -q tests -k codemap`) (hecho: 7da4eb3) <!-- tirith:{"id":"i-3d811adb"} -->
  - [x] Revisar: Cada lane recibe en su brief un mapa del código con los archivos relevantes para su elemento. (check: `cd collector && uv run -q pytest -q tests -k codemap`)
- [x] `tirith plan add` muestra el id corto del elemento y todos los comandos lo aceptan. (check: `cd collector && uv run -q pytest -q tests -k item_id`) <!-- tirith:{"id":"i-d27c6822"} -->
  - [x] Revisar: `tirith plan add` muestra el id corto del elemento y todos los comandos lo aceptan. (check: `cd collector && uv run -q pytest -q tests -k item_id`) — falló: the merge did not go through: the review was interrupted; run it again
- [x] El dueño recibe un resumen visual de lo que llegó a main, con un botón de opinión que se convierte en elementos del plan. (check: `cd collector && uv run -q pytest -q tests -k digest`) <!-- tirith:{"id":"i-d2bd7370"} -->
  - [x] Revisar: El dueño recibe un resumen visual de lo que llegó a main, con un botón de opinión que se convierte en elementos del plan. (check: `cd collector && uv run -q pytest -q tests -k digest`)
- [x] A lane that reports done with uncommitted changes is committed by Tirith through the guard hooks (message built from the item) and goes merge-ready without a send-back (check: `cd collector && uv run -q pytest -q tests -k done_report_dirty_worktree_merge_ready`) <!-- tirith:{"id":"i-913c63be","depends_on":[],"writes":[]} -->
  - [x] Revisar: A lane that reports done with uncommitted changes is committed by Tirith through the guard hooks (message built from the item) and goes merge-ready without a send-back (check: `cd collector && uv run -q pytest -q tests -k done_report_dirty_worktree_merge_ready`)
- [x] Lane briefs no longer tell agents not to commit, matching Tirith committing a dirty worktree at report (check: `cd collector && uv run -q pytest -q tests -k brief_allows_commit`) (hecho: e835ecc2) <!-- tirith:{"id":"i-416047ae","depends_on":[],"writes":[]} -->
  - [x] Revisar: Lane briefs no longer tell agents not to commit, matching Tirith committing a dirty worktree at report (check: `cd collector && uv run -q pytest -q tests -k brief_allows_commit`)

## 17. Tirith se mejora usando Tirith

- [x] Tirith se ejecuta desde una copia estable que solo avanza cuando la prueba completa pasa en la versión nueva, y vuelve atrás sola si la salud se pone roja. (check: `cd collector && uv run -q pytest -q tests -k stable`)
- [x] Tirith registra cada vez que hizo falta una persona o algo esperó, y pone la fricción mayor arriba del plan. (check: `cd collector && uv run -q pytest -q tests -k friction`) (hecho: a59ac81) <!-- tirith:{"id":"i-6b03a510"} -->
  - [x] Revisar: Tirith registra cada vez que hizo falta una persona o algo esperó, y pone la fricción mayor arriba del plan. (check: `cd collector && uv run -q pytest -q tests -k friction`) — falló: the merge did not go through: merging lane/tirith-se-mejora-usando-tirith-38 failed conflicts? : Auto-merging collector/tirith/cl
- [x] La cola de merge tiene prioridad: los arreglos de la propia cola y de sus etapas entran primero. (check: `cd collector && uv run -q pytest -q tests -k merge_priority`) (hecho: 70655b2) <!-- tirith:{"id":"i-f22f3f94"} -->
  - [x] Revisar: La cola de merge tiene prioridad: los arreglos de la propia cola y de sus etapas entran primero. (check: `cd collector && uv run -q pytest -q tests -k merge_priority`)
- [x] Las instrucciones de los lanes dicen quién revisa de verdad: Tirith hace el commit y la cola de merge revisa, nunca «para que lo revise el dueño». (check: `! grep -rq 'for the owner to review' collector/tirith`) (hecho: 39e0c0b)
- [x] Los lanes que lanza Tirith desde la app ven las variables de entorno del dueño (claves como GITHUB_TOKEN o GEMINI_API_KEY de su shell), sin copiarlas a ningún archivo del repo. (check: `cd collector && uv run -q pytest -q tests -k owner_env`) (hecho: 7e9adbc)
- [x] Los límites de tiempo de los comandos crecen con la carga de la máquina, así git lento con carga alta no pone la salud en rojo. (check: `cd collector && uv run -q pytest -q tests -k load_scaled`) (hecho: 583aa22)
- [x] La cola de merge puede cancelar el merge en curso cuando otro lane lo hace innecesario. (check: `cd collector && uv run -q pytest -q tests -k merge_cancel`) <!-- tirith:{"id":"i-58fc6600"} -->
  - [x] Revisar: La cola de merge puede cancelar el merge en curso cuando otro lane lo hace innecesario. (check: `cd collector && uv run -q pytest -q tests -k merge_cancel`)
- [x] Un lane vivo que espera su etapa de tests no cuenta como parado: ni el estado ni el autopilot lo marcan stalled solo porque su log lleva 10 minutos sin cambiar. (check: `cd collector && uv run -q pytest -q tests -k stalled_while_tests_run`) (hecho: f87217d)
- [x] La cola de merge no descarta un merge ya probado cuando main solo cambió en el plan: incorpora esos commits y sigue. (check: `cd collector && uv run -q pytest -q tests -k plan_only`) (hecho: b9f21d9)
- [x] Todas las llamadas a git usan el límite de la política de timeouts, sin límites fijos en el código. (check: `cd collector && uv run -q pytest -q tests -k no_fixed_git_timeout`) (hecho: 9786a90)
- [x] El manager se compacta cuantas veces haga falta y solo se reinicia si compactar no basta, como mucho una vez por hora. (check: `cd collector && uv run -q pytest -q tests -k compact`) (hecho: 9dfb0a7)
- [x] El brain solo se llama cuando cambia lo que tiene que juzgar, nunca para una lista vacía ni en cada ciclo del collector. (check: `cd collector && uv run -q pytest -q tests -k brain_calls`) (hecho: 9200fea)
- [x] Tren de integración A: todos los lanes terminados del collector, la cola y las reglas, juntos en una rama probada una sola vez. (check: `git merge-base --is-ancestor lane/u-mqplanmoves main`) (hecho: 8e3a1a9)
- [x] Tren de integración B: todos los lanes terminados de la app, la web y la landing, juntos en una rama probada una sola vez. (check: `git merge-base --is-ancestor lane/u-stonetower main`) (hecho: b6f5817)
- [x] Al devolver un lane, la cola de merge reabre su elemento del plan y comprueba que el agente arrancó. (check: `cd collector && uv run -q pytest -q tests -k send_back`) (hecho: 9a0d35c)
- [x] La ronda de seguimiento de un lane (keep-going) arranca su agente sin que la regla agent-cli la bloquee, y un lane con informe done no termina como failed por ese bloqueo. (check: `cd collector && uv run -q pytest -q tests -k keepgoing_resume_not_blocked`) (hecho: 1abd541)
- [x] Tirith sube main a GitHub como mucho 5 minutos después de cada cambio del plan. (check: `cd collector && uv run -q pytest -q tests -k push_plan`) (hecho: 2d7e4f0) <!-- tirith:{"id":"i-77418384"} -->
  - [x] Revisar: Tirith sube main a GitHub como mucho 5 minutos después de cada cambio del plan. (check: `cd collector && uv run -q pytest -q tests -k push_plan`) — falló: the merge did not go through: the review was interrupted; run it again
- [x] La cola de merge distingue un git cortado por tiempo de un conflicto real, y reintenta el primero sin devolver el lane. (check: `cd collector && uv run -q pytest -q tests -k merge_timeout`) (hecho: 1589d3d)
- [x] Un lane ya mezclado a mano sale de la cola y de las revisiones sin intervención. (check: `cd collector && uv run -q pytest -q tests -k already_merged`) (hecho: 2203df2)
- [x] i-aef081ed (hecho: fde18e0)
- [x] Dos ejecuciones simultáneas de etapas en worktrees distintos no comparten número de ejecución ni fichero de log. (check: `cd collector && uv run -q pytest -q tests -k concurrent_runs_separate_logs`) (hecho: 583aa22)
- [x] Un lane no deja su item marcado como hecho en el plan antes de que su trabajo entre en main. (check: `cd collector && uv run -q pytest -q tests -k item_stays_open_until_merged`) (hecho: b353554) <!-- tirith:{"id":"i-fab8ba1c"} -->
  - [x] Revisar: Un lane no deja su item marcado como hecho en el plan antes de que su trabajo entre en main. (check: `cd collector && uv run -q pytest -q tests -k item_stays_open_until_merged`) — falló: the merge did not go through: the review was interrupted; run it again
  - [x] Revisar: Un lane no deja su item marcado como hecho en el plan antes de que su trabajo entre en main. (check: `cd collector && uv run -q pytest -q tests -k item_stays_open_until_merged`)
- [x] tirith check: _coverage_problems in manifest.py counts pipelines from include files, so a project whose build or test stages exist only in an include passes the check while the merge queue finds no stage to run; count only gating pipelines (hecho: f87217d)
- [x] La cola de merge reintenta el commit de la lane cuando encuentra un index.lock abandonado en su worktree, en vez de marcarla red-suite y devolverla (check: `cd collector && uv run -q pytest -q tests -k commit_workspace_recovers_lock`) (hecho: 1d334c7)
- [x] Una lane de continuación que reutiliza el worktree de otra entra sola en la cola de merge al terminar con done (check: `cd collector && uv run -q pytest -q tests -k continuation_lane_auto_enqueued`) (hecho: 1d334c7)
- [x] Pasar un solo test tarda menos de 30 segundos también con carga alta: hoy test_code_move_is_retried tardó 8 minutos (pytest con -k, 1 passed in 493 s). (check: `cd collector && uv run -q pytest -q --durations=5 tests/test_mergeq.py -k test_code_move_is_retried`) (hecho: c906004)
- [x] tests/test_mergeq.py::test_code_move_is_retried_and_the_next_entry_still_runs no falla con carga alta: se averigua por qué el lane two queda en waiting (1 failed en ~/.tirith/logs/ci-168-test.log) y se arregla el test si es frágil o la cola si es un error real (check: `cd collector && uv run -q pytest -q tests/test_mergeq.py -k test_code_move_is_retried`)
- [x] Devolver una lane de continuación que reutiliza el worktree de otra funciona: relaunch_lane usa la rama real de la lane (mergeq.branch_of), no lane/<id>, que hoy falla con invalid reference: lane/u-testfix-port (check: `cd collector && uv run -q pytest -q tests -k send_back_continuation`) (hecho: 1d334c7)
- [x] La app muestra arriba, sin preguntar, qué entró hoy en main, qué se está haciendo, qué está atascado y quién lo lleva. (check: `cd collector && uv run -q pytest -q tests -k now_bar`) (hecho: 5e46807) <!-- tirith:{"id":"i-05bc1f8a"} -->
  - [x] Revisar: La app muestra arriba, sin preguntar, qué entró hoy en main, qué se está haciendo, qué está atascado y quién lo lleva. (check: `cd collector && uv run -q pytest -q tests -k now_bar`)
- [x] Cada petición del dueño en el chat queda ligada a un elemento del plan o a una respuesta, y la app lista sus peticiones con su estado. (check: `cd collector && uv run -q pytest -q tests -k owner_requests`) <!-- tirith:{"id":"i-37316566"} -->
  - [x] Revisar: Cada petición del dueño en el chat queda ligada a un elemento del plan o a una respuesta, y la app lista sus peticiones con su estado. (check: `cd collector && uv run -q pytest -q tests -k owner_requests`)
- [x] Al volver, el dueño ve un resumen de lo que pasó mientras no estaba: lo que entró, lo que falló y lo que sigue. (check: `cd collector && uv run -q pytest -q tests -k while_away`) (hecho: e92cad6) <!-- tirith:{"id":"i-a80aa182"} -->
  - [x] Revisar: Al volver, el dueño ve un resumen de lo que pasó mientras no estaba: lo que entró, lo que falló y lo que sigue. (check: `cd collector && uv run -q pytest -q tests -k while_away`)
- [x] Un ciclo del collector tarda menos de 5 segundos con carga alta: hoy llegó a 66 s, 49 s de ellos en el despachador. (check: `cd collector && uv run -q pytest -q tests -k cycle_budget`) (hecho: f22221a)
- [x] El tamaño de contexto del manager se mide con la última petición, no con la suma de todas las del turno. (check: `cd collector && uv run -q pytest -q tests -k context_tokens`) (hecho: ee2eb5a)
- [x] Launch the collector and the app from <home>/stable/current, and run 'tirith stable promote' automatically after each merge on main <!-- tirith:{"id":"i-77fc216c"} -->
  - [x] Revisar: Launch the collector and the app from <home>/stable/current, and run 'tirith stable promote' automatically after each merge on main
- [x] test_code_move_is_retried_and_the_next_entry_still_runs: root cause of lane two ending in waiting under load not found (36 mergeq tests pass at 800 s under heavy load); next failing CI log now prints the rows with the waiting message, read it and fix <!-- tirith:{"id":"i-3b952312"} -->
  - [x] Revisar: test_code_move_is_retried_and_the_next_entry_still_runs: root cause of lane two ending in waiting under load not found (36 mergeq tests pass at 800 s under heavy load); next failing CI log now prints the rows with the waiting message, read it and fix
- [x] test_plan_done_checks_the_commit y test_the_writer_lock_serializes_changes no fallan con carga alta: en ci#175 git commit del plan salió con error y salida vacía, y solos pasan (check: `cd collector && uv run -q pytest -q tests -k plan_commit_failure_says_why`) (hecho: 48dd464)
- [x] test_canary_run_reaches_the_bare_origin_and_ticks_the_plan no falla con carga alta: en ci#177 el canary acabó en el paso push con wait_s=60 (check: `cd collector && uv run -q pytest -q tests/test_canary.py -k canary_run_reaches`) (hecho: 552afdc)
- [x] Un cambio del plan no falla con carga alta por esperar el lock: LOCK_WAIT_S de 30 s en plangate.py hizo fallar test_the_writer_lock_serializes_changes en ci#177 con «another change to the plan is taking too long» (check: `cd collector && uv run -q pytest -q tests/test_plangate.py -k writer_lock`) (hecho: 8299cd0)
- [x] Antes de rechazar un merge, la cola repite los tests fallados de uno en uno: si pasan, entra y el test queda anotado como inestable. (check: `cd collector && uv run -q pytest -q tests -k rerun_failed`) (hecho: 56448d5)
- [x] Con 3 lanes o más esperando en la cola de merge no arranca ningún lane nuevo: primero se termina lo empezado. (check: `cd collector && uv run -q pytest -q tests -k wip_limit`) (hecho: 56448d5)
- [x] Deflake timing-sensitive tests that fail intermittently under load (test_idletimeout silent/chatty stage, test_mergeq code-move retry, test_plangate dirty plan) <!-- tirith:{"id":"i-e1cda5e6"} -->
  - [x] Revisar: Deflake timing-sensitive tests that fail intermittently under load (test_idletimeout silent/chatty stage, test_mergeq code-move retry, test_plangate dirty plan)
- [x] El trabajo de devolver lanes (reabrir el elemento del plan y comprobar que el agente arrancó) entra en main: la rama lane/tirith-se-mejora-usando-tirith-2 choca con main en test_mergeq.py y CONTRACT-mergeq.md, y su elemento quedó marcado como hecho con un commit de otro lane. (check: `cd collector && uv run -q pytest -q tests -k send_back_reopens_the_plan_item`) (hecho: 06391d4)
- [x] Un lane sin agente registrado también vuelve a su agente: la cola solo reenvía si lane.agent está en la configuración, y los lanes con agent null llegan al manager sin decir por qué. (check: `cd collector && uv run -q pytest -q tests -k send_back_without_agent`) (hecho: 1d334c7)
- [x] El piloto automático no trata una copia de merge (_mergeq-<lane>) como proyecto: mergeq-tirith-se-mejora-usando-tirith-5 generó cinco propuestas (pasos 3, 7, 9, 10 y 14) a partir de la copia de docs/TODO.md. (check: `cd collector && uv run -q pytest -q tests -k autopilot_ignores_merge_worktree`) (hecho: f87217d)
- [x] Un elemento solo se marca hecho si el lane tiene commits propios en main: una rama sin commits con trabajo sin guardar no cuenta como mezclada. (check: `cd collector && uv run -q pytest -q tests -k false_tick`) (hecho: fe73278)
- [x] Llevar a main el arreglo de lanes devueltos de lane/tirith-se-mejora-usando-tirith-2 (06391d4): tuvo conflicto en la cola y su elemento se marcó hecho sin haber entrado. (check: `git merge-base --is-ancestor 06391d4 main`) (hecho: 4263870)
- [x] `tirith project add` deja un proyecto listo de una vez: plan creado, tirith.toml mínimo y guarda con un clic del dueño. (check: `cd collector && uv run -q pytest -q tests -k project_ready`) <!-- tirith:{"id":"i-c3a4df1d"} -->
- [x] Un merge ya probado tampoco se descarta si main solo cambió en docs/ o site/, que no afectan a los tests. (check: `cd collector && uv run -q pytest -q tests -k merge_paths`) (hecho: a11dee5)
- [x] La comprobación de lo que dice un lane no devuelve a un lane de solo verificar: su diff vacío es lo esperado, y los archivos que cita como hallazgos no son cambios suyos. (check: `cd collector && uv run -q pytest -q tests -k claim_check_verify_lane`) <!-- tirith:{"id":"i-d1584adf"} -->
- [x] Tirith borra por sí mismo las carpetas temporales de tests antiguas, incluidas las de pytest-old-archive, sin esperar al dueño. (check: `cd collector && uv run -q pytest -q tests -k temp_cleanup`) (hecho: d0033ba)
- [x] Se sabe qué quitó la protección de los hooks del guard de Tirith a las 18:58:19Z del 30 oct y ya no puede volver a pasar. (check: `cd collector && uv run -q pytest -q tests -k guard_flags_stay`) <!-- tirith:{"id":"i-55ba859a"} -->
  - [x] Revisar: Se sabe qué quitó la protección de los hooks del guard de Tirith a las 18:58:19Z del 30 oct y ya no puede volver a pasar. (check: `cd collector && uv run -q pytest -q tests -k guard_flags_stay`)
- [x] Los tests de una lane corren en paralelo: hoy 41 tests de test_mergeq.py tardaron 17 minutos en serie porque el modo changed no pasa -n. (check: `cd collector && uv run -q pytest -q tests -k changed_tests_run_in_parallel`) (hecho: 62758db)
- [x] La suite de Python tarda menos de 6 minutos con carga alta: hoy tarda de 12 a 16, casi todo esperando y no en CPU. (check: `cd collector && uv run -q pytest -q tests -k suite_budget`) (hecho: c25befc)
- [x] Tren C: las lanes que aceleran la cola (u-mqfast, -11, u-cyclefast, -14) juntas en una rama probada una sola vez, en vez de cuatro fusiones que chocan entre sí. (check: `cd collector && uv run -q pytest -q tests/test_mergeq.py -k 'train or flaky or rerun'`) (hecho: 2b2da24)
- [x] La cola no tira una fusión ya probada porque la copia principal esté sucia: aterriza si los archivos sucios no chocan, o espera sin repetir los tests. (check: `cd collector && uv run -q pytest -q tests/test_mergeq.py -k dirty_checkout`) <!-- tirith:{"id":"i-229f2947"} -->
  - [x] Revisar: La cola no tira una fusión ya probada porque la copia principal esté sucia: aterriza si los archivos sucios no chocan, o espera sin repetir los tests. (check: `cd collector && uv run -q pytest -q tests/test_mergeq.py -k dirty_checkout`) — falló: the merge did not go through: /Users/andreseloyasanchezva/DEVELOP/Tirith has uncommitted changes, so nothing can be merged into it
- [x] Ningún test escribe en archivos versionados: web/js/planagents.test.mjs reescribe web/screenshots/plan-many-agents.png y ensució la copia principal. (check: `! grep -q 'plan-many-agents.png' web/js/planagents.test.mjs || grep -q 'UPDATE_SCREENSHOTS' web/js/planagents.test.mjs`) (hecho: 4350e44)
- [x] La regla workspace-write no toma las expresiones de sed -i -e como rutas de escritura (hoy bloquea /^=======$/d como si fuera un archivo). (check: `cd collector && uv run -q pytest -q tests -k workspace_write_sed_expression`) (hecho: ca1aa39)
- [x] Un árbol que ya pasó sus tests no se vuelve a probar: Tirith recuerda el árbol probado, y ni la lane, ni la claim check, ni la cola repiten esa etapa. (check: `cd collector && uv run -q pytest -q tests -k same_tree_is_not_tested_again`) (hecho: 1d334c7)
- [x] Una lane cuyo commit ya está en otra rama de la cola o en main se para y sale de la cola sola, sin correr más tests. (check: `cd collector && uv run -q pytest -q tests -k absorbed_lane_stops`) (hecho: 1d334c7)
- [x] Tirith mide el trabajo tirado (suites repetidas, fusiones probadas descartadas, lanes absorbidas) y lo muestra en la salud: la meta del dueño es no hacer trabajo estúpido. (check: `cd collector && uv run -q pytest -q tests -k wasted_work`) (hecho: 2b11e1d) <!-- tirith:{"id":"i-24455b97"} -->
  - [x] Revisar: Tirith mide el trabajo tirado (suites repetidas, fusiones probadas descartadas, lanes absorbidas) y lo muestra en la salud: la meta del dueño es no hacer trabajo estúpido. (check: `cd collector && uv run -q pytest -q tests -k wasted_work`)
- [x] El push automático de main (pushplan) no frena el despachador: corre fuera de la pasada, y un push que falla espera cada vez más (1, 2, 4… hasta 30 min) en vez de repetirse cada 60 s con sus hooks. (check: `cd collector && uv run -q pytest -q tests -k push_plan_backoff`) <!-- tirith:{"id":"i-c1227b58903f4ddaa9bb866613d5df4f","depends_on":[],"writes":[]} -->
  - [x] Revisar: El push automático de main (pushplan) no frena el despachador: corre fuera de la pasada, y un push que falla espera cada vez más (1, 2, 4… hasta 30 min) en vez de repetirse cada 60 s con sus hooks. (check: `cd collector && uv run -q pytest -q tests -k push_plan_backoff`)
- [x] Una revisión de lane en curso no se corta cuando el collector se recarga por un cambio de su código. (check: `cd collector && uv run -q pytest -q tests -k review_survives_reload`) <!-- tirith:{"id":"i-cb38390bc26345d5a6ff71380c48e5fd","depends_on":[],"writes":[]} -->
- [x] Una revisión de lane interrumpida se reanuda sola una vez: corre separada (su propio grupo de procesos, pid guardado) y no queda en merge-failed tras un reinicio del manager o recarga del collector (check: `cd collector && uv run pytest -q -k 'review and (resume or interrupted)'`) (hecho: a79862d) <!-- tirith:{"id":"i-c1d438cc","depends_on":[],"writes":[]} -->
  - [x] Revisar: Una revisión de lane interrumpida se reanuda sola una vez: corre separada (su propio grupo de procesos, pid guardado) y no queda en merge-failed tras un reinicio del manager o recarga del collector (check: `cd collector && uv run pytest -q -k 'review and (resume or interrupted)'`)
- [x] Arreglar en main tests/test_flows.py::test_automatic_flows (falla en main y bloquea los merges de -15 y otros lanes) (check: `cd collector && uv run pytest -q tests/test_flows.py`) (hecho: 3ba733a) <!-- tirith:{"id":"i-9d51e153","depends_on":[],"writes":[]} -->
- [x] La cola de merge resuelve sola los conflictos en docs/TODO.md: se queda con la versión de main y vuelve a aplicar los reportes de plan del lane con tirith plan (check: `cd collector && uv run pytest -q -k 'todo and conflict'`) <!-- tirith:{"id":"i-36205a4e","depends_on":[],"writes":[]} -->
- [x] Ronda 2 de tirith-se-mejora-usando-tirith-15: con lane/tirith-se-mejora-usando-tirith-15 y main mergeados, tests/test_flows.py::test_automatic_flows pasa (falla 3 veces en esa rama, pasa en main) (check: `cd collector && uv run pytest -q tests/test_flows.py`) (hecho: 2d7e4f0) <!-- tirith:{"id":"i-8fc180a6","depends_on":[],"writes":[]} -->
- [x] tirith lane review --tests-only que-los-agentes-no-se-queden-cal crea el enlace node_modules en su worktree, que hoy sigue sin él tras -30 y -33 aunque repo y package.json son correctos, con un test que reproduce ese lane. (check: `cd collector && uv run -q pytest -q tests -k review_link_real_lane`) <!-- tirith:{"id":"i-62aa57f5","depends_on":[],"writes":[]} -->
  - [x] Revisar: tirith lane review --tests-only que-los-agentes-no-se-queden-cal crea el enlace node_modules en su worktree, que hoy sigue sin él tras -30 y -33 aunque repo y package.json son correctos, con un test que reproduce ese lane. (check: `cd collector && uv run -q pytest -q tests -k review_link_real_lane`)
- [x] Quitar la worktree de un lane Node no borra el node_modules de main a través del enlace: el de ~/DEVELOP/agentsmessenger desapareció tras cerrar lanes con enlace. Encontrar qué lo borra, arreglarlo y volver a instalarlo con npm ci. (check: `test -d /Users/andreseloyasanchezva/DEVELOP/agentsmessenger/node_modules/tweetnacl && cd collector && uv run -q pytest -q tests -k worktree_removal_keeps_node_modules`) (hecho: ad9c8fb) <!-- tirith:{"id":"i-bf55f5fc","depends_on":[],"writes":[]} -->
- [x] Deflake test_plangate dirty-plan test (test_report_is_kept_when_the_plan_is_dirty_and_written_later): no timing cause found by reading; needs a reproduction under load <!-- tirith:{"id":"i-919d6769"} -->
  - [x] Revisar: Deflake test_plangate dirty-plan test (test_report_is_kept_when_the_plan_is_dirty_and_written_later): no timing cause found by reading; needs a reproduction under load
- [x] Reproduce the test_plangate dirty-plan flake under load (no timing cause found by reading; the test now asserts the plan is clean before sync, so a recurrence shows why) (hecho: f8ffde6b) <!-- tirith:{"id":"i-0a756df0"} -->
- [x] When a lane ends (done, stopped, removed) or a full suite finishes, Tirith kills every process left under Tirith-lanes/<lane>/ (exe or cwd), with SIGKILL if SIGTERM is ignored (check: `cd collector && uv run -q pytest -q tests -k reap_lane_leftovers`) <!-- tirith:{"id":"i-12303492","depends_on":[],"writes":[]} -->
  - [x] Revisar: When a lane ends (done, stopped, removed) or a full suite finishes, Tirith kills every process left under Tirith-lanes/<lane>/ (exe or cwd), with SIGKILL if SIGTERM is ignored (check: `cd collector && uv run -q pytest -q tests -k reap_lane_leftovers`) — falló: the report does not match the change
  - [x] Revisar: When a lane ends (done, stopped, removed) or a full suite finishes, Tirith kills every process left under Tirith-lanes/<lane>/ (exe or cwd), with SIGKILL if SIGTERM is ignored (check: `cd collector && uv run -q pytest -q tests -k reap_lane_leftovers`)
- [x] Lane commits with the agent's default attribution trailer pass the commit-msg guard (test lane_commit_trailer) (check: `cd collector && uv run -q pytest -q tests -k lane_commit_trailer`) <!-- tirith:{"id":"i-022f4c08","depends_on":[],"writes":["collector/tirith_guard.py"]} -->
  - [x] Revisar: Lane commits with the agent's default attribution trailer pass the commit-msg guard (test lane_commit_trailer) (check: `cd collector && uv run -q pytest -q tests -k lane_commit_trailer`)
- [x] Tirith kills processes still running under Tirith-lanes/<lane>/ for lanes that already ended (sweep at collector start and every few minutes), SIGKILL if SIGTERM is ignored (check: `cd collector && uv run -q pytest -q tests -k reap_ended_lanes`) (hecho: 93fff406) <!-- tirith:{"id":"i-e3c73db3","depends_on":[],"writes":[]} -->
- [x] Load limit counts the CPU used by Tirith's own processes (lanes, CI stages, reviews) instead of host-wide idle, sampled over at least 2 s or smoothed over several readings (collector/tirith/loadlimit.py) (check: `cd collector && uv run -q pytest -q tests -k loadlimit_tirith_cpu`) <!-- tirith:{"id":"i-9c097a27","depends_on":[],"writes":["collector/tirith/loadlimit.py"]} -->
- [x] The ended-lane process sweep skips a lane worktree while a review, merge or CI stage is running in it (lane -70's review pytest workers died twice with OSError cannot send) (check: `cd collector && uv run -q pytest -q tests -k reap_skips_live_review`) <!-- tirith:{"id":"i-3e3ccab1","depends_on":[],"writes":["collector/tirith/reaper.py"]} -->
  - [x] Revisar: Load limit counts the CPU used by Tirith's own processes (lanes, CI stages, reviews) instead of host-wide idle, sampled over at least 2 s or smoothed over several readings (collector/tirith/loadlimit.py) (check: `cd collector && uv run -q pytest -q tests -k loadlimit_tirith_cpu`) — falló: the tests failed: bringing up nodes... bringing up nodes... ..........................................................
  - [x] Revisar: The ended-lane process sweep skips a lane worktree while a review, merge or CI stage is running in it (lane -70's review pytest workers died twice with OSError cannot send) (check: `cd collector && uv run -q pytest -q tests -k reap_skips_live_review`) — falló: the report does not match the change
  - [x] Revisar: The ended-lane process sweep skips a lane worktree while a review, merge or CI stage is running in it (lane -70's review pytest workers died twice with OSError cannot send) (check: `cd collector && uv run -q pytest -q tests -k reap_skips_live_review`) — falló: the tests failed: bringing up nodes... bringing up nodes... ..........................................................
- [x] Plan-update lanes skip reporting a missing credential that already has an open owner item in waits.jsonl, so the Gemini key (w-8) is not re-reported every run (check: `cd collector && uv run -q pytest -q tests -k plan_update_skips_open_owner_items`) <!-- tirith:{"id":"i-4635d78b","depends_on":[],"writes":[]} -->
  - [x] Revisar: Plan-update lanes skip reporting a missing credential that already has an open owner item in waits.jsonl, so the Gemini key (w-8) is not re-reported every run (check: `cd collector && uv run -q pytest -q tests -k plan_update_skips_open_owner_items`)
- [x] The subprocess tests in test_cta, test_idletimeout and test_timeouts reap their children in a pytest fixture so no child outlives its test (check: `cd collector && uv run -q pytest -q tests -k reap_children_fixture`) <!-- tirith:{"id":"i-59eacfd8","depends_on":[],"writes":[]} -->
  - [x] Revisar: The subprocess tests in test_cta, test_idletimeout and test_timeouts reap their children in a pytest fixture so no child outlives its test (check: `cd collector && uv run -q pytest -q tests -k reap_children_fixture`)
- [x] A GitHub run with conclusion cancelled, stale or action_required never counts as failed: github.py _status returns 'cancelled' and last_finished/ci_main skip it so the newest real result counts, with a test (check: `cd collector && uv run -q pytest -q tests -k cancelled`) <!-- tirith:{"id":"i-e90a1f87","depends_on":[],"writes":[]} -->
  - [x] Revisar: A GitHub run with conclusion cancelled, stale or action_required never counts as failed: github.py _status returns 'cancelled' and last_finished/ci_main skip it so the newest real result counts, with a test (check: `cd collector && uv run -q pytest -q tests -k cancelled`)
- [x] A lane that follows its brief is never sent back for it: the launch.py brief rule 'Do NOT commit' and the keepgoing.py finish gate 'uncommitted changes in your worktree' agree, pinned by a test (check: `cd collector && uv run pytest -q tests -k brief_and_finish_gate_agree_on_commit`) <!-- tirith:{"id":"i-2f3273de","depends_on":[],"writes":[]} -->
  - [x] Revisar: A lane that follows its brief is never sent back for it: the launch.py brief rule 'Do NOT commit' and the keepgoing.py finish gate 'uncommitted changes in your worktree' agree, pinned by a test (check: `cd collector && uv run pytest -q tests -k brief_and_finish_gate_agree_on_commit`)
- [x] An item marked (en revisión) whose lane ended done with no review, queue entry or branch left is closed by code (ticked, or reopened if its work is missing) instead of waiting forever (check: `cd collector && uv run pytest -q tests -k in_review_without_review_closes`) <!-- tirith:{"id":"i-a6488803","depends_on":[],"writes":[]} -->
  - [x] Revisar: An item marked (en revisión) whose lane ended done with no review, queue entry or branch left is closed by code (ticked, or reopened if its work is missing) instead of waiting forever (check: `cd collector && uv run pytest -q tests -k in_review_without_review_closes`)
- [x] Merge queue picks up and merges canary lanes (project canary) (check: `timeout 600 tirith canary run && tail -1 $(ls -t ~/.tirith/logs/canary-*.log | head -1) | grep -qiv FAILED`) <!-- tirith:{"id":"i-5888dfc9","depends_on":[],"writes":[]} -->
  - [x] Revisar: Merge queue picks up and merges canary lanes (project canary) (check: `timeout 600 tirith canary run && tail -1 $(ls -t ~/.tirith/logs/canary-*.log | head -1) | grep -qiv FAILED`)
- [x] full suite has 3 failing process-kill tests (test_cta lane_stop_kills_child_ignoring_sigterm, test_idletimeout child_cpu_is_progress, test_timeouts kills_tree_children_left_group), lane diff does not touch that code; lane commit f44bf3b8 not yet merged to main; no real tirith canary run done yet (hecho: 0bcd5b94)
  - [x] Revisar: Merge queue picks up and merges canary lanes (project canary) (check: `timeout 600 tirith canary run && tail -1 $(ls -t ~/.tirith/logs/canary-*.log | head -1) | grep -qiv FAILED`) — falló: the report does not match the change
- [x] launch.py brief no longer says 'Do NOT commit': it tells the lane to commit its work on the lane branch, as the finish gate expects (check: `! grep -q 'Do NOT commit' collector/tirith/launch.py && cd collector && uv run -q pytest -q tests -k launch`) <!-- tirith:{"id":"i-ffe9fdaf","depends_on":[],"writes":[]} -->
  - [x] Revisar: launch.py brief no longer says 'Do NOT commit': it tells the lane to commit its work on the lane branch, as the finish gate expects (check: `! grep -q 'Do NOT commit' collector/tirith/launch.py && cd collector && uv run -q pytest -q tests -k launch`)
- [x] The autopilot remembers a 'wait' answer per item: an item the manager waited on is not proposed again while its own facts are unchanged, even when another item of the same step changes (check: `cd collector && uv run -q pytest -q tests -k waited_item_not_reproposed`) <!-- tirith:{"id":"i-d4de34c1","depends_on":[],"writes":[]} -->
  - [x] Revisar: The autopilot remembers a 'wait' answer per item: an item the manager waited on is not proposed again while its own facts are unchanged, even when another item of the same step changes (check: `cd collector && uv run -q pytest -q tests -k waited_item_not_reproposed`)
- [x] tirith plan add gives every new item a unique permanent task id even when its title repeats an earlier item (e.g. daily Revisar el plan), and prints whether the item was added (check: `cd collector && uv run -q pytest -q tests -k plan_add_same_title_twice`) (hecho: 76e3963) <!-- tirith:{"id":"i-0bc2d450","depends_on":[],"writes":[]} -->
  - [x] Revisar: tirith plan add gives every new item a unique permanent task id even when its title repeats an earlier item (e.g. daily Revisar el plan), and prints whether the item was added (check: `cd collector && uv run -q pytest -q tests -k plan_add_same_title_twice`)
- [x] Una lane interactiva (chat) que termina con result failed no marca su elemento del plan como hecho (hoy chat-check-1793855782 marcó hecho un elemento de step 9 en d59ae20 sin línea result), con un test (check: `cd collector && uv run -q pytest -q tests -k interactive_failed_keeps_item_open`) (hecho: 447abc3) <!-- tirith:{"id":"i-ba60f388","depends_on":[],"writes":[]} -->
- [x] test_items_whose_lane_is_in_the_merge_queue_get_no_proposal (test_autopilot.py) pasa también cuando se corre solo: hoy falla así (y falló en la construcción del mapa de tests de las 03:35 UTC), aunque pasa con el archivo entero (check: `cd collector && for i in 1 2 3 4 5; do uv run -q pytest -q -p no:xdist -p no:testmon tests/test_autopilot.py -k merge_queue_get_no_proposal || exit 1; done`) <!-- tirith:{"id":"i-5bf25de9","depends_on":[],"writes":[]} -->
  - [x] Revisar: test_items_whose_lane_is_in_the_merge_queue_get_no_proposal (test_autopilot.py) pasa también cuando se corre solo: hoy falla así (y falló en la construcción del mapa de tests de las 03:35 UTC), aunque pasa con el archivo entero (check: `cd collector && for i in 1 2 3 4 5; do uv run -q pytest -q -p no:xdist -p no:testmon tests/test_autopilot.py -k merge_queue_get_no_proposal || exit 1; done`)
- [x] Reaplicar sobre main el commit 837fb6f de lane/tirith-se-mejora-usando-tirith-16 (el ítem de una lane chat o interactiva sigue abierto mientras su trabajo no está en main: _unmerged_work en plangate.lane_ended), resolviendo el conflicto en collector/tests/test_plangate.py (check: `cd collector && uv run -q pytest -q tests/test_plangate.py`) <!-- tirith:{"id":"i-bda629eb","depends_on":[],"writes":[]} -->
- [x] La app muestra la lista de peticiones del dueño (state.owner_requests) con su vínculo y estado <!-- tirith:{"id":"i-0b701ae7"} -->
  - [x] Revisar: La app muestra la lista de peticiones del dueño (state.owner_requests) con su vínculo y estado
- [x] Confirm the 30 Oct 18:58:19Z cause: the logs only show a tirith run --stage build/test from the salas-privadas lane at that moment; the test-cleanup unlock confined to temp folders is a hypothesis <!-- tirith:{"id":"i-fe48e259"} -->
  - [x] Revisar: Confirm the 30 Oct 18:58:19Z cause: the logs only show a tirith run --stage build/test from the salas-privadas lane at that moment; the test-cleanup unlock confined to temp folders is a hypothesis
- [x] Guard: record who clears a hook's uchg flag (log a line when the guard check finds the flag lost, with the hooks' ctime and the runs active at that moment) so a repeat can be attributed (hecho: 2b99391) <!-- tirith:{"id":"i-f63f8c1e"} -->
  - [x] Revisar: Guard: record who clears a hook's uchg flag (log a line when the guard check finds the flag lost, with the hooks' ctime and the runs active at that moment) so a repeat can be attributed
- [x] La rama lane/tirith-se-mejora-usando-tirith-38 (registro de fricción) se fusiona con main: resolver el conflicto en collector/tirith/cli.py conservando ambos cambios y que pasen los tests de friction. (check: `cd collector && uv run -q pytest -q tests -k friction`) (hecho: a59ac81) <!-- tirith:{"id":"i-2ffb6c95","depends_on":[],"writes":[]} -->
  - [x] Revisar: La rama lane/tirith-se-mejora-usando-tirith-38 (registro de fricción) se fusiona con main: resolver el conflicto en collector/tirith/cli.py conservando ambos cambios y que pasen los tests de friction. (check: `cd collector && uv run -q pytest -q tests -k friction`)
- [x] La rama lane/tirith-se-mejora-usando-tirith-37 (el enlace node_modules nunca entra en un commit) se fusiona con main: resolver el conflicto en collector/tests/test_worktree_node_modules.py conservando los tests de ambos lados. (check: `cd collector && uv run -q pytest -q tests -k node_modules`) <!-- tirith:{"id":"i-cc75b3b1","depends_on":[],"writes":[]} -->
  - [x] Revisar: Quitar la worktree de un lane Node no borra el node_modules de main a través del enlace: el de ~/DEVELOP/agentsmessenger desapareció tras cerrar lanes con enlace. Encontrar qué lo borra, arreglarlo y volver a instalarlo con npm ci. (check: `test -d /Users/andreseloyasanchezva/DEVELOP/agentsmessenger/node_modules/tweetnacl && cd collector && uv run -q pytest -q tests -k worktree_removal_keeps_node_modules`) — falló: the merge did not go through: merging lane/tirith-se-mejora-usando-tirith-37 failed conflicts? : Auto-merging collector/tests/tes
  - [x] Revisar: La rama lane/tirith-se-mejora-usando-tirith-37 (el enlace node_modules nunca entra en un commit) se fusiona con main: resolver el conflicto en collector/tests/test_worktree_node_modules.py conservando los tests de ambos lados. (check: `cd collector && uv run -q pytest -q tests -k node_modules`)
- [x] tirith lane review sin repo explícito también enlaza node_modules: review_lane solo lo hace si repo no es None, y la revisión de que-los-agentes-no-se-queden-cal sigue fallando con tweetnacl tras -30. (check: `cd collector && uv run -q pytest -q tests -k review_links_without_repo`) (hecho: 6971a36) <!-- tirith:{"id":"i-82a2647f","depends_on":[],"writes":[]} -->
  - [x] Revisar: tirith lane review sin repo explícito también enlaza node_modules: review_lane solo lo hace si repo no es None, y la revisión de que-los-agentes-no-se-queden-cal sigue fallando con tweetnacl tras -30. (check: `cd collector && uv run -q pytest -q tests -k review_links_without_repo`)
- [x] La comprobación de afirmaciones acepta un recuento de tests de una ejecución parcial (-k o un archivo) si esa ejecución está registrada, porque velocidad-y-autonom-a-de-un-obje-3 y -4 fallan con '167 passed' frente a 1738. (check: `cd collector && uv run -q pytest -q tests -k claim_partial_run`) <!-- tirith:{"id":"i-d5dad651","depends_on":[],"writes":[]} -->
  - [x] Revisar: La comprobación de afirmaciones acepta un recuento de tests de una ejecución parcial (-k o un archivo) si esa ejecución está registrada, porque velocidad-y-autonom-a-de-un-obje-3 y -4 fallan con '167 passed' frente a 1738. (check: `cd collector && uv run -q pytest -q tests -k claim_partial_run`)
- [x] tirith lane review también enlaza node_modules en la worktree de un lane Node si falta, porque la de que-los-agentes-no-se-queden-cal es anterior a i-28f84f45 y su revisión sigue fallando con tweetnacl. (check: `cd collector && uv run -q pytest -q tests -k review_node_modules`) (hecho: 2239cd4) <!-- tirith:{"id":"i-1d8d6566","depends_on":[],"writes":[]} -->
  - [x] Revisar: tirith lane review también enlaza node_modules en la worktree de un lane Node si falta, porque la de que-los-agentes-no-se-queden-cal es anterior a i-28f84f45 y su revisión sigue fallando con tweetnacl. (check: `cd collector && uv run -q pytest -q tests -k review_node_modules`)
- [x] El enlace node_modules que Tirith crea en la worktree de un lane no cuenta como cambio sin commit (se añade a .git/info/exclude), porque .gitignore 'node_modules/' no cubre un enlace y el lane que-los-agentes-no-se-queden-cal-2 volvió incompleto. (check: `cd collector && uv run -q pytest -q tests -k node_modules_link_excluded`) (hecho: 1308247) <!-- tirith:{"id":"i-ac6adf26","depends_on":[],"writes":[]} -->
  - [x] Revisar: El enlace node_modules que Tirith crea en la worktree de un lane no cuenta como cambio sin commit (se añade a .git/info/exclude), porque .gitignore 'node_modules/' no cubre un enlace y el lane que-los-agentes-no-se-queden-cal-2 volvió incompleto. (check: `cd collector && uv run -q pytest -q tests -k node_modules_link_excluded`)
- [x] La worktree de un lane de un proyecto Node tiene node_modules (enlace al de main o npm ci) antes de correr el lane y su revisión, porque la revisión de agentsmessenger falla con ERR_MODULE_NOT_FOUND tweetnacl. (check: `cd collector && uv run -q pytest -q tests -k worktree_node_modules`) (hecho: 8d2a0da) <!-- tirith:{"id":"i-28f84f45","depends_on":[],"writes":[]} -->
  - [x] Revisar: La worktree de un lane de un proyecto Node tiene node_modules (enlace al de main o npm ci) antes de correr el lane y su revisión, porque la revisión de agentsmessenger falla con ERR_MODULE_NOT_FOUND tweetnacl. (check: `cd collector && uv run -q pytest -q tests -k worktree_node_modules`)
- [x] tirith launch, plan check y plan done vuelven a encontrar un elemento abierto por su texto exacto (hoy solo por id i-…: 'not an open item of step N' para todo texto de docs/TODO.md) (check: `cd collector && uv run -q pytest -q tests -k 'item and text'`) <!-- tirith:{"id":"i-7619dbb1","depends_on":[],"writes":[]} -->
- [x] Una lane nunca corre la suite completa por su cuenta: corre los tests de lo que cambió, y la suite completa solo la corre la cola, que es quien decide. (check: `cd collector && uv run -q pytest -q tests/test_fasttests.py -k lane_never_runs_the_full_suite`) (hecho: 665be8a)
- [x] Liberar disco nunca bloquea el ciclo del collector: borrar lanes viejas corre en segundo plano, y el chat y el estado siguen al día mientras tanto. (check: `cd collector && uv run -q pytest -q tests -k sweep_does_not_block_the_cycle`) (hecho: f87217d)
- [x] Un hook de Tirith tarda menos de 0,3 s: hoy cada petición de un agente espera 3,7 s de mediana en hooks porque cada llamada importa todo tirith.cli. (check: `cd collector && uv run -q pytest -q tests -k hook_imports_little`) (hecho: 583aa22)
- [x] La cola no mata su suite con un límite fijo de 30 minutos ni lo llama rojo: usa el límite de la clase stage:test, y si se agota dice que se agotó el tiempo y lo repite una vez. (check: `cd collector && uv run -q pytest -q tests -k stage_timeout_is_not_red`) (hecho: c610830)
- [x] El test test_false_tick_a_merged_lane_gets_the_merge_that_brought_it_not_a_later_one no falla al azar: hoy falla 1 de cada 2 veces incluso solo. (check: `cd collector && uv run -q pytest -q tests/test_plangate.py -k test_false_tick_a_merged_lane_gets`)
- [x] Llevar a main lane/u-stagetimeout resolviendo su conflicto con el tren en mergeq.py. (check: `cd collector && uv run -q pytest -q tests -k stage_timeout_is_not_red`) (hecho: c610830)
- [x] La lista de proyectos no muestra carpetas temporales de lanes o de la cola, como _mergeq-tirith-se-mejora o amchat. (check: `cd collector && uv run -q pytest -q tests -k temp_worktrees_are_not_projects`) (hecho: a320e76)
- [x] El ciclo del collector tarda menos de 10 s de verdad con la configuración real: hoy tarda de 1 a 4 minutos, casi todo en el paso inbox, y un proyecto nuevo o un QR tardan eso en verse. (check: `cd collector && uv run -q pytest -q tests -k real_cycle_budget`)
- [x] El límite de 3 lanes esperando en la cola entra de verdad en main: wip_block no está en main aunque su elemento quedó marcado con 56448d5, un commit de otro lane. Llevar b7d1965 de lane/tirith-se-mejora-usando-tirith-12, que choca con main en mergeq.py y CONTRACT-mergeq.md. (check: `cd collector && uv run -q pytest -q tests -k wip_limit`) (hecho: f2094a7)
  - [x] Revisar: El límite de 3 lanes esperando en la cola entra de verdad en main: wip_block no está en main aunque su elemento quedó marcado con 56448d5, un commit de otro lane. Llevar b7d1965 de lane/tirith-se-mejora-usando-tirith-12, que choca con main en mergeq.py y CONTRACT-mergeq.md. (check: `cd collector && uv run -q pytest -q tests -k wip_limit`) (hecho: f2094a7)
- [x] Un proyecto recién añadido, sin tirith.toml ni docs/TODO.md, puede configurar sus pipelines: tirith plan add --title crea el plan y tirith launch arranca ese elemento aunque tirith check falle solo por manifest-missing. Hoy se bloquea con SuperPolitika. (check: `cd collector && uv run -q pytest -q tests -k new_project_bootstrap`) (hecho: 7b4890c)
  - [x] Revisar: Un proyecto recién añadido, sin tirith.toml ni docs/TODO.md, puede configurar sus pipelines: tirith plan add --title crea el plan y tirith launch arranca ese elemento aunque tirith check falle solo por manifest-missing. Hoy se bloquea con SuperPolitika. (check: `cd collector && uv run -q pytest -q tests -k new_project_bootstrap`) — falló: the tests failed: …e-mejora-usando-tirith-17/collector/.venv/bin/python' = sys.executable tests/test_gates.py:320: Ass
- [x] La suite de tests de Python vuelve a pasar entera en main tras el tren de integración. (check: `cd collector && uv run -q pytest -q -n 8`) (hecho: b133b4b)
- [x] Autopilot releases an 'en curso' or 'en revisión' marker whose lane or review does not exist or has ended (check: `cd collector && uv run -q pytest -q tests -k autopilot_releases_ghost_marker`) <!-- tirith:{"id":"i-bcbf3a48","depends_on":[],"writes":[]} -->
  - [x] Revisar: Autopilot releases an 'en curso' or 'en revisión' marker whose lane or review does not exist or has ended (check: `cd collector && uv run -q pytest -q tests -k autopilot_releases_ghost_marker`) — falló: the report does not match the change
- [x] The sentinel's 'nothing working while items are open' signal and lanes_working in the CEO snapshot count a running merge-queue or review CI run as work (check: `cd collector && uv run -q pytest -q tests -k sentinel_counts_mergeq_as_work`) <!-- tirith:{"id":"i-e47c7d54","depends_on":[],"writes":[]} -->
  - [x] Revisar: The sentinel's 'nothing working while items are open' signal and lanes_working in the CEO snapshot count a running merge-queue or review CI run as work (check: `cd collector && uv run -q pytest -q tests -k sentinel_counts_mergeq_as_work`)
- [x] A plan edit or delete that fails with 'item changed meanwhile' only because a Roadmap marker commit touched the item reloads the plan and retries once by the item's permanent id (check: `cd collector && uv run -q pytest -q tests -k plan_edit_retries_after_marker_change`) <!-- tirith:{"id":"i-0b9588a8","depends_on":[],"writes":[]} -->
  - [x] Revisar: A plan edit or delete that fails with 'item changed meanwhile' only because a Roadmap marker commit touched the item reloads the plan and retries once by the item's permanent id (check: `cd collector && uv run -q pytest -q tests -k plan_edit_retries_after_marker_change`)
- [x] When the merge queue's push of main fails, it fetches origin: a merged commit already on origin's main ends MERGED, not PUSH_FAILED, and a main that moved ahead without it is rebased and pushed once more (check: `cd collector && uv run -q pytest -q tests -k push_race_already_on_origin_is_merged`) (hecho: 8789a106) <!-- tirith:{"id":"i-68d55e65","depends_on":[],"writes":[]} -->
  - [x] Revisar: When the merge queue's push of main fails, it fetches origin: a merged commit already on origin's main ends MERGED, not PUSH_FAILED, and a main that moved ahead without it is rebased and pushed once more (check: `cd collector && uv run -q pytest -q tests -k push_race_already_on_origin_is_merged`)
- [x] Canary: el paso 'tick' de collector/tirith/canary.py espera con _plan_item_done cada POLL_S hasta 120 s antes de fallar en vez de mirar una vez, y una ejecución hace un solo commit 'Roadmap: done' (averiguar por qué 8aa7558 y d47aec6 salieron dos) (check: `python3 -m pytest -q collector/tests -k 'canary and tick'`) (hecho: 9a132b5a) <!-- tirith:{"id":"i-2f2d297f","depends_on":[],"writes":[]} -->
  - [x] Revisar: Canary: el paso 'tick' de collector/tirith/canary.py espera con _plan_item_done cada POLL_S hasta 120 s antes de fallar en vez de mirar una vez, y una ejecución hace un solo commit 'Roadmap: done' (averiguar por qué 8aa7558 y d47aec6 salieron dos) (check: `python3 -m pytest -q collector/tests -k 'canary and tick'`) — falló: the merge did not go through: the review was interrupted FileNotFoundError: [Errno 2] No such file or directory: '/Users/andresel
- [x] Lanzar no pierde el worktree: diskdiet.sweep respeta un worktree de lane con menos de 15 min (rama igual a main, lane aún sin registrar), que hoy borra antes de que arranque el agente (FileNotFoundError en _run_child de -79 y -80) (check: `python3 -m pytest -q collector/tests -k 'diskdiet and fresh'`) (hecho: 0d0641f3) <!-- tirith:{"id":"i-a425a3b6","depends_on":[],"writes":[]} -->
  - [x] Revisar: Lanzar no pierde el worktree: diskdiet.sweep respeta un worktree de lane con menos de 15 min (rama igual a main, lane aún sin registrar), que hoy borra antes de que arranque el agente (FileNotFoundError en _run_child de -79 y -80) (check: `python3 -m pytest -q collector/tests -k 'diskdiet and fresh'`)
- [x] diskdiet.free_space sobrevive a salida no UTF-8 de git o de ficheros (decodifica con errors='replace', hoy falla cada 4 s con byte 0x8e desde 19:23Z) y tras un fallo espera al menos 60 s antes de reintentar (check: `cd collector && uv run -q pytest -q tests -k 'diskdiet and utf8'`) <!-- tirith:{"id":"i-016183b0","depends_on":[],"writes":[]} -->
  - [x] Revisar: diskdiet.free_space sobrevive a salida no UTF-8 de git o de ficheros (decodifica con errors='replace', hoy falla cada 4 s con byte 0x8e desde 19:23Z) y tras un fallo espera al menos 60 s antes de reintentar (check: `cd collector && uv run -q pytest -q tests -k 'diskdiet and utf8'`)
- [x] Cola de merge: una lane cuyo proyecto no tiene etapa build o test ejecutable se marca 'needs a test stage' y va una sola vez al manager, sin reintentos ni alarma 'no merge runner alive' (check: `cd collector && uv run -q pytest -q tests -k 'mergeq and no_stage'`) <!-- tirith:{"id":"i-662e7c9b","depends_on":[],"writes":[]} -->
- [x] CEO: la alarma de un check que sigue no ok 20 min relanza al CEO como mucho una vez cada 20 min con el mismo texto, y nunca antes del next_review_min de la última revisión, salvo texto nuevo o check nuevo fallido (hoy 22 runs por hora) (check: `cd collector && uv run -q pytest -q tests -k 'ceo and alarm_storm'`) <!-- tirith:{"id":"i-49f13620","depends_on":[],"writes":[]} -->
  - [x] Revisar: CEO: la alarma de un check que sigue no ok 20 min relanza al CEO como mucho una vez cada 20 min con el mismo texto, y nunca antes del next_review_min de la última revisión, salvo texto nuevo o check nuevo fallido (hoy 22 runs por hora) (check: `cd collector && uv run -q pytest -q tests -k 'ceo and alarm_storm'`) — falló: the tests failed: …................................... [ 3%] .........................................................
- [ ] Los 6 tests rojos en muchas lanes pasan estables bajo carga: test_autopilot items_in_merge_queue, test_cta lane_stop_kills_child, test_gates git_wrapper_install, test_idletimeout child_cpu, test_plans launch_plan_refuses, test_timeouts kill_tree (o se arregla el código si main está roto) (check: `cd collector && for i in 1 2 3; do uv run -q pytest -q tests -n 8 -k '(test_autopilot and items_in_merge_queue) or (test_cta and lane_stop_kills_child) or (test_gates and git_wrapper_install) or (test_idletimeout and child_cpu) or (test_plans and launch_plan_refuses) or (test_timeouts and kill_tree)' || exit 1; done`) (reabierto: tirith-se-mejora-usando-tirith-88: the lane's work is not on main) <!-- tirith:{"id":"i-b7e34a72","depends_on":[],"writes":[]} -->
- [x] Sandbox de lanes: swift test (SwiftPM) corre en una lane bajo tirith run sin el error de sandbox-exec 'Operation not permitted' en Package.swift, que hoy bloquea los checks ZooCore del zoo (check: `cd collector && uv run -q pytest -q tests -k 'sandbox and swift'`) (hecho: 7be8ebfa) <!-- tirith:{"id":"i-1dbb2e22","depends_on":[],"writes":[]} -->
  - [x] Revisar: Sandbox de lanes: swift test (SwiftPM) corre en una lane bajo tirith run sin el error de sandbox-exec 'Operation not permitted' en Package.swift, que hoy bloquea los checks ZooCore del zoo (check: `cd collector && uv run -q pytest -q tests -k 'sandbox and swift'`)
- [x] Cola de merge: un tren que espera porque el checkout principal tiene cambios sin commit en un fichero que toca solo una lane deja entrar a las demás lanes del tren (hoy el audio del zoo espera por EnclosureLocator.swift de primeros-pasos) (check: `cd collector && uv run -q pytest -q tests -k 'mergeq and train and dirty'`) (hecho: 9037963e) <!-- tirith:{"id":"i-fe10bc99","depends_on":[],"writes":[]} -->
  - [x] Revisar: Cola de merge: un tren que espera porque el checkout principal tiene cambios sin commit en un fichero que toca solo una lane deja entrar a las demás lanes del tren (hoy el audio del zoo espera por EnclosureLocator.swift de primeros-pasos) (check: `cd collector && uv run -q pytest -q tests -k 'mergeq and train and dirty'`)
- [x] Los 8 tests rojos en muchas lanes (red_in_many_lanes) pasan 5 veces seguidas bajo carga arreglando la causa (sleeps fijos, carreras al esperar procesos muertos), sin saltarlos (check: `cd collector && for i in 1 2 3 4 5; do uv run -q pytest -q tests -n 8 -k '(test_cta and lane_stop_kills_child) or (test_timeouts and kill_tree) or (test_idletimeout and child_cpu) or (test_gates and git_wrapper) or test_fallback or test_autoreview or (test_autopilot and merge_queue) or (test_plans and launch_plan)' || exit 1; done`) <!-- tirith:{"id":"i-b78c5e87","depends_on":[],"writes":[]} -->
- [x] Get a failing lane log for the 8 tests (red in many lanes) and fix from it; they pass 54/54 under heavy local load, so the cause may be lane-only (sandbox, proxy, env) (hecho: 11255e98) <!-- tirith:{"id":"i-584d6172"} -->
- [x] Revisión de lane: si el checkout principal tiene cambios sin commit en ficheros que la lane no toca (p. ej. zoo login cms-3: 136 ficheros sucios del dueño, ninguno en cms/), la revisión pasa la lane a la cola de merge con la misma regla del tren en vez de acabar en merge-failed (check: `cd collector && uv run -q pytest -q tests -k 'autoreview and dirty and unrelated'`) (hecho: 98ce7b46) <!-- tirith:{"id":"i-1d625477","depends_on":[],"writes":[]} -->
  - [x] Revisar: Revisión de lane: si el checkout principal tiene cambios sin commit en ficheros que la lane no toca (p. ej. zoo login cms-3: 136 ficheros sucios del dueño, ninguno en cms/), la revisión pasa la lane a la cola de merge con la misma regla del tren en vez de acabar en merge-failed (check: `cd collector && uv run -q pytest -q tests -k 'autoreview and dirty and unrelated'`)
- [x] El control de trabajo terminado (rondas 1-3) no se aplica al manager de guardia: su directorio es el checkout principal del dueño, no una lane, así que nunca hace git add ni pide commit allí (hoy dejó 24048 ficheros del dueño en el índice de zoo, output/research y tools/content incluidos) (check: `cd collector && uv run -q pytest -q tests -k 'finish_check and manager'`) (hecho: 08c12e0e) <!-- tirith:{"id":"i-c651e051","depends_on":[],"writes":[]} -->
  - [x] Revisar: El control de trabajo terminado (rondas 1-3) no se aplica al manager de guardia: su directorio es el checkout principal del dueño, no una lane, así que nunca hace git add ni pide commit allí (hoy dejó 24048 ficheros del dueño en el índice de zoo, output/research y tools/content incluidos) (check: `cd collector && uv run -q pytest -q tests -k 'finish_check and manager'`) — falló: the report does not match the change
  - [x] Revisar: El control de trabajo terminado (rondas 1-3) no se aplica al manager de guardia: su directorio es el checkout principal del dueño, no una lane, así que nunca hace git add ni pide commit allí (hoy dejó 24048 ficheros del dueño en el índice de zoo, output/research y tools/content incluidos) (check: `cd collector && uv run -q pytest -q tests -k 'finish_check and manager'`)
- [x] CEO: si ceo.start choca con PlanGateError porque docs/TODO.md está sucio un momento, reintenta hasta 60 s o arranca sin el commit 'start in step 0', y un arranque fallido deja el turno pendiente para el siguiente tick (check: `cd collector && uv run -q pytest -q tests/test_ceo.py -k 'dirty_plan and start'`) (hecho: 984ba054) <!-- tirith:{"id":"i-753cb018","depends_on":[],"writes":[]} -->
- [x] Cola de merge: un cambio sin commit del checkout principal que aplica limpio encima de la merge ya no la bloquea, y sigue sin commit en el checkout tras la merge (check: `cd collector && uv run -q pytest -q tests -k 'mergeq and dirty and keeps_change'`) (hecho: cc66de96) <!-- tirith:{"id":"i-eef25a9b","depends_on":[],"writes":[]} -->
  - [x] Revisar: Cola de merge: un cambio sin commit del checkout principal que aplica limpio encima de la merge ya no la bloquea, y sigue sin commit en el checkout tras la merge (check: `cd collector && uv run -q pytest -q tests -k 'mergeq and dirty and keeps_change'`)
- [x] Limpieza de lanes: con disk_free_gb bajo 30, borra la carpeta de toda lane terminada (done, stopped o failed, fuera de la cola de merge y sin review pendiente) en todos los proyectos, no solo las viejas, guardando antes sus cambios en ~/.tirith/kept/ (check: `python3 -m pytest -q collector/tests -k 'lane_cleanup and low_disk_removes_every_ended_lane'`) <!-- tirith:{"id":"i-176dee2e","depends_on":[],"writes":[]} -->
  - [x] Revisar: Limpieza de lanes: con disk_free_gb bajo 30, borra la carpeta de toda lane terminada (done, stopped o failed, fuera de la cola de merge y sin review pendiente) en todos los proyectos, no solo las viejas, guardando antes sus cambios en ~/.tirith/kept/ (check: `python3 -m pytest -q collector/tests -k 'lane_cleanup and low_disk_removes_every_ended_lane'`)
- [x] Cola de merge: unir a main la rama lane/tirith-se-mejora-usando-tirith-94-2-2-2 (2387ca4f) cambiando _bytes_git de mergeq.py para que use la política de timeouts en vez de timeout=300 fijo (check: `git merge-base --is-ancestor 2387ca4f HEAD && cd collector && uv run -q pytest -q tests/test_timeouts.py -k no_fixed_git_timeout && uv run -q pytest -q tests -k 'mergeq and dirty'`) (hecho: f20b3ae7) <!-- tirith:{"id":"i-07ccec5b","depends_on":[],"writes":[]} -->
  - [x] Revisar: Cola de merge: unir a main la rama lane/tirith-se-mejora-usando-tirith-94-2-2-2 (2387ca4f) cambiando _bytes_git de mergeq.py para que use la política de timeouts en vez de timeout=300 fijo (check: `git merge-base --is-ancestor 2387ca4f HEAD && cd collector && uv run -q pytest -q tests/test_timeouts.py -k no_fixed_git_timeout && uv run -q pytest -q tests -k 'mergeq and dirty'`) (hecho: 2387ca4f)
- [x] Actividad: las sesiones de manager no lanzan git diff --numstat -M completo sobre el checkout principal del proyecto (se salta git_files ahí), para que un índice enorme como el del zoo no cause timeouts de git (check: `cd collector && uv run -q pytest -q tests -k 'activity and main_checkout and no_full_diff'`) <!-- tirith:{"id":"i-ea9e2759","depends_on":[],"writes":[]} -->
  - [x] Revisar: Actividad: las sesiones de manager no lanzan git diff --numstat -M completo sobre el checkout principal del proyecto (se salta git_files ahí), para que un índice enorme como el del zoo no cause timeouts de git (check: `cd collector && uv run -q pytest -q tests -k 'activity and main_checkout and no_full_diff'`)
- [x] Tests: tests/test_plangate.py::test_report_is_kept_when_the_plan_is_dirty_and_written_later pasa estable en main (quitar lo que lo hace flaky), porque hoy devuelve trabajo bueno en muchas lanes (check: `cd collector && for i in 1 2 3 4 5; do uv run -q pytest -q tests/test_plangate.py -k test_report_is_kept_when_the_plan_is_dirty_and_written_later || exit 1; done`) <!-- tirith:{"id":"i-2a232d36","depends_on":[],"writes":[]} -->
  - [x] Revisar: Tests: tests/test_plangate.py::test_report_is_kept_when_the_plan_is_dirty_and_written_later pasa estable en main (quitar lo que lo hace flaky), porque hoy devuelve trabajo bueno en muchas lanes (check: `cd collector && for i in 1 2 3 4 5; do uv run -q pytest -q tests/test_plangate.py -k test_report_is_kept_when_the_plan_is_dirty_and_written_later || exit 1; done`)
- [x] Limpieza de lanes: no borra la carpeta de una lane done cuya review aún no ha corrido o falló (hoy -99 borró audio-de-los-animales-en-streami-7 y -6 de zoo nada más terminar, y su review ya no puede correr) (check: `cd collector && uv run -q pytest -q tests -k 'lane_cleanup and keeps_done_lane_awaiting_review'`) (hecho: db8e39a8) <!-- tirith:{"id":"i-5ededbd2","depends_on":[],"writes":[]} -->
  - [x] Revisar: Limpieza de lanes: no borra la carpeta de una lane done cuya review aún no ha corrido o falló (hoy -99 borró audio-de-los-animales-en-streami-7 y -6 de zoo nada más terminar, y su review ya no puede correr) (check: `cd collector && uv run -q pytest -q tests -k 'lane_cleanup and keeps_done_lane_awaiting_review'`) — falló: the report does not match the change
  - [x] Revisar: Limpieza de lanes: no borra la carpeta de una lane done cuya review aún no ha corrido o falló (hoy -99 borró audio-de-los-animales-en-streami-7 y -6 de zoo nada más terminar, y su review ya no puede correr) (check: `cd collector && uv run -q pytest -q tests -k 'lane_cleanup and keeps_done_lane_awaiting_review'`)
- [ ] Reproduce the test_report_is_kept_when_the_plan_is_dirty_and_written_later flake under load; if it recurs, the assertion now prints the outcome error
- [x] Claim check: 'merged main into the lane branch' o 'merged lane X into this branch' en el informe de una lane no cuenta como afirmar que se fusionó en main, para que las lanes de resolver conflictos no queden bloqueadas en claims-failed (check: `cd collector && uv run -q pytest -q tests -k 'claim and merged_into_lane_branch'`) <!-- tirith:{"id":"i-189ea982","depends_on":[],"writes":[]} -->
  - [x] Revisar: Claim check: 'merged main into the lane branch' o 'merged lane X into this branch' en el informe de una lane no cuenta como afirmar que se fusionó en main, para que las lanes de resolver conflictos no queden bloqueadas en claims-failed (check: `cd collector && uv run -q pytest -q tests -k 'claim and merged_into_lane_branch'`)
- [x] Re-run the full suite on the canary-tick change: a 'changed only' run showed one F (test not identified) and the mergeq/roadmap runs timed out in the lane (hecho: 9a132b5a)
- [x] agentrun: store_diff y build_patch no lanzan git diff completo sobre el checkout principal del proyecto en sesiones de manager (ids manager-), igual que activity.lane_files (check: `cd collector && uv run -q pytest -q tests -k 'store_diff and main_checkout and no_full_diff'`) (hecho: 3d836338) <!-- tirith:{"id":"i-7985915d","depends_on":[],"writes":[]} -->
  - [x] Revisar: agentrun: store_diff y build_patch no lanzan git diff completo sobre el checkout principal del proyecto en sesiones de manager (ids manager-), igual que activity.lane_files (check: `cd collector && uv run -q pytest -q tests -k 'store_diff and main_checkout and no_full_diff'`)
- [x] Tests: tests/test_idletimeout.py::test_a_silent_stage_is_stopped_but_a_chatty_one_is_not pasa estable con -n 8 arreglando la causa (sleeps fijos, tiempos bajo carga), sin saltarlo (check: `cd collector && for i in 1 2 3 4 5; do uv run -q pytest -q tests -n 8 -k 'test_idletimeout and silent_stage_is_stopped' || exit 1; done`) (hecho: 697ed610) <!-- tirith:{"id":"i-92202be4","depends_on":[],"writes":[]} -->
- [x] Claim check: frases negadas o que prohíben ('I did not merge', 'no push, no merge', 'I won't push', 'unless it is already merged') no cuentan como afirmación de merge o push (check: `cd collector && uv run pytest -q tests/test_claims_negation.py`) <!-- tirith:{"id":"i-5b1ec404","depends_on":[],"writes":[]} -->
  - [x] Revisar: Claim check: frases negadas o que prohíben ('I did not merge', 'no push, no merge', 'I won't push', 'unless it is already merged') no cuentan como afirmación de merge o push (check: `cd collector && uv run pytest -q tests/test_claims_negation.py`) — falló: the tests failed: bringing up nodes... bringing up nodes... ..........................................................
- [x] Dispatcher: una lane que reporta done entra en la cola de merge en un tick (los canary 004819 y 012912 se quedaron en 'queued' sin aparecer en mergeq.log) (check: `cd collector && uv run pytest -q tests/test_dispatch_enqueues_done.py`) <!-- tirith:{"id":"i-48b78ed8","depends_on":[],"writes":[]} -->
  - [x] Revisar: Dispatcher: una lane que reporta done entra en la cola de merge en un tick (los canary 004819 y 012912 se quedaron en 'queued' sin aparecer en mergeq.log) (check: `cd collector && uv run pytest -q tests/test_dispatch_enqueues_done.py`) — falló: the tests failed: …........................................................ [ 90%] ...................................
- [ ] Tests: test_canary_tick_waits_for_the_plan_item_instead_of_looking_once pasa estable con -n 8 (falló en la suite de review de -111, pasa solo en main) (check: `cd collector && for i in 1 2 3; do uv run -q pytest -q tests -n 8 -k 'test_canary and waits_for_the_plan_item' || exit 1; done`) <!-- tirith:{"id":"i-29ee85b6","depends_on":[],"writes":[]} -->
- [x] tirith run --stage test: si ningún test está mapeado a los ficheros cambiados, o el proyecto no tiene mapa, corre el stage test completo y registra la pasada en vez de 'nothing to run' (check: `cd collector && uv run pytest -q tests/test_stage_unmapped_runs_all.py`) <!-- tirith:{"id":"i-0c83c036","depends_on":[],"writes":[]} -->
- [x] Mapa de tests de zoo por rutas (cms/server → node --test cms/server/tests, cms/web → vitest, Packages/ZooCore → swift test) que tirith run --stage test usa y que aparece en test_maps.zoo (check: `cd collector && uv run pytest -q tests/test_zoo_path_testmap.py`) <!-- tirith:{"id":"i-eb54e0cd","depends_on":[],"writes":[]} -->
- [x] Claim check: el contador 'added N test(s)' cuenta funciones de test en ficheros no Python: test( e it( de node:test/jest/vitest en *.test.{ts,mjs,js} y *.spec.*, y func test... de Swift en *Tests.swift (cms-27 fue devuelta aunque su diff añade un test TS) (check: `cd collector && uv run -q pytest -q tests -k 'claim and nonpython'`) <!-- tirith:{"id":"i-0f8226f2","depends_on":[],"writes":[]} -->
- [x] Plan: si un commit de docs/TODO.md hecho por Tirith falla (p. ej. ENOSPC) y deja solo líneas del plan sin commit, la siguiente escritura del plan las commitea en vez de negarse con 'docs/TODO.md has uncommitted changes' (hoy en zoo bloquea plan add/done desde las 03:00Z) (check: `cd collector && uv run -q pytest -q tests -k 'plan and leftover and uncommitted'`) <!-- tirith:{"id":"i-359180d7","depends_on":[],"writes":[]} -->
- [ ] Manager: un turno del manager (sesión o one-shot) arranca aunque falle añadir su elemento del plan porque el docs/TODO.md de un proyecto tiene cambios sin commit: se registra el fallo y el turno corre igual (test con plan sucio en otro proyecto) (check: `cd collector && uv run -q pytest -q tests -k 'manager and dirty and plan'`) (reabierto: red-suite: stage ci/test failed on lane/tirith-se-mejora-usando-tirith-123 merged with main:) <!-- tirith:{"id":"i-a85a64bb","depends_on":[],"writes":[]} -->
- [x] Swift en lanes, segunda parte: swift test de ZooCore corre en la etapa test de una lane zoo, que tras -120 aún falla con 'zoocore: Invalid manifest' por sandbox-exec sandbox_apply Operation not permitted (ci-339-test.log, 03:29Z, bloquea GPS bias, AudioPrefetch y AudioPack) (check: `cd collector && uv run -q pytest -q tests -k 'swift and manifest and lane_stage'`) (hecho: 698c32a9) <!-- tirith:{"id":"i-7fafdfbf","depends_on":[],"writes":[]} -->
- [x] El control de trabajo terminado sigue pidiendo al manager de guardia (turnos one-shot) una corrida de tests y hechos de claim check aunque i-c651e051 está hecho: el manager no tiene worktree y nunca debe recibir rondas de devolución (check: `cd collector && uv run -q pytest -q tests -k 'finish_check and manager and oneshot'`) (hecho: d459f972) <!-- tirith:{"id":"i-b2a8bb51","depends_on":[],"writes":[]} -->
- [x] test_timeouts::test_timeout_kills_tree_including_children_that_left_the_group vuelve a fallar en CI de main (ci-831 y ci-842, 1 failed de 1086) tras tres arreglos (d8874b37, 0bcd5b94): encontrar la carrera con los logs y hacerlo estable bajo carga sin saltarlo (check: `cd collector && for i in 1 2 3 4 5; do uv run -q pytest -q tests/test_timeouts.py -k children_that_left_the_group -n 8 || exit 1; done`) <!-- tirith:{"id":"i-11dc6426","depends_on":[],"writes":[]} -->
- [x] Test extremo a extremo: tirith run --project P --stage test desde el shell del agente de una lane (env del launcher) ejecuta swift test con --disable-sandbox (zoo-31 falló con sandbox_apply a las 04:03Z) (check: `cd collector && uv run -q pytest -q tests -k 'lane_agent_entry and disable_sandbox'`) (hecho: 3b883a14) <!-- tirith:{"id":"i-f7d511d0","depends_on":[],"writes":[]} -->
- [x] Merge queue: el envío de vuelta de una lane con red-suite libera el claim de la lane original antes de arrancar la lane de arreglo (hoy -129 nunca se registró: task already claimed by -123) (check: `cd collector && uv run -q pytest -q tests -k 'mergeq and send_back and claim'`) (hecho: ef3d59c0) <!-- tirith:{"id":"i-02d4ab06","depends_on":[],"writes":[]} -->
- [ ] Los logs de etapa incluyen el proyecto (logs/ci-<proyecto>-<n>-<etapa>.log), así una corrida de zoo nunca escribe en el log de tirith del mismo número (zoo ci#349 y ci#350 añadieron a logs de tirith del 2026-11-04) (check: `cd collector && uv run -q pytest -q tests -k 'stage_log and per_project'`) (en revisión) <!-- tirith:{"id":"i-f3239822","depends_on":[],"writes":[]} -->
- [ ] Lanes: xcodebuild corre en la etapa de una lane sin 'sandbox-exec: Operation not permitted' (shim como el de swift: añade -IDEPackageSupportDisableManifestSandbox=YES y -IDEPackageSupportDisablePluginExecutionSandbox=YES), hoy bloquea los items iOS de zoo (check: `cd collector && uv run -q pytest -q tests -k 'xcodebuild and sandbox'`) (en curso: tirith-se-mejora-usando-tirith-132) <!-- tirith:{"id":"i-67632e5a","depends_on":[],"writes":[]} -->
- [ ] Zoo: la etapa test lanzada desde la raíz de una lane (zoo-lanes/<lane>) corre swift con --disable-sandbox en Packages/ZooCore, igual que desde ese cwd (ci#364 y ci#366 fallan con sandbox_apply, ci#363 y ci#365 con Could not find Package.swift) (check: `cd collector && uv run -q pytest -q tests -k 'swift and sandbox and lane_root'`) <!-- tirith:{"id":"i-4d845406","depends_on":[],"writes":[]} -->
- [x] netproxy.swiftpm adds --disable-sandbox when tirith run runs in a lane (TIRITH_LANE set) under the agent's own sandbox, not only under SANDBOXED_ENV=1, since zoo lanes still fail swift test with sandbox_apply on Package.swift (check: `cd collector && uv run -q pytest -q tests/test_netproxy.py -k swift`) <!-- tirith:{"id":"i-eea5b39f","depends_on":[],"writes":[]} -->
- [x] Disk diet clear_project_builds skips a build folder only when a busy process runs inside it, not when a process runs at the repo root above it (zoo root sessions now keep zoo/android/app/build from being cleared) (check: `tirith run --project tirith --stage test -- python -m pytest collector/tests/test_diskdiet.py collector/tests/test_disk_heal_project_builds.py -q`) <!-- tirith:{"id":"i-3b499725","depends_on":[],"writes":[]} -->
- [x] Disco bajo: la limpieza borra build/ y .gradle de checkouts y worktrees de proyectos, y worktrees en .wt/.wt-salvage con la rama ya unida, sin tocar trabajo sin unir (check: `cd collector && uv run pytest -q tests/test_disk_heal_project_builds.py`) <!-- tirith:{"id":"i-f7c4bc48","depends_on":[],"writes":[]} -->
- [x] Claim check: 'main merged into the lane', 'arrived through lane 21's merge' y citar 'merged' entre comillas no cuentan como afirmar que la lane se unió a main (bloquea la review de cms-23) (check: `cd collector && uv run pytest -q tests/test_claims_merge_direction.py`) <!-- tirith:{"id":"i-c3709faa","depends_on":[],"writes":[]} -->
- [x] Disk diet clears untracked build/ and .gradle folders nested below a checkout root (like zoo/android/app/build, 6.3 GB, no gradle file left), never tracked files or busy folders (check: `tirith run --project tirith --stage test -- python -m pytest collector/tests/test_diskdiet.py -q`) (hecho: 7a2c36b1) <!-- tirith:{"id":"i-c5a3f7cb","depends_on":[],"writes":[]} -->
- [x] Merge branch lane/tirith-se-mejora-usando-tirith-117 (7a2c36b1, nested build/ cleanup in diskdiet) onto main with test_diskdiet and test_disk_heal_project_builds green (check: `git merge-base --is-ancestor 7a2c36b1 HEAD && tirith run --project tirith --stage test -- python -m pytest collector/tests/test_diskdiet.py collector/tests/test_disk_heal_project_builds.py -q`) <!-- tirith:{"id":"i-ca2387ff","depends_on":[],"writes":[]} -->
- [x] Canary tick: unir a main la rama lane/tirith-se-mejora-usando-tirith-98 (3507809b, espera hasta 120 s y un solo commit 'Roadmap: done') sobre el main actual y pasar la suite completa (check: `git merge-base --is-ancestor 3507809b HEAD && python3 -m pytest -q collector/tests -k 'canary and tick'`) (hecho: 9a132b5a) <!-- tirith:{"id":"i-1a953383","depends_on":[],"writes":[]} -->
  - [x] Revisar: Cola de merge: un cambio sin commit del checkout principal que aplica limpio encima de la merge ya no la bloquea, y sigue sin commit en el checkout tras la merge (check: `cd collector && uv run -q pytest -q tests -k 'mergeq and dirty and keeps_change'`) — falló: the tests failed: ….......................................................... [ 3%] ..................................
  - [x] Revisar: Canary tick: unir a main la rama lane/tirith-se-mejora-usando-tirith-98 (3507809b, espera hasta 120 s y un solo commit 'Roadmap: done') sobre el main actual y pasar la suite completa (check: `git merge-base --is-ancestor 3507809b HEAD && python3 -m pytest -q collector/tests -k 'canary and tick'`)
- [x] CEO: la alarma se recuerda por nombre de check y no por su texto con cifras, así que pasar de 15.5 a 15.4 GB libres no relanza al CEO antes de ALARM_REPEAT_MIN (check: `cd collector && uv run -q pytest -q tests/test_ceo.py -k 'alarm and figures'`) (hecho: de9b996a) <!-- tirith:{"id":"i-b063b764","depends_on":[],"writes":[]} -->
  - [x] Revisar: CEO: la alarma se recuerda por nombre de check y no por su texto con cifras, así que pasar de 15.5 a 15.4 GB libres no relanza al CEO antes de ALARM_REPEAT_MIN (check: `cd collector && uv run -q pytest -q tests/test_ceo.py -k 'alarm and figures'`)
- [x] CEO: no arranca una revisión por una alarma con los mismos checks y texto que la última revisión terminada que mandó directiva por ella, hasta 20 min después de esa revisión o hasta que cambie el texto (hoy 3 runs seguidos por la alarma de disco) (check: `cd collector && uv run -q pytest -q tests -k 'ceo and same_alarm_no_rerun'`) <!-- tirith:{"id":"i-e17a03de","depends_on":[],"writes":[]} -->
  - [x] Revisar: CEO: no arranca una revisión por una alarma con los mismos checks y texto que la última revisión terminada que mandó directiva por ella, hasta 20 min después de esa revisión o hasta que cambie el texto (hoy 3 runs seguidos por la alarma de disco) (check: `cd collector && uv run -q pytest -q tests -k 'ceo and same_alarm_no_rerun'`)

## 18. Salas privadas: hablar con el manager desde el móvil

- [x] Tirith crea una sala cifrada de agentsmessenger.com, entra con su clave y envía y lee mensajes compatibles con am.mjs. (check: `cd collector && uv run -q pytest -q tests -k sealed`) (hecho: c5e27f2)
- [x] Lo que escribe un dispositivo del dueño en una sala llega al manager por el mismo chat, y su respuesta vuelve cifrada a la sala. (check: `cd collector && uv run -q pytest -q tests -k roombridge`) (hecho: 2abe6fd)
- [x] `tirith room new`, `invite` y `join` con QR de un solo uso, y state.json dice a qué salas estás conectado y quién hay en cada una. (check: `cd collector && uv run -q pytest -q tests -k private_rooms`) (hecho: b6453a4)
- [x] Vista Salas en la app: crear una sala, ver su QR, ver a qué salas estás conectado y quién hay, invitar a otro manager y salir. (check: `node web/js/rooms.test.mjs`) (hecho: 83c1a37)
- [x] La cámara del iPhone escanea el QR y abre el chat cifrado en agentsmessenger.com/chat, sin instalar nada. (check: `curl -fsS https://agentsmessenger.com/chat | grep -q chat.js`) (hecho: e4fb43f)
- [x] La cola de merge no pasa los tests de Python a un lane que solo cambia site/, ios/ o docs/. (check: `cd collector && uv run -q pytest -q tests -k merge_paths`) <!-- tirith:{"id":"i-aa4f31f1"} -->
  - [x] Revisar: La cola de merge no pasa los tests de Python a un lane que solo cambia site/, ios/ o docs/. (check: `cd collector && uv run -q pytest -q tests -k merge_paths`)
- [x] El QR de invitación de las salas privadas usa la URL https://agentsmessenger.com/chat#r=…&c=…&m=… del contrato, no otra. (check: `cd collector && uv run -q pytest -q tests -k room_invite_url`) (hecho: e4fb43f)
- [x] Los agentes de una sala privada se hablan entre sí libremente, como dice docs/CONTRACT-private-rooms.md. (check: `cd collector && uv run -q pytest -q tests -k room_agents_talk`) (hecho: e4fb43f)
- [x] El CLI y el collector no se pisan el registro de una sala cuando escriben a la vez con pocos milisegundos de diferencia (hallazgo de u-salas). (check: `cd collector && uv run -q pytest -q tests -k room_record_race`) (hecho: 16b6ee1) <!-- tirith:{"id":"i-ec169514"} -->
  - [x] Revisar: El CLI y el collector no se pisan el registro de una sala cuando escriben a la vez con pocos milisegundos de diferencia (hallazgo de u-salas). (check: `cd collector && uv run -q pytest -q tests -k room_record_race`) — falló: the merge did not go through: the review was interrupted; run it again
  - [x] Revisar: El CLI y el collector no se pisan el registro de una sala cuando escriben a la vez con pocos milisegundos de diferencia (hallazgo de u-salas). (check: `cd collector && uv run -q pytest -q tests -k room_record_race`) (hecho: 16b6ee1)
- [x] Ronda 2 de salas-privadas-hablar-con-el-man-3: git merge de lane/salas-privadas-hablar-con-el-man-3 (commit 248a93a, bloqueo del registro de sala), conflictos resueltos sin tocar docs/TODO.md, tests de salas en verde, reporte solo con lo que hay en el diff (nunca decir que se mergeó) (check: `git merge-base --is-ancestor 248a93a main`) <!-- tirith:{"id":"i-2f73712b","depends_on":[],"writes":[]} -->
  - [x] Revisar: Ronda 2 de salas-privadas-hablar-con-el-man-3: git merge de lane/salas-privadas-hablar-con-el-man-3 (commit 248a93a, bloqueo del registro de sala), conflictos resueltos sin tocar docs/TODO.md, tests de salas en verde, reporte solo con lo que hay en el diff (nunca decir que se mergeó) (check: `git merge-base --is-ancestor 248a93a main`)
- [x] Una clave que entra tarde en una sala no puede leer los mensajes siguientes del dueño sin que el dueño la haya aceptado (hallazgo de u-salas, pide cambiar la página de chat). (check: `cd collector && uv run -q pytest -q tests -k room_late_key`) (hecho: 9fecb3d)
- [x] El último mensaje de una sala en la vista Rooms incluye también lo que respondió el manager. (check: `cd collector && uv run -q pytest -q tests -k room_last_message_has_the_answer`) (hecho: 9652d0b) <!-- tirith:{"id":"i-5c5c1d1e"} -->
  - [x] Revisar: El último mensaje de una sala en la vista Rooms incluye también lo que respondió el manager. (check: `cd collector && uv run -q pytest -q tests -k room_last_message_has_the_answer`)
- [x] Cada sala muestra en Tirith un QR y un link permanentes para volver a entrar desde el teléfono ya conectado, tantas veces como quieras. (check: `cd collector && uv run -q pytest -q tests -k room_open_link`) (hecho: 8437586)
- [x] Una sala da un QR nuevo de un solo uso para conectar otro teléfono, sin tener que crear otra sala. (check: `cd collector && uv run -q pytest -q tests -k room_new_qr`) (hecho: 8437586)
- [x] El QR de las salas usa agentsmessenger.com/chat y los agentes de una sala hablan libres: solo se frenan tras 30 mensajes seguidos sin el dueño, como dice el contrato actualizado. (Corrige lo escrito por los lanes con el contrato anterior.) (check: `cd collector && ! grep -rq 'github.io/Tirith/chat' tirith ../web/js && uv run -q pytest -q tests -k roombridge_brake`) (hecho: e4fb43f)

## 19. Que los agentes de una sala no se queden callados

- [x] La vista Salas muestra el estado real de cada agente: escuchando, ocupado o dormido desde hace N minutos, con quién lo puede despertar, para que el dueño vea de un vistazo por qué alguien no contesta en vez de adivinarlo. (check: `node web/js/rooms.test.mjs`) (hecho: 4eeaa0d) <!-- tirith:{"id":"i-ff72ec2d"} -->
  - [x] Revisar: La vista Salas muestra el estado real de cada agente: escuchando, ocupado o dormido desde hace N minutos, con quién lo puede despertar, para que el dueño vea de un vistazo por qué alguien no contesta en vez de adivinarlo. (check: `node web/js/rooms.test.mjs`)
- [x] Si llega a una sala un mensaje para el manager y su sesión no corre, roombridge la despierta por el mismo chat, también si lo escribe otro agente, con el freno de 30 mensajes. Un agente se calla porque terminó su turno: ~/DEVELOP/agentsmessenger/docs/agentes-callados.md (check: `cd collector && uv run -q pytest -q tests -k room_wake`) (hecho: 1ccc134) <!-- tirith:{"id":"i-90933b7f"} -->
  - [x] Revisar: Si llega a una sala un mensaje para el manager y su sesión no corre, roombridge la despierta por el mismo chat, también si lo escribe otro agente, con el freno de 30 mensajes. Un agente se calla porque terminó su turno: ~/DEVELOP/agentsmessenger/docs/agentes-callados.md (check: `cd collector && uv run -q pytest -q tests -k room_wake`)
- [x] Un lane que espera respuesta en una sala queda marcado «esperando en la sala X» y Tirith lo reanuda al llegar la respuesta, como dice ~/DEVELOP/agentsmessenger/docs/agentes-callados.md. (check: `cd collector && uv run -q pytest -q tests -k lane_room_wait`) (hecho: 1b95b48) <!-- tirith:{"id":"i-24ab8900"} -->
  - [x] Revisar: Un lane que espera respuesta en una sala queda marcado «esperando en la sala X» y Tirith lo reanuda al llegar la respuesta, como dice ~/DEVELOP/agentsmessenger/docs/agentes-callados.md. (check: `cd collector && uv run -q pytest -q tests -k lane_room_wait`)
- [x] Tirith avisa al manager cuando un lane lleva 30 minutos esperando respuesta en una sala, como dice ~/DEVELOP/agentsmessenger/docs/agentes-callados.md. (check: `cd collector && uv run -q pytest -q tests -k lane_room_timeout`) (hecho: 78c0b6e) <!-- tirith:{"id":"i-0effabb7"} -->
  - [x] Revisar: Tirith avisa al manager cuando un lane lleva 30 minutos esperando respuesta en una sala, como dice ~/DEVELOP/agentsmessenger/docs/agentes-callados.md. (check: `cd collector && uv run -q pytest -q tests -k lane_room_timeout`)

## 20. Las pruebas no interrumpen al dueño

- [x] Los tests de web/js usan el Chromium de Playwright y no Microsoft Edge, porque cada arranque de Edge pasa CoreServicesUIAgent al frente y le quita el teclado a la app que el dueño esté usando. (check: `! grep -rq msedge web/js`) (hecho: 551dbe4)
- [x] Los tests de web/js encuentran playwright-core sin depender de otro proyecto: hoy apuntan a propuestavenezuela/node_modules, que ya no existe, y PLAYWRIGHT_FROM va a VenezuelaGame como parche. (check: `! grep -rq propuestavenezuela web/js`) (hecho: 995323f)

## 21. Auditoría de accesos de los agentes

- [x] Contrato de la auditoría de accesos (solo registra, nunca bloquea ni pregunta al dueño): qué cuenta como acceso (credenciales, variables del proyecto, git push, gh, APIs, navegador), formato del registro y retención. Idea de neoid «Agentenzugriffe». (check: `test -f docs/CONTRACT-access-audit.md`) (hecho: c09a9d3) <!-- tirith:{"id":"i-c061fdfb9d4b41f2ac5b3993a44751db","depends_on":[],"writes":[]} -->
  - [x] Revisar: Contrato de la auditoría de accesos (solo registra, nunca bloquea ni pregunta al dueño): qué cuenta como acceso (credenciales, variables del proyecto, git push, gh, APIs, navegador), formato del registro y retención. Idea de neoid «Agentenzugriffe». (check: `test -f docs/CONTRACT-access-audit.md`)
- [x] Registrar cada acceso de un agente en <home>/access/<proyecto>.ndjson: agente, punto del plan, sistema, acción y hora, desde el hook de herramientas, sin guardar valores secretos. (check: `test -f collector/tirith/access_audit.py`) (hecho: ece4004) <!-- tirith:{"id":"i-12b0de4eeef3451fa471a1f0218d9c95","depends_on":[],"writes":[]} -->
  - [x] Revisar: Registrar cada acceso de un agente en <home>/access/<proyecto>.ndjson: agente, punto del plan, sistema, acción y hora, desde el hook de herramientas, sin guardar valores secretos. (check: `test -f collector/tirith/access_audit.py`)
- [x] Vista «Accesos» en la página, solo para consultar cuando haga falta: lista por agente con filtros por sistema y fecha, enlazada al agente y a su punto del plan, sin avisos ni contador en Tu turno. (check: `grep -rqi 'accesos' web/js`) <!-- tirith:{"id":"i-277688afa3b44901b3a380c5d8d575b1","depends_on":["i-12b0de4eeef3451fa471a1f0218d9c95"],"writes":[]} -->
  - [x] Revisar: Vista «Accesos» en la página, solo para consultar cuando haga falta: lista por agente con filtros por sistema y fecha, enlazada al agente y a su punto del plan, sin avisos ni contador en Tu turno. (check: `grep -rqi 'accesos' web/js`)

## 22. Empresa que crece: worker, manager y CEO según el trabajo

- [x] Cuando el trabajo vuelve a ser una tarea pequeña, el CEO y el manager on-demand se retiran tras un tiempo sin nada que hacer, y queda registrado en company.jsonl (check: `cd collector && uv run -q pytest -q tests -k roles_shrink`) (hecho: d40f90c4) <!-- tirith:{"id":"i-55fab955","depends_on":["i-609029b8"],"writes":[]} -->
  - [x] Revisar: Cuando el trabajo vuelve a ser una tarea pequeña, el CEO y el manager on-demand se retiran tras un tiempo sin nada que hacer, y queda registrado en company.jsonl (check: `cd collector && uv run -q pytest -q tests -k roles_shrink`)
- [x] Proyecto de prueba company-lab (fixtures/company-lab) con 5 escenarios en orden de tamaño (typo, feature pequeña, login en 3 partes, app en 5 pasos desatendida, fallo provocado) más llegadas a mitad, objetivo vago, petición al dueño, test inestable y reinicio (check: `test -f fixtures/company-lab/scenarios.json`) <!-- tirith:{"id":"i-443e3461","depends_on":[],"writes":[]} -->
  - [x] Revisar: Proyecto de prueba company-lab (fixtures/company-lab) con 5 escenarios en orden de tamaño (typo, feature pequeña, login en 3 partes, app en 5 pasos desatendida, fallo provocado) más llegadas a mitad, objetivo vago, petición al dueño, test inestable y reinicio (check: `test -f fixtures/company-lab/scenarios.json`)
- [x] tirith lab run <escenario> ejecuta un escenario en una copia desechable de company-lab, hace de dueño (responde, espera, se va a dormir) y escribe un informe con coste, tiempo, items a main, interrupciones al dueño y promociones con su motivo (check: `cd collector && uv run -q pytest -q tests -k lab_run`) <!-- tirith:{"id":"i-44f22a7a","depends_on":["i-443e3461"],"writes":[]} -->
  - [x] Revisar: tirith lab run <escenario> ejecuta un escenario en una copia desechable de company-lab, hace de dueño (responde, espera, se va a dormir) y escribe un informe con coste, tiempo, items a main, interrupciones al dueño y promociones con su motivo (check: `cd collector && uv run -q pytest -q tests -k lab_run`)
- [x] tirith lab compare pone lado a lado los informes de roles on-demand y roles fijos de los mismos escenarios (coste, tiempo a main, primer intento) y recomienda umbrales (check: `cd collector && uv run -q pytest -q tests -k lab_compare`) <!-- tirith:{"id":"i-14bdd967","depends_on":["i-44f22a7a"],"writes":[]} -->
  - [x] Revisar: tirith lab compare pone lado a lado los informes de roles on-demand y roles fijos de los mismos escenarios (coste, tiempo a main, primer intento) y recomienda umbrales (check: `cd collector && uv run -q pytest -q tests -k lab_compare`) — falló: the report does not match the change
  - [x] Revisar: tirith lab compare pone lado a lado los informes de roles on-demand y roles fijos de los mismos escenarios (coste, tiempo a main, primer intento) y recomienda umbrales (check: `cd collector && uv run -q pytest -q tests -k lab_compare`)
- [x] La página muestra la vista Empresa: la línea de tiempo de company.jsonl con cada promoción o retirada, cuándo y por qué (check: `node web/js/company.test.mjs`) <!-- tirith:{"id":"i-c5db6948","depends_on":["i-e92781ab"],"writes":[]} -->
  - [x] Revisar: La página muestra la vista Empresa: la línea de tiempo de company.jsonl con cada promoción o retirada, cuándo y por qué (check: `node web/js/company.test.mjs`)
- [x] Cada proyecto elige sus roles en [roles] de tirith.toml: ceo y manager pueden ser always, on-demand u off, y Tirith los tiene en always (check: `cd collector && uv run -q pytest -q tests -k roles_setting`) <!-- tirith:{"id":"i-eb7c75f3","depends_on":[],"writes":[]} -->
  - [x] Revisar: Cada proyecto elige sus roles en [roles] de tirith.toml: ceo y manager pueden ser always, on-demand u off, y Tirith los tiene en always (check: `cd collector && uv run -q pytest -q tests -k roles_setting`)
- [x] Un proyecto nuevo empieza con ceo y manager en on-demand (check: `cd collector && uv run -q pytest -q tests -k roles_default_on_demand`) <!-- tirith:{"id":"i-efb2d1de","depends_on":["i-eb7c75f3"],"writes":[]} -->
  - [x] Revisar: Un proyecto nuevo empieza con ceo y manager en on-demand (check: `cd collector && uv run -q pytest -q tests -k roles_default_on_demand`)
- [x] Con roles on-demand, el worker pasa a manager cuando la tarea no cabe en una lane, hay 2 o más items en paralelo o pide sub-agentes (check: `cd collector && uv run -q pytest -q tests -k promote_to_manager`) <!-- tirith:{"id":"i-8aa596e4","depends_on":["i-eb7c75f3"],"writes":[]} -->
  - [x] Revisar: Con roles on-demand, el worker pasa a manager cuando la tarea no cabe en una lane, hay 2 o más items en paralelo o pide sub-agentes (check: `cd collector && uv run -q pytest -q tests -k promote_to_manager`)
- [x] Cada promoción o retirada queda en <home>/company.jsonl con la regla, el valor medido y el umbral (check: `cd collector && uv run -q pytest -q tests -k company_log`) <!-- tirith:{"id":"i-e92781ab","depends_on":["i-8aa596e4"],"writes":[]} -->
  - [x] Revisar: Cada promoción o retirada queda en <home>/company.jsonl con la regla, el valor medido y el umbral (check: `cd collector && uv run -q pytest -q tests -k company_log`)
- [x] Con roles on-demand, se crea un CEO con más de 2 workers a la vez, más de 1 h de trabajo sin nadie mirando o un fallo que el self-check no arregla (check: `cd collector && uv run -q pytest -q tests -k promote_to_ceo`) (hecho: c6ac7b21) <!-- tirith:{"id":"i-609029b8","depends_on":["i-e92781ab"],"writes":[]} -->
  - [x] Revisar: Con roles on-demand, se crea un CEO con más de 2 workers a la vez, más de 1 h de trabajo sin nadie mirando o un fallo que el self-check no arregla (check: `cd collector && uv run -q pytest -q tests -k promote_to_ceo`)
- [x] Run collector tests (cd collector && uv run -q pytest -q tests) to completion and confirm green for company-lab fixture (hecho: 9a4725f4)
- [x] Fix 3 failing process-kill tests: test_cta lane_stop_kills_a_child_that_ignores_sigterm, test_idletimeout child_cpu_is_progress, test_timeouts kill_tree_including_children_that_left_the_group (hecho: 0bcd5b94)
- [x] Load limit uses real free CPU (idle % from host CPU stats) instead of the 1-min load average in collector/tirith/loadlimit.py, with tests (check: `cd collector && uv run -q pytest -q tests -k loadlimit_free_cpu`) <!-- tirith:{"id":"i-057420c4","depends_on":[],"writes":[]} -->
- [x] Check whether test_cta lane_stop_kills_a_child, test_idletimeout child_cpu/silent_lane and test_timeouts kills_tree fail on main too, then fix or quarantine (hecho: 0bcd5b94)
- [x] Fix 3 pre-existing failing tests (process-kill/idle: test_cta lane_stop_kills_a_child_that_ignores_sigterm, test_idletimeout child_cpu_is_progress, test_timeouts timeout_kills_tree_including_children_that_left_the_group) so the full suite is green (hecho: 0bcd5b94)
- [x] Confirm whether test_cta::test_lane_stop_kills_a_child_that_ignores_sigterm, test_idletimeout::test_child_cpu_is_progress and test_timeouts::test_timeout_kills_tree_including_children_that_left_the_group fail on main without the roles change; the full collector suite is not green until they pass (hecho: 0bcd5b94)
- [x] The running Tirith measures how long the on-demand CEO and manager have been idle and calls retire_if_idle (promote.py) on its regular tick, so they retire on their own (check: `cd collector && uv run -q pytest -q tests -k retire_on_tick`) <!-- tirith:{"id":"i-d78a6bf4","depends_on":[],"writes":[]} -->
  - [x] Revisar: The running Tirith measures how long the on-demand CEO and manager have been idle and calls retire_if_idle (promote.py) on its regular tick, so they retire on their own (check: `cd collector && uv run -q pytest -q tests -k retire_on_tick`)

