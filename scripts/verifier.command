#!/bin/bash
# Vérifie Automat-PDF sur ce Mac : l'équivalent local du workflow « Vérifications » de GitHub
# Actions (désactivé le 19/11/2026, pas de service payant).
#
# Comme GitHub le faisait pour chaque envoi, sous Windows et sous macOS :
#   1. interface : npm ci, compilation TypeScript + Vite (npm run build), tests (npm test) ;
#   2. cœur : mise en forme (cargo fmt --check), lints Clippy stricts et tests Rust, pour macOS ;
#   3. cœur pour Windows : lints Clippy stricts et compilation des tests, depuis ce Mac (cargo-xwin) ;
#      les tests Windows eux-mêmes ne peuvent tourner que sur un PC (scripts\compiler-windows.cmd
#      compile ; cargo test sur le PC les lance).
#
# Options : --sans-windows  saute l'étape 3 (pas de téléchargement des outils Windows).
# Code de sortie : 0 si tout est vert. Double-cliquable depuis le Finder.

set -euo pipefail

racine="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=mac/outils.sh
. "$racine/scripts/mac/outils.sh"
windows=1
[ "${1:-}" = "--sans-windows" ] && windows=0

etape "Vérification des prérequis"
[ "$(uname -s)" = "Darwin" ] || echec "ce script se lance sur un Mac."
preparer_outils_apple
preparer_rust
rustup component add clippy rustfmt >/dev/null 2>&1 || echec "impossible d'ajouter Clippy et rustfmt à Rust."
preparer_node
[ "$windows" = 1 ] && preparer_outils_windows
cd "$racine"
preparer_dossier_cible

journaux="$(mktemp -d "${TMPDIR:-/tmp}/automat-pdf-verifier.XXXXXX")"
bilan=()
echecs=0
passer() { # nom commande…
  local nom="$1" journal debut=$SECONDS
  shift
  journal="$journaux/$(printf '%s' "$nom" | tr -c 'A-Za-z0-9' '-').log"
  printf '%-58s' "$nom"
  if "$@" >"$journal" 2>&1; then
    printf '\033[32m✓\033[0m %ss\n' "$((SECONDS - debut))"
    bilan+=("✓ $nom")
  else
    printf '\033[31m✗\033[0m %ss — %s\n' "$((SECONDS - debut))" "$journal"
    tail -n 30 "$journal" | sed 's/^/    /'
    bilan+=("✗ $nom")
    echecs=$((echecs + 1))
  fi
}
manifeste="src-tauri/Cargo.toml"

etape "Interface"
passer "Dépendances (npm ci)" npm ci --no-fund --no-audit
passer "Compilation TypeScript + Vite (npm run build)" npm run build
passer "Tests de l'interface (npm test)" npm test

etape "Cœur, pour macOS"
passer "Mise en forme (cargo fmt --check)" cargo fmt --manifest-path "$manifeste" --check
passer "Lints Clippy stricts" cargo clippy --manifest-path "$manifeste" --all-targets -- -D warnings
passer "Tests Rust" cargo test --manifest-path "$manifeste"

if [ "$windows" = 1 ]; then
  etape "Cœur, pour Windows (depuis ce Mac)"
  passer "Lints Clippy stricts (Windows)" cargo xwin clippy --manifest-path "$manifeste" --target x86_64-pc-windows-msvc --all-targets -- -D warnings
  passer "Compilation des tests (Windows)" cargo xwin test --manifest-path "$manifeste" --target x86_64-pc-windows-msvc --no-run
fi

etape "Bilan"
printf '%s\n' "${bilan[@]}"
if [ "$echecs" -eq 0 ]; then
  printf '\n\033[32mTout est vert.\033[0m Journaux : %s\n' "$journaux"
  attendre
  exit 0
fi
printf '\n\033[31m%d vérification(s) en échec.\033[0m Journaux : %s\n' "$echecs" "$journaux"
attendre
exit 1
