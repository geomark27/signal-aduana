# 012 — Formato oficial de `ui/app.js` con Prettier

**Fecha:** 2026-11-16
**Rama:** tarea/4.10-formato-ui → develop
**Fase:** 4
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta. El orquestador escribió la norma en `CLAUDE.md`, las filas del plan y la nota de la entrada 010.

## 1. Solicitud

Al preparar la 4.9, el orquestador vio que la 4.8 había llevado a `develop` el `ui/app.js` entero
reformateado: de 639 a 1798 líneas, 2211 cambiadas, en una tarea cuyo cambio eran 9 líneas (entrada
010). Se presentaron tres caminos: dejarlo, volver al estilo compacto original con una protección
contra la repetición, o adoptar el formato nuevo como oficial. El autor eligió **adoptarlo como el
oficial**.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Un formateador al guardar, sin configuración en el repo, reescribió todo `ui/app.js` | Un diff de 2211 líneas en una tarea de 9: historial y `blame` de la interfaz ruidosos, y revisión imposible |
| El formato dependía del editor de cada persona | Cada tarea que toque `ui/app.js` podía repetirlo o pelear con otro formato |

## 3. Diseño técnico

- **`.prettierrc.json` con `{ "tabWidth": 4 }`.** Con Prettier 3.10.1, `--check` sobre una copia de
  `ui/app.js` **falla sin opciones y pasa con `tabWidth` 4**: la configuración reproduce el formato
  actual exacto, sin cambiar una línea del archivo. Es coherente con que el formateador fuera
  Prettier con la sangría de 4 espacios del editor (inferencia: no se vio el editor).
- **Versión fijada** (`PRETTIER_VERSION := 3.10.1` en el `Makefile`) y ejecutada con `npx --yes`: el
  repo es de Rust y no se agregó `package.json` ni dependencias. Subir la versión es manual y puede
  cambiar el formato.
- **`.prettierignore` (`*`, `!*/`, `!ui/app.js`)**: Prettier actúa **solo** sobre `ui/app.js`, así que
  un formateo al guardar no reescribe Markdown (las tablas de la bitácora), JSON, HTML, CSS ni el
  `Makefile`. Probado en un directorio temporal con un `.json`, un `.md`, un `ui/otro.js` y un
  `ui/s.css` mal formateados: todos quedaron ignorados.
- **`ui/index.html` y `ui/styles.css` siguen en su estilo compacto**, fuera de Prettier. El autor
  eligió el formato de `app.js`; formatear más archivos tendría el mismo costo de diff y no se pidió.
- **Atajos**: `make ui-format` comprueba (`prettier --check`, con un mensaje que sugiere `make fmt-ui`)
  y entra en `make check` justo después de `ui-check`; `make fmt-ui` lo aplica.
- **Norma en `CLAUDE.md`**: `ui/app.js` sigue el formato de Prettier; Prettier solo actúa sobre ese
  archivo. El plan y la entrada 010 se actualizaron en la misma tarea.
- **Descartado**: volver al estilo compacto original (restaurar la versión previa más las 9 líneas y
  evitar el formateo con `.prettierignore`). El autor prefirió el formato nuevo.

## 4. Archivos

- `.prettierrc.json` (nuevo) — `tabWidth: 4`.
- `.prettierignore` (nuevo) — Prettier solo sobre `ui/app.js`.
- `Makefile` — `PRETTIER_VERSION`, `ui-format`, `fmt-ui` y `ui-format` dentro de `check`.
- `CLAUDE.md` — la norma y `check` actualizado.
- `docs/PLAN-FASE-4.md` — filas de la 4.8 y la 4.9 sin «en curso», y fila de la 4.10.
- `docs/bitacora/fase-4/010-aviso-inicio.md` — nota con fecha sobre el tamaño real del diff de la 4.8.
- `docs/bitacora/README.md` (+1) y esta entrada.
- `ui/app.js` **no cambia**.

## 5. Cómo verificar

Corridos por el ejecutor: `make ui-format` con salida 0; `make fmt-ui` deja `ui/app.js`
«unchanged» y `git diff --stat ui/app.js` vacío; `prettier --list-different .` sin archivos;
`make -n check` con el orden esperado; JSON válido; y en un directorio temporal una copia
desformateada hace fallar `make ui-format` con el mensaje sugerido (`make` informa 2 al fallar una
receta).

Corridos de nuevo por el orquestador: `ui/` idéntico a `develop`, `make ui-format` con salida 0,
`prettier --list-different .` vacío, el diff del `Makefile` leído completo y `make -n check` con
`ui-check`, `ui-format`, `bitacora`, `lint` y `test` en ese orden. El script de la bitácora sobre
esta rama sale con 0.

```bash
source ~/.nvm/nvm.sh
make ui-format
```

No probado: que **tu editor** respete `.prettierrc.json` y `.prettierignore` al guardar (depende de
su extensión); que `--check .` revise exactamente un archivo (se deduce de la prueba de aislamiento).

## 6. Pendientes

- **Red en la primera ejecución**: `npx --yes` baja Prettier a la caché de npm; sin red ni caché,
  `make ui-format` falla y, con él, `make check`.
- **Otros archivos de `ui/`**: `!ui/app.js` permite solo ese archivo. Si se quiere formatear más
  (por ejemplo `index.html` o `styles.css`), hay que ampliar `.prettierignore` y aceptar el diff de
  reformateo en una tarea aparte.
- Si cambia la versión de Prettier, volver a comprobar que `ui/app.js` sigue pasando.
