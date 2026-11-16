---
name: bitacora
description: Escribe o actualiza la entrada de bitácora (docs/bitacora/fase-N/) de la tarea en curso y su fila en el índice. Úsala SIEMPRE en toda tarea que cambie código, config o docs de Signal Aduana, antes del commit y antes del check final, y cuando se pregunte qué se hizo en una tarea.
---

# bitacora

Cada tarea (una rama `tarea/N.M-<tema>` → `fase/N-…`) tiene **una** entrada en
`docs/bitacora/fase-N/NNN-<tema>.md`, que viaja en el mismo commit que el cambio. Formato y
reglas: `docs/bitacora/README.md`.

## Quién la escribe

**Solo el orquestador.** Los ejecutores no tocan `docs/bitacora/`: cada uno cierra su reporte con

```
DEVLOG CONTRIBUTION
agent: <modelo o rol>
what: <qué cambió>
why: <por qué>
files: <archivos>
test: <qué se corrió de verdad y con qué resultado>
follow-up: <pendientes o dudas>
```

El orquestador convierte esos bloques en **una** entrada coherente (no los pega tal cual) y anota en
«Ejecutor» qué modelos hicieron la tarea y cuántas vueltas de corrección necesitó.

## Pasos

1. **Fase, número y nombre.** Si la rama ya tiene su entrada
   (`git diff --name-only <fase> -- docs/bitacora/`), actualízala; nunca agregues una segunda.
   Si no: la fase es la de la rama (`tarea/4.2-…` → `fase-4`); el número es el siguiente al mayor
   del índice (global, no se reinicia); el tema es el de la rama sin el `N.M-`.
2. **Contenido**, con estas fuentes:
   - **Solicitud:** lo que pidió el autor, en sus términos (y lo que dice `docs/PLAN-FASE-N.md`).
   - **Motivación:** problema → daño real.
   - **Diseño técnico:** decisiones con su porqué y alternativas descartadas; indica cuáles tomó el
     autor. Aquí va la historia que no debe quedar en comentarios del código.
   - **Archivos:** `git diff --stat <fase>` y una línea por archivo.
   - **Cómo verificar:** comandos que de verdad se corrieron, con su resultado; los que no se
     corrieron, dicho así.
   - **Pendientes:** lo que quedó fuera; si toca el plan de la fase, que coincida.
3. **Índice.** Agrega o ajusta la fila en la sección de su fase de `docs/bitacora/README.md`.
4. **Plan.** Si la tarea cierra un punto de `docs/PLAN-FASE-N.md`, refléjalo allí (solo el
   orquestador).

## Reglas

- No reescribir entradas de tareas ya fusionadas: solo una nota aclaratoria al final, con fecha.
- Hechos, no intenciones. Si algo no se probó (Jira real, Windows, la ventana), va en Pendientes.
- Sin tokens, contraseñas, URLs o correos de instancias, ni datos de personas o clientes.
- Español, directo; sin relleno.

## Lista de cierre

- [ ] Exactamente una entrada de la rama, en el mismo commit que el cambio.
- [ ] Las seis secciones con contenido concreto (Pendientes puede omitirse si no hay).
- [ ] Fila en el índice con el enlace correcto y la fase coincide con la carpeta.
- [ ] «Ejecutor» indica quién hizo la tarea y cuántas vueltas hubo.
