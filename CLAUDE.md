# CLAUDE.md

Guía para Claude Code al trabajar en este repositorio. Las normas de aquí mandan sobre
las skills genéricas (`delegar`, `devlog`) dentro de este repo.

## Qué es

**Signal Aduana de escritorio**: app de Windows 10/11 hecha con **Tauri v2 + Rust** que muestra
las horas, tareas y estados de Jira de la persona por semana y por tablero. La interfaz es
HTML/CSS/JS sin framework (`ui/`); Rust es el backend (sesión, cliente de Jira, cálculos,
preferencias, avisos). El estado y las decisiones de cada fase están en `docs/PLAN-FASE-N.md`:
léelo antes de empezar una tarea.

```
src/dominio/        Cálculos (semanas, horas, feriados, vista). La interfaz solo dibuja.
src/jira/           Cliente de Jira/Confluence: http real, simulado y operaciones de alto nivel
src/                auth, almacen (credenciales), config, preferencias, resumen, avisos,
                    sesion, comandos (comandos de Tauri), error, lib, main
ui/                 index.html, app.js, styles.css (CSP: sin scripts ni manejadores en línea)
examples/           volcar_simulado (volcado para la paridad)
pruebas/paridad/    Casos de paridad del dominio (entradas y salidas esperadas)
docs/               PLAN-FASE-N.md y bitácora (docs/bitacora/)
```

## Comandos

Usa siempre la toolchain de rustup: `export PATH="$HOME/.cargo/bin:$PATH"` (el `rustc` de
`/usr/bin` es viejo).

```bash
cargo test                      # pruebas
cargo clippy --all-targets      # sin advertencias
cargo build
node --check ui/app.js          # sintaxis de la interfaz
bash scripts/verificar-bitacora.sh   # bitácora coherente (solo lee; debe salir con 0)
```

No se compila ni se prueba el instalador desde WSL ni sobre `\\wsl$`: eso se hace en Windows
(ver `docs/COMPILAR-WINDOWS.md`). `SIGNAL_SIMULADO=1` fuerza el modo simulado;
**nunca** `SIGNAL_SIMULADO=0` desde una sesión de Claude.

## Reglas que no se negocian

- **Ningún servicio real de la empresa**: nada de Jira, Confluence ni credenciales reales. Solo
  el modo simulado, mocks (wiremock) y datos ficticios; nunca datos reales de personas ni clientes.
- **Sin tokens ni contraseñas** en logs, archivos, mensajes de error ni la bitácora. El token vive
  en el Administrador de credenciales de Windows.
- **Sin efectos en el sistema durante el desarrollo**: en modo simulado no se registra el inicio
  con el sistema operativo y los avisos llevan el prefijo «[Simulado]». No dejes procesos de la
  app corriendo (usa `timeout`).
- **Los cálculos viven en `src/dominio/`**; la interfaz no reimplementa reglas.
- Interfaz, mensajes, comentarios y documentación en **español**. Comentarios mínimos: el código se
  explica con nombres; la historia y el porqué van a la bitácora.
- Los criterios aduaneros, tributarios o legales que aparezcan en textos son referenciales y los
  valida el responsable competente.

## Flujo de ramas

```
main ── develop ── tarea/N.M-tema   (cada tarea en .worktrees/N.M)
```

- Cada **tarea** nace de `develop` al día, vive en su propio worktree `.worktrees/N.M` y vuelve a
  `develop` por PR con **merge commit** (nunca *squash* ni *rebase and merge*: los hashes cambian y
  la historia deja de compartirse).
- Las **fases no son ramas**: solo agrupan tareas en `docs/PLAN-FASE-N.md` y en
  `docs/bitacora/fase-N/`. El `N` de `N.M` es la fase.
- Una tarea termina en **un solo commit** (`Tarea: N.M <tema>`) que lleva su entrada de bitácora.
- Una tarea sin terminar se mantiene al día con `develop` (rebase o merge, con el «sí» del autor).
- `develop` → `main` por PR con merge commit, al publicar una versión.
- Tras fusionar una tarea se retiran su worktree y su rama local
  (`git worktree remove .worktrees/N.M` y `git branch -d tarea/N.M-tema`); la remota la borra el autor.
- **Claude no hace commit, push, PR, merge ni crea ramas sin el «sí» explícito del autor** para esa
  operación. Los agentes ejecutores tienen git bloqueado.

## Atajos de make (los corre el autor)

`make` sin argumentos lista todo. Los atajos del flujo de arriba, pensados para worktrees:

```bash
make tarea n=4.10 tema=algo   # crea .worktrees/4.10 y la rama tarea/4.10-algo desde develop al día
cd .worktrees/4.10            # se trabaja allí
make check                    # ui-check + bitácora + clippy + pruebas
make commit                   # un solo commit «Tarea: 4.10 algo» (enmienda si aún no se subió)
make ship                     # push + PR + fusión con merge commit (pregunta antes)
cd ../..                      # carpeta principal
make finish n=4.10            # develop al día; retira el worktree y la rama local ya fusionados
```

Además: `make update` (rebase sobre `origin/develop` y verificación de la bitácora), `make prune`
(retira todo lo ya fusionado), `make status`, `make sync` y `make promote` (PR `develop → main`; nunca
fusiona). Siguen mandando las reglas de arriba: Claude no ejecuta `commit`, `push`, `pr`, `merge`,
`ship`, `finish`, `update`, `prune` ni `promote` sin el «sí» del autor, y `check` y `run` (largos) los
corre el autor.

## Bitácora (obligatoria)

Una entrada por tarea en `docs/bitacora/fase-N/NNN-<tema>.md` (NNN global; el tema es el de la rama
sin el `N.M-`), más su fila en el índice `docs/bitacora/README.md`. Viaja en el mismo commit que el
cambio. Formato, reglas y la skill: `docs/bitacora/README.md` y `.claude/skills/bitacora/`.

- Hechos, no intenciones: lo que no se probó va en **Pendientes**.
- Las entradas de tareas ya fusionadas no se reescriben; solo admiten notas con fecha.
- Antes de entregar una rama, y de nuevo tras cada rebase o fusión que toque el índice, corre
  `bash scripts/verificar-bitacora.sh`: comprueba enlaces del índice, fase y secciones de cada
  entrada, numeración, marcas de conflicto de git y URLs o correos. Mira el resultado **fusionado**,
  no solo la rama: así se detectó un índice que llegó a `develop` con marcas de conflicto.
- Con varios agentes, **una rama = una entrada = un dueño**: el orquestador es el único que escribe
  la entrada. Cada ejecutor termina su reporte con un bloque `DEVLOG CONTRIBUTION` y no toca
  `docs/bitacora/`.

## Forma de trabajar: Claude orquesta, otros ejecutan

Se usa la skill `delegar` con la configuración de `.claude/delegar.json`.

- **Claude (modelo más avanzado) solo planifica**: parte el trabajo en tareas pequeñas, escribe
  cada tarea autocontenida (objetivo, archivos que posee, valores exactos, criterios comprobables)
  y revisa el resultado. No programa lo que puede delegar.
- **Ejecutores baratos**: subagentes Sonnet 5.5 (esfuerzo medio) u opencode con modelos
  `opencode/*-free`, según `~/.config/delegar/config.json`. Con modelos gratuitos, solo datos
  ficticios.
- **Antes de lanzar una ola**: Claude presenta tareas, archivos, modelo y nombres de rama y espera
  el «sí» del autor. Dos tareas de la misma ola nunca comparten archivo; lo que depende de otra
  va a una ola posterior.
- Las tablas compartidas (índice de bitácora, `docs/PLAN-FASE-N.md`, `CLAUDE.md`) las escribe solo
  Claude.
- **Revisión y check final solo con aprobación del autor**: al recibir el reporte, Claude lee el
  reporte y el diff y lo presenta; **espera la aprobación del autor** antes de hacer el check de
  cierre y de entregar la rama.
- **Comandos largos** (builds, `cargo test` completo, `make check`, levantar la app): Claude no
  los ejecuta; dice cuáles son y qué hace cada uno, el autor los corre y comparte el resultado.
- Los ejecutores no tocan git, no leen fuera de su carpeta y reportan desviaciones en vez de
  inventar. Respuesta final breve: archivos, verificaciones reales y pendientes.
