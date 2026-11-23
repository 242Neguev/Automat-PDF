//! Plusieurs pages par feuille (2 ou 4), à la suite ou chaque page répétée (des invitations à
//! découper), livret à plier, ou pages remises au format A4. Chaque page devient un « formulaire »
//! PDF (son contenu et ses ressources), placé à l'échelle dans une case de la feuille : rien n'est
//! recalculé, texte et images restent nets. Les annotations visibles (champs de formulaire remplis,
//! tampons) sont incrustées dans la page, pour ne pas disparaître. Les exemplaires à découper sont
//! posés à leur format fini (sans les marges d'un PDF prêt à imprimer), entourés de traits de coupe.

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};
use serde::Deserialize;

use crate::erreur::Erreur;
use crate::geometrie::{A4, Cadre, Matrice, boite, rectangle, resoudre};
use crate::pdf::{charger, contenu_page};

/// Marge autour des pages d'une feuille, et espace entre elles (points).
const MARGE: f64 = 14.0;

/// Traits de coupe autour des exemplaires : écart entre le coin de l'exemplaire et le début du trait,
/// puis longueur du trait (points). Le trait tient dans l'espace entre deux exemplaires (au moins
/// MARGE) sans toucher le voisin, dont les traits, sur les mêmes lignes de coupe, le recouvrent.
const ECART_TRAIT: f64 = 3.0;
const LONGUEUR_TRAIT: f64 = MARGE - 2.0 * ECART_TRAIT;

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

/// Une page devenue formulaire : son identifiant, son cadre (partie visible, rotation) et son
/// format fini (le même cadre, réduit à la zone de coupe d'un PDF prêt à imprimer).
struct PageSource {
    forme: ObjectId,
    cadre: Cadre,
    format_fini: Cadre,
}

impl PageSource {
    fn taille_vue(&self) -> (f64, f64) {
        self.cadre.taille_vue()
    }
}

/// Matrice qui pose la partie `cadre` d'une page, redressée et à l'échelle, au centre de la case
/// (x, y, l, h).
fn placement(cadre: &Cadre, (x, y, l, h): Case) -> Matrice {
    let (vue_l, vue_h) = cadre.taille_vue();
    let echelle = (l / vue_l).min(h / vue_h);
    cadre
        .vers_vue()
        .puis(Matrice::echelle(echelle, echelle))
        .puis(Matrice::translation(
            x + (l - echelle * vue_l) / 2.0,
            y + (h - echelle * vue_h) / 2.0,
        ))
}

/// Format fini d'une page : la zone de coupe (TrimBox) d'un PDF prêt à imprimer, sans les marges où
/// sont tracés ses propres traits de coupe ; à défaut, toute la partie visible.
fn format_fini(document: &Document, page: &Dictionary, cadre: Cadre) -> Cadre {
    let [gauche, bas, droite, haut] = cadre.boite;
    page.get(b"TrimBox")
        .ok()
        .and_then(|zone| rectangle(document, zone))
        .map(|[x0, y0, x1, y1]| [x0.max(gauche), y0.max(bas), x1.min(droite), y1.min(haut)])
        .filter(|&[x0, y0, x1, y1]| x1 - x0 > 1.0 && y1 - y0 > 1.0)
        .map_or(cadre, |zone| Cadre {
            boite: zone,
            ..cadre
        })
}

/// Traits de coupe d'un exemplaire posé sur la feuille, de (x0, y0) à (x1, y1) : deux à chaque coin,
/// dans le prolongement de ses bords, vers l'extérieur.
fn traits_de_coupe([x0, y0, x1, y1]: [f64; 4]) -> String {
    let (debut, fin) = (ECART_TRAIT, ECART_TRAIT + LONGUEUR_TRAIT);
    let mut traits = String::new();
    for (x, vers_x) in [(x0, -1.0), (x1, 1.0)] {
        for (y, vers_y) in [(y0, -1.0), (y1, 1.0)] {
            traits.push_str(&format!(
                "{:.3} {y:.3} m {:.3} {y:.3} l S {x:.3} {:.3} m {x:.3} {:.3} l S\n",
                x + vers_x * debut,
                x + vers_x * fin,
                y + vers_y * debut,
                y + vers_y * fin,
            ));
        }
    }
    traits
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
        format_fini: format_fini(document, &dictionnaire, cadre),
    })
}

/// Une feuille : sa taille et, pour chaque case, la page (à partir de 0) qui y va, s'il y en a une.
struct Feuille {
    taille: (f64, f64),
    cases: Vec<(Option<usize>, Case)>,
}

/// Les feuilles à produire pour `nombre` pages ; `taille_vue` donne la taille d'une page (format A4).
/// Avec `repeter`, 2 ou 4 par feuille : chaque page remplit toutes les cases de sa propre feuille.
fn feuilles(
    nombre: usize,
    disposition: Disposition,
    repeter: bool,
    taille_vue: impl Fn(usize) -> (f64, f64),
) -> Vec<Feuille> {
    let paysage = (A4.1, A4.0);
    let deux_cases = |(largeur, hauteur): (f64, f64)| {
        let l = (largeur - 3.0 * MARGE) / 2.0;
        let h = hauteur - 2.0 * MARGE;
        [(MARGE, MARGE, l, h), (2.0 * MARGE + l, MARGE, l, h)]
    };
    // Plusieurs pages par feuille : les pages à la suite (1, 2, 3, 4, puis 5…), ou chaque page
    // répétée dans toutes les cases de sa feuille (une invitation en 4 exemplaires à découper).
    let plusieurs = |taille: (f64, f64), cases: &[Case]| -> Vec<Feuille> {
        let pages: Vec<Vec<Option<usize>>> = if repeter {
            (0..nombre)
                .map(|page| vec![Some(page); cases.len()])
                .collect()
        } else {
            (0..nombre)
                .step_by(cases.len())
                .map(|debut| {
                    (debut..debut + cases.len())
                        .map(|page| (page < nombre).then_some(page))
                        .collect()
                })
                .collect()
        };
        pages
            .into_iter()
            .map(|pages| Feuille {
                taille,
                cases: pages.into_iter().zip(cases.iter().copied()).collect(),
            })
            .collect()
    };
    match disposition {
        Disposition::DeuxParFeuille => plusieurs(paysage, &deux_cases(paysage)),
        Disposition::QuatreParFeuille => {
            let (largeur, hauteur) = A4;
            let (l, h) = ((largeur - 3.0 * MARGE) / 2.0, (hauteur - 3.0 * MARGE) / 2.0);
            // Dans l'ordre de lecture : en haut à gauche, en haut à droite, puis en bas.
            plusieurs(
                A4,
                &[
                    (MARGE, 2.0 * MARGE + h, l, h),
                    (2.0 * MARGE + l, 2.0 * MARGE + h, l, h),
                    (MARGE, MARGE, l, h),
                    (2.0 * MARGE + l, MARGE, l, h),
                ],
            )
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

/// Le PDF mis en page selon la disposition choisie ; avec `repeter` (2 ou 4 par feuille), chaque
/// page est répétée sur sa propre feuille, à son format fini, avec des traits de coupe.
pub fn imposer(
    octets: &[u8],
    nom: &str,
    disposition: Disposition,
    repeter: bool,
) -> Result<Vec<u8>, Erreur> {
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
    // Exemplaires à découper : chaque page n'est posée qu'à son format fini, découpée à sa zone de
    // coupe, et l'application trace les traits de coupe autour.
    let decouper = repeter
        && matches!(
            disposition,
            Disposition::DeuxParFeuille | Disposition::QuatreParFeuille
        );
    let arbre = document.new_object_id();
    let mut enfants = Vec::new();
    for feuille in feuilles(sources.len(), disposition, repeter, |i| {
        sources[i].taille_vue()
    }) {
        let mut contenu = String::new();
        let mut objets_graphiques = Dictionary::new();
        let mut exemplaires = Vec::new();
        for (page, case) in feuille.cases {
            let Some(page) = page else { continue };
            let source = &sources[page];
            if decouper {
                let [x0, y0, x1, y1] = source.format_fini.boite;
                let matrice = placement(&source.format_fini, case);
                contenu.push_str(&format!(
                    "q {} {x0:.3} {y0:.3} {:.3} {:.3} re W n /P{page} Do Q\n",
                    matrice.operateur(),
                    x1 - x0,
                    y1 - y0,
                ));
                exemplaires.push(boite(matrice, &[(x0, y0), (x1, y1)]));
            } else {
                contenu.push_str(&format!(
                    "q {} /P{page} Do Q\n",
                    placement(&source.cadre, case).operateur()
                ));
            }
            objets_graphiques.set(format!("P{page}").as_bytes(), source.forme);
        }
        if !exemplaires.is_empty() {
            contenu.push_str("q 0 G 0.4 w 0 J [] 0 d\n");
            for exemplaire in exemplaires {
                contenu.push_str(&traits_de_coupe(exemplaire));
            }
            contenu.push_str("Q\n");
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

    /// Pixels sombres de la zone de (x0, y0) à (x1, y1) exclus, depuis le haut de l'image.
    fn pixels_sombres(
        image: &image::GrayImage,
        (x0, y0): (u32, u32),
        (x1, y1): (u32, u32),
    ) -> usize {
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| image.get_pixel(x, y).0[0] < 128)
            .count()
    }

    /// Pour chaque feuille, la page (à partir de 0) de chacune de ses cases.
    fn pages_des_cases(feuilles: &[Feuille]) -> Vec<Vec<Option<usize>>> {
        feuilles
            .iter()
            .map(|feuille| feuille.cases.iter().map(|(page, _)| *page).collect())
            .collect()
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
        let pdf = imposer(
            &pdf_herite(3, "Page"),
            "p.pdf",
            Disposition::DeuxParFeuille,
            false,
        )
        .unwrap();
        let feuilles = feuilles_de(&pdf);
        assert_eq!(feuilles.len(), 2);
        assert_eq!(feuilles[0].0, (841.89, 595.28));
        assert!(feuilles[0].1.contains("/P0 Do") && feuilles[0].1.contains("/P1 Do"));
        assert!(feuilles[1].1.contains("/P2 Do") && !feuilles[1].1.contains("/P3"));
        // Les deux moitiés de la première feuille sont bien dessinées.
        let image = rendu_feuille(pdf, 400);
        let h = image.height();
        assert!(pixels_sombres(&image, (0, 0), (200, h)) > 10);
        assert!(pixels_sombres(&image, (200, 0), (400, h)) > 10);
    }

    #[test]
    fn repete_une_page_seule_dans_les_quatre_cases() {
        // Une invitation d'une page, en 4 exemplaires sur une feuille A4 : la page n'est copiée
        // qu'une fois dans le PDF, et dessinée dans chaque quart de la feuille.
        let pdf = imposer(
            &pdf_herite(1, "Invitation"),
            "i.pdf",
            Disposition::QuatreParFeuille,
            true,
        )
        .unwrap();
        let feuilles = feuilles_de(&pdf);
        assert_eq!(feuilles.len(), 1);
        assert_eq!(feuilles[0].0, (595.28, 841.89));
        assert_eq!(feuilles[0].1.matches("/P0 Do").count(), 4);
        let image = rendu_feuille(pdf, 600);
        let (l, h) = (image.width() / 2, image.height() / 2);
        for (x, y) in [(0, 0), (l, 0), (0, h), (l, h)] {
            assert!(
                pixels_sombres(&image, (x, y), (x + l, y + h)) > 10,
                "quart de feuille vide en ({x}, {y})"
            );
        }
    }

    #[test]
    fn pose_le_format_fini_d_un_pdf_pret_a_imprimer() {
        // Invitation prête à imprimer : page de 402 × 546 points, format fini (TrimBox) de 360 × 504
        // peint en noir, et un carré noir dans la marge, en bas à gauche, comme ses traits de coupe.
        let pdf = pdf_modifie(|document, page| {
            let contenu = document.add_object(Stream::new(
                dictionary! {},
                b"0 g 21 21 360 504 re f 0 0 15 15 re f".to_vec(),
            ));
            let page = document.get_dictionary_mut(page).unwrap();
            page.set("MediaBox", vec![0.into(), 0.into(), 402.into(), 546.into()]);
            page.set(
                "TrimBox",
                vec![21.into(), 21.into(), 381.into(), 525.into()],
            );
            page.set("Contents", contenu);
        });
        let pdf = imposer(&pdf, "invitation.pdf", Disposition::QuatreParFeuille, true).unwrap();
        let feuilles = feuilles_de(&pdf);
        // Chaque exemplaire est découpé à son format fini et entouré de 8 traits de coupe.
        assert_eq!(feuilles[0].1.matches("re W n").count(), 4);
        assert_eq!(feuilles[0].1.matches(" l S").count(), 4 * 8);
        let image = rendu_feuille(pdf, 1191);
        let echelle = f64::from(image.width()) / A4.0;
        let pixel = |x: f64, y: f64| {
            image
                .get_pixel((x * echelle) as u32, ((A4.1 - y) * echelle) as u32)
                .0[0]
        };
        // Exemplaire en haut à gauche : le format fini prend toute la largeur de sa case (x de 14 à
        // 290,6 points) ; avec ses marges, la page ne lui laisserait que x de 28,5 à 276,2.
        assert!(pixel(20.0, 628.0) < 60 && pixel(285.0, 628.0) < 60);
        // Le carré de la marge, qui tomberait vers (6, 426), est coupé.
        assert!(pixel(6.0, 426.0) > 200);
    }

    #[test]
    fn trace_les_traits_de_coupe_dans_le_prolongement_des_bords() {
        let traits = traits_de_coupe([100.0, 200.0, 300.0, 500.0]);
        // Coin en bas à gauche : vers la gauche et vers le bas, de 3 à 11 points du coin.
        assert!(
            traits.contains("97.000 200.000 m 89.000 200.000 l S"),
            "{traits}"
        );
        assert!(
            traits.contains("100.000 197.000 m 100.000 189.000 l S"),
            "{traits}"
        );
        // Coin en haut à droite : vers la droite et vers le haut.
        assert!(
            traits.contains("303.000 500.000 m 311.000 500.000 l S"),
            "{traits}"
        );
        assert!(
            traits.contains("300.000 503.000 m 300.000 511.000 l S"),
            "{traits}"
        );
    }

    #[test]
    fn repete_chaque_page_sur_sa_propre_feuille() {
        // Recto et verso d'une invitation : une feuille de rectos, puis une feuille de versos.
        let quatre = feuilles(2, Disposition::QuatreParFeuille, true, |_| A4);
        assert_eq!(pages_des_cases(&quatre), [[Some(0); 4], [Some(1); 4]]);
        let deux = feuilles(2, Disposition::DeuxParFeuille, true, |_| A4);
        assert_eq!(pages_des_cases(&deux), [[Some(0); 2], [Some(1); 2]]);
        // Sans répétition, les pages se suivent, et les cases en trop restent vides.
        let suite = feuilles(2, Disposition::QuatreParFeuille, false, |_| A4);
        assert_eq!(pages_des_cases(&suite), [[Some(0), Some(1), None, None]]);
    }

    #[test]
    fn les_exemplaires_du_recto_et_du_verso_se_superposent() {
        // Imprimée en recto verso, la feuille se retourne sur le bord long ou sur le bord court :
        // chaque case du verso doit tomber exactement derrière une case du recto.
        for disposition in [Disposition::DeuxParFeuille, Disposition::QuatreParFeuille] {
            let feuille = &feuilles(1, disposition, true, |_| A4)[0];
            let (largeur, hauteur) = feuille.taille;
            let cases: Vec<Case> = feuille.cases.iter().map(|(_, case)| *case).collect();
            let derriere = |(x, y, l, h): Case| {
                cases.iter().any(|&(x2, y2, l2, h2)| {
                    [x - x2, y - y2, l - l2, h - h2]
                        .iter()
                        .all(|ecart| ecart.abs() < 1e-6)
                })
            };
            for &(x, y, l, h) in &cases {
                assert!(derriere((largeur - x - l, y, l, h)), "{disposition:?}");
                assert!(derriere((x, hauteur - y - h, l, h)), "{disposition:?}");
            }
        }
    }

    #[test]
    fn met_quatre_pages_par_feuille_dans_l_ordre_de_lecture() {
        let feuilles = feuilles(5, Disposition::QuatreParFeuille, false, |_| A4);
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
            feuilles(n, Disposition::Livret, false, |_| A4)
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
        let feuilles = feuilles_de(&imposer(&pdf, "t.pdf", Disposition::FormatA4, false).unwrap());
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
        let image = rendu_feuille(
            imposer(&pdf, "f.pdf", Disposition::FormatA4, false).unwrap(),
            595,
        );
        // Le cadre du champ (x 100 à 300, y 100 à 200 depuis le bas) est noir sur la feuille.
        assert!(image.get_pixel(200, 842 - 150).0[0] < 60);
        assert!(image.get_pixel(450, 842 - 150).0[0] > 200);
    }
}
