# shellcheck shell=bash
# Outils communs aux scripts du Mac (compiler-mac, verifier, publier) : à « sourcer », pas à lancer.
#
# Tout s'installe une fois, sans droits d'administrateur, dans ~/.local/share/automat-pdf-compilation,
# et chaque téléchargement est vérifié (empreinte SHA-256 notée ici, ou publiée par son auteur).
# Compatible avec le bash d'origine du Mac (3.2).
#
#   preparer_outils_apple     compilateur, éditeur de liens et SDK de macOS (DEVELOPER_DIR au besoin)
#   preparer_rust [cibles…]   Rust stable, et les processeurs demandés
#   preparer_node             Node.js 20 ou plus
#   preparer_dossier_cible    $cible : dossier de compilation (hors d'iCloud Drive)
#   preparer_outils_windows   compilation de la version Windows sur ce Mac : LLVM (clang-cl, lld-link,
#                             llvm-rc), NSIS (makensis) et cargo-xwin, qui fournit les bibliothèques
#                             Windows de Microsoft (sous leur licence) à la première compilation.

outils="${AUTOMAT_PDF_OUTILS:-$HOME/.local/share/automat-pdf-compilation}"

# Versions et empreintes des outils de la compilation Windows (relevées le 19/11/2026).
LLVM_VERSION="23.1.5"
LLVM_ARCHIVE="LLVM-$LLVM_VERSION-macOS-ARM64.tar.xz"
LLVM_SHA256="aba592b4a15e8f90c634c5367b51ec8418b8bcd2829865db331a17dc9418dcff"
NSIS_VERSION="3.13"
NSIS_SOURCES_SHA256="a8ffe024602d46b6d766f9e1ce30c324ad2a24daeacd3efc2642d436a0c157ac"
NSIS_ZIP_SHA256="ba63dffc4410ee89193e1cb5a41989991bd77c61068da17e3156d136b7b0b3d8"
SCONS_VERSION="4.11.1"
CARGO_XWIN_VERSION="0.23.1"

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

# Télécharge $1 vers $2 (reprise possible) et vérifie son empreinte $3 ; un fichier déjà là et juste
# n'est pas retéléchargé.
telecharger_verifie() {
  local url="$1" fichier="$2" attendu="$3"
  mkdir -p "$(dirname "$fichier")"
  if [ -f "$fichier" ] && [ "$(shasum -a 256 "$fichier" | awk '{print $1}')" = "$attendu" ]; then
    return 0
  fi
  echo "Téléchargement de $(basename "$fichier")…"
  local essai
  for essai in 1 2 3 4 5; do
    curl -fL --retry 3 --retry-delay 5 -C - -sS -o "$fichier" "$url" && break
    sleep 5
  done
  [ "$(shasum -a 256 "$fichier" | awk '{print $1}')" = "$attendu" ] \
    || { rm -f "$fichier"; echec "$(basename "$fichier") reçu abîmé (empreinte différente) : relancez le script."; }
}

preparer_outils_apple() {
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
    local attente=0
    until outils_apple; do
      sleep 10
      attente=$((attente + 10))
      if [ "$attente" -ge 2700 ]; then
        echec "les outils de ligne de commande d'Apple ne sont toujours pas installés. Si la fenêtre d'Apple n'a rien proposé (« logiciel non disponible »), mettez d'abord macOS à jour (Réglages Système › Général › Mise à jour de logiciels), puis relancez ce script."
      fi
    done
  fi
  echo "Outils d'Apple : OK ($(xcrun --show-sdk-path 2>/dev/null))"
}

preparer_rust() {
  [ -x "$HOME/.cargo/bin/rustup" ] && export PATH="$HOME/.cargo/bin:$PATH"
  if ! command -v rustup >/dev/null 2>&1; then
    etape "Installation de Rust (une seule fois)"
    local installateur
    installateur="$(mktemp -d)/rustup-init.sh"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$installateur" || echec "Rust n'a pas pu être téléchargé : vérifiez la connexion à internet."
    sh "$installateur" -y --profile minimal --default-toolchain stable || echec "l'installation de Rust a échoué : voir les messages ci-dessus."
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
  rustup show active-toolchain >/dev/null 2>&1 || rustup default stable
  if [ $# -gt 0 ]; then
    rustup target add "$@" >/dev/null || echec "impossible d'ajouter à Rust : $*."
  fi
  echo "Rust : $(rustc --version)"
}

preparer_node() {
  # Node.js 20 ou plus : celui de ce Mac, sinon une copie dans le dossier de compilation.
  [ -x "$outils/node/bin/node" ] && export PATH="$outils/node/bin:$PATH"
  local majeur
  majeur="$(node --version 2>/dev/null | sed 's/^v//' | cut -d. -f1 || true)"
  if [ -z "${majeur:-}" ] || [ "${majeur:-0}" -lt 20 ] 2>/dev/null; then
    etape "Installation de Node.js LTS (une seule fois, dans $outils/node)"
    local version processeur fichier telechargement attendu obtenu
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
}

# Dans iCloud Drive (Bureau et Documents synchronisés), les fichiers intermédiaires de la
# compilation (plusieurs Go) partiraient dans le nuage : la compilation se fait alors à part.
preparer_dossier_cible() {
  cible="${CARGO_TARGET_DIR:-}"
  [ -n "$cible" ] && return 0
  local nuage="$HOME/Library/Mobile Documents/com~apple~CloudDocs" dans_icloud=0
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
}

# --------------------------------------------------------------------------------------------------
# Compilation de la version Windows sur ce Mac (expérimentale chez Tauri : NSIS seulement, pas de
# .msi). Ce qu'un PC Windows apporte d'office est fourni ici :
#   - LLVM : clang-cl (compilateur C de Microsoft émulé), lld-link (éditeur de liens), llvm-rc
#     (ressources : icône, numéro de version) ; seuls ces outils sont gardés (environ 400 Mo) ;
#   - NSIS : makensis, compilé ici depuis ses sources (comme le fait Homebrew), avec les modules
#     Windows de sa distribution officielle ;
#   - cargo-xwin : télécharge à la première compilation les bibliothèques et en-têtes Windows de
#     Microsoft (CRT et SDK), sous la licence de Microsoft ; gardés dans ~/Library/Caches/cargo-xwin.
# --------------------------------------------------------------------------------------------------
preparer_outils_windows() {
  [ "$(uname -m)" = "arm64" ] || echec "la compilation de la version Windows se fait sur un Mac à puce Apple (outils arm64)."
  local llvm="$outils/llvm-$LLVM_VERSION" nsis="$outils/nsis-$NSIS_VERSION"

  if [ ! -x "$llvm/bin/llvm-rc" ] || [ ! -x "$llvm/bin/clang-cl" ] || [ ! -x "$llvm/bin/lld-link" ]; then
    etape "LLVM $LLVM_VERSION pour la compilation Windows (une seule fois, 1,5 Go à télécharger)"
    local archive="$outils/telechargements/$LLVM_ARCHIVE" dossier="${LLVM_ARCHIVE%.tar.xz}"
    telecharger_verifie "https://github.com/llvm/llvm-project/releases/download/llvmorg-$LLVM_VERSION/$LLVM_ARCHIVE" "$archive" "$LLVM_SHA256"
    rm -rf "$llvm" && mkdir -p "$llvm"
    local majeur="${LLVM_VERSION%%.*}"
    tar -xJf "$archive" -C "$llvm" --strip-components 1 \
      "$dossier/bin/clang" "$dossier/bin/clang-$majeur" "$dossier/bin/clang-cl" \
      "$dossier/bin/lld" "$dossier/bin/lld-link" "$dossier/bin/llvm-rc" "$dossier/bin/llvm-ar" "$dossier/bin/llvm-lib" \
      "$dossier/lib/clang" || echec "LLVM n'a pas pu être déballé."
    rm -f "$archive"
  fi

  if [ ! -x "$nsis/Bin/makensis" ]; then
    etape "NSIS $NSIS_VERSION (makensis), compilé sur ce Mac (une seule fois)"
    telecharger_verifie "https://downloads.sourceforge.net/project/nsis/NSIS%203/$NSIS_VERSION/nsis-$NSIS_VERSION-src.tar.bz2" \
      "$outils/telechargements/nsis-$NSIS_VERSION-src.tar.bz2" "$NSIS_SOURCES_SHA256"
    telecharger_verifie "https://downloads.sourceforge.net/project/nsis/NSIS%203/$NSIS_VERSION/nsis-$NSIS_VERSION.zip" \
      "$outils/telechargements/nsis-$NSIS_VERSION.zip" "$NSIS_ZIP_SHA256"
    [ -x "$outils/scons/bin/scons" ] || {
      /usr/bin/python3 -m venv "$outils/scons" >/dev/null && "$outils/scons/bin/pip" install -q "scons==$SCONS_VERSION" >/dev/null
    } || echec "SCons (outil de compilation de NSIS) n'a pas pu être installé."
    local sources sdk
    sources="$(mktemp -d)"
    tar -xjf "$outils/telechargements/nsis-$NSIS_VERSION-src.tar.bz2" -C "$sources" --strip-components 1
    sdk="$(xcrun --show-sdk-path)"
    # SCons vide l'environnement de ce qu'il lance : compilateur et SDK lui sont donnés en clair, et
    # DEVELOPER_DIR est remis dans chacun de ses environnements par site_scons/site_init.py, qu'il lit
    # avant tout (sans lui, ses tests de configuration appellent les outils d'Apple de /usr/bin, qui
    # ouvrent la fenêtre d'installation d'Apple quand ils ne sont pas installés à leur place habituelle).
    mkdir -p "$sources/site_scons"
    cat > "$sources/site_scons/site_init.py" <<'PY'
import os
import SCons.Platform.darwin as _darwin
_generer = _darwin.generate
def _generer_avec_outils_apple(env):
    _generer(env)
    for _nom in ('DEVELOPER_DIR', 'SDKROOT'):
        if _nom in os.environ:
            env['ENV'][_nom] = os.environ[_nom]
_darwin.generate = _generer_avec_outils_apple
PY
    (cd "$sources" && "$outils/scons/bin/scons" makensis \
      CC="$(xcrun --find clang)" CXX="$(xcrun --find clang++)" \
      "APPEND_CCFLAGS=-isysroot $sdk -mmacosx-version-min=11.0" "APPEND_LINKFLAGS=-isysroot $sdk -mmacosx-version-min=11.0" \
      SKIPMISC=all SKIPPLUGINS=all SKIPSTUBS=all SKIPUTILS=all STRIP=0 VERSION="$NSIS_VERSION" NSIS_CONFIG_CONST_DATA_PATH=no >/dev/null) \
      || echec "makensis n'a pas pu être compilé."
    rm -rf "$nsis" && mkdir -p "$outils/nsis-tmp"
    /usr/bin/ditto -x -k "$outils/telechargements/nsis-$NSIS_VERSION.zip" "$outils/nsis-tmp"
    mv "$outils/nsis-tmp/nsis-$NSIS_VERSION" "$nsis" && rm -rf "$outils/nsis-tmp"
    # Version « portable » : makensis cherche ses modules (Stubs, Plugins, Include) dans le dossier
    # parent du sien.
    cp "$sources/build/urelease/makensis/makensis" "$nsis/Bin/makensis"
    rm -rf "$sources"
  fi

  if [ "$(cargo xwin --version 2>/dev/null | awk '{print $2}')" != "$CARGO_XWIN_VERSION" ]; then
    etape "cargo-xwin $CARGO_XWIN_VERSION (une seule fois)"
    cargo install --locked --quiet "cargo-xwin@$CARGO_XWIN_VERSION" || echec "cargo-xwin n'a pas pu être installé."
  fi
  rustup target add x86_64-pc-windows-msvc >/dev/null || echec "impossible d'ajouter la cible Windows à Rust."

  # À la fin du PATH : les outils de macOS gardent la priorité pour ce qui se compile pour le Mac.
  export PATH="$PATH:$llvm/bin:$nsis/Bin"
  # makensis lit ses scripts en UTF-8.
  export LANG="${LANG:-en_US.UTF-8}"
  echo "Windows : LLVM $LLVM_VERSION, NSIS $("$nsis/Bin/makensis" -VERSION 2>/dev/null), cargo-xwin $CARGO_XWIN_VERSION"
}
