#!/bin/bash
# Crée, une fois par Mac de publication, le certificat de signature Mac d'Eau sèche, « Eau sèche -
# Automat-PDF », auto-signé, et l'installe dans le trousseau : publier.command (et compiler-mac.command)
# signent ensuite l'application avec lui.
#
# Pourquoi : signée « ad hoc », l'application change d'identité à chaque version. Après une mise à jour,
# macOS redemanderait le mot de passe du trousseau (celui de la messagerie) et l'autorisation de piloter
# Word, Excel, PowerPoint et Mail. Signée par ce certificat, elle garde la même identité d'une version à
# l'autre, et macOS lui garde ses autorisations.
# Ce que ce n'est PAS : une signature d'Apple. macOS affiche toujours son avertissement à la première
# ouverture (« Élément « Automat-PDF » non ouvert ») ; seuls un certificat Developer ID d'Apple et la
# notarisation le feraient disparaître.
#
# Double-cliquer sur ce fichier (ou le lancer dans un Terminal). Il :
#   1. crée la clé et le certificat (RSA 3072, valable 20 ans, pour signer du code seulement) ;
#   2. en fait une sauvegarde protégée par un mot de passe que vous choisissez :
#      ~/.tauri/automat-pdf-certificat-mac.p12 — à garder aussi en lieu sûr, avec ce mot de passe,
#      comme la clé des mises à jour ;
#   3. l'installe dans le trousseau de session, puis demande à macOS de l'approuver pour signer du code,
#      sur ce Mac seulement : macOS demande votre mot de passe (fenêtre de macOS) ;
#   4. fait une signature d'essai : la première fois, macOS demande si « codesign » peut utiliser la clé,
#      répondez « Toujours autoriser » (avec votre mot de passe).
# Relancé, il dit si tout est en place, sans rien refaire.
#
# Option : --restaurer [FICHIER.p12]  sur un autre Mac, ou après une réinstallation : reprend la
#          sauvegarde (par défaut ~/.tauri/automat-pdf-certificat-mac.p12) au lieu d'en créer une.
#          Un NOUVEAU certificat changerait l'identité de l'application : les Mac où elle est installée
#          redemanderaient ses autorisations une fois, après la mise à jour suivante.

set -euo pipefail

racine="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=mac/outils.sh
. "$racine/scripts/mac/outils.sh"

sauvegarde="$HOME/.tauri/automat-pdf-certificat-mac.p12"
restaurer=0
while [ $# -gt 0 ]; do
  case "$1" in
    --restaurer)
      restaurer=1
      if [ $# -gt 1 ] && [ "${2#-}" = "$2" ]; then shift; sauvegarde="$1"; fi ;;
    -h|--aide|--help) awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"; exit 0 ;;
    *) echec "option inconnue : $1 (voir --aide)" ;;
  esac
  shift
done

etape "Vérifications"
[ "$(uname -s)" = "Darwin" ] || echec "ce script se lance sur un Mac."
trousseau="$HOME/Library/Keychains/login.keychain-db"
[ -f "$trousseau" ] || echec "trousseau de session introuvable : $trousseau."
for outil in security codesign openssl; do
  command -v "$outil" >/dev/null 2>&1 || echec "$outil introuvable (il fait partie de macOS)."
done

identite="$(identite_mac)"
if [ -n "$identite" ]; then
  etape "Déjà en place"
  echo "Le certificat « $CERTIFICAT_MAC_NOM » est installé et approuvé (empreinte SHA-1 $identite)."
  if [ -f "$sauvegarde" ]; then
    echo "Sauvegarde : $sauvegarde"
  else
    echo "Pas de sauvegarde dans $sauvegarde : exportez-la depuis l'app Trousseaux d'accès"
    echo "(Mes certificats › « $CERTIFICAT_MAC_NOM » › Exporter, format .p12), et gardez-la en lieu sûr."
  fi
  attendre
  exit 0
fi

travail="$(mktemp -d "${TMPDIR:-/tmp}/automat-pdf-certificat.XXXXXX")"
chmod 700 "$travail"
trap 'rm -rf "$travail"' EXIT

# Déjà dans le trousseau mais pas approuvé (fenêtre de macOS refusée la dernière fois) : on reprend
# ce certificat-là, sans en créer un autre.
deja="$({ security find-identity -p codesigning "$trousseau" 2>/dev/null || true; } \
  | awk -v nom="\"$CERTIFICAT_MAC_NOM\"" 'index($0, nom) && t == "" { t = $2 } END { if (t != "") print t }')"

if [ -n "$deja" ]; then
  etape "Certificat déjà installé, pas encore approuvé"
  security find-certificate -c "$CERTIFICAT_MAC_NOM" -Z -p "$trousseau" \
    | awk -v h="$deja" '/^SHA-1 hash:/ { garde = ($3 == h) } garde && /BEGIN CERTIFICATE/, garde && /END CERTIFICATE/' \
    > "$travail/certificat.pem"
  [ -s "$travail/certificat.pem" ] || echec "certificat illisible dans le trousseau."
else
  if [ "$restaurer" = 1 ]; then
    etape "Reprise de la sauvegarde"
    [ -f "$sauvegarde" ] || echec "sauvegarde introuvable : $sauvegarde."
    read -r -s -p "Mot de passe de la sauvegarde : " mot_de_passe; echo
    # Le mot de passe passe par l'environnement, jamais par la ligne de commande.
    MOT_DE_PASSE="$mot_de_passe" openssl pkcs12 -in "$sauvegarde" -passin env:MOT_DE_PASSE -nodes -nocerts \
      -out "$travail/cle.pem" 2>/dev/null || echec "mot de passe faux, ou sauvegarde abîmée."
    MOT_DE_PASSE="$mot_de_passe" openssl pkcs12 -in "$sauvegarde" -passin env:MOT_DE_PASSE -nokeys -clcerts \
      -out "$travail/certificat.pem" 2>/dev/null || echec "certificat illisible dans la sauvegarde."
    openssl x509 -in "$travail/certificat.pem" -noout -subject 2>/dev/null | grep -q "Automat-PDF" \
      || echec "cette sauvegarde n'est pas celle du certificat « $CERTIFICAT_MAC_NOM »."
  else
    [ -f "$sauvegarde" ] && echec "une sauvegarde existe déjà ($sauvegarde) : reprenez-la avec --restaurer, plutôt que de créer un nouveau certificat (il changerait l'identité de l'application)."
    etape "Création du certificat « $CERTIFICAT_MAC_NOM »"
    cat > "$travail/certificat.cnf" <<EOF
[req]
distinguished_name = dn
prompt = no
x509_extensions = ext
utf8 = yes
string_mask = utf8only
[dn]
CN = $CERTIFICAT_MAC_NOM
O = Eau sèche
C = CG
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
subjectKeyIdentifier = hash
EOF
    openssl req -x509 -newkey rsa:3072 -sha256 -days 7300 -nodes -config "$travail/certificat.cnf" \
      -keyout "$travail/cle.pem" -out "$travail/certificat.pem" 2>/dev/null || echec "création du certificat impossible (openssl)."

    etape "Sauvegarde"
    echo "Choisissez un mot de passe pour la sauvegarde (8 caractères au moins) ; gardez-le avec elle."
    while :; do
      read -r -s -p "Mot de passe : " mot_de_passe; echo
      read -r -s -p "Encore une fois : " confirmation; echo
      [ "$mot_de_passe" = "$confirmation" ] && [ "${#mot_de_passe}" -ge 8 ] && break
      echo "Différents, ou trop courts : recommencez."
    done
    mkdir -p "$(dirname "$sauvegarde")"
    MOT_DE_PASSE="$mot_de_passe" openssl pkcs12 -export -inkey "$travail/cle.pem" -in "$travail/certificat.pem" \
      -name "$CERTIFICAT_MAC_NOM" -passout env:MOT_DE_PASSE -out "$sauvegarde" 2>/dev/null || echec "sauvegarde impossible à écrire."
    chmod 600 "$sauvegarde"
    echo "Sauvegarde : $sauvegarde"
  fi

  etape "Installation dans le trousseau de session"
  # -T : codesign peut se servir de la clé (macOS le confirme à la première signature).
  security import "$travail/cle.pem" -k "$trousseau" -T /usr/bin/codesign >/dev/null || echec "clé non installée dans le trousseau."
  security import "$travail/certificat.pem" -k "$trousseau" >/dev/null || echec "certificat non installé dans le trousseau."
fi

etape "Approbation par macOS (votre mot de passe)"
echo "macOS va demander votre mot de passe pour approuver ce certificat pour signer du code, sur ce Mac"
echo "seulement. Il n'est approuvé pour rien d'autre (ni sites web, ni e-mails)."
# Sans -k : le certificat est déjà dans le trousseau, seule sa confiance (pour signer du code) change.
security add-trusted-cert -r trustRoot -p codeSign "$travail/certificat.pem" \
  || echec "certificat pas approuvé (fenêtre de macOS refusée ?) : relancez ce script."
identite="$(identite_mac)"
[ -n "$identite" ] || echec "macOS ne présente pas le certificat comme identité de signature : relancez ce script."

etape "Signature d'essai"
echo "La première fois, macOS demande si « codesign » peut utiliser la clé « $CERTIFICAT_MAC_NOM » :"
echo "répondez « Toujours autoriser » (avec votre mot de passe), sinon il le redemandera à chaque signature."
cp /usr/bin/true "$travail/essai"
codesign --force -s "$identite" "$travail/essai" 2>"$travail/codesign.log" \
  || { cat "$travail/codesign.log"; echec "signature d'essai refusée : relancez ce script et répondez « Toujours autoriser »."; }
exigence="$(codesign -d -r- "$travail/essai" 2>&1 | sed -n 's/^designated => //p')"
echo "$exigence" | grep -qi "$identite" \
  || echec "l'identité de la signature ne dépend pas de ce certificat ($exigence) : à examiner avant de publier."

etape "Terminé"
echo "Certificat : « $CERTIFICAT_MAC_NOM »"
echo "Empreinte SHA-1 : $identite"
echo "Empreinte SHA-256 : $(openssl x509 -in "$travail/certificat.pem" -noout -fingerprint -sha256 | cut -d= -f2)"
echo "Identité d'une application signée : $exigence"
[ -f "$sauvegarde" ] && echo "Sauvegarde : $sauvegarde (gardez-la en lieu sûr avec son mot de passe)."
echo "publier.command signera désormais l'application Mac avec ce certificat."
attendre
