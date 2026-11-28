//! Capturer une zone d'une page, depuis le lecteur : son image, à copier ou à enregistrer en PNG
//! ou en JPG, ou un PDF qui ne garde qu'elle, son texte compris. Dans ce PDF, le reste de la page
//! est vraiment retiré (comme sous le correcteur blanc), pas seulement caché par le recadrage.

use std::sync::Arc;

use lopdf::{Document, Object, ObjectId};
use serde::Serialize;

use crate::caviardage::{self, Nettoyage, Zone};
use crate::erreur::Erreur;
use crate::geometrie::Cadre;
use crate::images::{self, FormatFeuille, ImageSource, OptionsImages};
use crate::pdf::{self, PageAssemblee};
use crate::rendu::{self, FormatImage};

/// Résolution de la zone reprise en image dans le PDF, quand son contenu ne se découpe pas.
const RESOLUTION_SECOURS: u32 = 300;
/// Autour de la zone, le contenu est retiré jusqu'à cette distance (points) : même ce qui
/// déborde de la page.
const LOIN: f64 = 1.0e6;
/// Ce qui déborde du cadre de moins de 6 points (une lettre coupée par son bord) est gardé : le
/// recadrage le coupe au bord, comme sur l'image de la zone.
const DEBORD: f64 = 6.0;

/// La zone ramenée dans la page (fractions de la page affichée) ; une erreur si elle est vide.
fn borner(zone: Zone) -> Result<Zone, Erreur> {
    let (x0, y0) = (zone.x.clamp(0.0, 1.0), zone.y.clamp(0.0, 1.0));
    let x1 = (zone.x + zone.largeur).clamp(0.0, 1.0);
    let y1 = (zone.y + zone.hauteur).clamp(0.0, 1.0);
    if x1 - x0 > 1e-4 && y1 - y0 > 1e-4 {
        Ok(Zone {
            x: x0,
            y: y0,
            largeur: x1 - x0,
            hauteur: y1 - y0,
        })
    } else {
        Err(Erreur::simple("Tracez d'abord un cadre sur la page."))
    }
}

/// Les pixels (rouge, vert, bleu) de la zone, la page dessinée à `dpi` points par pouce.
pub fn pixels_zone(
    octets: Arc<Vec<u8>>,
    nom: &str,
    page: usize,
    zone: Zone,
    dpi: u32,
) -> Result<(u32, u32, Vec<u8>), Erreur> {
    let zone = borner(zone)?;
    let (largeur, hauteur, rvb) = rendu::pixels_page(octets, nom, page, dpi)?;
    let (l, h) = (largeur as usize, hauteur as usize);
    let pixel = |fraction: f64, cote: usize| ((fraction * cote as f64).round() as usize).min(cote);
    let (x0, y0) = (
        pixel(zone.x, l).min(l.saturating_sub(1)),
        pixel(zone.y, h).min(h.saturating_sub(1)),
    );
    let x1 = pixel(zone.x + zone.largeur, l).clamp(x0 + 1, l.max(x0 + 1));
    let y1 = pixel(zone.y + zone.hauteur, h).clamp(y0 + 1, h.max(y0 + 1));
    let mut sortie = Vec::with_capacity((x1 - x0) * (y1 - y0) * 3);
    for ligne in y0..y1 {
        let debut = (ligne * l + x0) * 3;
        sortie.extend_from_slice(&rvb[debut..debut + (x1 - x0) * 3]);
    }
    Ok(((x1 - x0) as u32, (y1 - y0) as u32, sortie))
}

/// L'image de la zone, en PNG ou en JPG.
pub fn image_zone(
    octets: Arc<Vec<u8>>,
    nom: &str,
    page: usize,
    zone: Zone,
    dpi: u32,
    format: FormatImage,
) -> Result<Vec<u8>, Erreur> {
    rendu::encoder(pixels_zone(octets, nom, page, zone, dpi)?, format, 92)
}

/// Un PDF d'une page qui ne garde que la zone : la page recadrée sur elle, et tout ce qui est
/// autour vraiment retiré (texte, dessins, pixels des images, annotations). Si ce contenu ne se
/// découpe pas ainsi (champ de formulaire, image dans un format particulier), la zone est reprise
/// en image : `true` en second.
pub fn pdf_zone(
    octets: Arc<Vec<u8>>,
    nom: &str,
    page: usize,
    zone: Zone,
) -> Result<(Vec<u8>, bool), Erreur> {
    let zone = borner(zone)?;
    let une = [PageAssemblee {
        source: 0,
        page,
        rotation: 0,
    }];
    let seule = pdf::assembler_pages(&[(nom.to_string(), octets.as_slice())], &une)?;
    let mut document = pdf::charger(&seule, nom)?;
    let pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    let illisible = || Erreur::simple(format!("La page {} n'a pas pu être lue.", page + 1));
    let id = *pages.first().ok_or_else(illisible)?;
    let cadre = Cadre::de_page(
        &document,
        document.get_dictionary(id).map_err(|_| illisible())?,
    );
    let boite = caviardage::zones_page(&cadre, &[zone])[0];
    if caviardage::nettoyer_page(&mut document, id, &autour(boite), Nettoyage::Blanc).is_none() {
        let png = image_zone(
            octets,
            nom,
            page,
            zone,
            RESOLUTION_SECOURS,
            FormatImage::Png,
        )?;
        let source = ImageSource {
            nom: nom.to_string(),
            octets: &png,
            rotation: 0,
            ppp: Some(RESOLUTION_SECOURS as f32),
        };
        let options = OptionsImages {
            format: FormatFeuille::Reelle,
            marge_mm: 0.0,
        };
        return Ok((images::images_en_pdf(&[source], options)?, true));
    }
    poser_boite(&mut document, id, boite).map_err(|_| illisible())?;
    let max_id = document.max_id;
    Ok((pdf::assembler(document.objects, &pages, max_id)?, false))
}

/// Autour d'un cadre (repère de la page), jusqu'au-delà de la page : quatre rectangles, à
/// `DEBORD` points du cadre.
fn autour([x0, y0, x1, y1]: [f64; 4]) -> [[f64; 4]; 4] {
    let [a0, b0, a1, b1] = [x0 - DEBORD, y0 - DEBORD, x1 + DEBORD, y1 + DEBORD];
    [
        [-LOIN, -LOIN, LOIN, b0],
        [-LOIN, b1, LOIN, LOIN],
        [-LOIN, b0, a0, b1],
        [a1, b0, LOIN, b1],
    ]
}

/// La page recadrée sur `boite` (repère de la page) : ses boîtes de page ne montrent plus que lui.
fn poser_boite(document: &mut Document, page: ObjectId, boite: [f64; 4]) -> Result<(), ()> {
    let boite = Object::Array(
        boite
            .into_iter()
            .map(|valeur| Object::Real(valeur as f32))
            .collect(),
    );
    let dictionnaire = document.get_dictionary_mut(page).map_err(|_| ())?;
    dictionnaire.set("MediaBox", boite.clone());
    dictionnaire.set("CropBox", boite);
    for cle in [b"TrimBox".as_slice(), b"BleedBox", b"ArtBox"] {
        dictionnaire.remove(cle);
    }
    Ok(())
}

/// Ce qu'a donné un recadrage : les pages (à partir de 1) dont le contenu hors du cadre n'a pas pu
/// être retiré (formulaire, image dans un format particulier), où il est seulement caché ; les
/// pages blanches, laissées telles quelles quand chaque page suit son contenu.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recadrage {
    pub pages_cachees: Vec<usize>,
    pub pages_blanches: Vec<usize>,
}

/// Recadre les pages choisies (à partir de 0 ; vide : toutes) sur `zone`, donnée en fractions de
/// chaque page affichée ; sans zone, chaque page sur son contenu (ses marges retirées). Ce qui est
/// hors du cadre est retiré, comme pour une capture ; le reste du document (les autres pages,
/// signets, liens) est gardé.
pub fn recadrer(
    octets: Arc<Vec<u8>>,
    nom: &str,
    pages: &[usize],
    zone: Option<Zone>,
) -> Result<(Vec<u8>, Recadrage), Erreur> {
    let zone = zone.map(borner).transpose()?;
    let mut document = pdf::charger(&octets, nom)?;
    let toutes: Vec<ObjectId> = document.get_pages().into_values().collect();
    let choisies: Vec<usize> = if pages.is_empty() {
        (0..toutes.len()).collect()
    } else {
        pages.to_vec()
    };
    let mut bilan = Recadrage::default();
    for page in choisies {
        let illisible = || Erreur::simple(format!("La page {} n'a pas pu être lue.", page + 1));
        let &id = toutes.get(page).ok_or_else(|| {
            Erreur::simple(format!(
                "La page {} n'existe pas : le PDF en compte {}.",
                page + 1,
                toutes.len()
            ))
        })?;
        let cette_zone = match zone {
            Some(zone) => zone,
            None => match bords_du_contenu(octets.clone(), nom, page)? {
                Some(bords) => bords,
                None => {
                    bilan.pages_blanches.push(page + 1);
                    continue;
                }
            },
        };
        let cadre = Cadre::de_page(
            &document,
            document.get_dictionary(id).map_err(|_| illisible())?,
        );
        let boite = caviardage::zones_page(&cadre, &[cette_zone])[0];
        if caviardage::nettoyer_page(&mut document, id, &autour(boite), Nettoyage::Blanc).is_none()
        {
            bilan.pages_cachees.push(page + 1);
        }
        poser_boite(&mut document, id, boite).map_err(|_| illisible())?;
    }
    Ok((pdf::ecrire(document)?, bilan))
}

/// Le cadre de ce qui est dessiné sur la page (tout ce qui n'est pas blanc), un peu élargi, en
/// fractions de la page : pour retirer ses marges. `None` : la page est blanche.
pub fn bords_du_contenu(
    octets: Arc<Vec<u8>>,
    nom: &str,
    page: usize,
) -> Result<Option<Zone>, Erreur> {
    let (largeur, hauteur, rvb) = rendu::pixels_page(octets, nom, page, 72)?;
    let (l, h) = (largeur as usize, hauteur as usize);
    let (mut x0, mut y0, mut x1, mut y1) = (l, h, 0, 0);
    for (rang, &[rouge, vert, bleu]) in rvb.as_chunks::<3>().0.iter().enumerate() {
        if rouge.min(vert).min(bleu) < 235 {
            let (x, y) = (rang % l, rang / l);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return Ok(None);
    }
    // Un peu d'air autour du contenu : 6 pixels à 72 ppp, 6 points.
    let (x0, y0) = (x0.saturating_sub(6), y0.saturating_sub(6));
    let (x1, y1) = ((x1 + 6).min(l), (y1 + 6).min(h));
    Ok(Some(Zone {
        x: x0 as f64 / l as f64,
        y: y0 as f64 / h as f64,
        largeur: (x1 - x0) as f64 / l as f64,
        hauteur: (y1 - y0) as f64 / h as f64,
    }))
}

fn presse_papiers(erreur: arboard::Error) -> Erreur {
    Erreur::simple(format!(
        "Le presse-papiers n'a pas pu être rempli : {erreur}"
    ))
}

/// Met une image (pixels rouge, vert, bleu) dans le presse-papiers, pour la coller ailleurs.
pub fn copier_image((largeur, hauteur, rvb): (u32, u32, Vec<u8>)) -> Result<(), Erreur> {
    let rvba: Vec<u8> = rvb
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|&[rouge, vert, bleu]| [rouge, vert, bleu, 255])
        .collect();
    copier_pixels(largeur, hauteur, rvba)
}

/// Met une image (pixels rouge, vert, bleu, transparence) dans le presse-papiers.
pub fn copier_pixels(largeur: u32, hauteur: u32, rvba: Vec<u8>) -> Result<(), Erreur> {
    let image = arboard::ImageData {
        width: largeur as usize,
        height: hauteur as usize,
        bytes: rvba.into(),
    };
    arboard::Clipboard::new()
        .and_then(|mut presse| presse.set_image(image))
        .map_err(presse_papiers)
}

/// Met un texte dans le presse-papiers.
pub fn copier_texte(texte: &str) -> Result<(), Erreur> {
    arboard::Clipboard::new()
        .and_then(|mut presse| presse.set_text(texte))
        .map_err(presse_papiers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::pdf_herite;

    /// Les chaînes de texte montrées par les contenus des pages d'un PDF.
    fn textes(pdf: &[u8]) -> Vec<String> {
        crate::texte::textes(Arc::new(pdf.to_vec()), "capture.pdf").unwrap()
    }

    #[test]
    fn decoupe_l_image_de_la_zone() {
        let pdf = Arc::new(pdf_herite(1, "R"));
        let zone = Zone {
            x: 0.25,
            y: 0.5,
            largeur: 0.5,
            hauteur: 0.25,
        };
        // À 72 ppp, une page A4 fait 595 × 842 pixels : la zone, un quart de sa largeur sur un
        // huitième de sa hauteur... à l'arrondi près.
        let (l, h, rvb) = pixels_zone(pdf.clone(), "a.pdf", 0, zone, 72).unwrap();
        assert!(
            (297..=299).contains(&l) && (210..=211).contains(&h),
            "{l} × {h}"
        );
        assert_eq!(rvb.len(), (l * h * 3) as usize);
        let png = image_zone(pdf.clone(), "a.pdf", 0, zone, 144, FormatImage::Png).unwrap();
        let image = image::load_from_memory(&png).unwrap();
        assert!((595..=597).contains(&image.width()), "{}", image.width());
        // Une zone hors de la page, ou vide, est refusée.
        let vide = Zone {
            x: 1.2,
            y: 0.1,
            largeur: 0.3,
            hauteur: 0.3,
        };
        assert!(pixels_zone(pdf, "a.pdf", 0, vide, 72).is_err());
    }

    #[test]
    fn le_pdf_ne_garde_que_la_zone() {
        // « R 1 » est écrit en haut à gauche de la page (72, 720 : 122 points sous le haut).
        let pdf = Arc::new(pdf_herite(2, "R"));
        let haut = Zone {
            x: 0.05,
            y: 0.08,
            largeur: 0.5,
            hauteur: 0.1,
        };
        let (extrait, en_image) = pdf_zone(pdf.clone(), "a.pdf", 0, haut).unwrap();
        assert!(!en_image);
        let document = lopdf::Document::load_mem(&extrait).unwrap();
        assert_eq!(document.get_pages().len(), 1);
        let page = *document.get_pages().values().next().unwrap();
        let boite = crate::geometrie::rectangle(
            &document,
            document
                .get_dictionary(page)
                .unwrap()
                .get(b"MediaBox")
                .unwrap(),
        )
        .unwrap();
        assert!((boite[2] - boite[0] - 297.5).abs() < 1.0, "{boite:?}");
        assert!((boite[3] - boite[1] - 84.2).abs() < 1.0, "{boite:?}");
        assert!(textes(&extrait)[0].contains("R 1"));
        // La zone du bas de la page ne garde pas le texte du haut : il est retiré, pas caché.
        let bas = Zone {
            x: 0.0,
            y: 0.6,
            largeur: 1.0,
            hauteur: 0.4,
        };
        let (extrait, _) = pdf_zone(pdf.clone(), "a.pdf", 1, bas).unwrap();
        // Le bas du cadre coupe le pied des lettres de « R 1 » (ligne de base à 720 points du
        // bas, cadre jusqu'à 717) : elles restent, coupées au bord par le recadrage.
        let rase = Zone {
            x: 0.05,
            y: 42.0 / 842.0,
            largeur: 0.5,
            hauteur: 83.0 / 842.0,
        };
        let (coupe, _) = pdf_zone(pdf.clone(), "a.pdf", 0, rase).unwrap();
        assert!(textes(&coupe)[0].contains("R 1"));
        let contenu = String::from_utf8_lossy(&extrait).into_owned();
        assert!(!textes(&extrait)[0].contains("R 2"));
        assert!(!contenu.contains("(R 2)"));
    }

    #[test]
    fn suit_la_rotation_de_la_page() {
        // Une page tournée d'un quart de tour : la zone est prise sur la page telle qu'elle
        // s'affiche (842 × 595), et le PDF la montre de la même façon.
        let mut document = lopdf::Document::load_mem(&pdf_herite(1, "R")).unwrap();
        let page = *document.get_pages().values().next().unwrap();
        document.get_dictionary_mut(page).unwrap().set("Rotate", 90);
        let mut tournee = Vec::new();
        document.save_to(&mut tournee).unwrap();
        let zone = Zone {
            x: 0.0,
            y: 0.0,
            largeur: 0.5,
            hauteur: 1.0,
        };
        let (extrait, en_image) = pdf_zone(Arc::new(tournee.clone()), "t.pdf", 0, zone).unwrap();
        assert!(!en_image);
        let formats = rendu::formats(Arc::new(extrait), "t.pdf").unwrap();
        assert!((formats[0].largeur - 421.0).abs() < 1.5, "{:?}", formats[0]);
        assert!((formats[0].hauteur - 595.0).abs() < 1.5, "{:?}", formats[0]);
        let (l, h, _) = pixels_zone(Arc::new(tournee), "t.pdf", 0, zone, 72).unwrap();
        assert!(
            (420..=422).contains(&l) && (594..=596).contains(&h),
            "{l} × {h}"
        );
    }

    #[test]
    fn recadre_les_pages_choisies() {
        // Trois pages ; « R 1 », « R 2 », « R 3 » en haut à gauche.
        let pdf = pdf_herite(3, "R");
        let haut = Zone {
            x: 0.05,
            y: 0.08,
            largeur: 0.5,
            hauteur: 0.1,
        };
        let pdf = Arc::new(pdf);
        let (recadre, bilan) = recadrer(pdf.clone(), "a.pdf", &[0, 2], Some(haut)).unwrap();
        assert!(bilan.pages_cachees.is_empty());
        let formats = rendu::formats(Arc::new(recadre.clone()), "a.pdf").unwrap();
        assert_eq!(formats.len(), 3);
        assert!((formats[0].largeur - 297.5).abs() < 1.0, "{:?}", formats[0]);
        assert!((formats[2].hauteur - 84.2).abs() < 1.0, "{:?}", formats[2]);
        // La page 2 n'est pas recadrée.
        assert!((formats[1].largeur - 595.0).abs() < 1.0, "{:?}", formats[1]);
        let lus = textes(&recadre);
        assert!(lus[0].contains("R 1") && lus[1].contains("R 2") && lus[2].contains("R 3"));
        // Toutes les pages, sur leur bas : le texte du haut est retiré, pas caché.
        let bas = Zone {
            x: 0.0,
            y: 0.6,
            largeur: 1.0,
            hauteur: 0.4,
        };
        let (vide, _) = recadrer(pdf.clone(), "a.pdf", &[], Some(bas)).unwrap();
        assert!(textes(&vide).iter().all(|texte| !texte.contains('R')));
        assert!(recadrer(pdf.clone(), "a.pdf", &[3], Some(haut)).is_err());
        // Chaque page sur son contenu : « R n » seul, ses marges retirées.
        let (serre, bilan) = recadrer(pdf, "a.pdf", &[], None).unwrap();
        assert!(bilan.pages_blanches.is_empty());
        let formats = rendu::formats(Arc::new(serre.clone()), "a.pdf").unwrap();
        assert!(
            formats.iter().all(|f| f.largeur < 80.0 && f.hauteur < 45.0),
            "{formats:?}"
        );
        assert!(
            textes(&serre)
                .iter()
                .zip(["R 1", "R 2", "R 3"])
                .all(|(texte, attendu)| texte.contains(attendu))
        );
    }

    #[test]
    fn trouve_les_bords_du_contenu() {
        // « R 1 » en 24 points, à 72 points du bord gauche, ligne de base à 720 points du bas.
        let bords = bords_du_contenu(Arc::new(pdf_herite(1, "R")), "a.pdf", 0)
            .unwrap()
            .unwrap();
        assert!(bords.x > 0.08 && bords.x < 0.13, "{bords:?}");
        assert!(bords.y > 0.11 && bords.y < 0.15, "{bords:?}");
        assert!(bords.largeur < 0.15 && bords.hauteur < 0.06, "{bords:?}");
        // Une page blanche n'a pas de bords.
        let mut blanc = Vec::new();
        image::RgbImage::from_pixel(100, 100, image::Rgb([255, 255, 255]))
            .write_to(
                &mut std::io::Cursor::new(&mut blanc),
                image::ImageFormat::Png,
            )
            .unwrap();
        let source = ImageSource {
            nom: "blanc.png".into(),
            octets: &blanc,
            rotation: 0,
            ppp: None,
        };
        let options = OptionsImages {
            format: FormatFeuille::A4,
            marge_mm: 0.0,
        };
        let page_blanche = images::images_en_pdf(&[source], options).unwrap();
        assert!(
            bords_du_contenu(Arc::new(page_blanche), "b.pdf", 0)
                .unwrap()
                .is_none()
        );
    }

    /// Le presse-papiers de ce Mac : image puis texte, relus. Il remplace ce qui s'y trouvait
    /// (le texte d'avant est remis) : à lancer à la main.
    #[test]
    #[ignore = "remplit le presse-papiers de l'ordinateur"]
    fn remplit_le_presse_papiers() {
        let mut presse = arboard::Clipboard::new().unwrap();
        let avant = presse.get_text().ok();
        let pdf = Arc::new(pdf_herite(1, "R"));
        let zone = Zone {
            x: 0.0,
            y: 0.0,
            largeur: 0.5,
            hauteur: 0.25,
        };
        let pixels = pixels_zone(pdf, "a.pdf", 0, zone, 72).unwrap();
        let (l, h) = (pixels.0 as usize, pixels.1 as usize);
        copier_image(pixels).unwrap();
        let relue = presse.get_image().unwrap();
        assert_eq!((relue.width, relue.height), (l, h));
        copier_texte("Automat-PDF : texte d'une zone").unwrap();
        assert_eq!(presse.get_text().unwrap(), "Automat-PDF : texte d'une zone");
        if let Some(texte) = avant {
            presse.set_text(texte).unwrap();
        }
    }
}
