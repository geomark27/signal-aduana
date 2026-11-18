# 027 — Diagnóstico anónimo

**Fecha:** 2026-11-18
**Rama:** tarea/6.7-diagnostico-anonimo → develop
**Fase:** 6
**Ejecutor:** orquestador (Opus 5.5) planificó y revisó. Hubo dos subagentes Sonnet 5.5 en paralelo,
uno por tarea, cada uno en una vuelta: 6.7a en Rust y 6.7b en la interfaz. El orquestador corrigió
los finales de línea de tres archivos de Rust: las ediciones sobre `\\wsl.localhost` los habían dejado
en CRLF.

## 1. Solicitud

Con la 6.0 fusionada, el autor pidió seguir con el roadmap. Tocaba la ola B del plan, la tarea 6.7:
que la app arme un resumen de diagnóstico sin datos reales para pegarlo en los reportes de la ola A.
El autor aprobó la ola, la rama y los dos ejecutores.

## 2. Motivación

| Problema | Daño real |
|---|---|
| En `release` la app no tiene consola | Para las pruebas 6.1 a 6.5 el autor no tenía cómo ver tiempos, códigos de error ni la forma de los campos |
| Reconstruir a mano la forma de una respuesta | Lento, y un dato real (clave, título, correo, sitio) podía colarse en un reporte |

## 3. Diseño técnico

- **El diagnóstico se pide, no se registra en segundo plano.** El comando `build_diagnostics` (sin
  parámetros, devuelve un texto) repite con la sesión activa el recorrido de carga y mide cada paso.
  Son seis pasos: `myself`, `search_issues` (las mismas páginas que `load_issues`), `keys_with_hours`,
  `read_worklogs`, `keys_with_pr` y `find_documents`. Se descartó instrumentar el cliente HTTP para
  guardar tiempos y errores de cada carga: tocaba más archivos y guardaba estado que nadie pidió.
- **Ventana compartida.** `read_worklogs` usa la misma ventana que el resumen semanal: el lunes de
  hace 98 días. Ese cálculo salió de `load_for_summary` a `jira::summary_window`, sin cambiar lo que
  hace.
- **Sin valores por construcción.** El informe lleva solo esto:
  - versión, modo y sistema operativo;
  - pasos con `ms`, `ok`, conteos, `pages`, `failedKeys` y `skipped`;
  - totales de tareas y registros (`issuesWithHours`, `ownWorklogs`, `maxPerIssue`);
  - `prMode` (`pr` si la consulta de PR responde; si no, `keywords`, el mismo criterio que la
    interfaz);
  - por cada campo de `jira::FIELDS`, en ese orden: cuántas tareas lo traen, cuántas en null y en
    cuántas falta, más su **forma** (tipos y nombres de claves, hasta 4 niveles). Una clave que no
    parece un nombre de esquema sale como `[clave]`.
- **Anonimización de mensajes.** El único texto libre son los mensajes de error, y se limpian al
  final, cuando ya se conocen los títulos. Primero se cambian los valores conocidos (sitio y host,
  correo, nombre, accountId y títulos de las tareas cargadas) por `[sitio]`, `[correo]`, `[persona]`,
  `[cuenta]` y `[título]`. Después:
  - las URLs pasan a `[url]`;
  - los demás correos, a `[correo]`;
  - las claves de tarea, a `ISSUE-N`, con el mismo número en todo el informe;
  - el texto entre comillas, a `[texto]`, salvo nombres de campo (`customfield_N` o los de `FIELDS`)
    y marcadores;
  - todo se recorta a 300 caracteres.
- **Token rechazado.** Si Jira rechaza el token, el recorrido se corta y el comando devuelve
  `unauthorized`, igual que los demás: la sesión se cierra. Cualquier otro error queda en el paso y en
  `lastError`, y el recorrido sigue.
- **Interfaz.** En Preferencias hay una sección nueva, «Diagnóstico». El botón «Generar diagnóstico»
  muestra el texto tal cual, de solo lectura, para revisarlo antes de compartirlo. «Copiar» usa el
  portapapeles; si falla, selecciona el texto y pide Ctrl+C.
  - Sin sesión, no llama a Rust.
  - Con `unauthorized` o `no_session`, cierra el diálogo y reutiliza `handleAuth`, como las cargas.
- **Contrato revisado a mano.** El compilador no lo comprueba, así que se comparó: hay un solo
  `invoke("build_diagnostics")` sin argumentos y `commands::build_diagnostics` está registrado en
  `lib.rs`.
- **Límite conocido.** No se detecta el nombre de una persona ajena a la sesión si aparece sin
  comillas dentro del texto de un error de Jira. Por eso la interfaz pide revisar el texto antes de
  compartirlo.

## 4. Archivos

- `src/diagnostics.rs` (nuevo): recorrido cronometrado, estadística y forma de los campos,
  `anonymize`, `shape`, `to_text` y 7 pruebas.
- `src/commands.rs`: comando `build_diagnostics`, `build_diagnostics_for` y una prueba (sin sesión;
  con sesión simulada da JSON sin el sitio).
- `src/lib.rs`: `pub mod diagnostics` y el comando registrado.
- `src/jira/mod.rs`: `summary_window` extraída de `load_for_summary`.
- `ui/index.html`: sección «Diagnóstico» en el diálogo de preferencias.
- `ui/app.js`: reinicio de la sección en `openPrefs` y manejadores de `diagBtn` y `diagCopy`.
- `ui/styles.css`: estilo de `#diagText`; los botones de un `fieldset` no se estiran.
- `docs/bitacora/README.md` (fila) y esta entrada.

## 5. Cómo verificar

Corrido por los ejecutores en el worktree (WSL):

```bash
cargo test                    # lib: 168 en verde; parity: 1 en verde
cargo clippy --all-targets    # sin advertencias
node --check ui/app.js        # sale con 0
make ui-format                # «All matched files use Prettier code style!»
```

Las pruebas de privacidad:

- **Simulado.** Reúne todas las claves, títulos, URLs de documentos, sitio, correo, nombre y
  accountId de la fuente simulada, y comprueba que ninguno aparece en el informe.
- **`wiremock`, con datos ficticios.** El error 400 de la consulta de PR trae a propósito una clave,
  un correo, una URL y un título. La prueba comprueba que el informe no contiene ninguno, ni la
  dirección del servidor local, ni `http`, ni el nombre del sprint. También comprueba que sí aparecen
  `ISSUE-1`, `[correo]`, `[url]`, `customfield_10057`, `prMode: keywords` y `status: 400`.

Verificación de cierre del orquestador: pendiente de la aprobación del autor (ver Pendientes).

## 6. Pendientes

- Verificación de cierre en esta rama (`cargo test`, `cargo clippy --all-targets`, `cargo build`,
  `make check`): la corre el autor.
- No se probó en la app: generar y copiar en modo simulado, el aspecto del diálogo en tema claro y
  oscuro, y el portapapeles del WebView2 en Windows.
- No se probó con el Jira real: se verá al reportar la 6.1 a la 6.5.
- `read_worklogs` informa cuántas claves fallaron (`failedKeys`), pero no el mensaje de ese error
  parcial: si en la 6.5 aparece un 429 al leer registros, se agrega en una corrección.
