# 022 — Guía de Windows al día y segunda pasada

**Fecha:** 2026-11-17
**Rama:** tarea/5.7-guia-confirmada → develop
**Fase:** 5
**Ejecutor:** orquestador (Opus 5.5), una vuelta.

## 1. Solicitud

Tarea 5.7 de `docs/PLAN-FASE-5.md`. Pide:
- dejar la guía de Windows al día con lo que se confirmó en las tareas 5.1 a 5.3;
- agregar la segunda pasada de comprobaciones;
- incluir en el plan y en el roadmap lo que surgió durante la fase.

El autor pidió además cerrar la fase con lo que hay y atender las sorpresas de Windows más adelante,
en tareas propias.

## 2. Motivación

| Problema | Daño real |
|---|---|
| La guía no incluía lo que cambió en la fase 5: avisos al instante, resumen semanal en el primer día laborable, `id` renombrados en la ola T, errores sin código, `--hidden` | Una prueba en Windows habría dado por buenos comportamientos viejos o por fallos los nuevos |
| La lista pedía probar «Iniciar con Windows» en modo simulado, donde a propósito no se registra | Habría salido un falso fallo |
| Copiar con `robocopy /E` deja en el destino los archivos que la ola T renombró | Restos como `src/dominio/` o `tests/paridad.rs` en la carpeta de compilación de Windows |
| El roadmap no mencionaba la ola T | El roadmap no reflejaba lo hecho en la fase |

## 3. Diseño técnico

- **Resultados de Windows (5.1 a 5.3).**
  - El autor compiló en Windows, instaló la app y reportó que quedó funcional.
  - No se registraron los datos punto por punto: los seis «a verificar», la lista de 14 puntos, la
    segunda pasada, ni qué queda al desinstalar.
  - Por decisión del autor, la fase se cierra sin esos datos. Las sorpresas se atienden después, como
    tareas 5.15 o posteriores, o en fases siguientes.
  - Por eso **los «a verificar» de la guía se mantienen**. Este documento no afirma ningún dato que
    no se haya medido.
- **Guía (`docs/COMPILAR-WINDOWS.md`).**
  - `robocopy … /MIR`, con la explicación: borra en el destino lo renombrado y conserva `target`
    excluido. El código de salida 1 significa éxito.
  - Nueva sección **«Segunda pasada»**, con cinco puntos para marcar: hora de cierre al instante,
    feriado al instante, resumen semanal en el primer día laborable, que respondan todos los controles
    de la interfaz y el mensaje de error sin código.
  - «Iniciar con Windows» y el nombre de su entrada en el registro se prueban **solo en modo real**,
    en la fase 6.
  - «Resumen del lunes» pasa a ser «resumen semanal (primer día laborable)».
- **Plan.**
  - La segunda pasada, de (a) a (e).
  - El caso del inicio con Windows en modo simulado.
  - La entrada retroactiva de la 5.4, dentro de la 5.8.
  - Los criterios de fase hecha incluyen la segunda pasada.
- **Roadmap.** La ola T (5.9 a 5.14) dentro de la fase 5, `--hidden` y la ruta `src/domain/`.
- **Decisión del autor 2 (instalador).** Solo el `.exe` de NSIS, instalado por usuario, como en
  Nudge, otra app de escritorio Tauri del autor. El orquestador revisó su configuración: NSIS por
  usuario, release por GitHub Actions con nombre fijo, `install.ps1` de una línea y actualizador de
  Tauri.
  - Lo que entra ahora: NSIS por usuario, en la tarea **5.15**.
  - Lo que se deja para la fase 7, como referencia en el roadmap (7.2 y 7.3): la distribución y el
    actualizador, porque dependen de Tecnología y de la visibilidad del repositorio.
- **Decisión del autor 5 (desinstalación).**
  - Se conservan la configuración, las preferencias, el token y la caché, para reinstalar sin perder
    nada.
  - Se borran siempre el registro de avisos, el perfil de WebView2 y la entrada de inicio con
    Windows.
  - Antes de proponerlo se revisó la plantilla NSIS de Tauri 2.10.5:
    - ya borra el valor `HKCU\…\Run\<producto>` cuando no es una actualización;
    - ofrece el hook `NSIS_HOOK_POSTUNINSTALL`;
    - distingue las actualizaciones con `$UpdateMode`.

    El resto lo implementa la 5.15.
- **Numeración.** Los hallazgos de Windows pasan a numerarse desde la 5.16.

## 4. Archivos

- `docs/COMPILAR-WINDOWS.md`: `robocopy /MIR`, la segunda pasada, el inicio con Windows solo en
  modo real y el resumen semanal.
- `docs/PLAN-FASE-5.md`: las secciones de la 5.7 y la 5.8 y los criterios de fase hecha.
- `docs/ROADMAP.md`: la ola T, `--hidden` y `src/domain/`.
- `docs/bitacora/README.md` (+1 fila) y esta entrada.

## 5. Cómo verificar

Corrido por el orquestador en esta rama:

```bash
bash scripts/verificar-bitacora.sh
```

Solo cambian documentos: no aplica `cargo test`. Los comandos de la guía no se ejecutaron en Windows
desde esta sesión.

## 6. Pendientes

- Los seis «a verificar» de la guía:
  - la descarga de Build Tools;
  - la duración de la primera compilación;
  - las rutas y nombres de los instaladores;
  - la ruta del ejecutable instalado;
  - la carpeta del perfil de WebView2;
  - el nombre de la entrada de inicio (este solo en modo real).
- La lista de 14 puntos y la segunda pasada, marcadas punto por punto en Windows.
- Qué queda después de desinstalar (5.3). Se vuelve a comprobar con el desinstalador de la 5.15.
- La 5.15 (instalador como Nudge), que implementa las decisiones 2 y 5.
