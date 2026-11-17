# Glosario español → inglés del código

Fuente única de los nombres en inglés para la traducción del código (tareas 5.10 a 5.14,
`docs/PLAN-FASE-5.md`). Si un nombre está aquí, se usa **exactamente** este. Si no está (variables
locales, campos privados, auxiliares de prueba), se traduce siguiendo las convenciones de la
sección 1. Si algo no encaja, el ejecutor no inventa: lo reporta.

Decisiones del autor (2026-11-17): el código va en inglés; lo que ve el usuario de la app y la
documentación del repo siguen en español; los atajos de `make` y los scripts conservan su nombre;
la app no está instalada en ningún equipo, así que los archivos y las claves en disco se renombran
sin migración.

## 1. Convenciones

- **Qué no cambia (texto visible o dato):**
  - Mensajes de error, avisos, tooltips, títulos, menú de la bandeja e interfaz.
  - `title` y `detail` de las horas.
  - Valores mostrados: «Todos», «Sin tablero», «Sin componente», nombres de tableros, componentes y
    estados de Jira («Por hacer», «En curso»…), «Subtarea», las palabras de `NO_DOC_RE`,
    «Despliegue», los textos de `boardHow`, los datos del simulado (proyectos, resúmenes,
    prioridades, «Persona de Prueba») y el token de prueba «rechazar».
  - La regex `/prueba|producci/i` de `ui/app.js`, que se aplica sobre los nombres de estado.
  - Se traduce el **nombre** de la constante que guarda uno de estos valores, nunca su **valor**.
- **Qué no se toca nunca:**
  - `pruebas/paridad/legado.js`: se mueve con su carpeta sin cambiar su contenido. `generate.mjs`
    sigue llamando a su API en español (`crearLegado`, `estado`, `isFeriado`, `feriadosExtra`).
  - La clave de `localStorage` `spe.feriados`: sirve para migrar lo guardado en el navegador.
  - El identificador `ec.com.torresytorres.signal`, el paquete `signal-aduana` / `signal_aduana` y
    el servicio de la credencial `signal-aduana:<host>`.
  - La variable de entorno `SIGNAL_SIMULADO`: es una herramienta del autor, citada en el `Makefile`
    y en la documentación.
- **Módulos de prueba:** `mod pruebas` → `mod tests`. Los nombres de las pruebas también se
  traducen, en *snake_case* descriptivo (`cierre_ya_enviado_hoy` → `end_of_day_already_sent_today`).
  Siempre el mismo número de pruebas.
- **Comentarios:** todos en inglés, igual de breves (`//`, `///`, `//!`, `/* */` en JS). Los
  prefijos de log de `eprintln!` usan el módulo en inglés (`"watcher: …"`).
- **Verbos frecuentes:**

  | Español | Inglés |
  |---|---|
  | `cargar` | `load` |
  | `guardar` | `save` |
  | `leer` | `read` |
  | `borrar` | `delete` |
  | `ruta` | `path` |
  | `nuevo` | `new` |
  | `decidir` | `decide` |
  | `aplicar` | `apply` |
  | `validar_x` | `validate_x` |
  | `es_x` | `is_x` |
  | `texto_x` | `x_text` |

- **Estado:**
  - Estado de la app → `AppState`.
  - Estado de una tarea en Jira → `Status`.
  - Estado calculado de una semana u horas → `…State`.
- **Simulado:** `simulado` → `simulated` (`SIM`, `sim` en JS se quedan), y `.simulado.` en los
  nombres de archivo → `.simulated.`.

## 2. Tarea 5.10 — Dominio y paridad

Módulo `dominio` → `domain` (`src/domain/`).

| Archivo | Nuevo |
|---|---|
| `dominio/mod.rs` | `domain/mod.rs` |
| `calendario.rs` | `calendar.rs` |
| `horas.rs` | `hours.rs` |
| `semana.rs` | `week.rs` |
| `tablero.rs` | `board.rs` |
| `tarea.rs` | `issue.rs` |
| `vista.rs` | `view.rs` |

**`calendar`**

| Español | Inglés |
|---|---|
| `FeriadosExtra` | `ExtraHolidays` |
| `META_SEMANA_H` | `WEEK_TARGET_H` |
| `META_DIA_H` | `DAY_TARGET_H` |
| `DESFASE_HORAS` | `UTC_OFFSET_HOURS` |
| `FERIADOS_BASE` | `BASE_HOLIDAYS` |
| `MES` | `MONTHS` |
| `DIA` | `WEEKDAYS` |
| `fecha` | `parse_date` |
| `texto` | `date_text` |
| `instante_ms` | `instant_ms` |
| `dia_semana` | `weekday_index` |
| `es_lunes` | `is_monday` |
| `redondeo_js` | `js_round` |
| `decimas` | `tenths` |
| `es_feriado` | `is_holiday` |
| `dias_laborables` | `workdays` |
| `feriados_semana` | `week_holidays` |
| `meta_semana` | `week_target` |
| `validar_fecha` | `validate_date` |

`local_date`, `date_only`, `monday_of`, `add_days`, `iso_week`, `fmt_d` y `fh` ya están en inglés.

**`hours`**

| Español | Inglés |
|---|---|
| `EstadoSemana` | `WeekState` (sus campos ya están en inglés) |
| `estado_horas` | `hours_state` |
| `dias` | `days_word` (devuelve «día» o «días») |

**`week`**

| Español | Inglés |
|---|---|
| `estadisticas_semana` | `week_stats` |
| `segundos_del_dia` | `day_secs` |
| `ultima_actividad` | `last_activity` |
| `tableros` | `boards` |
| `comparar_locale` | `compare_locale` |
| `primaria` | `primary_key` |
| `PUNTUACION` | `PUNCTUATION` |

**`board`**

| Español | Inglés |
|---|---|
| `SprintDato` | `SprintInfo` |
| `SprintDato.nombre_visible` | `display_name` |
| `SprintDato.board_id_visible` | `display_board_id` |
| `DatosTablero` | `BoardData` |
| `DatosTablero.tipo` | `issue_type` |
| `Resolucion` | `BoardResolution` |
| `NOMBRES_TABLERO` | `BOARD_NAMES` |
| `PROYECTOS_KANBAN` | `KANBAN_PROJECTS` |
| `COMPONENTES` | `COMPONENT_BOARDS` |
| `PROYECTOS` | `PROJECT_BOARDS` |
| `PROFUNDIDAD_MAX` | `MAX_DEPTH` |
| `re_tipos_kanban` | `kanban_types_re` |
| `re_limpiar_sprint` | `sprint_cleanup_re` |
| `buscar` | `find_board` |
| `ultimo_sprint` | `latest_sprint` |
| `resolver_tablero` | `resolve_board` |

**`issue`**

| Español | Inglés |
|---|---|
| `RegistroCalc` | `WorklogCalc` |
| `TareaDerivada` | `DerivedIssue` |
| `TareaDerivada.campos` | `fields` |
| `Calc` | `Calc` (sus campos ya están en inglés) |
| `DIA_MS` | `DAY_MS` |
| `re_sin_documento` | `no_doc_re` |
| `texto` | `text` |
| `texto_no_vacio` | `non_empty_text` |
| `entero` | `integer` |
| `datos_tablero` | `board_data` |
| `dias_de_atraso` | `days_late` |
| `derivar_registro` | `derive_worklog` |
| `derivar` | `derive` |

**`view`.** Contrato con la interfaz y con la paridad. Todas las estructuras de la vista llevan
`#[serde(rename_all = "camelCase")]`.

| Español | Inglés | Clave JSON nueva |
|---|---|---|
| `Entrada` | `ViewInput` | — |
| `.tareas` | `issues` | `issues` |
| `.semana` | `week` | `week` |
| `.tablero` | `board` | `board` |
| `.feriados_extra` | `extra_holidays` | `extraHolidays` |
| `.hoy` | `today` | `today` |
| `Vista` | `View` | — |
| `.hoy` | `today` | `today` |
| `.semana` | `week` | `week` |
| `.tablero` | `board` | `board` |
| `.tareas` | `issues` | `issues` |
| `.tableros` | `boards` | `boards` |
| `.stats` | `stats` | `stats` |
| `.horas` | `hours` | `hours` |
| `.semanas` | `weeks` | `weeks` |
| `.por_tablero` | `by_board` | `byBoard` |
| `EstadoHoras` | `HoursState` | — |
| `.estado` | `week_state` | (aplanado, sin clave) |
| `.prev_secs` | `prev_secs` | `prevSecs` |
| `.laborables` | `workdays` | `workdays` |
| `.dias` | `days` | `days` |
| `.feriados` | `holidays` | `holidays` |
| `DiaHoras` | `DayHours` | — |
| `.day` | `day` | `day` |
| `.secs` | `secs` | `secs` |
| `.feriado` | `holiday` | `holiday` |
| `SemanaGrafico` | `ChartWeek` | — |
| `.wk` | `wk` | `wk` |
| `.semana_iso` | `iso_week` | `isoWeek` |
| `.secs` | `secs` | `secs` |
| `.per_board` | `per_board` | `perBoard` |
| `.target` | `target` | `target` |
| `FilaTablero` | `BoardRow` | — |
| `.board`, `.secs`, `.worked`, `.resolved` | sin cambio | sin cambio |
| `.abiertas` | `open` | `open` |
| `.estados` | `statuses` | `statuses` |
| `.ultima` | `last` | `last` |
| `TODOS` | `ALL_BOARDS` | el valor sigue siendo `"Todos"` |
| `calcular_vista` | `compute_view` | — |

`HoursState` aplana `WeekState`, que ya tiene `today`, `current` y `target`. Las claves nuevas
(`workdays`, `days`, `holidays`, `prevSecs`) no chocan con ellas.

**Paridad.**

| Español | Inglés |
|---|---|
| `pruebas/paridad/` | `tests/parity/` (carpeta de datos; Cargo no la toma como prueba porque no tiene `main.rs`) |
| `casos/` | `cases/` |
| `volcados/` | `dumps/` |
| `volcados/viernes.json` | `dumps/friday.json` |
| `volcados/lunes.json` | `dumps/monday.json` |
| `generar.mjs` | `generate.mjs` |
| `legado.js` | `legado.js` (se mueve sin cambios) |
| `README.md` | `README.md` (documentación en español; se actualizan sus rutas) |
| `*.entrada.json` | `*.input.json` |
| `*.esperado.json` | `*.expected.json` |
| `tests/paridad.rs` | `tests/parity.rs` |
| `examples/volcar_simulado.rs` | `examples/dump_simulated.rs` |

Nombres de los casos: el prefijo `v-` pasa a `fri-` y `l-` a `mon-`. Los fragmentos se traducen así:

| Español | Inglés |
|---|---|
| `semana-actual` | `current-week` |
| `semana-anterior` | `previous-week` |
| `semana-futura` | `future-week` |
| `hace-dos-semanas` | `two-weeks-ago` |
| `semana-sin-datos` | `week-without-data` |
| `semana-actual-toda-feriado` | `current-week-all-holidays` |
| `semana-pasada-toda-feriado` | `last-week-all-holidays` |
| `feriado-quitado` | `holiday-removed` |
| `feriado-agregado` | `holiday-added` |
| `feriado-agregado-hoy` | `holiday-added-today` |
| `feriados-base` | `base-holidays` |
| `hoy-feriado` | `today-holiday` |
| `cambio-de-anio-N` | `year-change-N` |
| `tablero-inexistente` | `board-missing` |
| `tablero-todos-semana-anterior` | `board-all-previous-week` |
| `tablero-<slug>` | `board-<slug>` |

En `board-<slug>`, el slug sale del nombre del tablero, que no se traduce: `board-sin-tablero`,
`board-tablero-adn`.

Los 62 casos se regeneran con `node tests/parity/generate.mjs`. La entrada que se escribe pasa a ser
`{ issues, week, board, extraHolidays, today }`, y el adaptador de `generate.mjs` produce las claves
nuevas de `View`.

## 3. Tarea 5.11 — Jira

| Archivo | Nuevo |
|---|---|
| `jira/modelo.rs` | `jira/model.rs` |
| `jira/simulado.rs` | `jira/simulated.rs` |

`jira/mod.rs` y `jira/http.rs` conservan su nombre.

**`model`.** El JSON de `Issue` y `Sprint` ya está en inglés y no cambia.

| Español | Inglés | Clave JSON nueva |
|---|---|---|
| `Usuario` | `User` | — |
| `.account_id` | `account_id` | `accountId` |
| `.nombre` | `name` | `name` |
| `.correo` | `email` | `email` |
| `Registro` | `Worklog` (campos `d`, `s`, `a` sin cambio) | — |
| `Tarea` | `Issue` | — |
| `.tipo` | `issue_type` | `type` |
| `desde_jira` | `from_jira` | — |
| `Registro::es_de` | `Worklog::is_by` | — |
| `normalizar_fecha` | `normalize_date` | — |
| `texto` | `text` | — |
| `CAMPO_INICIO` | `START_FIELD` | — |
| `CAMPO_SPRINTS` | `SPRINTS_FIELD` | — |
| `CAMPO_PUNTOS` | `POINTS_FIELD` | — |
| `CAMPO_FIN_DESARROLLO` | `DEV_END_FIELD` | — |

**`jira` (`mod.rs`)**

| Español | Inglés | Clave JSON nueva |
|---|---|---|
| `PaginaBusqueda` | `SearchPage` | — |
| `.siguiente` | `next` | — |
| `PaginaWiki` | `WikiPage` | — |
| `.titulo` | `title` | — |
| `EnlaceRemoto` | `RemoteLink` | — |
| `.titulo` | `title` | — |
| `Fuente` | `Source` | — |
| `Fuente::Simulado` | `Source::Simulated` | — |
| `ResultadoHoras` | `WorklogsResult` | — |
| `.por_clave` | `by_key` | `byKey` |
| `.fallidas` | `failed` | `failed` |
| `.error` | `error` | `error` |
| `JQL_TAREAS` | `ISSUES_JQL` | — |
| `JQL_PR` | `PR_JQL` | — |
| `CAMPOS` | `FIELDS` | — |
| `validar_clave` | `validate_key` | — |
| `yo` | `myself` | — |
| `buscar` | `search` | — |
| `buscar_todo` | `search_all` | — |
| `claves_de` | `keys_of` | — |
| `cargar_tareas` | `load_issues` | — |
| `claves_con_horas` | `keys_with_hours` | — |
| `claves_con_pr` | `keys_with_pr` | — |
| `leer_horas` | `read_worklogs` | — |
| `cargar_para_resumen` | `load_for_summary` | — |
| `buscar_documentos` | `find_documents` | — |

Los métodos de `Source`, `JiraHttp` y `SimulatedJira` siguen las mismas traducciones:
`myself`, `search`, `worklogs`, `confluence` y `links` (este último es el antiguo `enlaces`).

**`http`**

| Español | Inglés |
|---|---|
| `JiraHttp.cred` | `credentials` |
| `TIEMPO_MAXIMO` | `TIMEOUT` |
| `ESPERA_MAXIMA_429` | `MAX_429_WAIT_SECS` |
| `nuevo` | `new` |
| `con_base` | `with_base` |
| `cliente` | `client` |
| `enviar` | `send` |
| `revisar` | `check_response` |
| `resumir_error` | `summarize_error` |

**`simulated`.** Los datos del simulado no cambian (sección 1).

| Español | Inglés |
|---|---|
| `JiraSimulado` | `SimulatedJira` |
| `Estado` | `Status` |
| `PorHacer` | `ToDo` |
| `EnCurso` | `InProgress` |
| `EnPruebas` | `InTesting` |
| `Finalizada` | `Done` |
| `Estimacion` | `Estimate` |
| `Ninguna` | `NoEstimate` |
| `Horas` | `Hours` |
| `Factor` | `Factor` |
| `Documento` | `DocKind` |
| `Ninguno` | `Missing` |
| `Confluence` | `Confluence` |
| `SoloEnlace` | `LinkOnly` |
| `DefSprint` | `SprintDef` |
| `Def` | `IssueDef` |
| `Reg` | `SimWorklog` |
| `TareaSim` | `SimIssue` |
| `hoy_ecuador` | `today_ecuador` |
| `FERIADOS_BASE` | `BASE_HOLIDAYS` |
| `nombre_y_categoria` | `name_and_category` |
| `definiciones` | `definitions` |
| `generar_registros` | `generate_worklogs` |
| `texto_duracion` | `duration_text` |
| `nombre_proyecto` | `project_name` |
| `fecha_hora` | `date_time` |
| `atender` | `serve` |
| `tiene_horas_propias_desde` | `has_own_hours_since` |
| `pagina_wiki` | `wiki_page` |
| `a_json` | `to_json` |
| `inicio_jira` | `jira_started` |
| `inicio_ms` | `started_ms` |

Los campos privados de estas estructuras se traducen según la sección 1.

## 4. Tarea 5.12 — Base de la app

**`error`.** Los mensajes visibles no cambian.

| Español | Inglés |
|---|---|
| `ErrorApp` | `AppError` |
| `Resultado<T>` | `AppResult<T>` |
| `codigo()` | `code()` |

El JSON del error pasa de `{ codigo, mensaje }` a `{ code, message }`.

| Variante | Nueva | Código nuevo |
|---|---|---|
| `SitioInvalido` | `InvalidSite` | `invalid_site` |
| `CorreoInvalido` | `InvalidEmail` | `invalid_email` |
| `TokenVacio` | `EmptyToken` | `empty_token` |
| `SinSesion` | `NoSession` | `no_session` |
| `NoAutorizado` | `Unauthorized` | `unauthorized` |
| `SinPermiso` | `Forbidden` | `forbidden` |
| `Limite` | `RateLimited` | `rate_limit` |
| `Red` | `Network` | `network` |
| `Jira { estado, detalle }` | `Jira { status, detail }` | `jira` |
| `Formato` | `Format` | `format` |
| `ClaveInvalida` | `InvalidKey` | `invalid_key` |
| `Almacen` | `SecretStore` | `secret_store` |
| `Config` | `Config` | `config` |
| `Cache` | `Cache` | `cache` |

**Decisión del autor:** `ui/app.js` deja de añadir « [código]» al final del mensaje. La persona solo
ve el mensaje en español. Las cuatro comparaciones de la interfaz pasan a `"unauthorized"`,
`"no_session"`, `"network"` y `"rate_limit"`.

**`almacen` → `secret_store`**

| Español | Inglés |
|---|---|
| `AlmacenSecretos` | `SecretStore` |
| `AlmacenSistema` | `SystemSecretStore` |
| `AlmacenMemoria` | `MemorySecretStore` |
| `AlmacenMemoria.datos` | `entries` |
| `guardar` | `save` |
| `leer` | `read` |
| `borrar` | `delete` |
| `servicio_para` | `service_for` (el valor `signal-aduana:<host>` no cambia) |

**`auth`**

| Español | Inglés |
|---|---|
| `Sitio` | `Site` |
| `como_texto` | `as_text` |
| `validar_correo` | `validate_email` |
| `Credenciales` | `Credentials` |
| `TokenClasico` | `ClassicToken` |
| `token_clasico` | `classic_token` |
| `Autenticador` | `Authenticator` |
| `url_base` | `base_url` |
| `firmar` | `sign` |

**`config`.** El archivo `config.toml` no cambia de nombre; la variante simulada pasa a
`config.simulated.toml`.

| Español | Inglés | Clave nueva |
|---|---|---|
| `Config.sitio` | `site` | `site` |
| `Config.correo` | `email` | `email` |
| `cargar` | `load` | — |
| `guardar` | `save` | — |
| `ruta` | `path` | — |

**`cache`.** El archivo `cache.json` no cambia de nombre; la variante simulada pasa a
`cache.simulated.json`.

| Español | Inglés | Clave nueva |
|---|---|---|
| `Instantanea` | `Snapshot` (`camelCase`) | — |
| `.guardado` | `saved_at` | `savedAt` |
| `.datos` | `data` | `data` |
| `Archivo` | `CacheFile` (*snake_case*, como hoy) | — |
| `.sitio` | `site` | `site` |
| `.guardado` | `saved_at` | `saved_at` |
| `.datos` | `data` | `data` |
| `.version` | sin cambio | `version` |
| `.account_id` | sin cambio | `account_id` |

Las funciones se traducen así: `ruta` → `path`, `guardar` → `save`, `leer` → `read`, `borrar` →
`delete`. El cuerpo opaco que escribe la interfaz cambia su clave `formato` por `format`; una caché
anterior, sin esa clave, se descarta y se vuelve a cargar.

**`sesion` → `session`.** El valor de `REJECTED_TOKEN` sigue siendo `"rechazar"`.

| Español | Inglés | Clave JSON nueva |
|---|---|---|
| `Sesion` | `Session` | — |
| `.fuente` | `source` | — |
| `.usuario` | `user` | — |
| `.sitio` | `site` | — |
| `.correo` | `email` | — |
| `Estado` | `AppState` | — |
| `.simulado` | `simulated` | — |
| `.dir_config` | `config_dir` | — |
| `.dir_datos` | `data_dir` | — |
| `.almacen` | `secrets` | — |
| `.sesion` | `session` | — |
| `InfoSesion` | `SessionInfo` | — |
| `.conectado` | `connected` | `connected` |
| `.simulado` | `simulated` | `simulated` |
| `.usuario` | `user` | `user` |
| `.sitio` | `site` | `site` |
| `.correo` | `email` | `email` |
| `.aviso` | `notice` | `notice` |
| `nuevo` | `new` | — |
| `actual` | `current` | — |
| `limpiar` | `clear` | — |
| `crear_fuente` | `create_source` | — |
| `info` | `info` | — |
| `info_guardada` | `saved_info` | — |
| `ruta_config` | `config_path` | — |
| `iniciar` | `sign_in` | — |
| `restaurar` | `restore` | — |
| `cerrar` | `sign_out` | — |
| `TOKEN_RECHAZADO` | `REJECTED_TOKEN` | — |

## 5. Tarea 5.13 — Escritorio y comandos

**`comandos` → `commands`.** Contrato con la interfaz. Los nombres de los parámetros son las claves
que envía `invoke`.

| Comando | Nuevo | Parámetros nuevos |
|---|---|---|
| `restaurar_sesion` | `restore_session` | — |
| `iniciar_sesion` | `sign_in` | `site`, `email`, `token` |
| `cerrar_sesion` | `sign_out` | — |
| `cargar_tareas` | `load_issues` | — |
| `claves_con_horas` | `keys_with_hours` | `since` |
| `claves_con_pr` | `keys_with_pr` | — |
| `leer_horas` | `read_worklogs` | `keys`, `since` |
| `buscar_documentos` | `find_documents` | `keys` |
| `leer_cache` | `read_cache` | — |
| `guardar_cache` | `save_cache` | `data` |
| `calcular_vista` | `compute_view` | `input` |
| `leer_preferencias` | `read_preferences` | — |
| `guardar_preferencias` | `save_preferences` | `preferences` |

Las funciones auxiliares también cambian: `con_sesion` → `with_session`, `leer_cache_de` →
`read_cache_for` y `guardar_cache_de` → `save_cache_for`.

**`preferencias` → `preferences`.** Este es el formato del archivo y también el JSON de la
interfaz.

| Español | Inglés | Clave nueva |
|---|---|---|
| `Preferencias` | `Preferences` | — |
| `.avisos` | `notifications` | `notifications` |
| `.iniciar_con_windows` | `start_with_windows` | `startWithWindows` |
| `.minimizar_a_bandeja` | `minimize_to_tray` | `minimizeToTray` |
| `.feriados_extra` | `extra_holidays` | `extraHolidays` |
| `PrefAvisos` | `NotificationPrefs` | — |
| `.activos` | `enabled` | `enabled` |
| `.hora_cierre` | `end_of_day_time` | `endOfDayTime` |
| `.resumen_lunes` | `weekly_summary` | `weeklySummary` |
| `HORA_CIERRE_DEFECTO` | `DEFAULT_END_OF_DAY_TIME` | — |
| `validar_hora` | `validate_time` | — |
| `normalizar` | `normalize` | — |
| `afecta_al_monitor` | `affects_monitor` | — |
| `cargar` | `load` | — |
| `guardar` | `save` | — |
| `ruta` | `path` | — |
| `preferencias.toml` | `preferences.toml` | — |
| `preferencias.simulado.toml` | `preferences.simulated.toml` | — |

**`avisos` → `notifications`**

| Español | Inglés | Clave nueva |
|---|---|---|
| `TipoAviso` | `NotificationKind` | — |
| `CierreJornada` | `EndOfDay` | — |
| `ResumenLunes` | `WeeklySummary` | — |
| `Reconectar` | `Reconnect` | — |
| `Aviso` | `Notification` | — |
| `.tipo` | `kind` | — |
| `.titulo` | `title` | — |
| `.cuerpo` | `body` | — |
| `Registro` | `NotificationLog` | — |
| `.cierre` | `end_of_day` | `end_of_day` |
| `.lunes` | `weekly_summary` | `weekly_summary` |
| `.reconectar` | `reconnect` | `reconnect` |
| `EstadoMonitor` | `MonitorState` | — |
| `Resumen` | `Summary` | — |
| `NoAutorizado` | `Unauthorized` | — |
| `SinSesion` | `NoSession` | — |
| `Error` | `Error` | — |
| `texto_fecha` | `date_text` | — |
| `ya_enviado` | `already_sent` | — |
| `decidir` | `decide` | — |
| `marcar` | `mark_sent` | — |
| `ruta` | `path` | — |
| `cargar_registro` | `load_log` | — |
| `guardar_registro` | `save_log` | — |
| `avisos.json` | `notifications.json` | — |
| `avisos.simulado.json` | `notifications.simulated.json` | — |

**`resumen` → `summary`.** Su JSON no cruza a la interfaz.

| Español | Inglés |
|---|---|
| `Resumen` | `HoursSummary` |
| `.hoy` | `today` |
| `.semana_secs` | `week_secs` |
| `.semana_meta_h` | `week_target_h` |
| `.hoy_secs` | `today_secs` |
| `.hoy_laborable` | `today_is_workday` |
| `.anterior_secs` | `previous_week_secs` |
| `.anterior_meta_h` | `previous_week_target_h` |
| `.primer_laborable` | `first_workday` |
| `MAX_TEXTO_BANDEJA` | `MAX_TRAY_TEXT` |
| `vista_de` | `view_for` |
| `resumir` | `summarize` |
| `texto_bandeja` | `tray_text` |
| `horas_sin_unidad` | `hours_without_unit` |

**`monitor`**

| Español | Inglés |
|---|---|
| `Salida` | `Output` |
| `tooltip` | `tooltip` |
| `notificar` | `notify` |
| `Memoria` | `Memory` |
| `.ultimo` | `last` |
| `ResultadoCiclo` | `CycleResult` |
| `Datos` | `Data` |
| `SinSesion` | `NoSession` |
| `NoAutorizado` | `Unauthorized` |
| `Fallo` | `Failed` |
| `ahora_ecuador` | `now_ecuador` |
| `recortar` | `truncate` |
| `con_marca_simulado` | `with_simulated_mark` |
| `texto_sin_sesion` | `no_session_text` |
| `texto_error_interno` | `internal_error_text` |
| `leer_resumen` | `read_summary` |
| `ciclo` | `cycle` |

**`vigilancia` → `watcher`**

| Español | Inglés |
|---|---|
| `Motivo` | `Reason` |
| `Pedido` | `Requested` |
| `Tiempo` | `Timeout` |
| `Vigilante` | `Watcher` |
| `.aviso` | `signal` |
| `SalidaTauri` | `TauriOutput` |
| `ESPERA_INICIAL` | `INITIAL_DELAY` |
| `INTERVALO` | `INTERVAL` |
| `MAX_MENSAJE_PANICO` | `MAX_PANIC_MESSAGE` |
| `esperar` | `wait` |
| `nuevo` | `new` |
| `pedir` | `request` |
| `pedir_ciclo` | `request_cycle` |
| `iniciar` | `start` |
| `mensaje_de_panico` | `panic_message` |
| `ciclo_protegido` | `guarded_cycle` |
| `bucle` | `run_loop` |

**`escritorio` → `desktop`.** Los textos del menú no cambian.

| Español | Inglés | Valor nuevo |
|---|---|---|
| `ID_BANDEJA` | `TRAY_ID` | `"tray"` |
| `VENTANA` | `MAIN_WINDOW` | `"main"` (sin cambio) |
| `ARG_OCULTO` | `HIDDEN_ARG` | `"--hidden"` (decisión del autor) |
| `EVENTO_ACTUALIZAR` | `REFRESH_EVENT` | `"signal://refresh"` |
| `es_arranque_oculto` | `is_hidden_start` | — |
| `instancia_unica` | `single_instance` | — |
| `mostrar_ventana` | `show_window` | — |
| `fijar_tooltip` | `set_tooltip` | — |
| `debe_mostrar_al_arrancar` | `should_show_on_start` | — |
| `debe_esconder_al_cerrar` | `should_hide_on_close` | — |
| `crear_bandeja` | `create_tray` | — |
| `configurar` | `setup` | — |
| id de menú `"abrir"` | — | `"open"` |
| id de menú `"actualizar"` | — | `"refresh"` |
| id de menú `"salir"` | — | `"quit"` |

**`inicio` → `autostart`**

| Español | Inglés | Valor nuevo |
|---|---|---|
| `Accion` | `Action` | — |
| `SoloLog` | `LogOnly` | — |
| `Habilitar` | `Enable` | — |
| `Deshabilitar` | `Disable` | — |
| `Nada` | `Nothing` | — |
| `EVENTO_PREFERENCIAS` | `PREFERENCES_EVENT` | `"signal://preferences"` |
| `decidir` | `decide` | — |
| `sincronizar` | `sync` | — |
| `corregir` | `reconcile` | — |
| `aplicar` | `apply` | — |

**`lib`:** `modo_simulado` → `simulated_mode`. El título visible de la ventana no cambia.

**Documentos:** las rutas y los nombres de `docs/COMPILAR-WINDOWS.md` (archivos en disco,
`--hidden`) se actualizan en la misma tarea.

## 6. Tarea 5.14 — Interfaz

Identificadores de `ui/app.js` que todavía queden en español después de las tareas 5.10 a 5.13:

| Español | Inglés |
|---|---|
| `FERIADOS_BASE` | `BASE_HOLIDAYS` |
| `PREFS_DEFECTO` | `DEFAULT_PREFS` |
| `AVISO_INICIO` | `AUTOSTART_NOTICE` |
| `feriadosExtra` | `extraHolidays` |
| `isFeriado` | `isHoliday` |
| `aplicarPrefs` | `applyPrefs` |
| `guardarPrefs` | `savePrefs` |
| `toggleFeriado` | `toggleHoliday` |
| `migrarFeriados` | `migrateHolidays` |
| `cargarPrefs` | `loadPrefs` |
| `MES` | `MONTHS` |
| `DIA` | `WEEKDAYS` |
| `vista` | `view` |
| `vistaError` | `viewError` |
| `hoyAct` | `currentToday` |
| `setVista` | `setView` |
| `pedirVista` | `requestView` |
| `prefsAvisosDependientes` | `notificationPrefInputs` |
| `abrirPrefs` | `openPrefs` |
| `errorPrefs` | `showPrefsError` |
| `escucharEventos` | `listenEvents` |
| `en` | `on` |

Las variables locales siguen la sección 1: `sitio` → `site`, `correo` → `email`, `nuevo` → `next`,
`viejos` → `legacy` y así con las demás.

**`ui/index.html`, con su uso en `app.js` y `styles.css`**

| Español | Inglés |
|---|---|
| `prefAvisos` | `prefNotifications` |
| `prefHora` | `prefEndOfDay` |
| `prefLunes` | `prefWeekly` |
| `prefInicio` | `prefAutostart` |
| `prefBandeja` | `prefTray` |
| `connAviso` | `connNotice` |
| `data-fer` | `data-holiday` (`dataset.holiday`) |
| `name="sitio"` | `name="site"` |
| `name="correo"` | `name="email"` |

Los demás `id` y clases ya están en inglés. `lang="es"` se queda.

## 7. Colisiones resueltas

| Nombre en español | Se traduce a |
|---|---|
| `Estado` (sesión) | `AppState` |
| `Estado` (simulado) | `Status` |
| `EstadoMonitor` | `MonitorState` |
| `EstadoSemana` | `WeekState` |
| `EstadoHoras` | `HoursState` |
| `Registro` (Jira) | `Worklog` |
| `Registro` (avisos) | `NotificationLog` |
| `RegistroCalc` | `WorklogCalc` |
| `Tarea` | `Issue` |
| `TareaDerivada` | `DerivedIssue` |
| módulo `tarea` | `issue` |
| `Resultado` | `AppResult` (no `Result`, para no tapar el de `std`) |
| `ResultadoHoras` | `WorklogsResult` |
| `ResultadoCiclo` | `CycleResult` |
| `Resumen` | `HoursSummary` (módulo `summary`); el campo `summary` de las tareas no cambia |
| `Memoria` (monitor) | `Memory` |
| `AlmacenMemoria` | `MemorySecretStore` |
| `iniciar` (sesión) | `session::sign_in` |
| `iniciar` (vigilancia) | `watcher::start` |
| `iniciar_sesion` (comando) | `sign_in` |
| `decidir` | `notifications::decide`, `autostart::decide` |
| `ruta` | `path` en cada módulo |
