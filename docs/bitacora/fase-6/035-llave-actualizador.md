# 035 — Llave del actualizador

**Fecha:** 2026-11-18
**Rama:** tarea/6.15-llave-actualizador → develop
**Fase:** 6
**Ejecutor:** el autor corrió `make updater-key`. El orquestador (Opus 5.5) creó la tarea, revisó el
cambio sin leer la llave privada, escribió esta entrada y corrió `check`, `commit`, `ship`, `finish`,
`promote` y `release`, a pedido del autor.

## 1. Solicitud

Publicar la v0.3.0, la primera que se actualiza sola. La 6.14 subió la versión, pero se fusionó (PR
#40) antes de crear la llave del actualizador: `tauri.conf.json` seguía con
`"pubkey": "PENDIENTE: ejecuta make updater-key"`, y sin llave no se puede publicar.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Sin llave pública en la app y sin secreto en GitHub | `make release` y el workflow se niegan; ninguna instalación podría verificar una actualización |

## 3. Diseño técnico

- **Una tarea solo para la llave**, porque el cambio de `tauri.conf.json` tiene que llegar a `main`
  por PR, como cualquier otro.
- **`make updater-key`, corrido por el autor.**
  - Al orquestador el sistema de permisos le negó el comando: crea una llave privada y sube un
    secreto a GitHub.
  - Hizo tres cosas:
    1. guardó la privada en `~/.tauri/signal-aduana.key`, fuera del repositorio;
    2. la subió como secreto `TAURI_SIGNING_PRIVATE_KEY`;
    3. escribió la pública en `plugins.updater.pubkey`.
  - El respaldo de la privada lo hace el autor.
- **Comprobaciones posteriores.** Las de `make release` (la `pubkey` de `main` y el secreto en el
  repositorio) confirman el resultado sin leer la llave privada.

## 4. Archivos

- `tauri.conf.json`: la llave pública del actualizador en `plugins.updater.pubkey`. Es lo único que
  cambia, y no hay archivos de llave en el repositorio.
- `docs/bitacora/fase-6/034-version-0-3-0.md`: una nota con fecha. La entrada decía que la llave se
  había creado en la 6.14, y no fue así.
- `docs/bitacora/README.md` (fila) y esta entrada.

## 5. Cómo verificar

Corrido por el orquestador:

```bash
grep -c PENDIENTE tauri.conf.json        # 0
python3 -m json.tool tauri.conf.json     # JSON válido
git status --short                       # solo tauri.conf.json (más la bitácora)
bash scripts/verificar-bitacora.sh
```

Lo que comprueban `make check`, `DRY_RUN=1 make release` (llave y secreto) y el workflow se
corre en esta misma sesión; ver Pendientes.

## 6. Pendientes

- **Publicar la v0.3.0**: promover y correr `make release`. Para el resultado del workflow, consultar
  el release en GitHub.
- **Instalar la 0.3.0 a mano**, por última vez.
- **Probar la actualización automática** con la 0.3.1, en la primera corrección de la fase.
- **Respaldar `~/.tauri/signal-aduana.key`**: lo hace el autor.
