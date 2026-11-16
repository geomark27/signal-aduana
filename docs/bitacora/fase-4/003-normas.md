# 003 — Normas de trabajo y bitácora

**Fecha:** 2026-11-16
**Rama:** tarea/4.0-normas → fase/4-bandeja-avisos
**Fase:** 4
**Ejecutor:** orquestador (Claude Sonnet 5.5) para las normas y esta entrada; un subagente Sonnet 5.5 de solo lectura reunió los datos de la entrada 001, en una vuelta.

## 1. Solicitud

El autor pidió fijar dos normas antes de seguir con la fase 4:

1. Una bitácora estándar, como la de `draco-hq` y la skill `claude-devlog-skill`.
2. La forma de trabajar de `delegar`: el modelo más avanzado solo planifica y reparte; los
   ejecutores son modelos baratos (Sonnet 5.5 medio u opencode con modelos gratuitos); el check
   final solo se hace con su aprobación.

Aprobó el formato de `draco-hq` (`fase-N/NNN-tema.md`), la rama `tarea/4.0-normas` y escribir las
entradas retroactivas de las tareas 4.1 y 4.2.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Las tareas 4.1 y 4.2 se fusionaron sin registro de qué se decidió y por qué | La historia y el porqué vivían solo en la conversación |
| La 4.2 la hizo directamente el modelo avanzado | Se gasta el modelo caro en trabajo que un ejecutor barato puede hacer con un buen plan |
| El repo no tenía `CLAUDE.md` | Cada sesión nueva volvía a descubrir (o a olvidar) las reglas |

## 3. Diseño técnico

- **Formato de `draco-hq`** (secciones 1–6, índice con numeración global, carpetas por fase,
  entradas fusionadas inmutables, «hechos, no intenciones»). Se adaptó a este repo: la unidad es la
  **tarea** (`tarea/N.M-tema` → `fase/N-…`), y se agregó el campo **Ejecutor** (qué modelo hizo la
  tarea y cuántas vueltas) que pide `delegar`.
- **De `claude-devlog-skill` se tomó la regla multiagente**: una rama = una entrada = un dueño; el
  orquestador es el único que escribe y los ejecutores devuelven un bloque `DEVLOG CONTRIBUTION`.
- **Descartado**: la estructura `entries/` + `releases/` de `claude-devlog-skill` y sus scripts
  (`new-entry.sh`, `verify.sh`), porque este repo trabaja por fases y no tiene releases todavía.
  Los scripts de `draco-hq` (`check.sh`, Makefile) tampoco se copiaron: eran de Go.
- **Numeración**: las fases 1 a 3 no tienen entradas (anteriores a la bitácora). La 001 es la tarea
  4.1; la 4.0 (esta) lleva la 003 por haberse escrito después de las dos retroactivas.
- **`CLAUDE.md`** reúne reglas del repo ya existentes (las de `PLAN-FASE-1` y `PLAN-FASE-4`), el flujo
  de ramas con worktrees, la bitácora y la forma de trabajar. Dice expresamente que Claude no hace
  commit, push, PR, merge ni crea ramas sin el «sí» del autor, y que los comandos largos los corre
  el autor.
- **Conflicto resuelto a favor del autor**: `delegar` pide que el orquestador corra
  `verificar_cierre`, pero la norma del autor es que Claude no ejecuta comandos largos. Se entrega la
  lista de comandos y el autor los corre.
- **`.claude/delegar.json`** es una **propuesta**: queda sin usarse hasta que el autor la apruebe.
  Los comandos de verificación salen del plan de la fase; `fusionar` y `comandos_denegados` son
  supuestos que el autor debe revisar.

## 4. Archivos

- `CLAUDE.md` (nuevo) — qué es el proyecto, comandos, reglas, flujo de ramas, bitácora y forma de trabajar.
- `docs/bitacora/README.md` (nuevo) — índice, formato y reglas.
- `docs/bitacora/fase-4/001-preferencias-avisos.md`, `002-interfaz-preferencias.md` (nuevos, retroactivos) y esta entrada.
- `.claude/skills/bitacora/SKILL.md` (nuevo) — la skill de bitácora del repo.
- `.claude/delegar.json` (nuevo, **propuesta pendiente de aprobación**).

## 5. Cómo verificar

Corridos (ver el cierre de la tarea): validez del JSON de `.claude/delegar.json` y existencia de los
enlaces relativos del índice.

```bash
python3 -m json.tool .claude/delegar.json > /dev/null
git diff --stat fase/4-bandeja-avisos
```

No hay código que compilar ni probar en esta tarea.

## 6. Pendientes

- El autor debe aprobar o ajustar `.claude/delegar.json` antes de la primera ola delegada. Su
  `rama_base` cambia con cada fase.
- No existe un script que verifique la bitácora (equivalente al `check.sh` de `draco-hq`); la
  coherencia fase/carpeta, el índice y los enlaces se revisan a mano por ahora.
- Los datos de las entradas 001 y 002 son retroactivos: no consta qué verificaciones se corrieron
  en su momento.
