//! Commentaires déjà présents dans un PDF reçu (0.6.0) : les lister pour le lecteur (sorte, auteur,
//! date, texte, place sur la page affichée), et les retoucher : en supprimer, en changer le texte.
//!
//! Les retouches se font sur la copie en mémoire du PDF (`memoire::retoucher`), toujours depuis
//! son contenu d'origine : le lecteur les montre aussitôt, le PDF annoté enregistré (et
//! l'impression) les garde, et l'original n'est jamais modifié.

use std::collections::{HashMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::{Deserialize, Serialize};

use crate::annotations::texte_pdf;
use crate::geometrie::{Cadre, boite, rectangle, resoudre};

/// Commentaires lus au plus : un PDF qui en aurait des milliers reste lisible.
const COMMENTAIRES_MAX: usize = 5000;
/// Longueur gardée d'un texte de commentaire, en caractères.
const TEXTE_MAX: usize = 4000;
/// Les annotations qui ne sont pas des commentaires : liens, champs de formulaire, bulles
/// (« Popup », la fenêtre d'un commentaire), repères d'imprimerie, filigranes.
const PAS_DES_COMMENTAIRES: [&[u8]; 7] = [
    b"Link",
    b"Widget",
    b"Popup",
    b"PrinterMark",
    b"TrapNet",
    b"Watermark",
    b"3D",
];
/// Indicateur « Hidden » d'une annotation : ni affichée, ni imprimée.
const CACHEE: i64 = 2;

/// Un commentaire du PDF, pour la liste du lecteur ; place en points de la page affichée (depuis
/// son coin en haut à gauche, comme les mots et les annotations du lecteur).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commentaire {
    /// Repère stable dans le PDF d'origine : « 12 0 » (l'objet de l'annotation), ou « p2-5 » (la
    /// sixième annotation de la page 3, écrite dans la page même).
    pub id: String,
    /// À partir de 0.
    pub page: usize,
    /// Sorte PDF : Text (note), Highlight, Underline, StrikeOut, Squiggly, FreeText, Square,
    /// Circle, Line, Polygon, PolyLine, Ink, Stamp, Caret, FileAttachment…
    pub sorte: String,
    pub x: f64,
    pub y: f64,
    pub largeur: f64,
    pub hauteur: f64,
    pub texte: String,
    pub auteur: String,
    /// « 24/11/2026 09:15 », d'après la date de modification (ou de création).
    pub date: Option<String>,
    pub couleur: Option<[u8; 3]>,
    /// Réponse à un autre commentaire (/IRT).
    pub reponse: bool,
    /// Son texte peut changer sans redessiner le commentaire : tous, sauf le texte posé sur la
    /// page (FreeText), dont l'apparence montre le texte.
    pub modifiable: bool,
}

/// Les retouches demandées par le lecteur, toujours appliquées au PDF d'origine.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Retouches {
    /// Repères des commentaires supprimés (leur bulle et leurs réponses partent avec eux).
    pub supprimes: Vec<String>,
    /// Nouveau texte de commentaires : (repère, texte).
    pub textes: Vec<(String, String)>,
}

impl Retouches {
    pub fn vides(&self) -> bool {
        self.supprimes.is_empty() && self.textes.is_empty()
    }
}

fn repere_objet((numero, generation): ObjectId) -> String {
    format!("{numero} {generation}")
}

/// Le repère d'une annotation, d'après sa place dans le tableau /Annots de sa page.
fn repere(entree: &Object, page: usize, position: usize) -> String {
    match entree {
        Object::Reference(id) => repere_objet(*id),
        _ => format!("p{page}-{position}"),
    }
}

/// Le tableau /Annots d'une page (écrit dans la page, ou objet à part).
fn annotations_de<'a>(document: &'a Document, page: &'a Dictionary) -> Option<&'a Vec<Object>> {
    page.get(b"Annots")
        .ok()
        .map(|o| resoudre(document, o))
        .and_then(|o| o.as_array().ok())
}

fn nom<'a>(document: &'a Document, dictionnaire: &'a Dictionary, cle: &[u8]) -> Option<&'a [u8]> {
    dictionnaire
        .get(cle)
        .ok()
        .map(|o| resoudre(document, o))
        .and_then(|o| o.as_name().ok())
}

/// Un texte de l'annotation (contenu, auteur), d'une ligne par paragraphe, sans caractères de contrôle.
fn texte(document: &Document, dictionnaire: &Dictionary, cle: &[u8]) -> String {
    let brut = dictionnaire
        .get(cle)
        .ok()
        .map(|o| resoudre(document, o))
        .and_then(|o| lopdf::decode_text_string(o).ok())
        .unwrap_or_default();
    brut.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|c| *c == '\n' || !c.is_control())
        .take(TEXTE_MAX)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Une date PDF (« D:20261124091500+01'00' ») écrite pour l'utilisateur : « 24/11/2026 09:15 ».
fn date_lisible(date: &str) -> Option<String> {
    let chiffres: String = date
        .trim()
        .trim_start_matches("D:")
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if chiffres.len() < 8 {
        return None;
    }
    let (annee, mois, jour) = (&chiffres[0..4], &chiffres[4..6], &chiffres[6..8]);
    let valide =
        |valeur: &str, max: u32| valeur.parse::<u32>().is_ok_and(|n| (1..=max).contains(&n));
    if !valide(mois, 12) || !valide(jour, 31) {
        return None;
    }
    let mut lisible = format!("{jour}/{mois}/{annee}");
    if chiffres.len() >= 12 {
        lisible.push_str(&format!(" {}:{}", &chiffres[8..10], &chiffres[10..12]));
    }
    Some(lisible)
}

fn date(document: &Document, dictionnaire: &Dictionary) -> Option<String> {
    [b"M".as_slice(), b"CreationDate".as_slice()]
        .into_iter()
        .find_map(|cle| {
            let brut = dictionnaire.get(cle).ok().map(|o| resoudre(document, o))?;
            date_lisible(&lopdf::decode_text_string(brut).ok()?)
        })
}

/// La couleur (/C) : gris, RVB ou CMJN, de 0 à 1.
fn couleur(document: &Document, dictionnaire: &Dictionary) -> Option<[u8; 3]> {
    let valeurs: Vec<f64> = dictionnaire
        .get(b"C")
        .ok()
        .map(|o| resoudre(document, o))?
        .as_array()
        .ok()?
        .iter()
        .map(|v| {
            resoudre(document, v)
                .as_float()
                .ok()
                .map(|f| f64::from(f).clamp(0.0, 1.0))
        })
        .collect::<Option<_>>()?;
    let octet = |v: f64| (v * 255.0).round() as u8;
    match valeurs.as_slice() {
        [gris] => Some([octet(*gris); 3]),
        [r, v, b] => Some([octet(*r), octet(*v), octet(*b)]),
        [c, m, j, n] => Some([
            octet((1.0 - c) * (1.0 - n)),
            octet((1.0 - m) * (1.0 - n)),
            octet((1.0 - j) * (1.0 - n)),
        ]),
        _ => None,
    }
}

/// Les commentaires du PDF, page après page, dans l'ordre de leur page.
pub fn commentaires(document: &Document) -> Vec<Commentaire> {
    let mut liste = Vec::new();
    for (rang, (_, id_page)) in document.get_pages().into_iter().enumerate() {
        let Ok(page) = document.get_dictionary(id_page) else {
            continue;
        };
        let Some(annotations) = annotations_de(document, page) else {
            continue;
        };
        let cadre = Cadre::de_page(document, page);
        let vers_vue = cadre.vers_vue();
        let (_, hauteur_page) = cadre.taille_vue();
        for (position, entree) in annotations.iter().enumerate() {
            if liste.len() >= COMMENTAIRES_MAX {
                return liste;
            }
            let Object::Dictionary(annotation) = resoudre(document, entree) else {
                continue;
            };
            let Some(sorte) = nom(document, annotation, b"Subtype") else {
                continue;
            };
            if PAS_DES_COMMENTAIRES.contains(&sorte) {
                continue;
            }
            let drapeaux = annotation
                .get(b"F")
                .ok()
                .map(|o| resoudre(document, o))
                .and_then(|o| o.as_i64().ok())
                .unwrap_or(0);
            if drapeaux & CACHEE != 0 {
                continue;
            }
            let Some([x0, y0, x1, y1]) = annotation
                .get(b"Rect")
                .ok()
                .and_then(|r| rectangle(document, r))
            else {
                continue;
            };
            let [gauche, bas, droite, haut] =
                boite(vers_vue, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            liste.push(Commentaire {
                id: repere(entree, rang, position),
                page: rang,
                sorte: String::from_utf8_lossy(sorte).into_owned(),
                x: gauche,
                y: hauteur_page - haut,
                largeur: droite - gauche,
                hauteur: haut - bas,
                texte: texte(document, annotation, b"Contents"),
                auteur: texte(document, annotation, b"T"),
                date: date(document, annotation),
                couleur: couleur(document, annotation),
                reponse: annotation.has(b"IRT"),
                modifiable: sorte != b"FreeText",
            });
        }
    }
    liste
}

/// Applique les retouches : commentaires supprimés (avec leur bulle et, d'une génération à
/// l'autre, les réponses qui leur sont faites), textes changés. Les autres annotations (liens,
/// champs de formulaire) restent telles quelles.
pub fn retoucher(document: &mut Document, retouches: &Retouches) {
    let textes: HashMap<&str, &str> = retouches
        .textes
        .iter()
        .map(|(id, texte)| (id.as_str(), texte.as_str()))
        .collect();
    let mut supprimes: HashSet<String> = retouches.supprimes.iter().cloned().collect();
    let pages: Vec<ObjectId> = document.get_pages().into_values().collect();

    // Les réponses à un commentaire supprimé partent avec lui (et les réponses à ces réponses).
    loop {
        let mut ajoutes = false;
        for &id_page in &pages {
            let Ok(page) = document.get_dictionary(id_page) else {
                continue;
            };
            for entree in annotations_de(document, page).into_iter().flatten() {
                let (Object::Reference(id), Object::Dictionary(annotation)) =
                    (entree, resoudre(document, entree))
                else {
                    continue;
                };
                let parent = annotation
                    .get(b"IRT")
                    .ok()
                    .and_then(|o| o.as_reference().ok());
                if parent.is_some_and(|parent| supprimes.contains(&repere_objet(parent)))
                    && supprimes.insert(repere_objet(*id))
                {
                    ajoutes = true;
                }
            }
        }
        if !ajoutes {
            break;
        }
    }

    for (rang, &id_page) in pages.iter().enumerate() {
        let Ok(page) = document.get_dictionary(id_page) else {
            continue;
        };
        let Some(annotations) = annotations_de(document, page).cloned() else {
            continue;
        };
        let tableau_a_part = page.get(b"Annots").ok().and_then(|o| o.as_reference().ok());
        // Les bulles des commentaires supprimés.
        let bulles: HashSet<ObjectId> = annotations
            .iter()
            .enumerate()
            .filter(|(position, entree)| supprimes.contains(&repere(entree, rang, *position)))
            .filter_map(|(_, entree)| match resoudre(document, entree) {
                Object::Dictionary(annotation) => annotation
                    .get(b"Popup")
                    .ok()
                    .and_then(|o| o.as_reference().ok()),
                _ => None,
            })
            .collect();
        let mut gardees = Vec::with_capacity(annotations.len());
        for (position, entree) in annotations.iter().enumerate() {
            let id = repere(entree, rang, position);
            if supprimes.contains(&id)
                || matches!(entree, Object::Reference(objet) if bulles.contains(objet))
            {
                continue;
            }
            let mut entree = entree.clone();
            if let Some(nouveau) = textes.get(id.as_str()) {
                match &mut entree {
                    Object::Reference(objet) => {
                        if let Ok(annotation) = document.get_dictionary_mut(*objet) {
                            annotation.set("Contents", texte_pdf(nouveau));
                        }
                    }
                    Object::Dictionary(annotation) => {
                        annotation.set("Contents", texte_pdf(nouveau))
                    }
                    _ => {}
                }
            }
            gardees.push(entree);
        }
        if gardees.len() == annotations.len() {
            continue;
        }
        match tableau_a_part {
            Some(tableau) => {
                document.objects.insert(tableau, Object::Array(gardees));
            }
            None => {
                if let Ok(page) = document.get_dictionary_mut(id_page) {
                    page.set("Annots", Object::Array(gardees));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use lopdf::{StringFormat, dictionary};

    use super::*;
    use crate::essais::pdf_herite;

    /// Un PDF « reçu » : sur la première page, une note avec sa bulle et une réponse, un
    /// surlignage avec sa bulle, un texte posé écrit dans la page, un lien et un commentaire caché ;
    /// une rature sur la seconde page.
    fn pdf_commente() -> Document {
        let mut document = crate::pdf::charger(&pdf_herite(2, "Facture"), "facture.pdf").unwrap();
        let pages: Vec<ObjectId> = document.get_pages().into_values().collect();
        let chaine = |texte: &str| Object::String(texte.as_bytes().to_vec(), StringFormat::Literal);
        let note = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Text", "Rect" => vec![500.into(), 700.into(), 520.into(), 720.into()],
            "Contents" => texte_pdf("À vérifier : le montant"), "T" => texte_pdf("Comptabilité"),
            "M" => chaine("D:20261124091500+01'00'"), "C" => vec![1.into(), Object::Real(0.82), Object::Real(0.2)],
        });
        let bulle_note = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Popup", "Rect" => vec![520.into(), 600.into(), 590.into(), 700.into()], "Parent" => note,
        });
        document
            .get_dictionary_mut(note)
            .unwrap()
            .set("Popup", bulle_note);
        let reponse = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Text", "Rect" => vec![500.into(), 700.into(), 520.into(), 720.into()],
            "Contents" => texte_pdf("C'est vérifié"), "T" => chaine("Direction"), "IRT" => note,
        });
        let surlignage = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Highlight", "Rect" => vec![72.into(), 715.into(), 300.into(), 745.into()],
            "Contents" => texte_pdf("Net à payer"), "CreationDate" => chaine("D:20261123"),
        });
        let bulle_surlignage = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Popup", "Rect" => vec![300.into(), 600.into(), 400.into(), 700.into()], "Parent" => surlignage,
        });
        document
            .get_dictionary_mut(surlignage)
            .unwrap()
            .set("Popup", bulle_surlignage);
        let lien = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Link", "Rect" => vec![72.into(), 100.into(), 200.into(), 120.into()],
        });
        let cache = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Square", "Rect" => vec![10.into(), 10.into(), 50.into(), 50.into()], "F" => 2,
        });
        let pose = Object::Dictionary(dictionary! {
            "Type" => "Annot", "Subtype" => "FreeText", "Rect" => vec![100.into(), 400.into(), 200.into(), 420.into()],
            "Contents" => texte_pdf("Payé"),
        });
        document.get_dictionary_mut(pages[0]).unwrap().set(
            "Annots",
            vec![
                note.into(),
                bulle_note.into(),
                reponse.into(),
                surlignage.into(),
                bulle_surlignage.into(),
                pose,
                lien.into(),
                cache.into(),
            ],
        );
        // Sur la seconde page, le tableau /Annots est un objet à part.
        let rature = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "StrikeOut", "Rect" => vec![72.into(), 715.into(), 200.into(), 745.into()],
            "C" => vec![Object::Real(0.0), Object::Real(1.0), Object::Real(1.0), Object::Real(0.0)],
        });
        let tableau = document.add_object(Object::Array(vec![rature.into()]));
        document
            .get_dictionary_mut(pages[1])
            .unwrap()
            .set("Annots", tableau);
        document
    }

    #[test]
    fn lit_les_commentaires_d_un_pdf_recu() {
        let liste = commentaires(&pdf_commente());
        let sortes: Vec<(usize, &str)> = liste.iter().map(|c| (c.page, c.sorte.as_str())).collect();
        assert_eq!(
            sortes,
            vec![
                (0, "Text"),
                (0, "Text"),
                (0, "Highlight"),
                (0, "FreeText"),
                (1, "StrikeOut")
            ],
            "ni bulles, ni lien, ni commentaire caché"
        );
        let note = &liste[0];
        assert_eq!(note.texte, "À vérifier : le montant");
        assert_eq!(note.auteur, "Comptabilité");
        assert_eq!(note.date.as_deref(), Some("24/11/2026 09:15"));
        assert_eq!(note.couleur, Some([255, 209, 51]));
        // Rect [500 700 520 720] sur une page de 842 points : 122 points sous son haut.
        assert_eq!(
            (note.x, note.y, note.largeur, note.hauteur),
            (500.0, 122.0, 20.0, 20.0)
        );
        assert!(!note.reponse && liste[1].reponse);
        assert_eq!(liste[2].date.as_deref(), Some("23/11/2026"));
        assert_eq!(
            liste[3].id, "p0-5",
            "écrit dans la page : repéré par sa place"
        );
        assert!(
            !liste[3].modifiable,
            "le texte posé se montre par son apparence"
        );
        assert_eq!(liste[4].couleur, Some([255, 0, 0]), "CMJN");
    }

    #[test]
    fn supprime_un_commentaire_avec_sa_bulle_et_ses_reponses_et_change_un_texte() {
        let mut document = pdf_commente();
        let avant = commentaires(&document);
        let (note, surlignage, pose) = (
            avant[0].id.clone(),
            avant[2].id.clone(),
            avant[3].id.clone(),
        );
        retoucher(
            &mut document,
            &Retouches {
                supprimes: vec![note, pose],
                textes: vec![(surlignage.clone(), "Net à payer : vérifié".into())],
            },
        );
        let apres = commentaires(&document);
        let restes: Vec<(&str, &str)> = apres
            .iter()
            .map(|c| (c.sorte.as_str(), c.texte.as_str()))
            .collect();
        assert_eq!(
            restes,
            vec![("Highlight", "Net à payer : vérifié"), ("StrikeOut", "")]
        );
        assert_eq!(
            apres[0].id, surlignage,
            "les repères des objets ne changent pas"
        );
        // Sur la première page restent le surlignage, sa bulle, le lien et le commentaire caché.
        let premiere = document.get_pages().into_values().next().unwrap();
        let tableau = document
            .get_dictionary(premiere)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(tableau.len(), 4);
    }

    #[test]
    fn retouche_aussi_un_tableau_d_annotations_a_part() {
        let mut document = pdf_commente();
        let rature = commentaires(&document)[4].id.clone();
        retoucher(
            &mut document,
            &Retouches {
                supprimes: vec![rature],
                textes: vec![],
            },
        );
        assert_eq!(
            commentaires(&document)
                .iter()
                .filter(|c| c.page == 1)
                .count(),
            0
        );
        assert!(Retouches::default().vides());
    }

    #[test]
    fn ecrit_les_dates_pour_l_utilisateur() {
        assert_eq!(
            date_lisible("D:20261124091500+01'00'").as_deref(),
            Some("24/11/2026 09:15")
        );
        assert_eq!(date_lisible("20261124").as_deref(), Some("24/11/2026"));
        assert_eq!(date_lisible("D:2026"), None);
        assert_eq!(date_lisible("D:20261399"), None);
    }
}
