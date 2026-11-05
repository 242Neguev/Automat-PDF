//! Compression : un PDF plus léger, pour l'envoyer par e-mail ou par WhatsApp. Chaque image est
//! ramenée à la résolution choisie d'après la taille à laquelle elle s'affiche (une photo réduite
//! à un timbre n'a pas besoin de ses millions de pixels), puis recompressée : en JPEG si cela
//! allège nettement (photos, scans), sans perte sinon (dessins, captures d'écran). Les objets en
//! double (polices et logos répétés après une fusion) ne sont gardés qu'une fois, les données
//! inutiles à l'affichage (vignettes, données privées des logiciels de mise en page) sont
//! retirées, et le reste est compressé et regroupé.

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, GrayImage, RgbImage};
use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use serde::{Deserialize, Serialize};

use crate::erreur::Erreur;
use crate::geometrie::{A4, Matrice};
use crate::pdf::{charger, contenu_page, dictionnaire};
use crate::pixels::Pixels;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Niveau {
    /// Qualité d'impression (300 ppp).
    Legere,
    /// Bonne qualité à l'écran (200 ppp).
    Moyenne,
    /// Le plus léger, pour l'envoi (150 ppp).
    Forte,
}

impl Niveau {
    /// Résolution visée (points par pouce) et qualité JPEG.
    fn reglages(self) -> (f64, u8) {
        match self {
            Niveau::Legere => (300.0, 85),
            Niveau::Moyenne => (200.0, 72),
            Niveau::Forte => (150.0, 60),
        }
    }
}

/// Ce que la compression a fait : tailles (octets), images allégées, objets en double retirés.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bilan {
    pub avant: u64,
    pub apres: u64,
    pub images: usize,
    pub doublons: usize,
}

/// Au-delà, les formulaires imbriqués ne sont plus suivis (fichier malformé ou piégé).
const FORMULAIRES_MAX: usize = 10_000;

/// Relevé de la plus grande taille d'affichage (largeur et hauteur, en points) de chaque image.
struct Releve<'a> {
    document: &'a Document,
    tailles: HashMap<ObjectId, (f64, f64)>,
    formulaires: usize,
}

impl<'a> Releve<'a> {
    /// Images dessinées par les pages, y compris dans les formulaires qu'elles dessinent (logos
    /// répétés, pages imposées…).
    fn de(document: &'a Document) -> HashMap<ObjectId, (f64, f64)> {
        let mut releve = Releve {
            document,
            tailles: HashMap::new(),
            formulaires: 0,
        };
        for page in document.get_pages().into_values() {
            let Ok(dict) = document.get_dictionary(page) else {
                continue;
            };
            if let Some(contenu) = contenu_page(document, dict) {
                let ressources = dictionnaire(document, dict, b"Resources");
                releve.parcourir(&contenu, ressources, Matrice::IDENTITE, 0);
            }
        }
        releve.tailles
    }

    fn parcourir(
        &mut self,
        contenu: &[u8],
        ressources: Option<&'a Dictionary>,
        mut ctm: Matrice,
        profondeur: usize,
    ) {
        let Ok(contenu) = Content::decode(contenu) else {
            return;
        };
        let objets = ressources.and_then(|r| dictionnaire(self.document, r, b"XObject"));
        let mut pile = Vec::new();
        for operation in &contenu.operations {
            match operation.operator.as_str() {
                "q" => pile.push(ctm),
                "Q" => ctm = pile.pop().unwrap_or(ctm),
                "cm" => {
                    if let Some(matrice) = Matrice::de_nombres(&operation.operands) {
                        ctm = matrice.puis(ctm);
                    }
                }
                "Do" => {
                    let id = operation
                        .operands
                        .first()
                        .and_then(|nom| objets?.get(nom.as_name().ok()?).ok()?.as_reference().ok());
                    if let Some(id) = id {
                        self.dessiner(id, ressources, ctm, profondeur);
                    }
                }
                _ => {}
            }
        }
    }

    /// Une image ou un formulaire dessiné par la matrice `ctm`.
    fn dessiner(
        &mut self,
        id: ObjectId,
        ressources: Option<&'a Dictionary>,
        ctm: Matrice,
        profondeur: usize,
    ) {
        let document = self.document;
        let Ok(flux) = document.get_object(id).and_then(Object::as_stream) else {
            return;
        };
        match flux.dict.get(b"Subtype").and_then(Object::as_name).ok() {
            Some(b"Image") => {
                // L'image remplit le carré unité : ses côtés deviennent les vecteurs (a, b) et
                // (c, d). Un dessin dégénéré (invisible) ne compte pas.
                let [a, b, c, d, _, _] = ctm.0;
                let (largeur, hauteur) = (a.hypot(b), c.hypot(d));
                if largeur >= 0.5 && hauteur >= 0.5 {
                    let taille = self.tailles.entry(id).or_insert((0.0, 0.0));
                    *taille = (taille.0.max(largeur), taille.1.max(hauteur));
                }
            }
            Some(b"Form") if profondeur < 12 && self.formulaires < FORMULAIRES_MAX => {
                self.formulaires += 1;
                let matrice = flux
                    .dict
                    .get(b"Matrix")
                    .and_then(Object::as_array)
                    .ok()
                    .and_then(|m| Matrice::de_nombres(m))
                    .unwrap_or(Matrice::IDENTITE);
                // Un formulaire sans ressources propres emprunte celles de qui le dessine.
                let propres = dictionnaire(document, &flux.dict, b"Resources").or(ressources);
                if let Ok(contenu) = flux.decompressed_content() {
                    self.parcourir(&contenu, propres, matrice.puis(ctm), profondeur + 1);
                }
            }
            _ => {}
        }
    }
}

/// Taille supposée d'une image qu'aucune page ne dessine directement (dans une annotation, un
/// motif…) : la page A4 entière, dans le sens de l'image.
fn page_entiere(largeur: u32, hauteur: u32) -> (f64, f64) {
    let (l, h) = (f64::from(largeur), f64::from(hauteur));
    let (cadre_l, cadre_h) = if l > h { (A4.1, A4.0) } else { A4 };
    let echelle = (cadre_l / l).min(cadre_h / h);
    (l * echelle, h * echelle)
}

/// Peu de couleurs différentes (logo, dessin, capture d'écran) : le JPEG les abîmerait.
fn peu_de_couleurs(rvb: &[u8]) -> bool {
    let nombre = rvb.len() / 3;
    let pas = (nombre / 4096).max(1);
    let couleurs: HashSet<&[u8]> = (0..nombre)
        .step_by(pas)
        .map(|i| &rvb[i * 3..i * 3 + 3])
        .collect();
    couleurs.len() <= 256
}

fn sans_perte(octets: &[u8]) -> Option<Vec<u8>> {
    let mut encodeur = ZlibEncoder::new(Vec::new(), Compression::default());
    encodeur.write_all(octets).ok()?;
    encodeur.finish().ok()
}

/// L'image allégée : ramenée à la résolution visée si elle est nettement plus fine (à sa plus
/// grande taille d'affichage), puis recompressée ; `None` si l'on ne sait pas la relire sans
/// risque.
fn alleger(
    document: &Document,
    flux: &Stream,
    affichage: Option<(f64, f64)>,
    (ppp, qualite): (f64, u8),
) -> Option<Stream> {
    let Pixels {
        pixels: mut brut,
        largeur,
        hauteur,
        composantes,
        ..
    } = Pixels::de(document, flux)?;
    let gris = match composantes {
        1 => true,
        3 => false,
        _ => return None,
    };
    brut.truncate(largeur * hauteur * composantes);
    let (largeur, hauteur) = (u32::try_from(largeur).ok()?, u32::try_from(hauteur).ok()?);
    let mut image = if gris {
        DynamicImage::ImageLuma8(GrayImage::from_raw(largeur, hauteur, brut)?)
    } else {
        DynamicImage::ImageRgb8(RgbImage::from_raw(largeur, hauteur, brut)?)
    };
    // Pixels nécessaires dans chaque sens ; une image à peine plus fine garde sa taille (la
    // réduire ne gagnerait presque rien).
    let (l_pt, h_pt) = affichage.unwrap_or_else(|| page_entiere(largeur, hauteur));
    let echelle =
        (l_pt / 72.0 * ppp / f64::from(largeur)).max(h_pt / 72.0 * ppp / f64::from(hauteur));
    if echelle < 0.8 {
        let nouvelle = |longueur: u32| ((f64::from(longueur) * echelle).round() as u32).max(1);
        image = image.resize_exact(nouvelle(largeur), nouvelle(hauteur), FilterType::Lanczos3);
    }
    let (l, h) = (image.width(), image.height());
    let brut = if gris {
        image.into_luma8().into_raw()
    } else {
        image.into_rgb8().into_raw()
    };
    let mut dict = flux.dict.clone();
    for cle in [&b"Filter"[..], b"DecodeParms", b"Length"] {
        dict.remove(cle);
    }
    dict.set("Width", i64::from(l));
    dict.set("Height", i64::from(h));
    dict.set("BitsPerComponent", 8);
    // Le JPEG n'est retenu que s'il allège nettement : un dessin, qu'il abîmerait, se compresse
    // presque aussi bien sans perte.
    let jpeg = if gris || !peu_de_couleurs(&brut) {
        let couleurs = if gris {
            ExtendedColorType::L8
        } else {
            ExtendedColorType::Rgb8
        };
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, qualite)
            .encode(&brut, l, h, couleurs)
            .ok()
            .map(|()| jpeg)
    } else {
        None
    };
    let compresse = sans_perte(&brut)?;
    match jpeg {
        Some(jpeg) if jpeg.len() * 2 < compresse.len() => {
            dict.set("Filter", "DCTDecode");
            Some(Stream::new(dict, jpeg))
        }
        _ => {
            dict.set("Filter", "FlateDecode");
            Some(Stream::new(dict, compresse))
        }
    }
}

/// Images que l'on sait alléger : ni masque de transparence d'une autre image, ni image à couleur
/// transparente (/Mask), ni image dont le masque doit garder les mêmes dimensions (/Matte).
fn images_a_alleger(document: &Document) -> Vec<ObjectId> {
    let masque = |flux: &Stream| flux.dict.get(b"SMask").and_then(Object::as_reference).ok();
    let masques: HashSet<ObjectId> = document
        .objects
        .values()
        .filter_map(|objet| masque(objet.as_stream().ok()?))
        .collect();
    document
        .objects
        .iter()
        .filter(|(id, objet)| {
            let Ok(flux) = objet.as_stream() else {
                return false;
            };
            let assorti = masque(flux)
                .and_then(|id| document.get_object(id).and_then(Object::as_stream).ok())
                .is_some_and(|m| m.dict.has(b"Matte"));
            flux.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image")
                && !masques.contains(id)
                && !flux.dict.has(b"Mask")
                && !assorti
        })
        .map(|(id, _)| *id)
        .collect()
}

/// Dictionnaire d'un flux sans sa longueur (qui dépend de l'encodage), pour comparer.
fn sans_longueur(dict: &Dictionary) -> Dictionary {
    let mut copie = dict.clone();
    copie.remove(b"Length");
    copie
}

fn remplacer(objet: &mut Object, remplacements: &HashMap<ObjectId, ObjectId>) {
    match objet {
        Object::Reference(id) => {
            if let Some(nouveau) = remplacements.get(id) {
                *id = *nouveau;
            }
        }
        Object::Array(liste) => liste.iter_mut().for_each(|o| remplacer(o, remplacements)),
        Object::Dictionary(dict) => dict
            .iter_mut()
            .for_each(|(_, o)| remplacer(o, remplacements)),
        Object::Stream(flux) => flux
            .dict
            .iter_mut()
            .for_each(|(_, o)| remplacer(o, remplacements)),
        _ => {}
    }
}

/// Garde un seul exemplaire des flux identiques (polices, images, profils de couleur…) et fait
/// pointer toutes les références vers lui ; renvoie le nombre d'exemplaires retirés.
fn dedoublonner(document: &mut Document) -> usize {
    let mut par_empreinte: HashMap<u64, Vec<ObjectId>> = HashMap::new();
    let mut remplacements: HashMap<ObjectId, ObjectId> = HashMap::new();
    for (&id, objet) in &document.objects {
        let Object::Stream(flux) = objet else {
            continue;
        };
        let mut hacheur = DefaultHasher::new();
        flux.content.hash(&mut hacheur);
        format!("{:?}", sans_longueur(&flux.dict)).hash(&mut hacheur);
        let candidats = par_empreinte.entry(hacheur.finish()).or_default();
        let identique = candidats.iter().copied().find(|autre| {
            document
                .objects
                .get(autre)
                .and_then(|o| o.as_stream().ok())
                .is_some_and(|autre| {
                    autre.content == flux.content
                        && sans_longueur(&autre.dict) == sans_longueur(&flux.dict)
                })
        });
        match identique {
            Some(original) => {
                remplacements.insert(id, original);
            }
            None => candidats.push(id),
        }
    }
    if remplacements.is_empty() {
        return 0;
    }
    for objet in document.objects.values_mut() {
        remplacer(objet, &remplacements);
    }
    document
        .trailer
        .iter_mut()
        .for_each(|(_, o)| remplacer(o, &remplacements));
    for id in remplacements.keys() {
        document.objects.remove(id);
    }
    remplacements.len()
}

/// Le PDF allégé ; si rien n'a pu être gagné, le PDF d'origine tel quel.
pub fn compresser(octets: &[u8], nom: &str, niveau: Niveau) -> Result<(Vec<u8>, Bilan), Erreur> {
    let mut document = charger(octets, nom)?;
    let mut bilan = Bilan {
        avant: octets.len() as u64,
        ..Bilan::default()
    };
    let images = images_a_alleger(&document);
    let tailles = Releve::de(&document);
    let reglages = niveau.reglages();
    // Plusieurs images à la fois, quatre au plus : chacune occupe sa place en mémoire une fois
    // décodée.
    let fils = std::thread::available_parallelism()
        .map_or(2, |n| n.get())
        .clamp(1, 4);
    let allegees: Vec<(ObjectId, Stream)> = std::thread::scope(|portee| {
        let travaux: Vec<_> = images
            .chunks(images.len().div_ceil(fils).max(1))
            .map(|lot| {
                let (document, tailles) = (&document, &tailles);
                portee.spawn(move || {
                    lot.iter()
                        .filter_map(|&id| {
                            let flux = document.get_object(id).and_then(Object::as_stream).ok()?;
                            let nouvelle =
                                alleger(document, flux, tailles.get(&id).copied(), reglages)?;
                            // Gardée seulement si elle allège vraiment (10 % au moins).
                            (nouvelle.content.len() * 10 < flux.content.len() * 9)
                                .then_some((id, nouvelle))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        travaux
            .into_iter()
            .flat_map(|travail| travail.join().unwrap_or_default())
            .collect()
    });
    bilan.images = allegees.len();
    for (id, flux) in allegees {
        document.objects.insert(id, Object::Stream(flux));
    }
    for page in document.get_pages().into_values() {
        if let Ok(page) = document.get_dictionary_mut(page) {
            page.remove(b"Thumb");
            page.remove(b"PieceInfo");
        }
    }
    if let Ok(catalogue) = document.catalog_mut() {
        catalogue.remove(b"PieceInfo");
    }
    bilan.doublons = dedoublonner(&mut document);
    document.prune_objects();
    document.compress();
    let mut sortie = Vec::new();
    document
        .save_modern(&mut sortie)
        .map_err(|erreur| Erreur::simple(format!("Le PDF n'a pas pu être écrit : {erreur}")))?;
    if sortie.len() >= octets.len() {
        bilan.apres = bilan.avant;
        return Ok((octets.to_vec(), bilan));
    }
    bilan.apres = sortie.len() as u64;
    Ok((sortie, bilan))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use lopdf::dictionary;

    use super::*;
    use crate::essais::pdf_herite;
    use crate::pdf::fusionner;

    /// Image non compressée, en couleurs (RVB) ou en gris (première composante de `pixel`).
    fn image(
        largeur: usize,
        hauteur: usize,
        gris: bool,
        pixel: impl Fn(usize, usize) -> [u8; 3],
    ) -> Stream {
        let mut pixels = Vec::with_capacity(largeur * hauteur * 3);
        for y in 0..hauteur {
            for x in 0..largeur {
                let p = pixel(x, y);
                if gris {
                    pixels.push(p[0]);
                } else {
                    pixels.extend_from_slice(&p);
                }
            }
        }
        Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image",
                "Width" => largeur as i64, "Height" => hauteur as i64,
                "ColorSpace" => if gris { "DeviceGray" } else { "DeviceRGB" },
                "BitsPerComponent" => 8,
            },
            pixels,
        )
    }

    /// Une photo : des couleurs partout.
    fn photo(largeur: usize, hauteur: usize) -> Stream {
        image(largeur, hauteur, false, |x, y| {
            [(x * 7 + y * 3) as u8, (x * y % 251) as u8, (x ^ y) as u8]
        })
    }

    /// L'image affichée en 500 × 333 points.
    const AFFICHEE: &str = "q 500 0 0 333 50 300 cm /Im1 Do Q";

    /// PDF d'une page A4 dont le contenu dessine l'image `/Im1` ; avec `formulaire`, la page
    /// dessine le formulaire `/Fm1`, qui dessine l'image.
    fn pdf_avec(image: Stream, contenu: &str, formulaire: Option<&str>) -> Vec<u8> {
        let mut document = charger(&pdf_herite(1, "Photo"), "p.pdf").unwrap();
        let image = document.add_object(image);
        let objets = match formulaire {
            None => dictionary! { "Im1" => image },
            Some(dessin) => {
                let boite = [0, 0, 595, 842].map(Object::Integer).to_vec();
                let forme = document.add_object(Stream::new(
                    dictionary! {
                        "Type" => "XObject", "Subtype" => "Form", "BBox" => boite,
                        "Resources" => dictionary! { "XObject" => dictionary! { "Im1" => image } },
                    },
                    dessin.as_bytes().to_vec(),
                ));
                dictionary! { "Fm1" => forme }
            }
        };
        let page = document.get_pages()[&1];
        let contenu =
            document.add_object(Stream::new(Dictionary::new(), contenu.as_bytes().to_vec()));
        let dict = document.get_dictionary_mut(page).unwrap();
        let mut ressources = dict.get(b"Resources").unwrap().as_dict().unwrap().clone();
        ressources.set("XObject", objets);
        dict.set("Resources", ressources);
        dict.set("Contents", contenu);
        let mut octets = Vec::new();
        document.save_to(&mut octets).unwrap();
        octets
    }

    /// Largeur et filtre de chaque image du PDF.
    fn images(pdf: &[u8]) -> Vec<(i64, String)> {
        let document = Document::load_mem(pdf).unwrap();
        document
            .objects
            .values()
            .filter_map(|o| o.as_stream().ok())
            .filter(|f| f.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image"))
            .map(|f| {
                let filtre = f
                    .dict
                    .get(b"Filter")
                    .and_then(Object::as_name)
                    .map(|n| String::from_utf8_lossy(n).into_owned())
                    .unwrap_or_default();
                (f.dict.get(b"Width").unwrap().as_i64().unwrap(), filtre)
            })
            .collect()
    }

    fn jpeg(largeur: i64) -> Vec<(i64, String)> {
        vec![(largeur, "DCTDecode".to_string())]
    }

    #[test]
    fn ramene_une_grande_photo_a_la_resolution_choisie() {
        let pdf = pdf_avec(photo(3000, 2000), AFFICHEE, None);
        let (sortie, bilan) = compresser(&pdf, "photo.pdf", Niveau::Forte).unwrap();
        assert_eq!(bilan.images, 1);
        assert!(bilan.apres < bilan.avant / 10, "{bilan:?}");
        // 500 points (6,9 pouces) à 150 ppp : 1 042 pixels.
        assert_eq!(images(&sortie), jpeg(1042));
        // La page s'affiche toujours, image comprise.
        let rendu = crate::rendu::image_page(Arc::new(sortie), "c.pdf", 0, 300, 90).unwrap();
        let rendu = image::load_from_memory(&rendu).unwrap().to_rgb8();
        assert!(
            rendu
                .pixels()
                .filter(|p| p.0.iter().any(|&c| c < 200))
                .count()
                > 1000
        );
    }

    #[test]
    fn suit_la_taille_affichee_jusque_dans_les_formulaires() {
        // Petite : 2 pouces de large, soit 300 pixels à 150 ppp.
        let petite = pdf_avec(photo(3000, 2000), "q 144 0 0 96 50 300 cm /Im1 Do Q", None);
        let (sortie, _) = compresser(&petite, "p.pdf", Niveau::Forte).unwrap();
        assert_eq!(images(&sortie), jpeg(300));
        // Dans un formulaire réduit de moitié (comme sur une page imposée) : 250 points.
        let imposee = pdf_avec(
            photo(3000, 2000),
            "q 0.5 0 0 0.5 0 0 cm /Fm1 Do Q",
            Some(AFFICHEE),
        );
        let (sortie, _) = compresser(&imposee, "i.pdf", Niveau::Forte).unwrap();
        assert_eq!(images(&sortie), jpeg(521));
        // Jamais dessinée par une page : supposée occuper une page A4 (en paysage, 842 points).
        let cachee = pdf_avec(photo(3000, 2000), "", None);
        let (sortie, _) = compresser(&cachee, "c.pdf", Niveau::Forte).unwrap();
        assert_eq!(images(&sortie), jpeg(1754));
    }

    #[test]
    fn passe_un_scan_en_gris_en_jpeg() {
        // Page de texte scannée à 150 ppp : grain du papier (bruit sans motif, mélange
        // SplitMix64), lignes de texte sombres.
        let scan = image(1240, 1754, true, |x, y| {
            let mut n = ((y as u64) << 32) | x as u64;
            n = (n ^ (n >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            n = (n ^ (n >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            let grain = ((n ^ (n >> 31)) % 9) as u8;
            let encre = (y / 20) % 3 == 0 && x % 24 < 17 && (100..1140).contains(&x);
            [if encre { 40 } else { 228 } + grain; 3]
        });
        let pdf = pdf_avec(scan, "q 595.28 0 0 841.89 0 0 cm /Im1 Do Q", None);
        let (sortie, bilan) = compresser(&pdf, "scan.pdf", Niveau::Forte).unwrap();
        assert_eq!(bilan.images, 1);
        // Déjà à la bonne résolution : seulement recompressée.
        assert_eq!(images(&sortie), jpeg(1240));
    }

    #[test]
    fn garde_sans_perte_un_dessin_en_noir_et_blanc() {
        let damier = image(3000, 2000, false, |x, y| {
            if (x / 50 + y / 50) % 2 == 0 {
                [0, 0, 0]
            } else {
                [255, 255, 255]
            }
        });
        let pdf = pdf_avec(damier, AFFICHEE, None);
        let (sortie, bilan) = compresser(&pdf, "dessin.pdf", Niveau::Moyenne).unwrap();
        assert_eq!(bilan.images, 1);
        assert_eq!(images(&sortie), [(1389, "FlateDecode".to_string())]);
    }

    #[test]
    fn ne_garde_qu_un_exemplaire_des_objets_repetes() {
        let pdf = pdf_avec(
            image(200, 100, false, |x, _| [x as u8, 0, 0]),
            AFFICHEE,
            None,
        );
        let fusion = fusionner(&[
            ("a.pdf".to_string(), pdf.clone()),
            ("b.pdf".to_string(), pdf),
        ])
        .unwrap();
        assert_eq!(images(&fusion).len(), 2);
        let (sortie, bilan) = compresser(&fusion, "fusion.pdf", Niveau::Legere).unwrap();
        assert!(bilan.doublons >= 1, "{bilan:?}");
        assert_eq!(images(&sortie).len(), 1);
        assert_eq!(Document::load_mem(&sortie).unwrap().get_pages().len(), 2);
    }

    #[test]
    fn rend_le_pdf_d_origine_quand_rien_n_est_gagne() {
        let pdf = fusionner(&[("t.pdf".to_string(), pdf_herite(1, "Court"))]).unwrap();
        let (sortie, bilan) = compresser(&pdf, "court.pdf", Niveau::Forte).unwrap();
        assert!(bilan.apres <= bilan.avant);
        if bilan.apres == bilan.avant {
            assert_eq!(sortie, pdf);
        }
        assert_eq!(Document::load_mem(&sortie).unwrap().get_pages().len(), 1);
    }
}
