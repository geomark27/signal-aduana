# 001 — Preferencias, resumen y avisos (Rust)

**Fecha:** 2026-11-16
**Rama:** tarea/4.1-preferencias-avisos → fase/4-bandeja-avisos
**Fase:** 4
**Ejecutor:** no consta en git. Entrada **retroactiva**: la tarea se fusionó (PR #1, `2bb90ca`) antes de que existiera la bitácora; los datos los reunió un subagente de solo lectura (Sonnet 5.5) a partir del commit `0b2722c` y los contrastó el orquestador con el `git diff --stat`.

## 1. Solicitud

Tarea 1 de `docs/PLAN-FASE-4.md`: el núcleo en Rust de la fase 4 (preferencias del equipo, resumen
de horas para la bandeja y decisión de avisos), según los contratos E y F del plan. Sin interfaz ni
bandeja: eso son las tareas 2 y 3.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Las preferencias (y los feriados marcados) vivían en el navegador | Se pierden al limpiar los datos de la ventana y Rust no puede leerlas para el monitor de la bandeja |
| La lógica de «qué cargar para el resumen» estaba solo en el ejemplo `volcar_simulado` | El monitor de la tarea 3 no podría reutilizarla sin copiarla |
| No había una regla única de cuándo avisar | Avisos repetidos, a deshoras o en feriados |

## 3. Diseño técnico

Decisiones observadas en el código (el porqué, solo donde el código o el plan lo dicen):

- **Preferencias** (`src/preferencias.rs`): `cargar` nunca falla; sin archivo o con TOML dañado
  devuelve los valores por defecto, y si solo la hora guardada es inválida repone `16:30` y
  conserva el resto. `guardar` normaliza antes de escribir (hora inválida → `ErrorApp::Formato` y
  no deja archivo), escribe de forma atómica (`.tmp` + `rename`) y devuelve lo que quedó guardado.
  `feriadosExtra` es el `FeriadosExtra` del dominio, como pedía el plan; las fechas inválidas se
  descartan. Archivo `preferencias.toml` (`preferencias.simulado.toml` en simulado) en
  `dir_config`: son del equipo, no de la cuenta.
- **Validación de la hora sin cortes de texto**: se separa en `HH` y `MM` de dos dígitos ASCII, para
  que un carácter multibyte no rompa nada; las pruebas incluyen dígitos de ancho completo y tildes.
- **Resumen** (`src/resumen.rs`): `resumir` llama dos veces a `calcular_vista` (semana actual y
  anterior, tablero «Todos») y no reimplementa reglas de horas ni de feriados. `texto_bandeja`
  recorta a 120 caracteres porque Windows corta el tooltip cerca de 127.
- **Avisos** (`src/avisos.rs`): `decidir` es pura (reloj y preferencias entran por parámetro, sin
  Tauri). Ignora un `Resumen` cuyo `hoy` no coincide con la fecha de `ahora` («dato viejo no sirve
  para decidir»). `CierreJornada` usa `hoy_laborable` del resumen en vez de recalcular feriados.
  Si la hora de cierre no se puede interpretar, no envía el aviso (sin error). El registro de lo ya
  enviado va en `avisos.json` (`avisos.simulado.json`), con la misma escritura atómica; si falta o
  está dañado, vale el valor por defecto.
- **`jira::cargar_para_resumen`**: traslado de la lógica del ejemplo (lunes de `hoy − 98` días,
  `00:00` UTC−5 expresado como `05:00` UTC, `worklogs`/`wlTotal`, `spent` = suma si era 0). El
  ejemplo `volcar_simulado` pasa a usarla y solo agrega `hasPR`.
- **Comandos** `leer_preferencias` y `guardar_preferencias`, registrados en `lib.rs`. Guardar solo
  guarda: el efecto sobre el inicio con Windows lo agrega la tarea 3.
- **Desviación menor del plan**: `texto_bandeja` muestra «Hoy: no laborable» cuando hoy no es
  laborable, caso que el contrato F no define.

## 4. Archivos

- `src/preferencias.rs` (+217, nuevo) — tipos, defectos, validación, `cargar`/`guardar`, 8 pruebas.
- `src/resumen.rs` (+184, nuevo) — `Resumen`, `resumir`, `texto_bandeja`, 4 pruebas.
- `src/avisos.rs` (+344, nuevo) — `decidir`, `marcar`, registro en disco, 13 pruebas.
- `src/jira/mod.rs` (+47) — `cargar_para_resumen`.
- `src/comandos.rs` (+13) — comandos de preferencias.
- `src/lib.rs` (+5) — módulos y registro de comandos.
- `examples/volcar_simulado.rs` (+8 / −35) — usa `cargar_para_resumen`.
- Total del commit: 7 archivos, +818 / −35. `Cargo.toml` no cambia.

## 5. Cómo verificar

**No consta qué comandos se corrieron**: ni el commit ni esta sesión lo registran, y quien redacta
no los ejecutó. Los que pide el plan:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test                    # el plan esperaba las 91 previas más las nuevas
cargo clippy --all-targets    # sin advertencias
cargo build
cmp <volcado regenerado> pruebas/paridad/volcados/viernes.json   # debe ser idéntico
```

Las pruebas nuevas (25) se contaron leyendo el código: preferencias 8, resumen 4, avisos 13.

## 6. Pendientes

- Confirmar con `cargo` el resultado de las pruebas, de clippy y del `cmp` de paridad (no
  verificados por esta entrada).
- Falta una prueba directa de `cargar_para_resumen`; hoy se usa solo a través de `resumen.rs`.
- `decidir` no tiene prueba para: hora de cierre inválida, lunes ya enviado con una fecha de una
  semana anterior, lunes feriado con la semana anterior incompleta, ni un `Resumen` de otro día
  junto con `NoAutorizado`. Feriado y fin de semana se prueban fijando `hoy_laborable = false` a
  mano, no con un resumen calculado con feriados.
