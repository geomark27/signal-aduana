# 006 — Bandeja, cierre a la bandeja, una sola instancia y arranque oculto

**Fecha:** 2026-11-16
**Rama:** tarea/4.4-bandeja → develop
**Fase:** 4
**Ejecutor:** Sonnet 5.5 (subagente), dos vueltas: la segunda por un hueco de la tarea original, no del agente.

## 1. Solicitud

Punto 2 de la tarea 3 de `docs/PLAN-FASE-4.md` (`src/escritorio.rs`: bandeja, cerrar la ventana, una
instancia, arranque oculto). El orquestador dividió esa tarea 3 en cuatro (4.3 monitor, 4.4
bandeja, 4.5 integración, 4.6 guía de Windows) y el autor aprobó la división. Esta es la parte de la
ventana y la bandeja; no envía notificaciones, no lanza el monitor ni toca el inicio con Windows.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Con solo una ventana, cerrarla sale de la app | El monitor y los avisos no pueden seguir vivos entre jornadas |
| Sin control de instancias, abrir la app dos veces | Dos monitores, avisos duplicados y dos ventanas sobre los mismos datos |
| El inicio con Windows mostraría la ventana en cada arranque del equipo | Molestia diaria; hace falta arrancar oculto |

## 3. Diseño técnico

- **Bandeja**: ícono de la app, tooltip inicial `resumen::texto_bandeja(None, simulado)` y menú
  «Abrir Signal Aduana», «Actualizar ahora», separador y «Salir». Clic izquierdo (al soltar) abre la
  ventana; el derecho muestra el menú. «Actualizar ahora» **solo emite** `signal://actualizar`; el
  ciclo del monitor lo suma la 4.5.
- **Cerrar la ventana**: lee las preferencias en el momento de cerrar (la persona pudo cambiarlas);
  con `minimizarABandeja` se esconde, si no la app sale. «Salir» del menú siempre sale.
- **Una instancia**: `tauri-plugin-single-instance`, registrado como primer plugin; un segundo
  arranque trae la ventana al frente (`show`, `unminimize`, `set_focus`).
- **Arranque oculto**: con `--oculto` la ventana no se muestra. La ventana nace con
  `"visible": false` en `tauri.conf.json` y `configurar` la muestra, en vez de ocultarla después de
  crearla, para evitar un parpadeo.
- **Segunda vuelta (hueco de la tarea original, del orquestador)**: la tarea pedía que `configurar`
  devolviera `Err` si fallaba la bandeja. Con la ventana oculta y `windows_subsystem = "windows"`,
  un fallo (por ejemplo al iniciar sesión en Windows, con la barra de tareas aún no lista) abortaba
  `setup` y la app quedaba sin ventana ni mensaje. Ahora crear la bandeja es `crear_bandeja`; si
  falla, se registra y la app sigue, y dos funciones puras fijan las reglas: sin bandeja la ventana
  **se muestra aunque venga `--oculto`** y **cerrarla cierra la app** (esconderla la dejaría
  inalcanzable). `configurar` conserva su firma y ya no propaga ese error.
- **Permisos**: `capabilities/` sin cambios. Según lo leído, `core:default` incluye
  `core:event:default`, suficiente para que la interfaz escuche `signal://actualizar`; no se probó
  en una interfaz real.
- **Dependencias**: `tauri` con la *feature* `tray-icon` y `tauri-plugin-single-instance` 2.5.5
  (tomado de la caché de cargo; `Cargo.lock` +17 líneas).

## 4. Archivos

- `src/escritorio.rs` (nuevo) — API pública (`mostrar_ventana`, `fijar_tooltip`, `instancia_unica`,
  `configurar`, `es_arranque_oculto` y las constantes), `crear_bandeja`, las dos reglas puras y 5
  pruebas.
- `src/lib.rs` (+3) — módulo, plugin de instancia única como primero y `escritorio::configurar(app)?`
  en `setup`.
- `Cargo.toml`, `Cargo.lock` — *feature* `tray-icon` y el plugin.
- `tauri.conf.json` — `"visible": false` en la ventana.
- `docs/bitacora/README.md` (+1) y esta entrada.

## 5. Cómo verificar

Corridos por el ejecutor (constan en sus registros de `target/`), tras la corrección: `cargo check`,
`cargo test` (120 de la librería y 1 de paridad, sin fallos; antes de la corrección eran 118) y
`cargo clippy --all-targets` sin advertencias. Arranque en WSLg con
`SIGNAL_SIMULADO=1 timeout 40 cargo run -- --oculto`: sin `panic`, vivo hasta el límite de tiempo,
sin el mensaje de «no se pudo crear la bandeja» y sin procesos al final. **Antes** de la corrección,
un segundo arranque salió con código 0 y quedó un solo proceso (`pgrep` = 1): la instancia única
funciona; esa prueba no se repitió tras la corrección.

El arranque en WSLg imprime avisos de Mesa/EGL y un `Gtk-CRITICAL gtk_widget_get_scale_factor`; no se
determinó su origen. El ejecutor corrió un `cargo build` aparte pese a que la tarea lo prohibía (para
poder arrancar); no tuvo efectos.

No corridos: lo hace el autor.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test
cargo clippy --all-targets
cargo build
```

## 6. Pendientes

- **Probar en Windows real**: ícono, menú, clic izquierdo, cerrar con y sin `minimizarABandeja`,
  `--oculto`, que un segundo arranque traiga la ventana al frente y, si se puede, el caso de la
  bandeja no disponible al iniciar sesión. En WSLg no hay bandeja ni ventana que comprobar.
- La rama de fallo de la bandeja solo está cubierta por las pruebas de las dos funciones puras; no
  se provocó un fallo real.
- La 4.5 debe usar `fijar_tooltip` (seguro aunque la bandeja no exista: solo registra) y escuchar
  `signal://actualizar` en el monitor.
