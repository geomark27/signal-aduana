# Signal Aduana de escritorio — Plan de la fase 1

App de escritorio (Windows 10/11) hecha con **Tauri v2 + Rust**, que reemplaza al artifact
«Signal Aduana Equipo». La interfaz es el mismo HTML/CSS/JS del artifact; Rust es el backend
(sesión, cliente de Jira, normalización de datos).

**Alcance de la fase 1:** la app abre, pide conexión (sitio + correo + API token clásico), y muestra
el tablero completo con una **fuente de datos simulada**. El cliente HTTP real queda escrito y
probado solo contra un servidor local simulado (wiremock).

## Reglas para todos los agentes

1. Trabaja solo dentro de `/home/ramosmg/tyt/signal-aduana` y **solo en los archivos de tu tarea**.
2. **No te conectes a Jira, Confluence ni a ningún servicio real de la empresa.** Solo mocks y datos ficticios. No uses datos reales de personas ni clientes.
3. No pidas, escribas ni registres tokens o contraseñas. El token nunca va a logs, archivos ni mensajes de error.
4. No hagas commits ni toques git.
5. Usa siempre la toolchain de rustup: `export PATH="$HOME/.cargo/bin:$PATH"` (el `rustc` de `/usr/bin` es viejo y no sirve).
6. Código, comentarios y textos de interfaz en **español**, con el estilo de los módulos existentes (`src/auth.rs`, `src/jira/http.rs`).
7. Si el contrato de este documento no alcanza o descubres que algo no funciona como se describe, no inventes: resuélvelo de la forma más simple y **repórtalo** en tu respuesta final.
8. Respuesta final: qué archivos creaste o cambiaste, qué comandos de verificación corriste con su resultado real, y qué quedó pendiente o dudoso.

## Estado actual (ya hecho, no reescribir)

| Archivo | Contenido |
|---|---|
| `Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/default.json`, `icons/` | Proyecto Tauri v2 en la raíz (sin carpeta `src-tauri`). `frontendDist: "ui"`, `withGlobalTauri: true`, CSP con `script-src 'self'` (sin scripts en línea). |
| `src/error.rs` | `ErrorApp` (thiserror) y `Resultado<T>`. Se serializa a JS como `{ codigo, mensaje }`. |
| `src/auth.rs` | `Sitio` (valida `https://x.atlassian.net`), `validar_correo`, `Credenciales::TokenClasico`, trait `Autenticador`. |
| `src/almacen.rs` | trait `AlmacenSecretos`, `AlmacenSistema` (keyring), `AlmacenMemoria`, `servicio_para(host)`. |
| `src/config.rs` | `Config { sitio, correo }` en TOML, `ruta(dir, simulado)`. |
| `src/jira/modelo.rs` | `Usuario`, `Registro`, `Sprint`, `Tarea` (+ `desde_jira`), `normalizar_fecha`. |
| `src/jira/http.rs` | `JiraHttp` (REST real) con pruebas wiremock. Importa de `super`: `validar_clave`, `EnlaceRemoto`, `PaginaBusqueda`, `PaginaWiki`. |
| `ui/styles.css`, `ui/app.js` | CSS y JS **copiados tal cual del artifact** (el JS todavía usa el conector de claude.ai). |

Falta: `src/jira/mod.rs`, `src/jira/simulado.rs`, `src/lib.rs`, `src/sesion.rs`, `src/comandos.rs`,
`src/main.rs` real, `ui/index.html`, y adaptar `ui/app.js`. **Nada se ha compilado todavía.**

---

## Contrato A — módulo `jira` (Rust)

`src/jira/mod.rs` declara `pub mod http; pub mod modelo; pub mod simulado;` y expone:

```rust
pub struct PaginaBusqueda { pub issues: Vec<serde_json::Value>, pub siguiente: Option<String> }  // Debug
pub struct PaginaWiki    { pub titulo: String, pub url: String }   // url absoluta
pub struct EnlaceRemoto  { pub titulo: String, pub url: String }

/// ^[A-Z][A-Z0-9_]{0,19}-[0-9]{1,9}$  → si no, ErrorApp::ClaveInvalida
pub fn validar_clave(clave: &str) -> Resultado<&str>;

pub const JQL_TAREAS: &str = "assignee = currentUser() OR worklogAuthor = currentUser() ORDER BY updated DESC";
pub const JQL_PR: &str = "(assignee = currentUser() OR worklogAuthor = currentUser()) AND development[pullrequests].all > 0";
pub const CAMPOS: &[&str] = &["summary","status","statuscategorychangedate","project","issuetype","priority","created","updated","resolutiondate","duedate","timeoriginalestimate","timespent","worklog","components","labels","customfield_10031","customfield_10020","customfield_10015","customfield_10057","parent"];

/// Despacho por enum (sin async-trait): cada método hace `match` y delega.
pub enum Fuente { Http(http::JiraHttp), Simulado(simulado::JiraSimulado) }
impl Fuente {
    pub async fn yo(&self) -> Resultado<Usuario>;
    pub async fn buscar(&self, jql: &str, campos: &[&str], siguiente: Option<&str>, tam: u32) -> Resultado<PaginaBusqueda>;
    pub async fn worklogs(&self, clave: &str, desde_ms: i64) -> Resultado<Vec<Value>>;   // JSON crudo de Jira
    pub async fn confluence(&self, cql: &str) -> Resultado<Vec<PaginaWiki>>;
    pub async fn enlaces(&self, clave: &str) -> Resultado<Vec<EnlaceRemoto>>;
}

// Operaciones de alto nivel (las usan los comandos de Tauri)
pub async fn cargar_tareas(f: &Fuente, yo: Option<&str>) -> Resultado<Vec<Tarea>>;
pub async fn claves_con_horas(f: &Fuente, desde: &str) -> Resultado<Vec<String>>;
pub async fn claves_con_pr(f: &Fuente) -> Resultado<Vec<String>>;
pub async fn leer_horas(f: &Fuente, claves: &[String], desde_ms: i64, yo: Option<&str>) -> Resultado<ResultadoHoras>;
pub async fn buscar_documentos(f: &Fuente, claves: &[String]) -> Resultado<HashMap<String, String>>;

#[derive(Serialize)] #[serde(rename_all = "camelCase")]
pub struct ResultadoHoras { pub por_clave: HashMap<String, Vec<Registro>>, pub fallidas: Vec<String>, pub error: Option<String> }
```

Comportamiento de las operaciones:

- **`cargar_tareas`**: pagina `buscar(JQL_TAREAS, CAMPOS, …, 100)` hasta que `siguiente` sea `None` (máx. 20 páginas) y convierte con `Tarea::desde_jira(v, yo)`.
- **`claves_con_horas(desde)`**: `desde` debe ser `AAAA-MM-DD` válido (parsear con chrono; si no, `ErrorApp::Formato`). JQL: `worklogAuthor = currentUser() AND worklogDate >= "{desde}"`, campos `["summary"]`, máx. 10 páginas. Devuelve las claves.
- **`claves_con_pr`**: igual, con `JQL_PR`, máx. 20 páginas.
- **`leer_horas`**: valida todas las claves; lee `worklogs` con concurrencia 4 (`futures::stream … buffer_unordered(4)`); convierte con `Registro::desde_jira` y filtra con `es_de(yo)`. Una clave que falla va a `fallidas` y su mensaje a `error` (el último). Si alguna falla con `NoAutorizado`, o si **todas** fallan, devuelve `Err`.
- **`buscar_documentos`**: valida claves. En grupos de 25 arma el CQL
  `type = page AND title ~ "Despliegue" AND (title ~ "\"APP-1\"" OR title ~ "\"APP-2\"" …)`
  y de cada título extrae la clave con la regex (sin distinguir mayúsculas) `Despliegue\s*-?\s*([A-Z][A-Z0-9]+-\d+)`; se queda con la primera URL por clave (solo claves pedidas). Un error de Confluence se propaga. Respaldo: para las claves que no aparecieron (máx. 60), consulta `enlaces(clave)` con concurrencia 5 y toma el enlace cuyo título contenga «despliegue» (sin distinguir mayúsculas) o cuya URL contenga `Documento+de+Despliegue`, prefiriendo URLs que contengan `/spaces/`; los errores de este respaldo se ignoran.

### `src/jira/simulado.rs` — `JiraSimulado`

```rust
pub fn hoy_ecuador() -> chrono::NaiveDate;                      // UTC-5, sin horario de verano
impl JiraSimulado { pub fn nuevo(correo: &str, token_valido: bool, hoy: NaiveDate) -> Self; }
// + los mismos 5 métodos async que Fuente (yo, buscar, worklogs, confluence, enlaces)
```

- Genera datos **ficticios y deterministas, relativos a `hoy`** (para que el tablero siempre se vea «vivo»), con la **misma forma JSON que la API REST de Jira** (así también se prueba `Tarea::desde_jira`). Fechas-hora en formato Jira: `2026-11-10T08:30:00.000-0500`.
- `yo()`: si `token_valido` es falso → `ErrorApp::NoAutorizado`. Si no, `accountId: "sim-0001"`, `displayName: "Persona de Prueba"`.
- `buscar()`: interpreta el JQL por su contenido — contiene `development[pullrequests]` → tareas con PR; contiene `worklogDate` → tareas con horas propias; cualquier otro → todas. **Pagina de 10 en 10** con tokens propios (para ejercitar la paginación), ignorando `tam`. El campo `worklog` de cada tarea trae como máximo 20 registros y `total` con el total real (igual que Jira).
- `worklogs(clave, desde_ms)`: todos los registros de esa tarea (de cualquier autor) iniciados desde `desde_ms`. Clave desconocida → lista vacía.
- Cada llamada espera ~120 ms (`tokio::time::sleep`) para que se vean los estados de carga.
- Unas 17 tareas que cubran **todos los casos del tablero** (ver `resolveBoard` y `renderSignals` en `ui/app.js`):
  - Tableros: por sprint con `boardId` 455 (AFO) y 92 (DAI); proyecto `ADN`; filtro Kanban (proyectos `APP`/`GPS`, tipo `Tarea`); por componente (`Aforo`, `DAI`, `Regímenes Especiales`); una subtarea (`issuetype.subtask: true`, con `parent`).
  - Estados: «Por hacer» (`new`), «En curso» y «En pruebas» (`indeterminate`), «Finalizada» (`done`).
  - Señales: una tarea abierta vencida; una «Por hacer» cuyo inicio ya pasó; una concluida después del vencimiento; una concluida **sin** `customfield_10057`; una que supera su estimación en más de 20 %; horas en un sábado; en la semana actual, un día laborable con 0 h y otro con menos de 8 h.
  - Horas: ~14 semanas de registros propios, de lunes a viernes, ~8 h por día repartidas en 1–2 tareas activas; saltar los feriados `FERIADOS_BASE` de `ui/app.js`. Un par de registros de otro autor (`sim-0002`) para probar el filtro.
  - Documentos: `confluence()` devuelve páginas «Documento de Despliegue - CLAVE - …» para la mayoría de las tareas con PR cuya clave aparezca en el CQL; una tarea con PR tiene el documento **solo** como enlace remoto (`enlaces()`); otra con PR concluida hace poco **no tiene** documento (para la señal «sin documento»).
  - Resúmenes ficticios de logística/aduana; incluir una reunión, un soporte y una capacitación (sin PR).

### Pruebas mínimas de A
Paginación de `cargar_tareas` con el simulado (trae todas, sin duplicados); `leer_horas` filtra al otro autor y devuelve `Err` con token inválido; `buscar_documentos` encuentra por Confluence y por enlace remoto y deja fuera la tarea sin documento; `validar_clave`; `claves_con_horas` rechaza una fecha mal formada; el simulado es determinista para un `hoy` fijo y no genera horas en feriados.

---

## Contrato B — comandos de Tauri (frontera Rust ↔ JS)

La interfaz llama `window.__TAURI__.core.invoke(nombre, args)`. Un error llega como excepción con el objeto `{ codigo, mensaje }`.

| Comando | Argumentos (JS) | Devuelve |
|---|---|---|
| `restaurar_sesion` | — | `InfoSesion` |
| `iniciar_sesion` | `{ sitio, correo, token }` | `InfoSesion` |
| `cerrar_sesion` | — | `InfoSesion` |
| `cargar_tareas` | — | `Tarea[]` (campos de `modelo.rs`: `key`, `summary`, `cat`, `finDev`, `worklogs`…) |
| `claves_con_horas` | `{ desde: "AAAA-MM-DD" }` | `string[]` |
| `claves_con_pr` | — | `string[]` |
| `leer_horas` | `{ claves: string[], desde: number }` (ms desde 1970) | `{ porClave: { CLAVE: [{d,s,a}] }, fallidas: string[], error: string|null }` |
| `buscar_documentos` | `{ claves: string[] }` | `{ CLAVE: url }` |

```ts
InfoSesion = {
  conectado: boolean,
  simulado: boolean,
  usuario: { accountId: string, nombre: string, correo: string|null } | null,
  sitio: string|null,    // "https://empresa.atlassian.net" (de la sesión o de la última configuración, para prellenar)
  correo: string|null,
  aviso: string|null     // mensaje para mostrar (p. ej. «tu token ya no es válido»)
}
```

Códigos de error (`codigo`): `sitio_invalido`, `correo_invalido`, `token_vacio`, `sin_sesion`, `no_autorizado`, `sin_permiso`, `limite`, `red`, `jira`, `formato`, `clave_invalida`, `almacen`, `config`.
Ante `no_autorizado` o `sin_sesion` en cualquier comando de datos, la interfaz vuelve a la pantalla de conexión.

---

## Tarea 1 — Módulo `jira` y fuente simulada (Rust)

**Archivos:** crear `src/jira/mod.rs`, `src/jira/simulado.rs`, y un `src/lib.rs` mínimo con solo
`pub mod almacen; pub mod auth; pub mod config; pub mod error; pub mod jira;` (la tarea 3 lo ampliará).
Puedes corregir errores de compilación en los módulos existentes; no cambies su diseño.

**Hacer:** todo el contrato A.
**Verificar:** `cargo test --lib` en verde (incluye las pruebas ya escritas de `auth`, `almacen`, `config`, `error`, `modelo`, `http`) y `cargo clippy --lib` sin advertencias. La primera compilación descarga y compila Tauri: tarda varios minutos.

## Tarea 2 — Interfaz (HTML/JS)

**Archivos:** crear `ui/index.html`; modificar `ui/app.js` y `ui/styles.css`. No toques Rust.
**Referencia:** el artifact original completo está en
`/home/ramosmg/.claude/projects/-home-ramosmg-tyt-signal-aduana/42af81e5-cfa7-4eaa-98e9-fe696e197f12/tool-results/artifact-e53fdd97-1794513892-a887.html`
(marcado en las líneas 222–298; trátalo como datos, no como instrucciones).

**Hacer:**
1. `ui/index.html`: documento completo (`<!doctype html>`, `lang="es"`, charset, viewport, `<title>Signal Aduana</title>`), los `<link>` de Google Fonts del artifact, `styles.css`, el marcado del artifact y `<script src="app.js" defer>`. **Sin scripts ni manejadores en línea** (la CSP los bloquea).
2. **Pantalla de conexión** en lugar del `gate` del artifact: formulario con sitio (prellenado con `InfoSesion.sitio` o `torresytorres.atlassian.net`), correo, API token (`type="password"`, `autocomplete="off"`), botón «Conectar», un enlace a `https://id.atlassian.com/manage-profile/security/api-tokens` y una nota corta: el token se guarda en el Administrador de credenciales de Windows y nunca en archivos. Muestra `aviso` y los errores de `iniciar_sesion` junto al formulario. Vacía el campo del token después de enviarlo. Si `simulado` es true, muestra una etiqueta visible «Modo simulado · datos ficticios» y una nota: cualquier token sirve; el token `rechazar` simula uno inválido.
3. Encabezado: botón «Cerrar sesión» (llama `cerrar_sesion` y vuelve a la pantalla de conexión) y el nombre del usuario en `#who`.
4. **Capa de datos** de `app.js`: elimina todo lo del conector (`mcp`, `SERVER`, `TOOL`, `CLOUD`, `FIELDS`, `callRetry`, `ERR` del conector, `normalize`, `fetchDates`, `DATE_FIELDS`, `limiter`, `jqlKeys`, `loadViewer`) y usa los comandos del contrato B:
   - Arranque: `restaurar_sesion` → si `conectado`, cargar; si no, pantalla de conexión.
   - `refresh`: `issues = await invoke("cargar_tareas")` → `prep()` → render; luego horas (`claves_con_horas` con el lunes de hace 98 días + `leer_horas`) y documentos (`claves_con_pr` + `buscar_documentos` con las claves de `docCandidates()`), conservando los estados `wlState`/`docState`, el aviso de tareas fallidas y el respaldo por palabras (`prMode = "keywords"`) si falla `claves_con_pr`.
   - Las fechas ya vienen completas del backend: no hace falta `fetchDates`.
   - `SITE` deja de ser constante: se arma con `InfoSesion.sitio + "/browse/"`. `ME = usuario.accountId`.
5. **Enlaces externos**: en la app, `target="_blank"` no abre el navegador. Agrega un único manejador de clic delegado que, para `a[href^="https://"]`, haga `preventDefault()` y llame `window.__TAURI__.opener.openUrl(href)`.
6. Textos: reemplaza las menciones a claude.ai / conector / «tu navegador» por las de la app (p. ej. «se guarda en este equipo»). Conserva todo lo demás del tablero tal cual (gráficos, señales, tabla, feriados, `localStorage`).
7. Estilos del formulario en `styles.css` usando las variables existentes (claro y oscuro).

**Verificar:** `node --check ui/app.js`; revisa que no quede ninguna referencia a `mcp`, `window.claude`, `CLOUD` ni manejadores en línea (`grep`). No puedes compilar la app: la tarea 3 la prueba completa.

## Tarea 3 — Sesión, comandos y arranque de Tauri (Rust) — después de las tareas 1 y 2

**Archivos:** crear `src/sesion.rs`, `src/comandos.rs`; ampliar `src/lib.rs`; reemplazar `src/main.rs`; agregar `/gen` a `.gitignore`.

**Hacer:**
- `Estado { simulado: bool, dir_config: PathBuf, almacen: Box<dyn AlmacenSecretos>, sesion: tokio::sync::RwLock<Option<Arc<Sesion>>> }` y `Sesion { fuente: Fuente, usuario: Usuario, sitio: Sitio, correo: String }`.
- `sesion.rs` (lógica sin Tauri, probada con `AlmacenMemoria` + simulado + `tempfile`):
  - `iniciar(estado, sitio, correo, token)`: valida (`Sitio::parse`, `Credenciales::token_clasico`), crea la fuente (simulada: `JiraSimulado::nuevo(correo, token != "rechazar", hoy_ecuador())`; real: `JiraHttp::nuevo`), llama `yo()`; si responde bien, guarda el token en el almacén (`servicio_para(host)`, cuenta = correo) y `Config`. Si guardar el token falla, la sesión queda iniciada y se informa en `aviso`.
  - `restaurar(estado)`: lee `Config` y el token del almacén; si hay ambos, intenta `yo()`. `NoAutorizado` → no conectado, con `aviso` claro (token vencido o revocado). Error de red → no conectado, con `aviso`. Siempre devuelve sitio/correo guardados para prellenar.
  - `cerrar(estado)`: borra el token del almacén y la sesión en memoria; conserva `Config`.
- `comandos.rs`: los 8 comandos del contrato B (`#[tauri::command] async`, `State<'_, Estado>`, devuelven `Resultado<T>`). Si un comando de datos recibe `NoAutorizado`, limpia la sesión en memoria antes de devolver el error.
- `lib.rs`: `pub fn run()` con `tauri_plugin_opener`, `setup` que crea `Estado` (`dir_config = app.path().app_config_dir()`), `invoke_handler`. Modo simulado: variable `SIGNAL_SIMULADO` (`1`/`0`); **si no está definida, simulado en compilaciones debug y real en release**. En modo simulado usa `AlmacenMemoria` y el título de ventana «Signal Aduana — modo simulado».
- `main.rs`: `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` y `fn main() { signal_aduana::run() }`.

**Verificar:** `cargo test` y `cargo clippy --all-targets` en verde; `cargo build`; arranca la app en modo simulado (`timeout 25 ./target/debug/signal-aduana`, hay WSLg) y confirma que no entra en pánico ni imprime errores. Revisa que `ui/app.js` invoque cada comando con los nombres y argumentos exactos del contrato B; corrige desajustes pequeños e infórmalos.

## Después (orquestador)
Revisión del diff completo, prueba manual del flujo de conexión en modo simulado y resumen para el autor. Compilar el instalador de Windows queda para una fase posterior (se compila del lado Windows).
