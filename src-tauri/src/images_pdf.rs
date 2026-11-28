//! Les images d'un PDF, dans leur définition d'origine : celle que l'on désigne d'un clic droit
//! dans le lecteur (enregistrée ou copiée), ou toutes à la fois, dans un dossier (« Extraire les
//! images »). Un JPEG est repris tel quel, octet pour octet ; les autres images deviennent des PNG.

use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use serde::Serialize;

use crate::capture;
use crate::contenu::{self, ImagePlacee, Images};
use crate::docx::Format;
use crate::erreur::{Echec, Erreur};
use crate::fichiers::{self, FichierAEcrire};
use crate::memoire;
use crate::office::Conversions;
use crate::operations::{nom_de, sans_extension};
use crate::volume;

/// Une image d'une page, telle que le lecteur la désigne : sa place sur la page affichée (points,
/// depuis le coin haut gauche), sa clé (la même image, ailleurs, a la même), son format et sa
/// définition en pixels.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceImage {
    pub cle: String,
    pub x: f64,
    pub y: f64,
    pub largeur: f64,
    pub hauteur: f64,
    pub format: &'static str,
    pub pixels: (u32, u32),
}

fn extension(format: Format) -> &'static str {
    match format {
        Format::Jpeg => "jpg",
        Format::Png => "png",
    }
}

/// La place des images de la page `page` (à partir de 0), dans l'ordre où elle les dessine : la
/// dernière est au-dessus.
pub fn places(octets: Arc<Vec<u8>>, nom: &str, page: usize) -> Result<Vec<PlaceImage>, Erreur> {
    let pages = contenu::relever(octets, nom, Some(&[page]), Images::Places)?;
    Ok(pages
        .into_iter()
        .flat_map(|page| page.images)
        .map(|image| PlaceImage {
            cle: image.cle.to_string(),
            x: image.gauche,
            y: image.haut,
            largeur: image.largeur,
            hauteur: image.hauteur,
            format: match image.format {
                Format::Jpeg => "jpeg",
                Format::Png => "png",
            },
            pixels: image.pixels,
        })
        .collect())
}

/// L'image de clé `cle` de la page `page`, dans sa définition d'origine.
pub fn image(
    octets: Arc<Vec<u8>>,
    nom: &str,
    page: usize,
    cle: &str,
) -> Result<ImagePlacee, Erreur> {
    let illisible = || Erreur::simple("Cette image n'a pas pu être lue.");
    let cle: u128 = cle.parse().map_err(|_| illisible())?;
    contenu::relever(octets, nom, Some(&[page]), Images::Origine)?
        .into_iter()
        .flat_map(|page| page.images)
        .find(|image| image.cle == cle)
        .ok_or_else(illisible)
}

/// Format d'un fichier image d'après son extension : JPG pour .jpg et .jpeg, sinon PNG.
pub fn format_du_fichier(chemin: &str) -> Format {
    match Path::new(chemin)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => Format::Jpeg,
        _ => Format::Png,
    }
}

fn illisible(erreur: image::ImageError) -> Erreur {
    Erreur::simple(format!("Cette image n'a pas pu être lue : {erreur}"))
}

/// Une image posée sur un fond blanc (le JPG n'a pas de transparence).
fn sur_blanc(image: &DynamicImage) -> RgbImage {
    let rvba = image.to_rgba8();
    RgbImage::from_fn(rvba.width(), rvba.height(), |x, y| {
        let [rouge, vert, bleu, alpha] = rvba.get_pixel(x, y).0;
        let melange = |couleur: u8| {
            ((u32::from(couleur) * u32::from(alpha) + 255 * (255 - u32::from(alpha))) / 255) as u8
        };
        Rgb([melange(rouge), melange(vert), melange(bleu)])
    })
}

/// Les octets de l'image dans le format voulu : tels quels si c'est le sien, sinon convertis.
pub fn octets_en(image: &ImagePlacee, format: Format) -> Result<Vec<u8>, Erreur> {
    if image.format == format {
        return Ok(image.octets.to_vec());
    }
    let lue = image::load_from_memory(&image.octets).map_err(illisible)?;
    let mut sortie = Vec::new();
    match format {
        Format::Png => lue
            .write_to(&mut Cursor::new(&mut sortie), ImageFormat::Png)
            .map_err(illisible)?,
        Format::Jpeg => JpegEncoder::new_with_quality(&mut sortie, 92)
            .encode_image(&sur_blanc(&lue))
            .map_err(illisible)?,
    }
    Ok(sortie)
}

/// Met l'image dans le presse-papiers, transparence comprise.
pub fn copier(image: &ImagePlacee) -> Result<(), Erreur> {
    let rvba = image::load_from_memory(&image.octets)
        .map_err(illisible)?
        .to_rgba8();
    capture::copier_pixels(rvba.width(), rvba.height(), rvba.into_raw())
}

/// Petite image (puce, filet, icône) : moins de 32 pixels de côté, ou de 64 × 64 en tout.
pub fn petite(image: &ImagePlacee) -> bool {
    let (largeur, hauteur) = image.pixels;
    largeur.min(hauteur) < 32 || u64::from(largeur) * u64::from(hauteur) < 64 * 64
}

/// Les images de chaque PDF, dans un dossier « nom - images extraites » créé dans `dossier` :
/// chacune une seule fois, nommée d'après la première page où elle paraît (« page 03 - image
/// 2.jpg »). `ignorer_petites` : les puces, filets et icônes sont laissés. `avancement(faits,
/// fichiers)`.
pub fn extraire(
    fichiers: &[String],
    dossier: &str,
    ignorer_petites: bool,
    avancement: impl Fn(usize, usize),
) -> Result<Conversions, Erreur> {
    volume::autorise("Extraire les images", fichiers.len())?;
    let mut resultat = Conversions::default();
    for (rang, chemin) in fichiers.iter().enumerate() {
        avancement(rang, fichiers.len());
        match extraire_un(chemin, dossier, ignorer_petites) {
            Ok(cible) => resultat.crees.push(cible),
            Err(erreur) => resultat.echecs.push(Echec {
                rang,
                message: erreur.message,
            }),
        }
    }
    avancement(fichiers.len(), fichiers.len());
    Ok(resultat)
}

/// Les images d'un PDF, écrites au fur et à mesure ; le chemin du dossier créé.
fn extraire_un(chemin: &str, dossier: &str, ignorer_petites: bool) -> Result<String, Erreur> {
    let nom = nom_de(chemin);
    let sous_dossier = format!("{} - images extraites", sans_extension(&nom));
    let octets = memoire::pdf(chemin)?;
    let chiffres = crate::pdf::nombre_de_pages(&octets, &nom)?
        .to_string()
        .len()
        .max(2);
    let mut vues = HashSet::new();
    let mut cible = None;
    contenu::relever_une_a_une(octets, &nom, None, Images::Origine, |numero, page| {
        let mut rang = 0;
        for image in page.images {
            if !vues.insert(image.cle) || (ignorer_petites && petite(&image)) {
                continue;
            }
            rang += 1;
            let fichier = FichierAEcrire {
                nom: format!(
                    "page {:0chiffres$} - image {rang}.{}",
                    numero + 1,
                    extension(image.format)
                ),
                texte: None,
                octets: Some(image.octets.to_vec()),
            };
            cible = Some(fichiers::ecrire(dossier, &sous_dossier, &[fichier])?);
        }
        Ok(())
    })?;
    cible.ok_or_else(|| {
        Erreur::simple(if ignorer_petites {
            "Aucune image dans ce PDF (les petites, puces et icônes, sont laissées)."
        } else {
            "Aucune image dans ce PDF."
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::fichier_temporaire;
    use crate::images::{FormatFeuille, ImageSource, OptionsImages};

    /// Une photo JPEG de `largeur` × `hauteur` pixels, d'une couleur.
    fn photo(largeur: u32, hauteur: u32, couleur: [u8; 3]) -> Vec<u8> {
        let image = RgbImage::from_pixel(largeur, hauteur, Rgb(couleur));
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, 90)
            .encode_image(&image)
            .unwrap();
        jpeg
    }

    /// Un PDF d'une page par image, chacune à sa taille réelle (72 ppp).
    fn pdf_de(images: &[Vec<u8>]) -> Vec<u8> {
        let sources: Vec<ImageSource> = images
            .iter()
            .enumerate()
            .map(|(rang, octets)| ImageSource {
                nom: format!("photo {rang}.jpg"),
                octets,
                rotation: 0,
                ppp: Some(72.0),
            })
            .collect();
        let options = OptionsImages {
            format: FormatFeuille::Reelle,
            marge_mm: 0.0,
        };
        crate::images::images_en_pdf(&sources, options).unwrap()
    }

    #[test]
    fn situe_les_images_et_reprend_le_jpeg_tel_quel() {
        let jpeg = photo(640, 480, [200, 40, 40]);
        let pdf = Arc::new(pdf_de(std::slice::from_ref(&jpeg)));
        let places = places(pdf.clone(), "a.pdf", 0).unwrap();
        assert_eq!(places.len(), 1);
        let place = &places[0];
        assert_eq!((place.format, place.pixels), ("jpeg", (640, 480)));
        assert!((place.largeur - 640.0).abs() < 1.0 && (place.hauteur - 480.0).abs() < 1.0);
        let lue = image(pdf.clone(), "a.pdf", 0, &place.cle).unwrap();
        // Le JPEG d'origine, octet pour octet.
        assert_eq!(lue.octets.as_slice(), jpeg.as_slice());
        let png = octets_en(&lue, Format::Png).unwrap();
        let relue = image::load_from_memory(&png).unwrap();
        assert_eq!((relue.width(), relue.height()), (640, 480));
        assert!(image(pdf, "a.pdf", 0, "12345").is_err());
        assert_eq!(format_du_fichier("C:\\Photos\\a.JPEG"), Format::Jpeg);
        assert_eq!(format_du_fichier("/tmp/a.png"), Format::Png);
    }

    /// Le PDF avec, à la fin, une copie de sa première page : la même image y revient (le même
    /// objet du PDF, comme un logo sur chaque page).
    fn avec_la_premiere_page_en_double(pdf: Vec<u8>) -> Vec<u8> {
        use lopdf::Object;
        let mut document = lopdf::Document::load_mem(&pdf).unwrap();
        let premiere = *document.get_pages().values().next().unwrap();
        let copie = document.get_object(premiere).unwrap().clone();
        let copie = document.add_object(copie);
        let parent = document
            .get_dictionary(premiere)
            .unwrap()
            .get(b"Parent")
            .unwrap()
            .as_reference()
            .unwrap();
        let arbre = document.get_dictionary_mut(parent).unwrap();
        let mut enfants = arbre.get(b"Kids").unwrap().as_array().unwrap().clone();
        enfants.push(Object::Reference(copie));
        let nombre = enfants.len() as i64;
        arbre.set("Kids", enfants);
        arbre.set("Count", nombre);
        let mut sortie = Vec::new();
        document.save_to(&mut sortie).unwrap();
        sortie
    }

    #[test]
    fn extrait_chaque_image_une_fois_sans_les_petites() {
        let grande = photo(400, 300, [10, 120, 200]);
        let icone = photo(16, 16, [0, 0, 0]);
        // La grande photo revient en page 3 : elle n'est extraite qu'une fois.
        let pdf = avec_la_premiere_page_en_double(pdf_de(&[grande, icone]));
        assert_eq!(crate::pdf::nombre_de_pages(&pdf, "c.pdf").unwrap(), 3);
        let chemin = fichier_temporaire("images-extraites", "Catalogue.pdf", &pdf);
        let dossier = std::env::temp_dir()
            .join("automat-pdf-tests")
            .join("images-extraites");
        let _ = std::fs::remove_dir_all(dossier.join("Catalogue - images extraites"));
        let bilan = extraire(
            std::slice::from_ref(&chemin),
            dossier.to_str().unwrap(),
            true,
            |_, _| {},
        )
        .unwrap();
        assert!(bilan.echecs.is_empty(), "{:?}", bilan.echecs);
        let mut noms: Vec<String> = std::fs::read_dir(&bilan.crees[0])
            .unwrap()
            .map(|f| f.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        noms.sort();
        // Chaque page est une image ; la photo de la page 3 est celle de la page 1 (même clé).
        assert_eq!(noms, ["page 01 - image 1.jpg"]);
        let _ = std::fs::remove_dir_all(dossier.join("Catalogue - images extraites"));
        let avec_petites =
            extraire(&[chemin], dossier.to_str().unwrap(), false, |_, _| {}).unwrap();
        let nombre = std::fs::read_dir(&avec_petites.crees[0]).unwrap().count();
        assert_eq!(nombre, 2);
        // Un PDF sans image : un échec qui le dit.
        let texte = fichier_temporaire(
            "images-extraites",
            "Lettre.pdf",
            &crate::essais::pdf_herite(1, "L"),
        );
        let sans = extraire(&[texte], dossier.to_str().unwrap(), true, |_, _| {}).unwrap();
        assert!(sans.crees.is_empty());
        assert!(sans.echecs[0].message.contains("Aucune image"));
    }
}
