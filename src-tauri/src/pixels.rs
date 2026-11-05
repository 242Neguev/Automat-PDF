//! Pixels des images d'un document (8 bits par composante) : pour en noircir une partie
//! (caviardage) ou les réduire (compression). Seuls les formats que l'on sait relire sans risque
//! sont pris en charge ; les autres images sont laissées telles quelles.

use std::io::Cursor;

use image::codecs::jpeg::JpegDecoder;
use image::{DynamicImage, ImageDecoder};
use lopdf::{Document, Object, Stream};

use crate::geometrie::{Matrice, boite, resoudre};

/// Rectangle dans le repère de la page : x0, y0, x1, y1.
type Rect = [f64; 4];

/// Pixels d'une image (8 bits par composante).
pub(crate) struct Pixels {
    pub flux: Stream,
    pub pixels: Vec<u8>,
    pub largeur: usize,
    pub hauteur: usize,
    pub composantes: usize,
}

impl Pixels {
    /// Pixels d'une image du document ; `None` si son format ne permet pas de les modifier sûrement.
    pub fn de(document: &Document, flux: &Stream) -> Option<Pixels> {
        let dict = &flux.dict;
        let entier = |cle: &[u8]| {
            dict.get(cle)
                .ok()
                .and_then(|v| resoudre(document, v).as_i64().ok())
        };
        if dict.get(b"ImageMask").ok().and_then(|v| v.as_bool().ok()) == Some(true)
            || dict.has(b"Decode")
            || entier(b"BitsPerComponent") != Some(8)
        {
            return None;
        }
        let (largeur, hauteur) = (
            usize::try_from(entier(b"Width")?).ok()?,
            usize::try_from(entier(b"Height")?).ok()?,
        );
        let espace = resoudre(document, dict.get(b"ColorSpace").ok()?);
        let composantes = match espace {
            Object::Name(nom) if nom == b"DeviceGray" => 1,
            Object::Name(nom) if nom == b"DeviceRGB" => 3,
            Object::Name(nom) if nom == b"DeviceCMYK" => 4,
            Object::Array(espace)
                if espace.first().and_then(|n| n.as_name().ok()) == Some(b"ICCBased") =>
            {
                let profil = resoudre(document, espace.get(1)?).as_stream().ok()?;
                usize::try_from(profil.dict.get(b"N").ok()?.as_i64().ok()?).ok()?
            }
            _ => return None,
        };
        let taille = largeur.checked_mul(hauteur)?.checked_mul(composantes)?;
        let filtres: Vec<Vec<u8>> = match dict.get(b"Filter").map(|f| resoudre(document, f)) {
            Ok(Object::Name(nom)) => vec![nom.clone()],
            Ok(Object::Array(liste)) => liste
                .iter()
                .filter_map(|f| f.as_name().ok().map(<[u8]>::to_vec))
                .collect(),
            _ => Vec::new(),
        };
        let pixels = match filtres.as_slice() {
            [] => flux.content.clone(),
            [filtre] if filtre == b"DCTDecode" => {
                let decodeur = JpegDecoder::new(Cursor::new(&flux.content)).ok()?;
                if decodeur.dimensions()
                    != (u32::try_from(largeur).ok()?, u32::try_from(hauteur).ok()?)
                {
                    return None;
                }
                let image = DynamicImage::from_decoder(decodeur).ok()?;
                match composantes {
                    1 => image.to_luma8().into_raw(),
                    3 => image.to_rgb8().into_raw(),
                    _ => return None,
                }
            }
            liste
                if liste.iter().all(|f| {
                    f == b"FlateDecode"
                        || f == b"LZWDecode"
                        || f == b"ASCII85Decode"
                        || f == b"ASCIIHexDecode"
                        || f == b"RunLengthDecode"
                }) =>
            {
                // Plafonné : un flux piégé (« bombe de décompression ») ne peut pas saturer la mémoire.
                flux.decompressed_content_with_limit(
                    taille.saturating_mul(2).saturating_add(1 << 20),
                )
                .ok()?
            }
            _ => return None,
        };
        (pixels.len() >= taille).then(|| Pixels {
            flux: flux.clone(),
            pixels,
            largeur,
            hauteur,
            composantes,
        })
    }

    /// Noircit les pixels couverts par les zones ; l'image est dessinée par `ctm` (carré unité).
    pub fn noircir(&mut self, ctm: Matrice, zones: &[Rect]) {
        let inverse = ctm.inverse();
        for zone in zones {
            let [u0, v0, u1, v1] = boite(
                inverse,
                &[
                    (zone[0], zone[1]),
                    (zone[2], zone[1]),
                    (zone[0], zone[3]),
                    (zone[2], zone[3]),
                ],
            );
            if u1 <= 0.0 || v1 <= 0.0 || u0 >= 1.0 || v0 >= 1.0 {
                continue;
            }
            // La première ligne de l'image est en haut du carré unité ; un pixel de marge.
            let colonnes = (((u0 * self.largeur as f64).floor() as isize - 1).max(0) as usize)
                ..((u1 * self.largeur as f64).ceil() as usize + 1).min(self.largeur);
            let lignes = (((1.0 - v1) * self.hauteur as f64).floor() as isize - 1).max(0) as usize
                ..(((1.0 - v0) * self.hauteur as f64).ceil() as usize + 1).min(self.hauteur);
            for ligne in lignes {
                for colonne in colonnes.clone() {
                    let debut = (ligne * self.largeur + colonne) * self.composantes;
                    let pixel = &mut self.pixels[debut..debut + self.composantes];
                    if self.composantes == 4 {
                        pixel.copy_from_slice(&[0, 0, 0, 255]);
                    } else {
                        pixel.fill(0);
                    }
                }
            }
        }
    }

    /// Nouvelle image : pixels non compressés (le document est compressé à l'enregistrement).
    pub fn en_flux(self) -> Stream {
        let mut dict = self.flux.dict.clone();
        for cle in [&b"Filter"[..], b"DecodeParms", b"Length"] {
            dict.remove(cle);
        }
        Stream::new(dict, self.pixels)
    }
}
