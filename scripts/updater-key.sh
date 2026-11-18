#!/usr/bin/env bash
# Crea (una sola vez) la llave con la que se firman las actualizaciones automáticas y la configura:
#   - privada: ~/.tauri/signal-aduana.key  → secreto TAURI_SIGNING_PRIVATE_KEY del repo en GitHub (para el CI);
#   - pública: plugins.updater.pubkey de tauri.conf.json (va en el commit de la tarea).
# La llave privada nunca se imprime. Respáldala: si se pierde, las versiones ya instaladas no
# podrán actualizarse solas.
# Se corre dentro de una tarea (rama tarea/*), porque deja un cambio en tauri.conf.json que va por PR.
#
# Uso: make updater-key      (SIGNAL_UPDATER_KEY=<ruta> cambia la ubicación de la llave)
set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "✗ $*" >&2; exit 1; }
step() { echo "→ $*"; }

key="${SIGNAL_UPDATER_KEY:-$HOME/.tauri/signal-aduana.key}"
conf=tauri.conf.json

case "$(git branch --show-current)" in
  tarea/*) ;;
  *) die "corre make updater-key dentro de una tarea: el cambio de tauri.conf.json va por PR" ;;
esac

command -v gh >/dev/null || die "falta la CLI de GitHub (gh)"
gh auth status >/dev/null 2>&1 || die "gh no tiene sesión: ejecuta 'gh auth login'"

if [[ -e $key ]]; then
  step "Reutilizo la llave existente $key (crear otra rompería las actualizaciones)"
else
  step "Generando la llave en $key"
  mkdir -p "$(dirname "$key")"
  # --ci: sin contraseña, para que el CI firme solo con el secreto.
  npx --yes @tauri-apps/cli@2 signer generate --ci -w "$key" >/dev/null
  chmod 600 "$key"
fi
[[ -s "$key.pub" ]] || die "no encontré la llave pública $key.pub"
pub=$(tr -d '\n' < "$key.pub")

step "Guardando la llave privada como secreto TAURI_SIGNING_PRIVATE_KEY de $(gh repo view --json nameWithOwner -q .nameWithOwner)"
gh secret set TAURI_SIGNING_PRIVATE_KEY < "$key"

step "Escribiendo la llave pública en $conf"
sed -i "s|^      \"pubkey\": \".*\",$|      \"pubkey\": \"$pub\",|" "$conf"
grep -qF "\"pubkey\": \"$pub\"" "$conf" || die "no pude escribir la llave pública en $conf"

echo
echo "✓ Listo. Incluye tauri.conf.json en el commit de la tarea y guarda una copia de $key en un lugar"
echo "  seguro (gestor de contraseñas). No la subas al repositorio. Si se pierde, las versiones"
echo "  instaladas no podrán actualizarse solas."
