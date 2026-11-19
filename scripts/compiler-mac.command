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

set -euo pipefail

racine="$(cd "$(dirname "$0")/.." && pwd)"
outils="$HOME/.local/share/automat-pdf-compilation"
natif=0
[ "${1:-}" = "--natif" ] && natif=1

# Double-cliqué depuis le Finder, le script garde sa fenêtre ouverte à la fin : le temps de lire.
attendre() {
  if [ -t 0 ]; then
    echo
    read -r -p "Appuyez sur Entrée pour fermer cette fenêtre." _ || true
  fi
}
etape() { printf '\n\033[36m==> %s\033[0m\n' "$1"; }
echec() {
  printf '\n\033[31mÉCHEC : %s\033[0m\n' "$1"
  attendre
  exit 1
}

etape "Vérification des prérequis"
[ "$(uname -s)" = "Darwin" ] || echec "ce script compile la version Mac : lancez-le sur un Mac."

# Outils de ligne de commande d'Apple : compilateur, éditeur de liens, SDK de macOS. Des outils
# déballés dans le dossier de compilation (sans installation) servent s'il n'y en a pas d'autres.
if [ -z "${DEVELOPER_DIR:-}" ] && ! xcode-select -p >/dev/null 2>&1 && [ -x "$outils/CommandLineTools/usr/bin/clang" ]; then
  export DEVELOPER_DIR="$outils/CommandLineTools"
fi
outils_apple() { { [ -n "${DEVELOPER_DIR:-}" ] || xcode-select -p >/dev/null 2>&1; } && xcrun --find clang >/dev/null 2>&1; }
if ! outils_apple; then
  echo "Les outils de ligne de commande d'Apple manquent : une fenêtre d'Apple va proposer de les installer."
  echo "Cliquez sur « Installer », puis sur « Accepter » (environ 1 Go, 5 à 15 minutes) : le script reprendra tout seul."
  xcode-select --install >/dev/null 2>&1 || true
  attente=0
  until outils_apple; do
    sleep 10
    attente=$((attente + 10))
    if [ "$attente" -ge 2700 ]; then
      echec "les outils de ligne de commande d'Apple ne sont toujours pas installés. Si la fenêtre d'Apple n'a rien proposé (« logiciel non disponible »), mettez d'abord macOS à jour (Réglages Système › Général › Mise à jour de logiciels), puis relancez ce script."
    fi
  done
fi
echo "Outils d'Apple : OK ($(xcrun --show-sdk-path 2>/dev/null))"

# Rust (chaîne stable), et les deux processeurs des Mac pour l'application universelle.
[ -x "$HOME/.cargo/bin/rustup" ] && export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v rustup >/dev/null 2>&1; then
  etape "Installation de Rust (une seule fois)"
  installateur="$(mktemp -d)/rustup-init.sh"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$installateur" || echec "Rust n'a pas pu être téléchargé : vérifiez la connexion à internet."
  sh "$installateur" -y --profile minimal --default-toolchain stable || echec "l'installation de Rust a échoué : voir les messages ci-dessus."
  export PATH="$HOME/.cargo/bin:$PATH"
fi
rustup show active-toolchain >/dev/null 2>&1 || rustup default stable
if [ "$natif" = 0 ]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null || echec "impossible d'ajouter les processeurs Intel et Apple Silicon à Rust."
fi
echo "Rust : $(rustc --version)"

# Node.js 20 ou plus : celui de ce Mac, sinon une copie dans le dossier de compilation.
[ -x "$outils/node/bin/node" ] && export PATH="$outils/node/bin:$PATH"
majeur="$(node --version 2>/dev/null | sed 's/^v//' | cut -d. -f1 || true)"
if [ "${majeur:-0}" -lt 20 ] 2>/dev/null || [ -z "${majeur:-}" ]; then
  etape "Installation de Node.js LTS (une seule fois, dans $outils/node)"
  version="$(curl -fsSL https://nodejs.org/dist/index.tab | awk -F'\t' 'NR > 1 && $10 != "-" { print $1; exit }')"
  [ -n "$version" ] || echec "la dernière version de Node.js est introuvable : vérifiez la connexion à internet."
  processeur="$(uname -m)"
  [ "$processeur" = "x86_64" ] && processeur="x64"
  fichier="node-$version-darwin-$processeur.tar.gz"
  telechargement="$(mktemp -d)"
  curl -fsSL -o "$telechargement/$fichier" "https://nodejs.org/dist/$version/$fichier" || echec "Node.js n'a pas pu être téléchargé."
  curl -fsSL -o "$telechargement/SHASUMS256.txt" "https://nodejs.org/dist/$version/SHASUMS256.txt" || echec "les empreintes de Node.js n'ont pas pu être téléchargées."
  attendu="$(awk -v f="$fichier" '$2 == f { print $1 }' "$telechargement/SHASUMS256.txt")"
  obtenu="$(shasum -a 256 "$telechargement/$fichier" | awk '{ print $1 }')"
  [ -n "$attendu" ] && [ "$attendu" = "$obtenu" ] || echec "le fichier de Node.js reçu est abîmé (empreinte différente) : relancez le script."
  mkdir -p "$outils"
  rm -rf "$outils/node" "$outils/node-$version-darwin-$processeur"
  tar -xzf "$telechargement/$fichier" -C "$outils"
  mv "$outils/node-$version-darwin-$processeur" "$outils/node"
  rm -rf "$telechargement"
  export PATH="$outils/node/bin:$PATH"
fi
echo "Node.js : $(node --version)"

cd "$racine"

# Dans iCloud Drive (Bureau et Documents synchronisés), les fichiers intermédiaires de la
# compilation (plusieurs Go) partiraient dans le nuage : la compilation se fait alors à part.
cible="${CARGO_TARGET_DIR:-}"
if [ -z "$cible" ]; then
  nuage="$HOME/Library/Mobile Documents/com~apple~CloudDocs"
  dans_icloud=0
  case "$racine/" in
    "$HOME/Library/Mobile Documents/"*) dans_icloud=1 ;;
    "$HOME/Desktop/"*) [ -d "$nuage/Desktop" ] && dans_icloud=1 ;;
    "$HOME/Documents/"*) [ -d "$nuage/Documents" ] && dans_icloud=1 ;;
  esac
  if [ "$dans_icloud" = 1 ]; then
    cible="$HOME/Library/Caches/Automat-PDF-compilation/target"
    export CARGO_TARGET_DIR="$cible"
    echo "Le projet est dans iCloud Drive : la compilation se fait dans $cible"
  else
    cible="$racine/src-tauri/target"
  fi
fi

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
