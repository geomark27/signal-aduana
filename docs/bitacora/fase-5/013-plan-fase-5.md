# 013 — Plan de la fase 5 y roadmap

**Fecha:** 2026-11-17
**Rama:** tarea/5.0-plan-fase-5 → develop
**Fase:** 5
**Ejecutor:** orquestador (Opus 5.5), una vuelta.

## 1. Solicitud

Con la fase 4 cerrada, el autor pidió arrancar la siguiente siguiendo el roadmap y el plan de la
fase 5, con Claude (Opus) como líder que planifica y revisa, y con la implementación delegada en
modelos más baratos: Sonnet 5.5 con esfuerzo medio o modelos gratuitos de opencode. Esta tarea
registra en git los documentos de la fase con las decisiones que el autor tomó al aprobar el plan.

## 2. Motivación

| Problema | Daño real |
|---|---|
| `docs/ROADMAP.md` y `docs/PLAN-FASE-5.md` existían solo en la carpeta principal, sin registrar | Los worktrees de la ola 1 nacen de `develop` y los ejecutores no los habrían visto |
| `.claude/delegar.json` apuntaba a `PLAN-FASE-4.md` | La skill `delegar` les habría pasado a los ejecutores el plan de una fase cerrada |
| El resumen semanal solo mira si el día es lunes (`src/avisos.rs`) | Si el lunes es feriado, el aviso llega en el día libre; si ese día la PC está apagada, el resumen de la semana se pierde |

## 3. Diseño técnico

- **Decisión del autor 1 (lunes feriado): el resumen semanal sale el primer día laborable de la
  semana**, desde las 09:00. El orquestador explicó el comportamiento actual con el código y
  presentó tres opciones:
  - Dejarlo como estaba: avisa en el día libre y se pierde si ese día la PC está apagada.
  - Omitirlo los lunes feriado: esa semana no habría resumen.
  - Primer día laborable: es la que eligió el autor.
- **Cómo se implementará (tarea 5.4):**
  - `Resumen` gana el campo `primer_laborable`, calculado con
    `dominio::calendario::dias_laborables`, y `decidir` lo compara con hoy en lugar de comprobar el
    lunes. La regla de feriados no se duplica.
  - Por compatibilidad no se renombran `ResumenLunes`, `Registro.lunes` (está en el JSON en disco)
    ni `resumen_lunes`.
  - Con este cambio, la 5.4 deja de ser una tarea solo de pruebas.
- **Decisión del autor 4 (ejecutores): mixto.**
  - La 5.4 (cambia la lógica de avisos) y la 5.5 (concurrencia del monitor) van a subagentes
    Sonnet 5.5 medio.
  - La 5.6 (una función pura y una llamada en un comando) va a un modelo `opencode/*-free`, con
    datos ficticios.
  - El autor había propuesto dar la 5.4 al modelo gratuito porque eran solo pruebas. Cuando pasó a
    cambiar lógica, se intercambió con la 5.6.
- **Comprobaciones previas:** antes de aprobar se confirmó en el código lo que el plan daba por
  hecho:
  - `pedir_ciclo`, `ESPERA_INICIAL` e `INTERVALO` en `src/vigilancia.rs`.
  - `MAX_TEXTO_BANDEJA` en `src/resumen.rs`.
  - `JiraSimulado::nuevo(correo, token_valido, hoy)`.
  - `futures` en `Cargo.toml`.
  - Los módulos `pruebas` de los archivos que se tocan.
  - En `FeriadosExtra`, `true` agrega un feriado (`es_feriado`).
  - Las estructuras `Resumen` escritas a mano solo aparecen en las pruebas de `avisos.rs` y
    `resumen.rs`, ambas de la 5.4. Así, las tres tareas de la ola 1 no comparten archivos.
- `delegar.json` agrega `docs/ROADMAP.md` a `archivos_orquestador`: solo el orquestador escribe el
  roadmap.

## 4. Archivos

- `docs/ROADMAP.md` (nuevo en git): estado aprobado, fase 5 en curso y la 5.4 con el cambio del
  resumen semanal.
- `docs/PLAN-FASE-5.md` (nuevo en git):
  - estado aprobado;
  - la 5.4 redactada de nuevo, con el cambio de lógica y 11 pruebas;
  - los ejecutores en el mapa de tareas y en la ola 1;
  - el punto (c) en la segunda pasada de la 5.7;
  - las decisiones 1 y 4 resueltas.
- `.claude/delegar.json`: `PLAN-FASE-4.md` → `PLAN-FASE-5.md` en `instrucciones` y en
  `archivos_orquestador`, y se agrega `docs/ROADMAP.md`.
- `docs/bitacora/README.md` (sección y fila de la fase 5) y esta entrada.

## 5. Cómo verificar

Corrido por el orquestador en esta rama:
- `bash scripts/verificar-bitacora.sh`: 7 comprobaciones, 0 fallos y salida 0. Hay 13 entradas,
  numeradas de la 001 a la 013.
- `delegar.json`: JSON válido.

```bash
bash scripts/verificar-bitacora.sh
python3 -m json.tool .claude/delegar.json > /dev/null
```

No hay cambios de código: no se corrió `cargo test` ni `make check` para esta tarea.

## 6. Pendientes

- Las tareas 5.1 a 5.3 (Windows) las hace el autor; sus resultados van a la entrada de la 5.7.
- La decisión 2 (qué instalador se entrega) espera los resultados de la 5.1.
- Renombrar `ResumenLunes`, `Registro.lunes` y `resumen_lunes` para que digan «resumen semanal»
  requiere migrar el JSON del registro y las preferencias. No entra en la fase 5.
