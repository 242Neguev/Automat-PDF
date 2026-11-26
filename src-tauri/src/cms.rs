//! Signatures numériques (CMS, RFC 5652), pour vérifier celles des PDF reçus : structures ASN.1,
//! certificats X.509, empreintes (SHA-1, SHA-2) et signatures (RSA, RSA-PSS, ECDSA P-256 et
//! P-384), en Rust (RustCrypto).
//!
//! La lecture est tolérante : DER, et BER à longueurs indéfinies, que produisent certains
//! logiciels de signature. Elle ne fait confiance à rien : longueurs contrôlées, profondeur
//! bornée, rien n'est lu au-delà des octets reçus. La confiance dans le certificat (son autorité)
//! n'est pas jugée ici, mais par le système (`confiance.rs`).

use p256::ecdsa::signature::hazmat::PrehashVerifier;
use rsa::{BoxedUint, Pkcs1v15Sign, PublicKey, pss::Pss};
use sha1::Sha1;
use sha2::digest::{Digest, FixedOutputReset};
use sha2::{Sha224, Sha256, Sha384, Sha512};

/// Profondeur d'imbrication lue au plus (les structures réelles en ont moins de 15).
const PROFONDEUR_MAX: usize = 48;

// Identifiants d'objets (OID) utiles.
const SIGNED_DATA: &str = "1.2.840.113549.1.7.2";
pub(crate) const TST_INFO: &str = "1.2.840.113549.1.9.16.1.4";
const MESSAGE_DIGEST: &str = "1.2.840.113549.1.9.4";
const SIGNING_TIME: &str = "1.2.840.113549.1.9.5";
const JETON_HORODATAGE: &str = "1.2.840.113549.1.9.16.2.14";
const RSA: &str = "1.2.840.113549.1.1.1";
const RSA_PSS: &str = "1.2.840.113549.1.1.10";
const EC: &str = "1.2.840.10045.2.1";
const P256: &str = "1.2.840.10045.3.1.7";
const P384: &str = "1.3.132.0.34";
const SKI: &str = "2.5.29.14";
const SAN: &str = "2.5.29.17";
pub(crate) const CN: &str = "2.5.4.3";
const NOM_DE_FAMILLE: &str = "2.5.4.4";
const PRENOM: &str = "2.5.4.42";
pub(crate) const ORGANISATION: &str = "2.5.4.10";
const SERVICE: &str = "2.5.4.11";
pub(crate) const COURRIEL: &str = "1.2.840.113549.1.9.1";

// ------------------------------------------------------------------------------- ASN.1

/// Un élément ASN.1 : sa classe (0 universelle, 2 contextuelle), son numéro, son contenu, et
/// l'élément entier tel qu'il est écrit (`brut`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Tlv<'a> {
    pub classe: u8,
    pub construit: bool,
    pub numero: u32,
    pub contenu: &'a [u8],
    pub brut: &'a [u8],
}

impl<'a> Tlv<'a> {
    /// Un élément universel de ce numéro (16 : SEQUENCE, 17 : SET, 2 : INTEGER, 4 : OCTET STRING…).
    pub fn est(&self, numero: u32) -> bool {
        self.classe == 0 && self.numero == numero
    }

    /// Un élément contextuel `[numero]`.
    pub fn contexte(&self, numero: u32) -> bool {
        self.classe == 2 && self.numero == numero
    }

    /// Les éléments qu'il contient, dans l'ordre (s'arrête au premier illisible).
    pub fn enfants(&self) -> Enfants<'a> {
        Enfants {
            reste: if self.construit { self.contenu } else { &[] },
        }
    }
}

pub(crate) struct Enfants<'a> {
    reste: &'a [u8],
}

impl<'a> Iterator for Enfants<'a> {
    type Item = Tlv<'a>;

    fn next(&mut self) -> Option<Tlv<'a>> {
        let (tlv, taille) = lire(self.reste, 0)?;
        self.reste = &self.reste[taille..];
        Some(tlv)
    }
}

/// Lit le premier élément de `octets` ; rend aussi sa taille écrite.
fn lire(octets: &[u8], profondeur: usize) -> Option<(Tlv<'_>, usize)> {
    if profondeur > PROFONDEUR_MAX {
        return None;
    }
    let premier = *octets.first()?;
    let classe = premier >> 6;
    let construit = premier & 0x20 != 0;
    let mut i = 1;
    let mut numero = u32::from(premier & 0x1f);
    if numero == 0x1f {
        numero = 0;
        loop {
            let octet = *octets.get(i)?;
            i += 1;
            numero = numero.checked_mul(128)? | u32::from(octet & 0x7f);
            if octet & 0x80 == 0 {
                break;
            }
            if i > 5 {
                return None;
            }
        }
    }
    let longueur = *octets.get(i)?;
    i += 1;
    if longueur == 0x80 {
        // Longueur indéfinie (BER) : les éléments contenus, jusqu'à la fin de contenu (00 00).
        if !construit {
            return None;
        }
        let debut = i;
        let mut position = i;
        loop {
            if octets.get(position..position + 2)? == [0, 0] {
                let total = position + 2;
                let tlv = Tlv {
                    classe,
                    construit,
                    numero,
                    contenu: &octets[debut..position],
                    brut: &octets[..total],
                };
                return Some((tlv, total));
            }
            let (_, taille) = lire(&octets[position..], profondeur + 1)?;
            position += taille;
        }
    }
    let taille = if longueur < 0x80 {
        usize::from(longueur)
    } else {
        let n = usize::from(longueur & 0x7f);
        if n > 4 {
            return None;
        }
        let mut valeur = 0usize;
        for _ in 0..n {
            valeur = (valeur << 8) | usize::from(*octets.get(i)?);
            i += 1;
        }
        valeur
    };
    let fin = i.checked_add(taille)?;
    let tlv = Tlv {
        classe,
        construit,
        numero,
        contenu: octets.get(i..fin)?,
        brut: &octets[..fin],
    };
    Some((tlv, fin))
}

/// Le premier élément de `octets` (la suite, des zéros de remplissage par exemple, est ignorée).
pub(crate) fn premier(octets: &[u8]) -> Option<Tlv<'_>> {
    lire(octets, 0).map(|(tlv, _)| tlv)
}

/// Un identifiant d'objet, en notation pointée (« 1.2.840.113549.1.7.2 »).
pub(crate) fn oid(tlv: &Tlv) -> Option<String> {
    if !tlv.est(6) || tlv.contenu.is_empty() || tlv.contenu.len() > 64 {
        return None;
    }
    let mut parties: Vec<u64> = Vec::new();
    let mut valeur: u64 = 0;
    for &octet in tlv.contenu {
        valeur = valeur.checked_mul(128)? | u64::from(octet & 0x7f);
        if octet & 0x80 == 0 {
            if parties.is_empty() {
                let racine = (valeur / 40).min(2);
                parties.push(racine);
                parties.push(valeur - racine * 40);
            } else {
                parties.push(valeur);
            }
            valeur = 0;
        }
    }
    Some(
        parties
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join("."),
    )
}

/// Le contenu d'une chaîne d'octets, aussi quand le BER l'écrit en morceaux.
fn octets_de(tlv: &Tlv) -> Vec<u8> {
    fn ajouter(tlv: &Tlv, sortie: &mut Vec<u8>, profondeur: usize) {
        if !tlv.construit {
            sortie.extend_from_slice(tlv.contenu);
        } else if profondeur < 8 {
            for morceau in tlv.enfants() {
                ajouter(&morceau, sortie, profondeur + 1);
            }
        }
    }
    let mut sortie = Vec::new();
    ajouter(tlv, &mut sortie, 0);
    sortie
}

/// Un entier positif, sans ses zéros de tête.
fn entier_positif<'a>(tlv: &Tlv<'a>) -> Option<&'a [u8]> {
    if !tlv.est(2) || tlv.contenu.is_empty() {
        return None;
    }
    let mut chiffres = tlv.contenu;
    while chiffres.len() > 1 && chiffres[0] == 0 {
        chiffres = &chiffres[1..];
    }
    Some(chiffres)
}

/// Une chaîne de texte ASN.1 (UTF-8, imprimable, IA5, Latin-1, UTF-16, UTF-32).
fn texte(tlv: &Tlv) -> Option<String> {
    if tlv.classe != 0 {
        return None;
    }
    let texte = match tlv.numero {
        12 | 19 | 22 | 26 => String::from_utf8_lossy(tlv.contenu).into_owned(),
        20 => tlv.contenu.iter().map(|&o| char::from(o)).collect(),
        30 => char::decode_utf16(
            tlv.contenu
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&paire| u16::from_be_bytes(paire)),
        )
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect(),
        28 => tlv
            .contenu
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&quatre| {
                char::from_u32(u32::from_be_bytes(quatre)).unwrap_or(char::REPLACEMENT_CHARACTER)
            })
            .collect(),
        _ => return None,
    };
    Some(
        texte
            .chars()
            .filter(|c| !c.is_control())
            .take(200)
            .collect::<String>()
            .trim()
            .to_string(),
    )
}

// ------------------------------------------------------------------------------- Dates

/// Jours depuis le 1er janvier 1970 (calendrier grégorien).
fn jours_depuis_1970(annee: i64, mois: i64, jour: i64) -> i64 {
    let a = if mois <= 2 { annee - 1 } else { annee };
    let ere = a.div_euclid(400);
    let annee_de_l_ere = a - ere * 400;
    let mois_depuis_mars = (mois + 9) % 12;
    let jour_de_l_annee = (153 * mois_depuis_mars + 2) / 5 + jour - 1;
    let jour_de_l_ere =
        annee_de_l_ere * 365 + annee_de_l_ere / 4 - annee_de_l_ere / 100 + jour_de_l_annee;
    ere * 146_097 + jour_de_l_ere - 719_468
}

/// Une date (heure locale décalée de `decalage` minutes sur UTC), en secondes depuis 1970 (UTC).
pub(crate) fn moment(champs: [i64; 6], decalage: i64) -> Option<i64> {
    let [annee, mois, jour, heure, minute, seconde] = champs;
    let valide = (1..=9999).contains(&annee)
        && (1..=12).contains(&mois)
        && (1..=31).contains(&jour)
        && (0..=23).contains(&heure)
        && (0..=59).contains(&minute)
        && (0..=60).contains(&seconde);
    valide.then(|| {
        jours_depuis_1970(annee, mois, jour) * 86_400 + heure * 3600 + minute * 60 + seconde
            - decalage * 60
    })
}

/// Le décalage horaire d'une date : « Z », « +01'00' », « +0100 », « -05 » (en minutes).
pub(crate) fn decalage(reste: &str) -> i64 {
    let mut caracteres = reste.trim().chars();
    let signe = match caracteres.next() {
        Some('+') => 1,
        Some('-') => -1,
        _ => return 0,
    };
    let chiffres: String = caracteres.filter(char::is_ascii_digit).take(4).collect();
    let heures: i64 = chiffres.get(0..2).and_then(|h| h.parse().ok()).unwrap_or(0);
    let minutes: i64 = chiffres.get(2..4).and_then(|m| m.parse().ok()).unwrap_or(0);
    signe * (heures.min(23) * 60 + minutes.min(59))
}

/// Une date ASN.1 (UTCTime, GeneralizedTime), en secondes depuis 1970 (UTC).
pub(crate) fn temps(tlv: &Tlv) -> Option<i64> {
    let texte = std::str::from_utf8(tlv.contenu).ok()?;
    let chiffres: String = texte.chars().take_while(char::is_ascii_digit).collect();
    let reste = &texte[chiffres.len()..];
    let reste = reste.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let (annee, suite) = match (tlv.classe, tlv.numero) {
        (0, 23) => {
            let aa: i64 = chiffres.get(0..2)?.parse().ok()?;
            (
                if aa < 50 { 2000 + aa } else { 1900 + aa },
                chiffres.get(2..)?,
            )
        }
        (0, 24) => (chiffres.get(0..4)?.parse().ok()?, chiffres.get(4..)?),
        _ => return None,
    };
    let partie = |debut: usize| -> i64 {
        suite
            .get(debut..debut + 2)
            .and_then(|p| p.parse().ok())
            .unwrap_or(0)
    };
    if suite.len() < 6 {
        return None;
    }
    moment(
        [annee, partie(0), partie(2), partie(4), partie(6), partie(8)],
        decalage(reste),
    )
}

/// Une date en ISO 8601 (UTC), pour l'interface : « 2026-11-26T08:12:00Z ».
pub(crate) fn iso(secondes: i64) -> String {
    let (jours, reste) = (secondes.div_euclid(86_400), secondes.rem_euclid(86_400));
    let z = jours + 719_468;
    let ere = z.div_euclid(146_097);
    let jour_de_l_ere = z - ere * 146_097;
    let annee_de_l_ere = (jour_de_l_ere - jour_de_l_ere / 1460 + jour_de_l_ere / 36_524
        - jour_de_l_ere / 146_096)
        / 365;
    let jour_de_l_annee =
        jour_de_l_ere - (365 * annee_de_l_ere + annee_de_l_ere / 4 - annee_de_l_ere / 100);
    let mois_depuis_mars = (5 * jour_de_l_annee + 2) / 153;
    let jour = jour_de_l_annee - (153 * mois_depuis_mars + 2) / 5 + 1;
    let mois = if mois_depuis_mars < 10 {
        mois_depuis_mars + 3
    } else {
        mois_depuis_mars - 9
    };
    let annee = annee_de_l_ere + ere * 400 + i64::from(mois <= 2);
    format!(
        "{annee:04}-{mois:02}-{jour:02}T{:02}:{:02}:{:02}Z",
        reste / 3600,
        reste % 3600 / 60,
        reste % 60
    )
}

// ------------------------------------------------------------------------------- Certificats

/// Un nom X.509 (sujet ou émetteur) : ses attributs (OID, valeur), dans l'ordre.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Nom(pub Vec<(String, String)>);

impl Nom {
    pub fn attribut(&self, oid: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(o, valeur)| o == oid && !valeur.is_empty())
            .map(|(_, valeur)| valeur.as_str())
    }

    /// Le nom à montrer : nom commun, sinon prénom et nom, organisation, service ou e-mail.
    pub fn lisible(&self) -> Option<String> {
        if let Some(nom) = self.attribut(CN) {
            return Some(nom.to_string());
        }
        if let (Some(prenom), Some(nom)) = (self.attribut(PRENOM), self.attribut(NOM_DE_FAMILLE)) {
            return Some(format!("{prenom} {nom}"));
        }
        self.attribut(ORGANISATION)
            .or_else(|| self.attribut(SERVICE))
            .or_else(|| self.attribut(COURRIEL))
            .map(String::from)
    }
}

fn nom(tlv: &Tlv) -> Nom {
    let mut attributs = Vec::new();
    for ensemble in tlv.enfants().take(64) {
        for attribut in ensemble.enfants().take(16) {
            let mut parties = attribut.enfants();
            if let (Some(type_), Some(valeur)) = (
                parties.next().and_then(|t| oid(&t)),
                parties.next().and_then(|v| texte(&v)),
            ) {
                attributs.push((type_, valeur));
            }
        }
    }
    Nom(attributs)
}

/// La clé publique d'un certificat : son algorithme, ses paramètres (la courbe d'une clé ECDSA)
/// et la clé elle-même.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cle<'a> {
    pub algorithme: &'a str,
    pub courbe: Option<&'a str>,
    pub octets: &'a [u8],
}

/// Un certificat X.509, lu pour vérifier une signature et la présenter.
#[derive(Debug, Clone)]
pub(crate) struct Certificat<'a> {
    pub der: &'a [u8],
    pub serie: &'a [u8],
    pub emetteur_der: &'a [u8],
    pub emetteur: Nom,
    pub sujet: Nom,
    pub debut: Option<i64>,
    pub fin: Option<i64>,
    algorithme_cle: String,
    courbe: Option<String>,
    cle: &'a [u8],
    pub identifiant_cle: Option<&'a [u8]>,
    /// L'adresse e-mail du nom alternatif du sujet (rfc822Name).
    pub courriel: Option<String>,
}

impl<'a> Certificat<'a> {
    pub fn cle(&self) -> Cle<'_> {
        Cle {
            algorithme: &self.algorithme_cle,
            courbe: self.courbe.as_deref(),
            octets: self.cle,
        }
    }

    /// L'e-mail du sujet : attribut du nom, sinon nom alternatif.
    pub fn courriel(&self) -> Option<String> {
        self.sujet
            .attribut(COURRIEL)
            .map(String::from)
            .or_else(|| self.courriel.clone())
    }
}

/// Lit un certificat X.509 (seulement ce qui sert ici).
pub(crate) fn certificat(tlv: Tlv<'_>) -> Option<Certificat<'_>> {
    let tbs = tlv.enfants().next()?;
    let mut champs = tbs.enfants().peekable();
    if champs.peek()?.contexte(0) {
        champs.next();
    }
    let serie = entier_positif(&champs.next()?)?;
    let _algorithme = champs.next()?;
    let emetteur = champs.next()?;
    let validite = champs.next()?;
    let sujet = champs.next()?;
    let spki = champs.next()?;
    let mut dates = validite.enfants();
    let (debut, fin) = (dates.next(), dates.next());
    let mut parties_cle = spki.enfants();
    let algorithme = parties_cle.next()?;
    let mut parties_algorithme = algorithme.enfants();
    let algorithme_cle = oid(&parties_algorithme.next()?)?;
    let courbe = parties_algorithme.next().and_then(|p| oid(&p));
    let bits = parties_cle.next()?;
    if !bits.est(3) || bits.contenu.is_empty() {
        return None;
    }
    let mut identifiant_cle = None;
    let mut courriel = None;
    for reste in champs {
        if !reste.contexte(3) {
            continue;
        }
        for extension in reste.enfants().next()?.enfants().take(64) {
            let mut parties = extension.enfants();
            let Some(type_) = parties.next().and_then(|t| oid(&t)) else {
                continue;
            };
            let Some(valeur) = parties.find(|p| p.est(4)) else {
                continue;
            };
            match type_.as_str() {
                SKI => {
                    identifiant_cle = premier(valeur.contenu)
                        .filter(|t| t.est(4))
                        .map(|t| t.contenu);
                }
                SAN => {
                    courriel = premier(valeur.contenu).and_then(|noms| {
                        noms.enfants()
                            .find(|n| n.contexte(1) && !n.construit)
                            .map(|n| String::from_utf8_lossy(n.contenu).into_owned())
                    });
                }
                _ => {}
            }
        }
    }
    Some(Certificat {
        der: tlv.brut,
        serie,
        emetteur_der: emetteur.brut,
        emetteur: nom(&emetteur),
        sujet: nom(&sujet),
        debut: debut.and_then(|d| temps(&d)),
        fin: fin.and_then(|f| temps(&f)),
        algorithme_cle,
        courbe,
        cle: &bits.contenu[1..],
        identifiant_cle,
        courriel,
    })
}

// ------------------------------------------------------------------------------- CMS

/// Qui a signé : l'émetteur et le numéro de son certificat, ou l'identifiant de sa clé.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Sid<'a> {
    EmetteurSerie { emetteur: &'a [u8], serie: &'a [u8] },
    Cle(&'a [u8]),
}

/// Un signataire (SignerInfo).
#[derive(Debug, Clone)]
pub(crate) struct Signataire<'a> {
    pub sid: Sid<'a>,
    pub empreinte: String,
    /// Les attributs signés ([0] IMPLICIT), tels qu'écrits.
    pub attributs: Option<Tlv<'a>>,
    pub algorithme: String,
    pub parametres: Option<Tlv<'a>>,
    pub signature: &'a [u8],
    pub non_signes: Option<Tlv<'a>>,
}

/// Une signature CMS (SignedData) : ce qui est signé, les certificats joints, les signataires.
#[derive(Debug, Clone)]
pub(crate) struct Cms<'a> {
    pub type_contenu: String,
    /// Le contenu encapsulé (absent d'une signature détachée, comme celles des PDF).
    pub contenu: Option<Vec<u8>>,
    pub certificats: Vec<Certificat<'a>>,
    pub signataires: Vec<Signataire<'a>>,
}

/// Lit une signature CMS (ContentInfo contenant un SignedData).
pub(crate) fn cms(octets: &[u8]) -> Option<Cms<'_>> {
    let info = premier(octets)?;
    let mut parties = info.enfants();
    if oid(&parties.next()?)? != SIGNED_DATA {
        return None;
    }
    let donnees = parties.next().filter(|t| t.contexte(0))?.enfants().next()?;
    let mut champs = donnees.enfants();
    let _version = champs.next()?;
    let _empreintes = champs.next()?;
    let encapsule = champs.next()?;
    let mut parties_encapsulees = encapsule.enfants();
    let type_contenu = oid(&parties_encapsulees.next()?)?;
    let contenu = parties_encapsulees
        .next()
        .filter(|t| t.contexte(0))
        .and_then(|t| t.enfants().next())
        .map(|o| octets_de(&o));
    let mut certificats = Vec::new();
    let mut signataires = Vec::new();
    for champ in champs {
        if champ.contexte(0) {
            certificats.extend(
                champ
                    .enfants()
                    .take(64)
                    .filter(|c| c.est(16))
                    .filter_map(certificat),
            );
        } else if champ.est(17) {
            signataires.extend(champ.enfants().take(16).filter_map(signataire));
        }
    }
    Some(Cms {
        type_contenu,
        contenu,
        certificats,
        signataires,
    })
}

fn signataire(tlv: Tlv<'_>) -> Option<Signataire<'_>> {
    let mut parties = tlv.enfants();
    let _version = parties.next()?;
    let identite = parties.next()?;
    let sid = if identite.contexte(0) {
        Sid::Cle(identite.contenu)
    } else {
        let mut emetteur_serie = identite.enfants();
        let emetteur = emetteur_serie.next()?;
        let serie = entier_positif(&emetteur_serie.next()?)?;
        Sid::EmetteurSerie {
            emetteur: emetteur.brut,
            serie,
        }
    };
    let empreinte = oid(&parties.next()?.enfants().next()?)?;
    let mut suivant = parties.next()?;
    let attributs = if suivant.contexte(0) {
        let attributs = suivant;
        suivant = parties.next()?;
        Some(attributs)
    } else {
        None
    };
    let mut algorithme = suivant.enfants();
    let nom_algorithme = oid(&algorithme.next()?)?;
    let parametres = algorithme.next().filter(|p| !p.est(5));
    let signature = parties.next().filter(|s| s.est(4))?;
    let non_signes = parties.next().filter(|t| t.contexte(1));
    Some(Signataire {
        sid,
        empreinte,
        attributs,
        algorithme: nom_algorithme,
        parametres,
        signature: signature.contenu,
        non_signes,
    })
}

/// La première valeur de l'attribut `type_` d'une liste d'attributs CMS.
pub(crate) fn attribut<'a>(liste: Option<Tlv<'a>>, type_: &str) -> Option<Tlv<'a>> {
    liste?.enfants().take(64).find_map(|attribut| {
        let mut parties = attribut.enfants();
        (oid(&parties.next()?)? == type_).then(|| parties.next()?.enfants().next())?
    })
}

// ------------------------------------------------------------------------------- Vérification

/// Les empreintes reconnues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Empreinte {
    Sha1,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
}

impl Empreinte {
    pub fn de(oid: &str) -> Option<Self> {
        Some(match oid {
            "1.3.14.3.2.26" => Self::Sha1,
            "2.16.840.1.101.3.4.2.4" => Self::Sha224,
            "2.16.840.1.101.3.4.2.1" => Self::Sha256,
            "2.16.840.1.101.3.4.2.2" => Self::Sha384,
            "2.16.840.1.101.3.4.2.3" => Self::Sha512,
            _ => return None,
        })
    }

    /// L'empreinte de `morceaux`, mis bout à bout.
    pub fn calculer(self, morceaux: &[&[u8]]) -> Vec<u8> {
        fn avec<D: Digest>(morceaux: &[&[u8]]) -> Vec<u8> {
            let mut calcul = D::new();
            for morceau in morceaux {
                calcul.update(morceau);
            }
            calcul.finalize().to_vec()
        }
        match self {
            Self::Sha1 => avec::<Sha1>(morceaux),
            Self::Sha224 => avec::<Sha224>(morceaux),
            Self::Sha256 => avec::<Sha256>(morceaux),
            Self::Sha384 => avec::<Sha384>(morceaux),
            Self::Sha512 => avec::<Sha512>(morceaux),
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            Self::Sha1 => "SHA-1",
            Self::Sha224 => "SHA-224",
            Self::Sha256 => "SHA-256",
            Self::Sha384 => "SHA-384",
            Self::Sha512 => "SHA-512",
        }
    }
}

/// L'empreinte qu'impose l'algorithme de signature (sha256WithRSAEncryption, ecdsa-with-SHA384…).
fn empreinte_imposee(algorithme: &str, parametres: Option<&Tlv>) -> Option<Empreinte> {
    Some(match algorithme {
        "1.2.840.113549.1.1.5" | "1.2.840.10045.4.1" | "1.3.14.3.2.29" => Empreinte::Sha1,
        "1.2.840.113549.1.1.14" | "1.2.840.10045.4.3.1" => Empreinte::Sha224,
        "1.2.840.113549.1.1.11" | "1.2.840.10045.4.3.2" => Empreinte::Sha256,
        "1.2.840.113549.1.1.12" | "1.2.840.10045.4.3.3" => Empreinte::Sha384,
        "1.2.840.113549.1.1.13" | "1.2.840.10045.4.3.4" => Empreinte::Sha512,
        // RSASSA-PSS : l'empreinte de ses paramètres ([0], SHA-1 par défaut).
        RSA_PSS => parametres
            .and_then(|p| p.enfants().find(|c| c.contexte(0)))
            .and_then(|c| c.enfants().next())
            .and_then(|algorithme| algorithme.enfants().next())
            .and_then(|o| oid(&o))
            .map_or(Some(Empreinte::Sha1), |o| Empreinte::de(&o))?,
        _ => return None,
    })
}

/// Le verdict d'une signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// L'empreinte et la signature correspondent : rien n'a changé depuis la signature.
    Valide,
    /// L'empreinte ne correspond plus : les données signées ont changé.
    Alteree,
    /// La signature ne correspond pas à la clé du certificat (falsifiée ou abîmée).
    Fausse,
    /// Algorithme que l'on ne sait pas vérifier (son nom).
    NonPrisEnCharge(String),
    /// Structure illisible ou incomplète.
    Illisible,
}

/// Une clé publique RSA (RSAPublicKey : module et exposant).
fn cle_rsa(octets: &[u8]) -> Option<PublicKey> {
    let sequence = premier(octets)?;
    let mut parties = sequence.enfants();
    let module = entier_positif(&parties.next()?)?;
    let exposant = entier_positif(&parties.next()?)?;
    PublicKey::new(
        BoxedUint::from_be_slice_vartime(module),
        BoxedUint::from_be_slice_vartime(exposant),
    )
    .ok()
}

/// Le nom d'une clé, pour l'utilisateur : « RSA 2048 bits », « ECDSA P-256 ».
pub(crate) fn nom_de_cle(cle: &Cle) -> String {
    match (cle.algorithme, cle.courbe) {
        (RSA | RSA_PSS, _) => premier(cle.octets)
            .and_then(|s| s.enfants().next())
            .and_then(|n| entier_positif(&n))
            .map_or("RSA".into(), |n| {
                let bits = n.len() * 8 - n.first().map_or(0, |o| o.leading_zeros() as usize);
                format!("RSA {bits} bits")
            }),
        (EC, Some(P256)) => "ECDSA P-256".into(),
        (EC, Some(P384)) => "ECDSA P-384".into(),
        (EC, Some(courbe)) => format!("ECDSA ({courbe})"),
        (autre, _) => autre.to_string(),
    }
}

enum Essai {
    Bonne,
    Fausse,
    Impossible(String),
}

/// Vérifie la signature `signature` de l'empreinte `hache` avec la clé `cle`.
fn essayer(
    cle: &Cle,
    algorithme: &str,
    empreinte: Empreinte,
    hache: &[u8],
    signature: &[u8],
) -> Essai {
    let resultat = |bonne: bool| if bonne { Essai::Bonne } else { Essai::Fausse };
    match (cle.algorithme, cle.courbe) {
        (RSA | RSA_PSS, _) => {
            let Some(cle_publique) = cle_rsa(cle.octets) else {
                return Essai::Impossible(nom_de_cle(cle));
            };
            fn pss<D: Digest + FixedOutputReset>(
                cle: &PublicKey,
                hache: &[u8],
                signature: &[u8],
            ) -> bool {
                let schema = Pss::<D> {
                    blinded: false,
                    digest: D::new(),
                    salt_len: None,
                };
                cle.verify(schema, hache, signature).is_ok()
            }
            let bonne = if algorithme == RSA_PSS {
                match empreinte {
                    Empreinte::Sha1 => pss::<Sha1>(&cle_publique, hache, signature),
                    Empreinte::Sha224 => pss::<Sha224>(&cle_publique, hache, signature),
                    Empreinte::Sha256 => pss::<Sha256>(&cle_publique, hache, signature),
                    Empreinte::Sha384 => pss::<Sha384>(&cle_publique, hache, signature),
                    Empreinte::Sha512 => pss::<Sha512>(&cle_publique, hache, signature),
                }
            } else {
                let schema = match empreinte {
                    Empreinte::Sha1 => Pkcs1v15Sign::new::<Sha1>(),
                    Empreinte::Sha224 => Pkcs1v15Sign::new::<Sha224>(),
                    Empreinte::Sha256 => Pkcs1v15Sign::new::<Sha256>(),
                    Empreinte::Sha384 => Pkcs1v15Sign::new::<Sha384>(),
                    Empreinte::Sha512 => Pkcs1v15Sign::new::<Sha512>(),
                };
                cle_publique.verify(schema, hache, signature).is_ok()
            };
            resultat(bonne)
        }
        (EC, Some(P256)) => {
            let Ok(cle_publique) = p256::ecdsa::VerifyingKey::from_sec1_bytes(cle.octets) else {
                return Essai::Impossible(nom_de_cle(cle));
            };
            let signature = p256::ecdsa::Signature::from_der(signature)
                .or_else(|_| p256::ecdsa::Signature::from_slice(signature));
            resultat(signature.is_ok_and(|s| cle_publique.verify_prehash(hache, &s).is_ok()))
        }
        (EC, Some(P384)) => {
            let Ok(cle_publique) = p384::ecdsa::VerifyingKey::from_sec1_bytes(cle.octets) else {
                return Essai::Impossible(nom_de_cle(cle));
            };
            let signature = p384::ecdsa::Signature::from_der(signature)
                .or_else(|_| p384::ecdsa::Signature::from_slice(signature));
            resultat(signature.is_ok_and(|s| cle_publique.verify_prehash(hache, &s).is_ok()))
        }
        _ => Essai::Impossible(nom_de_cle(cle)),
    }
}

/// Ce que dit la vérification d'une signature CMS.
#[derive(Debug, Clone)]
pub(crate) struct Verification<'a> {
    pub verdict: Verdict,
    /// Le certificat du signataire (rang dans `Cms::certificats`), celui dont la clé a vérifié la
    /// signature ; à défaut, celui que désigne le signataire.
    pub certificat: Option<usize>,
    pub empreinte: Option<Empreinte>,
    /// La date de signature déclarée par le signataire (attribut signé signingTime).
    pub date: Option<i64>,
    /// La signature elle-même (son horodatage en porte l'empreinte).
    pub valeur: &'a [u8],
    /// Le jeton d'horodatage de la signature (attribut non signé), tel qu'écrit.
    pub jeton: Option<&'a [u8]>,
}

/// Le certificat que désigne le signataire.
fn designe(cms: &Cms, sid: &Sid) -> Option<usize> {
    cms.certificats.iter().position(|c| match sid {
        Sid::EmetteurSerie { emetteur, serie } => c.emetteur_der == *emetteur && c.serie == *serie,
        Sid::Cle(identifiant) => c.identifiant_cle == Some(*identifiant),
    })
}

/// Vérifie le premier signataire de `cms` : `detache`, les données signées (celles du PDF pour une
/// signature détachée), sinon le contenu encapsulé (un jeton d'horodatage).
pub(crate) fn verifier<'a>(cms: &Cms<'a>, detache: Option<&[&[u8]]>) -> Verification<'a> {
    let mut verification = Verification {
        verdict: Verdict::Illisible,
        certificat: None,
        empreinte: None,
        date: None,
        valeur: &[],
        jeton: None,
    };
    let Some(signataire) = cms.signataires.first() else {
        return verification;
    };
    verification.valeur = signataire.signature;
    verification.jeton = attribut(signataire.non_signes, JETON_HORODATAGE).map(|t| t.brut);
    verification.date = attribut(signataire.attributs, SIGNING_TIME).and_then(|t| temps(&t));
    verification.certificat = designe(cms, &signataire.sid);
    let Some(empreinte) = Empreinte::de(&signataire.empreinte) else {
        verification.verdict = Verdict::NonPrisEnCharge(signataire.empreinte.clone());
        return verification;
    };
    verification.empreinte = Some(empreinte);
    let contenu = match (detache, &cms.contenu) {
        (Some(morceaux), _) => empreinte.calculer(morceaux),
        (None, Some(contenu)) => empreinte.calculer(&[contenu]),
        (None, None) => return verification,
    };

    // Ce qui est signé : les attributs signés (l'empreinte des données y est), sinon l'empreinte
    // des données elle-même.
    let empreinte_signee =
        empreinte_imposee(&signataire.algorithme, signataire.parametres.as_ref())
            .unwrap_or(empreinte);
    let hache = match signataire.attributs {
        Some(attributs) => {
            let Some(annoncee) = attribut(Some(attributs), MESSAGE_DIGEST).filter(|t| t.est(4))
            else {
                return verification;
            };
            if octets_de(&annoncee) != contenu {
                verification.verdict = Verdict::Alteree;
                return verification;
            }
            // Signés sous la forme d'un SET OF (étiquette 0x31), et non du [0] IMPLICIT écrit.
            let mut signes = attributs.brut.to_vec();
            signes[0] = 0x31;
            empreinte_signee.calculer(&[&signes])
        }
        None if empreinte_signee == empreinte => contenu,
        None => return verification,
    };

    // Le certificat désigné d'abord, puis les autres : celui dont la clé vérifie la signature.
    let designe = verification.certificat;
    let mut ordre: Vec<usize> = designe.into_iter().collect();
    ordre.extend((0..cms.certificats.len()).filter(|&i| Some(i) != designe));
    let (mut essai_du_designe, mut une_cle_essayee, mut impossible) = (None, false, None);
    for rang in ordre {
        let essai = essayer(
            &cms.certificats[rang].cle(),
            &signataire.algorithme,
            empreinte_signee,
            &hache,
            signataire.signature,
        );
        match &essai {
            Essai::Bonne => {
                verification.verdict = Verdict::Valide;
                verification.certificat = Some(rang);
                return verification;
            }
            Essai::Fausse => une_cle_essayee = true,
            Essai::Impossible(nom) => {
                impossible.get_or_insert(nom.clone());
            }
        }
        if Some(rang) == designe {
            essai_du_designe = Some(essai);
        }
    }
    verification.verdict = match (essai_du_designe, impossible) {
        (Some(Essai::Impossible(nom)), _) => Verdict::NonPrisEnCharge(nom),
        (Some(_), _) => Verdict::Fausse,
        (None, _) if une_cle_essayee => Verdict::Fausse,
        (None, Some(nom)) => Verdict::NonPrisEnCharge(nom),
        (None, None) => Verdict::Illisible,
    };
    verification
}

/// Un horodatage (RFC 3161) : un jeton signé par une autorité d'horodatage, qui atteste que des
/// données (une signature, un document) existaient à sa date.
#[derive(Debug, Clone)]
pub(crate) struct Horodatage {
    pub verdict: Verdict,
    pub date: Option<i64>,
    /// L'autorité d'horodatage, telle que son certificat la nomme.
    pub autorite: Option<String>,
    /// Les certificats du jeton (celui de l'autorité d'abord), pour juger de la confiance.
    pub certificats: Vec<Vec<u8>>,
}

/// Vérifie le jeton d'horodatage `jeton` des données `donnees`.
pub(crate) fn horodatage(jeton: &[u8], donnees: &[&[u8]]) -> Horodatage {
    let mut lu = Horodatage {
        verdict: Verdict::Illisible,
        date: None,
        autorite: None,
        certificats: Vec::new(),
    };
    let Some(cms) = cms(jeton).filter(|c| c.type_contenu == TST_INFO) else {
        return lu;
    };
    let verification = verifier(&cms, None);
    if let Some(rang) = verification.certificat {
        lu.autorite = cms.certificats[rang].sujet.lisible();
        lu.certificats.push(cms.certificats[rang].der.to_vec());
    }
    lu.certificats.extend(
        cms.certificats
            .iter()
            .enumerate()
            .filter(|(rang, _)| Some(*rang) != verification.certificat)
            .map(|(_, c)| c.der.to_vec()),
    );
    // TSTInfo : version, politique, empreinte des données (messageImprint), numéro, date.
    let Some(info) = cms.contenu.as_deref().and_then(premier) else {
        return lu;
    };
    let mut champs = info.enfants();
    let (_version, _politique) = (champs.next(), champs.next());
    let (Some(empreinte_annoncee), _numero, Some(date)) =
        (champs.next(), champs.next(), champs.next())
    else {
        return lu;
    };
    lu.date = temps(&date);
    let mut parties = empreinte_annoncee.enfants();
    let algorithme = parties
        .next()
        .and_then(|a| a.enfants().next())
        .and_then(|o| oid(&o));
    let annoncee = parties.next().filter(|t| t.est(4)).map(|t| t.contenu);
    lu.verdict = match (algorithme.as_deref().and_then(Empreinte::de), annoncee) {
        (Some(empreinte), Some(annoncee)) if empreinte.calculer(donnees) != annoncee => {
            Verdict::Alteree
        }
        (Some(_), Some(_)) => verification.verdict,
        (None, _) => Verdict::NonPrisEnCharge(algorithme.unwrap_or_default()),
        _ => Verdict::Illisible,
    };
    lu
}

/// Vérifie une signature PKCS #1 seule (sous-filtre adbe.x509.rsa_sha1, abandonné) : `signature`
/// (une chaîne d'octets DER) de l'empreinte SHA-1 des données, avec la clé du certificat.
pub(crate) fn verifier_pkcs1(
    certificat: &Certificat,
    signature: &[u8],
    donnees: &[&[u8]],
) -> Verdict {
    let Some(valeur) = premier(signature).filter(|t| t.est(4)) else {
        return Verdict::Illisible;
    };
    let hache = Empreinte::Sha1.calculer(donnees);
    match essayer(
        &certificat.cle(),
        "1.2.840.113549.1.1.5",
        Empreinte::Sha1,
        &hache,
        valeur.contenu,
    ) {
        Essai::Bonne => Verdict::Valide,
        Essai::Fausse => Verdict::Fausse,
        Essai::Impossible(nom) => Verdict::NonPrisEnCharge(nom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_le_der_et_le_ber_a_longueur_indefinie() {
        // SEQUENCE { INTEGER 5, OCTET STRING "ab" }, en DER puis en BER (longueur indéfinie).
        let der = [0x30, 0x07, 0x02, 0x01, 0x05, 0x04, 0x02, b'a', b'b'];
        let ber = [
            0x30, 0x80, 0x02, 0x01, 0x05, 0x04, 0x02, b'a', b'b', 0, 0, 0xff,
        ];
        for octets in [&der[..], &ber[..]] {
            let sequence = premier(octets).unwrap();
            assert!(sequence.est(16));
            let enfants: Vec<Tlv> = sequence.enfants().collect();
            assert_eq!(enfants.len(), 2);
            assert_eq!(entier_positif(&enfants[0]), Some(&[5u8][..]));
            assert_eq!(enfants[1].contenu, b"ab");
        }
        assert_eq!(premier(&ber).unwrap().brut.len(), 11);
        // Longueurs impossibles, structure tronquée, imbrication sans fin : rien n'est lu.
        assert!(premier(&[0x30, 0x84, 0xff, 0xff, 0xff, 0xff]).is_none());
        assert!(premier(&[0x04, 0x80, 0, 0]).is_none());
        let profonde: Vec<u8> = std::iter::repeat_n([0x30, 0x80], 200).flatten().collect();
        assert!(premier(&profonde).is_none());
    }

    #[test]
    fn lit_les_identifiants_et_les_dates() {
        let identifiant = [
            0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02,
        ];
        assert_eq!(oid(&premier(&identifiant).unwrap()).unwrap(), SIGNED_DATA);
        let utc = [&[0x17, 0x0d][..], b"261126081200Z"].concat();
        assert_eq!(
            iso(temps(&premier(&utc).unwrap()).unwrap()),
            "2026-11-26T08:12:00Z"
        );
        let generalise = [&[0x18, 0x13][..], b"20261126091200+0100"].concat();
        assert_eq!(
            iso(temps(&premier(&generalise).unwrap()).unwrap()),
            "2026-11-26T08:12:00Z"
        );
        let fractions = [&[0x18, 0x13][..], b"20240229235959.123Z"].concat();
        assert_eq!(
            iso(temps(&premier(&fractions).unwrap()).unwrap()),
            "2024-02-29T23:59:59Z"
        );
        assert_eq!(iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(moment([2026, 13, 1, 0, 0, 0], 0), None);
        assert_eq!(decalage("+01'00'"), 60);
        assert_eq!(decalage("-0530"), -330);
        assert_eq!(decalage("Z"), 0);
    }

    #[test]
    fn lit_les_textes_des_noms() {
        let bmp = [0x1e, 0x04, 0x00, b'J', 0x00, 0xe9];
        assert_eq!(texte(&premier(&bmp).unwrap()).unwrap(), "Jé");
        let latin = [0x14, 0x02, b'J', 0xe9];
        assert_eq!(texte(&premier(&latin).unwrap()).unwrap(), "Jé");
        let nom = Nom(vec![
            (PRENOM.into(), "Jeanne".into()),
            (NOM_DE_FAMILLE.into(), "Martin".into()),
        ]);
        assert_eq!(nom.lisible().unwrap(), "Jeanne Martin");
    }
}
