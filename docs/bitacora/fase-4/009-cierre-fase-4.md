# 009 — Cierre de la fase 4

**Fecha:** 2026-11-16
**Rama:** tarea/4.7-cierre-fase-4 → develop
**Fase:** 4
**Ejecutor:** orquestador (Claude Sonnet 5.5) para el plan, el índice y `CLAUDE.md`; el `Makefile` y `scripts/fusionar.sh` los hizo un subagente Sonnet 5.5, en una vuelta.

## 1. Solicitud

Al comprobar el estado de `develop` tras fusionar la 4.5 y la 4.6, el orquestador detectó marcas de
conflicto de git dentro del índice de la bitácora. El autor aprobó tres tareas: esta (4.7, el
arreglo y el cierre de la fase), la 4.8 (aviso en la interfaz) y la 4.9 (script que verifica la
bitácora). Después pidió sumar a esta misma tarea los atajos de `make` de sus otros proyectos
(`draco-hq` y `api-integracion-echo-financiero`), adaptados a este repo.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El rebase de la 4.6 quedó a medias y se subió con `<<<<<<< HEAD`, `=======` y `>>>>>>> 4507c40…` dentro de `docs/bitacora/README.md` | Las dos filas (007 y 008) estaban completas, pero esas tres líneas rompían la tabla del índice en `develop` |
| `docs/PLAN-FASE-4.md` seguía diciendo que la tarea 3 era una sola | El plan no reflejaba el reparto real (4.3 a 4.6) ni lo que falta verificar |
| Las comprobaciones de la bitácora se habían hecho a mano, rama por rama | Nadie miraba el resultado fusionado: por eso el defecto pasó |
| El ciclo de una tarea (crear worktree, commit único, push, PR, fusión, limpieza) se hacía con comandos sueltos | Pasos olvidados o mal ordenados, ramas y worktrees fusionados sin retirar, y el rebase que dejó marcas de conflicto |

## 3. Diseño técnico

- **Arreglo del índice**: se quitaron solo las tres líneas de marcas y se conservaron las filas 007 y
  008 en orden; se añadió la fila 009. Ninguna entrada ya fusionada se tocó.
- **`docs/PLAN-FASE-4.md`**: la tabla original de tareas se **conserva** (es el plan y su historia);
  la fila de la tarea 3 dice que se hizo como 4.3 a 4.6, y se añadió la sección «Estado al cierre»:
  tareas hechas con su entrada de bitácora, qué se verificó (solo en WSL y en modo simulado), qué
  **no** se ha podido verificar (todo lo de Windows) y los pendientes abiertos. Las filas de la 4.8 y la
  4.9 están marcadas «en curso al escribir este cierre»: el índice de la bitácora es la fuente de verdad.
- **Alternativa descartada**: reescribir las entradas 007 y 008 o el plan original para que parezca que
  el reparto fue éste desde el principio. Se prefirió dejar el plan como estaba y anotar el cierre.
- **Lo que evita que se repita** es la tarea 4.9 (`scripts/verificar-bitacora.sh`), que comprueba también
  las marcas de conflicto y que verifica el resultado fusionado, no solo cada rama.
- **`Makefile` con los atajos del flujo**, tomando de `draco-hq` los atajos de git (`sync`, `push`,
  `pr`, `merge` con merge commit y pregunta, `ship`, `prune`) y la ayuda por secciones, y de
  `api-integracion-echo-financiero` `check` como definición de «terminado», `promote` y las opciones
  de la shell (`-eu -o pipefail`). **Adaptado a worktrees**: los atajos de `draco-hq` hacen `git switch`
  y fallan dentro de un worktree (`ship` terminaba cambiando a `develop`, abierta en la carpeta
  principal), y `fetch origin develop:develop` se niega a actualizar una rama abierta en otra carpeta.
  Aquí ningún atajo cambia de rama la carpeta desde la que se corre; `sync` actualiza `develop` donde
  esté abierta y solo con *fast-forward*; `finish` se corre desde la carpeta principal y `ship` no lo
  incluye porque no se puede quitar un worktree desde dentro.
- **Atajos**: `tarea`, `commit` (un solo commit por tarea, mensaje `Tarea: N.M <tema>` sacado de la
  rama; enmienda mientras no esté subido y exige `force=1` si ya lo está), `push`, `pr`, `merge`, `ship`,
  `update` (rebase sobre `origin/develop` y luego la verificación de la bitácora), `finish`, `prune`,
  `status`, `worktrees`, `sync`, `promote` (crea el PR `develop → main` tras confirmar; nunca fusiona),
  y de calidad `build`, `test`, `lint` (`clippy -D warnings`), `ui-check`, `run`, `bitacora` y `check`.
- **Salvaguardas**: ningún atajo usa `git switch`, `checkout`, `reset --hard`, `rm -rf` ni `--no-verify`;
  `--force-with-lease` solo con `force=1`; `finish` y `prune` retiran solo lo fusionado en
  `origin/develop` (con merge commit) y limpio, sin `--force`, y no borran ramas remotas (imprimen el
  comando); `run` lleva el modo simulado fijo y no se puede forzar el real desde `make`.
  `scripts/fusionar.sh` fusiona siempre con merge commit y no borra la rama.
- **`bitacora` no depende de la 4.9**: si `scripts/verificar-bitacora.sh` no existe, avisa y sigue; si
  existe, `commit` y `update` se detienen cuando falla.
- **Descartado**: `fmt` (el código no está formateado con `rustfmt` y reordenaría los `mod` de
  `lib.rs`) y `release` (la versión no está decidida).
- **Orden de fusión**: esta rama va primero, porque la 4.9 falla mientras el índice tenga marcas. Las
  filas 009, 010 y 011 caen en la misma línea del índice; las ramas 4.8 y 4.9 se rebasan sobre ésta.

## 4. Archivos

- `docs/bitacora/README.md` — sin marcas de conflicto y con la fila 009.
- `docs/PLAN-FASE-4.md` — fila de la tarea 3 y sección «Estado al cierre».
- `Makefile` (nuevo, 253 líneas) — los atajos de git y de calidad.
- `scripts/fusionar.sh` (nuevo, 44 líneas) — fusiona el PR de la rama con merge commit, tras validar y preguntar.
- `CLAUDE.md` — sección «Atajos de make».
- `docs/bitacora/fase-4/009-cierre-fase-4.md` — esta entrada.

## 5. Cómo verificar

Corridos por el orquestador (comprobaciones de lectura, sin código que compilar):

```bash
grep -rn -E '^(<<<<<<<|=======|>>>>>>>)' docs/bitacora        # sin resultados
# cada enlace del índice existe, la fase de cada entrada coincide con su carpeta y tiene sus secciones
```

No hay código de la app que compilar ni probar en esta tarea. Cuando se fusione la 4.9, `bash scripts/verificar-bitacora.sh`
hace estas mismas comprobaciones y debe salir con 0.

**Atajos de `make`.** Corridos por el ejecutor en un sandbox (remoto local `--bare`, clon como carpeta
principal y un `gh` falso primero en el `PATH`; el `gh` real nunca se llamó): 12 casos que pasaron, con
sus variantes de error: `help`, `tarea`, `sync` (desde la carpeta principal, desde un worktree y con
`develop` desviada), `commit` (mensaje derivado, `title=`, enmienda, ya subido con y sin `force=1`, dos
commits, sin cambios), `push`, `pr`, `merge` (con `n`, con `yes=1`, sin PR, PR hacia `main`), `ship`,
`finish` (desde un worktree, sin fusionar, con cambios), `prune`, `update` (limpio, con conflicto, con y
sin el script de la bitácora) y `promote` (sin diferencias y con PR, sin llamar nunca a `pr merge`).
Corridos por el orquestador en el repo real, de solo lectura: `bash -n scripts/fusionar.sh`, búsqueda
de comandos prohibidos (solo aparece `--force-with-lease`), `make help`, `make worktrees` y
`make bitacora`; los worktrees y las ramas no cambiaron.

**No probado**: `build`, `test`, `lint`, `ui-check` y `run` (largos; solo `make -n`); la sintaxis de
`--jq` y de `gh pr create --fill --web` con el `gh` real (el falso solo comprueba que los argumentos
lleguen); rutas con espacios. `make -n tarea` y `make -n finish` **no son simulacros del todo**: la línea
que llama a `make sync` se ejecuta aunque sea `-n` y hace `git fetch` y el avance *fast-forward*.

## 6. Pendientes

- Probar los atajos con el `gh` real, empezando por `make pr` y `make merge` en una tarea de poca importancia.
- Con tareas en paralelo, `make commit` se detendrá si el script de la 4.9 ve huecos de numeración (cada
  rama carece de las entradas de las otras): el remedio es fusionar en orden y `make update`.
- `make update` sobre una rama ya subida exige después `make push force=1` (el atajo lo avisa).
- Los pendientes abiertos de la fase están listados en `docs/PLAN-FASE-4.md`, en «Estado al cierre».
- Si las filas 4.8 y 4.9 ya están fusionadas al leer esto, la tabla del plan las muestra como «en
  curso» por haberse escrito antes; vale lo que diga el índice de la bitácora.
