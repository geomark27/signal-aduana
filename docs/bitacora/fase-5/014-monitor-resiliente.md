# 014 — El monitor sobrevive a un pánico del ciclo

**Fecha:** 2026-11-17
**Rama:** tarea/5.5-monitor-resiliente → develop
**Fase:** 5
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta, sin correcciones. El orquestador escribió la tarea, revisó el diff y escribió esta entrada.

## 1. Solicitud

Tarea 5.5 de `docs/PLAN-FASE-5.md`. Si `monitor::ciclo` entra en pánico, el bucle del monitor no
debe terminar: tiene que registrar el fallo, mostrarlo en el tooltip y seguir con el ciclo
siguiente. El bucle debe poder probarse sin Tauri.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El bucle de `vigilancia::iniciar` corría `ciclo` sin protección (entrada 007) | Un pánico en `ciclo`, o en `Salida::tooltip` o `Salida::notificar`, terminaba la tarea en segundo plano. El monitor dejaba de leer horas y de avisar hasta reiniciar la app, sin ninguna señal |
| En `release` la app no tiene consola (`windows_subsystem = "windows"`) | Ese fallo no se veía en ningún lado. Ahora el tooltip lo muestra |
| El bucle vivía dentro de `iniciar(app)`, atado a `AppHandle` | No había forma de probarlo |

## 3. Diseño técnico

- **`vigilancia::bucle(estado, salida, aviso, espera_inicial, intervalo, max_ciclos)`.** Es el bucle
  extraído y la `Memoria` vive dentro.
  - `iniciar(app)` sigue siendo el único punto que conoce `AppHandle`. Llama a
    `bucle(..., ESPERA_INICIAL, INTERVALO, None)`.
  - `max_ciclos` solo se usa en las pruebas.
  - La API pública (`pedir_ciclo`, `Vigilante`, `SalidaTauri`, `iniciar`, `esperar`, `Motivo`) y los
    tiempos (30 s y 15 min) no cambian.
- **`ciclo_protegido(f)`.** Envuelve el futuro con `AssertUnwindSafe(f).catch_unwind()`, de la
  dependencia `futures` que ya existía. Si hay pánico, devuelve `Err` con su mensaje (`&str`,
  `String` o «sin mensaje»), recortado a 200 caracteres.
- **Qué pasa ante un pánico:**
  - Se escribe `eprintln!("vigilancia: el ciclo entró en pánico: …")`.
  - El tooltip pasa a `monitor::texto_error_interno`: «Signal Aduana · Error interno, se reintenta»,
    con « (simulado)» en modo simulado, recortado a `MAX_TEXTO_BANDEJA` igual que los demás textos.
  - El siguiente ciclo llega al cabo de `INTERVALO`, o antes si alguien llama a `pedir_ciclo`.
- **Segundo pánico.** El tooltip de error usa la misma `Salida`, que puede ser la que falló. Por eso
  esa llamada va dentro de `std::panic::catch_unwind`; si vuelve a fallar, solo queda la línea del log.
- **Sigue funcionando en el instalador.** `[profile.release]` no usa `panic = "abort"`, así que
  `catch_unwind` también funciona en la versión de Windows.
- **Ajuste del ejecutor:** clippy pedía `Option::is_none_or`, estable desde Rust 1.82. La versión
  mínima de Rust del repo es 1.80, así que se usó `map_or(true, …)`.

## 4. Archivos

- `src/vigilancia.rs`:
  - nuevas funciones `ciclo_protegido`, `mensaje_de_panico` y `bucle`;
  - `iniciar` delega en `bucle`;
  - 4 pruebas nuevas.
- `src/monitor.rs`: `texto_error_interno` y 1 prueba. `ciclo` no cambia.
- `docs/bitacora/README.md` (+1 fila) y esta entrada.

## 5. Cómo verificar

Corrido por el ejecutor en WSL:
- `cargo test vigilancia`: 9 pruebas en verde, 4 de ellas nuevas.
- `cargo test monitor`: 8 en verde.
- `cargo test`: 140 de la librería y 1 de paridad, 0 fallos.
- `cargo clippy --all-targets`: sin advertencias.

Pruebas nuevas:
- `ciclo_protegido` devuelve `Ok(7)` en el caso normal, y un `Err` que contiene «boom» ante un pánico.
- Un mensaje de pánico de 500 caracteres queda en 200.
- Una `Salida` cuyo primer `tooltip` entra en pánico, con 2 ciclos: los tooltips quedan exactamente
  en `[texto_error_interno(true), texto_sin_sesion(true)]`.
- Una `Salida` que siempre entra en pánico, con 3 ciclos: el bucle termina.
- `texto_error_interno` lleva « (simulado)» solo en modo simulado y no supera `MAX_TEXTO_BANDEJA`.

Las pruebas del bucle tienen un límite de 5 s, para fallar en vez de colgarse. El orquestador leyó
el diff completo y no corrió las pruebas. Las corre el autor antes del commit:

```bash
make check
```

No probado: el tooltip de error en la bandeja real de Windows. Forzar un pánico en la app instalada
no forma parte de la lista de comprobación.

## 6. Pendientes

- Si `notificar` entra en pánico a mitad de la lista de avisos, ese ciclo no llega a guardar el
  registro de avisos. Los avisos ya mostrados se repetirían en el ciclo siguiente. Es poco probable
  y no se corrige en esta fase.
- Si un pánico ocurriera mientras se tiene tomado un `std::sync::Mutex` del estado, ese candado
  quedaría envenenado. Hoy `tooltip` y `notificar` se llaman fuera de cualquier candado.
- Contar los pánicos o avisar a la persona por notificación, si hiciera falta. El registro a archivo
  es la tarea 7.4.
- La tarea 5.4 (PR #16) se fusionó sin su entrada de bitácora. Por decisión del autor no se abrió una
  rama para escribirla: se recoge en el cierre de la fase (5.8).
