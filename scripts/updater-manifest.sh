#!/usr/bin/env bash
# Genera latest.json, el manifiesto que consulta el actualizador de Signal Aduana, a partir de la
# firma .sig que deja la compilación de Windows (<instalador>.sig, con el nombre fijo del release),
# y lo sube al release. Lo ejecuta el CI después de compilar.
# Necesita gh con GH_REPO y GH_TOKEN, y jq.  Uso: scripts/updater-manifest.sh v0.1.0 <carpeta-con-.sig>
#
# UPDATER_MANIFEST_OUT=<ruta>: escribe latest.json ahí y no lo sube (para probar sin red ni gh;
# GH_REPO puede quedar sin definir).
#
# Solo Windows: el actualizador busca primero "windows-x86_64-nsis" y si no existe, "windows-x86_64".
set -euo pipefail

tag=${1:?uso: scripts/updater-manifest.sh vX.Y.Z <carpeta-con-.sig>}
sigs=${2:?uso: scripts/updater-manifest.sh vX.Y.Z <carpeta-con-.sig>}
version=${tag#v}
installer=Signal-Aduana-windows-x64-setup.exe
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

command -v jq >/dev/null || { echo "falta jq" >&2; exit 1; }
[[ -f $sigs/$installer.sig ]] || { echo "no existe $sigs/$installer.sig" >&2; exit 1; }

repo=${GH_REPO:-}
[[ -n $repo || -n ${UPDATER_MANIFEST_OUT:-} ]] || { echo "falta GH_REPO" >&2; exit 1; }
url="https://github.com/${repo:-OWNER/REPO}/releases/download/$tag/$installer"
signature=$(tr -d '\r\n' < "$sigs/$installer.sig")

notes=$(bash "$(dirname "$0")/changelog-notes.sh" "$version")
jq -n --arg version "$version" --arg notes "$notes" --arg date "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg url "$url" --arg sig "$signature" \
  '{version: $version, notes: $notes, pub_date: $date,
    platforms: {
      "windows-x86_64-nsis": {url: $url, signature: $sig},
      "windows-x86_64": {url: $url, signature: $sig}
    }}' > "$dir/latest.json"

if [[ -n ${UPDATER_MANIFEST_OUT:-} ]]; then
  cp "$dir/latest.json" "$UPDATER_MANIFEST_OUT"
  echo "latest.json para $tag escrito en $UPDATER_MANIFEST_OUT (sin subir)"
  exit 0
fi

echo "latest.json para $tag:"
jq -r '.platforms | keys[] | "  " + .' "$dir/latest.json"
gh release upload "$tag" "$dir/latest.json" --clobber
