//! Contenu des pages pour la conversion en document Word : chaque caractère avec sa position, sa
//! taille, sa police et sa couleur (texte invisible des scans reconnus compris), et chaque image
//! avec sa place sur la page, lus par hayro. Un JPEG est repris tel quel ; les autres images sont
//! réencodées en PNG.

use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Deref;
use std::sync::Arc;

use hayro::hayro_interpret::font::Glyph;
use hayro::hayro_interpret::hayro_cmap::BfString;
use hayro::hayro_interpret::{
    BlendMode, CacheKey, ClipPath, Context, Device, GlyphDrawMode, Image, ImageData,
    InterpreterCache, InterpreterSettings, LumaData, Paint, PathDrawMode, RasterImage, SoftMask,
    TransformExt, interpret_page,
};
use hayro::hayro_syntax::object::Name;
use hayro::vello_cpu::kurbo::{Affine, BezPath, Point, Rect};
use lopdf::Object;

use crate::docx::Format;
use crate::erreur::Erreur;
use crate::pdf::charger;
use crate::rendu::ouvrir;

/// Police d'un caractère, comme un traitement de texte la nomme.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Police {
    pub famille: String,
    pub gras: bool,
    pub italique: bool,
}

/// Un caractère, dans un repère tourné pour que son sens d'écriture aille vers la droite.
#[derive(Debug, Clone)]
pub struct Caractere {
    pub texte: String,
    /// Sens d'écriture sur la page affichée, en quarts de tour.
    pub sens: u8,
    pub debut: f64,
    pub fin: f64,
    /// Ligne de base (vers le bas).
    pub ligne: f64,
    pub taille: f64,
    pub police: Arc<Police>,
    pub couleur: [u8; 3],
    /// Texte invisible : celui qu'une reconnaissance de caractères a posé sur un scan.
    pub invisible: bool,
}

/// Une image et sa place sur la page affichée (points, depuis le coin haut gauche).
#[derive(Debug, Clone)]
pub struct ImagePlacee {
    pub gauche: f64,
    pub haut: f64,
    pub largeur: f64,
    pub hauteur: f64,
    pub octets: Arc<Vec<u8>>,
    pub format: Format,
    /// Même image ailleurs dans le document : même clé.
    pub cle: u128,
}

/// Une page : son format affiché (points), ses caractères et ses images.
#[derive(Debug, Clone, Default)]
pub struct Page {
    pub largeur: f64,
    pub hauteur: f64,
    pub caracteres: Vec<Caractere>,
    pub images: Vec<ImagePlacee>,
    /// Ce qui ne se reprend pas en texte ni en image : tracés (dessins, cadres), caractères sans
    /// équivalent Unicode. Une page qui n'a que cela est reprise en image.
    pub autres_traces: usize,
}

/// Images déjà encodées, d'après leur clé : une même image (logo) revient souvent sur chaque page.
type Encodees = HashMap<u128, Option<(Arc<Vec<u8>>, Format)>>;

/// Résolution des images réencodées : assez pour l'impression.
const DPI_IMAGES: f64 = 300.0;

/// Famille, gras et italique d'après le nom PostScript d'une police : « ABCDEF+Arial-BoldMT »,
/// « TimesNewRomanPS-ItalicMT », « Arial,Bold », « Helvetica-Oblique »…
pub fn police_nommee(nom: &str) -> Police {
    let nom = match nom.split_once('+') {
        Some((prefixe, reste)) if prefixe.len() == 6 => reste,
        _ => nom,
    };
    let (base, style) = nom
        .split_once(['-', ','])
        .map_or((nom, ""), |(base, style)| (base, style));
    let style_minuscules = style.to_lowercase();
    let gras = ["bold", "black", "heavy", "semibold", "demi"]
        .iter()
        .any(|mot| style_minuscules.contains(mot));
    let italique = style_minuscules.contains("italic")
        || style_minuscules.contains("oblique")
        || style_minuscules.ends_with("it");
    let mut base = base
        .trim_end_matches("PSMT")
        .trim_end_matches("MT")
        .trim_end_matches("PS")
        .to_string();
    // Les polices du PDF de base ont leurs équivalents sous Windows.
    base = match base.as_str() {
        "Helvetica" => "Arial".into(),
        "Times" => "Times New Roman".into(),
        "Courier" => "Courier New".into(),
        _ => base,
    };
    // « TimesNewRoman » : une espace avant chaque majuscule qui suit une minuscule.
    let mut famille = String::new();
    let mut precedent: Option<char> = None;
    for c in base.chars() {
        if c.is_uppercase() && precedent.is_some_and(char::is_lowercase) {
            famille.push(' ');
        }
        famille.push(c);
        precedent = Some(c);
    }
    let famille = famille.replace("Deja Vu", "DejaVu");
    Police {
        famille: if famille.is_empty() {
            "Arial".into()
        } else {
            famille
        },
        gras,
        italique,
    }
}

/// Appareil qui, au lieu de dessiner, relève les caractères et les images.
struct Releve<'e> {
    page: Page,
    polices: HashMap<u128, Arc<Police>>,
    /// Police des caractères dont hayro ne connaît pas le nom (polices Type 1), d'après la page.
    par_defaut: Arc<Police>,
    encodees: &'e mut Encodees,
}

fn encoder_png(donnees: ImageData, alpha: Option<LumaData>) -> Option<Vec<u8>> {
    let (largeur, hauteur) = (donnees.width(), donnees.height());
    let alpha = alpha.filter(|alpha| alpha.width == largeur && alpha.height == hauteur);
    let image = match (donnees, alpha) {
        (ImageData::Rgb(rgb), None) => {
            image::DynamicImage::ImageRgb8(image::RgbImage::from_raw(largeur, hauteur, rgb.data)?)
        }
        (ImageData::Rgb(rgb), Some(alpha)) => {
            let pixels = rgb
                .data
                .as_chunks::<3>()
                .0
                .iter()
                .zip(alpha.data)
                .flat_map(|(&[rouge, vert, bleu], a)| [rouge, vert, bleu, a])
                .collect();
            image::DynamicImage::ImageRgba8(image::RgbaImage::from_raw(largeur, hauteur, pixels)?)
        }
        (ImageData::Luma(gris), None) => image::DynamicImage::ImageLuma8(
            image::GrayImage::from_raw(largeur, hauteur, gris.data)?,
        ),
        (ImageData::Luma(gris), Some(alpha)) => {
            let pixels = gris
                .data
                .into_iter()
                .zip(alpha.data)
                .flat_map(|(g, a)| [g, a])
                .collect();
            image::DynamicImage::ImageLumaA8(image::GrayAlphaImage::from_raw(
                largeur, hauteur, pixels,
            )?)
        }
    };
    let mut octets = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut octets), image::ImageFormat::Png)
        .ok()?;
    Some(octets)
}

/// Les octets d'une image : le JPEG d'origine s'il se lit tel quel (couleurs d'écran, sans
/// transparence), sinon un PNG, à `cible` pixels au plus si l'image est plus fine.
fn encoder(image: &RasterImage<'_>, cible: (u32, u32)) -> Option<(Arc<Vec<u8>>, Format)> {
    let flux = image.stream();
    let dictionnaire = flux.dict();
    let couleurs_ecran = dictionnaire
        .get::<Name<'_>>(b"ColorSpace")
        .is_some_and(|espace| matches!(espace.deref(), b"DeviceRGB" | b"DeviceGray"));
    let opaque = !dictionnaire.contains_key(b"SMask") && !dictionnaire.contains_key(b"Mask");
    // Un seul filtre, celui du JPEG : les octets bruts sont un fichier JPEG.
    let jpeg = dictionnaire
        .get::<Name<'_>>(b"Filter")
        .is_some_and(|filtre| matches!(filtre.deref(), b"DCTDecode" | b"DCT"));
    if couleurs_ecran && opaque && jpeg && !dictionnaire.contains_key(b"Decode") {
        return Some((Arc::new(flux.raw_data().into_owned()), Format::Jpeg));
    }
    let mut png = None;
    image.with_rgba(
        |donnees, alpha| png = encoder_png(donnees, alpha),
        Some(cible),
    );
    png.map(|octets| (Arc::new(octets), Format::Png))
}

impl<'a> Device<'a> for Releve<'_> {
    fn set_soft_mask(&mut self, _: Option<SoftMask<'a>>) {}
    fn set_blend_mode(&mut self, _: BlendMode) {}
    fn draw_path(&mut self, _: &BezPath, _: Affine, _: &Paint<'a>, _: &PathDrawMode) {
        self.page.autres_traces += 1;
    }
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn pop_clip_path(&mut self) {}
    fn pop_transparency_group(&mut self) {}

    fn draw_image(&mut self, image: Image<'a, '_>, transform: Affine) {
        // Les pochoirs (masques d'une couleur) ne sont pas des illustrations.
        let Image::Raster(image) = image else {
            return;
        };
        let (largeur, hauteur) = (f64::from(image.width()), f64::from(image.height()));
        let coins = [
            Point::new(0.0, 0.0),
            Point::new(largeur, 0.0),
            Point::new(0.0, hauteur),
            Point::new(largeur, hauteur),
        ]
        .map(|coin| transform * coin);
        let gauche = coins.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let droite = coins.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let haut = coins.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let bas = coins.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        let visible =
            droite > 0.0 && bas > 0.0 && gauche < self.page.largeur && haut < self.page.hauteur;
        if !visible || droite - gauche < 2.0 || bas - haut < 2.0 || !gauche.is_finite() {
            return;
        }
        let pixels = |points: f64, source: f64| {
            (points / 72.0 * DPI_IMAGES)
                .ceil()
                .clamp(1.0, source.max(1.0)) as u32
        };
        let cible = (
            pixels(droite - gauche, largeur),
            pixels(bas - haut, hauteur),
        );
        let cle = image.cache_key();
        let encodee = self
            .encodees
            .entry(cle)
            .or_insert_with(|| encoder(&image, cible))
            .clone();
        if let Some((octets, format)) = encodee {
            self.page.images.push(ImagePlacee {
                gauche,
                haut,
                largeur: droite - gauche,
                hauteur: bas - haut,
                octets,
                format,
                cle,
            });
        }
    }

    fn draw_glyph(
        &mut self,
        glyphe: &Glyph<'a>,
        transform: Affine,
        glyph_transform: Affine,
        peinture: &Paint<'a>,
        mode: &GlyphDrawMode,
    ) {
        let texte = match glyphe.as_unicode() {
            Some(BfString::Char(caractere)) => caractere.to_string(),
            Some(BfString::String(texte)) => texte,
            None => {
                self.page.autres_traces += 1;
                return;
            }
        };
        // Le glyphe compte 1 000 unités par cadratin ; avance inconnue : un demi-cadratin.
        let matrice = transform * glyph_transform;
        let avance = match glyphe {
            Glyph::Outline(contour) => contour.advance_width().map(f64::from),
            Glyph::Type3(_) => None,
        }
        .filter(|avance| *avance > 0.0)
        .unwrap_or(500.0);
        let origine = matrice * Point::ORIGIN;
        let arrivee = matrice * Point::new(avance, 0.0);
        let taille = (matrice * Point::new(0.0, 1000.0) - origine).hypot();
        if !taille.is_finite() || taille < 0.5 {
            return;
        }
        let angle = (arrivee.y - origine.y).atan2(arrivee.x - origine.x);
        let sens = (angle / std::f64::consts::FRAC_PI_2)
            .round()
            .rem_euclid(4.0) as u8;
        let tourner = |p: Point| match sens {
            0 => (p.x, p.y),
            1 => (p.y, -p.x),
            2 => (-p.x, -p.y),
            _ => (-p.y, p.x),
        };
        let (debut, ligne) = tourner(origine);
        let (fin, _) = tourner(arrivee);
        let par_defaut = self.par_defaut.clone();
        let police = match glyphe {
            Glyph::Outline(contour) => self
                .polices
                .entry(contour.font_cache_key())
                .or_insert_with(|| match contour.font_data() {
                    Some(donnees) => {
                        let mut police = donnees
                            .postscript_name
                            .as_deref()
                            .map_or_else(|| (*par_defaut).clone(), police_nommee);
                        police.gras |= donnees.weight.is_some_and(|poids| poids >= 600);
                        police.italique |= donnees.is_italic;
                        Arc::new(police)
                    }
                    None => par_defaut,
                })
                .clone(),
            Glyph::Type3(_) => par_defaut,
        };
        let couleur = match peinture {
            Paint::Color(couleur) => {
                let [rouge, vert, bleu, _] = couleur.to_rgba().to_rgba8();
                [rouge, vert, bleu]
            }
            Paint::Pattern(_) => [0, 0, 0],
        };
        self.page.caracteres.push(Caractere {
            texte,
            sens,
            debut,
            fin: fin.max(debut),
            ligne,
            taille,
            police,
            couleur,
            invisible: matches!(mode, GlyphDrawMode::Invisible),
        });
    }
}

/// Polices de chaque page dont hayro ne donne pas le nom (Type 1, TrueType non intégrée) : si la
/// page n'en a qu'une, c'est elle ; sinon, leur famille commune, ou Arial.
fn polices_par_defaut(octets: &[u8], nom: &str, pages: usize) -> Vec<Arc<Police>> {
    let arial = Arc::new(police_nommee("Arial"));
    let Ok(document) = charger(octets, nom) else {
        return vec![arial; pages];
    };
    document
        .get_pages()
        .values()
        .map(|&page| {
            let mut noms: Vec<String> = document
                .get_page_fonts(page)
                .map(|polices| {
                    polices
                        .values()
                        .filter(|police| {
                            matches!(
                                police.get(b"Subtype").and_then(Object::as_name),
                                Ok(b"Type1" | b"MMType1" | b"TrueType")
                            )
                        })
                        .filter_map(|police| police.get(b"BaseFont").and_then(Object::as_name).ok())
                        .map(|nom| String::from_utf8_lossy(nom).into_owned())
                        .collect()
                })
                .unwrap_or_default();
            noms.sort();
            noms.dedup();
            let polices: Vec<Police> = noms.iter().map(|nom| police_nommee(nom)).collect();
            match polices.as_slice() {
                [seule] => Arc::new(seule.clone()),
                [premiere, ..] if polices.iter().all(|p| p.famille == premiere.famille) => {
                    Arc::new(Police {
                        famille: premiere.famille.clone(),
                        gras: false,
                        italique: false,
                    })
                }
                _ => arial.clone(),
            }
        })
        .chain(std::iter::repeat(arial.clone()))
        .take(pages)
        .collect()
}

/// Le contenu de chaque page.
pub fn lire(octets: Arc<Vec<u8>>, nom: &str) -> Result<Vec<Page>, Erreur> {
    let pdf = ouvrir(octets.clone(), nom)?;
    let par_defaut = polices_par_defaut(&octets, nom, pdf.pages().len());
    let cache = InterpreterCache::new();
    let reglages = InterpreterSettings::default();
    let mut encodees = Encodees::new();
    let mut pages = Vec::new();
    for (page, par_defaut) in pdf.pages().iter().zip(par_defaut) {
        let (largeur, hauteur) = page.render_dimensions();
        let (largeur, hauteur) = (f64::from(largeur), f64::from(hauteur));
        let mut contexte = Context::new(
            page.initial_transform(true).to_kurbo(),
            Rect::new(0.0, 0.0, largeur, hauteur),
            &cache,
            page.xref(),
            reglages.clone(),
        );
        let mut releve = Releve {
            page: Page {
                largeur,
                hauteur,
                ..Page::default()
            },
            polices: HashMap::new(),
            par_defaut,
            encodees: &mut encodees,
        };
        interpret_page(page, &mut contexte, &mut releve);
        pages.push(releve.page);
    }
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomme_les_polices_comme_un_traitement_de_texte() {
        let police = |famille: &str, gras: bool, italique: bool| Police {
            famille: famille.into(),
            gras,
            italique,
        };
        assert_eq!(
            police_nommee("AAAAAA+Arial-BoldMT"),
            police("Arial", true, false)
        );
        assert_eq!(
            police_nommee("BAAAAA+ArialMT"),
            police("Arial", false, false)
        );
        assert_eq!(
            police_nommee("TimesNewRomanPS-BoldItalicMT"),
            police("Times New Roman", true, true)
        );
        assert_eq!(police_nommee("Arial,Italic"), police("Arial", false, true));
        assert_eq!(
            police_nommee("Helvetica-Oblique"),
            police("Arial", false, true)
        );
        assert_eq!(
            police_nommee("Times-Roman"),
            police("Times New Roman", false, false)
        );
        assert_eq!(
            police_nommee("Courier"),
            police("Courier New", false, false)
        );
        assert_eq!(
            police_nommee("ABCDEF+Calibri"),
            police("Calibri", false, false)
        );
        assert_eq!(
            police_nommee("SegoeUI-Semibold"),
            police("Segoe UI", true, false)
        );
        assert_eq!(
            police_nommee("MyriadPro-It"),
            police("Myriad Pro", false, true)
        );
        assert_eq!(
            police_nommee("DejaVuSans-Bold"),
            police("DejaVu Sans", true, false)
        );
    }
}
