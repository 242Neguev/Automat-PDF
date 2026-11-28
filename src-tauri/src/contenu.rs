//! Contenu des pages pour la conversion en document Word : chaque caractère avec sa position, sa
//! taille, sa police et sa couleur (texte invisible des scans reconnus compris), et chaque image
//! avec sa place sur la page, lus par hayro. Un JPEG est repris tel quel ; les autres images sont
//! réencodées en PNG.

use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Deref;
use std::sync::Arc;

use hayro::hayro_interpret::font::{Glyph, GlyphRun};
use hayro::hayro_interpret::hayro_cmap::BfString;
use hayro::hayro_interpret::{
    BlendMode, CacheKey, ClipPath, Context, Device, DrawMode, DrawProps, Image, ImageData,
    ImageDrawProps, InterpreterCache, InterpreterSettings, LumaData, Paint, RasterImage, SoftMask,
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
    /// Ses octets (vides quand seules les places sont relevées : `Images::Places`).
    pub octets: Arc<Vec<u8>>,
    pub format: Format,
    /// Même image ailleurs dans le document : même clé.
    pub cle: u128,
    /// Sa définition d'origine, en pixels (largeur, hauteur).
    pub pixels: (u32, u32),
}

/// Ce que le relevé fait des images.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Images {
    /// PDF en Word : les images à la taille affichée (300 ppp au plus), et le texte.
    Affichees,
    /// Leur place seulement, sans les lire (le clic droit du lecteur sur une photo).
    Places,
    /// Dans leur définition d'origine (Enregistrer l'image, Extraire les images).
    Origine,
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
    images: Images,
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

/// L'image est un JPEG qui se reprend tel quel : un seul filtre, celui du JPEG (les octets bruts
/// sont un fichier JPEG), couleurs d'écran, sans transparence ni inversion.
fn jpeg_tel_quel(image: &RasterImage<'_>) -> bool {
    let dictionnaire = image.stream().dict();
    let couleurs_ecran = dictionnaire
        .get::<Name<'_>>(b"ColorSpace")
        .is_some_and(|espace| matches!(espace.deref(), b"DeviceRGB" | b"DeviceGray"));
    let opaque = !dictionnaire.contains_key(b"SMask") && !dictionnaire.contains_key(b"Mask");
    let jpeg = dictionnaire
        .get::<Name<'_>>(b"Filter")
        .is_some_and(|filtre| matches!(filtre.deref(), b"DCTDecode" | b"DCT"));
    couleurs_ecran && opaque && jpeg && !dictionnaire.contains_key(b"Decode")
}

/// Les octets d'une image : le JPEG d'origine s'il se lit tel quel (couleurs d'écran, sans
/// transparence), sinon un PNG, à `cible` pixels au plus si l'image est plus fine.
fn encoder(image: &RasterImage<'_>, cible: (u32, u32)) -> Option<(Arc<Vec<u8>>, Format)> {
    if jpeg_tel_quel(image) {
        return Some((
            Arc::new(image.stream().raw_data().into_owned()),
            Format::Jpeg,
        ));
    }
    let mut png = None;
    image.with_rgba(
        |donnees, alpha| png = encoder_png(donnees, alpha),
        Some(cible),
    );
    png.map(|octets| (Arc::new(octets), Format::Png))
}

impl<'a> Device<'a> for Releve<'_> {
    fn draw_path(&mut self, _: &BezPath, _: DrawProps<'a>, _: &DrawMode) {
        self.page.autres_traces += 1;
    }
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn pop_clip(&mut self) {}
    fn pop_transparency_group(&mut self) {}

    fn draw_glyph_run(&mut self, suite: &GlyphRun<'_, 'a>, props: DrawProps<'a>, mode: &DrawMode) {
        // Les images seules : le texte n'est pas relevé.
        if self.images != Images::Affichees {
            return;
        }
        let invisible = matches!(mode, DrawMode::Invisible);
        for glyphe in suite.glyphs() {
            self.relever(
                glyphe,
                props.transform * glyphe.transform(),
                &props.paint,
                invisible,
            );
        }
    }

    fn draw_image(&mut self, image: Image<'a, '_>, props: ImageDrawProps<'a>) {
        let transform = props.transform;
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
        let cle = image.cache_key();
        let (octets, format) = match self.images {
            Images::Places => (
                Arc::new(Vec::new()),
                if jpeg_tel_quel(&image) {
                    Format::Jpeg
                } else {
                    Format::Png
                },
            ),
            Images::Affichees | Images::Origine => {
                let cible = if self.images == Images::Origine {
                    (image.width(), image.height())
                } else {
                    (
                        pixels(droite - gauche, largeur),
                        pixels(bas - haut, hauteur),
                    )
                };
                let encodee = self
                    .encodees
                    .entry(cle)
                    .or_insert_with(|| encoder(&image, cible))
                    .clone();
                let Some(encodee) = encodee else {
                    return;
                };
                encodee
            }
        };
        self.page.images.push(ImagePlacee {
            gauche,
            haut,
            largeur: droite - gauche,
            hauteur: bas - haut,
            octets,
            format,
            cle,
            pixels: (image.width(), image.height()),
        });
    }
}

impl Releve<'_> {
    /// Un caractère ; `matrice` place son glyphe (1 000 unités par cadratin) sur la page affichée.
    fn relever(
        &mut self,
        glyphe: &Glyph<'_>,
        matrice: Affine,
        peinture: &Paint<'_>,
        invisible: bool,
    ) {
        let texte = match glyphe.as_unicode() {
            Some(BfString::Char(caractere)) => caractere.to_string(),
            Some(BfString::String(texte)) => texte,
            None => {
                self.page.autres_traces += 1;
                return;
            }
        };
        // Avance inconnue : un demi-cadratin.
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
                .entry(contour.identifier().cache_key())
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
            invisible,
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
    relever(octets, nom, None, Images::Affichees)
}

/// Les pages choisies (`None` : toutes), dans l'ordre donné : leurs images, comme `images` le dit,
/// et leurs caractères pour PDF en Word.
pub fn relever(
    octets: Arc<Vec<u8>>,
    nom: &str,
    choisies: Option<&[usize]>,
    images: Images,
) -> Result<Vec<Page>, Erreur> {
    let mut pages = Vec::new();
    relever_une_a_une(octets, nom, choisies, images, |_, page| {
        pages.push(page);
        Ok(())
    })?;
    Ok(pages)
}

/// Comme `relever`, page après page : `traiter(numéro, page)` reçoit chacune dès qu'elle est
/// relevée. En `Images::Origine`, une image déjà donnée ne revient pas sur les pages suivantes, et
/// ses octets ne restent pas en mémoire : tout un catalogue de photos tient en mémoire.
pub fn relever_une_a_une(
    octets: Arc<Vec<u8>>,
    nom: &str,
    choisies: Option<&[usize]>,
    images: Images,
    mut traiter: impl FnMut(usize, Page) -> Result<(), Erreur>,
) -> Result<(), Erreur> {
    let pdf = ouvrir(octets.clone(), nom)?;
    let toutes = pdf.pages();
    let numeros: Vec<usize> = match choisies {
        Some(choisies) => choisies.to_vec(),
        None => (0..toutes.len()).collect(),
    };
    if let Some(&absente) = numeros.iter().find(|&&page| page >= toutes.len()) {
        return Err(Erreur::simple(format!(
            "La page {} n'existe pas : le PDF en compte {}.",
            absente + 1,
            toutes.len()
        )));
    }
    // Les noms des polices ne servent qu'au texte.
    let par_defaut = if images == Images::Affichees {
        polices_par_defaut(&octets, nom, toutes.len())
    } else {
        let arial = Arc::new(police_nommee("Arial"));
        vec![arial; toutes.len()]
    };
    let cache = InterpreterCache::new();
    let reglages = InterpreterSettings::default();
    let mut encodees = Encodees::new();
    for numero in numeros {
        let (page, par_defaut) = (&toutes[numero], par_defaut[numero].clone());
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
            images,
        };
        interpret_page(page, &mut contexte, &mut releve);
        let page = releve.page;
        if images == Images::Origine {
            // Les images données sont oubliées ; leur clé reste, pour ne pas les redonner.
            encodees.values_mut().for_each(|encodee| *encodee = None);
        }
        traiter(numero, page)?;
    }
    Ok(())
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
