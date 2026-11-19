#!/bin/bash
# Publie une version d'Automat-PDF depuis ce Mac : l'équivalent local du workflow « Installateurs »
# de GitHub Actions (désactivé le 19/11/2026, pas de service payant). GitHub ne sert plus qu'à
# héberger le code et les publications, gratuitement.
#
#   1. vérifie : dossier sans modification, commit déjà sur GitHub (origin/main), même numéro de
#      version partout, notes de version écrites, version pas encore publiée ;
#   2. lit la clé privée des mises à jour (fichier hors des dépôts, ~/.tauri/automat-pdf.key) et son
#      mot de passe (trousseau de macOS) ; vérifie qu'elle est bien celle des Automat-PDF installés ;
#   3. compile l'installateur Windows (NSIS, depuis ce Mac) et l'application Mac universelle (.dmg),
#      avec leurs fichiers de mise à jour signés, puis vérifie chaque signature ;
#   4. écrit latest.json (Windows, Mac Intel et Apple Silicon), que lisent les Automat-PDF installés ;
#   5. crée la publication GitHub en BROUILLON, avec les fichiers et les notes de version
#      (notes-de-version/<version>.md, complétées des empreintes SHA-256). À relire, essayer, puis
#      publier : gh release edit v<version> --draft=false (ou « Publish release » sur GitHub). C'est
#      la publication qui crée l'étiquette v<version> et que voient les applications installées.
#
# Options :
#   --essai         tout compiler et vérifier avec une clé jetable, sans rien envoyer (fichiers
#                   dans dist-publication/) : pour essayer la chaîne ou un installateur sur un PC
#   --remplacer     remplacer un brouillon déjà créé pour cette version
#   --sans-windows, --sans-mac
#   --cle FICHIER   la clé privée (sinon $AUTOMAT_PDF_CLE, sinon ~/.tauri/automat-pdf.key)
#
# Réglages hors du dépôt (jamais publiés) :
#   ~/.tauri/automat-pdf.key                      la clé privée des mises à jour (à garder aussi en lieu sûr)
#   trousseau, « automat-pdf-cle-mises-a-jour »   son mot de passe (demandé une fois, puis gardé)
#   ~/.config/automat-pdf/statistiques            l'adresse qui reçoit les statistiques anonymes
#                                                 (sans elle, l'application ne pose pas la question)

set -euo pipefail

racine="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=mac/outils.sh
. "$racine/scripts/mac/outils.sh"

depot="242Neguev/Automat-PDF"
service_trousseau="automat-pdf-cle-mises-a-jour"
essai=0; remplacer=0; windows=1; mac=1
cle="${AUTOMAT_PDF_CLE:-$HOME/.tauri/automat-pdf.key}"
while [ $# -gt 0 ]; do
  case "$1" in
    --essai) essai=1 ;;
    --remplacer) remplacer=1 ;;
    --sans-windows) windows=0 ;;
    --sans-mac) mac=0 ;;
    --cle) shift; cle="${1:?--cle attend le chemin du fichier de clé}" ;;
    -h|--aide|--help) sed -n '2,34p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echec "option inconnue : $1 (voir --aide)" ;;
  esac
  shift
done
[ "$windows" = 1 ] || [ "$mac" = 1 ] || echec "rien à compiler (--sans-windows et --sans-mac)."

etape "Vérification des prérequis"
[ "$(uname -s)" = "Darwin" ] || echec "ce script se lance sur un Mac."
preparer_outils_apple
preparer_rust aarch64-apple-darwin x86_64-apple-darwin
preparer_node
[ "$windows" = 1 ] && preparer_outils_windows
cd "$racine"
preparer_dossier_cible
if [ "$essai" = 0 ]; then
  command -v gh >/dev/null 2>&1 || [ -x "$HOME/.local/bin/gh" ] || echec "GitHub CLI (gh) introuvable : il crée la publication."
  command -v gh >/dev/null 2>&1 || export PATH="$HOME/.local/bin:$PATH"
  gh auth status >/dev/null 2>&1 || echec "gh n'est pas connecté à GitHub : lancez « gh auth login »."
fi

# ------------------------------------------------------------------------------------------- 1.
etape "Version et code à publier"
version="$(node -p "require('./package.json').version")"
version_tauri="$(node -p "require('./src-tauri/tauri.conf.json').version")"
version_cargo="$(awk -F'"' '/^version *=/ { print $2; exit }' src-tauri/Cargo.toml)"
[ "$version" = "$version_tauri" ] && [ "$version" = "$version_cargo" ] \
  || echec "numéros de version différents : package.json $version, tauri.conf.json $version_tauri, Cargo.toml $version_cargo."
etiquette="v$version"
commit="$(git rev-parse HEAD)"
echo "Automat-PDF $version, commit ${commit:0:7} ($(git log -1 --format=%s))"
notes="notes-de-version/$version.md"
if [ "$essai" = 0 ]; then
  [ -z "$(git status --porcelain)" ] || echec "le dossier a des modifications non commitées : la publication doit correspondre exactement à un commit."
  git fetch --quiet origin main || echec "GitHub injoignable (git fetch)."
  [ "$commit" = "$(git rev-parse origin/main)" ] || echec "ce commit n'est pas le dernier de main sur GitHub : fusionnez et poussez d'abord, puis placez-vous sur main à jour."
  [ -s "$notes" ] || echec "notes de version absentes : écrivez $notes (elles deviennent le texte de la publication)."
  if gh release view "$etiquette" --repo "$depot" >/dev/null 2>&1; then
    brouillon="$(gh release view "$etiquette" --repo "$depot" --json isDraft --jq .isDraft)"
    [ "$brouillon" = "true" ] || echec "$etiquette est déjà publiée : changez de numéro de version."
    [ "$remplacer" = 1 ] || echec "un brouillon $etiquette existe déjà : --remplacer pour le refaire."
  fi
fi

# ------------------------------------------------------------------------------------------- 2.
etape "Clé des mises à jour"
cle_publique="$(node -p "require('./src-tauri/tauri.conf.json').plugins.updater.pubkey")"
travail="$(mktemp -d "${TMPDIR:-/tmp}/automat-pdf-publier.XXXXXX")"
trap 'rm -rf "$travail"' EXIT
if [ "$essai" = 1 ]; then
  # Clé jetable : les fichiers produits ne pourront JAMAIS servir de mise à jour aux applications
  # installées (leur clé publique est autre), ni être publiés par ce script.
  mot_de_passe="essai-$(date +%s)"
  CI=true npx tauri signer generate --ci -w "$travail/essai.key" -p "$mot_de_passe" >/dev/null 2>&1 \
    || echec "clé d'essai impossible à créer."
  cle="$travail/essai.key"
  cle_publique="$(cat "$travail/essai.key.pub")"
  echo "Essai : clé jetable (rien ne sera envoyé)."
else
  [ -f "$cle" ] || echec "clé privée introuvable : $cle. Copiez-y le fichier de clé gardé en lieu sûr (ou --cle FICHIER)."
  mot_de_passe="$(security find-generic-password -a "$USER" -s "$service_trousseau" -w 2>/dev/null || true)"
  if [ -z "$mot_de_passe" ]; then
    echo "Mot de passe de la clé : il sera gardé dans le trousseau de macOS (« $service_trousseau »)."
    echo "Tapez-le, puis une seconde fois pour confirmer :"
    security add-generic-password -a "$USER" -s "$service_trousseau" -l "Automat-PDF : clé des mises à jour" -U -w \
      || echec "mot de passe non enregistré."
    mot_de_passe="$(security find-generic-password -a "$USER" -s "$service_trousseau" -w)"
  fi
fi
# La clé (son contenu) et son mot de passe passent par l'environnement, jamais par la ligne de
# commande ; « tauri build » et « tauri signer sign » les y lisent tous deux.
export TAURI_SIGNING_PRIVATE_KEY="$(tr -d '[:space:]' < "$cle")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$mot_de_passe"
printf 'Automat-PDF %s\n' "$version" > "$travail/controle.txt"
npx tauri signer sign "$travail/controle.txt" >/dev/null 2>&1 \
  || { [ "$essai" = 0 ] && security delete-generic-password -a "$USER" -s "$service_trousseau" >/dev/null 2>&1; \
       echec "la clé n'a pas pu signer : mot de passe faux (il est retiré du trousseau, il sera redemandé) ou fichier de clé abîmé."; }
node scripts/mac/verifier-signature.cjs "$travail/controle.txt" "$travail/controle.txt.sig" "$cle_publique" >/dev/null \
  || echec "cette clé n'est PAS celle des Automat-PDF installés (clé publique de src-tauri/tauri.conf.json) : ils refuseraient la mise à jour."
echo "Clé : OK (c'est bien celle qu'attendent les applications installées)."

# Adresse des statistiques anonymes (secret de GitHub autrefois) : hors du dépôt public.
unset AUTOMAT_PDF_STATISTIQUES
if [ "$essai" = 0 ]; then
  reglage="$HOME/.config/automat-pdf/statistiques"
  [ -s "$reglage" ] || echec "adresse des statistiques absente : écrivez-la dans $reglage (une ligne, https://…)."
  AUTOMAT_PDF_STATISTIQUES="$(head -n 1 "$reglage" | tr -d '[:space:]')"
  case "$AUTOMAT_PDF_STATISTIQUES" in https://*) ;; *) echec "adresse des statistiques invalide dans $reglage (https:// attendu)." ;; esac
  export AUTOMAT_PDF_STATISTIQUES
  echo "Statistiques : $AUTOMAT_PDF_STATISTIQUES"
else
  echo "Statistiques : aucune adresse (essai : l'application ne posera pas la question)."
fi

# ------------------------------------------------------------------------------------------- 3.
etape "Dépendances de l'interface (npm ci)"
npm ci --no-fund --no-audit || echec "npm ci a échoué."

sortie="$racine/dist-publication/$version$([ "$essai" = 1 ] && echo "-essai")"
rm -rf "$sortie" && mkdir -p "$sortie"
fichiers=()
signature_de() { tr -d '\n' < "$1"; }
verifier() { # fichier fichier.sig
  local confiance
  confiance="$(node scripts/mac/verifier-signature.cjs "$1" "$2" "$cle_publique")" || echec "signature refusée pour $(basename "$1")."
  echo "Signature vérifiée : $(basename "$1") ($confiance)"
}

if [ "$windows" = 1 ]; then
  etape "Installateur Windows (compilé sur ce Mac, 5 à 15 minutes)"
  npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis \
    --config src-tauri/tauri.publication.conf.json || echec "la compilation Windows a échoué : voir les messages ci-dessus."
  nsis="$cible/x86_64-pc-windows-msvc/release/bundle/nsis"
  exe="Automat-PDF_${version}_x64-setup.exe"
  [ -f "$nsis/$exe" ] && [ -f "$nsis/$exe.sig" ] || echec "installateur ou signature introuvable dans $nsis."
  cp "$nsis/$exe" "$nsis/$exe.sig" "$sortie/"
  verifier "$sortie/$exe" "$sortie/$exe.sig"
  fichiers+=("$sortie/$exe" "$sortie/$exe.sig")
fi

if [ "$mac" = 1 ]; then
  etape "Application Mac universelle (Intel et Apple Silicon, 10 à 30 minutes)"
  npm run tauri build -- --target universal-apple-darwin --bundles app,dmg \
    --config src-tauri/tauri.publication.conf.json || echec "la compilation Mac a échoué : voir les messages ci-dessus."
  bundle="$cible/universal-apple-darwin/release/bundle"
  dmg="Automat-PDF_${version}_universal.dmg"
  archive="Automat-PDF_${version}_universal.app.tar.gz"
  [ -f "$bundle/dmg/$dmg" ] && [ -f "$bundle/macos/Automat-PDF.app.tar.gz.sig" ] || echec "image disque ou mise à jour signée introuvable dans $bundle."
  cp "$bundle/dmg/$dmg" "$sortie/"
  cp "$bundle/macos/Automat-PDF.app.tar.gz" "$sortie/$archive"
  cp "$bundle/macos/Automat-PDF.app.tar.gz.sig" "$sortie/$archive.sig"
  verifier "$sortie/$archive" "$sortie/$archive.sig"
  fichiers+=("$sortie/$dmg" "$sortie/$archive" "$sortie/$archive.sig")
fi

# ------------------------------------------------------------------------------------------- 4.
etape "latest.json (ce que lisent les Automat-PDF installés)"
adresse="https://github.com/$depot/releases/download/$etiquette"
WINDOWS_SIG="$([ "$windows" = 1 ] && signature_de "$sortie/$exe.sig" || true)" \
MAC_SIG="$([ "$mac" = 1 ] && signature_de "$sortie/$archive.sig" || true)" \
VERSION="$version" ADRESSE="$adresse" EXE="${exe:-}" ARCHIVE="${archive:-}" \
node -e '
  const e = process.env, plateformes = {};
  // Clés lues par le module de mise à jour de Tauri : « système-processeur-type », puis « système-processeur ».
  if (e.WINDOWS_SIG) for (const cle of ["windows-x86_64", "windows-x86_64-nsis"]) plateformes[cle] = { signature: e.WINDOWS_SIG, url: `${e.ADRESSE}/${e.EXE}` };
  if (e.MAC_SIG) for (const cle of ["darwin-aarch64", "darwin-x86_64", "darwin-aarch64-app", "darwin-x86_64-app"]) plateformes[cle] = { signature: e.MAC_SIG, url: `${e.ADRESSE}/${e.ARCHIVE}` };
  process.stdout.write(JSON.stringify({ version: e.VERSION, notes: "", pub_date: new Date().toISOString(), platforms: plateformes }, null, 2) + "\n");
' > "$sortie/latest.json"
cat "$sortie/latest.json" | sed -n '1,4p'
fichiers+=("$sortie/latest.json")

# Empreintes, pour les notes de version, la page du site et la fiche produit.
empreintes="$sortie/SHA256SUMS.txt"
(cd "$sortie" && shasum -a 256 ./*.exe ./*.dmg 2>/dev/null | sed 's#\./##') > "$empreintes" || true
etape "Fichiers produits ($sortie)"
ls -l "$sortie" | tail -n +2
cat "$empreintes"

if [ "$essai" = 1 ]; then
  etape "Essai terminé : rien n'a été envoyé"
  echo "Les fichiers sont signés par une clé jetable : bons pour essayer l'installation sur un PC ou un Mac,"
  echo "jamais pour une mise à jour. La vraie publication : scripts/publier.command (sans --essai)."
  open "$sortie" 2>/dev/null || true
  attendre
  exit 0
fi

# ------------------------------------------------------------------------------------------- 5.
etape "Publication GitHub en brouillon"
texte="$travail/notes.md"
{
  cat "$notes"
  echo
  echo "## Vérifier les fichiers"
  echo
  [ "$windows" = 1 ] && printf 'Empreinte SHA-256 de `%s` (PowerShell : `Get-FileHash .\\%s`) :\n\n```\n%s\n```\n\n' "$exe" "$exe" "$(awk -v f="$exe" '$2 == f { print $1 }' "$empreintes")"
  [ "$mac" = 1 ] && printf 'Empreinte SHA-256 de `%s` (Terminal : `shasum -a 256 ~/Downloads/%s`) :\n\n```\n%s\n```\n\n' "$dmg" "$dmg" "$(awk -v f="$dmg" '$2 == f { print $1 }' "$empreintes")"
  printf 'Ces fichiers ont été compilés sur un Mac par `scripts/publier.command`, à partir du code de ce dépôt (commit %s) ; l'"'"'installateur Windows par compilation croisée (cargo-xwin, NSIS).\n' "${commit:0:7}"
} > "$texte"
if [ "$remplacer" = 1 ] && gh release view "$etiquette" --repo "$depot" >/dev/null 2>&1; then
  gh release delete "$etiquette" --repo "$depot" --yes || echec "l'ancien brouillon n'a pas pu être retiré."
fi
gh release create "$etiquette" --repo "$depot" --draft --target "$commit" \
  --title "Automat-PDF $version" --notes-file "$texte" "${fichiers[@]}" \
  || echec "la publication n'a pas pu être créée : voir les messages ci-dessus."
lien="$(gh release view "$etiquette" --repo "$depot" --json url --jq .url)"

etape "Brouillon prêt : $lien"
echo "1. Relisez la publication, essayez l'installateur Windows sur un PC et l'image disque sur un Mac."
echo "2. Publiez : gh release edit $etiquette --repo $depot --draft=false  (ou « Publish release » sur GitHub)."
echo "   La publication crée l'étiquette $etiquette ; dès lors, les Automat-PDF installés proposent la mise à jour."
echo "3. Empreintes pour la présentation et le site : $empreintes"
attendre
