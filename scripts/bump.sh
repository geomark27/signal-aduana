#!/usr/bin/env bash
# Sube la versión de Signal Aduana dentro de una tarea (no hace commit):
#   - fija la versión en Cargo.toml, tauri.conf.json y Cargo.lock;
#   - pasa [Unreleased] de CHANGELOG.md a la nueva versión y actualiza los enlaces del final.
# Después: make check · make commit · make ship (a develop), make promote y make release.
#
# Uso: scripts/bump.sh <patch|minor|major|""> [X.Y.Z] [--dry-run]
#   make bump bump=patch|minor|major      sube esa parte de la versión actual (Cargo.toml)
#   make bump ver=X.Y.Z                   versión exacta (la "v" es opcional); puede ser la actual
#                                         solo si su tag aún no existe (primer release)
#   DRY_RUN=1                             solo muestra qué haría
set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "✗ $*" >&2; exit 1; }
step() { echo "→ $*"; }

bump=${1:-}
version=${2#v}
dry_run=false
[[ ${3:-} == "--dry-run" ]] && dry_run=true

branch=$(git branch --show-current)
[[ $branch == tarea/* ]] \
  || die "la versión se sube en una tarea: make tarea n=N.M tema=version-X.Y.Z"
[[ -z $(git status --porcelain) ]] || die "hay cambios sin commitear: haz make commit antes de subir la versión"

current=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
[[ $current =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "no se pudo leer la versión actual de Cargo.toml"
[[ -z $bump || -z $version ]] || die "usa bump= o ver=, no los dos"
if [[ -n $bump ]]; then
  IFS=. read -r major minor patch <<<"$current"
  case $bump in
    patch) version="$major.$minor.$((patch + 1))" ;;
    minor) version="$major.$((minor + 1)).0" ;;
    major) version="$((major + 1)).0.0" ;;
    *) die "bump inválido: '$bump' (patch, minor o major)" ;;
  esac
fi
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
  || die "indica la versión: make bump bump=patch|minor|major  (o ver=X.Y.Z)"
tag="v$version"

same=false
if [[ $version == "$current" ]]; then
  same=true
  [[ -z $(git tag -l "$tag") ]] \
    || die "la versión actual ya está publicada ($tag existe): usa bump= o una versión mayor"
else
  newest=$(printf '%s\n%s\n' "$current" "$version" | sort -V | tail -1)
  [[ $newest == "$version" ]] || die "$version es anterior a la versión actual ($current)"
fi

[[ -f CHANGELOG.md ]] || die "falta CHANGELOG.md en la raíz del repo"
if grep -qE "^## \[$version\]" CHANGELOG.md; then
  die "CHANGELOG.md ya tiene una sección [$version]"
fi
notes=$(bash scripts/changelog-notes.sh Unreleased 2>/dev/null) \
  || die "anota los cambios en [Unreleased] de CHANGELOG.md"

# Último tag publicado (el mayor por versión, sin contar el de esta versión).
previous=$(git tag -l 'v[0-9]*' --sort=-v:refname | grep -vxF "$tag" | head -1 || true)
repo_url=$(git remote get-url origin 2>/dev/null \
  | sed -E 's#^git@([^:]+):#https://\1/#; s#^ssh://git@([^/]+)/#https://\1/#; s#\.git$##') \
  || true
[[ -n $repo_url ]] || die "no se pudo leer la URL del remoto origin"

echo
if $same; then
  echo "Versión $current (sin cambio de número; solo se cierra [Unreleased] en el CHANGELOG)${previous:+. Último tag: $previous}"
else
  echo "Versión $current → $version${previous:+ (último tag: $previous)}"
fi
echo "Notas:"
echo "$notes" | sed 's/^/  │ /'
echo

if $dry_run; then
  echo "Simulación (DRY_RUN): no se modificó nada."
  exit 0
fi

if ! $same; then
  step "Fijando la versión $version"
  sed -i "0,/^version = \".*\"$/s//version = \"$version\"/" Cargo.toml
  sed -i "s/^  \"version\": \".*\",$/  \"version\": \"$version\",/" tauri.conf.json

  # Imprime 1 si la entrada signal-aduana de Cargo.lock ya tiene la versión nueva.
  locked() {
    awk -v v="$version" '
      $0 == "name = \"signal-aduana\"" { f = 1; next }
      f && /^version = / { print ($0 == "version = \"" v "\"") ? 1 : 0; exit }
    ' Cargo.lock
  }
  if ! { cargo update --workspace --offline --quiet 2>/dev/null && [[ $(locked) == 1 ]]; }; then
    echo "! cargo update no pudo actualizar Cargo.lock: se cambió solo la línea de signal-aduana"
    awk -v v="$version" '
      $0 == "name = \"signal-aduana\"" { f = 1; print; next }
      f && /^version = / { print "version = \"" v "\""; f = 0; next }
      { print }
    ' Cargo.lock > Cargo.lock.tmp && mv Cargo.lock.tmp Cargo.lock
    [[ $(locked) == 1 ]] || die "no se pudo actualizar la versión en Cargo.lock"
  fi
fi

step "Actualizando CHANGELOG.md"
today=$(date +%F)
if [[ -n $previous ]]; then
  version_link="[$version]: $repo_url/compare/$previous...$tag"
else
  version_link="[$version]: $repo_url/releases/tag/$tag"
fi
unreleased_link="[Unreleased]: $repo_url/compare/$tag...HEAD"
if grep -q '^\[Unreleased\]: ' CHANGELOG.md; then
  awk -v header="## [$version] - $today" \
      -v unreleased_link="$unreleased_link" -v version_link="$version_link" '
    $0 == "## [Unreleased]" { print; print ""; print header; next }
    index($0, "[Unreleased]: ") == 1 { print unreleased_link; print version_link; next }
    { print }
  ' CHANGELOG.md > CHANGELOG.md.tmp && mv CHANGELOG.md.tmp CHANGELOG.md
else
  awk -v header="## [$version] - $today" '
    $0 == "## [Unreleased]" { print; print ""; print header; next }
    { print }
  ' CHANGELOG.md > CHANGELOG.md.tmp && mv CHANGELOG.md.tmp CHANGELOG.md
  # Sin enlaces en el archivo: se agregan al final.
  { echo; echo "$unreleased_link"; echo "$version_link"; } >> CHANGELOG.md
fi

echo
echo "✓ Versión $version lista en el árbol de trabajo (sin commit)."
echo "Revisa el diff y sigue con: make check · make commit · make ship"
