# 020 — Escritorio y comandos en inglés

**Fecha:** 2026-11-17
**Rama:** tarea/5.13-escritorio-en-ingles → develop
**Fase:** 5
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta, sin correcciones. El orquestador revisó el diff, comprobó por su cuenta los comandos y sus parámetros, actualizó `CLAUDE.md` y escribió esta entrada.

## 1. Solicitud

Tarea 5.13 de `docs/PLAN-FASE-5.md`, la última grande de Rust en la ola T. Pasar al inglés los
comandos de Tauri, las preferencias, los avisos, el resumen, el monitor, la vigilancia, la bandeja,
el inicio con Windows y `lib.rs`, siguiendo `docs/GLOSARIO.md` (sección 5).

## 2. Motivación

| Problema | Daño real |
|---|---|
| Los módulos de escritorio y los 13 comandos seguían en español | Eran lo último de Rust fuera de la regla de idioma (entrada 016) |
| Los nombres de los comandos y sus parámetros los resuelve Tauri en tiempo de ejecución | El compilador no detecta si no coinciden con lo que envía la interfaz: la función deja de responder sin un error visible al compilar |
| Las preferencias son a la vez un JSON de la interfaz y el archivo `preferences.toml` | Con `#[serde(default)]`, una clave mal renombrada se lee en silencio como valor por defecto |

## 3. Diseño técnico

- **Módulos.** Pasan a `commands`, `preferences`, `notifications`, `summary`, `watcher`, `desktop` y
  `autostart`. `monitor` conserva su nombre. Todos van traducidos completos, con sus pruebas y los
  textos para quien programa.
- **Comandos.** Los 13 comandos usan los nombres y parámetros del glosario:
  - `sign_in { site, email, token }`, `read_worklogs { keys, since }` y `keys_with_hours { since }`;
  - `find_documents { keys }`, `save_cache { data }` y `compute_view { input }`;
  - `save_preferences { preferences }`.

  En `ui/app.js`, cuando el nombre de la variable no coincide con la clave, la propiedad se escribe
  completa (`site: sitio`), porque las variables JS se traducen en la 5.14.
- **Preferencias.** Quedan con las claves `notifications { enabled, endOfDayTime, weeklySummary }`,
  `startWithWindows`, `minimizeToTray` y `extraHolidays`. Cambian a la vez en Rust, en el archivo, en
  `PREFS_DEFECTO`, en las lecturas y escrituras de la interfaz y en la prueba con el JSON de la
  interfaz.
- **Eventos y bandeja.** Los eventos pasan a `signal://refresh` y `signal://preferences`, y los ids
  del menú a `open`, `refresh` y `quit`. Los textos del menú no cambian.
- **Disco.**
  - Los archivos pasan a `preferences.toml` y `notifications.json`, con sus variantes `.simulated.`.
  - El argumento de arranque oculto pasa a `--hidden` (decisión del autor, entrada 016).
  - No hay migración. Un archivo anterior se ignora y la app arranca con los valores por defecto. Una
    entrada de «Iniciar con Windows» ya registrada con `--oculto` hay que volver a activarla.
- **Ajuste del ejecutor.** `Watcher` lleva `#[derive(Default)]`: con el nombre `new`, clippy pide
  `Default` (`new_without_default`). No cambia el comportamiento.

## 4. Archivos

- `src/commands.rs`, `src/preferences.rs`, `src/notifications.rs`, `src/summary.rs`,
  `src/watcher.rs`, `src/desktop.rs` y `src/autostart.rs`: antes `comandos`, `preferencias`,
  `avisos`, `resumen`, `vigilancia`, `escritorio` e `inicio`.
- `src/monitor.rs` y `src/lib.rs`, traducidos.
- `ui/app.js`: los 13 `invoke`, las claves de preferencias, `PREFS_DEFECTO` (solo sus claves) y los
  dos eventos.
- `docs/COMPILAR-WINDOWS.md`: los archivos en disco, `.simulated.`, `--hidden` y `extraHolidays`.
- `CLAUDE.md`: el árbol de `src/`.
- `docs/bitacora/README.md` (+1 fila) y esta entrada.

## 5. Cómo verificar

Corrido por el ejecutor en WSL:
- `cargo test`: 160 pruebas antes y 160 después, en verde.
- `cargo clippy --all-targets`: sin advertencias. `node --check ui/app.js` y `make ui-format`
  pasan.
- El `grep` de nombres viejos (`comandos`, `preferencias`, `avisos`, `resumen`, `vigilancia`,
  `escritorio`, `inicio::`, `--oculto` y los eventos viejos) no encuentra nada en `src`, `tests` ni
  `examples`.
- Comparó los literales de cada archivo con los anteriores. Todos los textos visibles son idénticos:
  avisos con «[Simulado]», tooltips, menú, título de la ventana y el error de la hora de cierre.

Corrido por el orquestador:
- Comparó los 13 comandos de `generate_handler!` con los 13 `invoke` de la interfaz: coinciden.
- Leyó los parámetros de `sign_in`, `read_worklogs`, `find_documents`, `save_preferences`,
  `keys_with_hours`, `save_cache` y `compute_view` contra las claves que envía la interfaz: coinciden.
- Leyó el diff completo de `ui/app.js`: ningún texto visible cambió.

Pendiente del autor antes del commit:

```bash
make check
```

No probado, porque necesita la app real: el arranque con `--hidden`, «Actualizar ahora» desde la
bandeja (`signal://refresh`), guardar preferencias y ver el aviso al instante. Se cubre en la segunda
pasada en Windows (5.7).

## 6. Pendientes

- La 5.14: los nombres de variables y funciones de `ui/app.js` (`PREFS_DEFECTO`, `feriadosExtra`,
  `aplicarPrefs`…), los `id` del HTML y la pasada final de textos para quien programa en
  `src/domain/` y `src/jira/`.
- En Windows, si ya se había activado «Iniciar con Windows», hay que desactivarlo y volver a
  activarlo.
