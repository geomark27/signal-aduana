# Datos de referencia de paridad (JS legado → Rust)

Demuestran que el módulo Rust `dominio` calcula **exactamente lo mismo** que el JS de la fase 2.

## Regenerar (desde la raíz del proyecto)

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -q --example volcar_simulado -- 2026-11-13 > pruebas/paridad/volcados/viernes.json
cargo run -q --example volcar_simulado -- 2026-11-16 > pruebas/paridad/volcados/lunes.json
node pruebas/paridad/generar.mjs
```

Solo el tercer comando es necesario si los volcados no cambiaron (el simulado es determinista para un `hoy`).
Todo es simulado: sin red, sin reloj, sin credenciales.

## Archivos

| Archivo | Qué es |
|---|---|
| `legado.js` | Copia literal de las funciones y constantes de cálculo de `ui/app.js` (fase 2), sin DOM. Los únicos cambios (estado inyectable, `hoy` inyectado) están marcados con «CAMBIO». No se corrige nada: si el JS tiene una rareza, aquí se conserva. |
| `generar.mjs` | Define los casos, ejecuta el legado y arma la `Vista` del contrato D (el adaptador vive aquí, no en `legado.js`). |
| `volcados/viernes.json`, `volcados/lunes.json` | Tareas del simulado con horas unidas y `hasPR`, para `hoy` = 2026-11-13 (viernes) y 2026-11-16 (lunes). Los produce `examples/volcar_simulado.rs`. |
| `casos/<nombre>.entrada.json` | `Entrada` completa (tareas **antes** de `prep()`, semana, tablero, `feriadosExtra`, `hoy`). |
| `casos/<nombre>.esperado.json` | `Vista` esperada. Claves ordenadas, 2 espacios, salto de línea final. |

Prefijo `v-` = `hoy` viernes; `l-` = `hoy` lunes. Los archivos generados se pueden regenerar y comparar con `sha256sum`: son reproducibles.
