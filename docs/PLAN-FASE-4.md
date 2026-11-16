# Signal Aduana de escritorio — Plan de la fase 4

**Punto de partida:** fases 1 a 3 completas (`docs/PLAN-FASE-1.md` a `PLAN-FASE-3.md`): la app funciona
en modo simulado con caché, actualización automática y fuentes locales; los cálculos viven en
`src/dominio/` y la interfaz solo dibuja; 91 pruebas en verde (incluida la paridad con 31 casos).

**Objetivo de la fase 4: que la app viva en la bandeja de Windows y avise.**

1. **Bandeja del sistema**: ícono con resumen en el tooltip («Semana: 24 de 40 h · Hoy: 3 de 8 h») y menú
   (Abrir, Actualizar ahora, Salir). Cerrar la ventana la esconde en la bandeja en vez de salir.
2. **Una sola instancia**: abrir la app de nuevo trae al frente la ventana que ya existe.
3. **Avisos de Windows**: al cierre de la jornada si faltan horas hoy; el lunes, si la semana pasada quedó
   incompleta; y si el token dejó de servir.
4. **Preferencias** guardadas en el equipo (no en el navegador): avisos, hora de cierre, iniciar con
   Windows, minimizar a la bandeja y los feriados marcados por el usuario.
5. **Guía para compilar el instalador en Windows** (no se puede compilar ni probar desde WSL).

Fuera de alcance: firma de código del instalador, actualizaciones automáticas de la app, prueba con el
Jira real (la hace el autor).

## Estado al cierre (2026-11-16)

La fase se ejecutó con un reparto distinto del de la tabla de abajo: la tarea 3 se dividió en cuatro y
se sumaron tareas de normas y de cierre. El flujo de ramas pasó a `develop → tarea/N.M`, sin rama de
fase (ver `CLAUDE.md`). El detalle de cada tarea, sus decisiones y lo que no se probó está en su
entrada de `docs/bitacora/fase-4/`; el índice `docs/bitacora/README.md` es la fuente de verdad.

| Tarea | Qué quedó hecho | Entrada |
|---|---|---|
| 4.1 | Preferencias, resumen y avisos en Rust (tarea 1 del plan) | 001 |
| 4.2 | Interfaz de preferencias (tarea 2 del plan) | 002 |
| 4.0 | Normas, bitácora y flujo de ramas (no estaba en el plan) | 003, 004 |
| 4.3 | Monitor: ciclo de lectura, tooltip y avisos sin Tauri (tarea 3) | 005 |
| 4.4 | Bandeja, cierre a la bandeja, una instancia y arranque oculto (tarea 3) | 006 |
| 4.5 | Integración: monitor en marcha, notificaciones, inicio con Windows y ganchos de sesión (tarea 3) | 007 |
| 4.6 | `docs/COMPILAR-WINDOWS.md` (tarea 3, punto 4) | 008 |
| 4.7 | Este cierre, el arreglo del índice de la bitácora y el `Makefile` con los atajos del flujo (`make` lista todo) | 009 |
| 4.8 | Aviso en la interfaz si falla «Iniciar con Windows» | 010 |
| 4.9 | `scripts/verificar-bitacora.sh` | 011 |
| 4.10 | Formato oficial de `ui/app.js` con Prettier (`make fmt-ui`, `make ui-format`) | 012 |

**Verificación.** Todo se hizo y se probó en WSL, en modo simulado. Última cuenta registrada: 135
pruebas de la librería y 1 de paridad (reporte de la 4.5); el autor repite `cargo test`,
`cargo clippy --all-targets` y `cargo build` al cerrar cada tarea.

**No verificado todavía** (solo se puede en Windows; la lista de comprobación está en
`docs/COMPILAR-WINDOWS.md`): ícono, menú y tooltip de la bandeja, cerrar a la bandeja con y sin la
preferencia, segunda instancia, arranque con `--oculto`, registro real del inicio con Windows, los tres
avisos de Windows, la migración de los feriados guardados en el navegador y el aspecto del panel de
preferencias (la ventana nunca se vio funcionando).

**Pendientes que quedan abiertos:**

- Seguir la guía en un equipo Windows y resolver sus seis puntos «a verificar».
- Falta una prueba directa de `cargar_para_resumen` y de algunos casos de `decidir` (hora de cierre
  inválida, lunes con fecha de una semana anterior, lunes feriado); ver la entrada 001.
- Los cambios de preferencias (hora de cierre, feriados) llegan al monitor en el siguiente ciclo, hasta
  15 minutos después.
- Si el ciclo del monitor entrara en pánico, su tarea terminaría sin reanudarse (entrada 007).
- La publicación `develop → main` (PR con merge commit) y el número de versión los decide el autor.

## Reglas para todos los agentes

Las 8 reglas de `docs/PLAN-FASE-1.md` (léelas allí): solo tus archivos; ninguna conexión a servicios
reales (todo simulado; nunca `SIGNAL_SIMULADO=0`); sin tokens en logs ni archivos; sin commits;
`export PATH="$HOME/.cargo/bin:$PATH"`; todo en español; reporta desviaciones en vez de inventar;
respuesta final breve con archivos, verificaciones reales y pendientes.

**Regla nueva — sin efectos en el sistema durante el desarrollo:** en modo simulado la app **no**
registra el inicio automático con el sistema operativo (solo lo informa en el log) y los avisos
llevan el prefijo «[Simulado]». No dejes procesos de la app corriendo al terminar (usa `timeout`).

| Tarea | Puede tocar | Cuándo |
|---|---|---|
| 1 | `src/preferencias.rs`, `src/avisos.rs`, `src/resumen.rs` (nuevos), `src/jira/mod.rs`, `examples/volcar_simulado.rs`, `src/comandos.rs`, `src/lib.rs` (solo `mod` y registrar comandos), `src/sesion.rs` (solo si hace falta) | Primera ola |
| 2 | `ui/app.js`, `ui/index.html`, `ui/styles.css` | Primera ola, en paralelo con la 1 |
| 3 | `src/escritorio.rs` (nuevo), `src/lib.rs`, `src/main.rs`, `Cargo.toml`, `tauri.conf.json`, `capabilities/`, `docs/COMPILAR-WINDOWS.md` (nuevo) | Segunda ola, después de la 1 y la 2. **Hecha como las tareas 4.3 a 4.6** (ver «Estado al cierre») |

---

## Contrato E — preferencias y eventos (Rust ↔ JS)

```ts
Preferencias = {
  avisos: {
    activos: boolean,          // por defecto true
    horaCierre: "HH:MM",       // por defecto "16:30"; hora de Ecuador; 00:00–23:59
    resumenLunes: boolean      // por defecto true
  },
  iniciarConWindows: boolean,  // por defecto false
  minimizarABandeja: boolean,  // por defecto true
  feriadosExtra: { [ymd: string]: boolean }   // mismas reglas que hoy en app.js
}
```

| Comando | Argumentos (JS) | Devuelve |
|---|---|---|
| `leer_preferencias` | — | `Preferencias` (con valores por defecto si no hay archivo o está dañado) |
| `guardar_preferencias` | `{ preferencias }` | `Preferencias` normalizadas tal como quedaron guardadas |

- No requieren sesión. Archivo `preferencias.toml` (`preferencias.simulado.toml` en simulado) en el
  directorio de configuración, escritura atómica como `config.rs`.
- Validación: `horaCierre` debe ser `HH:MM` válido; las claves de `feriadosExtra` deben ser fechas
  `AAAA-MM-DD` válidas (se descartan las inválidas). Hora inválida → `ErrorApp::Formato`.
- Las preferencias **no** se borran al cerrar sesión (son del equipo, no de la cuenta).

**Eventos de Rust a JS** (`window.__TAURI__.event.listen`):

| Evento | Cuándo | Qué hace la interfaz |
|---|---|---|
| `signal://actualizar` | El usuario eligió «Actualizar ahora» en la bandeja | `refresh(true)` si hay sesión |
| `signal://preferencias` | Las preferencias cambiaron desde Rust (por ejemplo, el inicio con Windows falló y quedó en false) | Vuelve a leerlas y repinta el panel y el tablero |

**Comando de Rust para la tarea 3** (lo usa el monitor, no la interfaz): ver «Contrato F».

## Contrato F — núcleo del monitor (Rust, para la tarea 3)

```rust
// src/jira/mod.rs — lo que hoy hace la interfaz al cargar (y el ejemplo volcar_simulado):
// cargar_tareas + claves_con_horas(lunes de hoy − 98 días) + leer_horas (con el instante
// 00:00 de ese lunes en UTC−5), reemplazando worklogs/wlTotal y poniendo spent = suma si spent es 0.
// Sin documentos ni PR. Devuelve las tareas en el formato de entrada de `calcular_vista`.
pub async fn cargar_para_resumen(f: &Fuente, yo: Option<&str>, hoy: NaiveDate)
    -> Resultado<Vec<serde_json::Map<String, Value>>>;
// El ejemplo volcar_simulado debe pasar a usar esta función (y seguir produciendo lo mismo).

// src/resumen.rs
pub struct Resumen {            // Serialize, camelCase
    pub hoy: String,            // AAAA-MM-DD
    pub semana_secs: i64, pub semana_meta_h: i64,
    pub hoy_secs: i64, pub hoy_laborable: bool,
    pub anterior_secs: i64, pub anterior_meta_h: i64,   // semana anterior completa
}
pub fn resumir(tareas: Vec<Map<String, Value>>, hoy: NaiveDate, feriados: &FeriadosExtra) -> Resultado<Resumen>;
// Usa dominio::calcular_vista (semana actual y anterior, tablero "Todos"): no reimplementa reglas.
pub fn texto_bandeja(r: Option<&Resumen>, simulado: bool) -> String;
// Ej.: "Signal Aduana · Semana: 24 de 40 h · Hoy: 3 de 8 h"; sin resumen: "Signal Aduana · Sin datos";
// en simulado agrega " (simulado)". Usa el formato de horas `fh` del dominio. Máx. 120 caracteres.

// src/avisos.rs — decisión pura, sin reloj ni Tauri
pub enum TipoAviso { CierreJornada, ResumenLunes, Reconectar }
pub struct Aviso { pub tipo: TipoAviso, pub titulo: String, pub cuerpo: String }
#[derive(Default, Serialize, Deserialize)]
pub struct Registro { pub cierre: Option<String>, pub lunes: Option<String>, pub reconectar: Option<String> } // última fecha enviada
pub enum EstadoMonitor<'a> { Resumen(&'a Resumen), NoAutorizado, SinSesion, Error }
pub fn decidir(estado: EstadoMonitor, ahora: NaiveDateTime /* hora de Ecuador */, prefs: &Preferencias,
               registro: &Registro, simulado: bool) -> Vec<Aviso>;
pub fn marcar(registro: &mut Registro, avisos: &[Aviso], hoy: NaiveDate);
pub fn cargar_registro(dir: &Path, simulado: bool) -> Registro;          // avisos.json / avisos.simulado.json en dir_datos; dañado → Default
pub fn guardar_registro(dir: &Path, simulado: bool, r: &Registro) -> Resultado<()>;
```

Reglas de `decidir` (con `prefs.avisos.activos == false` no se envía nada):
- **CierreJornada**: hoy es laborable, la hora ≥ `horaCierre`, `hoy_secs` < 8 h y no se envió hoy.
  Título «Te faltan X h hoy»; cuerpo «Llevas Y h de 8 h. Registra tus horas en Jira antes de cerrar el día.»
- **ResumenLunes**: hoy es lunes, hora ≥ 09:00, `resumenLunes` activo, `anterior_meta_h` > 0,
  `anterior_secs` < meta, y no se envió esta semana. Título «La semana pasada faltaron X h»;
  cuerpo «Registraste Y h de Z h. Si trabajaste esas horas, regístralas en Jira con su fecha real.»
- **Reconectar**: estado `NoAutorizado` y no se envió hoy. Título «Vuelve a conectar Signal Aduana»;
  cuerpo «Tu token de Jira venció o fue revocado. Abre la app y conéctate de nuevo.»
- `SinSesion` y `Error` no generan avisos. En simulado, los títulos llevan el prefijo «[Simulado] ».
- Horas en los textos con el formato `fh` del dominio (`7.5 h`, `8 h`).

---

## Tarea 1 — Preferencias, resumen y avisos (Rust)

1. `src/preferencias.rs`: tipo `Preferencias` (Serialize/Deserialize camelCase, `Default` con los
   valores del contrato), `cargar(dir, simulado)`, `guardar(dir, simulado, &Preferencias)`, `normalizar`.
   El tipo de `feriadosExtra` debe ser el mismo `FeriadosExtra` que ya usa `src/dominio/` (reúsalo).
2. Comandos `leer_preferencias` y `guardar_preferencias` en `comandos.rs`, registrados en `lib.rs`.
   `guardar_preferencias` solo guarda: el efecto sobre el inicio con Windows lo agrega la tarea 3.
3. `cargar_para_resumen` en `src/jira/mod.rs` y el ejemplo `volcar_simulado` usándola (regenera un volcado
   en un archivo temporal y compruébalo con `cmp` contra `pruebas/paridad/volcados/viernes.json`: debe ser idéntico).
4. `src/resumen.rs` y `src/avisos.rs` según el contrato F.
5. Pruebas: preferencias (defectos, ida y vuelta, archivo dañado, hora inválida, feriado inválido
   descartado); `resumir` con el simulado para un viernes y un lunes fijos (compara con lo que da
   `calcular_vista` directamente); `texto_bandeja` (con y sin resumen, simulado, largo máximo);
   `decidir` con tabla de casos: antes y después de `horaCierre`, feriado, fin de semana, jornada completa,
   ya enviado hoy, lunes antes y después de las 09:00, semana anterior completa, semana anterior toda
   feriado (meta 0), avisos desactivados, `NoAutorizado` repetido el mismo día; `marcar` + registro en disco.

**Verificar:** `cargo test` (hoy 91, todas siguen en verde), `cargo clippy --all-targets` sin advertencias, `cargo build`.

## Tarea 2 — Interfaz de preferencias (`ui/`)

1. **Botón «Preferencias»** en el encabezado (junto a «Actualizar» y «Cerrar sesión»), visible también en la
   pantalla de conexión. Abre un panel o diálogo (`<dialog>` está bien) con:
   - Avisos activos (casilla), Hora de cierre de la jornada (`<input type="time">`), Resumen del lunes (casilla).
   - Iniciar con Windows (casilla) y Minimizar a la bandeja al cerrar (casilla).
   - En modo simulado, una nota: «En modo simulado no se registra el inicio con Windows y los avisos salen marcados como [Simulado].»
   - Botones Guardar y Cancelar; los errores de `guardar_preferencias` se muestran en el panel.
2. **Feriados**: `feriadosExtra` deja de vivir en `localStorage`. Al arrancar, `leer_preferencias`. Si
   `localStorage["spe.feriados"]` tiene datos y las preferencias vienen sin `feriadosExtra`, migra una sola
   vez (guarda en preferencias y borra la clave de `localStorage`). `toggleFeriado` guarda con
   `guardar_preferencias` (con las preferencias completas, no solo los feriados) y vuelve a pedir la vista.
   Si guardar falla, revierte el cambio en pantalla y muestra el error en `#notice`.
   `spe.collapsed` y `spe.board` siguen en `localStorage` (son comodidades de la ventana).
3. **Eventos**: escucha `signal://actualizar` (→ `refresh(true)` si hay sesión y no hay carga en curso) y
   `signal://preferencias` (→ releer preferencias, repintar panel y tablero).
4. Textos del pie (`renderTrace`): «puedes marcar o quitar feriados… se guarda en este equipo» sigue siendo
   cierto; agrega una frase corta sobre la bandeja y los avisos.
5. Estilos del panel con las variables existentes (claro y oscuro), usable a 760 px de ancho (el mínimo de la ventana).

**Verificar:** `node --check ui/app.js`; sin scripts ni manejadores en línea; no puedes ver la ventana:
repasa a mano los flujos (primera vez sin preferencias; migración desde `localStorage`; cambiar la hora y
guardar; error al guardar; marcar feriado y que la vista cambie; evento de actualizar durante una carga).

## Tarea 3 — Bandeja, monitor, una instancia y guía de Windows — segunda ola

1. **Dependencias:** `tauri` con la *feature* `tray-icon`; plugins `tauri-plugin-single-instance`,
   `tauri-plugin-notification` y `tauri-plugin-autostart` (versiones 2.x). Permisos mínimos en
   `capabilities/default.json` (la interfaz necesita escuchar eventos; las notificaciones y el inicio
   automático los maneja Rust, así que la ventana no necesita permisos de esos plugins).
2. `src/escritorio.rs`:
   - **Bandeja**: ícono de la app; tooltip con `resumen::texto_bandeja`; menú «Abrir Signal Aduana»,
     «Actualizar ahora», separador, «Salir». Clic izquierdo abre/trae al frente. «Actualizar ahora» emite
     `signal://actualizar` a la ventana y además ejecuta un ciclo del monitor.
   - **Cerrar ventana**: si `minimizarABandeja`, se esconde (evento `CloseRequested` → `prevent_close` + `hide`);
     si no, sale. «Salir» del menú siempre sale.
   - **Una instancia**: el segundo arranque muestra y enfoca la ventana existente.
   - **Arranque oculto**: con el argumento `--oculto` (lo pone el inicio con Windows) la ventana empieza escondida.
   - **Monitor**: tarea asíncrona (`tauri::async_runtime::spawn`) que cada 15 minutos (y 30 s después de
     arrancar) toma la sesión activa del `Estado`, llama `cargar_para_resumen`, `resumir` (con los feriados
     de las preferencias), actualiza el tooltip, llama `avisos::decidir`, envía los avisos con el plugin de
     notificaciones, y guarda el registro. `NoAutorizado` → limpia la sesión en memoria (como los comandos),
     tooltip «Sin conexión con Jira» y aviso Reconectar. Errores de red → solo log y tooltip con la hora del
     último dato bueno. Nunca dos ciclos a la vez. La hora «ahora» es la de Ecuador (UTC−5).
   - **Inicio con Windows**: al arrancar y cada vez que `guardar_preferencias` guarde, sincroniza el plugin
     de autostart con `iniciarConWindows` (con el argumento `--oculto`). Si falla, deja la preferencia en
     false, guárdala y emite `signal://preferencias`. **En modo simulado no registra nada** (solo log).
     Para esto puedes ampliar `guardar_preferencias` en `comandos.rs` (excepción al reparto).
3. **Inicio de sesión y cierre**: tras `iniciar_sesion` y `restaurar_sesion` exitosos, pide un ciclo del monitor
   (para que el tooltip se llene enseguida); tras `cerrar_sesion`, tooltip «Sin sesión». Puedes ajustar
   `comandos.rs` para eso (excepción al reparto).
4. `docs/COMPILAR-WINDOWS.md`: guía paso a paso para el autor — requisitos (rustup con toolchain MSVC,
   Visual Studio Build Tools con «Desarrollo de escritorio con C++», WebView2 ya incluido en Windows 10/11
   actualizados, `cargo install tauri-cli --version "^2"`), copiar el proyecto a una carpeta de Windows (no
   compilar sobre `\\wsl$`), `cargo tauri build`, dónde quedan el `.msi` y el `-setup.exe`, aviso de
   SmartScreen por no estar firmado, cómo probar con el Jira real (release = modo real; `SIGNAL_SIMULADO=1`
   para forzar simulado), dónde quedan config, preferencias, caché y la credencial en Windows, y cómo
   desinstalar limpiando todo. Comprueba en la documentación oficial de Tauri 2 los nombres exactos de las
   carpetas de salida y requisitos; si no puedes confirmarlo, márcalo «a verificar».

**Verificar:** `cargo test`, `cargo clippy --all-targets` sin advertencias, `cargo build`; arranca la app en
modo simulado con `timeout 40` y confirma en el log (agrega trazas con `log`/`eprintln!` discretas, sin datos
sensibles) que el monitor corre un ciclo y calcula el tooltip, sin pánicos. Prueba `--oculto` y que un
segundo arranque no deja dos procesos (`pgrep`). Si la bandeja o las notificaciones no pueden verse en
WSLg, dilo con la salida exacta. Al terminar, ningún proceso de la app debe quedar vivo.

## Después (orquestador)
Compilar, pruebas, revisar diff y entregar al autor la lista de comprobación y la guía de Windows.
