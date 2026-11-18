# 033 — Actualizador automático

**Fecha:** 2026-11-18
**Rama:** tarea/6.13-actualizador → develop
**Fase:** 6
**Ejecutor:** tres subagentes Sonnet 5.5 en paralelo, una vuelta cada uno, con el contrato fijado
de antemano:
- 6.13a, Rust: plugin, comandos y chequeo de fondo;
- 6.13b, firma y manifiesto: workflow, scripts y `Makefile`;
- 6.13c, interfaz.

El orquestador (Opus 5.5) escribió la tarea y los documentos, revisó los tres trabajos y corrigió dos
cosas:
- una guarda en la interfaz, para que un aviso no reactive el botón durante la descarga;
- el espaciado de una línea del workflow.

## 1. Solicitud

El autor preguntó si la app se actualiza sola. No lo hacía: el actualizador estaba previsto para la
fase 7 (7.3). Se le ofrecieron dos opciones:
- **A**, el actualizador de Tauri, como Nudge;
- **B**, un aviso de versión nueva con descarga manual.

El autor eligió la **A** y la adelantó.

La tarea nació como `tarea/6.12-actualizador`, pero el 6.12 ya lo había usado el autor para la
versión 0.2.0 (entrada 032), así que pasó a ser la 6.13.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Cada versión nueva había que descargarla e instalarla a mano | Quien no se entera sigue con una versión vieja, con errores ya corregidos |
| Sin firma de actualización, una app que se actualiza sola podría instalar algo que no publicó el autor | Riesgo de seguridad: por eso el actualizador de Tauri exige una llave |

## 3. Diseño técnico

- **Rust (`src/updater.rs`), con `tauri-plugin-updater` 2.14.1:**
  - `check_update` devuelve `{ current, simulated, update: null | { version, notes } }`.
  - `install_update` descarga, verifica la firma e instala. Durante la descarga emite
    `signal://update-progress` con `[descargados, total | null]` y al terminar reinicia la app; en
    Windows el instalador `passive` la cierra.
  - Errores: `AppError::Update`, con el código `update`.
  - **Chequeo de fondo:** el primero a los 60 s del arranque y después cada 6 h. Emite
    `signal://update-available` y manda un solo aviso de Windows por versión mientras la app está
    abierta.
  - **En modo simulado** no hay red ni instalación: es la regla de no tener efectos en el sistema
    durante el desarrollo.
- **Configuración:**
  - `plugins.updater` en `tauri.conf.json`, con el endpoint
    `…/releases/latest/download/latest.json`, el modo `passive` y la `pubkey` todavía en
    `PENDIENTE: ejecuta make updater-key`;
  - `tauri.release.conf.json` activa `createUpdaterArtifacts`.
  - No se agregaron permisos a `capabilities/`: el plugin solo se usa desde Rust.
- **Firma y manifiesto (como Nudge, solo Windows):**
  - `scripts/updater-key.sh` (`make updater-key`) crea la llave una sola vez, sin contraseña, con la
    CLI de Tauri por `npx`. La guarda en `~/.tauri/signal-aduana.key`, la sube como secreto
    `TAURI_SIGNING_PRIVATE_KEY` y escribe la pública en `tauri.conf.json`.
  - Solo corre en una rama `tarea/*`, porque el cambio de `tauri.conf.json` va por PR, y nunca
    imprime la llave privada.
  - El workflow compila con `--config tauri.release.conf.json` y la llave, y pasa la firma `.sig` al
    job `publish`. Ese job arma `latest.json` con `scripts/updater-manifest.sh` (claves
    `windows-x86_64-nsis` y `windows-x86_64`) y publica el release.
- **Defensas:**
  - `prepare` en el workflow falla si la `pubkey` sigue en «PENDIENTE»;
  - `scripts/release.sh` se niega si la `pubkey` de `main` sigue en «PENDIENTE» o si falta el
    secreto;
  - `make release` rechaza `bump=` y `ver=` (lo que pasó en la 032).
- **Interfaz:**
  - **Barra «Signal Aduana X está disponible».** Va fuera de `#main`, así que también se ve en la
    pantalla de conexión. Muestra «Qué trae» y los botones «Actualizar ahora» (con el progreso en %
    o en MB) y «Más tarde».
  - **En Preferencias, la sección «Actualizaciones».** Muestra «Versión instalada» y el botón
    «Buscar actualizaciones».
  - Abrir Preferencias no consulta la red: la versión se muestra después de la primera búsqueda.

## 4. Archivos

- **Rust y configuración:**
  - `Cargo.toml` y `Cargo.lock`;
  - `src/updater.rs` (nuevo), `src/lib.rs` y `src/error.rs`;
  - `tauri.conf.json` y `tauri.release.conf.json` (nuevo).
- **Release:**
  - `.github/workflows/release.yml`;
  - `scripts/updater-key.sh` y `scripts/updater-manifest.sh` (nuevos);
  - `scripts/release.sh` y `Makefile` (`updater-key`, y `release` sin `bump=` ni `ver=`).
- **Interfaz:** `ui/index.html`, `ui/app.js` y `ui/styles.css`.
- **Documentación:**
  - `CHANGELOG.md`: una línea en «Añadido»;
  - `CLAUDE.md`: el actualizador, la llave y revisar el número de tarea;
  - `docs/PLAN-FASE-6.md`: 6.12, 6.13 y 6.14;
  - `docs/ROADMAP.md`: la 7.3 adelantada;
  - la entrada retroactiva 032, esta entrada y sus filas en el índice.

## 5. Cómo verificar

Corrido por los ejecutores:

- **Rust:**
  - `cargo build` sin errores;
  - `cargo test`: 173 pruebas de la librería y 1 de paridad, en verde (5 nuevas en `updater`: avisar
    una vez por versión, la forma del JSON en camelCase con `null`, el código y el mensaje del error,
    y los tiempos);
  - `cargo clippy --all-targets` sin advertencias;
  - los dos JSON de configuración son válidos.
- **Scripts:**
  - `bash -n` de los tres scripts y YAML válido;
  - `make release bump=minor` y `make release ver=1.0.0` fallan con el mensaje nuevo, sin red;
  - `latest.json` se generó en seco con `UPDATER_MANIFEST_OUT` y un `jq` simulado (`jq` no está
    instalado en WSL).
- **Interfaz:**
  - `node --check` y Prettier sin errores;
  - `check_update` e `install_update` se invocan sin argumentos;
  - los dos eventos, los 9 `id` nuevos presentes en `index.html` y sin manejadores en línea.

Corrido por el orquestador, tras sus correcciones:
- `node --check`, `make fmt-ui` y `make ui-format`;
- YAML válido;
- sin CRLF;
- `git diff --summary` sin cambios de modo;
- `bash scripts/verificar-bitacora.sh`.

`make check` lo corre el autor.

## 6. Pendientes

- **No se probó de extremo a extremo.** Falta la llave, que crea solo el autor en la 6.14
  (`make updater-key`, y respaldarla), un release firmado y la actualización en Windows.
- **El filtro `jq` real** de `updater-manifest.sh` se ejecuta por primera vez en el runner de Actions.
- **Las versiones 0.1.0 y 0.2.0** instaladas se actualizan a mano una última vez, a la 0.3.0.
- **Tecnología** debería saber que la app instalada consulta GitHub para buscar versiones nuevas.
- **La firma de código de Windows** (SmartScreen) sigue en la fase 7 (7.1).
