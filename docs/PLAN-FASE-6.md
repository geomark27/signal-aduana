# Signal Aduana de escritorio — Plan de la fase 6

**Estado:** aprobado por el autor el 2026-11-18, con las decisiones 1 a 4 de la tabla «Decisiones del
autor».

**Requisito previo (roadmap):** que Tecnología confirme que se permiten los API tokens personales de
Jira. **Cumplido** según el autor (2026-11-18): Tecnología ya los aprobó y el mismo tipo de token
(el API token personal de Atlassian) se usa en Draco. Las olas A0, A y B pueden empezar.

**Punto de partida:** `develop` en `7dcccb8` (fase 5 cerrada). 160 pruebas en verde en WSL. El cliente
real solo se probó contra `wiremock`. Contexto: `docs/ROADMAP.md` y «Estado al cierre» de
`docs/PLAN-FASE-5.md`.

## Objetivo

Que lo que muestra la app con los datos reales del autor coincida con Jira, y que cada diferencia
quede protegida por un caso de prueba con datos ficticios. Además, cerrar lo que la fase 5 dejó
pendiente en Windows.

## Qué entra y qué no

**Entra:**
- login real;
- campos personalizados y tableros;
- horas;
- PR y documentos de despliegue;
- volumen y límites;
- lo heredado de la fase 5 en Windows;
- una tarea de corrección por hallazgo.

**No entra:**
- firma, distribución, actualizador ni la versión 1.0.0 (fase 7);
- token con alcances u OAuth (fase 8).

## Reglas

Las de `CLAUDE.md`, que mandan sobre este documento. En esta fase pesan en especial estas:

- **Claude y los ejecutores nunca se conectan al Jira real ni a Confluence.** Las pruebas reales las
  hace el autor en su equipo, con su cuenta.
- **Lo que el autor reporta no lleva datos reales.** No incluye el token ni contraseñas. Tampoco
  incluye URLs o nombres del sitio, correos, nombres de personas, claves ni títulos de tareas reales,
  ni capturas con esos datos. Lo que sí se reporta:
  - números (horas, cantidades, tiempos);
  - códigos HTTP y mensajes de error;
  - la **forma** de una respuesta con los valores cambiados por ficticios (formato de abajo).
- Cada hallazgo se convierte en una prueba con `wiremock` o con el simulado que reproduce esa forma,
  y luego en la corrección.
- Una rama = una entrada de bitácora = un dueño. Los ejecutores no tocan git ni `docs/bitacora/` y
  terminan con un bloque `DEVLOG CONTRIBUTION`.
- Los comandos largos los corre el autor.

## Mapa de tareas

| Tarea | Qué | Quién | Rama | Ola |
|---|---|---|---|---|
| 6.0 | Este plan, el roadmap, `delegar.json` y la entrada de bitácora | Orquestador | `tarea/6.0-plan-fase-6` | 0 |
| 6.H | Lo heredado de la fase 5 en Windows, en modo simulado | Autor | sin rama | A0 |
| 6.1 | Conexión y login real | Autor | sin rama | A |
| 6.2 | Campos personalizados y tableros | Autor | sin rama | A |
| 6.3 | Horas por día y por semana | Autor | sin rama | A |
| 6.4 | PR y documentos de despliegue | Autor | sin rama | A |
| 6.5 | Volumen, tiempos y límites | Autor | sin rama | A |
| 6.7 | Diagnóstico anónimo: la app arma un resumen sin datos reales para reportar | Sonnet 5.5 medio | `tarea/6.7-diagnostico-anonimo` | B |
| 6.8 | Flujo de release: `make bump`, `make release` y GitHub Actions (adelantado de la fase 7, sin firma ni actualizador) | Sonnet 5.5 medio + orquestador | `tarea/6.8-release` | B |
| 6.9 | Versión 0.1.0: `make bump ver=0.1.0`, `make promote` y el primer `make release` | Autor y orquestador | `tarea/6.9-version-0-1-0` | B |
| 6.10 | Componente Lista: búsqueda, filtros, filas por página y paginado únicos para todas las listas | Sonnet 5.5 medio | `tarea/6.10-componente-lista` | E |
| 6.11 | Pestañas (propuesta A): Resumen · Tareas · Señales · Despliegues | Sonnet 5.5 medio | `tarea/6.11-pestanas` | E |
| 6.12 | Versión 0.2.0 con el rediseño (`make bump bump=minor`, `promote`, `release`) | Autor | `tarea/6.12-version-0-2-0` | E |
| 6.13 | Actualizador automático (adelantado de la 7.3): plugin de Tauri, firma, `latest.json` y aviso en la app | 3 × Sonnet 5.5 medio | `tarea/6.13-actualizador` | E |
| 6.14 | Versión 0.3.0, la primera con actualizador (`make updater-key` y `make bump bump=minor`) | Autor y orquestador | `tarea/6.14-version-0-3-0` | E |
| 6.6 y libres | Una corrección por hallazgo, con su prueba | Según el caso | por hallazgo | C |
| (libre) | Cierre de la fase | Orquestador | `tarea/N.M-cierre-fase-6` | D |

Los números siguen al roadmap (6.1 a 6.6). Las correcciones toman el 6.6 y después el primer
número libre (del 6.8 al 6.11 se usaron para el flujo de release, la versión 0.1.0 y el rediseño,
pedidos por el autor el 2026-11-18); el cierre también toma el primer número libre.

## Orden

```
Ola 0   6.0 plan ───────────────────────────────────────────────┐
Ola A0  6.H Windows en simulado (ya se puede)                    ─┤
Ola B   6.7 diagnóstico anónimo (WSL)                           ─┤
        (visto bueno de Tecnología: cumplido)                      │
Ola A   6.1 → 6.2 → 6.3 → 6.4 → 6.5 (autor, Jira real)            ─┤
Ola C   correcciones por hallazgo (WSL, una rama cada una)         ▼
Ola D   cierre
```

La 6.1 va primero: sin login no hay nada más que probar. Las demás pueden ir en cualquier orden.

---

## Ola A0 — 6.H Lo heredado de la fase 5 (autor, Windows, simulado)

No necesita Jira real; se puede hacer ya. Se sigue `docs/COMPILAR-WINDOWS.md`.

- **Instalador de la 5.15.** Comprobar:
  - que se genera solo el `-setup.exe`, en español y sin pedir administrador;
  - la ruta de instalación;
  - al desinstalar: que en `%APPDATA%` quedan config y preferencias, y que en `%LOCALAPPDATA%` queda
    la caché y **no** `notifications*.json` ni `EBWebView`.
- **Lista de comprobación (sección 7) y segunda pasada.** Marcar cada punto ✓, ✗ o «solo en modo
  real».
- **Los seis «a verificar»** de la guía:
  - la descarga de Build Tools;
  - la duración de la primera compilación;
  - el nombre y la ruta del instalador;
  - la ruta del `.exe`;
  - la carpeta de WebView2;
  - la entrada de inicio. Esta última se ve en la 6.1, en modo real.

## Ola A — Jira real (autor)

Para todas: compilar el `release` sin `SIGNAL_SIMULADO`, que en `release` arranca en modo real. Se
usa la cuenta del autor y se reporta con el formato de abajo.

**6.1 Conexión y login**
- Conectar con el sitio, el correo y el token reales: entra y muestra el nombre.
- Token revocado o vencido: aparece la pantalla de conexión con su mensaje y llega el aviso de
  reconexión una vez al día.
- Token de otra persona o sin permiso sobre un proyecto: anotar el mensaje.
- Cerrar sesión y volver a abrir la app: pide conectar otra vez.
- En `cmdkey /list` existe `signal-aduana:<host>`. Solo se reporta que existe, no el host.
- Modo real: «Iniciar con Windows» activado, cerrar sesión de Windows y volver a entrar: la app
  arranca oculta. Anotar el **nombre** del valor en `HKCU\…\Run`. Es el último «a verificar».

**6.2 Campos y tableros**
- El cliente pide cuatro campos personalizados fijos: `customfield_10015` (inicio), `10020`
  (sprints), `10031` (puntos) y `10057` (fin de desarrollo). En Jira, en «Configuración del
  sistema → Campos personalizados» o con el botón «…» de una incidencia, el autor confirma el **id**
  de cada uno.
- Por cada campo se reporta una de tres cosas:
  - el id coincide;
  - el id es otro: se reporta el id correcto, que no es un dato sensible;
  - el campo no existe.
- Los tableros que muestra la app son los que el autor usa, y cada tarea cae en su tablero. Si una
  cae en «Sin tablero» o en otro equivocado, se reporta la forma de esa tarea anonimizada: proyecto
  como `XXX`, sprint como «Sprint N», componente.

**6.3 Horas**
- Elegir **dos semanas cerradas** y la actual. Para cada día, comparar las horas de la app con las
  del registro de trabajo de Jira (o el reporte de horas que use el autor) y con el artifact original
  si todavía lo tiene.
- Reportar una tabla `día | app | Jira`, sin nombres de tareas.
- Probar una semana con un feriado y una tarea con muchos registros: más de 20, que es donde el
  cliente pide la paginación aparte.

**6.4 PR y documentos de despliegue**
- La consulta `development[pullrequests].all > 0` puede no responder con un token clásico. Hay que ver
  qué modo usa la app: con PR o con el respaldo por palabras. El modo se ve en la caché
  (`prMode: "pr"` o `"keywords"`) o por el comportamiento.
- Elegir una tarea con PR y documento en Confluence y otra sin documento: la app las marca bien.
  Reportar el resultado, sin los títulos.

**6.5 Volumen y límites**
- Cuántas tareas y cuántos registros trae la carga: solo números.
- Cuánto tarda la primera carga y cuánto «Actualizar».
- Si aparece un error de límite (429) o de tiempo (más de 30 s), anotar el mensaje y en qué momento
  pasó.

## Formato de hallazgo (fase 6)

```
Tarea: 6.N — <qué se probaba>
Esperado: <lo que dice Jira; números, no nombres>
Pasó: <lo que muestra la app; mensaje de error exacto si lo hay>
Forma anonimizada (si aplica): <JSON o descripción con claves y nombres cambiados por ficticios>
Cómo repetirlo: <pasos>
Versión: <commit de develop con el que se compiló>
```

## Ola B — 6.7 Diagnóstico anónimo (aprobada, decisión del autor 2)

**Problema.** En `release` la app no tiene consola, y el autor tiene que reconstruir a mano la forma
de una respuesta sin datos reales.

**Propuesta.**
- La app arma un **resumen de diagnóstico anónimo**:
  - versión y modo;
  - qué campos personalizados llegaron y de qué tipo;
  - número de tareas y de registros;
  - modo de PR;
  - tiempos de cada llamada y el último error, con su código.
- Lo arma reemplazando claves, títulos, nombres, correos y URLs por marcadores (`ISSUE-1`, `[título]`,
  `[persona]`…).
- La persona lo copia desde las preferencias y lo pega en el reporte.

**Archivos.** Un módulo nuevo `src/diagnostics.rs` (y su comando), más el botón en las preferencias
(`ui/`). La interfaz y Rust van en tareas separadas si el diseño lo pide.

**Criterio.** Pruebas con el simulado y con `wiremock` demuestran que la salida no contiene ninguna
clave, título, correo ni URL del origen.


## Ola C — Correcciones (6.6 y números libres)

Una tarea por hallazgo. El orquestador escribe su plan breve: objetivo, archivos, la prueba que
reproduce la forma anonimizada y el criterio. Lo presenta al autor antes de lanzarlo. Los hallazgos
de la interfaz y los de Rust no comparten tarea. Si un hallazgo cambia los ids de los campos
personalizados, se decide si van fijos o se configuran (decisión 3).

## Ola E — Rediseño de la interfaz (propuesta A)

Pedido por el autor el 2026-11-18, que eligió la propuesta A entre tres prototipos con datos
ficticios: pestañas Resumen · Tareas · Señales · Despliegues, con la semana y los tableros fijos
arriba, y un único componente de lista para todas las secciones.

- **6.10 Componente Lista.** Búsqueda, filtros con conteo, filas por página recordadas y un solo
  paginado para la tabla de tareas y Señales. No cambia el diseño ni ninguna regla.
- **6.11 Pestañas.** El diseño de la propuesta A sobre el componente de la 6.10.
- **6.12 Versión 0.2.0.** La publicó el autor el 2026-11-18 con el rediseño, todavía sin
  actualizador.
- **6.13 Actualizador.** El autor eligió adelantar la 7.3 el 2026-11-18. Va dividido en tres
  ejecutores en paralelo, con el contrato fijado: Rust, workflow y scripts, e interfaz.
- **6.14 Versión 0.3.0.** El autor crea la llave con `make updater-key` dentro de la tarea. Las
  versiones 0.1.0 y 0.2.0 instaladas se actualizan a mano una última vez; desde la 0.3.0 se
  actualizan solas.

Van en serie porque las dos tocan `ui/app.js`, `ui/index.html` y `ui/styles.css`. El autor las
revisa en la app (`make run` en modo simulado) antes de fusionar, y salen en la versión 0.2.0. Las
pruebas con el Jira real siguen en paralelo con la v0.1.0.

## Ola D — Cierre

«Estado al cierre» en este documento, roadmap, índice de la bitácora y
`bash scripts/verificar-bitacora.sh`.

## Hecha la fase cuando

1. Login real, campos, tableros, horas, PR y documentos coinciden con Jira, o cada diferencia tiene
   su corrección fusionada con su prueba.
2. Las horas de las tres semanas de la 6.3 coinciden día por día.
3. Lo heredado de la fase 5 está registrado: lista, segunda pasada, «a verificar» e instalador.
4. Cada tarea tiene su entrada de bitácora y `bash scripts/verificar-bitacora.sh` sale con 0.

## Decisiones del autor

| # | Pregunta | Decisión (2026-11-18) |
|---|---|---|
| 1 | Si la consulta de PR no funciona con token clásico, ¿se acepta el respaldo por palabras o se pasa a token con alcances? | **Respaldo por palabras** en la fase 6; el token con alcances queda para la fase 8 |
| 2 | ¿Se hace la 6.7 (diagnóstico anónimo)? | **Sí** |
| 3 | Si los ids de campos personalizados no coinciden, ¿se corrigen fijos en el código o se vuelven configurables? | **Fijos en el código** (un solo sitio de Jira) |
| 4 | ¿Ejecutores? | Sonnet 5.5 medio para la 6.7 y las correcciones de Rust; opencode free para correcciones pequeñas y mecánicas (propuesta del orquestador, aprobada con el plan) |

## Riesgos

| Riesgo | Efecto | Mitigación |
|---|---|---|
| Tecnología restringe más adelante los tokens personales | La ola A se detiene | Cumplido el 2026-11-18 según el autor; si cambia, se ajusta el plan |
| La consulta de PR no responde con token clásico | Los documentos de despliegue se marcan por palabras | Decisión 1; ya existe el respaldo |
| Ids de campos distintos en el Jira real | Tableros, inicio o puntos vacíos | 6.2 lo detecta primero; decisión 3 |
| Un dato real se cuela en un reporte o en la bitácora | Fuga de información de la empresa | Formato de hallazgo; 6.7; `verificar-bitacora.sh` rechaza URLs y correos |
| Muchas tareas o registros | Carga lenta o error 429 | 6.5 lo mide; corrección con su prueba |

## Estado al cierre

(Se completa en la tarea de cierre.)
