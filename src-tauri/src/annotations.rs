//! Éditeur de PDF, première étape : les annotations faites dans le lecteur, écrites dans un nouveau
//! PDF (l'original n'est jamais modifié).
//!
//! - Surlignages et notes deviennent de vraies annotations PDF (Highlight, Text), avec leur
//!   apparence : visibles partout, et modifiables dans Acrobat ou Aperçu.
//! - Texte et images s'écrivent dans le contenu de la page, comme Numéros et tampon et Signature
//!   ou cachet.
//!
//! Les positions viennent du lecteur : en points de la page affichée (rotation et recadrage
//! compris), depuis son coin en haut à gauche, comme les mots de `texte::mots`.

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, StringFormat, dictionary};
use serde::Deserialize;

use crate::erreur::Erreur;
use crate::geometrie::{Cadre, Matrice, boite};
use crate::images::ImagePdf;
use crate::marques::{self, Placement};
use crate::police;

/// Interligne du texte écrit, en tailles de police (le lecteur affiche le même).
pub const INTERLIGNE: f64 = 1.2;
/// Du haut d'une ligne à sa ligne de base, en tailles de police, avec cet interligne : la moitié de
/// l'interligne en trop, puis la hauteur des capitales et des lettres hautes d'Helvetica (0,718).
pub const LIGNE_DE_BASE: f64 = (INTERLIGNE - 0.925) / 2.0 + 0.718;
/// Côté de l'icône d'une note, en points.
const COTE_NOTE: f64 = 20.0;

/// Rectangle sur la page affichée, en points, depuis le coin en haut à gauche.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Zone {
    pub x: f64,
    pub y: f64,
    pub largeur: f64,
    pub hauteur: f64,
}

/// Couleurs proposées : celles du surligneur, puis celles du texte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Teinte {
    Jaune,
    Vert,
    Rose,
    Bleu,
    Noir,
    Rouge,
    Marine,
}

impl Teinte {
    fn rvb(self) -> [f64; 3] {
        match self {
            Teinte::Jaune => [1.0, 0.86, 0.12],
            Teinte::Vert => [0.42, 0.88, 0.45],
            Teinte::Rose => [1.0, 0.52, 0.76],
            Teinte::Bleu => [0.45, 0.76, 1.0],
            Teinte::Noir => [0.1, 0.11, 0.13],
            Teinte::Rouge => [0.78, 0.12, 0.12],
            Teinte::Marine => [0.1, 0.24, 0.55],
        }
    }

    fn operateur(self, remplissage: bool) -> String {
        let [r, v, b] = self.rvb();
        format!(
            "{r:.3} {v:.3} {b:.3} {}",
            if remplissage { "rg" } else { "RG" }
        )
    }
}

/// Une annotation du lecteur ; `page` compte à partir de 0.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "sorte", rename_all = "lowercase")]
pub enum Annotation {
    /// Passages surlignés : une zone par ligne.
    Surlignage {
        page: usize,
        teinte: Teinte,
        zones: Vec<Zone>,
    },
    /// Note : une icône à cet endroit, et son texte, qui s'ouvre au clic dans les lecteurs de PDF.
    Note {
        page: usize,
        x: f64,
        y: f64,
        texte: String,
    },
    /// Texte écrit sur la page ; (x, y) : le coin en haut à gauche de la première ligne.
    Texte {
        page: usize,
        x: f64,
        y: f64,
        texte: String,
        taille: f64,
        teinte: Teinte,
    },
    /// Image (signature, cachet, logo) posée dans la zone, sans être déformée.
    Image {
        page: usize,
        zone: Zone,
        chemin: String,
    },
}

impl Annotation {
    pub fn page(&self) -> usize {
        match self {
            Annotation::Surlignage { page, .. }
            | Annotation::Note { page, .. }
            | Annotation::Texte { page, .. }
            | Annotation::Image { page, .. } => *page,
        }
    }
}

/// Texte d'un champ PDF (contenu d'une note) : en UTF-16 avec sa marque, pour les accents.
fn texte_pdf(texte: &str) -> Object {
    if texte.is_ascii() {
        return Object::String(texte.as_bytes().to_vec(), StringFormat::Literal);
    }
    let mut octets = vec![0xFE, 0xFF];
    for unite in texte.encode_utf16() {
        octets.extend_from_slice(&unite.to_be_bytes());
    }
    Object::String(octets, StringFormat::Hexadecimal)
}

fn nombres(valeurs: impl IntoIterator<Item = f64>) -> Object {
    Object::Array(
        valeurs
            .into_iter()
            .map(|v| Object::Real(v as f32))
            .collect(),
    )
}

/// Ajoute l'annotation à la liste de la page (directe ou en référence).
fn rattacher(
    document: &mut Document,
    page: ObjectId,
    mut annotation: Dictionary,
) -> Result<(), Erreur> {
    annotation.set("P", Object::Reference(page));
    let id = document.add_object(annotation);
    let existantes = document
        .get_dictionary(page)
        .map_err(|_| Erreur::simple("Une page n'a pas pu être lue."))?
        .get(b"Annots")
        .ok()
        .cloned();
    let liste = match existantes {
        Some(Object::Reference(reference)) => {
            if let Ok(Object::Array(liste)) = document.get_object_mut(reference) {
                liste.push(Object::Reference(id));
                return Ok(());
            }
            vec![Object::Reference(id)]
        }
        Some(Object::Array(mut liste)) => {
            liste.push(Object::Reference(id));
            liste
        }
        _ => vec![Object::Reference(id)],
    };
    document
        .get_dictionary_mut(page)
        .map_err(|_| Erreur::simple("Une page n'a pas pu être modifiée."))?
        .set("Annots", liste);
    Ok(())
}

/// Apparence d'une annotation : un dessin (formulaire PDF) cadré sur son rectangle.
fn apparence(
    document: &mut Document,
    rectangle: [f64; 4],
    ressources: Dictionary,
    contenu: String,
) -> Object {
    let flux = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => nombres(rectangle),
            "Resources" => ressources,
        },
        contenu.into_bytes(),
    );
    Object::Dictionary(dictionary! { "N" => Object::Reference(document.add_object(flux)) })
}

/// Passages surlignés : une annotation Highlight, un quadrilatère par zone. Son apparence les
/// colore en mode « produit » : le texte dessous reste lisible.
fn surligner(
    document: &mut Document,
    page: ObjectId,
    vers_page: Matrice,
    hauteur: f64,
    teinte: Teinte,
    zones: &[Zone],
) -> Result<(), Erreur> {
    let zones: Vec<&Zone> = zones
        .iter()
        .filter(|z| z.largeur > 0.0 && z.hauteur > 0.0)
        .collect();
    if zones.is_empty() {
        return Ok(());
    }
    let mut quadrilateres = Vec::new();
    let mut points = Vec::new();
    let mut trace = String::from("/Surligneur gs ");
    trace.push_str(&teinte.operateur(true));
    trace.push('\n');
    for zone in zones {
        // Ordre des QuadPoints : haut gauche, haut droite, bas gauche, bas droite (comme Acrobat).
        let (haut, bas) = (hauteur - zone.y, hauteur - zone.y - zone.hauteur);
        let coins = [
            vers_page.point(zone.x, haut),
            vers_page.point(zone.x + zone.largeur, haut),
            vers_page.point(zone.x, bas),
            vers_page.point(zone.x + zone.largeur, bas),
        ];
        for (x, y) in coins {
            quadrilateres.push(x);
            quadrilateres.push(y);
            points.push((x, y));
        }
        let [hg, hd, bg, bd] = coins;
        trace.push_str(&format!(
            "{:.2} {:.2} m {:.2} {:.2} l {:.2} {:.2} l {:.2} {:.2} l h f\n",
            hg.0, hg.1, hd.0, hd.1, bd.0, bd.1, bg.0, bg.1
        ));
    }
    let rectangle = boite(Matrice::IDENTITE, &points);
    let etat = dictionary! { "Type" => "ExtGState", "BM" => "Multiply", "ca" => 0.5 };
    let ressources = dictionary! { "ExtGState" => dictionary! { "Surligneur" => etat } };
    let vue = apparence(document, rectangle, ressources, trace);
    rattacher(
        document,
        page,
        dictionary! {
            "Type" => "Annot",
            "Subtype" => "Highlight",
            "Rect" => nombres(rectangle),
            "QuadPoints" => nombres(quadrilateres),
            "C" => nombres(teinte.rvb()),
            "F" => 4,
            "AP" => vue,
        },
    )
}

/// Note : une annotation Text (l'icône « commentaire »), qui garde son texte.
fn noter(
    document: &mut Document,
    page: ObjectId,
    vers_page: Matrice,
    hauteur: f64,
    (x, y): (f64, f64),
    texte: &str,
) -> Result<(), Erreur> {
    // L'icône reste droite et à sa taille (indicateurs NoZoom et NoRotate) : coin haut gauche à
    // l'endroit choisi.
    let (px, py) = vers_page.point(x, hauteur - y);
    let rectangle = [px, py - COTE_NOTE, px + COTE_NOTE, py];
    let icone = "q 1 0.82 0.2 rg 0.55 0.42 0.05 RG 1 w 1 1 18 18 re B \
         0.42 0.32 0.04 RG 1.2 w 5 13.5 m 15 13.5 l S 5 10 m 15 10 l S 5 6.5 m 11.5 6.5 l S Q";
    let flux = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => nombres([0.0, 0.0, COTE_NOTE, COTE_NOTE]),
            "Matrix" => nombres([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
        },
        icone.as_bytes().to_vec(),
    );
    let vue =
        Object::Dictionary(dictionary! { "N" => Object::Reference(document.add_object(flux)) });
    rattacher(
        document,
        page,
        dictionary! {
            "Type" => "Annot",
            "Subtype" => "Text",
            "Rect" => nombres(rectangle),
            "Contents" => texte_pdf(texte),
            "Name" => "Comment",
            "C" => nombres([1.0, 0.82, 0.2]),
            "F" => 4 | 8 | 16,
            "Open" => false,
            "AP" => vue,
        },
    )
}

/// Texte écrit sur la page, ligne par ligne, en Helvetica (accents du français compris).
fn ecrire(
    document: &mut Document,
    page: ObjectId,
    hauteur: f64,
    (x, y): (f64, f64),
    texte: &str,
    taille: f64,
    teinte: Teinte,
) -> Result<(), Erreur> {
    let taille = taille.clamp(4.0, 144.0);
    let lignes: Vec<&str> = texte.lines().collect();
    if lignes.iter().all(|ligne| ligne.trim().is_empty()) {
        return Ok(());
    }
    let base = hauteur - y - LIGNE_DE_BASE * taille;
    marques::surimprimer(
        document,
        page,
        &[(
            "Font",
            "AutomatTexte",
            Object::Dictionary(police::police(false)),
        )],
        |noms| {
            let mut dessin = format!(
                "{} BT /{} {taille:.2} Tf {:.2} TL 1 0 0 1 {x:.2} {base:.2} Tm\n",
                teinte.operateur(true),
                noms[0],
                INTERLIGNE * taille
            );
            for (rang, ligne) in lignes.iter().enumerate() {
                if rang > 0 {
                    dessin.push_str("T* ");
                }
                dessin.push_str(&police::chaine(ligne));
                dessin.push_str(" Tj\n");
            }
            dessin.push_str("ET\n");
            dessin
        },
    )
}

/// Écrit les annotations dans le document. `image` lit et prépare une image d'après son chemin.
pub(crate) fn annoter(
    document: &mut Document,
    annotations: &[Annotation],
    image: impl Fn(&str) -> Result<ImagePdf, Erreur>,
) -> Result<(), Erreur> {
    let pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    for annotation in annotations {
        let index = annotation.page();
        let &page = pages.get(index).ok_or_else(|| {
            Erreur::simple(format!("La page {} n'existe pas dans ce PDF.", index + 1))
        })?;
        let dictionnaire = document
            .get_dictionary(page)
            .map_err(|_| Erreur::simple("Une page n'a pas pu être lue."))?
            .clone();
        let cadre = Cadre::de_page(document, &dictionnaire);
        let (largeur, hauteur) = cadre.taille_vue();
        let vers_page = cadre.depuis_vue();
        match annotation {
            Annotation::Surlignage { teinte, zones, .. } => {
                surligner(document, page, vers_page, hauteur, *teinte, zones)?
            }
            Annotation::Note { x, y, texte, .. } => {
                noter(document, page, vers_page, hauteur, (*x, *y), texte)?
            }
            Annotation::Texte {
                x,
                y,
                texte,
                taille,
                teinte,
                ..
            } => ecrire(document, page, hauteur, (*x, *y), texte, *taille, *teinte)?,
            Annotation::Image { zone, chemin, .. } => {
                let placement = Placement {
                    x: zone.x / largeur,
                    y: zone.y / hauteur,
                    largeur: zone.largeur / largeur,
                    hauteur: zone.hauteur / hauteur,
                };
                marques::apposer(document, image(chemin)?, placement, &[index])?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::Arc;

    use super::*;
    use crate::essais::pdf_herite;
    use crate::images::image_decodee;
    use crate::pdf::{charger, ecrire as ecrire_pdf};
    use crate::{rendu, texte};

    fn annote(octets: &[u8], annotations: &[Annotation]) -> Vec<u8> {
        let mut document = charger(octets, "essai.pdf").unwrap();
        annoter(&mut document, annotations, |_| {
            let mut png = Vec::new();
            image::RgbImage::from_pixel(40, 20, image::Rgb([20, 60, 160]))
                .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
                .unwrap();
            image_decodee(&png, "cachet.png")
        })
        .unwrap();
        ecrire_pdf(document).unwrap()
    }

    /// Couleur du pixel (x, y) de la page `page`, rendue à 72 points par pouce : un pixel par point.
    fn pixel(octets: &[u8], page: usize, (x, y): (u32, u32)) -> [u8; 3] {
        let (largeur, _, pixels) =
            rendu::pixels_page(Arc::new(octets.to_vec()), "essai.pdf", page, 72).unwrap();
        let i = ((y * largeur + x) * 3) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2]]
    }

    fn annotations_de(octets: &[u8], page: usize) -> Vec<Dictionary> {
        let document = charger(octets, "essai.pdf").unwrap();
        let id = *document.get_pages().get(&(page as u32 + 1)).unwrap();
        let liste = match document.get_dictionary(id).unwrap().get(b"Annots") {
            Ok(Object::Array(liste)) => liste.clone(),
            Ok(Object::Reference(r)) => {
                document.get_object(*r).unwrap().as_array().unwrap().clone()
            }
            _ => Vec::new(),
        };
        liste
            .iter()
            .map(|a| {
                document
                    .get_dictionary(a.as_reference().unwrap())
                    .unwrap()
                    .clone()
            })
            .collect()
    }

    /// PDF d'une page A4 tournée d'un quart de tour, avec du texte.
    fn page_tournee() -> Vec<u8> {
        let mut document = charger(&pdf_herite(1, "Tourner"), "essai.pdf").unwrap();
        let id = *document.get_pages().get(&1).unwrap();
        document.get_dictionary_mut(id).unwrap().set("Rotate", 90);
        ecrire_pdf(document).unwrap()
    }

    #[test]
    fn surligne_sans_cacher_le_texte() {
        let source = pdf_herite(1, "Bonjour");
        let zone = Zone {
            x: 300.0,
            y: 300.0,
            largeur: 120.0,
            hauteur: 20.0,
        };
        let sortie = annote(
            &source,
            &[Annotation::Surlignage {
                page: 0,
                teinte: Teinte::Jaune,
                zones: vec![zone],
            }],
        );
        let annotations = annotations_de(&sortie, 0);
        assert_eq!(annotations.len(), 1);
        assert_eq!(
            annotations[0].get(b"Subtype").unwrap().as_name().unwrap(),
            b"Highlight"
        );
        let quadrilateres = annotations[0]
            .get(b"QuadPoints")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(quadrilateres.len(), 8, "un quadrilatère par zone");
        // Haut gauche de la zone : (300, 842 − 300) sur la page, qui n'est pas tournée.
        let premier: Vec<f32> = quadrilateres[..2]
            .iter()
            .map(|v| v.as_float().unwrap())
            .collect();
        assert_eq!(premier, vec![300.0, 542.0]);
        // Au milieu de la zone, la page n'est plus blanche, mais jaune.
        let [r, v, b] = pixel(&sortie, 0, (360, 310));
        assert!(
            r > 200 && v > 180 && b < 160,
            "jaune attendu, vu {r} {v} {b}"
        );
        assert_eq!(
            pixel(&sortie, 0, (200, 310)),
            [255, 255, 255],
            "hors de la zone, rien"
        );
    }

    #[test]
    fn surligne_au_bon_endroit_sur_une_page_tournee() {
        // La page affichée fait 842 × 595 points : on surligne près de son coin en haut à gauche.
        let sortie = annote(
            &page_tournee(),
            &[Annotation::Surlignage {
                page: 0,
                teinte: Teinte::Bleu,
                zones: vec![Zone {
                    x: 40.0,
                    y: 40.0,
                    largeur: 100.0,
                    hauteur: 30.0,
                }],
            }],
        );
        let (largeur, hauteur, _) =
            rendu::pixels_page(Arc::new(sortie.clone()), "essai.pdf", 0, 72).unwrap();
        assert_eq!((largeur, hauteur), (842, 595), "rendue redressée");
        let [r, v, b] = pixel(&sortie, 0, (90, 55));
        assert!(
            b > 200 && r < 200,
            "bleu attendu en haut à gauche de la page affichée, vu {r} {v} {b}"
        );
        assert_eq!(pixel(&sortie, 0, (790, 540)), [255, 255, 255]);
    }

    #[test]
    fn garde_le_texte_de_la_note_avec_ses_accents() {
        let texte = "À vérifier : le montant du 3e trimestre";
        let sortie = annote(
            &pdf_herite(2, "Note"),
            &[Annotation::Note {
                page: 1,
                x: 500.0,
                y: 100.0,
                texte: texte.into(),
            }],
        );
        assert!(
            annotations_de(&sortie, 0).is_empty(),
            "rien sur la première page"
        );
        let annotations = annotations_de(&sortie, 1);
        assert_eq!(
            annotations[0].get(b"Subtype").unwrap().as_name().unwrap(),
            b"Text"
        );
        let Object::String(octets, _) = annotations[0].get(b"Contents").unwrap() else {
            panic!("pas de texte")
        };
        assert_eq!(&octets[..2], &[0xFE, 0xFF]);
        let unites: Vec<u16> = octets[2..]
            .chunks(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(String::from_utf16(&unites).unwrap(), texte);
        assert!(annotations[0].has(b"AP"), "l'icône a son apparence");
        let [r, v, b] = pixel(&sortie, 1, (503, 103));
        assert!(
            r > 200 && v > 150 && b < 120,
            "icône jaune attendue, vue {r} {v} {b}"
        );
    }

    #[test]
    fn ecrit_le_texte_la_ou_le_lecteur_l_affiche() {
        let taille = 14.0;
        let sortie = annote(
            &pdf_herite(1, "Texte"),
            &[Annotation::Texte {
                page: 0,
                x: 100.0,
                y: 400.0,
                texte: "Reçu le 25/11\nPayé".into(),
                taille,
                teinte: Teinte::Rouge,
            }],
        );
        let mots = texte::mots(Arc::new(sortie), "essai.pdf", 0).unwrap();
        let recu = mots
            .iter()
            .find(|m| m.texte == "Reçu")
            .expect("le mot écrit est lu dans la page");
        assert!((recu.x - 100.0).abs() < 1.0, "x : {}", recu.x);
        // Le lecteur place le haut d'un mot à sa ligne de base moins 0,8 fois sa taille.
        let haut_attendu = 400.0 + LIGNE_DE_BASE * taille - 0.8 * taille;
        assert!(
            (recu.y - haut_attendu).abs() < 1.0,
            "y : {} au lieu de {haut_attendu}",
            recu.y
        );
        let paye = mots
            .iter()
            .find(|m| m.texte == "Payé")
            .expect("deuxième ligne");
        assert!(
            (paye.y - recu.y - INTERLIGNE * taille).abs() < 1.0,
            "une ligne plus bas"
        );
    }

    #[test]
    fn pose_l_image_dans_sa_zone() {
        let zone = Zone {
            x: 400.0,
            y: 700.0,
            largeur: 120.0,
            hauteur: 60.0,
        };
        let sortie = annote(
            &pdf_herite(1, "Image"),
            &[Annotation::Image {
                page: 0,
                zone,
                chemin: "cachet.png".into(),
            }],
        );
        let [r, v, b] = pixel(&sortie, 0, (460, 730));
        assert!(b > 120 && r < 80, "bleu du cachet attendu, vu {r} {v} {b}");
        assert_eq!(pixel(&sortie, 0, (300, 730)), [255, 255, 255]);
    }

    #[test]
    fn ajoute_aux_annotations_existantes_et_refuse_une_page_absente() {
        let source = crate::essais::pdf_avec_lien(2, "Lien");
        let sortie = annote(
            &source,
            &[Annotation::Note {
                page: 0,
                x: 10.0,
                y: 10.0,
                texte: "Vu".into(),
            }],
        );
        let sous_types: Vec<Vec<u8>> = annotations_de(&sortie, 0)
            .iter()
            .map(|a| a.get(b"Subtype").unwrap().as_name().unwrap().to_vec())
            .collect();
        assert_eq!(
            sous_types,
            vec![b"Link".to_vec(), b"Text".to_vec()],
            "le lien reste"
        );
        let mut document = charger(&source, "essai.pdf").unwrap();
        let erreur = annoter(
            &mut document,
            &[Annotation::Note {
                page: 5,
                x: 0.0,
                y: 0.0,
                texte: String::new(),
            }],
            |_| unreachable!(),
        )
        .unwrap_err();
        assert!(erreur.message.contains("page 6"), "{}", erreur.message);
    }

    #[test]
    fn lit_les_annotations_envoyees_par_l_interface() {
        let json = r#"[
            {"sorte": "surlignage", "page": 0, "teinte": "jaune", "zones": [{"x": 1, "y": 2, "largeur": 3, "hauteur": 4}]},
            {"sorte": "note", "page": 1, "x": 5, "y": 6, "texte": "Vu"},
            {"sorte": "texte", "page": 0, "x": 7, "y": 8, "texte": "Payé", "taille": 12, "teinte": "marine"},
            {"sorte": "image", "page": 2, "zone": {"x": 1, "y": 1, "largeur": 9, "hauteur": 9}, "chemin": "C:\\cachet.png"}
        ]"#;
        let lues: Vec<Annotation> = serde_json::from_str(json).unwrap();
        assert_eq!(
            lues.iter().map(Annotation::page).collect::<Vec<_>>(),
            vec![0, 1, 0, 2]
        );
        assert_eq!(
            lues[2],
            Annotation::Texte {
                page: 0,
                x: 7.0,
                y: 8.0,
                texte: "Payé".into(),
                taille: 12.0,
                teinte: Teinte::Marine
            }
        );
    }
}
