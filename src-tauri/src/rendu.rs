//! Pages rendues en images (vignettes, aperçus) par hayro, un moteur de rendu PDF écrit en Rust :
//! rien à installer, les polices standard du PDF sont incluses.

use std::sync::{Arc, Mutex, OnceLock, Weak};

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::object::{Array, Dict, Name};
use hayro::hayro_syntax::{DecryptionError, LoadPdfError, Pdf};
use hayro::vello_cpu::color::palette::css::{TRANSPARENT, WHITE};
use hayro::{PixmapSettings, RenderCache, RenderSettings, render};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use serde::{Deserialize, Serialize};

use crate::erreur::Erreur;

/// Taille d'une page telle qu'elle s'affiche (en points, rotation comprise).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FormatPage {
    pub largeur: f32,
    pub hauteur: f32,
}

/// Plus grand côté d'une image rendue, en pixels (limite du moteur de rendu).
const COTE_MAX: f32 = 8000.0;
/// Copies « pour l'affichage » gardées : le rendu de chaque page d'un même PDF ne les refait pas.
const COPIES_GARDEES: usize = 6;

/// Un PDF déjà examiné pour l'affichage, et sa copie s'il lui en faut une.
struct CopieAffichage {
    source: Weak<Vec<u8>>,
    copie: Option<Arc<Vec<u8>>>,
}

fn copies() -> &'static Mutex<Vec<CopieAffichage>> {
    static COPIES: OnceLock<Mutex<Vec<CopieAffichage>>> = OnceLock::new();
    COPIES.get_or_init(|| Mutex::new(Vec::new()))
}

/// Le PDF tel que hayro doit le dessiner : le même, sauf si son formulaire demande ses apparences
/// (/NeedAppearances, champs sans apparence), que hayro ne dessine pas. Il lui faut alors une copie
/// préparée (`formulaires::pour_l_affichage`), gardée tant que le PDF l'est. Les cases à cocher et
/// les boutons radio n'en ont pas besoin : hayro 0.9 dessine l'apparence de leur état (/AS).
pub(crate) fn pour_l_affichage(octets: Arc<Vec<u8>>) -> Arc<Vec<u8>> {
    {
        let mut copies = copies()
            .lock()
            .unwrap_or_else(|empoisonne| empoisonne.into_inner());
        copies.retain(|c| c.source.strong_count() > 0);
        if let Some(examine) = copies.iter().find(|c| {
            c.source
                .upgrade()
                .is_some_and(|source| Arc::ptr_eq(&source, &octets))
        }) {
            return examine.copie.clone().unwrap_or(octets);
        }
    }
    let copie = a_preparer(&octets)
        .then(|| preparer(&octets))
        .flatten()
        .map(Arc::new);
    let mut copies = copies()
        .lock()
        .unwrap_or_else(|empoisonne| empoisonne.into_inner());
    copies.insert(
        0,
        CopieAffichage {
            source: Arc::downgrade(&octets),
            copie: copie.clone(),
        },
    );
    copies.truncate(COPIES_GARDEES);
    copie.unwrap_or(octets)
}

/// Faut-il préparer ce PDF pour l'affichage ? Lu par hayro, sans tout charger.
fn a_preparer(octets: &Arc<Vec<u8>>) -> bool {
    let Ok(pdf) = Pdf::new(octets.clone()) else {
        return false;
    };
    let xref = pdf.xref();
    let formulaire = xref
        .get::<Dict<'_>>(xref.root_id())
        .and_then(|catalogue| catalogue.get::<Dict<'_>>(b"AcroForm"));
    if formulaire
        .as_ref()
        .and_then(|f| f.get::<bool>(b"NeedAppearances"))
        .unwrap_or(false)
    {
        return true;
    }
    pdf.pages().iter().any(|page| {
        page.raw()
            .get::<Array<'_>>(b"Annots")
            .is_some_and(|annotations| {
                // Un champ sans apparence (formulaire).
                annotations.iter::<Dict<'_>>().any(|annotation| {
                    formulaire.is_some()
                        && !annotation.contains_key(b"AP")
                        && annotation
                            .get::<Name<'_>>(b"Subtype")
                            .is_some_and(|sorte| sorte.as_ref() == b"Widget")
                })
            })
    })
}

/// La copie préparée pour l'affichage (non compressée : elle ne sert qu'à dessiner).
fn preparer(octets: &[u8]) -> Option<Vec<u8>> {
    let mut document = crate::pdf::charger(octets, "affichage.pdf").ok()?;
    if !crate::formulaires::pour_l_affichage(&mut document) {
        return None;
    }
    let mut sortie = Vec::new();
    document.save_to(&mut sortie).ok()?;
    Some(sortie)
}

pub(crate) fn ouvrir(octets: Arc<Vec<u8>>, nom: &str) -> Result<Pdf, Erreur> {
    Pdf::new(octets).map_err(|erreur| match erreur {
        // Un PDF qui n'a que des restrictions (impression, copie…) s'ouvre : hayro essaie le mot de
        // passe vide. Celui-ci demande son mot de passe d'ouverture.
        LoadPdfError::Decryption(DecryptionError::PasswordProtected) => {
            Erreur::mot_de_passe(format!("« {nom} » est protégé par un mot de passe."))
        }
        _ => Erreur::simple(format!("« {nom} » n'a pas pu être lu.")),
    })
}

/// Taille de chaque page.
pub fn formats(octets: Arc<Vec<u8>>, nom: &str) -> Result<Vec<FormatPage>, Erreur> {
    let pdf = ouvrir(octets, nom)?;
    Ok(pdf
        .pages()
        .iter()
        .map(|page| {
            let (largeur, hauteur) = page.render_dimensions();
            FormatPage { largeur, hauteur }
        })
        .collect())
}

/// Format des images produites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FormatImage {
    Png,
    Jpeg,
}

/// La page `page` (à partir de 0) rendue sur fond blanc : largeur, hauteur et pixels RVB.
/// `echelle` donne le nombre de pixels par point d'après la taille de la page (en points) ; elle
/// est réduite au besoin (côté de 8 000 pixels au plus).
fn rendre(
    octets: Arc<Vec<u8>>,
    nom: &str,
    page: usize,
    echelle: impl Fn(f32, f32) -> f32,
) -> Result<(u32, u32, Vec<u8>), Erreur> {
    let pdf = ouvrir(pour_l_affichage(octets), nom)?;
    let page = pdf
        .pages()
        .get(page)
        .ok_or_else(|| Erreur::simple(format!("La page {} n'existe pas.", page + 1)))?;
    let (largeur, hauteur) = page.render_dimensions();
    let echelle =
        echelle(largeur.max(1.0), hauteur.max(1.0)).min(COTE_MAX / largeur.max(hauteur).max(1.0));
    let reglages = PixmapSettings {
        x_scale: echelle,
        y_scale: echelle,
        bg_color: WHITE,
    };
    let pixmap = render(
        page,
        &RenderCache::new(),
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &reglages,
    );
    // Fond blanc opaque : les pixels prémultipliés valent les pixels ordinaires.
    let rvb: Vec<u8> = pixmap
        .data_as_u8_slice()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[rouge, vert, bleu, _]| [rouge, vert, bleu])
        .collect();
    Ok((u32::from(pixmap.width()), u32::from(pixmap.height()), rvb))
}

fn encoder(
    (largeur, hauteur, rvb): (u32, u32, Vec<u8>),
    format: FormatImage,
    qualite: u8,
) -> Result<Vec<u8>, Erreur> {
    let mut sortie = Vec::new();
    let resultat = match format {
        FormatImage::Jpeg => JpegEncoder::new_with_quality(&mut sortie, qualite).encode(
            &rvb,
            largeur,
            hauteur,
            ExtendedColorType::Rgb8,
        ),
        FormatImage::Png => PngEncoder::new(&mut sortie).write_image(
            &rvb,
            largeur,
            hauteur,
            ExtendedColorType::Rgb8,
        ),
    };
    resultat.map_err(|erreur| {
        Erreur::simple(format!(
            "L'image de la page n'a pas pu être créée : {erreur}"
        ))
    })?;
    Ok(sortie)
}

/// La page `page` (à partir de 0) en JPEG, large de `largeur_px` pixels, sur fond blanc.
pub fn image_page(
    octets: Arc<Vec<u8>>,
    nom: &str,
    page: usize,
    largeur_px: u32,
    qualite: u8,
) -> Result<Vec<u8>, Erreur> {
    let rendu = rendre(octets, nom, page, |largeur, _| largeur_px as f32 / largeur)?;
    encoder(rendu, FormatImage::Jpeg, qualite)
}

/// La page `page` (à partir de 0) rendue à `dpi` points par pouce : largeur, hauteur, pixels RVB.
pub(crate) fn pixels_page(
    octets: Arc<Vec<u8>>,
    nom: &str,
    page: usize,
    dpi: u32,
) -> Result<(u32, u32, Vec<u8>), Erreur> {
    rendre(octets, nom, page, |_, _| dpi.clamp(36, 600) as f32 / 72.0)
}

/// La page `page` (à partir de 0) en image à `dpi` points par pouce (72 : taille réelle à l'écran).
pub fn exporter_page(
    octets: Arc<Vec<u8>>,
    nom: &str,
    page: usize,
    dpi: u32,
    format: FormatImage,
) -> Result<Vec<u8>, Erreur> {
    let rendu = rendre(octets, nom, page, |_, _| dpi.clamp(36, 600) as f32 / 72.0)?;
    encoder(rendu, format, 90)
}

/// La page `page` (à partir de 0) rendue à `dpi` points par pouce sur fond transparent, en PNG
/// avec transparence ; `None` si rien n'y est dessiné.
pub(crate) fn rendre_sur_transparent(
    octets: Arc<Vec<u8>>,
    nom: &str,
    page: usize,
    dpi: u32,
) -> Result<Option<Vec<u8>>, Erreur> {
    let pdf = ouvrir(pour_l_affichage(octets), nom)?;
    let page = pdf
        .pages()
        .get(page)
        .ok_or_else(|| Erreur::simple(format!("La page {} n'existe pas.", page + 1)))?;
    let (largeur, hauteur) = page.render_dimensions();
    let echelle = (dpi.clamp(36, 600) as f32 / 72.0).min(COTE_MAX / largeur.max(hauteur).max(1.0));
    let reglages = PixmapSettings {
        x_scale: echelle,
        y_scale: echelle,
        bg_color: TRANSPARENT,
    };
    let pixmap = render(
        page,
        &RenderCache::new(),
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &reglages,
    );
    // Pixels prémultipliés : chaque couleur est ramenée à ce qu'elle est sans transparence.
    let mut dessine = false;
    let rvba: Vec<u8> = pixmap
        .data_as_u8_slice()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[rouge, vert, bleu, alpha]| {
            if alpha == 0 {
                return [0, 0, 0, 0];
            }
            dessine = true;
            let sans = |c: u8| {
                ((u16::from(c) * 255 + u16::from(alpha) / 2) / u16::from(alpha)).min(255) as u8
            };
            [sans(rouge), sans(vert), sans(bleu), alpha]
        })
        .collect();
    if !dessine {
        return Ok(None);
    }
    let mut sortie = Vec::new();
    PngEncoder::new(&mut sortie)
        .write_image(
            &rvba,
            u32::from(pixmap.width()),
            u32::from(pixmap.height()),
            ExtendedColorType::Rgba8,
        )
        .map_err(|erreur| {
            Erreur::simple(format!(
                "L'image de la page n'a pas pu être créée : {erreur}"
            ))
        })?;
    Ok(Some(sortie))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::pdf_herite;

    #[test]
    fn donne_la_taille_de_chaque_page() {
        let formats = formats(Arc::new(pdf_herite(2, "T")), "t.pdf").unwrap();
        assert_eq!(formats.len(), 2);
        assert_eq!(
            formats[0],
            FormatPage {
                largeur: 595.0,
                hauteur: 842.0
            }
        );
    }

    #[test]
    fn rend_une_page_en_jpeg_a_la_largeur_demandee() {
        let jpeg = image_page(Arc::new(pdf_herite(2, "Bonjour")), "t.pdf", 1, 200, 85).unwrap();
        assert_eq!(&jpeg[..2], [0xFF, 0xD8]);
        let image = image::load_from_memory(&jpeg).unwrap().to_luma8();
        assert_eq!((image.width(), image.height()), (200, 283));
        // Le texte (police standard Helvetica, incluse) est bien dessiné : des pixels sombres en haut.
        let sombres = image.pixels().filter(|pixel| pixel.0[0] < 100).count();
        assert!(sombres > 20, "{sombres} pixels sombres");
        let erreur = image_page(Arc::new(pdf_herite(2, "T")), "t.pdf", 2, 200, 85).unwrap_err();
        assert!(erreur.message.contains("page 3"));
    }

    #[test]
    fn exporte_une_page_en_png_a_la_resolution_demandee() {
        let png = exporter_page(
            Arc::new(pdf_herite(1, "T")),
            "t.pdf",
            0,
            144,
            FormatImage::Png,
        )
        .unwrap();
        let image = image::load_from_memory(&png).unwrap();
        // A4 (595 × 842 points) à 144 points par pouce : deux pixels par point.
        assert_eq!((image.width(), image.height()), (1190, 1684));
    }
}
