# 010 — Aviso en la interfaz cuando falla «Iniciar con Windows»

**Fecha:** 2026-11-16
**Rama:** tarea/4.8-aviso-inicio → develop
**Fase:** 4
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta.

## 1. Solicitud

La entrada 007 dejó pendiente que, si el inicio con Windows falla, la interfaz solo vea la casilla
apagada sin explicación. El autor aprobó esta tarea (4.8), que solo cambia `ui/app.js`.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Rust deja la preferencia en `false` cuando no logra registrar el inicio, y la interfaz lo muestra sin decir nada | La persona cree que quedó activado, o no entiende por qué la casilla se apagó |

## 3. Diseño técnico

- **Un solo texto** (`AVISO_INICIO`): «No se pudo activar el inicio con Windows; la opción quedó
  desactivada.»
- **Al guardar desde el panel**: si se pidió activarlo y lo guardado volvió apagado, se apaga la
  casilla, el aviso sale dentro del panel con «Los demás cambios se guardaron.» y el panel **queda
  abierto**, porque la persona está mirando ese panel. En modo simulado Rust no toca el sistema y
  devuelve la preferencia tal cual (`true`), así que ahí no hay aviso.
- **Al llegar `signal://preferencias`**: si la preferencia pasó de `true` a `false`, el aviso va a
  `#notice`.
- **Limitaciones conocidas, no corregidas**:
  - **El fallo al arrancar no queda cubierto en la práctica.** Rust corrige la preferencia y emite el
    evento durante `setup`, antes de que la ventana exista y escuche; la interfaz lee después la
    preferencia ya corregida (`false`) y no tiene con qué compararla. Es un hueco de la tarea
    original, no del ejecutor. Para cubrirlo, Rust tendría que conservar el aviso y entregárselo a la
    interfaz.
  - El aviso de `#notice` solo se limpia al marcar un feriado o cuando otro mensaje lo reemplaza.
  - Si la preferencia ya estaba en `true` y falla la resincronización al guardar otros cambios, pueden
    verse el aviso del panel y el de `#notice` a la vez.

## 4. Archivos

- `ui/app.js` (+8 / −1) — la constante, el manejo al guardar y el del evento.
- `docs/bitacora/README.md` (+1) y esta entrada.

## 5. Cómo verificar

Corridos por el ejecutor: `node --check ui/app.js` sin errores y sin manejadores en línea en
`ui/index.html`. Los seis flujos del encargo (activar y guardar con éxito, con fallo, en simulado,
evento de fallo al arrancar, evento posterior y desactivar) se repasaron **leyendo el código**; el
orquestador leyó el diff completo. La ventana nunca se vio funcionando.

```bash
source ~/.nvm/nvm.sh
node --check ui/app.js
```

## 6. Pendientes

- Probar en Windows el aviso al guardar con un fallo real del registro.
- Cubrir el fallo al arrancar (ver Limitaciones).
- Limpiar `#notice` cuando el aviso deje de aplicar.

---

**Nota (2026-11-16):** el commit que se fusionó trajo `ui/app.js` entero reformateado: pasó de 639 a
1798 líneas (2211 cambiadas), no las +8 / −1 que cuenta la sección 4. Lo causó un formateador al
guardar, no el cambio de esta tarea. La lógica descrita aquí es la misma; el formato resultante pasó a
ser el oficial en la entrada 012.
