# 007 — Integración: monitor en marcha, avisos, inicio con Windows y ganchos de sesión

**Fecha:** 2026-11-16
**Rama:** tarea/4.5-integracion → develop
**Fase:** 4
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta. El propio ejecutor corrigió 2 advertencias de `clippy` en sus pruebas.

## 1. Solicitud

Partes de integración de la tarea 3 de `docs/PLAN-FASE-4.md`: ejecutar el monitor (4.3) sobre la
bandeja (4.4), enviar los avisos de Windows, sincronizar el inicio con Windows con la preferencia y
reaccionar a la sesión. El orquestador dividió la tarea 3 en 4.3 a 4.6 y el autor aprobó la división.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El monitor y la bandeja estaban sueltos: nada ejecutaba el ciclo | Ni tooltip con horas ni avisos |
| «Iniciar con Windows» solo guardaba el valor | La preferencia no hacía nada |
| Al cerrar sesión, el tooltip conservaba las horas de la cuenta anterior | Datos de una persona visibles tras cerrar su sesión |

## 3. Diseño técnico

- **Un solo bucle** (`vigilancia::iniciar`): espera 30 s (`ESPERA_INICIAL`), corre `monitor::ciclo` y
  repite cada 15 min (`INTERVALO`). Al ser un único bucle, nunca hay dos ciclos a la vez. Pedir un
  ciclo usa `Notify::notify_one`: peticiones seguidas se juntan en una, y una petición durante un
  ciclo hace que el siguiente empiece enseguida, una sola vez.
- **Quién pide un ciclo**: restaurar sesión (si hay conexión), iniciar sesión y «Actualizar ahora» de
  la bandeja. Cerrar sesión pone el tooltip «Sin sesión» (`monitor::texto_sin_sesion`, extraída de
  `ciclo` para usarla en los dos sitios).
- **Salida real** (`SalidaTauri`): el tooltip va por `escritorio::fijar_tooltip` y los avisos por el
  plugin de notificaciones; si mostrar un aviso falla, solo se registra.
- **Inicio con Windows** (`inicio.rs`): `decidir` / `sincronizar` / `corregir` / `aplicar`, al arrancar
  y cada vez que se guardan las preferencias. **En modo simulado `sincronizar` sale antes de tocar el
  plugin** (ni `is_enabled`, ni `enable`, ni `disable`) y solo deja una línea en el log. Si el plugin
  falla, la preferencia queda en `false`, se guarda y se emite `signal://preferencias`.
- **Plugins**: `tauri-plugin-notification` y `tauri-plugin-autostart` (con `--oculto`). `Cargo.lock`
  resolvió notification 2.8.1 (no la 2.7.0 que había en la caché) y añadió 17 paquetes, algunos de
  otras plataformas.
- **`capabilities/` sin cambios**: Rust llama a los plugins directamente, no la interfaz.
- **Descartado**: pedir un ciclo al guardar preferencias. Haría consultas a Jira por cada feriado
  marcado; los cambios de hora de cierre o de feriados se aplican en el siguiente ciclo (hasta 15 min).
- **Dato corregido del reporte del ejecutor**: dijo que la interfaz no escucha `signal://preferencias`;
  sí lo hace desde la 4.2 (`ui/app.js`).

## 4. Archivos

- `src/vigilancia.rs` (nuevo) — bucle, `Vigilante`, `SalidaTauri`, `pedir_ciclo` y 6 pruebas.
- `src/inicio.rs` (nuevo) — inicio con Windows y 3 pruebas.
- `src/monitor.rs` — `texto_sin_sesion` y una prueba.
- `src/comandos.rs` — los comandos de sesión y `guardar_preferencias` reciben `AppHandle`.
- `src/lib.rs` — módulos, plugins y arranque del vigilante y del inicio.
- `src/escritorio.rs` (+1) — «Actualizar ahora» pide un ciclo.
- `Cargo.toml`, `Cargo.lock` — los dos plugins.
- `docs/bitacora/README.md` (+1) y esta entrada.

## 5. Cómo verificar

Corridos por el ejecutor (constan en sus registros de `target/`): `cargo check`; `cargo test` con 135
pruebas de la librería y 1 de integración, sin fallos; arranque con
`SIGNAL_SIMULADO=1 timeout 50 cargo run -- --oculto` sin `panic` y con las líneas
`inicio: modo simulado, no se registra el inicio con Windows (preferencia: false)` y, a los 30 s,
`vigilancia: ciclo terminado: SinSesion`; sin procesos vivos al final (contrastado con `ps`).

**`clippy`**: la primera pasada dio 2 advertencias en las pruebas de `inicio.rs` (el ejecutor las
corrigió); la pasada limpia posterior **no consta** en los registros conservados, así que la repite
el autor. El arranque en WSLg imprime avisos de Mesa/EGL y un `Gtk-CRITICAL`, sin consecuencias.

No corridos: lo hace el autor.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test
cargo clippy --all-targets
cargo build
```

## 6. Pendientes

- **Probar en Windows real**: registro del inicio automático, notificaciones (que se vean y sin el
  prefijo `[Simulado]`), tooltip y ciclo a los 30 s y cada 15 min.
- **Si el inicio con Windows falla, la interfaz no lo explica**: la casilla queda apagada sin aviso
  tras pulsar Guardar. Mostrar un mensaje es una mejora de la interfaz.
- Si `ciclo` entrara en pánico, la tarea del bucle moriría sin reanudarse.
- Los cambios de preferencias (hora de cierre, feriados) llegan al monitor en el siguiente ciclo.
