# Datos de referencia de paridad (JS legado → Rust)

Demuestran que el módulo Rust `domain` calcula **exactamente lo mismo** que el JS de la fase 2.

## Regenerar (desde la raíz del proyecto)

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -q --example dump_simulated -- 2026-11-13 > tests/parity/dumps/friday.json
cargo run -q --example dump_simulated -- 2026-11-16 > tests/parity/dumps/monday.json
node tests/parity/generate.mjs
```

Solo el tercer comando es necesario si los volcados no cambiaron (el simulado es determinista para un `hoy`).
Todo es simulado: sin red, sin reloj, sin credenciales.

## Archivos

| Archivo | Qué es |
|---|---|
| `legado.js` | Copia literal de las funciones y constantes de cálculo de `ui/app.js` (fase 2), sin DOM. Los únicos cambios (estado inyectable, `hoy` inyectado) están marcados con «CAMBIO». No se corrige nada: si el JS tiene una rareza, aquí se conserva. |
| `generate.mjs` | Define los casos, ejecuta el legado y arma la `View` del contrato D (el adaptador vive aquí, no en `legado.js`). |
| `dumps/friday.json`, `dumps/monday.json` | Tareas del simulado con horas unidas y `hasPR`, para `hoy` = 2026-11-13 (viernes) y 2026-11-16 (lunes). Los produce `examples/dump_simulated.rs`. |
| `cases/<nombre>.input.json` | `ViewInput` completo (`issues` **antes** de `prep()`, `week`, `board`, `extraHolidays`, `today`). |
| `cases/<nombre>.expected.json` | `View` esperada. Claves ordenadas, 2 espacios, salto de línea final. |

Prefijo `fri-` = `hoy` viernes; `mon-` = `hoy` lunes. Los archivos generados se pueden regenerar y comparar con `sha256sum`: son reproducibles.
