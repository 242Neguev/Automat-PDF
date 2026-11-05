//! Surimpressions sur les pages : numéros de page, tampon ou filigrane, image (signature ou cachet).
//! Elles s'ajoutent par-dessus le contenu d'origine, à l'endroit tel que la page s'affiche (même
//! tournée) ; le reste du document (liens, signets, formulaires) n'est pas touché.

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};
use serde::Deserialize;

use crate::erreur::Erreur;
use crate::geometrie::{Cadre, Matrice, resoudre};
use crate::images::ImagePdf;
use crate::police;

const POINTS_PAR_MM: f64 = 72.0 / 25.4;
/// Hauteur des capitales d'Helvetica, en fraction de la taille.
const CAPITALES: f64 = 0.72;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    HautGauche,
    HautCentre,
    HautDroite,
    BasGauche,
    BasCentre,
    BasDroite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FormatNumero {
    /// « 3 »
    N,
    /// « Page 3 »
    PageN,
    /// « Page 3 sur 12 »
    PageNSurTotal,
    /// « 3 / 12 »
    NSurTotal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsNumeros {
    pub format: FormatNumero,
    pub position: Position,
    /// Taille du texte, en points.
    pub taille: f64,
    /// Numéro de la première page numérotée.
    pub premier: u32,
    /// Distance au bord de la page, en mm.
    pub marge_mm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StyleTampon {
    /// Grand texte en diagonale, transparent, au centre de la page.
    Filigrane,
    /// Texte encadré, légèrement incliné, en haut à droite.
    Tampon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Couleur {
    Gris,
    Rouge,
    Bleu,
    Vert,
}

impl Couleur {
    fn rvb(self) -> String {
        match self {
            Couleur::Gris => "0.45 0.45 0.45",
            Couleur::Rouge => "0.80 0.10 0.10",
            Couleur::Bleu => "0.12 0.30 0.70",
            Couleur::Vert => "0.10 0.50 0.25",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsTampon {
    pub texte: String,
    pub style: StyleTampon,
    pub couleur: Couleur,
    /// Opacité, de 0,05 (presque invisible) à 1 (opaque).
    pub opacite: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsMarques {
    pub numeros: Option<OptionsNumeros>,
    pub tampon: Option<OptionsTampon>,
    /// Pages concernées (à partir de 0) ; vide : toutes.
    #[serde(default)]
    pub pages: Vec<usize>,
}

/// Emplacement d'une image sur la page affichée, en fractions de sa largeur et de sa hauteur,
/// depuis le coin en haut à gauche.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub largeur: f64,
    pub hauteur: f64,
}

/// Une ressource à ajouter à une page : catégorie (Font, XObject…), début de son nom, objet.
type Ressource<'a> = (&'a str, &'a str, Object);

/// Ajoute un dessin par-dessus une page. Les ressources reçoivent des noms libres sur cette page ;
/// `dessin` les reçoit, dans l'ordre, et écrit dans le repère de la page affichée (origine en bas
/// à gauche, en points). Le contenu d'origine, encadré par « q … Q », n'est pas modifié.
fn surimprimer(
    document: &mut Document,
    page: ObjectId,
    ressources: &[Ressource],
    dessin: impl FnOnce(&[String]) -> String,
) -> Result<(), Erreur> {
    let dictionnaire = document
        .get_dictionary(page)
        .map_err(|_| Erreur::simple("Une page n'a pas pu être lue."))?
        .clone();
    let cadre = Cadre::de_page(document, &dictionnaire);
    let mut contenus: Vec<Object> = match dictionnaire.get(b"Contents") {
        Ok(Object::Reference(id)) => match document.get_object(*id) {
            Ok(Object::Array(elements)) => elements.clone(),
            _ => vec![Object::Reference(*id)],
        },
        Ok(Object::Array(elements)) => elements.clone(),
        Ok(Object::Stream(flux)) => vec![Object::Reference(document.add_object(flux.clone()))],
        _ => Vec::new(),
    };
    // Ressources propres à la page : celles d'origine peuvent être partagées par d'autres pages.
    let mut propres = dictionnaire
        .get(b"Resources")
        .ok()
        .and_then(|r| resoudre(document, r).as_dict().ok())
        .cloned()
        .unwrap_or_default();
    let mut noms = Vec::new();
    for (categorie, base, objet) in ressources {
        let mut sous = propres
            .get(categorie.as_bytes())
            .ok()
            .and_then(|d| resoudre(document, d).as_dict().ok())
            .cloned()
            .unwrap_or_default();
        let nom = (1..)
            .map(|rang| format!("{base}{rang}"))
            .find(|nom| !sous.has(nom.as_bytes()))
            .unwrap_or_default();
        sous.set(nom.as_bytes(), objet.clone());
        propres.set(categorie.as_bytes(), sous);
        noms.push(nom);
    }
    let surimpression = format!(
        "\nQ\nq {}\n{}\nQ\n",
        cadre.depuis_vue().operateur(),
        dessin(&noms)
    );
    let avant = document.add_object(Stream::new(Dictionary::new(), b"q\n".to_vec()));
    let apres = document.add_object(Stream::new(Dictionary::new(), surimpression.into_bytes()));
    contenus.insert(0, Object::Reference(avant));
    contenus.push(Object::Reference(apres));
    let page = document
        .get_dictionary_mut(page)
        .map_err(|_| Erreur::simple("Une page n'a pas pu être modifiée."))?;
    page.set("Contents", contenus);
    page.set("Resources", propres);
    Ok(())
}

fn texte_numero(format: FormatNumero, numero: u32, total: u32) -> String {
    match format {
        FormatNumero::N => numero.to_string(),
        FormatNumero::PageN => format!("Page {numero}"),
        FormatNumero::PageNSurTotal => format!("Page {numero} sur {total}"),
        FormatNumero::NSurTotal => format!("{numero} / {total}"),
    }
}

fn dessin_numero(
    options: &OptionsNumeros,
    texte: &str,
    police: &str,
    (largeur, hauteur): (f64, f64),
) -> String {
    let taille = options.taille.clamp(5.0, 72.0);
    let marge = options.marge_mm.clamp(0.0, 50.0) * POINTS_PAR_MM;
    let longueur = police::largeur(texte, false, taille);
    let x = match options.position {
        Position::HautGauche | Position::BasGauche => marge,
        Position::HautCentre | Position::BasCentre => (largeur - longueur) / 2.0,
        Position::HautDroite | Position::BasDroite => largeur - marge - longueur,
    };
    let y = match options.position {
        Position::BasGauche | Position::BasCentre | Position::BasDroite => marge,
        _ => hauteur - marge - CAPITALES * taille,
    };
    format!(
        "BT /{police} {taille:.2} Tf 0.25 g 1 0 0 1 {x:.2} {y:.2} Tm {} Tj ET\n",
        police::chaine(texte)
    )
}

fn dessin_tampon(
    options: &OptionsTampon,
    police: &str,
    transparence: &str,
    (largeur, hauteur): (f64, f64),
) -> String {
    let couleur = options.couleur.rvb();
    let chaine = police::chaine(&options.texte);
    let unite = police::largeur(&options.texte, true, 1.0).max(0.1);
    match options.style {
        StyleTampon::Filigrane => {
            // En diagonale, de coin à coin, sur les trois quarts de la diagonale.
            let angle = hauteur.atan2(largeur);
            let taille = (0.75 * largeur.hypot(hauteur) / unite).min(180.0);
            let (sinus, cosinus) = angle.sin_cos();
            let (demi_l, demi_h) = (unite * taille / 2.0, CAPITALES * taille / 2.0);
            let x = largeur / 2.0 - demi_l * cosinus + demi_h * sinus;
            let y = hauteur / 2.0 - demi_l * sinus - demi_h * cosinus;
            format!(
                "q /{transparence} gs {couleur} rg BT /{police} {taille:.2} Tf {cosinus:.5} {sinus:.5} {:.5} {cosinus:.5} {x:.2} {y:.2} Tm {chaine} Tj ET Q\n",
                -sinus
            )
        }
        StyleTampon::Tampon => {
            // Encadré en haut à droite, légèrement incliné, réduit s'il est trop long.
            let (marge, interieur) = (36.0, 8.0);
            let taille = 24f64
                .min((largeur - 2.0 * marge - 2.0 * interieur) / unite)
                .max(6.0);
            let (cadre_l, cadre_h) = (
                unite * taille + 2.0 * interieur,
                CAPITALES * taille + 2.0 * interieur,
            );
            let (x, y) = (largeur - marge - cadre_l, hauteur - marge - cadre_h);
            let (cx, cy) = (x + cadre_l / 2.0, y + cadre_h / 2.0);
            let inclinaison = Matrice::translation(-cx, -cy)
                .puis(Matrice::rotation(8.0))
                .puis(Matrice::translation(cx, cy));
            format!(
                "q /{transparence} gs {} {couleur} RG {couleur} rg 2.5 w {x:.2} {y:.2} {cadre_l:.2} {cadre_h:.2} re S BT /{police} {taille:.2} Tf 1 0 0 1 {:.2} {:.2} Tm {chaine} Tj ET Q\n",
                inclinaison.operateur(),
                x + interieur,
                y + interieur
            )
        }
    }
}

/// Numéro (et total) de la page, s'il faut en écrire un.
pub type Numero = Option<(u32, u32)>;

/// Numéro et tampon sur une page.
pub fn marquer_page(
    document: &mut Document,
    page: ObjectId,
    options: &OptionsMarques,
    numero: Numero,
) -> Result<(), Erreur> {
    let dictionnaire = document
        .get_dictionary(page)
        .map_err(|_| Erreur::simple("Une page n'a pas pu être lue."))?
        .clone();
    let taille_vue = Cadre::de_page(document, &dictionnaire).taille_vue();
    let mut ressources: Vec<Ressource> = Vec::new();
    if let Some(tampon) = options
        .tampon
        .as_ref()
        .filter(|t| !t.texte.trim().is_empty())
    {
        let opacite = tampon.opacite.clamp(0.05, 1.0);
        ressources.push((
            "Font",
            "AutomatGras",
            Object::Dictionary(police::police(true)),
        ));
        ressources.push((
            "ExtGState",
            "AutomatTransparence",
            Object::Dictionary(dictionary! { "Type" => "ExtGState", "ca" => opacite as f32, "CA" => opacite as f32 }),
        ));
    }
    let numeros = options.numeros.as_ref().zip(numero);
    if numeros.is_some() {
        ressources.push((
            "Font",
            "AutomatNormale",
            Object::Dictionary(police::police(false)),
        ));
    }
    if ressources.is_empty() {
        return Ok(());
    }
    surimprimer(document, page, &ressources, |noms| {
        let mut dessin = String::new();
        let mut noms = noms.iter();
        if let Some(tampon) = options
            .tampon
            .as_ref()
            .filter(|t| !t.texte.trim().is_empty())
        {
            let (police, transparence) = (
                noms.next().cloned().unwrap_or_default(),
                noms.next().cloned().unwrap_or_default(),
            );
            dessin.push_str(&dessin_tampon(tampon, &police, &transparence, taille_vue));
        }
        if let Some((options_numeros, (n, total))) = numeros {
            let police = noms.next().cloned().unwrap_or_default();
            let texte = texte_numero(options_numeros.format, n, total);
            dessin.push_str(&dessin_numero(options_numeros, &texte, &police, taille_vue));
        }
        dessin
    })
}

/// Pages choisies (à partir de 0, dans l'ordre du document) : toutes si la liste est vide.
fn pages_choisies(choix: &[usize], nombre: usize) -> Vec<usize> {
    if choix.is_empty() {
        return (0..nombre).collect();
    }
    let mut pages: Vec<usize> = choix
        .iter()
        .copied()
        .filter(|&page| page < nombre)
        .collect();
    pages.sort_unstable();
    pages.dedup();
    pages
}

/// Numéro de chaque page choisie : le premier numéro, puis la suite ; le total est le dernier numéro.
pub fn numeros(options: &OptionsMarques, nombre: usize) -> Vec<(usize, Numero)> {
    let pages = pages_choisies(&options.pages, nombre);
    let premier = options.numeros.as_ref().map_or(1, |n| n.premier);
    let total = premier + pages.len().saturating_sub(1) as u32;
    pages
        .into_iter()
        .enumerate()
        .map(|(rang, page)| (page, Some((premier + rang as u32, total))))
        .collect()
}

/// Numéros et tampon sur les pages choisies du document.
pub fn marquer(document: &mut Document, options: &OptionsMarques) -> Result<(), Erreur> {
    let pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    for (index, numero) in numeros(options, pages.len()) {
        marquer_page(document, pages[index], options, numero)?;
    }
    Ok(())
}

/// Image (signature, cachet) posée sur les pages choisies, contenue dans l'emplacement donné.
pub fn apposer(
    document: &mut Document,
    image: ImagePdf,
    placement: Placement,
    pages: &[usize],
) -> Result<(), Erreur> {
    let (image_l, image_h) = (
        f64::from(image.largeur.max(1)),
        f64::from(image.hauteur.max(1)),
    );
    let mut flux = image.flux;
    if let Some(transparence) = image.transparence {
        flux.dict.set("SMask", document.add_object(transparence));
    }
    let image = document.add_object(flux);
    let toutes: Vec<ObjectId> = document.get_pages().into_values().collect();
    for index in pages_choisies(pages, toutes.len()) {
        let page = toutes[index];
        let dictionnaire = document
            .get_dictionary(page)
            .map_err(|_| Erreur::simple("Une page n'a pas pu être lue."))?
            .clone();
        let (largeur, hauteur) = Cadre::de_page(document, &dictionnaire).taille_vue();
        let (bx, bl, bh) = (
            placement.x * largeur,
            placement.largeur * largeur,
            placement.hauteur * hauteur,
        );
        let by = hauteur - (placement.y + placement.hauteur) * hauteur;
        let echelle = (bl / image_l).min(bh / image_h);
        let (dessin_l, dessin_h) = (image_l * echelle, image_h * echelle);
        let (x, y) = (bx + (bl - dessin_l) / 2.0, by + (bh - dessin_h) / 2.0);
        surimprimer(
            document,
            page,
            &[("XObject", "AutomatImage", Object::Reference(image))],
            |noms| {
                format!(
                    "q {dessin_l:.3} 0 0 {dessin_h:.3} {x:.3} {y:.3} cm /{} Do Q\n",
                    noms[0]
                )
            },
        )?;
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
    use crate::pdf::{charger, ecrire};
    use crate::rendu;

    fn numeros_options(
        format: FormatNumero,
        position: Position,
        taille: f64,
        premier: u32,
    ) -> OptionsNumeros {
        OptionsNumeros {
            format,
            position,
            taille,
            premier,
            marge_mm: 10.0,
        }
    }

    fn textes(pdf: &[u8]) -> Vec<String> {
        let document = Document::load_mem(pdf).unwrap();
        document
            .get_pages()
            .keys()
            .map(|numero| document.extract_text(&[*numero]).unwrap_or_default())
            .collect()
    }

    /// Première page rendue (page affichée, rotation comprise), `largeur` pixels de large.
    fn rendu_rvb(pdf: Vec<u8>, largeur: u32) -> image::RgbImage {
        let jpeg = rendu::image_page(Arc::new(pdf), "t.pdf", 0, largeur, 95).unwrap();
        image::load_from_memory(&jpeg).unwrap().to_rgb8()
    }

    fn pixels(
        image: &image::RgbImage,
        (x0, y0, x1, y1): (u32, u32, u32, u32),
        garder: impl Fn([u8; 3]) -> bool,
    ) -> usize {
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| garder(image.get_pixel(x, y).0))
            .count()
    }

    fn png_uni(couleur: [u8; 3]) -> Vec<u8> {
        let image = image::RgbImage::from_pixel(40, 20, image::Rgb(couleur));
        let mut octets = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut octets), image::ImageFormat::Png)
            .unwrap();
        octets
    }

    #[test]
    fn numerote_les_pages_choisies_a_partir_du_numero_voulu() {
        let mut document = charger(&pdf_herite(3, "Texte"), "t.pdf").unwrap();
        let options = OptionsMarques {
            numeros: Some(numeros_options(
                FormatNumero::PageNSurTotal,
                Position::BasCentre,
                10.0,
                5,
            )),
            tampon: None,
            pages: vec![2, 1],
        };
        marquer(&mut document, &options).unwrap();
        let textes = textes(&ecrire(document).unwrap());
        assert!(!textes[0].contains("sur"), "{textes:?}");
        assert!(textes[1].contains("Page 5 sur 6"), "{textes:?}");
        assert!(
            textes[2].contains("Page 6 sur 6") && textes[2].contains("Texte 3"),
            "{textes:?}"
        );
    }

    #[test]
    fn ecrit_a_l_endroit_sur_une_page_tournee() {
        let mut document = charger(&pdf_herite(1, "T"), "t.pdf").unwrap();
        let page = document.get_pages()[&1];
        document.get_dictionary_mut(page).unwrap().set("Rotate", 90);
        let options = OptionsMarques {
            numeros: Some(numeros_options(
                FormatNumero::N,
                Position::BasCentre,
                40.0,
                7,
            )),
            tampon: None,
            pages: Vec::new(),
        };
        marquer(&mut document, &options).unwrap();
        // Page affichée en paysage (842 × 595 points, un pixel par point) : « 7 » en bas au centre.
        let image = rendu_rvb(ecrire(document).unwrap(), 842);
        assert_eq!((image.width(), image.height()), (842, 595));
        let sombre = |p: [u8; 3]| p[0] < 120;
        assert!(pixels(&image, (405, 535, 440, 570), sombre) > 30);
        assert_eq!(pixels(&image, (405, 20, 440, 60), sombre), 0);
    }

    #[test]
    fn pose_un_filigrane_transparent_et_garde_le_texte() {
        let mut document = charger(&pdf_herite(1, "Original"), "t.pdf").unwrap();
        let options = OptionsMarques {
            numeros: None,
            tampon: Some(OptionsTampon {
                texte: "CONFIDENTIEL".into(),
                style: StyleTampon::Filigrane,
                couleur: Couleur::Rouge,
                opacite: 0.3,
            }),
            pages: Vec::new(),
        };
        marquer(&mut document, &options).unwrap();
        let pdf = ecrire(document).unwrap();
        assert!(textes(&pdf)[0].contains("Original"));
        // Au centre : du rouge pâle (transparent), ni blanc ni rouge vif.
        let image = rendu_rvb(pdf, 595);
        let rose = |p: [u8; 3]| p[0] > 200 && p[1] > 140 && p[1] < 235;
        assert!(pixels(&image, (230, 360, 370, 480), rose) > 200);
        assert_eq!(
            pixels(&image, (230, 360, 370, 480), |p| p[0] > 150 && p[1] < 60),
            0
        );
    }

    #[test]
    fn pose_un_tampon_encadre_en_haut_a_droite() {
        let mut document = charger(&pdf_herite(1, "T"), "t.pdf").unwrap();
        let options = OptionsMarques {
            numeros: None,
            tampon: Some(OptionsTampon {
                texte: "PAYÉ".into(),
                style: StyleTampon::Tampon,
                couleur: Couleur::Rouge,
                opacite: 1.0,
            }),
            pages: Vec::new(),
        };
        marquer(&mut document, &options).unwrap();
        let image = rendu_rvb(ecrire(document).unwrap(), 595);
        let rouge = |p: [u8; 3]| p[0] > 150 && p[1] < 80 && p[2] < 80;
        assert!(pixels(&image, (440, 20, 570, 110), rouge) > 50);
        assert_eq!(pixels(&image, (20, 700, 200, 820), rouge), 0);
    }

    #[test]
    fn appose_deux_images_sans_que_la_seconde_remplace_la_premiere() {
        let mut document = charger(&pdf_herite(1, "T"), "t.pdf").unwrap();
        let noir = image_decodee(&png_uni([0, 0, 0]), "noir.png").unwrap();
        let rouge = image_decodee(&png_uni([220, 0, 0]), "rouge.png").unwrap();
        let placement = |x, y| Placement {
            x,
            y,
            largeur: 0.2,
            hauteur: 0.1,
        };
        apposer(&mut document, noir, placement(0.1, 0.1), &[]).unwrap();
        let pdf = ecrire(document).unwrap();
        let mut document = Document::load_mem(&pdf).unwrap();
        apposer(&mut document, rouge, placement(0.6, 0.6), &[]).unwrap();
        let image = rendu_rvb(ecrire(document).unwrap(), 595);
        // Image de 40 × 20 contenue dans un cadre de 119 × 84 points : centrée, 119 × 59,5.
        let centre_noir = image
            .get_pixel((0.2 * 595.0) as u32, (0.15 * 842.0) as u32)
            .0;
        let centre_rouge = image
            .get_pixel((0.7 * 595.0) as u32, (0.65 * 842.0) as u32)
            .0;
        assert!(centre_noir.iter().all(|&c| c < 60), "{centre_noir:?}");
        assert!(
            centre_rouge[0] > 180 && centre_rouge[1] < 60,
            "{centre_rouge:?}"
        );
    }
}
