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
- [ ] Confirmar el formato JSON de Muse Code con una ejecución real (capturada el 2026-10-29; adaptador en main, lane museformat2).
- [x] Codex y Copilot instalados (dueño).
- [ ] Codex y Copilot probados con `lane run` (presets por alias, salida JSON, permisos). (en revisión)
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
  - [ ] Revisar: Tirith no se bloquea solo: duplicados por elemento (no por paso) y recupera un bloqueo de git abandonado en el checkout principal. (en curso: revisión de reglas-de-c-digo-errores-de-comp)
- [ ] Todo comando tiene timeout por código (collector, etapas y comandos de los agentes): Tirith lo aprende del historial, los agentes pueden subirlo o bajarlo dentro de límites con motivo, y al vencer se detiene el proceso con sus hijos y se limpian sus bloqueos. (en revisión)
- [ ] Un elemento se marca hecho en cuanto su trabajo está en main y verificado, aunque la lane se haya fusionado sola; lo ya fusionado hoy se marca con su commit. (en revisión)
- [ ] Llevar a main el trabajo de la lane reglas-de-c-digo-errores-de-comp: aplicar ~/.tirith/diffs/reglas-de-c-digo-errores-de-comp.patch sobre main y reportar solo lo que está en el diff (2 tests nuevos, no 3; no nombrar index.lock como archivo cambiado). (bloqueado: Patch already in main nothing to apply, tree clean ; the test stage was killed exit 144 at ~67% with no failures, so a full pass was never confirmed.)
- [ ] Integrar: lane/reglas-de-c-digo-errores-de-comp (duplicados por elemento y bloqueo de git abandonado). (en revisión)
- [ ] Los agentes no piden permisos: dentro de su copia pueden ejecutar cualquier comando y solo los hooks de Tirith deciden (reglas actuales + límite: escribir solo en su copia y su temporal, nunca leer credenciales, sin servidores ni puertos, sin matar procesos ajenos). (en revisión)
- [ ] Cada bloqueo de un hook enseña la alternativa exacta y el agente sigue; varios bloqueos seguidos van al manager, nunca al dueño. (en revisión)
- [ ] Tests rápidos en la lane (solo lo relacionado con el cambio) y suite completa en la cola de fusión; el paralelismo se ajusta a CPU y disco. (en revisión)
- [ ] El tiempo límite de un agente mide inactividad (sin salida ni progreso durante N minutos), no el tiempo total; una lane que sigue trabajando nunca se corta por durar mucho. (en revisión)

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
- [ ] Confirmar qué agentes existen en cada plataforma.
- [x] Empaquetado por plataforma (configuración y scripts de build).
- [x] Firmar la app de macOS con el Developer ID del dueño.
- [x] Notarizar la app de macOS (`scripts/notarize-macos.sh`, perfil `tirith-notary`; aceptada por Apple el 2026-10-29).
- [ ] Muse protocol probe: read muse serve help and schema
- [ ] Chat de la app: adjuntar archivos con el clip y arrastrando funciona de verdad en la app de escritorio, y sin scroll horizontal. (reabierto: the merge did not go through: merging lane/app-de-escritorio-tauri-con-chat-7 failed conflicts? : Auto-merging app/README.md CONF)
  - [x] Revisar: Chat de la app: adjuntar archivos con el clip y arrastrando funciona de verdad en la app de escritorio, y sin scroll horizontal. — falló: the merge did not go through: merging lane/app-de-escritorio-tauri-con-chat-7 failed conflicts? : Auto-merging app/README.md CONF
- [ ] Los mensajes del chat nunca se cortan ni se salen por la derecha: texto, rutas y código largos se parten o tienen su propio scroll dentro del bloque. (en revisión)
- [ ] Probar a mano en la app Tauri real (npm run tauri dev): clip, arrastrar y sin scroll horizontal; no hubo sesión gráfica
- [ ] Integrar en main la rama lane/chatoverflow (mensajes del chat sin cortes). (en revisión)
- [ ] Llevar a main el cambio de la lane app-de-escritorio-tauri-con-chat-7 (clip, arrastrar, sin scroll horizontal; tests ya verdes): reaplicar a mano /Users/andreseloyasanchezva/.tirith/diffs/app-de-escritorio-tauri-con-chat-7.patch sobre main, sin rediseñar, y resolver el conflicto en app/README.md conservando lo de main. (reabierto: the merge did not go through: the review was interrupted; run it again)
  - [x] Revisar: Llevar a main el cambio de la lane app-de-escritorio-tauri-con-chat-7 (clip, arrastrar, sin scroll horizontal; tests ya verdes): reaplicar a mano /Users/andreseloyasanchezva/.tirith/diffs/app-de-escritorio-tauri-con-chat-7.patch sobre main, sin rediseñar, y resolver el conflicto en app/README.md conservando lo de main. — falló: the tests failed: …tauri-con-chat-8/collector/.venv/lib/python3.12/site-packages/_pytest/pathlib.py:123: PytestWarning
- [ ] La app de escritorio se recompila y se reinicia sola cuando cambia su código Rust en main (firmada), sin que el dueño tenga que hacer nada.
- [ ] Integrar en main el arreglo de adjuntos (clip y arrastrar) de la lane app-de-escritorio-tauri-con-chat-8, hoy sin commit. (en revisión)
- [ ] Arrastrar archivos al chat funciona de verdad en la app de escritorio, y la miniatura adjunta se ve bien (✕ pequeño dentro de la esquina, nombre y tamaño). (en curso: u-dragdrop)

## 10. Plan visible en el Overview

- [x] El collector lee este archivo (casillas por sección) y calcula el progreso de cada paso.
- [x] `tirith lane run --step <n>`: cada lane dice en qué paso trabaja.
- [x] Tarjeta «Plan» en el Overview: pasos, progreso, estado y agentes trabajando en cada uno.
- [x] Fecha límite opcional con cuenta atrás.
- [ ] Cabecera de un paso del Plan con varios agentes: avatares apilados (+N) y un solo botón, sin tapar el título ni la barra de progreso. (en revisión)

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
- [ ] Settings (y todas las vistas) con el mismo margen interior que la cabecera: el texto de introducción y las tarjetas no pegados al borde izquierdo. (en revisión)

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
- [ ] El piloto automático no lanza agentes: avisa al manager en el chat de cada paso listo y el manager decide (lanzar con buenas instrucciones, juntar, esperar o descartar). (en revisión)
- [ ] Tirith libera disco solo cuando queda poco (carpetas de lanes ya fusionadas y cachés de compilación) y nunca se lo pide al dueño. (en revisión)
- [ ] El manager persistente arranca siempre: comparte el elemento fijo del manager con los turnos sueltos (sin bloquearse por «ya en curso») y, si falla, el modo de respaldo responde en menos de 1 minuto. (en revisión)
- [ ] Los permisos de comandos seguros (solo lectura, comandos de Tirith) se aprueban por código; al dueño nunca le llega una pregunta de permiso que no sea suya.
- [ ] La autorrevisión no escribe en el chat lo que arregló sola: queda en el punto de salud (al hacer clic); el chat solo muestra lo que necesita al dueño.
- [ ] Llevar a main el trabajo de la lane manager-de-guardia-chat-y-piloto-2: aplicar a mano ~/.tirith/diffs/manager-de-guardia-chat-y-piloto-2.patch sobre main y resolver los conflictos en collector/tirith/agentrun.py y collector/tirith/managerd.py conservando lo que ya hay en main (los tests de la lane pasaban: 1113).
- [ ] Bucle «sigue hasta terminar» por código: si al acabar falta informe, verificación, tests o rama lista, Tirith reanuda la misma sesión con la lista de lo que falta (hasta 3 veces) y solo después avisa al manager. (en revisión)
- [ ] Cola única de fusión: los agentes no tocan main; Tirith fusiona de uno en uno (trae main, suite completa, fusiona, empuja) y un conflicto vuelve al mismo agente. (en revisión)
- [ ] Elementos pequeños con su comprobación: el manager parte cada pedido en elementos de ~30 min con un comando de «hecho»; Tirith rechaza elementos enormes. (en revisión)
- [ ] Integrar en main todo el trabajo terminado y sin fusionar de las lanes (lista del 2026-10-29) y descartar con motivo lo reemplazado. (en revisión)
- [ ] README y docs/SUBMISSION.md al día para el jurado del hackathon: qué hace Tirith hoy (autonomía con reglas en código, plan, manager en el chat, app notarizada), cómo usa los modelos de Meta, cómo probarlo y capturas reales. (en revisión)
- [ ] Quitar todo el GDScript: la carpeta world/ (ciudadela 3D de Godot), ./run --3d y toda referencia a Godot en el código, la web y los docs. (en revisión)
- [ ] Integrar en main lane/smallitems (elementos pequeños con comprobación; ya commiteado, cortado por el límite de 1 h antes de fusionar). (en revisión)
- [ ] Integrar en main las lanes terminadas teach, mergeq, fasttests, pilotask, idletimeout y diskauto (resolviendo conflictos). (en curso: u-land6)

## 15. Collector y CLI en Rust

- [x] Base: crate Rust compartido (workspace con la app), arnés de paridad que corre el collector de Python y el de Rust sobre las mismas carpetas de prueba y compara `state.json`.

## Orden

1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9, y 10 en cuanto se pidió.

Hasta el 4 de noviembre (entrega del hackathon): activar y proteger los hooks (3), el bloqueo de
sub-agentes fuera de `lane run` (7), los logs legibles (8) y el plan en el Overview (10).
