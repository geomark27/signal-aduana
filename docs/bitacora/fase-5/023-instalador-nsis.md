# 023 — Instalador como Nudge: solo NSIS por usuario

**Fecha:** 2026-11-17
**Rama:** tarea/5.15-instalador-nsis → develop
**Fase:** 5
**Ejecutor:** Sonnet 5.5 (subagente), una vuelta, sin correcciones. El orquestador revisó la plantilla NSIS de Tauri y el plugin de autoarranque antes de escribir la tarea, revisó el diff y escribió esta entrada.

## 1. Solicitud

El autor pidió que la app se entregue como un `.exe`, siguiendo el modelo de Nudge, otra app Tauri
suya. Al desinstalar debe quedar solo lo útil para volver a instalarla. Corresponde a las
decisiones 2 y 5 de `docs/PLAN-FASE-5.md`.

## 2. Motivación

| Problema | Daño real |
|---|---|
| Se generaban dos instaladores (MSI y NSIS). El MSI puede pedir permisos de administrador | Dos caminos que mantener y probar, y un posible bloqueo en equipos sin permisos |
| El desinstalador de Tauri, por defecto, no borra nada de datos. Con su casilla marcada borra todo | Quedaban el perfil de WebView2 y el registro de avisos sin uso, o se perdían la configuración y las preferencias al reinstalar |

## 3. Diseño técnico

- **Referencia, Nudge (`~/kaelence/nudge`).**
  - Usa solo el objetivo `nsis` e instala por usuario en `%LOCALAPPDATA%`.
  - Publica releases en GitHub Actions con nombre fijo.
  - Tiene un `install.ps1` de una línea y usa el actualizador de Tauri.

  De todo eso se tomó ahora solo el instalador. La distribución y el actualizador quedan como
  referencia en la fase 7 del roadmap (7.2 y 7.3), porque dependen de Tecnología y de la visibilidad
  del repositorio.
- **`tauri.conf.json`.**
  - `targets: ["nsis"]`.
  - `bundle.windows.nsis` con `installMode: "currentUser"`, `languages: ["Spanish"]` (el
    instalador lo ve la persona usuaria, así que va en español) e `installerHooks:
    "windows/hooks.nsh"`.
- **`windows/hooks.nsh`.** Define `NSIS_HOOK_POSTUNINSTALL`. Solo cuando **no** es una
  actualización (`$UpdateMode <> 1`) borra:
  - `notifications.json` y `notifications.simulated.json`;
  - la carpeta `EBWebView`;
  - el valor `StartupApproved\Run\Signal Aduana`.

  Se conservan la configuración, las preferencias, la caché y la credencial, como decidió el autor.
- **Lo que ya hacía Tauri y se aprovecha.** Se revisó la plantilla `installer.nsi` de
  `tauri-bundler` 2.10.5:
  - ya borra el valor `HKCU\…\Run\${PRODUCTNAME}` cuando no es una actualización;
  - expone `$UpdateMode`, `${BUNDLEID}` y `${PRODUCTNAME}`;
  - su casilla «borrar datos» se mantiene. Si la persona la marca, se borran ambas carpetas completas;
    la credencial no se toca.
- **Autoarranque.** El plugin (`auto-launch` 0.6) escribe en `HKCU\…\Run` y también usa la clave
  `StartupApproved\Run`. Por eso el hook la limpia.
- **Guía.**
  - Un solo instalador y ninguna mención del MSI.
  - La ruta esperada del ejecutable es `%LOCALAPPDATA%\Signal Aduana\signal-aduana.exe`.
  - La sección 9 pasa a llamarse «Desinstalar» y explica qué se conserva y qué se borra. La
    limpieza manual queda como opcional.

## 4. Archivos

- `tauri.conf.json`: `targets` y `bundle.windows.nsis`.
- `windows/hooks.nsh` (nuevo).
- `docs/COMPILAR-WINDOWS.md`: secciones 3, 4, 5 y 9.
- `docs/bitacora/README.md` (+1 fila) y esta entrada.

## 5. Cómo verificar

Corrido por el ejecutor en WSL:
- `tauri.conf.json` es JSON válido.
- `cargo build` termina sin errores. El `build.rs` de Tauri valida la configuración contra su
  esquema y acepta `installMode`, `languages` e `installerHooks`.
- `cargo test`: 160 pruebas en verde. `cargo clippy --all-targets`: sin advertencias.
- Ninguna mención de MSI en la configuración ni en la guía.

**No probado: todo lo de NSIS**, porque en WSL no hay `makensis`. Lo comprueba el autor en Windows:

```powershell
cargo tauri build                      # debe generar solo ...\bundle\nsis\*-setup.exe
# instalar: en español, sin pedir administrador, en %LOCALAPPDATA%\Signal Aduana
# usar la app en modo simulado y luego desinstalar desde «Aplicaciones instaladas»
Get-ChildItem "$env:APPDATA\ec.com.torresytorres.signal"        # quedan config y preferencias
Get-ChildItem "$env:LOCALAPPDATA\ec.com.torresytorres.signal"   # queda cache; no notifications ni EBWebView
```

## 6. Pendientes

- Confirmar en Windows:
  - que compila el instalador con el hook;
  - el nombre del `-setup.exe` y la ruta de instalación;
  - que el instalador sale en español y no pide administrador;
  - que al desinstalar se conserva y se borra lo indicado;
  - que la casilla «borrar datos» funciona.
- El nombre real del valor de autoarranque solo se ve en modo real, en la fase 6.
- Si se reinstala con el instalador sobre una versión ya instalada, sin pasar por el actualizador,
  puede correr el desinstalador anterior sin `/UPDATE` y borrar el registro de avisos y el perfil de
  WebView2. No se pierde nada de lo que se decidió conservar.
