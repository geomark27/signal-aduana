# 025 — Cierre de la fase 5

**Fecha:** 2026-11-17
**Rama:** tarea/5.8-cierre-fase-5 → develop
**Fase:** 5
**Ejecutor:** orquestador (Opus 5.5), una vuelta.

## 1. Solicitud

Tarea 5.8 de `docs/PLAN-FASE-5.md`: cerrar la fase con su estado final, el roadmap al día y la
entrada retroactiva de la 5.4. El autor pidió cerrarla sin esperar el registro detallado de
Windows y atender las sorpresas después.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El plan seguía «en curso» y su sección «Estado al cierre» estaba vacía | Quien retome el proyecto no sabría qué se hizo, qué quedó pendiente ni dónde sigue |
| La 5.4 se fusionó sin entrada de bitácora (PR #16) | Un cambio de comportamiento, el resumen semanal en el primer día laborable, sin su historia |
| Lo pendiente de Windows no tenía un lugar asignado | Se podía perder entre fases |

## 3. Diseño técnico

- **Entrada retroactiva de la 5.4 (024).** Se escribió en esta rama porque el plan lo dispone así,
  con el mismo criterio que las entradas 001 y 002. Toma lo que consta en el commit `424a2a9`, en el
  plan y en el reporte del ejecutor. Deja dicho que no viajó con su commit y que la prueba de `spent`
  no comprueba nada en la práctica.
- **Plan.** El estado pasa a «cerrada». La sección «Estado al cierre» incluye:
  - las 13 tareas, con su ejecutor, su PR y su entrada;
  - el resultado de Windows, tal como lo reportó el autor;
  - los cinco criterios de «hecha la fase cuando», con su estado real: dos se cumplen, dos se
    cumplen en parte y uno no;
  - los pendientes que pasan a la fase 6.
- **Roadmap.** La fase 5 queda «Hecha el 2026-11-17, con pendientes de Windows que pasan a la
  fase 6». La fase 6 recibe esos pendientes en un bloque «Heredado de la fase 5».
- **Por qué cerrar con criterios parciales.** Es decisión del autor: la app funciona en Windows y el
  detalle se verificará cuando se pruebe con el Jira real, que exige otra pasada en Windows de todos
  modos. Se dejan escritos como «parcial» o «no», no como hechos.

## 4. Archivos

- `docs/bitacora/fase-5/024-pruebas-avisos.md` (nueva, retroactiva de la 5.4).
- `docs/PLAN-FASE-5.md`: el estado y la sección «Estado al cierre».
- `docs/ROADMAP.md`: el estado, la fila de la fase 5 y lo heredado en la fase 6.
- `docs/bitacora/README.md` (+2 filas: 024 y 025) y esta entrada.

## 5. Cómo verificar

Corrido por el orquestador en esta rama:

```bash
bash scripts/verificar-bitacora.sh
```

Solo cambian documentos: no aplica `cargo test`. La lista de PR se tomó de
`git log --merges origin/develop` (del #15 al #26).

## 6. Pendientes

Los que pasan a la fase 6, en «Estado al cierre» de `docs/PLAN-FASE-5.md`:
- registrar en Windows la lista de comprobación y la segunda pasada;
- confirmar los «a verificar»;
- probar el instalador NSIS;
- comprobar el inicio con Windows en modo real;
- dos detalles menores de código.

La fase 6 necesita, antes de empezar, el visto bueno de Tecnología para usar API tokens personales.
