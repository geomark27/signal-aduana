# Roadmap — Signal Aduana de escritorio

**Estado:** aprobado por el autor el 2026-11-17. La fase 5 se cerró ese mismo día (ver «Estado al
cierre» en `docs/PLAN-FASE-5.md`). La fase 6 tiene su plan en `docs/PLAN-FASE-6.md` (2026-11-18),
aprobado por el autor; Tecnología ya autorizó los API tokens personales. Cada
plan se escribe al empezar su fase y el autor lo aprueba antes de lanzar la primera ola.

## Dónde estamos

| Fase | Tema | Estado |
|---|---|---|
| 1 | Proyecto Tauri, login con API token, cliente de Jira y Confluence, fuente simulada | Hecha |
| 2 | Caché local, actualización automática, fuentes dentro de la app | Hecha |
| 3 | Cálculos en Rust (hoy `src/domain/`) con paridad contra el JS (31 casos) | Hecha |
| 4 | Bandeja, avisos, una instancia, preferencias, guía de Windows (tareas 4.1 a 4.10) | Hecha en `develop` |
| 5 | Validar en Windows, cerrar deuda técnica, código en inglés e instalador NSIS (`docs/PLAN-FASE-5.md`) | Hecha el 2026-11-17, con pendientes de Windows que pasan a la fase 6 |
| 6 | Validar con el Jira real (`docs/PLAN-FASE-6.md`) | En curso desde el 2026-11-18 |

La app compila, se instala y funciona en Windows en modo simulado, según el autor; falta el registro
punto por punto y confirmar el instalador nuevo. **Nada se ha probado contra el Jira real.** De ahí
sale el orden de las fases que siguen: primero lo que falta comprobar, después lo que hace falta
para entregarlo.

## Resumen

| Fase | Objetivo | Quién lo hace | Depende de | Está lista cuando |
|---|---|---|---|---|
| **5** | Validar en Windows (sin Jira real) y cerrar deuda técnica | Autor en Windows; ejecutores en WSL | — | La lista de la guía de Windows pasa completa y no quedan «a verificar» |
| **6** | Validar con el Jira real | Autor con su cuenta; ejecutores para las correcciones | Fase 5; visto bueno de Tecnología al uso de API tokens | El tablero real coincide con Jira y los hallazgos son casos de prueba |
| **7** | Distribución y mantenimiento, versión 1.0.0 | Autor y Tecnología; ejecutores para el código | Fase 6 | Un instalador firmado y un camino de actualización documentados; `develop → main` |
| **8** | Evolución (opcional) | A decidir | Fase 7 | Lo que el autor priorice del listado |

Las fases 5 y 6 van separadas a propósito: si algo falla en Windows y con el Jira real a la vez, no
se sabe cuál de los dos es la causa.

---

## Fase 5 — Validar en Windows y cerrar deuda técnica

**Objetivo:** que la app instalada en Windows 10/11 haga todo lo que hoy solo hace en WSL, y dejar
atadas las deudas conocidas de la fase 4. Se sigue usando el **modo simulado**.

Dos vías en paralelo, porque no comparten archivos:

**Vía A — En Windows (la hace el autor)**

| Tarea | Qué se hace |
|---|---|
| 5.1 | Compilar e instalar siguiendo `docs/COMPILAR-WINDOWS.md` (MSI y `-setup.exe`) y anotar lo que falle |
| 5.2 | Pasar la lista de comprobación: ícono, menú y tooltip de la bandeja; cerrar a la bandeja con y sin la preferencia; segunda instancia; arranque con `--hidden`; inicio con Windows real (no aplica en simulado: pasa a la fase 6); los tres avisos; migración de feriados desde el navegador; aspecto del panel de preferencias |
| 5.3 | Instalar y desinstalar limpiamente (¿quedan credenciales, carpetas o entradas de inicio?) |

**Vía B — En WSL (ejecutores, con revisión del orquestador)**

| Tarea | Qué se hace |
|---|---|
| 5.4 | Pruebas que faltan: `cargar_para_resumen` y los casos pendientes de `decidir` (hora de cierre inválida, lunes con fecha de una semana anterior). Además, el resumen semanal pasa a salir el primer día laborable de la semana, no el lunes aunque sea feriado (decisión del autor) |
| 5.5 | Que un pánico en el ciclo del monitor no lo detenga para siempre (se reanuda y se registra) |
| 5.6 | Que los cambios de preferencias (hora de cierre, feriados) lleguen al monitor al instante y no hasta 15 minutos después |

**Ola T — Código en inglés (decisión del autor del 2026-11-17, hecha)**

| Tarea | Qué se hizo |
|---|---|
| 5.9 | Glosario español → inglés (`docs/GLOSARIO.md`) y regla de idioma en `CLAUDE.md` |
| 5.10 a 5.14 | Dominio y paridad, Jira, base de la app, escritorio y comandos, e interfaz pasados a inglés. Lo que ve la persona usuaria y la documentación siguen en español |

Las tareas 5.4 a 5.6 y 5.9 a 5.14 están fusionadas en `develop`.

**Cierre:** la guía `COMPILAR-WINDOWS.md` se actualiza con las rutas y nombres reales que se
confirmaron (se quitan los «a verificar»), y cada hallazgo de Windows se corrige en una tarea con su
prueba.

**Decidido por el autor (2026-11-17):** se entrega solo el `.exe` de NSIS, por usuario, como en Nudge; la desinstalación conserva configuración, preferencias, token y caché (tarea 5.15).

---

## Fase 6 — Validar con el Jira real

**Objetivo:** que lo que muestra la app con tus datos reales coincida con Jira. Hasta aquí el cliente
real solo se probó contra un servidor local simulado.

**Antes de empezar:** Tecnología confirma que los API tokens personales están permitidos.

**Heredado de la fase 5** («Estado al cierre» de `docs/PLAN-FASE-5.md`):
- registrar en Windows la lista de comprobación y la segunda pasada;
- confirmar los «a verificar» de la guía;
- probar el instalador NSIS de la 5.15 (instalar y desinstalar);
- en modo real, «Iniciar con Windows» y el nombre de su entrada en el registro.

**Cómo se trabaja:** Claude y los ejecutores nunca se conectan al Jira real. El autor prueba con su
cuenta y reporta los errores **sin el token**. Cada hallazgo se convierte en un caso de prueba con
datos ficticios que reproduce la forma de la respuesta real; así queda protegido de ahí en adelante.

| Tarea | Qué se comprueba |
|---|---|
| 6.1 | Conexión y login real: sitio, correo, token, mensajes de error (token vencido, sin permiso) |
| 6.2 | Campos y tableros: que los identificadores de campos personalizados (inicio, sprints, puntos, fin de desarrollo) y los nombres de tableros correspondan a los reales |
| 6.3 | Horas: que las horas por día y por semana coincidan con las de Jira y con el artifact original; paginación de registros; semanas con feriados |
| 6.4 | Pull requests y documentos de despliegue: la consulta de desarrollo con un token clásico (no verificado: puede no responder) y la búsqueda en Confluence; el respaldo por palabras si no funciona |
| 6.5 | Volumen y límites: tiempo de carga con tus tareas reales y comportamiento ante límites de consultas |
| 6.6 | Correcciones derivadas, una tarea por hallazgo |

**Decide el autor:** qué hacer si las consultas de desarrollo no funcionan con token clásico
(aceptar el respaldo por palabras o pasar a token con alcances en la fase 8).

---

## Fase 7 — Distribución y mantenimiento (versión 1.0.0)

**Objetivo:** que otras personas de la empresa puedan instalar la app, recibir actualizaciones y
reportar problemas.

| Tarea | Qué se hace |
|---|---|
| 7.1 | Firma de código del instalador, para evitar el aviso de SmartScreen. Necesita un certificado: se gestiona con Tecnología |
| 7.2 | Canal de distribución: carpeta compartida, política de grupo o Intune. Referencia: Nudge publica el `.exe` en releases de GitHub con nombre fijo y un `install.ps1` de una línea; aquí depende de Tecnología y de la visibilidad del repositorio |
| 7.3 | Actualizaciones: actualizador de Tauri (requiere un par de claves propio, distinto del certificado) o proceso manual documentado. Referencia: Nudge usa el actualizador de Tauri con `latest.json` |
| 7.4 | Registro de errores para diagnóstico, sin tokens ni datos de tareas |
| 7.5 | Caché: decidir entre dejarla en texto plano (hoy guarda títulos y horas de tareas en el perfil del usuario) o cifrarla; cerrar la carrera conocida al cerrar sesión mientras se guarda |
| 7.6 | Manual de usuario corto: instalación, conexión, qué significan las señales y los avisos |
| 7.7 | Versión 1.0.0: número de versión, notas de la versión y PR `develop → main` con merge commit |

**Decide el autor y Tecnología:** certificado, canal de distribución, política de actualización y
si la caché va cifrada.

**Adelantado en la fase 6 (tarea 6.8, 2026-11-18):** parte de la 7.2 y de la 7.7, a pedido del autor.
Ya existen `make bump`, `make release` y un workflow de GitHub Actions que compila el instalador NSIS
sin firmar y lo publica en los releases del repositorio, que es público por decisión del autor. Siguen
pendientes la firma (7.1), el actualizador (7.3) y la decisión del canal con Tecnología.

**Adelantado también (tarea 6.13, 2026-11-18):** el actualizador de la 7.3, con el plugin de Tauri
y `latest.json` en los releases, como Nudge. Sigue pendiente la firma de código (7.1).

---

## Fase 8 — Evolución (opcional)

Sin compromiso ni orden. Se decide cuando la 1.0.0 esté en uso.

- **Autenticación:** token con alcances o OAuth 2.0 en lugar del token clásico, según lo que pida Tecnología.
- **Rarezas heredadas del JS** que se replicaron a propósito para mantener la paridad (por ejemplo «1 día laborables», sprints llamados solo «Sprint N»): corregirlas con sus pruebas.
- **Versión de equipo:** el artifact original se llamaba «Signal Personal Equipo». Una vista para líderes sería un producto aparte, con permisos y datos de otras personas.
- **Accesibilidad e interfaz:** revisión de teclado, contraste y tamaños.

---

## Cómo se trabaja cada fase

- Cada tarea tiene su rama `tarea/N.M-tema` desde `develop` al día, su worktree `.worktrees/N.M`, un
  solo commit `Tarea: N.M <tema>` con su entrada de bitácora, y vuelve por PR con merge commit.
- Claude planifica y revisa; los ejecutores (Sonnet 5.5 medio u opencode) implementan. Antes de cada
  ola, Claude presenta tareas, archivos, modelo y nombres de rama, y espera el «sí» del autor.
- Claude no hace commit, push, PR, merge ni crea ramas sin el «sí» del autor. Los comandos largos
  (`cargo test` completo, `make check`, levantar la app) los corre el autor.
- Reglas que no cambian en ninguna fase: ningún servicio real desde una sesión de Claude, ningún
  token en logs, archivos ni bitácora, y los cálculos siguen viviendo en `src/dominio/`.

## Riesgos

| Riesgo | Efecto | Mitigación |
|---|---|---|
| El cliente real nunca ha hablado con Jira | Pueden aparecer fallos de formato o de campos que ningún mock anticipa | Fase 6 separada, con un caso de prueba por hallazgo |
| La consulta de pull requests puede no funcionar con token clásico | Los documentos de despliegue caen al respaldo por palabras | Decisión del autor en la fase 6 |
| Sin certificado, SmartScreen avisa al instalar | Desconfianza o bloqueo en equipos con política estricta | Tarea 7.1 con Tecnología |
| Todo lo de Windows se probó solo en WSL | La fase 5 puede destapar bastantes ajustes | Fase 5 antes que cualquier otra |
