#!/usr/bin/env bash
# Verifica la bitácora de docs/bitacora/. Solo lee: no escribe ni borra nada
# dentro de la raíz que comprueba.
#
# Uso: bash scripts/verificar-bitacora.sh [raíz]
# Sin argumento usa la raíz del repositorio git; si no hay git, el directorio
# padre de scripts/. Sale con 1 si hay algún fallo y con 0 si no.

set -u

fallos=0
advertencias=0
comprobaciones=0
detalles=()
entradas=()
enlaces=()
numeros=()

ok() {
  printf '✓ %s\n' "$1"
  detalles=()
}

fallo() {
  printf '✗ %s\n' "$1"
  fallos=$((fallos + 1))
  local detalle
  if [ "${#detalles[@]}" -gt 0 ]; then
    for detalle in "${detalles[@]}"; do
      printf '    %s\n' "$detalle"
    done
  fi
  detalles=()
}

aviso() {
  printf '⚠ %s\n' "$1"
  advertencias=$((advertencias + 1))
}

anotar() {
  detalles+=("$1")
}

# Raíz: argumento, repositorio git o directorio padre de scripts/.
raiz="${1:-}"
if [ -z "$raiz" ]; then
  if raiz_git=$(git rev-parse --show-toplevel 2>/dev/null); then
    raiz="$raiz_git"
  else
    raiz=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
  fi
fi
if [ "$raiz" != "/" ]; then
  raiz="${raiz%/}"
fi
if [ ! -d "$raiz" ]; then
  printf '✗ La raíz no existe o no es un directorio: %s\n' "$raiz"
  exit 1
fi

bitacora="$raiz/docs/bitacora"
indice="$bitacora/README.md"

# Entradas: solo docs/bitacora/fase-N/NNN-tema.md.
if [ -d "$bitacora" ]; then
  while IFS= read -r -d '' archivo; do
    entradas+=("$archivo")
  done < <(find "$bitacora" -mindepth 2 -maxdepth 2 -type f \
    -name '[0-9][0-9][0-9]-*.md' -print0 2>/dev/null)
fi

# 1. Existe el índice.
comprobaciones=$((comprobaciones + 1))
if [ -f "$indice" ]; then
  ok "1. Existe docs/bitacora/README.md"
else
  anotar "no se encontró $indice"
  fallo "1. Existe docs/bitacora/README.md"
fi

# 2. Los enlaces del índice apuntan a archivos que existen.
comprobaciones=$((comprobaciones + 1))
if [ ! -f "$indice" ]; then
  anotar "sin índice no se pueden comprobar los enlaces"
  fallo "2. Enlaces del índice"
else
  while IFS= read -r enlace; do
    [ -n "$enlace" ] && enlaces+=("$enlace")
  done < <(grep -oE '\(\./fase-[0-9]+/[0-9]{3}-[^()[:space:]]+\.md\)' "$indice" | tr -d '()')

  if [ "${#enlaces[@]}" -eq 0 ]; then
    anotar "el índice no contiene enlaces con la forma (./fase-N/NNN-tema.md)"
    fallo "2. Enlaces del índice"
  else
    for enlace in "${enlaces[@]}"; do
      relativo="${enlace#./}"
      if [ ! -f "$bitacora/$relativo" ]; then
        anotar "$enlace → el archivo no existe"
      fi
    done
    if [ "${#detalles[@]}" -gt 0 ]; then
      fallo "2. Enlaces del índice: ${#detalles[@]} enlace(s) roto(s)"
    else
      ok "2. Enlaces del índice: ${#enlaces[@]} enlace(s), todos existen"
    fi
  fi
fi

# 3. Cada entrada tiene su fila (su enlace) en el índice.
comprobaciones=$((comprobaciones + 1))
if [ ! -f "$indice" ]; then
  anotar "sin índice no se pueden comprobar las entradas huérfanas"
  fallo "3. Entradas huérfanas"
elif [ "${#entradas[@]}" -eq 0 ]; then
  ok "3. Entradas huérfanas: no hay entradas"
else
  for archivo in "${entradas[@]}"; do
    relativo="${archivo#"$bitacora"/}"
    esperado="./$relativo"
    encontrado=0
    for enlace in "${enlaces[@]}"; do
      if [ "$enlace" = "$esperado" ]; then
        encontrado=1
        break
      fi
    done
    if [ "$encontrado" -eq 0 ]; then
      anotar "$relativo → no tiene fila en el índice"
    fi
  done
  if [ "${#detalles[@]}" -gt 0 ]; then
    fallo "3. Entradas huérfanas: ${#detalles[@]} entrada(s) sin fila"
  else
    ok "3. Entradas huérfanas: ninguna"
  fi
fi

# 4. Encabezado y secciones de cada entrada.
comprobaciones=$((comprobaciones + 1))
if [ "${#entradas[@]}" -eq 0 ]; then
  ok "4. Entradas: no hay entradas que revisar"
else
  for archivo in "${entradas[@]}"; do
    relativo="${archivo#"$bitacora"/}"
    carpeta=$(basename "$(dirname "$archivo")")
    fase_carpeta="${carpeta#fase-}"

    fase_cabecera=$(sed -nE 's/^\*\*Fase:\*\*[[:space:]]*([0-9]+).*/\1/p' "$archivo" | head -n 1)
    if [ -z "$fase_cabecera" ]; then
      anotar "$relativo: falta la línea **Fase:**"
    elif [ "$fase_cabecera" != "$fase_carpeta" ]; then
      anotar "$relativo: **Fase:** $fase_cabecera no coincide con la carpeta fase-$fase_carpeta"
    fi

    if ! grep -qE '^\*\*Fecha:\*\*[[:space:]]*[^[:space:]]' "$archivo"; then
      anotar "$relativo: falta la línea **Fecha:**"
    fi
    if ! grep -qE '^\*\*Rama:\*\*[[:space:]]*[^[:space:]]' "$archivo"; then
      anotar "$relativo: falta la línea **Rama:**"
    fi
    if ! grep -qE '^\*\*Ejecutor:\*\*[[:space:]]*[^[:space:]]' "$archivo"; then
      anotar "$relativo: falta la línea **Ejecutor:**"
    fi

    for seccion in 1 2 3 4 5; do
      if ! grep -qE "^##[[:space:]]+${seccion}\\." "$archivo"; then
        anotar "$relativo: falta la sección ## ${seccion}."
      fi
    done
    if ! grep -qE '^##[[:space:]]+6\.' "$archivo"; then
      aviso "$relativo: falta la sección opcional ## 6. Pendientes"
    fi
  done

  if [ "${#detalles[@]}" -gt 0 ]; then
    fallo "4. Entradas: ${#detalles[@]} problema(s) de encabezado o secciones"
  else
    ok "4. Entradas: ${#entradas[@]} entrada(s) con encabezado y secciones"
  fi
fi

# 5. Numeración global: única y correlativa desde 001.
comprobaciones=$((comprobaciones + 1))
if [ "${#entradas[@]}" -gt 0 ]; then
  for archivo in "${entradas[@]}"; do
    nombre=$(basename "$archivo")
    numero=${nombre%%-*}
    case "$numero" in
      [0-9][0-9][0-9]) numeros+=("$((10#$numero))") ;;
    esac
  done
fi
if [ "${#numeros[@]}" -eq 0 ]; then
  ok "5. Numeración: no hay entradas que numerar"
else
  mapfile -t ordenados < <(printf '%s\n' "${numeros[@]}" | sort -n)
  anterior=""
  esperado=1
  for numero in "${ordenados[@]}"; do
    if [ "$numero" = "$anterior" ]; then
      anotar "número repetido: $(printf '%03d' "$numero")"
    fi
    if [ "$numero" -gt "$esperado" ]; then
      faltante=$esperado
      while [ "$faltante" -lt "$numero" ]; do
        anotar "falta el número: $(printf '%03d' "$faltante")"
        faltante=$((faltante + 1))
      done
    fi
    if [ "$numero" -ge "$esperado" ]; then
      esperado=$((numero + 1))
    fi
    anterior="$numero"
  done

  if [ "${#detalles[@]}" -gt 0 ]; then
    fallo "5. Numeración: ${#detalles[@]} problema(s)"
  else
    ultimo=$(printf '%03d' "${ordenados[${#ordenados[@]}-1]}")
    ok "5. Numeración: 001 a $ultimo, sin repeticiones ni huecos"
  fi
fi

# Busca un patrón en un archivo y anota cada línea encontrada.
anotar_lineas() {
  local archivo="$1"
  local patron="$2"
  local relativo linea
  relativo="${archivo#"$raiz"/}"
  while IFS= read -r linea; do
    [ -n "$linea" ] && anotar "$relativo:$linea"
  done < <(grep -nE "$patron" "$archivo" 2>/dev/null)
}

# 6. Marcas de conflicto de git.
comprobaciones=$((comprobaciones + 1))
if [ -d "$bitacora" ]; then
  while IFS= read -r -d '' archivo; do
    anotar_lineas "$archivo" '^(<<<<<<<|=======[[:space:]]*$|>>>>>>>)'
  done < <(find "$bitacora" -type f -name '*.md' -print0 2>/dev/null)
fi
for archivo in "$raiz/CLAUDE.md" "$raiz"/docs/*.md; do
  [ -f "$archivo" ] || continue
  anotar_lineas "$archivo" '^(<<<<<<<|>>>>>>>)'
done
if [ "${#detalles[@]}" -gt 0 ]; then
  fallo "6. Marcas de conflicto: ${#detalles[@]} línea(s)"
else
  ok "6. Marcas de conflicto: ninguna"
fi

# 7. URL y correos en la bitácora.
comprobaciones=$((comprobaciones + 1))
if [ -d "$bitacora" ]; then
  while IFS= read -r -d '' archivo; do
    anotar_lineas "$archivo" \
      'https?://|[[:alnum:]._%+-]+@[[:alnum:].-]+\.[[:alpha:]]{2,}'
  done < <(find "$bitacora" -type f -name '*.md' -print0 2>/dev/null)
fi
if [ "${#detalles[@]}" -gt 0 ]; then
  fallo "7. Datos prohibidos en la bitácora: ${#detalles[@]} línea(s)"
else
  ok "7. Datos prohibidos en la bitácora: ninguno"
fi

printf '\n%d comprobaciones, %d fallos, %d advertencias\n' \
  "$comprobaciones" "$fallos" "$advertencias"

if [ "$fallos" -gt 0 ]; then
  exit 1
fi
exit 0
