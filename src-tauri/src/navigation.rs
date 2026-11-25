//! Navigation dans un PDF, pour le lecteur : ses signets (le sommaire) et les liens de ses pages.
//! Les positions sont en points de la page affichée, depuis son coin en haut à gauche, comme les
//! mots (`texte.rs`) : le lecteur les place sans calcul.
//!
//! Un lien vers un site ne garde que les adresses web et e-mail (`adresse_sure`) : jamais un
//! fichier, un programme ou un script. Le lecteur demande encore l'accord avant de l'ouvrir.

use std::collections::{BTreeMap, HashMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::Serialize;

use crate::geometrie::{Cadre, boite, rectangle, resoudre};

/// Signets lus au plus (le sommaire d'un livre en compte quelques centaines).
const SIGNETS_MAX: usize = 5000;
/// Profondeur des signets et des arbres de noms au plus : un PDF piégé ne fait pas boucler.
const PROFONDEUR_MAX: usize = 32;
/// Liens d'une page au plus.
const LIENS_MAX: usize = 1000;
/// Longueur d'un titre de signet au plus, en caractères.
const TITRE_MAX: usize = 300;

/// Où mène un signet ou un lien interne.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Destination {
    /// Page visée (0 : la première).
    pub page: usize,
    /// Hauteur visée, depuis le haut de la page affichée, quand le PDF la donne.
    pub y: Option<f64>,
}

/// Un signet, à plat : son niveau dit sa place dans l'arbre.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Signet {
    pub titre: String,
    /// 0 pour le premier niveau.
    pub niveau: usize,
    pub destination: Option<Destination>,
    /// Adresse web ou e-mail, pour un signet qui y mène.
    pub adresse: Option<String>,
    /// Ses sous-signets sont dépliés à l'ouverture (le PDF le dit).
    pub ouvert: bool,
}

/// Un lien d'une page : sa zone, et où il mène.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Lien {
    pub x: f64,
    pub y: f64,
    pub largeur: f64,
    pub hauteur: f64,
    pub destination: Option<Destination>,
    pub adresse: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Navigation {
    pub signets: Vec<Signet>,
    /// Les liens de chaque page, dans l'ordre des pages.
    pub liens: Vec<Vec<Lien>>,
}

/// Une adresse que le lecteur peut ouvrir sans risque : une page web ou un e-mail, sans espace ni
/// caractère qu'un système pourrait interpréter. « www.… » sans protocole devient « http://www.… ».
pub fn adresse_sure(adresse: &str) -> Option<String> {
    let adresse = adresse.trim();
    if adresse.is_empty()
        || adresse.len() > 2048
        || adresse.chars().any(|c| {
            c.is_whitespace() || c.is_control() || matches!(c, '"' | '<' | '>' | '\\' | '`')
        })
    {
        return None;
    }
    let minuscules = adresse.to_ascii_lowercase();
    if ["http://", "https://", "mailto:"]
        .iter()
        .any(|debut| minuscules.starts_with(debut) && minuscules.len() > debut.len())
    {
        Some(adresse.to_string())
    } else if minuscules.starts_with("www.") {
        Some(format!("http://{adresse}"))
    } else {
        None
    }
}

/// Signets et liens du document, chargé par `pdf::charger` (attributs hérités recopiés).
pub fn navigation(document: &Document) -> Navigation {
    let lecteur = Lecteur::new(document);
    Navigation {
        signets: lecteur.signets(),
        liens: document
            .get_pages()
            .into_values()
            .map(|id| lecteur.liens(id))
            .collect(),
    }
}

struct Lecteur<'a> {
    document: &'a Document,
    /// Rang de chaque page, d'après son objet.
    pages: HashMap<ObjectId, usize>,
    /// Destinations nommées : dictionnaire `/Dests` (PDF 1.1) et arbre de noms (depuis PDF 1.2).
    nommees: HashMap<Vec<u8>, &'a Object>,
}

impl<'a> Lecteur<'a> {
    fn new(document: &'a Document) -> Self {
        let pages: BTreeMap<u32, ObjectId> = document.get_pages();
        let pages = pages
            .into_iter()
            .map(|(numero, id)| (id, numero.saturating_sub(1) as usize))
            .collect();
        let mut nommees = HashMap::new();
        if let Ok(catalogue) = document.catalog() {
            if let Ok(Object::Dictionary(dests)) =
                catalogue.get(b"Dests").map(|o| resoudre(document, o))
            {
                for (nom, valeur) in dests.iter() {
                    nommees.insert(nom.clone(), valeur);
                }
            }
            if let Ok(Object::Dictionary(noms)) =
                catalogue.get(b"Names").map(|o| resoudre(document, o))
                && let Ok(Object::Dictionary(arbre)) =
                    noms.get(b"Dests").map(|o| resoudre(document, o))
            {
                arbre_de_noms(document, arbre, &mut nommees, 0);
            }
        }
        Lecteur {
            document,
            pages,
            nommees,
        }
    }

    fn signets(&self) -> Vec<Signet> {
        let mut signets = Vec::new();
        let Ok(catalogue) = self.document.catalog() else {
            return signets;
        };
        if let Ok(Object::Dictionary(racine)) = catalogue
            .get(b"Outlines")
            .map(|o| resoudre(self.document, o))
        {
            let mut vus = HashSet::new();
            self.parcourir(reference(racine, b"First"), 0, &mut vus, &mut signets);
        }
        signets
    }

    /// Les signets d'un niveau, puis, sous chacun, ceux du niveau suivant.
    fn parcourir(
        &self,
        premier: Option<ObjectId>,
        niveau: usize,
        vus: &mut HashSet<ObjectId>,
        signets: &mut Vec<Signet>,
    ) {
        let mut courant = premier;
        while let Some(id) = courant {
            if niveau > PROFONDEUR_MAX || signets.len() >= SIGNETS_MAX || !vus.insert(id) {
                return;
            }
            let Ok(element) = self.document.get_dictionary(id) else {
                return;
            };
            let titre = element
                .get(b"Title")
                .ok()
                .map(|o| resoudre(self.document, o))
                .and_then(|o| lopdf::decode_text_string(o).ok())
                .map(|titre| propre(&titre))
                .unwrap_or_default();
            let (destination, adresse) = self.cible(element);
            let ouvert = element
                .get(b"Count")
                .ok()
                .map(|o| resoudre(self.document, o))
                .and_then(|o| o.as_i64().ok())
                .is_some_and(|n| n > 0);
            signets.push(Signet {
                titre,
                niveau,
                destination,
                adresse,
                ouvert,
            });
            self.parcourir(reference(element, b"First"), niveau + 1, vus, signets);
            courant = reference(element, b"Next");
        }
    }

    fn liens(&self, id: ObjectId) -> Vec<Lien> {
        let Ok(page) = self.document.get_dictionary(id) else {
            return Vec::new();
        };
        let Ok(Object::Array(annotations)) =
            page.get(b"Annots").map(|o| resoudre(self.document, o))
        else {
            return Vec::new();
        };
        let cadre = Cadre::de_page(self.document, page);
        let vers_vue = cadre.vers_vue();
        let (_, hauteur_page) = cadre.taille_vue();
        let mut liens = Vec::new();
        for annotation in annotations.iter().take(LIENS_MAX) {
            let Object::Dictionary(annotation) = resoudre(self.document, annotation) else {
                continue;
            };
            let sous_type = annotation
                .get(b"Subtype")
                .ok()
                .map(|o| resoudre(self.document, o))
                .and_then(|o| o.as_name().ok());
            if sous_type != Some(b"Link".as_slice()) {
                continue;
            }
            let Some([x0, y0, x1, y1]) = annotation
                .get(b"Rect")
                .ok()
                .and_then(|r| rectangle(self.document, r))
            else {
                continue;
            };
            let [gauche, bas, droite, haut] =
                boite(vers_vue, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            if droite - gauche < 1.0 || haut - bas < 1.0 {
                continue;
            }
            let (destination, adresse) = self.cible(annotation);
            if destination.is_none() && adresse.is_none() {
                continue;
            }
            liens.push(Lien {
                x: gauche,
                y: hauteur_page - haut,
                largeur: droite - gauche,
                hauteur: haut - bas,
                destination,
                adresse,
            });
        }
        liens
    }

    /// Où mène un signet ou un lien : sa destination (`/Dest`) ou son action (`/A`).
    fn cible(&self, element: &Dictionary) -> (Option<Destination>, Option<String>) {
        if let Ok(destination) = element.get(b"Dest") {
            return (self.destination(destination, 0), None);
        }
        let Ok(Object::Dictionary(action)) = element.get(b"A").map(|o| resoudre(self.document, o))
        else {
            return (None, None);
        };
        let genre = action
            .get(b"S")
            .ok()
            .map(|o| resoudre(self.document, o))
            .and_then(|o| o.as_name().ok());
        match genre {
            Some(b"GoTo") => (
                action.get(b"D").ok().and_then(|d| self.destination(d, 0)),
                None,
            ),
            Some(b"URI") => (
                None,
                action
                    .get(b"URI")
                    .ok()
                    .map(|o| resoudre(self.document, o))
                    .and_then(|o| o.as_str().ok())
                    .and_then(|octets| adresse_sure(&String::from_utf8_lossy(octets))),
            ),
            // Autre document, programme, script… : jamais depuis le lecteur.
            _ => (None, None),
        }
    }

    fn destination(&self, objet: &Object, profondeur: usize) -> Option<Destination> {
        if profondeur > 4 {
            return None;
        }
        match resoudre(self.document, objet) {
            Object::Array(valeurs) => self.destination_explicite(valeurs),
            Object::Name(nom) | Object::String(nom, _) => self
                .nommees
                .get(nom)
                .and_then(|cible| self.destination(cible, profondeur + 1)),
            Object::Dictionary(dictionnaire) => dictionnaire
                .get(b"D")
                .ok()
                .and_then(|d| self.destination(d, profondeur + 1)),
            _ => None,
        }
    }

    /// `[page /XYZ gauche haut zoom]`, `[page /FitH haut]`, `[page /FitR gauche bas droite haut]`,
    /// `[page /Fit]`… : la page, et la hauteur visée quand il y en a une.
    fn destination_explicite(&self, valeurs: &[Object]) -> Option<Destination> {
        let id = valeurs.first()?.as_reference().ok()?;
        let page = *self.pages.get(&id)?;
        let nombre = |rang: usize| -> Option<f64> {
            let valeur = resoudre(self.document, valeurs.get(rang)?);
            valeur.as_float().ok().map(f64::from)
        };
        let ajustement = valeurs
            .get(1)
            .map(|o| resoudre(self.document, o))
            .and_then(|o| o.as_name().ok())
            .unwrap_or(b"");
        let (gauche, haut) = match ajustement {
            b"XYZ" => (nombre(2), nombre(3)),
            b"FitH" | b"FitBH" => (None, nombre(2)),
            b"FitR" => (nombre(2), nombre(5)),
            _ => (None, None),
        };
        let y = haut.and_then(|haut| {
            let dictionnaire = self.document.get_dictionary(id).ok()?;
            let cadre = Cadre::de_page(self.document, dictionnaire);
            let (_, y_vue) = cadre
                .vers_vue()
                .point(gauche.unwrap_or(cadre.boite[0]), haut);
            let (_, hauteur) = cadre.taille_vue();
            Some((hauteur - y_vue).clamp(0.0, hauteur))
        });
        Some(Destination { page, y })
    }
}

/// Une référence d'un dictionnaire (`/First`, `/Next`).
fn reference(dictionnaire: &Dictionary, cle: &[u8]) -> Option<ObjectId> {
    dictionnaire
        .get(cle)
        .ok()
        .and_then(|o| o.as_reference().ok())
}

/// Les feuilles d'un arbre de noms : `/Names [clé valeur …]`, et les branches `/Kids`.
fn arbre_de_noms<'a>(
    document: &'a Document,
    noeud: &'a Dictionary,
    sortie: &mut HashMap<Vec<u8>, &'a Object>,
    profondeur: usize,
) {
    if profondeur > PROFONDEUR_MAX {
        return;
    }
    if let Ok(Object::Array(noms)) = noeud.get(b"Names").map(|o| resoudre(document, o)) {
        for paire in noms.chunks(2) {
            if let [cle, valeur] = paire
                && let Ok(cle) = resoudre(document, cle).as_str()
            {
                sortie.insert(cle.to_vec(), valeur);
            }
        }
    }
    if let Ok(Object::Array(enfants)) = noeud.get(b"Kids").map(|o| resoudre(document, o)) {
        for enfant in enfants {
            if let Object::Dictionary(enfant) = resoudre(document, enfant) {
                arbre_de_noms(document, enfant, sortie, profondeur + 1);
            }
        }
    }
}

/// Titre d'un signet sur une ligne, sans caractères de contrôle, de longueur raisonnable.
fn propre(titre: &str) -> String {
    let une_ligne: String = titre
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    une_ligne
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(TITRE_MAX)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::{pdf_avec_lien, pdf_herite};
    use lopdf::{StringFormat, dictionary};

    /// Trois pages ; signets : « Introduction » (page 1, à 100 pt du haut) et son sous-signet en
    /// UTF-16 vers une destination nommée (page 2) ; « Annexe » par une action vers un nom de
    /// l'arbre (page 3, à 400 pt du haut) ; « Site » vers une page web ; « Script », refusé.
    fn pdf_avec_signets() -> Vec<u8> {
        let mut document = Document::load_mem(&pdf_herite(3, "Page")).unwrap();
        let pages = document.get_pages();
        let (p1, p2, p3) = (pages[&1], pages[&2], pages[&3]);
        let racine = document.new_object_id();
        let introduction = document.new_object_id();
        let details = document.new_object_id();
        let annexe = document.new_object_id();
        let site = document.new_object_id();
        let script = document.new_object_id();
        let titre_utf16: Vec<u8> = [0xFE, 0xFF]
            .into_iter()
            .chain("Détails".encode_utf16().flat_map(|u| u.to_be_bytes()))
            .collect();
        let chaine = |texte: &str| Object::String(texte.as_bytes().to_vec(), StringFormat::Literal);
        document.objects.insert(
            racine,
            Object::Dictionary(dictionary! { "Type" => "Outlines", "First" => introduction, "Last" => script, "Count" => 5 }),
        );
        document.objects.insert(
            introduction,
            Object::Dictionary(dictionary! {
                "Title" => chaine("Introduction\n"), "Parent" => racine, "Next" => annexe,
                "First" => details, "Last" => details, "Count" => 1,
                "Dest" => vec![Object::Reference(p1), "XYZ".into(), 0.into(), 742.into(), Object::Null],
            }),
        );
        document.objects.insert(
            details,
            Object::Dictionary(dictionary! {
                "Title" => Object::String(titre_utf16, StringFormat::Hexadecimal), "Parent" => introduction,
                "Dest" => Object::Name(b"chap2".to_vec()),
            }),
        );
        document.objects.insert(
            annexe,
            Object::Dictionary(dictionary! {
                "Title" => chaine("Annexe"), "Parent" => racine, "Prev" => introduction, "Next" => site,
                "A" => dictionary! { "S" => "GoTo", "D" => chaine("annexe") },
            }),
        );
        document.objects.insert(
            site,
            Object::Dictionary(dictionary! {
                "Title" => chaine("Site"), "Parent" => racine, "Prev" => annexe, "Next" => script,
                "A" => dictionary! { "S" => "URI", "URI" => chaine("https://www.sitech-global.com/fr") },
            }),
        );
        document.objects.insert(
            script,
            Object::Dictionary(dictionary! {
                "Title" => chaine("Script"), "Parent" => racine, "Prev" => site,
                "A" => dictionary! { "S" => "URI", "URI" => chaine("javascript:alert(1)") },
            }),
        );
        let catalogue = document
            .trailer
            .get(b"Root")
            .unwrap()
            .as_reference()
            .unwrap();
        let catalogue = document.get_dictionary_mut(catalogue).unwrap();
        catalogue.set("Outlines", racine);
        catalogue.set(
            "Dests",
            dictionary! { "chap2" => vec![Object::Reference(p2), "Fit".into()] },
        );
        catalogue.set(
            "Names",
            dictionary! { "Dests" => dictionary! {
                "Names" => vec![chaine("annexe"), vec![Object::Reference(p3), "FitH".into(), 442.into()].into()],
            } },
        );
        let mut octets = Vec::new();
        document.save_to(&mut octets).unwrap();
        octets
    }

    /// Un signet en résumé : titre, niveau, destination, adresse, déplié.
    type Resume<'a> = (&'a str, usize, Option<Destination>, Option<&'a str>, bool);

    fn charger(octets: &[u8]) -> Document {
        crate::pdf::charger(octets, "essai.pdf").unwrap()
    }

    #[test]
    fn lit_les_signets_leurs_niveaux_et_leurs_destinations() {
        let navigation = navigation(&charger(&pdf_avec_signets()));
        let resume: Vec<Resume> = navigation
            .signets
            .iter()
            .map(|s| {
                (
                    s.titre.as_str(),
                    s.niveau,
                    s.destination.clone(),
                    s.adresse.as_deref(),
                    s.ouvert,
                )
            })
            .collect();
        assert_eq!(
            resume,
            vec![
                (
                    "Introduction",
                    0,
                    Some(Destination {
                        page: 0,
                        y: Some(100.0)
                    }),
                    None,
                    true
                ),
                (
                    "Détails",
                    1,
                    Some(Destination { page: 1, y: None }),
                    None,
                    false
                ),
                (
                    "Annexe",
                    0,
                    Some(Destination {
                        page: 2,
                        y: Some(400.0)
                    }),
                    None,
                    false
                ),
                (
                    "Site",
                    0,
                    None,
                    Some("https://www.sitech-global.com/fr"),
                    false
                ),
                ("Script", 0, None, None, false),
            ]
        );
    }

    #[test]
    fn place_un_lien_interne_comme_les_mots() {
        let navigation = navigation(&charger(&pdf_avec_lien(3, "Chapitre")));
        assert!(navigation.signets.is_empty());
        assert_eq!(navigation.liens.len(), 3);
        // Rectangle [72 700 300 740] sur une page de 842 pt de haut ; destination en haut de la page 3.
        assert_eq!(
            navigation.liens[0],
            vec![Lien {
                x: 72.0,
                y: 102.0,
                largeur: 228.0,
                hauteur: 40.0,
                destination: Some(Destination {
                    page: 2,
                    y: Some(0.0)
                }),
                adresse: None,
            }]
        );
        assert!(navigation.liens[1].is_empty() && navigation.liens[2].is_empty());
    }

    #[test]
    fn n_ouvre_que_les_pages_web_et_les_e_mails() {
        assert_eq!(
            adresse_sure("https://www.sitech-global.com/fr?a=1&b=2").as_deref(),
            Some("https://www.sitech-global.com/fr?a=1&b=2")
        );
        assert_eq!(
            adresse_sure(" mailto:contact@exemple.com ").as_deref(),
            Some("mailto:contact@exemple.com")
        );
        assert_eq!(
            adresse_sure("www.exemple.com").as_deref(),
            Some("http://www.exemple.com")
        );
        for refuse in [
            "javascript:alert(1)",
            "file:///C:/Windows/System32/calc.exe",
            "C:\\Windows\\System32\\calc.exe",
            "https://exemple.com/a b",
            "https://exemple.com/\"&calc",
            "https://",
            "",
        ] {
            assert_eq!(adresse_sure(refuse), None, "{refuse}");
        }
    }

    #[test]
    fn ne_boucle_pas_sur_des_signets_qui_se_suivent_en_rond() {
        let mut document = charger(&pdf_avec_signets());
        // « Script » renvoie à « Introduction » : le parcours s'arrête au premier signet déjà vu.
        let script = document
            .objects
            .iter()
            .find(|(_, objet)| matches!(objet, Object::Dictionary(d) if d.get(b"Title").ok().and_then(|t| t.as_str().ok()) == Some(b"Script".as_slice())))
            .map(|(id, _)| *id)
            .unwrap();
        let premier = {
            let catalogue = document.catalog().unwrap();
            let racine = catalogue.get(b"Outlines").unwrap().as_reference().unwrap();
            document
                .get_dictionary(racine)
                .unwrap()
                .get(b"First")
                .unwrap()
                .as_reference()
                .unwrap()
        };
        document
            .get_dictionary_mut(script)
            .unwrap()
            .set("Next", Object::Reference(premier));
        assert_eq!(navigation(&document).signets.len(), 5);
    }
}
