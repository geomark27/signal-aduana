# Bitácora — Signal Aduana de escritorio

Una entrada por tarea: cada rama `tarea/N.M-<tema>` que llega a `develop` por PR. La entrada viaja
en el mismo commit que el cambio que describe y vive en la carpeta de su fase, `fase-N/` (la fase
agrupa tareas; no es una rama).

La historia y el porqué de cada decisión van aquí, no en comentarios del código.

## Índice

La numeración es global y correlativa: no se reinicia en cada fase, porque las entradas se citan
por número.

### Fases 1 a 3

Sin entradas: se hicieron antes de que existiera la bitácora. Su historia está en
`docs/PLAN-FASE-1.md` a `PLAN-FASE-3.md`.

### Fase 4 — Bandeja y avisos

| # | Tarea | Rama | Fecha | Resumen |
|---|---|---|---|---|
| [001](./fase-4/001-preferencias-avisos.md) | 4.1 Preferencias, resumen y avisos | `tarea/4.1-preferencias-avisos` | 2026-11-16 | Entrada retroactiva |
| [002](./fase-4/002-interfaz-preferencias.md) | 4.2 Interfaz de preferencias | `tarea/4.2-interfaz-preferencias` | 2026-11-16 | Entrada retroactiva |
| [003](./fase-4/003-normas.md) | 4.0 Normas de trabajo y bitácora | `tarea/4.0-normas` | 2026-11-16 | `CLAUDE.md`, bitácora, skill y configuración de `delegar` |
| [004](./fase-4/004-flujo-directo.md) | 4.0 Flujo directo `develop → tarea` | `tarea/4.0-flujo-directo` | 2026-11-16 | Sin rama de fase: cada tarea nace de `develop` y vuelve por un solo PR; normas y `delegar` ajustados |
| [005](./fase-4/005-monitor.md) | 4.3 Monitor: ciclo de lectura, tooltip y avisos | `tarea/4.3-monitor` | 2026-11-16 | `monitor::ciclo` sin Tauri, con `Salida` para el tooltip y las notificaciones; sin sesión, token rechazado y fallo de red probados con el simulado y `wiremock` |
| [006](./fase-4/006-bandeja.md) | 4.4 Bandeja, cierre a la bandeja, una instancia y arranque oculto | `tarea/4.4-bandeja` | 2026-11-16 | `src/escritorio.rs`: ícono con menú, cerrar la ventana la esconde según la preferencia, un segundo arranque trae la ventana al frente, `--oculto`; si la bandeja falla, la app sigue alcanzable |
| [007](./fase-4/007-integracion.md) | 4.5 Integración: monitor en marcha, avisos, inicio con Windows y ganchos de sesión | `tarea/4.5-integracion` | 2026-11-16 | `vigilancia.rs` (bucle de 15 min sin ciclos simultáneos) e `inicio.rs` (inicio con Windows según la preferencia; en simulado no toca el sistema); notificaciones reales, tooltip al cambiar de sesión y plugins `notification` y `autostart` |
| [008](./fase-4/008-guia-windows.md) | 4.6 Guía para compilar, instalar y probar en Windows | `tarea/4.6-guia-windows` | 2026-11-16 | `docs/COMPILAR-WINDOWS.md`: requisitos, instalador, SmartScreen, modo simulado y real, lista de comprobación en Windows, datos y desinstalación; lo no confirmable queda «a verificar» |
| [009](./fase-4/009-cierre-fase-4.md) | 4.7 Cierre de la fase 4 | `tarea/4.7-cierre-fase-4` | 2026-11-16 | Quita las marcas de conflicto que quedaron en este índice; `PLAN-FASE-4.md` con el reparto real (4.1 a 4.9), el estado y los pendientes; `Makefile` con los atajos del flujo para worktrees (`tarea`, `commit`, `ship`, `finish`, `prune`, `check`…) |
| [010](./fase-4/010-aviso-inicio.md) | 4.8 Aviso si falla «Iniciar con Windows» | `tarea/4.8-aviso-inicio` | 2026-11-16 | Si se pide activar el inicio con Windows y vuelve apagado, el panel de preferencias lo explica y sigue abierto; si cambia por dentro con la interfaz activa, aviso en `#notice` |
| [011](./fase-4/011-verificar-bitacora.md) | 4.9 Script que verifica la bitácora | `tarea/4.9-verificar-bitacora` | 2026-11-16 | `scripts/verificar-bitacora.sh`: enlaces del índice, entradas huérfanas, fase y secciones, numeración, marcas de conflicto y URLs o correos; solo lee, sale con 1 si algo falla |
| [012](./fase-4/012-formato-ui.md) | 4.10 Formato oficial de `ui/app.js` con Prettier | `tarea/4.10-formato-ui` | 2026-11-16 | `.prettierrc.json` (`tabWidth: 4`, reproduce el formato actual sin tocar el archivo), `.prettierignore` (Prettier solo sobre `ui/app.js`) y `make ui-format` / `make fmt-ui`, con la versión fijada; `ui-format` entra en `make check` |

### Fase 5 — Validar en Windows y cerrar deuda técnica

| # | Tarea | Rama | Fecha | Resumen |
|---|---|---|---|---|
| [013](./fase-5/013-plan-fase-5.md) | 5.0 Plan de la fase 5 y roadmap | `tarea/5.0-plan-fase-5` | 2026-11-17 | `ROADMAP.md` y `PLAN-FASE-5.md` registrados y aprobados; resumen semanal en el primer día laborable (decisión del autor); ejecutores mixtos (Sonnet para la 5.4 y la 5.5, opencode free para la 5.6); `delegar.json` apunta al plan de la fase 5 |
| [014](./fase-5/014-monitor-resiliente.md) | 5.5 El monitor sobrevive a un pánico del ciclo | `tarea/5.5-monitor-resiliente` | 2026-11-17 | `vigilancia::bucle`, que se prueba sin Tauri; cada ciclo corre bajo `catch_unwind`: un pánico se registra (mensaje recortado a 200), el tooltip pasa a «Error interno, se reintenta» y el bucle sigue; `monitor::texto_error_interno` |
| [015](./fase-5/015-preferencias-al-instante.md) | 5.6 Los cambios de preferencias llegan al monitor al instante | `tarea/5.6-preferencias-al-instante` | 2026-11-17 | `Preferencias::afecta_al_monitor` (avisos y feriados, no inicio ni bandeja); `guardar_preferencias` pide un ciclo con `pedir_ciclo` solo si cambió algo que usa el monitor |
| [016](./fase-5/016-glosario-ingles.md) | 5.9 Glosario español → inglés y regla de idioma | `tarea/5.9-glosario-ingles` | 2026-11-17 | `docs/GLOSARIO.md` (nombres nuevos por tarea, claves JSON/TOML, valores visibles que no cambian, colisiones resueltas); regla de idioma en `CLAUDE.md`; ola T (5.10 a 5.14, en serie) en el plan; códigos de error en inglés sin mostrarse, `--hidden` |
| [017](./fase-5/017-dominio-en-ingles.md) | 5.10 Dominio y paridad en inglés | `tarea/5.10-dominio-en-ingles` | 2026-11-17 | `src/dominio/` → `src/domain/` traducido; claves de la vista en inglés (`today`, `week`, `board`, `issues`, `byBoard`…) en Rust, `ui/app.js` y paridad; `tests/parity/` con los 31 casos regenerados (mismos valores); `tests/parity.rs`, `examples/dump_simulated.rs` |
| [018](./fase-5/018-jira-en-ingles.md) | 5.11 Jira en inglés | `tarea/5.11-jira-en-ingles` | 2026-11-17 | `src/jira/` traducido (`model.rs`, `simulated.rs`; `Issue`, `Worklog`, `User`, `Source`, `SimulatedJira`…); contrato `byKey`/`failed`/`name` en Rust y `ui/app.js`; volcados idénticos; glosario: `Estimate::Unset` y textos para quien programa en inglés |
| [019](./fase-5/019-base-en-ingles.md) | 5.12 Base de la app en inglés | `tarea/5.12-base-en-ingles` | 2026-11-17 | `AppError`/`AppResult` (JSON `{ code, message }`, códigos en inglés), `secret_store`, `auth`, `config`, `cache` (`Snapshot`), `session` (`AppState`, `SessionInfo`); `ui/app.js` deja de mostrar « [código]»; variantes `.simulated.` de config y caché |
| [020](./fase-5/020-escritorio-en-ingles.md) | 5.13 Escritorio y comandos en inglés | `tarea/5.13-escritorio-en-ingles` | 2026-11-17 | `commands`, `preferences`, `notifications`, `summary`, `monitor`, `watcher`, `desktop`, `autostart`, `lib` traducidos; 13 comandos y sus parámetros en inglés (comprobados contra `ui/app.js`); claves de preferencias, eventos `signal://refresh`/`preferences`, `preferences.toml`, `notifications.json`, `--hidden` |
| [021](./fase-5/021-interfaz-en-ingles.md) | 5.14 Interfaz en inglés y cierre de la traducción | `tarea/5.14-interfaz-en-ingles` | 2026-11-17 | `ui/app.js` (nombres, comentarios y consola; `vista` → `viewData`), `id`/`name`/`data-holiday` de `index.html`; textos para quien programa de `src/jira/`; 786 literales y 214 textos visibles idénticos; regla de idioma cerrada en `CLAUDE.md` |
| [022](./fase-5/022-guia-confirmada.md) | 5.7 Guía de Windows al día y segunda pasada | `tarea/5.7-guia-confirmada` | 2026-11-17 | Guía: `robocopy /MIR`, segunda pasada (5 puntos), inicio con Windows solo en modo real; plan y roadmap con la ola T; decisiones del autor: solo `.exe` NSIS como Nudge y desinstalación que conserva config, preferencias, token y caché (5.15); los «a verificar» siguen sin datos |
| [023](./fase-5/023-instalador-nsis.md) | 5.15 Instalador como Nudge: solo NSIS por usuario | `tarea/5.15-instalador-nsis` | 2026-11-17 | `tauri.conf.json`: solo `nsis`, `currentUser`, en español; `windows/hooks.nsh` borra al desinstalar (no al actualizar) avisos, `EBWebView` y `StartupApproved\Run`, y conserva config, preferencias, caché y token; guía sin MSI. Sin probar en Windows |
| [024](./fase-5/024-pruebas-avisos.md) | 5.4 Resumen semanal en el primer día laborable y pruebas | `tarea/5.4-pruebas-avisos` | 2026-11-17 | Entrada retroactiva (escrita en la 5.8; el PR #16 se fusionó sin ella): `primer_laborable` en el resumen y `decidir` lo usa en vez del lunes; 11 pruebas (una de `spent` sin efecto real) |
| [025](./fase-5/025-cierre-fase-5.md) | 5.8 Cierre de la fase 5 | `tarea/5.8-cierre-fase-5` | 2026-11-17 | «Estado al cierre» en el plan (13 tareas, PR #15 a #26, criterios con su estado real); roadmap: fase 5 hecha y pendientes de Windows heredados por la fase 6 |

### Fase 6 — Validar con el Jira real

| # | Tarea | Rama | Fecha | Resumen |
|---|---|---|---|---|
| [026](./fase-6/026-plan-fase-6.md) | 6.0 Plan de la fase 6 | `tarea/6.0-plan-fase-6` | 2026-11-18 | `PLAN-FASE-6.md`: reglas de datos reales (el autor prueba y reporta formas anonimizadas), 6.H heredado de Windows, 6.1 a 6.5 con lo que mirar en el cliente real, 6.7 diagnóstico anónimo (aprobada); decisiones: respaldo por palabras para PR, ids de campos fijos; Tecnología ya autorizó los tokens; `delegar.json` al plan nuevo |

## Formato de cada entrada

Archivo `fase-N/NNN-<tema>.md`: `N` es la fase, `NNN` el número correlativo global de tres dígitos
y el tema, el de la rama sin el `N.M-`. La primera entrada de una fase crea su carpeta y su
sección en el índice. Encabezado y secciones obligatorias:

```markdown
# NNN — Título corto

**Fecha:** AAAA-MM-DD
**Rama:** tarea/N.M-<tema> → develop
**Fase:** N
**Ejecutor:** modelo(s) que hicieron la tarea y vueltas de corrección (o «orquestador»)

## 1. Solicitud
Qué se pidió, en lenguaje natural.

## 2. Motivación
Por qué hacía falta. Problema → daño real si no se hace.

## 3. Diseño técnico
Decisiones tomadas y sus razones, incluidas las alternativas descartadas y las decisiones
que tomó el autor (indicarlo).

## 4. Archivos
Cada archivo tocado con una línea sobre su cambio.

## 5. Cómo verificar
Comandos concretos. Distingue los que se corrieron de los que no.

## 6. Pendientes
Lo que quedó fuera o se descubrió en el camino. Omitir si no hay.
```

## Reglas

- La fase del encabezado y la carpeta coinciden.
- Una entrada de una tarea ya fusionada **no se reescribe**. Solo admite notas aclaratorias al
  final, con fecha. Lo nuevo va en la entrada de la tarea nueva.
- **Hechos, no intenciones**: no se anota como hecho lo que no se verificó; si algo no se probó
  (por ejemplo, contra el Jira real o en Windows), se dice en Pendientes.
- **Una rama = una entrada = un dueño**: la escribe el orquestador a partir de los
  `DEVLOG CONTRIBUTION` de los ejecutores.
- Sin datos reales: ni tokens, ni contraseñas, ni URLs o correos de instancias, ni datos de
  personas o clientes.
- Español, directo y concreto: «la semana anterior sin feriados tenía meta 0», no «se corrigió un
  error».
