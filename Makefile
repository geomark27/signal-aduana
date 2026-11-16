# Atajos de Signal Aduana para el flujo de git (con worktrees) y la calidad del código.
# `make` (o `make help`) lista todo; las opciones van como variables:
#   make tarea n=4.10 tema=aviso-inicio, make commit title="aviso de inicio", make test run=semana.

SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c
.DEFAULT_GOAL := help

# El rustc de /usr/bin es viejo: va primero la toolchain de rustup.
export PATH := $(HOME)/.cargo/bin:$(PATH)

REMOTE := origin
BASE   := develop
MAIN   := main
BRANCH := $(shell git branch --show-current 2>/dev/null)
MAIN_DIR := $(shell git worktree list --porcelain 2>/dev/null | awk '/^worktree /{sub(/^worktree /,""); print; exit}')
TAREA  := $(shell echo "$(BRANCH)" | sed -nE 's/^tarea\/([0-9]+\.[0-9]+)-.*/\1/p')

# Enlace web del remoto (acepta https://… y git@host:… ) para armar enlaces de comparación.
REPO_URL_SH = git remote get-url $(REMOTE) | sed -E 's\#^git@([^:]+):\#https://\1/\#; s\#^ssh://git@([^/]+)/\#https://\1/\#; s\#\.git$$\#\#'

# Los parámetros (n, tema, title) llegan al shell como variables de entorno.
export n tema title

GUARD_TAREA = case "$(BRANCH)" in tarea/*) ;; *) echo "✗ '$(BRANCH)' no es una rama de tarea (tarea/N.M-tema)" >&2; exit 1;; esac
GUARD_RAMA = case "$(BRANCH)" in main|$(BASE)|"") echo "✗ no se sube ni se abre PR desde '$(BRANCH)': trabaja en una rama tarea/N.M-tema" >&2; exit 1;; esac
GH_OK = command -v gh >/dev/null && gh auth status >/dev/null 2>&1

.PHONY: help sync tarea status worktrees update commit push pr merge ship finish prune promote \
        build test lint ui-check run bitacora check

##@ Git

sync: ## Trae origin y deja develop al día solo con fast-forward, desde la carpeta que sea
	@git fetch $(REMOTE); \
	dev_dir=$$(git worktree list --porcelain | awk -v b="refs/heads/$(BASE)" '/^worktree /{p=substr($$0,10)} $$0=="branch " b {print p; exit}'); \
	nofast() { echo "✗ $(BASE) no avanza con fast-forward (se desvió de $(REMOTE)/$(BASE) o hay cambios locales que chocan): no se mezcla ni se reescribe, revísalo a mano" >&2; exit 1; }; \
	if [ "$(BRANCH)" = "$(BASE)" ]; then \
		git pull --ff-only $(REMOTE) $(BASE) || nofast; \
	elif [ -n "$$dev_dir" ]; then \
		git -C "$$dev_dir" merge --ff-only $(REMOTE)/$(BASE) || nofast; \
	else \
		git fetch $(REMOTE) $(BASE):$(BASE) || nofast; \
	fi; \
	echo "✓ $(BASE) al día$(if $(filter-out $(BASE),$(BRANCH)), (sigues en $(BRANCH)),)"

tarea: ## Crea la tarea N.M en su worktree desde develop al día: make tarea n=4.10 tema=aviso-inicio
	@[[ "$${n:-}" =~ ^[0-9]+\.[0-9]+$$ ]] || { echo "✗ n=<fase.tarea> con el formato N.M (ej: n=4.10)" >&2; exit 1; }
	@[[ "$${tema:-}" =~ ^[a-z0-9]+(-[a-z0-9]+)*$$ ]] || { echo "✗ tema=<tema> en minúsculas y con guiones (ej: tema=aviso-inicio)" >&2; exit 1; }
	@[ ! -e "$(MAIN_DIR)/.worktrees/$$n" ] || { echo "✗ la tarea $$n ya existe: $(MAIN_DIR)/.worktrees/$$n" >&2; exit 1; }
	@! git show-ref --verify --quiet "refs/heads/tarea/$$n-$$tema" || { echo "✗ la rama tarea/$$n-$$tema ya existe" >&2; exit 1; }
	@$(MAKE) --no-print-directory sync
	@excl="$$(git rev-parse --path-format=absolute --git-common-dir)/info/exclude"; \
	mkdir -p "$$(dirname "$$excl")"; touch "$$excl"; \
	grep -qxF '/.worktrees/' "$$excl" || echo '/.worktrees/' >> "$$excl"; \
	wt="$(MAIN_DIR)/.worktrees/$$n"; \
	git worktree add --no-track -b "tarea/$$n-$$tema" "$$wt" $(BASE); \
	echo "✓ tarea $$n creada: rama tarea/$$n-$$tema en $$wt"; \
	echo; echo "Siguientes pasos:"; echo "  cd $$wt"; echo "  make commit   (cuando haya algo que guardar)"; echo "  make ship     (sube, abre el PR y lo fusiona)"

status: ## Estado de la tarea actual: rama, commits, subida, cambios y PR
	@br="$(BRANCH)"; \
	if [ -z "$$br" ]; then echo "Sin rama (HEAD suelto): no hay estado de tarea que mostrar."; exit 0; fi; \
	echo "Rama:          $$br"; \
	case "$$br" in tarea/*) ;; *) echo "No estás en una rama de tarea (tarea/N.M-tema): no hay estado que mostrar."; exit 0;; esac; \
	echo "Tarea:         $(TAREA)"; \
	echo "Commits sobre $(REMOTE)/$(BASE): $$(git rev-list --count $(REMOTE)/$(BASE)..HEAD 2>/dev/null || echo '?')"; \
	if git rev-parse --verify --quiet "refs/remotes/$(REMOTE)/$$br" >/dev/null; then \
		sin=$$(git rev-list --count "$(REMOTE)/$$br..HEAD"); \
		if [ "$$sin" -eq 0 ]; then echo "Subida:        sí, al día"; else echo "Subida:        sí, con $$sin commit(s) sin subir"; fi; \
	else echo "Subida:        no"; fi; \
	if [ -n "$$(git status --porcelain)" ]; then echo "Cambios:       sí, sin commitear"; else echo "Cambios:       ninguno"; fi; \
	if $(GH_OK); then \
		num=$$(gh pr list --head "$$br" --state open --json number --jq '.[0].number // empty' 2>/dev/null || true); \
		if [ -n "$$num" ]; then url=$$(gh pr list --head "$$br" --state open --json url --jq '.[0].url // empty' 2>/dev/null || true); echo "PR abierto:    #$$num $$url"; \
		else echo "PR abierto:    ninguno"; fi; \
	else echo "PR abierto:    (gh no está autenticado)"; fi

worktrees: ## Lista los worktrees (una carpeta por tarea)
	@git worktree list

update: ## Rebase de la tarea sobre origin/develop (con el árbol limpio) y revisa la bitácora
	@$(GUARD_TAREA)
	@[ -z "$$(git status --porcelain)" ] || { echo "✗ hay cambios sin commitear: haz make commit antes de actualizar" >&2; exit 1; }
	@git fetch $(REMOTE); \
	if ! git rebase $(REMOTE)/$(BASE); then \
		echo "✗ el rebase se detuvo por conflictos. Resuélvelos, luego: git add <archivos> && git rebase --continue, y al terminar make bitacora" >&2; exit 1; \
	fi; \
	if git rev-parse --verify --quiet "refs/remotes/$(REMOTE)/$(BRANCH)" >/dev/null; then \
		echo "→ la rama ya estaba subida y el rebase cambió su historia: sube con make push force=1"; \
	fi
	@$(MAKE) --no-print-directory bitacora

commit: ## Un solo commit por tarea: "Tarea: N.M tema" (title= cambia el tema); enmienda si no se ha subido
	@$(GUARD_TAREA)
	@$(MAKE) --no-print-directory bitacora
	@tema_rama=$$(echo "$(BRANCH)" | sed -E 's/^tarea\/[0-9]+\.[0-9]+-//; s/-/ /g'); \
	msg="Tarea: $(TAREA) $${title:-$$tema_rama}"; \
	ahead=$$(git rev-list --count $(REMOTE)/$(BASE)..HEAD); \
	pushed=no; \
	if git rev-parse --verify --quiet "refs/remotes/$(REMOTE)/$(BRANCH)" >/dev/null && git merge-base --is-ancestor HEAD "$(REMOTE)/$(BRANCH)"; then pushed=yes; fi; \
	if [ "$$ahead" -eq 0 ]; then \
		git add -A; \
		if git diff --cached --quiet; then echo "✗ no hay nada que commitear" >&2; exit 1; fi; \
		git commit -m "$$msg"; \
	elif [ "$$ahead" -eq 1 ]; then \
		if [ "$$pushed" = yes ] && [ -z "$(force)" ]; then \
			echo "✗ el commit de la tarea ya está subido: con force=1 se enmienda y después hace falta make push force=1" >&2; exit 1; \
		fi; \
		git add -A; \
		if git diff --cached --quiet && [ -z "$${title:-}" ]; then echo "✗ no hay cambios que sumar al commit (ni title= para cambiar el mensaje)" >&2; exit 1; fi; \
		if [ -n "$${title:-}" ]; then git commit --amend -m "$$msg"; else git commit --amend --no-edit; fi; \
		if [ "$$pushed" = yes ]; then echo "→ enmendaste un commit ya subido: sube con make push force=1"; fi; \
	else \
		echo "✗ la tarea lleva $$ahead commits y debe quedar en uno solo: git reset --soft $(REMOTE)/$(BASE) y repite make commit" >&2; exit 1; \
	fi; \
	echo "✓ commit listo: $$(git log -1 --format=%s)"

push: ## Sube la rama actual y fija su upstream; force=1 usa --force-with-lease
	@$(GUARD_RAMA)
	git push -u $(REMOTE) $(BRANCH) $(if $(force),--force-with-lease,)

pr: ## Muestra el PR abierto de la rama o crea uno hacia develop (web=1 lo abre en el navegador); sin gh, el enlace
	@$(GUARD_RAMA)
	@if $(GH_OK); then \
		open=$$(gh pr list --head "$(BRANCH)" --state open --json url --jq '.[0].url // empty'); \
		if [ -n "$$open" ]; then echo "✓ PR ya abierto: $$open"; \
		else gh pr create --base $(BASE) --head $(BRANCH) --fill $(if $(web),--web,); fi; \
	else \
		url=$$($(REPO_URL_SH)); echo "gh no está autenticado; abre el PR aquí:"; echo "$$url/compare/$(BASE)...$(BRANCH)?expand=1"; \
	fi

merge: ## Fusiona en develop el PR de la rama actual con merge commit; pregunta antes (yes=1 no pregunta)
	@MERGE_YES=$(yes) bash scripts/fusionar.sh

ship: ## push + pr + merge, deteniéndose en el primer fallo; el cierre (finish) va desde la carpeta principal
	@$(GUARD_TAREA)
	@$(MAKE) --no-print-directory push
	@$(MAKE) --no-print-directory pr
	@FUSIONAR_AVISO=0 $(MAKE) --no-print-directory merge
	@echo "ahora, desde la carpeta principal: make finish n=$(TAREA)"

finish: ## Desde la carpeta principal, tras fusionar el PR: retira el worktree y la rama local: make finish n=4.10
	@[[ "$${n:-}" =~ ^[0-9]+\.[0-9]+$$ ]] || { echo "✗ n=<fase.tarea> con el formato N.M (ej: make finish n=4.10)" >&2; exit 1; }
	@[ "$$(git rev-parse --absolute-git-dir)" = "$$(git rev-parse --path-format=absolute --git-common-dir)" ] || { echo "✗ make finish se corre desde la carpeta principal: cd $(MAIN_DIR)" >&2; exit 1; }
	@[ -d "$(MAIN_DIR)/.worktrees/$$n" ] || { echo "✗ no existe el worktree $(MAIN_DIR)/.worktrees/$$n" >&2; exit 1; }
	@git fetch --prune $(REMOTE)
	@$(MAKE) --no-print-directory sync
	@wt="$(MAIN_DIR)/.worktrees/$$n"; \
	br=$$(git -C "$$wt" branch --show-current); \
	case "$$br" in tarea/*) ;; *) echo "✗ el worktree $$wt no está en una rama tarea/*" >&2; exit 1;; esac; \
	git merge-base --is-ancestor "$$br" $(REMOTE)/$(BASE) || { echo "✗ '$$br' no está fusionada en $(REMOTE)/$(BASE): fusiona su PR con merge commit (squash y rebase no sirven para este chequeo)" >&2; exit 1; }; \
	git worktree remove "$$wt" || { echo "✗ no se retiró $$wt: tiene cambios sin commitear o archivos sin seguimiento; guárdalos o descártalos a mano" >&2; exit 1; }; \
	git branch -d "$$br" || { echo "✗ el worktree se retiró pero git no borró la rama local '$$br'" >&2; exit 1; }; \
	echo "✓ tarea $$n cerrada: worktree y rama local '$$br' retirados"; \
	echo "La rama remota la borra el autor (no se ejecuta aquí):"; \
	echo "  git push $(REMOTE) --delete $$br"

prune: ## Retira los worktrees de tarea ya fusionados y limpios, y las ramas tarea/* fusionadas sin worktree
	@git fetch --quiet --prune $(REMOTE); \
	wtdir="$(MAIN_DIR)/.worktrees"; \
	top=$$(git rev-parse --show-toplevel); \
	firstparent=$$(git rev-list --first-parent $(REMOTE)/$(BASE)); \
	listing=$$(git worktree list --porcelain | awk -v RS= -F'\n' '{p="";b=""; for(i=1;i<=NF;i++){ if($$i ~ /^worktree /) p=substr($$i,10); else if($$i ~ /^branch refs\/heads\//) b=substr($$i,19)}; print p "\t" b}'); \
	checked_out=" $$(echo "$$listing" | awk -F'\t' '$$2!=""{printf "%s ", $$2}')"; \
	removed=(); kept=(); \
	why_not() { \
		local br="$$1" path="$$2"; \
		if ! git merge-base --is-ancestor "$$br" $(REMOTE)/$(BASE); then echo "no está fusionada en $(REMOTE)/$(BASE)"; \
		elif echo "$$firstparent" | grep -qxF "$$(git rev-parse "$$br")"; then echo "no tiene commits propios (recién creada)"; \
		elif [ -n "$$path" ] && [ -n "$$(git -C "$$path" status --porcelain)" ]; then echo "tiene cambios sin commitear"; \
		fi; \
	}; \
	while IFS=$$'\t' read -r path br; do \
		case "$$path" in "$$wtdir"/*) ;; *) continue;; esac; \
		name=$${path#$$wtdir/}; \
		case "$$br" in tarea/*) ;; *) kept+=("worktree $$name: no tiene una rama tarea/*"); continue;; esac; \
		if [ "$$path" = "$$top" ]; then kept+=("$$br ($$name): es la carpeta desde la que corres"); continue; fi; \
		why=$$(why_not "$$br" "$$path"); \
		if [ -n "$$why" ]; then kept+=("$$br ($$name): $$why"); continue; fi; \
		if git worktree remove "$$path" && git branch -d "$$br" >/dev/null; then removed+=("$$br (worktree $$name)"); \
		else kept+=("$$br ($$name): git se negó a retirarlo"); fi; \
	done <<< "$$listing"; \
	for br in $$(git for-each-ref --format='%(refname:short)' refs/heads/tarea/); do \
		case "$$checked_out" in *" $$br "*) continue;; esac; \
		why=$$(why_not "$$br" ""); \
		if [ -n "$$why" ]; then kept+=("$$br (sin worktree): $$why"); continue; fi; \
		if git branch -d "$$br" >/dev/null; then removed+=("$$br (rama sin worktree)"); else kept+=("$$br (sin worktree): git se negó a borrarla"); fi; \
	done; \
	echo "Retirado: $${#removed[@]}"; \
	for r in "$${removed[@]}"; do echo "  ✓ $$r"; done; \
	echo "Conservado: $${#kept[@]}"; \
	for k in "$${kept[@]}"; do echo "  · $$k"; done

promote: ## Revisa lo que falta llevar de develop a main y ofrece crear el PR (yes=1 no pregunta); nunca fusiona
	@git fetch --prune $(REMOTE)
	@n=$$(git rev-list --count $(REMOTE)/$(MAIN)..$(REMOTE)/$(BASE)); \
	if [ "$$n" -eq 0 ]; then echo "✓ $(MAIN) ya tiene todo lo de $(BASE): no hay nada que publicar"; exit 0; fi; \
	echo "$$n commit(s) de $(BASE) que aún no están en $(MAIN):"; \
	git log --oneline --no-decorate $(REMOTE)/$(MAIN)..$(REMOTE)/$(BASE); \
	echo; \
	if $(GH_OK); then \
		open=$$(gh pr list --base $(MAIN) --head $(BASE) --state open --json url --jq '.[0].url // empty'); \
		if [ -n "$$open" ]; then echo "✓ PR $(BASE) → $(MAIN) ya abierto: $$open"; \
		else \
			if [ "$(yes)" != 1 ]; then \
				printf '¿Crear el PR %s → %s? [s/N]: ' "$(BASE)" "$(MAIN)" >&2; read -r answer || answer=""; \
				[[ "$$answer" =~ ^[sSyY] ]] || { echo "Sin crear el PR."; exit 0; }; \
			fi; \
			gh pr create --base $(MAIN) --head $(BASE) --title "Publicar develop en main" --body "Fusionar con merge commit: nunca squash ni rebase and merge."; \
		fi; \
	else \
		url=$$($(REPO_URL_SH)); echo "gh no está autenticado; abre el PR aquí:"; echo "$$url/compare/$(MAIN)...$(BASE)?expand=1"; \
	fi; \
	echo "Recuerda: este PR se fusiona con merge commit (nunca squash ni rebase and merge); make promote no fusiona."

##@ Calidad

build: ## Compila (cargo build)
	cargo build

test: ## Pruebas (cargo test); run=<nombre> filtra por nombre
	cargo test $(run)

lint: ## clippy sin advertencias (cargo clippy --all-targets -- -D warnings)
	cargo clippy --all-targets -- -D warnings

ui-check: ## Sintaxis de la interfaz (node --check ui/app.js)
	@command -v node >/dev/null || { echo "✗ falta node: source ~/.nvm/nvm.sh" >&2; exit 1; }
	node --check ui/app.js

# El modo simulado va fijo: desde make no se puede forzar el real.
run: ## Corre la app en modo simulado (SIGNAL_SIMULADO=1); args= pasa argumentos
	SIGNAL_SIMULADO=1 cargo run -- $(args)

bitacora: ## Verifica la bitácora con scripts/verificar-bitacora.sh (si aún no existe, avisa y sigue)
	@if [ -f scripts/verificar-bitacora.sh ]; then bash scripts/verificar-bitacora.sh; else echo "(aún no existe scripts/verificar-bitacora.sh)"; fi

check: ## Definición de «terminado» de CLAUDE.md: ui-check, bitácora, lint y test, en ese orden
	@$(MAKE) --no-print-directory ui-check
	@$(MAKE) --no-print-directory bitacora
	@$(MAKE) --no-print-directory lint
	@$(MAKE) --no-print-directory test

##@ Ayuda

help: ## Muestra esta ayuda
	@awk 'BEGIN { FS = ":.*## " } \
	/^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } \
	/^[a-z-]+:.*## / { printf "  \033[36m%-11s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@echo
	@echo "Opciones: n=4.10 tema=algo  title=  force=1  web=1  yes=1  run=  args="
