# 002 — Interfaz de preferencias (`ui/`)

**Fecha:** 2026-11-16
**Rama:** tarea/4.2-interfaz-preferencias → fase/4-bandeja-avisos
**Fase:** 4
**Ejecutor:** orquestador (Claude Sonnet 5.5), sin delegar, en una vuelta. Entrada **retroactiva**: la tarea se fusionó (PR #3, `9e4a52b`, commit `3e4c9ae`) antes de que existiera la bitácora. Fue anterior a la norma de delegar (ver 003).

## 1. Solicitud

Tarea 2 de `docs/PLAN-FASE-4.md`: un panel de preferencias, feriados guardados por Rust en lugar del
navegador y la escucha de los eventos de la bandeja. Solo `ui/app.js`, `ui/index.html` y
`ui/styles.css`.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Los feriados marcados vivían en `localStorage` | Se perdían al limpiar la ventana y el monitor de la bandeja (tarea 3) no los vería, así que calcularía mal «hoy es laborable» |
| No había dónde cambiar avisos, hora de cierre, inicio con Windows ni la bandeja | La tarea 1 guardaba preferencias que nadie podía editar |

## 3. Diseño técnico

- **Panel**: `<dialog>` con casillas (avisos, resumen del lunes, iniciar con Windows, minimizar a
  la bandeja), hora de cierre (`<input type="time">`), nota del modo simulado, y Guardar/Cancelar.
  El botón «Preferencias» está en el encabezado, también en la pantalla de conexión. Un error de
  `guardar_preferencias` se muestra dentro del panel.
- **Hora y resumen del lunes dependen de «Avisos activos»**: con los avisos apagados quedan
  deshabilitados. Decisión de quien implementó, no del plan.
- **Feriados**: se leen con `leer_preferencias` antes de pedir la primera vista. `toggleFeriado`
  no cambia nada en pantalla hasta que Rust confirma el guardado; si falla, queda como estaba y el
  error sale en `#notice` (equivale a «revertir» del plan, sin un estado intermedio). Un guardado a
  la vez (`prefsBusy`).
- **Migración única** de `localStorage["spe.feriados"]`: solo si las preferencias vienen sin
  feriados; se conservan las claves `AAAA-MM-DD` con valor booleano. Si guardar falla, la clave se
  conserva y se reintenta en el siguiente arranque. Si las preferencias ya traen feriados, la clave
  vieja se descarta sin migrar. Decisión de quien implementó; el plan solo pedía migrar una vez.
- **Eventos**: `signal://actualizar` ejecuta `refresh(true)` si hay sesión y no hay carga en curso;
  `signal://preferencias` relee y repinta. Con el panel abierto solo se actualiza «Iniciar con
  Windows», lo único que Rust cambia por su cuenta, para no pisar lo que la persona está editando.
- **Permisos**: no se tocó `capabilities/`. Según la documentación de Tauri 2, `core:default`
  incluye los permisos de eventos; no se comprobó en ejecución.
- **Pie de página**: una frase sobre la bandeja y los avisos.

## 4. Archivos

- `ui/app.js` (81 líneas cambiadas) — preferencias, migración, feriados, panel y eventos.
- `ui/index.html` (+26) — botón «Preferencias» y el `<dialog>`.
- `ui/styles.css` (+17) — estilos del panel con las variables existentes (claro y oscuro).
- Total del commit: 3 archivos, +119 / −5.

## 5. Cómo verificar

Corridos:

```bash
node --check ui/app.js          # ok
```

También se comprobó que `index.html` no tiene manejadores en línea y que todos los ids del panel
existen y se usan en el JS.

No corridos: la ventana nunca se vio funcionando; los flujos del plan (primera vez sin
preferencias, migración, cambiar la hora y guardar, error al guardar, marcar un feriado, evento de
actualizar durante una carga) se repasaron leyendo el código.

## 6. Pendientes

- Probar la interfaz en la ventana: panel, tema claro y oscuro, ancho mínimo de 760 px.
- Los eventos no se disparan hasta que la tarea 3 cree la bandeja y el monitor; confirmar entonces
  que `listen` funciona con los permisos actuales.
- Guardar «Iniciar con Windows» solo guarda el valor: el registro con el sistema lo hace la tarea 3.
