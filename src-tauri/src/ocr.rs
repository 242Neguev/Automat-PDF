//! Reconnaissance du texte des pages numérisées (OCR), par le moteur du système, sur l'ordinateur
//! et sans Internet : Vision sur Mac, Windows.Media.Ocr sous Windows 10 et 11. Rien à installer.
//!
//! Chaque page sans texte est dessinée à 300 points par pouce et lue par le moteur. Une couche de
//! texte invisible est ensuite posée sur son image, mot par mot, à la place de chaque mot. Le PDF
//! devient cherchable et son texte se copie, dans Automat-PDF (recherche, Séparer par personne, PDF
//! en Word, Masquer, Corriger) comme dans les autres lecteurs de PDF. L'image ne change pas.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use lopdf::{Document, Object, ObjectId};
use serde::Serialize;

use crate::erreur::{Echec, Erreur};
use crate::geometrie::Cadre;
use crate::office::Conversions;
use crate::{fichiers, marques, memoire, pdf, police, rendu, signatures, texte, volume};

/// Un mot reconnu : son texte et sa boîte dans l'image de la page, en fractions de l'image (0 à 1)
/// depuis le coin en haut à gauche : gauche, haut, largeur, hauteur.
#[derive(Debug, Clone, PartialEq)]
pub struct Mot {
    pub texte: String,
    pub boite: [f64; 4],
}

/// Une ligne reconnue : ses mots, dans l'ordre de lecture.
pub type Ligne = Vec<Mot>;

/// Résolution de lecture, celle des scans courants.
const PPP: u32 = 300;
/// Plus grand côté de l'image confiée au moteur, en pixels : au-delà, la page est lue moins finement.
const COTE_MAX: u32 = 4200;
/// En dessous de ce nombre de caractères (hors espaces), une page compte comme sans texte.
const TEXTE_MIN: usize = 3;
/// Pages lues en même temps.
const EN_PARALLELE: usize = 4;
/// Helvetica, de la descente (-0,207) à la montée (0,718) : la hauteur d'une ligne de texte, en
/// tailles de police.
const HAUTEUR_LIGNE: f64 = 0.925;
const DESCENTE: f64 = 0.207;

/// Ce qu'a donné la reconnaissance d'un PDF.
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bilan {
    /// Pages sans texte, confiées au moteur.
    pub pages_lues: usize,
    /// Parmi elles, celles où il a reconnu du texte.
    pub pages_reconnues: usize,
    pub mots: usize,
}

/// Le moteur du système est-il prêt ? Sinon, pourquoi (langue de reconnaissance absente…).
pub fn disponible() -> Result<(), Erreur> {
    systeme::disponible()
}

/// Pages regardées pour dire si un PDF semble numérisé.
const PAGES_REGARDEES: usize = 5;

/// Le PDF semble-t-il numérisé : aucune de ses premières pages n'a de texte ?
pub fn semble_numerise(octets: Arc<Vec<u8>>, nom: &str) -> Result<bool, Erreur> {
    let pages = rendu::formats(octets.clone(), nom)?.len();
    Ok(pages > 0 && !texte::a_du_texte(octets, nom, PAGES_REGARDEES)?)
}

/// Les pages sans texte d'un PDF (à partir de 0).
fn pages_sans_texte(octets: &Arc<Vec<u8>>, nom: &str) -> Result<Vec<usize>, Erreur> {
    Ok(texte::textes(octets.clone(), nom)?
        .iter()
        .enumerate()
        .filter(|(_, texte)| texte.chars().filter(|c| !c.is_whitespace()).count() < TEXTE_MIN)
        .map(|(page, _)| page)
        .collect())
}

/// Le PDF, avec le texte de ses pages numérisées. `avancement(pages lues, pages à lire)`.
pub fn reconnaitre_pdf(
    octets: Arc<Vec<u8>>,
    nom: &str,
    avancement: &(dyn Fn(usize, usize) + Sync),
) -> Result<(Vec<u8>, Bilan), Erreur> {
    let mut document = pdf::charger(&octets, nom)?;
    if signatures::a_des_signatures(&document, &octets) {
        return Err(Erreur::simple(
            "Ce PDF est signé numériquement : y ajouter son texte changerait ce qui a été signé.",
        ));
    }
    let a_lire = pages_sans_texte(&octets, nom)?;
    if a_lire.is_empty() {
        return Err(Erreur::simple(
            "Ce PDF a déjà du texte sur toutes ses pages : il n'y a rien à reconnaître.",
        ));
    }
    let lues = lire_pages(&octets, nom, &a_lire, avancement)?;
    let pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    let mut bilan = Bilan {
        pages_lues: a_lire.len(),
        ..Bilan::default()
    };
    for (page, lignes) in lues {
        let (Some(&id), false) = (pages.get(page), lignes.is_empty()) else {
            continue;
        };
        bilan.pages_reconnues += 1;
        bilan.mots += lignes.iter().map(Vec::len).sum::<usize>();
        poser(&mut document, id, &lignes)?;
    }
    if bilan.mots == 0 {
        return Err(Erreur::simple(
            "Aucun texte n'a été reconnu sur ses pages : ce sont peut-être des photos ou des dessins.",
        ));
    }
    Ok((pdf::ecrire(document)?, bilan))
}

/// Les pages données, lues par le moteur, quelques-unes à la fois. La première erreur arrête tout.
fn lire_pages(
    octets: &Arc<Vec<u8>>,
    nom: &str,
    pages: &[usize],
    avancement: &(dyn Fn(usize, usize) + Sync),
) -> Result<Vec<(usize, Vec<Ligne>)>, Erreur> {
    let formats = rendu::formats(octets.clone(), nom)?;
    let suivante = AtomicUsize::new(0);
    let faites = AtomicUsize::new(0);
    let arret = AtomicBool::new(false);
    let lues = Mutex::new(Vec::new());
    let erreur = Mutex::new(None);
    std::thread::scope(|portee| {
        for _ in 0..EN_PARALLELE.min(pages.len()) {
            portee.spawn(|| {
                while !arret.load(Ordering::Relaxed) {
                    let Some(&page) = pages.get(suivante.fetch_add(1, Ordering::Relaxed)) else {
                        break;
                    };
                    // 300 ppp, moins si la page est grande : son plus grand côté, en pixels, borné.
                    let cote = formats
                        .get(page)
                        .map_or(842.0, |f| f64::from(f.largeur.max(f.hauteur)).max(1.0));
                    let ppp = (f64::from(systeme::cote_max().min(COTE_MAX)) * 72.0 / cote)
                        .min(f64::from(PPP)) as u32;
                    let lecture = rendu::pixels_page(octets.clone(), nom, page, ppp.max(72))
                        .and_then(|(largeur, hauteur, rvb)| {
                            systeme::reconnaitre(largeur, hauteur, &rvb)
                        });
                    match lecture {
                        Ok(lignes) => lues
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .push((page, lignes)),
                        Err(e) => {
                            arret.store(true, Ordering::Relaxed);
                            *erreur.lock().unwrap_or_else(|e| e.into_inner()) = Some(e);
                            break;
                        }
                    }
                    avancement(faites.fetch_add(1, Ordering::Relaxed) + 1, pages.len());
                }
            });
        }
    });
    if let Some(erreur) = erreur.into_inner().unwrap_or_else(|e| e.into_inner()) {
        return Err(erreur);
    }
    let mut lues = lues.into_inner().unwrap_or_else(|e| e.into_inner());
    lues.sort_by_key(|(page, _)| *page);
    Ok(lues)
}

/// La couche de texte invisible d'une page : chaque mot reconnu, à sa place sur l'image, à la
/// hauteur de sa ligne, étiré à la largeur du mot (Helvetica, mode de rendu 3 : invisible).
fn poser(document: &mut Document, page: ObjectId, lignes: &[Ligne]) -> Result<(), Erreur> {
    let dictionnaire = document
        .get_dictionary(page)
        .map_err(|_| Erreur::simple("Une page n'a pas pu être lue."))?
        .clone();
    let (largeur, hauteur) = Cadre::de_page(document, &dictionnaire).taille_vue();
    marques::surimprimer(
        document,
        page,
        &[(
            "Font",
            "AutomatTexte",
            Object::Dictionary(police::police(false)),
        )],
        |noms| dessin_texte(&noms[0], lignes, (largeur, hauteur)),
    )
}

/// Les opérateurs de la couche de texte, dans le repère de la page affichée (origine en bas à
/// gauche, en points).
fn dessin_texte(nom: &str, lignes: &[Ligne], (largeur, hauteur): (f64, f64)) -> String {
    let mut dessin = String::from("BT 3 Tr\n");
    for ligne in lignes.iter().filter(|ligne| !ligne.is_empty()) {
        let haut = ligne.iter().map(|m| m.boite[1]).fold(1.0, f64::min);
        let bas = ligne
            .iter()
            .map(|m| m.boite[1] + m.boite[3])
            .fold(0.0, f64::max);
        let taille = ((bas - haut) * hauteur / HAUTEUR_LIGNE).clamp(1.0, 300.0);
        let base = hauteur - bas * hauteur + DESCENTE * taille;
        for (rang, mot) in ligne.iter().enumerate() {
            // Une espace après chaque mot, sauf le dernier : le texte copié garde ses mots séparés.
            let texte = if rang + 1 < ligne.len() {
                format!("{} ", mot.texte)
            } else {
                mot.texte.clone()
            };
            let naturelle = police::largeur(&mot.texte, false, taille).max(0.01);
            let echelle = (100.0 * mot.boite[2] * largeur / naturelle).clamp(10.0, 1000.0);
            dessin.push_str(&format!(
                "/{nom} {taille:.2} Tf {echelle:.1} Tz 1 0 0 1 {:.2} {base:.2} Tm {} Tj\n",
                mot.boite[0] * largeur,
                police::chaine(&texte)
            ));
        }
    }
    dessin.push_str("ET\n");
    dessin
}

/// Le texte des pages numérisées du PDF `chemin`, reconnu en mémoire seulement (`memoire::reconnaitre`) :
/// le lecteur y cherche et en copie le texte, Séparer par personne y repère les personnes, sans
/// rien écrire sur le disque. `avancement(pages lues, pages à lire)`.
pub fn reconnaitre_en_memoire(
    chemin: &str,
    avancement: &(dyn Fn(usize, usize) + Sync),
) -> Result<Bilan, Erreur> {
    disponible()?;
    let nom = crate::operations::nom_de(chemin);
    let mut bilan = Bilan::default();
    memoire::reconnaitre(chemin, |octets| {
        let (avec_texte, lu) = reconnaitre_pdf(octets, &nom, avancement)?;
        bilan = lu;
        Ok(avec_texte)
    })?;
    Ok(bilan)
}

/// Le texte reconnu des pages données (à partir de 0), sans toucher au PDF : la couche de texte est
/// posée sur une copie en mémoire, puis relue comme le texte d'un PDF ordinaire, ligne après ligne
/// d'après la place des mots (les colonnes d'un tableau séparées par des tabulations).
/// `avancement(pages lues, pages à lire)`.
pub fn textes_reconnus(
    octets: &Arc<Vec<u8>>,
    nom: &str,
    pages: &[usize],
    avancement: &(dyn Fn(usize, usize) + Sync),
) -> Result<Vec<(usize, String)>, Erreur> {
    disponible()?;
    let lues = lire_pages(octets, nom, pages, avancement)?;
    let mut document = pdf::charger(octets, nom)?;
    let ids: Vec<ObjectId> = document.get_pages().into_values().collect();
    for (page, lignes) in &lues {
        if let (Some(&id), false) = (ids.get(*page), lignes.is_empty()) {
            poser(&mut document, id, lignes)?;
        }
    }
    let textes = texte::textes(Arc::new(pdf::ecrire(document)?), nom)?;
    Ok(pages
        .iter()
        .map(|&page| (page, textes.get(page).cloned().unwrap_or_default()))
        .collect())
}

/// PDF numérisés rendus cherchables : chacun devient « … - texte reconnu.pdf » dans `dossier`.
/// `avancement(fichiers faits, fichiers, pages lues du fichier en cours, ses pages à lire)`.
pub fn reconnaitre_fichiers(
    fichiers: &[String],
    dossier: &str,
    avancement: impl Fn(usize, usize, usize, usize) + Sync,
) -> Result<Conversions, Erreur> {
    volume::autorise("Reconnaître le texte", fichiers.len())?;
    disponible()?;
    std::fs::create_dir_all(dossier).map_err(|erreur| {
        Erreur::simple(format!("Impossible d'écrire dans « {dossier} » : {erreur}"))
    })?;
    let cibles = cibles(fichiers, Path::new(dossier));
    let mut resultat = Conversions::default();
    for (rang, (source, cible)) in fichiers.iter().zip(cibles).enumerate() {
        avancement(rang, fichiers.len(), 0, 0);
        let nom = crate::operations::nom_de(source);
        let fait = memoire::pdf(source).and_then(|octets| {
            let (ecrit, _) = reconnaitre_pdf(octets, &nom, &|lues, pages| {
                avancement(rang, fichiers.len(), lues, pages)
            })?;
            let cible = cible.to_string_lossy().into_owned();
            fichiers::enregistrer(&cible, &ecrit)?;
            Ok(cible)
        });
        match fait {
            Ok(cible) => resultat.crees.push(cible),
            Err(erreur) => resultat.echecs.push(Echec {
                rang,
                message: erreur.message,
            }),
        }
    }
    avancement(fichiers.len(), fichiers.len(), 0, 0);
    Ok(resultat)
}

/// « nom - texte reconnu.pdf » dans le dossier, sans écraser un fichier qui existe ni deux fois le
/// même nom.
fn cibles(sources: &[String], dossier: &Path) -> Vec<PathBuf> {
    let mut pris = HashSet::new();
    sources
        .iter()
        .map(|source| {
            let base = Path::new(source)
                .file_stem()
                .map(|nom| nom.to_string_lossy().into_owned())
                .unwrap_or_else(|| "document".into());
            (1..)
                .map(|n| match n {
                    1 => dossier.join(format!("{base} - texte reconnu.pdf")),
                    _ => dossier.join(format!("{base} - texte reconnu ({n}).pdf")),
                })
                .find(|chemin| {
                    !chemin.exists() && pris.insert(chemin.to_string_lossy().to_lowercase())
                })
                .unwrap_or_default()
        })
        .collect()
}

/// Les mots d'une ligne de texte, avec leur place dans la chaîne en unités UTF-16 (début, longueur),
/// comme les compte Vision.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn mots_utf16(ligne: &str) -> Vec<(usize, usize, String)> {
    let mut mots = Vec::new();
    let mut position = 0;
    let mut courant: Option<(usize, String)> = None;
    for caractere in ligne.chars() {
        let longueur = caractere.len_utf16();
        if caractere.is_whitespace() {
            if let Some((debut, mot)) = courant.take() {
                mots.push((debut, position - debut, mot));
            }
        } else {
            courant
                .get_or_insert_with(|| (position, String::new()))
                .1
                .push(caractere);
        }
        position += longueur;
    }
    if let Some((debut, mot)) = courant {
        mots.push((debut, position - debut, mot));
    }
    mots
}

#[cfg(target_os = "macos")]
mod systeme {
    use std::io::Cursor;

    use objc2::AnyThread;
    use objc2::rc::{Retained, autoreleasepool};
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSArray, NSData, NSDictionary, NSError, NSRange, NSString};
    use objc2_vision::{
        VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest, VNRecognizedText,
        VNRectangleObservation, VNRequest, VNRequestTextRecognitionLevel,
    };

    use super::{Ligne, Mot, mots_utf16};
    use crate::erreur::Erreur;

    pub fn disponible() -> Result<(), Erreur> {
        Ok(())
    }

    /// Vision lit les grandes images ; au-delà, la lecture ralentit sans gagner en finesse.
    pub fn cote_max() -> u32 {
        super::COTE_MAX
    }

    /// Le texte d'une image (pixels RVB), ligne par ligne, mot par mot.
    pub fn reconnaitre(largeur: u32, hauteur: u32, rvb: &[u8]) -> Result<Vec<Ligne>, Erreur> {
        // Un BMP, sans compression : vite écrit, et lu directement par Vision.
        let image = image::RgbImage::from_raw(largeur, hauteur, rvb.to_vec())
            .ok_or_else(|| Erreur::simple("L'image de la page n'a pas pu être préparée."))?;
        let mut bmp = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bmp), image::ImageFormat::Bmp)
            .map_err(|e| {
                Erreur::simple(format!("L'image de la page n'a pas pu être préparée : {e}"))
            })?;
        autoreleasepool(|_| lire(&bmp))
    }

    fn lire(image: &[u8]) -> Result<Vec<Ligne>, Erreur> {
        let donnees = NSData::with_bytes(image);
        let options = NSDictionary::<VNImageOption, AnyObject>::new();
        let gestionnaire = VNImageRequestHandler::initWithData_options(
            VNImageRequestHandler::alloc(),
            &donnees,
            &options,
        );
        let requete = VNRecognizeTextRequest::new();
        requete.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        requete.setUsesLanguageCorrection(true);
        requete.setRecognitionLanguages(&NSArray::from_retained_slice(&[
            NSString::from_str("fr-FR"),
            NSString::from_str("en-US"),
        ]));
        let generique: Retained<VNRequest> =
            Retained::into_super(Retained::into_super(requete.clone()));
        gestionnaire
            .performRequests_error(&NSArray::from_retained_slice(&[generique]))
            .map_err(|erreur| {
                Erreur::simple(format!(
                    "La reconnaissance du texte a échoué : {}",
                    erreur.localizedDescription()
                ))
            })?;
        let mut lignes = Vec::new();
        let Some(resultats) = requete.results() else {
            return Ok(lignes);
        };
        for observation in resultats.iter() {
            let candidats = observation.topCandidates(1);
            let Some(texte) = candidats.firstObject() else {
                continue;
            };
            let chaine = texte.string().to_string();
            // Boîte de la ligne : Vision compte depuis le coin en bas à gauche.
            // SAFETY : une observation de texte a toujours sa boîte.
            let boite = unsafe { observation.boundingBox() };
            let ligne_boite = [
                boite.origin.x,
                1.0 - boite.origin.y - boite.size.height,
                boite.size.width,
                boite.size.height,
            ];
            let mots = mots_utf16(&chaine);
            let total: usize = mots.iter().map(|(_, n, _)| *n).sum::<usize>().max(1);
            let mut ligne = Vec::new();
            let mut avant = 0;
            for (debut, longueur, mot) in mots {
                let boite = boite_du_mot(&texte, debut, longueur).unwrap_or_else(|| {
                    // À défaut : sa part de la ligne, d'après le nombre de caractères.
                    let gauche = ligne_boite[0] + ligne_boite[2] * avant as f64 / total as f64;
                    [
                        gauche,
                        ligne_boite[1],
                        ligne_boite[2] * longueur as f64 / total as f64,
                        ligne_boite[3],
                    ]
                });
                avant += longueur;
                ligne.push(Mot { texte: mot, boite });
            }
            if !ligne.is_empty() {
                lignes.push(ligne);
            }
        }
        Ok(lignes)
    }

    /// La boîte d'un mot de la ligne reconnue (fractions de l'image, depuis le haut à gauche).
    fn boite_du_mot(texte: &VNRecognizedText, debut: usize, longueur: usize) -> Option<[f64; 4]> {
        let plage = NSRange::new(debut, longueur);
        // SAFETY : méthode de VNRecognizedText (macOS 10.15), avec une plage de sa chaîne.
        let rectangle: Result<Retained<VNRectangleObservation>, Retained<NSError>> =
            unsafe { objc2::msg_send![texte, boundingBoxForRange: plage, error: _] };
        let rectangle = rectangle.ok()?;
        // SAFETY : une observation de rectangle a toujours sa boîte.
        let boite = unsafe { rectangle.boundingBox() };
        let valide = boite.size.width > 0.0 && boite.size.height > 0.0;
        valide.then_some([
            boite.origin.x,
            1.0 - boite.origin.y - boite.size.height,
            boite.size.width,
            boite.size.height,
        ])
    }
}

#[cfg(windows)]
mod systeme {
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};
    use windows::core::HSTRING;

    use super::{Ligne, Mot};
    use crate::erreur::Erreur;

    const SANS_LANGUE: &str = "La reconnaissance du texte de Windows n'a aucune langue installée sur ce PC. Ajoutez-la : Paramètres › Heure et langue › Langue et région › Français › Options de langue › Reconnaissance optique de caractères.";

    fn initialiser() {
        // SAFETY : sans effet si ce fil est déjà initialisé (l'erreur est alors ignorée).
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    }

    /// Le moteur, en français si Windows en a la reconnaissance, sinon dans les langues de
    /// l'utilisateur.
    fn moteur() -> Result<OcrEngine, Erreur> {
        initialiser();
        let francais = Language::CreateLanguage(&HSTRING::from("fr-FR"))
            .map_err(|e| Erreur::simple(format!("{SANS_LANGUE} ({e})")))?;
        if OcrEngine::IsLanguageSupported(&francais).unwrap_or(false)
            && let Ok(moteur) = OcrEngine::TryCreateFromLanguage(&francais)
        {
            return Ok(moteur);
        }
        OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| Erreur::simple(SANS_LANGUE))
    }

    pub fn disponible() -> Result<(), Erreur> {
        moteur().map(|_| ())
    }

    /// Le plus grand côté d'image que le moteur de Windows accepte.
    pub fn cote_max() -> u32 {
        initialiser();
        OcrEngine::MaxImageDimension().unwrap_or(2600).max(1000)
    }

    /// Le texte d'une image (pixels RVB), ligne par ligne, mot par mot.
    pub fn reconnaitre(largeur: u32, hauteur: u32, rvb: &[u8]) -> Result<Vec<Ligne>, Erreur> {
        let echec = |e: windows::core::Error| {
            Erreur::simple(format!("La reconnaissance du texte a échoué : {e}"))
        };
        let moteur = moteur()?;
        let bgra: Vec<u8> = rvb
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[rouge, vert, bleu]| [bleu, vert, rouge, 255])
            .collect();
        let ecrivain = DataWriter::new().map_err(echec)?;
        ecrivain.WriteBytes(&bgra).map_err(echec)?;
        let tampon = ecrivain.DetachBuffer().map_err(echec)?;
        let image = SoftwareBitmap::CreateCopyFromBuffer(
            &tampon,
            BitmapPixelFormat::Bgra8,
            largeur as i32,
            hauteur as i32,
        )
        .map_err(echec)?;
        let resultat = moteur
            .RecognizeAsync(&image)
            .map_err(echec)?
            .join()
            .map_err(echec)?;
        let (l, h) = (f64::from(largeur.max(1)), f64::from(hauteur.max(1)));
        let mut lignes = Vec::new();
        for ligne in resultat.Lines().map_err(echec)? {
            let mut mots = Vec::new();
            for mot in ligne.Words().map_err(echec)? {
                let texte = mot.Text().map_err(echec)?.to_string();
                let boite = mot.BoundingRect().map_err(echec)?;
                if texte.trim().is_empty() {
                    continue;
                }
                mots.push(Mot {
                    texte,
                    boite: [
                        f64::from(boite.X) / l,
                        f64::from(boite.Y) / h,
                        f64::from(boite.Width) / l,
                        f64::from(boite.Height) / h,
                    ],
                });
            }
            if !mots.is_empty() {
                lignes.push(mots);
            }
        }
        Ok(lignes)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod systeme {
    use super::Ligne;
    use crate::erreur::Erreur;

    const ABSENT: &str = "La reconnaissance du texte n'existe que sous Windows et sur Mac.";

    pub fn disponible() -> Result<(), Erreur> {
        Err(Erreur::simple(ABSENT))
    }

    pub fn cote_max() -> u32 {
        super::COTE_MAX
    }

    pub fn reconnaitre(_: u32, _: u32, _: &[u8]) -> Result<Vec<Ligne>, Erreur> {
        Err(Erreur::simple(ABSENT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::{pdf_herite, scan};

    fn mot(texte: &str, boite: [f64; 4]) -> Mot {
        Mot {
            texte: texte.into(),
            boite,
        }
    }

    #[test]
    fn decoupe_les_mots_comme_vision_compte_les_caracteres() {
        assert_eq!(
            mots_utf16("Net à payer : 389 250"),
            vec![
                (0, 3, "Net".to_string()),
                (4, 1, "à".to_string()),
                (6, 5, "payer".to_string()),
                (12, 1, ":".to_string()),
                (14, 3, "389".to_string()),
                (18, 3, "250".to_string()),
            ]
        );
        assert_eq!(mots_utf16("  "), vec![]);
    }

    #[test]
    fn pose_une_couche_de_texte_invisible_que_la_recherche_retrouve() {
        let octets = scan("Rien");
        let mut document = pdf::charger(&octets, "scan.pdf").unwrap();
        let page = *document.get_pages().get(&1).unwrap();
        // Une ligne à 10 % du haut de la page A4 (595 × 842) : « Bulletin de paie ».
        let lignes = vec![vec![
            mot("Bulletin", [0.1, 0.1, 0.2, 0.03]),
            mot("de", [0.32, 0.1, 0.05, 0.03]),
            mot("paie", [0.39, 0.1, 0.1, 0.03]),
        ]];
        poser(&mut document, page, &lignes).unwrap();
        let ecrit = Arc::new(pdf::ecrire(document).unwrap());
        // Le texte est lu, invisible, à la place des mots.
        let textes = texte::textes(ecrit.clone(), "scan.pdf").unwrap();
        assert!(textes[0].contains("Bulletin de paie"), "{:?}", textes[0]);
        let mots = texte::mots(ecrit.clone(), "scan.pdf", 0).unwrap();
        let bulletin = mots.iter().find(|m| m.texte == "Bulletin").unwrap();
        assert!((bulletin.x - 0.1 * 595.0).abs() < 2.0, "x {}", bulletin.x);
        assert!(
            (bulletin.longueur - 0.2 * 595.0).abs() < 3.0,
            "longueur {}",
            bulletin.longueur
        );
        let haut = 0.1 * 842.0;
        assert!(
            bulletin.y > haut - 6.0 && bulletin.y < haut + 6.0,
            "y {}",
            bulletin.y
        );
        // L'image n'a pas changé : le texte posé ne se voit pas.
        let (_, _, avant) = rendu::pixels_page(Arc::new(octets), "scan.pdf", 0, 72).unwrap();
        let (_, _, apres) = rendu::pixels_page(ecrit, "scan.pdf", 0, 72).unwrap();
        assert_eq!(avant, apres);
    }

    #[test]
    fn trouve_les_pages_sans_texte_et_refuse_un_pdf_qui_en_a() {
        let octets = Arc::new(scan("Rien"));
        assert_eq!(pages_sans_texte(&octets, "scan.pdf").unwrap(), vec![0]);
        assert!(semble_numerise(octets.clone(), "scan.pdf").unwrap());
        assert!(!semble_numerise(Arc::new(pdf_herite(1, "Texte")), "texte.pdf").unwrap());
        let texte = Arc::new(pdf_herite(2, "Texte"));
        assert!(pages_sans_texte(&texte, "texte.pdf").unwrap().is_empty());
        let erreur = reconnaitre_pdf(texte, "texte.pdf", &|_, _| {}).unwrap_err();
        assert!(
            erreur.message.contains("déjà du texte"),
            "{}",
            erreur.message
        );
    }

    #[test]
    fn nomme_les_pdf_reconnus_sans_ecraser() {
        let dossier = std::env::temp_dir()
            .join("automat-pdf-tests")
            .join("ocr-cibles");
        std::fs::create_dir_all(&dossier).unwrap();
        let existant = dossier.join("Scan - texte reconnu.pdf");
        std::fs::write(&existant, b"x").unwrap();
        let cibles = cibles(
            &[
                "/a/Scan.pdf".into(),
                "/b/Scan.pdf".into(),
                "/c/Lettre.pdf".into(),
            ],
            &dossier,
        );
        let noms: Vec<String> = cibles
            .iter()
            .map(|c| c.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            noms,
            [
                "Scan - texte reconnu (2).pdf",
                "Scan - texte reconnu (3).pdf",
                "Lettre - texte reconnu.pdf"
            ]
        );
    }

    /// Vision, pour de vrai, en mémoire : le scan d'un fichier devient cherchable dans le lecteur,
    /// sans que le fichier change ; l'onglet fermé, il redevient tel quel.
    #[cfg(target_os = "macos")]
    #[test]
    fn reconnait_en_memoire_sans_toucher_au_fichier() {
        let octets = scan("Attestation de travail");
        let chemin = crate::essais::fichier_temporaire("ocr-memoire", "Attestation.pdf", &octets);
        let bilan = reconnaitre_en_memoire(&chemin, &|_, _| {}).unwrap();
        assert_eq!(bilan.pages_reconnues, 1);
        let textes = texte::textes(memoire::pdf(&chemin).unwrap(), "Attestation.pdf").unwrap();
        assert!(
            textes[0].contains("Attestation de travail"),
            "{:?}",
            textes[0]
        );
        assert_eq!(std::fs::read(&chemin).unwrap(), octets);
        memoire::reverrouiller(&chemin);
        assert!(semble_numerise(memoire::pdf(&chemin).unwrap(), "Attestation.pdf").unwrap());
    }

    /// Vision, pour de vrai : un scan de « Bulletin de paie 1 » devient cherchable.
    #[cfg(target_os = "macos")]
    #[test]
    fn vision_reconnait_le_texte_d_un_scan() {
        let octets = Arc::new(scan("Bulletin de paie"));
        let (ecrit, bilan) = reconnaitre_pdf(octets, "scan.pdf", &|_, _| {}).unwrap();
        assert_eq!(bilan.pages_lues, 1);
        assert_eq!(bilan.pages_reconnues, 1);
        let textes = texte::textes(Arc::new(ecrit), "scan.pdf").unwrap();
        assert!(textes[0].contains("Bulletin de paie"), "{:?}", textes[0]);
    }
}
