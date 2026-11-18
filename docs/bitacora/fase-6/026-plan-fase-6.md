# 026 — Plan de la fase 6

**Fecha:** 2026-11-18
**Rama:** tarea/6.0-plan-fase-6 → develop
**Fase:** 6
**Ejecutor:** orquestador (Opus 5.5), una vuelta.

## 1. Solicitud

Con la fase 5 cerrada, el autor pidió crear la tarea 6.0 y escribir el plan de la fase 6: validar
con el Jira real.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El cliente real solo se probó contra `wiremock` | Campos personalizados, paginación, PR o límites pueden fallar con el Jira de la empresa sin que ninguna prueba lo anticipe |
| La fase 6 implica datos reales de la empresa | Sin reglas escritas de qué se reporta, un dato real podía terminar en un reporte, en el código o en la bitácora |
| La fase 5 dejó pruebas de Windows sin registrar | Se podían perder entre fases |

## 3. Diseño técnico

- **Requisito previo.** Según el autor (2026-11-18), Tecnología ya aprobó los API tokens personales
  de Atlassian, y el mismo tipo de token se usa en Draco. Las tres olas pueden empezar: A0 (Windows en
  simulado), A (Jira real) y B (diagnóstico anónimo).
- **Reglas de los datos reales.**
  - Claude y los ejecutores nunca se conectan a Jira ni a Confluence.
  - El autor reporta números, códigos y mensajes de error, y la **forma** de una respuesta con los
    valores cambiados por ficticios.
  - No reporta URLs, correos, nombres, claves ni títulos reales.
  - Cada hallazgo se convierte en una prueba con `wiremock` o con el simulado.
- **Qué probar en concreto.** Se revisó el cliente:
  - endpoints: `myself`, `search/jql` con `nextPageToken`, `worklog`, `remotelink` y la búsqueda de
    Confluence;
  - los cuatro campos personalizados fijos: `customfield_10015`, `10020`, `10031` y `10057`;
  - la consulta de PR, `development[pullrequests].all > 0`;
  - el tiempo límite de 30 s y el reintento ante el código 429, con una espera de hasta 10 s.

  Las tareas 6.1 a 6.5 dicen qué mirar de cada uno.
- **Decisiones del autor (2026-11-18):**
  1. Si la consulta de PR no funciona con un token clásico, se usa el respaldo por palabras. El token
     con alcances queda para la fase 8.
  2. **Se hace la 6.7**, el diagnóstico anónimo: la app arma un resumen sin datos reales para pegarlo
     en el reporte, porque en `release` no hay consola.
  3. Si los ids de los campos personalizados no coinciden, se corrigen **fijos en el código**, porque
     hay un solo sitio de Jira.
  4. Ejecutores: Sonnet 5.5 para la 6.7 y las correcciones de Rust, y opencode free para las
     pequeñas.
- **Numeración.**
  - 6.1 a 6.5 siguen el roadmap.
  - 6.H es lo heredado de la fase 5.
  - Las correcciones son la 6.6 y de la 6.8 en adelante.
  - El cierre toma el primer número libre.

## 4. Archivos

- `docs/PLAN-FASE-6.md` (nuevo).
- `docs/ROADMAP.md`: estado y fila de la fase 6.
- `.claude/delegar.json`: `PLAN-FASE-5.md` → `PLAN-FASE-6.md` en `instrucciones` y en
  `archivos_orquestador`.
- `docs/bitacora/README.md` (nueva sección «Fase 6» y su fila) y esta entrada.

## 5. Cómo verificar

Corrido por el orquestador en esta rama:

```bash
bash scripts/verificar-bitacora.sh
python3 -m json.tool .claude/delegar.json > /dev/null
```

Solo cambian documentos: no aplica `cargo test`.

## 6. Pendientes

- Las olas A0 (Windows en simulado), A (Jira real, el autor) y B (6.7).
- El orquestador no verificó por su cuenta la aprobación de Tecnología: consta según lo informado por
  el autor.
