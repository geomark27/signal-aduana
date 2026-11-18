# 031 — Pestañas (propuesta A)

**Fecha:** 2026-11-18
**Rama:** tarea/6.11-pestanas → develop
**Fase:** 6
**Ejecutor:** un subagente Sonnet 5.5, en una vuelta. El orquestador (Opus 5.5) escribió el
CHANGELOG, revisó el contrato entre los `id` del HTML y `app.js`, y cambió `selectTab` para que cada
pestaña empiece justo debajo de la barra fija.

## 1. Solicitud

Es la segunda tarea del rediseño que pidió el autor: la propuesta A, elegida entre tres prototipos
con datos ficticios. Se organiza en pestañas Resumen · Tareas · Señales · Despliegues, con la semana
y los tableros fijos arriba, sobre el componente Lista de la 6.10.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Todo estaba en una sola página larga: indicadores, gráficos, tableros, señales y la tabla | Para llegar a la tabla había que desplazarse; la tabla cambiaba según el indicador elegido sin que quedara claro dónde se estaba |
| Los documentos de despliegue eran una «vista» escondida de la tabla de tareas | No tenían un lugar propio ni su propia lista |
| Paneles plegables para ganar espacio | Un parche para una página demasiado larga |

## 3. Diseño técnico

- **Barra fija (`.stickybar`)**, con `position: sticky`. Lleva los controles de semana y tableros de
  siempre y las pestañas.
  - Las pestañas siguen el patrón accesible: `role="tablist"`, `tab` y `tabpanel`, `aria-selected`,
    `tabindex` móvil, y las flechas, Inicio y Fin con activación automática.
  - La pestaña activa se recuerda en `localStorage` (`spe.tab`).
- **Contadores en las pestañas:**
  - Tareas: las tareas de la vista «Semana» del tablero actual.
  - Señales: las altas, en rojo; si no hay, las medias, en ámbar.
  - Despliegues: los que tienen PR y no tienen documento. Solo se muestra cuando ya se revisaron los
    documentos.

  Un contador en 0 se oculta.
- **Resumen:** el aviso de horas, los indicadores, los dos gráficos y «Horas por tablero» a todo el
  ancho.
- **Tareas:**
  - Un selector de vista (Semana, Trabajadas, Concluidas, Abiertas) con sus conteos, y la tabla con
    el componente Lista.
  - Sale el botón «Ver todas las de la semana», porque el selector lo reemplaza.
  - La vista `nodoc` deja de ser una vista de la tabla.
- **Señales:** la lista completa, con búsqueda y filtros.
- **Despliegues:** una lista propia (`renderList("d", …)`) con las mismas tareas y el mismo orden que
  la antigua vista `nodoc`: primero las que faltan.
  - Muestra solo las columnas que importan ahí: clave, resumen, tablero, estado, fin de desarrollo y
    despliegue.
  - Filtra por documento.
  - El marcado de las celdas lo comparten Tareas y Despliegues: `summaryCell`, `boardCell` y
    `resDayCell`.
- **Navegación.** Todo pasa por `selectTab`, y la regla de orden de las vistas está en una sola
  función, `chooseView`.
  - Cada indicador lleva a su pestaña y vista: «Doc. de despliegue» a Despliegues; Trabajadas,
    Concluidas y Abiertas a Tareas con esa vista.
  - «N abiertas» de un tablero lleva a Tareas → Abiertas con ese tablero.
  - «Ver lista» de una señal lleva a Despliegues.
  - Se quitó el comportamiento de «volver a Semana» al repetir el clic en un indicador.
- **Al cambiar de pestaña con la página desplazada**, la nueva empieza justo debajo de la barra
  fija. Es el cambio del orquestador.
- **Se quitaron los paneles plegables:** `collapsed`, `applyCollapse`, `[data-panel]`,
  `spe.collapsed` y sus estilos.
- **Sin cambios de reglas.** Las vistas, las señales, `docStatus`, `docWeek`, los indicadores y los
  gráficos se calculan igual. `renderList` no se tocó.

## 4. Archivos

- `ui/index.html`: la barra fija con las pestañas, los cuatro paneles y la tabla de Despliegues.
- `ui/app.js`:
  - funciones nuevas: `selectTab`, `chooseView`, `renderDocs`, `renderTabCounts`, `setTcount`,
    `scrollToEl`, `goDocs`, `docHint`, `boardSuffix`, `summaryCell`, `boardCell`, `resDayCell` y
    `SEG_VIEWS`;
  - se quitaron el plegado, `tReset`, `VIEWS.nodoc` y los `aria-pressed` de los indicadores.
- `ui/styles.css`:
  - reglas nuevas: `.stickybar`, `.tabs`, `.tcount`, `.pane`, `.seg` y `.segb`;
  - se quitaron `.ptoggle`, `.chev`, `.panel.collapsed`, `.grid2.top` y `#signalsBody .ltools`.
- `CHANGELOG.md`: una línea en `[Unreleased]`, sección «Cambiado».
- `docs/bitacora/README.md` (fila) y esta entrada.

## 5. Cómo verificar

**Lo corrió el ejecutor:**
- `node --check ui/app.js`: sin errores.
- `make fmt-ui` y `make ui-format`: «All matched files use Prettier code style!».
- Un grep de plegado y de manejadores en línea: sin resultados.
- Los 20 `id` que usa `app.js` existen una sola vez en `index.html`.
- Sin CRLF.

**Lo revisó el ejecutor leyendo el código, sin abrir la app:**
- el teclado en las pestañas;
- que se recuerda la pestaña;
- la navegación desde los indicadores, los tableros y las señales;
- que los contadores se ocultan en 0;
- que Despliegues vuelve a la página 1 al cambiar de semana o de tablero.

**Lo corrió el orquestador, tras su cambio:** `node --check`, `make fmt-ui`, `make ui-format` y
`bash scripts/verificar-bitacora.sh`.

`make check` lo corre el autor.

## 6. Pendientes

- **No se vio en la app.** El autor lo prueba con `make run` en el worktree. Hay que revisar:
  - la barra fija al desplazarse;
  - las pestañas con el teclado;
  - los contadores;
  - Despliegues con documentos cargados y sin cargar;
  - la ventana angosta (pestañas con desplazamiento lateral);
  - el tema oscuro.
- **Ordenar por columna** solo se puede en la tabla de Tareas: `querySelector("thead")` toma la
  primera tabla. Despliegues no tiene columnas ordenables.
- **Versión 0.2.0.** Cuando el autor apruebe el rediseño en su equipo: una tarea con
  `make bump bump=minor`, después `make promote` y `make release`.
