# 018 — Jira en inglés

**Fecha:** 2026-11-17
**Rama:** tarea/5.11-jira-en-ingles → develop
**Fase:** 5
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta. El orquestador revisó el diff, cambió un nombre que había fijado mal en el glosario (`Estimate::NoEstimate` → `Unset`), agregó al glosario la regla de los textos para quien programa y escribió esta entrada.

## 1. Solicitud

Tarea 5.11 de `docs/PLAN-FASE-5.md`: pasar al inglés el módulo `src/jira/` siguiendo
`docs/GLOSARIO.md` (sección 3), sin cambiar el comportamiento, los datos del simulado ni ningún
texto visible.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El cliente de Jira, el modelo y el simulado estaban en español (`Tarea`, `Registro`, `Fuente`, `JiraSimulado`…) | Incumplía la regla de idioma (entrada 016) en la segunda capa de la que depende todo |
| `ResultadoHoras` y `Usuario` cruzan a la interfaz (`porClave`, `fallidas`, `nombre`) | Si se cambiaba un solo lado, las horas o el nombre de la persona dejaban de mostrarse |
| El simulado tiene muchos datos en español mezclados con el código | Traducir de más habría cambiado lo que se ve en pantalla y la paridad |

## 3. Diseño técnico

- **Archivos.** `modelo.rs` pasa a `model.rs` y `simulado.rs` a `simulated.rs`. `mod.rs` y `http.rs`
  conservan su nombre. Tipos, funciones, campos, comentarios y pruebas quedan con los nombres del
  glosario (`Issue`, `Worklog`, `User`, `Source`, `SimulatedJira`, `WorklogsResult`, `load_issues`,
  `read_worklogs`, `load_for_summary`…).
- **Contrato.** `User` pasa a `{ accountId, name, email }` y `WorklogsResult` a
  `{ byKey, failed, error }`. El JSON de `Issue`, `Sprint` y `Worklog` no cambia, porque ya estaba en
  inglés. En `ui/app.js` solo cambian cuatro líneas: `usuario.name`, `r.byKey`, `r.failed` y el valor
  inicial. La clave `usuario` de `InfoSesion` espera a la 5.12.
- **Lo que no se traduce todavía.** Los comandos de Tauri, los campos de `Sesion` e `InfoSesion` y
  `ErrorApp`, `Resultado`, `Sitio` y `Credenciales` son de las tareas 5.12 y 5.13. En el resto de
  los archivos solo cambiaron las referencias a Jira.
- **Simulado.** Los duplicados locales de los feriados se tradujeron (`BASE_HOLIDAYS`,
  `is_holiday`, `is_workday`), sin reemplazarlos por los del dominio: eso sería cambio de código, no
  un renombre.
- **Nombre corregido por el orquestador.** El glosario fijaba `Estimate::NoEstimate`. Clippy lo
  marca (`enum_variant_names`) porque repite el nombre del enum, y el ejecutor lo había silenciado
  con `#[allow]`. Se cambió a `Estimate::Unset`, se quitó el `allow` y se corrigió el glosario.
- **Regla nueva en el glosario (sección 1).** El ejecutor dejó en español los mensajes de
  aserciones y los `expect`, porque son literales. No los ve la persona usuaria, así que van en
  inglés:
  - las tareas 5.12 y 5.13 los traducen en sus archivos;
  - la 5.14 hace la pasada final por `src/domain/` y `src/jira/`.

## 4. Archivos

- `src/jira/model.rs` y `src/jira/simulated.rs` (movidos y traducidos).
- `src/jira/mod.rs` y `src/jira/http.rs` (traducidos).
- `src/sesion.rs`, `src/monitor.rs`, `src/comandos.rs` y `src/resumen.rs`: solo las referencias a
  Jira.
- `examples/dump_simulated.rs`: lo que quedaba de Jira. Sus mensajes y su salida no cambian.
- `ui/app.js`: el contrato de Jira, cuatro líneas.
- `docs/GLOSARIO.md`: `Unset` y la regla de los textos para quien programa.
- `docs/bitacora/README.md` (+1 fila) y esta entrada.

## 5. Cómo verificar

Corrido por el ejecutor en WSL:
- `cargo test`: 160 pruebas antes y 160 después, todas en verde. La paridad pasa sin regenerar los
  casos.
- `cargo clippy --all-targets`: sin advertencias. `node --check ui/app.js` y `make ui-format`
  pasan.
- Regeneró los volcados con `cargo run -q --example dump_simulated` para el 2026-11-13 y el
  2026-11-16. Comparados con `cmp`, son **idénticos** a `tests/parity/dumps/friday.json` y
  `monday.json`: el simulado produce exactamente los mismos datos.
- Comparó los literales de los cuatro archivos de Jira con los anteriores. Hay los mismos en cada
  archivo; solo cambian nombres dentro de `format!` y una clave JSON de prueba (`porClave` →
  `byKey`).
- El `grep` de nombres viejos solo encuentra «Fuente:», que es un texto visible de la ayuda de
  `ui/app.js`.

Corrido por el orquestador: la lectura del diff de `ui/app.js` y del cambio `Unset`. El cambio
`Unset` no se compiló. Lo cubre el autor antes del commit:

```bash
make check
```

## 6. Pendientes

- Los comentarios de `domain/calendar.rs` y de `BASE_HOLIDAYS` en `simulated.rs` citan el nombre que
  la constante de la interfaz tendrá después de la 5.14.
- Mensajes de aserción y `expect` en español dentro de `src/domain/` y `src/jira/`: se traducen en la
  pasada final de la 5.14.
