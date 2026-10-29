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
- [ ] Manager de guardia: atender el chat (en curso: manager-264ef4ad)
- [ ] Revisar el plan (2026-10-29) (en revisión)
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
- [ ] Verificar el payload real de Muse Code y ajustar su adaptador (en curso: u-museformat2)

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
- [ ] Confirmar el formato JSON de Muse Code con una ejecución real (desde 2026-11-02: cuota de Muse).
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
- [ ] Tirith no se bloquea solo: duplicados por elemento (no por paso) y recupera un bloqueo de git abandonado en el checkout principal. (en curso: reglas-de-c-digo-errores-de-comp)

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
- [ ] Chat de la app: adjuntar archivos con el clip y arrastrando funciona de verdad en la app de escritorio, y sin scroll horizontal. (en curso: app-de-escritorio-tauri-con-chat-7)

## 10. Plan visible en el Overview

- [x] El collector lee este archivo (casillas por sección) y calcula el progreso de cada paso.
- [x] `tirith lane run --step <n>`: cada lane dice en qué paso trabaja.
- [x] Tarjeta «Plan» en el Overview: pasos, progreso, estado y agentes trabajando en cada uno.
- [x] Fecha límite opcional con cuenta atrás.

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
- [ ] Un solo manager con sesión persistente que vive en Tirith (misma memoria con la app abierta o cerrada).
  - [ ] Revisar: Un solo manager con sesión persistente que vive en Tirith (misma memoria con la app abierta o cerrada). (en curso: revisión de manager-de-guardia-chat-y-piloto-2)
- [x] El chat muestra el trabajo del manager en vivo: texto, herramientas y resultados de cada paso (secretos tapados).
- [ ] El chat del manager como el CLI: texto palabra a palabra, diffs en color, lista de tareas; Esc interrumpe y ↑ recupera.
- [ ] Sub-agentes anidados en la conversación; permisos seguros aprobados por código y el resto como botones Allow/Deny.
- [x] Autorrevisión cada minuto (collector, manager, piloto, disco, revisiones) con arreglo automático y una línea en el chat.
- [x] Lo que dice un agente en su informe se verifica contra su cambio real antes de fusionar.
- [ ] El collector vuelve a tardar menos de 3 s por ciclo (hoy ~20 s) y un test lo vigila.
- [ ] Fusionar nunca es una pregunta para el dueño: si los tests y la verificación pasan, Tirith fusiona solo; un duplicado o conflicto va a la cola del manager. Nunca dos agentes en el mismo elemento.
- [ ] Regla en código, sin excepciones: ningún agente arranca sin estar ligado a un elemento abierto del plan (revisiones y revisiones del plan incluidas: Tirith añade su elemento).
- [ ] Regla en código: ningún agente termina sin informar a Tirith del resultado de su elemento (hecho / en parte + nuevos elementos / bloqueado + motivo); Tirith es el único que escribe el plan y marca «hecho» solo cuando la revisión pasa.
- [ ] Llevar a main el commit 0a71483 de la rama lane/chat-wrap-long-reply-buttons-dra-2 (botones de respuesta largos que se ajustan, adjuntar o arrastrar cualquier archivo al chat): resolver los conflictos con main y pasar los tests
- [ ] El collector se reinicia solo cuando cambia su código en main (así ningún cambio fusionado queda sin cargar).

## 15. Collector y CLI en Rust

- [x] Base: crate Rust compartido (workspace con la app), arnés de paridad que corre el collector de Python y el de Rust sobre las mismas carpetas de prueba y compara `state.json`.

## Orden

1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9, y 10 en cuanto se pidió.

Hasta el 4 de noviembre (entrega del hackathon): activar y proteger los hooks (3), el bloqueo de
sub-agentes fuera de `lane run` (7), los logs legibles (8) y el plan en el Overview (10).
