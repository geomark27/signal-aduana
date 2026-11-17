# Signal Aduana de escritorio — Plan de la fase 5

**Estado:** aprobado por el autor el 2026-11-17, con dos decisiones (tabla «Decisiones del autor»):
ejecutores mixtos y resumen semanal en el primer día laborable. En curso: tarea 5.0.

**Punto de partida:** `develop` en `cfb4e5f` (fase 4 completa: tareas 4.1 a 4.10). 135 pruebas de la
librería y 1 de paridad en verde en WSL, en modo simulado. **Nada se ha probado en Windows.**
Contexto y orden de las fases: `docs/ROADMAP.md`.

## Objetivo

Que la app instalada en Windows 10/11 haga todo lo que hoy solo se ha visto en WSL, y dejar atadas
las deudas técnicas conocidas de la fase 4. Se sigue usando el **modo simulado**: el Jira real es la
fase 6. Separarlas permite saber, ante un fallo, si la causa es Windows o es Jira.

## Qué entra y qué no

**Entra:** compilar e instalar en Windows; pasar la lista de comprobación de
`docs/COMPILAR-WINDOWS.md` (sección 7); instalar y desinstalar limpiamente; confirmar los seis
puntos «a verificar» de esa guía; tres tareas de código en WSL (pruebas pendientes y resumen semanal
en el primer día laborable, monitor que resiste un pánico, preferencias que llegan al monitor al
instante); una tarea por cada hallazgo de Windows.

**No entra:** nada contra el Jira real (fase 6); firma de código, actualizaciones y distribución
(fase 7); registro de errores a archivo (7.4). Nota: en `release` la app arranca con
`windows_subsystem = "windows"` y no tiene consola, así que los `eprintln!` actuales no se ven en
Windows; por eso los fallos del monitor en Windows se reconocen por el tooltip, no por el log.

## Reglas

Las de `CLAUDE.md`, que mandan sobre este documento. En lo que toca a esta fase:

- Ninguna tarea toca un servicio real. Ni tokens ni datos reales en logs, archivos, capturas ni bitácora.
- Los cálculos siguen en `src/dominio/`; la interfaz no los reimplementa.
- Una rama = una entrada de bitácora = un dueño (el orquestador la escribe). Cada ejecutor termina su
  reporte con un bloque `DEVLOG CONTRIBUTION` y no toca `docs/bitacora/` (formato en `.claude/skills/bitacora/`).
- Los ejecutores no tocan git, no leen fuera de su carpeta y reportan desviaciones en vez de inventar.
- Dos tareas de la misma ola nunca comparten archivo.
- Los comandos largos (`cargo test` completo, `make check`, levantar la app) los corre el autor.

## Mapa de tareas

La numeración continúa la del roadmap (5.1 a 5.6); los números no implican orden de ejecución.

| Tarea | Qué | Quién | Rama | Ola |
|---|---|---|---|---|
| 5.0 | Documentos de la fase: roadmap, este plan, `delegar.json` y entrada de bitácora | Orquestador | `tarea/5.0-plan-fase-5` | 0 |
| 5.1 | Compilar e instalar en Windows | Autor | sin rama | A |
| 5.2 | Pasar la lista de comprobación en Windows | Autor | sin rama | A |
| 5.3 | Instalar y desinstalar limpiamente | Autor | sin rama | A |
| 5.4 | Pruebas pendientes y resumen semanal en el primer día laborable | Sonnet 5.5 medio | `tarea/5.4-pruebas-avisos` | 1 |
| 5.5 | El monitor sobrevive a un pánico del ciclo | Sonnet 5.5 medio | `tarea/5.5-monitor-resiliente` | 1 |
| 5.6 | Cambios de preferencias llegan al monitor al instante | opencode `*-free` | `tarea/5.6-preferencias-al-instante` | 1 |
| 5.9 | Glosario español → inglés y regla de idioma en `CLAUDE.md` | Orquestador | `tarea/5.9-glosario-ingles` | T |
| 5.10 | Código en inglés: dominio (`src/dominio/` → `src/domain/`) y paridad | Sonnet 5.5 medio | `tarea/5.10-dominio-en-ingles` | T |
| 5.11 | Código en inglés: Jira (`src/jira/`) | Sonnet 5.5 medio | `tarea/5.11-jira-en-ingles` | T |
| 5.12 | Código en inglés: base (error, config, almacén, caché, auth, sesión) | Sonnet 5.5 medio | `tarea/5.12-base-en-ingles` | T |
| 5.13 | Código en inglés: escritorio y comandos (preferencias, avisos, resumen, monitor, vigilancia, escritorio, inicio, comandos, archivos en disco) | Sonnet 5.5 medio | `tarea/5.13-escritorio-en-ingles` | T |
| 5.14 | Código en inglés: interfaz (`ui/app.js`, `id` y clases) | Sonnet 5.5 medio | `tarea/5.14-interfaz-en-ingles` | T |
| 5.7 | Guía de Windows con los datos confirmados y segunda pasada | Orquestador | `tarea/5.7-guia-confirmada` | 2 |
| 5.15 | Instalador como Nudge: solo NSIS por usuario, en español, y desinstalación que conserva lo útil | Sonnet 5.5 medio | `tarea/5.15-instalador-nsis` | 2 |
| 5.16+ | Una tarea por cada hallazgo de Windows | Según el caso | por hallazgo | 2 |
| 5.8 | Cierre de la fase | Orquestador | `tarea/5.8-cierre-fase-5` | 3 |

Las 5.1 a 5.3 no producen código ni rama: el autor reporta los resultados y estos quedan en la
entrada de la 5.7. Las olas 1 y A corren a la vez; no comparten nada.

Las tareas de la ola T (traducción del código al inglés, decisión del autor del 2026-11-17) van
**una detrás de otra**, nunca en paralelo: cada renombre alcanza a los archivos que usan lo
renombrado. Cada una es una rama con su PR y sale con las pruebas en verde por sí sola.
Originalmente los hallazgos de Windows iban a numerarse desde la 5.9; luego en la 5.15 y, con la tarea del instalador, empiezan en la 5.16.

## Orden

```
Ola 0   5.0 documentos de la fase ───────────────────────────────┐
Ola A   5.1 → 5.2 → 5.3 (autor, Windows, sobre develop actual)  ─┤
Ola 1   5.4 ‖ 5.5 ‖ 5.6 (WSL, tres worktrees en paralelo)        ─┤
                                                                  ▼
Ola T   5.9 → 5.10 → 5.11 → 5.12 → 5.13 → 5.14 (código en inglés, en serie; hecha el 2026-11-17)
                                                                  ▼
Ola 2   5.15 instalador, hallazgos 5.16+ y 5.7 (guía + segunda pasada en Windows)
Ola 3   5.8 cierre
```

## Ola T — Código en inglés

**Decisión del autor (2026-11-17):**
- El código pasa al inglés: identificadores, módulos, archivos, comentarios, campos serializados,
  comandos de Tauri, códigos de error y archivos en disco.
- Queda en español lo que ve el usuario de la app y la documentación del repo.
- Los atajos de `make` y los scripts conservan sus nombres.
- La app no está instalada en ningún equipo, así que los archivos en disco se renombran sin
  migración.

**Glosario.** `docs/GLOSARIO.md` es la fuente única de los nombres nuevos y lo escribe el
orquestador en la 5.9, a partir de un inventario del código. Ningún ejecutor inventa un nombre que
el glosario fije. Para los nombres locales que no aparecen ahí, se siguen sus convenciones.

**Reglas comunes a las tareas 5.10 a 5.14:**
- **Ningún texto visible cambia:** mensajes, etiquetas, avisos, tooltips ni valores mostrados. El
  orquestador lo comprueba en el diff con la lista de literales.
- **Pruebas:** el mismo número en verde que antes de la tarea, con los nombres traducidos. Además,
  `cargo clippy --all-targets` sin advertencias, `node --check ui/app.js` y `make ui-format`.
- Los archivos se mueven con `mv`; git detecta el renombre en el commit.
- `pruebas/paridad/legado.js` no se toca.
- Si una tarea cambia un contrato, cambia en el mismo diff `ui/app.js` y los documentos que lo citan
  (por ejemplo, las rutas de `docs/COMPILAR-WINDOWS.md`).

**Convivencia con Windows.**
- Las pruebas manuales 5.1 a 5.3 pueden seguir.
- Desde la 5.13, los archivos en disco cambian de nombre: las preferencias de prueba vuelven a sus
  valores por defecto.
- Las correcciones de código que salgan de Windows esperan a que termine la 5.14.

Cada tarea de la ola T recibe su redacción detallada (alcance exacto y criterios) cuando le toca, a
partir del glosario aprobado.

---

## Tarea 5.0 — Documentos de la fase (orquestador)

- Archivos: `docs/ROADMAP.md` (ya escrito, sin registrar), `docs/PLAN-FASE-5.md` (este),
  `.claude/delegar.json` (la lista `instrucciones` apunta a `PLAN-FASE-4.md`; pasa a `PLAN-FASE-5.md`, y
  lo mismo en `archivos_orquestador`), `docs/bitacora/fase-5/013-plan-fase-5.md` y su fila en el índice.
- Hecha cuando: `bash scripts/verificar-bitacora.sh` sale con 0 y el autor aprueba el diff.

## Vía A — En Windows (la hace el autor)

Sigue `docs/COMPILAR-WINDOWS.md`. Usa el modo simulado (`SIGNAL_SIMULADO=1`); no pongas tus
credenciales reales en esta fase.

**5.1 Compilar e instalar** (secciones 1 a 4 de la guía)
- Compilar con `cargo tauri build` en una carpeta de Windows (no sobre `\\wsl$`).
- Anotar qué instaladores se generan y sus nombres exactos, cuánto tarda la primera compilación y
  cualquier error de compilación o de instalación.
- Probar ambos instaladores (MSI y `-setup.exe`) y anotar diferencias: si pide permisos de
  administrador, dónde se instala, si crea acceso directo. Hipótesis a confirmar, no verificada: el
  de NSIS instala por usuario y el MSI por equipo.

**5.2 Lista de comprobación** (sección 7 de la guía, 14 puntos)
- Marca cada punto. Para cada uno que falle, reporta el formato de hallazgo de abajo.
- Para los avisos, recuerda que el monitor corre a los 30 s de arrancar y luego cada 15 min.

**5.3 Instalar y desinstalar limpiamente** (secciones 8 y 9)
- Verificar dónde quedan realmente la configuración, las preferencias, la caché y la credencial.
- Desinstalar y comprobar qué queda: carpetas de datos, perfil de WebView2, credencial, entrada de
  inicio con Windows.

**Qué se espera de 5.1–5.3:** que resuelvan los seis «a verificar» de la guía (nombre de la descarga
de Build Tools, duración de la primera compilación, rutas y nombres de los instaladores, ruta del
ejecutable instalado, carpeta del perfil de WebView2, entrada de inicio con Windows).

**Formato de hallazgo** (para reportarme cada falla, sin datos reales ni capturas con datos):

```
Punto: <número y texto del punto de la lista, o sección de la guía>
Esperado: <qué debía pasar>
Pasó: <qué pasó, con el mensaje de error exacto si lo hay>
Cómo repetirlo: <pasos>
Instalador: MSI | NSIS | cargo run
```

Cada hallazgo se convierte en una tarea 5.16+ con su prueba cuando se pueda automatizar.

## Ola 1 — Tres tareas en WSL

Tres worktrees desde `develop` al día, con la 5.0 ya fusionada: `.worktrees/5.4`, `.worktrees/5.5`,
`.worktrees/5.6`. La 5.4 y la 5.5 van a subagentes Sonnet 5.5 (esfuerzo medio), porque cambian la
lógica de avisos y la concurrencia del monitor. La 5.6 es una función pura y una llamada en un
comando, y va a opencode con un modelo `opencode/*-free` (solo datos ficticios); el orquestador
confirma el modelo exacto antes de lanzarla.
Cada ejecutor verifica con `cargo test`, `cargo clippy --all-targets` (y `node --check ui/app.js` si
tocara la interfaz; aquí no se toca) y termina con el bloque `DEVLOG CONTRIBUTION`.

### Tarea 5.4 — Pruebas pendientes y resumen semanal en el primer día laborable

**Objetivo.** Dos partes:
- **Cambio de lógica (decisión del autor 1).** El resumen semanal sale el **primer día laborable de
  la semana**, desde las 09:00. Hoy `decidir` solo comprueba si el día es lunes (`src/avisos.rs`,
  condición `weekday() == Weekday::Mon`). Por eso el aviso llega aunque el lunes sea feriado y, si
  ese día la PC está apagada, se pierde para esa semana.
- **Pruebas.** Cubrir lo que la entrada 001 dejó sin probar. Si una prueba ajena al cambio de lógica
  demuestra un defecto, **no se corrige**: la prueba queda con `#[ignore = "motivo"]` y se reporta.

**Archivos que posee:** `src/avisos.rs` y `src/resumen.rs` (código y módulos `pruebas`), y el módulo
`pruebas` de `src/jira/mod.rs` (si no existe, créalo al final del archivo).

**Cambio de lógica.**
1. `Resumen` gana el campo `primer_laborable: Option<String>` (`AAAA-MM-DD`). En `resumir` se calcula
   con `dominio::calendario::dias_laborables(<lunes de la semana actual>, feriados)` y se toma el
   primero. No se reimplementa la regla de feriados. En `FeriadosExtra`, `true` agrega un feriado y
   `false` quita uno base (`es_feriado`).
2. En `decidir`, la condición del resumen semanal deja de ser `ahora.date().weekday() == Weekday::Mon`
   y pasa a ser `r.primer_laborable.as_deref() == Some(hoy.as_str())`. Lo demás no cambia: desde las
   09:00, `prefs.avisos.resumen_lunes`, `anterior_meta_h > 0`, semana anterior incompleta y
   `ya_enviado`.
3. Si toda la semana es feriado, `primer_laborable` es `None` y no sale el resumen.
4. Por compatibilidad no se renombran `TipoAviso::ResumenLunes`, `Registro.lunes` (está en un JSON
   en disco) ni `prefs.avisos.resumen_lunes`. El renombre queda en Pendientes.
5. El campo se serializa como `primerLaborable`. La interfaz no lo usa y `ui/` no se toca.
6. Las estructuras `Resumen` escritas a mano en las pruebas existentes de `avisos.rs` y `resumen.rs`
   reciben el campo nuevo. Las de `avisos.rs` llevan `Some` del lunes de la fecha que usan, para que
   esas pruebas no cambien de significado.

**Pruebas a agregar** (nombres sugeridos; los valores son los que se comprueban). Fechas fijas: el
2026-11-16 es lunes, el 2026-11-17 martes y el 2026-11-13 viernes.

En `avisos.rs`:
1. `cierre_con_hora_invalida_no_avisa_ni_entra_en_panico`: `Preferencias` armadas a mano con
   `hora_cierre = "99:99"`.
   - Viernes 17:00 con la jornada incompleta → lista vacía.
   - Lunes 10:00 con `primer_laborable = Some("2026-11-16")` y la semana anterior incompleta →
     exactamente un aviso `ResumenLunes`.
2. `resumen_enviado_la_semana_pasada_vuelve_a_avisar`: `registro.lunes = Some("2026-11-09")`, ahora
   2026-11-16 10:00, `primer_laborable = Some("2026-11-16")` y la semana anterior incompleta → un
   aviso `ResumenLunes`. Tras `marcar`, el registro guarda `"2026-11-16"`.
3. `lunes_feriado_no_envia_resumen`: `hoy_laborable = false`, `primer_laborable = Some("2026-11-17")`,
   la semana anterior incompleta, lunes 2026-11-16 10:00 → ni `ResumenLunes` ni `CierreJornada`.
4. `resumen_sale_el_martes_si_el_lunes_fue_feriado`: `primer_laborable = Some("2026-11-17")`, la
   semana anterior incompleta, martes 2026-11-17 10:00 → exactamente un `ResumenLunes`. A las 08:59
   del mismo día → ninguno.
5. `martes_normal_no_envia_resumen`: `primer_laborable = Some("2026-11-16")`, la semana anterior
   incompleta, martes 2026-11-17 10:00 → ningún `ResumenLunes`.
6. `semana_toda_feriado_no_envia_resumen`: `primer_laborable = None`, lunes 10:00 → ningún
   `ResumenLunes`.

En `resumen.rs`:
7. `resumir_marca_feriado_base_y_extra`: con `resumir` y tareas simuladas:
   - `hoy = 2026-11-03` (feriado base) → `hoy_laborable == false` y `semana_meta_h == 24`.
   - Con `feriados_extra = {"2026-11-03": false}`, que quita ese feriado → `hoy_laborable == true` y
     `semana_meta_h == 32`.
   - `hoy = 2026-11-14` (sábado) → `hoy_laborable == false`.
8. `resumir_calcula_primer_laborable`:
   - `hoy = 2026-11-18` sin feriados extra → `Some("2026-11-16")`.
   - Con `{"2026-11-16": true}` → `Some("2026-11-17")`.
   - `hoy = 2026-11-04`: el 2 y el 3 de noviembre son feriados base → `Some("2026-11-04")`.
   - Con los cinco días del 2026-11-16 al 2026-11-20 en `true` → `None`.

En `jira/mod.rs`, con `JiraSimulado::nuevo("a@b.com", true, 2026-11-13)`:
9. `cargar_para_resumen_devuelve_tareas_con_horas_propias`: la lista no está vacía; cada elemento
   tiene `key` y `worklogs` (un arreglo); ningún elemento tiene `hasPR`; ningún registro es de
   `sim-0002` (lo comprueba la tarea simulada APP-401); ningún registro es anterior al lunes de
   (hoy − 98 días).
10. `cargar_para_resumen_con_token_invalido_es_no_autorizado`: `token_valido = false` →
    `Err(NoAutorizado)`.
11. Si alguna tarea simulada tiene `timespent` 0, comprobar que `spent` es la suma de sus registros.
    Si ninguna lo tiene, se omite y se anota en Pendientes.

**Descartado por diseño:** «un `Resumen` de otro día junto con `NoAutorizado`» (entrada 001). El tipo
`EstadoMonitor` no permite combinarlos. Se anota en la bitácora, sin prueba.

**Hecha cuando:**
- `cargo test` en verde con al menos 10 pruebas nuevas.
- `cargo clippy --all-targets` sin advertencias.
- Fuera de los módulos `pruebas`, solo cambian `src/avisos.rs` y `src/resumen.rs`.
- `ui/` intacto.

### Tarea 5.5 — El monitor sobrevive a un pánico del ciclo

**Objetivo.** Hoy, si `monitor::ciclo` entrara en pánico, la tarea del bucle terminaría y el monitor
dejaría de funcionar sin avisar (entrada 007). El bucle debe registrar el fallo, mostrarlo en el
tooltip y seguir con el ciclo siguiente.

**Archivos que posee:** `src/vigilancia.rs`, `src/monitor.rs`.

**Qué hacer.**
1. Un pánico dentro de `ciclo` (incluido uno dentro de `Salida::tooltip` o `Salida::notificar`) no
   termina el bucle. Sugerencia: `futures::FutureExt::catch_unwind` con `AssertUnwindSafe`
   (`futures` ya es dependencia). Si eliges otra técnica, dilo en el reporte.
2. Ante un pánico: `eprintln!("vigilancia: el ciclo entró en pánico: <mensaje>")` con el mensaje del
   pánico recortado a 200 caracteres (sin datos de tareas ni de cuenta), tooltip
   `Signal Aduana · Error interno, se reintenta` (con ` (simulado)` al final en modo simulado, y con
   el mismo recorte a `MAX_TEXTO_BANDEJA` que los demás), y el siguiente ciclo ocurre al cabo de `INTERVALO`.
3. El bucle debe poder probarse sin Tauri: extráelo a una función que reciba `&Estado`, `&dyn Salida`,
   `&Notify`, la espera inicial, el intervalo y un tope opcional de iteraciones para pruebas.
   `iniciar(app)` sigue siendo el único punto que conoce `AppHandle`.
4. No cambies los valores de `ESPERA_INICIAL` (30 s) ni `INTERVALO` (15 min) ni la API pública que usan
   `escritorio.rs`, `inicio.rs` y `comandos.rs` (`pedir_ciclo`, `Vigilante`, `SalidaTauri`).

**Pruebas a agregar.**
- Un futuro que entra en pánico produce `Err` con el texto del pánico; uno normal produce `Ok`.
- Bucle con una `Salida` falsa cuyo primer `tooltip` entra en pánico y los siguientes funcionan, con
  esperas de milisegundos y tope de 2 iteraciones: el segundo ciclo llega a fijar su tooltip.
- El texto del tooltip de error lleva el sufijo de simulado cuando corresponde y no pasa de
  `MAX_TEXTO_BANDEJA` caracteres.

**Hecha cuando:** `cargo test` en verde con al menos 3 pruebas nuevas, `cargo clippy --all-targets`
sin advertencias, y los tests existentes de `vigilancia.rs` y `monitor.rs` siguen pasando sin cambios
de significado.

### Tarea 5.6 — Cambios de preferencias llegan al monitor al instante

**Objetivo.** Hoy, cambiar la hora de cierre o un feriado surte efecto en el siguiente ciclo, hasta
15 minutos después (entrada 007). Debe pedirse un ciclo al guardar.

**Archivos que posee:** `src/comandos.rs`, `src/preferencias.rs`.

**Qué hacer.**
1. En `preferencias.rs`: método puro `afecta_al_monitor(&self, otra: &Preferencias) -> bool` que es
   `true` si cambia cualquier campo de `avisos` o `feriados_extra`, y `false` si solo cambian
   `iniciar_con_windows` o `minimizar_a_bandeja`. Agrega `PartialEq` donde haga falta.
2. En `guardar_preferencias` (`comandos.rs`): leer las preferencias anteriores antes de guardar; tras
   guardar y aplicar el inicio con Windows, si `anteriores.afecta_al_monitor(&guardadas)` llamar a
   `vigilancia::pedir_ciclo(&app)`. Si guardar falla, no se pide nada. La primera vez (sin archivo) las
   anteriores son las de por defecto.
3. No toques `vigilancia.rs` (lo posee la 5.5): usa `pedir_ciclo` tal como está.

**Pruebas a agregar** (en `preferencias.rs`): `afecta_al_monitor` es `false` para preferencias iguales;
`true` al cambiar `hora_cierre`, `resumen_lunes`, `activos`, al agregar un feriado y al quitarlo;
`false` al cambiar solo `iniciar_con_windows` y `false` al cambiar solo `minimizar_a_bandeja`.
El comando en sí necesita `AppHandle` y no tiene prueba automática: se comprueba en Windows (5.7).

**Pendiente que se asume:** cada cambio de feriado dispara un ciclo completo (una carga de tareas y
horas). Con datos simulados no importa; con el Jira real (fase 6) se evalúa si hace falta esperar unos
segundos tras el último cambio. Se anota en Pendientes de la entrada.

**Hecha cuando:** `cargo test` en verde con al menos 6 pruebas nuevas, `cargo clippy --all-targets`
sin advertencias, y `ui/` intacto.

## Ola 2 — Hallazgos y guía

**5.7 Guía de Windows con los datos confirmados** (orquestador, `docs/COMPILAR-WINDOWS.md`)
- Reemplaza cada «a verificar» por lo que se confirmó en 5.1 a 5.3, o lo deja marcado con el motivo si
  no se pudo confirmar.
- Agrega la **segunda pasada** a la lista de comprobación, para hacerla con las tareas 5.5 y 5.6 ya
  fusionadas:
  - (a) Cambiar la hora de cierre a una que ya pasó, con horas faltantes: el aviso llega sin esperar
    15 minutos.
  - (b) Marcar un feriado: el tooltip se actualiza en segundos.
  - (c) Con la 5.4 fusionada: marcar como feriado el lunes de la semana en curso, con la semana
    anterior incompleta. Si hoy es lunes, el resumen semanal no sale. Si hoy es martes, desde las
    09:00, sí sale.
  - (d) Con la ola T fusionada (código en inglés), todos los controles de la interfaz responden:
    preferencias, conexión, feriados del calendario, filtros por tablero, vistas y paginación de la
    tabla. Los `id` cambiaron de nombre en la 5.14.
  - (e) Conectar con el token «rechazar»: el mensaje de error sale sin el código al final
    (decisión del autor, 5.12).
- **«Iniciar con Windows» en modo simulado.** En modo simulado no se registra en el sistema, a
  propósito (regla de `CLAUDE.md`). El punto de la lista de comprobación y el «a verificar» del
  nombre de la entrada de inicio se marcan como «no aplica en simulado» y pasan a la fase 6.
- La guía también recoge lo que se aprendió al compilar: para copiar el proyecto se usa
  `robocopy … /MIR`, que borra en el destino los archivos renombrados por la ola T y deja intacta la
  carpeta `target` excluida.
- Se actualiza `docs/ROADMAP.md` con la ola T (5.9 a 5.14) dentro de la fase 5.
- La entrada de bitácora recoge los resultados de 5.1 a 5.3 (hechos, no intenciones).

**5.15 Instalador como Nudge** (decisiones del autor 2 y 5).
- `tauri.conf.json`:
  - `bundle.targets` pasa a `["nsis"]`;
  - `bundle.windows.nsis` lleva `installMode: "currentUser"`, el idioma español y
    `installerHooks: "windows/hooks.nsh"`.
- `windows/hooks.nsh` define `NSIS_HOOK_POSTUNINSTALL`. Solo cuando **no** es una actualización
  (`$UpdateMode <> 1`), borra:
  - `notifications*.json` en `%LOCALAPPDATA%\<identificador>`;
  - la carpeta `EBWebView` (perfil de WebView2);
  - el valor de `StartupApproved\Run`.
- La entrada `Run` ya la borra la plantilla de Tauri.
- No se tocan la configuración, las preferencias, la caché ni la credencial.
- La casilla «borrar datos» que trae Tauri se mantiene: si la persona la marca, Tauri borra todo
  menos la credencial.
- `docs/COMPILAR-WINDOWS.md`:
  - solo el `.exe`;
  - dónde se instala;
  - qué queda al desinstalar y cómo borrarlo a mano.
- Solo se comprueba de verdad en Windows. Lo hace el autor.

**Hallazgos 5.16+:** una tarea por hallazgo, con su plan breve (objetivo, archivos, criterio) escrito
por el orquestador cuando llegue el reporte, y aprobado por el autor antes de lanzarla. Los hallazgos
de la interfaz (`ui/`) y los de Rust no comparten tarea.

## Ola 3 — Cierre

**5.8 Cierre de la fase 5** (orquestador):
- la sección «Estado al cierre» en este documento;
- el índice de la bitácora;
- el estado en `docs/ROADMAP.md`;
- la verificación con `bash scripts/verificar-bitacora.sh`.

Incluye también la entrada de bitácora de la **5.4**, que se fusionó sin ella (PR #16): se escribe
como entrada retroactiva, con la fecha de la fusión y lo que consta en el commit y en el plan, como
se hizo con las entradas 001 y 002.

## Hecha la fase cuando

1. Los 14 puntos de la lista de comprobación y los cinco de la segunda pasada pasan en Windows, o cada
   uno que falla tiene su tarea 5.16+. Los que no aplican en simulado quedan anotados para la fase 6.
2. Los seis «a verificar» de la guía están confirmados o justificados.
3. Instalar y desinstalar no deja credenciales, entradas de inicio ni carpetas inesperadas, o lo que
   deja está documentado en la guía.
4. 5.4, 5.5 y 5.6 fusionadas en `develop`, con `cargo test` y `cargo clippy --all-targets` en verde.
5. Cada tarea tiene su entrada de bitácora y `bash scripts/verificar-bitacora.sh` sale con 0.

## Decisiones del autor

| # | Pregunta | Decisión |
|---|---|---|
| 1 | Si el lunes es feriado y la semana anterior quedó incompleta, ¿se envía el resumen del lunes? | **Decidido (2026-11-17): el resumen sale el primer día laborable de la semana**, desde las 09:00. Se descartaron dos opciones: dejarlo como estaba (avisa en el día libre y se pierde si ese lunes la PC está apagada) y omitirlo en los lunes feriados (esa semana no hay resumen). Lo implementa la 5.4 |
| 2 | ¿Qué instalador se entrega? | **Decidido (2026-11-17): solo el `.exe` de NSIS, como en Nudge** (otra app de escritorio del autor): instalación por usuario y sin permisos de administrador. Se quita el MSI. Lo implementa la 5.15 |
| 5 | ¿Qué conserva la desinstalación? | **Decidido (2026-11-17):** se conservan la configuración (sitio y correo), las preferencias, el token de Jira y la caché de tareas, para reinstalar sin perder nada. Se borran siempre el registro de avisos, el perfil de WebView2 y la entrada de «Iniciar con Windows». Lo implementa la 5.15 |
| 3 | ¿Nombres de ramas y numeración? | Los de la tabla de arriba (aprobado) |
| 4 | ¿Qué ejecutores? | **Decidido (2026-11-17): mixto.** Sonnet 5.5 medio para la 5.4 y la 5.5; opencode `*-free` para la 5.6 |

## Riesgos

| Riesgo | Efecto | Mitigación |
|---|---|---|
| Fallos de compilación o de dependencias en Windows (por ejemplo del inicio automático o del almacén de credenciales) | Bloquea 5.1 | Reportarlos con el formato de hallazgo; es el motivo de hacer esta fase aparte |
| Sin consola en `release`, los fallos del monitor no se ven | Diagnóstico más lento en Windows | Tooltip de error (5.5); registro a archivo en la 7.4 |
| Los avisos tardan hasta 15 minutos en aparecer | Se da por fallido un aviso que solo estaba esperando | La guía lo advierte; la 5.6 lo reduce para los cambios de preferencias |
| Ola 1 y Vía A con versiones distintas de `develop` | Se prueba algo que luego cambia | La segunda pasada de 5.7 repite solo lo que 5.5 y 5.6 tocan |

## Estado al cierre

(Se completa en la tarea 5.8.)
