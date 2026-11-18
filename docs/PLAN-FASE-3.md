# Signal Aduana de escritorio — Plan de la fase 3

**Punto de partida:** fases 1 y 2 completas (`docs/PLAN-FASE-1.md`, `docs/PLAN-FASE-2.md`): la app
funciona en modo simulado con caché, actualización automática y fuentes locales; 55 pruebas en verde.

**Objetivo de la fase 3: «Rust calcula, JS pinta».** Hoy todos los cálculos del tablero viven en
`ui/app.js` sin ninguna prueba. Se pasan a un módulo `dominio` en Rust, con pruebas unitarias y
**pruebas de paridad** contra el JS actual, y la interfaz deja de calcular: pide una «vista» ya
calculada y la dibuja. Esto además deja listos los cálculos para la fase siguiente (avisos y bandeja
del sistema, que necesitan saber «te faltan 3 h» sin la ventana abierta).

**Regla de oro: el tablero debe mostrar exactamente lo mismo que hoy.** No se cambia ninguna regla de
negocio, texto ni redondeo. Si el JS actual tiene un comportamiento raro, se replica y se reporta.

Fuera de alcance: señales (`renderSignals`), tabla, documentos y gráficos siguen armándose en JS, pero
alimentados por la vista. Bandeja, notificaciones e instalador son fases posteriores.

## Reglas para todos los agentes

Las 8 reglas de `docs/PLAN-FASE-1.md` (léelas allí): solo tus archivos; ninguna conexión a servicios
reales (todo simulado; nunca `SIGNAL_SIMULADO=0`); sin tokens; sin commits;
`export PATH="$HOME/.cargo/bin:$PATH"`; todo en español; reporta desviaciones; respuesta final breve
con archivos, verificaciones reales y pendientes.

| Tarea | Puede tocar | Cuándo |
|---|---|---|
| 1 | `pruebas/paridad/` (nuevo), `examples/volcar_simulado.rs` (nuevo) | Primera ola |
| 2 | `src/dominio/` (nuevo), `src/comandos.rs`, `src/lib.rs`, `tests/` (nuevo) | Primera ola, en paralelo con la 1 |
| 3 | `ui/app.js` | Segunda ola, después de la 1 y la 2 |

---

## Contrato D — la vista

Comando `calcular_vista`, **sin estado**: recibe todo lo que necesita y no requiere sesión.

```ts
invoke("calcular_vista", { entrada }) → Vista

Entrada = {
  tareas: Tarea[],            // como las devuelve `cargar_tareas`, con `worklogs` ya unidos; pueden traer `hasPR: boolean` y otros campos extra
  semana: "AAAA-MM-DD",       // lunes de la semana elegida (si no es lunes → error `formato`)
  tablero: string,            // "Todos" o el nombre de un tablero
  feriadosExtra: { [ymd]: boolean },   // cambios del usuario sobre los feriados base
  hoy?: "AAAA-MM-DD"          // opcional (pruebas); por defecto, hoy en Ecuador (UTC-5)
}

Vista = {
  hoy, semana,
  tablero: string,            // el recibido, o "Todos" si ese tablero no existe en los datos
  tareas: TareaDerivada[],    // mismo orden que la entrada
  tableros: string[],         // nombres, en el orden de `boards()` del JS (depende de la semana)
  stats: Stats,               // semana elegida, filtrada por `tablero`  → weekStats(semana)
  horas: EstadoHoras,         // siempre con todos los tableros          → hoursState(semana) + datos de renderHoursAlert
  semanas: SemanaGrafico[],   // 12 semanas, de la más antigua a la elegida, filtradas por `tablero`
  porTablero: FilaTablero[]   // una por cada elemento de `tableros`, mismo orden, sin filtro de tablero
}

TareaDerivada = todos los campos recibidos (también los que Rust no conoce: usar `#[serde(flatten)]`
  con un mapa) más: board, boardHow, finDay, resDay, noFin, late, hasWl, needDoc, creDay
  — exactamente lo que hoy calcula `prep()` — y cada elemento de `worklogs` con day, weekend, monday, wk.

Stats = { secs, worked: string[], perBoard: {tablero: secs}, perDay: {ymd: {tablero: secs}},
          resolved: string[], est, spent, withDue, onTime }
  // `worked` y `resolved` son claves de tarea, en el orden de `tareas`.

EstadoHoras = todos los campos que devuelve `hoursState(wk)`:
  { total, target, targetH, expected, missing, behind, sev, title, detail, current, todaySecs, todayLabor, today }
  más: prevSecs (segundos de la semana anterior, todos los tableros),
       laborables: number (días laborables de la semana),
       dias: [{ day, secs, feriado }] × 5 (lunes a viernes, todos los tableros),
       feriados: string[] (los días feriados de la semana)
  // `sev`, `title` y `detail` salen tal como los deja `hoursState`, SIN los ajustes posteriores de
  // `renderHoursAlert` (cambio de "warn" a "info", estados de carga): esos se quedan en JS.

SemanaGrafico = { wk, semanaIso: number, secs, perBoard: {tablero: secs}, target: number /* horas */ }

FilaTablero = { board, secs, worked: number, resolved: number,
                abiertas: number, estados: {estado: cantidad} /* de las abiertas */, ultima: string /* lastActivity */ }
```

Detalles obligatorios:
- Segundos como enteros; horas meta como enteros. Fechas `AAAA-MM-DD`.
- Los textos (`title`, `detail`, `boardHow`) deben coincidir **carácter por carácter** con el JS, incluido el formato de horas de `fh` (`7.5 h`, `8 h`).
- Mapas con orden estable (`BTreeMap`) para que el JSON sea reproducible.
- Errores: `ErrorApp::Formato` para fechas o semana inválidas.

Funciones del JS actual que definen el comportamiento (todas en `ui/app.js`): `isFeriado`, `laborDays`,
`weekTarget`, `weekHolidays`, `resolveBoard` (con `BOARD_NAMES`, `KANBAN_PROJECTS`, `KANBAN_TYPES`,
`COMP_BOARDS`, `PROJECT_BOARDS`), `localDate`, `dateOnly`, `mondayOf`, `addDays`, `isoWeek`, `fmtD`,
`h`, `fh`, `prep` (con `NO_DOC_RE`), `lastActivity`, `boards`, `weekStats`, `daySecs`, `hoursState`,
`FERIADOS_BASE`, `WEEK_TARGET`, `DAY_TARGET`.

---

## Tarea 1 — Datos de referencia desde el JS actual

Produce el «resultado esperado» ejecutando **el JS de hoy** con Node, para que la tarea 2 demuestre
que Rust da lo mismo.

1. `examples/volcar_simulado.rs`: programa mínimo que imprime en JSON las tareas del simulado para un
   `hoy` dado (argumento), ya con las horas unidas — es decir, lo que la interfaz tiene en memoria
   tras `cargar_tareas` + `claves_con_horas` + `leer_horas` (las horas leídas reemplazan `worklogs`
   y `wlTotal`; si `spent` es 0 se pone la suma; mira `fetchWorklogs` en `ui/app.js`). Agrega también
   `hasPR` según `claves_con_pr`. Usa las funciones públicas de `signal_aduana::jira`.
2. `pruebas/paridad/legado.js`: copia **literal** de las funciones y constantes del JS listadas en el
   contrato D, tomadas de `ui/app.js`, sin DOM. Únicos cambios permitidos: recibir `hoy`,
   `feriadosExtra`, `issues` y `board` como estado inyectable en vez de globales del navegador. Marca
   cada cambio con un comentario.
3. `pruebas/paridad/generar.mjs`: para cada caso, carga las tareas, ejecuta el legado y arma un objeto
   con **la forma exacta de `Vista`** (contrato D). Escribe:
   - `pruebas/paridad/casos/<nombre>.entrada.json` (la `Entrada` completa) y
   - `pruebas/paridad/casos/<nombre>.esperado.json` (la `Vista`), con claves ordenadas y formato estable.
4. Casos (al menos 12), con dos volcados de `hoy` distintos — un viernes y un lunes — y que cubran:
   semana actual y dos anteriores; una semana futura; una semana con feriados base (2 y 3 de noviembre
   de 2026); `feriadosExtra` que quita un feriado base y que agrega uno; cada tablero como filtro, uno
   inexistente y "Todos"; una semana completa en feriado (vía `feriadosExtra`).
5. `pruebas/paridad/README.md`: cómo regenerar (dos comandos) y qué significa cada archivo.

**Verificar:** `cargo run --example volcar_simulado -- 2026-11-13` produce JSON válido;
`node pruebas/paridad/generar.mjs` termina sin errores y es reproducible (dos corridas → mismos
archivos, compruébalo con `sha256sum`). No compiles mientras la tarea 2 compila si choca el bloqueo de
cargo: espera y reintenta.

## Tarea 2 — Módulo `dominio` en Rust

1. `src/dominio/` con funciones **puras** (sin Tauri, sin reloj: `hoy` siempre llega como parámetro), separadas por tema:
   `calendario.rs` (fechas, lunes, semana ISO, feriados, metas, formato `fh`/`fmtD`),
   `tablero.rs` (`resolveBoard`), `tarea.rs` (derivados de `prep`), `semana.rs` (`weekStats`, `boards`, `lastActivity`),
   `horas.rs` (`hoursState`), `vista.rs` (arma `Vista` desde `Entrada`). `chrono` para fechas.
2. Comando `calcular_vista(entrada)` en `comandos.rs` (no requiere sesión), registrado en `lib.rs`. Si
   `hoy` no llega, usa `jira::simulado::hoy_ecuador()` (o muévela a `dominio::calendario` y reexporta).
3. Pruebas unitarias por módulo, con casos de borde: cambio de año en semana ISO; `localDate` de una hora
   cercana a medianoche UTC; feriado quitado/agregado; subtarea que hereda tablero del padre (y el
   límite de profundidad); tarea sin fechas; semana futura, pasada completa e incompleta, y actual
   atrasada / al día / primer día.
4. **Prueba de paridad** `tests/paridad.rs`: recorre `pruebas/paridad/casos/*.entrada.json`, calcula la
   vista y la compara con el `.esperado.json` como `serde_json::Value`. Al fallar debe decir el caso y
   la primera ruta JSON que difiere. Si la carpeta no tiene casos, la prueba **falla** (no se salta).
   Los casos los genera la tarea 1 en paralelo: empieza por los puntos 1–3 y, si al llegar aquí aún no
   existen, espera y reintenta unas cuantas veces.
5. Si la paridad falla: el JS legado manda. Corrige Rust. Solo si el error está en el **adaptador**
   `generar.mjs` (no en las funciones copiadas de `legado.js`) puedes corregirlo ahí y regenerar;
   repórtalo. Si descubres un comportamiento del JS que parece un error, replícalo igual y repórtalo.

**Verificar:** `cargo test` (hoy 55; más las tuyas y la de paridad con todos los casos en verde),
`cargo clippy --all-targets` sin advertencias, `cargo build`.

## Tarea 3 — La interfaz deja de calcular (`ui/app.js`) — segunda ola

1. `render()` pasa a pedir la vista: `calcular_vista` con `{ tareas: issues, semana: weekKey, tablero: board, feriadosExtra }` y dibuja con el resultado. Como ahora es asíncrono, usa un contador para descartar respuestas viejas (clics rápidos de semana o tablero) y no dejes la pantalla en blanco mientras llega.
2. Reemplaza por datos de la vista: `prep()` (usa `vista.tareas` como fuente de los campos derivados para señales, tabla y documentos), `weekStats`, `boards`, `lastActivity`, `daySecs`, `hoursState`, `laborDays`/`weekTarget`/`weekHolidays` donde se usen para calcular. `worked` y `resolved` llegan como listas de claves: conviértelas a lo que necesiten `renderSignals` y `renderTable` (`Set` y lista de tareas).
3. Elimina del JS las funciones de cálculo que queden sin uso. Se quedan las de **presentación**: formato (`fmtD`, `fh`, `dayName`), colores, `stackBars`, HTML, paginación, y `toggleFeriado` con su `localStorage` (al cambiar un feriado se vuelve a pedir la vista).
4. `docCandidates`, `docWeek`, `docStatus`, la caché (`restoreCache`/`saveCache`) y la carga de datos deben seguir funcionando: la caché guarda las tareas **sin** depender de campos derivados (se recalculan con la vista).
5. Los paginadores y cambios de orden de la tabla no necesitan pedir la vista otra vez: reutiliza la última.
6. Nada visible debe cambiar.

**Verificar:** `node --check ui/app.js`; `cargo build`; abre la app en modo simulado
(`timeout 25 ./target/debug/signal-aduana`) y confirma que no hay pánicos ni errores en la terminal.
No puedes ver la ventana: repasa a mano cada `render*` contra la forma de `Vista` y lista en tu
respuesta cualquier campo que hayas necesitado y la vista no traiga.

## Después (orquestador)
Compilar, pruebas, revisar la paridad y entregar al autor una lista de comprobación visual contra la fase 2.
