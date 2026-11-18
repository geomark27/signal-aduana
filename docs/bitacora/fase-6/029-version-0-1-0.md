# 029 — Versión 0.1.0

**Fecha:** 2026-11-18
**Rama:** tarea/6.9-version-0-1-0 → develop
**Fase:** 6
**Ejecutor:** orquestador (Opus 5.5), una vuelta. Con el «sí» del autor corrió `make bump ver=0.1.0`
y escribió las correcciones y los documentos. Sin subagentes.

## 1. Solicitud

Con la 6.8 fusionada, el autor eligió sacar el primer release, la v0.1.0, antes de probar en Windows.
Así el instalador lo compila GitHub Actions y no hace falta preparar Rust ni Build Tools en el
equipo.

Al crear la tarea, `make tarea n=6.9 tema=version-0.1.0` falló: `tema` no admite puntos. El ejemplo
venía de la 6.8, de `CLAUDE.md` y de un mensaje de `bump.sh`. El autor pidió corregirlo en esta misma
tarea.

## 2. Motivación

| Problema | Daño real |
|---|---|
| No había ningún release ni tag | Para la 6.H y la 6.1 el autor tenía que compilar en su equipo |
| El ejemplo `tema=version-X.Y.Z` de la 6.8 no pasa la validación de `make tarea` | El primer uso del flujo de release falló en el primer paso |

## 3. Diseño técnico

- `make bump ver=0.1.0` mantiene el número, porque el tag `v0.1.0` no existe. Solo cerró
  `[Unreleased]` en `## [0.1.0] - 2026-11-18` y agregó los enlaces del final del CHANGELOG
  (`[Unreleased]`, que compara contra `v0.1.0`, y `[0.1.0]`, que apunta a su release). Antes se
  trajeron los tags del remoto, y no había ninguno.
- El ejemplo pasa a `tema=version-X-Y-Z`, con guiones, en `CLAUDE.md` y en el mensaje de `bump.sh`.
  Se descartó aceptar puntos en `make tarea`: el tema forma parte del nombre de la rama y de la
  entrada de bitácora, que hoy solo llevan minúsculas, números y guiones.
- La 6.9 queda para la versión, así que las correcciones de la fase pasan a ser la 6.6 y de la 6.10
  en adelante. La entrada 028, ya fusionada, dice «6.9 en adelante»; manda el plan.

## 4. Archivos

- `CHANGELOG.md`: `[Unreleased]` → `## [0.1.0] - 2026-11-18`, con sus enlaces.
- `CLAUDE.md`: el ejemplo de «Publicar una versión» usa `tema=version-0-1-1`.
- `scripts/bump.sh`: el mensaje de error sugiere `tema=version-X-Y-Z`.
- `docs/PLAN-FASE-6.md`: fila 6.9; las correcciones pasan a 6.6 y 6.10 en adelante.
- `docs/bitacora/README.md` (fila) y esta entrada.

## 5. Cómo verificar

Corrido por el orquestador en esta rama:

```bash
git fetch --tags origin          # sin tags en el remoto
make bump ver=0.1.0              # «Versión 0.1.0 (sin cambio de número…)»; solo cambió CHANGELOG.md
bash -n scripts/bump.sh
bash scripts/verificar-bitacora.sh
```

`make check` lo corre el autor. Tras fusionar, `make promote` y `DRY_RUN=1 make release` comprueban
que la versión y el CHANGELOG de `main` estén listos.

## 6. Pendientes

- Fusionar esta tarea, luego `make promote` y fusionar el PR `develop → main` (con merge commit), y
  después `make release`.
- Es la primera ejecución del workflow y la primera compilación en Windows. Hay que confirmar:
  - el nombre del NSIS que encuentra el workflow;
  - cuánto tarda;
  - que el release queda publicado con `Signal-Aduana-windows-x64-setup.exe`.
- Probar el instalador del release en la 6.H y en la 6.1 a la 6.5.
