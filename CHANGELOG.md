# Cambios por versión

Los cambios que nota quien usa Signal Aduana se anotan aquí. El formato sigue
[Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y las versiones siguen
[Versionado Semántico](https://semver.org/lang/es/): `MAYOR.MENOR.PARCHE`.

Cada tarea que cambia algo visible anota su línea en **[Unreleased]**, bajo *Añadido*, *Cambiado*,
*Corregido* o *Eliminado*. El detalle técnico de cada tarea va en la bitácora (`docs/bitacora/`).
Para publicar, `make bump` convierte [Unreleased] en la sección de la versión y `make release`
usa esa sección como notas del release en GitHub (ver «Publicar una versión» en `CLAUDE.md`).

## [Unreleased]

## [0.2.0] - 2026-11-18

### Cambiado

- La pantalla se organiza en pestañas: Resumen (aviso de horas, indicadores, gráficos y horas por tablero), Tareas (con un selector de vista: semana, trabajadas, concluidas o abiertas), Señales y Despliegues. La semana, los tableros y las pestañas quedan fijos arriba, cada pestaña muestra su contador y se recuerda la última abierta. Los paneles plegables ya no existen.
- Las listas de tareas y de señales tienen búsqueda por clave o texto, filtros con su conteo (por estado, por documento de despliegue o por severidad) y un selector de filas por página que se recuerda. Todas usan el mismo paginado.

## [0.1.0] - 2026-11-18

### Añadido

- Tablero semanal con las horas, tareas y estados de Jira de la persona: indicadores de la semana,
  horas por día y por tablero, señales y la lista de tareas con filtros, vistas y orden.
- Conexión con Jira Cloud mediante el API token personal. El token se guarda en el Administrador de
  credenciales de Windows y nunca en archivos. La última carga queda en caché para abrir la app sin
  red.
- Detección de pull requests y de documentos de despliegue (Confluence o enlaces de la tarea). Si la
  consulta de PR no está disponible, se usa una regla por palabras.
- Feriados de Ecuador y feriados propios marcados en el calendario, descontados de la meta de horas.
- Ícono en la bandeja con el resumen de horas en el tooltip y un menú para abrir, actualizar o salir.
  Cerrar la ventana la esconde en la bandeja si así se configura, y la app abre una sola vez.
- Avisos de Windows: faltan horas al cierre de la jornada, la semana anterior quedó incompleta (el
  primer día laborable de la semana) y hay que reconectar con Jira.
- Preferencias: avisos, hora de cierre de la jornada, resumen semanal, iniciar con Windows (arranca
  oculta en la bandeja) y minimizar a la bandeja.
- Diagnóstico anónimo en Preferencias: un resumen sin claves, títulos, nombres, correos ni URLs, para
  copiarlo en un reporte de problema.
- Instalador para Windows, por usuario, en español y sin permisos de administrador. Al desinstalar se
  conservan la configuración, las preferencias, la caché y el token.

### Pendiente de validar

- La comparación con el Jira real (fase 6) sigue en curso.
- El instalador no tiene firma de código: Windows SmartScreen muestra un aviso al abrirlo.

[Unreleased]: https://github.com/geomark27/signal-aduana/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/geomark27/signal-aduana/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/geomark27/signal-aduana/releases/tag/v0.1.0
