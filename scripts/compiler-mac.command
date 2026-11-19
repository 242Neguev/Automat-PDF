#!/bin/bash
# Compile Automat-PDF pour Mac sur ce Mac : l'application (.app) et son image disque (.dmg), pour
# les Mac à processeur Intel comme Apple Silicon. Pendant de compiler-windows.cmd.
#
# Double-cliquer sur ce fichier dans le Finder (ou le lancer dans un Terminal) :
#   1. installe si besoin les prérequis, une seule fois et sans droits d'administrateur : les outils
#      de ligne de commande d'Apple (fenêtre d'Apple), Rust (dans ~/.cargo) et Node.js LTS (dans
#      ~/.local/share/automat-pdf-compilation) ;
#   2. installe les dépendances de l'interface (npm ci) ;
#   3. compile l'application et produit le .dmg, puis l'affiche dans le Finder.
#
# Option : --natif  seulement pour le processeur de ce Mac (deux fois plus rapide, pour un essai).
# Pour publier une version (Windows et Mac, fichiers de mise à jour signés) : publier.command.

set -euo pipefail

racine="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=mac/outils.sh
. "$racine/scripts/mac/outils.sh"
natif=0
[ "${1:-}" = "--natif" ] && natif=1

etape "Vérification des prérequis"
[ "$(uname -s)" = "Darwin" ] || echec "ce script compile la version Mac : lancez-le sur un Mac."
preparer_outils_apple
if [ "$natif" = 0 ]; then
  # Les deux processeurs des Mac, pour l'application universelle.
  preparer_rust aarch64-apple-darwin x86_64-apple-darwin
else
  preparer_rust
fi
preparer_node
cd "$racine"
preparer_dossier_cible

etape "Installation des dépendances de l'interface (npm ci)"
npm ci --no-fund --no-audit || echec "npm ci a échoué : voir les messages ci-dessus."

if [ "$natif" = 1 ]; then
  etape "Compilation de l'application pour ce Mac (la première fois : 5 à 15 minutes)"
  npm run tauri build -- --bundles app,dmg || echec "la compilation a échoué : voir les messages ci-dessus."
  bundle="$cible/release/bundle"
else
  etape "Compilation de l'application pour les Mac Intel et Apple Silicon (la première fois : 10 à 30 minutes)"
  npm run tauri build -- --target universal-apple-darwin --bundles app,dmg || echec "la compilation a échoué : voir les messages ci-dessus."
  bundle="$cible/universal-apple-darwin/release/bundle"
fi

dmg="$(ls -t "$bundle"/dmg/*.dmg 2>/dev/null | head -n 1 || true)"
[ -n "$dmg" ] || echec "aucune image disque (.dmg) trouvée dans $bundle/dmg."

etape "Terminé : image disque produite"
printf '  \033[32m%s\033[0m\n' "$dmg"
echo
echo "Pour installer : ouvrir le .dmg, puis glisser Automat-PDF dans le dossier Applications."
echo "L'application n'est pas signée par Apple : la première fois, macOS refuse de l'ouvrir."
echo "Ouvrez alors Réglages Système › Confidentialité et sécurité, puis « Ouvrir quand même »."
open -R "$dmg"
attendre
