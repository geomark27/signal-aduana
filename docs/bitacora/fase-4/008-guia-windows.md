# 008 — Guía para compilar, instalar y probar en Windows

**Fecha:** 2026-11-16
**Rama:** tarea/4.6-guia-windows → develop
**Fase:** 4
**Ejecutor:** opencode con `opencode/deepseek-v4.1-flash-free`, una vuelta. El orquestador cambió una línea de la guía (recomendación sobre el antivirus) y una frase de `CLAUDE.md`.

## 1. Solicitud

Punto 4 de la tarea 3 de `docs/PLAN-FASE-4.md`: una guía para compilar el instalador en Windows,
porque desde WSL no se puede compilar ni probar. El orquestador la separó como tarea 4.6 de la
división de la tarea 3 (aprobada por el autor).

## 2. Motivación

| Problema | Daño real |
|---|---|
| El instalador, la bandeja, las notificaciones y el inicio con Windows solo se comprueban en Windows | Sin una guía, el autor reconstruye a mano los pasos, las rutas y qué probar, y se olvidan casos (por ejemplo, el arranque con `--oculto` o la migración de feriados) |
| Quedan datos fuera del desinstalador (credencial, carpetas, entrada de inicio) | Restos con un token de Jira en el equipo tras desinstalar |

## 3. Diseño técnico

- **Diez secciones**: requisitos, copiar el proyecto fuera de `\\wsl$`, compilar, SmartScreen, modo
  simulado, Jira real, lista de comprobación, dónde quedan los datos, desinstalar y problemas
  frecuentes. 297 líneas, con comandos de PowerShell y una línea que explica cada uno.
- **Los datos los puso el orquestador, verificados en el código**: identificador de la app, nombres de
  archivo (`config.toml`, `preferencias.toml`, `cache.json`, `avisos.json` y sus variantes
  `.simulado.`), la credencial `signal-aduana:<host>`, que el modo simulado es el predeterminado en
  debug y el real en `release`, y el `downloadBootstrapper` de WebView2. El modelo no tenía acceso a
  internet ni a la documentación de Tauri.
- **Lo no confirmable queda «a verificar»** (6 puntos) en lugar de inventarse: nombre de la descarga
  de Build Tools, duración de la primera compilación, rutas y nombres exactos de los instaladores,
  ruta del ejecutable instalado, carpeta del perfil de WebView2 y nombre del valor de inicio en el
  Registro.
- **La lista de comprobación reúne los pendientes de las entradas 002, 005, 006 y 007** (ícono y menú,
  cerrar a la bandeja con y sin preferencia, instancia única, `--oculto`, inicio con Windows, los tres
  avisos y la migración de feriados). Describe el estado final de la fase 4; algunas funciones (avisos
  e inicio con Windows) llegan con la 4.5.
- **Ejemplos ficticios** (un sitio y un correo de ejemplo, ninguno real); el token nunca se escribe.
- **Correcciones del orquestador**: el modelo recomendaba añadir una exclusión al antivirus por cuenta
  propia; en un equipo de la empresa eso se pide a Tecnología. Y `CLAUDE.md` deja de decir «cuando
  exista» al citar la guía.

## 4. Archivos

- `docs/COMPILAR-WINDOWS.md` (nuevo, 297 líneas).
- `CLAUDE.md` — la referencia a la guía ya no dice «cuando exista».
- `docs/bitacora/README.md` (+1) y esta entrada.

## 5. Cómo verificar

Corridos por el ejecutor (su reporte): `grep -c '^## '` da 10; seis marcas «a verificar»; `wc -l` da
297; las coincidencias de `token`, `@` y `http` son solo palabras en instrucciones y los dos ejemplos
ficticios. El orquestador leyó el documento completo y comprobó que cada dato de su tabla aparece sin
cambios y que no hay datos reales. No hay código que compilar ni probar en esta tarea.

## 6. Pendientes

- **Ejecutar la guía en un equipo Windows** y resolver los seis «a verificar» con lo que se vea.
- La guía no se ha seguido nunca de principio a fin: puede haber pasos que fallen.
- La lista de comprobación asume la 4.5 fusionada (avisos e inicio con Windows).
