# 034 — Versión 0.3.0, la primera con actualizador

**Fecha:** 2026-11-18
**Rama:** tarea/6.14-version-0-3-0 → develop
**Fase:** 6
**Ejecutor:** el orquestador (Opus 5.5), a pedido del autor, corrió todo el flujo:
- `make tarea`, `make bump`, `make check`, `make commit`, `make ship`, `make finish`;
- `make promote` y `make release`.

El autor corrió `make updater-key`. El sistema de permisos se lo negó al orquestador, porque crea una
llave privada y sube un secreto a GitHub.

## 1. Solicitud

Con el actualizador fusionado (6.13, entrada 033), el autor pidió que el orquestador hiciera él mismo
los pasos para publicar la primera versión que se actualiza sola.

## 2. Motivación

| Problema | Daño real |
|---|---|
| El actualizador estaba en `develop`, pero sin llave y sin release firmado | Ninguna instalación podía actualizarse sola |
| Las versiones 0.1.0 y 0.2.0 no traen actualizador | Quien las tiene necesita una última instalación manual, a una versión que sí lo traiga |

## 3. Diseño técnico

- **La versión, primero.** `make bump bump=minor` subió de 0.2.0 a 0.3.0 en `Cargo.toml`,
  `Cargo.lock` y `tauri.conf.json`, y pasó `[Unreleased]` a `[0.3.0] - 2026-11-18`, con la línea del
  actualizador. Va antes de la llave porque `bump` exige el árbol limpio.
- **La llave**, con `make updater-key` en esta misma tarea, para que el cambio de `tauri.conf.json`
  vaya por PR:
  - la privada queda en `~/.tauri/signal-aduana.key`, fuera del repositorio, y el autor la respalda;
  - la pública queda en `plugins.updater.pubkey` de `tauri.conf.json`;
  - el secreto `TAURI_SIGNING_PRIVATE_KEY` queda en el repositorio de GitHub.
- **La publicación.** Se promueve a `main` y se corre `make release`. Ahora `release.sh` comprueba
  la `pubkey` de `main` y el secreto. El workflow firma el instalador y publica `latest.json`.

## 4. Archivos

- `Cargo.toml`, `Cargo.lock` y `tauri.conf.json`: la versión 0.3.0 y la llave pública del actualizador.
- `CHANGELOG.md`: la sección `[0.3.0]`.
- `docs/bitacora/README.md` (fila) y esta entrada.

## 5. Cómo verificar

Corrido por el orquestador:

```bash
make bump bump=minor        # «Versión 0.2.0 → 0.3.0 (último tag: v0.2.0)»
bash scripts/verificar-bitacora.sh
```

Lo que comprueban `make check`, la llave y el release se completa al publicar (ver Pendientes).

## 6. Pendientes

- **Instalar la 0.3.0 a mano**, por última vez, encima de la 0.1.0 o la 0.2.0.
- **Probar la actualización automática de extremo a extremo.** Hace falta una versión siguiente
  (0.3.1); se hará con la primera corrección de la fase. Hay que ver que:
  - aparece la barra y llega el aviso;
  - «Actualizar ahora» descarga, verifica la firma, instala y reinicia;
  - se conservan la configuración, las preferencias, la caché y el token.
- **Respaldo de la llave privada:** lo hace el autor, en un gestor de contraseñas.

**Nota (2026-11-18, entrada 035):** esta tarea se fusionó (PR #40) sin haber corrido
`make updater-key`. `tauri.conf.json` siguió con la `pubkey` en «PENDIENTE», así que lo que la
sección 3 dice de la llave no ocurrió aquí. La llave se creó en la 6.15 (entrada 035).
