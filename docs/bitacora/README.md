# Bitácora — Signal Aduana de escritorio

Una entrada por tarea: cada rama `tarea/N.M-<tema>` que llega a `develop` por PR. La entrada viaja
en el mismo commit que el cambio que describe y vive en la carpeta de su fase, `fase-N/` (la fase
agrupa tareas; no es una rama).

La historia y el porqué de cada decisión van aquí, no en comentarios del código.

## Índice

La numeración es global y correlativa: no se reinicia en cada fase, porque las entradas se citan
por número.

### Fases 1 a 3

Sin entradas: se hicieron antes de que existiera la bitácora. Su historia está en
`docs/PLAN-FASE-1.md` a `PLAN-FASE-3.md`.

### Fase 4 — Bandeja y avisos

| # | Tarea | Rama | Fecha | Resumen |
|---|---|---|---|---|
| [001](./fase-4/001-preferencias-avisos.md) | 4.1 Preferencias, resumen y avisos | `tarea/4.1-preferencias-avisos` | 2026-11-16 | Entrada retroactiva |
| [002](./fase-4/002-interfaz-preferencias.md) | 4.2 Interfaz de preferencias | `tarea/4.2-interfaz-preferencias` | 2026-11-16 | Entrada retroactiva |
| [003](./fase-4/003-normas.md) | 4.0 Normas de trabajo y bitácora | `tarea/4.0-normas` | 2026-11-16 | `CLAUDE.md`, bitácora, skill y configuración de `delegar` |
| [004](./fase-4/004-flujo-directo.md) | 4.0 Flujo directo `develop → tarea` | `tarea/4.0-flujo-directo` | 2026-11-16 | Sin rama de fase: cada tarea nace de `develop` y vuelve por un solo PR; normas y `delegar` ajustados |
| [005](./fase-4/005-monitor.md) | 4.3 Monitor: ciclo de lectura, tooltip y avisos | `tarea/4.3-monitor` | 2026-11-16 | `monitor::ciclo` sin Tauri, con `Salida` para el tooltip y las notificaciones; sin sesión, token rechazado y fallo de red probados con el simulado y `wiremock` |

## Formato de cada entrada

Archivo `fase-N/NNN-<tema>.md`: `N` es la fase, `NNN` el número correlativo global de tres dígitos
y el tema, el de la rama sin el `N.M-`. La primera entrada de una fase crea su carpeta y su
sección en el índice. Encabezado y secciones obligatorias:

```markdown
# NNN — Título corto

**Fecha:** AAAA-MM-DD
**Rama:** tarea/N.M-<tema> → develop
**Fase:** N
**Ejecutor:** modelo(s) que hicieron la tarea y vueltas de corrección (o «orquestador»)

## 1. Solicitud
Qué se pidió, en lenguaje natural.

## 2. Motivación
Por qué hacía falta. Problema → daño real si no se hace.

## 3. Diseño técnico
Decisiones tomadas y sus razones, incluidas las alternativas descartadas y las decisiones
que tomó el autor (indicarlo).

## 4. Archivos
Cada archivo tocado con una línea sobre su cambio.

## 5. Cómo verificar
Comandos concretos. Distingue los que se corrieron de los que no.

## 6. Pendientes
Lo que quedó fuera o se descubrió en el camino. Omitir si no hay.
```

## Reglas

- La fase del encabezado y la carpeta coinciden.
- Una entrada de una tarea ya fusionada **no se reescribe**. Solo admite notas aclaratorias al
  final, con fecha. Lo nuevo va en la entrada de la tarea nueva.
- **Hechos, no intenciones**: no se anota como hecho lo que no se verificó; si algo no se probó
  (por ejemplo, contra el Jira real o en Windows), se dice en Pendientes.
- **Una rama = una entrada = un dueño**: la escribe el orquestador a partir de los
  `DEVLOG CONTRIBUTION` de los ejecutores.
- Sin datos reales: ni tokens, ni contraseñas, ni URLs o correos de instancias, ni datos de
  personas o clientes.
- Español, directo y concreto: «la semana anterior sin feriados tenía meta 0», no «se corrigió un
  error».
