# 016 — Glosario español → inglés y regla de idioma

**Fecha:** 2026-11-17
**Rama:** tarea/5.9-glosario-ingles → develop
**Fase:** 5
**Ejecutor:** orquestador (Opus 5.5). Un subagente Sonnet 5.5 hizo el inventario de identificadores, en solo lectura y fuera del repo.

## 1. Solicitud

El autor pidió que todo el código quede en inglés, salvo lo que ve el usuario. Lo decidió así
(2026-11-17):
- solo pasa al inglés el código: la documentación del repo sigue en español;
- también se traducen los contratos internos (comandos, códigos de error, campos JSON y archivos en
  disco), sin migración;
- primero se cerraba la 5.6;
- los atajos de `make` y los scripts conservan sus nombres.

Esta tarea deja escrita la regla y el glosario que seguirán las tareas de traducción.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El código está en español (unas 6 700 líneas de Rust y 1 800 de JS) | El autor quiere el código en inglés, como el resto de su trabajo |
| Muchos nombres cruzan de Rust a la interfaz, a los casos de paridad y al disco | Si dos ejecutores traducen el mismo nombre de forma distinta, la app o la paridad se rompen sin un error de compilación que lo avise |
| Hay literales en español que son a la vez texto visible y valor interno (por ejemplo «Todos», los nombres de tablero o los estados de Jira) | Traducirlos rompe la app o cambia lo que ve el usuario |

## 3. Diseño técnico

- **Inventario primero.** Un subagente hizo el inventario de solo lectura de los 30 archivos Rust,
  `ui/` y la paridad. Encontró:
  - 61 tipos, 213 funciones y 160 pruebas;
  - 13 comandos, 2 eventos y 14 códigos de error;
  - 4 archivos en disco;
  - los 62 casos de paridad;
  - 23 literales dudosos y varias colisiones de nombres.

  Quedó en el *scratchpad* de la sesión, no en el repo.
- **`docs/GLOSARIO.md`** es la fuente única de los nombres nuevos. Está organizado por tarea, con
  las claves JSON y TOML nuevas de cada contrato, convenciones para lo que no lista y la resolución
  de las colisiones:
  - `Estado` → `AppState` (sesión) o `Status` (simulado).
  - `Registro` → `Worklog` o `NotificationLog`.
  - `Resultado` → `AppResult`, para no tapar el `Result` de `std`.
- **Valores que no se traducen:** lo que ve el usuario o viene de Jira. Por ejemplo «Todos», «Sin
  tablero», «Sin componente», los nombres de tableros y estados, «Subtarea» y «Despliegue». Solo se
  traduce el nombre de la constante que los guarda.
- **Lo que tampoco se toca:**
  - `legado.js`;
  - la clave `spe.feriados`, que hace falta para la migración;
  - el identificador de la app y el servicio de la credencial;
  - `SIGNAL_SIMULADO`.
- **Decisiones del autor en esta tarea:**
  1. Los códigos de error pasan al inglés y la interfaz deja de añadir « [código]» al mensaje.
  2. `--oculto` pasa a `--hidden`.
  3. El token de prueba «rechazar» se queda, porque es texto que la persona escribe y lee.
- **Orden:** cinco tareas en serie (5.10 dominio y paridad, 5.11 Jira, 5.12 base, 5.13 escritorio y
  comandos, 5.14 interfaz), nunca en paralelo, porque cada renombre alcanza a quien lo usa. Cada
  una sale con las pruebas en verde por sí sola y cambia los dos lados de cada contrato que toca.
- **Numeración del plan:** los hallazgos de Windows, que iban a ser las 5.9 y siguientes, pasan a
  empezar en la 5.15.

## 4. Archivos

- `docs/GLOSARIO.md` (nuevo): el glosario.
- `CLAUDE.md`: la regla de idioma (código en inglés; en español lo que ve el usuario y la
  documentación; excepciones).
- `docs/PLAN-FASE-5.md`:
  - las tareas 5.9 a 5.14 en el mapa;
  - la sección «Ola T — Código en inglés»;
  - el diagrama de orden;
  - los hallazgos renumerados desde la 5.15.
- `docs/bitacora/README.md` (+1 fila) y esta entrada.

## 5. Cómo verificar

Corrido por el orquestador en esta rama:

```bash
bash scripts/verificar-bitacora.sh
```

No hay cambios de código: no aplica `cargo test`. El glosario no se contrastó con un compilador;
lo validan las tareas 5.10 a 5.14, que deben salir con el mismo número de pruebas en verde.

## 6. Pendientes

- Las tareas 5.10 a 5.14. Cada una actualizará el árbol de `CLAUDE.md` (rutas como `src/dominio/`
  o `pruebas/paridad/`) cuando su cambio llegue a `develop`.
- Si una tarea encuentra un nombre que el glosario no resuelve, se agrega al glosario en esa misma
  tarea, con la aprobación del orquestador.
