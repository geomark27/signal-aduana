# 030 — Componente Lista

**Fecha:** 2026-11-18
**Rama:** tarea/6.10-componente-lista → develop
**Fase:** 6
**Ejecutor:** un subagente Sonnet 5.5, en una vuelta. El orquestador (Opus 5.5) escribió el plan y
el CHANGELOG, revisó el trabajo y agregó una regla de CSS (el margen de la barra de la tabla).

## 1. Solicitud

El autor pidió que la sección de detalle sea un componente reutilizable. Hoy la tabla cambia según
el indicador elegido («Abiertas», «Doc. de despliegue»…), pero el paginado solo se ve en algunas
listas y los filtros no se comparten. También pidió una interfaz más limpia.

Se armaron tres propuestas en un lienzo con datos ficticios, y el autor eligió la **A**: pestañas
Resumen · Tareas · Señales · Despliegues y un solo componente de lista. El plan lo divide en dos
tareas en serie, porque las dos tocan los mismos archivos:
- esta, la 6.10, que crea el componente sin cambiar el diseño;
- la 6.11, que arma las pestañas.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Había dos paginados escritos a mano: la tabla de 20 en 20 y Señales de 4 en 4, con su estado y sus manejadores | Cualquier mejora había que hacerla dos veces, y el rediseño iba a sumar un tercero |
| No había búsqueda ni filtros por lista; el único filtro era el de tablero, global | Con muchas tareas abiertas no había forma de encontrar una por clave o por estado |
| El paginado solo aparecía con más de 20 tareas | Para el autor parecía una función de «Abiertas» y no de todas las listas |

## 3. Diseño técnico

- **`renderList(id, cfg)` en `ui/app.js`.** Es un solo componente, con el estado de cada lista en
  `listStates[id]` y `cfg` con este formato:
  `{ items, filters, text, sizes, defaultSize, resetKey, renderBody, emptyText, searchLabel, pagerLabels, onPage }`.
  Escribe en contenedores fijos: `#<id>Tools`, el cuerpo (lo pinta `renderBody`) y `#<id>Pager`.
- **La barra se construye una sola vez por lista**, con eventos por delegación. Al escribir solo
  se vuelven a pintar los chips, el cuerpo y el paginado, nunca el `<input>`, y así no se pierde el
  foco. Cada cambio repinta solo esa lista, no la pantalla entera.
- **Búsqueda.** No distingue mayúsculas ni tildes. En Señales se quitan las etiquetas HTML del
  texto (`stripTags`) y se decodifican cuatro entidades (`&amp;`, `&lt;`, `&gt;`, `&quot;`), porque
  ese texto llega escapado.
- **Filtros.** Son chips con su conteo, calculado después de la búsqueda. Un chip en 0 se oculta
  salvo que esté activo.
  - Tareas: por estado (Por hacer, En curso, Concluidas). En la vista de documentos de despliegue,
    por documento (Faltan, Con documento, No aplica, Sin revisar).
  - Señales: por severidad (Altas, Medias, Info).
- **Filas por página.** Tareas usa 10, 20 o 50 (20 por defecto); Señales, 4, 8 o 20 (4 por
  defecto, como antes). Lo elegido se recuerda por lista en `localStorage` (`spe.list.<id>.size`).
- **Reinicio.** Cada lista tiene una `resetKey`:
  - Tareas: vista, tablero, semana y orden, la misma que antes.
  - Señales: tablero, semana y cantidad de señales.

  Si cambia, la lista vuelve a la página 1. Un filtro que ya no existe en la lista, o que quedó en 0
  tras ese cambio, vuelve a «Todas».
- **Sin cambios de reglas ni de diseño.** Las vistas, las señales, el orden, `docStatus` y las
  pistas se calculan igual que antes. Se quitaron `T_PAGE`, `tPage`, `tSig`, `SIG_PAGE`, `sigPage`,
  `sigCount` y los dos manejadores de paginado.

## 4. Archivos

- `ui/app.js`: el componente (`renderList`, `buildListBar`, `readListSize`, `normalizeText`,
  `stripTags`); `renderTable` y `renderSignals` lo usan.
- `ui/index.html`: barras `#tTools` y `#sigTools`.
- `ui/styles.css`: `.ltools`, `.lsearch`, `.lchips`, `.lsize`, más el margen de `#tTools` que agregó
  el orquestador. Solo usan las variables existentes, así que el tema oscuro funciona sin reglas
  nuevas.
- `CHANGELOG.md`: una línea en `[Unreleased]`, sección «Cambiado».
- `docs/PLAN-FASE-6.md`:
  - la ola E, el rediseño (6.10 y 6.11);
  - las correcciones toman la 6.6 y después el primer número libre.
- `docs/bitacora/README.md` (fila) y esta entrada.

## 5. Cómo verificar

Corrido por el ejecutor:

```bash
node --check ui/app.js          # sin errores
make fmt-ui && make ui-format   # «All matched files use Prettier code style!» (con nvm cargado)
```

Además:
- un grep de los nombres viejos (`T_PAGE`, `tPage`, `sigPage`…) da 0 resultados;
- no hay manejadores en línea;
- no hay CRLF.

El ejecutor revisó leyendo el código, sin abrir la app, cinco casos:
1. el foco al escribir;
2. que un cambio de semana o de tablero vuelve a la página 1;
3. los filtros de la vista de documentos;
4. que se recuerda el tamaño elegido;
5. que un filtro vacío vuelve a «Todas».

El orquestador revisó el diff y corrió `bash scripts/verificar-bitacora.sh`. `make check` lo corre el
autor.

## 6. Pendientes

- **No se vio en la app.** El autor lo prueba con `make run` en el worktree, en modo simulado:
  - escribir en las dos búsquedas, también con tildes;
  - los chips y sus conteos;
  - cambiar las filas por página y volver a pintar;
  - la ventana angosta;
  - el tema oscuro.
- **Chip activo en 0 tras una búsqueda.** Queda visible y activo, con «Nada coincide…». Se decide
  en la revisión si debe volver solo a «Todas».
- **Sigue la 6.11:** las pestañas de la propuesta A sobre este componente.
