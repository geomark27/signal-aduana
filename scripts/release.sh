#!/usr/bin/env bash
# Publica una versión de Signal Aduana ya fusionada en main (tras make promote):
#   1. verifica que origin/main tenga todo develop, la versión de main y que el tag esté libre;
#   2. pone el tag vX.Y.Z sobre origin/main, lo sube y crea el release en borrador con `gh`.
# Al recibir el tag, GitHub Actions (.github/workflows/release.yml) compila el instalador de
# Windows y lo firma para el actualizador, lo adjunta al release con latest.json y lo publica.
# Se corre desde cualquier carpeta del repo; nunca cambia de rama ni hace commits.
#
# Uso: scripts/release.sh [--dry-run]     (YES=1 no pregunta)
#   make release            DRY_RUN=1 solo muestra qué haría; yes=1 no pregunta
set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "✗ $*" >&2; exit 1; }
step() { echo "→ $*"; }

dry_run=false
[[ ${1:-} == "--dry-run" ]] && dry_run=true

command -v gh >/dev/null || die "falta la CLI de GitHub (gh)"
gh auth status >/dev/null 2>&1 || die "gh no tiene sesión: ejecuta 'gh auth login'"

step "Comprobando el remoto"
git fetch --quiet --prune --tags origin main develop

pending=$(git rev-list --count origin/main..origin/develop)
[[ $pending -eq 0 ]] \
  || die "develop tiene $pending commit(s) que no están en main: corre make promote y fusiona el PR"

version=$(git show origin/main:Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -1)
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "no se pudo leer la versión de Cargo.toml en origin/main"
app=$(git show origin/main:tauri.conf.json | sed -n 's/^  "version": "\(.*\)",$/\1/p')
[[ $app == "$version" ]] \
  || die "origin/main dice $version en Cargo.toml y '$app' en tauri.conf.json: corrígelo con make bump en una tarea"
tag="v$version"

# Sin la llave, las versiones instaladas no podrían verificar ni recibir actualizaciones.
! git show origin/main:tauri.conf.json | grep -q '"pubkey": "PENDIENTE'   || die "falta la llave del actualizador en main: corre make updater-key en una tarea y fusiónala"
gh secret list --json name -q '.[].name' | grep -qx TAURI_SIGNING_PRIVATE_KEY   || die "el repo no tiene el secreto TAURI_SIGNING_PRIVATE_KEY: corre make updater-key"

taken="ya existe: sube la versión con make bump en una tarea"
git rev-parse -q --verify "refs/tags/$tag" >/dev/null && die "$tag $taken"
remote_tag=$(git ls-remote --tags origin "refs/tags/$tag") || die "no se pudo consultar los tags del remoto"
[[ -z $remote_tag ]] || die "$tag $taken"

previous=$(git describe --tags --abbrev=0 origin/main 2>/dev/null || true)
if [[ -n $previous ]]; then
  newest=$(printf '%s\n%s\n' "${previous#v}" "$version" | sort -V | tail -1)
  [[ $newest == "$version" ]] || die "$version no es mayor que el último tag ($previous)"
fi

notes=$(git show origin/main:CHANGELOG.md | bash scripts/changelog-notes.sh "$version" - 2>/dev/null) \
  || die "CHANGELOG.md de main no tiene la sección [$version]: corre make bump en una tarea"
commit=$(git rev-parse --short origin/main)
repo_url=$(git remote get-url origin \
  | sed -E 's#^git@([^:]+):#https://\1/#; s#^ssh://git@([^/]+)/#https://\1/#; s#\.git$##')

echo
echo "Release $tag sobre origin/main ($commit), último tag: ${previous:-ninguno}"
echo "Notas:"
echo "$notes" | sed 's/^/  │ /'
echo

if $dry_run; then
  echo "Simulación (DRY_RUN): no se creó ningún tag ni release."
  exit 0
fi

if [[ ${YES:-} != 1 ]]; then
  read -r -p "¿Publicar $tag en $repo_url? [s/N] " answer || answer=""
  [[ $answer =~ ^[sS]$ ]] || die "cancelado"
fi

step "Creando y subiendo el tag $tag"
git tag -a "$tag" origin/main -m "Signal Aduana $tag"
git push --quiet origin "$tag"

# El job «prepare» del workflow también crea el borrador: vale el que llegue primero.
step "Creando el release en borrador"
gh release view "$tag" >/dev/null 2>&1 \
  || printf '%s\n' "$notes" | gh release create "$tag" --draft --verify-tag \
    --title "Signal Aduana $tag" --notes-file -

echo
echo "✓ $tag publicado. GitHub Actions está compilando el instalador (~15–25 min):"
echo "    gh run watch \$(gh run list --workflow release.yml --limit 1 --json databaseId -q '.[0].databaseId')"
echo "  Al terminar, el release se publica solo: $repo_url/releases/tag/$tag"
