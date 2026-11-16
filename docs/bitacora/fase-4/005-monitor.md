# 005 — Monitor: ciclo de lectura, tooltip y avisos

**Fecha:** 2026-11-16
**Rama:** tarea/4.3-monitor → develop
**Fase:** 4
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta. El orquestador cambió una aserción de la prueba 3 (ver Diseño técnico).

## 1. Solicitud

Punto «Monitor» de la tarea 3 de `docs/PLAN-FASE-4.md`: lo que cada 15 minutos lee las horas,
actualiza el texto de la bandeja y decide los avisos. El orquestador dividió esa tarea 3 en cuatro
(4.3 monitor, 4.4 bandeja, 4.5 integración, 4.6 guía de Windows) y el autor aprobó la división. Esta
es la parte del ciclo, sin Tauri.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El ciclo del monitor dentro del código de Tauri no se puede probar sin ventana ni bandeja | En WSL no hay forma de comprobar el tooltip, los avisos ni los casos de error; se descubrirían en Windows |
| Un token revocado o una caída de red a mitad de jornada | Sin tratamiento propio: tooltip viejo, avisos repetidos o la sesión viva con un token muerto |

## 3. Diseño técnico

- **`monitor::ciclo(estado, salida, ahora, memoria)`** es una pasada completa y no usa Tauri ni reloj
  propio: la hora de Ecuador entra por parámetro y la salida real se conecta con el `trait Salida`
  (`tooltip`, `notificar`). El bucle de 15 minutos, la bandeja y las notificaciones reales son de la
  4.5.
- **Preferencias y registro de avisos se leen en cada ciclo**: la persona puede cambiarlas mientras
  la app está abierta.
- **Resultados**: sin sesión (`SinSesion`, tooltip «Sin sesión»); token rechazado (`NoAutorizado`:
  limpia la sesión en memoria, como hacen los comandos, y el tooltip dice «Sin conexión con Jira»);
  cualquier otro fallo (`Fallo`: no toca la sesión, solo registra el error, y el tooltip conserva el
  último dato bueno con su hora, o «Sin datos» si no había). `Memoria` guarda ese último dato.
- **Orden**: el tooltip se fija una vez por ciclo y antes de notificar; solo si hubo avisos se marca
  el registro y se guarda. Un fallo al guardar el registro se anota en el log y no detiene el ciclo.
- **Texto del tooltip** recortado a `MAX_TEXTO_BANDEJA` (120) con `chars().take(..)`, nunca por bytes.
  En el caso «último dato», `(simulado)` queda antes de « · último dato HH:MM» porque sale de
  concatenar `texto_bandeja` con el sufijo.
- **Pruebas con el modo simulado y fecha fija.** El simulado solo genera horas de días **anteriores**
  a `hoy`, así que el viernes 2026-11-13 tiene 0 h y el aviso de cierre de jornada se dispara a las
  17:00. La prueba 3 lo exige sin condición. La versión del ejecutor lo comprobaba solo «si
  `hoy_secs < 8 h`», que podía pasar sin comprobar nada; el orquestador la volvió incondicional tras
  confirmar en `simulado.rs` que `hoy_secs` es 0 ese día. Hueco de la tarea original, no del agente.
- **Fallo de red probado con `wiremock`** (servidor que responde 500), usando `JiraHttp::con_base`,
  que ya existía solo para pruebas del crate; no se modificó ningún otro archivo.
- **Descartado**: poner el bucle del monitor aquí. Necesita el `AppHandle` de Tauri, así que va en la
  4.5 y esta tarea queda probable entera.

## 4. Archivos

- `src/monitor.rs` (nuevo, 372 líneas) — `Salida`, `Memoria`, `ResultadoCiclo`, `ahora_ecuador`,
  `ciclo` y 6 pruebas.
- `src/lib.rs` (+1) — `pub mod monitor;`.
- `docs/bitacora/README.md` (+1) y esta entrada.

## 5. Cómo verificar

Corridos por el ejecutor (consta en sus registros de `target/`): `cargo test monitor` (6 de 6),
`cargo test` (121 de la librería y 1 de paridad, sin fallos) y `cargo clippy --all-targets` sin
advertencias.

**No corridos tras el cambio del orquestador** (aserción de la prueba 3): lo corre el autor.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test monitor           # 6 pruebas, incluida la 3 con la aserción incondicional
cargo test                   # todas
cargo clippy --all-targets   # sin advertencias
```

## 6. Pendientes

- Falta conectar el ciclo: bucle a los 30 s del arranque y cada 15 minutos sin solapar ciclos, y
  `Salida` real con la bandeja y el plugin de notificaciones (4.5).
- Los ejecutores (subagentes) no pueden escribir `REPORTE.md`: devolvieron el reporte en su mensaje
  final.
