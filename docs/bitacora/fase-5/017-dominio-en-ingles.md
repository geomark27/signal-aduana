# 017 — Dominio y paridad en inglés

**Fecha:** 2026-11-17
**Rama:** tarea/5.10-dominio-en-ingles → develop
**Fase:** 5
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta. El orquestador revisó el diff, corrigió un nombre que había fijado mal en el glosario (`days_between` → `days_word`), actualizó `CLAUDE.md` y escribió esta entrada.

## 1. Solicitud

Tarea 5.10 de `docs/PLAN-FASE-5.md`: primera tarea de código de la ola T. Pasa al inglés el módulo
del dominio y la paridad siguiendo `docs/GLOSARIO.md` (sección 2), sin cambiar el comportamiento ni
ningún texto visible.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El dominio estaba en español (`src/dominio/`, `Vista`, `Entrada`, `FeriadosExtra`…) y la regla del repo ahora es tener el código en inglés (entrada 016) | El código del que depende todo lo demás seguía fuera de la norma |
| Las claves JSON de la vista son un contrato triple: Rust, `ui/app.js` y los 62 casos de paridad | Si se cambiaba un solo lado, la interfaz dejaba de mostrar datos o la paridad fallaba |

## 3. Diseño técnico

- **Módulo.** `src/dominio/` pasa a `src/domain/` con `calendar`, `hours`, `week`, `board`, `issue` y
  `view`. Se tradujeron tipos, campos, funciones, constantes, comentarios, `mod tests` y los nombres
  de las pruebas, con los nombres del glosario.
- **Contrato de la vista.** Todas las estructuras serializadas de `view.rs` llevan
  `rename_all = "camelCase"`. Las claves quedan así:
  - `View`: `today`, `week`, `board`, `issues`, `boards`, `stats`, `hours`, `weeks`, `byBoard`.
  - `HoursState`: `prevSecs`, `workdays`, `days`, `holidays`.
  - `DayHours`: `holiday`.
  - `ChartWeek`: `isoWeek`.
  - `BoardRow`: `open`, `statuses`, `last`.
  - `ViewInput`: `{ issues, week, board, extraHolidays, today }`.

  Las claves que ya estaban en inglés y vienen del legado (`WeekState`, `Stats` y los derivados de
  las tareas) no cambian.
- **Interfaz.** En `ui/app.js` solo cambian el objeto que se envía a `calcular_vista` y las lecturas
  de la vista. Las variables JS (`vista`, `feriadosExtra`, `VC.tareas`…) esperan a la 5.14, y el
  nombre del comando y su parámetro `entrada` esperan a la 5.13.
- **Otros archivos.** En `lib`, `comandos`, `resumen`, `avisos` y `preferencias` solo cambian rutas,
  tipos y accesos a campos del dominio. El resto lo traducen las tareas 5.11 a 5.13.
- **Paridad.**
  - `pruebas/paridad/` se movió a `tests/parity/`. Cargo no la toma como prueba porque no tiene
    `main.rs`. Dentro quedan `cases/`, `dumps/friday.json` y `dumps/monday.json`, `generate.mjs` y
    `README.md`. `legado.js` se movió idéntico byte a byte.
  - `generate.mjs` sigue llamando a la API en español del legado y escribe las claves nuevas.
  - Los 31 casos se regeneraron con nombres `fri-…` / `mon-…` y archivos `.input.json` y
    `.expected.json`.
  - `tests/paridad.rs` pasó a `tests/parity.rs` y `examples/volcar_simulado.rs` a
    `examples/dump_simulated.rs`.
  - Los volcados no se regeneraron, porque su JSON de tareas no cambia. Solo se movieron.
- **Nombre corregido por el orquestador.** El glosario traducía `dias(n)` de `hours.rs` como
  `days_between`, pero la función devuelve la palabra «día» o «días». El ejecutor lo señaló y se
  renombró `days_word` en el código y en el glosario.
- **Decisión del ejecutor:** el local `dia_seg` pasó a `day_target_secs` y no a `day_secs`, para no
  tapar la función `week::day_secs`.

## 4. Archivos

- `src/domain/` (7 archivos, movidos desde `src/dominio/` y traducidos).
- `src/lib.rs`, `src/comandos.rs`, `src/resumen.rs`, `src/avisos.rs`, `src/preferencias.rs`: solo las
  referencias al dominio.
- `ui/app.js`: el contrato de la vista (48 líneas).
- `tests/parity/` (movida desde `pruebas/paridad/`):
  - `generate.mjs` traducido;
  - los 62 casos regenerados;
  - `dumps/` y `legado.js` sin cambios de contenido;
  - el README con las rutas nuevas.
- `tests/parity.rs` (antes `tests/paridad.rs`) y `examples/dump_simulated.rs` (antes
  `examples/volcar_simulado.rs`).
- `docs/GLOSARIO.md`: `days_word`.
- `CLAUDE.md`: las rutas del árbol (`src/domain/`, `tests/parity/`, `dump_simulated`), la regla «los
  cálculos viven en `src/domain/`» y la ruta de `legado.js`.
- `docs/bitacora/README.md` (+1 fila) y esta entrada.

## 5. Cómo verificar

Corrido por el ejecutor en WSL:
- `cargo test`: 160 pruebas antes (159 de la librería y 1 de paridad) y 160 después, todas en verde.
  Se mantiene el número de pruebas de cada archivo del dominio.
- `cargo clippy --all-targets`: sin advertencias.
- `node --check ui/app.js`: sin errores. `make fmt-ui` no cambió nada y `make ui-format` pasa.
- `node tests/parity/generate.mjs`: 31 casos y 62 archivos.
- Comparó los 31 casos con `git show HEAD:` sobre los antiguos, aplicándoles solo el cambio de
  claves: los 31 tienen valores idénticos. Los volcados y `legado.js`, comparados con `cmp`, son
  idénticos.

Corrido por el orquestador:
- **Literales de los archivos del dominio, viejos frente a nuevos.** Comparados con un script. Solo
  cambian:
  - los nombres de variables dentro de los formatos (`{mes}` → `{month}`, `{padre}` → `{parent}`,
    `{semana}` → `{week}`), que dejan el texto mostrado igual;
  - las claves JSON de las pruebas;
  - dos palabras dentro de comentarios.
- **Diff de `ui/app.js`.** Los textos visibles («abiertas», «Semana anterior», «Todos»…) no cambian.
  No queda ninguna lectura de claves viejas de la vista.

Pendiente del autor antes del commit, porque el cambio `days_word` no se compiló:

```bash
make check
```

## 6. Pendientes

- Siguen en español: `VC.tareas` y las variables de `ui/app.js` (5.14); el comentario de
  `src/jira/mod.rs` que cita `calcular_vista` y lo de Jira en `examples/dump_simulated.rs` (5.11).
- `tests/parity/legado.js` conserva la palabra `dominio` en un comentario, porque no se toca.
- Los planes de fases anteriores, el roadmap y las entradas de bitácora ya fusionadas siguen citando
  `src/dominio/` y `pruebas/paridad/`. Son historia y no se reescriben.
