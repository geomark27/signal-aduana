# 028 — Flujo de release

**Fecha:** 2026-11-18
**Rama:** tarea/6.8-release → develop
**Fase:** 6
**Ejecutor:** un subagente Sonnet 5.5, en una vuelta, escribió los scripts, el workflow y el
`Makefile`. El orquestador (Opus 5.5) escribió `CHANGELOG.md` y los documentos, revisó el trabajo y
corrigió una línea de `release.sh`.

## 1. Solicitud

El autor pidió ajustar el `Makefile` para publicar versiones «como lo hacemos con Draco o Nudge». El
modelo es el de Nudge:
- `make release`;
- `scripts/release.sh`;
- un workflow de GitHub Actions que compila el instalador al subir el tag.

Es trabajo de la fase 7, que se adelanta: una parte de la 7.2 y de la 7.7.

**Decisiones del autor (2026-11-18):**
1. El repositorio **sigue público**. El instalador que se adjunta a cada release se puede descargar
   sin restricción.
2. Se **respeta el flujo por PR**: `main` solo recibe PR desde `develop`, sin commits directos.
3. **Sin actualizador** por ahora. Queda en la fase 7, igual que la firma de código.

## 2. Motivación

| Problema | Daño real |
|---|---|
| No había forma de publicar una versión | Cada instalador se compilaba a mano en un equipo Windows, sin número de versión ni notas |
| La versión estaba repetida en tres archivos (`Cargo.toml`, `Cargo.lock`, `tauri.conf.json`) | Al subirla a mano, era fácil dejar uno atrás |
| No había notas de versión para quien usa la app | La bitácora es técnica y no sirve como notas de un release |

## 3. Diseño técnico

- **Diferencia con Nudge.** En Nudge, `make release` hace en `main` el commit «release: vX», pone el
  tag y sube los dos. Aquí eso rompería la regla de que `main` solo recibe PR, y además obligaría a
  cambiar de rama en la carpeta principal, que tiene `develop` abierta. Por eso el proceso se partió
  en dos:
  - **`make bump`** corre dentro de una tarea. Cambia la versión en los tres archivos y pasa
    `[Unreleased]` del CHANGELOG a `## [X.Y.Z] - fecha`, con los enlaces de comparación. No hace
    commit: el cambio viaja como cualquier tarea (`make check · commit · ship`) y llega a `main` con
    `make promote`.
  - **`make release`** solo pone el tag. Se corre desde cualquier carpeta y no cambia de rama. Antes
    de crear nada comprueba:
    1. que `origin/main` tiene todo lo de `origin/develop`;
    2. que la versión de `origin/main` coincide en `Cargo.toml` y en `tauri.conf.json`;
    3. que el tag está libre, en local y en el remoto, y es mayor que el anterior;
    4. que el CHANGELOG de `main` tiene la sección de esa versión.

    Después pregunta, pone el tag `vX.Y.Z` sobre `origin/main`, lo sube y crea el release en borrador
    con las notas.
- **Primer release.** `bump` acepta la versión actual mientras su tag no exista, así que el primero
  puede salir como v0.1.0 sin subir el número.
- **Workflow (`.github/workflows/release.yml`)**, solo `windows-latest`:
  - comprueba que el tag coincide con `tauri.conf.json`;
  - compila con la CLI de Tauri desde `npx`, que trae binarios precompilados;
  - adjunta el NSIS con nombre fijo, `Signal-Aduana-windows-x64-setup.exe`, para que el enlace a la
    última versión no cambie;
  - publica el release.

  Respecto de Nudge se quitaron Linux, macOS, la llave del actualizador, las firmas `.sig` y
  `latest.json`.
- **Corrección del orquestador.** El job `prepare` del workflow también crea el borrador si no
  existe, y puede adelantarse a `release.sh`, que habría fallado con el tag ya subido. Ahora
  `release.sh` solo crea el release si no existe.
- **Último tag.** `bump` toma el último tag por orden de versión y no con `git describe`: los tags
  están en commits de fusión de `main`, que no son ancestros de la rama de la tarea.
- **CHANGELOG.** Sigue Keep a Changelog, como en Nudge. `[Unreleased]` ya resume lo que trae la 0.1.0,
  y cada tarea con cambios visibles agrega su línea ahí. La bitácora sigue siendo el registro técnico.

## 4. Archivos

- `scripts/bump.sh`, `scripts/release.sh` y `scripts/changelog-notes.sh` (nuevos). `changelog-notes.sh`
  acepta un archivo o `-` para leer de la entrada estándar.
- `.github/workflows/release.yml` (nuevo).
- `Makefile`: sección «Versiones» con `version`, `notes`, `bump` y `release`.
- `CHANGELOG.md` (nuevo).
- `CLAUDE.md`: sección «Publicar una versión».
- `docs/PLAN-FASE-6.md`: fila 6.8. Las correcciones pasan a numerarse 6.6 y 6.9 en adelante, y el
  cierre toma el primer número libre.
- `docs/ROADMAP.md`: nota en la fase 7 sobre lo adelantado.
- `docs/bitacora/README.md` (fila) y esta entrada.

## 5. Cómo verificar

Corrido por el ejecutor, sin red y sin escribir en el repositorio:

- `bash -n` de los tres scripts y el YAML del workflow cargado con Python: sin errores.
- `make help` muestra la sección «Versiones». `make version` imprime `versión: 0.1.0` y
  `último tag: ninguno`. `make notes` imprime `[Unreleased]`.
- `changelog-notes.sh` con un CHANGELOG ficticio: sección `Unreleased`, una versión, una sección vacía
  (sale con 1), una inexistente (sale con 1) y la lectura por `-`.
- `bump.sh`, en clones de `/tmp` con commits de prueba:
  - `DRY_RUN=1` no cambia nada;
  - `ver=0.1.0` solo cambia el CHANGELOG;
  - `bump=minor` deja 0.2.0 en los tres archivos (en `Cargo.lock`, solo la línea de `signal-aduana`)
    y el CHANGELOG con sus enlaces;
  - 8 casos de error con su mensaje: una rama que no es de tarea, `bump=` y `ver=` juntos, una
    versión menor, un `bump` inválido, sin versión, un tag que ya existe, `[Unreleased]` vacío y
    cambios sin commit.
- `release.sh`: solo `bash -n`. Se probó en un clon con un remoto ficticio y falló limpio en
  `git fetch` porque GitHub rechazó la conexión SSH. **Esa prueba intentó una conexión de red**, en
  contra de la tarea; no llegó a ningún repositorio real.

Lo corrió el orquestador después de su corrección: `bash -n scripts/release.sh`, sin CRLF en los
archivos de la rama, `make version`, `make notes` y `bash scripts/verificar-bitacora.sh`.

No corrido: `make check` (lo corre el autor), `release.sh` contra el remoto real y el workflow en
GitHub Actions.

## 6. Pendientes

- El primer release prueba el workflow por primera vez, y será también la primera compilación de la
  app en Windows desde Actions. Para hacerlo: una tarea con `make bump ver=0.1.0`, luego
  `make promote` y luego `make release`. Si algo falla, el release queda en borrador.
- `bump` lee los tags locales. Si no se trajeron del remoto (`git fetch --tags`), los enlaces de
  comparación del CHANGELOG pueden apuntar a un tag anterior equivocado.
- `shellcheck` no está instalado: los scripts no pasaron por ese análisis.
- El instalador no tiene firma de código (7.1) ni hay actualizador (7.3).
- El repositorio es público: el instalador de una app interna queda descargable para cualquiera.
  Conviene que Tecnología lo sepa al decidir el canal de distribución (7.2).
