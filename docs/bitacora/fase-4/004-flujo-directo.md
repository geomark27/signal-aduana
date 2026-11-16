# 004 — Flujo directo `develop → tarea`, sin rama de fase

**Fecha:** 2026-11-16
**Rama:** tarea/4.0-flujo-directo → develop
**Fase:** 4
**Ejecutor:** orquestador (Claude Sonnet 5.5), sin delegar: es un ajuste de normas y documentación, en una vuelta.

## 1. Solicitud

Con las normas de la 003 ya fusionadas, el autor pidió simplificar el flujo de ramas: «es mucho
enredo develop → fase4 → tarea; mejor ajustemos develop → tarea/x.x». Aprobó crear la rama
`tarea/4.0-flujo-directo` y la limpieza local de las ramas y worktrees ya fusionados.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Cada tarea exigía dos PR (tarea → fase y luego fase → `develop`); la propia 003 necesitó los PR #4 y #5 | Doble fusión, doble actualización de ramas y más ocasiones de que las ramas hijas queden desfasadas de su padre |
| La rama de fase repetía lo que ya dicen `docs/PLAN-FASE-N.md` y `docs/bitacora/fase-N/` | Una estructura más que mantener sin aportar información nueva |

## 3. Diseño técnico

- **Flujo nuevo**: `main ── develop ── tarea/N.M-tema`. Cada tarea nace de `develop` al día, vive en
  su worktree `.worktrees/N.M` y vuelve a `develop` por PR con merge commit. `develop → main`, al
  publicar una versión, sigue por PR con merge commit.
- **Las fases dejan de ser ramas** y quedan como agrupación documental: `docs/PLAN-FASE-N.md` y
  `docs/bitacora/fase-N/`. El `N` de `N.M` es la fase.
- **Descartado**: mantener `fase/N-tema` como rama padre (lo que se venía haciendo); el autor la
  descartó por el enredo descrito arriba.
- **`delegar`**: `rama_base` pasa a `develop` (ya no cambia con cada fase) y el PR de `fusionar` se
  crea con `--base develop`. Las ramas de cada ola de delegación nacen de `develop`.
- **Entradas ya fusionadas**: 001 y 002 conservan `→ fase/4-bandeja-avisos`, que fue cierto cuando se
  hicieron; la 003 recibe una nota con fecha al final.
- **Limpieza local hecha** (con `git branch -d`, que se niega a borrar una rama sin fusionar):
  worktrees `.worktrees/4.0`, `4.1` y `4.2` retirados (estaban limpios; solo tenían `gen/` y
  `target/`, que git ignora) y ramas locales `tarea/4.0-normas`, `tarea/4.1-preferencias-avisos`,
  `tarea/4.2-interfaz-preferencias` y `fase/4-bandeja-avisos` borradas. Esta tarea reutiliza la ruta
  `.worktrees/4.0` para su propia rama.
- **Riesgo aceptado**: sin rama de fase, cada tarea llega a `develop` por separado y una fase a medias
  puede quedar ahí. Aquí no pesa: 4.1 y 4.2 ya están en `develop` y la 4.3 se apoya en ellas.

## 4. Archivos

- `CLAUDE.md` — sección «Flujo de ramas» reescrita (sin rama de fase; limpieza tras fusionar).
- `docs/bitacora/README.md` — descripción y plantilla con `→ develop`; fila 004 en el índice.
- `.claude/skills/bitacora/SKILL.md` — `develop` en lugar de la rama de fase en los comandos y el encabezado.
- `.claude/delegar.json` — `rama_base: develop` y PR con `--base develop`.
- `docs/bitacora/fase-4/003-normas.md` — nota aclaratoria con fecha al final.
- `docs/bitacora/fase-4/004-flujo-directo.md` — esta entrada.

## 5. Cómo verificar

Corridos:

```bash
python3 -m json.tool .claude/delegar.json > /dev/null     # JSON válido
git grep -n 'fase/' -- CLAUDE.md .claude                   # sin rama de fase en normas ni configuración
git branch -vv && git worktree list                        # ramas y worktrees tras la limpieza
```

No hay código que compilar ni probar en esta tarea.

## 6. Pendientes

- El autor borra las ramas remotas ya fusionadas: `fase/4-bandeja-avisos` y las `tarea/4.x`
  (`git push origin --delete <rama>`). No se hizo push.
- `.claude/delegar.json` se fusionó con la 003; se entiende aprobado, pero conviene que el autor lo
  confirme antes de la primera ola delegada.
- La tarea 3 de `docs/PLAN-FASE-4.md` será `tarea/4.3-…`, nacida de `develop`.
