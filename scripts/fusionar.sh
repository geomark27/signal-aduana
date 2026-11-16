#!/usr/bin/env bash
# Fusiona en develop el PR de la rama actual, siempre con merge commit. Lo corre
# el autor (necesita gh autenticado).
#   make merge [yes=1]
set -euo pipefail

remote=origin
base=develop

fail() { echo "✗ $*" >&2; exit 1; }
ok() { echo "✓ $*"; }

command -v gh >/dev/null && gh auth status >/dev/null 2>&1 || fail "hace falta gh autenticado: gh auth login"

branch=$(git branch --show-current)
case "$branch" in
	tarea/*) ;;
	*) fail "no hay PR que fusionar desde '${branch:-HEAD suelto}': corre make merge desde una rama tarea/N.M-tema" ;;
esac

git fetch --quiet "$remote" "$branch" 2>/dev/null || fail "'$branch' no está en $remote: corre make push"
[ "$(git rev-list --count "$remote/$branch..$branch")" -eq 0 ] || fail "'$branch' tiene commits sin subir: corre make push"

pr=$(gh pr list --head "$branch" --state open --json number --jq '.[0].number // empty')
[ -n "$pr" ] || fail "no hay un PR abierto de '$branch': corre make pr"
target=$(gh pr view "$pr" --json baseRefName --jq .baseRefName)
[ "$target" = "$base" ] || fail "el PR #$pr va hacia '$target', no hacia $base: develop → main es make promote"
title=$(gh pr view "$pr" --json title --jq .title)

if [ "${MERGE_YES:-}" != 1 ]; then
	printf '¿Fusionar el PR #%s «%s» en %s con merge commit? [s/N]: ' "$pr" "$title" "$base" >&2
	read -r answer || answer=""
	[[ "$answer" =~ ^[sSyY] ]] || fail "sin fusionar: el PR #$pr queda abierto"
fi

# La rama no se borra aquí: la retira make finish desde la carpeta principal.
gh pr merge "$pr" --merge
ok "PR #$pr fusionado en $base con merge commit"

# make ship pone FUSIONAR_AVISO=0 porque imprime este mismo aviso al terminar.
if [ "${FUSIONAR_AVISO:-1}" != 0 ]; then
	tarea=$(echo "$branch" | sed -nE 's/^tarea\/([0-9]+\.[0-9]+)-.*/\1/p')
	echo "ahora, desde la carpeta principal: make finish n=${tarea:-<N.M>}"
fi
