# 015 — Los cambios de preferencias llegan al monitor al instante

**Fecha:** 2026-11-17
**Rama:** tarea/5.6-preferencias-al-instante → develop
**Fase:** 5
**Ejecutor:** opencode con `opencode/deepseek-v4.1-flash-free`, una vuelta, sin correcciones. El orquestador escribió la tarea, revisó el diff, hizo el rebase sobre `develop` (autorizado por el autor) y escribió esta entrada.

## 1. Solicitud

Tarea 5.6 de `docs/PLAN-FASE-5.md`. Al guardar las preferencias, se pide un ciclo del monitor si
cambió algo que el monitor usa, en lugar de esperar al siguiente ciclo programado.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El monitor lee las preferencias al empezar cada ciclo, y los ciclos van cada 15 minutos (entrada 007) | Al cambiar la hora de cierre o marcar un feriado, el tooltip y los avisos tardaban hasta 15 minutos en reflejarlo. Durante ese tiempo parecía que el cambio no había funcionado |

## 3. Diseño técnico

- **`Preferencias::afecta_al_monitor(&self, otra) -> bool`.** Es un método puro: `true` si cambian
  `avisos` (`activos`, `hora_cierre` o `resumen_lunes`) o `feriados_extra`.
  - Ignora `iniciar_con_windows` y `minimizar_a_bandeja`, que el monitor no usa. Así esos cambios
    no disparan una carga de horas innecesaria.
  - Las estructuras ya derivaban `PartialEq`.
- **`guardar_preferencias`.** Hace cuatro pasos, en este orden:
  1. Lee las preferencias anteriores. La primera vez, sin archivo, son las de por defecto.
  2. Guarda.
  3. Aplica el inicio con Windows.
  4. Llama a `vigilancia::pedir_ciclo(&app)` solo si `anteriores.afecta_al_monitor(&guardadas)`.

  Si guardar falla, el `?` sale antes y no se pide ningún ciclo. La comparación es contra lo que
  quedó guardado, ya normalizado, no contra lo que envió la interfaz.
- **Concurrencia.** `pedir_ciclo` usa el `Notify` del vigilante: si llegan varias peticiones
  seguidas, se juntan en una. El bucle nunca corre dos ciclos a la vez (entrada 007).

## 4. Archivos

- `src/preferencias.rs`: `afecta_al_monitor` y 8 pruebas.
- `src/comandos.rs`: cambia solo el cuerpo de `guardar_preferencias`.
- `docs/bitacora/README.md` (+1 fila) y esta entrada.

## 5. Cómo verificar

Corrido por el ejecutor en WSL, antes del rebase:
- `cargo test preferencias`: 16 pruebas en verde, 8 de ellas nuevas.
- `cargo test`: 143 de la librería y 1 de paridad, 0 fallos.
- `cargo clippy --all-targets`: sin advertencias.

Las pruebas nuevas cubren estos casos de `afecta_al_monitor`:
- Da `false` con preferencias iguales.
- Da `true` al cambiar la hora de cierre, el resumen semanal o `activos`, y al agregar o quitar un
  feriado.
- Da `false` al cambiar solo «Iniciar con Windows» o solo «Minimizar a la bandeja».

Las pruebas no se repitieron después del rebase sobre `develop` (que ya trae la 5.4 y la 5.5). Las
corre el autor antes del commit:

```bash
make check
```

No probado: que el aviso llegue sin esperar 15 minutos. El comando necesita `AppHandle` y no tiene
prueba automática. Se comprueba en la segunda pasada en Windows (tarea 5.7, puntos a y b).

## 6. Pendientes

- Cada cambio de feriado dispara un ciclo completo, es decir, una carga de tareas y horas. Con el
  simulado no importa. Con el Jira real (fase 6) hay que evaluar si conviene esperar unos segundos
  después del último cambio.
