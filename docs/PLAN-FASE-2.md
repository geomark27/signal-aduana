# Signal Aduana de escritorio — Plan de la fase 2

**Punto de partida:** la fase 1 está completa (ver `docs/PLAN-FASE-1.md`): la app compila, pasan 45
pruebas y el tablero funciona en modo simulado, probado a mano por el autor. El cliente real de Jira y
Confluence ya está escrito (`src/jira/http.rs`) y probado contra un servidor local.

**Objetivo de la fase 2:** que la app sirva para el uso diario sin depender de la red en cada apertura.

1. **Caché local**: al abrir, la app muestra de inmediato los últimos datos guardados y actualiza en segundo plano. Sin red, sigue mostrando lo guardado.
2. **Actualización automática** cada 15 minutos.
3. **Fuentes dentro de la app**: sin depender de Google Fonts (funciona sin internet y la CSP queda sin dominios externos).

Fuera de alcance: la prueba con el Jira real (la hace el autor con su propia cuenta), el instalador de Windows, pasar cálculos de JS a Rust, bandeja y notificaciones.

## Reglas para todos los agentes

Las mismas 8 reglas de `docs/PLAN-FASE-1.md` (léelas allí). En resumen: solo tus archivos; **ninguna conexión a Jira, Confluence ni servicios de la empresa** (todo en modo simulado; nunca `SIGNAL_SIMULADO=0`); nada de tokens en logs o archivos; sin commits; `export PATH="$HOME/.cargo/bin:$PATH"`; todo en español; reporta desviaciones en vez de inventar; respuesta final breve con archivos, verificaciones reales y pendientes.

Tres agentes trabajan **al mismo tiempo**. El reparto de archivos es estricto:

| Tarea | Puede tocar |
|---|---|
| 1 | `src/cache.rs` (nuevo), `src/sesion.rs`, `src/comandos.rs`, `src/lib.rs` |
| 2 | `ui/app.js` únicamente |
| 3 | `ui/fonts/` (nuevo), `ui/styles.css`, `ui/index.html`, `tauri.conf.json` |

---

## Contrato C — caché (frontera Rust ↔ JS)

| Comando | Argumentos (JS) | Devuelve |
|---|---|---|
| `leer_cache` | — | `{ guardado: string, datos: any }` o `null` |
| `guardar_cache` | `{ datos: any }` | `null` |

- `datos` es **opaco para Rust** (`serde_json::Value`): la interfaz decide qué guarda.
- `guardado` es la fecha-hora RFC 3339 (UTC) en que Rust escribió el archivo.
- Ambos requieren sesión (`sin_sesion` si no hay). `leer_cache` devuelve `null` si no hay archivo, si está dañado, si es de otra versión o si pertenece a otra cuenta o sitio. Nunca falla por un archivo dañado.
- `cerrar_sesion` borra la caché de esa cuenta.
- Nuevo código de error: `cache` (no se pudo escribir, o `datos` supera el tamaño máximo).

## Tarea 1 — Caché local (Rust)

- `src/cache.rs`, lógica sin Tauri y con pruebas (`tempfile`):
  - Archivo `cache.json` (`cache.simulado.json` en modo simulado) en el directorio de datos locales de la app. Contenido: `{ version: 1, account_id, sitio, guardado, datos }`.
  - `guardar(dir, simulado, account_id, sitio, datos) -> Resultado<()>`: escritura atómica (temporal + renombrar, como `config.rs`); rechaza con `ErrorApp::Cache` si el JSON serializado supera 20 MB.
  - `leer(dir, simulado, account_id, sitio) -> Option<Instantanea>` con las reglas del contrato C.
  - `borrar(dir, simulado)`: que no exista no es error.
- `ErrorApp::Cache(String)` con código `cache` — **excepción al reparto:** puedes agregar esa variante en `src/error.rs`.
- `Estado` gana `dir_datos: PathBuf` (`app.path().app_local_data_dir()` en `lib.rs`); ajusta `Estado::nuevo` y las pruebas existentes.
- Comandos `leer_cache` y `guardar_cache` en `comandos.rs`, registrados en `lib.rs`. Usan `account_id` y sitio de la sesión activa.
- `sesion::cerrar` borra la caché.
- En el archivo de caché nunca va el token ni el correo.

**Verificar:** `cargo test` (hoy 45 pruebas; agrega las tuyas: ida y vuelta, otra cuenta → `None`, archivo dañado → `None`, versión distinta → `None`, límite de tamaño, cerrar sesión borra) y `cargo clippy --all-targets` sin advertencias; `cargo build`.

## Tarea 2 — Interfaz con caché y actualización automática (`ui/app.js`)

Lee la capa de datos actual de `ui/app.js` (`enterApp`, `refresh`, `fetchWorklogs`, `fetchDocs`, `handleAuth`, contador `gen`).

1. **Al entrar** (`enterApp`): llama `leer_cache`. Si hay datos, píntalos de inmediato (restaura `issues`, `docs`, `prMode`, y marca horas y documentos como ya cargados) con el texto de estado «Datos guardados · <fecha y hora>» y lanza `refresh` en segundo plano. Si no hay, flujo actual.
2. **Guardar**: cuando una carga completa termina bien (tareas + horas + documentos, sin tareas fallidas), llama `guardar_cache({ datos })` con lo necesario para repintar: `issues` (con sus horas ya unidas), `docs`, `prMode`, `syncedAt` y un número de `formato` propio (si al leer el `formato` no coincide, ignora la caché). Un fallo al guardar no interrumpe nada: solo `console.warn`.
3. **Sin red**: si `refresh` falla con `red` o `limite` y hay datos en pantalla (de caché o de una carga anterior), consérvalos y muestra el aviso «Sin conexión con Jira. Mostrando los datos guardados del <fecha y hora>.» con el punto de estado en error. Los errores `no_autorizado`/`sin_sesion` siguen llevando a la pantalla de conexión.
4. **Actualización automática**: cada 15 minutos, si hay sesión, la ventana está visible (`document.visibilityState`) y no hay una carga en curso. También al volver a estar visible si la última carga correcta tiene más de 15 minutos. Detén el temporizador al cerrar sesión. Durante una actualización automática **no** vacíes ni parpadees el tablero: los datos anteriores se quedan hasta que lleguen los nuevos (revisa que `refresh` no resetee `docs`/`docState`/horas a «cargando» cuando ya hay datos en pantalla).
5. Los datos de caché son de la misma cuenta (Rust lo garantiza); aun así, al cerrar sesión limpia el estado en memoria como hoy.
6. Actualiza el texto del pie (`renderTrace`) con una frase sobre la caché: los últimos datos se guardan en este equipo y se borran al cerrar sesión.

**Verificar:** `node --check ui/app.js`. No puedes probar en vivo (la tarea 1 corre en paralelo): repasa a mano los flujos — primera vez sin caché; apertura con caché; caché con `formato` viejo; sin red con y sin datos en pantalla; cerrar sesión durante una carga; actualización automática con la ventana oculta — e informa cualquier duda.

## Tarea 3 — Fuentes dentro de la app

1. Descarga desde Google Fonts (servicio público; es la única conexión externa permitida en esta tarea) los `.woff2` de: **Bricolage Grotesque** 600 y 700; **IBM Plex Sans** 400, 500 y 600; **IBM Plex Mono** 400 y 500. Subconjuntos `latin` y `latin-ext` (el español los necesita). Guárdalos en `ui/fonts/` con nombres claros. Las tres familias tienen licencia SIL OFL 1.1: agrega `ui/fonts/LICENCIAS.md` con familia, autor, licencia y enlace.
2. `ui/styles.css`: declara los `@font-face` al inicio (con `font-display: swap` y los `unicode-range` de cada subconjunto), apuntando a `fonts/…`. Mantén los nombres de familia que ya usan las variables `--display`, `--body`, `--mono`.
3. `ui/index.html`: quita los tres `<link>` de Google Fonts.
4. `tauri.conf.json`: deja la CSP sin dominios externos: `style-src 'self' 'unsafe-inline'`; `font-src 'self'`. No cambies nada más de ese archivo.

**Verificar:** los `.woff2` son válidos (`file`, tamaño razonable, cabecera `wOF2`); ningún archivo de `ui/` ni `tauri.conf.json` menciona `googleapis` o `gstatic`; cada `url()` de los `@font-face` existe en disco. No compiles (otro agente está compilando): el orquestador compila y abre la app al final.

## Después (orquestador)
Compilar todo junto, pruebas, abrir la app en modo simulado y entregar al autor los pasos para la prueba con su Jira real.
