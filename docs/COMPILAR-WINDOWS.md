# Compilar, instalar y probar Signal Aduana en Windows

Signal Aduana es una app de escritorio hecha con Tauri 2 + Rust. El instalador se compila y se
prueba **en Windows**; no se compila ni se prueba desde WSL ni sobre una ruta `\\wsl$`.

El proyecto **no tiene carpeta `src-tauri`**: `tauri.conf.json`, `Cargo.toml` y `capabilities\` están
en la raíz del repositorio. Todos los comandos de esta guía se ejecutan desde esa raíz.

Producto: «Signal Aduana» 0.1.0. Identificador: `ec.com.torresytorres.signal`. Paquete y binario:
`signal-aduana` (librería `signal_aduana`).

## 1. Requisitos

Instala y comprueba estas piezas antes de compilar:

- **Rust con rustup** y la toolchain MSVC `stable-x86_64-pc-windows-msvc`. La versión mínima es
  **1.80**, la que declara `Cargo.toml` (`rust-version = "1.80"`).
- **Visual Studio Build Tools** con la carga de trabajo «Desarrollo de escritorio con C++». Aporta
  el compilador y el enlazador de C++ que Rust necesita en Windows.
- **WebView2**, que ya viene en Windows 10 y Windows 11 actualizados. El instalador usa
  `downloadBootstrapper`: si el equipo no lo tiene, lo descarga durante la instalación y para eso
  necesita internet.
- **Tauri CLI 2**, instalado con Cargo.

Muestra las toolchains instaladas; debe aparecer `stable-x86_64-pc-windows-msvc`:

```powershell
rustup show
```

Muestra la versión de Rust; debe ser 1.80 o mayor:

```powershell
rustc --version
```

Si falta la toolchain MSVC, la establece como predeterminada:

```powershell
rustup default stable-x86_64-pc-windows-msvc
```

Instala la CLI de Tauri 2, necesaria para `cargo tauri`:

```powershell
cargo install tauri-cli --version "^2"
```

Para Visual Studio Build Tools, abre **Visual Studio Installer**, elige «Modificar» y marca
«Desarrollo de escritorio con C++». El nombre exacto de la descarga y del instalador puede variar
según la versión: a verificar en el equipo.

## 2. Copiar el proyecto a una carpeta de Windows

No compiles sobre `\\wsl$`: las rutas de red son lentas y algunos pasos del empaquetado fallan con
ellas. Copia el proyecto a una carpeta local, por ejemplo `C:\dev\signal-aduana`.

Copia el proyecto desde WSL sin `target\`, `.git` ni los worktrees anidados. Cambia `<distro>` y
`<usuario>` por los tuyos; si el repositorio está en otra ruta, ajusta el origen. El comando crea
la carpeta de destino:

```powershell
robocopy "\\wsl$\<distro>\home\<usuario>\signal-aduana" "C:\dev\signal-aduana" /E /XD target .git .worktrees
```

Entra en la carpeta copiada:

```powershell
Set-Location C:\dev\signal-aduana
```

Confirma que estás en la raíz correcta; debe responder `True` (no existe `src-tauri`):

```powershell
Test-Path tauri.conf.json
```

## 3. Compilar el instalador

`cargo tauri build` compila la app en modo `release` y genera los instaladores. La primera vez
compila todas las dependencias en release y puede tardar bastante; no hay una cifra confirmada de
duración: a verificar en el primer equipo. Un `release` usa el **modo real** de Jira salvo que
fuerces el modo simulado con `SIGNAL_SIMULADO=1`.

Compila y empaqueta:

```powershell
cargo tauri build
```

El proyecto genera dos instaladores: `nsis` y `msi`. Como no hay `src-tauri`, la salida queda en la
raíz del proyecto:

- `target\release\bundle\msi\Signal Aduana_0.1.0_x64_en-US.msi`
- `target\release\bundle\nsis\Signal Aduana_0.1.0_x64-setup.exe`

Las rutas y los nombres exactos de estos archivos están **a verificar** en la primera compilación:
no se pudieron generar desde WSL. Lista lo que se produjo:

```powershell
Get-ChildItem .\target\release\bundle -Recurse -Include *.msi,*setup.exe | Select-Object FullName
```

Instala con uno de los dos archivos (doble clic y sigue el asistente). Los dos instalan la misma app;
elige uno. El instalador no está firmado.

## 4. Aviso de SmartScreen

El instalador no tiene firma de código, así que Windows puede mostrar «Windows protegió su equipo».
No es un error de la compilación: es la advertencia normal para un ejecutable sin firmar.

Para continuar, pulsa «Más información» y luego «Ejecutar de todas formas». Si el instalador es un
`.msi`, Windows también puede pedir confirmación de administrador. La firma de código del instalador
está fuera del alcance de esta fase.

## 5. Probar primero en modo simulado

Haz la primera prueba con datos ficticios, sin tocar Jira. `SIGNAL_SIMULADO=1` fuerza el modo
simulado y `SIGNAL_SIMULADO=0` el real; sin definir, una compilación debug arranca simulada y una
release arranca real. La variable solo afecta a los procesos que se abren desde esa misma ventana de
PowerShell; si abres la app desde el menú Inicio o con doble clic, no llega a la app.

Localiza el ejecutable instalado; la ruta exacta depende del instalador usado y está a verificar:

```powershell
Get-ChildItem "$env:LOCALAPPDATA\Programs","C:\Program Files" -Recurse -Filter signal-aduana.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
```

Activa el modo simulado en esta ventana de PowerShell:

```powershell
$env:SIGNAL_SIMULADO = "1"
```

Abre la app instalada desde esta misma ventana, usando la ruta que encontraste antes:

```powershell
Start-Process "<ruta al ejecutable>\signal-aduana.exe"
```

En modo simulado debes ver:

- El título de la ventana «Signal Aduana — modo simulado».
- Avisos con el prefijo «[Simulado]».
- El inicio con Windows **no** se registra; solo queda constancia en el log.
- Los archivos de datos con `.simulado.` en el nombre (`config.simulado.toml`,
  `preferencias.simulado.toml`, `cache.simulado.json`, `avisos.simulado.json`).
- Ninguna petición a Jira.

Para volver al modo real en esa ventana, quita la variable:

```powershell
Remove-Item Env:\SIGNAL_SIMULADO
```

## 6. Probar con el Jira real

Esta prueba la hace solo el autor, con su propio token de API clásico de Jira. **Nunca** pegues el
token en un chat, en un archivo, en una captura ni en un commit. El token se guarda en el
Administrador de credenciales de Windows con el nombre `signal-aduana:<host de Jira>`; la
configuración `config.toml` guarda el sitio y el correo, nunca el token.

Un `release` ya usa el modo real. Asegúrate de que la variable no esté definida en esta ventana:

```powershell
Remove-Item Env:\SIGNAL_SIMULADO -ErrorAction SilentlyContinue
```

Abre la app instalada desde el menú Inicio o con doble clic y escribe los datos. En los ejemplos usa
valores ficticios como estos; en la app usa los tuyos:

- Sitio: `https://empresa.atlassian.net`
- Correo: `persona@empresa.com`
- Token: el token de API clásico de tu cuenta, escrito solo en la app.

Si el token se revoca o vence, la app deja de leer Jira y muestra el aviso de reconexión. Si sospechas
que el token quedó expuesto, revócalo en tu cuenta de Atlassian y crea otro.

## 7. Lista de comprobación en Windows

Esto es lo que no se pudo probar en WSL. Marca cada punto con la app instalada y en una sesión de
Windows normal.

- [ ] El ícono de Signal Aduana aparece en la bandeja del sistema.
- [ ] El tooltip muestra el resumen con el formato «Semana: X de 40 h · Hoy: Y de 8 h».
- [ ] El menú de la bandeja tiene «Abrir Signal Aduana», «Actualizar ahora», un separador y «Salir».
- [ ] El clic izquierdo en el ícono abre la ventana y la trae al frente.
- [ ] «Actualizar ahora» refresca los datos y el tooltip sin cerrar la app.
- [ ] Con «Minimizar a la bandeja» activo, cerrar la ventana la esconde y la app sigue viva.
- [ ] Con «Minimizar a la bandeja» desactivado, cerrar la ventana termina el proceso.
- [ ] Abrir la app una segunda vez trae al frente la ventana existente y no crea otro proceso.
- [ ] El arranque con `--oculto` (acceso directo con ese argumento) empieza sin ventana visible y deja
      el ícono en la bandeja.
- [ ] Al activar «Iniciar con Windows», cerrar sesión de Windows y volver a entrar, la app arranca
      oculta (solo bandeja).
- [ ] Llega el aviso de cierre de jornada cuando pasa la hora configurada y faltan horas del día.
- [ ] Llega el resumen del lunes cuando la semana anterior quedó incompleta.
- [ ] Llega el aviso de reconexión cuando el token fue revocado.
- [ ] Los feriados marcados antes en el navegador aparecen migrados en las preferencias
      (`feriadosExtra` en `preferencias.toml`).

Prueba el arranque oculto con este comando; no debe aparecer la ventana:

```powershell
Start-Process "<ruta al ejecutable>\signal-aduana.exe" -ArgumentList "--oculto"
```

Comprueba que solo queda un proceso; debe aparecer una sola fila:

```powershell
Get-Process signal-aduana -ErrorAction SilentlyContinue | Select-Object Id, MainWindowTitle
```

El ciclo del monitor corre a los 30 segundos de arrancar y luego cada 15 minutos, así que espera ese
tiempo antes de dar por fallido un aviso.

## 8. Dónde quedan los datos

| Qué | Ruta |
|---|---|
| Configuración (sitio y correo, nunca el token) | `%APPDATA%\ec.com.torresytorres.signal\config.toml` |
| Preferencias (avisos, inicio con Windows, bandeja, feriados) | `%APPDATA%\ec.com.torresytorres.signal\preferencias.toml` |
| Datos locales: última carga | `%LOCALAPPDATA%\ec.com.torresytorres.signal\cache.json` |
| Datos locales: avisos ya enviados hoy | `%LOCALAPPDATA%\ec.com.torresytorres.signal\avisos.json` |
| Credencial del token de Jira | Administrador de credenciales de Windows, nombre `signal-aduana:<host>` |
| Archivos del modo simulado | Los mismos, con `.simulado.` en el nombre |

Abre la carpeta de configuración y preferencias:

```powershell
explorer "$env:APPDATA\ec.com.torresytorres.signal"
```

Abre la carpeta de datos locales (caché y avisos):

```powershell
explorer "$env:LOCALAPPDATA\ec.com.torresytorres.signal"
```

Lista las credenciales; busca una cuyo nombre empiece por `signal-aduana:`. No muestra la contraseña:

```powershell
cmdkey /list
```

## 9. Desinstalar y limpiar todo

Desinstala desde «Aplicaciones instaladas» (Configuración > Aplicaciones) y luego borra los datos,
la credencial y el inicio con Windows. El desinstalador no borra por sí solo todos estos restos.

Borra la configuración y las preferencias, incluidos los archivos simulados:

```powershell
Remove-Item -Recurse -Force "$env:APPDATA\ec.com.torresytorres.signal"
```

Borra la caché, los avisos y, si existe ahí, el perfil de WebView2:

```powershell
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\ec.com.torresytorres.signal"
```

La carpeta del perfil de WebView2 puede estar dentro de la carpeta de datos locales (por ejemplo,
una carpeta como `EBWebView`); su nombre exacto está a verificar. Si está ahí, el comando anterior
la borra junto con el resto; si está en otro sitio, búscala en el Explorador y bórrala.

Borra la credencial del host que usaste; cambia el ejemplo ficticio por tu host real:

```powershell
cmdkey /delete:signal-aduana:empresa.atlassian.net
```

Si usaste más de un host de Jira, repite el comando para cada `signal-aduana:<host>` que aparezca en
`cmdkey /list`.

Revisa la entrada de inicio con Windows; debe estar en la clave del usuario actual:

```powershell
Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
```

Si queda una entrada que apunta a Signal Aduana, bórrala con su nombre exacto (está a verificar; no
borres valores de otras aplicaciones):

```powershell
Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "<nombre exacto>"
```

## 10. Problemas frecuentes

| Síntoma | Causa probable | Solución |
|---|---|---|
| Al compilar: no se encuentra `link.exe` o falta el compilador de C++ | Visual Studio Build Tools sin la carga de trabajo «Desarrollo de escritorio con C++» | Instala esa carga de trabajo en Visual Studio Installer y abre una terminal nueva. |
| La app abre con la ventana en blanco y no hay internet | WebView2 ausente y el `downloadBootstrapper` no pudo descargarlo | Conecta el equipo a internet y vuelve a ejecutar el instalador; si sigue igual, instala WebView2 y repite. |
| El antivirus bloquea o elimina el `.exe` | El instalador no está firmado | Pide a Tecnología una exclusión para el ejecutable o la carpeta del proyecto; no la agregues por tu cuenta ni desactives la protección del equipo. |
| El ícono no aparece en la bandeja | Windows lo tiene en «íconos ocultos» | Pulsa la flecha de la bandeja y arrastra el ícono a la zona visible; revisa la configuración de la barra de tareas. |
| No aparecen los avisos de Windows | «No molestar» o el asistente de concentración está activo | Desactiva ese modo y revisa que Signal Aduana tenga permiso para mostrar notificaciones en Configuración > Sistema > Notificaciones. |
