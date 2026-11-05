//! Images (photos, scans) réunies en un PDF, une image par page. Les JPEG sont repris tels quels :
//! aucune perte de qualité, et une photo prise en portrait est redressée par la rotation de la page.
//! Les autres formats (PNG, WEBP, TIFF, BMP, GIF, JPEG en CMJN ou en miroir) sont décodés puis
//! compressés sans perte, transparence comprise.

use std::io::Cursor;

use image::codecs::jpeg::{JpegDecoder, JpegEncoder};
use image::metadata::Orientation;
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageFormat, ImageReader};
use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use serde::Deserialize;

use crate::erreur::Erreur;

const POINTS_PAR_MM: f32 = 72.0 / 25.4;
/// Feuille A4 en portrait, en points.
const A4: (f32, f32) = (595.28, 841.89);

/// Taille des pages : A4 (portrait ou paysage selon l'image), ou ajustée aux proportions de l'image.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FormatFeuille {
    A4,
    Ajustee,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsImages {
    pub format: FormatFeuille,
    /// Marge autour de l'image sur une feuille A4, en mm.
    pub marge_mm: f32,
}

/// Une image à placer : son nom (pour les messages), son contenu et la rotation ajoutée par
/// l'utilisateur (degrés, sens horaire).
pub struct ImageSource<'a> {
    pub nom: String,
    pub octets: &'a [u8],
    pub rotation: i64,
}

/// Image prête à être incorporée : son flux PDF (et celui de sa transparence), sa taille en
/// pixels et la rotation qui la redresse.
pub(crate) struct ImagePdf {
    pub flux: Stream,
    pub transparence: Option<Stream>,
    pub largeur: u32,
    pub hauteur: u32,
    pub rotation: i64,
}

/// Précision, taille et nombre de composantes d'un JPEG, lus dans son en-tête ; seuls les JPEG
/// classiques ou progressifs (que tout lecteur de PDF affiche) sont retenus.
fn entete_jpeg(octets: &[u8]) -> Option<(u8, u16, u16, u8)> {
    if octets.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut i = 2;
    while i + 4 <= octets.len() {
        if octets[i] != 0xFF {
            i += 1;
            continue;
        }
        let marqueur = octets[i + 1];
        match marqueur {
            0xFF => i += 1,
            0x01 | 0xD0..=0xD8 => i += 2,
            0xC0..=0xC2 => {
                let d = octets.get(i + 4..i + 10)?;
                return Some((
                    d[0],
                    u16::from_be_bytes([d[1], d[2]]),
                    u16::from_be_bytes([d[3], d[4]]),
                    d[5],
                ));
            }
            // Autres codages (arithmétique, sans perte…) ou image commencée sans en-tête.
            0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF | 0xDA => return None,
            _ => i += 2 + usize::from(u16::from_be_bytes([octets[i + 2], octets[i + 3]])),
        }
    }
    None
}

/// Rotation (degrés, sens horaire) qui correspond à une orientation EXIF, si c'est une simple
/// rotation (sans effet miroir).
fn quart_de_tour(orientation: Orientation) -> Option<i64> {
    match orientation {
        Orientation::NoTransforms => Some(0),
        Orientation::Rotate90 => Some(90),
        Orientation::Rotate180 => Some(180),
        Orientation::Rotate270 => Some(270),
        _ => None,
    }
}

/// Image décodée et redressée (orientation EXIF appliquée).
fn decoder(octets: &[u8], nom: &str) -> Result<DynamicImage, Erreur> {
    let illisible = |erreur: image::ImageError| {
        Erreur::simple(format!("« {nom} » n'a pas pu être lue : {erreur}"))
    };
    let lecteur = ImageReader::new(Cursor::new(octets))
        .with_guessed_format()
        .map_err(|erreur| Erreur::simple(format!("« {nom} » n'a pas pu être lue : {erreur}")))?;
    let mut decodeur = lecteur.into_decoder().map_err(illisible)?;
    let orientation = decodeur.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decodeur).map_err(illisible)?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn flux_image(largeur: u32, hauteur: u32, gris: bool, pixels: Vec<u8>) -> Stream {
    Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image",
            "Width" => i64::from(largeur), "Height" => i64::from(hauteur),
            "ColorSpace" => if gris { "DeviceGray" } else { "DeviceRGB" },
            "BitsPerComponent" => 8,
        },
        pixels,
    )
}

fn preparer(source: &ImageSource) -> Result<ImagePdf, Erreur> {
    if image::guess_format(source.octets).ok() == Some(ImageFormat::Jpeg)
        && let Some((8, hauteur, largeur, composantes @ (1 | 3))) = entete_jpeg(source.octets)
    {
        let orientation = JpegDecoder::new(Cursor::new(source.octets))
            .and_then(|mut decodeur| decodeur.orientation())
            .unwrap_or(Orientation::NoTransforms);
        if let Some(rotation) = quart_de_tour(orientation) {
            let mut flux = flux_image(
                u32::from(largeur),
                u32::from(hauteur),
                composantes == 1,
                source.octets.to_vec(),
            );
            flux.dict.set("Filter", "DCTDecode");
            return Ok(ImagePdf {
                flux,
                transparence: None,
                largeur: u32::from(largeur),
                hauteur: u32::from(hauteur),
                rotation,
            });
        }
    }
    image_decodee(source.octets, &source.nom)
}

/// Image décodée et redressée, prête à être incorporée : pixels compressés sans perte, masque de
/// transparence s'il y en a un (signature, cachet sur fond transparent).
pub(crate) fn image_decodee(octets: &[u8], nom: &str) -> Result<ImagePdf, Erreur> {
    let image = decoder(octets, nom)?;
    let (largeur, hauteur) = (image.width(), image.height());
    let gris = !image.color().has_color();
    let transparence = image
        .color()
        .has_alpha()
        .then(|| image.to_rgba8())
        .and_then(|rvba| {
            let alpha: Vec<u8> = rvba.pixels().map(|pixel| pixel.0[3]).collect();
            // Une image entièrement opaque n'a pas besoin de masque.
            alpha
                .iter()
                .any(|&a| a < 255)
                .then(|| flux_image(largeur, hauteur, true, alpha))
        });
    let pixels = if gris {
        image.to_luma8().into_raw()
    } else {
        image.to_rgb8().into_raw()
    };
    Ok(ImagePdf {
        flux: flux_image(largeur, hauteur, gris, pixels),
        transparence,
        largeur,
        hauteur,
        rotation: 0,
    })
}

/// Un PDF fait des images données, une par page, dans l'ordre.
pub fn images_en_pdf(images: &[ImageSource], options: OptionsImages) -> Result<Vec<u8>, Erreur> {
    if images.is_empty() {
        return Err(Erreur::simple("Aucune image à mettre en PDF."));
    }
    let mut document = Document::with_version("1.7");
    let arbre = document.new_object_id();
    let mut enfants = Vec::new();
    for source in images {
        let image = preparer(source)?;
        let rotation = (image.rotation + source.rotation).rem_euclid(360);
        let tournee = rotation % 180 != 0;
        let (largeur, hauteur) = (image.largeur as f32, image.hauteur as f32);
        // Taille de l'image et de la page telles qu'elles s'affichent.
        let (vue_l, vue_h) = if tournee {
            (hauteur, largeur)
        } else {
            (largeur, hauteur)
        };
        let (page_l, page_h, marge) = match options.format {
            FormatFeuille::A4 => {
                let marge = options.marge_mm.clamp(0.0, 50.0) * POINTS_PAR_MM;
                if vue_l > vue_h {
                    (A4.1, A4.0, marge)
                } else {
                    (A4.0, A4.1, marge)
                }
            }
            FormatFeuille::Ajustee if vue_l >= vue_h => (A4.1, A4.1 * vue_h / vue_l, 0.0),
            FormatFeuille::Ajustee => (A4.1 * vue_l / vue_h, A4.1, 0.0),
        };
        let echelle = ((page_l - 2.0 * marge) / vue_l).min((page_h - 2.0 * marge) / vue_h);
        // Les coordonnées de la page sont celles d'avant la rotation : la boîte tourne avec l'image.
        let (boite_l, boite_h) = if tournee {
            (page_h, page_l)
        } else {
            (page_l, page_h)
        };
        let (dessin_l, dessin_h) = (largeur * echelle, hauteur * echelle);
        let (x, y) = ((boite_l - dessin_l) / 2.0, (boite_h - dessin_h) / 2.0);
        let mut flux = image.flux;
        if let Some(transparence) = image.transparence {
            flux.dict.set("SMask", document.add_object(transparence));
        }
        let image_id = document.add_object(flux);
        let contenu = format!("q {dessin_l:.3} 0 0 {dessin_h:.3} {x:.3} {y:.3} cm /Im0 Do Q");
        let contenu_id = document.add_object(Stream::new(Dictionary::new(), contenu.into_bytes()));
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => arbre,
            "MediaBox" => vec![0.into(), 0.into(), boite_l.into(), boite_h.into()],
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image_id } },
            "Contents" => contenu_id,
            "Rotate" => rotation,
        });
        enfants.push(Object::Reference(page));
    }
    let nombre = enfants.len() as i64;
    document.objects.insert(
        arbre,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => enfants, "Count" => nombre }),
    );
    let catalogue = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => arbre });
    document.trailer.set("Root", catalogue);
    document.compress();
    let mut octets = Vec::new();
    document
        .save_to(&mut octets)
        .map_err(|erreur| Erreur::simple(format!("Le PDF n'a pas pu être écrit : {erreur}")))?;
    Ok(octets)
}

/// Pixels RVB d'une image, sa transparence posée sur un fond blanc (comme sur la page).
fn sur_fond_blanc(image: &DynamicImage) -> image::RgbImage {
    if !image.color().has_alpha() {
        return image.to_rgb8();
    }
    let rvba = image.to_rgba8();
    image::RgbImage::from_fn(rvba.width(), rvba.height(), |x, y| {
        let [r, v, b, a] = rvba.get_pixel(x, y).0;
        let melange =
            |c: u8| ((u16::from(c) * u16::from(a) + 255 * u16::from(255 - a)) / 255) as u8;
        image::Rgb([melange(r), melange(v), melange(b)])
    })
}

/// Vignette JPEG d'une image (redressée), large de `largeur` pixels au plus.
pub fn vignette(octets: &[u8], nom: &str, largeur: u32) -> Result<Vec<u8>, Erreur> {
    let image = decoder(octets, nom)?.thumbnail(largeur, largeur.saturating_mul(4));
    let rvb = sur_fond_blanc(&image);
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 80)
        .encode(&rvb, rvb.width(), rvb.height(), ExtendedColorType::Rgb8)
        .map_err(|erreur| {
            Erreur::simple(format!("La vignette n'a pas pu être créée : {erreur}"))
        })?;
    Ok(jpeg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, Rgba, RgbaImage};

    fn jpeg(largeur: u32, hauteur: u32) -> Vec<u8> {
        let image = DynamicImage::new_rgb8(largeur, hauteur);
        let mut octets = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut octets), ImageFormat::Jpeg)
            .unwrap();
        octets
    }

    fn png_transparent(largeur: u32, hauteur: u32) -> Vec<u8> {
        let image = RgbaImage::from_fn(largeur, hauteur, |x, _| {
            Rgba([200, 30, 30, if x < largeur / 2 { 0 } else { 255 }])
        });
        let mut octets = Vec::new();
        image::codecs::png::PngEncoder::new(&mut octets)
            .write_image(image.as_raw(), largeur, hauteur, ExtendedColorType::Rgba8)
            .unwrap();
        octets
    }

    /// Pour chaque page : sa boîte (largeur, hauteur), sa rotation et le filtre de son image.
    fn pages(pdf: &[u8]) -> Vec<(f32, f32, i64, String)> {
        let document = Document::load_mem(pdf).unwrap();
        document
            .get_pages()
            .values()
            .map(|id| {
                let page = document.get_dictionary(*id).unwrap();
                let boite = page.get(b"MediaBox").unwrap().as_array().unwrap();
                let nombre = |o: &Object| o.as_float().unwrap();
                let ressources = page.get(b"Resources").unwrap().as_dict().unwrap();
                let image = ressources
                    .get(b"XObject")
                    .unwrap()
                    .as_dict()
                    .unwrap()
                    .get(b"Im0")
                    .unwrap();
                let image = document
                    .get_object(image.as_reference().unwrap())
                    .unwrap()
                    .as_stream()
                    .unwrap();
                let filtre = image
                    .dict
                    .get(b"Filter")
                    .and_then(Object::as_name)
                    .map(|n| String::from_utf8_lossy(n).into_owned())
                    .unwrap_or_default();
                (
                    nombre(&boite[2]),
                    nombre(&boite[3]),
                    page.get(b"Rotate").unwrap().as_i64().unwrap(),
                    filtre,
                )
            })
            .collect()
    }

    #[test]
    fn reprend_les_jpeg_tels_quels_sur_une_feuille_a4() {
        let photo = jpeg(400, 300);
        let images = [
            ImageSource {
                nom: "paysage.jpg".into(),
                octets: &photo,
                rotation: 0,
            },
            ImageSource {
                nom: "tournee.jpg".into(),
                octets: &photo,
                rotation: 90,
            },
        ];
        let pdf = images_en_pdf(
            &images,
            OptionsImages {
                format: FormatFeuille::A4,
                marge_mm: 10.0,
            },
        )
        .unwrap();
        let pages = pages(&pdf);
        // Image en paysage : A4 en paysage ; tournée d'un quart de tour : la boîte tourne avec elle.
        assert_eq!(pages[0], (841.89, 595.28, 0, "DCTDecode".into()));
        assert_eq!(pages[1], (841.89, 595.28, 90, "DCTDecode".into()));
        // Le JPEG d'origine est dans le PDF, octet pour octet.
        assert!(
            pdf.windows(photo.len())
                .any(|fenetre| fenetre == photo.as_slice())
        );
    }

    #[test]
    fn garde_la_transparence_d_un_png_et_ajuste_la_page() {
        let png = png_transparent(200, 100);
        let images = [ImageSource {
            nom: "logo.png".into(),
            octets: &png,
            rotation: 0,
        }];
        let pdf = images_en_pdf(
            &images,
            OptionsImages {
                format: FormatFeuille::Ajustee,
                marge_mm: 10.0,
            },
        )
        .unwrap();
        let (largeur, hauteur, _, filtre) = pages(&pdf)[0].clone();
        assert_eq!(largeur, 841.89);
        assert!((hauteur - 420.945).abs() < 0.01, "{hauteur}");
        assert_eq!(filtre, "FlateDecode");
        let document = Document::load_mem(&pdf).unwrap();
        assert!(
            document
                .objects
                .values()
                .any(|o| o.as_stream().is_ok_and(|s| s.dict.has(b"SMask")))
        );
    }

    #[test]
    fn lit_l_en_tete_des_jpeg_et_refuse_le_reste() {
        assert_eq!(entete_jpeg(&jpeg(40, 30)), Some((8, 30, 40, 3)));
        assert_eq!(entete_jpeg(b"pas une image"), None);
        let erreur = images_en_pdf(
            &[ImageSource {
                nom: "faux.png".into(),
                octets: b"pas une image",
                rotation: 0,
            }],
            OptionsImages {
                format: FormatFeuille::A4,
                marge_mm: 0.0,
            },
        )
        .unwrap_err();
        assert!(erreur.message.contains("faux.png"));
        assert!(
            vignette(&jpeg(400, 300), "v.jpg", 100)
                .unwrap()
                .starts_with(&[0xFF, 0xD8])
        );
    }

    #[test]
    fn pose_la_transparence_des_vignettes_sur_du_blanc() {
        let vignette = vignette(&png_transparent(200, 100), "logo.png", 100).unwrap();
        let image = image::load_from_memory(&vignette).unwrap().to_rgb8();
        // Moitié gauche transparente : blanche ; moitié droite opaque : rouge.
        assert!(image.get_pixel(10, 25).0.iter().all(|&c| c > 240));
        assert!(image.get_pixel(90, 25).0[0] > 150 && image.get_pixel(90, 25).0[1] < 80);
    }
}
