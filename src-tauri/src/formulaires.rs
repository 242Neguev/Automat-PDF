//! Formulaires PDF (0.7.0) : les champs d'un PDF (AcroForm et leurs annotations « Widget »), lus
//! pour le lecteur, remplis, puis, au choix, aplatis.
//!
//! - **Lire** : nom complet, sorte (texte, case, bouton radio, liste), valeur, indicateurs, et la
//!   place de chaque widget sur la page affichée (comme les commentaires).
//! - **Remplir** : la valeur et son apparence, dessinée ici (police standard, codage WinAnsi),
//!   pour que le champ rempli s'affiche pareil dans tous les lecteurs de PDF. La version XFA d'un
//!   formulaire hybride est retirée : sans elle, chaque lecteur montre les valeurs écrites.
//! - **Aplatir** : le dessin de chaque champ rejoint le contenu de sa page ; le PDF n'est plus un
//!   formulaire.
//!
//! Le remplissage se fait sur la copie en mémoire du PDF (`memoire::retoucher`) : le lecteur
//! montre le champ rempli, l'impression et le PDF enregistré le gardent, l'original n'est jamais
//! modifié. Pas de JavaScript : les calculs d'un formulaire (totaux, contrôles) ne se font pas, et
//! le lecteur le signale.
//!
//! hayro 0.7 ne choisit pas l'apparence d'une case selon son état (/AS) : `pour_l_affichage`
//! prépare, pour le rendu seulement, une copie où chaque case a l'apparence de son état.

use std::collections::{HashMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};
use serde::{Deserialize, Serialize};

use crate::annotations::texte_pdf;
use crate::erreur::Erreur;
use crate::geometrie::{Cadre, Matrice, boite, rectangle, resoudre};
use crate::police::{Famille, chaine, police_standard};

/// Champs lus au plus, et profondeur de leur arbre : un PDF piégé reste lisible.
const CHAMPS_MAX: usize = 5000;
const PROFONDEUR_MAX: usize = 32;
/// Indicateurs d'un champ (/Ff), d'après la norme (ISO 32000, 12.7).
const LECTURE_SEULE: i64 = 1;
const OBLIGATOIRE: i64 = 1 << 1;
const MULTILIGNE: i64 = 1 << 12;
const SECRET: i64 = 1 << 13;
const SANS_DECOCHER: i64 = 1 << 14;
const RADIO: i64 = 1 << 15;
const POUSSOIR: i64 = 1 << 16;
const DEROULANTE: i64 = 1 << 17;
const SAISIE_LIBRE: i64 = 1 << 18;
const CHOIX_MULTIPLE: i64 = 1 << 21;
const PEIGNE: i64 = 1 << 24;
/// Indicateurs d'une annotation (/F) : cachée ; invisible à l'écran.
const CACHEE: i64 = 1 << 1;
const SANS_VUE: i64 = 1 << 5;
/// Marge intérieure d'un champ de texte, en points (celle des lecteurs de PDF courants).
const MARGE: f64 = 2.0;
/// Interligne d'un champ de plusieurs lignes, et hauteur d'une ligne de liste, en tailles.
const INTERLIGNE: f64 = 1.15;
/// Taille du texte d'un champ « automatique » de plusieurs lignes, au plus ; et au moins.
const TAILLE_AUTO_MAX: f64 = 12.0;
const TAILLE_MIN: f64 = 4.0;
/// Fond des options choisies d'une liste (bleu clair, comme dans les lecteurs de PDF courants).
const FOND_CHOISI: &str = "0.6 0.75 0.85 rg";

/// Sorte d'un champ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Sorte {
    Texte,
    Case,
    Radio,
    /// Liste (une ou plusieurs options visibles à la fois).
    Liste,
    /// Liste déroulante.
    Combo,
    /// Bouton poussoir (« Imprimer », « Envoyer ») : rien à remplir.
    Bouton,
    Signature,
}

/// Valeur d'un champ : un texte ; l'état d'une case ou d'un bouton radio (« Off » : décoché) ;
/// une option (ou plusieurs) d'une liste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Valeur {
    Texte(String),
    Liste(Vec<String>),
}

impl Valeur {
    fn textes(&self) -> Vec<&str> {
        match self {
            Valeur::Texte(texte) => vec![texte.as_str()],
            Valeur::Liste(liste) => liste.iter().map(String::as_str).collect(),
        }
    }
}

/// Une option d'une liste : la valeur écrite dans le PDF, et le texte affiché.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OptionChoix {
    pub valeur: String,
    pub libelle: String,
}

/// Un widget du champ (sa place sur une page), en points de la page affichée depuis son coin en
/// haut à gauche, comme les mots et les annotations du lecteur.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    /// À partir de 0.
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub largeur: f64,
    pub hauteur: f64,
    /// État « coché » de ce widget (case, bouton radio) : le nom de son apparence (« Yes », « 1 »…).
    pub etat: Option<String>,
    /// Taille du texte demandée par le PDF (0 : automatique).
    pub taille: f64,
    /// 0 : à gauche ; 1 : centré ; 2 : à droite.
    pub alignement: u8,
}

/// Un champ du formulaire.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Champ {
    /// Nom complet (« Identité.Nom ») : son repère.
    pub nom: String,
    /// Infobulle donnée par le PDF (/TU).
    pub libelle: Option<String>,
    pub sorte: Sorte,
    pub valeur: Valeur,
    pub lecture_seule: bool,
    pub obligatoire: bool,
    pub multiligne: bool,
    /// Mot de passe : le texte s'affiche masqué.
    pub secret: bool,
    pub longueur_max: Option<usize>,
    /// Une case par caractère (/MaxLen cases de même largeur).
    pub peigne: bool,
    pub options: Vec<OptionChoix>,
    pub multiple: bool,
    /// Liste déroulante où l'on peut aussi écrire sa propre valeur.
    pub saisie_libre: bool,
    /// Un bouton radio coché ne se décoche pas d'un clic.
    pub sans_decocher: bool,
    /// Champ de signature déjà signé (signature numérique) : on n'y pose pas de signature dessinée.
    pub signe: bool,
    pub widgets: Vec<Widget>,
}

/// Les champs d'un PDF, pour le lecteur.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Formulaire {
    pub champs: Vec<Champ>,
    /// Le formulaire a aussi une version XFA (format abandonné) ; seulement elle : rien à remplir ici.
    pub xfa: bool,
    pub xfa_seul: bool,
    /// Des calculs ou des contrôles (JavaScript) qu'Automat-PDF ne fait pas.
    pub calculs: bool,
}

// ------------------------------------------------------------------------------- Lecture

/// Ce qu'un champ hérite de ses parents dans l'arbre des champs.
#[derive(Debug, Clone, Default)]
struct Herite {
    sorte: Option<Vec<u8>>,
    drapeaux: Option<i64>,
    valeur: Option<Object>,
    apparence: Option<String>,
    alignement: Option<i64>,
    longueur_max: Option<i64>,
}

/// Un champ terminal (celui qui porte la valeur) et ses widgets.
#[derive(Debug, Clone)]
struct Noeud {
    id: ObjectId,
    nom: String,
    herite: Herite,
    widgets: Vec<ObjectId>,
}

impl Noeud {
    fn drapeaux(&self) -> i64 {
        self.herite.drapeaux.unwrap_or(0)
    }

    fn sorte(&self) -> Option<Sorte> {
        let drapeaux = self.drapeaux();
        Some(match self.herite.sorte.as_deref()? {
            b"Tx" => Sorte::Texte,
            b"Btn" if drapeaux & POUSSOIR != 0 => Sorte::Bouton,
            b"Btn" if drapeaux & RADIO != 0 => Sorte::Radio,
            b"Btn" => Sorte::Case,
            b"Ch" if drapeaux & DEROULANTE != 0 => Sorte::Combo,
            b"Ch" => Sorte::Liste,
            b"Sig" => Sorte::Signature,
            _ => return None,
        })
    }
}

fn entier(document: &Document, dictionnaire: &Dictionary, cle: &[u8]) -> Option<i64> {
    dictionnaire
        .get(cle)
        .ok()
        .map(|o| resoudre(document, o))
        .and_then(|o| o.as_i64().ok())
}

fn nom(document: &Document, dictionnaire: &Dictionary, cle: &[u8]) -> Option<Vec<u8>> {
    dictionnaire
        .get(cle)
        .ok()
        .map(|o| resoudre(document, o))
        .and_then(|o| o.as_name().ok())
        .map(<[u8]>::to_vec)
}

fn texte(document: &Document, dictionnaire: &Dictionary, cle: &[u8]) -> Option<String> {
    dictionnaire
        .get(cle)
        .ok()
        .map(|o| resoudre(document, o))
        .and_then(|o| lopdf::decode_text_string(o).ok())
}

/// Chaîne d'octets d'une entrée (/DA), lue telle quelle.
fn chaine_brute(document: &Document, dictionnaire: &Dictionary, cle: &[u8]) -> Option<String> {
    match dictionnaire.get(cle).ok().map(|o| resoudre(document, o)) {
        Some(Object::String(octets, _)) => Some(String::from_utf8_lossy(octets).into_owned()),
        _ => None,
    }
}

fn dictionnaire_de<'a>(document: &'a Document, objet: &'a Object) -> Option<&'a Dictionary> {
    resoudre(document, objet).as_dict().ok()
}

/// Le dictionnaire du formulaire (/AcroForm du catalogue).
fn acroform(document: &Document) -> Option<&Dictionary> {
    dictionnaire_de(document, document.catalog().ok()?.get(b"AcroForm").ok()?)
}

/// Les champs terminaux du formulaire, dans l'ordre de l'arbre ; et s'il a des calculs.
fn noeuds(document: &Document) -> (Vec<Noeud>, bool) {
    let mut sortie = Vec::new();
    let mut calculs = false;
    let Some(formulaire) = acroform(document) else {
        return (sortie, calculs);
    };
    calculs |= formulaire
        .get(b"CO")
        .ok()
        .map(|o| resoudre(document, o))
        .and_then(|o| o.as_array().ok())
        .is_some_and(|ordre| !ordre.is_empty());
    let racines = formulaire
        .get(b"Fields")
        .ok()
        .map(|o| resoudre(document, o))
        .and_then(|o| o.as_array().ok())
        .cloned()
        .unwrap_or_default();
    let mut vus = HashSet::new();
    for racine in &racines {
        visiter(
            document,
            racine,
            None,
            &Herite::default(),
            0,
            &mut vus,
            &mut sortie,
            &mut calculs,
        );
    }
    (sortie, calculs)
}

#[allow(clippy::too_many_arguments)]
fn visiter(
    document: &Document,
    objet: &Object,
    parent: Option<&str>,
    herite: &Herite,
    profondeur: usize,
    vus: &mut HashSet<ObjectId>,
    sortie: &mut Vec<Noeud>,
    calculs: &mut bool,
) {
    let Object::Reference(id) = objet else {
        return;
    };
    if profondeur > PROFONDEUR_MAX || sortie.len() >= CHAMPS_MAX || !vus.insert(*id) {
        return;
    }
    let Ok(champ) = document.get_dictionary(*id) else {
        return;
    };
    let partiel = texte(document, champ, b"T");
    let nom_complet = match (parent, partiel) {
        (Some(parent), Some(partiel)) => format!("{parent}.{partiel}"),
        (None, Some(partiel)) => partiel,
        (Some(parent), None) => parent.to_string(),
        (None, None) => String::new(),
    };
    let herite = Herite {
        sorte: nom(document, champ, b"FT").or_else(|| herite.sorte.clone()),
        drapeaux: entier(document, champ, b"Ff").or(herite.drapeaux),
        valeur: champ
            .get(b"V")
            .ok()
            .map(|o| resoudre(document, o).clone())
            .or_else(|| herite.valeur.clone()),
        apparence: chaine_brute(document, champ, b"DA").or_else(|| herite.apparence.clone()),
        alignement: entier(document, champ, b"Q").or(herite.alignement),
        longueur_max: entier(document, champ, b"MaxLen").or(herite.longueur_max),
    };
    *calculs |= a_des_scripts(document, champ);
    let enfants = champ
        .get(b"Kids")
        .ok()
        .map(|o| resoudre(document, o))
        .and_then(|o| o.as_array().ok())
        .cloned()
        .unwrap_or_default();
    let mut widgets = Vec::new();
    if enfants.is_empty() {
        // Champ et widget réunis dans un même objet.
        if champ.has(b"Rect") {
            widgets.push(*id);
        }
    } else {
        for enfant in &enfants {
            let Object::Reference(id_enfant) = enfant else {
                continue;
            };
            let Ok(dictionnaire) = document.get_dictionary(*id_enfant) else {
                continue;
            };
            if dictionnaire.has(b"T") || dictionnaire.has(b"Kids") {
                visiter(
                    document,
                    enfant,
                    Some(&nom_complet),
                    &herite,
                    profondeur + 1,
                    vus,
                    sortie,
                    calculs,
                );
            } else if vus.insert(*id_enfant) {
                *calculs |= a_des_scripts(document, dictionnaire);
                widgets.push(*id_enfant);
            }
        }
    }
    if !widgets.is_empty() && herite.sorte.is_some() && !nom_complet.is_empty() {
        sortie.push(Noeud {
            id: *id,
            nom: nom_complet,
            herite,
            widgets,
        });
    }
}

/// Actions de calcul, de format ou de contrôle (JavaScript) d'un champ ou d'un widget.
fn a_des_scripts(document: &Document, dictionnaire: &Dictionary) -> bool {
    dictionnaire
        .get(b"AA")
        .ok()
        .and_then(|o| dictionnaire_de(document, o))
        .is_some_and(|actions| {
            [b"C".as_slice(), b"K", b"F", b"V"]
                .iter()
                .any(|cle| actions.has(cle))
        })
}

/// La page (à partir de 0) de chaque annotation présente dans le tableau /Annots d'une page.
fn pages_des_annotations(document: &Document) -> HashMap<ObjectId, usize> {
    let mut pages = HashMap::new();
    for (rang, (_, id_page)) in document.get_pages().into_iter().enumerate() {
        let Ok(page) = document.get_dictionary(id_page) else {
            continue;
        };
        let Some(annotations) = page
            .get(b"Annots")
            .ok()
            .map(|o| resoudre(document, o))
            .and_then(|o| o.as_array().ok())
        else {
            continue;
        };
        for annotation in annotations {
            if let Object::Reference(id) = annotation {
                pages.entry(*id).or_insert(rang);
            }
        }
    }
    pages
}

/// La valeur d'un champ, d'après sa sorte.
fn valeur_de(document: &Document, noeud: &Noeud, sorte: Sorte) -> Valeur {
    let brute = noeud.herite.valeur.as_ref();
    match sorte {
        Sorte::Case | Sorte::Radio => {
            let etat = brute
                .and_then(|o| o.as_name().ok())
                .map(|n| String::from_utf8_lossy(n).into_owned())
                .or_else(|| {
                    // Sans /V : l'état du premier widget coché.
                    noeud.widgets.iter().find_map(|&id| {
                        let widget = document.get_dictionary(id).ok()?;
                        let etat = nom(document, widget, b"AS")?;
                        (etat != b"Off").then(|| String::from_utf8_lossy(&etat).into_owned())
                    })
                })
                .unwrap_or_else(|| "Off".into());
            Valeur::Texte(etat)
        }
        Sorte::Liste | Sorte::Combo => match brute {
            Some(Object::Array(liste)) => Valeur::Liste(
                liste
                    .iter()
                    .filter_map(|o| lopdf::decode_text_string(resoudre(document, o)).ok())
                    .collect(),
            ),
            Some(objet) => Valeur::Texte(lopdf::decode_text_string(objet).unwrap_or_default()),
            None => Valeur::Texte(String::new()),
        },
        _ => Valeur::Texte(
            brute
                .and_then(|o| match o {
                    Object::Stream(flux) => flux
                        .decompressed_content()
                        .ok()
                        .map(|octets| String::from_utf8_lossy(&octets).into_owned()),
                    autre => lopdf::decode_text_string(autre).ok(),
                })
                .unwrap_or_default(),
        ),
    }
}

/// Les options d'une liste (/Opt) : un texte, ou une paire [valeur, texte affiché].
fn options_de(document: &Document, champ: &Dictionary) -> Vec<OptionChoix> {
    champ
        .get(b"Opt")
        .ok()
        .map(|o| resoudre(document, o))
        .and_then(|o| o.as_array().ok())
        .map(|options| {
            options
                .iter()
                .filter_map(|option| match resoudre(document, option) {
                    Object::Array(paire) if paire.len() >= 2 => {
                        let valeur =
                            lopdf::decode_text_string(resoudre(document, &paire[0])).ok()?;
                        let libelle =
                            lopdf::decode_text_string(resoudre(document, &paire[1])).ok()?;
                        Some(OptionChoix { valeur, libelle })
                    }
                    autre => {
                        let valeur = lopdf::decode_text_string(autre).ok()?;
                        Some(OptionChoix {
                            libelle: valeur.clone(),
                            valeur,
                        })
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// L'état « coché » d'un widget : la clé de son apparence normale qui n'est pas « Off ».
fn etat_actif(document: &Document, widget: &Dictionary) -> Option<String> {
    let apparences = widget
        .get(b"AP")
        .ok()
        .and_then(|o| dictionnaire_de(document, o))?;
    let normale = apparences
        .get(b"N")
        .ok()
        .and_then(|o| dictionnaire_de(document, o))?;
    normale
        .iter()
        .map(|(cle, _)| cle)
        .find(|cle| cle.as_slice() != b"Off")
        .map(|cle| String::from_utf8_lossy(cle).into_owned())
}

/// Les champs du formulaire, avec la place de leurs widgets sur les pages affichées.
pub fn formulaire(document: &Document) -> Formulaire {
    let xfa = acroform(document).is_some_and(|f| f.has(b"XFA"));
    let (noeuds, calculs) = noeuds(document);
    let pages = pages_des_annotations(document);
    let ids_pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    let mut cadres: HashMap<usize, Cadre> = HashMap::new();
    let mut champs = Vec::new();
    for noeud in &noeuds {
        let (Some(sorte), Ok(dictionnaire)) = (noeud.sorte(), document.get_dictionary(noeud.id))
        else {
            continue;
        };
        let drapeaux = noeud.drapeaux();
        let mut widgets = Vec::new();
        for &id in &noeud.widgets {
            let (Some(&page), Ok(widget)) = (pages.get(&id), document.get_dictionary(id)) else {
                continue;
            };
            if entier(document, widget, b"F").unwrap_or(0) & (CACHEE | SANS_VUE) != 0 {
                continue;
            }
            let Some([x0, y0, x1, y1]) = widget
                .get(b"Rect")
                .ok()
                .and_then(|r| rectangle(document, r))
            else {
                continue;
            };
            let cadre = *cadres.entry(page).or_insert_with(|| {
                ids_pages
                    .get(page)
                    .and_then(|&p| document.get_dictionary(p).ok())
                    .map(|p| Cadre::de_page(document, p))
                    .unwrap_or(Cadre {
                        boite: [0.0, 0.0, 595.0, 842.0],
                        rotation: 0,
                    })
            });
            let (_, hauteur_page) = cadre.taille_vue();
            let [gauche, bas, droite, haut] =
                boite(cadre.vers_vue(), &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            let apparence = chaine_brute(document, widget, b"DA")
                .or_else(|| noeud.herite.apparence.clone())
                .unwrap_or_default();
            widgets.push(Widget {
                page,
                x: gauche,
                y: hauteur_page - haut,
                largeur: droite - gauche,
                hauteur: haut - bas,
                etat: matches!(sorte, Sorte::Case | Sorte::Radio)
                    .then(|| etat_actif(document, widget).unwrap_or_else(|| "Yes".into())),
                taille: taille_demandee(&apparence),
                alignement: entier(document, widget, b"Q")
                    .or(noeud.herite.alignement)
                    .unwrap_or(0)
                    .clamp(0, 2) as u8,
            });
        }
        champs.push(Champ {
            nom: noeud.nom.clone(),
            libelle: texte(document, dictionnaire, b"TU").filter(|t| !t.trim().is_empty()),
            sorte,
            valeur: valeur_de(document, noeud, sorte),
            lecture_seule: drapeaux & LECTURE_SEULE != 0,
            obligatoire: drapeaux & OBLIGATOIRE != 0,
            multiligne: sorte == Sorte::Texte && drapeaux & MULTILIGNE != 0,
            secret: sorte == Sorte::Texte && drapeaux & SECRET != 0,
            longueur_max: noeud
                .herite
                .longueur_max
                .filter(|&n| n > 0)
                .map(|n| n as usize),
            peigne: sorte == Sorte::Texte
                && drapeaux & PEIGNE != 0
                && drapeaux & (MULTILIGNE | SECRET) == 0
                && noeud.herite.longueur_max.is_some_and(|n| n > 0),
            options: options_de(document, dictionnaire),
            multiple: sorte == Sorte::Liste && drapeaux & CHOIX_MULTIPLE != 0,
            saisie_libre: sorte == Sorte::Combo && drapeaux & SAISIE_LIBRE != 0,
            sans_decocher: sorte == Sorte::Radio && drapeaux & SANS_DECOCHER != 0,
            signe: sorte == Sorte::Signature
                && noeud
                    .herite
                    .valeur
                    .as_ref()
                    .is_some_and(|v| v.as_dict().is_ok()),
            widgets,
        });
    }
    Formulaire {
        xfa_seul: xfa && champs.iter().all(|c| c.widgets.is_empty()),
        champs,
        xfa,
        calculs,
    }
}

/// Les champs de signature signés (signature numérique) : nom complet, valeur (/V, le dictionnaire
/// de la signature) et place de leurs widgets sur les pages affichées.
pub(crate) fn champs_signes(document: &Document) -> Vec<(String, Object, Vec<Widget>)> {
    let (noeuds, _) = noeuds(document);
    // `formulaire` garde, dans l'ordre, les champs d'une sorte connue dont le dictionnaire se lit.
    noeuds
        .into_iter()
        .filter(|noeud| noeud.sorte().is_some() && document.get_dictionary(noeud.id).is_ok())
        .zip(formulaire(document).champs)
        .filter(|(_, champ)| champ.sorte == Sorte::Signature && champ.signe)
        .filter_map(|(noeud, champ)| Some((champ.nom, noeud.herite.valeur?, champ.widgets)))
        .collect()
}

// ------------------------------------------------------------------------------- Apparences

/// Style du texte d'un champ, d'après son apparence par défaut (/DA, « /Helv 10 Tf 0 g »).
#[derive(Debug, Clone, PartialEq)]
struct Style {
    famille: Famille,
    gras: bool,
    italique: bool,
    /// 0 : taille automatique.
    taille: f64,
    /// Opérateur de couleur du texte (« 0 g », « 0 0 1 rg »…).
    couleur: String,
}

/// Taille du texte demandée par /DA (0 : automatique).
fn taille_demandee(apparence: &str) -> f64 {
    let jetons: Vec<&str> = apparence.split_whitespace().collect();
    jetons
        .iter()
        .rposition(|&j| j == "Tf")
        .filter(|&i| i >= 1)
        .and_then(|i| jetons[i - 1].parse::<f64>().ok())
        .filter(|t| t.is_finite() && *t >= 0.0)
        .unwrap_or(0.0)
}

/// Police standard la plus proche de celle que nomme /DA : son nom PostScript dans les ressources
/// du formulaire (/DR), ou les noms courts habituels (« Helv », « TiRo », « Cour »).
fn style(document: &Document, apparence: &str, polices: Option<&Dictionary>) -> Style {
    let jetons: Vec<&str> = apparence.split_whitespace().collect();
    let mut police = "Helv".to_string();
    let mut couleur = "0 g".to_string();
    for (i, &jeton) in jetons.iter().enumerate() {
        match jeton {
            "Tf" if i >= 2 => police = jetons[i - 2].trim_start_matches('/').to_string(),
            "g" | "rg" | "k" => {
                let n = match jeton {
                    "g" => 1,
                    "rg" => 3,
                    _ => 4,
                };
                if i >= n
                    && jetons[i - n..i]
                        .iter()
                        .all(|v| v.parse::<f64>().is_ok_and(f64::is_finite))
                {
                    couleur = format!("{} {jeton}", jetons[i - n..i].join(" "));
                }
            }
            _ => {}
        }
    }
    let base = polices
        .and_then(|p| p.get(police.as_bytes()).ok())
        .and_then(|o| dictionnaire_de(document, o))
        .and_then(|f| nom(document, f, b"BaseFont"))
        .map(|n| String::from_utf8_lossy(&n).into_owned())
        .unwrap_or_else(|| {
            match police.as_str() {
                "HeBo" => "Helvetica-Bold",
                "TiRo" => "Times-Roman",
                "TiBo" => "Times-Bold",
                "TiIt" => "Times-Italic",
                "Cour" => "Courier",
                "CoBo" => "Courier-Bold",
                _ => "Helvetica",
            }
            .to_string()
        });
    Style {
        famille: Famille::de(&base),
        gras: base.contains("Bold") || base.contains("Bd"),
        italique: base.contains("Italic") || base.contains("Oblique"),
        taille: taille_demandee(apparence),
        couleur,
    }
}

/// Fond, bordure et rotation d'un widget (/MK, /BS).
#[derive(Debug, Clone, Default, PartialEq)]
struct Decor {
    /// Opérateur de remplissage du fond.
    fond: Option<String>,
    /// Opérateur de trait de la bordure, et son épaisseur.
    bord: Option<(String, f64)>,
    /// Bordure soulignée seulement (style « U »).
    souligne: bool,
    /// 0, 90, 180 ou 270 degrés, dans le sens inverse des aiguilles d'une montre.
    rotation: i64,
}

/// Opérateur de couleur d'un tableau de 1, 3 ou 4 composantes (gris, RVB, CMJN).
fn operateur_couleur(document: &Document, objet: &Object, trait_: bool) -> Option<String> {
    let valeurs: Vec<f64> = resoudre(document, objet)
        .as_array()
        .ok()?
        .iter()
        .map(|v| resoudre(document, v).as_float().ok().map(f64::from))
        .collect::<Option<_>>()?;
    let nombres = valeurs
        .iter()
        .map(|v| format!("{:.3}", v.clamp(0.0, 1.0)))
        .collect::<Vec<_>>()
        .join(" ");
    let operateur = match (valeurs.len(), trait_) {
        (1, false) => "g",
        (1, true) => "G",
        (3, false) => "rg",
        (3, true) => "RG",
        (4, false) => "k",
        (4, true) => "K",
        _ => return None,
    };
    Some(format!("{nombres} {operateur}"))
}

fn decor(document: &Document, widget: &Dictionary) -> Decor {
    let caracteristiques = widget
        .get(b"MK")
        .ok()
        .and_then(|o| dictionnaire_de(document, o));
    let bordure = widget
        .get(b"BS")
        .ok()
        .and_then(|o| dictionnaire_de(document, o));
    let epaisseur = bordure
        .and_then(|b| b.get(b"W").ok())
        .map(|o| resoudre(document, o))
        .and_then(|o| o.as_float().ok())
        .map_or(1.0, f64::from)
        .clamp(0.0, 12.0);
    let style_bordure = bordure.and_then(|b| nom(document, b, b"S"));
    Decor {
        fond: caracteristiques
            .and_then(|mk| mk.get(b"BG").ok())
            .and_then(|o| operateur_couleur(document, o, false)),
        bord: caracteristiques
            .and_then(|mk| mk.get(b"BC").ok())
            .and_then(|o| operateur_couleur(document, o, true))
            .filter(|_| epaisseur > 0.0)
            .map(|operateur| (operateur, epaisseur)),
        souligne: style_bordure.as_deref() == Some(b"U".as_slice()),
        rotation: caracteristiques
            .and_then(|mk| entier(document, mk, b"R"))
            .unwrap_or(0)
            .rem_euclid(360)
            / 90
            * 90,
    }
}

impl Decor {
    /// Fond et bordure, dans une boîte de `l` × `h` points.
    fn dessin(&self, l: f64, h: f64) -> String {
        let mut dessin = String::new();
        if let Some(fond) = &self.fond {
            dessin.push_str(&format!("{fond} 0 0 {l:.3} {h:.3} re f\n"));
        }
        if let Some((trait_, epaisseur)) = &self.bord {
            let demi = epaisseur / 2.0;
            if self.souligne {
                dessin.push_str(&format!(
                    "{trait_} {epaisseur:.3} w 0 {demi:.3} m {l:.3} {demi:.3} l S\n"
                ));
            } else {
                dessin.push_str(&format!(
                    "{trait_} {epaisseur:.3} w {demi:.3} {demi:.3} {:.3} {:.3} re S\n",
                    l - epaisseur,
                    h - epaisseur
                ));
            }
        }
        dessin
    }

    /// Épaisseur de la bordure dessinée (0 sans bordure).
    fn epaisseur(&self) -> f64 {
        self.bord.as_ref().map_or(0.0, |(_, e)| *e)
    }

    /// Boîte où dessiner, rotation du widget comprise, et la matrice de l'apparence.
    fn boite(&self, largeur: f64, hauteur: f64) -> (f64, f64, Option<[f64; 6]>) {
        match self.rotation {
            90 => (hauteur, largeur, Some([0.0, 1.0, -1.0, 0.0, 0.0, 0.0])),
            180 => (largeur, hauteur, Some([-1.0, 0.0, 0.0, -1.0, 0.0, 0.0])),
            270 => (hauteur, largeur, Some([0.0, -1.0, 1.0, 0.0, 0.0, 0.0])),
            _ => (largeur, hauteur, None),
        }
    }
}

/// Le texte coupé en lignes d'au plus `largeur` points (mots entiers, sauf un mot trop long).
fn lignes(texte: &str, style: &Style, taille: f64, largeur: f64) -> Vec<String> {
    let mesure = |t: &str| style.famille.largeur(t, style.gras, taille);
    let mut sortie = Vec::new();
    for paragraphe in texte.split('\n') {
        let mut ligne = String::new();
        for mot in paragraphe.split(' ') {
            let essai = if ligne.is_empty() {
                mot.to_string()
            } else {
                format!("{ligne} {mot}")
            };
            if mesure(&essai) <= largeur || ligne.is_empty() && mesure(mot) <= largeur {
                ligne = essai;
                continue;
            }
            if !ligne.is_empty() {
                sortie.push(std::mem::take(&mut ligne));
            }
            // Un mot plus long que la ligne : coupé où il le faut.
            for c in mot.chars() {
                let essai = format!("{ligne}{c}");
                if mesure(&essai) > largeur && !ligne.is_empty() {
                    sortie.push(std::mem::take(&mut ligne));
                    ligne.push(c);
                } else {
                    ligne = essai;
                }
            }
        }
        sortie.push(ligne);
    }
    sortie
}

/// Position horizontale d'un texte de largeur `mesure` dans une boîte de `largeur` points.
fn x_aligne(alignement: u8, mesure: f64, largeur: f64, marge: f64) -> f64 {
    match alignement {
        1 => (largeur - mesure) / 2.0,
        2 => largeur - marge - mesure,
        _ => marge,
    }
}

/// Ligne de base d'une ligne de texte de `taille` centrée dans `hauteur` (Helvetica : hampes 0,718,
/// jambages 0,207).
fn ligne_de_base(hauteur: f64, taille: f64) -> f64 {
    (hauteur - taille * 0.925) / 2.0 + taille * 0.207
}

/// Comment écrire le texte d'un champ.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mise {
    Ligne,
    Lignes,
    /// Une case par caractère.
    Peigne(usize),
}

/// L'apparence d'un champ de texte (ou d'une liste déroulante et de son texte affiché).
fn flux_texte(
    largeur: f64,
    hauteur: f64,
    style: &Style,
    alignement: u8,
    texte: &str,
    mise: Mise,
    decor: &Decor,
) -> Stream {
    let (l, h, matrice) = decor.boite(largeur, hauteur);
    let bord = decor.epaisseur();
    let marge = MARGE.max(bord * 2.0);
    let mut contenu = String::from("/Tx BMC\nq\n");
    contenu.push_str(&decor.dessin(l, h));
    contenu.push_str(&format!(
        "{bord:.3} {bord:.3} {:.3} {:.3} re W n\n",
        (l - 2.0 * bord).max(0.0),
        (h - 2.0 * bord).max(0.0)
    ));
    if !texte.is_empty() {
        let mesure = |t: &str, taille: f64| style.famille.largeur(t, style.gras, taille);
        let disponible = (l - 2.0 * marge).max(1.0);
        contenu.push_str("BT\n");
        match mise {
            Mise::Ligne => {
                let mut taille = if style.taille > 0.0 {
                    style.taille
                } else {
                    ((h - 2.0 * bord - 2.0) * 0.8).max(TAILLE_MIN)
                };
                if style.taille <= 0.0 {
                    let largeur_texte = mesure(texte, taille);
                    if largeur_texte > disponible {
                        taille = (taille * disponible / largeur_texte).max(TAILLE_MIN);
                    }
                }
                let x = x_aligne(alignement, mesure(texte, taille), l, marge);
                contenu.push_str(&format!(
                    "/F0 {taille:.2} Tf {}\n{x:.3} {:.3} Td {} Tj\n",
                    style.couleur,
                    ligne_de_base(h, taille),
                    chaine(texte)
                ));
            }
            Mise::Peigne(cases) => {
                let taille = if style.taille > 0.0 {
                    style.taille
                } else {
                    ((h - 2.0 * bord - 2.0) * 0.8).max(TAILLE_MIN)
                };
                let case = l / cases.max(1) as f64;
                contenu.push_str(&format!("/F0 {taille:.2} Tf {}\n", style.couleur));
                let y = ligne_de_base(h, taille);
                for (rang, c) in texte.chars().take(cases).enumerate() {
                    let lettre = c.to_string();
                    let x = rang as f64 * case + (case - mesure(&lettre, taille)) / 2.0;
                    contenu.push_str(&format!(
                        "1 0 0 1 {x:.3} {y:.3} Tm {} Tj\n",
                        chaine(&lettre)
                    ));
                }
            }
            Mise::Lignes => {
                let hauteur_utile = (h - 2.0 * marge).max(1.0);
                let mut taille = if style.taille > 0.0 {
                    style.taille
                } else {
                    TAILLE_AUTO_MAX
                };
                let mut coupees = lignes(texte, style, taille, disponible);
                if style.taille <= 0.0 {
                    while taille > TAILLE_MIN
                        && coupees.len() as f64 * taille * INTERLIGNE > hauteur_utile
                    {
                        taille = (taille - 0.5).max(TAILLE_MIN);
                        coupees = lignes(texte, style, taille, disponible);
                    }
                }
                contenu.push_str(&format!("/F0 {taille:.2} Tf {}\n", style.couleur));
                let mut y = h - marge - taille * 0.78;
                for ligne in &coupees {
                    let x = x_aligne(alignement, mesure(ligne, taille), l, marge);
                    contenu.push_str(&format!("1 0 0 1 {x:.3} {y:.3} Tm {} Tj\n", chaine(ligne)));
                    y -= taille * INTERLIGNE;
                }
            }
        }
        contenu.push_str("ET\n");
    }
    contenu.push_str("Q\nEMC\n");
    let mut dictionnaire = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Form",
        "BBox" => vec![0.into(), 0.into(), Object::Real(l as f32), Object::Real(h as f32)],
        "Resources" => dictionary! {
            "Font" => dictionary! { "F0" => police_standard(style.famille, style.gras, style.italique) },
        },
    };
    if let Some(matrice) = matrice {
        dictionnaire.set(
            "Matrix",
            matrice
                .iter()
                .map(|&v| Object::Real(v as f32))
                .collect::<Vec<_>>(),
        );
    }
    Stream::new(dictionnaire, contenu.into_bytes())
}

/// L'apparence d'une liste (non déroulante) : ses options, les choisies sur fond bleu clair.
fn flux_liste(
    largeur: f64,
    hauteur: f64,
    style: &Style,
    options: &[OptionChoix],
    choisies: &[usize],
    premiere: usize,
    decor: &Decor,
) -> Stream {
    let (l, h, matrice) = decor.boite(largeur, hauteur);
    let bord = decor.epaisseur();
    let taille = if style.taille > 0.0 {
        style.taille
    } else {
        TAILLE_AUTO_MAX
    };
    let pas = taille * INTERLIGNE;
    let mut contenu = String::from("/Tx BMC\nq\n");
    contenu.push_str(&decor.dessin(l, h));
    contenu.push_str(&format!(
        "{bord:.3} {bord:.3} {:.3} {:.3} re W n\n",
        (l - 2.0 * bord).max(0.0),
        (h - 2.0 * bord).max(0.0)
    ));
    let mut haut = h - bord;
    for (rang, option) in options.iter().enumerate().skip(premiere) {
        if haut <= bord {
            break;
        }
        if choisies.contains(&rang) {
            contenu.push_str(&format!(
                "{FOND_CHOISI} {bord:.3} {:.3} {:.3} {pas:.3} re f\n",
                haut - pas,
                l - 2.0 * bord
            ));
        }
        contenu.push_str(&format!(
            "BT /F0 {taille:.2} Tf {} {:.3} {:.3} Td {} Tj ET\n",
            style.couleur,
            MARGE.max(bord * 2.0),
            haut - pas + ligne_de_base(pas, taille),
            chaine(&option.libelle)
        ));
        haut -= pas;
    }
    contenu.push_str("Q\nEMC\n");
    let mut dictionnaire = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Form",
        "BBox" => vec![0.into(), 0.into(), Object::Real(l as f32), Object::Real(h as f32)],
        "Resources" => dictionary! {
            "Font" => dictionary! { "F0" => police_standard(style.famille, style.gras, style.italique) },
        },
    };
    if let Some(matrice) = matrice {
        dictionnaire.set(
            "Matrix",
            matrice
                .iter()
                .map(|&v| Object::Real(v as f32))
                .collect::<Vec<_>>(),
        );
    }
    Stream::new(dictionnaire, contenu.into_bytes())
}

/// Apparences d'une case ou d'un bouton radio qui n'en a pas : décoché (son décor), coché (le
/// signe de /MK /CA dans la police ZapfDingbats : coche, rond…).
fn flux_case(
    document: &Document,
    widget: &Dictionary,
    largeur: f64,
    hauteur: f64,
    radio: bool,
) -> (Stream, Stream) {
    let decor = decor(document, widget);
    let (l, h, _) = decor.boite(largeur, hauteur);
    let signe = widget
        .get(b"MK")
        .ok()
        .and_then(|o| dictionnaire_de(document, o))
        .and_then(|mk| texte(document, mk, b"CA"))
        .and_then(|t| t.chars().next())
        .filter(char::is_ascii_graphic)
        .unwrap_or(if radio { 'l' } else { '4' });
    let couleur = chaine_brute(document, widget, b"DA")
        .map(|da| style(document, &da, None).couleur)
        .unwrap_or_else(|| "0 g".into());
    let cote = l.min(h);
    let taille = cote * 0.8;
    // ZapfDingbats : largeur de la plupart des signes, à peu près 0,8 de la taille.
    let x = (l - taille * 0.8) / 2.0;
    let y = (h - taille * 0.7) / 2.0;
    let fond = decor.dessin(l, h);
    let coche =
        format!("q\n{fond}BT /ZaDb {taille:.2} Tf {couleur} {x:.3} {y:.3} Td ({signe}) Tj ET\nQ\n");
    let flux = |contenu: String| {
        Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Form",
                "BBox" => vec![0.into(), 0.into(), Object::Real(l as f32), Object::Real(h as f32)],
                "Resources" => dictionary! {
                    "Font" => dictionary! {
                        "ZaDb" => dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "ZapfDingbats" },
                    },
                },
            },
            contenu.into_bytes(),
        )
    };
    (flux(coche), flux(format!("q\n{fond}Q\n")))
}

/// Les polices du formulaire (/DR /Font), pour lire /DA.
fn polices_du_formulaire(document: &Document) -> Option<Dictionary> {
    acroform(document)?
        .get(b"DR")
        .ok()
        .and_then(|o| dictionnaire_de(document, o))?
        .get(b"Font")
        .ok()
        .and_then(|o| dictionnaire_de(document, o))
        .cloned()
}

/// Taille du rectangle d'un widget (/Rect), en points.
fn taille_widget(document: &Document, widget: &Dictionary) -> Option<(f64, f64)> {
    let [x0, y0, x1, y1] = widget
        .get(b"Rect")
        .ok()
        .and_then(|r| rectangle(document, r))?;
    Some((x1 - x0, y1 - y0))
}

/// L'apparence normale d'un widget de texte ou de liste, pour une valeur donnée.
fn apparence_widget(
    document: &Document,
    noeud: &Noeud,
    sorte: Sorte,
    widget: &Dictionary,
    valeur: &Valeur,
    polices: Option<&Dictionary>,
) -> Option<Stream> {
    let (largeur, hauteur) = taille_widget(document, widget)?;
    let apparence = chaine_brute(document, widget, b"DA")
        .or_else(|| noeud.herite.apparence.clone())
        .unwrap_or_else(|| "/Helv 0 Tf 0 g".into());
    let style = style(document, &apparence, polices);
    let decor = decor(document, widget);
    let alignement = entier(document, widget, b"Q")
        .or(noeud.herite.alignement)
        .unwrap_or(0)
        .clamp(0, 2) as u8;
    let drapeaux = noeud.drapeaux();
    let champ = document.get_dictionary(noeud.id).ok()?;
    match sorte {
        Sorte::Texte => {
            let brut = valeur.textes().join("\n");
            let texte = if drapeaux & SECRET != 0 {
                "*".repeat(brut.chars().count())
            } else {
                brut.replace("\r\n", "\n").replace('\r', "\n")
            };
            let longueur = noeud.herite.longueur_max.filter(|&n| n > 0);
            let mise = if drapeaux & MULTILIGNE != 0 {
                Mise::Lignes
            } else if drapeaux & PEIGNE != 0 && drapeaux & SECRET == 0 {
                longueur.map_or(Mise::Ligne, |n| Mise::Peigne(n as usize))
            } else {
                Mise::Ligne
            };
            let texte = match mise {
                Mise::Lignes => texte,
                _ => texte.replace('\n', " "),
            };
            Some(flux_texte(
                largeur, hauteur, &style, alignement, &texte, mise, &decor,
            ))
        }
        Sorte::Combo => {
            let options = options_de(document, champ);
            let choisie = valeur
                .textes()
                .first()
                .map(|t| t.to_string())
                .unwrap_or_default();
            let affiche = options
                .iter()
                .find(|o| o.valeur == choisie)
                .map_or(choisie, |o| o.libelle.clone());
            Some(flux_texte(
                largeur,
                hauteur,
                &style,
                alignement,
                &affiche,
                Mise::Ligne,
                &decor,
            ))
        }
        Sorte::Liste => {
            let options = options_de(document, champ);
            let choisies = indices(&options, valeur);
            let premiere = entier(document, champ, b"TI").unwrap_or(0).max(0) as usize;
            Some(flux_liste(
                largeur,
                hauteur,
                &style,
                &options,
                &choisies,
                premiere.min(options.len().saturating_sub(1)),
                &decor,
            ))
        }
        _ => None,
    }
}

/// Rangs des options choisies d'une liste.
fn indices(options: &[OptionChoix], valeur: &Valeur) -> Vec<usize> {
    let textes = valeur.textes();
    let mut rangs: Vec<usize> = options
        .iter()
        .enumerate()
        .filter(|(_, o)| textes.contains(&o.valeur.as_str()))
        .map(|(rang, _)| rang)
        .collect();
    rangs.sort_unstable();
    rangs
}

/// Pose une apparence normale sur un widget (les autres, périmées, s'en vont).
fn poser_apparence(document: &mut Document, widget: ObjectId, apparence: Stream) {
    let id = document.add_object(apparence);
    if let Ok(dictionnaire) = document.get_dictionary_mut(widget) {
        dictionnaire.set("AP", dictionary! { "N" => Object::Reference(id) });
    }
}

// ------------------------------------------------------------------------------- Remplissage

/// Valeur d'un champ, écrite dans le PDF.
fn objet_valeur(sorte: Sorte, valeur: &Valeur) -> Object {
    match (sorte, valeur) {
        (Sorte::Case | Sorte::Radio, valeur) => Object::Name(
            valeur
                .textes()
                .first()
                .unwrap_or(&"Off")
                .as_bytes()
                .to_vec(),
        ),
        (_, Valeur::Liste(liste)) if liste.len() == 1 => texte_pdf(&liste[0]),
        (_, Valeur::Liste(liste)) => Object::Array(liste.iter().map(|t| texte_pdf(t)).collect()),
        (_, Valeur::Texte(texte)) => texte_pdf(texte),
    }
}

/// Remplit les champs nommés : leur valeur et l'apparence de chacun de leurs widgets. Un nom
/// inconnu, un champ en lecture seule ou un bouton poussoir sont laissés tels quels.
pub fn remplir(document: &mut Document, valeurs: &[(String, Valeur)]) -> Result<(), Erreur> {
    if valeurs.is_empty() {
        return Ok(());
    }
    let (noeuds, _) = noeuds(document);
    let polices = polices_du_formulaire(document);
    let mut par_nom: HashMap<&str, Vec<&Noeud>> = HashMap::new();
    for noeud in &noeuds {
        par_nom.entry(noeud.nom.as_str()).or_default().push(noeud);
    }
    for (nom_champ, valeur) in valeurs {
        for &noeud in par_nom.get(nom_champ.as_str()).into_iter().flatten() {
            let Some(sorte) = noeud.sorte() else {
                continue;
            };
            if matches!(sorte, Sorte::Bouton | Sorte::Signature)
                || noeud.drapeaux() & LECTURE_SEULE != 0
            {
                continue;
            }
            if let Ok(champ) = document.get_dictionary_mut(noeud.id) {
                champ.set("V", objet_valeur(sorte, valeur));
            }
            match sorte {
                Sorte::Case | Sorte::Radio => {
                    let voulu = valeur.textes().first().unwrap_or(&"Off").to_string();
                    for &id in &noeud.widgets {
                        let Ok(widget) = document.get_dictionary(id) else {
                            continue;
                        };
                        match etat_actif(document, widget) {
                            // Le widget a ses apparences : coché s'il porte l'état voulu.
                            Some(actif) => {
                                let etat = if actif == voulu { actif } else { "Off".into() };
                                if let Ok(widget) = document.get_dictionary_mut(id) {
                                    widget.set("AS", Object::Name(etat.into_bytes()));
                                }
                            }
                            // Sans apparence : dessinée ici, l'état coché s'appelant « Yes ».
                            None => {
                                let Some((l, h)) = taille_widget(document, widget) else {
                                    continue;
                                };
                                let apparences =
                                    flux_case(document, widget, l, h, sorte == Sorte::Radio);
                                poser_etats(document, id, apparences, &voulu);
                            }
                        }
                    }
                }
                Sorte::Texte | Sorte::Combo | Sorte::Liste => {
                    if sorte == Sorte::Liste {
                        let options = document
                            .get_dictionary(noeud.id)
                            .map(|champ| options_de(document, champ))
                            .unwrap_or_default();
                        let rangs = indices(&options, valeur);
                        if let Ok(champ) = document.get_dictionary_mut(noeud.id) {
                            if rangs.is_empty() {
                                champ.remove(b"I");
                            } else {
                                champ.set(
                                    "I",
                                    rangs
                                        .iter()
                                        .map(|&r| Object::Integer(r as i64))
                                        .collect::<Vec<_>>(),
                                );
                            }
                        }
                    }
                    let mut apparences = Vec::new();
                    for &id in &noeud.widgets {
                        let Ok(widget) = document.get_dictionary(id) else {
                            continue;
                        };
                        if let Some(flux) = apparence_widget(
                            document,
                            noeud,
                            sorte,
                            widget,
                            valeur,
                            polices.as_ref(),
                        ) {
                            apparences.push((id, flux));
                        }
                    }
                    for (id, flux) in apparences {
                        poser_apparence(document, id, flux);
                    }
                }
                Sorte::Bouton | Sorte::Signature => {}
            }
        }
    }
    retirer_xfa(document);
    Ok(())
}

/// Retire la version XFA d'un formulaire hybride : les lecteurs qui la préfèrent montreraient
/// ses valeurs, et non celles écrites dans les champs.
fn retirer_xfa(document: &mut Document) {
    let reference = document
        .catalog()
        .ok()
        .and_then(|c| c.get(b"AcroForm").ok())
        .and_then(|o| o.as_reference().ok());
    let formulaire = match reference {
        Some(id) => document.get_dictionary_mut(id).ok(),
        None => document
            .catalog_mut()
            .ok()
            .and_then(|c| c.get_mut(b"AcroForm").ok())
            .and_then(|o| o.as_dict_mut().ok()),
    };
    if let Some(formulaire) = formulaire {
        formulaire.remove(b"XFA");
    }
}

/// Apparences des champs qui n'en ont pas, ou dont le formulaire demande qu'on les refasse
/// (/NeedAppearances) : dessinées ici d'après leur valeur, comme le ferait un autre lecteur de PDF.
/// Renvoie vrai si une apparence a été posée.
pub fn completer_apparences(document: &mut Document) -> bool {
    let a_refaire = acroform(document)
        .and_then(|f| f.get(b"NeedAppearances").ok())
        .and_then(|o| o.as_bool().ok())
        .unwrap_or(false);
    let (noeuds, _) = noeuds(document);
    let polices = polices_du_formulaire(document);
    let mut apparences = Vec::new();
    let mut cases = Vec::new();
    for noeud in &noeuds {
        let Some(sorte) = noeud.sorte() else {
            continue;
        };
        let valeur = valeur_de(document, noeud, sorte);
        for &id in &noeud.widgets {
            let Ok(widget) = document.get_dictionary(id) else {
                continue;
            };
            let sans_apparence = !widget.has(b"AP");
            match sorte {
                Sorte::Texte | Sorte::Combo | Sorte::Liste if a_refaire || sans_apparence => {
                    if let Some(flux) =
                        apparence_widget(document, noeud, sorte, widget, &valeur, polices.as_ref())
                    {
                        apparences.push((id, flux));
                    }
                }
                Sorte::Case | Sorte::Radio if sans_apparence => {
                    if let Some((l, h)) = taille_widget(document, widget) {
                        let etat = valeur
                            .textes()
                            .first()
                            .map(|t| t.to_string())
                            .unwrap_or_default();
                        cases.push((
                            id,
                            flux_case(document, widget, l, h, sorte == Sorte::Radio),
                            etat,
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    let change = !apparences.is_empty() || !cases.is_empty();
    for (id, flux) in apparences {
        poser_apparence(document, id, flux);
    }
    for (id, apparences, etat) in cases {
        poser_etats(document, id, apparences, &etat);
    }
    change
}

/// Pose les deux apparences d'une case dessinée ici (cochée sous le nom « Yes », décochée sous
/// « Off ») et l'état voulu : « Off », ou coché.
fn poser_etats(
    document: &mut Document,
    widget: ObjectId,
    (coche, vide): (Stream, Stream),
    voulu: &str,
) {
    let (coche, vide) = (document.add_object(coche), document.add_object(vide));
    let etat: &[u8] = if voulu.is_empty() || voulu == "Off" {
        b"Off"
    } else {
        b"Yes"
    };
    if let Ok(widget) = document.get_dictionary_mut(widget) {
        widget.set(
            "AP",
            dictionary! { "N" => dictionary! { "Yes" => Object::Reference(coche), "Off" => Object::Reference(vide) } },
        );
        widget.set("AS", Object::Name(etat.to_vec()));
    }
}

// ------------------------------------------------------------------------------- Affichage

/// Le flux de l'apparence normale d'une annotation : son flux, ou celui de son état (/AS) quand
/// elle en a plusieurs (« Off » à défaut).
fn flux_normal(document: &Document, annotation: &Dictionary) -> Option<ObjectId> {
    let apparences = annotation
        .get(b"AP")
        .ok()
        .and_then(|o| dictionnaire_de(document, o))?;
    match apparences.get(b"N").ok()? {
        Object::Reference(id) => match document.get_object(*id).ok()? {
            Object::Stream(_) => Some(*id),
            Object::Dictionary(etats) => choisir_etat(document, annotation, etats),
            _ => None,
        },
        Object::Dictionary(etats) => choisir_etat(document, annotation, etats),
        _ => None,
    }
}

fn choisir_etat(
    document: &Document,
    annotation: &Dictionary,
    etats: &Dictionary,
) -> Option<ObjectId> {
    let etat = nom(document, annotation, b"AS").unwrap_or_else(|| b"Off".to_vec());
    etats
        .get(&etat)
        .or_else(|_| etats.get(b"Off"))
        .ok()
        .and_then(|o| o.as_reference().ok())
}

/// Prépare le document pour le rendu (hayro 0.7) : les apparences qui manquent, puis, pour
/// chaque annotation qui a plusieurs états (cases, boutons radio), l'apparence de son état seul.
/// Ne sert qu'à dessiner : le PDF écrit garde ses états. Renvoie vrai si quelque chose a changé.
pub fn pour_l_affichage(document: &mut Document) -> bool {
    let mut change = completer_apparences(document);
    let mut choix: Vec<(ObjectId, Option<ObjectId>)> = Vec::new();
    for id_page in document.get_pages().into_values() {
        let Ok(page) = document.get_dictionary(id_page) else {
            continue;
        };
        let Some(annotations) = page
            .get(b"Annots")
            .ok()
            .map(|o| resoudre(document, o))
            .and_then(|o| o.as_array().ok())
        else {
            continue;
        };
        for entree in annotations {
            let Object::Reference(id) = entree else {
                continue;
            };
            let Ok(annotation) = document.get_dictionary(*id) else {
                continue;
            };
            let a_des_etats = annotation
                .get(b"AP")
                .ok()
                .and_then(|o| dictionnaire_de(document, o))
                .and_then(|ap| ap.get(b"N").ok())
                .is_some_and(|n| resoudre(document, n).as_dict().is_ok());
            if a_des_etats {
                choix.push((*id, flux_normal(document, annotation)));
            }
        }
    }
    for (id, flux) in choix {
        if let Ok(annotation) = document.get_dictionary_mut(id) {
            match flux {
                Some(flux) => annotation.set("AP", dictionary! { "N" => Object::Reference(flux) }),
                None => {
                    annotation.remove(b"AP");
                }
            }
            change = true;
        }
    }
    change
}

// ------------------------------------------------------------------------------- Signature dessinée

/// Retire une référence d'un tableau (/Fields, /Kids, /Annots), écrit dans son dictionnaire ou
/// objet à part. Renvoie vrai si le tableau est vide ensuite.
fn retirer_de(
    document: &mut Document,
    proprietaire: ObjectId,
    cle: &[u8],
    cible: ObjectId,
) -> bool {
    let tableau = document
        .get_dictionary(proprietaire)
        .ok()
        .and_then(|d| d.get(cle).ok())
        .cloned();
    let filtrer = |liste: &mut Vec<Object>| {
        liste.retain(|o| o.as_reference().ok() != Some(cible));
        liste.is_empty()
    };
    match tableau {
        Some(Object::Reference(id)) => match document.get_object_mut(id) {
            Ok(Object::Array(liste)) => filtrer(liste),
            _ => false,
        },
        Some(Object::Array(_)) => match document
            .get_dictionary_mut(proprietaire)
            .ok()
            .and_then(|d| d.get_mut(cle).ok())
        {
            Some(Object::Array(liste)) => filtrer(liste),
            _ => false,
        },
        _ => false,
    }
}

/// Retire un champ de l'arbre du formulaire : des /Kids de son parent (et le parent, s'il n'en a
/// plus), ou des /Fields du formulaire.
fn retirer_champ(document: &mut Document, champ: ObjectId, profondeur: usize) {
    if profondeur > PROFONDEUR_MAX {
        return;
    }
    let parent = document
        .get_dictionary(champ)
        .ok()
        .and_then(|d| d.get(b"Parent").ok())
        .and_then(|o| o.as_reference().ok());
    match parent {
        Some(parent) => {
            if retirer_de(document, parent, b"Kids", champ) {
                retirer_champ(document, parent, profondeur + 1);
            }
        }
        None => {
            let formulaire = document
                .catalog()
                .ok()
                .and_then(|c| c.get(b"AcroForm").ok())
                .and_then(|o| o.as_reference().ok());
            if let Some(formulaire) = formulaire {
                retirer_de(document, formulaire, b"Fields", champ);
            } else if let Ok(Object::Array(champs)) = document
                .catalog_mut()
                .and_then(|c| c.get_mut(b"AcroForm"))
                .and_then(|f| f.as_dict_mut())
                .and_then(|f| f.get_mut(b"Fields"))
            {
                champs.retain(|o| o.as_reference().ok() != Some(champ));
            }
        }
    }
}

/// Les champs de signature (pas encore signés) où l'on a posé une signature dessinée — `signatures`
/// : (page, zone x, y, largeur, hauteur en points de la page affichée) — s'en vont du formulaire :
/// le PDF enregistré n'invite plus à signer là où c'est fait.
pub fn retirer_champs_signes(document: &mut Document, signatures: &[(usize, [f64; 4])]) {
    if signatures.is_empty() {
        return;
    }
    let (noeuds, _) = noeuds(document);
    let pages = pages_des_annotations(document);
    let ids_pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    let mut a_retirer: Vec<(ObjectId, ObjectId, usize)> = Vec::new();
    for noeud in noeuds
        .iter()
        .filter(|n| n.sorte() == Some(Sorte::Signature) && n.herite.valeur.is_none())
    {
        for &widget in &noeud.widgets {
            let (Some(&page), Ok(dictionnaire)) =
                (pages.get(&widget), document.get_dictionary(widget))
            else {
                continue;
            };
            let (Some([x0, y0, x1, y1]), Some(id_page)) = (
                dictionnaire
                    .get(b"Rect")
                    .ok()
                    .and_then(|r| rectangle(document, r)),
                ids_pages.get(page),
            ) else {
                continue;
            };
            let Ok(dictionnaire_page) = document.get_dictionary(*id_page) else {
                continue;
            };
            let cadre = Cadre::de_page(document, dictionnaire_page);
            let (_, hauteur_page) = cadre.taille_vue();
            let [gauche, bas, droite, haut] =
                boite(cadre.vers_vue(), &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            let (dessus, dessous) = (hauteur_page - haut, hauteur_page - bas);
            let signe = signatures.iter().any(|&(p, [x, y, l, h])| {
                let (cx, cy) = (x + l / 2.0, y + h / 2.0);
                p == page && cx >= gauche && cx <= droite && cy >= dessus && cy <= dessous
            });
            if signe {
                a_retirer.push((widget, noeud.id, page));
            }
        }
    }
    for (widget, champ, page) in a_retirer {
        if let Some(&id_page) = ids_pages.get(page) {
            retirer_de(document, id_page, b"Annots", widget);
        }
        // Champ et widget réunis, ou dernier widget du champ : le champ s'en va avec lui.
        if widget == champ || retirer_de(document, champ, b"Kids", widget) {
            retirer_champ(document, champ, 0);
        }
    }
}

// ------------------------------------------------------------------------------- Aplatissement

/// Ajoute un formulaire (XObject) aux ressources d'une page, sous un nom libre ; renvoie ce nom.
fn ajouter_xobjet(
    document: &mut Document,
    page: ObjectId,
    xobjet: ObjectId,
    rang: usize,
) -> Option<String> {
    let ressources = document
        .get_dictionary(page)
        .ok()?
        .get(b"Resources")
        .ok()
        .cloned();
    let id_ressources = match ressources {
        Some(Object::Reference(id)) => Some(id),
        _ => None,
    };
    // Le sous-dictionnaire /XObject : écrit dans les ressources, ou objet à part.
    let sous = {
        let dictionnaire = match id_ressources {
            Some(id) => document.get_dictionary(id).ok(),
            None => document
                .get_dictionary(page)
                .ok()
                .and_then(|p| p.get(b"Resources").ok())
                .and_then(|o| o.as_dict().ok()),
        };
        dictionnaire.and_then(|d| d.get(b"XObject").ok()).cloned()
    };
    let existants: HashSet<Vec<u8>> = match &sous {
        Some(Object::Reference(id)) => document
            .get_dictionary(*id)
            .map(|d| d.iter().map(|(k, _)| k.clone()).collect())
            .unwrap_or_default(),
        Some(Object::Dictionary(d)) => d.iter().map(|(k, _)| k.clone()).collect(),
        _ => HashSet::new(),
    };
    let mut nom = format!("ChampAplati{rang}");
    while existants.contains(nom.as_bytes()) {
        nom.push('b');
    }
    match sous {
        Some(Object::Reference(id)) => {
            document
                .get_dictionary_mut(id)
                .ok()?
                .set(nom.as_bytes().to_vec(), Object::Reference(xobjet));
        }
        _ => {
            let ressources = match id_ressources {
                Some(id) => document.get_dictionary_mut(id).ok()?,
                None => {
                    let page = document.get_dictionary_mut(page).ok()?;
                    if !page.has(b"Resources") {
                        page.set("Resources", Dictionary::new());
                    }
                    page.get_mut(b"Resources").ok()?.as_dict_mut().ok()?
                }
            };
            if !ressources.has(b"XObject") {
                ressources.set("XObject", Dictionary::new());
            }
            ressources
                .get_mut(b"XObject")
                .ok()?
                .as_dict_mut()
                .ok()?
                .set(nom.as_bytes().to_vec(), Object::Reference(xobjet));
        }
    }
    Some(nom)
}

/// Ajoute du dessin au contenu d'une page, après le sien (encadré par q … Q, pour partir de l'état
/// graphique de départ).
fn ajouter_au_contenu(document: &mut Document, page: ObjectId, dessin: String) {
    let existants: Vec<Object> = match document
        .get_dictionary(page)
        .ok()
        .and_then(|p| p.get(b"Contents").ok())
        .cloned()
    {
        Some(Object::Reference(id)) => match document.get_object(id) {
            Ok(Object::Array(liste)) => liste.clone(),
            _ => vec![Object::Reference(id)],
        },
        Some(Object::Array(liste)) => liste,
        _ => Vec::new(),
    };
    let avant = document.add_object(Stream::new(Dictionary::new(), b"q\n".to_vec()));
    let apres = document.add_object(Stream::new(
        Dictionary::new(),
        format!("\nQ\n{dessin}").into_bytes(),
    ));
    let mut contenu = vec![Object::Reference(avant)];
    contenu.extend(existants);
    contenu.push(Object::Reference(apres));
    if let Ok(page) = document.get_dictionary_mut(page) {
        page.set("Contents", contenu);
    }
}

/// Aplatit le formulaire : le dessin de chaque champ rejoint le contenu de sa page, les champs
/// et le formulaire s'en vont. Les autres annotations (commentaires, liens) restent.
pub fn aplatir(document: &mut Document) -> Result<(), Erreur> {
    completer_apparences(document);
    let pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    for page in pages {
        let Some(annotations) = document
            .get_dictionary(page)
            .ok()
            .and_then(|p| p.get(b"Annots").ok())
            .map(|o| resoudre(document, o))
            .and_then(|o| o.as_array().ok())
            .cloned()
        else {
            continue;
        };
        let mut gardees = Vec::new();
        let mut dessins = Vec::new();
        for entree in annotations {
            let annotation = match &entree {
                Object::Reference(id) => document.get_dictionary(*id).ok(),
                Object::Dictionary(d) => Some(d),
                _ => None,
            };
            let Some(annotation) = annotation else {
                gardees.push(entree);
                continue;
            };
            if nom(document, annotation, b"Subtype").as_deref() != Some(b"Widget".as_slice()) {
                gardees.push(entree);
                continue;
            }
            if entier(document, annotation, b"F").unwrap_or(0) & (CACHEE | SANS_VUE) != 0 {
                continue;
            }
            let (Some(flux), Some(cadre)) = (
                flux_normal(document, annotation),
                annotation
                    .get(b"Rect")
                    .ok()
                    .and_then(|r| rectangle(document, r)),
            ) else {
                continue;
            };
            dessins.push((flux, cadre));
        }
        let mut dessin = String::new();
        for (rang, (flux, [x0, y0, x1, y1])) in dessins.into_iter().enumerate() {
            let Ok(Object::Stream(apparence)) = document.get_object(flux) else {
                continue;
            };
            let Some(bbox) = apparence
                .dict
                .get(b"BBox")
                .ok()
                .and_then(|b| rectangle(document, b))
            else {
                continue;
            };
            let matrice = apparence
                .dict
                .get(b"Matrix")
                .ok()
                .map(|o| resoudre(document, o))
                .and_then(|o| o.as_array().ok())
                .and_then(|valeurs| Matrice::de_nombres(valeurs))
                .unwrap_or(Matrice::IDENTITE);
            let [bx0, by0, bx1, by1] = boite(
                matrice,
                &[
                    (bbox[0], bbox[1]),
                    (bbox[2], bbox[1]),
                    (bbox[2], bbox[3]),
                    (bbox[0], bbox[3]),
                ],
            );
            if bx1 - bx0 <= 0.0 || by1 - by0 <= 0.0 {
                continue;
            }
            let (sx, sy) = ((x1 - x0) / (bx1 - bx0), (y1 - y0) / (by1 - by0));
            let a = Matrice([sx, 0.0, 0.0, sy, x0 - bx0 * sx, y0 - by0 * sy]);
            // Le formulaire doit se déclarer comme tel pour être peint par « Do ».
            if let Ok(Object::Stream(apparence)) = document.get_object_mut(flux) {
                apparence.dict.set("Type", "XObject");
                apparence.dict.set("Subtype", "Form");
            }
            if let Some(nom) = ajouter_xobjet(document, page, flux, rang) {
                dessin.push_str(&format!("q {} /{nom} Do Q\n", a.operateur()));
            }
        }
        if !dessin.is_empty() {
            ajouter_au_contenu(document, page, dessin);
        }
        let tableau_a_part = document
            .get_dictionary(page)
            .ok()
            .and_then(|p| p.get(b"Annots").ok())
            .and_then(|o| o.as_reference().ok());
        match tableau_a_part {
            Some(id) => {
                if let Ok(objet) = document.get_object_mut(id) {
                    *objet = Object::Array(gardees);
                }
            }
            None => {
                if let Ok(page) = document.get_dictionary_mut(page) {
                    if gardees.is_empty() {
                        page.remove(b"Annots");
                    } else {
                        page.set("Annots", gardees);
                    }
                }
            }
        }
    }
    if let Ok(catalogue) = document.catalog_mut() {
        catalogue.remove(b"AcroForm");
    }
    document.prune_objects();
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::essais::{fichier_temporaire, pdf_formulaire};
    use crate::pdf::{charger, ecrire};
    use crate::police::chaine;

    fn charge(octets: &[u8]) -> Document {
        charger(octets, "formulaire.pdf").unwrap()
    }

    fn champ<'a>(formulaire: &'a Formulaire, nom: &str) -> &'a Champ {
        formulaire
            .champs
            .iter()
            .find(|c| c.nom == nom)
            .unwrap_or_else(|| panic!("champ « {nom} » absent"))
    }

    fn texte(t: &str) -> Valeur {
        Valeur::Texte(t.into())
    }

    /// Le pixel le plus sombre (moyenne des trois couleurs, 0 noir, 255 blanc) du rectangle donné en
    /// points de la page affichée (rendue à 72 ppp : un pixel par point).
    fn plus_sombre(octets: &[u8], [x0, y0, x1, y1]: [u32; 4]) -> u32 {
        let (largeur, _, pixels) =
            crate::rendu::pixels_page(Arc::new(octets.to_vec()), "formulaire.pdf", 0, 72).unwrap();
        let mut minimum = 255;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * largeur + x) * 3) as usize;
                let gris =
                    (u32::from(pixels[i]) + u32::from(pixels[i + 1]) + u32::from(pixels[i + 2]))
                        / 3;
                minimum = minimum.min(gris);
            }
        }
        minimum
    }

    /// Le contenu de l'apparence normale du premier widget d'un champ.
    fn apparence(document: &Document, nom: &str) -> String {
        let (liste, _) = noeuds(document);
        let noeud = liste.iter().find(|n| n.nom == nom).unwrap();
        let widget = document.get_dictionary(noeud.widgets[0]).unwrap();
        let id = flux_normal(document, widget).unwrap();
        let Object::Stream(flux) = document.get_object(id).unwrap() else {
            panic!("pas de flux");
        };
        String::from_utf8_lossy(&flux.decompressed_content().unwrap_or(flux.content.clone()))
            .into_owned()
    }

    // Zones des widgets du formulaire d'essai, en points de la page affichée (haut à gauche).
    const RADIO_CONGES: [u32; 4] = [152, 222, 160, 230];
    const RADIO_RTT: [u32; 4] = [292, 222, 300, 230];
    const CASE: [u32; 4] = [58, 400, 66, 408];
    const NOM: [u32; 4] = [152, 132, 328, 148];

    #[test]
    fn lit_les_champs_du_formulaire() {
        let lu = formulaire(&charge(&pdf_formulaire()));
        let noms: Vec<&str> = lu.champs.iter().map(|c| c.nom.as_str()).collect();
        assert_eq!(
            noms,
            [
                "Salarie.Nom",
                "Salarie.Prenom",
                "Salarie.Matricule",
                "Service",
                "Type",
                "Du",
                "Au",
                "Jours",
                "Motif",
                "Certifie",
                "Visa",
                "Signature"
            ]
        );
        assert!(!lu.xfa && !lu.calculs);
        // La sorte et l'apparence par défaut viennent du groupe « Salarie ».
        let nom = champ(&lu, "Salarie.Nom");
        assert_eq!(nom.sorte, Sorte::Texte);
        let w = &nom.widgets[0];
        assert_eq!(
            (w.page, w.x, w.y, w.largeur, w.hauteur),
            (0, 150.0, 130.0, 180.0, 20.0)
        );
        assert_eq!(w.taille, 0.0, "taille automatique");
        assert_eq!(
            champ(&lu, "Salarie.Prenom").libelle.as_deref(),
            Some("Prénom")
        );
        let matricule = champ(&lu, "Salarie.Matricule");
        assert!(matricule.peigne);
        assert_eq!(matricule.longueur_max, Some(5));
        let service = champ(&lu, "Service");
        assert_eq!(service.sorte, Sorte::Combo);
        assert_eq!(service.options.len(), 4);
        assert_eq!(service.options[0].libelle, "Comptabilité");
        let type_conge = champ(&lu, "Type");
        assert_eq!(type_conge.sorte, Sorte::Radio);
        assert!(type_conge.sans_decocher);
        assert_eq!(type_conge.valeur, texte("Off"));
        let etats: Vec<&str> = type_conge
            .widgets
            .iter()
            .map(|w| w.etat.as_deref().unwrap())
            .collect();
        assert_eq!(etats, ["Conges", "RTT", "SansSolde"]);
        let certifie = champ(&lu, "Certifie");
        assert_eq!(
            (certifie.sorte, certifie.widgets[0].etat.as_deref()),
            (Sorte::Case, Some("Oui"))
        );
        let jours = &champ(&lu, "Jours").widgets[0];
        assert_eq!((jours.alignement, jours.taille), (2, 11.0));
        assert!(champ(&lu, "Motif").multiligne);
        let visa = champ(&lu, "Visa");
        assert!(visa.lecture_seule);
        assert_eq!(visa.valeur, texte("En attente"));
        let signature = champ(&lu, "Signature");
        assert_eq!(signature.sorte, Sorte::Signature);
        assert!(!signature.signe, "pas encore signé");
    }

    #[test]
    fn remplit_les_champs_et_les_dessine() {
        let mut document = charge(&pdf_formulaire());
        let valeurs = vec![
            ("Salarie.Nom".to_string(), texte("NKOULOU")),
            ("Salarie.Prenom".into(), texte("Éloïse")),
            ("Salarie.Matricule".into(), texte("00123")),
            ("Service".into(), texte("Informatique")),
            ("Type".into(), texte("RTT")),
            ("Jours".into(), texte("3")),
            (
                "Motif".into(),
                texte("Mariage de ma sœur à Pointe-Noire.\nRetour le lundi."),
            ),
            ("Certifie".into(), texte("Oui")),
            // En lecture seule : laissé tel quel.
            ("Visa".into(), texte("Accordé")),
        ];
        remplir(&mut document, &valeurs).unwrap();
        let octets = ecrire(document).unwrap();
        let relu = formulaire(&charge(&octets));
        for (nom, valeur) in &valeurs[..8] {
            assert_eq!(&champ(&relu, nom).valeur, valeur, "{nom}");
        }
        assert_eq!(champ(&relu, "Visa").valeur, texte("En attente"));
        // Les apparences : le bouton radio choisi, la case cochée, le nom écrit.
        assert!(
            plus_sombre(&octets, RADIO_RTT) < 80,
            "bouton radio RTT coché"
        );
        assert!(
            plus_sombre(&octets, RADIO_CONGES) > 200,
            "bouton radio Congés vide"
        );
        assert!(plus_sombre(&octets, CASE) < 120, "case cochée");
        assert!(plus_sombre(&octets, NOM) < 120, "nom écrit");
        // Accents : la valeur en UTF-16, l'apparence en WinAnsi.
        let document = charge(&octets);
        assert!(apparence(&document, "Salarie.Prenom").contains(&chaine("Éloïse")));
    }

    #[test]
    fn dessine_le_texte_comme_le_demande_le_champ() {
        let mut document = charge(&pdf_formulaire());
        remplir(
            &mut document,
            &[
                ("Salarie.Matricule".into(), texte("00123")),
                ("Jours".into(), texte("3")),
                (
                    "Motif".into(),
                    texte("Un motif assez long pour tenir sur deux lignes au moins dans ce champ de plusieurs lignes."),
                ),
                ("Service".into(), texte("Informatique")),
            ],
        )
        .unwrap();
        // Matricule : une case par caractère.
        assert_eq!(
            apparence(&document, "Salarie.Matricule")
                .matches(" Tm ")
                .count(),
            5
        );
        // Jours : aligné à droite, à 11 points (« 3 » : 556 millièmes), marge de 2 points.
        assert!(apparence(&document, "Jours").contains("/F0 11.00 Tf 0 g\n51.884 "));
        // Motif : plusieurs lignes, à 10 points.
        let motif = apparence(&document, "Motif");
        assert!(motif.contains("/F0 10.00 Tf"));
        assert!(motif.matches(" Tm ").count() >= 2);
        // Liste déroulante : son texte affiché.
        assert!(apparence(&document, "Service").contains(&chaine("Informatique")));
    }

    #[test]
    fn decoche_et_change_de_bouton_radio() {
        let mut document = charge(&pdf_formulaire());
        remplir(
            &mut document,
            &[
                ("Type".into(), texte("Conges")),
                ("Certifie".into(), texte("Oui")),
            ],
        )
        .unwrap();
        remplir(
            &mut document,
            &[
                ("Type".into(), texte("SansSolde")),
                ("Certifie".into(), texte("Off")),
            ],
        )
        .unwrap();
        let octets = ecrire(document).unwrap();
        let relu = formulaire(&charge(&octets));
        assert_eq!(champ(&relu, "Type").valeur, texte("SansSolde"));
        assert_eq!(champ(&relu, "Certifie").valeur, texte("Off"));
        assert!(plus_sombre(&octets, RADIO_CONGES) > 200);
        assert!(
            plus_sombre(&octets, [402, 222, 410, 230]) < 80,
            "Sans solde coché"
        );
        assert!(
            plus_sombre(&octets, CASE) > 120,
            "case décochée : son bord seulement"
        );
    }

    #[test]
    fn aplatit_le_formulaire_sans_rien_perdre_de_son_dessin() {
        let mut document = charge(&pdf_formulaire());
        remplir(
            &mut document,
            &[
                ("Type".into(), texte("RTT")),
                ("Certifie".into(), texte("Oui")),
                ("Salarie.Nom".into(), texte("NKOULOU")),
            ],
        )
        .unwrap();
        aplatir(&mut document).unwrap();
        let octets = ecrire(document).unwrap();
        let relu = charge(&octets);
        assert!(
            relu.catalog().unwrap().get(b"AcroForm").is_err(),
            "plus de formulaire"
        );
        assert!(formulaire(&relu).champs.is_empty());
        let page = relu
            .get_dictionary(*relu.get_pages().get(&1).unwrap())
            .unwrap();
        assert!(page.get(b"Annots").is_err(), "plus de widgets");
        // Le dessin des champs est dans la page : bouton radio, case, nom, visa.
        assert!(plus_sombre(&octets, RADIO_RTT) < 80);
        assert!(plus_sombre(&octets, RADIO_CONGES) > 200);
        assert!(plus_sombre(&octets, CASE) < 120);
        assert!(plus_sombre(&octets, NOM) < 120);
        assert!(
            plus_sombre(&octets, [202, 446, 398, 464]) < 120,
            "visa « En attente »"
        );
    }

    #[test]
    fn retire_la_version_xfa_et_signale_les_calculs() {
        let mut document = charge(&pdf_formulaire());
        let id = document
            .catalog()
            .unwrap()
            .get(b"AcroForm")
            .unwrap()
            .as_reference()
            .unwrap();
        document
            .get_dictionary_mut(id)
            .unwrap()
            .set("XFA", Object::Array(Vec::new()));
        let (liste, _) = noeuds(&document);
        let jours = liste.iter().find(|n| n.nom == "Jours").unwrap().id;
        document.get_dictionary_mut(jours).unwrap().set(
            "AA",
            dictionary! { "C" => dictionary! { "S" => "JavaScript", "JS" => texte_pdf("AFSimple_Calculate()") } },
        );
        let lu = formulaire(&document);
        assert!(lu.xfa && !lu.xfa_seul && lu.calculs);
        remplir(&mut document, &[("Jours".into(), texte("4"))]).unwrap();
        assert!(!formulaire(&document).xfa, "version XFA retirée");
    }

    #[test]
    fn refait_les_apparences_que_demande_le_formulaire() {
        let mut document = charge(&pdf_formulaire());
        // Une valeur écrite par un autre logiciel, sans apparence : « NeedAppearances ».
        let (liste, _) = noeuds(&document);
        let du = liste.iter().find(|n| n.nom == "Du").unwrap().id;
        document
            .get_dictionary_mut(du)
            .unwrap()
            .set("V", texte_pdf("01/12/2026"));
        let id = document
            .catalog()
            .unwrap()
            .get(b"AcroForm")
            .unwrap()
            .as_reference()
            .unwrap();
        document
            .get_dictionary_mut(id)
            .unwrap()
            .set("NeedAppearances", true);
        assert!(pour_l_affichage(&mut document));
        assert!(apparence(&document, "Du").contains(&chaine("01/12/2026")));
        // Les boutons radio n'ont plus qu'une apparence : celle de leur état (pour hayro 0.7).
        let (liste, _) = noeuds(&document);
        let radio = liste.iter().find(|n| n.nom == "Type").unwrap().widgets[0];
        let apparences = document
            .get_dictionary(radio)
            .unwrap()
            .get(b"AP")
            .unwrap()
            .as_dict()
            .unwrap();
        assert!(apparences.get(b"N").unwrap().as_reference().is_ok());
    }

    #[test]
    fn retire_le_champ_de_signature_ou_l_on_a_signe() {
        let mut document = charge(&pdf_formulaire());
        // Le champ « Signature » : x 56 à 260, y 290 à 340 (842 - 340 = 502 sur la page affichée).
        // Une signature posée ailleurs ne le retire pas ; posée dedans, si.
        retirer_champs_signes(&mut document, &[(0, [300.0, 500.0, 100.0, 30.0])]);
        assert!(
            formulaire(&document)
                .champs
                .iter()
                .any(|c| c.nom == "Signature")
        );
        retirer_champs_signes(&mut document, &[(0, [80.0, 510.0, 120.0, 34.0])]);
        let apres = formulaire(&document);
        assert!(
            !apres.champs.iter().any(|c| c.nom == "Signature"),
            "champ retiré"
        );
        assert_eq!(apres.champs.len(), 11, "les autres champs restent");
        let octets = ecrire(document).unwrap();
        let relu = charge(&octets);
        let page = relu
            .get_dictionary(*relu.get_pages().get(&1).unwrap())
            .unwrap();
        let annotations = page.get(b"Annots").unwrap().as_array().unwrap();
        assert_eq!(
            annotations.len(),
            13,
            "14 widgets, moins celui de la signature"
        );
    }

    #[test]
    fn remplit_en_memoire_puis_enregistre_un_pdf_aplati() {
        use crate::commentaires::Retouches;
        let chemin = fichier_temporaire(
            "formulaire-lecteur",
            "Demande de congés.pdf",
            &pdf_formulaire(),
        );
        let retouches = Retouches {
            champs: vec![
                ("Salarie.Nom".into(), texte("MOUKOKO")),
                ("Type".into(), texte("RTT")),
            ],
            ..Retouches::default()
        };
        crate::operations::retoucher_lecture(&chemin, &retouches).unwrap();
        // Le lecteur montre la copie remplie ; l'original, lui, est intact.
        let en_memoire = formulaire(&charge(&crate::memoire::pdf(&chemin).unwrap()));
        assert_eq!(champ(&en_memoire, "Salarie.Nom").valeur, texte("MOUKOKO"));
        let recu = crate::operations::formulaire_pdf(&chemin).unwrap();
        assert_eq!(champ(&recu, "Salarie.Nom").valeur, texte(""));
        assert_eq!(std::fs::read(&chemin).unwrap(), pdf_formulaire());
        // Enregistré aplati : un PDF sans formulaire, où le nom et le bouton radio sont dessinés.
        let sortie = chemin.replace(".pdf", " - rempli.pdf");
        crate::operations::annoter_pdf(&chemin, &[], &sortie, true, None).unwrap();
        let octets = std::fs::read(&sortie).unwrap();
        assert!(formulaire(&charge(&octets)).champs.is_empty());
        assert!(plus_sombre(&octets, NOM) < 120);
        assert!(plus_sombre(&octets, RADIO_RTT) < 80);
        crate::memoire::reverrouiller(&chemin);
    }
}
