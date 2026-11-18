# 011 — Script que verifica la bitácora

**Fecha:** 2026-11-16
**Rama:** tarea/4.9-verificar-bitacora → develop
**Fase:** 4
**Ejecutor:** opencode con `opencode/deepseek-v4.1-flash-free`, una vuelta. El orquestador enlazó el script en `CLAUDE.md` y en la skill de bitácora.

## 1. Solicitud

La entrada 009 documenta que un índice con marcas de conflicto de git llegó a `develop` pese a que la
bitácora se comprobaba a mano rama por rama. El autor aprobó esta tarea (4.9): un script que haga esas
comprobaciones sobre el resultado, y que falle si algo no cuadra.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Las comprobaciones se hacían a mano y miraban cada rama por separado | El resultado de fusionar dos ramas (índice con marcas de conflicto) pasó sin ser visto |

## 3. Diseño técnico

- **`scripts/verificar-bitacora.sh [raíz]`**: bash, solo lee (no crea, modifica ni borra nada en la raíz
  que verifica). Sin argumento usa la raíz de git, o el directorio padre de `scripts/` si no hay git.
  Siete comprobaciones, cada una con `✓` o `✗`; las advertencias `⚠` no cambian la salida:
  1. existe `docs/bitacora/README.md`;
  2. los enlaces del índice apuntan a archivos que existen;
  3. cada entrada tiene su fila en el índice (sin huérfanas);
  4. fase del encabezado igual a la de su carpeta, líneas **Fecha**, **Rama** y **Ejecutor**, y
     secciones 1 a 5 (la 6, Pendientes, es opcional y avisa);
  5. numeración única y correlativa desde 001;
  6. marcas de conflicto: `<<<<<<<`, `=======` y `>>>>>>>` en `docs/bitacora/`; solo las dos primeras en
     `CLAUDE.md` y `docs/*.md`, porque un `=======` puede ser el subrayado de un título de Markdown;
  7. URLs o correos en `docs/bitacora/` (la regla de la bitácora lo prohíbe).
- **Se prueba sobre el resultado**: el script acepta la raíz como argumento, de modo que se puede
  verificar un árbol fusionado o una copia temporal.
- **Enlazado por el orquestador**: línea en el bloque de comandos de `CLAUDE.md`, una viñeta en
  «Bitácora» que pide correrlo antes de entregar y tras cada rebase o fusión, y un punto en la lista de
  cierre de la skill.
- **Descartado**: ejecutarlo de forma automática en un gancho de git; el autor decide cuándo.
  Tampoco se tocó `scripts/` más allá de este archivo.

## 4. Archivos

- `scripts/verificar-bitacora.sh` (nuevo, 273 líneas).
- `CLAUDE.md` — el comando y la viñeta que pide correrlo.
- `.claude/skills/bitacora/SKILL.md` — punto en la lista de cierre.
- `docs/bitacora/README.md` (+1) y esta entrada.

## 5. Cómo verificar

Corridos por el ejecutor sobre copias temporales (su reporte): sobre el árbol con las marcas falla solo
por la comprobación 6; sin las marcas sale con 0, también con una ruta con espacios; y cada uno de
estos casos rotos hace fallar solo su comprobación: enlace roto, entrada huérfana, fase distinta,
número repetido, hueco de numeración, sección `## 3.` borrada, URL y correo.

Corridos de nuevo por el orquestador: `bash -n` sin errores; el script sobre el árbol de la 4.9 falla solo
por las 3 marcas heredadas de `develop`; **sobre el árbol de la 4.7 (que quita esas marcas) sale con 0 y
las siete comprobaciones en `✓`**; sobre el de la 4.8 falla por las marcas y por la entrada 009 que aún
no tiene; un caso roto propio (fase distinta y una URL) falla exactamente en las comprobaciones 4 y 7; y
el script no dejó cambios en los árboles verificados.

```bash
bash scripts/verificar-bitacora.sh; echo "salida=$?"
```

No probado: la rama «sin git» del cálculo de la raíz (el entorno no permitía una prueba sin git).

## 6. Pendientes

- Esta tarea no se ejecuta sola: hay que correr el script antes de entregar y tras cada rebase o fusión
  que toque el índice (lo pide `CLAUDE.md`).
- Requiere `bash` 4 o superior (usa `mapfile`); en macOS hay que usar un bash más nuevo.
