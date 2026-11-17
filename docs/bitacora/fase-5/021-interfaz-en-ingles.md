# 021 — Interfaz en inglés y cierre de la traducción

**Fecha:** 2026-11-17
**Rama:** tarea/5.14-interfaz-en-ingles → develop
**Fase:** 5
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta. El orquestador revisó el reporte, tradujo los mensajes de consola de `ui/app.js` (un hueco de sus propios criterios), registró `viewData` en el glosario, cerró la regla de idioma en `CLAUDE.md` y escribió esta entrada.

## 1. Solicitud

Tarea 5.14 de `docs/PLAN-FASE-5.md`, la última de la ola T. Comprende:
- los identificadores y comentarios de `ui/app.js`;
- los `id` y atributos de `ui/index.html`;
- la pasada final de textos para quien programa en `src/domain/` y `src/jira/`.

Los textos visibles no cambian.

## 2. Motivación

| Problema | Daño real |
|---|---|
| `ui/app.js` seguía con nombres y comentarios en español (`vista`, `aplicarPrefs`, `feriadosExtra`…), y `index.html` con `id` en español (`prefAvisos`, `connAviso`, `data-fer`) | La regla de idioma (entrada 016) no se cumplía en la interfaz |
| La interfaz mezcla código con muchos textos visibles | Un renombre descuidado cambia lo que ve la persona, y un `id` mal renombrado deja un control sin efecto sin ningún error de sintaxis |

## 3. Diseño técnico

- **`ui/app.js`.** Se tradujeron:
  - los nombres del glosario (`DEFAULT_PREFS`, `BASE_HOLIDAYS`, `extraHolidays`, `applyPrefs`,
    `savePrefs`, `setView`, `listenEvents`…);
  - las variables locales (`simulated`, `site`, `email`, `VC.issues`…);
  - todos los comentarios.

  Las etiquetas de la interfaz citadas dentro de comentarios quedan en español, entre « ».
- **Colisión.** El glosario decía `vista` → `view`, pero `view` ya existía como el modo de la
  tabla (`data-view`, `VIEWS[view]`). El ejecutor eligió `viewData`, y el glosario lo registra.
- **`ui/index.html`.** Los `id` pasan a `prefNotifications`, `prefEndOfDay`, `prefWeekly`,
  `prefAutostart`, `prefTray` y `connNotice`; los `name` a `site` y `email`; `data-fer` a
  `data-holiday`. `lang="es"` se queda. En `styles.css` solo cambió un comentario.
- **Pasada final en Rust.** Se tradujeron los mensajes de `expect`, `assert` y `panic!` y los
  `eprintln!` que quedaban en `src/jira/*` y `examples/dump_simulated.rs`, y los comentarios de
  `domain/calendar.rs` que citaban nombres viejos de la interfaz. Los demás archivos de `src/` ya
  estaban en inglés.
- **Mensajes de consola: hueco de los criterios del orquestador.** La tarea pedía literales
  idénticos en `app.js`. Por eso el ejecutor dejó en español los 8 mensajes de `console.warn`, que
  solo se ven en las herramientas de desarrollo. Por la regla de textos para quien programa van en
  inglés: los tradujo el orquestador, volvió a aplicar Prettier, y el glosario ahora cita
  `console.*` de forma explícita.
- **Lo que queda en español a propósito:**
  - todos los textos visibles y los datos: mensajes, chips, abreviaturas de meses y días, «Todos»,
    «Sin tablero», `boardHow`, tableros, estados y `NO_DOC_RE`;
  - la regex `/prueba|producci/i`;
  - la clave `spe.feriados`;
  - `legado.js` y la API del legado que usa `generate.mjs`.
- **`CLAUDE.md`.** La regla de idioma ya no dice «hasta entonces conviven ambos idiomas»: la
  traducción terminó. Ahora incluye los textos para quien programa.

## 4. Archivos

- `ui/app.js`, `ui/index.html` y `ui/styles.css` (un comentario).
- `src/jira/mod.rs`, `src/jira/http.rs`, `src/jira/simulated.rs`, `src/jira/model.rs`,
  `src/domain/calendar.rs` y `examples/dump_simulated.rs`: textos para quien programa y comentarios.
- `docs/GLOSARIO.md`: `viewData` y `console.*`.
- `docs/PLAN-FASE-5.md`: la ola T marcada como hecha.
- `CLAUDE.md`: la regla de idioma.
- `docs/bitacora/README.md` (+1 fila) y esta entrada.

## 5. Cómo verificar

Corrido por el ejecutor en WSL:
- `cargo test`: 160 pruebas antes y 160 después, en verde.
- `cargo clippy --all-targets`: sin advertencias. `node --check ui/app.js` y `make ui-format`
  pasan.
- **Literales de `app.js`.** Comparó las cadenas, las plantillas (con `${…}` normalizado) y la
  regex, viejas contra nuevas: 786 contra 786. Las 20 diferencias son todas claves internas: ids de
  `$("…")` y `data-fer` → `data-holiday`.
- **Texto de `index.html`.** Comparó los nodos de texto y los atributos visibles (`title`,
  `placeholder`, `aria-label`): 214 contra 214, idénticos.
- **Ids.** Cada `$("x")` de `app.js` existe en `index.html`, y los 55 ids usados antes son los 55 de
  ahora con sus nombres nuevos.
- El `grep` de nombres viejos solo encuentra `tests/parity/generate.mjs` y `legado.js`, que usan a
  propósito la API del legado.

Corrido por el orquestador después de traducir los mensajes de consola:
- `make fmt-ui` y `make ui-format`: sin errores. `node --check ui/app.js`: sin errores.

Pendiente del autor antes del commit:

```bash
make check
```

No probado: la interfaz en la app real. Cubre panel de preferencias, conexión, feriados en el
calendario, filtros y tabla, que son los que dependen de los `id` renombrados. Se ve al levantar la
app o en la segunda pasada en Windows (5.7).

## 6. Pendientes

- Algunos comentarios de `src/domain/*.rs` dicen que una función viene «de `ui/app.js`», pero esas
  funciones hoy solo existen en `tests/parity/legado.js`. Son comentarios históricos del port; se
  pueden precisar en una tarea futura.
- En Windows hay que revisar a mano que todos los controles de la interfaz respondan.
