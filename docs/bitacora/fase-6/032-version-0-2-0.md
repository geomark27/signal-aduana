# 032 — Versión 0.2.0

**Fecha:** 2026-11-18
**Rama:** tarea/6.12-version-0-2-0 → develop
**Fase:** 6
**Ejecutor:** el autor. Esta es una **entrada retroactiva**: la escribió el orquestador en la rama del
actualizador (6.13), porque el PR #37 se fusionó sin entrada, como ya pasó con la 024.

## 1. Solicitud

Con el rediseño fusionado (6.10 y 6.11), el autor publicó la versión 0.2.0. Usó el flujo de la 6.8:
una tarea con `make bump bump=minor`, después `make promote` y luego `make release`.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El rediseño estaba en `develop`, pero la versión instalada seguía siendo la 0.1.0 | Las pestañas y las listas nuevas no le llegaban a quien usa la app |

## 3. Diseño técnico

- **Primer intento.** Primero se promovió `develop` a `main` sin subir la versión (PR #36), y
  `make release bump=minor` se detuvo con «v0.1.0 ya existe». Dos detalles:
  - `make release` ignoraba `bump=`, porque no sube versiones;
  - `main` todavía decía 0.1.0.

  Nada quedó a medias: no se crearon tag ni release.
- **Cómo se resolvió.** El autor hizo la tarea de versión (PR #37, `Tarea: 6.12 version 0 2 0`), una
  segunda promoción (PR #38) y `make release`. El workflow compiló y publicó en 8 min 3 s.
- **Para que no se repita**, la 6.13 hace que `make release` rechace `bump=` y `ver=` con un mensaje
  que manda a `make bump` en una tarea.
- **Número de tarea.** El orquestador había usado el mismo número, 6.12, para el actualizador sin ver
  que ya estaba tomado. El actualizador pasó a ser la 6.13, y `CLAUDE.md` ahora pide revisar el mapa
  de tareas antes de `make tarea`.

## 4. Archivos

Commit `5453309` del autor:
- `Cargo.toml`, `Cargo.lock` y `tauri.conf.json`: la versión pasa de 0.1.0 a 0.2.0.
- `CHANGELOG.md`: `[Unreleased]` pasa a `[0.2.0] - 2026-11-18`, con las dos líneas del rediseño.

Esta entrada y su fila en el índice viajan en la rama de la 6.13.

## 5. Cómo verificar

Revisado por el orquestador el 2026-11-18:
- en el remoto existen los tags `v0.1.0` y `v0.2.0`;
- `gh release list` muestra «Signal Aduana v0.2.0» como «Latest»;
- el workflow `Release` de `v0.2.0` terminó con `success`;
- en `develop`, `Cargo.toml` dice `0.2.0`.

## 6. Pendientes

- La 0.2.0 no trae actualizador. Quien la tenga instalada tiene que instalar a mano la primera
  versión que lo traiga (la 0.3.0, en la tarea 6.14).
- La actualización encima de una versión instalada (de 0.1.0 a 0.2.0) conserva la configuración, las
  preferencias, la caché y el token: lo confirma el autor en Windows.
