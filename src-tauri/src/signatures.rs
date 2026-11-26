//! Signatures numériques des PDF reçus (0.7.0) : « Signé par …, le … ; document non modifié
//! depuis la signature ».
//!
//! Pour chaque champ de signature signé :
//! - **l'intégrité** : l'empreinte des octets signés (/ByteRange) est recalculée et comparée à
//!   celle de la signature, elle-même vérifiée avec la clé du certificat (`cms.rs`) ;
//! - **la couverture** : la signature porte-t-elle sur tout le fichier, ou des révisions ont-elles
//!   été ajoutées depuis ? Le document est alors comparé à la version signée : formulaire rempli,
//!   autre signature, commentaires, contenu des pages (`analyser`) ;
//! - **l'identité** : le certificat du signataire est jugé par le système (`confiance.rs`), à la
//!   date de la signature (celle de son horodatage, quand il en a un) ;
//! - **l'horodatage**, vérifié comme une signature.
//!
//! Le PDF est lu tel qu'il est sur le disque : les octets signés ne passent par aucune réécriture.
//! Un PDF signé que l'on annote ou remplit s'enregistre par une révision ajoutée à la suite
//! (`pdf::ecrire_a_la_suite`) : ses signatures restent valables pour la version qu'elles signent.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::Serialize;

use crate::cms::{self, Empreinte, Verdict};
use crate::confiance::{self, Confiance};
use crate::formulaires;
use crate::geometrie::resoudre;
use crate::pdf;

/// Signatures lues au plus ; objets parcourus au plus pour comparer deux versions.
const SIGNATURES_MAX: usize = 64;
const PARCOURS_MAX: usize = 2_000_000;

/// Sorte de signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Sorte {
    /// Une signature ordinaire.
    Approbation,
    /// Une signature de certification : l'auteur fixe les modifications permises ensuite.
    Certification,
    /// Un horodatage du document (pas de signataire, une autorité d'horodatage).
    Horodatage,
}

/// La signature correspond-elle au document signé ?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Integrite {
    Intacte,
    /// L'empreinte ne correspond plus : la version signée a été modifiée.
    Alteree,
    /// La signature ne correspond pas à la clé du certificat.
    Fausse,
    /// Algorithme qu'Automat-PDF ne sait pas vérifier.
    NonPriseEnCharge,
    /// Signature illisible, incomplète, ou mal placée dans le fichier.
    Illisible,
}

/// Ce qui a changé depuis la version signée.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Modifications {
    /// Champs du formulaire remplis ou changés.
    pub formulaire: bool,
    /// Autres signatures, horodatages ou données de validation ajoutés.
    pub signatures: bool,
    /// Commentaires ajoutés, changés ou retirés.
    pub commentaires: bool,
    /// Contenu des pages, pages ajoutées, retirées ou déplacées.
    pub pages: bool,
    /// Autres changements (actions à l'ouverture, scripts, signets…).
    pub autres: bool,
}

/// Le certificat du signataire, pour l'interface.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Certificat {
    pub sujet: Option<String>,
    pub organisation: Option<String>,
    pub courriel: Option<String>,
    pub emetteur: Option<String>,
    /// Valable du … au … (ISO 8601).
    pub debut: Option<String>,
    pub fin: Option<String>,
    /// « RSA 2048 bits », « ECDSA P-256 ».
    pub cle: String,
    /// Numéro de série, en hexadécimal.
    pub numero: String,
    /// Empreinte SHA-256 du certificat, en hexadécimal.
    pub empreinte: String,
}

/// L'horodatage d'une signature.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HorodatageLu {
    pub date: Option<String>,
    pub autorite: Option<String>,
    pub valide: bool,
    pub confiance: Confiance,
}

/// Une signature numérique du PDF, vérifiée.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignaturePdf {
    /// Nom complet du champ de signature.
    pub champ: String,
    /// Page (à partir de 0) et zone de la signature visible, sur la page affichée (x, y, largeur,
    /// hauteur) ; rien pour une signature invisible.
    pub page: Option<usize>,
    pub zone: Option<[f64; 4]>,
    pub sorte: Sorte,
    /// Modifications permises par une certification : 1 aucune ; 2 remplir et signer ; 3 aussi
    /// commenter.
    pub permissions: Option<u8>,
    /// Le signataire, tel que son certificat le nomme (sinon tel que la signature le déclare).
    pub signataire: Option<String>,
    pub raison: Option<String>,
    pub lieu: Option<String>,
    pub contact: Option<String>,
    /// Date de la signature (ISO 8601) : celle de son horodatage vérifié, sinon celle que
    /// déclare l'ordinateur du signataire.
    pub date: Option<String>,
    pub date_horodatee: bool,
    pub horodatage: Option<HorodatageLu>,
    pub integrite: Integrite,
    /// L'algorithme qu'Automat-PDF ne sait pas vérifier.
    pub non_pris_en_charge: Option<String>,
    /// Empreinte utilisée (« SHA-256 ») ; SHA-1 est ancienne, et ne protège plus assez.
    pub empreinte: Option<String>,
    pub empreinte_ancienne: bool,
    pub confiance: Confiance,
    pub certificat: Option<Certificat>,
    /// La signature porte sur tout le fichier ; sinon, ce qui a changé depuis.
    pub entiere: bool,
    pub modifications: Modifications,
    /// Taille de la version signée, en octets : la voir (`version_signee`).
    pub fin: usize,
}

/// Les signatures numériques d'un PDF (`octets`, tel qu'il est sur le disque), vérifiées, dans
/// l'ordre où elles ont été faites.
pub fn signatures(octets: &[u8], nom: &str) -> Vec<SignaturePdf> {
    let Ok(document) = pdf::charger(octets, nom) else {
        return Vec::new();
    };
    let chiffre = document.encryption_state.is_some();
    let mut liste: Vec<SignaturePdf> = formulaires::champs_signes(&document)
        .into_iter()
        .take(SIGNATURES_MAX)
        .filter_map(|(champ, valeur, widgets)| {
            let mut signature = lire(&document, octets, valeur.as_dict().ok()?, chiffre);
            signature.champ = champ;
            if let Some(widget) = widgets
                .iter()
                .find(|w| w.largeur >= 1.0 && w.hauteur >= 1.0)
            {
                signature.page = Some(widget.page);
                signature.zone = Some([widget.x, widget.y, widget.largeur, widget.hauteur]);
            }
            Some(signature)
        })
        .collect();
    let mut analyses: HashMap<usize, Modifications> = HashMap::new();
    for signature in liste.iter_mut().filter(|s| !s.entiere && s.fin > 0) {
        let fin = signature.fin;
        signature.modifications = *analyses
            .entry(fin)
            .or_insert_with(|| analyser(&document, &octets[..fin], nom));
    }
    liste.sort_by_key(|s| s.fin);
    liste
}

/// Le PDF a-t-il des signatures numériques à leur place (pour l'enregistrer à la suite, sans les
/// rompre) ?
pub(crate) fn a_des_signatures(document: &Document, octets: &[u8]) -> bool {
    let chiffre = document.encryption_state.is_some();
    formulaires::champs_signes(document)
        .iter()
        .any(|(_, valeur, _)| {
            valeur
                .as_dict()
                .is_ok_and(|d| plages(document, d, octets, chiffre).is_some())
        })
}

/// La version du PDF qu'a signée une signature (les `fin` premiers octets), telle qu'elle était,
/// et son rang parmi les versions signées (à partir de 1).
pub fn version_signee(octets: &[u8], nom: &str, fin: usize) -> Option<(usize, Vec<u8>)> {
    let document = pdf::charger(octets, nom).ok()?;
    let chiffre = document.encryption_state.is_some();
    let fins: BTreeSet<usize> = formulaires::champs_signes(&document)
        .iter()
        .filter_map(|(_, valeur, _)| plages(&document, valeur.as_dict().ok()?, octets, chiffre))
        .map(|([_, (suite, longueur)], _)| suite + longueur)
        .collect();
    let rang = fins.iter().position(|&f| f == fin)?;
    Some((rang + 1, octets[..fin].to_vec()))
}

// ------------------------------------------------------------------------------- Lecture

fn propre(texte: &str) -> String {
    texte
        .chars()
        .filter(|c| !c.is_control())
        .take(300)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Une date PDF (« D:20261126091200+01'00' »), en secondes depuis 1970 (UTC).
fn date_pdf(texte: &str) -> Option<i64> {
    let texte = texte.trim().trim_start_matches("D:");
    let chiffres: String = texte
        .chars()
        .take_while(char::is_ascii_digit)
        .take(14)
        .collect();
    if chiffres.len() < 4 {
        return None;
    }
    let partie = |debut: usize, defaut: i64| {
        chiffres
            .get(debut..debut + 2)
            .and_then(|p| p.parse().ok())
            .unwrap_or(defaut)
    };
    cms::moment(
        [
            chiffres[0..4].parse().ok()?,
            partie(4, 1),
            partie(6, 1),
            partie(8, 0),
            partie(10, 0),
            partie(12, 0),
        ],
        cms::decalage(&texte[chiffres.len()..]),
    )
}

/// Des chiffres hexadécimaux (espaces permis) ; un chiffre seul à la fin vaut « x0 ».
fn hexadecimal(octets: &[u8]) -> Option<Vec<u8>> {
    let mut sortie = Vec::with_capacity(octets.len() / 2);
    let mut moitie: Option<u8> = None;
    for &octet in octets {
        let valeur = match octet {
            b'0'..=b'9' => octet - b'0',
            b'a'..=b'f' => octet - b'a' + 10,
            b'A'..=b'F' => octet - b'A' + 10,
            b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' | b'\0' => continue,
            _ => return None,
        };
        match moitie.take() {
            Some(haute) => sortie.push(haute << 4 | valeur),
            None => moitie = Some(valeur),
        }
    }
    sortie.extend(moitie.map(|haute| haute << 4));
    Some(sortie)
}

/// Deux plages d'octets : (début, longueur).
type Plages = [(usize, usize); 2];

/// Les deux plages d'octets signées (/ByteRange) et la signature (/Contents), lue entre elles.
///
/// Contrôles : la première plage part du début du fichier ; entre les deux, il n'y a que la
/// signature, en hexadécimal ; c'est bien celle du dictionnaire. Sinon, la signature pourrait
/// couvrir autre chose que ce qu'elle annonce.
fn plages(
    document: &Document,
    dictionnaire: &Dictionary,
    octets: &[u8],
    chiffre: bool,
) -> Option<(Plages, Vec<u8>)> {
    let valeurs: Vec<usize> = resoudre(document, dictionnaire.get(b"ByteRange").ok()?)
        .as_array()
        .ok()?
        .iter()
        .map(|o| o.as_i64().ok().and_then(|n| usize::try_from(n).ok()))
        .collect::<Option<_>>()?;
    let [debut, longueur, suite, longueur_suite] = valeurs[..] else {
        return None;
    };
    if debut != 0 || longueur >= suite || suite.checked_add(longueur_suite)? > octets.len() {
        return None;
    }
    let trou = &octets[longueur..suite];
    let (Some(b'<'), Some(b'>')) = (trou.first(), trou.last()) else {
        return None;
    };
    let contenu = hexadecimal(&trou[1..trou.len() - 1])?;
    // Une chaîne d'un PDF chiffré est lue déchiffrée par lopdf : on ne peut que la comparer brute.
    if !chiffre {
        let annoncee = resoudre(document, dictionnaire.get(b"Contents").ok()?)
            .as_str()
            .ok()?;
        if annoncee != contenu.as_slice() {
            return None;
        }
    }
    Some(([(0, longueur), (suite, longueur_suite)], contenu))
}

/// Une certification (DocMDP) : les modifications qu'elle permet ensuite (1, 2 ou 3).
fn permissions_de_certification(document: &Document, dictionnaire: &Dictionary) -> Option<u8> {
    let references = resoudre(document, dictionnaire.get(b"Reference").ok()?)
        .as_array()
        .ok()?;
    references.iter().find_map(|reference| {
        let reference = resoudre(document, reference).as_dict().ok()?;
        if reference.get(b"TransformMethod").ok()?.as_name().ok()? != b"DocMDP" {
            return None;
        }
        let permissions = reference
            .get(b"TransformParams")
            .ok()
            .and_then(|p| resoudre(document, p).as_dict().ok())
            .and_then(|p| p.get(b"P").ok())
            .and_then(|p| p.as_i64().ok())
            .unwrap_or(2);
        Some(permissions.clamp(1, 3) as u8)
    })
}

fn hexa(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02X}")).collect()
}

fn certificat_lu(certificat: &cms::Certificat) -> Certificat {
    Certificat {
        sujet: certificat.sujet.lisible(),
        organisation: certificat
            .sujet
            .attribut(cms::ORGANISATION)
            .map(String::from),
        courriel: certificat.courriel(),
        emetteur: certificat.emetteur.lisible(),
        debut: certificat.debut.map(cms::iso),
        fin: certificat.fin.map(cms::iso),
        cle: cms::nom_de_cle(&certificat.cle()),
        numero: hexa(certificat.serie),
        empreinte: hexa(&Empreinte::Sha256.calculer(&[certificat.der])),
    }
}

fn integrite(verdict: &Verdict, non_pris_en_charge: &mut Option<String>) -> Integrite {
    match verdict {
        Verdict::Valide => Integrite::Intacte,
        Verdict::Alteree => Integrite::Alteree,
        Verdict::Fausse => Integrite::Fausse,
        Verdict::NonPrisEnCharge(nom) => {
            *non_pris_en_charge = Some(nom.clone());
            Integrite::NonPriseEnCharge
        }
        Verdict::Illisible => Integrite::Illisible,
    }
}

/// Juge un certificat (le premier de `certificats`, les autres complétant sa chaîne).
fn juger(certificats: &[Vec<u8>], moment: Option<i64>) -> Confiance {
    let ders: Vec<&[u8]> = certificats.iter().map(Vec::as_slice).collect();
    confiance::juger(&ders, moment)
}

/// Lit et vérifie la signature du dictionnaire `dictionnaire`.
fn lire(
    document: &Document,
    octets: &[u8],
    dictionnaire: &Dictionary,
    chiffre: bool,
) -> SignaturePdf {
    let nom = |cle: &[u8]| {
        dictionnaire
            .get(cle)
            .ok()
            .and_then(|o| resoudre(document, o).as_name().ok())
            .map(|n| String::from_utf8_lossy(n).into_owned())
    };
    let texte = |cle: &[u8]| {
        dictionnaire
            .get(cle)
            .ok()
            .and_then(|o| lopdf::decode_text_string(resoudre(document, o)).ok())
            .map(|t| propre(&t))
            .filter(|t| !t.is_empty())
    };
    let sous_filtre = nom(b"SubFilter").unwrap_or_default();
    let du_document =
        nom(b"Type").as_deref() == Some("DocTimeStamp") || sous_filtre == "ETSI.RFC3161";
    let permissions = permissions_de_certification(document, dictionnaire);
    let declaree = texte(b"M").and_then(|m| date_pdf(&m));
    let mut signature = SignaturePdf {
        champ: String::new(),
        page: None,
        zone: None,
        sorte: match (du_document, permissions) {
            (true, _) => Sorte::Horodatage,
            (false, Some(_)) => Sorte::Certification,
            (false, None) => Sorte::Approbation,
        },
        permissions: permissions.filter(|_| !du_document),
        signataire: texte(b"Name"),
        raison: texte(b"Reason"),
        lieu: texte(b"Location"),
        contact: texte(b"ContactInfo"),
        date: declaree.map(cms::iso),
        date_horodatee: false,
        horodatage: None,
        integrite: Integrite::Illisible,
        non_pris_en_charge: None,
        empreinte: None,
        empreinte_ancienne: false,
        confiance: Confiance::Inconnue,
        certificat: None,
        entiere: false,
        modifications: Modifications::default(),
        fin: 0,
    };
    let Some(([premiere, (suite, longueur)], contenu)) =
        plages(document, dictionnaire, octets, chiffre)
    else {
        return signature;
    };
    signature.fin = suite + longueur;
    // Des blancs après la fin signée (ajoutés par une messagerie) ne changent rien.
    signature.entiere = octets[signature.fin..]
        .iter()
        .all(|o| b" \t\r\n\x0c\0".contains(o));
    let morceaux = [
        &octets[premiere.0..premiere.0 + premiere.1],
        &octets[suite..signature.fin],
    ];

    // Horodatage du document : un jeton d'horodatage de ses octets.
    if du_document {
        let jeton = cms::horodatage(&contenu, &morceaux);
        signature.integrite = integrite(&jeton.verdict, &mut signature.non_pris_en_charge);
        signature.signataire = jeton.autorite.clone();
        signature.date = jeton.date.map(cms::iso).or(signature.date);
        signature.date_horodatee = jeton.verdict == Verdict::Valide && jeton.date.is_some();
        signature.confiance = juger(&jeton.certificats, jeton.date);
        signature.certificat = jeton
            .certificats
            .first()
            .and_then(|der| cms::premier(der))
            .and_then(cms::certificat)
            .map(|c| certificat_lu(&c));
        return signature;
    }

    // Signature PKCS #1 seule (adbe.x509.rsa_sha1, abandonnée) : le certificat est dans /Cert.
    if sous_filtre == "adbe.x509.rsa_sha1" {
        let ders: Vec<Vec<u8>> = dictionnaire
            .get(b"Cert")
            .ok()
            .map(|c| match resoudre(document, c) {
                Object::Array(liste) => liste
                    .iter()
                    .filter_map(|o| resoudre(document, o).as_str().ok().map(<[u8]>::to_vec))
                    .collect(),
                autre => autre
                    .as_str()
                    .ok()
                    .map(<[u8]>::to_vec)
                    .into_iter()
                    .collect(),
            })
            .unwrap_or_default();
        if let Some(certificat) = ders
            .first()
            .and_then(|der| cms::premier(der))
            .and_then(cms::certificat)
        {
            let verdict = cms::verifier_pkcs1(&certificat, &contenu, &morceaux);
            signature.integrite = integrite(&verdict, &mut signature.non_pris_en_charge);
            signature.empreinte = Some(Empreinte::Sha1.nom().into());
            signature.empreinte_ancienne = true;
            signature.signataire = certificat.sujet.lisible().or(signature.signataire);
            signature.certificat = Some(certificat_lu(&certificat));
            signature.confiance = juger(&ders, declaree);
        }
        return signature;
    }

    // Signature CMS : détachée (adbe.pkcs7.detached, ETSI.CAdES.detached), ou portant l'empreinte
    // SHA-1 des octets signés (adbe.pkcs7.sha1).
    let Some(lue) = cms::cms(&contenu) else {
        return signature;
    };
    let mut verification = if sous_filtre == "adbe.pkcs7.sha1" {
        let mut verification = cms::verifier(&lue, None);
        if verification.verdict == Verdict::Valide
            && lue.contenu.as_deref() != Some(Empreinte::Sha1.calculer(&morceaux).as_slice())
        {
            verification.verdict = Verdict::Alteree;
        }
        verification
    } else {
        cms::verifier(&lue, Some(&morceaux[..]))
    };
    if sous_filtre == "adbe.pkcs7.sha1" && verification.empreinte.is_some() {
        verification.empreinte = Some(Empreinte::Sha1);
    }
    signature.integrite = integrite(&verification.verdict, &mut signature.non_pris_en_charge);
    signature.empreinte = verification.empreinte.map(|e| e.nom().into());
    signature.empreinte_ancienne = verification.empreinte == Some(Empreinte::Sha1);
    let certificat = verification.certificat.map(|rang| &lue.certificats[rang]);
    signature.signataire = certificat
        .and_then(|c| c.sujet.lisible())
        .or(signature.signataire);
    signature.certificat = certificat.map(certificat_lu);

    // La date : celle de l'horodatage vérifié, sinon celle que déclare le signataire.
    let horodatage = verification
        .jeton
        .map(|jeton| cms::horodatage(jeton, &[verification.valeur]));
    let date_horodatee = horodatage
        .as_ref()
        .filter(|h| h.verdict == Verdict::Valide)
        .and_then(|h| h.date);
    let date = date_horodatee.or(verification.date).or(declaree);
    signature.date = date.map(cms::iso);
    signature.date_horodatee = date_horodatee.is_some();
    signature.horodatage = horodatage.map(|h| HorodatageLu {
        date: h.date.map(cms::iso),
        autorite: h.autorite.clone(),
        valide: h.verdict == Verdict::Valide,
        confiance: juger(&h.certificats, h.date),
    });

    // L'identité : le certificat du signataire, les autres joints complétant sa chaîne.
    if let Some(rang) = verification.certificat {
        let mut chaine = vec![lue.certificats[rang].der.to_vec()];
        chaine.extend(
            lue.certificats
                .iter()
                .enumerate()
                .filter(|(autre, _)| *autre != rang)
                .map(|(_, c)| c.der.to_vec()),
        );
        signature.confiance = juger(&chaine, date);
    }
    signature
}

// ------------------------------------------------------------------------------- Comparaison

/// À quoi sert un objet du PDF : de la moins à la plus importante (un objet qui sert à deux
/// choses compte pour la plus importante).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Usage {
    Signatures,
    Formulaire,
    Commentaires,
    Autres,
    Pages,
}

/// Clés que le parcours ne suit pas : retour à la page (/P), au parent, aux autres annotations.
const NON_SUIVIES: [&[u8]; 3] = [b"P", b"Parent", b"Annots"];

fn marquer(usages: &mut HashMap<ObjectId, Usage>, id: ObjectId, usage: Usage) {
    let actuel = usages.entry(id).or_insert(usage);
    *actuel = (*actuel).max(usage);
}

/// Marque `usage` sur tout ce qu'atteint `depart`.
fn parcourir<'a>(
    document: &'a Document,
    depart: &'a Object,
    usage: Usage,
    usages: &mut HashMap<ObjectId, Usage>,
    budget: &mut usize,
) {
    let mut pile = vec![depart];
    while let Some(objet) = pile.pop() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        match objet {
            Object::Reference(id) => {
                if usages.get(id).is_some_and(|&u| u >= usage) {
                    continue;
                }
                marquer(usages, *id, usage);
                if let Ok(suite) = document.get_object(*id) {
                    pile.push(suite);
                }
            }
            Object::Array(valeurs) => pile.extend(valeurs),
            Object::Dictionary(dictionnaire) => pile.extend(
                dictionnaire
                    .iter()
                    .filter(|(cle, _)| !NON_SUIVIES.contains(&cle.as_slice()))
                    .map(|(_, valeur)| valeur),
            ),
            Object::Stream(flux) => pile.extend(
                flux.dict
                    .iter()
                    .filter(|(cle, _)| !NON_SUIVIES.contains(&cle.as_slice()))
                    .map(|(_, valeur)| valeur),
            ),
            _ => {}
        }
    }
}

/// La sorte (/FT) d'un champ ou d'un widget, héritée de ses parents.
fn sorte_de_champ(document: &Document, dictionnaire: &Dictionary) -> Option<Vec<u8>> {
    let mut courant = Some(dictionnaire);
    for _ in 0..32 {
        let champ = courant?;
        if let Ok(sorte) = champ.get(b"FT").and_then(Object::as_name) {
            return Some(sorte.to_vec());
        }
        courant = champ
            .get(b"Parent")
            .ok()
            .and_then(|p| resoudre(document, p).as_dict().ok());
    }
    None
}

fn usage_de_champ(sorte: Option<&[u8]>) -> Usage {
    if sorte == Some(b"Sig") {
        Usage::Signatures
    } else {
        Usage::Formulaire
    }
}

/// Un champ du formulaire et ses enfants : signature ou formulaire, selon leur sorte.
fn parcourir_champ(
    document: &Document,
    objet: &Object,
    herite: Option<&[u8]>,
    usages: &mut HashMap<ObjectId, Usage>,
    budget: &mut usize,
    profondeur: usize,
) {
    let Ok(champ) = resoudre(document, objet).as_dict() else {
        return;
    };
    if profondeur > 32 {
        return;
    }
    let sorte = champ.get(b"FT").and_then(Object::as_name).ok().or(herite);
    let usage = usage_de_champ(sorte);
    if let Object::Reference(id) = objet {
        marquer(usages, *id, usage);
    }
    for (cle, valeur) in champ.iter() {
        if cle.as_slice() == b"Kids" {
            if let Object::Reference(id) = valeur {
                marquer(usages, *id, Usage::Signatures);
            }
            if let Ok(enfants) = resoudre(document, valeur).as_array() {
                for enfant in enfants {
                    parcourir_champ(document, enfant, sorte, usages, budget, profondeur + 1);
                }
            }
        } else if !NON_SUIVIES.contains(&cle.as_slice()) {
            parcourir(document, valeur, usage, usages, budget);
        }
    }
}

/// L'usage de chaque objet d'une version du PDF : contenu des pages, commentaires, formulaire,
/// signatures. Un tableau qui ne fait que lister (annotations d'une page, champs du formulaire)
/// compte pour le moins : ce qu'il liste compte pour lui-même.
fn marquer_usages(document: &Document, usages: &mut HashMap<ObjectId, Usage>) {
    let mut budget = PARCOURS_MAX;
    for id_page in document.get_pages().into_values() {
        let Ok(page) = document.get_dictionary(id_page) else {
            continue;
        };
        for (cle, valeur) in page.iter() {
            match cle.as_slice() {
                b"Annots" => {
                    if let Object::Reference(id) = valeur {
                        marquer(usages, *id, Usage::Signatures);
                    }
                    let Ok(annotations) = resoudre(document, valeur).as_array() else {
                        continue;
                    };
                    for annotation in annotations {
                        let Ok(dictionnaire) = resoudre(document, annotation).as_dict() else {
                            continue;
                        };
                        let usage = if dictionnaire.get(b"Subtype").and_then(Object::as_name).ok()
                            == Some(b"Widget".as_slice())
                        {
                            usage_de_champ(sorte_de_champ(document, dictionnaire).as_deref())
                        } else {
                            Usage::Commentaires
                        };
                        parcourir(document, annotation, usage, usages, &mut budget);
                    }
                }
                b"Parent" => {}
                _ => parcourir(document, valeur, Usage::Pages, usages, &mut budget),
            }
        }
    }
    let Ok(catalogue) = document.catalog() else {
        return;
    };
    for (cle, valeur) in catalogue.iter() {
        match cle.as_slice() {
            b"Pages" | b"Metadata" => {}
            b"AcroForm" => {
                if let Object::Reference(id) = valeur {
                    marquer(usages, *id, Usage::Signatures);
                }
                let Ok(formulaire) = resoudre(document, valeur).as_dict() else {
                    continue;
                };
                for (cle, valeur) in formulaire.iter() {
                    if cle.as_slice() == b"Fields" {
                        if let Object::Reference(id) = valeur {
                            marquer(usages, *id, Usage::Signatures);
                        }
                        for champ in resoudre(document, valeur).as_array().into_iter().flatten() {
                            parcourir_champ(document, champ, None, usages, &mut budget, 0);
                        }
                    } else {
                        parcourir(document, valeur, Usage::Formulaire, usages, &mut budget);
                    }
                }
            }
            // Données de validation des signatures ; la signature de certification.
            b"DSS" | b"Perms" => {
                parcourir(document, valeur, Usage::Signatures, usages, &mut budget)
            }
            _ => parcourir(document, valeur, Usage::Autres, usages, &mut budget),
        }
    }
}

/// Un dictionnaire (ou celui d'un flux) sans certaines clés, pour comparer le reste.
fn sans<'a>(objet: Option<&'a Object>, cles: &[&[u8]]) -> Option<BTreeMap<&'a [u8], &'a Object>> {
    let dictionnaire = match objet? {
        Object::Dictionary(d) => d,
        Object::Stream(flux) => &flux.dict,
        _ => return None,
    };
    Some(
        dictionnaire
            .iter()
            .filter(|(cle, _)| !cles.contains(&cle.as_slice()))
            .map(|(cle, valeur)| (cle.as_slice(), valeur))
            .collect(),
    )
}

/// Ce qui a changé dans le catalogue : formulaire, signatures et données de validation comptent à
/// part (leurs objets sont comparés eux-mêmes) ; les métadonnées ne comptent pas.
fn comparer_catalogues(
    avant: Option<&Object>,
    apres: Option<&Object>,
    modifications: &mut Modifications,
) {
    const A_PART: [&[u8]; 7] = [
        b"AcroForm",
        b"DSS",
        b"Perms",
        b"Metadata",
        b"Extensions",
        b"Version",
        b"Pages",
    ];
    let (Some(avant_), Some(apres_)) = (sans(avant, &A_PART), sans(apres, &A_PART)) else {
        modifications.autres = true;
        return;
    };
    if avant_ != apres_ {
        modifications.autres = true;
    }
    let pages = |objet: Option<&Object>| -> Option<Object> {
        sans(objet, &[])?
            .get(b"Pages".as_slice())
            .map(|o| (*o).clone())
    };
    if pages(avant) != pages(apres) {
        modifications.pages = true;
    }
}

/// Ce qui a changé entre la version signée (`octets_avant`) et le document (`apres`).
fn analyser(apres: &Document, octets_avant: &[u8], nom: &str) -> Modifications {
    let mut modifications = Modifications::default();
    let Ok(avant) = pdf::charger(octets_avant, nom) else {
        modifications.autres = true;
        return modifications;
    };
    let pages_avant: Vec<ObjectId> = avant.get_pages().into_values().collect();
    let pages_apres: Vec<ObjectId> = apres.get_pages().into_values().collect();
    if pages_avant != pages_apres {
        modifications.pages = true;
    }
    let mut usages = HashMap::new();
    marquer_usages(&avant, &mut usages);
    marquer_usages(apres, &mut usages);
    let racine = |document: &Document| {
        document
            .trailer
            .get(b"Root")
            .and_then(Object::as_reference)
            .ok()
    };
    let racines: HashSet<ObjectId> = [racine(&avant), racine(apres)]
        .into_iter()
        .flatten()
        .collect();
    let pages: HashSet<ObjectId> = pages_avant.iter().chain(&pages_apres).copied().collect();
    let ids: BTreeSet<ObjectId> = avant
        .objects
        .keys()
        .chain(apres.objects.keys())
        .copied()
        .collect();
    for id in ids {
        let (objet_avant, objet_apres) = (avant.objects.get(&id), apres.objects.get(&id));
        if objet_avant == objet_apres {
            continue;
        }
        if racines.contains(&id) {
            comparer_catalogues(objet_avant, objet_apres, &mut modifications);
        } else if pages.contains(&id) {
            // Une page : ses annotations comptent pour elles-mêmes.
            if sans(objet_avant, &[b"Annots"]) != sans(objet_apres, &[b"Annots"]) {
                modifications.pages = true;
            }
        } else {
            match usages.get(&id) {
                Some(Usage::Pages) => modifications.pages = true,
                Some(Usage::Autres) => modifications.autres = true,
                Some(Usage::Commentaires) => modifications.commentaires = true,
                Some(Usage::Formulaire) => modifications.formulaire = true,
                Some(Usage::Signatures) => modifications.signatures = true,
                None => {}
            }
        }
    }
    modifications
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use lopdf::{IncrementalDocument, dictionary};

    use super::*;
    use crate::annotations::{Annotation, Point, Teinte, Trace, Zone};
    use crate::commentaires::Retouches;
    use crate::essais::{CleEssai, Identite, PdfSigne, fichier_temporaire, pdf_signe};

    struct Essais {
        autorite: Identite,
        signataire: Identite,
        signataire_rsa: Identite,
        signataire_pss: Identite,
        horodateur: Identite,
    }

    fn identites() -> &'static Essais {
        static IDENTITES: OnceLock<Essais> = OnceLock::new();
        IDENTITES.get_or_init(|| {
            let autorite =
                Identite::autorite(CleEssai::p256("autorité"), "Autorité d'essai Automat-PDF");
            let cle_rsa = CleEssai::rsa("Paul");
            Essais {
                signataire: Identite::emise_par(
                    &autorite,
                    CleEssai::p256("Jeanne"),
                    "Jeanne Martin",
                    Some("Exemple SARL"),
                    1001,
                ),
                signataire_rsa: Identite::emise_par(
                    &autorite,
                    CleEssai::Rsa(cle_rsa.clone()),
                    "Paul Durand",
                    None,
                    1002,
                ),
                signataire_pss: Identite::emise_par(
                    &autorite,
                    CleEssai::RsaPss(cle_rsa),
                    "Paul Durand",
                    None,
                    1003,
                ),
                horodateur: Identite::emise_par(
                    &autorite,
                    CleEssai::p256("horodatage"),
                    "Horodatage d'essai",
                    Some("Automat-PDF (essais)"),
                    2001,
                ),
                autorite,
            }
        })
    }

    /// Le 26 novembre 2026, 8 h 12 (UTC).
    fn date() -> i64 {
        cms::moment([2026, 11, 26, 8, 12, 0], 0).unwrap()
    }

    fn options<'a>(signataire: &'a Identite, sous_filtre: &'a str) -> PdfSigne<'a> {
        PdfSigne {
            signataire,
            autres: vec![&identites().autorite.certificat],
            sous_filtre,
            date: date(),
            horodatage: None,
            certification: None,
            horodatage_du_document: false,
        }
    }

    fn contrat() -> Vec<u8> {
        pdf_signe(&options(&identites().signataire, "adbe.pkcs7.detached"))
    }

    /// Une révision ajoutée à la suite du PDF, comme le ferait un autre logiciel.
    fn revision(octets: &[u8], modifier: impl FnOnce(&mut IncrementalDocument)) -> Vec<u8> {
        let mut suite: IncrementalDocument = octets.try_into().unwrap();
        modifier(&mut suite);
        let mut sortie = Vec::new();
        suite.save_to(&mut sortie).unwrap();
        sortie
    }

    fn premiere_page(suite: &IncrementalDocument) -> ObjectId {
        suite.get_prev_documents().page_iter().next().unwrap()
    }

    #[test]
    fn verifie_une_signature_ecdsa_de_tout_le_document() {
        let octets = contrat();
        let lues = signatures(&octets, "Contrat.pdf");
        assert_eq!(lues.len(), 1);
        let signature = &lues[0];
        assert_eq!(signature.champ, "Signature1");
        assert_eq!(signature.integrite, Integrite::Intacte);
        assert!(signature.entiere);
        assert_eq!(signature.modifications, Modifications::default());
        assert_eq!(signature.sorte, Sorte::Approbation);
        assert_eq!(signature.signataire.as_deref(), Some("Jeanne Martin"));
        assert_eq!(signature.raison.as_deref(), Some("J'approuve ce contrat"));
        assert_eq!(signature.lieu.as_deref(), Some("Lyon"));
        assert_eq!(signature.date.as_deref(), Some("2026-11-26T08:12:00Z"));
        assert!(!signature.date_horodatee);
        assert_eq!(signature.empreinte.as_deref(), Some("SHA-256"));
        assert!(!signature.empreinte_ancienne);
        let certificat = signature.certificat.as_ref().unwrap();
        assert_eq!(certificat.sujet.as_deref(), Some("Jeanne Martin"));
        assert_eq!(certificat.organisation.as_deref(), Some("Exemple SARL"));
        assert_eq!(
            certificat.emetteur.as_deref(),
            Some("Autorité d'essai Automat-PDF")
        );
        assert_eq!(certificat.cle, "ECDSA P-256");
        assert_eq!(certificat.debut.as_deref(), Some("2026-01-01T00:00:00Z"));
        assert_eq!(certificat.numero, "03E9");
        assert_eq!(certificat.empreinte.len(), 64);
        // La signature visible, sur la page affichée (origine en haut à gauche).
        assert_eq!(signature.page, Some(0));
        let [x, y, largeur, hauteur] = signature.zone.unwrap();
        assert!((x - 300.0).abs() < 0.01 && (y - 662.0).abs() < 0.01);
        assert!((largeur - 220.0).abs() < 0.01 && (hauteur - 60.0).abs() < 0.01);
        assert_eq!(signature.fin, octets.len());
        // Une autorité d'essai n'est pas reconnue par le système.
        #[cfg(any(target_os = "macos", windows))]
        assert_eq!(signature.confiance, Confiance::NonReconnue);
    }

    #[test]
    fn verifie_les_signatures_rsa_et_rsa_pss_horodatees() {
        let horodatage = date() + 5;
        for (signataire, sous_filtre) in [
            (&identites().signataire_rsa, "ETSI.CAdES.detached"),
            (&identites().signataire_pss, "adbe.pkcs7.detached"),
        ] {
            let octets = pdf_signe(&PdfSigne {
                horodatage: Some((&identites().horodateur, horodatage)),
                ..options(signataire, sous_filtre)
            });
            let signature = &signatures(&octets, "Contrat.pdf")[0];
            assert_eq!(signature.integrite, Integrite::Intacte, "{sous_filtre}");
            assert_eq!(signature.signataire.as_deref(), Some("Paul Durand"));
            assert_eq!(signature.certificat.as_ref().unwrap().cle, "RSA 2048 bits");
            // La date est celle de l'horodatage, vérifié.
            assert!(signature.date_horodatee);
            assert_eq!(signature.date.as_deref(), Some("2026-11-26T08:12:05Z"));
            let jeton = signature.horodatage.as_ref().unwrap();
            assert!(jeton.valide);
            assert_eq!(jeton.autorite.as_deref(), Some("Horodatage d'essai"));
        }
    }

    #[test]
    fn voit_la_version_signee_alteree() {
        let mut octets = contrat();
        let position = octets.windows(7).position(|f| f == b"Contrat").unwrap();
        octets[position] = b'K';
        let signature = &signatures(&octets, "Contrat.pdf")[0];
        assert_eq!(signature.integrite, Integrite::Alteree);
        assert_eq!(signature.signataire.as_deref(), Some("Jeanne Martin"));
    }

    #[test]
    fn refuse_une_signature_qui_ne_couvre_pas_ce_qu_elle_annonce() {
        let mut octets = contrat();
        let document = pdf::charger(&octets, "Contrat.pdf").unwrap();
        let (_, valeur, _) = formulaires::champs_signes(&document).remove(0);
        let debut = valeur
            .as_dict()
            .unwrap()
            .get(b"ByteRange")
            .unwrap()
            .as_array()
            .unwrap()[1]
            .as_i64()
            .unwrap();
        // La première plage s'arrête un octet plus loin : la signature n'est plus seule entre elles.
        let avant = format!("{debut:<10}");
        let position = octets
            .windows(avant.len())
            .position(|f| f == avant.as_bytes())
            .unwrap();
        octets[position..position + 10].copy_from_slice(format!("{:<10}", debut + 1).as_bytes());
        let signature = &signatures(&octets, "Contrat.pdf")[0];
        assert_eq!(signature.integrite, Integrite::Illisible);
        assert_eq!(signature.signataire.as_deref(), Some("Jeanne Martin"));
        assert_eq!(signature.date.as_deref(), Some("2026-11-26T08:12:00Z"));
    }

    #[test]
    fn dit_ce_qui_a_change_apres_la_signature() {
        let original = contrat();
        let fin = original.len();
        let nom = "Contrat.pdf";
        let changements = |octets: &[u8]| {
            let signature = signatures(octets, nom).remove(0);
            assert_eq!(signature.integrite, Integrite::Intacte);
            assert!(!signature.entiere);
            assert_eq!(signature.fin, fin);
            signature.modifications
        };

        // Un commentaire ajouté.
        let commentee = revision(&original, |suite| {
            let page = premiere_page(suite);
            suite.opt_clone_object_to_new_document(page).unwrap();
            let note = suite.new_document.add_object(dictionary! {
                "Type" => "Annot", "Subtype" => "Text",
                "Rect" => vec![50.into(), 50.into(), 70.into(), 70.into()],
                "Contents" => Object::string_literal("A revoir"),
            });
            let page = suite.new_document.get_dictionary_mut(page).unwrap();
            let annotations = page.get_mut(b"Annots").unwrap().as_array_mut().unwrap();
            annotations.push(note.into());
        });
        let attendu = Modifications {
            commentaires: true,
            ..Default::default()
        };
        assert_eq!(changements(&commentee), attendu);

        // Le texte de la page changé.
        let modifiee = revision(&original, |suite| {
            let page = premiere_page(suite);
            let contenu = suite
                .get_prev_documents()
                .get_dictionary(page)
                .unwrap()
                .get(b"Contents")
                .unwrap()
                .as_reference()
                .unwrap();
            suite.opt_clone_object_to_new_document(contenu).unwrap();
            let flux = suite.new_document.get_object_mut(contenu).unwrap();
            flux.as_stream_mut()
                .unwrap()
                .set_content(b"BT /F1 22 Tf 72 760 Td (Contrat modifie) Tj ET".to_vec());
        });
        let attendu = Modifications {
            pages: true,
            ..Default::default()
        };
        assert_eq!(changements(&modifiee), attendu);

        // Des données de validation des signatures (DSS) : rien d'inquiétant.
        let validee = revision(&original, |suite| {
            let racine = suite
                .get_prev_documents()
                .trailer
                .get(b"Root")
                .unwrap()
                .as_reference()
                .unwrap();
            suite.opt_clone_object_to_new_document(racine).unwrap();
            let dss = suite
                .new_document
                .add_object(dictionary! { "Certs" => Vec::<Object>::new() });
            let catalogue = suite.new_document.get_dictionary_mut(racine).unwrap();
            catalogue.set("DSS", dss);
        });
        let attendu = Modifications {
            signatures: true,
            ..Default::default()
        };
        assert_eq!(changements(&validee), attendu);

        // Une action à l'ouverture : autre chose.
        let action = revision(&original, |suite| {
            let racine = suite
                .get_prev_documents()
                .trailer
                .get(b"Root")
                .unwrap()
                .as_reference()
                .unwrap();
            suite.opt_clone_object_to_new_document(racine).unwrap();
            let catalogue = suite.new_document.get_dictionary_mut(racine).unwrap();
            catalogue.set(
                "OpenAction",
                dictionary! { "S" => "URI", "URI" => Object::string_literal("https://exemple.invalid") },
            );
        });
        assert!(changements(&action).autres);

        // La version signée, telle qu'elle était.
        assert_eq!(
            version_signee(&modifiee, nom, fin),
            Some((1, original.clone()))
        );
        assert_eq!(version_signee(&modifiee, nom, fin - 1), None);
    }

    #[test]
    fn reconnait_une_certification_et_un_horodatage_du_document() {
        let certifie = pdf_signe(&PdfSigne {
            certification: Some(2),
            ..options(&identites().signataire, "adbe.pkcs7.detached")
        });
        let signature = &signatures(&certifie, "Contrat.pdf")[0];
        assert_eq!(signature.sorte, Sorte::Certification);
        assert_eq!(signature.permissions, Some(2));
        assert_eq!(signature.integrite, Integrite::Intacte);

        let horodate = pdf_signe(&PdfSigne {
            horodatage_du_document: true,
            ..options(&identites().horodateur, "")
        });
        let signature = &signatures(&horodate, "Contrat.pdf")[0];
        assert_eq!(signature.sorte, Sorte::Horodatage);
        assert_eq!(signature.integrite, Integrite::Intacte);
        assert_eq!(signature.signataire.as_deref(), Some("Horodatage d'essai"));
        assert!(signature.date_horodatee);
        assert_eq!(signature.date.as_deref(), Some("2026-11-26T08:12:00Z"));
    }

    #[test]
    fn enregistre_un_pdf_signe_annote_sans_rompre_sa_signature() {
        let original = contrat();
        let chemin = fichier_temporaire("signatures", "Contrat signé.pdf", &original);
        let sortie = chemin.replace("Contrat signé.pdf", "Contrat signé - annoté.pdf");
        let mut png = Vec::new();
        image::RgbImage::from_pixel(40, 20, image::Rgb([20, 60, 160]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let cachet = fichier_temporaire("signatures", "cachet.png", &png);
        let zone = |x, y| Zone {
            x,
            y,
            largeur: 160.0,
            hauteur: 60.0,
        };
        // Une note, et ce qui irait sinon dans la page : signature dessinée, texte, image.
        let ajouts = [
            Annotation::Note {
                page: 0,
                x: 60.0,
                y: 60.0,
                texte: "Vu".into(),
            },
            Annotation::Signature {
                page: 0,
                zone: zone(60.0, 300.0),
                trace: Trace {
                    proportion: 160.0 / 60.0,
                    traits: vec![vec![Point { x: 0.1, y: 0.5 }, Point { x: 0.9, y: 0.4 }]],
                    epaisseur: 0.05,
                },
                teinte: Teinte::Marine,
                initiales: false,
            },
            Annotation::Texte {
                page: 0,
                x: 60.0,
                y: 380.0,
                texte: "Bon pour accord".into(),
                taille: 12.0,
                teinte: Teinte::Noir,
            },
            Annotation::Image {
                page: 0,
                zone: zone(60.0, 420.0),
                chemin: cachet,
            },
        ];
        let enregistrer = |ajouts: &[Annotation], sortie: &str| {
            crate::operations::annoter_pdf(
                &chemin,
                ajouts,
                sortie,
                false,
                Some(&Retouches::default()),
            )
            .unwrap();
            let annote = std::fs::read(sortie).unwrap();
            assert!(
                annote.starts_with(&original),
                "la version signée reste telle quelle"
            );
            signatures(&annote, "Contrat signé - annoté.pdf").remove(0)
        };
        let signature = enregistrer(&ajouts, &sortie);
        assert_eq!(signature.integrite, Integrite::Intacte);
        let attendu = Modifications {
            commentaires: true,
            ..Default::default()
        };
        assert_eq!(signature.modifications, attendu);
        // Le correcteur blanc, lui, change la page : la signature le dit.
        let blanc = [Annotation::Blanc {
            page: 0,
            zone: zone(60.0, 60.0),
        }];
        let signature = enregistrer(&blanc, &sortie);
        assert_eq!(signature.integrite, Integrite::Intacte);
        assert!(signature.modifications.pages);
        assert!(!a_des_signatures(
            &pdf::charger(&crate::essais::pdf_herite(1, "x"), "x").unwrap(),
            &original
        ));
    }

    #[test]
    fn lit_les_dates_et_l_hexadecimal_du_pdf() {
        assert_eq!(
            date_pdf("D:20261126091200+01'00'").map(cms::iso).as_deref(),
            Some("2026-11-26T08:12:00Z")
        );
        assert_eq!(
            date_pdf("D:2026").map(cms::iso).as_deref(),
            Some("2026-01-01T00:00:00Z")
        );
        assert_eq!(date_pdf("hier"), None);
        assert_eq!(hexadecimal(b"0A b\n1"), Some(vec![0x0a, 0xb1]));
        assert_eq!(hexadecimal(b"F"), Some(vec![0xf0]));
        assert_eq!(hexadecimal(b"0G"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn le_systeme_ne_reconnait_pas_une_autorite_d_essai() {
        let essais = identites();
        let chaine = [
            essais.signataire.certificat.as_slice(),
            essais.autorite.certificat.as_slice(),
        ];
        assert_eq!(
            confiance::juger(&chaine, Some(date())),
            Confiance::NonReconnue
        );
        assert_eq!(confiance::juger(&[], None), Confiance::Inconnue);
    }
}
