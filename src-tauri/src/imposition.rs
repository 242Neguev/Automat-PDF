//! Plusieurs pages par feuille (2 ou 4), livret à plier, ou pages remises au format A4. Chaque page
//! devient un « formulaire » PDF (son contenu et ses ressources), placé à l'échelle dans une case de
//! la feuille : rien n'est recalculé, texte et images restent nets. Les annotations visibles (champs
//! de formulaire remplis, tampons) sont incrustées dans la page, pour ne pas disparaître.

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};
use serde::Deserialize;

use crate::erreur::Erreur;
use crate::geometrie::{A4, Cadre, Matrice, rectangle, resoudre};
use crate::pdf::{charger, contenu_page};

/// Marge autour des pages d'une feuille, et espace entre elles (points).
const MARGE: f64 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disposition {
    /// Deux pages côte à côte sur une feuille A4 en paysage.
    DeuxParFeuille,
    /// Quatre pages sur une feuille A4 en portrait.
    QuatreParFeuille,
    /// Livret : feuilles A4 en paysage à imprimer recto verso, puis à plier en deux.
    Livret,
    /// Chaque page mise à l'échelle sur une feuille A4 (portrait ou paysage selon la page).
    FormatA4,
}

/// Case d'une feuille : x, y (coin bas gauche), largeur et hauteur, en points.
type Case = (f64, f64, f64, f64);

/// Une page devenue formulaire : son identifiant et son cadre (partie visible, rotation).
struct PageSource {
    forme: ObjectId,
    cadre: Cadre,
}

impl PageSource {
    fn taille_vue(&self) -> (f64, f64) {
        self.cadre.taille_vue()
    }

    /// Matrice qui place la page, redressée et à l'échelle, au centre de la case (x, y, l, h).
    fn placement(&self, (x, y, l, h): Case) -> Matrice {
        let (vue_l, vue_h) = self.taille_vue();
        let echelle = (l / vue_l).min(h / vue_h);
        self.cadre
            .vers_vue()
            .puis(Matrice::echelle(echelle, echelle))
            .puis(Matrice::translation(
                x + (l - echelle * vue_l) / 2.0,
                y + (h - echelle * vue_h) / 2.0,
            ))
    }
}

/// Apparence visible d'une annotation (formulaire PDF), s'il faut l'incruster.
fn apparence(document: &Document, annotation: &Dictionary) -> Option<Object> {
    let sous_type = annotation.get(b"Subtype").and_then(Object::as_name).ok()?;
    if sous_type == b"Popup" || sous_type == b"Link" {
        return None;
    }
    // Annotations masquées (bit 2) ou à ne pas afficher à l'écran (bit 6).
    let drapeaux = annotation.get(b"F").and_then(Object::as_i64).unwrap_or(0);
    if drapeaux & (2 | 32) != 0 {
        return None;
    }
    let normale = resoudre(document, annotation.get(b"AP").ok()?)
        .as_dict()
        .ok()?
        .get(b"N")
        .ok()?;
    match resoudre(document, normale) {
        Object::Stream(_) => Some(normale.clone()),
        // Plusieurs états (case cochée ou non) : celui de /AS.
        Object::Dictionary(etats) => {
            let etat = annotation.get(b"AS").and_then(Object::as_name).ok()?;
            etats.get(etat).ok().cloned()
        }
        _ => None,
    }
}

/// Ajoute au contenu de la page le dessin de ses annotations visibles ; leurs apparences rejoignent
/// les ressources (sous des noms qui ne masquent rien).
fn incruster_annotations(
    document: &mut Document,
    page: &Dictionary,
    contenu: &mut Vec<u8>,
    objets_graphiques: &mut Dictionary,
) {
    let Ok(annotations) = page.get(b"Annots").map(|a| resoudre(document, a).clone()) else {
        return;
    };
    let Ok(annotations) = annotations.as_array() else {
        return;
    };
    for (rang, annotation) in annotations.iter().enumerate() {
        let Ok(annotation) = resoudre(document, annotation).as_dict().cloned() else {
            continue;
        };
        let (Some(apparence), Some(cadre)) = (
            apparence(document, &annotation),
            annotation
                .get(b"Rect")
                .ok()
                .and_then(|r| rectangle(document, r)),
        ) else {
            continue;
        };
        let reference = match apparence {
            Object::Reference(id) => id,
            Object::Stream(stream) => document.add_object(stream),
            _ => continue,
        };
        let Ok(forme) = document.get_object(reference).and_then(Object::as_stream) else {
            continue;
        };
        let Some(boite) = forme
            .dict
            .get(b"BBox")
            .ok()
            .and_then(|b| rectangle(document, b))
        else {
            continue;
        };
        let matrice = forme
            .dict
            .get(b"Matrix")
            .and_then(Object::as_array)
            .ok()
            .and_then(|m| Matrice::de_nombres(m))
            .unwrap_or(Matrice::IDENTITE);
        // Boîte de l'apparence une fois sa matrice appliquée, puis mise à la place du cadre.
        let coins = [
            matrice.point(boite[0], boite[1]),
            matrice.point(boite[2], boite[1]),
            matrice.point(boite[0], boite[3]),
            matrice.point(boite[2], boite[3]),
        ];
        let (bx0, by0) = coins
            .iter()
            .fold((f64::MAX, f64::MAX), |(x, y), c| (x.min(c.0), y.min(c.1)));
        let (bx1, by1) = coins
            .iter()
            .fold((f64::MIN, f64::MIN), |(x, y), c| (x.max(c.0), y.max(c.1)));
        if bx1 - bx0 < 1e-6 || by1 - by0 < 1e-6 {
            continue;
        }
        let (sx, sy) = (
            (cadre[2] - cadre[0]) / (bx1 - bx0),
            (cadre[3] - cadre[1]) / (by1 - by0),
        );
        let place = Matrice([sx, 0.0, 0.0, sy, cadre[0] - sx * bx0, cadre[1] - sy * by0]);
        let nom = format!("AutomatAnnot{rang}");
        objets_graphiques.set(nom.as_bytes(), reference);
        contenu.extend_from_slice(format!("\nq {} /{nom} Do Q\n", place.operateur()).as_bytes());
    }
}

/// Une page devient un formulaire : contenu, ressources et annotations visibles.
fn formulaire(
    document: &mut Document,
    page: ObjectId,
    numero: usize,
) -> Result<PageSource, Erreur> {
    let dictionnaire = document
        .get_dictionary(page)
        .map_err(|_| Erreur::simple(format!("La page {numero} n'a pas pu être lue.")))?
        .clone();
    let cadre = Cadre::de_page(document, &dictionnaire);
    let mut contenu = contenu_page(document, &dictionnaire).ok_or_else(|| {
        Erreur::simple(format!(
            "Le contenu de la page {numero} n'a pas pu être lu."
        ))
    })?;
    let mut ressources = dictionnaire
        .get(b"Resources")
        .ok()
        .and_then(|r| resoudre(document, r).as_dict().ok())
        .cloned()
        .unwrap_or_default();
    let mut objets_graphiques = ressources
        .get(b"XObject")
        .ok()
        .and_then(|x| resoudre(document, x).as_dict().ok())
        .cloned()
        .unwrap_or_default();
    incruster_annotations(
        document,
        &dictionnaire,
        &mut contenu,
        &mut objets_graphiques,
    );
    if !objets_graphiques.is_empty() {
        ressources.set("XObject", objets_graphiques);
    }
    let mut forme = dictionary! {
        "Type" => "XObject", "Subtype" => "Form",
        "BBox" => cadre.boite.iter().map(|&v| Object::Real(v as f32)).collect::<Vec<_>>(),
        "Resources" => ressources,
    };
    if let Ok(groupe) = dictionnaire.get(b"Group") {
        forme.set("Group", groupe.clone());
    }
    let mut flux = Stream::new(forme, contenu);
    let _ = flux.compress();
    Ok(PageSource {
        forme: document.add_object(flux),
        cadre,
    })
}

/// Une feuille : sa taille et, pour chaque case, la page (à partir de 0) qui y va, s'il y en a une.
struct Feuille {
    taille: (f64, f64),
    cases: Vec<(Option<usize>, Case)>,
}

/// Les feuilles à produire pour `nombre` pages ; `taille_vue` donne la taille d'une page (format A4).
fn feuilles(
    nombre: usize,
    disposition: Disposition,
    taille_vue: impl Fn(usize) -> (f64, f64),
) -> Vec<Feuille> {
    let paysage = (A4.1, A4.0);
    let deux_cases = |(largeur, hauteur): (f64, f64)| {
        let l = (largeur - 3.0 * MARGE) / 2.0;
        let h = hauteur - 2.0 * MARGE;
        [(MARGE, MARGE, l, h), (2.0 * MARGE + l, MARGE, l, h)]
    };
    match disposition {
        Disposition::DeuxParFeuille => (0..nombre)
            .step_by(2)
            .map(|debut| Feuille {
                taille: paysage,
                cases: deux_cases(paysage)
                    .into_iter()
                    .enumerate()
                    .map(|(i, case)| ((debut + i < nombre).then_some(debut + i), case))
                    .collect(),
            })
            .collect(),
        Disposition::QuatreParFeuille => {
            let (largeur, hauteur) = A4;
            let (l, h) = ((largeur - 3.0 * MARGE) / 2.0, (hauteur - 3.0 * MARGE) / 2.0);
            let positions = [
                (MARGE, 2.0 * MARGE + h),
                (2.0 * MARGE + l, 2.0 * MARGE + h),
                (MARGE, MARGE),
                (2.0 * MARGE + l, MARGE),
            ];
            (0..nombre)
                .step_by(4)
                .map(|debut| Feuille {
                    taille: A4,
                    cases: positions
                        .iter()
                        .enumerate()
                        .map(|(i, &(x, y))| {
                            ((debut + i < nombre).then_some(debut + i), (x, y, l, h))
                        })
                        .collect(),
                })
                .collect()
        }
        Disposition::Livret => {
            // Pages complétées par des pages blanches jusqu'à un multiple de 4 ; feuille i :
            // au recto (dernière, première), au verso (deuxième, avant-dernière)…
            let total = nombre.div_ceil(4).max(1) * 4;
            let page = |i: usize| (i < nombre).then_some(i);
            let [gauche, droite] = deux_cases(paysage);
            (0..total / 4)
                .flat_map(|i| {
                    [
                        (page(total - 1 - 2 * i), page(2 * i)),
                        (page(2 * i + 1), page(total - 2 - 2 * i)),
                    ]
                })
                .map(|(a, b)| Feuille {
                    taille: paysage,
                    cases: vec![(a, gauche), (b, droite)],
                })
                .collect()
        }
        Disposition::FormatA4 => (0..nombre)
            .map(|i| {
                let (l, h) = taille_vue(i);
                let taille = if l > h { paysage } else { A4 };
                Feuille {
                    taille,
                    cases: vec![(Some(i), (0.0, 0.0, taille.0, taille.1))],
                }
            })
            .collect(),
    }
}

/// Le PDF mis en page selon la disposition choisie.
pub fn imposer(octets: &[u8], nom: &str, disposition: Disposition) -> Result<Vec<u8>, Erreur> {
    let mut document = charger(octets, nom)?;
    let pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    if pages.is_empty() {
        return Err(Erreur::simple(format!(
            "« {nom} » ne contient aucune page."
        )));
    }
    let sources = pages
        .iter()
        .enumerate()
        .map(|(i, page)| formulaire(&mut document, *page, i + 1))
        .collect::<Result<Vec<_>, _>>()?;
    let arbre = document.new_object_id();
    let mut enfants = Vec::new();
    for feuille in feuilles(sources.len(), disposition, |i| sources[i].taille_vue()) {
        let mut contenu = String::new();
        let mut objets_graphiques = Dictionary::new();
        for (page, case) in feuille.cases {
            let Some(page) = page else { continue };
            let source = &sources[page];
            contenu.push_str(&format!(
                "q {} /P{page} Do Q\n",
                source.placement(case).operateur()
            ));
            objets_graphiques.set(format!("P{page}").as_bytes(), source.forme);
        }
        let contenu = document.add_object(Stream::new(Dictionary::new(), contenu.into_bytes()));
        let (largeur, hauteur) = feuille.taille;
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => arbre,
            "MediaBox" => vec![0.into(), 0.into(), Object::Real(largeur as f32), Object::Real(hauteur as f32)],
            "Resources" => dictionary! { "XObject" => objets_graphiques },
            "Contents" => contenu,
        });
        enfants.push(Object::Reference(page));
    }
    let nombre = enfants.len() as i64;
    document.objects.insert(
        arbre,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => enfants, "Count" => nombre }),
    );
    let catalogue = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => arbre });
    document.trailer = Dictionary::new();
    document.trailer.set("Root", catalogue);
    document.prune_objects();
    document.renumber_objects();
    document.compress();
    let mut sortie = Vec::new();
    document
        .save_to(&mut sortie)
        .map_err(|erreur| Erreur::simple(format!("Le PDF n'a pas pu être écrit : {erreur}")))?;
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::essais::pdf_herite;
    use crate::rendu;

    /// Taille et contenu de chaque feuille produite.
    fn feuilles_de(pdf: &[u8]) -> Vec<((f32, f32), String)> {
        let document = Document::load_mem(pdf).unwrap();
        document
            .get_pages()
            .values()
            .map(|id| {
                let boite = document
                    .get_dictionary(*id)
                    .unwrap()
                    .get(b"MediaBox")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .clone();
                let contenu = String::from_utf8(document.get_page_content(*id)).unwrap();
                (
                    (boite[2].as_float().unwrap(), boite[3].as_float().unwrap()),
                    contenu,
                )
            })
            .collect()
    }

    /// Image en niveaux de gris de la première feuille, `largeur` pixels de large.
    fn rendu_feuille(pdf: Vec<u8>, largeur: u32) -> image::GrayImage {
        let jpeg = rendu::image_page(Arc::new(pdf), "feuille.pdf", 0, largeur, 95).unwrap();
        image::load_from_memory(&jpeg).unwrap().to_luma8()
    }

    fn pixels_sombres(image: &image::GrayImage, x0: u32, x1: u32) -> usize {
        (0..image.height())
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| image.get_pixel(x, y).0[0] < 128)
            .count()
    }

    /// PDF d'une page, modifiée par `modifier` avant l'enregistrement.
    fn pdf_modifie(modifier: impl FnOnce(&mut Document, ObjectId)) -> Vec<u8> {
        let mut document = Document::load_mem(&pdf_herite(1, "Texte")).unwrap();
        let page = document.get_pages()[&1];
        modifier(&mut document, page);
        let mut octets = Vec::new();
        document.save_to(&mut octets).unwrap();
        octets
    }

    #[test]
    fn met_deux_pages_par_feuille() {
        let pdf = imposer(&pdf_herite(3, "Page"), "p.pdf", Disposition::DeuxParFeuille).unwrap();
        let feuilles = feuilles_de(&pdf);
        assert_eq!(feuilles.len(), 2);
        assert_eq!(feuilles[0].0, (841.89, 595.28));
        assert!(feuilles[0].1.contains("/P0 Do") && feuilles[0].1.contains("/P1 Do"));
        assert!(feuilles[1].1.contains("/P2 Do") && !feuilles[1].1.contains("/P3"));
        // Les deux moitiés de la première feuille sont bien dessinées.
        let image = rendu_feuille(pdf, 400);
        assert!(pixels_sombres(&image, 0, 200) > 10 && pixels_sombres(&image, 200, 400) > 10);
    }

    #[test]
    fn met_quatre_pages_par_feuille_dans_l_ordre_de_lecture() {
        let feuilles = feuilles(5, Disposition::QuatreParFeuille, |_| A4);
        assert_eq!(feuilles.len(), 2);
        let premiere: Vec<_> = feuilles[0]
            .cases
            .iter()
            .map(|(page, (x, y, _, _))| (page.unwrap(), *x < 200.0, *y > 400.0))
            .collect();
        // En haut à gauche, en haut à droite, en bas à gauche, en bas à droite.
        assert_eq!(
            premiere,
            [
                (0, true, true),
                (1, false, true),
                (2, true, false),
                (3, false, false)
            ]
        );
        assert_eq!(
            feuilles[1]
                .cases
                .iter()
                .filter(|(page, _)| page.is_some())
                .count(),
            1
        );
    }

    #[test]
    fn ordonne_les_pages_d_un_livret() {
        let ordre = |n: usize| {
            feuilles(n, Disposition::Livret, |_| A4)
                .iter()
                .map(|feuille| {
                    let pages: Vec<String> = feuille
                        .cases
                        .iter()
                        .map(|(page, _)| page.map_or("-".into(), |p| (p + 1).to_string()))
                        .collect();
                    pages.join(",")
                })
                .collect::<Vec<_>>()
                .join(" / ")
        };
        assert_eq!(ordre(8), "8,1 / 2,7 / 6,3 / 4,5");
        // Complété de pages blanches jusqu'à un multiple de 4.
        assert_eq!(ordre(6), "-,1 / 2,- / 6,3 / 4,5");
        assert_eq!(ordre(1), "-,1 / -,-");
    }

    #[test]
    fn redresse_une_page_tournee_au_format_a4() {
        let pdf = pdf_modifie(|document, page| {
            let page = document.get_dictionary_mut(page).unwrap();
            page.set("MediaBox", vec![0.into(), 0.into(), 420.into(), 595.into()]);
            page.set("Rotate", 90);
        });
        let feuilles = feuilles_de(&imposer(&pdf, "t.pdf", Disposition::FormatA4).unwrap());
        // Page A5 affichée en paysage : feuille A4 en paysage, page tournée d'un quart de tour.
        assert_eq!(feuilles[0].0, (841.89, 595.28));
        assert!(
            feuilles[0].1.starts_with("q 0.00000 -"),
            "{}",
            feuilles[0].1
        );
    }

    #[test]
    fn incruste_les_champs_de_formulaire_remplis() {
        let pdf = pdf_modifie(|document, page| {
            let apparence = document.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Form",
                    "BBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
                },
                b"0 0 0 rg 0 0 200 100 re f".to_vec(),
            ));
            let champ = document.add_object(dictionary! {
                "Type" => "Annot", "Subtype" => "Widget",
                "Rect" => vec![100.into(), 100.into(), 300.into(), 200.into()],
                "AP" => dictionary! { "N" => apparence },
            });
            document
                .get_dictionary_mut(page)
                .unwrap()
                .set("Annots", vec![Object::Reference(champ)]);
        });
        let image = rendu_feuille(imposer(&pdf, "f.pdf", Disposition::FormatA4).unwrap(), 595);
        // Le cadre du champ (x 100 à 300, y 100 à 200 depuis le bas) est noir sur la feuille.
        assert!(image.get_pixel(200, 842 - 150).0[0] < 60);
        assert!(image.get_pixel(450, 842 - 150).0[0] > 200);
    }
}
