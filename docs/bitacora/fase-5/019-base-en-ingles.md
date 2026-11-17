# 019 — Base de la app en inglés

**Fecha:** 2026-11-17
**Rama:** tarea/5.12-base-en-ingles → develop
**Fase:** 5
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta, sin correcciones. El orquestador revisó el diff, actualizó `CLAUDE.md` y escribió esta entrada.

## 1. Solicitud

Tarea 5.12 de `docs/PLAN-FASE-5.md`: pasar al inglés la base de la app (errores, almacén de
credenciales, autenticación, config, caché y sesión) siguiendo `docs/GLOSARIO.md` (sección 4).
Incluye la decisión del autor de la 5.9: los códigos de error pasan al inglés y la interfaz deja
de mostrarlos.

## 2. Motivación

| Problema | Daño real |
|---|---|
| `ErrorApp`, `Resultado`, `Estado` y `Sesion` estaban en español y se usan en casi todos los archivos | Era la mayor parte de lo que quedaba en español en el código |
| El error, la sesión y la caché cruzan a la interfaz (`codigo`, `mensaje`, `conectado`, `usuario`, `guardado`, `datos`, `formato`) | Si se cambiaba un solo lado, se perdían la pantalla de conexión, la detección de token vencido o la caché |
| La interfaz añadía « [código]» al final de cada mensaje de error | Con los códigos en inglés, la persona habría visto `[unauthorized]` junto a un mensaje en español |

## 3. Diseño técnico

- **Archivos.** `almacen.rs` pasa a `secret_store.rs` y `sesion.rs` a `session.rs`. `error.rs`,
  `auth.rs`, `config.rs` y `cache.rs` quedan traducidos completos, con sus comentarios, pruebas y
  textos para quien programa.
- **Nombres nuevos:** `AppError`, `AppResult`, `SecretStore`, `Site`, `Credentials`, `AppState`,
  `Session`, `SessionInfo` y `Snapshot`.
- **Error.** El JSON pasa a `{ code, message }` con los códigos del glosario (`unauthorized`,
  `no_session`, `network`, `rate_limit`, `format`…). Los mensajes `#[error]` son idénticos.
- **Decisión del autor.** `errText` devuelve solo el mensaje, recortado a 300 caracteres. Es el único
  cambio de comportamiento visible de toda la traducción.
- **Sesión.** Las claves de `SessionInfo` pasan a `connected`, `simulated`, `user`, `site`, `email` y
  `notice`. El token de prueba sigue siendo «rechazar».
- **Caché.**
  - `Snapshot` pasa a `{ savedAt, data }`. El archivo usa `site`, `saved_at` y `data`.
  - El cuerpo que escribe la interfaz usa `format`. Una caché anterior no tiene esa clave: se descarta
    y se vuelve a cargar, como estaba previsto, sin subir `CACHE_FORMAT`.
- **Disco.** Las variantes simuladas pasan a `config.simulated.toml` y `cache.simulated.json`.
  `config.toml`, `cache.json` y el servicio de la credencial `signal-aduana:<host>` no cambian.
- **Resto del código.** Solo cambiaron las referencias. Los comandos de Tauri, sus parámetros y los
  módulos de escritorio y avisos son de la 5.13.
- **`Debug` de las credenciales.** El marcador que tapa el token pasó de `[oculto]` a `[hidden]`.
  Solo aparece en la salida de depuración, nunca en pantalla, así que cuenta como texto para quien
  programa. La prueba que comprueba que el token no se muestra sigue igual.

## 4. Archivos

- `src/error.rs`, `src/auth.rs`, `src/config.rs` y `src/cache.rs`, traducidos.
- `src/secret_store.rs` (antes `almacen.rs`) y `src/session.rs` (antes `sesion.rs`).
- `src/lib.rs` y las referencias en el resto de `src/` (`comandos`, `monitor`, `vigilancia`,
  `avisos`, `preferencias`, `resumen`, `escritorio`, `inicio`, `domain/*`, `jira/*`).
- `ui/app.js`: error y `errText`, `SessionInfo`, `Snapshot` y la clave `format` de la caché.
- `docs/COMPILAR-WINDOWS.md`: los dos nombres de archivo simulados.
- `CLAUDE.md`: el árbol de `src/` (`secret_store`, `session`).
- `docs/bitacora/README.md` (+1 fila) y esta entrada.

## 5. Cómo verificar

Corrido por el ejecutor en WSL:
- `cargo test`: 160 pruebas en verde (159 de la librería y 1 de paridad), las mismas que antes de
  la tarea.
- `cargo clippy --all-targets`: sin advertencias. `node --check ui/app.js` y `make ui-format`
  pasan.
- Comparó los literales de los seis archivos con los anteriores. Solo cambian:
  - los códigos y las claves JSON y TOML;
  - los nombres de los archivos simulados;
  - los nombres dentro de `format!`, que dejan el texto mostrado igual;
  - los textos para quien programa.

  Los mensajes de error visibles, «rechazar» y `signal-aduana:<host>` son idénticos.
- El `grep` de nombres viejos solo encuentra `texto_sin_sesion` y nombres de pruebas que contienen
  `sin_sesion`. Son de la 5.13.

El ejecutor no pudo contar las pruebas antes de empezar (su primera corrida falló por las
comillas). Se toma la cifra de `develop` (160).

Corrido por el orquestador: la lectura del diff completo de `ui/app.js`. Ningún texto visible
cambió y los contratos cambiaron a la vez en los dos lados.

Pendiente del autor antes del commit:

```bash
make check
```

No probado: la pantalla de conexión y un token vencido en la app real (sin el código al final del
mensaje). Se ve en Windows o al levantar la app.

## 6. Pendientes

- Quedan para la 5.13: `texto_sin_sesion`, los parámetros `estado`, `sitio`, `correo` y `datos`
  de los comandos y los nombres de los comandos.
- El comentario de `ui/app.js` que explica `handleAuth` sigue en español. Se traduce en la 5.14.
